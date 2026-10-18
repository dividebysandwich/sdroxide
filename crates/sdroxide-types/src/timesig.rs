//! Standard time-signal stations: which ones the receiver knows, what it
//! reports while listening to one, and the station-wide NTP server that hands
//! the result on to other machines.
//!
//! The decoder itself is `sdroxide-timesig`; this is the vocabulary the engine,
//! the panel and a remote client share, so it has to stay free of anything the
//! browser build cannot compile.
//!
//! # The stations
//!
//! The long-wave services whose time code is amplitude keyed — the carrier is
//! reduced (or switched off) for a length of time each second that says what
//! the bit is. They share everything up to the bit decision, which is why they
//! arrived together: DCF77, MSF, WWVB and the two JJY transmitters. Each
//! publishes its own format, cited in `sdroxide-timesig` where it is decoded.

use serde::{Deserialize, Serialize};

/// Where the carrier sits in the receive audio: the dial is tuned this far
/// below the station's frequency in USB, so a 77.5 kHz carrier is heard as a
/// 1 kHz tone with the dial on 76.5 kHz.
///
/// Fixed rather than a cursor the operator drags, for the reason NAVTEX's tone
/// is: these are services on a published frequency, and the useful click on
/// the waterfall is the one that brings the dial to the signal. The decoder
/// searches a few hundred hertz either side for the carrier itself, which
/// covers a front end whose reference is off by the usual few ppm — or an
/// up-converter's, which can be off by a great deal more.
pub const TIMESIG_TONE_HZ: f32 = 1000.0;

/// How far either side of [`TIMESIG_TONE_HZ`] the decoder looks for the
/// carrier. The mode's default passband is this wide and no wider: a carrier
/// found outside it would be one the receive filter had already attenuated.
pub const TIMESIG_SEARCH_HZ: f32 = 300.0;

/// A time-signal station the receiver can decode.
///
/// Appended to rather than reordered: the value rides the wire inside
/// [`TimeSigConfig`] and postcard numbers enum variants by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TimeStation {
    /// DCF77, Mainflingen, Germany — PTB's 77.5 kHz service. Heard across
    /// most of Europe by ground wave.
    #[default]
    Dcf77,
    /// MSF, Anthorn, United Kingdom — NPL's 60 kHz service.
    Msf,
    /// WWVB, Fort Collins, Colorado — NIST's 60 kHz service. Decoded here from
    /// its amplitude code; the phase code added in 2012 is not read yet.
    Wwvb,
    /// JJY on 40 kHz, Mount Ōtakadoya, Fukushima — NICT's eastern transmitter.
    Jjy40,
    /// JJY on 60 kHz, Mount Hagane, on the Saga/Fukuoka border — NICT's
    /// western transmitter. The same time code as [`TimeStation::Jjy40`].
    Jjy60,
}

impl TimeStation {
    /// Every station, in the order the panel offers them.
    pub const ALL: [TimeStation; 5] = [
        TimeStation::Dcf77,
        TimeStation::Msf,
        TimeStation::Wwvb,
        TimeStation::Jjy40,
        TimeStation::Jjy60,
    ];

    /// The call sign, as the station's operator writes it.
    pub fn label(self) -> &'static str {
        match self {
            TimeStation::Dcf77 => "DCF77",
            TimeStation::Msf => "MSF",
            TimeStation::Wwvb => "WWVB",
            TimeStation::Jjy40 => "JJY 40",
            TimeStation::Jjy60 => "JJY 60",
        }
    }

    /// The carrier frequency, Hz.
    pub fn carrier_hz(self) -> f64 {
        match self {
            TimeStation::Dcf77 => 77_500.0,
            TimeStation::Msf | TimeStation::Wwvb | TimeStation::Jjy60 => 60_000.0,
            TimeStation::Jjy40 => 40_000.0,
        }
    }

    /// The USB dial frequency that puts the carrier at [`TIMESIG_TONE_HZ`].
    pub fn dial_hz(self) -> f64 {
        self.carrier_hz() - f64::from(TIMESIG_TONE_HZ)
    }

    /// Where the transmitter is, for the reader.
    pub fn site(self) -> &'static str {
        match self {
            TimeStation::Dcf77 => "Mainflingen, Germany",
            TimeStation::Msf => "Anthorn, United Kingdom",
            TimeStation::Wwvb => "Fort Collins, Colorado, USA",
            TimeStation::Jjy40 => "Mt. Ōtakadoya, Fukushima, Japan",
            TimeStation::Jjy60 => "Mt. Hagane, Kyushu, Japan",
        }
    }

    /// The transmitter's position, degrees north and east.
    ///
    /// Used for one thing: the signal's travel time to this station, which at
    /// the speed of light is 3.3 µs a kilometre. A kilometre or two of error in
    /// these is a few microseconds and well inside anything else in the chain.
    pub fn lat_lon(self) -> (f64, f64) {
        match self {
            TimeStation::Dcf77 => (50.0156, 9.0108),
            TimeStation::Msf => (54.9116, -3.2785),
            TimeStation::Wwvb => (40.6781, -105.0469),
            TimeStation::Jjy40 => (37.3725, 140.8489),
            TimeStation::Jjy60 => (33.4653, 130.1758),
        }
    }

    /// The four-character reference identifier an NTP server locked to this
    /// station announces, from RFC 5905's table of radio clocks.
    pub fn ntp_refid(self) -> [u8; 4] {
        match self {
            TimeStation::Dcf77 => *b"DCF\0",
            TimeStation::Msf => *b"MSF\0",
            TimeStation::Wwvb => *b"WWVB",
            TimeStation::Jjy40 | TimeStation::Jjy60 => *b"JJY\0",
        }
    }
}

