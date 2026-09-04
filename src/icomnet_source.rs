//! An [`IqSource`] for an Icom reached over its LAN or WiFi port.
//!
//! The radio is both the front end and the rig: control, audio and the
//! spectrum scope all arrive on one connection, so this source owns the whole
//! conversation the way [`crate::audio_cat_source::AudioCatSource`] owns a
//! serial port and a sound card.
//!
//! # Two receive paths
//!
//! No Icom offers I/Q over its network port — the IC-7760's RF deck has a USB
//! 3.0 socket that does, at 1.92 Msps, but it is reached through a
//! manufacturer-supplied FTDI D3XX driver and shares nothing with this
//! protocol. What the LAN gives instead is two different things, and the
//! operator picks between them:
//!
//! * **AF** — the radio demodulates and we get audio, exactly as with a CAT rig
//!   and a sound card. `caps.audio_mode` is set, and the *main* panadapter is
//!   the radio's own scope rather than an FFT of that audio: what the rig sends
//!   here is not a picture of the band, it is a picture of what already came
//!   through its filter, one-sided by construction and never wider than that
//!   filter. (The audio FFT comes back for the digital modes, which place
//!   stations by their offset inside the passband.)
//! * **12 kHz IF** — Icom's DRM output, a real IF in the audio stream. Mixed to
//!   baseband and decimated by two here, it becomes a 24 kHz complex stream the
//!   engine treats like any other front end, so sdroxide's own filters, noise
//!   reduction and decoders apply over roughly ±12 kHz.
//!
//! Either way the *wide* panadapter is fed from the radio's own `27 00` scope
//! sweep through [`IqSource::wide_spectrum_db`] — 475 finished bins covering up
//! to ±500 kHz. That is not I/Q and cannot be demodulated; it is a picture, and
//! it is the only wide view an Icom has. The span it sweeps is commanded from
//! here (`IcomNetConfig::scope_span`) rather than left at whatever the operator
//! last set on the radio's own screen, which is what made the strip come up
//! barely wider than the ±12 kHz IF beneath it.
//!
//! # Status
//!
//! An IC-705 has received through this over WiFi, on the 12 kHz IF. An IC-7760
//! has connected over its RF deck's LAN port and streamed audio, its dial, its
//! meter and its scope (issue #183) — but that session ran *before* the model
//! was in the table, so what the trace proves is the transport, not the menu
//! writes or the 689-bin sweep now built from its CI-V reference guide. The one
//! thing that model has since been reported to do differently is its 12 kHz IF,
//! which arrives mirrored: see `sdroxide_icomnet::protocol::Model::if_inverted`
//! for what that costs and how little of it Icom documents.
//! Everything else — transmit, the other models' menu numbering, the meters —
//! is still only exercised against `sdroxide_icomnet::sim`, and the session
//! trace is copyable from the settings tab so a user with a radio can report
//! what actually happened.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use sdroxide_cat::{DUPLEX_FLOOR_HZ, civ};
use sdroxide_dsp::Ddc;
use sdroxide_icomnet::{IcomNetDevice, IcomNetOptions};
use sdroxide_radio::rtrb;
use sdroxide_radio::{Complex32, ControlUpdate, IqSource, Result};
use sdroxide_types::{Band, CwKeying, IcomNetConfig, IcomRxSource, Mode, TxTelemetry};

use crate::dial::Dial;
use crate::session_trace::TraceStore;

/// The last session's trace of each radio, kept after the source is dropped.
/// See [`crate::session_trace`] — including why there is one per radio rather
/// than one for the process.
static TRACES: TraceStore = TraceStore::new();

/// What tells two Icoms on one LAN apart, in the one form both sides of the
/// question can spell: the source knows what it dialled, and the tab asking for
/// the report knows what its own `radio.json` says.
fn session_key(cfg: &IcomNetConfig) -> String {
    format!("{}:{}", cfg.address.trim().to_ascii_lowercase(), cfg.control_port)
}

/// The trace of the radio *this* configuration names, and an explanation of
/// what to do with it either way — the settings dialog offers the button
/// whether or not a session has run.
pub fn diagnostics_or_hint(cfg: &IcomNetConfig) -> String {
    match TRACES.get(&session_key(cfg)) {
        Some(t) => format!("{t}\n{}\n", sdroxide_icomnet::FIELD_REPORT_HINT),
        None => format!(
            "No Icom LAN session has run yet for {} — connect to that radio first.\n\n{}\n",
            if cfg.address.trim().is_empty() { "this radio" } else { cfg.address.trim() },
            sdroxide_icomnet::FIELD_REPORT_HINT
        ),
    }
}

fn record_trace(key: &str, dev: &IcomNetDevice) {
    TRACES.record(key, dev.trace().dump());
}

/// The IF the radio puts in the audio stream when its LAN output is set to IF.
/// Fixed by Icom — it is the DRM standard's intermediate frequency, which is
/// why the setting exists at all.
const IF_CENTER_HZ: f64 = 12_000.0;

/// Complex rate the 12 kHz IF is delivered at. Exactly half the 48 kHz stream,
/// which puts the usable band at ±12 kHz — the whole of what a real IF at that
/// centre can carry.
const IF_OUT_RATE: f64 = 24_000.0;

/// How long a reading from the rig's own meter stands in for the next one, as
/// for a serial CAT rig: enough to cover an ordinary gap between answers, not
/// enough to leave a needle standing on a link that has gone quiet.
const METER_MAX_AGE: Duration = Duration::from_millis(1500);

/// How often to ask the radio where it is tuned and what its meter reads.
const POLL_PERIOD: Duration = Duration::from_millis(200);

/// How long a mode we have commanded outranks the mode the radio reports.
///
/// The mode is polled every [`POLL_PERIOD`], so a read issued just before the
/// operator changed mode comes back carrying the mode the radio was in a moment
/// ago — and adopting that would drag the app straight back off the mode it had
/// just chosen. Several poll periods, to cover a WiFi link's latency, and no
/// more: a radio that simply will not go where it was told has to be allowed to
/// win, or the two stay out of step for the rest of the session.
const MODE_SETTLE: Duration = Duration::from_millis(1000);

/// How long to wait at open for the radio to say what is in the Set-mode menu
/// items this session is about to write over (issue #252).
///
/// Best-effort: an item that goes unanswered is simply one that will not be put
/// back, which is where this backend stood before. Two poll periods is a
/// generous allowance for a LAN — and for the WiFi an IC-705 is usually on —
/// without holding the connect open for a radio that never answers.
const MENU_READ_WAIT: Duration = Duration::from_millis(500);

/// How long to keep the session up after sending the menu items back, so a
/// restore the radio missed can still be asked for.
///
/// The frames are written to the socket before the shutdown that follows them
/// — the session thread drains its queue in order — but this is UDP, and the
/// only thing that recovers a lost datagram is the radio noticing the gap and
/// asking for it (see `sdroxide_icomnet::stream`). That request has to find a
/// session still standing.
const MENU_RESTORE_SETTLE: Duration = Duration::from_millis(250);

/// A scope that has sent nothing for this long has stopped. Sweeps arrive
/// about ten times a second, so this is a silence, not a slow sweep — and it is
/// long enough to sit through a band change or a menu the operator opened on
/// the radio without reading it as a fault.
const SCOPE_STALL: Duration = Duration::from_secs(3);

/// How soon after a stall to ask the radio again, and the ceiling the interval
/// backs off to while it stays quiet. The enables are idempotent, but a radio
/// with no scope at all — or one whose operator has switched it off — must not
/// be asked twice a second forever.
const SCOPE_RETRY: Duration = Duration::from_secs(2);
const SCOPE_RETRY_MAX: Duration = Duration::from_secs(30);

/// How long to leave between repeats of the antenna-socket read, and how many
/// of them to send, while the radio has answered none. See where they are
/// spent in `IcomNetSource::pump`.
const ANTENNA_PROBE_RETRY: Duration = Duration::from_secs(2);
const ANTENNA_PROBE_RETRIES: u8 = 3;

/// The scope's amplitude scale runs from 0 to the model's own full scale, and
/// Icom documents no dB per step for any of them. The engine's `auto_levels`
/// normalises whatever it is given, so this only has to put the trace in a
/// plausible range and keep it linear; the constant is a calibration knob for
/// whoever first sees one against a known signal.
const SCOPE_DB_PER_UNIT: f32 = 0.5;

pub struct IcomNetSource {
    dev: IcomNetDevice,
    /// Which radio this session is with, for [`TRACES`]. Held rather than
    /// rebuilt from the configuration on the way out: the trace has to be
    /// filed under the address it was actually dialled at, even if the
    /// operator has retyped that address since.
    key: String,
    audio: rtrb::Consumer<f32>,
    tx: Option<rtrb::Producer<f32>>,
    rate: f64,
    rx_source: IcomRxSource,
    audio_bw: f64,
    cw_keying: CwKeying,
    civ_addr: u8,
    label: String,
    status: Option<String>,

    dial: Dial,
    /// Rolling buffer for the CI-V parser. Frames arrive whole, but reusing the
    /// tested parser costs nothing and it drops our own echoes for us.
    civ_buf: Vec<u8>,
    pending: Vec<ControlUpdate>,

    // The 12 kHz IF path.
    ddc: Option<Ddc>,
    if_in: Vec<Complex32>,
    if_out: VecDeque<Complex32>,

    // The radio's own scope.
    scope: civ::ScopeAssembler,
    scope_frame: Option<(f64, f64, Vec<u8>)>,
    /// Whether this session uses the radio's scope at all, and the half-span it
    /// was asked for — both kept so the watchdog can ask again.
    scope_wanted: bool,
    scope_half_span: Option<f64>,
    /// When the last complete sweep landed — the watchdog's clock.
    last_sweep: Instant,
    /// When the watchdog last re-asserted the enables, and how long it waits
    /// before doing so again.
    last_scope_nudge: Instant,
    scope_retry: Duration,
    /// Whether the scope is currently considered stopped, so the log says so
    /// once per episode rather than once per attempt.
    scope_stalled: bool,

    last_poll: Instant,
    /// The CI-V mode byte last commanded from here, and when — see
    /// [`MODE_SETTLE`]. Held as the wire byte rather than a [`Mode`] because
    /// the two do not spell the same thing: every digital mode is commanded as
    /// plain USB, and comes back as plain USB.
    mode_cmd: Option<(u8, Instant)>,
    last_signal: Option<(Instant, f32)>,
    last_telem: Option<TxTelemetry>,
    transmitting: bool,
    /// Whether the squelch level has been settled — by the radio answering the
    /// opening read, or by this end setting one. What it suppresses is an
    /// answer that crossed a command on the wire, which would otherwise put the
    /// rail back where the radio was before the operator moved it.
    squelch_set: bool,
    /// What the radio last said its break-in was set to, from the read
    /// `configure` sends and from this end's own writes. `None` until it has
    /// answered. See [`Self::send_cw`] for why it is worth knowing.
    break_in: Option<civ::BreakIn>,
    /// Which socket the radio says it is on, as the command numbers them:
    /// `None` until it has answered the read `configure` sends, which a radio
    /// with neither a selector nor a receiving antenna NAKs instead — see
    /// [`civ::read_antenna_frame`].
    socket: Option<u8>,
    /// Whether the radio's separate *receiving* antenna is switched into the
    /// receive path. `None` on a radio whose antenna reply was the socket
    /// alone, which is how it says it has no such connector (issue #229).
    rx_ant: Option<bool>,
    /// How many more times to ask, and when the last ask went out. See where
    /// they are spent in [`Self::pump`].
    antenna_probes_left: u8,
    last_antenna_probe: Instant,
    /// The dial this end has already put the radio's own repeater shift back
    /// to simplex for — see [`civ::simplex_frame`], and [`Self::pump`] for why
    /// it is a dial rather than a one-off.
    simplex_dial: Option<f64>,
    /// The Set-mode menu items [`Self::configure`] is asking about, and the
    /// answers as they arrive. Emptied once the reads are done with, so a
    /// later `1A` reply — the operator reading a menu on another client — is
    /// not mistaken for one of them.
    menu_read: Vec<(u16, Option<Vec<u8>>)>,
    /// What those items held before this session wrote over them, sent back
    /// when it ends (issue #252). Only the ones actually being changed: a rig
    /// already set to LAN has nothing to restore, which is also what keeps a
    /// second session on the same radio from "restoring" the first one's LAN.
    menu_restore: Vec<(u16, Vec<u8>)>,
}

