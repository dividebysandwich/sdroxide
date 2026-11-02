//! `VaraController` — VARA HF on the vendored xara modem core.
//!
//! The modem ([`sdroxide_vara`]) is a full-duplex, sample-clocked machine: hand
//! it a block of received audio and it hands back a block of transmit audio,
//! keying and unkeying itself as the ARQ session demands. The engine is
//! half-duplex and burst-shaped instead — it keys, pulls transmit blocks until
//! the burst says it is done, unkeys, and goes back to delivering receive
//! audio. This controller is the bridge, and its one real job is keeping the
//! modem's clock honest across that bridge:
//!
//! - **Unkeyed**, the receive tap drives the clock: every block goes to the
//!   modem at 48 kHz. Whatever the modem puts out once it keys starts the
//!   burst, and the next `poll` asks the engine to key.
//! - **Keyed**, the transmit path drives it: each `fill_tx_block` runs the
//!   modem over silence for exactly as many samples as the engine asked for.
//!   Receive audio arriving meanwhile (a full-duplex front end) is not ours to
//!   hear and is dropped.
//! - **In between**, nothing arrives at all: the engine's own turnaround, and
//!   a whole source block of deafness after unkey on a sound-card rig. ARQ
//!   timing is measured against the far end's replies, so a modem whose clock
//!   stopped would hear them early. The gap is filled with silence on the wall
//!   clock before the next real block goes in.
//!
//! Who drives the link is [`sdroxide_vara::HostMux`]'s business: the operator's
//! panel (through an in-process client claimed here), the Winlink window
//! (through the handle the engine keeps), or an external program on VARA's TCP
//! ports. One at a time.

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use sdroxide_dsp::MonoResampler;
use sdroxide_types::{
    DigiConfig, DigiStatus, Mode, QsoStep, TranscriptLine, VARA_TRANSCRIPT_MAX, VaraBw, VaraHolder,
    VaraLine, VaraLineKind, VaraStatus,
};
use sdroxide_vara::{
    Bandwidth, Holder, LocalClient, Modem, Observed, RATE, Response, Vara, VaraHandle,
};

use crate::DigiEngine;
use crate::controller::DigiAction;

/// Who the panel's in-process client claims the modem as.
const PANEL: &str = "the VARA panel";

/// A gap in the receive audio shorter than this is jitter, not a gap.
const PAD_THRESHOLD_S: f64 = 0.1;
/// And one longer than this is not worth simulating sample by sample: the
/// session's timers will have expired regardless.
const PAD_MAX_S: f64 = 1.0;
/// Re-anchor the receive clock this often, so a sound card running a little
/// slow against the wall clock never accumulates into a phantom gap.
const REANCHOR_S: f64 = 30.0;

/// The transmit latency the modem assumes when the operator has not set one:
/// how long after a sample leaves `fill_tx_block` it is actually on the air.
/// An IQ SDR plays what it is given almost at once; a sound-card rig has an
/// output buffer in the way.
const LATENCY_SDR_MS: u16 = 20;
const LATENCY_CAT_MS: u16 = 120;

fn to_xara_bw(bw: VaraBw) -> Bandwidth {
    match bw {
        VaraBw::Hz500 => Bandwidth::Hz500,
        VaraBw::Hz2300 => Bandwidth::Hz2300,
        VaraBw::Hz2750 => Bandwidth::Hz2750,
    }
}

