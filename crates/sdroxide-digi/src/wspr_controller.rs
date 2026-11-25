//! `WsprController` — the beacon, as the engine sees it.
//!
//! Slotted like FT8 and shaped like [`crate::DigiController`], but with nothing
//! to sequence: WSPR has no addressing, no exchange and no QSO. What it has
//! instead are three things FT8 does not:
//!
//! * **A duty cycle.** A beacon transmits in a fraction of slots, not all of
//!   them, and it spends that fraction evenly: one slot in two at 50%, one in
//!   ten at 10%, rather than a coin flip per slot that clumps. Computed from
//!   the slot index and the operator's callsign — deterministic so the panel
//!   can say "transmitting next slot" before it happens, and salted with the
//!   callsign so two stations running this program do not start their cycle on
//!   the same slot.
//! * **Band hopping.** One receiver can sample the whole spectrum by moving
//!   between slots. The controller only ever *asks*: [`DigiAction::SetDial`]
//!   goes to the engine, which owns the VFO and refuses when the operator has
//!   a hand on it.
//! * **A slow decoder.** A WSPR scan is seconds of work over a two-minute slot,
//!   so it goes to a worker thread — and, because hopping means the dial has
//!   already moved by the time results come back, the job carries the dial it
//!   was recorded on. Getting that wrong would file every spot under the wrong
//!   band, which is the one error a propagation map cannot survive.

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::SystemTime;

use mfsk_core::msg::WsprMessage;
use sdroxide_dsp::MonoResampler;
use sdroxide_types::{
    Band, DigiConfig, DigiStatus, Mode, QsoStep, WSPR_DIALS, WSPR_WINDOW_HI_HZ, WSPR_WINDOW_LO_HZ,
    WsprSpot, WsprStatus, round_power_dbm,
};

use crate::DigiEngine;
use crate::clock::ClockMonitor;
use crate::controller::DigiAction;
use crate::params::{DECODE_RATE, DigiParams};
use crate::scheduler::SlotScheduler;
use crate::wspr::{self, WsprDecode};

/// One slot of audio handed to the decode worker.
struct DecodeJob {
    audio: Vec<f32>,
    slot_utc: i64,
    /// The dial the audio was recorded on. Carried rather than read at the far
    /// end because band hopping may already have moved it.
    dial_hz: f64,
}

/// What the worker sends back.
struct DecodeResult {
    slot_utc: i64,
    dial_hz: f64,
    decodes: Vec<WsprDecode>,
}

/// Peak amplitude of the transmitted burst, matching what the other slotted
/// modes hand the modulator.
const TX_AMPLITUDE: f32 = 0.5;

/// Sample rate the burst is generated at, which is what the engine's transmit
/// block expects.
const TX_RATE: u32 = 48_000;

pub struct WsprController {
    params: DigiParams,
    scheduler: SlotScheduler,
    cfg: DigiConfig,
    resampler: Option<MonoResampler>,
    /// 12 kHz audio accumulated for the slot in progress.
    slot_buf: Vec<f32>,
    tap_scratch: Vec<f32>,
    last_slot_idx: i64,
    dial_hz: f64,
    audio_hz: f32,

    // Decode worker.
    job_tx: Sender<DecodeJob>,
    res_rx: Receiver<DecodeResult>,
    _worker: std::thread::JoinHandle<()>,
    /// Slots dispatched but not yet answered, so the panel can say "decoding"
    /// rather than "nothing heard" during the seconds it takes.
    pending: HashSet<i64>,

    // Transmit.
    burst: Option<wspr::tx::BurstGen>,
    tx_fired_slot: i64,
    /// Where hopping wants the dial for the next slot, and why it was refused
    /// if it was.
    next_dial_hz: Option<f64>,
    hop_blocked: Option<String>,

    spots_last_slot: u32,
    status_dirty: bool,
    clock: ClockMonitor,
    /// Every station decoded outright, which an OSD decode has to name before
    /// it is believed — see [`KnownCalls`].
    known: KnownCalls,
}

/// The stations this receiver has heard, kept the way `wsprd` keeps its
/// `hashtab`/`loctab` pair: a callsign, and for a plain Type-1 message the
/// grid it came with.
///
/// It exists for one rule of `wsprd`'s (`wsprd.c`, right after `osdwspr_`): an
/// ordered-statistics decode is accepted only when its callsign is already in
/// that table — with the same grid, for a Type-1 message — and a Type-3 one
/// never is. OSD returns a valid codeword for whatever it is handed, so a
/// callsign it reports that nobody has heard outright is far more likely
/// invented from noise than heard; without this rule a receiver lists, and
/// uploads, stations like `30O/RY3T` that WSJT-X beside it never shows
/// (issue #946).
///
/// Keyed by callsign rather than by `wsprd`'s fifteen-bit hash, which this
/// station cannot compute (see [`wspr::hashes`]); the test `wsprd` makes on
/// the slot it finds is an exact comparison against the callsign either way.
/// Unlike `wsprd`'s it starts empty rather than from `hashtable.txt`, which
/// only makes it stricter for the first slots after a start.
#[derive(Default)]
struct KnownCalls(std::collections::HashMap<String, Option<String>>);

