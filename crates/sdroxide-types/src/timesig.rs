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
//! Two families. The long-wave services key their carrier's amplitude — it is
//! reduced (or switched off) for a length of time each second that says what
//! the bit is: DCF77, MSF, WWVB and the two JJY transmitters. WWVB also keys
//! its carrier's phase with a second code, which is read alongside. ALS162 is
//! the long-wave exception: its carrier stays at full power and the seconds
//! and bits are swings of its phase alone.
//!
//! The short-wave services, WWV and WWVH, are AM broadcasts: a tick on every
//! second, voice and tones, and the time code on a 100 Hz subcarrier underneath.
//! They share their frequencies, so either may be heard alone or both at once,
//! told apart by the pitch of their ticks.
//!
//! Each station publishes its own format, cited in `sdroxide-timesig` where it
//! is decoded. CHU, Canada's short-wave service, closed on 22 June 2026 and is
//! not here: a decoder for a station that no longer transmits would only ever
//! be fed by whoever imitates it.

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
/// carrier. On long wave the mode's default passband is this wide and no
/// wider: a carrier found outside it would be one the receive filter had
/// already attenuated.
pub const TIMESIG_SEARCH_HZ: f32 = 300.0;

/// The top of the passband on short wave, Hz of receive audio. WWV and WWVH
/// put everything they send within 1.5 kHz above their carrier — the ticks at
/// 1000 and 1200 Hz, the hour tone at 1500 — so with the carrier at
/// [`TIMESIG_TONE_HZ`] this keeps all of it and the search's slack.
pub const TIMESIG_HF_TOP_HZ: f32 = 2_800.0;

/// Where long wave ends for the mode's passband: a dial above this is tuned to
/// a short-wave station.
pub const TIMESIG_HF_FROM_HZ: f64 = 1_000_000.0;

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
    /// WWVB, Fort Collins, Colorado — NIST's 60 kHz service. Both its codes
    /// are read: the amplitude code every long-wave station has, and the phase
    /// code added in 2012, which holds on to the minute further into the noise.
    Wwvb,
    /// JJY on 40 kHz, Mount Ōtakadoya, Fukushima — NICT's eastern transmitter.
    Jjy40,
    /// JJY on 60 kHz, Mount Hagane, on the Saga/Fukuoka border — NICT's
    /// western transmitter. The same time code as [`TimeStation::Jjy40`].
    Jjy60,
    /// WWV, Fort Collins, Colorado — NIST's short-wave service, on 2.5, 5, 10,
    /// 15, 20 and 25 MHz. Its seconds are 1000 Hz ticks.
    Wwv,
    /// WWVH, Kekaha, Kauaʻi, Hawaii — NIST's Pacific short-wave service, on
    /// 2.5, 5, 10 and 15 MHz, the same frequencies as [`TimeStation::Wwv`]. Its
    /// seconds are 1200 Hz ticks, which is how it is told apart.
    Wwvh,
    /// ALS162, Allouis, France — the French legal time on 162 kHz, kept by
    /// LNE-SYRTE and transmitted by ANFR; once France Inter's long-wave
    /// transmitter (TDF). A DCF77-like code sent by swinging the carrier's
    /// phase, so its amplitude never moves.
    Als162,
}

impl TimeStation {
    /// Every station, in the order the panel offers them.
    pub const ALL: [TimeStation; 8] = [
        TimeStation::Dcf77,
        TimeStation::Als162,
        TimeStation::Msf,
        TimeStation::Wwvb,
        TimeStation::Jjy40,
        TimeStation::Jjy60,
        TimeStation::Wwv,
        TimeStation::Wwvh,
    ];

