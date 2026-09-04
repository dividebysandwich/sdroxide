use serde::{Deserialize, Serialize};

use crate::{AgcMode, Band, Mode, NrLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Vfo {
    A,
    B,
}

impl Vfo {
    /// 0 for A, 1 for B — for the per-VFO arrays the engine keeps alongside the
    /// two dials (the mode each one was left in, and its filter).
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Vfo::A => 0,
            Vfo::B => 1,
        }
    }
}

/// Receiver slot: the main receiver or the sub receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RxId {
    Main,
    Sub,
}

impl RxId {
    pub fn index(self) -> usize {
        match self {
            RxId::Main => 0,
            RxId::Sub => 1,
        }
    }
}

/// RIT/XIT style offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OffsetState {
    pub enabled: bool,
    pub hz: i32,
}

impl OffsetState {
    pub fn effective_hz(self) -> f64 {
        if self.enabled { self.hz as f64 } else { 0.0 }
    }
}

/// Squelch fully open (slider minimum).
pub const SQUELCH_OPEN_DB: f32 = -150.0;

/// Ceiling for [`RxState::manual_gain_db`], matching the AGC's own maximum.
pub const MAX_MANUAL_GAIN_DB: f32 = 120.0;

/// Per-receiver settings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RxState {
    pub mode: Mode,
    /// Passband edges in Hz relative to the VFO frequency.
    pub filter_lo: f32,
    pub filter_hi: f32,
    pub agc: AgcMode,
    pub agc_max_gain_db: f32,
    /// Fixed audio gain applied while `agc` is [`AgcMode::Off`]. Without one,
    /// switching the AGC off would leave the demodulator's raw output — for a
    /// weak SSB signal, tens of dB below anything audible.
    pub manual_gain_db: f32,
    /// 0.0..=1.0
    pub volume: f32,
    pub muted: bool,
    /// Audio gates closed below this post-filter power (dBFS).
    /// [`SQUELCH_OPEN_DB`] = always open.
    pub squelch_db: f32,
    /// Spectral noise-reduction intensity on the demodulated audio.
    pub noise_reduction: NrLevel,
    /// Adaptive auto-notch (ANC): cancel constant tone elements in the audio.
    pub auto_notch: bool,
    /// Allow WFM broadcast stereo when the 19 kHz pilot locks. Ignored in every
    /// other mode.
    pub wfm_stereo: bool,
    /// Tone squelch: the CTCSS tone or DCS code that must also be present before
    /// the audio gate opens. `None` (the default) is carrier squelch — the
    /// gate follows [`Self::squelch_db`] alone. NFM only.
    pub tone_sql: Option<crate::SubTone>,
    /// Binaural (pseudo-stereo) audio: spread the passband across the stereo
    /// image, so that pitch becomes direction and tuning a signal floats it
    /// from one ear to the other. CW and SSB ([`crate::Mode::binaural_audio`]),
    /// and read from the main receiver alone — the sub receiver *is* the other
    /// ear, and claims it whenever it is running.
    pub binaural: bool,
}

impl RxState {
    pub fn with_mode(mode: Mode) -> Self {
        let (filter_lo, filter_hi) = mode.default_filter();
        RxState {
            mode,
            filter_lo,
            filter_hi,
            agc: AgcMode::Med,
            agc_max_gain_db: 90.0,
            manual_gain_db: 20.0,
            volume: 0.5,
            muted: false,
            squelch_db: SQUELCH_OPEN_DB,
            noise_reduction: NrLevel::Off,
            auto_notch: false,
            wfm_stereo: true,
            tone_sql: None,
            binaural: false,
        }
    }
}