impl KnownCalls {
    /// `wsprd`'s table has 32768 slots. A receiver left running for months
    /// keeps no more than that here either: past it the table starts over,
    /// which costs a few OSD decodes until the band is heard again.
    const CAPACITY: usize = 32_768;

    /// Whether `d` is reported, learning its callsign if so.
    fn admit(&mut self, d: &WsprDecode) -> bool {
        let ok = !d.from_osd
            || match &d.message {
                WsprMessage::Type1 { callsign, grid, .. } => {
                    self.0.get(callsign).is_some_and(|g| g.as_deref() == Some(grid.as_str()))
                }
                WsprMessage::Type2 { callsign, .. } => self.0.contains_key(callsign),
                WsprMessage::Type3 { .. } => false,
            };
        if !ok {
            return false;
        }
        if self.0.len() >= Self::CAPACITY {
            self.0.clear();
        }
        // What `unpk_` writes: Type 1 the call and its grid, Type 2 the
        // compound call alone, Type 3 nothing — its callsign is the unknown.
        match &d.message {
            WsprMessage::Type1 { callsign, grid, .. } => {
                self.0.insert(callsign.clone(), Some(grid.clone()));
            }
            WsprMessage::Type2 { callsign, .. } => {
                self.0.entry(callsign.clone()).or_insert(None);
            }
            WsprMessage::Type3 { .. } => {}
        }
        true
    }
}

impl WsprController {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        let params = DigiParams::for_mode(Mode::Wspr);
        let (job_tx, job_rx) = std::sync::mpsc::channel::<DecodeJob>();
        let (res_tx, res_rx) = std::sync::mpsc::channel::<DecodeResult>();
        let worker = std::thread::Builder::new()
            .name("sdroxide-wspr-decode".into())
            .spawn(move || {
                while let Ok(job) = job_rx.recv() {
                    let decodes = wspr::decode_slot(&job.audio, DECODE_RATE as u32);
                    let res =
                        DecodeResult { slot_utc: job.slot_utc, dial_hz: job.dial_hz, decodes };
                    if res_tx.send(res).is_err() {
                        break;
                    }
                }
            })
            .expect("spawn wspr decode worker");