    /// The call sign, as the station's operator writes it.
    pub fn label(self) -> &'static str {
        match self {
            TimeStation::Dcf77 => "DCF77",
            TimeStation::Msf => "MSF",
            TimeStation::Wwvb => "WWVB",
            TimeStation::Jjy40 => "JJY 40",
            TimeStation::Jjy60 => "JJY 60",
            TimeStation::Wwv => "WWV",
            TimeStation::Wwvh => "WWVH",
            TimeStation::Als162 => "ALS162",
        }
    }

    /// Every frequency the station transmits on, Hz, lowest first.
    pub fn carriers_hz(self) -> &'static [f64] {
        match self {
            TimeStation::Dcf77 => &[77_500.0],
            TimeStation::Msf | TimeStation::Wwvb | TimeStation::Jjy60 => &[60_000.0],
            TimeStation::Jjy40 => &[40_000.0],
            TimeStation::Wwv => {
                &[2_500_000.0, 5_000_000.0, 10_000_000.0, 15_000_000.0, 20_000_000.0, 25_000_000.0]
            }
            TimeStation::Wwvh => &[2_500_000.0, 5_000_000.0, 10_000_000.0, 15_000_000.0],
            TimeStation::Als162 => &[162_000.0],
        }
    }

    /// The carrier frequency a press of the station's chip tunes to, Hz: the
    /// only one on long wave, and 10 MHz on short wave — the one both NIST
    /// stations use that is open by day and by night across most of their
    /// coverage.
    pub fn carrier_hz(self) -> f64 {
        match self {
            TimeStation::Wwv | TimeStation::Wwvh => 10_000_000.0,
            s => s.carriers_hz()[0],
        }
    }

    /// The USB dial frequency that puts the carrier at [`TIMESIG_TONE_HZ`].
    pub fn dial_hz(self) -> f64 {
        Self::dial_for(self.carrier_hz())
    }

    /// The USB dial frequency that puts a carrier at `carrier_hz` on
    /// [`TIMESIG_TONE_HZ`].
    pub fn dial_for(carrier_hz: f64) -> f64 {
        carrier_hz - f64::from(TIMESIG_TONE_HZ)
    }

    /// A short-wave station, heard by sky wave: an AM broadcast whose seconds
    /// are ticks, rather than a long-wave carrier whose seconds are its keying.
    pub fn is_hf(self) -> bool {
        matches!(self, TimeStation::Wwv | TimeStation::Wwvh)
    }

    /// The pitch of the station's seconds ticks, Hz — for the short-wave
    /// stations, which are told apart by it.
    pub fn tick_hz(self) -> Option<f64> {
        match self {
            TimeStation::Wwv => Some(1000.0),
            TimeStation::Wwvh => Some(1200.0),
            _ => None,
        }
    }

    /// The station that shares this one's frequencies, if any: WWV and WWVH
    /// transmit on the same four, and a receiver often hears both.
    pub fn sibling(self) -> Option<TimeStation> {
        match self {
            TimeStation::Wwv => Some(TimeStation::Wwvh),
            TimeStation::Wwvh => Some(TimeStation::Wwv),
            _ => None,
        }
    }

    /// Where the transmitter is, for the reader.
    pub fn site(self) -> &'static str {
        match self {
            TimeStation::Dcf77 => "Mainflingen, Germany",
            TimeStation::Msf => "Anthorn, United Kingdom",
            TimeStation::Wwvb => "Fort Collins, Colorado, USA",
            TimeStation::Jjy40 => "Mt. Ōtakadoya, Fukushima, Japan",
            TimeStation::Jjy60 => "Mt. Hagane, Kyushu, Japan",
            TimeStation::Wwv => "Fort Collins, Colorado, USA",
            TimeStation::Wwvh => "Kekaha, Kauaʻi, Hawaii, USA",
            TimeStation::Als162 => "Allouis, France",
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
            TimeStation::Wwv => (40.6809, -105.0408),
            TimeStation::Wwvh => (21.9878, -159.7644),
            TimeStation::Als162 => (47.1694, 2.2042),
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
            TimeStation::Wwv => *b"WWV\0",
            TimeStation::Wwvh => *b"WWVH",
            // RFC 5905 still calls Allouis by its old operator's name.
            TimeStation::Als162 => *b"TDF\0",
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
    /// Uncalibrated, a measurement is late by however long that path is: around
    /// ten milliseconds on a local USB receiver, much more on a network one.
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
    /// +60 for CET, +120 for CEST, 0 for GMT, +540 for JST. The NIST stations
    /// send UTC and report 0.
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

/// Which of a station's codes a frame was read from. Only WWVB has two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TimeCode {
    /// The amplitude code — the only one every other station sends.
    #[default]
    Amplitude,
    /// WWVB's phase code alone; the amplitude code did not read that minute.
    Phase,
    /// Both, and they agreed.
    Both,
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
    /// the second — the picture of the keying the operator tunes by. ALS162's
    /// is its carrier's phase, from a tenth of a second before the second, so
    /// the swing that marks it is drawn whole.
    pub scope: Vec<u8>,
    /// The most recent frame that decoded.
    pub last_frame: Option<TimeFrame>,
    /// Which code it was read from.
    pub last_code: TimeCode,
    /// Frames that decoded and passed every check, and those that did not.
    pub frames_good: u32,
    pub frames_bad: u32,
    /// How many of [`TimeSigStatus::frames_good`] the phase code read, alone
    /// or with the amplitude code — WWVB only.
    pub frames_phase: u32,
    /// The current minute's phase-code bits, WWVB only: 0, 1, or
    /// [`TimeSigStatus::UNKNOWN`] for a second too noisy to call. Laid out
    /// like [`TimeSigStatus::symbols`].
    pub phase_symbols: Vec<u8>,
    /// How cleanly the phase code reads, dB: a bit's mean against its
    /// scatter. WWVB only, once the carrier's phase is being followed.
    pub phase_snr_db: Option<f32>,
    /// The other station on these frequencies, when its ticks are heard
    /// clearly enough to lock to — WWVH while listening to WWV, or the other
    /// way about. Both share the time code, so either decodes it; the ticks
    /// say whose time is being measured.
    pub sibling_heard: Option<TimeStation>,
    /// Whether this station's own ticks are being heard. `false` with
    /// [`TimeSigStatus::sibling_heard`] set means the other one is what is on
    /// the air here, and choosing it would get a lock.
    pub ticks_heard: bool,
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
    /// The ionospheric hops that travel time was worked out for — short wave
    /// only, where the signal goes up and down rather than along the ground.
    pub propagation_hops: Option<u8>,
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
    /// [`TimeSigStatus::symbols`] value for WWV's second 0, which carries no
    /// pulse at all: the hole that marks the minute.
    pub const HOLE: u8 = 9;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dial_puts_the_carrier_on_the_tone() {
        for s in TimeStation::ALL {
            assert_eq!(s.dial_hz() + f64::from(TIMESIG_TONE_HZ), s.carrier_hz());
            assert!(s.carriers_hz().contains(&s.carrier_hz()), "{s:?}");
            // Long wave and short wave are told apart by the dial alone.
            assert_eq!(s.dial_hz() >= TIMESIG_HF_FROM_HZ, s.is_hf(), "{s:?}");
        }
        assert_eq!(TimeStation::Dcf77.dial_hz(), 76_500.0);
        assert_eq!(TimeStation::Wwv.dial_hz(), 9_999_000.0);
    }

    #[test]
    fn the_short_wave_pair_point_at_each_other() {
        assert_eq!(TimeStation::Wwv.sibling(), Some(TimeStation::Wwvh));
        assert_eq!(TimeStation::Wwvh.sibling(), Some(TimeStation::Wwv));
        assert_eq!(TimeStation::Dcf77.sibling(), None);
        // Every WWVH frequency is a WWV one, which is why they are heard
        // together.
        for f in TimeStation::Wwvh.carriers_hz() {
            assert!(TimeStation::Wwv.carriers_hz().contains(f));
        }
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
        assert_eq!(&TimeStation::Wwv.ntp_refid(), b"WWV\0");
        assert_eq!(&TimeStation::Wwvh.ntp_refid(), b"WWVH");
        assert_eq!(&TimeStation::Als162.ntp_refid(), b"TDF\0");
        assert_eq!(TimeStation::Jjy40.ntp_refid(), TimeStation::Jjy60.ntp_refid());
    }
}
