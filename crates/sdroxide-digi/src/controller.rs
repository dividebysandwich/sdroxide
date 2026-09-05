//! `DigiController` — the real-time-loop glue that ties the modem, QSO
//! machine, and slot scheduler into the engine.
//!
//! The engine calls [`on_rx_audio`](DigiController::on_rx_audio) with the
//! demodulated audio tap each block, and [`poll`](DigiController::poll) once
//! per loop tick. `poll` never blocks: heavy LDPC decode runs on a worker
//! thread, and the controller returns [`DigiAction`]s for the engine to
//! apply (emit events, key/unkey PTT).

use std::cmp::Ordering;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::SystemTime;

use sdroxide_dsp::MonoResampler;
use sdroxide_types::{
    Decode, DigiConfig, DigiStatus, Mode, QsoRecord, RifpMeta, RifpStatus, SstvMode, SstvStatus,
    adif_band,
};

use crate::clock::ClockMonitor;
use crate::modem::{ApHints, Ft8Modem};
use crate::params::{DECODE_RATE, DigiParams};

/// How much of a slot's audio has to arrive before the period counts as whole.
///
/// Not 100%: a slot legitimately ends a few blocks short, because the boundary
/// is noticed on a poll rather than on a sample, and the tap is handed over in
/// blocks of a few tens of milliseconds. Five per cent of a 15 s slot is 750 ms
/// — far more slack than that, and far less than the gap that costs a decode.
const SLOT_COMPLETE_FRAC: f64 = 0.95;

/// How often a run of short periods says so again — every tenth, which at
/// fifteen seconds a slot is about once every two and a half minutes.
const SHORT_SLOT_REPORT_EVERY: u64 = 10;
use crate::qso::QsoMachine;
use crate::scheduler::SlotScheduler;

/// What the engine should do in response to a [`poll`](DigiController::poll).
#[derive(Debug, Clone)]
pub enum DigiAction {
    /// New decodes from a completed receive slot.
    Decodes(Vec<Decode>),
    /// Status change (QSO step, pending TX, etc.).
    Status(DigiStatus),
    /// A completed QSO to append to the log.
    QsoLogged(QsoRecord),
    /// Begin transmitting the queued burst this slot.
    KeyTx,
    /// Stop transmitting.
    UnkeyTx,
    /// SSTV: a freshly decoded scanline (`rgb` is `3 * width` bytes) at row `y`.
    SstvLine { image_id: u32, y: u16, rgb: Vec<u8> },
    /// SSTV: a completed image (raw RGB) — the engine encodes/persists it.
    SstvImage { image_id: u32, mode: SstvMode, w: u16, h: u16, rgb: Vec<u8> },
    /// SSTV: engine status change (tx/rx active, detected mode, progress).
    SstvStatus(SstvStatus),
    /// Weather fax: a freshly decoded scan line, one byte per pixel.
    WefaxLine { image_id: u32, y: u16, gray: Vec<u8> },
    /// Weather fax: a finished chart (grayscale) — the engine encodes and
    /// persists it. Unlike SSTV the height is not known in advance: it is
    /// however many lines arrived before the stop tone.
    WefaxImage { image_id: u32, w: u16, h: u16, gray: Vec<u8> },
    /// Weather fax: receiver status (tuning, phasing, line count).
    WefaxStatus(sdroxide_types::WefaxStatus),
    /// RIFP: reassembled raster rows of an incoming picture — `rows` grayscale
    /// bytes starting at row `y`, `w` per row. Only the unencoded raster can be
    /// painted before the object is whole.
    RifpRows { image_id: u32, y: u16, w: u16, h: u16, rows: Vec<u8> },
    /// RIFP: a completed, digest-verified picture (raw RGB) — the engine
    /// encodes and persists it.
    RifpImage { image_id: u32, meta: RifpMeta, w: u16, h: u16, rgb: Vec<u8> },
    /// RIFP: engine status change (transfer progress, sessions, counters).
    RifpStatus(RifpStatus),
    /// FSQ image: a completed grayscale-as-RGB image — the engine encodes it.
    DigiImage { w: u16, h: u16, rgb: Vec<u8> },
    /// Hellschreiber: a run of freshly received dot columns, column-major
    /// (`cols[c * rows + r]`, row 0 at the top), 0 = black … 255 = white.
    ///
    /// Batched rather than one action per column: X9 makes 157 columns a second
    /// and each is only fourteen bytes, so per-column messages would be mostly
    /// framing. `seq` is the absolute column index, which is how the panel tells
    /// a dropped batch (leave a gap) from a restarted receiver (clear).
    HellColumns { seq: u64, rows: u8, cols: Vec<u8> },
    /// WSPR: what a completed two-minute slot decoded.
    ///
    /// Not `Decodes`: a WSPR reception is a measurement of a path, not a
    /// message addressed to anyone, and the transmit power and drift that make
    /// it one have nowhere to live in a [`Decode`].
    WsprSpots(Vec<sdroxide_types::WsprSpot>),
    /// WSPR band hopping: put the dial here for the slot now starting.
    ///
    /// A request, not an instruction. The engine owns the VFO and refuses when
    /// it is transmitting, when the device cannot reach the frequency, or when
    /// the operator has a hand on the tuning — a beacon and its operator
    /// fighting over the dial is the failure this must not have.
    SetDial(f64),
    /// RADE: a remote station's callsign, recovered from its End-of-Over frame,
    /// with the SNR at the end of the over and the dial it was heard on.
    ///
    /// An event rather than a field on [`sdroxide_types::RadeStatus`]: status is
    /// emitted by diff, so decoding the *same* station twice running — the
    /// common case in a QSO — would stop re-reporting it.
    RadeCallsign { call: String, snr_db: f32, freq_hz: f64 },
    /// CW for the *rig's* keyer to send, rather than sidetone for the engine to
    /// transmit. A transceiver in CW mode keys its own transmitter and ignores
    /// what arrives at its sound card, so on a CAT rig this is the only route
    /// to the air — and it carries no PTT with it, because the rig switches to
    /// transmit for the length of the message itself.
    ///
    /// `seconds` is how long that will take at the speed the rig was told to
    /// key at. Nothing comes back from a rig part way through a message, so an
    /// engine that has to know how long this radio is on the air for — to hold
    /// the station's transmit interlock across it — has only this.
    SendCw { text: String, seconds: f32 },
    /// Stop CW the rig is part way through sending.
    AbortCw,
}

