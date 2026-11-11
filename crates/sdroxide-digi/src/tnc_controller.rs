//! `TncController` — Mercury and Data2G, the TNC modems.
//!
//! Both modems are sample-clocked machines behind a TNC host
//! interface: hand one a block of received audio and it hands back a block of
//! transmit audio, asking for the key as its ARQ session demands. The engine is
//! half-duplex and burst-shaped instead — it keys, pulls transmit blocks until
//! the burst says it is done, unkeys, and goes back to delivering receive
//! audio. This controller is the bridge, and its one real job is keeping the
//! modem's clock honest across it:
//!
//! - **Unkeyed**, the receive tap drives the clock: every block goes to the
//!   modem at 8 kHz. Whatever the modem puts out once it keys starts the
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
//! Who drives the link is [`HostMux`]'s business: the operator's panel
//! (through an in-process client claimed here), the Winlink window (through
//! the handle the engine keeps), or an external program on the TCP host ports.
//! One at a time. The modem answers their lines itself; this controller only
//! carries them, and reads the replies for the panel.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use sdroxide_dsp::MonoResampler;
use sdroxide_tnc::{Holder, HostMux, Input, LocalClient, RATE, Reply, TncHandle, TncModem};
use sdroxide_types::{
    DigiConfig, DigiStatus, Mode, QsoStep, TNC_TRANSCRIPT_MAX, TncBw, TncHolder, TncLine,
    TncLineKind, TncStatus, TranscriptLine,
};

use crate::DigiEngine;
use crate::controller::DigiAction;

/// Who the panel's in-process client claims the modem as.
const PANEL: &str = "the operator's panel";

/// A gap in the receive audio shorter than this is jitter, not a gap.
const PAD_THRESHOLD_S: f64 = 0.1;
/// And one longer than this is not worth simulating sample by sample: the
/// session's timers will have expired regardless.
const PAD_MAX_S: f64 = 1.0;
/// Re-anchor the receive clock this often, so a sound card running a little
/// slow against the wall clock never accumulates into a phantom gap.
const REANCHOR_S: f64 = 30.0;
/// Stop reading a TCP client's data while this much is queued unsent, so TCP's
/// own window pushes back on it.
const HOLD_BYTES: u32 = 64 * 1024;

/// The silence the modem puts ahead of each burst when the operator has not
/// set one: an IQ SDR keys at once; a CAT rig's relay and sound card take time.
const LEAD_SDR_MS: u16 = 50;
const LEAD_CAT_MS: u16 = 150;

/// The PTT lead `cfg` asks for, or the default for the radio.
pub fn ptt_lead_ms(cfg: &DigiConfig, iq_source: bool) -> u16 {
    cfg.tnc_ptt_lead_ms.unwrap_or(if iq_source { LEAD_SDR_MS } else { LEAD_CAT_MS })
}

/// The callsign the modem answers to.
pub fn tnc_mycall(cfg: &DigiConfig) -> String {
    let c = cfg.tnc_mycall.trim();
    let c = if c.is_empty() { cfg.my_call.trim() } else { c };
    c.to_ascii_uppercase()
}

fn bw_of(mode: Mode, cfg: &DigiConfig) -> TncBw {
    let bw = if mode == Mode::Data2g { cfg.data2g_bw } else { cfg.mercury_bw };
    // A width this modem does not offer (Data2G's 1200 carried over to
    // Mercury by hand) falls to the next one up.
    let offered = TncBw::offered(mode);
    if offered.contains(&bw) { bw } else { *offered.last().unwrap_or(&TncBw::Hz2300) }
}

/// Start (or attach to) the modem for `mode`. `config_dir` is where Mercury's
/// generated ini and log go.
pub fn open_modem(
    mode: Mode,
    cfg: &DigiConfig,
    iq_source: bool,
    config_dir: PathBuf,
) -> Result<Box<dyn TncModem>, String> {
    let lead = ptt_lead_ms(cfg, iq_source);
    match mode {
        Mode::Data2g => {
            let call = tnc_mycall(cfg);
            let m = sdroxide_data2g::Data2g::new(
                if call.is_empty() { "N0CALL" } else { &call },
                None,
                f64::from(lead) / 1000.0,
            )?;
            Ok(Box::new(m))
        }
        Mode::Mercury => open_mercury(cfg, lead, config_dir),
        other => Err(format!("{} is not a TNC modem", other.label())),
    }
}