/// Transmit-side settings and status.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct TxState {
    pub ptt: bool,
    pub tune: bool,
    /// 0.0..=1.0 fraction of maximum drive.
    pub drive: f32,
    /// Drive used while `tune` is active.
    pub tune_drive: f32,
    /// 0.0..=1.0
    pub mic_gain: f32,
    /// Parametric EQ on the mic/modulator audio (voice modes only).
    pub eq: TxEqState,
    /// Whether the SWR guard is armed, and the ratio it trips at.
    ///
    /// Carried in the broadcast state rather than read from `config.toml` by
    /// each client, because the guard is a property of the RADIO's antenna
    /// system and the config file of a remote client is on the wrong machine
    /// entirely — the same reasoning that keeps the IARU region here.
    ///
    /// ⚠️ `TxState` derives `Default`, so `swr_limit` is `0.0` until the engine
    /// has spoken. The engine sets both at construction and clamps anything it
    /// is sent to a sane range, so a client that edits the field before the
    /// first state arrives cannot arm a zero threshold.
    pub swr_guard: bool,
    pub swr_limit: f32,
    /// Set when the SWR guard has tripped, holding the SWR that tripped it.
    /// While it is `Some`, transmit is refused until the operator acknowledges
    /// it with [`Command::ClearSwrTrip`].
    ///
    /// It lives in the shared state rather than being inferred from the notice
    /// text so that every client, native and remote, can offer the
    /// acknowledgement and can grey out transmit for the right reason. Matching
    /// on a human-readable string would break the moment the wording changed.
    pub swr_tripped: Option<f32>,
    /// Controlled-envelope SSB: how hard the voice is driven into the envelope
    /// processor, in decibels. Zero is off, and off is the default.
    ///
    /// One number rather than a switch and a level, because that is the control
    /// it is: the processor with nothing driven into it cannot do anything, so
    /// "how much" already answers "whether". Voice single sideband only — the
    /// engine applies it to USB and LSB and to nothing else, since every
    /// digital mode carries its information in the envelope this would be
    /// flattening (issue #283).
    ///
    /// Clamped by the engine to `0..=`[`crate::CESSB_MAX_DB`], so a client that
    /// sends a wild figure gets a sane one back.
    #[serde(default)]
    pub cessb_db: f32,
}

/// The most controlled-envelope compression the control offers, in decibels.
///
/// Beyond this the clipper is doing more than controlling an envelope: speech
/// driven fifteen decibels into a limiter sounds like speech driven fifteen
/// decibels into a limiter, whatever is done about its bandwidth afterwards.
///
/// Here rather than in the DSP crate so there is one number: the engine clamps
/// to it, the settings slider offers exactly this range, and the processor
/// itself clamps to it again — none of the three can come to disagree.
pub const CESSB_MAX_DB: f32 = 12.0;

/// The range a configured SWR limit is clamped to, wherever it arrives from.
///
/// Below about 1.1:1 no real antenna ever sits, so a lower figure would refuse
/// every transmission and look like a broken radio. Above 10:1 is off the top of
/// the scale the rigs actually report — the Icom SWR curve saturates exactly
/// there — so a higher one could never trip at all.
///
/// Shared rather than restated by each side: the engine clamps to it, and the
/// settings widget offers exactly this range so it can never show a figure that
/// comes back changed.
pub const SWR_LIMIT_MIN: f32 = 1.1;
pub const SWR_LIMIT_MAX: f32 = 10.0;

/// How much [`swr_tune_limit`] raises the trip limit during a tune.
pub const SWR_TUNE_LIMIT_SCALE: f32 = 2.0;

/// The SWR limit in force during a TUNE, given the operator's on-air figure.
///
/// A tune is the deliberate act of transmitting into a load that is known to be
/// mismatched — an external ATU has nothing to work on until the rig keys into
/// the very mismatch the ATU exists to remove — so it is held to a looser limit
/// than an over, and to a much longer grace period (the engine's
/// `SWR_TUNE_SETTLE_SAMPLES`). A dead feeder still reads at the top of the scale
/// and is still caught.
///
/// Derived rather than configured, so there is one number to set; it lives here
/// rather than in the engine because the settings panel shows the operator what
/// it works out to, and two copies of this formula would drift.
pub fn swr_tune_limit(limit: f32) -> f32 {
    (limit * SWR_TUNE_LIMIT_SCALE).min(SWR_LIMIT_MAX)
}

/// One band of [`TxEqState`]: corner/center frequency and gain, plus either Q
/// (the mid peaking band, where higher is narrower) or shelf slope (the
/// low/high shelf bands, the RBJ cookbook's `S`, `0.1..=1.0`; `1.0` is the
/// steepest shelf that stays monotonic).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TxEqBand {
    pub freq_hz: f32,
    pub gain_db: f32,
    pub q: f32,
}