struct DecodeJob {
    audio: Vec<i16>,
    /// Which slot this audio was captured in, as the scheduler's own index.
    ///
    /// The index rather than the Unix start time: only FT8's slots begin on a
    /// whole second. FT4's odd slots start on a half second and FT2's on a
    /// quarter, so a start time carried as whole seconds reads back as the slot
    /// *before* it — which inverted the parity of every reply and put it on top
    /// of the station being answered (issue #191).
    slot_idx: i64,
    /// The same slot as a Unix time, which is what stamps each [`Decode`].
    slot_utc: i64,
    /// Our callsign and the DX's. They seed the worker's hash table, so a hashed
    /// `<...>` naming either resolves on first sight, and they bias the decoder
    /// towards the message we are actually waiting for (see [`ApHints`]).
    ap: ApHints,
    /// Where we are listening, for FT4's targeted a-priori pass.
    audio_hz: f32,
}

pub struct DigiController {
    params: DigiParams,
    scheduler: SlotScheduler,
    qso: QsoMachine,
    modem: Ft8Modem,
    resampler: Option<MonoResampler>,
    /// 12 kHz i16 audio accumulated for the current slot.
    slot_buf: Vec<i16>,
    tap_scratch: Vec<f32>,
    last_slot_idx: i64,
    /// Receive periods that arrived short of a full slot — see
    /// [`DigiController::check_slot_arrived_whole`].
    short_slots: u64,
    dial_hz: f64,
    audio_hz: f32,
    /// Which slot period we transmit in (even/odd), and a per-slot guard so
    /// we key at most once per slot.
    tx_even: bool,
    tx_fired_slot: i64,
    /// Slot index we last decoded each station in, so a reply always lands in
    /// the slot *opposite* to when the DX actually transmitted — even if
    /// stations run out of the usual even/odd sequence.
    ///
    /// Kept for [`HEARD_SLOTS`], which is much longer than the frequency
    /// chooser's window below: what this answers is "which period does that
    /// station keep", and a station keeps one all evening.
    last_heard: std::collections::HashMap<String, i64>,
    /// `(slot index, tone offset)` of every recent decode, so we can see which
    /// part of the band is busy *in the period we transmit in*. Duplicated tones
    /// are kept: two stations on one frequency is exactly the situation worth
    /// avoiding.
    recent_activity: Vec<(i64, f32)>,
    // Decode worker.
    job_tx: Sender<DecodeJob>,
    res_rx: Receiver<(i64, Vec<Decode>)>,
    _worker: std::thread::JoinHandle<()>,
    // TX burst playback.
    burst: Option<BurstPlayer>,
    status_dirty: bool,
    /// What the decoders' `dt` values say about our own slot timing.
    clock: ClockMonitor,
}

/// Metered playback of a synthesized TX burst (48 kHz mono).
pub struct BurstPlayer {
    pub samples: Vec<f32>,
    pub pos: usize,
}

/// The widest stretch of the passband [`clearest_tx_hz`] will choose from, and
/// the grid it searches on. Kept inside the usual SSB filter with room for the
/// ~50 Hz signal at either edge.
///
/// The band plan narrows this further where it has an opinion, so these are the
/// outer bounds rather than the range actually searched. See
/// [`DigiController::emission_permitted`].
const TX_PICK_MIN_HZ: f32 = 400.0;
const TX_PICK_MAX_HZ: f32 = 2600.0;
const TX_PICK_STEP_HZ: f32 = 10.0;
/// How many slots back the transmit-frequency chooser counts as "now". Who is
/// on the air this minute is what it is asking about.
const ACTIVITY_SLOTS: i64 = 8;
/// How many slots a station's transmit period is remembered for.
///
/// Long enough to cover the rows the decode list can still be showing — it
/// holds two hundred of them, which on a quiet band is a good many minutes.
/// Answering one of those from a record of when they last transmitted is right;
/// guessing at it is a coin toss that lands on top of them half the time.
const HEARD_SLOTS: i64 = 40;

/// Separation past which a spot counts as simply clear. An FT8 signal is about
/// 50 Hz wide, so a couple of signal-widths either side is all the room there
/// is any point in having — beyond it, nearness to where we already are is the
/// better tie-break.
const CLEAR_ENOUGH_HZ: f32 = 120.0;

/// Choose a transmit tone offset with the least company.
///
/// `busy` are the tone offsets of stations decoded in the period we are about
/// to transmit in — the only ones we can actually collide with. The result is
/// the spot furthest from all of them; among spots that are equally clear, the
/// one nearest `current`, so the transmit marker doesn't wander across the band
/// between contacts for no reason.
fn clearest_tx_hz(busy: &[f32], current: f32, permitted: impl Fn(f32) -> bool) -> f32 {
    let steps = ((TX_PICK_MAX_HZ - TX_PICK_MIN_HZ) / TX_PICK_STEP_HZ) as i32;
    let candidates = || {
        (0..=steps).map(|i| TX_PICK_MIN_HZ + i as f32 * TX_PICK_STEP_HZ).filter(|&hz| permitted(hz))
    };
    let clearance = |hz: f32| {
        busy.iter().map(|b| (b - hz).abs()).fold(f32::INFINITY, f32::min).min(CLEAR_ENOUGH_HZ)
    };
    let best = candidates().map(clearance).fold(f32::NEG_INFINITY, f32::max);
    candidates()
        .filter(|&hz| clearance(hz) >= best)
        .min_by(|a, b| {
            (a - current).abs().partial_cmp(&(b - current).abs()).unwrap_or(Ordering::Equal)
        })
        .unwrap_or(current)
}

