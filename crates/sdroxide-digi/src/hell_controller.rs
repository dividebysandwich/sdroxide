//! `HellController` — the Hellschreiber counterpart to the keyboard-mode
//! [`TextModemController`](crate::TextModemController).
//!
//! Transmit is the same shape: type, and it streams out while the switch is
//! held. Receive is not — Hell carries no characters, so there is nothing to
//! decode into text. What comes back is a stream of dot columns that the panel
//! paints, and `DigiStatus::text_rx` stays empty on purpose.

use std::time::SystemTime;

use sdroxide_dsp::{HELL_ROWS, HellRx, HellTx};
use sdroxide_types::{DigiConfig, DigiStatus, Mode, QsoStep, TranscriptLine};

use crate::DigiEngine;
use crate::controller::DigiAction;

/// TX audio sample rate (injected as "mic", then USB-modulated).
///
/// Receive runs at the tap rate and transmit at this one, with no resampler on
/// either side — the modem derives its dot clock from whatever rate it is
/// handed, the way `SstvRx`/`SstvTx` do.
const OUT_RATE: f64 = 48_000.0;

/// Default audio centre, matching the MFSK keyboard modes.
const DEFAULT_AUDIO_HZ: f32 = 1500.0;

/// Whole columns that trigger a flush to the UI.
///
/// The engine polls at whatever rate the source returns blocks — often
/// 100–300 Hz — and Hell X9 produces 157 columns a second. Shipping one action
/// per column would be that many events carrying fourteen payload bytes each;
/// batching costs the same bytes in a twentieth of the messages.
const FLUSH_COLS: usize = 8;

/// Milliseconds after which a partial batch ships anyway, so the raster stays
/// live when columns only trickle in (Slow Hell manages two a second).
const FLUSH_MS: u128 = 50;

pub struct HellController {
    cfg: DigiConfig,
    audio_hz: f32,
    dial_hz: f64,

    // RX
    rx: HellRx,
    rx_px: Vec<u8>,
    /// Received dots not yet shipped. May hold a partial column between blocks.
    pend: Vec<u8>,
    /// Absolute index of the next column to ship. The panel uses it to tell a
    /// dropped batch from a restarted receiver.
    seq: u64,
    sq: crate::squelch::Squelch,
    last_cols: Option<SystemTime>,

    // TX
    tx: HellTx,
    /// Scratch for the transmit echo, kept so the 10 ms block pump does not
    /// allocate.
    echo: Vec<u8>,
    tx_buffer: String,
    tx_pushed: usize,
    tx_active: bool,
    keyed: bool,
    last_sent: usize,
    /// Something has actually been sent since transmit was switched on — see
    /// the release under `send_on_enter` in [`Self::fill_tx_block`]. Holding
    /// the channel over an empty buffer is a real thing to do in Hell, so it
    /// must not be mistaken for an over that has finished.
    over_had_text: bool,

    status_dirty: bool,
}