/// Transmit parametric EQ on the microphone/modulator audio path. Voice
/// modes only (SSB/AM/FM); digital and CW carry synthesized/keyed audio that
/// never passes through it. Off by default and flat when turned on, so
/// enabling it changes nothing on the air until a band is actually moved.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TxEqState {
    pub enabled: bool,
    /// Low shelf: cuts/boosts rumble and handling noise.
    pub low: TxEqBand,
    /// Mid peak: presence/proximity shaping.
    pub mid: TxEqBand,
    /// High shelf: brightness/de-ess.
    pub high: TxEqBand,
}

impl Default for TxEqState {
    fn default() -> Self {
        TxEqState {
            enabled: false,
            low: TxEqBand { freq_hz: 300.0, gain_db: 0.0, q: 0.7 },
            mid: TxEqBand { freq_hz: 1500.0, gain_db: 0.0, q: 1.0 },
            high: TxEqBand { freq_hz: 2800.0, gain_db: 0.0, q: 0.7 },
        }
    }
}

impl TxEqState {
    /// UI range for every band's gain.
    pub const GAIN_DB_RANGE: std::ops::RangeInclusive<f32> = -15.0..=15.0;
    pub const LOW_FREQ_HZ_RANGE: std::ops::RangeInclusive<f32> = 100.0..=1000.0;
    pub const MID_FREQ_HZ_RANGE: std::ops::RangeInclusive<f32> = 300.0..=3000.0;
    pub const HIGH_FREQ_HZ_RANGE: std::ops::RangeInclusive<f32> = 1000.0..=4000.0;
    /// Q range for the mid (peaking) band only.
    pub const MID_Q_RANGE: std::ops::RangeInclusive<f32> = 0.3..=5.0;
    /// Shelf-slope range for the low/high (shelving) bands.
    pub const SHELF_SLOPE_RANGE: std::ops::RangeInclusive<f32> = 0.1..=1.0;

    /// This state with every field forced into the range the UI offers.
    ///
    /// The controls cannot leave these ranges, but two other doors into this
    /// type can: a `SetTxEq` from a WebSocket client, and the `session.json`
    /// restored at startup, which is a file an operator may have edited. Both
    /// go through here, so the coefficient maths in `sdroxide-dsp` only ever
    /// sees corner frequencies inside the voice band and a Q it was designed
    /// for.
    pub fn clamped(self) -> Self {
        let band = |b: TxEqBand,
                    freq: std::ops::RangeInclusive<f32>,
                    q: std::ops::RangeInclusive<f32>| {
            TxEqBand {
                freq_hz: b.freq_hz.clamp(*freq.start(), *freq.end()),
                gain_db: b.gain_db.clamp(*Self::GAIN_DB_RANGE.start(), *Self::GAIN_DB_RANGE.end()),
                q: b.q.clamp(*q.start(), *q.end()),
            }
        };
        TxEqState {
            enabled: self.enabled,
            low: band(self.low, Self::LOW_FREQ_HZ_RANGE, Self::SHELF_SLOPE_RANGE),
            mid: band(self.mid, Self::MID_FREQ_HZ_RANGE, Self::MID_Q_RANGE),
            high: band(self.high, Self::HIGH_FREQ_HZ_RANGE, Self::SHELF_SLOPE_RANGE),
        }
    }
}

/// Decimation off — the receiver runs at the device's own rate.
fn no_decimation() -> u32 {
    1
}

/// The narrowest span decimation may leave.
///
/// The receiver's DDC takes the span down to a ~48 kHz channel, so this is the
/// point where there is exactly one channel left in it and nothing to tune
/// across. It is also about where a decimated waterfall stops being a band
/// display and becomes a single-signal one.
pub const MIN_DECIMATED_RATE_HZ: f64 = 48_000.0;

/// The deepest decimation offered, whatever the device rate. Past this the
/// operator is asking for a spectrum display of one signal, which is what the
/// digital modes' channel waterfall already is.
pub const MAX_DECIMATION: u32 = 64;