/// The operator's time-signal settings, carried on the tail of
/// [`crate::DigiConfig`] and so remembered per radio.
///
/// Per radio on purpose: [`TimeSigConfig::latency_ms`] is a property of the
/// receive chain it was measured on, and a second radio has its own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TimeSigConfig {
    /// The station being listened to.
    pub station: TimeStation,
    /// How long, in milliseconds, a sample takes from the antenna to the
    /// decoder — the front end's buffering and transfer, and the receive
    /// chain's filters. Subtracted from every measurement.
    ///
    /// Zero until the operator calibrates it, which the panel does in one
    /// press against a clock already known to be right (one kept by NTP, say).
    /// Uncalibrated, a measurement is late by however long that path is: a few
    /// milliseconds on a local USB receiver, much more on a network one.
    pub latency_ms: f64,
    /// How many consecutive minutes have to decode and agree with each other
    /// before the time is believed. Two is the least that means anything —
    /// one frame that passed its parity checks can still be wrong — and the
    /// floor this is held to wherever it is read.
    pub confirmations: u8,
    /// Correct the computer's clock without asking, whenever it is further out
    /// than [`TimeSigConfig::auto_threshold_ms`].
    ///
    /// Only possible where the program may set the clock without a password
    /// prompt — running as root, or on Linux with `CAP_SYS_TIME`. Elsewhere
    /// the switch is shown but has nothing to act through, and says so.
    pub auto_set: bool,
    /// How far out the clock has to be before an automatic correction is made.
    /// Below it the clock is left alone: a correction is a step, and stepping a
    /// clock every few minutes by a millisecond does more harm than the
    /// millisecond.
    pub auto_threshold_ms: f64,
}

impl Default for TimeSigConfig {
    fn default() -> Self {
        TimeSigConfig {
            station: TimeStation::default(),
            latency_ms: 0.0,
            confirmations: 2,
            auto_set: false,
            auto_threshold_ms: 100.0,
        }
    }
}

impl TimeSigConfig {
    /// [`TimeSigConfig::confirmations`] with its floor applied.
    pub fn min_confirmations(&self) -> u32 {
        u32::from(self.confirmations.max(2))
    }
}

/// One decoded minute, as the station sent it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeFrame {
    /// The UTC instant the frame names, Unix seconds — always a whole minute.
    pub utc: i64,
    /// The station's own civil time zone at that instant, minutes east of UTC:
    /// +60 for CET, +120 for CEST, 0 for GMT, +540 for JST. WWVB sends UTC and
    /// reports 0.
    pub zone_min: i16,
    /// Daylight saving time is in force, where the code says so.
    pub dst: bool,
    /// The code announces a change of daylight saving time within the hour.
    pub dst_change_soon: bool,
    /// The code announces a leap second at the end of the month or the hour.
    pub leap_second_soon: bool,
    /// UT1 − UTC in milliseconds, for the codes that carry it.
    pub dut1_ms: Option<i16>,
}

/// Who may set this computer's clock, and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ClockAuthority {
    /// The process may set the clock itself (root, or `CAP_SYS_TIME`).
    Direct,
    /// Setting the clock needs an administrator's password, which the system
    /// asks for in a dialog of its own (`pkexec` on Linux, the authorisation
    /// prompt on macOS).
    Prompt,
    /// Neither: no way to set the clock from here. A headless server not
    /// running as root lands here, and its operator's way to share the
    /// received time is the NTP server instead.
    #[default]
    Unavailable,
}

/// The built-in NTP server's settings, kept station-wide (`ntpserver.json`).
///
/// It serves the time the receiver measured — the computer's clock corrected
/// by the latest offset — not the computer's clock as it stands, so another
/// machine can be kept on radio time without this one ever being set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NtpServerConfig {
    pub enabled: bool,
    /// Interface to bind: `127.0.0.1` keeps it on this machine, `0.0.0.0`
    /// serves the network.
    pub bind: String,
    /// UDP port. 123 is what every client expects, but on Linux and older
    /// macOS a port below 1024 needs privilege to bind; chrony and ntpd can
    /// both be pointed at another one (`server 127.0.0.1 port 1123`).
    pub port: u16,
}