impl HellController {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        let audio_hz = DEFAULT_AUDIO_HZ;
        let variant = cfg.hell_variant;
        let agc = cfg.hell_rx_agc;
        HellController {
            rx: HellRx::new(tap_rate, audio_hz as f64, variant, agc),
            tx: HellTx::new(OUT_RATE, audio_hz as f64, variant),
            cfg,
            audio_hz,
            dial_hz: 0.0,
            rx_px: Vec::new(),
            pend: Vec::new(),
            seq: 0,
            sq: crate::squelch::Squelch::new(),
            last_cols: None,
            echo: Vec::new(),
            tx_buffer: String::new(),
            tx_pushed: 0,
            tx_active: false,
            keyed: false,
            last_sent: 0,
            over_had_text: false,
            status_dirty: true,
        }
    }

    /// True while transmit audio should keep being generated: the switch is
    /// held, or queued text is still draining after it was released.
    fn producing(&self) -> bool {
        self.tx_active || !self.tx.drained()
    }

    /// Discard a part-built column.
    ///
    /// Called whenever the strip changes hands between the receiver and the
    /// transmit echo: splicing the tail of one stream's column onto the head of
    /// the other's would put a smear of neither on the raster.
    fn drop_partial_column(&mut self) {
        let whole = self.pend.len() / HELL_ROWS * HELL_ROWS;
        self.pend.truncate(whole);
    }

    /// Clamp the audio centre so the whole occupied bandwidth stays inside the
    /// USB passband. X9 is 2645 Hz wide and barely fits, so it ends up pinned
    /// to a narrow range — which is the honest answer, not a bug.
    fn clamp_audio_hz(&self, hz: f32) -> f32 {
        let half = (self.cfg.hell_variant.bandwidth_hz() * 0.5) as f32;
        let (lo, hi) = (150.0 + half, 3450.0 - half);
        if lo >= hi { (lo + hi) * 0.5 } else { hz.clamp(lo, hi) }
    }

    fn build_status(&self) -> DigiStatus {
        DigiStatus {
            mode: Mode::Hell,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: self.tx_active,
            tx_pending_msg: (!self.tx_buffer.is_empty()).then(|| self.tx_buffer.clone()),
            audio_hz: self.audio_hz,
            tx_even: false,
            transmitting: self.keyed,
            tx_watchdog: false,
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            // Deliberately empty: Hell is read by eye, not decoded.
            text_rx: String::new(),
            tx_sent: self.tx.sent_chars(),
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            rade: None,
            packet: None,
            navtex: None,
            aprs: None,
            js8: None,
            atchat: None,
            fox_queue: Vec::new(),
            call_queue: Vec::new(),
            clock_offset_s: None,
            cw: None,
            wspr: None,
            qso: None,
        }
    }
}

impl DigiEngine for HellController {
    fn mode(&self) -> Mode {
        Mode::Hell
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        // While transmitting, the strip shows the local echo instead. The tap
        // carries our own signal at monitor level — which would wreck the
        // receive AGC — and, more basically, one strip cannot carry two
        // independent column streams at once.
        if self.keyed {
            return;
        }
        self.rx_px.clear();
        let mut px = std::mem::take(&mut self.rx_px);
        self.rx.process(tap, &mut px);
        if !px.is_empty() {
            // Squelched dots are painted black rather than dropped: dropping
            // them would compress the time base and shear the text sideways.
            if self.sq.open(self.rx.magnitude(), self.cfg.digi_squelch) {
                self.pend.extend_from_slice(&px);
            } else {
                self.pend.resize(self.pend.len() + px.len(), 0);
            }
        }
        px.clear();
        self.rx_px = px;
    }

