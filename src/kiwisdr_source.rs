//! An [`IqSource`] for a KiwiSDR or Web-888.
//!
//! Two lanes from one session, and the split is the same one
//! [`crate::spyserver_source`] makes in its VFO interface — for the same
//! reason, and here with no alternative at all. A KiwiSDR has no wideband I/Q
//! to offer: what it has is a ~12 kHz user channel, which it will hand over as
//! complex baseband instead of as audio, and a 1024-bin picture of its whole
//! 0–30 MHz band. So the main panadapter is 12 kHz and honest about it, and the
//! strip above it is the receiver's own waterfall, reaching the engine through
//! [`IqSource::wide_spectrum_db`] — the seam an Icom's scope sweep and the
//! RX-888's full band already use.
//!
//! Tuning across the strip works because the engine notices the VFO has left
//! the received slice and retunes, which here means the receiver slides its
//! channel along. Deliberately *not* [`IqSource::center_is_dial`]: that flag is
//! for a front end whose centre is some rig's dial, one synthesiser doing both
//! jobs, and there is no rig here — only a window this end places. The
//! `SpyServerVfo` interface leaves it false for exactly the same reason.
//!
//! # The S-meter comes off the wire, not out of the samples
//!
//! The receiver's AGC sits ahead of the I/Q, and it is on by default because
//! the measured alternative was worse (see [`KiwiConfig::agc`]). That makes the
//! sample amplitude a measurement of the AGC as much as of the signal, so
//! [`IqSource::rx_signal_dbm`] hands over the figure the receiver puts in every
//! audio frame instead. It is already in dBm on its operator's own calibration,
//! which is why the engine returns it without applying `cal_offset_db`.
//!
//! One honest caveat: that figure is for the receiver's own ±6 kHz passband,
//! not for whatever filter sdroxide has in front of the demodulator. A narrow
//! CW filter here still reads the wideband level.
//!
//! # Receive only
//!
//! Not a missing feature. These are other people's antennas, and the trait's
//! transmit methods already default to errors.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sdroxide_kiwisdr::KiwiHandle;
use sdroxide_radio::{Complex32, IqSource, Result};
use sdroxide_types::KiwiConfig;

/// How long the receiver may deliver nothing before the link counts as dead.
/// The same window the SpyServer and `rtl_tcp` backends use: long enough that a
/// congested hop is not read as a dead receiver and reconnected out from under
/// a stream that was about to recover.
const SILENCE_BEFORE_RECONNECT: Duration = Duration::from_secs(5);

pub struct KiwiSdrSource {
    handle: KiwiHandle,
    /// The centre this end has asked for and the receiver accepted — see
    /// [`IqSource::center_hz`] on the SpyServer source for why this rather than
    /// a readback.
    center: f64,
    rx_scratch: Vec<f32>,
    label: String,
    agc: bool,
    man_gain: u8,
    /// How far the receiver's waterfall is zoomed in, as a power of two of its
    /// band — see [`KiwiConfig::wf_zoom`]. Held here as well as in the handle
    /// so [`Self::wide_span_hz`] and `current_gains` can answer without
    /// reaching into the threads.
    wf_zoom: u8,
    wide_lane: bool,
    /// Set once a refusal has been allowed through to the engine's reconnect
    /// path. See [`KiwiSdrSource::needs_reopen`], which is where the whole
    /// point of this field is.
    refusal_reported: AtomicBool,
}