impl DigiController {
    pub fn new(mode: Mode, cfg: DigiConfig, tap_rate: f64) -> Self {
        let params = DigiParams::for_mode(mode);
        let resampler = MonoResampler::new(tap_rate, DECODE_RATE);

        // Decode worker: owns its own modem, runs LDPC off the RT thread.
        let (job_tx, job_rx) = std::sync::mpsc::channel::<DecodeJob>();
        let (res_tx, res_rx) = std::sync::mpsc::channel::<(i64, Vec<Decode>)>();
        let worker_mode = params.mode;
        let worker = std::thread::Builder::new()
            .name("sdroxide-ft8-decode".into())
            .spawn(move || {
                let mut modem = Ft8Modem::new(worker_mode);
                while let Ok(job) = job_rx.recv() {
                    modem.seed_hashes(&job.ap.calls());
                    let decodes =
                        modem.decode_slot(&job.audio, job.slot_utc, &job.ap, job.audio_hz);
                    if res_tx.send((job.slot_idx, decodes)).is_err() {
                        break;
                    }
                }
            })
            .expect("spawn ft8 decode worker");

        let tx_even = cfg.tx_even;
        let qso = QsoMachine::new(params.mode, cfg);

        DigiController {
            params,
            scheduler: SlotScheduler::for_mode(mode),
            qso,
            modem: Ft8Modem::new(params.mode),
            resampler,
            slot_buf: Vec::with_capacity(params.slot_samples()),
            tap_scratch: Vec::new(),
            last_slot_idx: i64::MIN,
            short_slots: 0,
            dial_hz: 0.0,
            audio_hz: 1500.0,
            tx_even,
            tx_fired_slot: i64::MIN,
            last_heard: std::collections::HashMap::new(),
            recent_activity: Vec::new(),
            job_tx,
            res_rx,
            _worker: worker,
            burst: None,
            status_dirty: true,
            clock: ClockMonitor::new(),
        }
    }

    pub fn mode(&self) -> Mode {
        self.params.mode
    }

    pub fn set_config(&mut self, cfg: DigiConfig) {
        // A Fox owns its period: it transmits in the configured one and the
        // whole pile-up sequences off that, so nothing may flip it later.
        if cfg.dxped_mode == sdroxide_types::DxpedMode::Fox {
            self.tx_even = cfg.tx_even;
        }
        self.qso.set_config(cfg);
        self.status_dirty = true;
    }

    /// Set our transmit tone offset, as the operator asked.
    ///
    /// A Hound is held out of the Fox's half of the passband: the low end
    /// belongs to the DXpedition's own signals, and a Hound calling down there
    /// transmits on top of the station the whole pile-up is trying to work. The
    /// one legitimate move into it — following the Fox once it has answered us —
    /// goes through [`tune_audio_hz`](Self::tune_audio_hz) instead.
    pub fn set_audio_hz(&mut self, hz: f32) {
        // Held means held, whoever is asking. This is the operator's own route
        // (a click on a decode or the waterfall, the offset box, a key binding)
        // *and* the follow-the-DX branch of `start_qso`, so one check covers
        // both the deliberate move and the automatic one.
        if self.tx_held() {
            return;
        }
        let hz = if self.is_hound() { hz.max(sdroxide_types::FOX_ZONE_MAX_HZ) } else { hz };
        self.tune_audio_hz(hz);
    }

    /// True when the operator has pinned the transmit tone.
    ///
    /// Deliberately not overridable by a modifier key the way WSJT-X's Hold Tx
    /// Freq is: the case this exists for is a licence edge, where a move made by
    /// accident is an out-of-band transmission and not merely bad manners.
    fn tx_held(&self) -> bool {
        self.qso.status(false).config.hold_tx_freq
    }

    fn is_hound(&self) -> bool {
        self.qso.dxped() == sdroxide_types::DxpedMode::Hound
    }

    /// Move the transmit tone with no zone check — the sequencer's own moves.
    fn tune_audio_hz(&mut self, hz: f32) {
        self.audio_hz = hz.clamp(200.0, 3500.0);
        self.qso.set_audio_hz(self.audio_hz);
        self.status_dirty = true;
    }

    pub fn audio_hz(&self) -> f32 {
        self.audio_hz
    }

    pub fn call_cq(&mut self) {
        // Call CQ in our configured period.
        self.tx_even = self.qso.status(false).tx_even;
        self.pick_tx_freq();
        self.qso.call_cq();
        self.status_dirty = true;
    }

    pub fn start_qso(
        &mut self,
        from: String,
        grid: Option<String>,
        snr: i16,
        audio_hz: f32,
        wait_for_cq: bool,
    ) {
        let now_sys = SystemTime::now();
        let now = SlotScheduler::unix_now(now_sys) as i64;
        // Reply in the slot *opposite* to when the DX actually transmitted, using
        // the slot we last heard them in — so a late reply never lands in their
        // slot. Fall back to "the slot before this one" only if we've no record
        // of them (which reproduces the old parity == parity(now) behaviour).
        let dx_slot = self
            .last_heard
            .get(&from)
            .copied()
            .unwrap_or_else(|| self.scheduler.slot_index(now_sys) - 1);
        self.tx_even = !self.scheduler.is_even(dx_slot);
        // Where to answer from. Moving onto the DX's own frequency is the
        // obvious choice and the wrong one — see `pick_tx_freq`. A Hound is
        // exempt twice over: its calling frequency is the operator's, and the
        // Fox's own half of the band is out of bounds. Both branches below are
        // no-ops under `hold_tx_freq`, which is checked inside each of them
        // rather than here, so every other caller is covered by the same gate.
        if !self.is_hound() {
            if self.qso.status(false).config.auto_tx_freq {
                self.pick_tx_freq();
            } else {
                self.set_audio_hz(audio_hz);
            }
        }
        self.qso.start_qso(from, grid, snr, wait_for_cq, now);
        self.status_dirty = true;
    }

    /// Move our transmit tone to the quietest part of the band *in the period
    /// we are about to transmit in*.
    ///
    /// Answering on the station we are working is the natural-looking thing and
    /// it is backwards: they transmit in the period opposite ours, so their
    /// frequency says nothing about who is transmitting there when we do. The
    /// stations that matter are the ones decoded in our own period — those are
    /// the ones we would land on top of, and neither of us would be heard.
    ///
    /// Does nothing when the operator has turned it off, or in DXpedition mode
    /// where both roles have their frequencies decided for them.
    fn pick_tx_freq(&mut self) {
        let cfg_ok = {
            let s = self.qso.status(false);
            !s.config.hold_tx_freq
                && s.config.auto_tx_freq
                && s.config.dxped_mode == sdroxide_types::DxpedMode::Normal
        };
        if !cfg_ok {
            return;
        }
        let busy: Vec<f32> = self
            .recent_activity
            .iter()
            .filter(|(slot, _)| self.scheduler.is_even(*slot) == self.tx_even)
            .map(|(_, hz)| *hz)
            .collect();
        let hz = clearest_tx_hz(&busy, self.audio_hz, |hz| self.emission_permitted(hz));
        self.tune_audio_hz(hz);
    }

