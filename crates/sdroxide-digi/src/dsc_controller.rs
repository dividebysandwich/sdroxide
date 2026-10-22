//! `DscController` — the marine distress and calling system, receive only.
//!
//! The decoder is [`sdroxide_dsp::DscRx`]; what lives here is the rolling
//! message list and the status the panel draws. A DSC sequence is one short
//! burst — a second or two — and complete in itself: there is no session to
//! reassemble, so this is thin. Stamp each sequence, keep the newest few
//! hundred, and report the meter and the detector's confidence.
//!
//! # Why there is no transmitter
//!
//! Not a limitation. DSC is distress and safety traffic on the one marine
//! channel a listener can decode; an amateur station putting a false alert on
//! it is not a mode choice but a hoax. See [`Mode::is_rx_only`].
//!
//! # The tail
//!
//! [`DscRx::flush`] is called whenever the audio stream goes quiet, because
//! the FSK detector's band-pass holds back a group delay's worth of samples and
//! the end-of-sequence character sits in exactly that tail. On a live band the
//! next burst pushes it out; on a file decode, or a channel that has just gone
//! quiet, nothing would.

use std::time::SystemTime;

use sdroxide_dsp::DscRx;
use sdroxide_types::{
    DSC_MESSAGE_MAX, DigiConfig, DigiStatus, DscHeard, DscStatus, Mode, QsoStep, TranscriptLine,
};

use crate::DigiEngine;
use crate::controller::DigiAction;

pub struct DscController {
    cfg: DigiConfig,
    rx: DscRx,
    messages: Vec<DscHeard>,
    status_dirty: bool,
    last_status: Option<SystemTime>,
    /// The tail has already been flushed since the last audio with energy, so
    /// a stretch of silence does not flush on every block.
    flushed: bool,
}

impl DscController {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        DscController {
            cfg,
            rx: DscRx::new(tap_rate),
            messages: Vec::new(),
            status_dirty: true,
            last_status: None,
            flushed: true,
        }
    }

    fn now_unix() -> i64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    fn dsc_status(&self) -> DscStatus {
        DscStatus {
            level: self.rx.level(),
            messages: self.messages.clone(),
            sequences: self.rx.sequences(),
            separation: self.rx.separation(),
        }
    }

    fn digi_status(&self) -> DigiStatus {
        DigiStatus {
            stanag: None,
            mode: Mode::Dsc,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: false,
            tx_pending_msg: None,
            audio_hz: sdroxide_types::DSC_TONE_HZ,
            tx_even: false,
            transmitting: false,
            tx_watchdog: false,
            tx_refused: None,
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            text_rx: String::new(),
            tx_sent: 0,
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            rade: None,
            packet: None,
            navtex: None,
            acars: None,
            dsc: Some(self.dsc_status()),
            uvpacket: None,
            jtty: None,
            ale: None,
            aprs: None,
            js8: None,
            atchat: None,
            fox_queue: Vec::new(),
            call_queue: Vec::new(),
            clock_offset_s: None,
            cw: None,
            wspr: None,
            pi4: None,
            qso: None,
        }
    }
}

impl DigiEngine for DscController {
    fn mode(&self) -> Mode {
        Mode::Dsc
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        // The detector's own level is the meter; a block with no energy at all
        // is the end of a burst, which is when the withheld tail has to be
        // released.
        let energy: f32 = tap.iter().map(|s| s * s).sum();
        let mut out = Vec::new();
        if energy > 1e-9 {
            self.rx.process(tap, &mut out);
            self.flushed = false;
        } else if !self.flushed {
            self.rx.process(tap, &mut out);
            self.rx.flush(&mut out);
            self.flushed = true;
        }
        for message in out {
            let at = Self::now_unix();
            self.messages.push(DscHeard { message, at });
            if self.messages.len() > DSC_MESSAGE_MAX {
                let cut = self.messages.len() - DSC_MESSAGE_MAX;
                self.messages.drain(..cut);
            }
            self.status_dirty = true;
        }
    }

