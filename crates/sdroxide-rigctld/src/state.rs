//! The slice of radio state rigctld clients can see, plus the Hamlib mode
//! vocabulary it is reported in.

use sdroxide_types::{Band, Mode, Vfo};

/// What the protocol layer needs to answer a query. Refreshed by the engine on
/// every state change; client threads only ever read it.
#[derive(Debug, Clone, PartialEq)]
pub struct RigState {
    pub vfo_a_hz: f64,
    pub vfo_b_hz: f64,
    pub active_vfo: Vfo,
    pub split: bool,
    pub mode: Mode,
    /// Passband edges in Hz relative to the carrier.
    pub filter_lo: f32,
    pub filter_hi: f32,
    pub ptt: bool,
    pub tune: bool,
    pub rit_hz: i32,
    pub xit_hz: i32,
    /// 0.0..=1.0, reported as Hamlib's `RFPOWER`.
    pub drive: f32,
    /// 0.0..=1.0, reported as Hamlib's `AF`.
    pub volume: f32,
    /// 0.0..=1.0, reported as Hamlib's `MICGAIN`.
    pub mic_gain: f32,
    /// Current band, so `vfo_op BAND_UP`/`BAND_DOWN` know where to step from.
    pub band: Band,
    pub muted: bool,
    /// Signal strength in dBm, reported as Hamlib's `STRENGTH` (S9 = −73 dBm).
    pub strength_dbm: i32,
    pub noise_blanker: bool,
    pub noise_reduction: bool,
    pub auto_notch: bool,
    /// Whether this radio can transmit at all. False empties the TX range list
    /// in `\dump_state`, which makes Hamlib itself refuse to key.
    pub can_tx: bool,
    /// Tunable ranges in Hz, from the device capabilities.
    pub rx_ranges: Vec<(f64, f64)>,
    pub tx_ranges: Vec<(f64, f64)>,
}

impl Default for RigState {
    fn default() -> Self {
        RigState {
            vfo_a_hz: 14_074_000.0,
            vfo_b_hz: 14_074_000.0,
            active_vfo: Vfo::A,
            split: false,
            mode: Mode::Usb,
            filter_lo: 150.0,
            filter_hi: 2850.0,
            ptt: false,
            tune: false,
            rit_hz: 0,
            xit_hz: 0,
            drive: 0.0,
            volume: 0.5,
            mic_gain: 0.5,
            band: Band::M20,
            muted: false,
            strength_dbm: -73,
            noise_blanker: false,
            noise_reduction: false,
            auto_notch: false,
            can_tx: false,
            rx_ranges: Vec::new(),
            tx_ranges: Vec::new(),
        }
    }
}

impl RigState {
    pub fn freq(&self, vfo: Vfo) -> f64 {
        match vfo {
            Vfo::A => self.vfo_a_hz,
            Vfo::B => self.vfo_b_hz,
        }
    }

    /// The VFO transmit uses: the other one when split, otherwise the active one.
    pub fn tx_vfo(&self) -> Vfo {
        if self.split {
            match self.active_vfo {
                Vfo::A => Vfo::B,
                Vfo::B => Vfo::A,
            }
        } else {
            self.active_vfo
        }
    }

    pub fn passband_hz(&self) -> i32 {
        (self.filter_hi - self.filter_lo).abs().round() as i32
    }
}

/// The Hamlib mode string we report for one of ours.
///
/// Every mode driven by the digital engine (FT8/FT4, the keyboard modems,
/// SSTV, RF Paint, RADE) reports `PKTUSB`, because that is what they are on the
/// air: upper sideband with data in the audio. See [`from_hamlib_mode`] for why
/// that matters. RIFP is the exception — its CPFSK profile keys the carrier —
/// so it reports `PKTFM`.
///
/// From the mode alone, so it cannot see the two whose sideband follows the
/// band (`Mode::sideband_follows_band`): SSTV and RADE on 160/80/40 m really
/// are on the lower sideband and are still reported here as `PKTUSB`. What the
/// radio does is unaffected — the sideband is decided in the engine and at the
/// rig's own control layer — and the no-op rule in `set_mode` compares against
/// whatever this says, so a client echoing the mode back still changes nothing.
pub fn to_hamlib_mode(m: Mode) -> &'static str {
    match m {
        Mode::Lsb => "LSB",
        Mode::Usb => "USB",
        Mode::Cw => "CW",
        // Hamlib has no DRM mode, and the nearest true statement is AM:
        // the carrier is on the dial and the channel is about as wide.
        // It is also how the signal would reach an outboard decoder — off
        // a receiver set to AM.
        Mode::Am | Mode::Drm => "AM",
        Mode::Sam => "SAM",
        Mode::Nfm => "FM",
        // RIFP, VHF packet and VHF SSTV are data on an FM carrier, not on a
        // sideband.
        Mode::Rifp | Mode::Packet | Mode::Aprs | Mode::SstvFm | Mode::RttyFm => "PKTFM",
        // No rig has an ADS-B mode; a remote hamlib client asking is told the
        // widest FM there is, which is at least the right kind of receiver.
        Mode::Wfm | Mode::Adsb | Mode::Vdl2 | Mode::Ais => "WFM",
        Mode::Digu => "PKTUSB",
        Mode::Digl => "PKTLSB",
        Mode::Dsb => "DSB",
        // Hamlib has no independent-sideband mode. AM is the nearest true
        // statement about the signal — carrier on the dial, both sidebands
        // wanted — and it is the setting an outboard decoder would want.
        Mode::Isb => "AM",
        Mode::Spec => "SPEC",
        Mode::Rtty => "RTTY",
        Mode::Ft8
        | Mode::Js8
        | Mode::Wspr
        | Mode::Ft4
        | Mode::Ft2
        | Mode::Psk
        | Mode::Sstv
        | Mode::Wefax
        | Mode::Navtex
        | Mode::Olivia
        | Mode::Thor
        | Mode::Fsq
        | Mode::Hell
        | Mode::RfPaint
        | Mode::PacketHf
        | Mode::Rade => "PKTUSB",
    }
}