impl KiwiSdrSource {
    /// Connect and start the I/Q stream.
    ///
    /// `ident` is what this end announces itself as to the receiver's owner and
    /// its other listeners — [`KiwiConfig::ident_or`] resolves it from the
    /// station callsign.
    pub fn connect(cfg: &KiwiConfig, ident: &str, center_hz: f64) -> anyhow::Result<Self> {
        let handle = KiwiHandle::connect(cfg, ident, center_hz)?;
        let label = handle.label.clone();
        tracing::info!(
            "KiwiSDR source ready: {label}, {} at {:.3} kHz",
            handle.info.describe(),
            center_hz / 1e3
        );
        Ok(KiwiSdrSource {
            center: center_hz,
            rx_scratch: Vec::new(),
            label,
            agc: cfg.agc,
            man_gain: cfg.man_gain,
            wf_zoom: cfg.wf_zoom.min(KiwiConfig::WF_ZOOM_MAX),
            wide_lane: cfg.wide_lane,
            refusal_reported: AtomicBool::new(false),
            handle,
        })
    }

    /// The band this receiver covers, from its own opening burst rather than
    /// from the directory listing — a private Kiwi is in no listing at all.
    pub fn tuning_range(&self) -> (f64, f64) {
        let i = &self.handle.info;
        if i.bandwidth_hz <= 0.0 {
            // Every KiwiSDR ever made covers this; a receiver that did not say
            // is better given the family's range than none at all.
            return (0.0, 30_000_000.0);
        }
        (i.center_hz - i.bandwidth_hz / 2.0, i.center_hz + i.bandwidth_hz / 2.0)
    }
}