    fn poll(&mut self, _now: SystemTime, _dial_hz: f64) -> Vec<DigiAction> {
        let now = SystemTime::now();
        // On change, and a few times a second regardless: the meter and the
        // separation figure are what an operator watches while tuning, and
        // neither changes the message list.
        let due = self
            .last_status
            .map(|t| now.duration_since(t).map(|d| d.as_secs_f32() > 0.25).unwrap_or(true))
            .unwrap_or(true);
        if !self.status_dirty && !due {
            return Vec::new();
        }
        self.status_dirty = false;
        self.last_status = Some(now);
        vec![DigiAction::Status(self.digi_status())]
    }

    fn tx_burst_active(&self) -> bool {
        false
    }

    fn fill_tx_block(&mut self, _out: &mut [f32]) -> bool {
        false
    }

    fn on_burst_done(&mut self) {}

    fn abort(&mut self) {
        self.status_dirty = true;
    }

    fn abort_tx(&mut self) {}

    fn set_config(&mut self, cfg: DigiConfig) {
        self.cfg = cfg;
    }

    fn clear_rx(&mut self) {
        self.messages.clear();
        self.status_dirty = true;
    }

    fn set_audio_hz(&mut self, _hz: f32) {}

    /// Fixed by the standard: MF/HF tones are 1615 and 1785 Hz about 1700 Hz
    /// (100 baud; VHF channel 70 is 1300/2100 Hz at 1200 baud), and there is
    /// nothing here for an operator to tune.
    fn audio_hz(&self) -> f32 {
        sdroxide_types::DSC_TONE_HZ
    }

    fn status(&self) -> DigiStatus {
        self.digi_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::{DSC_PHASING_SYMBOL, DscFormat, DscNature, dsc_bch};

    fn char_bits(symbol: u8) -> Vec<bool> {
        let cw = dsc_bch::encode(symbol);
        (0..10).rev().map(|i| (cw >> i) & 1 != 0).collect()
    }

    fn sequence(bits: &mut Vec<bool>, body: &[u8]) {
        for i in 0..8u8 {
            bits.extend(char_bits(DSC_PHASING_SYMBOL));
            bits.extend(char_bits(111 - i));
        }
        for &s in body {
            bits.extend(char_bits(s));
            bits.extend(char_bits(s));
        }
        bits.extend(char_bits(127));
    }

    fn modulate(bits: &[bool], rate: f64) -> Vec<f32> {
        let spb = (rate / 1200.0).round() as usize;
        let mut out = Vec::with_capacity(bits.len() * spb);
        let mut phase = 0.0f64;
        for &b in bits {
            let freq = if b { 1300.0 } else { 2100.0 };
            let inc = std::f64::consts::TAU * freq / rate;
            for _ in 0..spb {
                out.push(phase.sin() as f32);
                phase += inc;
                if phase > std::f64::consts::TAU {
                    phase -= std::f64::consts::TAU;
                }
            }
        }
        out
    }

    #[test]
    fn it_is_a_receive_only_engine() {
        let c = DscController::new(DigiConfig::default(), 48_000.0);
        assert_eq!(c.mode(), Mode::Dsc);
        assert!(Mode::Dsc.is_rx_only());
        assert!(!c.tx_burst_active());
    }

    /// A distress alert through the whole engine, with the silence after it
    /// that a real channel has — the flush that closes the sequence rides on
    /// that silence.
    #[test]
    fn a_distress_alert_reaches_the_message_list() {
        let mut body = vec![112u8];
        body.extend([36, 61, 23, 45, 60]);
        body.push(105);
        body.extend([0, 51, 30, 0, 7]);
        body.extend([12, 34]);
        let mut bits = Vec::new();
        sequence(&mut bits, &body);
        let mut audio = modulate(&bits, 48_000.0);
        audio.extend(std::iter::repeat_n(0.0f32, 48_000));

        let mut c = DscController::new(DigiConfig::default(), 48_000.0);
        for chunk in audio.chunks(1024) {
            c.on_rx_audio(chunk);
        }
        let s = c.dsc_status();
        let m = &s
            .messages
            .iter()
            .find(|h| h.message.format == DscFormat::Distress)
            .expect("a distress alert")
            .message;
        assert_eq!(m.self_mmsi, 366_123_456);
        assert_eq!(m.nature, DscNature::Sinking);
    }
}
