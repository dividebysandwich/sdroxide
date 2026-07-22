//! `SstvController` — the image-mode counterpart to the text/slotted digital
//! controllers. RX decodes incoming pictures scanline-by-scanline (emitting one
//! action per row so the UI paints progressively); TX encodes a composed image
//! that the UI has already cropped, captioned, and PNG-decoded into RGB.

use std::time::SystemTime;

use sdroxide_dsp::{SstvEvent, SstvRx, SstvTx};
use sdroxide_types::{DigiConfig, DigiStatus, Mode, QsoStep, SstvMode, SstvStatus, TranscriptLine};

use crate::DigiEngine;
use crate::controller::DigiAction;

/// TX audio sample rate (injected as "mic", then USB-modulated).
const OUT_RATE: f64 = 48_000.0;

pub struct SstvController {
    cfg: DigiConfig,
    dial_hz: f64,

    // RX
    rx: SstvRx,
    rx_scratch: Vec<SstvEvent>,
    /// Accumulating RGB of the image currently being received.
    rx_image: Vec<u8>,
    rx_mode: SstvMode,
    rx_w: u16,
    rx_h: u16,
    rx_active: bool,
    detected: Option<SstvMode>,
    image_id: u32,

    // TX
    tx: Option<SstvTx>,
    tx_mode: SstvMode,
    keyed: bool,

    // Actions queued for the next `poll`.
    queued: Vec<DigiAction>,
    status_dirty: bool,
    last_status: Option<SystemTime>,
}

impl SstvController {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        let mut rx = SstvRx::new(tap_rate);
        // Free-run decode uses the operator-selected mode until a VIS is heard.
        rx.set_expected(Some(SstvMode::default()));
        SstvController {
            cfg,
            dial_hz: 0.0,
            rx,
            rx_scratch: Vec::new(),
            rx_image: Vec::new(),
            rx_mode: SstvMode::default(),
            rx_w: 0,
            rx_h: 0,
            rx_active: false,
            detected: None,
            image_id: 0,
            tx: None,
            tx_mode: SstvMode::default(),
            keyed: false,
            queued: Vec::new(),
            status_dirty: true,
            last_status: None,
        }
    }

    fn sstv_status(&self) -> SstvStatus {
        let progress = if self.keyed {
            self.tx.as_ref().map(|t| t.progress()).unwrap_or(0.0)
        } else if self.rx_active {
            self.rx.progress()
        } else {
            0.0
        };
        SstvStatus {
            tx_mode: self.tx_mode,
            tx_active: self.keyed,
            rx_active: self.rx_active,
            detected: self.detected,
            progress,
            signal: self.rx.level(),
        }
    }

    /// A minimal FT8-style status; SSTV state travels via `DigiAction::SstvStatus`.
    fn digi_status(&self) -> DigiStatus {
        DigiStatus {
            mode: Mode::Sstv,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: self.keyed,
            tx_pending_msg: None,
            audio_hz: 1500.0,
            tx_even: false,
            transmitting: self.keyed,
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            text_rx: String::new(),
            tx_sent: 0,
        }
    }
}

impl DigiEngine for SstvController {
    fn mode(&self) -> Mode {
        Mode::Sstv
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        self.rx_scratch.clear();
        let mut events = std::mem::take(&mut self.rx_scratch);
        self.rx.process(tap, &mut events);
        for e in events.drain(..) {
            match e {
                SstvEvent::ModeDetected(mode) => {
                    self.image_id = self.image_id.wrapping_add(1);
                    self.rx_mode = mode;
                    let (w, h) = mode.dimensions();
                    self.rx_w = w;
                    self.rx_h = h;
                    self.rx_image = vec![0u8; w as usize * h as usize * 3];
                    self.rx_active = true;
                    self.detected = Some(mode);
                    // RX mode determines the next transmit mode (and free-run mode).
                    self.tx_mode = mode;
                    self.rx.set_expected(Some(mode));
                    self.status_dirty = true;
                }
                SstvEvent::Line { y, rgb } => {
                    let w = self.rx_w as usize;
                    let row = y as usize * w * 3;
                    if row + rgb.len() <= self.rx_image.len() {
                        self.rx_image[row..row + rgb.len()].copy_from_slice(&rgb);
                    }
                    self.queued.push(DigiAction::SstvLine { image_id: self.image_id, y, rgb });
                    self.status_dirty = true;
                }
                SstvEvent::ImageComplete => {
                    self.queued.push(DigiAction::SstvImage {
                        image_id: self.image_id,
                        mode: self.rx_mode,
                        w: self.rx_w,
                        h: self.rx_h,
                        rgb: std::mem::take(&mut self.rx_image),
                    });
                    self.rx_active = false;
                    self.status_dirty = true;
                }
            }
        }
        self.rx_scratch = events; // give the buffer back for reuse
    }

    fn poll(&mut self, now: SystemTime, dial_hz: f64) -> Vec<DigiAction> {
        self.dial_hz = dial_hz;
        let mut actions = std::mem::take(&mut self.queued);
        if self.tx.is_some() && !self.keyed {
            self.keyed = true;
            self.status_dirty = true;
            actions.push(DigiAction::KeyTx);
        }
        // Emit status on change, and at least a few times a second regardless so
        // the signal-level meter and progress stay live even while hunting.
        let periodic = match self.last_status {
            Some(t) => now.duration_since(t).map(|d| d.as_millis() >= 150).unwrap_or(true),
            None => true,
        };
        if self.status_dirty || periodic {
            self.status_dirty = false;
            self.last_status = Some(now);
            actions.push(DigiAction::SstvStatus(self.sstv_status()));
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
        1500.0
    }

    fn status(&self) -> DigiStatus {
        self.digi_status()
    }

    fn set_sstv_mode(&mut self, mode: SstvMode) {
        self.tx_mode = mode;
        self.rx.set_expected(Some(mode));
        self.status_dirty = true;
    }

    fn set_sstv_image(&mut self, mode: SstvMode, rgb: Vec<u8>, w: u16, h: u16) {
        self.tx_mode = mode;
        self.tx = Some(SstvTx::new(mode, &rgb, w, h, OUT_RATE));
        self.status_dirty = true;
    }
}