/// The deepest decimation a device streaming `device_rate_hz` can carry without
/// falling through [`MIN_DECIMATED_RATE_HZ`]. `1` means this device has no
/// bandwidth to spare and the control has nothing to offer.
///
/// Both the engine and the RX box work from this, so a rate the operator cannot
/// reach through the UI is also one a remote client cannot ask the engine for.
pub fn max_decimation(device_rate_hz: f64) -> u32 {
    let mut factor = 1;
    while factor < MAX_DECIMATION && device_rate_hz / (factor * 2) as f64 >= MIN_DECIMATED_RATE_HZ {
        factor *= 2;
    }
    factor
}

/// How much wider than the viewport the panadapter's zoom lane runs, so the
/// decimator's transition band stays off the edge of the display.
///
/// Here rather than in the engine because the client has to work out the same
/// answer: see [`panadapter_fft_ceiling`].
pub const ZOOM_LANE_MARGIN: f64 = 1.4;

/// The power-of-two decimation a panadapter zoom lane would run at to cover a
/// `view_span_hz` window out of a `full_span_hz` stream, or `1` when the window
/// is too wide for a lane to be any narrower than the stream itself.
pub fn zoom_lane_decimation(full_span_hz: f64, view_span_hz: f64) -> u32 {
    if !(full_span_hz.is_finite() && view_span_hz.is_finite())
        || full_span_hz <= 0.0
        || view_span_hz <= 0.0
    {
        return 1;
    }
    let want = view_span_hz * ZOOM_LANE_MARGIN;
    let mut decim = 1u32;
    while decim < 1 << 14 && full_span_hz / f64::from(decim * 2) >= want {
        decim *= 2;
    }
    decim
}

/// The largest device-wide FFT worth asking for while looking at a
/// `view_span_hz` window of a `full_span_hz` stream on a `display_bins`-wide
/// panadapter — or `None` where there is no zoom lane to defer to and the
/// device-wide analyser is the only thing that can resolve the window.
///
/// Zooming used to be answered by making the device-wide FFT bigger, and that
/// is a transform over *everything the front end streams*: on a 2 Msps HackRF,
/// going from 4096 points to 32768 costs about two thirds of the whole receive
/// path's throughput, which on a small machine is a receiver that starts
/// dropping samples the moment somebody zooms in (issue #195).
///
/// Since the zoom lane exists there is a much cheaper answer: the window mixed
/// down and decimated to its own width, and analysed there. The engine builds
/// one exactly while the device-wide analyser has fewer than one bin per column
/// inside the window — so growing that analyser past this ceiling does not
/// merely cost more, it *switches the cheap lane off* and pays several times
/// over for a picture the lane had already drawn.
pub fn panadapter_fft_ceiling(
    full_span_hz: f64,
    view_span_hz: f64,
    display_bins: u32,
) -> Option<u32> {
    if zoom_lane_decimation(full_span_hz, view_span_hz) < 2 || display_bins == 0 {
        return None;
    }
    // The engine's own test, rearranged: a lane is wanted while
    // `fft * view / full < display_bins`.
    let limit = f64::from(display_bins) * full_span_hz / view_span_hz;
    if !limit.is_finite() || limit < 2.0 {
        return None;
    }
    // The largest power of two strictly under it.
    let mut fft = 1u32;
    while f64::from(fft * 2) < limit && fft < 1 << 20 {
        fft *= 2;
    }
    Some(fft)
}