impl IqSource for KiwiSdrSource {
    /// The rate the receiver stated, not the round 12 kHz it also advertises as
    /// `audio_rate`.
    ///
    /// They differ — 11998.876765 Hz on the receiver this was measured against
    /// — and it is this one the resampler has to use. Rounding it would put the
    /// audio clock a hundred parts per million out, which a long digimode
    /// decode would notice even though a listener would not.
    fn sample_rate(&self) -> f64 {
        self.handle.info.sample_rate_hz
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        let (lo, hi) = self.tuning_range();
        if hz < lo || hz > hi {
            return Err(sdroxide_radio::RadioError::Msg(format!(
                "this receiver tunes {:.3}–{:.3} MHz",
                lo / 1e6,
                hi / 1e6
            )));
        }
        self.center = hz;
        self.handle.set_center_hz(hz);
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let need = buf.len() * 2;
        if self.rx_scratch.len() < need {
            self.rx_scratch.resize(need, 0.0);
        }
        let n = self.handle.rx_read(&mut self.rx_scratch[..need]);
        let pairs = n / 2;
        if pairs == 0 {
            // Nothing yet — brief nap so the DSP loop doesn't spin hot.
            std::thread::sleep(Duration::from_millis(2));
            return Ok(0);
        }
        for p in 0..pairs {
            buf[p] = Complex32::new(self.rx_scratch[2 * p], self.rx_scratch[2 * p + 1]);
        }
        Ok(pairs)
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    /// The receiver's own waterfall, as the full-band strip.
    ///
    /// Finished dBm bins on its operator's calibration, quantised into the
    /// window this client asked for. Nothing here can be demodulated — it is a
    /// picture, like an Icom's scope sweep — and the engine's own auto-levelling
    /// decides how it is drawn.
    fn wide_spectrum_db(&mut self, out: &mut Vec<f32>) -> Option<(f64, f64)> {
        let frame = self.handle.take_waterfall()?;
        out.clear();
        out.extend_from_slice(&frame.bins);
        Some((frame.center_hz, frame.span_hz))
    }

    /// What the receiver's waterfall covers, and so how far the panadapter may
    /// be zoomed out: its whole band at zoom 0, and a power-of-two slice of it
    /// centred on the dial at anything else (issue #303).
    fn wide_span_hz(&self) -> f64 {
        if self.wide_lane { self.handle.wf_span_hz() } else { 0.0 }
    }

    /// The receiver's own S-meter. See the module note for why not the samples.
    fn rx_signal_dbm(&mut self) -> Option<f32> {
        Some(self.handle.smeter_dbm())
    }

    /// The receiver's AGC and manual gain, on the pseudo-element route the
    /// SpyServer's controls use — no `Command` variant, no `DeviceCaps` change,
    /// and the names live in [`KiwiConfig`] so the wasm settings UI can build
    /// them without seeing the driver crate.
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            KiwiConfig::AGC_ELEMENT => {
                self.agc = db >= 0.5;
                self.handle.set_agc(self.agc, self.man_gain);
            }
            KiwiConfig::MAN_GAIN_ELEMENT => {
                // The receiver's own 0–90 scale, not decibels of anything
                // stated — the protocol calls it `manGain` and says no more.
                self.man_gain = db.round().clamp(0.0, 90.0) as u8;
                self.handle.set_agc(self.agc, self.man_gain);
            }
            KiwiConfig::WF_SPEED_ELEMENT => {
                self.handle.set_wf_speed(db.round().clamp(1.0, 4.0) as u8);
            }
            KiwiConfig::WF_ZOOM_ELEMENT => {
                self.wf_zoom = db.round().clamp(0.0, f64::from(KiwiConfig::WF_ZOOM_MAX)) as u8;
                self.handle.set_wf_zoom(self.wf_zoom);
            }
            _ => {}
        }
        Ok(())
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        vec![
            (KiwiConfig::AGC_ELEMENT.to_string(), f64::from(u8::from(self.agc))),
            (KiwiConfig::MAN_GAIN_ELEMENT.to_string(), f64::from(self.man_gain)),
            (KiwiConfig::WF_ZOOM_ELEMENT.to_string(), f64::from(self.wf_zoom)),
        ]
    }

    /// Whether to reconnect — and, for this backend, mostly whether *not* to.
    ///
    /// A receiver that dropped the link because it was restarted, or because
    /// the network hiccuped, should be reconnected like any other. A receiver
    /// that *refused* — full, wrong password, inactivity timeout, kicked by its
    /// operator — must not be: it is working perfectly and has said no, and a
    /// client that answered that by reconnecting every fifteen seconds forever
    /// would be the reason sdroxide got blocked.
    ///
    /// So a refusal is allowed through exactly once. That single attempt is
    /// what carries the receiver's own words to the operator: the reconnect
    /// fails, and the engine reports the reason through its ordinary
    /// open-failure path. Afterwards this stays false and the radio sits quiet.
    fn needs_reopen(&self) -> bool {
        if self.handle.refusal().is_some() {
            return !self.refusal_reported.swap(true, Ordering::Relaxed);
        }
        !self.handle.is_alive() || self.handle.silent_for() >= SILENCE_BEFORE_RECONNECT
    }

    /// Close the session and free the receiver's channel.
    ///
    /// This one blocks on the socket threads on purpose — see
    /// [`KiwiHandle::release`]. A receiver has four or eight channels, and
    /// holding one open after the operator closed the radio is taking it from
    /// somebody.
    fn release(&mut self) {
        self.handle.release();
    }

    /// What an operator needs to know but cannot see from the panadapter.
    ///
    /// The panadapter being 12 kHz wide is the whole shape of this interface
    /// and looks exactly like a receiver that has lost its bandwidth unless it
    /// is said out loud — the same notice the SpyServer's VFO interface posts.
    fn open_status(&self) -> Option<String> {
        if let Some(r) = self.handle.refusal() {
            return Some(format!("{}: {r}", self.label));
        }
        let rate = self.handle.info.sample_rate_hz;
        let (lo, hi) = self.tuning_range();
        Some(if self.wide_lane {
            format!(
                "{}: the panadapter is the {:.1} kHz being received; the band view above it is \
                 the receiver's own waterfall over {:.0}–{:.0} kHz. Tuning across it retunes \
                 the receiver. Receive only.",
                self.label,
                rate / 1e3,
                lo / 1e3,
                hi / 1e3,
            )
        } else {
            format!(
                "{}: the panadapter is the {:.1} kHz being received, and the band view is \
                 switched off, so there is nothing wider to see. Receive only.",
                self.label,
                rate / 1e3,
            )
        })
    }
}