impl Default for NtpServerConfig {
    fn default() -> Self {
        NtpServerConfig { enabled: false, bind: "0.0.0.0".into(), port: 123 }
    }
}

impl NtpServerConfig {
    /// Bind addresses offered in the UI.
    pub const BINDS: [&'static str; 2] = ["127.0.0.1", "0.0.0.0"];

    pub fn addr(&self) -> String {
        format!("{}:{}", self.bind, self.port)
    }
}

/// What the built-in NTP server is doing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NtpServerStatus {
    /// Bound and answering.
    pub running: bool,
    /// The address it is bound to, or tried to bind to.
    pub addr: String,
    /// Why it is not running, when it was asked to.
    pub error: Option<String>,
    /// Requests answered since it started.
    pub requests: u64,
    /// Whether it is answering as a synchronised stratum 1 server — a recent
    /// enough fix is in hand — or as an unsynchronised one.
    pub synced: bool,
}

/// The engine host's side of the time-signal panel: who may set the clock,
/// what the last attempt did, and the NTP server.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClockStatus {
    pub authority: ClockAuthority,
    /// The outcome of the last attempt to set the clock, for the panel to show.
    pub last_step: Option<String>,
    /// A clock-setting attempt is waiting on the administrator's password.
    pub step_pending: bool,
    /// Something on this machine that will put the clock back after we set it
    /// — systemd-timesyncd, chrony, ntpd — named so the operator knows.
    pub sync_daemon: Option<String>,
    pub ntp: NtpServerStatus,
}

/// What the time-signal receiver is doing. Carried in
/// [`crate::DigiStatus::timesig`] while the mode is selected.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TimeSigStatus {
    pub station: TimeStation,
    /// The carrier, as found in the receive audio, Hz. `None` until found.
    pub carrier_hz: Option<f32>,
    /// The carrier's level, dB relative to full scale.
    pub level_db: f32,
    /// How cleanly the keying stands out of the noise: the depth of the
    /// modulation against the scatter of the envelope, in dB.
    pub snr_db: f32,
    /// The start of each second has been found and is being tracked.
    pub second_lock: bool,
    /// Where the current second is in the minute, once a frame has been read.
    pub second_of_minute: Option<u8>,
    /// The symbols heard in the current minute, oldest first: 0 and 1 for
    /// bits (0–3 for MSF's bit pairs, A + 2·B), [`TimeSigStatus::MARKER`] for
    /// a marker, [`TimeSigStatus::UNKNOWN`] for a second that could not be
    /// read.
    pub symbols: Vec<u8>,
    /// The envelope of an average second, 0–255, 100 points from the start of
    /// the second — the picture of the keying the operator tunes by.
    pub scope: Vec<u8>,
    /// The most recent frame that decoded.
    pub last_frame: Option<TimeFrame>,
    /// Frames that decoded and passed every check, and those that did not.
    pub frames_good: u32,
    pub frames_bad: u32,
    /// Consecutive frames that agree with each other. The time is believed
    /// once this reaches [`TimeSigConfig::confirmations`].
    pub confirmed: u32,
    /// How far this computer's clock is behind the received time, ms —
    /// positive means the clock is slow. `None` until the time is believed.
    pub offset_ms: Option<f64>,
    /// The spread of the measurements behind [`TimeSigStatus::offset_ms`], ms.
    pub jitter_ms: Option<f64>,
    /// The signal's travel time from the transmitter, ms, already added in.
    /// `None` without a locator to measure from.
    pub propagation_ms: Option<f64>,
    /// The distance it was computed from, km.
    pub distance_km: Option<f64>,
    /// The engine host's clock and NTP server.
    pub clock: ClockStatus,
}

impl TimeSigStatus {
    /// [`TimeSigStatus::symbols`] value for a marker second.
    pub const MARKER: u8 = 8;
    /// [`TimeSigStatus::symbols`] value for a second that could not be read.
    pub const UNKNOWN: u8 = 255;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dial_puts_the_carrier_on_the_tone() {
        for s in TimeStation::ALL {
            assert_eq!(s.dial_hz() + f64::from(TIMESIG_TONE_HZ), s.carrier_hz());
        }
        assert_eq!(TimeStation::Dcf77.dial_hz(), 76_500.0);
    }

    #[test]
    fn fewer_than_two_confirmations_is_not_a_setting() {
        let cfg = TimeSigConfig { confirmations: 0, ..TimeSigConfig::default() };
        assert_eq!(cfg.min_confirmations(), 2);
        let cfg = TimeSigConfig { confirmations: 4, ..TimeSigConfig::default() };
        assert_eq!(cfg.min_confirmations(), 4);
    }

    #[test]
    fn refids_are_the_rfc_5905_ones() {
        assert_eq!(&TimeStation::Dcf77.ntp_refid(), b"DCF\0");
        assert_eq!(&TimeStation::Wwvb.ntp_refid(), b"WWVB");
        assert_eq!(TimeStation::Jjy40.ntp_refid(), TimeStation::Jjy60.ntp_refid());
    }
}