/// Complete radio state snapshot. Kept small (~300 bytes serialized) so full
/// snapshots — never deltas — travel on every change, latest-wins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadioState {
    pub vfo_a_hz: f64,
    pub vfo_b_hz: f64,
    pub active_vfo: Vfo,
    pub split: bool,

    /// SDR hardware center frequency.
    pub center_hz: f64,
    /// The rate the receiver chain actually runs at: the device's own sample
    /// rate divided by [`Self::decimation`]. This — not the hardware figure —
    /// is what the span on screen, the DDCs and everything else derived from
    /// "how much bandwidth is there" are built from.
    pub sample_rate: f64,
    /// How far the raw IQ is decimated before anything downstream sees it, as
    /// a power of two. 1 is off, which is what a device streaming a rate the
    /// operator wants to see all of runs at.
    ///
    /// Defaulted for peers that predate it, which is also what a device with no
    /// IQ to decimate (a CAT rig on a sound card) always reports.
    #[serde(default = "no_decimation")]
    pub decimation: u32,

    /// Indexed by [`RxId::index`]: main, sub.
    pub rx: [RxState; 2],
    pub sub_rx_enabled: bool,
    /// Where the sub receiver listens. Independent of A/B: the sub is a second
    /// receiver, not a second view of a VFO, so swapping VFOs or retuning the
    /// dial leaves it where the operator parked it — including across switching
    /// the sub off and back on.
    ///
    /// Zero means "never placed". The engine parks it on the inactive VFO then,
    /// and again whenever a band change moves the hardware out from under it:
    /// the sub is a DDC on the same IQ as the main receiver, so a frequency
    /// outside the device passband is one it cannot hear.
    #[serde(default)]
    pub sub_rx_hz: f64,

    pub rit: OffsetState,
    pub xit: OffsetState,

    /// Working a repeater: the transmit shift, the sub-audible tone that goes
    /// out under the voice, and the 1750 Hz burst. Off in every field by
    /// default, so a station that never touches it transmits exactly where it
    /// listens with nothing under the voice.
    #[serde(default)]
    pub repeater: crate::RepeaterState,

    pub tx: TxState,
    pub band: Band,
    /// What the scanner is doing. The settings behind it travel separately —
    /// see [`crate::ScannerConfig`] — because this struct is sent whole on
    /// every change and a skip list does not belong in that.
    pub scan: crate::ScanState,
    /// Impulse noise blanker on the raw IQ stream.
    pub noise_blanker: bool,
    /// Which wideband skimmers (CW / PSK / RTTY) run, and the squelch each
    /// applies to its spots.
    pub skimmer: crate::SkimmerSettings,
    /// Which ISM-band device decoders run, and how hard they squelch.
    pub ism: crate::IsmSettings,

    /// SoapySDR RX gain elements: (name, dB).
    pub gains: Vec<(String, f64)>,
    /// SoapySDR TX gain elements: (name, dB). Default all-minimum (safety).
    pub tx_gains: Vec<(String, f64)>,
    pub antenna_rx: String,
    pub antenna_tx: String,

    /// Whether the receiver audio is being recorded to an MP3 file.
    #[serde(default)]
    pub recording: bool,
    /// Filename of the active recording (basename only), for display. `None`
    /// when not recording.
    #[serde(default)]
    pub recording_file: Option<String>,
    /// Mix both sides down to a single channel instead of RX left / TX right —
    /// for RX-only listening, or anyone who doesn't want split-ear audio.
    /// Takes effect on the next `SetRecording(true)`, not on an already-running
    /// recording.
    #[serde(default)]
    pub recording_mono: bool,
    /// Whether the raw I/Q capture is running — see
    /// [`crate::Command::SetIqRecording`]. Separate from
    /// [`Self::recording`]: the two record different things and either may run
    /// without the other.
    pub iq_recording: bool,
    /// Filename of the active I/Q capture (basename only), for display, and the
    /// megabytes written so far. `None` when nothing is being captured.
    pub iq_recording_file: Option<String>,
    pub iq_recording_mb: u32,
    /// The engine will key outside the amateur bands.
    ///
    /// Set by the `--oob-tx` command-line flag, never by anything in the UI:
    /// it is a deliberate act at launch, not a setting to be toggled by
    /// accident. Published here rather than kept in the binary so a *remote*
    /// client is warned too — the operator sitting at the UI is the one whose
    /// licence is on the line, and they may not be the one who started the
    /// engine.
    #[serde(default)]
    pub oob_tx: bool,
    /// The *radio's own* squelch threshold, as a `0..1` fraction of its scale —
    /// `0` open, `1` closed, the way the knob on the front panel reads.
    ///
    /// Kept apart from [`RxState::squelch_db`], which is sdroxide's own gate on
    /// a passband it demodulated itself. This one is a setting in the rig, and
    /// on a transceiver that hands us audio it has already gated it is the only
    /// squelch there is: the software one can close further on what got
    /// through, never open what was shut out (issue #192). Which of the two the
    /// operator is given is [`crate::DeviceCaps::commands_squelch`]'s answer.
    ///
    /// *Adopted* from the radio when the control link opens, so this starts
    /// where the operator left the rig rather than imposing a remembered level
    /// on it. Appended last: postcard numbers fields by position.
    #[serde(default)]
    pub rig_squelch: f32,
    /// How the ADS-B decoder behaves, and whether it can run at all.
    ///
    /// Here rather than only in `adsb.json` for the reason every other
    /// decoder's settings are: a remote client edits it, and the engine's reply
    /// is this field coming back changed. It is also where the engine says no —
    /// a front end that hands over demodulated audio cannot feed a 2 Msps
    /// demodulator, and [`crate::AdsbSettings::OFF`] arriving back is how the
    /// panel learns that. Appended last: postcard numbers fields by position.
    #[serde(default)]
    pub adsb: crate::AdsbSettings,
    /// The QO-100 beacon decoder: whether it runs, and how wide a search it
    /// makes around [`crate::QO100_BEACON_HZ`]. Live status (lock, offset,
    /// decoded text) is [`crate::Qo100Status`], sent separately like
    /// [`crate::IsmStatus`] — this struct is settings, sent whole on every
    /// change like [`Self::ism`].
    #[serde(default)]
    pub qo100: crate::Qo100Settings,
    /// How the VDL Mode 2 decoder behaves, and whether it can run at all.
    ///
    /// Here rather than only in `vdl2.json` for the reason [`Self::adsb`] is:
    /// a remote client edits it, and the engine's reply is this field coming
    /// back changed. It is also where the engine says no — a front end handing
    /// over demodulated audio cannot feed a D8PSK demodulator, and
    /// [`crate::Vdl2Settings::OFF`] arriving back is how the panel learns that.
    /// Appended last: postcard numbers fields by position.
    #[serde(default)]
    pub vdl2: crate::Vdl2Settings,
    /// How the AIS decoder behaves, and whether it can run at all.
    ///
    /// Here rather than only in `ais.json` for the reason [`Self::adsb`] is: a
    /// remote client edits it, and the engine's reply is this field coming back
    /// changed. It is also where the engine says no — a front end handing over
    /// demodulated audio cannot feed a GMSK demodulator, and
    /// [`crate::AisSettings::OFF`] arriving back is how the panel learns that.
    /// Appended last: postcard numbers fields by position.
    #[serde(default)]
    pub ais: crate::AisSettings,
    /// Whether the radio's separate receiving antenna is switched into the
    /// receive path.
    ///
    /// Meaningful only where [`crate::DeviceCaps::has_rx_antenna`]. Adopted
    /// from the radio and re-read whenever the radio may have moved it — it
    /// recalls the setting per band on its own — rather than asserted: writing
    /// a receive-only input nobody asked about takes an aerial out of use with
    /// nothing on screen to say so. Appended last: postcard numbers fields by
    /// position.
    #[serde(default)]
    pub rx_antenna: bool,
}