impl IcomNetSource {
    /// The sockets this model's selector has, by name. Empty on a radio with
    /// one, whose `0x12` is not a selector at all — see
    /// `IcomModel::antenna_sockets`, which this mirrors.
    fn antennas(&self) -> &'static [&'static str] {
        let n = self.dev.info().model.antenna_sockets;
        if n > 1 { &civ::ANTENNAS[..n.min(civ::ANTENNAS.len())] } else { &[] }
    }

    /// How many sockets this model has, selector or not.
    fn antenna_sockets(&self) -> usize {
        self.dev.info().model.antenna_sockets
    }

    /// Connect, then put the radio into the state this session needs.
    pub fn open(cfg: &IcomNetConfig) -> anyhow::Result<IcomNetSource> {
        let rx_source = cfg.effective_rx_source();
        // The trace lives out here so a connect that *fails* still leaves one
        // behind — the wedged-radio and wrong-address sessions are exactly the
        // ones a bug report is filed about, and they never become a device.
        let trace = sdroxide_icomnet::Trace::new();
        let dev = IcomNetDevice::connect_traced(
            IcomNetOptions {
                address: cfg.address.clone(),
                control_port: cfg.control_port,
                username: cfg.username.clone(),
                password: cfg.password.clone(),
                client_name: "sdroxide".into(),
                rx_sample_rate: cfg.sample_rate_hz,
                tx_sample_rate: cfg.sample_rate_hz,
                tx_buffer_ms: cfg.tx_latency_ms,
                civ_address_override: cfg.civ_address_override,
                timeout: Duration::from_secs(10),
            },
            trace.clone(),
        );
        let key = session_key(cfg);
        let dev = match dev {
            Ok(dev) => dev,
            Err(e) => {
                TRACES.record(&key, trace.dump());
                return Err(e.into());
            }
        };

        let info = dev.info().clone();
        let audio = dev.take_audio_rx().ok_or_else(|| anyhow::anyhow!("no receive stream"))?;
        let tx = dev.take_audio_tx();
        let civ_addr = info.civ_address;

        // The model's own answer unless the operator has overruled it. Read
        // here rather than in `configure`, because the mixer below is built
        // once and the radio cannot change under a session.
        let invert_if = cfg.invert_if.unwrap_or(info.model.if_inverted);

        let mut notes = Vec::new();
        if rx_source == IcomRxSource::If12k && invert_if {
            tracing::info!(
                radio = %info.radio_name,
                overridden = cfg.invert_if.is_some(),
                "Icom LAN: mirroring the 12 kHz IF about its centre"
            );
        }
        if cfg.rx_source == IcomRxSource::If12k && rx_source == IcomRxSource::Af {
            notes.push(format!(
                "The 12 kHz IF needs a 48 kHz audio stream; this connection asked for {} Hz, \
                 so the radio's demodulated audio is being used instead.",
                cfg.sample_rate_hz
            ));
        }
        if !info.can_transmit {
            notes.push(format!("{} offered no transmit stream — receive only.", info.radio_name));
        }

        let mut src = IcomNetSource {
            dev,
            key,
            audio,
            tx,
            rate: match rx_source {
                IcomRxSource::Af => f64::from(cfg.sample_rate_hz),
                IcomRxSource::If12k => IF_OUT_RATE,
            },
            rx_source,
            audio_bw: cfg.audio_bw_hz,
            cw_keying: cfg.cw_keying,
            civ_addr,
            label: format!(
                "{} over LAN at {} ({})",
                info.radio_name,
                cfg.address,
                match rx_source {
                    IcomRxSource::Af => "AF",
                    IcomRxSource::If12k => "12 kHz IF",
                }
            ),
            status: None,
            dial: Dial::default(),
            civ_buf: Vec::with_capacity(1024),
            pending: Vec::new(),
            ddc: (rx_source == IcomRxSource::If12k).then(|| {
                let mut d = Ddc::new(f64::from(cfg.sample_rate_hz), IF_OUT_RATE);
                // Which of the real IF's two halves lands on DC, and so which
                // way up the baseband comes out. Mixing down from +12 kHz keeps
                // it; mixing *up* from -12 kHz lands on the mirror instead and
                // conjugates the lot, which is the whole of the fix for a radio
                // whose IF arrives the other way round. See
                // `Model::if_inverted`.
                d.set_offset_hz(if invert_if { -IF_CENTER_HZ } else { IF_CENTER_HZ });
                d
            }),
            if_in: Vec::new(),
            if_out: VecDeque::new(),
            scope: civ::ScopeAssembler::default(),
            scope_frame: None,
            scope_wanted: false,
            scope_half_span: None,
            last_sweep: Instant::now(),
            last_scope_nudge: Instant::now(),
            scope_retry: SCOPE_RETRY,
            scope_stalled: false,
            last_poll: Instant::now(),
            mode_cmd: None,
            last_signal: None,
            last_telem: None,
            transmitting: false,
            squelch_set: false,
            break_in: None,
            socket: None,
            rx_ant: None,
            antenna_probes_left: ANTENNA_PROBE_RETRIES,
            last_antenna_probe: Instant::now(),
            simplex_dial: None,
            menu_read: Vec::new(),
            menu_restore: Vec::new(),
        };
        notes.extend(src.configure(cfg));
        // Adopt the radio's current dial before returning, the way the CAT
        // backend does with `query_once`. Without this the source hands the
        // engine a centre of 0 Hz — the `Dial::default()` above, before the
        // first frequency reply has folded in — and the engine clamps that up
        // to the receiver's lowest frequency and *commands the radio there*.
        // That is the "it jumps to ~30 kHz on reconnect / power-toggle" bug:
        // `configure` has already asked for the dial, so this only waits for
        // the answer.
        src.adopt_initial_dial();
        src.status = (!notes.is_empty()).then(|| notes.join("\n"));
        Ok(src)
    }

    /// Put the radio into the state the session needs, and say what could not
    /// be done. Returns operator-facing notes, not errors: none of this is
    /// fatal, and a session that receives is worth having even when a menu item
    /// could not be written.
    fn configure(&mut self, cfg: &IcomNetConfig) -> Vec<String> {
        let mut notes = Vec::new();
        let model = self.dev.info().model;

        // The rig may still be holding an offset from a previous session; we
        // carry RIT, XIT and split on the dial ourselves.
        for f in civ::clear_offsets_frames(self.civ_addr) {
            self.send(f);
        }
        self.send(civ::read_freq_frame(self.civ_addr));
        self.send(civ::read_mode_frame(self.civ_addr));
        // What the radio's power is set to, so the Drive slider starts where the
        // radio already is instead of imposing a remembered level on it.
        self.send(civ::read_power_frame(self.civ_addr));
        // And where its squelch is, adopted the same way and for the same
        // reason — on AF it is the gate the operator hears (issue #192).
        self.send(civ::read_squelch_frame(self.civ_addr));
        // Which antenna socket it is on — and, because a radio with a single
        // connector NAKs this rather than answering it, whether there is a
        // selector here to offer at all (issue #238).
        self.send(civ::read_antenna_frame(self.civ_addr));
        // Whether break-in is on, which is what decides whether a CW message
        // handed to the radio's own keyer is transmitted at all. Not adopted —
        // `send_cw` asserts it — but read so an operator running full break-in
        // is not knocked down to semi by their own first message (issue #282).
        self.send(civ::read_break_in_frame(self.civ_addr));

        // Which Set-mode menu items this session has to put its own value in,
        // collected before any of them is written: they are read back first,
        // and a read has to go out ahead of every write, not just its own.
        let mut writes: Vec<(u16, u8)> = Vec::new();
        match model.lan_afif_select {
            Some(item) => writes.push((item, cfg.rx_source.menu_value())),
            None if cfg.rx_source == IcomRxSource::If12k => notes.push(
                "sdroxide does not know this model's menu numbering, so it cannot switch the \
                 LAN output to IF — set SET > Connectors > LAN AF/IF Output > Output Select \
                 to IF on the radio."
                    .into(),
            ),
            None => {}
        }

        // Only where there is something to modulate. A receive-only radio has no
        // modulation input to set and no transmit audio that could go unheard,
        // so both the write and its warning are noise — and an IC-R8600 got the
        // warning, because the menu numbering of a set that cannot transmit is
        // naturally not in the table.
        if cfg.set_mod_input_on_open && self.can_transmit() {
            match model.lan_mod_input {
                Some((items, lan)) => {
                    // DATA-OFF and every DATA slot the radio has: two on an
                    // IC-7300MK2, four on an IC-7760.
                    writes.extend(items.iter().map(|item| (*item, lan)));
                }
                None => notes.push(
                    "sdroxide does not know this model's menu numbering, so it cannot switch \
                     the modulation input to LAN — set SET > Connectors > MOD Input on the \
                     radio, or transmit audio will not be heard."
                        .into(),
                ),
            }
        }
        self.claim_menu_items(&writes);

        if cfg.scope {
            self.scope_wanted = true;
            self.scope_half_span = cfg.scope_span.half_span_hz();
            self.enable_scope();
        }
        notes
    }

    /// Take over the Set-mode menu items in `writes`: ask what is in each of
    /// them, write this session's value, and remember whatever the operator had
    /// there so it can be handed back by [`Self::restore_menu`].
    ///
    /// The reason the read comes first is issue #252: a radio left with its
    /// DATA and DATA-OFF modulation inputs on LAN is deaf to its own microphone
    /// and its own key the next time it is used standalone in the shack, and
    /// nothing on the radio says why. sdroxide changes the setting because it
    /// has to — transmit audio arrives over the network, and the rig only
    /// listens to one input at a time — but that is a loan, not a keep.
    ///
    /// Best-effort by construction: an item the radio does not answer for is
    /// still written, just not recorded, and the session is no worse off than
    /// it was before this existed.
    fn claim_menu_items(&mut self, writes: &[(u16, u8)]) {
        if writes.is_empty() {
            return;
        }
        self.menu_read = writes.iter().map(|(item, _)| (*item, None)).collect();
        for (item, _) in writes {
            self.send(civ::read_menu_frame(self.civ_addr, *item));
        }
        let deadline = Instant::now() + MENU_READ_WAIT;
        while self.menu_read.iter().any(|(_, v)| v.is_none()) && Instant::now() < deadline {
            self.pump();
            std::thread::sleep(Duration::from_millis(5));
        }

        for (item, value) in writes {
            // An item the radio already had this session's value in is not one
            // this session changed, so putting it back is not ours to do. That
            // is also what stops a second tab on the same radio from handing
            // the first tab's LAN setting back at the end of its own session.
            let before = self
                .menu_read
                .iter()
                .find(|(i, _)| i == item)
                .and_then(|(_, v)| v.clone())
                .filter(|v| v.as_slice() != [*value]);
            if let Some(before) = before {
                self.menu_restore.push((*item, before));
            }
            self.send(civ::set_menu_frame(self.civ_addr, *item, &[*value]));
        }
        let unanswered = self.menu_read.iter().filter(|(_, v)| v.is_none()).count();
        if unanswered > 0 {
            tracing::warn!(
                unanswered,
                "Icom LAN: the radio did not say what was in every menu item being written, \
                 so those will not be put back when the session ends"
            );
        }
        self.menu_read.clear();
    }

    /// Put the Set-mode menu items this session borrowed back the way the
    /// operator had them (issue #252).
    ///
    /// Called on both ways out — [`IqSource::release`] on a reopen, and `Drop`
    /// for a radio switched off, a tab closed or the application quitting —
    /// and drains what it sends, so running twice sends nothing twice.
    fn restore_menu(&mut self) {
        let items = std::mem::take(&mut self.menu_restore);
        // Nothing borrowed, or nothing left to hand it back over: a session
        // that has already died — the radio unplugged, the WiFi gone — kept
        // whatever it had at the moment the link went, and this end cannot
        // reach it to say otherwise. Waiting on it would only slow every
        // failed reconnect down.
        if items.is_empty() || !self.dev.is_alive() {
            return;
        }
        tracing::info!(items = items.len(), "Icom LAN: putting the radio's menu back as it was");
        for (item, value) in &items {
            self.send(civ::set_menu_frame(self.civ_addr, *item, value));
        }
        let deadline = Instant::now() + MENU_RESTORE_SETTLE;
        while Instant::now() < deadline {
            self.pump();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Wait, briefly, for the radio to say where its dial is, and fold it in.
    ///
    /// The engine reads [`IqSource::center_hz`] the instant it adopts a source
    /// and builds the whole display window — and, on this backend where the
    /// dial *is* the VFO, the tune it commands back at the rig — from it. A
    /// centre of 0 Hz (the opening [`Dial::default`]) is clamped up to the
    /// receiver's lowest frequency and sent to the radio, which is why a
    /// reconnect used to yank an IC-R8600 down to ~30 kHz. `configure` has
    /// already asked for the frequency; this just gives the answer a moment to
    /// arrive, re-asking through [`Self::pump`] if the first read was lost.
    ///
    /// Bounded and best-effort: a radio that never answers times out and the
    /// old behaviour stands, which is no worse than before. A real dial is
    /// never 0 Hz, so that is a safe "not yet known" sentinel.
    fn adopt_initial_dial(&mut self) {
        let deadline = Instant::now() + Duration::from_millis(1500);
        while self.dial.vfo == 0.0 && Instant::now() < deadline {
            self.pump();
            std::thread::sleep(Duration::from_millis(10));
        }
        // Whatever arrived came in as a pending `Freq` update; the engine has
        // not been built yet, so drop it — the dial itself now carries the
        // frequency, and `center_hz` reports it. Leaving it would make the
        // first `poll_control` announce a "retune" to where the radio already
        // is.
        self.pending.retain(|u| !matches!(u, ControlUpdate::Freq(_)));
        if self.dial.vfo == 0.0 {
            tracing::warn!(
                "Icom LAN: the radio did not report its frequency at connect; the display                  will adopt it on the first poll instead"
            );
        }
    }

    /// Whether the radio offered a transmit stream. An IC-R8600 does not, and
    /// neither does a transceiver somebody else is already transmitting from.
    pub fn can_transmit(&self) -> bool {
        self.dev.info().can_transmit && self.tx.is_some()
    }

    /// The label the settings dialog and the engine show for this session.
    pub fn describe(&self) -> String {
        self.label.clone()
    }

    fn send(&self, frame: Vec<u8>) {
        self.dev.send_civ(frame);
    }

    /// Ask the radio to run its scope and stream it here.
    ///
    /// Every frame is idempotent, which is what lets [`Self::watch_scope`] send
    /// the lot again without having to know what went missing.
    fn enable_scope(&mut self) {
        if !self.scope_wanted {
            return;
        }
        self.send(civ::scope_on_frame(self.civ_addr, true));
        self.send(civ::scope_output_frame(self.civ_addr, true));
        // The radio keeps whatever span was last chosen on its own screen, and
        // that is routinely ±5 kHz — a full-band strip no wider than the
        // panadapter above it. Centre mode as well as the span: a scope left in
        // one of the fixed modes ignores `27 15` and sits on a slice of band the
        // dial is not even in.
        if let Some(half) = self.scope_half_span {
            self.send(civ::scope_mode_frame(self.civ_addr, civ::ScopeMode::Center));
            self.send(civ::scope_span_frame(self.civ_addr, half));
        }
    }

    /// Start the scope again when it stops on its own.
    ///
    /// Nothing on this link reports that the sweeps have stopped, and several
    /// ordinary things stop them: `27 11` is fire-and-forget over UDP, so an
    /// enable lost on the way is never missed; the radio drops its output when
    /// its own scope screen closes; and a session that comes up slowly can have
    /// the whole opening burst land before the radio will accept any of it. The
    /// waterfall then sat dead until the operator reconnected — and a reconnect
    /// can lose the enable exactly the same way, which is why it sometimes took
    /// several. Asking again costs two frames, so the watchdog does what the
    /// operator was doing by hand.
    fn watch_scope(&mut self) {
        if !self.scope_wanted {
            return;
        }
        // A radio does not sweep while it transmits, and an over is not a fault:
        // hold the clock rather than nudging through every transmission.
        if self.transmitting {
            self.last_sweep = Instant::now();
            return;
        }
        if self.last_sweep.elapsed() < SCOPE_STALL
            || self.last_scope_nudge.elapsed() < self.scope_retry
        {
            return;
        }
        self.last_scope_nudge = Instant::now();
        if !self.scope_stalled {
            self.scope_stalled = true;
            tracing::warn!(
                "Icom LAN: the radio's scope has stopped sending; asking it to stream again"
            );
            // Also into the wire trace, where it sits beside the frames that
            // stopped — which is where anyone diagnosing a recurrence looks.
            self.dev.trace().note("scope stalled; re-sending the enables");
        }
        // A sweep left half-assembled when the stream stopped would otherwise be
        // joined to the first one that comes back.
        self.scope = civ::ScopeAssembler::default();
        self.enable_scope();
        // Back off while it stays quiet: a radio that has no scope, or whose
        // operator has switched it off for good, is not worth two frames every
        // two seconds for the rest of the session.
        self.scope_retry = (self.scope_retry * 2).min(SCOPE_RETRY_MAX);
    }

    /// Drain whatever the radio has said, and ask it the periodic questions.
    fn pump(&mut self) {
        while let Ok(frame) = self.dev.civ_frames().try_recv() {
            self.civ_buf.extend_from_slice(&frame);
        }
        for reply in civ::parse_frames(&mut self.civ_buf) {
            // Our own frames come back on a shared bus; the parser already
            // drops the ones we sent.
            self.on_reply(reply);
        }

        // The radio's own repeater shift, put back to simplex whenever the dial
        // has moved. A band stacking register restores whatever duplex that
        // band was last left on the moment the dial crosses into it, so
        // clearing it once with the other offsets in `configure` is not enough
        // — see [`civ::simplex_frame`] (issue #192) — and the auto-repeater
        // function arms on the *frequency*, so a move between two channels of
        // one band puts it back without crossing a band edge at all (issue
        // #233). Not while keyed: the transmit frequency went out with the
        // key-down, and mid-over the link belongs to the meters.
        if !self.transmitting && self.dial.vfo > 0.0 {
            self.assert_simplex(self.dial.vfo);
        }

        // Ask again for the antenna socket while the radio has not answered.
        // The absence of a reply is how a single-connector radio says it has no
        // selector, so a *lost* reply is indistinguishable from that and leaves
        // the panel with no antenna control at all until the next reconnect
        // (issue #258). A few more chances, then let it go.
        if self.socket.is_none()
            && self.antenna_probes_left > 0
            && self.last_antenna_probe.elapsed() >= ANTENNA_PROBE_RETRY
        {
            self.antenna_probes_left -= 1;
            self.last_antenna_probe = Instant::now();
            self.send(civ::read_antenna_frame(self.civ_addr));
        }

        // A tune the radio has not agreed to, sent again. A CI-V frame on this
        // link is a UDP datagram, and one lost on the way is a band change that
        // never happened and that nothing here would otherwise notice — see
        // [`Dial::retry`], and issue #297.
        if !self.transmitting
            && let Some(f) = self.dial.retry()
        {
            self.send(civ::set_freq_frame(self.civ_addr, f));
        }

        if self.last_poll.elapsed() >= POLL_PERIOD {
            self.last_poll = Instant::now();
            self.send(civ::read_freq_frame(self.civ_addr));
            self.send(civ::read_mode_frame(self.civ_addr));
            if self.transmitting {
                self.send(civ::read_swr_frame(self.civ_addr));
            } else {
                self.send(civ::read_smeter_frame(self.civ_addr));
            }
        }
        self.watch_scope();
    }

    /// Put the radio's own repeater shift back to simplex for `dial_hz`, unless
    /// that has already been done for this dial.
    ///
    /// The HF/VHF split is [`sdroxide_cat`]'s: below 28 MHz nothing but a band
    /// stacking register can restore a duplex setting, so once per band is
    /// enough and the link stays quiet while the operator tunes; above it an
    /// auto-repeater function arms on the frequency and every move has to be
    /// answered.
    fn assert_simplex(&mut self, dial_hz: f64) {
        let same = match self.simplex_dial {
            None => false,
            Some(prev) => {
                prev == dial_hz
                    || (prev < DUPLEX_FLOOR_HZ
                        && dial_hz < DUPLEX_FLOOR_HZ
                        && Band::containing(prev) == Band::containing(dial_hz))
            }
        };
        if same {
            return;
        }
        self.simplex_dial = Some(dial_hz);
        self.send(civ::simplex_frame(self.civ_addr));
    }

    fn on_reply(&mut self, reply: civ::CivReply) {
        match reply.cmd {
            // Frequency, both the polled read and an unsolicited transceive
            // report when the operator turns the knob.
            0x00 | 0x03 => {
                if let Some(hz) = civ::decode_freq(&reply.data) {
                    // The dial carries RIT, and for the length of an over it
                    // carries XIT or split instead, so what the radio reports
                    // has to be folded back before it means "the VFO moved".
                    if let Some(vfo) = self.dial.report(hz) {
                        self.pending.push(ControlUpdate::Freq(vfo));
                    }
                }
            }
            0x01 | 0x04 => {
                let Some(&b) = reply.data.first() else { return };
                // A report that disagrees with a mode we have just commanded is
                // the stale one — the read went out before the set did.
                if let Some((want, at)) = self.mode_cmd {
                    if b == want {
                        self.mode_cmd = None;
                    } else if at.elapsed() < MODE_SETTLE {
                        return;
                    } else {
                        // Long enough: the radio is somewhere else and means
                        // it. Its mode is the real one, so stop arguing.
                        self.mode_cmd = None;
                    }
                }
                if let Some(m) = civ::civ_to_mode(b) {
                    self.pending.push(ControlUpdate::Mode(m));
                }
            }
            // The antenna: which socket the radio is on, and behind it the
            // receiving antenna's own in/out setting. Both adopted, and the
            // reply's arrival — and its *shape* — is what says which of the two
            // this radio has; see `Self::learned_antennas` and
            // [`civ::antenna_frame`].
            0x12 => {
                if let Some(r) = civ::parse_antenna_reply(&reply.data) {
                    self.socket = Some(r.socket);
                    if let Some(on) = r.rx_ant
                        && self.rx_ant != Some(on)
                    {
                        self.rx_ant = Some(on);
                        self.pending.push(ControlUpdate::RxAntenna(on));
                    }
                    // Not reported on a radio with one socket: there is no
                    // selector there, and a panel told "ANT1" would draw a chip
                    // whose other position that radio rejects.
                    if self.antenna_sockets() > 1
                        && let Some(name) = civ::antenna_name(r.socket, self.antenna_sockets())
                    {
                        self.pending.push(ControlUpdate::Antenna(name));
                    }
                }
            }
            // The transmit power the radio is set to, asked for once when the
            // session opens. Adopted into the Drive slider rather than
            // commanded back: the radio's own setting is the operator's.
            0x14 => {
                if let Some(frac) = civ::parse_power_reply(&reply.data) {
                    self.pending.push(ControlUpdate::TxDrive(frac));
                } else if let Some(frac) = civ::parse_squelch_reply(&reply.data) {
                    // Only the opening read ever answers here: once this end
                    // has set a level, the suppression in `set_squelch` keeps
                    // the radio's own answer from dragging the rail back.
                    if !self.squelch_set {
                        self.squelch_set = true;
                        self.pending.push(ControlUpdate::Squelch(frac));
                    }
                }
            }
            // The "various settings" block, of which this session asks for one
            // thing only: whether break-in is on (issue #282). Not surfaced to
            // the app — it is not a control here, it is what `send_cw` needs to
            // know before it asserts semi break-in over an operator's QSK.
            0x16 => {
                if let Some(bk) = civ::parse_break_in_reply(&reply.data) {
                    self.break_in = Some(bk);
                }
            }
            0x15 => {
                if let Some(dbm) = civ::parse_smeter_reply(&reply.data) {
                    self.last_signal = Some((Instant::now(), dbm));
                }
                if let Some(swr) = civ::parse_swr_reply(&reply.data) {
                    self.last_telem = Some(TxTelemetry { swr: Some(swr), ..Default::default() });
                }
            }
            // A Set-mode menu item, answering one of the reads `configure`
            // sends before it writes over them (issue #252). A read carries no
            // value, so our own question coming back off a shared bus is not
            // mistaken for the radio's answer to it — see
            // [`civ::parse_menu_reply`]. Only an item still being waited on is
            // filled in, and only until `claim_menu_items` has done with them:
            // what a menu says later in a session is the operator's business,
            // not this session's to hand back.
            0x1A => {
                if let Some((item, value)) = civ::parse_menu_reply(&reply.data)
                    && let Some(slot) =
                        self.menu_read.iter_mut().find(|(i, v)| *i == item && v.is_none())
                {
                    slot.1 = Some(value.to_vec());
                }
            }
            0x27 => {
                if let Some((info, bins)) =
                    civ::parse_scope_frame(&reply.data).and_then(|s| self.scope.push(s))
                {
                    self.scope_frame = Some((info.center_hz, info.span_hz, bins));
                    // A whole sweep is the only thing that proves the lane is
                    // alive — frames still arriving while none of them ever
                    // completes leaves the waterfall just as frozen.
                    self.last_sweep = Instant::now();
                    self.scope_retry = SCOPE_RETRY;
                    if self.scope_stalled {
                        self.scope_stalled = false;
                        tracing::info!("Icom LAN: the radio's scope is sweeping again");
                        self.dev.trace().note("scope sweeping again");
                    }
                }
            }
            _ => {}
        }
    }

    /// Move audio out of the network ring, converting on the way when the
    /// stream is carrying a 12 kHz IF rather than demodulated audio.
    fn fill(&mut self, want: usize) {
        match self.ddc.as_mut() {
            None => {}
            Some(ddc) => {
                // Two input samples per output sample, plus a little slack so a
                // partly-filled filter still produces something.
                let need = want.saturating_sub(self.if_out.len()) * 2 + 64;
                self.if_in.clear();
                for _ in 0..need {
                    match self.audio.pop() {
                        // A real IF: the imaginary half is zero, and the mixer
                        // plus the decimating low-pass is what removes the
                        // image the real signal carries.
                        Ok(s) => self.if_in.push(Complex32::new(s, 0.0)),
                        Err(_) => break,
                    }
                }
                if !self.if_in.is_empty() {
                    let mut out = Vec::with_capacity(self.if_in.len() / 2 + 8);
                    let input = std::mem::take(&mut self.if_in);
                    ddc.process(&input, &mut out);
                    self.if_in = input;
                    self.if_out.extend(out);
                }
            }
        }
    }

    /// Take whatever the radio has already sent, and no more.
    fn drain(&mut self, buf: &mut [Complex32]) -> usize {
        self.pump();
        match self.rx_source {
            IcomRxSource::Af => {
                let mut n = 0;
                while n < buf.len() {
                    let Ok(s) = self.audio.pop() else { break };
                    buf[n] = Complex32::new(s, 0.0);
                    n += 1;
                }
                n
            }
            IcomRxSource::If12k => {
                self.fill(buf.len());
                let mut n = 0;
                while n < buf.len() {
                    let Some(s) = self.if_out.pop_front() else { break };
                    buf[n] = s;
                    n += 1;
                }
                n
            }
        }
    }
}

impl IqSource for IcomNetSource {
    fn sample_rate(&self) -> f64 {
        self.rate
    }

    fn center_hz(&self) -> f64 {
        self.dial.vfo
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        if let Some(f) = self.dial.set_vfo(hz) {
            self.send(civ::set_freq_frame(self.civ_addr, f));
        }
        Ok(())
    }

    fn set_rit_hz(&mut self, hz: f64) {
        if let Some(f) = self.dial.set_rit(hz) {
            self.send(civ::set_freq_frame(self.civ_addr, f));
        }
    }

    /// Both receive paths hang off the radio's own dial: the 12 kHz IF is
    /// mixed down from it, and AF is what it demodulated there. Either way
    /// there is no second oscillator to leave parked while the dial moves.
    fn center_is_dial(&self) -> bool {
        true
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let n = self.drain(buf);
        if n == 0 {
            // Nothing yet: the radio paces this stream, so yielding here is
            // what keeps the engine thread off a spin.
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(n)
    }

    /// The same drain without the nap — see `AudioCatSource::read_available`
    /// for why a source that may not be the one pacing the engine needs it.
    fn read_available(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        Ok(self.drain(buf))
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    fn open_status(&self) -> Option<String> {
        self.status.clone()
    }

    fn display_bandwidth(&self) -> Option<f64> {
        (self.rx_source == IcomRxSource::Af).then_some(self.audio_bw)
    }

    /// The radio's own scope, as the full-band panadapter.
    ///
    /// These are finished magnitude bins on the radio's own scale — 0..=160 on
    /// the IC-7300 generation, 0..=200 on an IC-7760 — not anything derived
    /// from I/Q, of which there is none on this interface. Mapping them onto a
    /// dB axis is a linear guess with an uncalibrated slope; the engine's
    /// auto-levelling makes the picture right even where the absolute numbers
    /// are not, which is also why reading the top of the scale off the wrong
    /// model is a 20 dB offset rather than a broken trace.
    fn wide_spectrum_db(&mut self, out: &mut Vec<f32>) -> Option<(f64, f64)> {
        let (center, span, bins) = self.scope_frame.take()?;
        let full = f32::from(self.dev.info().model.scope_full_scale);
        out.clear();
        out.extend(bins.iter().map(|&b| (f32::from(b) - full) * SCOPE_DB_PER_UNIT));
        Some((center, span))
    }

    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        self.pump();
        std::mem::take(&mut self.pending)
    }

    /// The radio in front of us is the rig, so the mode control has to reach it
    /// on both receive paths — including the 12 kHz IF, where sdroxide does the
    /// demodulating but the radio's mode still picks the IF filter the stream
    /// arrives through, and its own display would otherwise disagree with ours.
    ///
    /// Not [`IqSource::tracks_rx_mode`], which would also assert the session's
    /// mode the moment the connection opens: this backend adopts the dial and
    /// the mode the transceiver is already on, the way the CAT backend does.
    fn commands_rx_mode(&self) -> bool {
        true
    }

    /// From the dial this backend already tracks — see [`Self::set_control_mode`].
    fn resolves_band_sideband(&self) -> bool {
        true
    }

    fn set_control_mode(&mut self, mode: Mode) -> Result<()> {
        // CW keyed as audio (MCW) rides a sideband: a rig put in CW keys its
        // own transmitter and ignores the modulator input, so the keyed
        // sidetone only reaches the air from USB — the same plain USB this
        // backend already commands for every digital mode (`mode_to_civ`
        // folds them all to 0x01). Mapped before the echo-suppression store
        // so the rig's USB report matches what was commanded.
        let mode = match mode {
            Mode::Cw if matches!(self.cw_keying, CwKeying::Audio) => Mode::Usb,
            // Analog SSTV's sideband follows the band, and RADE's does too, and
            // the engine leaves that to us (see `IqSource::resolves_band_sideband`)
            // because `mode_to_civ` is a table with no dial in it. DATA on both
            // sidebands: the picture — or the over — is modulated here and
            // arrives at the radio's data input, which is what these modes
            // already ask this family for on the bands where they ride USB.
            m if m.sideband_follows_band() && m.is_lower_sideband_at(self.dial.vfo) => Mode::Digl,
            m => m,
        };
        self.mode_cmd = Some((civ::mode_to_civ(mode), Instant::now()));
        // The mode *and* the DATA switch beside it, which is the pair the
        // serial CAT path has always sent. The plain mode command clears the
        // switch, so sending it alone took a rig the operator had put in FM-D
        // straight back to FM — and it is sent before every over, so a beacon
        // could never go out through the data path at all (issue #150).
        for f in civ::set_mode_frames(self.civ_addr, mode, self.dev.info().model.data_mode_sub) {
            self.send(f);
        }
        Ok(())
    }

    // ── CW from the rig's own keyer ─────────────────────────────────────────
    // A rig *in* CW does not modulate what we send it — it keys its own
    // transmitter — so the keyer hands it text rather than a sidetone.
    fn cw_text_keying(&self) -> Option<usize> {
        matches!(self.cw_keying, CwKeying::Cat).then_some(civ::CW_MAX)
    }
    // …and with the keyer on the sound path instead, the engine must expect
    // the mode poll to answer USB while the panel shows CW.
    fn cw_audio_keyed(&self) -> bool {
        matches!(self.cw_keying, CwKeying::Audio)
    }
    fn send_cw(&mut self, text: &str) {
        let Some(f) = civ::send_cw_frame(self.civ_addr, text) else { return };
        // Break-in first, or the radio takes the message and never transmits
        // it: its own reference sends a message from a PC only while
        // `[TRANSMIT]` is on, an external TX switch is closed, or break-in is
        // on — and over a network link there is no front panel to reach for.
        // That was issue #282 on an IC-9700. Semi is what a computer-sent
        // message wants, but never *down* from a full break-in the operator
        // chose: the read in `configure` is what tells this end which.
        if self.break_in != Some(civ::BreakIn::Full) {
            self.break_in = Some(civ::BreakIn::Semi);
            self.send(civ::set_break_in_frame(self.civ_addr, civ::BreakIn::Semi));
        }
        self.send(f);
    }
    fn abort_cw(&mut self) {
        self.send(civ::stop_cw_frame(self.civ_addr));
    }
    fn set_cw_wpm(&mut self, wpm: f32) {
        self.send(civ::keyer_speed_frame(self.civ_addr, wpm));
    }

    // ── Output power ────────────────────────────────────────────────────────
    // The radio's own power control, which is the only transmit level that
    // applies in every mode: the audio on this link is just the modulating
    // signal, and in CW there is none at all — the radio keys itself from its
    // own keyer. The engine asserts the level that applies (the drive, or the
    // tune level while tuning) before each over.
    fn set_tx_drive(&mut self, frac: f64) {
        self.send(civ::set_power_frame(self.civ_addr, frac as f32));
    }
    /// One power register on the radio; the engine commands the tune level
    /// through [`Self::set_tx_drive`] for as long as TUNE holds it.
    fn set_tune_drive(&mut self, _frac: f64) {}
    fn commands_tx_power(&self) -> bool {
        true
    }

    /// The radio's own squelch, which on AF over the network is the only one
    /// there is: what the LAN stream carries has already been through it.
    fn set_squelch(&mut self, frac: f32) {
        self.squelch_set = true;
        self.send(civ::set_squelch_frame(self.civ_addr, frac));
    }
    /// AF only. On the 12 kHz IF the stream is real spectrum that sdroxide
    /// demodulates itself, so the radio's gate decides nothing about the audio
    /// heard here and the engine's own threshold is the one that does.
    fn commands_squelch(&self) -> bool {
        self.rx_source == IcomRxSource::Af
    }

    /// Put the radio on one of its antenna sockets. Icom's selector is one
    /// relay for both directions, so there is no transmit port to pick
    /// separately (issue #238).
    fn set_antenna(&mut self, name: &str) -> Result<()> {
        // A name from whatever front end was on this radio before — or a socket
        // this model has not got — is dropped rather than turned into a socket
        // number by accident.
        let Some(n) = self.antennas().iter().position(|a| a.eq_ignore_ascii_case(name)) else {
            return Ok(());
        };
        // The receiving antenna's setting rides out with the socket, because
        // the same command writes both: a zero here would switch the operator's
        // receive aerial out of circuit (issue #229).
        self.send(civ::antenna_frame(self.civ_addr, n as u8, self.rx_ant.unwrap_or(false)));
        self.socket = Some(n as u8);
        // And on an IC-7610 the flag belongs to the socket being selected
        // rather than to the one being left, so ask what it is now.
        self.send(civ::read_antenna_frame(self.civ_addr));
        Ok(())
    }

    fn current_antenna(&self) -> String {
        self.socket
            .and_then(|n| civ::antenna_name(n, self.antenna_sockets()))
            .unwrap_or_default()
            .to_string()
    }

    fn rx_antenna(&self) -> Option<bool> {
        self.rx_ant
    }

    /// The same command with the socket held still. Nothing goes out until the
    /// radio has said which socket that is, and that it has a receiving antenna
    /// at all — see `civ::antenna_frame`.
    fn set_rx_antenna(&mut self, on: bool) -> Result<()> {
        let (Some(socket), true) = (self.socket, self.rx_ant.is_some()) else {
            return Ok(());
        };
        self.rx_ant = Some(on);
        self.send(civ::antenna_frame(self.civ_addr, socket, on));
        Ok(())
    }

    /// Ask the radio again, after something it recalls per band. Never an
    /// assertion: what comes back is adopted.
    fn reread_rx_antenna(&mut self) {
        self.send(civ::read_antenna_frame(self.civ_addr));
    }

    /// ANT1/ANT2, once the radio has answered the opening read — and nothing
    /// at all until it has. Every Icom speaks one dialect and only some have a
    /// selector, so the list is a question for the radio rather than a claim
    /// about the protocol; see [`civ::ANTENNAS`].
    /// The radio's own power switch, over the network (issue #239).
    ///
    /// The point of doing it here rather than over a serial cable: the radio's
    /// network module stays awake while the set is off — that is what
    /// **Network > Network Control** leaves running — so the control session
    /// this rides on is still there to carry the switch back on again.
    fn set_rig_power(&mut self, on: bool) -> Result<()> {
        for f in civ::power_frames(self.civ_addr, on, civ::WAKE_BYTES_LAN) {
            self.send(f);
        }
        Ok(())
    }

    fn commands_rig_power(&self) -> bool {
        true
    }

    fn learned_antennas(&self) -> Option<&'static [&'static str]> {
        Some(if self.socket.is_some() { self.antennas() } else { &[] })
    }

    fn tx_begin(&mut self, center_hz: f64, _rate: f64) -> Result<f64> {
        // Split and XIT have no DDC to ride on: the radio's dial is its whole
        // frequency control, so an over that transmits away from where we
        // listen borrows the dial until unkey.
        if let Some(f) = self.dial.begin_tx(center_hz) {
            self.send(civ::set_freq_frame(self.civ_addr, f));
            // …and the radio may have armed a shift of its own on that
            // frequency the moment it took it (issue #233). Only on a dial the
            // over actually moved: where transmit lands where we listen there
            // is nothing new for an auto-repeater to have acted on.
            self.assert_simplex(f);
        }
        self.send(civ::ptt_frame(self.civ_addr, true));
        self.transmitting = true;
        // Transmit audio is always plain AF at the negotiated rate, whatever
        // the receive stream happens to be carrying.
        Ok(f64::from(self.dev.info().tx_sample_rate))
    }

    fn tx_write_audio(&mut self, audio: &[f32]) -> Result<()> {
        let Some(tx) = self.tx.as_mut() else {
            return Ok(()); // receive-only radio; PTT still keyed it
        };
        // Block on a full ring so the engine's transmit loop is paced by the
        // radio rather than by the CPU: without this a long burst is generated
        // far faster than 48 kHz and most of it is dropped.
        for &s in audio {
            let mut v = s;
            let mut tries = 0u32;
            while let Err(rtrb::PushError::Full(x)) = tx.push(v) {
                v = x;
                tries += 1;
                if tries > 200 {
                    break; // the link stalled — drop rather than hang transmit
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        Ok(())
    }

    fn tx_drain(&mut self) {
        // Wait for the ring to empty before unkey, so the tail of a burst —
        // which is exactly what an FT8 decoder needs — is not cut off.
        let Some(tx) = self.tx.as_ref() else { return };
        let cap = tx.buffer().capacity();
        for _ in 0..1000 {
            if cap.saturating_sub(tx.slots()) <= cap / 40 {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn tx_end(&mut self) -> Result<()> {
        self.send(civ::ptt_frame(self.civ_addr, false));
        self.transmitting = false;
        if let Some(f) = self.dial.end_tx() {
            self.send(civ::set_freq_frame(self.civ_addr, f));
        }
        self.last_telem = None;
        Ok(())
    }

    fn discard_pending_rx(&mut self) {
        while self.audio.pop().is_ok() {}
        self.if_out.clear();
    }

    fn tx_telemetry(&mut self) -> Option<TxTelemetry> {
        self.pump();
        self.last_telem
    }

    fn rx_signal_dbm(&mut self) -> Option<f32> {
        self.pump();
        self.last_signal.filter(|(at, _)| at.elapsed() < METER_MAX_AGE).map(|(_, dbm)| dbm)
    }

    fn needs_reopen(&self) -> bool {
        if !self.dev.is_alive() {
            // The session that just died is the one worth reporting on.
            record_trace(&self.key, &self.dev);
            return true;
        }
        false
    }

    fn release(&mut self) {
        // Before the session goes: the radio keeps its Set-mode menu whatever
        // happens to this end of the link, so anything borrowed from it has to
        // be handed back while there is still a link to hand it back over.
        self.restore_menu();
        record_trace(&self.key, &self.dev);
        self.dev.shutdown();
    }
}

impl Drop for IcomNetSource {
    fn drop(&mut self) {
        // `release` is only called on the reopen paths. A radio switched off, a
        // tab closed, or the application quitting simply drops the source — and
        // those are exactly the ways a session ends before the operator goes on
        // to use the rig on its own, which is the case issue #252 is about.
        // `restore_menu` drains itself, so the two paths cannot double up.
        self.restore_menu();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_icomnet::sim::{Sim, SimOptions};

    use crate::dial::FREQ_SETTLE;

    fn cfg(sim: &Sim) -> IcomNetConfig {
        IcomNetConfig {
            address: "127.0.0.1".into(),
            control_port: sim.port(),
            username: "operator".into(),
            password: "hunter2".into(),
            ..Default::default()
        }
    }

    fn wait_for(what: &str, mut f: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if f() {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("timed out waiting for {what}");
    }

    #[test]
    fn the_dial_is_adopted_at_open_so_the_engine_never_retunes_the_radio() {
        // The engine reads `center_hz()` the instant it adopts the source and
        // commands the rig there. If that read is 0 (the dial before the first
        // frequency reply), the rig is dragged to the bottom of its range — the
        // "jumps to ~30 kHz on reconnect" bug. `open` must not return until the
        // radio's real dial is in hand.
        let sim =
            Sim::start(SimOptions { freq_hz: 7_074_000.0, scope: false, ..Default::default() })
                .unwrap();
        let src = IcomNetSource::open(&cfg(&sim)).expect("open");
        assert_eq!(src.center_hz(), 7_074_000.0, "the radio's own dial, not 0");
    }

    /// Issue #297: a band change on a networked Icom took three to five clicks
    /// to stick.
    ///
    /// A CI-V frame on this link is a UDP datagram with nothing above it that
    /// says the radio acted on it, so a set-frequency lost on the way is a
    /// tune that simply never happened — and one poll period later the radio
    /// answers from where it always was, the settling guard runs out, and the
    /// dial snaps back. The operator's own remedy was to click again; this is
    /// sdroxide doing it for them. The simulator swallows the first tune, which
    /// is exactly what that loss looks like from this end.
    #[test]
    fn a_tune_the_radio_never_took_is_sent_again_until_it_sticks() {
        let sim = Sim::start(SimOptions {
            freq_hz: 14_074_000.0,
            ignore_tunes: 1,
            scope: false,
            ..Default::default()
        })
        .unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");
        assert_eq!(src.center_hz(), 14_074_000.0);

        // The operator picks 40 m for FT8. That frame is the one the radio
        // never sees.
        src.set_center_hz(7_074_000.0).expect("tune");
        wait_for("the radio to be tuned to the band that was asked for", || {
            let _ = src.poll_control();
            sim.dial() == 7_074_000.0
        });
        // And it has to stay there: the radio's answers from before it moved
        // must not be folded back in on top of the tune.
        let deadline = Instant::now() + FREQ_SETTLE * 2;
        while Instant::now() < deadline {
            let _ = src.poll_control();
            assert_eq!(src.center_hz(), 7_074_000.0, "the band the operator chose did not stick");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// And the resend is a recovery, not a habit: a radio that takes the tune
    /// is not asked over and over for the rest of the settling time.
    ///
    /// At most twice, rather than exactly once, because the radio's agreement
    /// arrives on the next frequency poll — and a poll that had only just gone
    /// out when the operator clicked leaves a full period before the next one.
    /// One redundant set-frequency costs a frame on a link that already sends
    /// four every poll; a stream of them would be this end arguing with a radio
    /// that never disagreed.
    #[test]
    fn a_tune_the_radio_takes_is_not_asked_for_over_and_over() {
        let sim =
            Sim::start(SimOptions { freq_hz: 14_074_000.0, scope: false, ..Default::default() })
                .unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");
        src.set_center_hz(7_074_000.0).expect("tune");
        // Past the settling time, after which nothing is re-asked at all.
        let deadline = Instant::now() + FREQ_SETTLE * 2;
        while Instant::now() < deadline {
            let _ = src.poll_control();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(sim.dial(), 7_074_000.0);
        assert_eq!(src.center_hz(), 7_074_000.0);
        let tunes = sim.civ_frames().iter().filter(|f| f.get(4) == Some(&0x05)).count();
        assert!(tunes <= 2, "a radio that answered was asked {tunes} times");
    }

    /// Two Icoms on one LAN, each keeping its own session trace.
    ///
    /// The trace is what a field report is built from, and a station with an
    /// IC-7300 and an IC-9700 on it is the ordinary case. One slot for the
    /// process handed whichever of them last hung up to whoever pressed the
    /// button — so the 9700's tab answered with the 7300's conversation, which
    /// is worse than answering with nothing (issue #169).
    #[test]
    fn each_radio_keeps_its_own_session_trace() {
        let a = Sim::start(SimOptions { civ_address: 0xB6, scope: false, ..Default::default() })
            .unwrap();
        let b = Sim::start(SimOptions { civ_address: 0xA2, scope: false, ..Default::default() })
            .unwrap();
        let (ca, cb) = (cfg(&a), cfg(&b));
        let mut sa = IcomNetSource::open(&ca).expect("open the first radio");
        let mut sb = IcomNetSource::open(&cb).expect("open the second radio");
        // Both sessions end, in the order that used to decide the answer.
        sa.release();
        sb.release();

        // Each radio's CI-V address is in its own frames and nowhere else, so
        // it says which conversation came back.
        let (ra, rb) = (diagnostics_or_hint(&ca), diagnostics_or_hint(&cb));
        assert!(ra.contains("fe fe b6"), "the first radio's report is not its own session");
        assert!(!ra.contains("fe fe a2"), "the first radio's report carries the second's session");
        assert!(rb.contains("fe fe a2"), "the second radio's report is not its own session");
        assert!(!rb.contains("fe fe b6"), "the second radio's report carries the first's session");

        // And a radio nothing has connected to says so, rather than handing
        // over the nearest session it can find.
        let never = IcomNetConfig { address: "192.0.2.7".into(), ..Default::default() };
        assert!(diagnostics_or_hint(&never).contains("No Icom LAN session has run yet"));
    }

    #[test]
    fn af_mode_delivers_packed_real_audio_at_the_negotiated_rate() {
        let sim =
            Sim::start(SimOptions { tone_hz: Some(1_000.0), scope: false, ..Default::default() })
                .unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");
        assert_eq!(src.sample_rate(), 48_000.0);
        assert_eq!(src.display_bandwidth(), Some(4000.0));

        let mut buf = vec![Complex32::default(); 4096];
        let mut got = Vec::new();
        wait_for("audio", || {
            let n = src.read(&mut buf).unwrap();
            got.extend_from_slice(&buf[..n]);
            got.len() > 4_800
        });
        // Demodulated audio is real: the engine's audio-mode path reads the
        // real part and would see a phantom sideband if anything landed in the
        // imaginary one.
        assert!(got.iter().all(|c| c.im == 0.0));
        assert!(got.iter().any(|c| c.re.abs() > 0.4));
    }

    #[test]
    fn the_12_khz_if_lands_a_tone_at_the_offset_it_was_sent_at() {
        // The radio emits a real IF with a tone 3 kHz above the 12 kHz centre.
        // After mixing down, that tone must sit at +3 kHz — this is the one
        // piece of the IF path that can be checked without hardware.
        let sim = Sim::start(SimOptions {
            tone_hz: Some(IF_CENTER_HZ + 3_000.0),
            scope: false,
            ..Default::default()
        })
        .unwrap();
        let mut c = cfg(&sim);
        c.rx_source = IcomRxSource::If12k;
        let mut src = IcomNetSource::open(&c).expect("open");
        assert_eq!(src.sample_rate(), IF_OUT_RATE);
        assert_eq!(src.display_bandwidth(), None, "the IF path is not audio mode");

        let mut buf = vec![Complex32::default(); 4096];
        let mut got = Vec::new();
        wait_for("IF samples", || {
            let n = src.read(&mut buf).unwrap();
            got.extend_from_slice(&buf[..n]);
            got.len() > 12_000
        });
        // Drop the filter's start-up transient.
        let got = &got[2_000..];
        let at = |hz: f64| goertzel(got, hz, IF_OUT_RATE);
        let wanted = at(3_000.0);
        assert!(
            wanted > 20.0 * at(-3_000.0),
            "the image must be gone: {wanted} vs {}",
            at(-3_000.0)
        );
        assert!(wanted > 20.0 * at(6_000.0), "energy is at +3 kHz, not elsewhere");
        assert!(wanted > 20.0 * at(0.0), "and not left at DC");
    }

    /// A mirrored IF is the IC-7760's reported behaviour, and it is what makes
    /// SSB come out on the opposite sideband: the same tone the test above puts
    /// at +3 kHz has to land at -3 kHz instead. Checked three ways, because the
    /// model default and the operator's override are the two halves of the fix
    /// and either one alone would leave somebody stuck.
    #[test]
    fn a_mirrored_if_puts_the_tone_below_the_dial_instead_of_above_it() {
        // A tone 3 kHz above the IF centre, read back as the offset it lands at.
        let offset_of = |radio_name: &str, civ_address: u8, invert_if: Option<bool>| {
            let sim = Sim::start(SimOptions {
                tone_hz: Some(IF_CENTER_HZ + 3_000.0),
                civ_address,
                radio_name: radio_name.into(),
                scope: false,
                ..Default::default()
            })
            .unwrap();
            let mut c = cfg(&sim);
            c.rx_source = IcomRxSource::If12k;
            c.invert_if = invert_if;
            let mut src = IcomNetSource::open(&c).expect("open");

            let mut buf = vec![Complex32::default(); 4096];
            let mut got = Vec::new();
            wait_for("IF samples", || {
                let n = src.read(&mut buf).unwrap();
                got.extend_from_slice(&buf[..n]);
                got.len() > 12_000
            });
            let got = &got[2_000..];
            let (up, down) =
                (goertzel(got, 3_000.0, IF_OUT_RATE), goertzel(got, -3_000.0, IF_OUT_RATE));
            assert!(
                up > 20.0 * down || down > 20.0 * up,
                "one side or the other, not both: {up} vs {down}"
            );
            if up > down { 3_000.0 } else { -3_000.0 }
        };

        // The IC-7760's own default, with nothing set by hand.
        assert_eq!(offset_of("IC-7760", 0xB2, None), -3_000.0, "an IC-7760 mirrors its IF");
        // Every other model keeps the convention Icom built the output for.
        assert_eq!(offset_of("IC-705", 0xA4, None), 3_000.0, "and an IC-705 does not");
        // Either way the operator has the last word — the model table is a
        // report, not a measurement, so it has to be possible to overrule it in
        // both directions.
        assert_eq!(offset_of("IC-7760", 0xB2, Some(false)), 3_000.0, "override off");
        assert_eq!(offset_of("IC-705", 0xA4, Some(true)), -3_000.0, "override on");
    }

    /// Complex energy at one frequency, positive or negative — the sign is the
    /// whole point of the test, so a real-valued Goertzel would not do.
    fn goertzel(x: &[Complex32], hz: f64, rate: f64) -> f64 {
        let w = std::f64::consts::TAU * hz / rate;
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (n, s) in x.iter().enumerate() {
            let (sn, cs) = (-w * n as f64).sin_cos();
            re += f64::from(s.re) * cs - f64::from(s.im) * sn;
            im += f64::from(s.re) * sn + f64::from(s.im) * cs;
        }
        (re * re + im * im).sqrt() / x.len() as f64
    }

    #[test]
    fn the_radios_scope_reaches_the_full_band_lane() {
        let sim = Sim::start(SimOptions { freq_hz: 14_100_000.0, ..Default::default() }).unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");

        let mut bins = Vec::new();
        let mut span = None;
        let mut buf = vec![Complex32::default(); 1024];
        wait_for("a scope sweep", || {
            let _ = src.read(&mut buf);
            span = src.wide_spectrum_db(&mut bins);
            span.is_some()
        });
        let (center, width) = span.unwrap();
        assert_eq!(center, 14_100_000.0);
        // The simulator reports a ±50 kHz half-span; the width is the full one.
        assert_eq!(width, 100_000.0);
        assert_eq!(bins.len(), 475);
        // Mapped to a dB axis with the peak near the top of the scale.
        assert!(bins.iter().all(|&d| d <= 0.0));
        assert!(bins.iter().any(|&d| d > -10.0), "the planted peak survived the mapping");
    }

    /// What the operator was doing by hand: the sweeps stop — an enable lost on
    /// the way, the radio's scope screen closed — and the waterfall is dead
    /// until the session is rebuilt. The watchdog asks again instead, so the
    /// strip comes back on its own.
    #[test]
    fn a_scope_that_stops_sending_is_started_again() {
        let sim = Sim::start(SimOptions { freq_hz: 14_100_000.0, ..Default::default() }).unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");

        let mut bins = Vec::new();
        let mut buf = vec![Complex32::default(); 1024];
        let mut sweeps = |src: &mut IcomNetSource, bins: &mut Vec<f32>| {
            let _ = src.read(&mut buf);
            src.wide_spectrum_db(bins).is_some()
        };
        wait_for("the first scope sweep", || sweeps(&mut src, &mut bins));

        // The radio stops streaming and says nothing about it.
        sim.stall_scope();
        assert!(!sim.scope_streaming());

        // ...and the lane comes back without anybody reconnecting. The stall has
        // to be noticed first, so this waits longer than `SCOPE_STALL`.
        let deadline = Instant::now() + SCOPE_STALL + Duration::from_secs(4);
        let mut back = false;
        while Instant::now() < deadline && !back {
            back = sweeps(&mut src, &mut bins);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(back, "the scope never restarted");
        assert!(sim.scope_streaming(), "the radio was never asked to stream again");
        assert_eq!(bins.len(), 475);
    }

    /// The watchdog must not fire while the radio is transmitting: a scope that
    /// stops for the length of an over is a radio behaving normally, and asking
    /// it to stream mid-over would be two frames per retry for nothing.
    #[test]
    fn an_over_is_not_a_stalled_scope() {
        let sim = Sim::start(SimOptions { scope: false, ..Default::default() }).unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");
        src.scope_wanted = true;
        src.transmitting = true;
        src.last_sweep = Instant::now() - SCOPE_STALL * 4;
        src.last_scope_nudge = Instant::now() - SCOPE_RETRY_MAX;

        src.watch_scope();
        assert!(!src.scope_stalled, "an over was read as a fault");
        // And the clock was held, so the first quiet moment after the over is
        // not instantly a stall either.
        assert!(src.last_sweep.elapsed() < SCOPE_STALL);
    }

    #[test]
    fn switching_mode_sends_the_radio_a_set_mode() {
        // Field report, 2026-08-19: an IC-705 on the 12 kHz IF followed mode
        // changes made at the radio but not ones made in sdroxide. The stream
        // is not `audio_mode`, so the engine's "command the rig's mode" gate
        // never fired — see `IqSource::commands_rx_mode`.
        let sim = Sim::start(SimOptions { scope: false, ..Default::default() }).unwrap();
        let mut c = cfg(&sim);
        c.rx_source = IcomRxSource::If12k;
        let mut src = IcomNetSource::open(&c).expect("open");
        assert!(src.commands_rx_mode(), "the radio in front of us owns its mode");

        src.set_control_mode(Mode::Lsb).unwrap();
        wait_for("a set-mode frame selecting LSB", || {
            sim.civ_frames().iter().any(|f| f.get(4) == Some(&0x06) && f.get(5) == Some(&0x00))
        });
    }

    /// Issue #150: an IC-705 on APRS transmitted, sounded like packet, and was
    /// decoded by nobody — and a rig the operator had put into FM-D by hand
    /// dropped back to plain FM the moment a beacon went out.
    ///
    /// The plain set-mode command *clears* the DATA switch, and this backend
    /// sent only that while the serial CAT path had always sent the pair. So
    /// every over went out through the microphone path, with the rig's speech
    /// processing and — on FM — its pre-emphasis, which tilts a Bell 202 tone
    /// pair about 6 dB. Structurally perfect, audibly normal, unreadable.
    #[test]
    fn a_digital_mode_is_commanded_with_the_data_switch_beside_it() {
        let sim = Sim::start(SimOptions { scope: false, ..Default::default() }).unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");

        // APRS: FM (0x05) *and* the DATA switch on.
        src.set_control_mode(Mode::Aprs).unwrap();
        wait_for("FM for APRS", || {
            sim.civ_frames().iter().any(|f| f.get(4) == Some(&0x06) && f.get(5) == Some(&0x05))
        });
        wait_for("the DATA switch turned on for APRS", || {
            sim.civ_frames()
                .iter()
                .any(|f| f.get(4) == Some(&0x1A) && f.get(5) == Some(&0x06) && f.get(6) == Some(&1))
        });

        // ...and a voice mode turns it off again, or a rig left in a data mode
        // by the last over would stay there.
        let sim = Sim::start(SimOptions { scope: false, ..Default::default() }).unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");
        src.set_control_mode(Mode::Nfm).unwrap();
        wait_for("the DATA switch turned off for plain FM", || {
            sim.civ_frames()
                .iter()
                .any(|f| f.get(4) == Some(&0x1A) && f.get(5) == Some(&0x06) && f.get(6) == Some(&0))
        });
    }

    /// Issue #119: a rig put in CW keys its own transmitter and ignores its
    /// modulator input, so with the keyer on the sound path (MCW) selecting CW
    /// must keep the radio in plain USB — the same mode every digital mode
    /// rides here. With the rig's own keyer, CW is still commanded as CW.
    #[test]
    fn cw_keyed_as_audio_keeps_the_radio_in_usb() {
        for (keying, sent, never) in
            [(CwKeying::Audio, 0x01u8, 0x03u8), (CwKeying::Cat, 0x03u8, 0x01u8)]
        {
            let sim = Sim::start(SimOptions { scope: false, ..Default::default() }).unwrap();
            let mut c = cfg(&sim);
            c.cw_keying = keying;
            let mut src = IcomNetSource::open(&c).expect("open");
            src.set_control_mode(Mode::Cw).unwrap();
            wait_for("the set-mode frame", || {
                sim.civ_frames().iter().any(|f| f.get(4) == Some(&0x06) && f.get(5) == Some(&sent))
            });
            assert!(
                !sim.civ_frames()
                    .iter()
                    .any(|f| f.get(4) == Some(&0x06) && f.get(5) == Some(&never)),
                "{keying:?}: the wrong mode went to the radio"
            );
        }
    }

    #[test]
    fn a_poll_already_in_flight_does_not_drag_the_mode_back() {
        // The mode is polled every 200 ms, so a read issued just before the
        // operator switched comes back carrying the old mode. The simulator
        // never actually changes mode — it answers every read with USB — which
        // is the worst case: without the guard the app would be pulled straight
        // back off the LSB it had just been put in.
        let sim = Sim::start(SimOptions { scope: false, ..Default::default() }).unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");
        // Let the opening read of the mode go by first.
        wait_for("the radio's opening mode report", || {
            src.poll_control().iter().any(|u| matches!(u, ControlUpdate::Mode(_)))
        });

        src.set_control_mode(Mode::Lsb).unwrap();
        let until = Instant::now() + Duration::from_millis(600);
        while Instant::now() < until {
            for u in src.poll_control() {
                assert!(
                    !matches!(u, ControlUpdate::Mode(_)),
                    "a stale USB report was let through while LSB was settling"
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        // But only for a bounded while: a radio that will not go where it was
        // told still gets the last word, or the two stay out of step for good.
        wait_for("the radio winning after the settle window", || {
            src.poll_control().iter().any(|u| matches!(u, ControlUpdate::Mode(Mode::Usb)))
        });
    }

    #[test]
    fn tuning_sends_the_radio_a_set_frequency() {
        let sim = Sim::start(SimOptions { scope: false, ..Default::default() }).unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");
        src.set_center_hz(21_074_000.0).unwrap();
        wait_for("a set-frequency frame", || {
            sim.civ_frames().iter().any(|f| {
                f.get(4) == Some(&0x05)
                    && civ::decode_freq(&f[5..f.len() - 1]) == Some(21_074_000.0)
            })
        });
    }

    #[test]
    fn a_known_model_gets_its_modulation_input_switched_to_lan() {
        let sim = Sim::start(SimOptions { scope: false, ..Default::default() }).unwrap();
        let _src = IcomNetSource::open(&cfg(&sim)).expect("open");
        // 1A 05 00 84 05 and 1A 05 00 85 05 — DATA OFF MOD and DATA MOD to LAN.
        wait_for("the modulation-input writes", || {
            let frames = sim.civ_frames();
            let seen = |item: u8| {
                frames.iter().any(|f| {
                    f.len() >= 10 && f[4] == 0x1A && f[5] == 0x05 && f[7] == item && f[8] == 0x05
                })
            };
            seen(0x84) && seen(0x85)
        });
    }

    /// Issue #252: an IC-7300MK2 was left with DATA MOD and DATA OFF MOD on
    /// LAN, so the next time it was used standalone in the shack it put out a
    /// carrier and nothing else — no microphone, no key, and nothing on the
    /// radio saying why. What sdroxide borrows from the Set-mode menu it has
    /// to give back.
    #[test]
    fn the_menu_items_the_session_borrowed_go_back_when_it_ends() {
        let sim = Sim::start(SimOptions { scope: false, ..Default::default() }).unwrap();
        let src = IcomNetSource::open(&cfg(&sim)).expect("open");
        // DATA OFF MOD and DATA MOD on LAN for the length of the session — that
        // is what makes transmit audio heard, and it is not in dispute.
        wait_for("the modulation-input writes", || {
            sim.menu_item(0x0084) == 0x05 && sim.menu_item(0x0085) == 0x05
        });

        // The way every session but a reopen ends: the source is dropped —
        // a radio switched off, its tab closed, or the application quitting.
        drop(src);
        assert_eq!(sim.menu_item(0x0084), 0x00, "DATA OFF MOD was left on LAN");
        assert_eq!(sim.menu_item(0x0085), 0x00, "DATA MOD was left on LAN");
        // The LAN output select is borrowed on the same terms.
        assert_eq!(
            sim.menu_item(0x0079),
            0x00,
            "the LAN output select was left as sdroxide set it"
        );
    }

    #[test]
    fn a_menu_item_the_radio_already_agreed_with_is_not_written_back() {
        // A rig whose operator keeps it on LAN anyway — and, for the same
        // reason, a second session on a radio the first one has already
        // switched: there is nothing to put back, and a restore here would be
        // this session inventing a state to leave behind.
        let sim = Sim::start(SimOptions { scope: false, menu_default: 0x05, ..Default::default() })
            .unwrap();
        let src = IcomNetSource::open(&cfg(&sim)).expect("open");
        let writes = |item: u8| {
            sim.civ_frames()
                .iter()
                .filter(|f| {
                    f.len() >= 10 && f[4] == 0x1A && f[5] == 0x05 && f[6] == 0x00 && f[7] == item
                })
                .count()
        };
        wait_for("the modulation-input writes", || writes(0x84) > 0 && writes(0x85) > 0);
        drop(src);
        assert_eq!(writes(0x84), 1, "DATA OFF MOD was written back to where it already was");
        assert_eq!(writes(0x85), 1, "DATA MOD was written back to where it already was");
    }

    #[test]
    fn an_ic705_gets_its_own_menu_numbers_and_its_own_lan_value() {
        // Field report, 2026-08-19: an IC-705 on WiFi came up with both "menu
        // numbering unknown" notes. Its Connectors block is a hundred items
        // further on than the MK2's, and its modulation source is `03` (WLAN),
        // not `05` (LAN) — writing the MK2's numbers here would have hit
        // something else entirely.
        let sim = Sim::start(SimOptions {
            civ_address: 0xA4,
            radio_name: "IC-705".into(),
            scope: false,
            ..Default::default()
        })
        .unwrap();
        let mut c = cfg(&sim);
        c.rx_source = IcomRxSource::If12k;
        let src = IcomNetSource::open(&c).expect("open");
        assert_eq!(src.open_status(), None, "nothing left for the operator to do by hand");

        let menu = |hi: u8, lo: u8, value: u8| {
            sim.civ_frames().iter().any(|f| {
                f.len() >= 10
                    && f[4] == 0x1A
                    && f[5] == 0x05
                    && f[6] == hi
                    && f[7] == lo
                    && f[8] == value
            })
        };
        wait_for("the IC-705 menu writes", || {
            // WLAN AF/IF Output > Output Select = IF, and both MOD inputs to WLAN.
            menu(0x01, 0x14, 0x01) && menu(0x01, 0x18, 0x03) && menu(0x01, 0x19, 0x03)
        });
        // And never the USB port's copy of the same setting, one item block
        // earlier — that would leave the network stream on AF.
        assert!(!menu(0x01, 0x09, 0x01), "the USB output select is not ours to write");
    }

    /// Field report, issue #183: an IC-7760 over its RF deck's LAN port came
    /// up with both "menu numbering unknown" notes, so its LAN output stayed on
    /// AF — a 3 kHz-wide audio FFT where the operator had asked for the 12 kHz
    /// IF — and nothing it transmitted was modulated by sdroxide at all.
    #[test]
    fn an_ic7760_gets_its_own_menu_numbers_and_every_one_of_its_data_slots() {
        let sim = Sim::start(SimOptions {
            civ_address: 0xB2,
            radio_name: "IC-7760".into(),
            scope: false,
            ..Default::default()
        })
        .unwrap();
        let mut c = cfg(&sim);
        c.rx_source = IcomRxSource::If12k;
        let src = IcomNetSource::open(&c).expect("open");
        assert_eq!(src.open_status(), None, "nothing left for the operator to do by hand");

        let menu = |hi: u8, lo: u8, value: u8| {
            sim.civ_frames().iter().any(|f| {
                f.len() >= 10
                    && f[4] == 0x1A
                    && f[5] == 0x05
                    && f[6] == hi
                    && f[7] == lo
                    && f[8] == value
            })
        };
        wait_for("the IC-7760 menu writes", || {
            // LAN AF/IF Output > Output Select = IF, and DATA OFF plus all
            // three DATA slots to LAN — which is `09` on this radio.
            menu(0x01, 0x23, 0x01)
                && menu(0x01, 0x29, 0x09)
                && menu(0x01, 0x30, 0x09)
                && menu(0x01, 0x31, 0x09)
                && menu(0x01, 0x32, 0x09)
        });
        // Never the [USB B] port's or the LINE-OUT socket's copy of the output
        // select, and never the MK2's or the IC-705's value for "LAN" — `03`
        // is ACC on this radio and `05` is MIC+LINE-IN.
        assert!(!menu(0x01, 0x03, 0x01), "the USB output select is not ours to write");
        assert!(!menu(0x01, 0x10, 0x01), "the LINE-OUT output select is not ours either");
        assert!(!menu(0x01, 0x29, 0x03) && !menu(0x01, 0x29, 0x05), "another model's LAN value");
    }

    /// The IC-7760 is the first LAN Icom whose sweep is neither 475 bins nor a
    /// 0..=160 scale: 689 points on 0..=200. Reading the old shape would draw
    /// the right band at the wrong width and 20 dB out.
    #[test]
    fn an_ic7760_sweep_arrives_as_689_bins_on_its_own_taller_scale() {
        let sim = Sim::start(SimOptions {
            civ_address: 0xB2,
            radio_name: "IC-7760".into(),
            ..Default::default()
        })
        .unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");

        let mut buf = vec![Complex32::default(); 1024];
        let mut bins = Vec::new();
        wait_for("a scope sweep", || {
            let _ = src.read(&mut buf);
            src.wide_spectrum_db(&mut bins).is_some()
        });
        assert_eq!(bins.len(), 689, "the whole sweep, in the one frame a LAN Icom sends");
        // The simulator plants its peak just below full scale, so the loudest
        // bin has to land just below 0 dB. On the 0..=160 scale the other
        // models use, the same byte would come out well above it.
        let peak = bins.iter().copied().fold(f32::MIN, f32::max);
        assert!((-10.0..0.0).contains(&peak), "peak came back at {peak} dB");
    }

    /// Field report, issue #190: an IC-7610 over its LAN port was told
    /// sdroxide did not know its menu numbering, and so had to be sent to
    /// SET > Connectors > MOD Input by hand before anything it transmitted was
    /// modulated at all.
    #[test]
    fn an_ic7610_gets_its_own_menu_numbers_and_every_one_of_its_data_slots() {
        let sim = Sim::start(SimOptions {
            civ_address: 0x98,
            radio_name: "IC-7610".into(),
            scope: false,
            ..Default::default()
        })
        .unwrap();
        let mut c = cfg(&sim);
        c.rx_source = IcomRxSource::If12k;
        let src = IcomNetSource::open(&c).expect("open");
        assert_eq!(src.open_status(), None, "nothing left for the operator to do by hand");

        let menu = |hi: u8, lo: u8, value: u8| {
            sim.civ_frames().iter().any(|f| {
                f.len() >= 10
                    && f[4] == 0x1A
                    && f[5] == 0x05
                    && f[6] == hi
                    && f[7] == lo
                    && f[8] == value
            })
        };
        wait_for("the IC-7610 menu writes", || {
            // LAN AF/IF Output > Output Select = IF, and DATA OFF plus all
            // three DATA slots to LAN — `05` on this radio.
            menu(0x00, 0x86, 0x01)
                && menu(0x00, 0x91, 0x05)
                && menu(0x00, 0x92, 0x05)
                && menu(0x00, 0x93, 0x05)
                && menu(0x00, 0x94, 0x05)
        });
        // Never the [USB] port's copy of the output select, six items earlier,
        // and never the IC-7760's value for "LAN" even though the two radios
        // have the same four slots: `09` is off the end of this rig's list.
        assert!(!menu(0x00, 0x80, 0x01), "the USB output select is not ours to write");
        assert!(!menu(0x00, 0x91, 0x09), "another model's LAN value");
    }

    /// The IC-7610 sweeps the IC-7760's shape — 689 bins on 0..=200 — from a
    /// generation that otherwise sends 475 on 0..=160.
    #[test]
    fn an_ic7610_sweep_arrives_as_689_bins_on_its_own_taller_scale() {
        let sim = Sim::start(SimOptions {
            civ_address: 0x98,
            radio_name: "IC-7610".into(),
            ..Default::default()
        })
        .unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");

        let mut buf = vec![Complex32::default(); 1024];
        let mut bins = Vec::new();
        wait_for("a scope sweep", || {
            let _ = src.read(&mut buf);
            src.wide_spectrum_db(&mut bins).is_some()
        });
        assert_eq!(bins.len(), 689, "the whole sweep, in the one frame a LAN Icom sends");
        let peak = bins.iter().copied().fold(f32::MIN, f32::max);
        assert!((-10.0..0.0).contains(&peak), "peak came back at {peak} dB");
    }

    /// An IC-R8600 is in the model table for the output select alone. The
    /// modulation-input writes must stay away from it: `00 89` is one item
    /// past the Speech-output block on that set, and it has no MOD input at
    /// all to point at LAN.
    #[test]
    fn a_receiver_gets_its_output_select_and_no_modulation_write() {
        let sim = Sim::start(SimOptions {
            civ_address: 0x96,
            radio_name: "IC-R8600".into(),
            can_transmit: false,
            scope: false,
            ..Default::default()
        })
        .unwrap();
        let mut c = cfg(&sim);
        c.rx_source = IcomRxSource::If12k;
        let src = IcomNetSource::open(&c).expect("open");
        // It says the set cannot transmit, which is a fact about the radio —
        // and nothing about a menu, which would be an errand on a rig with no
        // modulation input to run it on.
        let status = src.open_status().unwrap_or_default();
        assert!(!status.contains("menu numbering"), "{status}");

        wait_for("the output select", || {
            sim.civ_frames().iter().any(|f| {
                f.len() >= 10 && f[4] == 0x1A && f[5] == 0x05 && f[6..9] == [0x00, 0x89, 0x01]
            })
        });
        let mod_write = sim
            .civ_frames()
            .iter()
            .any(|f| f.len() >= 10 && f[4] == 0x1A && f[5] == 0x05 && f[6] == 0x00 && f[7] > 0x89);
        assert!(!mod_write, "nothing past the output select is ours on a receiver");
    }

    #[test]
    fn the_scope_span_is_commanded_rather_than_left_wherever_the_radio_had_it() {
        let sim = Sim::start(SimOptions { civ_address: 0xA4, ..Default::default() }).unwrap();
        let mut c = cfg(&sim);
        c.scope_span = sdroxide_types::IcomScopeSpan::Khz500;
        let _src = IcomNetSource::open(&c).expect("open");
        wait_for("centre mode and a ±500 kHz span", || {
            let frames = sim.civ_frames();
            let centre = frames
                .iter()
                .any(|f| f.len() >= 8 && f[4] == 0x27 && f[5] == 0x14 && f[6..8] == [0x00, 0x00]);
            let span = frames.iter().any(|f| {
                f.len() >= 12
                    && f[4] == 0x27
                    && f[5] == 0x15
                    && f[6] == 0x00
                    && civ::decode_freq(&f[7..12]) == Some(500_000.0)
            });
            centre && span
        });
    }

    #[test]
    fn leaving_the_span_to_the_radio_touches_neither_setting() {
        let sim = Sim::start(SimOptions { civ_address: 0xA4, ..Default::default() }).unwrap();
        let mut c = cfg(&sim);
        c.scope_span = sdroxide_types::IcomScopeSpan::Radio;
        let mut src = IcomNetSource::open(&c).expect("open");
        // Wait for something the session does send, so "nothing was sent" is a
        // real observation rather than a race with the opening burst.
        let mut buf = vec![Complex32::default(); 1024];
        let mut bins = Vec::new();
        wait_for("a scope sweep", || {
            let _ = src.read(&mut buf);
            src.wide_spectrum_db(&mut bins).is_some()
        });
        assert!(
            !sim.civ_frames()
                .iter()
                .any(|f| f.len() >= 6 && f[4] == 0x27 && (f[5] == 0x14 || f[5] == 0x15)),
            "the operator asked for the radio's own span"
        );
    }

    #[test]
    fn an_unknown_model_says_what_the_operator_has_to_set_by_hand() {
        // An IC-7100: no network port, so it can never be in the model table.
        // The IC-9700 that used to stand in here is in it now.
        let sim = Sim::start(SimOptions {
            civ_address: 0x88,
            radio_name: "IC-7100".into(),
            scope: false,
            ..Default::default()
        })
        .unwrap();
        let src = IcomNetSource::open(&cfg(&sim)).expect("open");
        let status = src.open_status().expect("a note about the menu");
        assert!(status.contains("modulation input"), "{status}");
    }

    #[test]
    fn asking_for_the_if_at_a_rate_that_cannot_carry_it_falls_back_and_says_so() {
        let sim =
            Sim::start(SimOptions { sample_rate: 24_000, scope: false, ..Default::default() })
                .unwrap();
        let mut c = cfg(&sim);
        c.rx_source = IcomRxSource::If12k;
        c.sample_rate_hz = 24_000;
        let src = IcomNetSource::open(&c).expect("open");
        assert_eq!(src.sample_rate(), 24_000.0);
        assert!(src.open_status().unwrap().contains("12 kHz IF needs a 48 kHz"));
    }

    #[test]
    fn transmitting_keys_the_radio_and_unkeying_releases_it() {
        let sim = Sim::start(SimOptions { scope: false, ..Default::default() }).unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");
        let ptt = |on: u8| {
            sim.civ_frames()
                .iter()
                .any(|f| f.len() >= 7 && f[4] == 0x1C && f[5] == 0x00 && f[6] == on)
        };
        src.tx_begin(14_074_000.0, 48_000.0).unwrap();
        wait_for("PTT on", || ptt(1));
        src.tx_end().unwrap();
        wait_for("PTT off", || ptt(0));
    }

    #[test]
    fn a_dead_session_asks_to_be_reopened() {
        let sim = Sim::start(SimOptions { scope: false, ..Default::default() }).unwrap();
        let mut src = IcomNetSource::open(&cfg(&sim)).expect("open");
        assert!(!src.needs_reopen());
        src.release();
        assert!(src.needs_reopen());
    }
}