#[cfg(not(target_env = "msvc"))]
fn open_mercury(
    cfg: &DigiConfig,
    lead: u16,
    config_dir: PathBuf,
) -> Result<Box<dyn TncModem>, String> {
    let engine = sdroxide_mercury::start(&sdroxide_mercury::Config {
        private_port: cfg.mercury_port,
        config_dir,
        tx_delay_ms: lead,
    })?;
    Ok(Box::new(sdroxide_mercury::MercuryModem::new(engine)?))
}

#[cfg(target_env = "msvc")]
fn open_mercury(_: &DigiConfig, _: u16, _: PathBuf) -> Result<Box<dyn TncModem>, String> {
    Err("this build has no Mercury: it needs a POSIX toolchain (Linux, macOS, or MinGW on Windows)"
        .into())
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

pub struct TncController {
    mode: Mode,
    cfg: DigiConfig,
    clock: Clock,
    /// `None` when the modem could not be started; the reason is in the status.
    modem: Option<Box<dyn TncModem>>,
    mux: HostMux,

    rx_rs: Option<MonoResampler>,
    rx: Vec<f32>,
    out: Vec<f32>,

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

    status: TncStatus,
    status_dirty: bool,
    last_status: Option<Instant>,
}

impl TncController {
    /// A controller for `mode` around `modem` (or the reason there is none).
    pub fn new(
        mode: Mode,
        cfg: DigiConfig,
        tap_rate: f64,
        modem: Result<Box<dyn TncModem>, String>,
    ) -> Self {
        let (modem, unavailable) = match modem {
            Ok(m) => (Some(m), None),
            Err(e) => {
                tracing::warn!("{}: {e}", mode.label());
                (None, Some(e))
            }
        };
        let version = modem.as_ref().map(|m| m.version()).unwrap_or_default();
        let label = if mode == Mode::Data2g { "Data2G" } else { "Mercury" };
        let mut c = TncController {
            rx_rs: MonoResampler::new(tap_rate, RATE),
            rx: Vec::new(),
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
            status: TncStatus {
                bw: bw_of(mode, &cfg),
                version,
                unavailable: unavailable.clone(),
                ..TncStatus::default()
            },
            status_dirty: true,
            last_status: None,
            mux: HostMux::new(label),
            modem,
            mode,
            cfg,
        };
        if let Some(e) = unavailable {
            c.line(TncLineKind::Error, e);
        }
        c.sync_host_server();
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
    pub fn handle(&self) -> TncHandle {
        self.mux.handle()
    }

    fn sync_host_server(&mut self) {
        let want =
            (self.cfg.tnc_host_server && self.modem.is_some()).then_some(self.cfg.tnc_host_port);
        let have = self.mux.tcp_addrs().map(|a| a[0].port());
        if want == have || (want.is_some() && self.status.host_error.is_some()) {
            return;
        }
        match want {
            None => {
                self.mux.close_tcp();
                self.status.host_listening = None;
                self.status.host_error = None;
            }
            Some(port) => match self.mux.listen_tcp("127.0.0.1", port) {
                Ok(()) => {
                    let a = self.mux.tcp_addrs().map(|a| a[0].to_string());
                    tracing::info!(addr = ?a, "{} host ports open", self.mode.label());
                    self.status.host_listening = a;
                    self.status.host_error = None;
                }
                Err(e) => {
                    tracing::warn!(
                        "{} host ports {port}/{}: {e}",
                        self.mode.label(),
                        port.wrapping_add(1)
                    );
                    self.status.host_listening = None;
                    self.status.host_error =
                        Some(format!("ports {port}/{}: {e}", port.wrapping_add(1)));
                }
            },
        }
        self.status_dirty = true;
    }

    fn line(&mut self, kind: TncLineKind, text: impl Into<String>) {
        let t = &mut self.status.transcript;
        t.push(TncLine { at_unix: unix_now(), kind, text: text.into() });
        if t.len() > TNC_TRANSCRIPT_MAX {
            let n = t.len() - TNC_TRANSCRIPT_MAX;
            t.drain(..n);
        }
        self.status_dirty = true;
    }

    fn tcp_holds(&self) -> bool {
        matches!(self.mux.holder(), Some(Holder::Tcp(_)))
    }

    /// Run the modem over `n` samples of `self.rx` (or of silence) and route
    /// what it produced.
    fn run(&mut self, n: usize, silent: bool) {
        if n == 0 {
            return;
        }
        let Some(modem) = self.modem.as_mut() else {
            self.fed += n as u64;
            return;
        };
        if silent {
            self.rx.clear();
            self.rx.resize(n, 0.0);
        }
        self.out.clear();
        self.out.resize(n, 0.0);
        modem.process(&self.rx[..n], &mut self.out);
        self.fed += n as u64;
        if modem.keyed() {
            if self.discard {
                // Thrown away: the engine is not transmitting this burst.
            } else if !self.engine_keyed {
                if self.tx_fifo.len() - self.tx_pos < (RATE as usize) * 4 {
                    self.tx_fifo.extend_from_slice(&self.out);
                }
                self.want_key = true;
            }
        } else if self.discard {
            self.discard = false;
        }
        self.pump_host();
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
                let k = left.min(800);
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

    /// Carry the hosts' lines and bytes to the modem, and its replies back.
    fn pump_host(&mut self) {
        if self.modem.is_none() {
            return;
        }
        let mut inputs = Vec::new();
        while let Some(i) = self.mux.try_recv() {
            inputs.push(i);
        }
        let tcp = self.tcp_holds();
        for i in inputs {
            let Some(modem) = self.modem.as_mut() else { return };
            match i {
                Input::Command(l) => {
                    modem.command(&l);
                    if tcp {
                        self.line(TncLineKind::Host, format!("> {l}"));
                    }
                }
                Input::Data(d) => {
                    modem.data(&d);
                    self.status.tx_bytes += d.len() as u64;
                    if self.panel.is_some() {
                        let t = printable(&d).unwrap_or_else(|| format!("[{} bytes]", d.len()));
                        self.line(TncLineKind::Tx, t);
                    }
                }
                Input::Connected => {}
                Input::Closed => modem.client_gone(),
            }
        }
        let Some(modem) = self.modem.as_mut() else { return };
        let (lines, data) = modem.take_output();
        if !data.is_empty() {
            self.mux.data(&data);
            self.status.rx_bytes += data.len() as u64;
            if self.panel.is_some() {
                let t = printable(&data).unwrap_or_else(|| format!("[{} bytes]", data.len()));
                self.line(TncLineKind::Rx, t);
            }
        }
        for l in lines {
            self.mux.line(&l);
            self.on_reply(&l);
        }
        self.mux.hold_data(self.status.buffered > HOLD_BYTES);
    }

    fn on_reply(&mut self, l: &str) {
        let r = Reply::parse(l);
        let panel_holds = self.panel.is_some();
        self.status_dirty = true;
        match &r {
            Reply::Connected { caller, answerer, bw } => {
                let me = self.panel_call.clone().unwrap_or_else(|| tnc_mycall(&self.cfg));
                let peer = if caller.eq_ignore_ascii_case(&me) { answerer } else { caller };
                self.status.connected = true;
                self.status.pending = false;
                self.status.peer = peer.clone();
                self.status.rx_bytes = 0;
                self.status.tx_bytes = 0;
                let w = bw.as_ref().map(|b| format!(" ({b} Hz)")).unwrap_or_default();
                self.line(TncLineKind::Event, format!("Connected: {caller} → {answerer}{w}"));
            }
            Reply::Disconnected => {
                if self.status.connected {
                    let (rx, tx) = (self.status.rx_bytes, self.status.tx_bytes);
                    self.line(
                        TncLineKind::Event,
                        format!("Disconnected ({rx} bytes in, {tx} out)"),
                    );
                } else if panel_holds && !self.status.peer.is_empty() {
                    let who = self.status.peer.clone();
                    self.line(TncLineKind::Error, format!("No link with {who}"));
                }
                self.status.connected = false;
                self.status.pending = false;
                self.status.level = 0;
                self.status.bps = 0;
                self.status.snr_db = None;
                self.status.buffered = 0;
                self.status.peer.clear();
                self.call_over();
            }
            Reply::Pending => {
                self.status.pending = true;
                if panel_holds {
                    self.line(TncLineKind::Event, "Incoming call…");
                }
            }
            Reply::CancelPending => self.status.pending = false,
            Reply::Bitrate { level, bps } => {
                self.status.level = *level;
                self.status.bps = *bps;
            }
            Reply::Mode(m) => self.status.submode = m.clone(),
            Reply::Sn(s) => self.status.snr_db = Some(*s),
            Reply::Ptt(on) => self.status.ptt = *on,
            Reply::Busy(on) => self.status.busy = *on,
            Reply::Buffer(n) => self.status.buffered = *n,
            Reply::Wrong if panel_holds => self.line(TncLineKind::Error, "The modem refused that"),
            _ => {}
        }
        if self.tcp_holds() && !r.is_chatter() {
            self.line(TncLineKind::Host, format!("< {l}"));
        }
    }

    /// Take the modem for the panel, configured as the operator set it.
    fn claim_panel(&mut self) -> bool {
        if self.panel.is_some() {
            return true;
        }
        if self.modem.is_none() {
            let why = self.status.unavailable.clone().unwrap_or_default();
            self.line(TncLineKind::Error, why);
            return false;
        }
        let call = tnc_mycall(&self.cfg);
        if call.is_empty() {
            self.line(TncLineKind::Error, "Set your callsign first (modem setup)");
            return false;
        }
        match self.handle().claim(PANEL) {
            Ok(c) => {
                c.command(&format!("MYCALL {call}"));
                c.command(&bw_of(self.mode, &self.cfg).command());
                c.command(if self.listens() { "LISTEN ON" } else { "LISTEN OFF" });
                self.panel = Some(c);
                self.panel_call = Some(call);
                true
            }
            Err(e) => {
                self.line(TncLineKind::Error, e);
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
    /// let go by the call's own end, not here.
    fn listens(&self) -> bool {
        self.cfg.tnc_listen && !self.cfg.tnc_host_server
    }

    fn sync_panel_listen(&mut self) {
        if self.listens()
            && self.panel.is_none()
            && self.mux.holder().is_none()
            && self.modem.is_some()
        {
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
        let s = &mut self.status;
        s.bw = bw_of(self.mode, &self.cfg);
        if let Some(m) = &self.modem {
            s.ptt = m.keyed();
            s.busy = s.busy || m.busy();
        }
        s.listening = self.cfg.tnc_listen;
        s.holder = match self.mux.holder() {
            None => TncHolder::Nobody,
            Some(Holder::Tcp(a)) => TncHolder::Tcp(a),
            Some(Holder::Local(w)) if w == PANEL => TncHolder::Panel,
            Some(Holder::Local(_)) => TncHolder::Winlink,
        };
        let mut st = DigiStatus::idle(self.cfg.clone());
        st.mode = self.mode;
        st.step = QsoStep::Idle;
        st.audio_hz = 1500.0;
        st.transmitting = self.engine_keyed;
        st.transcript = Vec::<TranscriptLine>::new();
        st.tnc = Some(self.status.clone());
        st
    }
}

impl DigiEngine for TncController {
    fn mode(&self) -> Mode {
        self.mode
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        if self.engine_keyed {
            // Our own over; a full-duplex front end hears it, we must not.
            return;
        }
        let mut rx = std::mem::take(&mut self.rx);
        rx.clear();
        match &mut self.rx_rs {
            Some(rs) => rs.push(tap, &mut rx),
            None => rx.extend_from_slice(tap),
        }
        let n = rx.len();
        // pad_gap runs the modem on silence, which reuses the buffer.
        self.rx = Vec::new();
        self.pad_gap(n);
        self.rx = rx;
        self.run(n, false);
    }

    fn poll(&mut self, _now: SystemTime, dial_hz: f64) -> Vec<DigiAction> {
        let mut actions = Vec::new();
        // The receive path is what normally runs the modem; when no audio is
        // arriving at all, run it on silence so host commands are still
        // answered and timers still expire.
        if !self.engine_keyed
            && self.now().saturating_duration_since(self.anchor) > Duration::from_millis(500)
        {
            self.pad_gap(0);
        }
        if let Some(m) = self.modem.as_mut()
            && dial_hz > 0.0
        {
            m.set_dial_hz(dial_hz as u64);
        }
        self.drain_panel();
        self.sync_panel_listen();
        self.pump_host();
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
            out[i..].copy_from_slice(&self.out[..n]);
        }
        // Done once the modem has let go of the key and the queue is empty.
        !self.modem.as_ref().is_some_and(|m| m.keyed()) && self.tx_fifo.is_empty()
    }

    fn tx_peak(&self) -> f32 {
        self.modem.as_ref().map_or(1.0, |m| m.tx_peak())
    }

    /// Both modems make their transmit audio at 8 kHz; the engine rate-matches.
    fn tx_rate(&self) -> f64 {
        RATE
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
        if let Some(m) = self.modem.as_mut() {
            m.abort();
            if self.status.tuning {
                m.tune(false);
                self.status.tuning = false;
            }
        }
        self.pump_host();
        self.abort_tx();
    }

    /// The engine refused or cut short the key-up. The modem is left to finish
    /// the over into the void rather than told, so the session sees what it
    /// would see on the air — an over nobody answered — and retries or gives
    /// up by its own rules.
    fn abort_tx(&mut self) {
        if self.modem.as_ref().is_some_and(|m| m.keyed()) {
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
        let listen_changed = cfg.tnc_listen != self.cfg.tnc_listen;
        let host_changed = cfg.tnc_host_server != self.cfg.tnc_host_server
            || cfg.tnc_host_port != self.cfg.tnc_host_port;
        let bw_changed = bw_of(self.mode, &cfg) != bw_of(self.mode, &self.cfg);
        self.cfg = cfg;
        if host_changed {
            // A fresh attempt: the operator may have freed the port.
            self.status.host_error = None;
        }
        self.sync_host_server();
        if let Some(p) = &self.panel {
            if bw_changed {
                p.command(&bw_of(self.mode, &self.cfg).command());
            }
            if listen_changed {
                p.command(if self.listens() { "LISTEN ON" } else { "LISTEN OFF" });
            }
        }
        // Stopped listening with nothing under way: give the modem back.
        if listen_changed && !self.listens() && !self.status.connected && !self.status.pending {
            self.release_panel();
        }
        self.status_dirty = true;
    }

    /// The signal is centred by the waveform; there is no tone to move.
    fn set_audio_hz(&mut self, _hz: f32) {}

    fn audio_hz(&self) -> f32 {
        1500.0
    }

    fn status(&self) -> DigiStatus {
        let mut st = DigiStatus::idle(self.cfg.clone());
        st.mode = self.mode;
        st.tnc = Some(self.status.clone());
        st
    }

    fn clear_rx(&mut self) {
        self.tnc_term_clear();
    }

    fn tnc_connect(&mut self, call: String) {
        let call = call.trim().to_ascii_uppercase();
        if call.is_empty() || !self.claim_panel() {
            return;
        }
        let me = self.panel_call.clone().unwrap_or_default();
        self.status.peer = call.clone();
        self.line(TncLineKind::Event, format!("Calling {call}…"));
        if let Some(p) = &self.panel {
            p.command(&format!("CONNECT {me} {call}"));
        }
    }

    fn tnc_send(&mut self, text: String) {
        match &self.panel {
            Some(p) if self.status.connected => {
                let mut b = text.into_bytes();
                b.push(b'\r');
                p.send_data(&b);
            }
            _ => self.line(TncLineKind::Error, "Not connected"),
        }
    }

    fn tnc_disconnect(&mut self) {
        match &self.panel {
            Some(p) => p.command("DISCONNECT"),
            None => {
                self.line(TncLineKind::Error, "The link is not the panel's to hang up — use ABORT")
            }
        }
    }

    fn tnc_tune(&mut self, on: bool) {
        let Some(m) = self.modem.as_mut() else { return };
        if self.status.connected && on {
            self.line(TncLineKind::Error, "Cannot tune while a link is up");
        } else if m.tune(on) {
            self.status.tuning = on;
        } else if on {
            let who = self.mode.label();
            self.line(TncLineKind::Error, format!("{who} has no tune tone"));
        }
        self.status_dirty = true;
    }

    fn tnc_term_clear(&mut self) {
        self.status.transcript.clear();
        self.status_dirty = true;
    }
}
