//! `RfPaintController` — the transmit-only "RF Paint" (Spectrum Painting) mode.
//!
//! There is no decoder: the picture *is* the signal, read off the operator's
//! own waterfall. The UI rasterises text or an image into a grayscale bitmap
//! and sends it through the ordinary `DigiImageTx` command path, which the
//! engine grayscales and hands here via [`set_image`](DigiEngine::set_image);
//! we build a [`SpectrumPaintTx`] burst and key the transmitter.
//!
//! The transmit state machine mirrors the SSTV controller: `tx: Option<..>`
//! plus a `keyed` flag. `set_image` queues the burst; the next `poll` emits
//! `KeyTx`; `fill_tx_block` pumps the audio until `done`, then `on_burst_done`
//! returns to idle.

use std::time::SystemTime;

use sdroxide_dsp::{RF_PAINT_CENTER, SpectrumPaintTx};
use sdroxide_types::{DigiConfig, DigiStatus, Mode, QsoStep, TranscriptLine};

use crate::DigiEngine;
use crate::controller::DigiAction;

/// TX audio sample rate (injected as "mic", then USB-modulated).
const OUT_RATE: f64 = 48_000.0;

pub struct RfPaintController {
    cfg: DigiConfig,
    dial_hz: f64,

    tx: Option<SpectrumPaintTx>,
    keyed: bool,

    queued: Vec<DigiAction>,
    status_dirty: bool,
    last_status: Option<SystemTime>,
}

impl RfPaintController {
    pub fn new(cfg: DigiConfig, _tap_rate: f64) -> Self {
        RfPaintController {
            cfg,
            dial_hz: 0.0,
            tx: None,
            keyed: false,
            queued: Vec::new(),
            status_dirty: true,
            last_status: None,
        }
    }

    fn digi_status(&self) -> DigiStatus {
        DigiStatus {
            mode: Mode::RfPaint,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: self.keyed,
            tx_pending_msg: None,
            audio_hz: RF_PAINT_CENTER,
            tx_even: false,
            transmitting: self.keyed,
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            text_rx: String::new(),
            // Reuse the "sent" cursor as a permille progress readout (0..1000) so
            // the panel can show a real transmit-progress bar without new types.
            tx_sent: self.tx.as_ref().map(|t| (t.progress() * 1000.0) as usize).unwrap_or(0),
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
        }
    }
}

impl DigiEngine for RfPaintController {
    fn mode(&self) -> Mode {
        Mode::RfPaint
    }

    // No decoder — RF Paint is read directly off the waterfall.
    fn on_rx_audio(&mut self, _tap: &[f32]) {}

    fn poll(&mut self, now: SystemTime, dial_hz: f64) -> Vec<DigiAction> {
        self.dial_hz = dial_hz;
        let mut actions = std::mem::take(&mut self.queued);
        if self.tx.is_some() && !self.keyed {
            self.keyed = true;
            self.status_dirty = true;
            actions.push(DigiAction::KeyTx);
        }
        // Emit status on change, and a few times a second while keyed so the
        // transmit-progress bar stays live.
        let periodic = match self.last_status {
            Some(t) => now.duration_since(t).map(|d| d.as_millis() >= 150).unwrap_or(true),
            None => true,
        };
        if self.status_dirty || periodic {
            self.status_dirty = false;
            self.last_status = Some(now);
            actions.push(DigiAction::Status(self.digi_status()));
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        self.keyed
    }

    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        match &mut self.tx {
            Some(tx) => {
                tx.next_block(out);
                self.status_dirty = true;
                tx.done()
            }
            None => {
                for s in out.iter_mut() {
                    *s = 0.0;
                }
                true
            }
        }
    }

    fn on_burst_done(&mut self) {
        self.tx = None;
        self.keyed = false;
        self.status_dirty = true;
    }

    fn abort(&mut self) {
        self.abort_tx();
    }

    fn abort_tx(&mut self) {
        self.tx = None;
        self.keyed = false;
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        self.cfg = cfg;
        self.status_dirty = true;
    }

    fn set_audio_hz(&mut self, _hz: f32) {}

    fn audio_hz(&self) -> f32 {
        RF_PAINT_CENTER
    }

    fn status(&self) -> DigiStatus {
        self.digi_status()
    }

    /// Queue a grayscale bitmap (`w*h` bytes, row 0 = top) as a painting burst.
    fn set_image(&mut self, gray: Vec<u8>, w: u16, h: u16) {
        self.tx = Some(SpectrumPaintTx::new(&gray, w as usize, h as usize, OUT_RATE));
        self.status_dirty = true;
    }
}