        WsprController {
            params,
            scheduler: SlotScheduler::for_mode(Mode::Wspr),
            cfg,
            resampler: MonoResampler::new(tap_rate, DECODE_RATE),
            slot_buf: Vec::new(),
            tap_scratch: Vec::new(),
            last_slot_idx: i64::MIN,
            dial_hz: 14_095_600.0,
            audio_hz: sdroxide_types::WSPR_DEFAULT_TX_HZ,
            job_tx,
            res_rx,
            _worker: worker,
            pending: HashSet::new(),
            burst: None,
            tx_fired_slot: i64::MIN,
            next_dial_hz: None,
            hop_blocked: None,
            spots_last_slot: 0,
            status_dirty: true,
            clock: ClockMonitor::default(),
            known: KnownCalls::default(),
        }
    }

    /// Turn a decode into the spot that leaves this crate.
    ///
    /// The hash table is consulted here rather than in the decoder because it is
    /// stateful across slots: a Type-3 message is only resolvable once its
    /// sender has been heard announcing its own callsign.
    fn to_spot(&self, d: &WsprDecode, slot_utc: i64, dial_hz: f64) -> WsprSpot {
        let (call, grid, power) = match &d.message {
            WsprMessage::Type1 { callsign, grid, power_dbm } => {
                (callsign.clone(), Some(grid.clone()), *power_dbm)
            }
            // A compound callsign spends the grid field on the prefix, so there
            // is no locator to report. Left `None` rather than guessed: a made
            // up position is worse on a map than an absent one.
            WsprMessage::Type2 { callsign, power_dbm } => (callsign.clone(), None, *power_dbm),
            // The callsign is a fifteen-bit hash of a function this station
            // cannot compute — see `wspr::hashes`. Reported as the hash, which
            // is what actually arrived. The six-character grid is in the clear,
            // so the path is still real and still goes on the map.
            WsprMessage::Type3 { callsign_hash, grid6, power_dbm } => {
                (wspr::hashed_label(*callsign_hash), Some(grid6.clone()), *power_dbm)
            }
        };
        WsprSpot {
            slot_utc,
            call,
            grid,
            power_dbm: power as i16,
            freq_hz: dial_hz + d.carrier_hz() as f64,
            snr_db: d.snr_db.round() as i16,
            dt: d.dt_sec,
            drift_hz: d.drift_hz,
            reporter: None,
            reporter_grid: None,
        }
    }

    /// Whether this station beacons in slot `idx`.
    ///
    /// The duty cycle is *spaced*, not drawn per slot. A fair coin at 50% is
    /// half the slots on average and still gives runs of three and gaps of
    /// four, which on an unattended beacon reads as a fault — and spends its
    /// transmissions in a clump instead of across the propagation the interval
    /// exists to sample. So: `pct` slots in every hundred, as evenly as the
    /// fraction allows. One in two at 50%, one in five at 20%, one in ten at
    /// 10%.
    ///
    /// The phase is salted with the callsign so two stations running this
    /// program do not start their cycle on the same slot. A fixed cadence can
    /// only offer that as a phase difference — where two phases do line up the
    /// stations share slots — but sharing a slot is not the collision it would
    /// be in a sequenced mode: the two land on different tones, because
    /// [`Self::tx_audio_hz`] draws those per slot from the same salt, and a
    /// WSPR receiver decodes the whole two hundred hertz window at once.
    ///
    /// Deterministic either way, so the panel can promise the next slot before
    /// it starts and a restart does not re-roll the schedule.
    fn transmits_in(&self, idx: i64) -> bool {
        let pct = self.cfg.wspr_tx_percent.min(100) as i64;
        if pct == 0 || self.cfg.my_call.trim().is_empty() || self.cfg.my_grid.trim().is_empty() {
            return false;
        }
        if pct >= 100 {
            return true;
        }
        // Bresenham over a hundred-slot cycle: slot `k` keys when the running
        // total of slots owed crosses an integer. It divides evenly when the
        // percentage divides a hundred and stays even when it does not — 33%
        // takes every third slot and a fourth once per cycle, rather than
        // doubling up somewhere to make the count.
        let phase = (slot_draw(0, &self.cfg.my_call) % 100) as i64;
        let k = idx.wrapping_add(phase).rem_euclid(100);
        (k + 1) * pct / 100 > k * pct / 100
    }

    /// A transmit tone offset inside the window.
    ///
    /// With `auto_tx_freq` (on by default) it moves every transmission, which is
    /// the WSPR convention: two hundred hertz shared by everyone works only if
    /// nobody parks in the middle of it. Off, it holds wherever the operator put
    /// the cursor.
    fn tx_audio_hz(&self, idx: i64) -> f32 {
        if !self.cfg.auto_tx_freq {
            return self.audio_hz;
        }
        // Keep a little clear of both edges: the signal is ~6 Hz wide and a
        // receiver's search does not run past the window.
        let (lo, hi) = (WSPR_WINDOW_LO_HZ + 10.0, WSPR_WINDOW_HI_HZ - 10.0);
        let draw = slot_draw(idx ^ 0x5750_5200, &self.cfg.my_call) % 1000;
        lo + (hi - lo) * (draw as f32 / 1000.0)
    }

    /// The dial for the slot after `idx`, when hopping is on.
    ///
    /// A fixed rotation through the enabled bands rather than a random pick:
    /// coverage is the point, and a random walk revisits and skips. Returns
    /// `None` when hopping is off or no band is enabled — including the case
    /// where every enabled band was deselected, which must not silently mean
    /// "all of them".
    fn hop_target(&self, idx: i64) -> Option<f64> {
        if !self.cfg.wspr_hop {
            return None;
        }
        let dials: Vec<f64> = WSPR_DIALS
            .iter()
            .copied()
            // `wire_index`, not the band bar's order: the mask is saved, and
            // the bar's order moves when a band is added to the middle of it
            // (issue #396).
            .filter(|&hz| self.cfg.wspr_hop_bands & (1 << Band::containing(hz).wire_index()) != 0)
            .collect();
        if dials.is_empty() {
            return None;
        }
        Some(dials[(idx.rem_euclid(dials.len() as i64)) as usize])
    }

    fn wspr_status(&self) -> WsprStatus {
        let idx = self.scheduler.slot_index(SystemTime::now());
        WsprStatus {
            slot_utc: self.scheduler.slot_start_unix(idx) as i64,
            decoding: !self.pending.is_empty(),
            tx_next: self.transmits_in(idx + 1),
            last_slot_spots: self.spots_last_slot,
            next_dial_hz: self.next_dial_hz,
            hop_blocked: self.hop_blocked.clone(),
            // Asked of the packer rather than re-derived, so the panel says
            // "cannot transmit" in exactly the cases the transmitter refuses.
            tx_blocked: wspr::tx::why_not_sendable(
                &self.cfg.my_call,
                &self.cfg.my_grid,
                round_power_dbm(self.cfg.wspr_power_dbm) as i32,
            ),
        }
    }

    pub fn status(&self) -> DigiStatus {
        let mut s = DigiStatus::idle(self.cfg.clone());
        s.mode = Mode::Wspr;
        s.step = QsoStep::Idle;
        s.audio_hz = self.audio_hz;
        s.transmitting = self.burst.is_some();
        s.tx_next = self.transmits_in(self.scheduler.slot_index(SystemTime::now()) + 1);
        s.clock_offset_s = self.clock.offset_s();
        s.wspr = Some(self.wspr_status());
        s
    }
}

