//! An allocation-free fingerprint of everything the announcer can talk about.
//!
//! Modelled on `RigDigest` in the engine, which does the same job for rigctld.
//! One difference matters: where that one compares floats by bit pattern,
//! this one **quantises** them to the resolution they are *spoken* at. The
//! announcer says "thirty five percent", so a drive change of 0.0004 is not a
//! change at all — and that alone stops a slider drag producing a difference on
//! every pixel. The settle timers then deal with what is left.

use sdroxide_types::{AgcMode, Band, Mode, NrLevel, RadioState, SegmentKind, Vfo, segment_kind_at};

/// Everything worth announcing, reduced to `Copy` scalars that derive `Eq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeechDigest {
    // ── discrete: announced as soon as they change ──────────────────────
    pub vfo: Vfo,
    pub split: bool,
    pub mode: Mode,
    pub band: Band,
    pub agc: AgcMode,
    pub nr: NrLevel,
    pub ptt: bool,
    pub tune: bool,
    pub muted: bool,
    pub noise_blanker: bool,
    pub auto_notch: bool,
    pub rit_on: bool,
    pub xit_on: bool,
    pub sub_rx: bool,
    pub scan_running: bool,
    pub scan_holding: bool,
    pub recording: bool,
    /// Whether the frequency we would *transmit* on sits in an amateur band.
    pub in_ham_band: bool,
    /// Which sub-segment the receiver is in.
    pub segment: Option<SegmentKind>,
    /// Receive antenna, hashed — a change here means the front end was
    /// reconfigured.
    pub antenna_rx: u64,

    // ── continuous: a settle timer decides when these speak ─────────────
    /// Active VFO frequency, whole Hz.
    pub freq_hz: i64,
    pub rit_hz: i32,
    pub xit_hz: i32,
    /// Passband edges, whole Hz.
    pub filter_lo: i32,
    pub filter_hi: i32,
    /// 0..=100.
    pub drive_pct: u8,
    pub tune_drive_pct: u8,
    pub mic_pct: u8,
    pub volume_pct: u8,
    /// Whole dB.
    pub agc_max_gain_db: i16,
    pub manual_gain_db: i16,
    pub squelch_db: i16,
}

impl SpeechDigest {
    pub fn of(s: &RadioState) -> Self {
        let pct = |v: f32| (v.clamp(0.0, 1.0) * 100.0).round() as u8;
        let rx = &s.rx[0];
        SpeechDigest {
            vfo: s.active_vfo,
            split: s.split,
            mode: rx.mode,
            band: s.band,
            agc: rx.agc,
            nr: rx.noise_reduction,
            ptt: s.tx.ptt,
            tune: s.tx.tune,
            muted: rx.muted,
            noise_blanker: s.noise_blanker,
            auto_notch: rx.auto_notch,
            rit_on: s.rit.enabled,
            xit_on: s.xit.enabled,
            sub_rx: s.sub_rx_enabled,
            scan_running: s.scan.running,
            scan_holding: s.scan.holding,
            recording: s.recording,
            // The transmit frequency, not the active one: with split on, the
            // frequency your licence cares about is the one you would key on.
            in_ham_band: Band::containing(s.tx_freq_hz()).is_amateur(),
            segment: segment_kind_at(s.rx_freq_hz()),
            antenna_rx: fnv1a(s.antenna_rx.as_bytes()),
            freq_hz: s.active_freq_hz().round() as i64,
            rit_hz: s.rit.hz,
            xit_hz: s.xit.hz,
            filter_lo: rx.filter_lo.round() as i32,
            filter_hi: rx.filter_hi.round() as i32,
            drive_pct: pct(s.tx.drive),
            tune_drive_pct: pct(s.tx.tune_drive),
            mic_pct: pct(s.tx.mic_gain),
            volume_pct: pct(rx.volume),
            agc_max_gain_db: rx.agc_max_gain_db.round() as i16,
            manual_gain_db: rx.manual_gain_db.round() as i16,
            squelch_db: rx.squelch_db.round() as i16,
        }
    }
}

/// FNV-1a. Used only to notice that a string changed, never to identify it.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pixel_of_slider_travel_is_not_a_change() {
        let mut s = RadioState::default();
        s.tx.drive = 0.350;
        let a = SpeechDigest::of(&s);
        s.tx.drive = 0.3504;
        assert_eq!(a, SpeechDigest::of(&s), "quantisation let a sub-percent change through");
        s.tx.drive = 0.36;
        assert_ne!(a, SpeechDigest::of(&s), "a real one-percent change was swallowed");
    }

    #[test]
    fn the_band_check_follows_the_transmit_frequency() {
        let mut s = RadioState::default();
        // Receiving inside 20 m, but split to transmit out of band.
        s.vfo_a_hz = 14_074_000.0;
        s.vfo_b_hz = 13_900_000.0;
        s.active_vfo = sdroxide_types::Vfo::A;
        s.split = true;
        assert!(!SpeechDigest::of(&s).in_ham_band);
        s.split = false;
        assert!(SpeechDigest::of(&s).in_ham_band);
    }

    #[test]
    fn hashing_notices_an_antenna_change() {
        let mut s = RadioState::default();
        s.antenna_rx = "ANT1".into();
        let a = SpeechDigest::of(&s);
        s.antenna_rx = "ANT2".into();
        assert_ne!(a.antenna_rx, SpeechDigest::of(&s).antenna_rx);
    }
}