    fn poll(&mut self, now: SystemTime, dial_hz: f64) -> Vec<DigiAction> {
        self.dial_hz = dial_hz;
        let mut actions = Vec::new();

        // Ship whole columns; a partial one waits for the rest of itself.
        let whole = self.pend.len() / HELL_ROWS;
        if whole > 0 {
            let due = whole >= FLUSH_COLS
                || self
                    .last_cols
                    .is_none_or(|t| now.duration_since(t).is_ok_and(|d| d.as_millis() >= FLUSH_MS));
            if due {
                let cols: Vec<u8> = self.pend.drain(..whole * HELL_ROWS).collect();
                actions.push(DigiAction::HellColumns {
                    seq: self.seq,
                    rows: HELL_ROWS as u8,
                    cols,
                });
                self.seq += whole as u64;
                self.last_cols = Some(now);
            }
        }

        if self.tx_active && !self.keyed {
            self.keyed = true;
            self.status_dirty = true;
            // The strip passes from the receiver to the transmit echo here.
            self.drop_partial_column();
            actions.push(DigiAction::KeyTx);
        }
        if self.status_dirty {
            self.status_dirty = false;
            actions.push(DigiAction::Status(self.build_status()));
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        self.keyed
    }

    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        if self.producing() {
            self.tx.next_block(out, self.tx_active);
        } else {
            out.fill(0.0);
        }
        // Echo what is going out onto the same strip, so the operator can read
        // their own transmission — Hell offers no other confirmation that the
        // font and the timing are right.
        let mut echo = std::mem::take(&mut self.echo);
        self.tx.take_columns(&mut echo);
        self.pend.append(&mut echo);
        self.echo = echo;
        if self.tx.sent_chars() != self.last_sent {
            self.last_sent = self.tx.sent_chars();
            self.status_dirty = true;
        }
        // A committed line is a whole over, and it ends when the line does.
        // Blank paper between overs is how Hell holds a channel, and that is
        // still what the TX button is for — but under `send_on_enter` the
        // operator composes off the air, so holding would paint a blank strip
        // for as long as they take to type.
        //
        // The character count rather than `HellTx::drained`, which stays false
        // while the switch is held by design: it answers "has the carrier
        // stopped", and the question here is "has the text finished".
        if self.tx.sent_chars() < self.tx.total_chars() {
            self.over_had_text = true;
        } else if self.cfg.send_on_enter && self.over_had_text {
            self.over_had_text = false;
            self.tx_active = false;
            self.status_dirty = true;
        }
        !self.producing()
    }

    fn on_burst_done(&mut self) {
        self.keyed = false;
        // ...and back to the receiver.
        self.drop_partial_column();
        self.status_dirty = true;
    }

    fn abort(&mut self) {
        self.abort_tx();
    }

    fn abort_tx(&mut self) {
        // See `CwController::abort_tx`: a refused key-up must not leave the
        // latch set, or nothing can ever key again.
        self.keyed = false;
        self.tx.clear();
        self.tx_buffer.clear();
        self.tx_pushed = 0;
        self.tx_active = false;
        self.last_sent = 0;
        self.over_had_text = false;
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        let retune =
            cfg.hell_variant != self.cfg.hell_variant || cfg.hell_rx_agc != self.cfg.hell_rx_agc;
        let revariant = cfg.hell_variant != self.cfg.hell_variant;
        self.cfg = cfg;
        if retune {
            self.audio_hz = self.clamp_audio_hz(self.audio_hz);
            let hz = self.audio_hz as f64;
            self.rx.set_params(hz, self.cfg.hell_variant, self.cfg.hell_rx_agc);
            self.tx.set_params(hz, self.cfg.hell_variant);
        }
        if revariant {
            // A new variant means a new dot rate, so nothing already on the
            // raster lines up with what comes next. Rewinding `seq` is the
            // panel's cue to start a fresh strip.
            self.pend.clear();
            self.seq = 0;
            self.last_cols = None;
        }
        self.status_dirty = true;
    }

    fn set_audio_hz(&mut self, hz: f32) {
        self.audio_hz = self.clamp_audio_hz(hz);
        let hz = self.audio_hz as f64;
        self.rx.set_params(hz, self.cfg.hell_variant, self.cfg.hell_rx_agc);
        self.tx.set_params(hz, self.cfg.hell_variant);
        self.status_dirty = true;
    }

    fn audio_hz(&self) -> f32 {
        self.audio_hz
    }

    fn status(&self) -> DigiStatus {
        self.build_status()
    }

    fn call_cq(&mut self) {
        let call = if self.cfg.my_call.is_empty() { "NOCALL" } else { &self.cfg.my_call };
        let cq = format!("CQ CQ CQ DE {call} {call} {call} PSE K ");
        self.set_tx_text(cq);
        self.set_tx_active(true);
    }

    fn set_tx_text(&mut self, text: String) {
        let n = text.chars().count();
        if n > self.tx_pushed {
            let tail: String = text.chars().skip(self.tx_pushed).collect();
            self.tx.push_text(&tail);
            self.tx_pushed = n;
        }
        self.tx_buffer = text;
        self.status_dirty = true;
    }