impl Default for RadioState {
    fn default() -> Self {
        let (freq, mode) = Band::M20.default_entry();
        RadioState {
            vfo_a_hz: freq,
            vfo_b_hz: freq,
            active_vfo: Vfo::A,
            split: false,
            center_hz: freq,
            sample_rate: 1_536_000.0,
            decimation: no_decimation(),
            rx: [RxState::with_mode(mode), RxState::with_mode(mode)],
            sub_rx_enabled: false,
            sub_rx_hz: 0.0, // never placed; the engine parks it on first use
            rit: OffsetState::default(),
            xit: OffsetState::default(),
            repeater: crate::RepeaterState::default(),
            // Low drive defaults: digital amplitude stays far from full
            // scale until the operator raises it deliberately.
            tx: TxState { drive: 0.1, tune_drive: 0.05, mic_gain: 0.5, ..TxState::default() },
            band: Band::M20,
            scan: crate::ScanState::default(),
            noise_blanker: false,
            skimmer: crate::SkimmerSettings::default(),
            ism: crate::IsmSettings::default(),
            adsb: crate::AdsbSettings::default(),
            gains: Vec::new(),
            tx_gains: Vec::new(),
            antenna_rx: String::new(),
            antenna_tx: String::new(),
            recording: false,
            recording_file: None,
            recording_mono: false,
            iq_recording: false,
            iq_recording_file: None,
            iq_recording_mb: 0,
            oob_tx: false,
            // Open, until the radio says otherwise: the level is adopted from
            // the rig, and until one has answered there is nothing to claim.
            rig_squelch: 0.0,
            qo100: crate::Qo100Settings::default(),
            vdl2: crate::Vdl2Settings::default(),
            ais: crate::AisSettings::default(),
            rx_antenna: false,
        }
    }
}