    /// Would a signal at this offset sit inside the band plan's segment?
    ///
    /// The same question the engine's transmit lockout asks at key-down, asked
    /// early so the automatic chooser stops picking slots that would then be
    /// refused. Without it the two disagree in the one case that matters: on a
    /// UK 60 m dial of 5357 kHz the allocation ends at 5358.0, so everything
    /// above 950 Hz is out of band, while the chooser hunts to 2600 Hz and has
    /// no idea. The operator gets a transmit lockout instead of a contact, and
    /// nothing on screen says which of the two settings caused it.
    ///
    /// So Auto TX FRQ becomes usable where a licence is narrower than the mode's
    /// habits, rather than being a thing to be avoided there. Hold TX is
    /// unaffected and is still the way to pin an exact figure.
    ///
    /// Fails OPEN on the same rule as the lockout: unless the dial itself is
    /// inside a listed segment, no opinion is offered. A band plan with a gap in
    /// it must not silently strand the chooser with nowhere to go, and a mode
    /// with no stated bandwidth is not one this can reason about.
    ///
    /// The dial here is the RECEIVE dial, which is the one the controller is
    /// given. Under split or XIT the transmitted dial differs and this can
    /// therefore approve a slot the lockout then refuses, which is exactly
    /// today's behaviour and no worse. It cannot go the other way and approve
    /// something that radiates out of band, because the lockout still has the
    /// final say at key-down.
    fn emission_permitted(&self, offset_hz: f32) -> bool {
        let Some(bw) = self.mode().occupied_bw_hz() else { return true };
        if sdroxide_types::segment_kind_at(self.dial_hz).is_none() {
            return true;
        }
        let lo = self.dial_hz + f64::from(offset_hz);
        sdroxide_types::span_within_segment(lo, lo + f64::from(bw))
    }

    /// Pick which message goes out next (the operator's Tx1–Tx6).
    pub fn set_step(&mut self, step: sdroxide_types::QsoStep) {
        if self.qso.set_step(step) {
            self.status_dirty = true;
        }
    }

    /// Queue a message to send verbatim in the next transmit slot.
    pub fn send_text(&mut self, text: String) {
        self.qso.queue_text(text);
        self.status_dirty = true;
    }

    /// Mark a station to work when the sequencer is next free.
    pub fn queue_add(&mut self, entry: sdroxide_types::QueuedCall) {
        self.qso.queue_add(entry);
        self.status_dirty = true;
    }

    /// Drop a station from the call queue; an empty callsign clears it.
    pub fn queue_remove(&mut self, call: &str) {
        self.qso.queue_remove(call);
        self.status_dirty = true;
    }

    pub fn stop_qso(&mut self) {
        self.qso.stop();
        self.status_dirty = true;
    }

    /// Abort any in-progress burst immediately.
    pub fn abort_tx(&mut self) {
        self.burst = None;
        self.status_dirty = true;
    }

    /// Hard reset (leaving the mode).
    pub fn abort(&mut self) {
        self.burst = None;
        self.qso.abort();
    }

    /// Feed one block of demodulated audio (at `tap_rate`) into the current
    /// receive slot after resampling to 12 kHz.
    pub fn on_rx_audio(&mut self, tap: &[f32]) {
        self.tap_scratch.clear();
        match &mut self.resampler {
            Some(r) => r.push(tap, &mut self.tap_scratch),
            None => self.tap_scratch.extend_from_slice(tap),
        }
        // Cap the slot buffer so a stuck boundary can't grow it unbounded.
        let cap = self.params.slot_samples() + self.params.slot_samples() / 4;
        for &s in &self.tap_scratch {
            if self.slot_buf.len() < cap {
                self.slot_buf.push((s.clamp(-1.0, 1.0) * 28_000.0) as i16);
            }
        }
    }

    /// Say so when a slot's audio arrived short, because a slot that is short
    /// is a slot that will not decode.
    ///
    /// The buffer is filled by *arrival*: whatever the sound card handed over
    /// during the slot, spliced end to end. So a capture stream that loses
    /// samples — a virtual audio cable that is not keeping pace, a machine that
    /// is not emptying the card — does not merely make the audio shorter, it
    /// moves every symbol after the gap earlier than the moment it was actually
    /// sent. The decoder aligns what it is given to the slot it belongs to, so
    /// a gap anywhere in the middle costs the whole slot.
    ///
    /// That failure is invisible from the operator's side: the waterfall is
    /// full of signals, the panel says nothing, and no decodes appear
    /// (issue #338). The audio layer reports the glitch, but nothing there
    /// knows it has cost a decode — and nothing here knew the audio had a hole
    /// in it. This is the join between the two.
    ///
    /// A slot this station transmitted in is skipped: the receiver is stood
    /// down for the length of an over on a half-duplex radio, and its buffer is
    /// legitimately a fraction of a slot.
    fn check_slot_arrived_whole(&mut self) {
        if self.last_slot_idx == self.tx_fired_slot {
            return;
        }
        let want = (self.params.slot_s * DECODE_RATE) as usize;
        let got = self.slot_buf.len();
        if want == 0 || got >= (want as f64 * SLOT_COMPLETE_FRAC) as usize {
            return;
        }
        self.short_slots = self.short_slots.saturating_add(1);
        // One line per run of them, not one per slot: this fires every fifteen
        // seconds for as long as the fault lasts, and the log has to stay
        // readable enough to find the rest of the session in.
        if self.short_slots == 1 || self.short_slots.is_multiple_of(SHORT_SLOT_REPORT_EVERY) {
            tracing::warn!(
                "{}: the last receive period arrived {:.1} s short of the {:.1} s it should be —                  the audio device is losing samples, which moves every tone after the gap and                  costs the whole period. Nothing will decode while this lasts. {} such period(s)                  so far; look for the audio glitch warnings above and at the device feeding                  sdroxide (a virtual audio cable is the usual one).",
                self.params.mode.label(),
                (want - got) as f64 / DECODE_RATE,
                self.params.slot_s,
                self.short_slots,
            );
        }
    }

    /// Whether a TX burst is currently on the air (drives the engine's PTT
    /// via [`DigiAction::KeyTx`]/[`UnkeyTx`]).
    pub fn tx_burst_active(&self) -> bool {
        self.burst.is_some()
    }