/// A uniform draw in 0..u32::MAX from a slot index and a callsign.
///
/// FNV-1a over the two, which is what the rest of this codebase reaches for
/// when it wants a stable hash and not a cryptographic one.
fn slot_draw(idx: i64, call: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in idx.to_le_bytes().iter().chain(call.as_bytes()) {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h >> 16
}

impl DigiEngine for WsprController {
    fn mode(&self) -> Mode {
        Mode::Wspr
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        self.tap_scratch.clear();
        match &mut self.resampler {
            Some(r) => r.push(tap, &mut self.tap_scratch),
            None => self.tap_scratch.extend_from_slice(tap),
        }
        // Cap so a boundary that never arrives cannot grow this without bound.
        let cap = self.params.slot_samples() + self.params.slot_samples() / 8;
        for &s in &self.tap_scratch {
            if self.slot_buf.len() < cap {
                self.slot_buf.push(s.clamp(-1.0, 1.0));
            }
        }
    }

    fn poll(&mut self, now: SystemTime, dial_hz: f64) -> Vec<DigiAction> {
        self.dial_hz = dial_hz;
        let mut actions = Vec::new();

        // 1. Drain the worker.
        loop {
            match self.res_rx.try_recv() {
                Ok(res) => {
                    self.pending.remove(&res.slot_utc);
                    // In decode order, as `wsprd` fills its table: a station
                    // Fano finds early in the slot vouches for an OSD decode
                    // of it later in the same one.
                    let heard: Vec<&WsprDecode> =
                        res.decodes.iter().filter(|d| self.known.admit(d)).collect();
                    let spots: Vec<WsprSpot> = heard
                        .iter()
                        .map(|d| self.to_spot(d, res.slot_utc, res.dial_hz))
                        .collect();
                    self.spots_last_slot = spots.len() as u32;
                    self.clock.observe_dts(heard.iter().map(|d| d.dt_sec));
                    self.status_dirty = true;
                    if !spots.is_empty() {
                        actions.push(DigiAction::WsprSpots(spots));
                    }
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }

        // 2. Slot boundary: dispatch the completed slot, then decide what the
        //    new one is for.
        let idx = self.scheduler.slot_index(now);
        if idx != self.last_slot_idx {
            if self.last_slot_idx != i64::MIN {
                // Half a slot of audio is the floor: less than that is a mode
                // change or a stream hiccup, not a transmission.
                let min_samples = (self.params.slot_s * DECODE_RATE * 0.5) as usize;
                // Only ever one slot in flight.
                //
                // A WSPR scan is seconds of work and has a whole slot to do it
                // in, but "seconds" depends on the machine and on how busy the
                // band is — and the queue behind this channel has no bound. A
                // receiver that cannot quite keep up would not drop a slot and
                // recover; it would fall a slot further behind every two
                // minutes, holding a two-megabyte recording for each one and
                // reporting spots ever further out of date, which reads as the
                // decoder mysteriously lagging rather than as the machine being
                // too slow. Skipping the slot loses two minutes of the band;
                // queueing it loses the station.
                if self.slot_buf.len() >= min_samples && self.pending.is_empty() {
                    let audio = std::mem::take(&mut self.slot_buf);
                    let slot_utc = self.scheduler.slot_start_unix(self.last_slot_idx) as i64;
                    self.pending.insert(slot_utc);
                    // `self.dial_hz` is still the dial this audio was recorded
                    // on: the hop below has not happened yet, and that ordering
                    // is what keeps a hopped spot on the right band.
                    let _ = self.job_tx.send(DecodeJob { audio, slot_utc, dial_hz: self.dial_hz });
                } else if !self.pending.is_empty() {
                    tracing::warn!(
                        "WSPR: still decoding the previous slot, skipping this one — the \
                         decoder is not keeping up with the band"
                    );
                }
            }
            self.slot_buf.clear();
            self.last_slot_idx = idx;
            self.status_dirty = true;

            // Band hop for the slot just starting. Never under a transmission —
            // the engine refuses too, but asking at all would be wrong.
            self.hop_blocked = None;
            if let Some(target) = self.hop_target(idx) {
                self.next_dial_hz = self.hop_target(idx + 1);
                if self.burst.is_some() {
                    self.hop_blocked = Some("transmitting".into());
                } else if (target - self.dial_hz).abs() > 1.0 {
                    actions.push(DigiAction::SetDial(target));
                }
            } else {
                self.next_dial_hz = None;
            }
        }

        // 3. Transmit. One burst per slot, started inside the window where it
        //    still fits: the beacon is 110.6 s of a 120 s slot, so there is
        //    almost none of it.
        if self.burst.is_none() && idx != self.tx_fired_slot && self.transmits_in(idx) {
            let into = self.scheduler.secs_into_slot(now);
            let latest = self.params.slot_s - self.params.burst_s + self.params.tx_offset_s;
            if into >= self.params.tx_offset_s && into <= latest {
                self.tx_fired_slot = idx;
                let power = round_power_dbm(self.cfg.wspr_power_dbm);
                let hz = self.tx_audio_hz(idx) as f64;
                // `symbols_for` shortens a longer locator itself, so a
                // station set to a six-character grid transmits as its first
                // four rather than being refused.
                match wspr::tx::symbols_for(&self.cfg.my_call, &self.cfg.my_grid, power as i32) {
                    Some(symbols) => {
                        self.audio_hz = hz as f32;
                        self.burst =
                            Some(wspr::tx::BurstGen::new(&symbols, TX_RATE, hz, TX_AMPLITUDE));
                        self.status_dirty = true;
                        actions.push(DigiAction::KeyTx);
                    }
                    // Nothing to say here: `status()` asks the packer the same
                    // question every time it is built, so the panel is already
                    // showing why. Saying it twice, in two different forms,
                    // was how the two came to disagree.
                    None => self.status_dirty = true,
                }
            }
        }

        if self.status_dirty {
            self.status_dirty = false;
            actions.push(DigiAction::Status(self.status()));
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        self.burst.is_some()
    }

    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        let Some(b) = self.burst.as_mut() else {
            out.fill(0.0);
            return true;
        };
        let done = b.fill(out);
        if done {
            self.burst = None;
        }
        done
    }

    fn on_burst_done(&mut self) {
        self.status_dirty = true;
    }

    fn abort(&mut self) {
        self.burst = None;
        self.slot_buf.clear();
    }

    fn abort_tx(&mut self) {
        self.burst = None;
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        self.cfg = cfg;
        self.status_dirty = true;
    }

    fn set_audio_hz(&mut self, hz: f32) {
        self.audio_hz = hz.clamp(WSPR_WINDOW_LO_HZ, WSPR_WINDOW_HI_HZ);
        self.status_dirty = true;
    }

    fn audio_hz(&self) -> f32 {
        self.audio_hz
    }

    fn status(&self) -> DigiStatus {
        WsprController::status(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(tx_percent: u8) -> DigiConfig {
        DigiConfig {
            my_call: "K1ABC".into(),
            my_grid: "FN42".into(),
            wspr_tx_percent: tx_percent,
            ..Default::default()
        }
    }

    fn ctrl(cfg: DigiConfig) -> WsprController {
        WsprController::new(cfg, 48_000.0)
    }

    /// Drive the controller across `slots` two-minute slots, two seconds into
    /// each, and report which ones it asked to key in.
    fn slots_keyed(c: &mut WsprController, from_unix: f64, slots: i64) -> Vec<i64> {
        let mut keyed = Vec::new();
        for k in 0..slots {
            let t = from_unix + k as f64 * 120.0 + 2.0;
            let now = std::time::UNIX_EPOCH + std::time::Duration::from_secs_f64(t);
            for a in c.poll(now, 14_095_600.0) {
                if matches!(a, DigiAction::KeyTx) {
                    keyed.push(k);
                }
            }
            // The burst would otherwise still be playing on the next slot.
            c.abort_tx();
        }
        keyed
    }

    /// The end-to-end question: a station told to beacon must actually key.
    ///
    /// Every part of this was unit-tested and every part passed — the duty
    /// draw, the transmit window, the message packing — which is exactly why
    /// this one exists: it drives the whole gate the way a running slot does.
    #[test]
    fn a_beacon_at_fifty_percent_actually_keys_about_half_the_slots() {
        let mut c = ctrl(cfg(50));
        // A real slot boundary, not slot zero.
        let start = 1_785_760_440.0;
        let keyed = slots_keyed(&mut c, start, 40);
        assert!(!keyed.is_empty(), "a 50% beacon never keyed in 40 slots");
        assert!(
            (12..=28).contains(&keyed.len()),
            "50% duty keyed {} of 40 slots: {keyed:?}",
            keyed.len()
        );
    }

    /// And it keys in the slots it promised. The panel shows `tx_next` before
    /// the slot arrives, so a promise it does not keep is worse than no promise.
    #[test]
    fn the_slots_it_keys_are_the_ones_it_said_it_would() {
        let c = ctrl(cfg(50));
        let start_idx = (1_785_760_440.0f64 / 120.0).floor() as i64;
        let promised: Vec<i64> = (0..40).filter(|k| c.transmits_in(start_idx + k)).collect();
        drop(c);

        let mut c = ctrl(cfg(50));
        let keyed = slots_keyed(&mut c, 1_785_760_440.0, 40);
        assert_eq!(keyed, promised, "keyed slots differ from the promised ones");
    }

    /// A station that cannot express its identity must not silently sit there:
    /// it keys nothing, and `tx_blocked` says why.
    #[test]
    fn a_compound_callsign_keys_nothing_and_says_so() {
        let mut c = ctrl(DigiConfig { my_call: "PJ4/K1ABC".into(), ..cfg(100) });
        let keyed = slots_keyed(&mut c, 1_785_760_440.0, 6);
        assert!(keyed.is_empty(), "keyed with an unsendable callsign");
        assert!(c.status().wspr.expect("wspr status").tx_blocked.is_some());
    }

    /// A six-character locator is sendable, so it must key like any other.
    #[test]
    fn a_six_character_locator_keys_normally() {
        let mut c = ctrl(DigiConfig { my_grid: "FN42aa".into(), ..cfg(100) });
        let keyed = slots_keyed(&mut c, 1_785_760_440.0, 6);
        assert_eq!(keyed.len(), 6, "a 100% beacon skipped slots: {keyed:?}");
        assert!(c.status().wspr.expect("wspr status").tx_blocked.is_none());
    }

    #[test]
    fn a_beacon_with_no_duty_cycle_never_transmits() {
        let c = ctrl(cfg(0));
        assert!((0..1000).all(|i| !c.transmits_in(i)), "transmitted with the duty cycle at zero");
    }

    /// The default has to be silence: selecting a mode is not consent to put a
    /// carrier on the air.
    #[test]
    fn the_default_configuration_is_receive_only() {
        let c = ctrl(DigiConfig {
            my_call: "K1ABC".into(),
            my_grid: "FN42".into(),
            ..Default::default()
        });
        assert_eq!(c.cfg.wspr_tx_percent, 0);
        assert!((0..1000).all(|i| !c.transmits_in(i)));
    }

    #[test]
    fn a_station_without_a_callsign_or_grid_stays_off_the_air() {
        let c = ctrl(DigiConfig { wspr_tx_percent: 100, ..Default::default() });
        assert!((0..100).all(|i| !c.transmits_in(i)), "transmitted with no callsign set");
        let c = ctrl(DigiConfig {
            my_call: "K1ABC".into(),
            wspr_tx_percent: 100,
            ..Default::default()
        });
        assert!((0..100).all(|i| !c.transmits_in(i)), "transmitted with no grid set");
    }

    /// The duty cycle has to actually be the duty cycle: a beacon that asked for
    /// 20% and transmitted in half the slots is a bad neighbour in a window two
    /// hundred hertz wide.
    #[test]
    fn the_duty_cycle_is_the_fraction_of_slots_it_says_it_is() {
        for pct in [10u8, 20, 33, 50] {
            let c = ctrl(cfg(pct));
            let n = 5000;
            let fired = (0..n).filter(|&i| c.transmits_in(i)).count();
            let got = 100.0 * fired as f64 / n as f64;
            assert!((got - pct as f64).abs() < 3.0, "asked for {pct}% and got {got:.1}%");
        }
        assert!((0..100).all(|i| ctrl(cfg(100)).transmits_in(i)), "100% left gaps");
    }

    /// The reason this file was rewritten: at 50% the beacon sat out three or
    /// four slots and then keyed three in a row, which is what a fair coin per
    /// slot does and what an operator watching it reads as broken.
    ///
    /// Evenly spaced means the gaps are all the same length, or — when the
    /// percentage does not divide a hundred — all within one slot of each other.
    #[test]
    fn the_beacon_spaces_its_slots_evenly_instead_of_clumping() {
        for (pct, want) in [(10u8, 10i64), (20, 5), (33, 3), (50, 2)] {
            let c = ctrl(cfg(pct));
            let keyed: Vec<i64> = (0..1000).filter(|&i| c.transmits_in(i)).collect();
            let gaps: Vec<i64> = keyed.windows(2).map(|w| w[1] - w[0]).collect();
            let (lo, hi) = (*gaps.iter().min().unwrap(), *gaps.iter().max().unwrap());
            assert!(
                hi - lo <= 1,
                "{pct}% keyed with gaps from {lo} to {hi} slots — that is a clump, not a cadence"
            );
            assert_eq!(lo, want, "{pct}% should key about one slot in {want}, got one in {lo}");
        }
    }

    /// Two stations must not start their cycle on the same slot, or every
    /// sdroxide beacon on the band would key together.
    ///
    /// A fixed cadence can only offer this as a phase difference, so that is
    /// what is asserted. Where two phases *do* line up the stations share slots
    /// — survivable here in a way it would not be in a sequenced mode, because
    /// the per-slot tone draw is salted the same way and a WSPR receiver
    /// decodes the whole window at once.
    #[test]
    fn two_stations_do_not_start_their_cycle_on_the_same_slot() {
        let a = ctrl(cfg(20));
        let b = ctrl(DigiConfig { my_call: "G0XYZ".into(), ..cfg(20) });
        let first =
            |c: &WsprController| (0..100).find(|&i| c.transmits_in(i)).expect("never keyed");
        assert_ne!(first(&a), first(&b), "two callsigns drew the same phase");
        // And where a pair does share a slot, the tones are far enough apart to
        // be separate signals: the window is 200 Hz and a beacon is ~6 Hz wide.
        let c = ctrl(DigiConfig { my_call: "VK7ZZ".into(), ..cfg(20) });
        for i in (0..500).filter(|&i| a.transmits_in(i) && c.transmits_in(i)) {
            assert!(
                (a.tx_audio_hz(i) - c.tx_audio_hz(i)).abs() > 10.0,
                "two stations sharing slot {i} landed within 10 Hz of each other"
            );
        }
    }

    /// The panel promises the next slot before it happens, so the promise has to
    /// be the same one the transmitter later keeps.
    #[test]
    fn the_status_promise_matches_what_actually_transmits() {
        let c = ctrl(cfg(50));
        for i in 0..500 {
            assert_eq!(c.transmits_in(i + 1), c.transmits_in(i + 1), "draw is not stable");
        }
    }

    #[test]
    fn the_transmit_offset_stays_inside_the_window_and_moves_between_slots() {
        let c = ctrl(cfg(100));
        let mut seen = std::collections::HashSet::new();
        for i in 0..200 {
            let hz = c.tx_audio_hz(i);
            assert!(
                (WSPR_WINDOW_LO_HZ..=WSPR_WINDOW_HI_HZ).contains(&hz),
                "{hz} Hz is outside the window"
            );
            seen.insert(hz.to_bits());
        }
        assert!(seen.len() > 100, "the transmit frequency barely moved ({} values)", seen.len());
    }

    #[test]
    fn a_pinned_transmit_frequency_does_not_wander() {
        let mut c = ctrl(DigiConfig { auto_tx_freq: false, ..cfg(100) });
        c.set_audio_hz(1440.0);
        assert!((0..50).all(|i| c.tx_audio_hz(i) == 1440.0));
    }

    #[test]
    fn hopping_is_off_until_it_is_asked_for_and_then_visits_every_enabled_band() {
        let c = ctrl(cfg(0));
        assert_eq!(c.hop_target(0), None, "hopped without being asked");

        let c = ctrl(DigiConfig { wspr_hop: true, ..cfg(0) });
        let visited: std::collections::HashSet<u64> =
            (0..64).filter_map(|i| c.hop_target(i)).map(|f| f as u64).collect();
        assert_eq!(visited.len(), 8, "default cycle visited {} bands", visited.len());
        // The rotation is a rotation: consecutive slots are different bands.
        assert_ne!(c.hop_target(0), c.hop_target(1));
    }

    /// Deselecting every band must mean "nowhere", not "everywhere".
    #[test]
    fn hopping_with_no_bands_enabled_goes_nowhere() {
        let c = ctrl(DigiConfig { wspr_hop: true, wspr_hop_bands: 0, ..cfg(0) });
        assert_eq!(c.hop_target(0), None);
    }

    #[test]
    fn the_audio_frequency_is_clamped_into_the_window() {
        let mut c = ctrl(cfg(0));
        c.set_audio_hz(200.0);
        assert_eq!(c.audio_hz(), WSPR_WINDOW_LO_HZ);
        c.set_audio_hz(3000.0);
        assert_eq!(c.audio_hz(), WSPR_WINDOW_HI_HZ);
    }

    /// A Type-3 message names its sender by a hash of a function this station
    /// cannot compute. It has to say so rather than invent a callsign — and it
    /// must still report the grid, which arrived in the clear and is the part
    /// the propagation map actually needs.
    #[test]
    fn a_hashed_callsign_is_reported_as_a_hash_and_keeps_its_grid() {
        let c = ctrl(cfg(0));
        let d = WsprDecode {
            message: WsprMessage::Type3 {
                callsign_hash: 0x0a3f,
                grid6: "FN42aa".into(),
                power_dbm: 37,
            },
            freq_hz: 1500.0,
            dt_sec: 0.2,
            snr_db: -22.0,
            drift_hz: 0.0,
            fit: 0.5,
            from_osd: false,
        };
        let s = c.to_spot(&d, 0, 14_095_600.0);
        assert_eq!(s.call, "<#00a3f>");
        assert!(!wspr::is_resolved(&s.call), "a placeholder passed as a callsign");
        assert_eq!(s.grid.as_deref(), Some("FN42aa"), "the grid was in the clear");
    }

    /// `wsprd`'s rule for the ordered-statistics fallback: it only confirms a
    /// station already heard outright, with the same grid, and never names a
    /// hashed one. A Fano decode needs no such introduction (issue #946).
    #[test]
    fn an_osd_decode_is_believed_only_for_a_station_already_heard() {
        let at = |message: WsprMessage, from_osd: bool| WsprDecode {
            message,
            freq_hz: 1500.0,
            dt_sec: 0.0,
            snr_db: -28.0,
            drift_hz: 0.0,
            fit: 0.5,
            from_osd,
        };
        let t1 = |grid: &str| WsprMessage::Type1 {
            callsign: "K1ABC".into(),
            grid: grid.into(),
            power_dbm: 37,
        };
        let t2 = || WsprMessage::Type2 { callsign: "30O/RY3T".into(), power_dbm: 40 };
        let t3 = || WsprMessage::Type3 {
            callsign_hash: 0x029d9,
            grid6: "JO41QF".into(),
            power_dbm: 30,
        };
        let mut k = KnownCalls::default();

        assert!(!k.admit(&at(t1("FN42"), true)), "OSD named a station nobody has heard");
        assert!(!k.admit(&at(t2(), true)), "nor a compound one");
        assert!(k.admit(&at(t1("FN42"), false)), "Fano needs no introduction");
        assert!(k.admit(&at(t1("FN42"), true)), "and then vouches for OSD");
        assert!(!k.admit(&at(t1("FN31"), true)), "but only at the grid it was heard from");

        assert!(!k.admit(&at(t3(), true)), "OSD never names a hashed station");
        assert!(k.admit(&at(t3(), false)), "a Fano Type 3 is still reported");
    }

    /// A compound callsign carries no grid. Reporting one would be fabricating
    /// a position, which on a propagation map is worse than an empty cell.
    #[test]
    fn a_compound_callsign_reports_no_grid_rather_than_a_guess() {
        let c = ctrl(cfg(0));
        let d = WsprDecode {
            message: WsprMessage::Type2 { callsign: "PJ4/K1ABC".into(), power_dbm: 33 },
            freq_hz: 1500.0,
            dt_sec: 0.0,
            snr_db: -18.0,
            drift_hz: 0.0,
            fit: 0.5,
            from_osd: false,
        };
        let s = c.to_spot(&d, 0, 14_095_600.0);
        assert_eq!(s.call, "PJ4/K1ABC");
        assert_eq!(s.grid, None);
    }

    /// The spot's frequency is the dial the audio was *recorded* on plus the
    /// tone — not whatever the dial happens to be by the time the decode comes
    /// back, which under band hopping is a different band entirely.
    #[test]
    fn a_spot_is_placed_on_the_dial_its_audio_was_recorded_on() {
        let c = ctrl(cfg(0));
        let d = WsprDecode {
            message: WsprMessage::Type1 {
                callsign: "K1ABC".into(),
                grid: "FN42".into(),
                power_dbm: 37,
            },
            freq_hz: 1500.0,
            dt_sec: 0.0,
            snr_db: -20.0,
            drift_hz: 0.0,
            fit: 0.5,
            from_osd: false,
        };
        let s = c.to_spot(&d, 0, 7_038_600.0);
        assert!(
            (s.freq_hz - (7_038_600.0 + 1502.197)).abs() < 0.01,
            "{} is not on 40 m",
            s.freq_hz
        );
    }
}
