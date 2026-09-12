//! `RadeController` — FreeDV **RADE V1** digital voice.
//!
//! Structurally this is the keyboard-modem controller with speech in place of
//! text: transmit is a continuous over the operator opens and closes rather
//! than a timed burst, and receive runs incrementally as audio arrives.
//!
//! Two things make it unlike every other mode here, and they are why
//! [`DigiEngine`] grew `rx_audio_out`, `wants_mic` and `on_tx_mic`:
//!
//! * it *produces* receive audio (synthesized speech) instead of text or an
//!   image, so the engine plays its output in place of the demodulated signal;
//! * it transmits the live microphone rather than a burst it synthesised
//!   itself.
//!
//! All the signal processing — acquisition, demodulation, neural decode,
//! vocoding and every sample-rate conversion — happens on
//! [`RadeWorker`]'s own thread. This controller only moves samples across ring
//! buffers, so nothing here can stall the engine's audio loop.

use std::time::SystemTime;

use sdroxide_rade::RadeWorker;
use sdroxide_types::{DigiConfig, DigiStatus, Mode, QsoStep, RadeStatus, TranscriptLine};
use tracing::error;

use crate::DigiEngine;
use crate::controller::DigiAction;

/// Engine audio rate: the rate of decoded speech out and microphone audio in.
const OUT_RATE: f64 = 48_000.0;

pub struct RadeController {
    cfg: DigiConfig,
    /// `None` when the modem could not be opened — the mode then behaves as a
    /// plain SSB receiver rather than taking the app down.
    worker: Option<RadeWorker>,

    tx_active: bool,
    keyed: bool,
    /// True once the end-of-over frame has gone out and transmit has drained.
    tx_done: bool,

    /// Dial frequency at the last poll, so a decoded callsign can be reported
    /// at an absolute frequency.
    dial_hz: f64,
    /// The last callsign decoded from a remote End-of-Over frame, shown until
    /// the next over starts.
    last_dx_call: Option<String>,

    last_status: DigiStatus,
    status_dirty: bool,
}

impl RadeController {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        let worker = match RadeWorker::new(tap_rate, OUT_RATE) {
            Ok(w) => Some(w),
            Err(e) => {
                error!(?e, "could not start the RADE modem; audio will pass through");
                None
            }
        };
        if let Some(w) = worker.as_ref() {
            w.set_callsign(&cfg.my_call);
        }
        let last_status = build_status(&cfg, false, None, None);
        RadeController {
            cfg,
            worker,
            tx_active: false,
            keyed: false,
            tx_done: false,
            dial_hz: 0.0,
            last_dx_call: None,
            last_status,
            status_dirty: true,
        }
    }
}