fn from_xara_bw(bw: Bandwidth) -> VaraBw {
    match bw {
        Bandwidth::Hz500 => VaraBw::Hz500,
        Bandwidth::Hz2300 => VaraBw::Hz2300,
        Bandwidth::Hz2750 => VaraBw::Hz2750,
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Payload as text for the transcript: printable ASCII kept, line ends
/// honoured, anything else (compressed B2F, binary) shown as a count.
fn printable(d: &[u8]) -> Option<String> {
    let text =
        d.iter().filter(|b| !matches!(b, b'\r' | b'\n' | b'\t')).all(|b| (0x20..0x7f).contains(b));
    text.then(|| String::from_utf8_lossy(d).replace("\r\n", "\n").replace('\r', "\n"))
}

/// Where the controller reads the wall clock from. Swappable so a test can run
/// a simulated link faster than real time with the gap-filling still exercised.
pub type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;

pub struct VaraController {
    cfg: DigiConfig,
    clock: Clock,
    modem: Vara,
    /// Whether the radio is an IQ SDR, for the default transmit latency.
    iq_source: bool,

    rx_rs: Option<MonoResampler>,
    rx_f32: Vec<f32>,
    rx_f64: Vec<f64>,
    out: Vec<f64>,

    /// The receive clock: samples fed since `anchor`, against the wall clock.
    anchor: Instant,
    fed: u64,

    /// The engine has keyed for us and is pulling transmit blocks.
    engine_keyed: bool,
    /// The engine refused (or cut short) a burst: let the modem finish it into
    /// the void, so the session sees an over that went unheard and retries.
    discard: bool,
    /// Transmit audio made while the receive path was driving the clock.
    tx_fifo: Vec<f32>,
    tx_pos: usize,
    /// The engine should be asked to key on the next poll.
    want_key: bool,

    panel: Option<LocalClient>,
    panel_call: Option<String>,

    status: VaraStatus,
    status_dirty: bool,
    last_status: Option<Instant>,
}

impl VaraController {
    /// `iq_source` picks the default transmit latency; see
    /// [`DigiConfig::vara_tx_latency_ms`].
    pub fn new(cfg: DigiConfig, tap_rate: f64, iq_source: bool) -> Self {
        let call = Self::mycall_of(&cfg);
        let modem = sdroxide_vara::new_modem(
            to_xara_bw(cfg.vara_bw),
            if call.is_empty() { "N0CALL" } else { &call },
            cfg.vara_registered,
            unix_now() as u64,
        );
        let mut c = VaraController {
            rx_rs: MonoResampler::new(tap_rate, RATE),
            rx_f32: Vec::new(),
            rx_f64: Vec::new(),
            out: Vec::new(),
            anchor: Instant::now(),
            clock: Arc::new(Instant::now),
            fed: 0,
            engine_keyed: false,
            discard: false,
            tx_fifo: Vec::new(),
            tx_pos: 0,
            want_key: false,
            panel: None,
            panel_call: None,
            status: VaraStatus { bw: cfg.vara_bw, ..VaraStatus::default() },
            status_dirty: true,
            last_status: None,
            iq_source,
            modem,
            cfg,
        };
        c.apply_config();
        c
    }

    /// Read the wall clock from `clock` instead of [`Instant::now`].
    pub fn set_clock(&mut self, clock: Clock) {
        self.anchor = clock();
        self.fed = 0;
        self.clock = clock;
    }

    fn now(&self) -> Instant {
        (self.clock)()
    }

    /// The handle the Winlink window claims the modem through.
    pub fn handle(&self) -> VaraHandle {
        self.modem.host().handle()
    }

    fn mycall_of(cfg: &DigiConfig) -> String {
        let c = cfg.vara_mycall.trim();
        let c = if c.is_empty() { cfg.my_call.trim() } else { c };
        c.to_ascii_uppercase()
    }

    fn latency_ms(&self) -> u16 {
        self.cfg.vara_tx_latency_ms.unwrap_or(if self.iq_source {
            LATENCY_SDR_MS
        } else {
            LATENCY_CAT_MS
        })
    }

    /// Push the operator's settings into the modem. The parts a running link
    /// cannot take are retried from `poll` once it is idle.
    fn apply_config(&mut self) -> bool {
        let ms = i64::from(self.latency_ms());
        self.modem.set_tx_latency(ms * sdroxide_vara::SECOND / 1000, 0);
        let call = Self::mycall_of(&self.cfg);
        let (bw, reg, retries, cwid) = (
            to_xara_bw(self.cfg.vara_bw),
            self.cfg.vara_registered,
            self.cfg.vara_retries.max(1),
            self.cfg.vara_cwid,
        );
        let applied = self.modem.reconfigure_with(|c| {
            c.bw = bw;
            c.host.registered = reg;
            c.host.retries = retries;
            c.cwid = cwid;
            if !call.is_empty() && c.host.mycalls.first().map(String::as_str) == Some("N0CALL") {
                c.host.mycalls = vec![call.clone()];
            }
        });
        self.sync_host_server();
        applied
    }

    fn sync_host_server(&mut self) {
        let want = self.cfg.vara_host_server.then_some(self.cfg.vara_host_port);
        let have = self.modem.host().tcp_addrs().map(|a| a[0].port());
        if want == have || (want.is_some() && self.status.host_error.is_some()) {
            return;
        }
        match want {
            None => {
                self.modem.host_mut().close_tcp();
                self.status.host_listening = None;
                self.status.host_error = None;
            }
            Some(port) => match self.modem.host_mut().listen_tcp("127.0.0.1", port) {
                Ok(()) => {
                    let a = self.modem.host().tcp_addrs().map(|a| a[0].to_string());
                    tracing::info!(addr = ?a, "VARA host ports open");
                    self.status.host_listening = a;
                    self.status.host_error = None;
                }
                Err(e) => {
                    tracing::warn!("VARA host ports {port}/{}: {e}", port.wrapping_add(1));
                    self.status.host_listening = None;
                    self.status.host_error =
                        Some(format!("ports {port}/{}: {e}", port.wrapping_add(1)));
                }
            },
        }
        self.status_dirty = true;
    }

    fn line(&mut self, kind: VaraLineKind, text: impl Into<String>) {
        let t = &mut self.status.transcript;
        t.push(VaraLine { at_unix: unix_now(), kind, text: text.into() });
        if t.len() > VARA_TRANSCRIPT_MAX {
            let n = t.len() - VARA_TRANSCRIPT_MAX;
            t.drain(..n);
        }
        self.status_dirty = true;
    }

    /// Run the modem over `input` (48 kHz) and route what it produced.
    fn run(&mut self, input_len: usize, silent: bool) {
        if input_len == 0 {
            return;
        }
        self.out.clear();
        self.out.resize(input_len, 0.0);
        if silent {
            self.rx_f64.clear();
            self.rx_f64.resize(input_len, 0.0);
        }
        self.modem.process(&self.rx_f64[..input_len], &mut self.out);
        self.fed += input_len as u64;
        if self.modem.keyed() {
            if self.discard {
                // Thrown away: the engine is not transmitting this burst.
            } else if !self.engine_keyed {
                if self.tx_fifo.len() - self.tx_pos < (RATE as usize) * 4 {
                    self.tx_fifo.extend(self.out.iter().map(|&v| v as f32));
                }
                self.want_key = true;
            }
        } else if self.discard {
            self.discard = false;
        }
        self.absorb();
    }

    /// Fill a gap in the receive audio with silence, on the wall clock.
    fn pad_gap(&mut self, block: usize) {
        let elapsed = self.now().saturating_duration_since(self.anchor).as_secs_f64();
        let expected = (elapsed * RATE) as i64;
        let deficit = expected - self.fed as i64 - block as i64;
        if deficit as f64 > PAD_THRESHOLD_S * RATE {
            let n = (deficit as f64).min(PAD_MAX_S * RATE) as usize;
            let mut left = n;
            while left > 0 {
                let k = left.min(4_800);
                self.run(k, true);
                left -= k;
            }
            self.reanchor();
        } else if elapsed > REANCHOR_S {
            self.reanchor();
        }
    }

    fn reanchor(&mut self) {
        self.anchor = self.now();
        self.fed = 0;
    }

    /// Fold what the modem reported into the panel's status and transcript.
    fn absorb(&mut self) {
        let panel_holds = self.panel.is_some();
        for o in self.modem.take_observed() {
            self.status_dirty = true;
            match o {
                Observed::Reply(r) => self.on_reply(r, panel_holds),
                Observed::Data(d) => {
                    self.status.rx_bytes += d.len() as u64;
                    if panel_holds {
                        let t = printable(&d).unwrap_or_else(|| format!("[{} bytes]", d.len()));
                        self.line(VaraLineKind::Rx, t);
                    }
                }
                Observed::Sent(d) => {
                    self.status.tx_bytes += d.len() as u64;
                    if panel_holds {
                        let t = printable(&d).unwrap_or_else(|| format!("[{} bytes]", d.len()));
                        self.line(VaraLineKind::Tx, t);
                    }
                }
                Observed::Command(c) => {
                    if matches!(self.modem.host().holder(), Some(Holder::Tcp(_))) {
                        self.line(VaraLineKind::Host, format!("> {c}"));
                    }
                }
                Observed::Trace(_) => {}
                Observed::UnableToConnect => {
                    let who = std::mem::take(&mut self.status.peer);
                    self.line(VaraLineKind::Error, format!("No answer from {who}"));
                    self.call_over();
                }
                Observed::TimedOut => {
                    self.line(VaraLineKind::Error, "Link timed out");
                    self.call_over();
                }
            }
        }
    }

    fn on_reply(&mut self, r: Response, panel_holds: bool) {
        match &r {
            Response::Connected { caller, answerer, width, .. } => {
                let me = Self::mycall_of(&self.cfg);
                let peer = if caller.eq_ignore_ascii_case(&me) { answerer } else { caller };
                self.status.connected = true;
                self.status.peer = peer.clone();
                self.status.rx_bytes = 0;
                self.status.tx_bytes = 0;
                let w = match width {
                    sdroxide_vara::LinkWidth::Hz(hz) => format!(" ({hz} Hz)"),
                    _ => String::new(),
                };
                self.line(VaraLineKind::Event, format!("Connected: {caller} → {answerer}{w}"));
            }
            Response::Disconnected => {
                if self.status.connected {
                    let (rx, tx) = (self.status.rx_bytes, self.status.tx_bytes);
                    self.line(
                        VaraLineKind::Event,
                        format!("Disconnected ({rx} bytes in, {tx} out)"),
                    );
                }
                self.status.connected = false;
                self.status.level = 0;
                self.status.bps = 0;
                self.status.snr_db = None;
                self.call_over();
            }
            Response::Bitrate { level, bps, .. } => {
                self.status.level = *level as u8;
                self.status.bps = *bps;
            }
            Response::Sn(s) => self.status.snr_db = Some(*s as f32),
            Response::Ptt(on) => self.status.ptt = *on,
            Response::Buffer(n) => self.status.buffered = *n as u32,
            Response::Pending => self.line(VaraLineKind::Event, "Incoming call…"),
            Response::Wrong if panel_holds => {
                self.line(VaraLineKind::Error, "The modem refused that")
            }
            _ => {}
        }
        if matches!(self.modem.host().holder(), Some(Holder::Tcp(_)))
            && !matches!(
                r,
                Response::Ptt(_) | Response::Busy(_) | Response::Buffer(_) | Response::IAmAlive
            )
        {
            self.line(VaraLineKind::Host, format!("< {r}"));
        }
    }

    /// Take the modem for the panel, configured as the operator set it.
    fn claim_panel(&mut self) -> bool {
        if self.panel.is_some() {
            return true;
        }
        let call = Self::mycall_of(&self.cfg);
        if call.is_empty() {
            self.line(VaraLineKind::Error, "Set your callsign first (VARA setup)");
            return false;
        }
        match self.handle().claim(PANEL) {
            Ok(c) => {
                c.command(&format!("MYCALL {call}"));
                c.command(&format!("BW{}", self.cfg.vara_bw.hz()));
                c.command(if self.listens() { "LISTEN ON" } else { "LISTEN OFF" });
                self.panel = Some(c);
                self.panel_call = Some(call);
                true
            }
            Err(e) => {
                self.line(VaraLineKind::Error, e);
                false
            }
        }
    }

    fn release_panel(&mut self) {
        self.panel = None;
        self.panel_call = None;
        self.status_dirty = true;
    }

    /// Listening means holding the modem whenever nobody else does, so calls
    /// are answered. Not while the TCP host ports are offered — an idle panel
    /// holding the modem would turn every program away at the door, and a
    /// program that connects wants the calls. The claim a CONNECT makes is
    /// let go by the call's own end (hang-up, no answer), not here.
    fn listens(&self) -> bool {
        self.cfg.vara_listen && !self.cfg.vara_host_server
    }

    fn sync_panel_listen(&mut self) {
        if self.listens() && self.panel.is_none() && self.modem.host().holder().is_none() {
            self.claim_panel();
        }
    }

    /// The panel's call has ended; give the modem back unless listening.
    fn call_over(&mut self) {
        if self.panel.is_some() && !self.listens() {
            self.release_panel();
        }
    }

    fn drain_panel(&mut self) {
        if let Some(p) = &self.panel {
            while p.try_recv_line().is_some() {}
            while p.try_recv_data().is_ok() {}
        }
    }

    fn build_status(&mut self) -> DigiStatus {
        let link = self.modem.station().link();
        let s = &mut self.status;
        s.state = link.state;
        s.attempts = link.attempts;
        s.afc_hz = link.afc_hz.map(|v| v as f32);
        s.bw = from_xara_bw(self.modem.station().bandwidth());
        s.ptt = self.modem.keyed();
        // The channel detector itself, not the `BUSY` lines a host is sent:
        // VARA also toggles those when a host attaches, which is protocol
        // ritual rather than news about the band.
        s.busy = self.modem.station().busy();
        s.tuning = self.modem.tuning();
        s.listening = self.cfg.vara_listen;
        s.holder = match self.modem.host().holder() {
            None => VaraHolder::Nobody,
            Some(Holder::Tcp(a)) => VaraHolder::Tcp(a),
            Some(Holder::Local(w)) if w == PANEL => VaraHolder::Panel,
            Some(Holder::Local(_)) => VaraHolder::Winlink,
        };
        if !s.connected && !link.peer.is_empty() {
            s.peer = link.peer;
        }
        let mut st = DigiStatus::idle(self.cfg.clone());
        st.mode = Mode::VaraHf;
        st.step = QsoStep::Idle;
        st.audio_hz = 1500.0;
        st.transmitting = self.engine_keyed;
        st.transcript = Vec::<TranscriptLine>::new();
        st.vara = Some(self.status.clone());
        st
    }
}

impl DigiEngine for VaraController {
    fn mode(&self) -> Mode {
        Mode::VaraHf
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        if self.engine_keyed {
            // Our own over; a full-duplex front end hears it, we must not.
            return;
        }
        self.rx_f32.clear();
        match &mut self.rx_rs {
            Some(rs) => rs.push(tap, &mut self.rx_f32),
            None => self.rx_f32.extend_from_slice(tap),
        }
        let n = self.rx_f32.len();
        self.pad_gap(n);
        self.rx_f64.clear();
        self.rx_f64.extend(self.rx_f32.iter().map(|&v| f64::from(v)));
        self.run(n, false);
    }

    fn poll(&mut self, _now: SystemTime, _dial_hz: f64) -> Vec<DigiAction> {
        let mut actions = Vec::new();
        // The receive path is what normally runs the modem; when no audio is
        // arriving at all, run it on silence so host commands are still
        // answered and timers still expire.
        if !self.engine_keyed
            && self.now().saturating_duration_since(self.anchor) > Duration::from_millis(500)
        {
            self.pad_gap(0);
        }
        self.drain_panel();
        self.sync_panel_listen();
        self.absorb();
        if self.want_key && !self.engine_keyed {
            self.want_key = false;
            self.engine_keyed = true;
            actions.push(DigiAction::KeyTx);
        }
        let due = self.last_status.is_none_or(|t| t.elapsed() > Duration::from_millis(250));
        if self.status_dirty || due {
            self.status_dirty = false;
            self.last_status = Some(Instant::now());
            actions.push(DigiAction::Status(self.build_status()));
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        self.engine_keyed
    }

    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        let mut i = 0;
        while i < out.len() && self.tx_pos < self.tx_fifo.len() {
            out[i] = self.tx_fifo[self.tx_pos];
            i += 1;
            self.tx_pos += 1;
        }
        if self.tx_pos >= self.tx_fifo.len() {
            self.tx_fifo.clear();
            self.tx_pos = 0;
        }
        if i < out.len() {
            let n = out.len() - i;
            self.run(n, true);
            for (o, v) in out[i..].iter_mut().zip(&self.out) {
                *o = *v as f32;
            }
        }
        // Done once the modem has let go of the key and the queue is empty.
        !self.modem.keyed() && self.tx_fifo.is_empty()
    }

    fn tx_peak(&self) -> f32 {
        sdroxide_vara::TX_PEAK
    }

    fn low_latency_rx(&self) -> bool {
        true
    }

    fn on_burst_done(&mut self) {
        self.engine_keyed = false;
        self.reanchor();
        self.status_dirty = true;
    }

    /// The operator's stop: the link goes, whoever holds it.
    fn abort(&mut self) {
        self.modem.abort();
        self.absorb();
        self.abort_tx();
    }

    /// The engine refused or cut short the key-up. The modem is left to finish
    /// the over into the void rather than told, so the session sees what it
    /// would see on the air — an over nobody answered — and retries or gives
    /// up by its own rules.
    fn abort_tx(&mut self) {
        if self.modem.keyed() {
            self.discard = true;
        }
        self.engine_keyed = false;
        self.want_key = false;
        self.tx_fifo.clear();
        self.tx_pos = 0;
        self.reanchor();
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        let listen_changed = cfg.vara_listen != self.cfg.vara_listen;
        let host_changed = cfg.vara_host_server != self.cfg.vara_host_server
            || cfg.vara_host_port != self.cfg.vara_host_port;
        self.cfg = cfg;
        if host_changed {
            // A fresh attempt: the operator may have freed the port.
            self.status.host_error = None;
        }
        self.apply_config();
        if listen_changed && let Some(p) = &self.panel {
            p.command(if self.listens() { "LISTEN ON" } else { "LISTEN OFF" });
            // Stopped listening with nothing under way: give the modem back.
            if !self.listens() && !self.status.connected && self.modem.station().idle() {
                self.release_panel();
            }
        }
        self.status_dirty = true;
    }

    /// The signal is centred on 1500 Hz by the waveform; there is no tone to
    /// move.
    fn set_audio_hz(&mut self, _hz: f32) {}

    fn audio_hz(&self) -> f32 {
        1500.0
    }

    fn status(&self) -> DigiStatus {
        let mut st = DigiStatus::idle(self.cfg.clone());
        st.mode = Mode::VaraHf;
        st.vara = Some(self.status.clone());
        st
    }

    fn clear_rx(&mut self) {
        self.vara_term_clear();
    }

    fn vara_connect(&mut self, call: String) {
        let call = call.trim().to_ascii_uppercase();
        if call.is_empty() {
            return;
        }
        if !self.claim_panel() {
            return;
        }
        let me = self.panel_call.clone().unwrap_or_default();
        self.status.peer = call.clone();
        self.line(VaraLineKind::Event, format!("Calling {call}…"));
        if let Some(p) = &self.panel {
            p.command(&format!("CONNECT {me} {call}"));
        }
    }

    fn vara_send(&mut self, text: String) {
        match &self.panel {
            Some(p) if self.status.connected => {
                let mut b = text.into_bytes();
                b.push(b'\r');
                p.send_data(&b);
            }
            _ => self.line(VaraLineKind::Error, "Not connected"),
        }
    }

    fn vara_disconnect(&mut self) {
        match &self.panel {
            Some(p) => p.command("DISCONNECT"),
            None => {
                self.line(VaraLineKind::Error, "The link is not the panel's to hang up — use ABORT")
            }
        }
    }

    fn vara_tune(&mut self, on: bool) {
        if !self.modem.tune(on) && on {
            self.line(VaraLineKind::Error, "Cannot tune while a link is up");
        }
        self.status_dirty = true;
    }

    fn vara_term_clear(&mut self) {
        self.status.transcript.clear();
        self.status_dirty = true;
    }
}