/// Parse a Hamlib mode string.
///
/// The reverse-sideband variants (`CWR`, `RTTYR`) fold onto their base mode:
/// sdroxide has no separate reverse mode, and refusing them would break
/// clients that only offer that spelling. `DIGU`/`DIGL`/`DATA-U`/`DATA-L` are
/// accepted because operators type them, but they are never emitted — Hamlib
/// has no such names and would not parse them back.
pub fn from_hamlib_mode(s: &str) -> Option<Mode> {
    Some(match s.to_ascii_uppercase().as_str() {
        "USB" => Mode::Usb,
        "LSB" => Mode::Lsb,
        "CW" | "CWR" => Mode::Cw,
        "AM" | "AMN" => Mode::Am,
        "AMS" | "SAM" => Mode::Sam,
        "FM" | "FMN" | "PKTFM" => Mode::Nfm,
        "WFM" => Mode::Wfm,
        "DSB" => Mode::Dsb,
        "RTTY" | "RTTYR" => Mode::Rtty,
        "PKTUSB" | "DIGU" | "DATA-U" | "DATA_U" => Mode::Digu,
        "PKTLSB" | "DIGL" | "DATA-L" | "DATA_L" => Mode::Digl,
        "PSK" | "PSKR" => Mode::Psk,
        "SPEC" => Mode::Spec,
        _ => return None,
    })
}

/// Passband edges for a mode at a requested Hamlib width, keeping the mode's
/// own sideband placement.
pub fn filter_for(mode: Mode, width_hz: i32) -> (f32, f32) {
    let w = (width_hz as f32).clamp(50.0, mode.max_filter_hz());
    let (dlo, dhi) = mode.default_filter();
    if dlo >= 0.0 {
        // Upper-sideband style: keep the low edge, move the high one.
        (dlo, dlo + w)
    } else if dhi <= 0.0 {
        // Lower sideband: keep the high edge, move the low one.
        (dhi - w, dhi)
    } else {
        // Symmetric about the carrier (AM/FM/DSB).
        (-w / 2.0, w / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The interop rule that keeps FT8 working: WSJT-X polls the mode every
    /// few seconds and re-sends what it read. Whatever we report for a digital
    /// mode must therefore parse back to something we treat as unchanged.
    #[test]
    fn digital_modes_report_pktusb() {
        for m in [
            Mode::Ft8,
            Mode::Ft4,
            Mode::Ft2,
            Mode::Psk,
            Mode::Sstv,
            Mode::Fsq,
            Mode::Hell,
            Mode::Rade,
        ] {
            assert_eq!(to_hamlib_mode(m), "PKTUSB", "{m:?}");
        }
    }

    #[test]
    fn every_mode_reports_a_name_hamlib_can_parse_back() {
        for m in Mode::ALL {
            let s = to_hamlib_mode(m);
            assert!(from_hamlib_mode(s).is_some(), "{m:?} reports unparseable {s}");
        }
    }

    #[test]
    fn reverse_sideband_variants_fold_onto_the_base_mode() {
        assert_eq!(from_hamlib_mode("CWR"), Some(Mode::Cw));
        assert_eq!(from_hamlib_mode("RTTYR"), Some(Mode::Rtty));
        assert_eq!(from_hamlib_mode("pktusb"), Some(Mode::Digu));
        assert_eq!(from_hamlib_mode("NOPE"), None);
    }

    #[test]
    fn filter_keeps_the_mode_sideband() {
        let (lo, hi) = filter_for(Mode::Usb, 2400);
        assert!(lo > 0.0 && hi > lo);
        assert_eq!((hi - lo).round(), 2400.0);

        let (lo, hi) = filter_for(Mode::Lsb, 2400);
        assert!(hi < 0.0 && lo < hi);
        assert_eq!((hi - lo).round(), 2400.0);

        let (lo, hi) = filter_for(Mode::Am, 6000);
        assert_eq!(lo, -3000.0);
        assert_eq!(hi, 3000.0);
    }

    #[test]
    fn tx_vfo_follows_split() {
        let mut s = RigState { active_vfo: Vfo::A, ..RigState::default() };
        assert_eq!(s.tx_vfo(), Vfo::A);
        s.split = true;
        assert_eq!(s.tx_vfo(), Vfo::B);
    }
}