impl RadioState {
    /// Frequency of the currently active VFO.
    pub fn active_freq_hz(&self) -> f64 {
        match self.active_vfo {
            Vfo::A => self.vfo_a_hz,
            Vfo::B => self.vfo_b_hz,
        }
    }

    /// Receive frequency including RIT.
    pub fn rx_freq_hz(&self) -> f64 {
        self.active_freq_hz() + self.rit.effective_hz()
    }

    /// Transmit frequency including the repeater shift, XIT and split.
    pub fn tx_freq_hz(&self) -> f64 {
        let base = if self.split {
            match self.active_vfo {
                Vfo::A => self.vfo_b_hz,
                Vfo::B => self.vfo_a_hz,
            }
        } else {
            self.active_freq_hz()
        };
        // The repeater shift rides on top of split rather than instead of it:
        // both are "transmit somewhere other than the dial", they are set from
        // different places for different reasons, and a radio that silently
        // dropped one of them would transmit where neither control says.
        base + self.repeater.shift_hz() + self.xit.effective_hz()
    }
}

#[cfg(test)]
mod zoom_lane_tests {
    use super::{panadapter_fft_ceiling, zoom_lane_decimation};

    /// The ladder the engine's zoom lane walks: the narrowest output that still
    /// spans the window with room for the decimator's skirt.
    #[test]
    fn the_ladder_keeps_the_decimators_skirt_off_the_display() {
        // A window a shade under half the stream: halving it once would leave
        // the skirt inside the picture, so there is no lane to build.
        assert_eq!(zoom_lane_decimation(2_000_000.0, 800_000.0), 1);
        // A quarter of the stream: one step down still covers it with margin.
        assert_eq!(zoom_lane_decimation(2_000_000.0, 500_000.0), 2);
        assert_eq!(zoom_lane_decimation(2_000_000.0, 250_000.0), 4);
        assert_eq!(zoom_lane_decimation(2_000_000.0, 31_250.0), 32);
        // Nonsense in, no lane out.
        assert_eq!(zoom_lane_decimation(0.0, 1.0), 1);
        assert_eq!(zoom_lane_decimation(f64::NAN, 1.0), 1);
    }

    /// The ceiling a client must not ask past, and the reason it exists: the
    /// engine builds its zoom lane exactly while the device-wide analyser has
    /// fewer than one bin per column inside the window.
    #[test]
    fn the_ceiling_is_the_largest_fft_that_still_leaves_the_lane_in_charge() {
        // 2 Msps, a 500 kHz window on a 2048-column panadapter: the analyser
        // has one bin per column at 8192 points, so 4096 is the most that still
        // leaves the lane to draw it.
        assert_eq!(panadapter_fft_ceiling(2_000_000.0, 500_000.0, 2048), Some(4096));
        // Deeper in, the ceiling rises with the zoom — the analyser needs more
        // points to reach the same one-bin-per-column there.
        assert_eq!(panadapter_fft_ceiling(2_000_000.0, 62_500.0, 2048), Some(32_768));
        // A window too wide for a lane has no ceiling: the device-wide analyser
        // is the only thing that can resolve it, so let the client have it.
        assert_eq!(panadapter_fft_ceiling(2_000_000.0, 800_000.0, 2048), None);
        assert_eq!(panadapter_fft_ceiling(2_000_000.0, 500_000.0, 0), None);
    }
}