fn build_status(
    cfg: &DigiConfig,
    keyed: bool,
    rade: Option<RadeStatus>,
    dx_call: Option<String>,
) -> DigiStatus {
    DigiStatus {
        mode: Mode::Rade,
        step: QsoStep::Idle,
        dx_call,
        dx_grid: None,
        tx_next: keyed,
        tx_pending_msg: None,
        // RADE's carriers are fixed by the waveform, so there is no operator
        // tone offset to report; the centre of the occupied band is the honest
        // value for the panadapter marker.
        audio_hz: 1470.0,
        tx_even: false,
        transmitting: keyed,
        tx_watchdog: false,
        transcript: Vec::<TranscriptLine>::new(),
        config: cfg.clone(),
        text_rx: String::new(),
        tx_sent: 0,
        fsq_heard: Vec::new(),
        fsq_messages: Vec::new(),
        rade,
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

impl DigiEngine for RadeController {
    fn mode(&self) -> Mode {
        Mode::Rade
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        if self.keyed {
            return; // our own transmit audio is not a signal to decode
        }
        if let Some(w) = self.worker.as_mut() {
            w.push_rx(tap);
        }
    }

    fn rx_audio_out(&mut self, out: &mut Vec<f32>) -> bool {
        let Some(w) = self.worker.as_mut() else { return false };
        if self.keyed {
            return false;
        }
        // Out of sync there is nothing to play, and substituting silence would
        // mute the receiver while the operator is still tuning. Let the raw
        // audio through instead.
        if !w.stats().sync {
            return false;
        }
        w.pop_rx(out);
        true
    }

    /// With the panel's analog mute set, the raw signal is silenced whenever
    /// the decoder has nothing to play — out of sync, and while transmitting.
    fn mutes_analog_audio(&self) -> bool {
        self.cfg.rade_mute_analog
    }

    fn poll(&mut self, _now: SystemTime, dial_hz: f64) -> Vec<DigiAction> {
        let mut actions = Vec::new();
        self.dial_hz = dial_hz;
        if self.tx_active && !self.keyed {
            self.keyed = true;
            self.tx_done = false;
            self.status_dirty = true;
            actions.push(DigiAction::KeyTx);
        }

        // Callsigns the far end put in its End-of-Over frame. At most one per
        // received over, so this is empty on nearly every poll.
        if let Some(w) = self.worker.as_ref() {
            for t in w.poll_text() {
                self.last_dx_call = Some(t.call.clone());
                self.status_dirty = true;
                actions.push(DigiAction::RadeCallsign {
                    call: t.call,
                    snr_db: t.snr_db,
                    freq_hz: self.dial_hz,
                });
            }
        }

        let status = build_status(
            &self.cfg,
            self.keyed,
            self.worker.as_ref().map(stats_of),
            self.last_dx_call.clone(),
        );
        if self.status_dirty || status.rade != self.last_status.rade {
            self.status_dirty = false;
            self.last_status = status.clone();
            actions.push(DigiAction::Status(status));
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        self.keyed
    }

    fn wants_mic(&self) -> bool {
        true
    }

    fn on_tx_mic(&mut self, mic_48k: &[f32]) {
        if !self.keyed {
            return;
        }
        if let Some(w) = self.worker.as_mut() {
            w.push_mic(mic_48k);
        }
    }

    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        let Some(w) = self.worker.as_mut() else {
            out.fill(0.0);
            return true;
        };
        w.pop_tx(out);
        // The over ends when the operator has released transmit *and* the
        // end-of-over frame has been generated and read out. Returning true
        // early would cut the tail the far end needs to close cleanly.
        if !self.tx_active && !self.tx_done && w.tx_drained() {
            self.tx_done = true;
        }
        self.tx_done
    }

    /// RADE's 6 dB is not headroom the transmitter may have back.
    ///
    /// `sdroxide_rade::TX_REAL_SCALE` puts the waveform's *nominal* unit
    /// amplitude at half scale, exactly as the C library specifies — but this
    /// is a multi-carrier waveform whose peaks ride well above nominal, and
    /// that is what the 6 dB is there for. Declaring the level it is already at
    /// leaves those peaks intact; scaling it up to a nominal full scale would
    /// flatten every one of them against the limiter, which is the one thing a
    /// digital voice modem cannot survive.
    fn tx_peak(&self) -> f32 {
        1.0
    }

    fn on_burst_done(&mut self) {
        self.keyed = false;
        self.tx_done = false;
        // The receiver has been fed our own transmit audio for the length of
        // the over; start it clean.
        if let Some(w) = self.worker.as_ref() {
            w.reset();
        }
        self.status_dirty = true;
    }

    fn abort(&mut self) {
        self.abort_tx();
    }

    fn abort_tx(&mut self) {
        // See `CwController::abort_tx`: a refused key-up must not leave the
        // latch set, or nothing can ever key again.
        self.keyed = false;
        self.tx_active = false;
        if let Some(w) = self.worker.as_ref() {
            w.set_tx(false);
        }
        self.tx_done = true;
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        if cfg.my_call != self.cfg.my_call
            && let Some(w) = self.worker.as_ref()
        {
            w.set_callsign(&cfg.my_call);
        }
        self.cfg = cfg;
        self.status_dirty = true;
    }

    /// RADE's carriers are fixed by the waveform — there is no tone offset to
    /// move, so the audio-frequency control does nothing here.
    fn set_audio_hz(&mut self, _hz: f32) {}

    fn audio_hz(&self) -> f32 {
        self.last_status.audio_hz
    }

    fn status(&self) -> DigiStatus {
        build_status(
            &self.cfg,
            self.keyed,
            self.worker.as_ref().map(stats_of),
            self.last_dx_call.clone(),
        )
    }

    fn set_tx_active(&mut self, on: bool) {
        if on == self.tx_active {
            return;
        }
        if on {
            // A new over of our own: whoever we last heard is no longer the
            // station on frequency as far as the display is concerned.
            self.last_dx_call = None;
        }
        self.tx_active = on;
        if let Some(w) = self.worker.as_ref() {
            w.set_tx(on);
        }
        self.status_dirty = true;
    }
}

fn stats_of(w: &RadeWorker) -> RadeStatus {
    let s = w.stats();
    RadeStatus {
        sync: s.sync,
        snr_db: s.snr_db,
        freq_offset_hz: s.freq_offset_hz,
        rx_level: s.rx_level,
        eoo_count: s.eoo_count,
        dropped: s.dropped,
    }
}