    fn set_tx_active(&mut self, on: bool) {
        self.tx_active = on;
        // A fresh over: what the last one sent says nothing about this one.
        self.over_had_text = false;
        self.status_dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::HellVariant;

    /// One 10 ms transmit block at 48 kHz, as the engine pumps them.
    const BLOCK: usize = 480;

    fn cfg(variant: HellVariant) -> DigiConfig {
        DigiConfig { hell_variant: variant, digi_squelch: 0.0, ..Default::default() }
    }

    /// Loop transmit audio straight back into receive and collect the batches.
    /// This is the one test that exercises the batching and `seq` accounting,
    /// which no DSP-level test can reach.
    fn loopback(variant: HellVariant, secs: f64) -> (Vec<(u64, usize)>, usize) {
        let mut c = HellController::new(cfg(variant), OUT_RATE);
        c.set_tx_text("HELL DE TEST ".into());
        c.set_tx_active(true);

        let mut audio = [0.0f32; BLOCK];
        let mut batches = Vec::new();
        let mut rows_seen = 0usize;
        let blocks = (OUT_RATE * secs / BLOCK as f64) as usize;
        // A fixed epoch: the poll cadence only matters relative to itself, and
        // real time would make the flush timer non-deterministic.
        let epoch = SystemTime::UNIX_EPOCH;
        for i in 0..blocks {
            let now = epoch + std::time::Duration::from_millis(i as u64 * 10);
            // Poll first, so the transmitter is keyed before the first block —
            // the engine's own order, and what makes the echo own the strip.
            let mut actions = c.poll(now, 14_063_000.0);
            c.fill_tx_block(&mut audio);
            c.on_rx_audio(&audio);
            actions.extend(c.poll(now, 14_063_000.0));
            for a in actions {
                if let DigiAction::HellColumns { seq, rows, cols } = a {
                    assert_eq!(rows as usize, HELL_ROWS);
                    assert_eq!(cols.len() % HELL_ROWS, 0, "batch is not whole columns");
                    rows_seen = rows as usize;
                    batches.push((seq, cols.len() / HELL_ROWS));
                }
            }
        }
        (batches, rows_seen)
    }

    #[test]
    fn columns_arrive_at_the_variant_rate() {
        for variant in HellVariant::ALL {
            let secs = 2.0;
            let (batches, rows) = loopback(variant, secs);
            assert_eq!(rows, HELL_ROWS, "{variant:?}");

            let total: usize = batches.iter().map(|&(_, n)| n).sum();
            let want = variant.column_rate() * secs;
            // 5%, but never tighter than a couple of columns: Slow Hell only
            // manages nine in two seconds, so a part-column at each end is a
            // large fraction of the count without meaning anything is wrong.
            let slack = (want * 0.05).max(2.0);
            assert!(
                (total as f64 - want).abs() <= slack,
                "{variant:?}: {total} columns in {secs}s, want ~{want:.1} ± {slack:.1}"
            );
        }
    }

    #[test]
    fn seq_tracks_the_running_column_count() {
        for variant in HellVariant::ALL {
            let (batches, _) = loopback(variant, 1.0);
            let mut expect = 0u64;
            for &(seq, n) in &batches {
                assert_eq!(seq, expect, "{variant:?}: batch seq jumped");
                expect += n as u64;
            }
            assert!(!batches.is_empty(), "{variant:?}: nothing shipped");
        }
    }

    #[test]
    fn a_variant_change_restarts_the_raster() {
        let mut c = HellController::new(cfg(HellVariant::Feld), OUT_RATE);
        let mut audio = [0.0f32; BLOCK];
        c.set_tx_active(true);
        let epoch = SystemTime::UNIX_EPOCH;
        for i in 0..200 {
            c.fill_tx_block(&mut audio);
            c.on_rx_audio(&audio);
            c.poll(epoch + std::time::Duration::from_millis(i * 10), 0.0);
        }
        assert!(c.seq > 0, "nothing was shipped before the switch");

        c.set_config(cfg(HellVariant::Hell80));
        assert_eq!(c.seq, 0, "a new dot rate must rewind seq so the panel clears");
        assert!(c.pend.is_empty(), "stale dots must not carry across a rate change");
    }

    #[test]
    fn the_audio_centre_keeps_the_signal_in_the_passband() {
        // X9 occupies 2645 Hz, so it can only sit near the middle of the
        // passband — the clamp should say so rather than let it hang off an edge.
        let mut c = HellController::new(cfg(HellVariant::X9), OUT_RATE);
        c.set_audio_hz(300.0);
        let half = (HellVariant::X9.bandwidth_hz() * 0.5) as f32;
        assert!(c.audio_hz() - half >= 149.0, "low edge {} is outside", c.audio_hz() - half);
        c.set_audio_hz(3400.0);
        assert!(c.audio_hz() + half <= 3451.0, "high edge {} is outside", c.audio_hz() + half);

        // Feld Hell is narrow enough to tune freely.
        let mut c = HellController::new(cfg(HellVariant::Feld), OUT_RATE);
        c.set_audio_hz(1000.0);
        assert!((c.audio_hz() - 1000.0).abs() < 1.0);
    }

    #[test]
    fn releasing_transmit_flushes_the_queued_text() {
        let mut c = HellController::new(cfg(HellVariant::Feld), OUT_RATE);
        c.set_tx_text("HI".into());
        c.set_tx_active(true);
        let mut audio = [0.0f32; BLOCK];
        // Held: never finishes, because holding the switch is what keeps the
        // channel between overs.
        for _ in 0..500 {
            assert!(!c.fill_tx_block(&mut audio), "should not finish while held");
        }
        // Released: drains the rest of the text, then reports done.
        c.set_tx_active(false);
        let mut done = false;
        for _ in 0..2000 {
            if c.fill_tx_block(&mut audio) {
                done = true;
                break;
            }
        }
        assert!(done, "transmit never finished after release");
        assert_eq!(c.tx.sent_chars(), c.tx.total_chars());
    }

    /// Send on return ends the over with the line. Holding the channel with
    /// blank paper is what the switch is for, and it is exactly wrong here: the
    /// operator is composing the next line off the air, and the strip would
    /// carry a blank for as long as they take to type it.
    #[test]
    fn a_committed_line_releases_the_channel_when_it_has_been_painted() {
        let cfg = DigiConfig { send_on_enter: true, ..cfg(HellVariant::Feld) };
        let mut c = HellController::new(cfg, OUT_RATE);
        c.set_tx_text("HI\n".into());
        c.set_tx_active(true);
        let mut audio = [0.0f32; BLOCK];
        let mut done = false;
        for _ in 0..20_000 {
            if c.fill_tx_block(&mut audio) {
                done = true;
                break;
            }
        }
        assert!(done, "the channel was still held after the line had been painted");
        assert!(!c.status().tx_next);
        assert_eq!(c.tx.sent_chars(), c.tx.total_chars());
    }

    /// The switch is still a switch: pressed over an empty box it holds the
    /// channel, because that is what blank paper between overs is for. An over
    /// that never started is not an over that finished.
    #[test]
    fn holding_the_channel_over_an_empty_box_still_holds() {
        let cfg = DigiConfig { send_on_enter: true, ..cfg(HellVariant::Feld) };
        let mut c = HellController::new(cfg, OUT_RATE);
        c.set_tx_active(true);
        let mut audio = [0.0f32; BLOCK];
        for _ in 0..500 {
            assert!(!c.fill_tx_block(&mut audio), "an empty box ended the over on its own");
        }
    }
}