    /// Meter the next block of TX audio (48 kHz) into `out`. Returns true
    /// when the burst has finished (engine should unkey and call
    /// [`on_burst_done`](Self::on_burst_done)).
    pub fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        let Some(b) = self.burst.as_mut() else {
            out.fill(0.0);
            return true;
        };
        let mut done = false;
        for slot in out.iter_mut() {
            if b.pos < b.samples.len() {
                *slot = b.samples[b.pos];
                b.pos += 1;
            } else {
                *slot = 0.0;
                done = true;
            }
        }
        if done {
            self.burst = None;
        }
        done
    }

    /// Notify the QSO machine that the burst finished going out. A final
    /// message (73 / RR73) leaving is what logs the contact and moves it to
    /// `Confirming`; anything else just counts towards the unanswered-call
    /// limit. Marks the status dirty so the engine re-broadcasts it.
    pub fn on_burst_done(&mut self) {
        let now = SlotScheduler::unix_now(SystemTime::now()) as i64;
        self.qso.note_tx_sent(now);
        self.status_dirty = true;
    }

    /// Synthesize one slot's transmission into a 48 kHz mono burst (12 kHz
    /// GFSK, resampled). Returns the audio and each message as the far end will
    /// read it, which is what the transcript logs — a compound-call or over-long
    /// message is degraded to a form FT8 can carry (see
    /// [`Ft8Modem::encode_burst_12k`]).
    ///
    /// `msgs` is usually one message; a Fox transmits several at once, summed,
    /// each on its own tone. The per-signal amplitude is divided between them so
    /// the total stays inside the same headroom a single burst uses — five
    /// signals at full drive would clip the transmitter, and an intermodulating
    /// Fox is heard as splatter across the whole pile-up.
    fn synth_burst_48k(&mut self, msgs: &[(String, f32)]) -> Option<(Vec<f32>, Vec<String>)> {
        let amp = 0.5 / msgs.len().max(1) as f32;
        let mut mix: Vec<f32> = Vec::new();
        let mut sent = Vec::with_capacity(msgs.len());
        for (msg, hz) in msgs {
            let Some((burst12, as_sent)) = self.modem.encode_burst_12k(msg, *hz, amp) else {
                continue;
            };
            if mix.len() < burst12.len() {
                mix.resize(burst12.len(), 0.0);
            }
            for (m, s) in mix.iter_mut().zip(&burst12) {
                *m += s;
            }
            sent.push(as_sent);
        }
        if mix.is_empty() {
            return None;
        }
        // Resample 12 k → 48 k.
        match MonoResampler::new(DECODE_RATE, 48_000.0) {
            Some(mut r) => {
                let mut out = Vec::with_capacity(mix.len() * 4 + 2048);
                r.push(&mix, &mut out);
                Some((out, sent))
            }
            None => Some((mix, sent)),
        }
    }

    /// Called each engine loop tick. Detects slot boundaries, dispatches the
    /// finished slot to the decode worker, drains results, advances the QSO
    /// machine, and (D3) schedules TX. Returns actions for the engine.
    pub fn poll(&mut self, now: SystemTime, dial_hz: f64) -> Vec<DigiAction> {
        self.dial_hz = dial_hz;
        let mut actions = Vec::new();

        // 0. Advance QSO timeouts (WaitCq give-up, Confirming retire).
        if self.qso.tick(SlotScheduler::unix_now(now) as i64) {
            self.status_dirty = true;
        }

        // 0b. Free, with stations marked: take the next one. This is what makes
        // the queue hands-off — every contact ending, however it ends, walks the
        // sequencer on to the station the operator marked next.
        if let Some(next) = self.qso.take_next_queued() {
            self.start_qso(next.call, next.grid, next.snr_db, next.audio_hz, next.wait_for_cq);
        }

        // 1. Drain finished decodes from the worker.
        loop {
            match self.res_rx.try_recv() {
                Ok((slot_idx, decodes)) => {
                    if !decodes.is_empty() {
                        let slot_utc = self.scheduler.slot_start_unix(slot_idx) as i64;
                        // Remember which slot we heard each station in (reply
                        // timing), pruned to stay bounded.
                        for d in &decodes {
                            if let Some(from) = d.from.as_deref().filter(|s| !s.is_empty()) {
                                self.last_heard.insert(from.to_string(), slot_idx);
                            }
                            self.recent_activity.push((slot_idx, d.audio_hz));
                        }
                        self.last_heard.retain(|_, &mut s| s >= slot_idx - HEARD_SLOTS);
                        self.recent_activity.retain(|(s, _)| *s >= slot_idx - ACTIVITY_SLOTS);
                        // What everyone else's timing says about ours.
                        if self.clock.observe(&decodes) {
                            self.status_dirty = true;
                        }
                        // Advance the QSO from anything addressed to us.
                        if self.qso.on_rx(&decodes, slot_utc) {
                            self.status_dirty = true;
                        }
                        // Hound: the Fox answered, so finish the contact on its
                        // frequency instead of up in the calling zone. The one
                        // move into the Fox's half that is not a mistake.
                        if let Some(hz) = self.qso.take_qsy() {
                            self.tune_audio_hz(hz);
                        }
                        // Keep our transmit slot opposite to the DX's most recent
                        // transmission, so replies stay out of their slot even if
                        // they shift the even/odd sequence mid-QSO.
                        if let Some(dx) = self.qso.dx_call().map(str::to_string) {
                            if let Some(&slot) = self.last_heard.get(&dx) {
                                self.tx_even = !self.scheduler.is_even(slot);
                            }
                        }
                        actions.push(DigiAction::Decodes(decodes));
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        // 2. Slot boundary: dispatch the just-completed slot for decode.
        let idx = self.scheduler.slot_index(now);
        if idx != self.last_slot_idx {
            if self.last_slot_idx != i64::MIN {
                self.check_slot_arrived_whole();
                let min_samples = (self.params.slot_s * DECODE_RATE * 0.5) as usize;
                if self.slot_buf.len() >= min_samples {
                    let audio = std::mem::take(&mut self.slot_buf);
                    let slot_idx = self.last_slot_idx;
                    let slot_utc = self.scheduler.slot_start_unix(slot_idx) as i64;
                    let ap = ApHints {
                        my_call: self.qso.my_call().to_string(),
                        dx_call: self.qso.dx_call().map(str::to_string),
                        eu_vhf: self.qso.contest_selected(),
                    };
                    let _ = self.job_tx.send(DecodeJob {
                        audio,
                        slot_idx,
                        slot_utc,
                        ap,
                        audio_hz: self.audio_hz,
                    });
                }
            }
            self.slot_buf.clear();
            self.last_slot_idx = idx;
        }

        // 3. Transmit scheduling: when it's our period and we're past the TX
        // offset, synthesize the burst and ask the engine to key.
        if self.burst.is_none()
            && idx != self.tx_fired_slot
            && self.qso.wants_tx()
            && self.scheduler.is_even(idx) == self.tx_even
        {
            let into = self.scheduler.secs_into_slot(now);
            // Start any time from the nominal offset up to the last moment our
            // burst would end as the DX's next slot begins (`slot_s - burst_s +
            // tx_offset`). So a reply pressed well after our slot has begun still
            // goes out *this* slot instead of waiting a full cycle: our transmit
            // finishes exactly as the DX starts theirs — we don't run into their
            // slot, and we still hear their next transmission in full. The far
            // end loses at most `tx_offset` of our burst tail, which its FEC
            // easily rides out. (Being our opposite slot, this is never the DX's.)
            let latest = self.params.slot_s - self.params.burst_s + self.params.tx_offset_s;
            if into >= self.params.tx_offset_s && into <= latest {
                let msgs = self.qso.plan_tx_all();
                if let Some((samples, sent)) = self.synth_burst_48k(&msgs) {
                    self.burst = Some(BurstPlayer { samples, pos: 0 });
                    self.tx_fired_slot = idx;
                    for m in &sent {
                        self.qso.record_tx(m);
                    }
                    self.status_dirty = true;
                    actions.push(DigiAction::KeyTx);
                }
            }
        }

        // 4. Completed QSOs → log them (fill freq/band from the dial). A Fox can
        // finish several in one transmission, so drain rather than take one.
        while let Some(mut rec) = self.qso.take_completed() {
            rec.freq_hz = self.dial_hz + self.audio_hz as f64;
            rec.band = adif_band(rec.freq_hz).to_string();
            actions.push(DigiAction::QsoLogged(rec));
            self.status_dirty = true;
        }

        // 4. Emit a status update if anything changed.
        if self.status_dirty {
            self.status_dirty = false;
            actions.push(DigiAction::Status(self.status()));
        }

        actions
    }

    pub fn status(&self) -> DigiStatus {
        let mut s = self.qso.status(self.tx_burst_active());
        s.audio_hz = self.audio_hz;
        // The period we will actually transmit in, which is not always the
        // configured one: answering a station takes the slot opposite theirs,
        // whatever the operator set for calling CQ. The readout said otherwise,
        // so an operator whose reply went out in the wrong period was told it
        // had gone out in the right one.
        s.tx_even = self.tx_even;
        s.clock_offset_s = self.clock.offset_s();
        s
    }
}

impl crate::DigiEngine for DigiController {
    /// Straight to `tune_audio_hz`, deliberately bypassing the hold that
    /// [`set_audio_hz`](DigiController::set_audio_hz) enforces. See the trait
    /// method for why a band change is the exception.
    fn restore_audio_hz(&mut self, hz: f32) {
        self.tune_audio_hz(hz);
    }
    fn mode(&self) -> Mode {
        DigiController::mode(self)
    }
    fn on_rx_audio(&mut self, tap: &[f32]) {
        DigiController::on_rx_audio(self, tap)
    }
    fn poll(&mut self, now: SystemTime, dial_hz: f64) -> Vec<DigiAction> {
        DigiController::poll(self, now, dial_hz)
    }
    fn tx_burst_active(&self) -> bool {
        DigiController::tx_burst_active(self)
    }
    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        DigiController::fill_tx_block(self, out)
    }
    fn on_burst_done(&mut self) {
        DigiController::on_burst_done(self)
    }
    fn abort(&mut self) {
        DigiController::abort(self)
    }
    fn abort_tx(&mut self) {
        DigiController::abort_tx(self)
    }
    fn set_config(&mut self, cfg: DigiConfig) {
        DigiController::set_config(self, cfg)
    }
    fn set_audio_hz(&mut self, hz: f32) {
        DigiController::set_audio_hz(self, hz)
    }
    fn audio_hz(&self) -> f32 {
        DigiController::audio_hz(self)
    }
    fn status(&self) -> DigiStatus {
        DigiController::status(self)
    }
    fn call_cq(&mut self) {
        DigiController::call_cq(self)
    }
    fn start_qso(
        &mut self,
        from: String,
        grid: Option<String>,
        snr: i16,
        audio_hz: f32,
        wait_for_cq: bool,
    ) {
        DigiController::start_qso(self, from, grid, snr, audio_hz, wait_for_cq)
    }
    fn stop_qso(&mut self) {
        DigiController::stop_qso(self)
    }
    fn set_step(&mut self, step: sdroxide_types::QsoStep) {
        DigiController::set_step(self, step)
    }
    fn send_text(&mut self, text: String) {
        DigiController::send_text(self, text)
    }
    fn queue_add(&mut self, entry: sdroxide_types::QueuedCall) {
        DigiController::queue_add(self, entry)
    }
    fn queue_remove(&mut self, call: &str) {
        DigiController::queue_remove(self, call)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn cfg() -> DigiConfig {
        DigiConfig {
            my_call: "AB1CD".into(),
            my_grid: "FN42".into(),
            tx_even: true,
            ..Default::default()
        }
    }

    #[test]
    fn call_cq_keys_an_aligned_burst() {
        let mut c = DigiController::new(Mode::Ft8, cfg(), 12_000.0);
        c.call_cq();

        // A time 1 s into an even 15 s slot (past the 0.5 s TX offset).
        // 1_609_459_200 / 15 = 107_297_280 (even).
        let now = UNIX_EPOCH + Duration::from_secs_f64(1_609_459_201.0);
        let actions = c.poll(now, 14_074_000.0);

        assert!(
            actions.iter().any(|a| matches!(a, DigiAction::KeyTx)),
            "expected KeyTx, got {actions:?}"
        );
        assert!(c.tx_burst_active(), "burst should be loaded");

        // The burst plays out non-silent audio.
        let mut block = [0.0f32; 480];
        let mut any_signal = false;
        for _ in 0..50 {
            c.fill_tx_block(&mut block);
            if block.iter().any(|s| s.abs() > 0.01) {
                any_signal = true;
                break;
            }
        }
        assert!(any_signal, "burst produced only silence");
    }

    /// The headroom this mode declares has to be the headroom it actually
    /// leaves: the transmit chain divides `tx_peak` back out to put the over on
    /// the air at full scale, so a burst quieter than it claims is transmitted
    /// quiet and one louder than it claims is transmitted into the limiter.
    /// See [`crate::DigiEngine::tx_peak`] and issue #131.
    #[test]
    fn the_burst_is_as_loud_as_the_headroom_it_declares() {
        use crate::DigiEngine;

        let mut c = DigiController::new(Mode::Ft8, cfg(), 12_000.0);
        c.call_cq();
        let now = UNIX_EPOCH + Duration::from_secs_f64(1_609_459_201.0);
        c.poll(now, 14_074_000.0);
        assert!(c.tx_burst_active(), "no burst to measure");

        // A second of audio: long enough to be past the burst's opening ramp
        // and well into the tones themselves.
        let mut block = [0.0f32; 480];
        let mut peak = 0.0f32;
        for _ in 0..100 {
            DigiController::fill_tx_block(&mut c, &mut block);
            peak = block.iter().fold(peak, |a, s| a.max(s.abs()));
        }
        let declared = DigiEngine::tx_peak(&c);
        assert!(
            (peak - declared).abs() < 0.02,
            "the burst peaks at {peak} against the {declared} it declares"
        );
    }

    #[test]
    fn no_burst_without_a_callsign() {
        let mut c = DigiController::new(Mode::Ft8, DigiConfig::default(), 12_000.0);
        c.call_cq();
        let now = UNIX_EPOCH + Duration::from_secs_f64(1_609_459_201.0);
        let actions = c.poll(now, 14_074_000.0);
        assert!(!actions.iter().any(|a| matches!(a, DigiAction::KeyTx)));
        assert!(!c.tx_burst_active());
    }

    // 1_609_459_200 / 15 = 107_297_280, an even slot index.
    const EVEN_SLOT_UNIX: f64 = 1_609_459_200.0;
    const EVEN_SLOT_IDX: i64 = 107_297_280;

    #[test]
    fn reply_targets_slot_opposite_the_dx() {
        let mut c = DigiController::new(Mode::Ft8, cfg(), 12_000.0);
        // We heard the DX in an even slot → we must reply in odd slots.
        c.last_heard.insert("W9XYZ".into(), EVEN_SLOT_IDX);
        c.start_qso("W9XYZ".into(), Some("EM48".into()), -10, 1500.0, false);
        assert!(!c.tx_even, "reply slot should be odd (opposite the even DX slot)");

        // Keying in our (odd) slot works even 2.5 s in — well past where the
        // whole burst still fits (~2.36 s for FT8), and past the old ~2.06 s
        // window. The point is it fires *this* slot, not a full cycle later.
        let our_odd = UNIX_EPOCH + Duration::from_secs_f64(EVEN_SLOT_UNIX + 15.0 + 2.5);
        let actions = c.poll(our_odd, 14_074_000.0);
        assert!(
            actions.iter().any(|a| matches!(a, DigiAction::KeyTx)),
            "should key late in our opposite slot, got {actions:?}"
        );
    }

    #[test]
    fn the_chooser_stays_inside_what_the_band_plan_allows() {
        // UK 60 m in the shape the chooser sees it: on a 5357 kHz dial the
        // allocation ends at 5358.0, so an FT8 signal may start no higher than
        // 950 Hz, while the search runs to 2600. The real predicate reads the
        // band plan; here it is handed in directly, so this pins the CHOOSER
        // and does not depend on which bandplan.json happens to be installed,
        // nor touch the process-wide plan that other tests are reading.
        let ceiling = 950.0;

        // Crowded right up to the edge of what is legal. The quiet space above
        // is the obvious choice and the illegal one.
        let busy: Vec<f32> = (0..8).map(|i| 420.0 + i as f32 * 60.0).collect();
        let hz = clearest_tx_hz(&busy, 1500.0, |hz| hz <= ceiling);
        assert!(hz <= ceiling, "picked {hz} Hz, which is past the band edge");
        assert!(hz >= TX_PICK_MIN_HZ, "picked {hz} Hz, below the search floor");

        // Nothing legal anywhere: it stays put rather than jumping to an
        // arbitrary edge. Declining to choose is the safe answer, and the
        // transmit lockout is still there to refuse what it was already on.
        assert_eq!(clearest_tx_hz(&[], 700.0, |_| false), 700.0);
    }

    #[test]
    fn the_transmit_frequency_goes_where_our_own_period_is_quiet() {
        // A crowded stretch below 1200 Hz and a clear one above it.
        let busy: Vec<f32> = (0..12).map(|i| 500.0 + i as f32 * 60.0).collect();
        let hz = clearest_tx_hz(&busy, 1500.0, |_| true);
        assert!(hz > 1220.0, "picked {hz} Hz, inside the crowd");
        assert!(busy.iter().all(|b| (b - hz).abs() >= 60.0), "picked {hz} Hz, on top of a station");

        // With the band empty it stays where it already is rather than
        // wandering off to an arbitrary edge.
        assert_eq!(clearest_tx_hz(&[], 1500.0, |_| true), 1500.0);
        // One station in the middle: it moves clear, but no further than it has
        // to — a spot 120 Hz away is as good as one 900 Hz away.
        let hz = clearest_tx_hz(&[1500.0], 1500.0, |_| true);
        assert!((hz - 1500.0).abs() >= CLEAR_ENOUGH_HZ, "{hz} is still on top of them");
        assert!(
            (hz - 1500.0).abs() <= CLEAR_ENOUGH_HZ + TX_PICK_STEP_HZ,
            "{hz} is a needless jump"
        );
    }

    #[test]
    fn answering_a_station_does_not_move_us_onto_them() {
        let mut c = DigiController::new(Mode::Ft8, cfg(), 12_000.0);
        c.set_audio_hz(1500.0);
        // Auto TX FRQ is on by default: their frequency is not ours to take.
        c.start_qso("W9XYZ".into(), None, -10, 800.0, false);
        assert_ne!(c.audio_hz(), 800.0);

        // Turned off, the operator's own choice stands — including following
        // the station they are answering.
        c.set_config(DigiConfig { auto_tx_freq: false, ..cfg() });
        c.start_qso("K1ABC".into(), None, -10, 800.0, false);
        assert_eq!(c.audio_hz(), 800.0);
    }

    #[test]
    fn holding_the_transmit_frequency_stops_every_mover() {
        // The case this exists for: UK 60 m, where the allocation ends 1 kHz
        // above a 5357 kHz dial and either automatic mover walks out of it.
        let mut c =
            DigiController::new(Mode::Ft8, DigiConfig { hold_tx_freq: true, ..cfg() }, 12_000.0);
        // Placed by the sequencer's own route, which is what the operator's
        // click becomes once the hold is lifted.
        c.tune_audio_hz(820.0);

        // 1. The operator's own set is refused while held — no modifier-key
        //    escape, unlike WSJT-X's Hold Tx Freq.
        c.set_audio_hz(2400.0);
        assert_eq!(c.audio_hz(), 820.0, "an operator click moved a held frequency");

        // 2. Answering a station does not follow it, with Auto TX FRQ either way.
        c.start_qso("W9XYZ".into(), None, -10, 2400.0, false);
        assert_eq!(c.audio_hz(), 820.0, "answering moved a held frequency");
        c.set_config(DigiConfig { hold_tx_freq: true, auto_tx_freq: false, ..cfg() });
        c.start_qso("K1ABC".into(), None, -10, 2400.0, false);
        assert_eq!(c.audio_hz(), 820.0, "follow-the-DX moved a held frequency");

        // 3. Calling CQ does not hunt for a clear slot. `pick_tx_freq` would
        //    otherwise roam 400..2600 Hz, most of which is out of band here.
        c.set_config(DigiConfig { hold_tx_freq: true, ..cfg() });
        c.call_cq();
        assert_eq!(c.audio_hz(), 820.0, "calling CQ moved a held frequency");

        // Lifting it hands control back, and the operator's click lands.
        c.set_config(DigiConfig { hold_tx_freq: false, ..cfg() });
        c.set_audio_hz(2400.0);
        assert_eq!(c.audio_hz(), 2400.0, "lifting the hold did not release the frequency");
    }

    #[test]
    fn a_band_change_reaches_a_held_frequency() {
        use crate::DigiEngine as _;
        // The second exception, and the reason it is one: a hold pins the tone
        // against everything that would move it by itself, but a change of band
        // is the operator's own act, and what comes back is a figure they set
        // on that band themselves. Holding through it would carry 60 m's
        // sub-1 kHz figure onto 20 m, or 20 m's 1500 onto 60 m, where it cannot
        // legally go.
        let mut c =
            DigiController::new(Mode::Ft8, DigiConfig { hold_tx_freq: true, ..cfg() }, 12_000.0);
        c.tune_audio_hz(820.0);

        // The ordinary route is still refused, so the hold is genuinely on and
        // the restore below is not passing for the trivial reason.
        c.set_audio_hz(1500.0);
        assert_eq!(c.audio_hz(), 820.0, "an operator click moved a held frequency");

        // The band-memory route goes through it.
        c.restore_audio_hz(1500.0);
        assert_eq!(c.audio_hz(), 1500.0, "a band change did not reach a held frequency");
    }

    #[test]
    fn a_held_hound_still_follows_the_fox() {
        use sdroxide_types::DxpedMode;
        // The one move a hold does not block: a Fox that has answered us owns
        // the frequency the contact finishes on, and it is not ours to pin.
        let mut c = DigiController::new(
            Mode::Ft8,
            DigiConfig { hold_tx_freq: true, dxped_mode: DxpedMode::Hound, ..cfg() },
            12_000.0,
        );
        c.tune_audio_hz(1800.0);
        c.start_qso("DX1FOX".into(), None, -10, 700.0, false);
        assert_eq!(c.audio_hz(), 1800.0, "calling frequency should be held");
        c.tune_audio_hz(700.0);
        assert_eq!(c.audio_hz(), 700.0, "the Fox's QSY is exempt");
    }

    #[test]
    fn a_hound_is_kept_out_of_the_fox_zone() {
        use sdroxide_types::{DxpedMode, FOX_ZONE_MAX_HZ};
        let mut c = DigiController::new(
            Mode::Ft8,
            DigiConfig { dxped_mode: DxpedMode::Hound, ..cfg() },
            12_000.0,
        );
        // Tuning down among the Fox's own signals is refused.
        c.set_audio_hz(600.0);
        assert_eq!(c.audio_hz(), FOX_ZONE_MAX_HZ);
        c.set_audio_hz(1800.0);
        assert_eq!(c.audio_hz(), 1800.0, "the calling zone is free");

        // Answering a Fox heard at 700 Hz keeps our calling frequency; only the
        // Fox's own reply moves us down onto it.
        c.start_qso("DX1FOX".into(), None, -10, 700.0, false);
        assert_eq!(c.audio_hz(), 1800.0);
        c.tune_audio_hz(700.0);
        assert_eq!(c.audio_hz(), 700.0);

        // Outside Hound mode the whole passband is the operator's.
        let mut c = DigiController::new(Mode::Ft8, cfg(), 12_000.0);
        c.set_audio_hz(600.0);
        assert_eq!(c.audio_hz(), 600.0);
    }

    #[test]
    fn reply_never_keys_in_the_dx_slot() {
        // Simulate pressing reply late — during a slot with the *same* parity as
        // the DX (the old code would then transmit right on top of them).
        let mut c = DigiController::new(Mode::Ft8, cfg(), 12_000.0);
        c.last_heard.insert("W9XYZ".into(), EVEN_SLOT_IDX);
        c.start_qso("W9XYZ".into(), Some("EM48".into()), -10, 1500.0, false);

        // A later even slot (the DX's parity): must NOT key here.
        let dx_slot = UNIX_EPOCH + Duration::from_secs_f64(EVEN_SLOT_UNIX + 30.0 + 1.0);
        let actions = c.poll(dx_slot, 14_074_000.0);
        assert!(
            !actions.iter().any(|a| matches!(a, DigiAction::KeyTx)),
            "keyed in the DX's own slot: {actions:?}"
        );
        assert!(!c.tx_burst_active());
    }
}
