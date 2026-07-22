//! `DigiController` — the real-time-loop glue that ties the modem, QSO
//! machine, and slot scheduler into the engine.
//!
//! The engine calls [`on_rx_audio`](DigiController::on_rx_audio) with the
//! demodulated audio tap each block, and [`poll`](DigiController::poll) once
//! per loop tick. `poll` never blocks: heavy LDPC decode runs on a worker
//! thread, and the controller returns [`DigiAction`]s for the engine to
//! apply (emit events, key/unkey PTT).

use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::SystemTime;

use sdroxide_dsp::MonoResampler;
use sdroxide_types::{Decode, DigiConfig, DigiStatus, Mode, QsoRecord, adif_band};

use crate::modem::Ft8Modem;
use crate::params::{DECODE_RATE, DigiParams};
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
}

struct DecodeJob {
    audio: Vec<i16>,
    slot_utc: i64,
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
    dial_hz: f64,
    audio_hz: f32,
    /// Which slot period we transmit in (even/odd), and a per-slot guard so
    /// we key at most once per slot.
    tx_even: bool,
    tx_fired_slot: i64,
    // Decode worker.
    job_tx: Sender<DecodeJob>,
    res_rx: Receiver<(i64, Vec<Decode>)>,
    _worker: std::thread::JoinHandle<()>,
    // TX burst playback.
    burst: Option<BurstPlayer>,
    status_dirty: bool,
}

/// Metered playback of a synthesized TX burst (48 kHz mono).
pub struct BurstPlayer {
    pub samples: Vec<f32>,
    pub pos: usize,
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
                let modem = Ft8Modem::new(worker_mode);
                while let Ok(job) = job_rx.recv() {
                    let decodes = modem.decode_slot(&job.audio, job.slot_utc);
                    if res_tx.send((job.slot_utc, decodes)).is_err() {
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
            dial_hz: 0.0,
            audio_hz: 1500.0,
            tx_even,
            tx_fired_slot: i64::MIN,
            job_tx,
            res_rx,
            _worker: worker,
            burst: None,
            status_dirty: true,
        }
    }

    pub fn mode(&self) -> Mode {
        self.params.mode
    }

    pub fn set_config(&mut self, cfg: DigiConfig) {
        self.qso.set_config(cfg);
        self.status_dirty = true;
    }

    pub fn set_audio_hz(&mut self, hz: f32) {
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
        self.qso.call_cq();
        self.status_dirty = true;
    }

    pub fn start_qso(&mut self, from: String, grid: Option<String>, snr: i16, audio_hz: f32) {
        self.set_audio_hz(audio_hz);
        let now_t = SystemTime::now();
        let now = SlotScheduler::unix_now(now_t) as i64;
        // We answer in the slot right after the DX transmitted. Their decode
        // completes early in that following slot, so its parity — the parity
        // of "now" — is exactly the period we should transmit in.
        self.tx_even = self.scheduler.is_even(self.scheduler.slot_index(now_t));
        self.qso.start_qso(from, grid, snr, now);
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

    /// Notify the QSO machine that the burst finished going out (advances a
    /// terminal 73 to Done). Returns actions (status change) for the engine.
    pub fn on_burst_done(&mut self) {
        let now = SlotScheduler::unix_now(SystemTime::now()) as i64;
        self.qso.note_tx_sent(now);
        self.status_dirty = true;
    }

    /// Synthesize `msg` into a 48 kHz mono burst (12 kHz GFSK, resampled).
    fn synth_burst_48k(&mut self, msg: &str) -> Option<Vec<f32>> {
        let burst12 = self.modem.encode_burst_12k(msg, self.audio_hz, 0.5)?;
        // Resample 12 k → 48 k.
        match MonoResampler::new(DECODE_RATE, 48_000.0) {
            Some(mut r) => {
                let mut out = Vec::with_capacity(burst12.len() * 4 + 2048);
                r.push(&burst12, &mut out);
                Some(out)
            }
            None => Some(burst12),
        }
    }

    /// Called each engine loop tick. Detects slot boundaries, dispatches the
    /// finished slot to the decode worker, drains results, advances the QSO
    /// machine, and (D3) schedules TX. Returns actions for the engine.
    pub fn poll(&mut self, now: SystemTime, dial_hz: f64) -> Vec<DigiAction> {
        self.dial_hz = dial_hz;
        let mut actions = Vec::new();

        // 1. Drain finished decodes from the worker.
        loop {
            match self.res_rx.try_recv() {
                Ok((slot_utc, decodes)) => {
                    if !decodes.is_empty() {
                        // Advance the QSO from anything addressed to us.
                        if self.qso.on_rx(&decodes, slot_utc) {
                            self.status_dirty = true;
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
                let min_samples = (self.params.slot_s * DECODE_RATE * 0.5) as usize;
                if self.slot_buf.len() >= min_samples {
                    let audio = std::mem::take(&mut self.slot_buf);
                    let slot_utc = self.scheduler.slot_start_unix(self.last_slot_idx) as i64;
                    let _ = self.job_tx.send(DecodeJob { audio, slot_utc });
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
            if into >= self.params.tx_offset_s && into < self.params.tx_offset_s + 1.5 {
                if let Some(msg) = self.qso.plan_tx() {
                    if let Some(samples) = self.synth_burst_48k(&msg) {
                        self.burst = Some(BurstPlayer { samples, pos: 0 });
                        self.tx_fired_slot = idx;
                        self.qso.record_tx(&msg);
                        self.status_dirty = true;
                        actions.push(DigiAction::KeyTx);
                    }
                }
            }
        }

        // 4. Completed QSO → log it (fill freq/band from the dial).
        if let Some(mut rec) = self.qso.take_completed() {
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
        s
    }
}

impl crate::DigiEngine for DigiController {
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
    fn start_qso(&mut self, from: String, grid: Option<String>, snr: i16, audio_hz: f32) {
        DigiController::start_qso(self, from, grid, snr, audio_hz)
    }
    fn stop_qso(&mut self) {
        DigiController::stop_qso(self)
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

    #[test]
    fn no_burst_without_a_callsign() {
        let mut c = DigiController::new(Mode::Ft8, DigiConfig::default(), 12_000.0);
        c.call_cq();
        let now = UNIX_EPOCH + Duration::from_secs_f64(1_609_459_201.0);
        let actions = c.poll(now, 14_074_000.0);
        assert!(!actions.iter().any(|a| matches!(a, DigiAction::KeyTx)));
        assert!(!c.tx_burst_active());
    }
}
