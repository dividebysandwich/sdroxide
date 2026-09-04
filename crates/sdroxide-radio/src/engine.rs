//! `RadioCore`: the engine thread that owns the IQ source, all DSP, and the
//! authoritative [`RadioState`].
//!
//! M4 scope: main + sub receiver chains mixed to stereo (main left, sub
//! right), all demodulators, band-stack registers, memory channels
//! (persisted engine-side), hardware gain/antenna control, and
//! viewport-aware spectrum frames. TX arrives in M5.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use tracing::{debug, info, trace, warn};

use sdroxide_adsb::{AdsbAction, AdsbController};
use sdroxide_ais::{AisAction, AisController};
use sdroxide_config::BandStacks;
use sdroxide_digi::{
    AprsController, CwController, DigiAction, DigiController, DigiEngine, FsqController,
    HellController, Js8Controller, NavtexController, PacketController, RadeController,
    RfPaintController, RifpController, SstvController, TextModemController, WefaxController,
    WsprController,
};
use sdroxide_drm::DrmDemod;
use sdroxide_dsp::{
    AdcMeter, Agc, AutoNotch, Binaural, Cessb, DcBlock, Ddc, Decimator, DeepFilterNr, Demodulator,
    Duc, Modulator, MonoResampler, Nco, NeuralNr, NoiseBlanker, ParametricEq, SpecBleachNr,
    SpectralNr, SpectrumAnalyzer, StereoResampler, SubToneGen, ToneBurst, channel_target,
    make_demod, make_modulator,
};
use sdroxide_ism::{IsmAction, IsmController};
use sdroxide_qo100::Qo100Controller;
use sdroxide_rigctld::{RigState, RigctldController};
use sdroxide_skimmer::{SkimmerAction, SkimmerController};
use sdroxide_tci::server::{ServerRequest, TciServerController, TciStateSnapshot};
use sdroxide_types::{
    AgcMode, Band, BandStackEntry, Command, DeviceCaps, DigiConfig, Direction, ImageKind,
    MemoryChannel, MemoryFolder, Meters, Mode, NrEngine, NrLevel, RadioEvent, RadioState,
    RepeaterState, RigctldConfig, RxId, RxState, ScanKind, ScanResume, SpectrumConfig,
    SpectrumFrame, TciServerConfig, TxEqState, TxMeters, Vfo,
};
use sdroxide_vdl2::{Vdl2Action, Vdl2Controller};

use crate::recorder::{Recorder, RecordingChannels};
use crate::voice::VoiceKeyer;
use crate::{Complex32, ControlUpdate, IqSource};

/// The most waterfall rows an engine will hold for a client that has stopped
/// collecting them.
///
/// A hitch on the client — a tab switched away, a compositor stall — must cost
/// the rows it happened over and no more: replaying a second of backlog into
/// the texture at once would draw a second of band in one repaint and put the
/// time axis out by that much. At the fastest row clock this is an eighth of a
/// second.
const MAX_BATCH_ROWS: usize = 64;

/// Bins in a full-band frame.
///
/// A constant where the main panadapter's width is not, because this lane does
/// not follow anybody's screen: the strip that draws it keeps its own
/// 1024-column history, never zooms, and is a few dozen pixels tall. This is
/// already twice what it can show, and `sdroxide_spyserver`'s
/// `FFT_DISPLAY_PIXELS` is matched to it.
///
/// The main panadapter's width is [`sdroxide_types::SpectrumConfig::bins`].
pub const WIDE_BINS: usize = 2048;

/// The zoom lane's FFT size for a display `display_bins` columns wide.
///
/// Two dilutions sit between a transform and a column, and both have to be paid
/// for. The decimation ladder is powers of two, so the lane's output lands
/// between [`sdroxide_types::ZOOM_LANE_MARGIN`] and twice that times the
/// viewport — the bins inside the viewport are between a 2.8th and a 1.4th of
/// these. And the viewport is itself about twice the window actually on screen,
/// the client asking for slack so that panning inside it needs no
/// reconfiguration. So a column gets between a 5.6th and a 2.8th of the
/// transform, and eight times the columns is what puts at least one bin under
/// every one of them at the wide end of the ladder.
///
/// It used to be twice the columns, which counted only the first dilution and
/// left about half a bin per column on screen. That was invisible while the
/// client answered a zoom by growing the *device-wide* FFT instead — but that
/// transform runs at the front end's whole rate and is several times the price
/// of this one, which is what made zooming in on a 2 Msps HackRF start dropping
/// samples (issue #195). This lane now resolves the window on its own, so it
/// has to resolve it properly.
///
/// Floored at 4096 and capped at 32768: past there a single transform covers
/// more signal than the lane's hop can hide (see [`MAX_HOP_DIV`]).
///
/// How much *signal* that transform looks at is a separate question, answered
/// by [`analysis_window`] — the padding is what lets this be as large as the
/// screen wants without the picture being an average of five seconds of band.
fn zoom_lane_fft(display_bins: usize) -> usize {
    (display_bins * 8).next_power_of_two().clamp(4096, 32_768)
}

/// How many distinct things a transform has to be able to tell apart across the
/// window on screen.
///
/// Not one per column. A column is a *drawing* unit and there can be thousands
/// of them on a wide panel; this is a *measurement* unit, and past a certain
/// density another one buys nothing an eye can use while costing a doubling of
/// the time each row of the waterfall smears over. A thousand of them across
/// the window is finer than any signal sdroxide draws is worth resolving, and
/// on the FT8 sub-band it lands on exactly the transform WSJT-X's own waterfall
/// uses — 2.9 Hz bins over a third of a second — which is the picture operators
/// are comparing sdroxide's against.
const RESOLVED_PER_VIEW: f64 = 1024.0;

/// The analysis window, in samples, for a lane running at `rate_hz` whose
/// visible window is `view_span_hz` wide, drawn on an `fft_size`-point
/// transform.
///
/// A transform cannot resolve finer than the reciprocal of the time it covers,
/// so resolution and time smear are one dial, not two — and until issue #302
/// that dial was set by the *screen*. A 3.7 kHz FT8 window across a 2560-column
/// panel asks for 1.4 Hz bins; at the few kilohertz a zoomed lane runs at, a
/// transform that fine is looking at seconds of band at a time, and every FT8
/// transmission in it is drawn as one unbroken bar. Turning the FFT size up
/// made it worse, which is what the report said.
///
/// So the window is chosen from the *signal* instead — enough to resolve
/// [`RESOLVED_PER_VIEW`] elements across what is on screen — and the transform
/// is zero-padded out to whatever the display asked for
/// ([`SpectrumAnalyzer::with_window`]). The screen still gets a smooth trace
/// with a point under every column; the waterfall gets its time back.
///
/// The answer scales with `rate_hz / view_span_hz`, so a deep zoom keeps every
/// bit of the resolution it went looking for — the rule is "enough to resolve
/// what is on screen", not a fixed number of milliseconds.
///
/// Only for a lane drawn *narrower* than what it is fed. A window as wide as
/// the lane comes back with the transform it was given: there is no padding to
/// be had there and nothing to trade for it.
fn analysis_window(fft_size: usize, rate_hz: f64, view_span_hz: f64) -> usize {
    if !(rate_hz.is_finite() && view_span_hz.is_finite()) || view_span_hz <= 0.0 || rate_hz <= 0.0 {
        return fft_size;
    }
    // A lane whose window is the whole of what it is fed has nothing to trade:
    // every bin it computes is on screen, the padding would interpolate between
    // points that are already a column apart, and shortening the transform
    // would simply throw resolution away. That is the device-wide panadapter,
    // and it is left exactly as it was.
    if view_span_hz >= rate_hz {
        return fft_size;
    }
    let want = (rate_hz / view_span_hz * RESOLVED_PER_VIEW).ceil();
    if !want.is_finite() || want >= fft_size as f64 {
        return fft_size;
    }
    (want as usize).next_power_of_two().clamp(256, fft_size)
}

/// Where a `span_hz` window on `center_hz` sits inside a `full_span_hz` band on
/// `full_center_hz`, as the ascending pair of fractions
/// [`SpectrumAnalyzer::seed_from`] takes.
///
/// Outside `0.0..=1.0` where the window is not wholly inside the band, which is
/// what `seed_from` refuses on: there is no picture of what it cannot see.
fn span_fraction(
    full_center_hz: f64,
    full_span_hz: f64,
    center_hz: f64,
    span_hz: f64,
) -> (f64, f64) {
    if full_span_hz <= 0.0 {
        return (0.0, 0.0);
    }
    let base = full_center_hz - full_span_hz / 2.0;
    let lo = (center_hz - span_hz / 2.0 - base) / full_span_hz;
    (lo, lo + span_hz / full_span_hz)
}

/// How many transforms a waterfall row is built from, when there are enough to
/// choose. More than one so a signal straddling a window boundary is still seen
/// whole in a neighbouring window, and so the peak hold has something to pick a
/// maximum from; not many more, because every one past that is folded into the
/// same row and thrown away.
const TRANSFORMS_PER_ROW: f64 = 4.0;

/// The overlap to run an analyser at: the divisor of its FFT size that gives
/// about [`TRANSFORMS_PER_ROW`] transforms per waterfall row.
///
/// Overlap is not free and it is not uniformly useful. It exists so a signal
/// that lands across a window boundary is still seen whole in the next window,
/// which matters enormously when transforms are scarce — a zoomed lane running
/// at a few kilohertz fills a 4096-point window barely twice a second, and the
/// eighth-hop it has always used is what puts rows on its waterfall at all.
///
/// It is close to pure waste when transforms are abundant. An RX-888 at
/// 8.1 MHz through a 4096-point window runs 3955 transforms a second at the
/// customary half-hop; the waterfall consumes at most 224 of them and the peak
/// hold folds the rest into the same rows. That was measured at 18% of the
/// process — the single largest thing in the DSP thread after the receive
/// chain — for detail no display can show.
///
/// So the overlap follows the rate and the scroll speed rather than being a
/// constant. The clamp keeps both ends honest: never coarser than one window
/// per hop (which would skip samples outright, and a signal shorter than a
/// window could then be missed entirely), and never finer than an eighth,
/// which is what the zoom lane wants and what this returns for it unchanged.
fn hop_div_for(rate_hz: f64, fft_size: usize, rows_per_sec: f64) -> usize {
    if !rate_hz.is_finite() || rate_hz <= 0.0 || rows_per_sec <= 0.0 {
        return 2;
    }
    let want_transforms = rows_per_sec * TRANSFORMS_PER_ROW;
    let hop = (rate_hz / want_transforms).max(1.0);
    ((fft_size as f64 / hop).ceil() as usize).clamp(1, MAX_HOP_DIV)
}

/// The device-wide panadapter analyser, at the overlap its rate and the
/// operator's scroll speed call for — see [`hop_div_for`].
///
/// `view_span_hz` is the window this analyser is actually going to be *drawn*
/// in, where that is narrower than everything it is fed — which in practice
/// means audio mode, where the lane runs at the sound card's rate and the
/// picture is the rig's few kilohertz of passband. Naming it lets the transform
/// be zero-padded rather than made to cover more than a fifth of a second of
/// signal, and the FFT setting stop being a control over how badly the
/// waterfall smears (issue #302). `None` where the lane draws all of what it
/// sees: a device-wide panadapter resolves the whole span and there is nothing
/// to trade away.
fn build_analyzer(
    fft_size: usize,
    rate_hz: f64,
    avg_tc: f32,
    rows_per_sec: f64,
    view_span_hz: Option<f64>,
) -> SpectrumAnalyzer {
    let window = view_span_hz.map_or(fft_size, |v| analysis_window(fft_size, rate_hz, v));
    SpectrumAnalyzer::with_window(
        fft_size,
        window,
        rate_hz,
        avg_tc,
        hop_div_for(rate_hz, window, rows_per_sec),
    )
}

/// Workers in the pool the block loop forks its lanes onto.
///
/// One block of receive samples divides into three independent pieces — the
/// device-wide panadapter analyser, the panadapter's zoom lane, and the receive
/// chain — and the third of them runs on the engine thread itself, which is
/// where it has to stay (see [`Engine::process_block`]). So two workers, not
/// three. None of the three writes anything another reads, and on a wide front
/// end each is tens of percent of a core: an RX-888 at 32.4 Msps ran all three
/// down one thread and left thirty other cores idle.
///
/// A pool of this crate's own rather than rayon's global one, which is sized to
/// the machine: this forks about two thousand times a second on a fast front
/// end, and thirty-two workers waking to look for two pieces of work cost more
/// than they finish. Measured: 2.3 µs a fork on a pool of three, 9.8 µs on the
/// global one.
const LANE_WORKERS: usize = 2;

/// Cores below which the block loop stays on one thread.
///
/// Forking is only ever worth it where there is somewhere for the work to go.
/// A Raspberry Pi running the GUI, the receive chain and a compositor on four
/// cores has no spare one to steal a lane onto, and the hand-off would be pure
/// loss.
const LANE_POOL_MIN_CORES: usize = 4;

/// The pool the block loop forks onto, or `None` on a machine that should stay
/// on one thread — see [`LANE_WORKERS`].
fn lane_pool() -> Option<rayon::ThreadPool> {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    if cores < LANE_POOL_MIN_CORES {
        return None;
    }
    match rayon::ThreadPoolBuilder::new()
        .num_threads(LANE_WORKERS)
        .thread_name(|i| format!("sdroxide-lane{i}"))
        .build()
    {
        Ok(pool) => Some(pool),
        Err(e) => {
            warn!("could not start the panadapter lane pool, staying single-threaded: {e}");
            None
        }
    }
}

/// The finest overlap any lane runs at, and the one the zoom lane reaches.
///
/// An eighth rather than the usual half: the finer the zoom the longer one
/// transform takes to fill — resolving a hertz needs a second of signal, on any
/// analyser ever built — and at the deep end a half-window hop would leave the
/// waterfall crawling. It used to be a constant the zoom lane passed by hand;
/// [`hop_div_for`] now arrives at the same number from the lane's rate, and
/// this is the ceiling it stops at.
/// See [`sdroxide_dsp::SpectrumAnalyzer::with_hop_div`].
const MAX_HOP_DIV: usize = 8;

/// Where the decoder's tap is levelled to, as an RMS of full scale.
///
/// The controller that reads it scales to sixteen-bit at 28000 and clips at
/// full scale, so this is a compromise between quantisation at the bottom and
/// headroom at the top: 0.08 puts the average at 2240 counts — seventy-odd
/// decibels above the last bit — and leaves twenty decibels of crest factor
/// before anything is clipped, which is more than a band of overlapping
/// transmissions ever has.
const TAP_TARGET_RMS: f32 = 0.08;

/// How long the tap's level estimate takes to follow the band.
///
/// Long against a transmission and short against an evening. An FT8 slot is
/// fifteen seconds, so six is slow enough that the gain is very nearly a
/// constant across any one of them — which is the whole point, see
/// [`tap_gain_for`] — and quick enough that a band that has really changed is
/// followed inside half a minute.
const TAP_LEVEL_TC_S: f32 = 6.0;

/// Bounds on the tap gain: 60 dB of lift and 40 dB of cut, which spans
/// everything between a bare demodulator on a dead band and a front end handing
/// over full scale.
const TAP_GAIN_MIN: f32 = 0.01;
const TAP_GAIN_MAX: f32 = 1_000.0;

/// Averaging time constant for the digital modes' channel analyser. Short: a
/// slotted mode's picture is a sequence of transmissions starting and stopping,
/// and averaging is the enemy of seeing where one ended.
const CHANNEL_AVG_TC_S: f32 = 0.10;

/// The largest transform the digital modes' channel analyser will ask for.
///
/// It runs at the channel rate — tens of kilohertz, against the megahertz a
/// device-wide lane sees — so a large one here is cheap, and the display is
/// what wants it: a 4K panel drawing a 3.7 kHz sub-band asks for a point every
/// 1.4 Hz. The window it actually looks at is a separate and much smaller
/// number ([`analysis_window`]).
const CHANNEL_FFT_MAX: usize = 65_536;

/// The window a slotted digital mode is drawn in when the client has not said
/// otherwise: the FT8/FT4 sub-band, 200 Hz below the dial to 3.5 kHz above it.
/// The same figures [`Engine::make_spectrum_frame`] falls back to.
const DIGI_VIEW_SPAN_HZ: f64 = 3_700.0;

/// How long the main panadapter keeps drawing a front end's own spectrum after
/// the last sweep landed.
///
/// Long enough to sit through the source's own scope watchdog — an Icom's stops
/// sweeping for several ordinary reasons and is asked again after three seconds
/// — and short enough that a scope switched off at the radio for good hands the
/// lane back to the audio FFT rather than leaving a frozen picture up.
const SCOPE_MAIN_STALE: Duration = Duration::from_secs(10);

/// How often S-meter / TX telemetry is emitted. 30 Hz matches the default
/// spectrum rate, so the meter moves as smoothly as the panadapter does; the
/// payload is a handful of floats, so the extra traffic is immaterial even over
/// the remote-client WebSocket.
const METER_INTERVAL: Duration = Duration::from_millis(33);

/// The longest the engine's thread will be held waiting for the external T/R
/// switch's contacts before RF is let out.
///
/// A relay throws in five to fifteen milliseconds and a sequencer wants a
/// little more; anything beyond this is an audible hole in the receive tail and
/// a delay between the operator's thumb and their own transmitter, and is far
/// more likely to be a mistyped setting than a real requirement. Capped rather
/// than refused, because a switch that leads by a quarter of a second is still
/// a switch that protects the receiver.
const MAX_TR_LEAD: Duration = Duration::from_millis(250);

/// Consecutive over-limit SWR readings required before the guard fires. At the
/// 33 ms meter interval this is about a fifth of a second of genuinely bad SWR,
/// which is long enough to ride out the key-up transient and short enough that
/// the transmitter is not left into a fault.
const SWR_TRIP_SAMPLES: u16 = 6;

/// Meter samples to ignore after key-up before the SWR is believed at all.
///
/// This is the right way to discard the key-up transient, and it replaced a
/// forward-power floor that looked equivalent and was not. See
/// [`SWR_MIN_FWD_W`].
const SWR_SETTLE_SAMPLES: u16 = 6;

/// The same grace period during a TUNE: 150 samples ≈ 5 seconds. See
/// [`swr_rails`] for why a tune is not an ordinary over.
///
/// Five seconds covers an external auto-ATU comfortably — an LDG or SGC
/// finishes a sweep in one to four — and is short enough that a genuinely dead
/// feeder is still caught in the same key-down. A manual tuner can easily take
/// longer than this; that operator wants the guard turned off for the session,
/// which is a checkbox, rather than a grace period long enough to be no
/// protection at all.
const SWR_TUNE_SETTLE_SAMPLES: u16 = 150;

/// The limit clamp and the tune-limit derivation live in `sdroxide-types`, with
/// [`TxState`], because the settings panel shows the operator what the tune
/// limit works out to and a second copy of the formula would drift from this
/// one. Only the grace periods above are the engine's own business — they are
/// counted in meter samples, which nothing outside this file knows about.
use sdroxide_types::{SWR_LIMIT_MAX, SWR_LIMIT_MIN, swr_tune_limit};

/// Forward power below which an SWR reading is ignored, purely to reject a
/// reading taken when the transmitter is not really producing anything.
///
/// ⛔ THIS MUST STAY LOW. It was 5.0 W until 17 August 2026 and that was a real
/// bug, found on the first live test: a rig with SWR foldback REDUCES POWER
/// BECAUSE THE SWR IS BAD. Rodger's IC-7300 dropped to 3 W into a 7.5:1 load,
/// every reading was therefore discarded as untrustworthy, and the guard sat
/// silent through exactly the fault it exists to catch. A power floor high
/// enough to be useful as a transient filter is high enough to gate out the
/// emergency.
///
/// Discarding the transient is [`SWR_SETTLE_SAMPLES`]'s job, which is what a
/// key-up transient actually is: a property of time, not of power.
const SWR_MIN_FWD_W: f32 = 0.5;

/// The trip limit and the grace period in force for the over now on the air.
///
/// Tuning is the deliberate act of transmitting into a load
/// that is *known* to be mismatched: an external ATU has nothing to work on
/// until the rig keys into the very mismatch the ATU exists to remove, and it
/// needs seconds of carrier to sweep. Held to the operator's on-air limit — 2.5
/// by default, against a starting SWR that is routinely 10:1 — the guard would
/// abort every tune-up after a fifth of a second and then latch transmit out,
/// on exactly the stations that own a tuner because they need one.
///
/// So a tune gets both rails moved: the limit is doubled ([`swr_tune_limit`],
/// capped at the top of the scale the rigs report) and the grace period runs to
/// about five seconds ([`SWR_TUNE_SETTLE_SAMPLES`]) instead of a fifth of one.
/// With the default 2.5:1 that is 5:1 while tuning. What survives is the case
/// worth keeping: a feeder that is simply not connected still reads at or near
/// the top of the scale after the ATU has had its five seconds, and still stops
/// the carrier.
fn swr_rails(tuning: bool, limit: f32) -> (f32, u16) {
    if tuning {
        (swr_tune_limit(limit), SWR_TUNE_SETTLE_SAMPLES)
    } else {
        (limit, SWR_SETTLE_SAMPLES)
    }
}

/// How often the RDS snapshot goes out. Far slower than the meters: a station
/// name changes never, radio text every few seconds, and the only thing arriving
/// continuously is the diagnostics log, which is a delta and so costs the same
/// whatever the cadence.
const RDS_INTERVAL: Duration = Duration::from_millis(500);

/// How often the DRM status is polled. Faster than RDS because the sync
/// indicators are what an operator watches while tuning one in, and a
/// half-second lag on those reads as a decoder that is not working.
const DRM_INTERVAL: Duration = Duration::from_millis(250);

/// How far the dial has to move before the DRM decoder is told to re-acquire.
/// A tenth of what RDS uses: broadcasts sit on a 5 kHz raster on shortwave, so
/// anything past a couple of kHz is a different transmission, not drift.
const DRM_RETUNE_HZ: f64 = 2_000.0;

/// How far the dial has to move before the RDS decoder is told to forget the
/// station.
///
/// Broadcast channels are at least 100 kHz apart, so anything within half of
/// that is still the same station and a nudge of the tuning should not blank the
/// display. Anything further is somebody else, and leaving the previous
/// station's name sitting under the new one's audio is worse than showing
/// nothing for the second it takes to re-acquire.
const RDS_RETUNE_HZ: f64 = 50_000.0;

pub struct EngineHandles {
    pub cmd_tx: Sender<Command>,
    pub event_rx: Receiver<RadioEvent>,
    pub spectrum_out: triple_buffer::Output<SpectrumFrame>,
    /// Full-band spectrum from front ends that can see far more than the IQ
    /// they deliver (the RX-888's whole 0–32 MHz). Empty frames on every other
    /// source. Separate from `spectrum_out` rather than multiplexed onto it
    /// because the two have different rates, spans and lifetimes.
    pub wide_spectrum_out: triple_buffer::Output<SpectrumFrame>,
    /// Runtime device swaps: audio-device changes (rebuilt cpal ring endpoints)
    /// and radio-interface changes (rebuild the IQ source from the persisted
    /// config, no restart).
    pub swap_tx: Sender<EngineSwap>,
    /// Join before process exit so device teardown (SoapySDR/libusb) can't
    /// race the C libraries' own exit handlers.
    pub thread: Option<std::thread::JoinHandle<()>>,
}

/// A live device change from the frontend. Audio `None` payloads mean "no
/// device" (run silent / TX carries silence); `ReopenSource` asks the engine
/// to rebuild the IQ front-end from the (freshly persisted) radio config.
pub enum EngineSwap {
    Output(Option<AudioParams>),
    Input(Option<MicParams>),
    /// Rebuild the radio source at runtime (backend / CAT audio / HPSDR-TCI
    /// address changed). The engine calls its [`ReopenFn`] factory.
    ReopenSource,
    /// Re-read the station-shared stores (memories, folders, band stacks, digi
    /// operator config) from disk. Sent to every *other* engine after one of
    /// them saves: each engine holds a whole in-memory copy and writes it back
    /// whole, so without this nudge radio B's next edit would clobber what
    /// radio A just added.
    ReloadSharedStores,
}

/// Factory that (re)opens the configured IQ source at runtime, given the
/// current dial frequency as the requested center. Lives in the binary (only it
/// knows how to build each backend); the engine calls it on [`EngineSwap::ReopenSource`].
/// Returns an error (leaving the current source running) when the new interface
/// can't be opened.
pub type ReopenFn = Box<dyn FnMut(f64) -> Result<(Box<dyn IqSource>, DeviceCaps), String> + Send>;

/// Audio sink the engine feeds with interleaved stereo frames.
pub struct AudioParams {
    pub producer: rtrb::Producer<f32>,
    /// The rate the audio device actually runs at.
    pub out_rate: f64,
}

/// Microphone feed (created by the frontend from `sdroxide-audio`).
pub struct MicParams {
    pub consumer: rtrb::Consumer<f32>,
    pub rate: f64,
}

pub struct EngineConfig {
    pub audio: Option<AudioParams>,
    pub mic: Option<MicParams>,
    /// dBFS → dBm S-meter calibration offset.
    pub cal_offset_db: f32,
    /// Startup mode override (e.g. from `--mode wfm`).
    pub initial_mode: Option<Mode>,
    /// Startup antenna overrides (`--antenna`, `--tx-antenna`), RX then TX.
    /// Each outranks what the session remembered, and is ignored when the front
    /// end does not offer a port by that name.
    pub initial_antenna: (Option<String>, Option<String>),
    /// Refuse to key up outside amateur bands.
    pub tx_ham_only: bool,
    /// Abort the over and latch out transmit when the rig reports an SWR at or
    /// above [`Self::swr_limit`]. Inert on rigs that do not measure SWR.
    pub swr_guard: bool,
    /// The SWR ratio the guard trips at.
    pub swr_limit: f32,
    /// Rebuilds the IQ source at runtime when the operator switches interfaces.
    /// `None` disables runtime interface switching (a restart is then required).
    pub reopen: Option<ReopenFn>,
    /// Write the dial and mode to `session.json` as the radio is used, so the
    /// next start comes up where this one left off.
    ///
    /// Off by default: the headless smoke tests and the integration tests bring
    /// engines up on synthetic sources, and none of them may overwrite what the
    /// operator left the radio on.
    pub remember_session: bool,
    /// Where this engine's radio-scoped files live (`radio.json`,
    /// `session.json`, `scanner.json`, the server configs). The station-shared
    /// stores stay on the root paths regardless. Defaults to the station scope,
    /// which is also radio 0's — a single-radio start is unchanged.
    pub store: sdroxide_config::Store,
    /// This engine's radio id: names the DSP thread and prefixes recording
    /// filenames, so two radios recording in the same second don't collide.
    pub instance: u32,
    /// Write every raw IQ sample the receiver delivers to this file, as
    /// interleaved little-endian `f32` pairs — the format [`crate::FileSource`]
    /// reads back.
    ///
    /// For capturing a band to work on offline: what a decoder does with a real
    /// signal is not a question a synthetic one can answer, and a capture is the
    /// only way to ask it twice. Written from the DSP thread with no buffering
    /// beyond the OS's, and at 2 Msps it is 16 MB a second, so it is a
    /// deliberate command-line act rather than anything the UI can start.
    pub record_iq: Option<std::path::PathBuf>,
    /// Whether this engine runs the station-wide network services (spot feeds,
    /// the rotator client). Exactly one engine per process should — they hold
    /// logins and sockets that must not be duplicated per radio.
    pub primary: bool,
    /// The station's transmit interlock, shared by every engine in the
    /// process. `None` (the default, and every single-radio start) keys freely.
    pub tx_gate: Option<Arc<crate::TxGate>>,
    /// Change signal for the station-shared stores, shared like `tx_gate`.
    /// `None` (the default): no other engine exists, nothing to watch.
    pub store_sync: Option<Arc<crate::StoreSync>>,
    /// Which radio is on FreeDV, shared like `tx_gate`. `None` (the default,
    /// and every single-radio start): this engine's own mode is the answer.
    pub rade_watch: Option<Arc<crate::RadeWatch>>,
    /// The station's external T/R switch, shared like `tx_gate` — the relay
    /// that grounds the SDR's antenna while anything in this process is on the
    /// air. `None` (the default) switches nothing.
    ///
    /// Shared rather than per-engine because the relay is in the antenna line,
    /// not in a front end: whichever radio keys, the same contacts have to
    /// throw, and they must not open while another one is still transmitting.
    pub tr_switch: Option<Arc<crate::TrSwitch>>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            audio: None,
            mic: None,
            cal_offset_db: 0.0,
            initial_mode: None,
            initial_antenna: (None, None),
            tx_ham_only: true,
            swr_guard: true,
            swr_limit: 2.5,
            reopen: None,
            remember_session: false,
            store: sdroxide_config::Store::station(),
            instance: 0,
            primary: true,
            tx_gate: None,
            store_sync: None,
            rade_watch: None,
            tr_switch: None,
            record_iq: None,
        }
    }
}

/// Spawn the engine thread. It runs until the last command sender is dropped
/// or the source fails.
pub fn start(source: Box<dyn IqSource>, caps: DeviceCaps, cfg: EngineConfig) -> EngineHandles {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    let (swap_tx, swap_rx) = crossbeam_channel::unbounded();
    let empty = SpectrumFrame {
        seq: 0,
        center_hz: 0.0,
        span_hz: 0.0,
        db_floor: 0.0,
        db_ceil: 0.0,
        bins: Vec::new(),
        rows: Vec::new(),
        rows_clocked: false,
    };
    let (spec_in, spectrum_out) = triple_buffer::triple_buffer(&empty);
    let (wide_in, wide_spectrum_out) = triple_buffer::triple_buffer(&empty);

    // Radio 0 keeps the historical name — profiling notes and thread filters
    // key on it — and every further radio is distinguishable at a glance.
    let thread_name = match cfg.instance {
        0 => "sdroxide-dsp".to_string(),
        n => format!("sdroxide-dsp-{n}"),
    };
    let thread = std::thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            engine_thread(source, caps, cfg, cmd_rx, swap_rx, event_tx, spec_in, wide_in)
        })
        .expect("spawn dsp thread");

    EngineHandles {
        cmd_tx,
        event_rx,
        spectrum_out,
        wide_spectrum_out,
        swap_tx,
        thread: Some(thread),
    }
}

/// Build a DeepFilterNet denoiser into `slot` if it is not there and has not
/// already failed.
///
/// The model load is expensive — an 8 MB archive unpacked and three graphs
/// optimised — and it happens on the engine thread, so it is done once on the
/// first block that asks for it and never retried: a model that would not load
/// will not load next block either, and retrying would turn one glitch into a
/// permanently broken receiver. Both NR sites share this.
fn ensure_dfnr(slot: &mut Option<Box<DeepFilterNr>>, failed: &mut bool) {
    if slot.is_some() || *failed {
        return;
    }
    match DeepFilterNr::new() {
        Ok(df) => *slot = Some(Box::new(df)),
        Err(e) => {
            warn!("DeepFilterNet noise reduction is unavailable: {e}");
            *failed = true;
        }
    }
}

/// Whether this host can run DeepFilterNet at all, building the model to find
/// out. Used to answer a selection before it reaches the audio thread, so the
/// level the UI is shown is the level that will actually run.
fn dfnr_available(slot: &mut Option<Box<DeepFilterNr>>, failed: &mut bool) -> bool {
    ensure_dfnr(slot, failed);
    slot.is_some()
}

/// Whether a receiver's demod may decode stereo right now.
///
/// Noise reduction and the auto-notch disqualify it: every NR engine carries a
/// latency — a frame for `SpectralNr`, an RNNoise frame for `NeuralNr`, a
/// DeepFilterNet hop, three quarters of a 20 ms frame for `SpecBleachNr` — and
/// all of them would run on the sum only. An 8–10 ms delay on one side of
/// `L = M±S` is three cycles of phase error at 1 kHz — the matrix would collapse
/// into a comb filter with a randomly wandering image. They are HF speech tools
/// that buy nothing on a broadcast signal, so stereo simply yields to them.
fn stereo_allowed(rx: &RxState) -> bool {
    // ISB is not a decode and has no pilot to lose: the two sidebands carry
    // two different transmissions and putting one in each ear *is* the mode,
    // so the WFM stereo switch does not gate it. The latency rule above still
    // does — with NR or the notch running the matrix would comb, and half of
    // an ISB pair through a comb filter is worse than the two summed.
    let wanted = rx.wfm_stereo || rx.mode == Mode::Isb;
    wanted && !rx.auto_notch && !rx.noise_reduction.is_on()
}

/// The gain the decoder's tap rides, from a mean-square estimate of the
/// demodulated audio that takes [`TAP_LEVEL_TC_S`] to move.
///
/// Levelling the tap at all is necessary — the demodulator's output is whatever
/// the band handed over, and a signal sixty decibels below full scale would be
/// decoded from a couple of dozen quantisation steps once the controller has
/// made it sixteen-bit. Levelling it *quickly* is what must not happen: a gain
/// that moves inside a transmission modulates every signal in the passband and
/// spreads each of them across the band, which is what the AGC was doing to the
/// decoder before issue #307. Seconds of time constant is what makes this a
/// constant over a fifteen-second slot and still lets it follow a band that
/// changes through the evening.
///
/// Updated once a block and applied to the whole of it, so it cannot move
/// within one either. A free function rather than a method because the caller
/// is already holding a mutable borrow of the demodulator beside it.
fn tap_gain_for(level_db: &mut f32, gain: &mut f32, audio: &[f32], rate: f64) -> f32 {
    if audio.is_empty() {
        return *gain;
    }
    let ms: f32 = audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32;
    // A block the chain could not compute says nothing about the band's level,
    // so the estimate keeps the one it had rather than being dragged to a floor
    // by it — the same rule the AGC's envelope follows (issue #305).
    if !ms.is_finite() {
        return *gain;
    }
    let block_db = 10.0 * (ms + 1e-20).log10();
    // One pole per block, at the block's own length: a source that hands over
    // ten milliseconds at a time and one that hands over a hundred must reach
    // the same place in the same number of seconds.
    let block_s = audio.len() as f32 / rate.max(1.0) as f32;
    let alpha = 1.0 - (-block_s / TAP_LEVEL_TC_S).exp();
    // In decibels rather than in power, which is the difference between a
    // filter that is slow and one that is only slow when the step is small: a
    // one-pole on the mean square answers a twenty-decibel change with eleven
    // decibels of gain in the first half second, because a tenth of a hundredfold
    // step is still an elevenfold one. A signal is quiet or loud by orders of
    // magnitude, so the estimate lives where those are a distance.
    *level_db = if level_db.is_finite() {
        *level_db + (block_db - *level_db) * alpha
    } else {
        // Nothing measured yet: start where the signal is rather than climbing
        // to it over the first few seconds of a session.
        block_db
    };
    let want_db = 20.0 * TAP_TARGET_RMS.log10();
    *gain = 10f32.powf((want_db - *level_db) / 20.0).clamp(TAP_GAIN_MIN, TAP_GAIN_MAX);
    *gain
}

/// One receiver: DDC → demod → AGC → volume → resample to the device rate.
struct RxChain {
    in_rate: f64,
    ddc: Ddc,
    demod: Option<Box<dyn Demodulator>>,
    mode: Mode,
    agc: Agc,
    resampler: Option<MonoResampler>,
    out_rate: f64,
    offset_hz: f64,
    /// Smoothed squelch gate gain (0 = closed, 1 = open).
    sq_gain: f32,
    /// Whether this chain has already reported a sample that was not a number.
    /// Said once and not again: the value is knowing that it happened at all
    /// and roughly when, not a line per block from a stage that has gone wrong.
    said_impossible: bool,
    /// When true, `tap_out` receives a copy of the demodulated audio for the
    /// digital-mode decoder and the TCI receive stream, ahead of the AGC and of
    /// everything after it (mute, volume, squelch, noise reduction).
    tap_enabled: bool,
    tap_out: Vec<f32>,
    /// Slow estimate of the tap audio's level in dBFS, and the gain derived
    /// from it. See [`tap_gain_for`] — this is the levelling a decoder can
    /// have, as against the AGC's, which it cannot.
    tap_level_db: f32,
    tap_gain: f32,
    /// Adaptive auto-notch (constant-tone canceller) on the listener audio.
    notch: AutoNotch,
    notch_on: bool,
    /// Spectral noise reduction on the listener audio (after the digital tap).
    nr: SpectralNr,
    /// The libspecbleach port — the other classical engine.
    sbnr: SpecBleachNr,
    /// Neural (RNNoise) noise reduction.
    nnr: NeuralNr,
    /// DeepFilterNet3. Built on first use: it unpacks an 8 MB model, which is
    /// not worth spending on a receiver that may never select it. Stays `None`
    /// once a load has failed, so a broken model is not retried every block.
    dfnr: Option<Box<DeepFilterNr>>,
    dfnr_failed: bool,
    nr_level: NrLevel,
    channel_buf: Vec<Complex32>,
    audio_buf: Vec<f32>,
    out_buf: Vec<f32>,
    /// WFM stereo difference channel (L−R)/2 at the demod rate, empty when the
    /// demod is mono. See [`RxChain::run`].
    side_buf: Vec<f32>,
    /// L/R interleaved for the stereo resampler, and the right channel it
    /// yields. Both stay empty on the mono path.
    lr_buf: Vec<f32>,
    lr_out: Vec<f32>,
    out_buf_r: Vec<f32>,
    /// Resamples L and R together so the two can't drift a sample apart.
    stereo_rs: Option<StereoResampler>,
    /// Post-squelch audio at `out_rate`, same shape as `out_buf`/`out_buf_r`,
    /// but without the AF volume/mute scaling applied to those — the QSO
    /// recorder taps this instead, so turning down the AF knob or hitting
    /// mute doesn't touch the archived recording. See [`RxChain::take_rec_audio`].
    rec_buf: Vec<f32>,
    rec_buf_r: Vec<f32>,
}

impl RxChain {
    fn new(in_rate: f64, rx: &RxState, out_rate: f64) -> Self {
        let mut chain = RxChain {
            in_rate,
            ddc: Ddc::new(in_rate, channel_target(rx.mode)),
            demod: None,
            mode: rx.mode,
            agc: Agc::new(48_000.0),
            resampler: None,
            out_rate,
            offset_hz: 0.0,
            sq_gain: 1.0,
            said_impossible: false,
            tap_enabled: false,
            tap_out: Vec::new(),
            tap_level_db: f32::NAN,
            tap_gain: 1.0,
            notch: AutoNotch::new(),
            notch_on: false,
            nr: SpectralNr::new(),
            sbnr: SpecBleachNr::new(),
            nnr: NeuralNr::new(),
            dfnr: None,
            dfnr_failed: false,
            nr_level: NrLevel::Off,
            channel_buf: Vec::new(),
            audio_buf: Vec::new(),
            out_buf: Vec::new(),
            side_buf: Vec::new(),
            lr_buf: Vec::new(),
            lr_out: Vec::new(),
            out_buf_r: Vec::new(),
            stereo_rs: None,
            rec_buf: Vec::new(),
            rec_buf_r: Vec::new(),
        };
        chain.build_for_mode(rx);
        chain
    }

    /// Audio rate of the demod tap (equals the demod's output rate).
    fn audio_rate(&self) -> f64 {
        self.demod.as_ref().map(|d| d.audio_rate()).unwrap_or(48_000.0)
    }

    /// The DDC output (complex baseband, VFO at DC) from the last `run`.
    fn channel_iq(&self) -> &[Complex32] {
        &self.channel_buf
    }

    /// The channel (DDC output) sample rate.
    fn channel_rate(&self) -> f64 {
        self.ddc.out_rate()
    }

    /// (Re)build demod/AGC/resampler for the mode in `rx`, and the DDC if
    /// the channel target changed. Keeps the NCO offset.
    fn build_for_mode(&mut self, rx: &RxState) {
        self.mode = rx.mode;
        let target = channel_target(rx.mode);
        if (self.ddc.out_rate() - target).abs() / target > 0.5 || self.ddc.out_rate() < target {
            self.ddc = Ddc::new(self.in_rate, target);
            self.ddc.set_offset_hz(self.offset_hz);
        }
        // Release the old demodulator before building the new one. For most
        // modes that is housekeeping; for DRM it is the difference between one
        // Dream receiver and two, because assigning over `self.demod` would
        // construct the replacement first and only then drop what was there.
        // Two of them briefly coexisting is a lot of vendored C++ running
        // twice on two threads for no reason, and `set_rx_mode` does not
        // early-return when the mode has not actually changed — so a rig
        // reporting its mode back can trigger it at any moment.
        self.demod = None;
        // Every mode but this one comes from `make_demod`. DRM's decoder links
        // a vendored C++ receiver, which `sdroxide-dsp` cannot depend on and
        // still build for the browser, so it is constructed here instead — see
        // `Demodulator::take_drm`.
        self.demod = if rx.mode == Mode::Drm {
            Some(Box::new(DrmDemod::new(self.ddc.out_rate())) as Box<dyn Demodulator>)
        } else {
            make_demod(rx.mode, self.ddc.out_rate())
        };
        if let Some(d) = self.demod.as_mut() {
            d.set_filter(rx.filter_lo, rx.filter_hi);
            d.set_stereo_enabled(stereo_allowed(rx));
        }
        let audio_rate =
            self.demod.as_ref().map(|d| d.audio_rate()).unwrap_or_else(|| self.ddc.out_rate());
        self.agc = Agc::new(audio_rate);
        self.agc.set_mode(rx.agc);
        self.agc.set_max_gain_db(rx.agc_max_gain_db);
        self.agc.set_manual_gain_db(rx.manual_gain_db);
        self.resampler = MonoResampler::new(audio_rate, self.out_rate);
        self.stereo_rs = StereoResampler::new(audio_rate, self.out_rate);
    }

    fn set_offset_hz(&mut self, hz: f64) {
        self.offset_hz = hz;
        self.ddc.set_offset_hz(hz);
    }

    /// Process a device-rate block. The first slice is audio at `out_rate`
    /// (empty when this chain produces no audio, e.g. SPEC); the second is the
    /// right channel, present only while WFM stereo is actually being decoded —
    /// otherwise the caller plays the first slice in both ears.
    ///
    /// `want_rec` asks for the pre-volume recorder tap ([`RxChain::rec_buf`])
    /// to be filled as well. It is off whenever nothing is recording, which is
    /// most of the time — the tap is a second copy of every block, and there is
    /// no reason to make it for a file nobody opened.
    fn run(&mut self, iq: &[Complex32], rx: &RxState, want_rec: bool) -> (&[f32], Option<&[f32]>) {
        self.out_buf.clear();
        self.out_buf_r.clear();
        self.rec_buf.clear();
        self.rec_buf_r.clear();
        if self.demod.is_none() {
            return (&self.out_buf, None);
        }
        let demod = self.demod.as_mut().expect("checked above");
        let audio_rate = demod.audio_rate();
        // Split out of the borrow `demod` holds: the tap reads a different
        // field of the same struct.

        self.channel_buf.clear();
        self.ddc.process(iq, &mut self.channel_buf);

        self.audio_buf.clear();
        self.side_buf.clear();
        demod.process(&self.channel_buf, &mut self.audio_buf);
        demod.set_stereo_enabled(stereo_allowed(rx));
        let stereo = demod.take_side(&mut self.side_buf);
        // The decoder's tap, taken *before* the AGC.
        //
        // A modem wants the signal the antenna delivered, levelled if at all by
        // something that does not move inside a transmission. The AGC is the
        // opposite of that: it attacks in two milliseconds, which is a gain
        // fluctuating across the whole audio band, and multiplying a signal by
        // that in time is convolving it with it in frequency. FT8's tones are
        // 6.25 Hz apart and its own noise reference sits four tones away, so
        // the smear lands squarely on the reference: measured on a synthetic
        // forty-signal slot, the AGC cost ten of thirteen decodes and put every
        // reported SNR about sixteen decibels low — both halves of issue #307,
        // and both of them worse the stronger the loudest station in the band
        // is. What the tap gets instead is [`RxChain::tap_gain_for`], a level
        // that takes seconds to move and so is a constant across any one
        // transmission.
        if self.tap_enabled {
            let g = tap_gain_for(
                &mut self.tap_level_db,
                &mut self.tap_gain,
                &self.audio_buf,
                audio_rate,
            );
            self.tap_out.clear();
            self.tap_out.extend(self.audio_buf.iter().map(|s| s * g));
        }

        // FM skips the AGC entirely — a true unity bypass, not AgcMode::Off's
        // manual gain: the discriminator output is already deviation-scaled to
        // ±full scale, so there is no level to restore (Mode::audio_agc).
        if self.mode.audio_agc() {
            if stereo {
                // One gain trajectory and one lookahead delay across both
                // channels: levelling them separately would pump the stereo
                // image.
                self.agc.process_pair(&mut self.audio_buf, &mut self.side_buf);
            } else {
                self.agc.process(&mut self.audio_buf);
            }
            // Nothing in a receive chain should ever hand the AGC a sample that
            // is not a number, and until issue #305 one of them silenced the
            // receiver for the rest of the session. It is stepped over now, but
            // it still means a stage upstream computed something impossible,
            // and this line is the only evidence of that anyone will ever have.
            let bad = self.agc.take_impossible();
            if bad > 0 && !self.said_impossible {
                self.said_impossible = true;
                warn!(
                    samples = bad,
                    mode = ?self.mode,
                    "the receive chain produced samples that are not numbers; the AGC has \
                     stepped over them, but something ahead of it is computing infinities"
                );
            }
        }

        // Auto-notch first (remove constant tones), then spectral NR (remove the
        // residual noise floor) — both on the listener audio only.
        if self.notch_on != rx.auto_notch {
            if rx.auto_notch {
                self.notch.reset();
            }
            self.notch_on = rx.auto_notch;
        }
        if self.notch_on {
            self.notch.process(&mut self.audio_buf);
        }
        if self.nr_level != rx.noise_reduction {
            let (prev, now) = (self.nr_level, rx.noise_reduction);
            self.nr_level = now;
            // Reset only when the *engine* changes: an operator riding the
            // strength should not restart a network's hidden state, or a noise
            // estimator's, on every click.
            let switched = prev.engine() != now.engine();
            match now.engine() {
                Some(NrEngine::Rnn) => {
                    if switched {
                        self.nnr.reset();
                    }
                    self.nnr.set_mix(now.rnn_mix());
                }
                Some(NrEngine::DeepFilter) => {
                    if switched {
                        ensure_dfnr(&mut self.dfnr, &mut self.dfnr_failed);
                        if let Some(df) = self.dfnr.as_mut() {
                            df.reset();
                        }
                    }
                    if let Some(df) = self.dfnr.as_mut() {
                        df.set_atten_lim_db(now.df_atten_db());
                    }
                }
                Some(NrEngine::SpecBleach) => {
                    if switched {
                        self.sbnr.reset();
                    }
                    let (db, whiten) = now.spec_params();
                    self.sbnr.set_params(db, whiten);
                }
                Some(NrEngine::Spectral) => {
                    if switched {
                        self.nr.reset();
                    }
                    let (over, floor) = now.params();
                    self.nr.set_params(over, floor);
                }
                None => {}
            }
        }
        if self.nr_level.is_on() {
            // Every engine but the original one is rate-aware, and the rate can
            // move under us on a mode change, so it is re-asserted per block
            // rather than on the level change. All of them no-op cheaply when
            // nothing has moved.
            let fs = demod.audio_rate();
            match self.nr_level.engine() {
                Some(NrEngine::Rnn) => {
                    self.nnr.set_rate(fs);
                    self.nnr.process(&mut self.audio_buf);
                }
                Some(NrEngine::DeepFilter) => match self.dfnr.as_mut() {
                    Some(df) => {
                        df.set_rate(fs);
                        df.process(&mut self.audio_buf);
                    }
                    // The model would not load. The engine has already said so
                    // and put the level back on RNNoise, but a block can arrive
                    // in between; RNNoise is the honest stand-in.
                    None => {
                        self.nnr.set_rate(fs);
                        self.nnr.process(&mut self.audio_buf);
                    }
                },
                Some(NrEngine::SpecBleach) => {
                    self.sbnr.set_rate(fs);
                    self.sbnr.process(&mut self.audio_buf);
                }
                Some(NrEngine::Spectral) => self.nr.process(&mut self.audio_buf),
                None => {}
            }
            // Suppression lowers the level; boost it back up per NR strength.
            let g = self.nr_level.makeup_gain();
            for s in &mut self.audio_buf {
                *s = (*s * g).clamp(-1.0, 1.0);
            }
        }

        // Squelch: gate on post-filter (pre-AGC) power, smoothed ~10 ms so
        // opening and closing don't click. Tone squelch, where the operator has
        // set one, is an extra condition on the same gate rather than a second
        // one: a repeater's own tone takes about a second to identify, and
        // running two gates in series would make that a second of clipped audio
        // every over instead of a slightly later opening.
        let tone_ok = rx.tone_sql.is_none_or(|want| demod.sub_tone() == Some(want));
        let open = demod.power_dbfs() >= rx.squelch_db && tone_ok;
        let sq_target = if open { 1.0 } else { 0.0 };
        // AF volume/mute is applied further down, after the recorder tap is
        // snapshotted — squelch is the only gate shared by both.
        if stereo {
            // A single loop over both: `sq_gain` advances per *sample*, so
            // gating the two channels in separate passes would run the gate
            // twice as fast and hand them different gains.
            for (m, sd) in self.audio_buf.iter_mut().zip(self.side_buf.iter_mut()) {
                self.sq_gain += (sq_target - self.sq_gain) * 0.002;
                *m *= self.sq_gain;
                *sd *= self.sq_gain;
            }
        } else {
            for s in &mut self.audio_buf {
                self.sq_gain += (sq_target - self.sq_gain) * 0.002;
                *s *= self.sq_gain;
            }
        }
        let vol = if rx.muted { 0.0 } else { rx.volume * rx.volume };

        if !stereo {
            match &mut self.resampler {
                Some(r) => r.push(&self.audio_buf, &mut self.out_buf),
                None => self.out_buf.extend_from_slice(&self.audio_buf),
            }
            // Recorder tap: post-squelch, pre-volume/mute (see `rec_buf`),
            // clamped on the way in so interpolation overshoot can't escape
            // into the file either.
            if want_rec {
                self.rec_buf.extend(self.out_buf.iter().map(|s| s.clamp(-1.0, 1.0)));
            }
            // Volume *then* clamp, never the other way round: clamping first
            // would hard-clip a hot block at full scale and only then scale it
            // down, baking in distortion that turning the AF knob down is
            // supposed to avoid. Clamping last still catches the resampler's
            // interpolation overshoot.
            for s in &mut self.out_buf {
                *s = (*s * vol).clamp(-1.0, 1.0);
            }
            return (&self.out_buf, None);
        }

        // Matrix last: everything upstream ran on the sum, which is what the
        // taps, the recorder downmix and the remote stream all want.
        self.lr_buf.clear();
        self.lr_buf.reserve(self.audio_buf.len() * 2);
        for (&m, &sd) in self.audio_buf.iter().zip(self.side_buf.iter()) {
            self.lr_buf.push(m + sd);
            self.lr_buf.push(m - sd);
        }
        let lr: &[f32] = match &mut self.stereo_rs {
            Some(r) => {
                self.lr_out.clear();
                r.push(&self.lr_buf, &mut self.lr_out);
                &self.lr_out
            }
            None => &self.lr_buf,
        };
        self.out_buf.reserve(lr.len() / 2);
        self.out_buf_r.reserve(lr.len() / 2);
        for f in lr.chunks_exact(2) {
            // Volume before the clamp for the speakers, as in the mono branch
            // above — clamping first would bake in distortion the AF knob is
            // supposed to be able to avoid.
            self.out_buf.push((f[0] * vol).clamp(-1.0, 1.0));
            self.out_buf_r.push((f[1] * vol).clamp(-1.0, 1.0));
        }
        // Recorder tap: post-squelch, pre-volume/mute (see `rec_buf`). A second
        // pass rather than a branch in the loop above, so the common case of
        // not recording walks `lr` exactly once.
        if want_rec {
            self.rec_buf.reserve(lr.len() / 2);
            self.rec_buf_r.reserve(lr.len() / 2);
            for f in lr.chunks_exact(2) {
                self.rec_buf.push(f[0].clamp(-1.0, 1.0));
                self.rec_buf_r.push(f[1].clamp(-1.0, 1.0));
            }
        }
        (&self.out_buf, Some(&self.out_buf_r))
    }

    /// The speaker-path audio from the last [`RxChain::run`] — the same slice
    /// `run` returned. Exists so a caller can hold this and the recorder tap at
    /// once: `run`'s own return keeps the chain mutably borrowed, which rules
    /// out asking for [`RxChain::take_rec_audio`] alongside it.
    fn out_audio(&self) -> &[f32] {
        &self.out_buf
    }

    /// The recorder's copy of the last [`RxChain::run`] block: post-squelch,
    /// pre-AF-volume/mute. Shape matches `run`'s return (second slice present
    /// only in the stereo case).
    fn take_rec_audio(&self) -> (&[f32], Option<&[f32]>) {
        let right = (!self.rec_buf_r.is_empty()).then_some(self.rec_buf_r.as_slice());
        (&self.rec_buf, right)
    }

    fn power_dbfs(&self) -> Option<f32> {
        self.demod.as_ref().map(|d| d.power_dbfs())
    }

    fn stereo_locked(&self) -> bool {
        self.demod.as_ref().is_some_and(|d| d.stereo_locked())
    }

    fn sub_tone(&self) -> Option<sdroxide_types::SubTone> {
        self.demod.as_ref().and_then(|d| d.sub_tone())
    }

    /// What the RDS decoder has made of the station since the last poll, or
    /// `None` when nothing has moved. Only WFM ever answers.
    fn take_rds(&mut self) -> Option<sdroxide_types::RdsData> {
        self.demod.as_mut().and_then(|d| d.take_rds())
    }

    /// Forget the station: the dial has moved, and the demod cannot see that for
    /// itself because the DDC ahead of it absorbs the retune.
    fn reset_rds(&mut self) {
        if let Some(d) = self.demod.as_mut() {
            d.reset_rds();
        }
    }

    /// What the DRM decoder has made of the broadcast since the last poll, or
    /// `None` when nothing has moved. Only DRM ever answers.
    fn take_drm(&mut self) -> Option<sdroxide_types::DrmStatus> {
        self.demod.as_mut().and_then(|d| d.take_drm())
    }

    /// Re-acquire, for the same reason as [`RxChain::reset_rds`].
    fn reset_drm(&mut self) {
        if let Some(d) = self.demod.as_mut() {
            d.reset_drm();
        }
    }

    /// Decode a different service of the DRM multiplex.
    fn set_drm_service(&mut self, service: u8) {
        if let Some(d) = self.demod.as_mut() {
            d.set_drm_service(service);
        }
    }

    /// Start or stop reading back a DRM constellation.
    fn set_drm_constellation(&mut self, channel: Option<sdroxide_types::DrmChannel>) {
        if let Some(d) = self.demod.as_mut() {
            d.set_drm_constellation(channel);
        }
    }
}

/// Where a running scan has got to.
///
/// The two shapes a scan can take are one type because they differ only in how
/// `queue` is refilled: a memory scan takes the next stored channel, a range
/// scan on a wideband front end asks the FFT for everything busy in a whole
/// span at once, and a range scan on a CAT rig walks the channel grid. After
/// that they all visit candidates the same way.
struct Scan {
    phase: ScanPhase,
    /// Still to visit before the queue has to be refilled.
    queue: std::collections::VecDeque<ScanTarget>,
    /// Hardware centres a range sweep works through, and which one is up.
    slices: Vec<f64>,
    slice: usize,
    /// Channels a stepped range scan walks, and how far along it is.
    stepped: Vec<f64>,
    step_at: usize,
    /// A fast memory scan's plan: each hardware centre to look from, with the
    /// memory ids it can see from there — and which one is up.
    ///
    /// Ids rather than indices into the store: the store is republished
    /// whenever anything edits it, and a plan holding positions in a list that
    /// has since been re-ordered would scan the wrong channels. Emptied at the
    /// end of every lap, so a channel stored, skipped or re-filed mid-scan is
    /// picked up on the next one — the same bargain the plain memory scan
    /// strikes by rebuilding its queue each lap.
    mem_slices: Vec<(f64, Vec<u32>)>,
    mem_slice: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ScanPhase {
    /// The front end has just moved; neither the FFT nor the meter means
    /// anything yet.
    Settling(Instant),
    /// Parked on a candidate, listening long enough to be sure.
    Probing(Instant),
    /// Stopped on something. `last_busy` is the last moment the signal was
    /// actually there, which is what a carrier-resume waits on.
    Holding { since: Instant, last_busy: Instant },
}

/// What a refill did, so the caller knows whether there is anything to visit
/// yet.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Refill {
    /// The queue has candidates in it now.
    Queued,
    /// The front end is moving; the next poll picks things up.
    Waiting,
    /// The scan is over.
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ScanTarget {
    /// A stored channel, which brings its own mode and filter.
    Memory(u32),
    /// A bare frequency, in whichever mode the range scan is set to.
    Freq(f64),
}

/// Interleaves two mono streams into the stereo ring. The second is whichever
/// source has claimed the right ear — the sub receiver, or the right channel of
/// a WFM stereo broadcast; with neither, the first goes to both ears.
struct StereoMixer {
    out: rtrb::Producer<f32>,
    main_q: Vec<f32>,
    sub_q: Vec<f32>,
    dropped: u64,
    /// When recording: RX in the left channel, TX audio in the right (or both
    /// time-multiplexed onto a single channel — see `rec_mono`). With nothing
    /// to put in the right channel the recording is dual mono instead — see
    /// `rec_split`.
    rec_tap: Option<rtrb::Producer<f32>>,
    /// Fed from `push`'s `rec_left`/`rec_right`, which the caller builds
    /// independent of the speaker path's AF volume/mute — see `RxChain::run`.
    /// Queued separately from `main_q`/`sub_q` so the two can drain at
    /// different times without one starving the other.
    rec_main_q: Vec<f32>,
    rec_sub_q: Vec<f32>,
    /// Resamples TX audio to `rec_tap`'s rate. `None` if they already match.
    tx_rec_rs: Option<MonoResampler>,
    tx_rec_scratch: Vec<f32>,
    /// False while transmitting, so a full-duplex source's live RX push
    /// doesn't land in the recording tap alongside `push_tx`'s TX audio.
    rx_rec_enabled: bool,
    /// Whether `rec_tap` was opened for a mono recording (one channel) rather
    /// than stereo (two) — must match what `Recorder::start` configured.
    rec_mono: bool,
    /// Whether the recording's right channel has a source of its own: a sub
    /// receiver, or a stereo broadcast (WFM or DRM). When it hasn't, a stereo recording would be
    /// half silence — one ear of RX and, over on the other side, one ear of
    /// TX — so both sides go to both channels instead and the file plays
    /// centred. Latched from the last block that carried samples, because an
    /// over stops feeding the RX tap altogether and `push_tx` still has to
    /// know which of the two layouts the file is in.
    rec_split: bool,
    /// Attenuation applied to the speaker path while a local spoken
    /// announcement plays. 1.0 is off.
    ///
    /// Applied here rather than to the buffers upstream for two reasons: this
    /// is the single funnel every audio path goes through, and the recorder's
    /// channels arrive as separate arguments — so ducking here cannot leak into
    /// an MP3 however the caller assembled the two sides.
    duck: f32,
    /// Silence the speaker path for the length of an over, where the receiver
    /// keeps running through it: a radio with another radio attached as its
    /// panadapter, set to blank nothing but still to mute
    /// ([`IqSource::mutes_rx_audio_on_tx`]). A half-duplex front end is not
    /// read during an over at all and never reaches this.
    ///
    /// A third scalar here rather than a branch upstream, for the reason `duck`
    /// gives: this is the one funnel every speaker path goes through, and the
    /// recording tap arrives as separate arguments that it must not touch —
    /// what the transmitter hears of the band is not the operator's archive.
    tx_muted: bool,
    /// Linear form of [`sdroxide_types::RadioConfig::rx_audio_gain_db`] — the
    /// operator's fixed trim for a rig whose audio arrives quiet. 1.0 is off.
    ///
    /// Here for the same two reasons as `duck`, and for a third: this is the
    /// only stage above unity in the whole receive path, so it is the only one
    /// that can put a sample past full scale — and the clamp that catches that
    /// has to be where the ring is filled, after ducking and the AF rail have
    /// had their say.
    trim: f32,
}

/// Bound on per-channel queueing (≈¼ s at 48 kHz) so a stalled side can't
/// grow the other without limit.
const MIXER_CAP: usize = 12_000;

impl StereoMixer {
    fn new(out: rtrb::Producer<f32>) -> Self {
        StereoMixer {
            out,
            main_q: Vec::new(),
            sub_q: Vec::new(),
            dropped: 0,
            rec_tap: None,
            rec_main_q: Vec::new(),
            rec_sub_q: Vec::new(),
            tx_rec_rs: None,
            tx_rec_scratch: Vec::new(),
            rx_rec_enabled: true,
            rec_mono: false,
            rec_split: false,
            duck: 1.0,
            tx_muted: false,
            trim: 1.0,
        }
    }

    /// Set the speaker-path attenuation. See [`StereoMixer::duck`].
    fn set_duck(&mut self, gain: f32) {
        self.duck = gain.clamp(0.0, 1.0);
    }

    /// Set the operator's fixed receive-audio trim, in dB. See
    /// [`StereoMixer::trim`].
    ///
    /// Bounded rather than taken at face value: this is the one gain in the
    /// path that can make a radio louder, and a `radio.json` with a mistyped
    /// figure in it should not be able to put 60 dB into somebody's headphones.
    /// The ceiling is what an ordinary transceiver's USB codec is short by,
    /// with room to spare.
    fn set_trim_db(&mut self, db: f32) {
        self.trim = if db.is_finite() { 10f32.powf(db.clamp(-20.0, 30.0) / 20.0) } else { 1.0 };
    }

    /// `left`/`right` are the speaker path (post AF-volume/mute); `rec_left`/
    /// `rec_right` are what the recorder sees instead, so turning down the AF
    /// knob or hitting mute doesn't touch the archived recording.
    fn push(
        &mut self,
        left: &[f32],
        right: Option<&[f32]>,
        rec_left: &[f32],
        rec_right: Option<&[f32]>,
    ) {
        self.main_q.extend_from_slice(left);
        let dual = match right {
            Some(s) => {
                self.sub_q.extend_from_slice(s);
                true
            }
            None => {
                self.sub_q.clear();
                false
            }
        };
        self.rec_main_q.extend_from_slice(rec_left);
        let rec_dual = match rec_right {
            Some(s) => {
                self.rec_sub_q.extend_from_slice(s);
                true
            }
            None => {
                self.rec_sub_q.clear();
                false
            }
        };

        let n = if dual { self.main_q.len().min(self.sub_q.len()) } else { self.main_q.len() };
        let rec_n = if rec_dual {
            self.rec_main_q.len().min(self.rec_sub_q.len())
        } else {
            self.rec_main_q.len()
        };

        if rec_n > 0 {
            // Recording tap: RX left, sub receiver (if any) right — and the
            // same RX in both when there is no second receiver to put there,
            // rather than a file with one silent ear. Suppressed by
            // rx_rec_enabled while transmitting.
            self.rec_split = rec_dual;
            if self.rx_rec_enabled {
                if let Some(rec) = self.rec_tap.as_mut() {
                    let need = if self.rec_mono { 1 } else { 2 };
                    for i in 0..rec_n {
                        // Never write a partial frame: a ring with one free
                        // slot pushing just the left sample would desync
                        // every following sample by a channel. Give up on the
                        // rest of the block rather than skipping samples one
                        // at a time — a stalled recorder should cost a clean
                        // gap, not a stutter stitched out of whatever happened
                        // to fit.
                        if rec.slots() < need {
                            break;
                        }
                        let l = self.rec_main_q[i];
                        if self.rec_mono {
                            let _ = rec.push(l);
                        } else {
                            let r = if rec_dual { self.rec_sub_q[i] } else { l };
                            let _ = rec.push(l);
                            let _ = rec.push(r);
                        }
                    }
                }
            }
            self.rec_main_q.drain(..rec_n);
            if rec_dual {
                self.rec_sub_q.drain(..rec_n);
            }
        }

        if n > 0 {
            if self.out.slots() >= n * 2 {
                let g = if self.tx_muted { 0.0 } else { self.duck * self.trim };
                for i in 0..n {
                    // Clamped because `trim` may be above unity, and a sample
                    // past full scale does not merely distort: the conversion
                    // to a device's own sample format is free to wrap it, and a
                    // wrap is a click at the loudest the card can go.
                    let l = (self.main_q[i] * g).clamp(-1.0, 1.0);
                    let r = if dual { (self.sub_q[i] * g).clamp(-1.0, 1.0) } else { l };
                    let _ = self.out.push(l);
                    let _ = self.out.push(r);
                }
            } else {
                self.dropped += n as u64;
                if self.dropped.is_power_of_two() {
                    warn!(dropped = self.dropped, "audio ring full, dropping");
                }
            }
            self.main_q.drain(..n);
            if dual {
                self.sub_q.drain(..n);
            }
        }
        // Safety bound if one side stalls (e.g. sub warming up).
        if self.main_q.len() > MIXER_CAP {
            let cut = self.main_q.len() - MIXER_CAP;
            self.main_q.drain(..cut);
        }
        if self.sub_q.len() > MIXER_CAP {
            let cut = self.sub_q.len() - MIXER_CAP;
            self.sub_q.drain(..cut);
        }
        if self.rec_main_q.len() > MIXER_CAP {
            let cut = self.rec_main_q.len() - MIXER_CAP;
            self.rec_main_q.drain(..cut);
        }
        if self.rec_sub_q.len() > MIXER_CAP {
            let cut = self.rec_sub_q.len() - MIXER_CAP;
            self.rec_sub_q.drain(..cut);
        }
    }

    /// Recording tap for TX audio: right channel, silence in the left (or the
    /// sole channel in mono, or both channels where the recording has no
    /// second receiver to keep them apart — see `rec_split`). Resampled from
    /// `TX_MONITOR_RATE` to match `rec_tap`.
    fn push_tx(&mut self, tx: &[f32]) {
        if self.rec_tap.is_none() {
            return;
        }
        let samples: &[f32] = match self.tx_rec_rs.as_mut() {
            Some(rs) => {
                self.tx_rec_scratch.clear();
                rs.push(tx, &mut self.tx_rec_scratch);
                &self.tx_rec_scratch
            }
            None => tx,
        };
        let rec = self.rec_tap.as_mut().expect("checked above");
        let need = if self.rec_mono { 1 } else { 2 };
        for &s in samples {
            // Whole frames only, and a clean gap rather than a stutter — see
            // the matching guard in `push`.
            if rec.slots() < need {
                break;
            }
            if self.rec_mono {
                let _ = rec.push(s);
            } else {
                // Where a second receiver holds the right channel, the over
                // stays on that side and the two ends of the QSO keep an ear
                // each; where nothing does, the file is dual mono throughout
                // and the over is centred like the receive audio around it.
                let _ = rec.push(if self.rec_split { 0.0 } else { s });
                let _ = rec.push(s);
            }
        }
    }
}

/// The transmit chain: mic 48 k → EQ → modulator → drive → DUC → device.
///
/// The EQ is not in here — it lives on the [`Engine`], because a rig that
/// modulates its own audio has no `TxChain` at all and still wants it. See
/// [`apply_tx_eq`].
struct TxChain {
    modulator: Option<Box<dyn Modulator>>,
    /// Controlled-envelope SSB, present only on the two voice sidebands.
    ///
    /// A digital mode never gets one. FT8, PSK, SSTV and the rest carry their
    /// information in the very envelope this flattens, and "compressing" one is
    /// not compression, it is distortion of the thing being sent — so the
    /// processor is not built at all rather than being built and left switched
    /// off, which is a state something could later get wrong (issue #283).
    cessb: Option<Cessb>,
    dc: DcBlock,
    duc: Duc,
    /// The DUC's output rate — what `sat_nco` has to be programmed against.
    tx_rate: f64,
    /// Satellite lock: the TX Doppler correction (plus any transponder drift
    /// since key-down) mixed onto the upconverted IQ. An NCO rather than a
    /// hardware retune because `tx_begin` sets the TX LO exactly once per
    /// over, and the correction has to keep moving through a long one —
    /// phase-continuous, so the retunes are inaudible. `None` while no lock
    /// is shifting this over.
    sat_nco: Option<Nco>,
    mod_buf: Vec<Complex32>,
    tx_buf: Vec<Complex32>,
    alc_peak: f32,
}

/// 10 ms of TX audio per iteration.
const TX_AUDIO_BLOCK: usize = 480;
/// The loudest a microphone may be over a whole voice over and still count as
/// silent — about 60 dB below full scale, which is quieter than the noise floor
/// of any sound card anyone transmits through.
const SILENT_MIC_PEAK: f32 = 0.001;
/// How far ahead of the transmitter a keying TCI client is asked to run
/// (240 ms).
///
/// The transmit clock is what drives the request — see [`TciTxPace`] — so this
/// is the whole of the buffering: the client is asked for everything the
/// transmitter has consumed *plus* this, and a chrono-honouring client's queue
/// therefore stands at this depth and cannot exceed it. It has to cover the
/// round trip from issuing a chrono to the audio landing, which for an
/// application answering off a GUI timer is a couple of hundred milliseconds.
///
/// It is also the transmit latency: the over runs this far behind the client's
/// own waveform, and `Engine::end_tci_tx` plays the remainder out rather than
/// dropping it, so the tail of a timed burst is not cut off.
const TCI_TX_LEAD: usize = TX_AUDIO_BLOCK * 24;
/// Blocks (500 ms) of complete silence from a keyed TCI client before what it
/// was asked for is written off and asked for again.
///
/// The request count is what keeps this loop from asking twice for the same
/// audio, so a chrono the client simply dropped would otherwise leave the
/// stream short by that much for the rest of the over. Long enough that a
/// client which is merely slow is never written off while its answer is still
/// on the way, and well inside [`TCI_TX_STARVE_LIMIT`], so a client that really
/// has died is still unkeyed rather than asked forever.
const TCI_TX_ASK_TIMEOUT_BLOCKS: u32 = 50;

/// How a keying TCI client is asked for transmit audio.
///
/// A `TxChrono` is a request with no acknowledgement, so the loop that issues
/// them has to do its own bookkeeping or it asks for the same missing audio on
/// every one of the blocks a slow client takes to answer the first request.
/// What comes back is then several times what the queue can hold, and the
/// overflow is not a lost buffer of microphone — it is the client's *waveform*
/// jumping forward, which is how a 15 s FT8 slot went out as a few seconds of
/// signal (issue #202).
///
/// **The demand is the transmit clock, not the queue.** That distinction is the
/// whole of this type. Pacing towards a standing queue depth — ask for whatever
/// would refill it, minus what is already on its way — looks right and is
/// unstable in exactly the case that matters: once the queue is dry it stays
/// dry, every request is answered, and the arithmetic then asks for precisely
/// what arrived and never a frame more. The ask rate is pinned to the arrival
/// rate, wherever that happens to have fallen, and the shortfall is permanent.
/// Measured against WSJT-X on a Hermes-Lite: a 13.6 s FT8 slot was asked for
/// 372 135 frames, answered with 360 436 of them — 97 % of what it was asked
/// for and 55 % of the over, with 44 % of the blocks going out as silence.
///
/// So the running total asked for is held at "everything the transmitter has
/// consumed, plus [`TCI_TX_LEAD`]". Consumption is a wall clock — one block per
/// block, whether that block was audio or the silence that stood in for it — so
/// a client that falls behind is asked for *more*, not less, and one that has
/// caught up is asked for exactly real time. It also bounds the queue by
/// construction: a client is never asked for more than the transmitter will
/// have used, so a chrono-honouring one cannot overrun the FIFO at all.
///
/// Its own type so the rules can be exercised without a socket, a rig or an
/// engine: they are a few interacting counters, and every case worth checking
/// is a sequence of blocks.
#[derive(Debug, Clone, Copy, Default)]
struct TciTxPace {
    /// Frames asked for since key-down.
    asked: usize,
    /// Frames that have arrived since key-down.
    supplied: usize,
    /// Frames the transmit clock has consumed since key-down, silence included.
    played: usize,
    /// Consecutive blocks the client has sent nothing at all.
    quiet: u32,
}

impl TciTxPace {
    /// One block: `queued` frames are in hand, `got` arrived since the last
    /// call, and `playing` says whether a block is actually going on the air
    /// (it is not during the pre-roll, which spins far faster than the transmit
    /// clock and would otherwise run the demand away). Returns the frame count
    /// to chrono for, or `None` when enough is already on its way.
    fn request(&mut self, queued: usize, got: usize, playing: bool) -> Option<u32> {
        self.supplied += got;
        if playing {
            self.played += TX_AUDIO_BLOCK;
        }
        // A client that has gone quiet for longer than any answer could take is
        // not going to send what it still owes; write that off so the next
        // block asks for it again rather than waiting on it forever.
        self.quiet = if got > 0 { 0 } else { self.quiet + 1 };
        if self.quiet >= TCI_TX_ASK_TIMEOUT_BLOCKS {
            self.quiet = 0;
            self.asked = self.supplied;
        }
        // A client that streams on its own clock and ignores chronos (as
        // sdroxide's own does against a rig that never sends one) fills the
        // queue without being asked; there is nothing to request while that
        // much of its audio is already in hand. Twice the lead rather than the
        // lead itself, because a chrono-honouring client's queue *stands* at
        // the lead and this must not fire on the ordinary case.
        if queued >= TCI_TX_LEAD * 2 {
            return None;
        }
        let want = self.played + TCI_TX_LEAD;
        let deficit = want.saturating_sub(self.asked);
        if deficit < TX_AUDIO_BLOCK {
            return None;
        }
        self.asked += deficit;
        Some(deficit as u32)
    }

    /// A new over: none of the last one's counters carry into it.
    fn rekey(&mut self) {
        *self = TciTxPace::default();
    }
}

/// Consecutive short TX blocks (1.5 s) before we conclude a keyed TCI client
/// has died and unkey. A brief gap is normal on a WebSocket and must not chop
/// the over — half a transmitted FT8 burst decodes nowhere.
const TCI_TX_STARVE_LIMIT: u32 = 150;
/// IQ rate advertised to TCI clients before any of them has picked one — the
/// widest of TCI's standard rates, snapped to a divisor of the device rate.
const TCI_IQ_DEFAULT_HZ: f64 = 192_000.0;
/// Queue bound for a TCI over (0.5 s). Deliberately looser than the mic's
/// 100 ms: network audio arrives in bursts, and this only trims a client whose
/// clock genuinely runs fast — nobody can transmit faster than real time.
const TCI_TX_FIFO_CAP: usize = 24_000;
/// Sample rate of the TX baseband/audio fed to the TX-monitor analyzer.
const TX_MONITOR_RATE: f64 = 48_000.0;
/// The TX monitor's baseband/IQ runs near digital full scale (~0 dBFS), far
/// hotter than any received signal, so on the shared floor/ceil it would clamp
/// the waterfall to maximum. Dim it so the strongest TX lands this many dB below
/// the display ceiling — i.e. about as bright as a strong received signal.
const TX_MON_HEADROOM_DB: f32 = -30.0;

/// The gain that brings a digital mode's modulating signal up to full scale.
///
/// [`sdroxide_digi::DigiEngine::tx_peak`] is the fraction of full scale the
/// mode's own synthesiser reaches — half of it for nearly all of them, which
/// is 6 dB of headroom the transmitter must not be made to pay for. Anything
/// outside `(0, 1]` is not a headroom figure at all, so it is read as "already
/// full scale" and the over goes out at the level the mode built it at, rather
/// than at some wild multiple of it.
fn digi_tx_gain(peak: f32) -> f32 {
    if peak > 0.0 && peak <= 1.0 { 1.0 / peak } else { 1.0 }
}

/// Wall-clock pace one produced TX block to real time so the downstream buffer
/// (sound card, HPSDR/TCI network ring) stays near-empty instead of filling to
/// its full 0.5–1 s depth. Every backend's `tx_write` already blocks on
/// backpressure, but only *once the ring is full* — that is the latency. This
/// caps the feed AT real time (never slower: `checked_sub` yields no sleep when
/// we're already behind), so it can only *reduce* buffering, never starve a
/// consumer that was keeping up. A head-start of `cushion_ms` is fed out before
/// pacing engages, so the hardware/network consumer downstream has that much
/// slack against jitter or a consumer clock slightly faster than nominal
/// 48 kHz — see [`IqSource::tx_pace_cushion_ms`].
fn pace_tx_block(tx_pace: &mut Option<(Instant, u64)>, cushion_ms: f64) {
    let cushion = (cushion_ms.max(0.0) / 1000.0 * TX_MONITOR_RATE) as u64;
    let (start, fed) = tx_pace.get_or_insert_with(|| (Instant::now(), 0));
    *fed += TX_AUDIO_BLOCK as u64;
    let paced = fed.saturating_sub(cushion);
    let target = Duration::from_secs_f64(paced as f64 / TX_MONITOR_RATE);
    if let Some(d) = target.checked_sub(start.elapsed()) {
        std::thread::sleep(d);
    }
}

/// Shape one block of microphone audio with the transmit EQ, retuning `eq`
/// first if the operator has moved a band since the last block.
///
/// Voice only: every caller is a voice branch. A digital-mode burst and a CW
/// keyer's sidetone are synthesised at exactly the shape they need to land on
/// the air with, and colouring them would only cost decodes.
///
/// A free function, like [`pace_tx_block`], so it can be called from a branch
/// that is already holding a `&mut` borrow of the [`TxChain`] — these are
/// disjoint fields of the engine, and a method taking `&mut self` would not be.
fn apply_tx_eq(
    eq: &mut ParametricEq,
    built_from: &mut TxEqState,
    want: &TxEqState,
    audio: &mut [f32],
) {
    if built_from != want {
        eq.configure(want, TX_MONITOR_RATE);
        *built_from = *want;
    }
    // Retuned even while disabled, so switching it on mid-over starts from the
    // operator's real settings rather than a block of whatever was there last.
    if want.enabled {
        eq.process(audio);
    }
}

/// Put the repeater signalling on one block of outgoing FM audio: the 1750 Hz
/// burst while one is running, and the sub-audible tone under the voice
/// otherwise. Returns true when a burst finished inside this block.
///
/// FM only, and the caller checks that: a CTCSS tone is a slice of an FM
/// channel's deviation budget and means nothing on a sideband, where the same
/// 88.5 Hz would simply be an audible hum on the operator's audio.
///
/// A free function, like [`apply_tx_eq`] and for the same reason: both callers
/// are already holding a `&mut` borrow of the [`TxChain`], and a method taking
/// `&mut self` could not be called from there.
fn apply_tx_signalling(
    sub: &mut Option<SubToneGen>,
    burst: &mut Option<ToneBurst>,
    audio: &mut [f32],
) -> bool {
    if let Some(b) = burst.as_mut() {
        // The burst replaces the microphone rather than mixing with it. A
        // repeater's decoder is listening for a clean 1750 Hz tone, and every
        // radio that has this button mutes the microphone behind it.
        let n = b.fill(audio);
        if !b.finished() {
            return false;
        }
        *burst = None;
        // Whatever is left of the block once the burst ends is voice again,
        // and still wants its tone under it.
        if let Some(g) = sub.as_mut() {
            g.mix(&mut audio[n..]);
        }
        return true;
    }
    if let Some(g) = sub.as_mut() {
        g.mix(audio);
    }
    false
}

/// Unix seconds as a float — the time base the satellite propagator runs on.
fn unix_now_f64() -> f64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

/// Convert Unix seconds to a UTC civil date-time `(year, month, day, hour, min,
/// sec)`. Howard Hinnant's `civil_from_days` algorithm — exact, no leap-second
/// or timezone handling (UTC), and no external crate.
fn utc_civil(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, mi, s) = ((rem / 3600) as u32, ((rem % 3600) / 60) as u32, (rem % 60) as u32);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as i64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if month <= 2 { year + 1 } else { year }, month, day, h, mi, s)
}

impl TxChain {
    fn new(mode: Mode, tx_rate: f64, passband: (f32, f32)) -> Self {
        TxChain {
            modulator: make_modulator(mode, 48_000.0, passband),
            cessb: matches!(mode, Mode::Usb | Mode::Lsb)
                .then(|| Cessb::new(48_000.0, passband.0, passband.1)),
            dc: DcBlock::new(100.0, 48_000.0),
            duc: Duc::new(48_000.0, tx_rate),
            tx_rate,
            sat_nco: None,
            mod_buf: Vec::new(),
            tx_buf: Vec::new(),
            alc_peak: 0.0,
        }
    }
}

/// The receive chain over one block, with everything it touches handed in.
///
/// Free-standing rather than a method on [`Engine`] because it is one half of a
/// fork: the other half holds `&mut Engine`, so this one may hold nothing of
/// it. `out` is (speaker left, speaker right, recorder left, recorder right).
///
/// The audio is copied out rather than left borrowed from the chain because a
/// digital-voice mode may replace it wholesale, and deciding that needs the
/// digi engine — which would otherwise be borrowed against the chain.
fn run_chain_block(
    chain: &mut RxChain,
    rx: &RxState,
    iq: &[Complex32],
    want_rec: bool,
    out: (&mut Vec<f32>, &mut Vec<f32>, &mut Vec<f32>, &mut Vec<f32>),
) {
    let (play, play_r, rec, rec_r) = out;
    play.clear();
    play_r.clear();
    let (audio, right) = chain.run(iq, rx, want_rec);
    play.extend_from_slice(audio);
    if let Some(r) = right {
        play_r.extend_from_slice(r);
    }
    rec.clear();
    rec_r.clear();
    if want_rec {
        let (rec_audio, rec_right) = chain.take_rec_audio();
        rec.extend_from_slice(rec_audio);
        if let Some(r) = rec_right {
            rec_r.extend_from_slice(r);
        }
    }
}

/// Binaural (pseudo-stereo) audio: spread the receive passband across the two
/// ears, so that pitch becomes direction and tuning a signal floats it from one
/// ear to the other (issue #263). What it actually does to the audio is
/// [`sdroxide_dsp::Binaural`]'s business; this is where the receiver decides
/// whether it runs at all.
///
/// `mono` comes in as the speaker audio and goes out as the left ear, with the
/// right in `right`; `scratch` is where the left is built before the two are
/// swapped, so nothing is copied twice. Both are left exactly as they were —
/// and the widener dropped, rather than left holding a filter's worth of stale
/// audio — whenever it is not wanted:
///
/// * the operator has not asked for it, or the mode is not one that has it
///   ([`Mode::binaural_audio`]);
/// * something else already owns the right ear. The sub receiver claims it
///   explicitly and WFM stereo fills it automatically, and neither is worth
///   giving up for an effect: the same order of precedence the stereo
///   broadcast itself yields to.
///
/// A free function rather than a method for the reason [`run_chain_block`] is
/// one: the buffers it works on and the widener that owns the filter state are
/// separate fields of the engine, and both receive paths — a demodulated radio
/// and a transceiver handing over its own audio — have to reach it.
fn binaural_split(
    slot: &mut Option<Binaural>,
    rx: &RxState,
    rate: f64,
    right_free: bool,
    mono: &mut Vec<f32>,
    scratch: &mut Vec<f32>,
    right: &mut Vec<f32>,
) {
    if !rx.binaural || !rx.mode.binaural_audio() || !right_free || !right.is_empty() {
        *slot = None;
        return;
    }
    let b = slot.get_or_insert_with(|| Binaural::new(rate, rx.filter_lo, rx.filter_hi));
    // Both re-asserted per block and both no-ops when nothing has moved: the
    // speaker's rate changes when the operator picks another sound card, and
    // the passband every time a filter edge is dragged.
    b.set_rate(rate);
    b.set_passband(rx.filter_lo, rx.filter_hi);
    scratch.clear();
    b.process(mono, scratch, right);
    std::mem::swap(mono, scratch);
}

/// A satellite lock in progress: the parsed propagator, who is watching from
/// where, and the numbers most recently computed from them.
struct ActiveSatLock {
    cfg: sdroxide_types::SatLockConfig,
    sat: sdroxide_solar::Satellite,
    /// Observer geodetic (lat, lon), degrees.
    observer: (f64, f64),
    /// The latest computation — what `update_tuning`, the TX path and the
    /// status stream all read.
    track: sdroxide_types::SatTrackStatus,
    next_calc: Instant,
    next_emit: Instant,
    /// Unix time after which the slow work runs again: the next-pass summary,
    /// and — once the elements have gone stale — another look at the caches.
    next_slow_unix: f64,
}

/// How often the lock's geometry is recomputed. LEO Doppler at UHF moves at up
/// to ~75 Hz/s near closest approach; five recomputes a second keep the applied
/// correction within ~15 Hz of the truth, and the NCO retunes are free.
const SAT_CALC_INTERVAL: Duration = Duration::from_millis(200);
/// How often the [`RadioEvent::SatTrack`] stream goes out — readout rate, not
/// correction rate.
const SAT_EMIT_INTERVAL: Duration = Duration::from_millis(500);
/// The slow lane: pass prediction is a search, not a lookup, and stale-element
/// recovery reads the TLE caches off disk. Neither belongs at 5 Hz.
const SAT_SLOW_INTERVAL_S: f64 = 300.0;

/// A front end's gain stages as `(element name, dB)` — the shape the device
/// API, [`RadioState::gains`] and `session.json` all use.
type GainSet = Vec<(String, f64)>;

/// The panadapter's zoom lane: the window the operator is actually looking at,
/// mixed down to baseband and decimated to its own width, so the FFT over it
/// resolves the *view* rather than the whole of what the front end streams.
///
/// The device-wide analyser is a fixed number of bins across whatever is
/// arriving, so the further in the operator zooms the fewer of them land on
/// screen — and a wide front end runs out of them long before they have
/// finished zooming. An RX-888 asked for 8.1 MHz gives 247 Hz a bin through the
/// largest FFT the display will ask for, which draws a 68 kHz window out of 275
/// numbers and stair-steps visibly. Front-end decimation was the only cure, and
/// it buys the resolution by throwing the rest of the band away.
///
/// So this lane costs the band nothing: the raw stream is still analysed whole
/// for the zoomed-out view, the skimmers and every other lane, and this is one
/// more decimation off the same samples — the arrangement the CW skimmer and
/// the digital channel analyser already use.
struct ZoomLane {
    ddc: Ddc,
    analyzer: SpectrumAnalyzer,
    /// The lane's own axis: where the DDC is pointed and the width it produces.
    /// Also what the frame built from it is described by.
    center_hz: f64,
    rate_hz: f64,
    /// What this lane was built for. All three are its identity: a zoom that
    /// changes the decimation needs new filters, and a front end that changes
    /// rate invalidates the whole ladder — where a pan that moves neither only
    /// needs the NCO re-pointed.
    in_rate_hz: f64,
    decim: u32,
    /// Where the NCO is pointed now, as an offset from the front end's centre.
    /// A retune moves the front end out from under the lane, and this has to
    /// follow or the window would slide across the band with it.
    offset_hz: f64,
    buf: Vec<Complex32>,
}

impl ZoomLane {
    /// A lane covering `in_rate / decim` centred on `center_hz`, with the front
    /// end currently on `dev_center_hz`, analysed `fft` points at a time.
    ///
    /// `fft` comes from [`zoom_lane_fft`] and so from the width the client is
    /// drawing: a lane that resolved a 2048-column display would stair-step a
    /// 4096-column one, which is the whole complaint this lane exists to answer,
    /// one zoom level further in.
    fn new(
        in_rate_hz: f64,
        decim: u32,
        center_hz: f64,
        dev_center_hz: f64,
        avg_tc: f32,
        fft: usize,
        view_span_hz: f64,
        rows_per_sec: f64,
    ) -> Self {
        // The ladder is powers of two, so `Ddc` reaches this rate exactly and
        // `out_rate` is a formality — read back rather than assumed, because
        // the frame's axis has to be the width actually produced.
        let mut ddc = Ddc::new(in_rate_hz, in_rate_hz / f64::from(decim));
        let offset_hz = center_hz - dev_center_hz;
        ddc.set_offset_hz(offset_hz);
        let rate_hz = ddc.out_rate();
        // How much signal one transform looks at — set by what is on screen and
        // not by how many columns are drawing it, which is the whole of issue
        // #302. The rest of `fft` is zero padding, so the trace still has a
        // point per column.
        let window = analysis_window(fft, rate_hz, view_span_hz);
        // The same rule the device-wide lane uses, and asked about the window
        // rather than the transform: the hop is a step through the *signal*.
        // At the rates a zoom lane runs at it returns the eighth-hop this used
        // to hard-code, and it stops asking for one on a shallow zoom that is
        // still streaming megahertz.
        let hop_div = hop_div_for(rate_hz, window, rows_per_sec);
        let mut analyzer = SpectrumAnalyzer::with_window(fft, window, rate_hz, avg_tc, hop_div);
        // DC here is the middle of the operator's window, not the front end's
        // LO leakage, so the usual spike suppression would punch a hole through
        // whatever they had centred.
        analyzer.set_dc_suppress(false);
        ZoomLane {
            ddc,
            analyzer,
            center_hz,
            rate_hz,
            in_rate_hz,
            decim,
            offset_hz,
            buf: Vec::new(),
        }
    }

    /// Whether this lane is the one `want` asks for, on a front end streaming
    /// `in_rate_hz`. A lane that is keeps its filters and its averaging — and
    /// so keeps the waterfall running — across everything except a real change
    /// of window.
    ///
    /// **Where the window sits is not part of that.** Moving it is a matter of
    /// re-pointing the NCO ([`ZoomLane::aim`]), and the filter ladder behind it
    /// does not care where it was pointed; only the decimation it runs at and
    /// the rate it is fed decide what those filters are. A pan sends a viewport
    /// once per displayed frame, so a lane that treated a moved window as a
    /// different lane rebuilt itself sixty times a second — and each of those
    /// replacements held no spectrum at all until its first transform, which is
    /// a quarter of a second at the rate a zoom lane runs at. The waterfall
    /// went black for the whole of the drag and stayed black until after the
    /// operator let go.
    fn serves(&self, want: (f64, u32), in_rate_hz: f64) -> bool {
        self.decim == want.1 && (self.in_rate_hz - in_rate_hz).abs() < 0.5
    }

    /// Put the lane's window on `center_hz` with the front end on
    /// `dev_center_hz` — either of which may have moved: the operator drags the
    /// window across the band, and the panadapter's own pan carries the front
    /// end under it.
    ///
    /// The NCO retunes phase-continuously and the decimator keeps its state, so
    /// this costs a `sin`/`cos` and the averaging survives. The one thing that
    /// does not survive is the filter's memory of the old window, which washes
    /// out over a few tap lengths — a hundredth of what rebuilding costs.
    ///
    /// Written only when it is worth a write: the comparison is against the
    /// offset actually in force, not against the last request, so a drag of
    /// many sub-hertz steps cannot creep away from the window it is labelled
    /// with.
    fn aim(&mut self, center_hz: f64, dev_center_hz: f64) {
        self.center_hz = center_hz;
        let want = center_hz - dev_center_hz;
        if (want - self.offset_hz).abs() >= 0.5 {
            self.offset_hz = want;
            self.ddc.set_offset_hz(want);
        }
    }

    fn process(&mut self, iq: &[Complex32]) {
        self.buf.clear();
        self.ddc.process(iq, &mut self.buf);
        self.analyzer.process(&self.buf);
    }
}

/// The listening position a VFO keeps while the other one is in use: the mode
/// it was left in and the passband that went with it.
///
/// The filter travels with the mode because it belongs to it — selecting a mode
/// installs that mode's default width, so a VFO restored on its mode alone
/// would come back with a passband the operator had already narrowed and lost.
#[derive(Debug, Clone, Copy, PartialEq)]
struct VfoMemory {
    mode: Mode,
    filter_lo: f32,
    filter_hi: f32,
}

impl VfoMemory {
    /// What the receiver is listening in right now, ready to be shelved under
    /// the VFO that is being left.
    fn of(rx: &RxState) -> VfoMemory {
        VfoMemory { mode: rx.mode, filter_lo: rx.filter_lo, filter_hi: rx.filter_hi }
    }
}

struct Engine {
    source: Box<dyn IqSource>,
    caps: DeviceCaps,
    state: RadioState,
    cfg: SpectrumConfig,
    /// Where the centre has been, newest last, for a front end whose samples
    /// arrive later than the command that moved them
    /// ([`IqSource::stream_delay_s`]).
    ///
    /// The panadapter's axis is a statement about the samples in the frame, not
    /// about the last order given to the radio, and on a radio at the end of a
    /// socket those are a tenth of a second apart. Empty — and never even
    /// looked at — for every source that answers zero.
    ///
    /// Stamped with [`Engine::samples_read`] rather than with a clock. The
    /// question is "how much signal has gone by since that retune", and a wall
    /// clock answers it only while nothing is queued: a frame is built some
    /// time *after* the block it draws was read, and at a drag's speed that gap
    /// is worth as much error as a whole step. Counting samples is the same
    /// arithmetic against the clock that actually carries the data.
    center_trail: VecDeque<(u64, f64)>,
    /// The centre the samples now arriving were taken at: the last entry of
    /// [`Engine::center_trail`] to have aged in. Seeded from the front end's
    /// own centre at open, and re-seeded whenever the source is replaced.
    ///
    /// Only meaningful on a front end that *declares* a stream delay — the
    /// others have no trail to age anything in, and this stays on its seed
    /// however far the dial travels. Read it through
    /// [`Engine::stream_center_now`], never directly.
    stream_center_hz: f64,
    /// Device-rate samples this engine has taken from the front end, ever.
    ///
    /// A position on the stream, not a statistic — the only thing that reads it
    /// is [`Engine::display_center_hz`]. At 32 Msps a `u64` runs for eighteen
    /// thousand years.
    samples_read: u64,
    /// How many samples the newest block held.
    ///
    /// [`Engine::samples_read`] is the position at the *end* of that block, and
    /// the picture drawn from it covers the whole of it — so taking the end as
    /// "when this was" reads the stream half a block too new, every time. On a
    /// pan that bias is half a block of drag, and it showed up as a frame here
    /// and there jumping a whole step ahead of the rest.
    last_block: u64,
    analyzer: SpectrumAnalyzer,
    /// The newest finished sweep from a source that computes its own spectrum.
    ///
    /// Cached rather than handed straight to the strip, because two lanes may
    /// want the same sweep: the full-band strip takes every one, and on a
    /// receive path with no I/Q at all the *main* panadapter is drawn from
    /// these bins too — see [`Engine::scope_main_window`]. Reused so the poll
    /// does not allocate.
    wide_bins: Vec<f32>,
    /// Centre and span the cached sweep covers, and when it landed.
    wide_window: Option<(f64, f64)>,
    wide_at: Instant,
    /// Sweeps a front end with its own spectrum has finished since this engine
    /// started.
    ///
    /// Diagnostic, and the one number that says how much of the picture is
    /// real on a rig whose scope *is* the main panadapter: the waterfall is
    /// scrolled on the wall clock there, so at three sweeps a second and a
    /// hundred rows a second every sweep is drawn thirty times over. Counted
    /// beside the transform and frame rates in the panadapter diagnostic, which
    /// is where the same question gets asked about the other lanes.
    wide_sweeps: u64,
    /// Whether the cached sweep is one the full-band lane has not published.
    /// The main lane has no such flag: it emits at the display rate whether or
    /// not a new sweep arrived, exactly as the FFT analyser does.
    wide_fresh: bool,
    /// Sequence number for full-band frames, kept apart from the main
    /// analyser's so a client can tell one lane's frames from the other's.
    wide_seq: u32,
    /// Samples fed to the panadapter since the last row was clocked.
    ///
    /// The waterfall's time axis is measured in *signal*, not in wall clock or
    /// in blocks: a source hands over a block a few dozen times a second, and
    /// clocking a row per block would cap the waterfall at the block rate for
    /// no better reason than the size of a read. Counting samples instead puts
    /// the rows exactly evenly along the axis they are drawn on, at whatever
    /// rate was asked for, up to the rate the analyser produces transforms.
    row_samples: usize,
    /// Whether rows are being clocked off the sample stream (a wideband I/Q
    /// path) rather than off the wall clock (a demod-audio rig, a radio's own
    /// sweep — lanes where a block *is* the update).
    row_sample_clock: bool,
    /// Rows clocked since this engine started. Diagnostic; see the
    /// `sdroxide::panadapter` log.
    rows_clocked: u64,
    /// Of those, the ones that reached a published frame. Diagnostic: the two
    /// part company when a row is built on one picture and the frame that would
    /// have carried it is of another, which is what a waterfall scrolling
    /// slower than its own time axis looks like from here.
    rows_sent: u64,
    /// The receive chain's DDC output rate, as of the top of this block.
    ///
    /// Cached rather than asked of the chain, because the frame builder may run
    /// while the chain is away on another core — see [`Engine::process_block`]
    /// — and a digital mode's channel analyser is described by this number. It
    /// changes only when the chain is rebuilt, which is never inside a block.
    channel_rate_hz: f64,
    /// Waterfall rows clocked since the last frame went out, oldest first,
    /// `row_axis`'s width apiece. Emptied into every published frame.
    row_batch: Vec<u8>,
    /// The axis and width the rows in hand were pooled on. A zoom, a retune or
    /// a change of lane makes the older rows a picture of somewhere else, so
    /// they are thrown away rather than drawn on the new axis.
    row_axis: Option<(f64, f64, usize)>,
    /// Sequence number for main-lane frames built from those same bins. A third
    /// counter because a client de-duplicates each lane by its own sequence,
    /// and the two lanes emit at different rates.
    scope_seq: u32,
    /// Auto-ranged dB window for the full-band lane, smoothed across frames.
    wide_levels: Option<(f32, f32)>,
    event_tx: Sender<RadioEvent>,
    main: Option<RxChain>,
    sub: Option<RxChain>,
    mixer: Option<StereoMixer>,
    audio_out_rate: f64,
    /// Active MP3 recording of the receiver audio, if any.
    recorder: Option<Recorder>,
    cal_offset_db: f32,
    stacks: BandStacks,
    memories: Vec<MemoryChannel>,
    mem_folders: Vec<MemoryFolder>,
    mic: Option<MicParams>,
    mic_resampler: Option<MonoResampler>,
    /// The rate the digital-mode engine synthesises at — the receive tap's
    /// rate, because a `DigiEngine` keeps one clock for both directions.
    digi_rate: f64,
    /// Rate-matching for the digital modes' transmit audio, when the radio
    /// consumes it at a different rate from the one it was synthesised at.
    /// `None` when the two agree, which is the usual case.
    digi_tx_rs: Option<MonoResampler>,
    /// Resampled transmit audio waiting to be handed to the radio, and whether
    /// the modem behind it has finished. The over is over when both are done:
    /// unkeying on the modem alone would cut off whatever is still queued.
    digi_tx_fifo: Vec<f32>,
    digi_tx_scratch: Vec<f32>,
    digi_tx_done: bool,
    mic_fifo: Vec<f32>,
    /// Voice-only parametric EQ on the microphone audio, ahead of whatever
    /// modulates it. Here rather than in [`TxChain`] because a rig that
    /// modulates its own audio (a CAT rig's sound card, TCI, Icom LAN, a FLEX)
    /// never builds one, and the EQ applies to those exactly as it does to a
    /// radio we modulate ourselves — it is the last point at which this
    /// program still owns the audio either way. See [`apply_tx_eq`].
    tx_eq: ParametricEq,
    /// The [`TxEqState`] `tx_eq`'s coefficients were last built from, so a
    /// block that hasn't changed the EQ skips recomputing them. Same idea as
    /// how `RxChain::run` reconfigures NR/notch only on change.
    tx_eq_cfg: TxEqState,
    tx: Option<TxChain>,
    tx_active: bool,
    /// The loudest the microphone got this over, and how many blocks of it were
    /// modulated — see [`Engine::report_silent_microphone`].
    voice_peak: f32,
    voice_blocks: u32,
    /// Whether the radio's own PTT line is what is holding this over — set by
    /// [`Self::apply_hw_ptt`], and the reason its key-up is honoured.
    hw_ptt: bool,
    /// The transceiver in front of us is transmitting on its own — someone has
    /// their hand on its microphone. See [`ControlUpdate::RigTx`]: this engine
    /// is not driving that over and must not try to, so nothing here keys, and
    /// nothing here modulates. It only watches.
    rig_tx: bool,
    /// The transmit frequency the source has already been told, so
    /// [`Self::push_tx_freq`] only speaks when it moves.
    tx_freq_told: Option<f64>,
    /// This band's transmit drive calibration in dB of output power, resolved
    /// from `radio.json`'s table against wherever we would transmit (issue
    /// #295).
    ///
    /// Cached rather than looked up per block: it is a band-plan search and a
    /// file the engine deliberately does not hold, and it can only change when
    /// the transmit frequency moves or the operator edits the table — which is
    /// exactly where it is refreshed ([`Self::refresh_drive_trim`]).
    drive_trim_db: f32,
    /// The operator's table, kept so a retune can be answered without going
    /// back to the store. Empty on the overwhelming majority of stations, which
    /// is what makes [`Self::refresh_drive_trim`] free there.
    drive_trim: Vec<sdroxide_types::BandDriveTrim>,
    /// The operator's fixed receive-audio trim, in dB — the other part of
    /// `radio.json` the engine keeps a copy of, and for the same reason as
    /// `drive_trim`: the mixer that applies it is rebuilt whenever the sound
    /// card changes, and a rebuilt mixer has to be told again.
    /// See [`sdroxide_types::RadioConfig::rx_audio_gain_db`].
    rx_af_gain_db: f32,
    tx_center_hz: f64,
    tx_ham_only: bool,
    /// SWR guard: trip threshold, and the latch it sets.
    ///
    /// ⚠️ The count is the part that makes this usable rather than infuriating.
    /// SWR is meaningless until forward power has actually risen, and the first
    /// readings after key-up are routinely nonsense, so a bare `swr > limit`
    /// would abort a large share of perfectly good overs — and a protection
    /// that cries wolf gets switched off, which leaves the operator worse off
    /// than having none. It therefore needs [`SWR_TRIP_SAMPLES`] consecutive
    /// readings over the limit, each with forward power above
    /// [`SWR_MIN_FWD_W`], before it fires.
    swr_guard: bool,
    swr_limit: f32,
    /// Consecutive over-limit meter samples seen so far this over.
    swr_over: u16,
    /// Meter samples since this over began, counted so the key-up transient can
    /// be ignored by TIME rather than by forward power. See [`SWR_MIN_FWD_W`]
    /// for why the power-based version of this was wrong.
    ///
    /// `u16` rather than `u8` because a tune's grace period is measured in
    /// seconds, and [`SWR_TUNE_SETTLE_SAMPLES`] alone is over half of what a
    /// byte can hold.
    swr_settle: u16,
    /// Whether the samples counted above were taken during a tune, so that
    /// switching between the two sets of rails ([`swr_rails`]) inside one
    /// key-down restarts the grace period rather than carrying a spent one
    /// across.
    swr_tuning: bool,
    /// Set when the guard has fired, holding the SWR that fired it. While this
    /// is `Some`, transmit is refused. Cleared only by the operator
    /// acknowledging it ([`Command::ClearSwrTrip`]), which is the "latch"
    /// half: a fault that clears itself the moment you release PTT teaches you
    /// nothing and lets you keep transmitting into it.
    swr_tripped: Option<f32>,
    /// TX monitor: FFTs the transmitted 48 kHz baseband (the modulator output,
    /// or the outgoing audio for a CAT rig) so the operator sees their own signal
    /// on the panadapter while transmitting.
    tx_analyzer: SpectrumAnalyzer,
    /// Scratch for packing real TX audio into complex samples for `tx_analyzer`.
    tx_mon_buf: Vec<Complex32>,
    /// Phase accumulator for the TUNE tone on audio-modulated rigs (CAT/TCI),
    /// which need an audio carrier to key up.
    tune_phase: f32,
    /// The CTCSS tone or DCS stream going out under the voice, built from
    /// [`RadioState::repeater`] and rebuilt whenever that changes. `None` with
    /// the tone off, which is also what every non-FM mode gets: sub-audible
    /// signalling is a property of an FM channel and means nothing on SSB.
    sub_tone: Option<SubToneGen>,
    /// The 1750 Hz burst in progress, if one is.
    burst: Option<ToneBurst>,
    /// Unkey when that burst finishes — set when it was fired from receive
    /// ([`Command::ToneBurst`]), which keys the transmitter for the length of
    /// the burst and no longer.
    burst_unkeys: bool,
    /// The dial the automatic repeater shift was last resolved against, so the
    /// band plan is only consulted when the dial has actually moved.
    auto_shift_dial: Option<f64>,
    nb: NoiseBlanker,
    /// Converter headroom, read straight off the samples the device handed
    /// over. Fed before the blanker and before decimation — see
    /// [`sdroxide_dsp::AdcMeter`], which records why either of those would
    /// erase the evidence.
    adc: AdcMeter,
    /// Auto-notch + spectral NR for the CAT/demod-audio path (the IQ path uses
    /// per-`RxChain` instances instead).
    audio_notch: AutoNotch,
    audio_notch_on: bool,
    audio_nr: SpectralNr,
    audio_sbnr: SpecBleachNr,
    audio_nnr: NeuralNr,
    audio_dfnr: Option<Box<DeepFilterNr>>,
    audio_dfnr_failed: bool,
    audio_nr_level: NrLevel,
    /// Digital-mode engine (slotted FT8/FT4 or continuous PSK/RTTY), present
    /// only while a digital mode is active.
    digi: Option<Box<dyn DigiEngine>>,
    digi_config: DigiConfig,
    /// `digi_config` holds a change that is not on disk yet.
    ///
    /// Only `Command::SetDigiTxLevel` sets it: every other route through this
    /// configuration saves as it goes, and can, because each is a discrete act
    /// by the operator. The transmit-audio rail is a drag — one command per
    /// frame for as long as it lasts — so it is applied at once and written by
    /// [`Engine::flush_digi_config`] on the periodic tick and at shutdown, the
    /// way the session is.
    digi_dirty: bool,
    /// The band the running controller's transmit offset belongs to, so a move
    /// to another one can be noticed. `None` means "not yet applied", which is
    /// how a fresh controller asks for its band's stored offset: startup and a
    /// mode change both go through the same path as a retune that way, rather
    /// than each needing its own hook.
    digi_tx_band: Option<sdroxide_types::Band>,
    /// True while the current TX burst is driven by the digi engine.
    digi_tx: bool,
    /// WSPR band hopping has been stood down because the operator moved the
    /// dial. Cleared when the setup is applied again, which is how it is turned
    /// back on — the same bargain the scanner strikes in
    /// [`Engine::stop_scan_for_operator`], and for the same reason.
    hop_suspended: bool,
    /// Voice keyer: ten recorded messages plus whichever is being recorded or
    /// transmitted right now.
    voice: VoiceKeyer,
    /// The five transmit-image presets and their overlay messages. Owned here
    /// rather than by whichever screen is attached: the pictures an operator
    /// sends belong to the station, and a browser tab showing five empty slots
    /// beside a console showing five full ones was the bug.
    images: crate::image_store::ImagePresetStore,
    /// Directory walks, thumbnailing, full-size reads and upload decoding — all
    /// of it off this thread, because this loop is the audio loop and decoding
    /// a two-megapixel chart in it drops a block. Drained by `poll_images`.
    gallery: crate::image_store::GalleryWorker,
    /// True while the voice keyer owns the transmitter. Set from the moment it
    /// keys up until the over has fully ended — including a digital-voice tail
    /// after the message itself has played out, so the live microphone can
    /// never leak into the end of a keyer over.
    voice_tx: bool,
    /// Scratch for microphone audio on its way into a recording.
    voice_rec_buf: Vec<f32>,
    /// Local monitor ("preview") playback: the message resampled to the speaker
    /// rate and queued, so each audio block takes exactly the samples it needs
    /// and the monitor plays at real time without a ring of its own.
    voice_prev_q: Vec<f32>,
    voice_prev_rs: Option<MonoResampler>,
    voice_prev_rate: f64,
    /// The monitored block handed to whichever speaker path is in use.
    voice_prev_out: Vec<f32>,
    /// When the current keyer over was requested, so one that never reached the
    /// air (the transmit rails refused, a digital-voice burst was aborted)
    /// releases the keyer instead of leaving it stuck "transmitting".
    voice_started: Option<Instant>,
    /// When the running record/playback position was last published. The status
    /// is otherwise event-driven; this paces the moving-position updates.
    voice_tick: Option<Instant>,
    /// Wall-clock pacer for audio-mode digi TX: (burst start, samples fed at
    /// 48 kHz). Ensures the burst plays at real time even if the sound card
    /// drains its ring faster than real time (otherwise FT8/FT4 finish early).
    tx_pace: Option<(std::time::Instant, u64)>,
    /// High-resolution spectrum over the VFO channel (digital modes only):
    /// fed the decimated channel IQ so an FFT gives ~3 Hz/bin resolution.
    channel_analyzer: Option<SpectrumAnalyzer>,
    /// The panadapter's zoomed window at its own resolution — see [`ZoomLane`].
    /// `None` while the device-wide analyser still has a bin for every column
    /// of the display, which is most of the time on a narrow front end and
    /// almost never on a wide one.
    zoom: Option<ZoomLane>,
    /// CW skimmer: a dedicated wideband decimator off the raw IQ plus a
    /// worker-thread decoder, present only while the skimmer is enabled.
    skim_ddc: Option<Ddc>,
    skimmer: Option<SkimmerController>,
    skim_buf: Vec<Complex32>,
    /// The client's visible waterfall window, in absolute Hz; `None` until a
    /// client says otherwise (a headless server skims the whole window).
    skim_view: Option<(f64, f64)>,
    /// Where the skim window is currently centred, in absolute Hz, so a retune
    /// or a pan can tell whether it has to move. Meaningless while `skim_ddc`
    /// is `None`.
    skim_center_hz: f64,
    /// The front-end rate the skim chain was built from — the one thing about
    /// it that cannot be retuned in place. See [`Engine::sync_skim_window`].
    skim_in_rate: f64,
    /// The operator's persisted skimmer preference. Distinct from
    /// `state.skimmer`, which is the *live* setting and is forced off on an
    /// audio-mode source — this is what a wideband source gets restored to.
    skim_cfg: sdroxide_types::SkimmerSettings,
    /// ISM decoder: its own decimation of the raw IQ onto the 868 MHz channel
    /// plan, plus a worker thread, present only while the decoder is enabled.
    ///
    /// A second window rather than a share of the skimmer's: that one is 192 kHz
    /// wide and follows the operator's waterfall, and the ISM plan needs about
    /// 1.5 MHz placed on 868.9 MHz.
    ism_ddc: Option<Ddc>,
    ism: Option<IsmController>,
    ism_buf: Vec<Complex32>,
    /// Where the ISM window is currently centred, so a retune can tell whether it
    /// has to move.
    ism_center_hz: f64,
    /// The stream rate `ism_ddc` was built to decimate, so a retune can tell
    /// whether the chain still belongs to the front end feeding it. A `Ddc`
    /// bakes its input rate into both the NCO and the decimation chain and
    /// neither can be changed in place.
    ism_in_rate: f64,
    /// The operator's persisted ISM preference, kept apart from `state.ism` for
    /// the same reason as `skim_cfg`.
    ism_cfg: sdroxide_types::IsmSettings,

    /// The ADS-B lane (issue #160): a third window, on 1090 MHz.
    ///
    /// Its own rather than a share of anything else's for the plainest reason
    /// in the tree — it is two and a half megahertz wide and a gigahertz away
    /// from every other lane. It only runs in `Mode::Adsb`, because a receiver
    /// parked on 1090 MHz at 2.4 Msps is not listening to anything else.
    adsb_ddc: Option<Ddc>,
    adsb: Option<AdsbController>,
    adsb_buf: Vec<Complex32>,
    /// Absolute frequency the window is centred on.
    adsb_center_hz: f64,
    /// The stream rate `adsb_ddc` was built to decimate, so a retune can tell a
    /// window that merely moved from one that has to be rebuilt.
    adsb_in_rate: f64,
    /// The operator's persisted preference, kept apart from `state.adsb` for the
    /// same reason `ism_cfg` is.
    adsb_cfg: sdroxide_types::AdsbSettings,
    /// The station's own position, so a surface squitter has something to be
    /// decoded against. Sent to the worker when it changes.
    adsb_home: Option<(f64, f64)>,
    /// The last "cannot run" sentence sent to the panel, so it is sent once
    /// rather than on every block.
    ///
    /// The outer `None` means nothing has been said yet, which is different
    /// from having said "there is nothing wrong". Without the distinction a
    /// panel that connected while the lane was down would sit on "starting the
    /// decoder" forever.
    adsb_idle_sent: Option<Option<String>>,

    /// The VDL Mode 2 lane: a fourth window, on the 136.7-137.0 MHz datalink
    /// channels.
    ///
    /// Its own for the same reason ADS-B's is — it is somewhere else entirely —
    /// but shaped like the ISM decoder's inside, because VDL2 is a plan of seven
    /// channels rather than one frequency. It only runs in `Mode::Vdl2`: the
    /// window is a third of a megahertz wide and nothing else can be listened to
    /// through a receiver parked on it.
    vdl2_ddc: Option<Ddc>,
    vdl2: Option<Vdl2Controller>,
    vdl2_buf: Vec<Complex32>,
    /// Absolute frequency the window is centred on.
    vdl2_center_hz: f64,
    /// The stream rate `vdl2_ddc` was built to decimate, so a retune can tell a
    /// window that merely moved from one that has to be rebuilt.
    vdl2_in_rate: f64,
    /// The operator's persisted preference, kept apart from `state.vdl2` for the
    /// same reason `adsb_cfg` is.
    vdl2_cfg: sdroxide_types::Vdl2Settings,
    /// The last "cannot run" sentence sent to the panel, so it is sent once
    /// rather than on every block. The outer `None` means nothing has been said
    /// yet, which is different from having said "there is nothing wrong".
    vdl2_idle_sent: Option<Option<String>>,

    /// The AIS lane: a fifth window, on the two ship-reporting channels either
    /// side of 162.000 MHz.
    ///
    /// Its own for the same reason ADS-B's and VDL2's are — it is somewhere
    /// else entirely — and shaped like VDL2's inside, because AIS is a plan of
    /// two channels rather than one frequency. It only runs in `Mode::Ais`:
    /// nothing else can be listened to through a receiver parked on 162 MHz.
    ais_ddc: Option<Ddc>,
    ais: Option<AisController>,
    ais_buf: Vec<Complex32>,
    /// Absolute frequency the window is centred on.
    ais_center_hz: f64,
    /// The stream rate `ais_ddc` was built to decimate, so a retune can tell a
    /// window that merely moved from one that has to be rebuilt.
    ais_in_rate: f64,
    /// The operator's persisted preference, kept apart from `state.ais` for the
    /// same reason `adsb_cfg` is.
    ais_cfg: sdroxide_types::AisSettings,
    /// The last "cannot run" sentence sent to the panel, so it is sent once
    /// rather than on every block. The outer `None` means nothing has been said
    /// yet, which is different from having said "there is nothing wrong".
    ais_idle_sent: Option<Option<String>>,
    /// QO-100 beacon decoder: a fixed downconversion onto
    /// [`sdroxide_types::QO100_BEACON_HZ`] plus a worker-thread demodulator,
    /// present only while the decoder is enabled. Simpler than the ISM
    /// window: the target frequency never moves, so retuning it is always
    /// just re-seating the mixer — see `sync_qo100_window`.
    qo100_ddc: Option<Ddc>,
    qo100: Option<Qo100Controller>,
    qo100_buf: Vec<Complex32>,
    /// The stream rate `qo100_ddc` was built to decimate — the same reason
    /// `ism_in_rate` exists.
    qo100_in_rate: f64,
    /// The last QO-100 setting the operator chose, held in step with
    /// `state.qo100` for symmetry with `ism_cfg` and `skim_cfg`. Nothing
    /// reads it back yet: like the ISM decoder, a source swap into audio mode
    /// turns the decoder off and a swap back leaves it off until the operator
    /// turns it on again. Not persisted to disk — there is nothing here worth
    /// surviving a restart.
    qo100_cfg: sdroxide_types::Qo100Settings,
    /// The QO-100 spectral tracker's closed loop: what it has done to
    /// `RadioConfig::converter_offset_hz` since the tracker was switched on.
    /// Only live while `state.qo100.auto_apply` is set.
    qo100_auto: Qo100Auto,
    /// Open capture file for `--record-iq`, and the interleaving scratch it is
    /// written from.
    iq_rec: Option<std::io::BufWriter<std::fs::File>>,
    iq_rec_buf: Vec<u8>,
    /// The operator's own I/Q capture, started from the REC popup — a WAV file
    /// other programs can open, where `iq_rec` above is the command line's raw
    /// stream. Separate because they answer different questions: `--record-iq`
    /// exists to be read back by `--file`, this one to be handed to somebody.
    iq_wav: Option<crate::iq_wav::IqWavWriter>,
    /// The operator's persisted scanner settings. What the scanner is *doing*
    /// lives in `state.scan`, which every client already receives.
    scan_cfg: sdroxide_types::ScannerConfig,
    /// The running scan, or `None` when it is stopped.
    scan: Option<Scan>,
    /// Scratch for the sweep's spectrum read, kept so a scan allocates nothing
    /// per slice.
    scan_db: Vec<f32>,
    /// Smoothed audio power on a demod-audio (CAT) source, which has no
    /// `RxChain` and so no `power_dbfs`. Tracked with the same time constant as
    /// the real one so a scan behaves the same way on either.
    audio_level: f32,
    /// Built-in Hamlib rigctld server: the control-only surface every
    /// "NET rigctl" client speaks (WSJT-X, fldigi, N1MM, Log4OM, GPredict).
    /// Present while enabled and successfully bound.
    rigctld: Option<RigctldController>,
    rigctld_cfg: RigctldConfig,
    /// Last bind failure, kept so the settings dialog can show why it is not
    /// running — usually a real `rigctld` already holding the port.
    rigctld_err: Option<String>,
    /// Digest of the last state published to rigctld clients. Comparing scalars
    /// keeps the per-tick check allocation-free; the full snapshot is only
    /// built when something actually moved.
    rigctld_seen: Option<RigDigest>,
    /// Most recent S-meter reading, so rigctld's `STRENGTH` level has a value
    /// to report between meter updates.
    last_s_dbm: f32,
    /// Dial the RDS decoder was last reading, so a retune can be told from the
    /// endless small offset changes this engine makes for its own reasons
    /// (centre moves, satellite Doppler). See [`RDS_RETUNE_HZ`].
    rds_dial_hz: f64,
    /// The same, for the DRM decoder — see [`DRM_RETUNE_HZ`].
    drm_dial_hz: f64,
    /// WSJT-X UDP broadcast: decodes, status and logged QSOs sent out for
    /// GridTracker, JTAlert, N1MM+ and Log4OM. Present while enabled.
    wsjtx: Option<sdroxide_wsjtx::WsjtxUdp>,
    wsjtx_cfg: sdroxide_types::WsjtxConfig,
    /// When the last WSJT-X heartbeat went out (clients time a station out
    /// without one).
    wsjtx_beat: Instant,
    /// The band the last tick saw, so a crossing into another one can be told
    /// from the tuning about inside a band that goes on all session. See
    /// [`Engine::poll_band_change`].
    band_seen: Band,
    /// Built-in TCI server: third-party clients (WSJT-X, JTDX, skimmers)
    /// driving this radio. Present while enabled and successfully bound.
    tci_srv: Option<TciServerController>,
    tci_cfg: TciServerConfig,
    /// Last bind failure, kept so the settings dialog can show why the server
    /// isn't running.
    tci_srv_err: Option<String>,
    /// Dedicated wideband decimation feeding the TCI IQ stream, at the rate the
    /// clients asked for. `None` while nobody is subscribed.
    tci_iq_ddc: Option<Ddc>,
    tci_iq_buf: Vec<Complex32>,
    /// Scratch for the interleaved I,Q the server takes.
    tci_iq_ilv: Vec<f32>,
    /// Resamples the clean audio tap to the 48 kHz TCI clients expect. The tap
    /// runs at the demod's rate, which is a device-rate divisor near 48 kHz for
    /// most modes and 64 kHz for WFM, so this is rebuilt when the mode changes.
    tci_aud_rs: Option<MonoResampler>,
    tci_aud_in_rate: f64,
    tci_aud_buf: Vec<f32>,
    /// Digital voice (RADE) synthesises its own receive audio at 48 kHz; these
    /// carry it to the speaker rate in place of the demodulated signal.
    voice_rs: Option<MonoResampler>,
    voice_rs_out_rate: f64,
    voice_buf: Vec<f32>,
    voice_play: Vec<f32>,
    /// The main chain's audio for this block, copied out so the borrow of the
    /// chain ends before a digital-voice mode gets the chance to replace it.
    main_play: Vec<f32>,
    /// Right channel of the main chain, non-empty only while WFM stereo is
    /// decoding and the sub receiver is off — or while the binaural widener
    /// below is placing the passband across the two ears.
    main_play_r: Vec<f32>,
    /// Binaural (pseudo-stereo) audio: built on the first block that asks for it
    /// and dropped when the operator switches it off, so a receiver that never
    /// uses it carries no filter state. Its left ear lands in `bin_left`, which
    /// is then swapped into `main_play` — see [`binaural_split`].
    binaural: Option<Binaural>,
    bin_left: Vec<f32>,
    /// Speaker-path attenuation asked for by a local spoken announcement.
    /// Held here as well as in the mixer so a device swap, which builds a new
    /// mixer, does not come back at full volume mid-announcement.
    speech_duck: f32,
    /// The recorder's copy of this block — see [`RxChain::take_rec_audio`].
    /// Independent of `main_play`/`main_play_r` once AF volume/mute (and the
    /// voice-keyer/digital-voice overrides) are applied to those.
    main_play_rec: Vec<f32>,
    main_play_r_rec: Vec<f32>,
    /// True while the current over is fed by a TCI client's audio stream.
    tci_tx: bool,
    /// Consecutive short TX blocks this over, for the dead-client unkey.
    tci_tx_starved: u32,
    /// How the keying TCI client is being asked for transmit audio — see
    /// [`TciTxPace`].
    tci_pace: TciTxPace,
    /// The client has unkeyed and what it already handed over is being played
    /// out — see [`Engine::tci_key`]. Nothing more is read from it.
    tci_tx_drain: bool,
    /// What we last published to TCI clients, so unchanged ticks cost nothing.
    tci_last_snap: Option<TciStateSnapshot>,
    /// Demod-audio (CAT-rig) mode: the source delivers already-demodulated real
    /// audio, so the DDC/demod/skimmer path is bypassed for a narrow
    /// audio-band panadapter mapped to RF.
    audio_mode: bool,
    /// Sound-card sample rate feeding `analyzer` in audio mode.
    radio_fs: f64,
    /// Front-end decimation, when the operator has asked for any: the raw IQ
    /// goes through this before the analyzer, the receivers, the skimmer and
    /// the TCI IQ stream, all of which run at the reduced rate
    /// ([`RadioState::sample_rate`]) rather than at [`Self::radio_fs`].
    ///
    /// `None` is decimation off, and is the case worth keeping free: it is a
    /// borrow rather than a filter chain of one pass-through stage.
    decim: Option<Decimator>,
    /// Displayed RF window width in audio mode (Hz).
    audio_bw: f64,
    /// Scratch real-audio buffers for audio mode.
    audio_re: Vec<f32>,
    audio_play: Vec<f32>,
    /// The recorder's copy of `audio_play`, pre-volume/mute — see `main_play_rec`.
    audio_play_rec: Vec<f32>,
    /// Resamples audio that arrived as audio to the speaker rate — a demod-audio
    /// rig, or the transceiver behind an attached panadapter receiver.
    audio_resampler: Option<MonoResampler>,
    /// The rate pair `audio_resampler` was built for, so `play_rx_audio` can
    /// tell when either end has moved.
    audio_rs_in_rate: f64,
    audio_rs_out_rate: f64,
    /// The rate [`IqSource::rx_audio`] last reported, for the arrangement where
    /// receive audio comes from a transceiver while an attached receiver
    /// supplies the I/Q ([`DeviceCaps::rx_audio_external`]). Remembered because
    /// a block in which nothing arrived says nothing about the rate, and the
    /// digital-mode tap is built from it.
    ext_audio_rate: f64,
    /// Rebuilds the IQ source when the operator switches radio interface at
    /// runtime (see [`EngineSwap::ReopenSource`]). Shared with the background
    /// reconnect thread, which uses the same factory (never both at once — the
    /// lock serialises them).
    reopen: Option<Arc<Mutex<ReopenFn>>>,
    /// Result channel of a background reconnect attempt in flight (see
    /// [`IqSource::needs_reopen`]), and its thread — joined before the engine
    /// goes away so an open in progress can't outlive the process and race a
    /// device library's own exit handlers.
    retry: Option<Receiver<Result<(Box<dyn IqSource>, DeviceCaps), String>>>,
    retry_join: Option<std::thread::JoinHandle<()>>,
    /// Earliest time the next background attempt may start, and the current
    /// spacing (doubles on each failure up to [`RETRY_MAX`]).
    retry_at: Option<Instant>,
    retry_every: Duration,
    /// The active VFO as of the last tune the front end accepted. A tune it
    /// refuses puts the dial back here rather than leaving the operator on a
    /// frequency the radio cannot receive (see [`Engine::tune_refused`]).
    good_vfo_hz: f64,
    /// What each VFO was left listening in, indexed by [`Vfo::index`]: the mode
    /// and the passband that went with it.
    ///
    /// A VFO is a whole listening position, not just a number — CW on A while B
    /// sits on an SSB net is the reason the radio has two of them (issue #286),
    /// and every transceiver with an A/B pair remembers the mode with the
    /// frequency. Kept here rather than in [`RadioState`] because only the
    /// *active* one is ever in force: the receiver has one mode, the state
    /// carries it, and this is the shelf the other one waits on.
    vfo_memory: [VfoMemory; 2],
    /// Network cockpit: owns the spot feeds (DX cluster / POTA / SOTA / PSK)
    /// and the lookup/upload worker threads. The engine only drains it.
    spots: sdroxide_net::SpotManager,
    /// Winlink radio email: owns the mailbox and the forwarding-session worker.
    /// `None` when the mailbox directory could not be opened, which must not
    /// stop the radio from running — the operator simply gets an error the
    /// first time they open the window.
    winlink: Option<sdroxide_winlink::WinlinkManager>,
    /// The KISS TNC server, when the operator has one running.
    kiss: Option<sdroxide_kiss::KissServer>,
    /// The session end of the packet link, while a packet mode is running.
    packet_port: Option<sdroxide_ax25::PortHandle>,
    /// The network-cockpit config as last persisted. The spot manager has its
    /// own copy but does not hand it back, and a remote settings dialog has to
    /// be told what this station is set to — see [`Engine::emit_station_config`].
    net_cfg: sdroxide_types::NetworkConfig,
    /// The operator's satellite additions. The engine does not track anything
    /// itself; it persists this and announces it, because a browser client has
    /// no config directory of its own and the subscribed listings are fetched
    /// (and cached) on this machine.
    sat_cfg: sdroxide_types::SatConfig,
    /// A running [`Command::RefreshTleSubs`], joined when it finishes so the
    /// fetched status reaches the clients that asked for it.
    tle_refresh: Option<std::thread::JoinHandle<Vec<sdroxide_types::TleSubStatus>>>,
    /// The satellite lock, when one is active — see [`Engine::poll_sat_track`]
    /// for the tracking loop and `start_sat_lock` for how one begins.
    sat_lock: Option<ActiveSatLock>,
    /// The rotctld client's config as persisted (`rotator.json`), announced in
    /// the station bundle the way the servers' are.
    rot_cfg: sdroxide_types::RotatorConfig,
    /// The rotctld client itself, when enabled. `poll_sat_track` feeds it;
    /// `poll_rotator_status` reads its health back for the clients.
    rotator: Option<sdroxide_rotator::RotctldClient>,
    /// What was last told to the clients, and when az/el may next go out —
    /// connection transitions bypass the throttle, movement does not.
    rot_last_status: Option<sdroxide_rotator::RotStatus>,
    next_rot_emit: Instant,
    /// The external T/R switch's config as persisted (`relay.json`), announced
    /// in the station bundle the way the rotator's is.
    relay_cfg: sdroxide_types::RelayConfig,
    /// The switch itself, shared with every other engine. The driver behind it
    /// is built by the primary engine only — one station, one antenna line —
    /// but every engine publishes into it, because any of them keying is the
    /// station being on the air.
    tr_switch: Option<Arc<crate::TrSwitch>>,
    /// What was last told to the clients, so a status only goes out when it
    /// changed.
    relay_last_status: Option<sdroxide_types::RelayStatus>,
    /// A configuration that arrived while the station was on the air, waiting
    /// for the over to end. See [`Engine::sync_relay`].
    relay_pending: bool,
    /// Whether the lead cap has already been complained about this session.
    relay_lead_capped: bool,
    /// What was last written to `session.json`, so the periodic check only
    /// touches the disk when the operator has actually moved. `None` when this
    /// engine does not remember its session (see
    /// [`EngineConfig::remember_session`]).
    session: Option<sdroxide_config::Session>,
    /// The antenna ports the operator wants, RX and TX: the command line's
    /// choice, else the remembered session's, else whatever they last picked in
    /// the UI. Re-applied whenever a front end is (re)opened, because a
    /// reconnect or an interface switch arrives on the driver's default port and
    /// nothing else would put it back.
    ///
    /// `None` means "no preference" — a device with a single port, or a start
    /// that never expressed one. A name the current device does not offer is
    /// held rather than dropped: swapping back to the radio it belongs to
    /// restores it.
    want_antenna: (Option<String>, Option<String>),
    /// The socket the operator last chose on each band, `(RX, TX)`, restored
    /// as the dial crosses into that band — see
    /// [`sdroxide_config::Session::band_antenna`] and
    /// [`Engine::follow_band_antenna`].
    band_antenna: sdroxide_config::BandAntennas,
    /// The front-end gain stages the operator has set, RX then TX, as
    /// `(element, dB)` — held for the same reason [`Self::want_antenna`] is,
    /// and applied by [`Engine::restore_gains`] after every open.
    ///
    /// Only elements that have actually been set are listed: an untouched
    /// device stays on its driver's own gains. Names the current device does
    /// not offer are held rather than dropped, so swapping back to the front
    /// end they belong to brings them back.
    want_gains: (GainSet, GainSet),
    /// How far the operator wants the raw IQ decimated, held for the same
    /// reason [`Self::want_gains`] is and re-asked of every front end that is
    /// opened.
    ///
    /// Separate from [`RadioState::decimation`], which is what the *current*
    /// device can actually carry. The two differ exactly when the front end has
    /// no span to throw away — a rig on a sound card, and the 48 kHz stand-in a
    /// radio switched off runs on — and keeping only the effective figure is
    /// what used to make switching a radio off and on again forget the
    /// decimation outright, session file included (issue #209).
    want_decimation: u32,
    /// Where this engine's radio-scoped files live. See [`EngineConfig::store`].
    store: sdroxide_config::Store,
    /// This engine's radio id. See [`EngineConfig::instance`].
    instance: u32,
    /// Whether this engine runs the station-wide network services.
    /// See [`EngineConfig::primary`].
    primary: bool,
    /// The station's transmit interlock. See [`EngineConfig::tx_gate`].
    tx_gate: Option<Arc<crate::TxGate>>,
    /// When CW the radio is keying itself finishes, and with it this engine's
    /// claim on [`Self::tx_gate`]. That route has no key-up and no key-down of
    /// its own — the message is handed over and the rig transmits it — so the
    /// interlock is held against the clock instead.
    cw_gate_until: Option<Instant>,
    /// Shared-store change signal, and the generation this engine has already
    /// caught up with. See [`EngineConfig::store_sync`].
    store_sync: Option<Arc<crate::StoreSync>>,
    shared_gen_seen: u64,
    /// Which of the station's radios is on FreeDV. See
    /// [`EngineConfig::rade_watch`].
    rade_watch: Option<Arc<crate::RadeWatch>>,
}

/// Target width of the skimmers' window (Hz); the Ddc snaps to the nearest
/// integer decimation of the device rate.
///
/// Not the width of the front end, and deliberately so. Every stage below is
/// sized from this rate — the detector's bins are `rate / 4096`, and DeepCW's
/// front end builds a transform proportional to it — so a window as wide as an
/// RX-888's span would put 8 kHz in a bin and cost a transform to match. 192 kHz
/// resolves CW to about 47 Hz and covers a whole HF band; where the operator is
/// looking at more than that, [`skim_center_for`] decides which part of it gets
/// skimmed.
const SKIM_TARGET_HZ: f64 = 192_000.0;

/// How much of a front end's stream the ADS-B window may claim.
///
/// The outer edges of any receiver's span are where its own anti-alias filter
/// is already rolling off, and a decoder that slices half-microsecond chips has
/// no margin to spend on a signal that arrives tilted. Three quarters is the
/// same figure the ISM plan uses, for the same reason.
///
/// On the commonest receiver for this the fraction never binds: an RTL-SDR at
/// 2.4 Msps hands over a stream the window is exactly as wide as, the
/// downconverter decimates by one, and nothing is trimmed.
const ADSB_USABLE_FRACTION: f64 = 0.75;

/// Where the skim window belongs, in absolute Hz.
///
/// The window is a decimation of the front end's stream, not a second receiver:
/// it is one slice of what is arriving, and something has to choose which. It
/// used to be pinned to the hardware centre, which is right on a front end whose
/// span is a band and wrong on one whose span is all of HF — an RX-888 handed
/// 32 Msps centres on 16.2 MHz, so the skimmers sat in the middle of nowhere
/// while the operator watched 20 m, found nothing to decode and looked broken.
///
/// So it follows the waterfall instead:
///
/// * A view that fits inside the window is centred in it, and the slack either
///   side is what a pan travels through before anything has to move.
/// * A view wider than the window keeps the dial in it — of a band-wide screen,
///   the part the operator cares about is where they are listening — clamped so
///   the window stays inside the view rather than hanging off the edge of it.
/// * With no view at all (a headless server, nobody watching) the window stays
///   where it is, and a fresh one starts on the hardware centre.
///
/// `current` is where the window is now, and gets the benefit of the doubt:
/// moving it costs every track in it and every callsign half-read, so one that
/// still covers what is on screen stays put. Hence a rule and not just an
/// arithmetic centre — a drag that re-cut the window every frame would decode
/// nothing at all.
fn skim_center_for(
    view: Option<(f64, f64)>,
    dial_hz: f64,
    dev_center_hz: f64,
    dev_span_hz: f64,
    win_hz: f64,
    current: Option<f64>,
) -> f64 {
    let half = win_hz / 2.0;
    let (dev_lo, dev_hi) = (dev_center_hz - dev_span_hz / 2.0, dev_center_hz + dev_span_hz / 2.0);
    // Nothing to choose: the window is everything the front end delivers.
    if !(win_hz.is_finite() && dev_span_hz.is_finite()) || win_hz >= dev_span_hz {
        return dev_center_hz;
    }
    let Some((lo, hi)) = view else { return current.unwrap_or(dev_center_hz) };
    // What of the view the front end actually reaches. A client whose window
    // runs past the end of the span is asking for spectrum nobody has.
    let (lo, hi) = (lo.max(dev_lo), hi.min(dev_hi));
    if hi <= lo {
        return current.unwrap_or(dev_center_hz);
    }
    let want = if hi - lo <= win_hz || !(lo..=hi).contains(&dial_hz) {
        (lo + hi) / 2.0
    } else {
        dial_hz.clamp(lo + half, hi - half)
    };
    // The window is an NCO offset inside the stream, so both its edges have to
    // stay inside what was sampled.
    let want = want.clamp(dev_lo + half, dev_hi - half);
    let Some(cur) = current else { return want };
    // How much of the screen a window centred there would reach.
    let covered = |c: f64| (hi.min(c + half) - lo.max(c - half)).max(0.0);
    let holds = cur - half >= dev_lo - 1.0
        && cur + half <= dev_hi + 1.0
        && covered(cur) >= covered(want) - 1.0
        // On a screen wider than the window, covering it is all a placement can
        // do and every placement does it — so the dial decides, with a dead band
        // wide enough that ordinary tuning does not keep re-cutting the window.
        && (hi - lo <= win_hz
            || !(lo..=hi).contains(&dial_hz)
            || (dial_hz - cur).abs() <= win_hz / 4.0);
    if holds { cur } else { want }
}

/// The QO-100 beacon decoder's down-converter output rate for a search
/// half-width of `half_width_hz`: about 2.5× the search so the requested span
/// fits under Nyquist with margin — see `sdroxide_qo100::bpsk` for why the
/// decoder wants that oversampling — and floored at 16 kHz so the narrow
/// widths (±5 kHz and near it) still land on a sane, cheap rate. Widening
/// the search really does widen the capture the decoder is handed; the
/// demodulator inside it always runs at a fixed rate regardless
/// (`sdroxide_qo100`'s `DEMOD_RATE_HZ`), which is what keeps the search cost
/// from growing with the square of the width.
fn qo100_capture_rate_for(half_width_hz: f64) -> f64 {
    (half_width_hz * 2.5).max(16_000.0)
}

/// The capture rate the QO-100 worker needs for `cfg`.
///
/// The frame decoder wants [`qo100_capture_rate_for`]'s ~2.5× oversampling of
/// its search width (see `sdroxide_qo100::bpsk`). The spectral tracker is much
/// cheaper to feed: it only needs the parking window plus a lobe's worth of
/// margin under Nyquist, so when it is running *without* the decoder the rate
/// is just `park_hi + margin` — a third of what 2.5×25 kHz would ask for, and
/// the difference is real load on a marginal receiver.
fn qo100_rate_for_cfg(cfg: &sdroxide_types::Qo100Settings) -> f64 {
    let park = if cfg.enabled { cfg.park_hi_hz.max(cfg.park_lo_hz).max(0.0) } else { 0.0 };
    let decoder = if cfg.decode_telemetry {
        qo100_capture_rate_for(cfg.search_half_width_hz.max(park))
    } else {
        0.0
    };
    // Tracker-only: enough to hold the parked beacon's upper lobe (~800 Hz
    // above the carrier) under Nyquist with room, and no more — a smaller
    // parking window is a lighter receiver.
    let tracker = if cfg.enabled { (park + 2_000.0) * 2.2 } else { 0.0 };
    decoder.max(tracker).max(16_000.0)
}

/// The QO-100 spectral tracker's closed loop, engine side. The worker reports
/// where the beacon sits; this decides — slowly, and only on a clean, steady
/// estimate — whether to nudge `RadioConfig::converter_offset_hz` so the
/// beacon (and the receiver behind it) lands back on 10489.750 MHz.
#[derive(Default)]
struct Qo100Auto {
    /// When the last correction was written, for the rate limit.
    last_apply: Option<Instant>,
    /// Everything the decision below carries from one cycle to the next.
    run: Qo100AutoRun,
    /// How many corrections have been written since the tracker came on, and
    /// the last one, with the unix second it happened — shown on the page.
    applies: u64,
    last_hz: f64,
    last_unix: i64,
}

/// What one cycle of the closed loop hands to the next. Kept apart from
/// [`Qo100Auto`]'s clocks and counters so the decision itself is a pure
/// function of it, testable without an engine.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Qo100AutoRun {
    /// The most recent clean estimate, and how many clean estimates in a row
    /// (this one included) have agreed with it — a correction needs
    /// [`QO100_AUTO_AGREE_RUN`] before it will move a running receiver, so one
    /// or two stray readings never reopen the front end.
    agree_ref_hz: Option<f64>,
    agree_run: u8,
    /// A correction that has been written and not yet been shown to have
    /// worked, and how many in a row have failed that test.
    ///
    /// This is the loop checking its own work. A correction is a claim about
    /// the world — write this many Hz and the beacon lands on 10489.750 MHz —
    /// so the first estimate to arrive once the front end has settled is the
    /// answer to it: the beacon should be back near the target, not still out
    /// where it was. It will not be whenever the number written is not the
    /// number the receiver reads — a `Transverter` row covering the beacon
    /// takes precedence over the single converter offset, a front end that
    /// will not reopen never picks the new value up. Without this the loop
    /// writes the same correction every [`QO100_AUTO_MIN_INTERVAL`] for as
    /// long as it is left armed, reopening the receiver each time and walking
    /// the offset further from the truth on every pass.
    ///
    /// Deliberately *not* a comparison of one correction against the last. A
    /// steadily warming LNB earns a fresh correction of much the same size
    /// every few minutes, and that is the loop working perfectly — what
    /// separates it from a runaway is not the size of the corrections but
    /// whether the beacon goes back to the target in between.
    pending_hz: Option<f64>,
    stalled: u8,
    /// Signed Hz written into the converter offset since the tracker came on.
    total_hz: f64,
}

/// The tracker's estimate must clear all of these before its correction is
/// written: a convincing twin-lobe shape, not a marginal one.
const QO100_AUTO_NULL_DB: f32 = 5.0;
const QO100_AUTO_SYM: f32 = 0.7;
const QO100_AUTO_SNR_DB: f32 = 4.0;
/// Consecutive clean estimates must agree within this to count toward the run.
const QO100_AUTO_AGREE_HZ: f64 = 250.0;
/// How many agreeing clean estimates in a row it takes before a correction
/// goes out — a drifting LNB gives a long steady run; a noisy estimate that
/// jumps around never builds one.
const QO100_AUTO_AGREE_RUN: u8 = 3;
/// Corrections smaller than this are left alone: below it the reopen is not
/// worth the interruption, and the residual is already close enough that the
/// receiver behind the beacon is calibrated.
const QO100_AUTO_DEADBAND_HZ: f64 = 200.0;
/// The soonest a second correction can follow the first — the LNB drifts over
/// minutes, not seconds, so this only ever bites on a bad run, and it keeps
/// the front-end reopen each correction costs to at most one every half a
/// minute.
const QO100_AUTO_MIN_INTERVAL: Duration = Duration::from_secs(30);
/// A correction counts as having taken effect if the error left behind it is
/// smaller than this share of it. Half is loose on purpose: the question being
/// asked is only "did the beacon go back toward the target at all", not "how
/// precisely" — the deadband above is what judges that.
const QO100_AUTO_SHRINK: f64 = 0.5;
/// How long after a correction the loop waits before believing what it reads.
/// A correction reopens the front end, and the tracker's window has to refill
/// through that before its estimate says anything about the new offset rather
/// than the old one.
const QO100_AUTO_SETTLE: Duration = Duration::from_secs(6);
/// How many corrections in a row may fail to shrink before the loop gives up
/// and disarms. Two, because the first could be a genuine second drift step
/// arriving while the first correction was still settling; a third says the
/// write is not reaching the receiver.
const QO100_AUTO_MAX_STALLED: u8 = 2;
/// A ceiling on what the loop may move the offset by in one session, as a
/// backstop under the convergence check for any way of not converging that
/// still shrinks. Far more than an uncalibrated LNB needs (tens of kHz) plus a
/// day of thermal drift (a few kHz), and small enough that a runaway is caught
/// while the station is still recognisable.
const QO100_AUTO_TOTAL_MAX_HZ: f64 = 150_000.0;

/// Why the closed loop took itself off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Qo100AutoStop {
    /// Corrections are going out and the beacon is not moving.
    NotConverging,
    /// The running total left what any real LNB could be out by.
    Runaway,
}

impl Qo100AutoStop {
    /// What the operator is told on the notice line.
    fn notice(self) -> &'static str {
        match self {
            Qo100AutoStop::NotConverging => {
                "QO-100 auto-correct switched off: the offset is being written but the beacon is                  not moving. If this station has a transverter row covering 10489.750 MHz, check                  that it is the one being corrected; otherwise check that the receiver reopens."
            }
            Qo100AutoStop::Runaway => {
                "QO-100 auto-correct switched off: it has moved the converter offset further than                  any LNB should need. The offset it has written is unlikely to be right — check                  Settings ▸ Radio ▸ Converter."
            }
        }
    }
}

/// What one pass of the tracker's closed loop decided.
struct Qo100AutoOutcome {
    /// Hz to add to the converter offset now — `None` means "not this cycle".
    correction: Option<f64>,
    /// The state to carry forward.
    run: Qo100AutoRun,
    /// Set when the loop has decided to take itself off the air.
    stop: Option<Qo100AutoStop>,
}

/// The closed loop's decision, pure so it can be tested without an engine: a
/// correction goes out only after [`QO100_AUTO_AGREE_RUN`] *clean* estimates
/// (a convincing twin-lobe shape) in a row that all agree, that clears the
/// deadband, and only if the last correction has had time to settle.
/// `since_last` is `None` before the loop has ever acted.
///
/// It also watches its own effect. A loop that writes a correction the
/// receiver never sees would otherwise write it again, and again, for as long
/// as the operator left it armed — see [`Qo100AutoRun::stalled`]. When that
/// happens, or when the running total leaves what an LNB could plausibly be
/// out by, the outcome carries a [`Qo100AutoStop`] and the caller takes the
/// loop off rather than going round again.
fn qo100_auto_correction(
    est_offset_hz: Option<f64>,
    null_db: f32,
    sym: f32,
    snr_db: f32,
    prev: Qo100AutoRun,
    since_last: Option<Duration>,
) -> Qo100AutoOutcome {
    // A cycle with no usable estimate keeps everything the loop knows about
    // its own effect; only the agreement run moves.
    let idle = |n: u8| Qo100AutoOutcome {
        correction: None,
        run: Qo100AutoRun { agree_ref_hz: None, agree_run: n, ..prev },
        stop: None,
    };

    let Some(off) = est_offset_hz else { return idle(0) };
    let clean =
        null_db >= QO100_AUTO_NULL_DB && sym >= QO100_AUTO_SYM && snr_db >= QO100_AUTO_SNR_DB;
    if !clean {
        return idle(0); // a bad cycle breaks the run
    }

    // Did the last correction do what it said it would? This estimate is the
    // answer, once the front end has had QO100_AUTO_SETTLE to reopen and the
    // tracker's window to refill through it.
    let mut pending = prev.pending_hz;
    let mut stalled = prev.stalled;
    if let Some(c) = pending
        && since_last.is_none_or(|d| d >= QO100_AUTO_SETTLE)
    {
        pending = None;
        if off.abs() < c.abs() * QO100_AUTO_SHRINK {
            stalled = 0;
        } else {
            stalled = stalled.saturating_add(1);
            if stalled > QO100_AUTO_MAX_STALLED {
                return Qo100AutoOutcome {
                    correction: None,
                    run: Qo100AutoRun {
                        agree_ref_hz: None,
                        agree_run: 0,
                        pending_hz: None,
                        stalled,
                        ..prev
                    },
                    stop: Some(Qo100AutoStop::NotConverging),
                };
            }
        }
    }

    // From here the effect check has run, so holding has to carry it too.
    let hold = |r: Option<f64>, n: u8| Qo100AutoOutcome {
        correction: None,
        run: Qo100AutoRun { agree_ref_hz: r, agree_run: n, pending_hz: pending, stalled, ..prev },
        stop: None,
    };

    // Extend the run if this estimate agrees with the reference (and has the
    // same sign — a drift keeps one sign, an oscillation flips), else restart.
    let run = match prev.agree_ref_hz {
        Some(r) if (r - off).abs() <= QO100_AUTO_AGREE_HZ && r.signum() == off.signum() => {
            prev.agree_run.saturating_add(1)
        }
        _ => 1,
    };
    if run < QO100_AUTO_AGREE_RUN {
        return hold(Some(off), run);
    }

    let cooled = since_last.is_none_or(|d| d >= QO100_AUTO_MIN_INTERVAL);
    if !cooled || off.abs() < QO100_AUTO_DEADBAND_HZ {
        return hold(Some(off), run);
    }

    // A correction is due, unless the running total says the loop has already
    // moved the offset further than any LNB could have been out by.
    let total = prev.total_hz + off;
    if total.abs() > QO100_AUTO_TOTAL_MAX_HZ {
        return Qo100AutoOutcome {
            correction: None,
            run: Qo100AutoRun {
                agree_ref_hz: None,
                agree_run: 0,
                pending_hz: None,
                stalled,
                ..prev
            },
            stop: Some(Qo100AutoStop::Runaway),
        };
    }

    // Correction written: start a fresh run so the next one needs its own
    // steady stretch of agreeing estimates, and put it up to be checked.
    Qo100AutoOutcome {
        correction: Some(off),
        run: Qo100AutoRun {
            agree_ref_hz: None,
            agree_run: 0,
            pending_hz: Some(off),
            stalled,
            total_hz: total,
        },
        stop: None,
    }
}

/// How soon after noticing a disconnected front-end the first reconnect attempt
/// runs, and the ceiling the spacing doubles up to while attempts keep failing.
const RETRY_FIRST: Duration = Duration::from_secs(1);
const RETRY_MAX: Duration = Duration::from_secs(15);

/// How long [`Engine::abandon_retry`] waits for the answer of an attempt it is
/// throwing away. It is only ever called with the factory lock held, so the
/// attempt has already finished and this covers the instruction between its
/// releasing that lock and sending — never a whole open. A worker that died
/// without answering drops its sender and is noticed at once rather than at
/// the end of this.
const ABANDON_WAIT: Duration = Duration::from_millis(250);

/// How often the dial and mode are compared against what is in `session.json`.
/// Only a change writes anything, and a clean exit flushes as well, so this
/// interval only decides how much tuning a crash or a kill can lose.
const SESSION_SAVE_INTERVAL: Duration = Duration::from_secs(10);

/// Run a block of raw IQ through the front-end decimator, if there is one.
///
/// With decimation off this hands the block straight back rather than copying
/// it into `out`, so the ordinary case pays nothing at all.
fn decimate<'a>(
    decim: Option<&mut Decimator>,
    raw: &'a [Complex32],
    out: &'a mut Vec<Complex32>,
) -> &'a [Complex32] {
    match decim {
        Some(d) => {
            out.clear();
            d.process(raw, out);
            out
        }
        None => raw,
    }
}

/// The front-end decimation actually usable: `want` rounded *down* to a power
/// of two and clamped to what this device rate can carry, or 1 for a source
/// with no IQ to decimate.
///
/// Every route in goes through here — the remembered session, the operator's
/// chip, a remote client's command, and a device swap re-asking the question
/// for the new front end — so none of them can leave the receiver with a span
/// narrower than [`sdroxide_types::MIN_DECIMATED_RATE_HZ`].
fn decimation_for(want: u32, device_rate_hz: f64, audio_mode: bool) -> u32 {
    if audio_mode || want < 2 {
        return 1;
    }
    wanted_decimation(want).min(sdroxide_types::max_decimation(device_rate_hz))
}

/// What the operator asked for, as a figure worth *remembering*: rounded down
/// to a power of two and capped at [`sdroxide_types::MAX_DECIMATION`], but with
/// no device rate involved.
///
/// The counterpart to [`decimation_for`], which answers the other half of the
/// question — what this particular front end can carry. Kept apart because the
/// answer to the second one is worthless as a memory: a radio switched off is
/// on a 48 kHz stand-in that can carry nothing, and letting that overwrite the
/// operator's choice is what issue #209 was.
fn wanted_decimation(want: u32) -> u32 {
    if want < 2 { 1 } else { (1u32 << want.ilog2()).min(sdroxide_types::MAX_DECIMATION) }
}

/// Drop the commands at the end of a drained batch that a later one in the same
/// batch has already overwritten.
///
/// The case this exists for is a panadapter drag. Once the view is the whole
/// captured span there is nothing left to slide, so the gesture moves the
/// *window* instead and the dial with it (issue #133) — which means one
/// [`Command::SetCenter`] and one [`Command::SetVfo`] per frame of the UI
/// drawing it, for as long as the operator's hand is down. Every one of those
/// is a hardware retune, a skimmer restart and a waterfall remap here; an SDR
/// on a Pi carrying a station cannot do sixty of them a second and does not
/// need to, because fifty-nine of them are answers nobody ever sees. Only the
/// last of each is a state anything observes.
///
/// Both are absolute setters, so the last one wins — but only where nothing
/// between them could have *read* what an earlier one set. `SwapVfos` and
/// `CopyAtoB` do exactly that, and `TuneInSpan` reads the centre, so this is
/// deliberately narrow: it collapses the run of setters at the *end* of the
/// batch and stops at the first command that is anything else. Interleaving the
/// two with each other is fine — between them they set only the two things they
/// each overwrite.
fn collapse_superseded(batch: &mut Vec<Command>) {
    let settles = |c: &Command| matches!(c, Command::SetCenter(_) | Command::SetVfo { .. });
    // Everything from here to the end is setters, so nothing in it can observe
    // what an earlier one of them did.
    let start = batch.iter().rposition(|c| !settles(c)).map_or(0, |i| i + 1);
    if batch.len() - start < 2 {
        return;
    }
    let slot = |vfo: &Vfo| usize::from(matches!(vfo, Vfo::B));
    let (mut last_center, mut last_vfo) = (None, [None, None]);
    for (i, cmd) in batch.iter().enumerate().skip(start) {
        match cmd {
            Command::SetCenter(_) => last_center = Some(i),
            Command::SetVfo { vfo, .. } => last_vfo[slot(vfo)] = Some(i),
            _ => {}
        }
    }
    let mut i = 0;
    batch.retain(|cmd| {
        let keep = i < start
            || match cmd {
                Command::SetCenter(_) => last_center == Some(i),
                Command::SetVfo { vfo, .. } => last_vfo[slot(vfo)] == Some(i),
                _ => true,
            };
        i += 1;
        keep
    });
}

fn engine_thread(
    source: Box<dyn IqSource>,
    mut caps: DeviceCaps,
    engine_cfg: EngineConfig,
    cmd_rx: Receiver<Command>,
    swap_rx: Receiver<EngineSwap>,
    event_tx: Sender<RadioEvent>,
    mut spec_in: triple_buffer::Input<SpectrumFrame>,
    mut wide_in: triple_buffer::Input<SpectrumFrame>,
) {
    // Answered by the source rather than assembled with the rest of the
    // capabilities, so every backend reports it without having to remember to:
    // it is the trait's own answer, passed on.
    caps.center_is_dial = source.center_is_dial();
    caps.cw_audio_keyed = source.cw_audio_keyed();
    caps.commands_squelch = source.commands_squelch();
    caps.commands_rig_power = source.commands_rig_power();
    caps.wide_span_hz = source.wide_span_hz();
    let audio_mode = caps.audio_mode;
    let radio_fs = source.sample_rate();
    let audio_bw = source.display_bandwidth().unwrap_or(radio_fs / 2.0);
    let source_center_hz = source.center_hz();

    // Before the first `Band::containing` below: every band edge and
    // sub-segment in the process reads these, and a band worked out under the
    // wrong plan would be wrong in the band stack from the first frame. Set
    // here rather than only in the binary so a headless `--server` engine — and
    // any test that brings one up — is on the station's band plan too. The
    // first call also seeds `bandplan.json` if the operator has none.
    sdroxide_types::set_band_plan(sdroxide_config::load_band_plan());
    sdroxide_types::set_region(sdroxide_config::load_region());
    // The operator's own additions to the digital modes' frequency tables. Read
    // here for the same reason as the plan above: they are process-wide, every
    // radio at the station offers the same ones, and a headless `--server`
    // engine has to have them too — its clients draw the picker from what it
    // announces.
    sdroxide_types::set_digi_presets(sdroxide_config::load_digi_presets());

    let mut state = RadioState::default();
    state.center_hz = source.center_hz();
    state.sample_rate = source.sample_rate();
    state.vfo_a_hz = source.center_hz();
    state.vfo_b_hz = source.center_hz();
    state.band = Band::containing(state.vfo_a_hz);
    state.gains = source.current_gains();
    state.tx_gains = source.current_tx_gains();
    // Read back rather than assumed: the remembered antenna is applied further
    // down, once the engine exists to own the preference, and a front end that
    // refuses it must leave the UI showing the port actually in use.
    state.antenna_rx = source.current_antenna();
    state.antenna_tx = source.current_tx_antenna();
    // Published so every UI attached to this engine — including a remote one
    // started by somebody else — can warn about it.
    state.oob_tx = !engine_cfg.tx_ham_only;
    // Seeded here, next to the other config-derived state, so the very first
    // broadcast carries the real guard settings and no client ever renders the
    // 0.0 that `TxState::default()` would give it.
    state.tx.swr_guard = engine_cfg.swr_guard;
    state.tx.swr_limit = engine_cfg.swr_limit.clamp(SWR_LIMIT_MIN, SWR_LIMIT_MAX);
    if let Some(mode) = engine_cfg.initial_mode {
        for rx in &mut state.rx {
            *rx = RxState::with_mode(mode);
        }
        // The APRS channel rule, which `set_rx_mode` applies when the operator
        // picks the mode and which has to apply here too: the mode can also
        // arrive from the command line or from the restored session, and a
        // station that comes up in APRS on 20 metres is a station receiving
        // nothing and heard by nobody.
        //
        // Left alone if the dial is already on any region's channel — a
        // traveller who tuned Japan's 144.640 keeps it — and said out loud
        // rather than done silently, because it is the one mode selection that
        // moves the dial.
        if mode.is_aprs() && !sdroxide_types::is_aprs_channel(state.active_freq_hz()) {
            let hz = sdroxide_types::aprs_dial();
            info!(
                from = state.active_freq_hz(),
                to = hz,
                region = sdroxide_types::region().number(),
                "APRS is a shared channel; tuning to this region's"
            );
            match state.active_vfo {
                Vfo::A => state.vfo_a_hz = hz,
                Vfo::B => state.vfo_b_hz = hz,
            }
            state.band = Band::containing(hz);
        }
        // The same for ADS-B, which is a channel in the strongest sense there
        // is: one frequency, worldwide, and a receiver anywhere else hears
        // nothing at all. Unconditional here, unlike the in-session rule — the
        // capabilities are not known yet at this point, and a dial that turns
        // out to be unreachable is reported by `sync_adsb` a moment later.
        if mode.is_adsb() {
            let hz = sdroxide_types::ADSB_FREQ_HZ;
            info!(from = state.active_freq_hz(), to = hz, "ADS-B is on 1090 MHz; tuning there");
            match state.active_vfo {
                Vfo::A => state.vfo_a_hz = hz,
                Vfo::B => state.vfo_b_hz = hz,
            }
            state.band = Band::containing(hz);
        }
        // ...and VDL2, which is a plan of seven channels rather than one, so the
        // dial goes to the middle of the group and the lane's own window slides
        // from there to take in as much of it as the front end can reach.
        if mode.is_vdl2() {
            let hz = sdroxide_types::VDL2_PLAN_CENTER_HZ;
            info!(
                from = state.active_freq_hz(),
                to = hz,
                "VDL2 lives around 136.8 MHz; tuning there"
            );
            match state.active_vfo {
                Vfo::A => state.vfo_a_hz = hz,
                Vfo::B => state.vfo_b_hz = hz,
            }
            state.band = Band::containing(hz);
        }
        // ...and AIS, which is two channels 50 kHz apart, so the dial goes
        // between them and the lane's own window takes in both.
        if mode.is_ais() {
            let hz = sdroxide_types::AIS_PLAN_CENTER_HZ;
            info!(
                from = state.active_freq_hz(),
                to = hz,
                "AIS is on the two channels either side of 162.000 MHz; tuning there"
            );
            match state.active_vfo {
                Vfo::A => state.vfo_a_hz = hz,
                Vfo::B => state.vfo_b_hz = hz,
            }
            state.band = Band::containing(hz);
        }
    }
    let skim_cfg = sdroxide_config::load_skimmer_config();
    state.skimmer = if audio_mode {
        sdroxide_types::SkimmerSettings::OFF // wideband-only feature
    } else {
        skim_cfg
    };
    // Opened before the stream starts, so a bad path is a startup error rather
    // than a surprise several minutes into a capture.
    let iq_rec = match engine_cfg.record_iq.as_ref() {
        Some(path) => match std::fs::File::create(path) {
            Ok(f) => {
                info!(path = %path.display(), "recording raw IQ");
                Some(std::io::BufWriter::with_capacity(1 << 20, f))
            }
            Err(e) => {
                warn!(path = %path.display(), "cannot open the IQ capture file: {e}");
                None
            }
        },
        None => None,
    };
    let ism_cfg = sdroxide_config::load_ism_config();
    state.ism = if audio_mode {
        sdroxide_types::IsmSettings::OFF // wideband-only, like the skimmers
    } else {
        ism_cfg
    };
    let vdl2_cfg = sdroxide_config::load_vdl2_config();
    state.vdl2 = if audio_mode {
        sdroxide_types::Vdl2Settings::OFF // wideband-only, like the rest of the lanes
    } else {
        vdl2_cfg
    };
    let adsb_cfg = sdroxide_config::load_adsb_config();
    state.adsb = if audio_mode {
        sdroxide_types::AdsbSettings::OFF // wideband-only, and by far the widest
    } else {
        adsb_cfg
    };
    let ais_cfg = sdroxide_config::load_ais_config();
    state.ais = if audio_mode {
        sdroxide_types::AisSettings::OFF // wideband-only, like the rest of the lanes
    } else {
        ais_cfg
    };

    // Read before the DSP below rather than with the rest of the session
    // further down: the remembered decimation decides what rate the analyzer
    // and the receiver chain are built at, and building them at the device rate
    // first would mean tearing them down again before the first block.
    let session = engine_cfg.remember_session.then(|| engine_cfg.store.load_session());
    // Held separately from what this front end can carry: a start on a stand-in
    // (a radio switched off, a rig that isn't there yet) must not be the thing
    // that forgets it — see `Engine::want_decimation`.
    let want_decimation = wanted_decimation(session.as_ref().map_or(1, |s| s.decimation));
    state.decimation = decimation_for(want_decimation, radio_fs, audio_mode);
    let decim = (state.decimation > 1).then(|| Decimator::new(state.decimation));
    state.sample_rate = radio_fs / state.decimation as f64;

    let cfg = SpectrumConfig::default();
    // In audio mode the analyzer FFTs the real audio at the card rate;
    // otherwise it sees whatever the decimation left, which is what every span
    // downstream is measured in.
    let analyzer_rate = if audio_mode { radio_fs } else { state.sample_rate };
    let analyzer = build_analyzer(
        cfg.fft_size as usize,
        analyzer_rate,
        cfg.avg_tc,
        f64::from(cfg.rows()),
        audio_mode.then_some(audio_bw),
    );

    // In audio mode there is no RxChain (the source is already audio); the
    // speaker path is a plain resampler → mixer instead.
    let (main, mixer, audio_out_rate, audio_resampler) = match engine_cfg.audio {
        Some(audio) if audio_mode => {
            let rs = MonoResampler::new(radio_fs, audio.out_rate);
            (None, Some(StereoMixer::new(audio.producer)), audio.out_rate, rs)
        }
        Some(audio) => {
            let chain = RxChain::new(state.sample_rate, &state.rx[0], audio.out_rate);
            info!(channel_rate = chain.ddc.out_rate(), out_rate = audio.out_rate, "audio chain up");
            (Some(chain), Some(StereoMixer::new(audio.producer)), audio.out_rate, None)
        }
        None => (None, None, 48_000.0, None),
    };

    let memories = sdroxide_config::load_memories();
    let mem_folders = sdroxide_config::load_memory_folders();
    let mut scan_cfg = engine_cfg.store.load_scanner_config();
    // A hand-edited `scanner.json` can name a range and a skip list that were
    // never taken in it; the list is the part that goes.
    scan_cfg.forget_stale_skips();
    let stacks = sdroxide_config::load_bandstacks();
    let digi_config = sdroxide_config::load_digi_config();
    // Only the per-band drive calibration is kept out of `radio.json` — the
    // engine deliberately does not hold that file (see
    // [`Engine::emit_radio_config`]), and this one table is consulted on every
    // transmitted block.
    let radio_cfg = engine_cfg.store.load_radio_config();

    info!(source = %source.describe(), "engine started");
    let _ = event_tx.send(RadioEvent::Capabilities(caps.clone()));
    let _ = event_tx.send(RadioEvent::Memories(memories.clone()));
    let _ = event_tx.send(RadioEvent::MemoryFolders(mem_folders.clone()));
    let _ = event_tx.send(RadioEvent::Scanner(scan_cfg.clone()));
    // Surface any warning captured while opening the source (e.g. radio audio
    // device unavailable / mono card chosen for IQ) so the UI can show it
    // instead of an unexplained "waiting for spectrum" — together with any
    // configuration file that had to be reset to defaults on load, which
    // would otherwise announce itself only as a radio that forgot its setup.
    let mut notes: Vec<String> = source.open_status().into_iter().collect();
    notes.extend(sdroxide_config::take_load_alerts());
    if !notes.is_empty() {
        let _ = event_tx.send(RadioEvent::Notice(Some(notes.join("\n"))));
    }

    // Seeded with what is already on disk rather than with the state this
    // engine came up in, so the file always ends up describing where the
    // radio actually was: a start that overrode the dial (`--freq`, or a
    // CAT rig reporting its own) is a difference like any other, and gets
    // written even if nothing is touched afterwards.
    // Volume, RX gain, AGC mode, squelch, noise reduction, drive, mic gain and
    // the recording channel layout have no command-line override, so the
    // remembered session (if any) always wins over the hardcoded defaults
    // `RadioState::default()` / `engine_cfg.initial_mode` set above.
    //
    // The audio chain above was built before this and carries the defaults, but
    // it re-reads the receiver's settings per block and rebuilds what changed —
    // squelch is read straight from the state, and the NR engine is configured
    // the first time the level it holds differs from the one it was built with.
    if let Some(s) = session.as_ref() {
        // Both VFOs, and which of the two was in use. The front end was opened
        // on the active one — `apply_session` handed it that dial, unless
        // `--freq` or a rig reporting its own overrode it — so *that* VFO keeps
        // what the source actually came up on, and the other one, which has no
        // centre frequency of its own to be overridden, comes back exactly as
        // it was left. A remembered B outside what this front end can hear is
        // left alone here; `keep_vfo_in_span` judges it if and when the
        // operator switches to it, the same as any other tune.
        state.active_vfo = s.active_vfo;
        match s.active_vfo {
            Vfo::A => state.vfo_b_hz = s.vfo_b_hz.unwrap_or(state.vfo_a_hz),
            Vfo::B => state.vfo_a_hz = s.freq_hz,
        }
        state.band = Band::containing(state.active_freq_hz());
        state.rx[0].volume = s.volume;
        state.rx[0].muted = s.muted;
        state.rx[0].manual_gain_db = s.rx_gain_db;
        state.rx[0].agc = s.agc;
        state.rx[0].squelch_db = s.squelch_db;
        state.rx[0].noise_reduction = s.noise_reduction;
        state.rx[0].binaural = s.binaural;
        state.tx.drive = s.drive;
        state.tx.tune_drive = s.tune_drive;
        state.tx.mic_gain = s.mic_gain;
        state.tx.cessb_db = s.cessb_db.clamp(0.0, sdroxide_types::CESSB_MAX_DB);
        // Clamped on the way in: `session.json` is a file, and an out-of-range
        // corner frequency or Q from a hand edit would reach the filter design.
        state.tx.eq = s.tx_eq.clamped();
        // Clamped for the same reason, and with more at stake: this one decides
        // a transmit frequency.
        state.repeater = s.repeater.clamped();
        state.recording_mono = s.recording_mono;
    }
    // The command line outranks the remembered session, exactly as it does for
    // the dial and the mode.
    let (cli_rx, cli_tx) = engine_cfg.initial_antenna;
    let want_antenna = (
        cli_rx.or_else(|| session.as_ref().and_then(|s| s.antenna_rx.clone())),
        cli_tx.or_else(|| session.as_ref().and_then(|s| s.antenna_tx.clone())),
    );
    // The front-end gains have no command line to lose to; they are whatever
    // the operator last set on this radio, and nothing at all on a first run.
    let want_gains =
        session.as_ref().map(|s| (s.gains.clone(), s.tx_gains.clone())).unwrap_or_default();
    let band_antenna = session.as_ref().map(|s| s.band_antenna.clone()).unwrap_or_default();
    // Taken before the state is moved into the engine. Both VFOs open on the
    // mode the receiver came up in — the command line's, the session's, or the
    // default — and the *inactive* one is then given back the mode it was
    // actually left in, just below.
    let initial_vfo_memory = VfoMemory::of(&state.rx[0]);

    let mut engine = Engine {
        // Out of declaration order on purpose: literal fields are evaluated
        // top to bottom and the `state` shorthand below moves it, so the band
        // has to be copied out of it first.
        band_seen: state.band,
        // Read before `state` is moved below, for the same reason `band_seen`
        // is: the stream starts wherever the front end already was.
        stream_center_hz: state.center_hz,
        source,
        caps,
        state,
        cfg,
        analyzer,
        event_tx,
        main,
        sub: None,
        mixer,
        audio_out_rate,
        recorder: None,
        cal_offset_db: engine_cfg.cal_offset_db,
        stacks,
        memories,
        mem_folders,
        mic: engine_cfg.mic,
        mic_resampler: None,
        digi_rate: 48_000.0,
        digi_tx_rs: None,
        digi_tx_fifo: Vec::new(),
        digi_tx_scratch: Vec::new(),
        digi_tx_done: false,
        mic_fifo: Vec::new(),
        tx_eq: ParametricEq::new(),
        // Deliberately not the restored EQ state: `apply_tx_eq` compares this
        // against `state.tx.eq` and retunes on any difference, so a default
        // here just means the first block of the first over retunes once.
        tx_eq_cfg: TxEqState::default(),
        tx: None,
        tx_active: false,
        voice_peak: 0.0,
        voice_blocks: 0,
        hw_ptt: false,
        rig_tx: false,
        tx_freq_told: None,
        drive_trim_db: 0.0,
        drive_trim: radio_cfg.tx_drive_trim.clone(),
        rx_af_gain_db: radio_cfg.rx_audio_gain_db,
        tx_center_hz: 0.0,
        tx_ham_only: engine_cfg.tx_ham_only,
        swr_guard: engine_cfg.swr_guard,
        swr_limit: engine_cfg.swr_limit.clamp(SWR_LIMIT_MIN, SWR_LIMIT_MAX),
        swr_over: 0,
        swr_settle: 0,
        swr_tuning: false,
        swr_tripped: None,
        wide_bins: Vec::new(),
        row_batch: Vec::new(),
        center_trail: VecDeque::new(),
        samples_read: 0,
        last_block: 0,
        row_axis: None,
        row_samples: 0,
        row_sample_clock: false,
        rows_clocked: 0,
        rows_sent: 0,
        channel_rate_hz: 48_000.0,
        wide_window: None,
        wide_at: Instant::now(),
        wide_sweeps: 0,
        wide_fresh: false,
        wide_seq: 0,
        scope_seq: 0,
        wide_levels: None,
        tx_analyzer: SpectrumAnalyzer::new(cfg.fft_size as usize, TX_MONITOR_RATE, cfg.avg_tc),
        tx_mon_buf: Vec::new(),
        tune_phase: 0.0,
        sub_tone: None,
        burst: None,
        burst_unkeys: false,
        auto_shift_dial: None,
        nb: NoiseBlanker::new(),
        adc: AdcMeter::new(),
        audio_notch: AutoNotch::new(),
        audio_notch_on: false,
        audio_nr: SpectralNr::new(),
        audio_sbnr: SpecBleachNr::new(),
        audio_nnr: NeuralNr::new(),
        audio_dfnr: None,
        audio_dfnr_failed: false,
        audio_nr_level: NrLevel::Off,
        digi: None,
        digi_config,
        digi_dirty: false,
        digi_tx_band: None,
        digi_tx: false,
        hop_suspended: false,
        voice: VoiceKeyer::load(),
        images: crate::image_store::ImagePresetStore::load(),
        gallery: crate::image_store::GalleryWorker::new(),
        voice_tx: false,
        voice_rec_buf: Vec::new(),
        voice_prev_q: Vec::new(),
        voice_prev_rs: None,
        voice_prev_rate: 0.0,
        voice_prev_out: Vec::new(),
        voice_started: None,
        voice_tick: None,
        tx_pace: None,
        channel_analyzer: None,
        zoom: None,
        skim_ddc: None,
        skimmer: None,
        skim_buf: Vec::new(),
        skim_view: None,
        skim_center_hz: 0.0,
        skim_in_rate: 0.0,
        skim_cfg,
        ism_ddc: None,
        ism: None,
        ism_buf: Vec::new(),
        ism_center_hz: 0.0,
        ism_in_rate: 0.0,
        ism_cfg,
        adsb_ddc: None,
        adsb: None,
        adsb_buf: Vec::new(),
        adsb_center_hz: 0.0,
        adsb_in_rate: 0.0,
        adsb_cfg,
        adsb_home: None,
        adsb_idle_sent: None,
        vdl2_ddc: None,
        vdl2: None,
        vdl2_buf: Vec::new(),
        vdl2_center_hz: 0.0,
        vdl2_in_rate: 0.0,
        vdl2_cfg,
        vdl2_idle_sent: None,
        ais_ddc: None,
        ais: None,
        ais_buf: Vec::new(),
        ais_center_hz: 0.0,
        ais_in_rate: 0.0,
        ais_cfg,
        ais_idle_sent: None,
        qo100_ddc: None,
        qo100: None,
        qo100_buf: Vec::new(),
        qo100_in_rate: 0.0,
        // Session-scoped only, unlike `skim_cfg`/`ism_cfg` — see
        // `sync_qo100`'s doc for why this one is not read back from disk.
        qo100_cfg: sdroxide_types::Qo100Settings::default(),
        qo100_auto: Qo100Auto::default(),
        iq_rec,
        iq_rec_buf: Vec::new(),
        iq_wav: None,
        scan_cfg,
        scan: None,
        scan_db: Vec::new(),
        audio_level: 0.0,
        wsjtx: None,
        wsjtx_cfg: sdroxide_types::WsjtxConfig::default(),
        wsjtx_beat: Instant::now(),
        rigctld: None,
        rigctld_cfg: RigctldConfig::default(),
        rigctld_err: None,
        rigctld_seen: None,
        last_s_dbm: -127.0,
        rds_dial_hz: 0.0,
        drm_dial_hz: 0.0,
        tci_srv: None,
        tci_cfg: TciServerConfig::default(),
        tci_srv_err: None,
        tci_iq_ddc: None,
        tci_iq_buf: Vec::new(),
        tci_iq_ilv: Vec::new(),
        tci_aud_rs: None,
        tci_aud_in_rate: 0.0,
        tci_aud_buf: Vec::new(),
        voice_rs: None,
        voice_rs_out_rate: 0.0,
        voice_buf: Vec::new(),
        voice_play: Vec::new(),
        main_play: Vec::new(),
        main_play_r: Vec::new(),
        binaural: None,
        bin_left: Vec::new(),
        speech_duck: 1.0,
        main_play_rec: Vec::new(),
        main_play_r_rec: Vec::new(),
        tci_tx: false,
        tci_tx_starved: 0,
        tci_pace: TciTxPace::default(),
        tci_tx_drain: false,
        tci_last_snap: None,
        audio_mode,
        radio_fs,
        decim,
        audio_bw,
        audio_re: Vec::new(),
        audio_play: Vec::new(),
        audio_play_rec: Vec::new(),
        audio_resampler,
        audio_rs_in_rate: radio_fs,
        audio_rs_out_rate: audio_out_rate,
        ext_audio_rate: 48_000.0,
        reopen: engine_cfg.reopen.map(|f| Arc::new(Mutex::new(f))),
        retry: None,
        retry_join: None,
        retry_at: None,
        retry_every: RETRY_FIRST,
        // Where the source opened, which is by definition a frequency it took.
        good_vfo_hz: source_center_hz,
        // Both VFOs start on the mode the receiver came up in; the remembered
        // pair is folded in below, once the engine owns them.
        vfo_memory: [initial_vfo_memory; 2],
        spots: sdroxide_net::SpotManager::new(),
        winlink: open_mailbox(&sdroxide_types::WinlinkConfig::default()),
        kiss: None,
        packet_port: None,
        net_cfg: sdroxide_types::NetworkConfig::default(),
        sat_cfg: sdroxide_types::SatConfig::default(),
        tle_refresh: None,
        sat_lock: None,
        rot_cfg: sdroxide_types::RotatorConfig::default(),
        relay_cfg: sdroxide_types::RelayConfig::default(),
        tr_switch: engine_cfg.tr_switch.clone(),
        relay_last_status: None,
        relay_pending: false,
        relay_lead_capped: false,
        rotator: None,
        rot_last_status: None,
        next_rot_emit: Instant::now(),
        session,
        want_antenna,
        band_antenna,
        want_gains,
        want_decimation,
        store: engine_cfg.store,
        instance: engine_cfg.instance,
        primary: engine_cfg.primary,
        tx_gate: engine_cfg.tx_gate,
        cw_gate_until: None,
        shared_gen_seen: engine_cfg.store_sync.as_ref().map_or(0, |s| s.generation()),
        store_sync: engine_cfg.store_sync,
        rade_watch: engine_cfg.rade_watch,
    };
    // After the struct, not before: the preference has to be applied through
    // the same path a reconnect uses, so both land on the same port.
    engine.restore_antennas();
    engine.restore_gains();
    // The opening state goes out here rather than with the capabilities above,
    // so the first thing every UI sees is the port the radio is actually on.
    let _ = engine.event_tx.send(RadioEvent::State(engine.state.clone()));
    if let Some(mic) = &engine.mic {
        engine.mic_resampler = MonoResampler::new(mic.rate, 48_000.0);
    }
    // Seed clients with the operator config (callsign/grid/templates) up front,
    // so the settings editors are populated even before any digital mode.
    let _ = engine
        .event_tx
        .send(RadioEvent::Ft8Status(sdroxide_types::DigiStatus::idle(engine.digi_config.clone())));
    // Likewise the voice keyer: the UI's slot list is whatever is on disk.
    engine.emit_voice_status();
    // And the transmit-image presets — five pictures the operator arranged once
    // and expects to find wherever they next sit down.
    engine.emit_image_presets();
    // If we start up already in a digital mode, spin up the controller.
    engine.sync_digi_mode();
    if !audio_mode {
        engine.sync_skimmer(); // starts if any kind is enabled in the saved config
        engine.sync_ism(); // likewise, from ism.json
        engine.sync_adsb_home();
        engine.sync_adsb(); // and the aircraft lane, if the mode is already ADS-B
        engine.sync_vdl2(); // ...and the datalink lane, likewise
        engine.sync_ais(); // ...and the shipping lane, likewise
        engine.sync_qo100(); // a no-op today: `qo100_cfg` starts disabled and is never loaded
    }
    // Start any enabled network spot feeds from the persisted config. The
    // operator identity comes from the digi config — one identity for the whole
    // app — and has to be in place before the feeds that log in with it.
    //
    // Only the primary engine brings the feeds up: they hold logins and
    // sockets (DX cluster, RBN, the reporters) that a station has one of, not
    // one per radio. First, before anything can start one.
    if !engine.primary {
        engine.spots.stand_down();
    }
    engine.spots.set_operator(&engine.digi_config.my_call, &engine.report_grid());
    engine.net_cfg = sdroxide_config::load_network_config();
    // Hand the persisted account to the mailbox. Without this the manager keeps
    // the defaults it was built with and every session refuses with "set a
    // callsign and password first", which reads exactly like the settings
    // having been lost — they were saved, they simply never arrived here.
    match engine.winlink.as_mut() {
        Some(wl) => wl.set_config(engine.net_cfg.winlink.clone()),
        None => engine.winlink = open_mailbox(&engine.net_cfg.winlink),
    }
    // Applied on every engine, not just the primary: the manager holds the
    // credentials a callsign lookup and a logbook upload need, and both are
    // per-request work whichever radio asks. `stand_down` above is what keeps
    // the station's *sockets* on the one engine meant to hold them, wherever
    // the settings were applied from. Withholding the config here instead left
    // a second radio without it — and left the operator's next APPLY from that
    // radio's window opening a duplicate of every feed, a second DX cluster
    // login and a second FreeDV Reporter session under the same callsign.
    engine.spots.set_config(engine.net_cfg.clone());
    // Bring up the built-in TCI server (enabled by default) so third-party
    // clients can connect without the operator having to arm anything. Each
    // radio has its own scoped config — additional radios are seeded with the
    // server disabled, or the port would collide.
    engine.tci_cfg = engine.store.load_tci_server_config();
    engine.sync_tci_server();
    // The rigctld server is off unless the operator turned it on: port 4532 is
    // commonly already taken by a real rigctld, and it has no authentication.
    engine.rigctld_cfg = engine.store.load_rigctld_config();
    engine.sync_rigctld();
    // WSJT-X UDP broadcast is likewise off unless the operator turned it on.
    engine.wsjtx_cfg = engine.store.load_wsjtx_config();
    engine.sync_wsjtx();
    // The satellite additions: the display tracker lives in the UI, but the
    // element sets are persisted here — and the satellite *lock*, when the
    // operator engages one, runs here too (see `poll_sat_track`).
    engine.sat_cfg = sdroxide_config::load_sat_config();
    // The rotctld client a satellite lock steers the antenna through. Primary
    // only: there is one antenna rotator, and one rotctld to speak to.
    engine.rot_cfg = sdroxide_config::load_rotator_config();
    if engine.primary {
        engine.sync_rotator();
    }
    engine.relay_cfg = sdroxide_config::load_relay_config();
    // One station, one antenna line: only the primary engine opens the switch.
    // Every engine still publishes into it — see `Engine::poll_tr_switch`.
    if engine.primary {
        engine.sync_relay();
    }
    // Seed clients with the whole station configuration up front, for the same
    // reason as the operator config above: a settings dialog that has not been
    // told what the station is set to would show defaults, and applying those
    // would write them over the real thing.
    engine.emit_station_config();
    engine.emit_tle_sub_status();
    // And this radio's own interface configuration, for the same reason again.
    // It is the only route a remote operator has to the settings that belong to
    // the device rather than to the receiver chain — a dongle's AGC mode, its
    // ppm correction, its bias tee.
    engine.emit_radio_config();
    // The mixer above was built before `radio.json` was read, so the trim it
    // holds has to be handed over once here — everything after this arrives
    // through `SetRadioConfig`.
    engine.refresh_af_trim();
    // The source opened with its LO on the requested frequency, which is also
    // where the VFO now sits — on zero-IF hardware that is the one place the VFO
    // must not be, so let the span check park the LO clear of it before the
    // first block arrives.
    // A front end that follows the operator's mode is told it before the first
    // block, not on the first change: the mode came out of the session, and a
    // panadapter pairing whose receiver offset depends on it would otherwise
    // open on the wrong intermediate frequency.
    // The mode the *inactive* VFO was left in. The active one is already in
    // force — it came from `--mode`, from the session, or from the default, and
    // whichever it was outranks this — so only the other slot is filled in.
    // Nothing is applied to the receiver here: this is the shelf, and it is read
    // the first time the operator switches VFO (issue #286).
    if let Some(modes) = engine.session.as_ref().and_then(|s| s.vfo_modes) {
        let idle = 1 - engine.state.active_vfo.index();
        engine.vfo_memory[idle].mode = modes[idle];
        let (lo, hi) = modes[idle].default_filter_at(match idle {
            0 => engine.state.vfo_a_hz,
            _ => engine.state.vfo_b_hz,
        });
        (engine.vfo_memory[idle].filter_lo, engine.vfo_memory[idle].filter_hi) = (lo, hi);
    }
    engine.push_rx_mode();
    engine.keep_vfo_in_span();
    engine.update_tuning();
    // And a session restored into CW puts a radio that keys its own transmitter
    // a sidetone off our dial, which the span check above has no reason to ask
    // for — the VFO is sitting exactly on the centre. See `sync_cw_dial`.
    engine.sync_cw_dial();

    // Where the block loop's lanes go when the machine has cores to spare.
    // Built once: a pool is threads, and starting them per block would cost
    // more than the fork saves. `None` on a small machine — see [`lane_pool`].
    let pool = lane_pool();

    let mut buf = vec![Complex32::default(); 16_384];
    // Where a decimated block lands. A local rather than a field on the engine,
    // so the borrow of the decimator ends before the samples are handed on.
    let mut dbuf: Vec<Complex32> = Vec::new();
    let mut next_frame = Instant::now();
    let mut next_row = Instant::now();
    let mut next_meters = Instant::now();
    // Panadapter rate diagnostics. Three numbers, because they fail
    // differently: samples/s says whether the front end is delivering, fft/s
    // is the rate the picture actually changes at, and frames/s is only the
    // rate it is *published* at — a frame carries a fresh `seq` whether or not
    // a transform landed behind it, so the two part company on any slow lane.
    // A 24 kHz front end through a 4096-point window hopped by half fills one
    // every 85 ms, which is 11.7 fft/s against 30 frames/s.
    let mut lane_at = Instant::now();
    let mut lane_samples: u64 = 0;
    let mut lane_frames: u64 = 0;
    // Frames the client never fetched, and whether the last one was among them
    // — see the publish below.
    let mut lane_dropped: u64 = 0;
    let mut dropped_frame = false;
    let mut lane_rows: u64 = 0;
    let mut lane_rows_sent: u64 = 0;
    let mut lane_ffts = 0u64;
    let mut lane_sweeps: u64 = 0;
    let mut next_rds = Instant::now();
    let mut next_drm = Instant::now();
    let mut next_session = Instant::now() + SESSION_SAVE_INTERVAL;

    // The band-dependent gain ranges, once, before anything is published: the
    // ones the device was opened with are the model's widest, and the band the
    // session came up on is very likely not that band. Kept out of the loop
    // below — unlike the antenna refresh beside it, building this allocates,
    // and the three things that can move it (retune, port, HDR) each ask for
    // it where they happen.
    engine.refresh_rx_gains();

    // The commands waiting at the top of a tick, kept between iterations so the
    // batch costs no allocation. See [`collapse_superseded`].
    let mut batch: Vec<Command> = Vec::new();
    loop {
        batch.clear();
        loop {
            match cmd_rx.try_recv() {
                Ok(cmd) => batch.push(cmd),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if engine.tx_active {
                        let _ = engine.source.tx_end();
                    }
                    // A dying engine must not leave the station interlock
                    // claimed, or no surviving radio could ever key again —
                    // nor its claim on the T/R switch, or the station's antenna
                    // relay stays where this engine left it.
                    engine.tx_active = false;
                    engine.rig_tx = false;
                    engine.cw_gate_until = None;
                    engine.release_tx_gate();
                    info!("all controllers gone; engine stopping");
                    return;
                }
            }
        }
        collapse_superseded(&mut batch);
        for cmd in batch.drain(..) {
            engine.apply(cmd);
        }

        // Frontend device swaps: audio (rebuilt cpal ring endpoints) and radio
        // interface (rebuild the IQ source from the persisted config).
        while let Ok(swap) = swap_rx.try_recv() {
            match swap {
                EngineSwap::Output(a) => engine.set_audio_output(a),
                EngineSwap::Input(m) => engine.set_audio_input(m),
                EngineSwap::ReopenSource => engine.reopen_source(),
                EngineSwap::ReloadSharedStores => engine.reload_shared_stores(),
            }
        }

        // Catch up with shared-store writes by any other engine in the process
        // (an atomic load per tick; reloads only when the generation moved).
        engine.poll_shared_stores();

        // The band plan's repeater shift, if the operator asked to follow it.
        // Ahead of `push_tx_freq`, which is what tells an accessory where we
        // would transmit — and the shift is part of that answer.
        engine.refresh_auto_shift();

        // Where we would transmit, for band-switching accessories that have to
        // be on the right band before any RF appears. No-op unless the source
        // has something downstream of it to switch.
        engine.push_tx_freq();

        // Out-of-band control changes from a CAT rig (dial/mode moved on the
        // radio itself). No-op for SoapySDR/siggen/file.
        let updates = engine.source.poll_control();
        for u in updates {
            engine.apply_control(u);
        }
        // Asked after them, because one of those updates may be the answer:
        // whether the front end's centre is a dial we can move is not fixed for
        // every source (see `Self::refresh_center_is_dial`).
        engine.refresh_center_is_dial();
        // …and whether it has an antenna selector, which a CI-V rig only
        // answers once its control link has been open a round trip.
        engine.refresh_antennas();

        // Drive the FT8/FT4 slot machine (runs in both RX and TX). Returns
        // owned actions to avoid borrowing `engine.digi` and `engine` at once.
        engine.poll_digi();
        engine.poll_voice();
        engine.poll_skimmer();
        engine.poll_ism();
        engine.poll_adsb();
        engine.poll_vdl2();
        engine.poll_ais();
        engine.poll_qo100();
        engine.poll_scanner();
        engine.poll_tci_server();
        engine.poll_rigctld();
        engine.poll_band_change();
        engine.wsjtx_heartbeat();
        engine.poll_spots();
        engine.poll_winlink();
        engine.poll_kiss_server();
        engine.poll_images();
        engine.poll_tle_refresh();
        engine.poll_sat_track();
        engine.poll_rotator_status();
        engine.poll_tr_switch();
        // Attach (or re-attach) the configured radio on its own when the
        // front-end is only a stand-in — no trip through Settings.
        engine.poll_reconnect();

        if engine.tx_active {
            // Blocking TX write paces this loop at ~10 ms per block.
            if let Err(e) = engine.tx_block() {
                let _ = engine.event_tx.send(RadioEvent::ConnectionLost(e.to_string()));
                // Unkey on the way out. This thread is the only thing that ever
                // calls `tx_end`, so returning without it leaves the
                // transmitter exactly as the failed block found it: keyed. On
                // a CAT rig that is a PTT command sent and never taken back —
                // the radio stays on the air with nothing left running to stop
                // it, which is how a mis-declared transmit path (a source with
                // no `tx_write`) turned a refused over into a stuck one.
                // Best-effort by necessity: the failure being reported may well
                // be the same link this has to travel over.
                if let Err(e) = engine.source.tx_end() {
                    warn!("could not unkey after a transmit failure: {e}");
                }
                // Same as the controller-gone exit: a dead engine must not
                // keep the station interlock, nor the T/R switch.
                engine.tx_active = false;
                engine.rig_tx = false;
                engine.cw_gate_until = None;
                engine.release_tx_gate();
                return;
            }
            // Full-duplex hardware keeps receiving during TX — but only from
            // what has already arrived. This thread owes the transmitter a
            // block every 10 ms, and a receive read that waits for samples
            // spends that budget: the transmit ring empties, and hardware that
            // answers an underrun by skipping ahead (SoapySX does) puts the
            // over on the air as chirps. See `IqSource::read_available`.
            if engine.caps.full_duplex && !engine.audio_mode {
                if let Ok(n @ 1..) = engine.source.read_available(&mut buf) {
                    engine.samples_read += n as u64;
                    engine.last_block = n as u64;
                    engine.adc.observe(&buf[..n]);
                    let iq = decimate(engine.decim.as_mut(), &buf[..n], &mut dbuf);
                    engine.run_audio(iq);
                }
            }
        } else {
            match engine.source.read(&mut buf) {
                Ok(0) => continue, // timeout
                Ok(n) if engine.audio_mode => {
                    lane_samples += n as u64;
                    engine.samples_read += n as u64;
                    engine.last_block = n as u64;
                    engine.adc.observe(&buf[..n]);
                    engine.run_audio_mode(&buf[..n]);
                }
                Ok(n) => {
                    lane_samples += n as u64;
                    // Where on the stream we are, for a front end whose retunes
                    // land later than they are given — see `center_trail`.
                    engine.samples_read += n as u64;
                    engine.last_block = n as u64;
                    // Ahead of the blanker and of `decimate`, both of which
                    // destroy what this is looking for. See `AdcMeter`.
                    engine.adc.observe(&buf[..n]);
                    // Blanking comes first, at the device rate: an impulse is
                    // only an impulse before the anti-alias filter smears it
                    // over a filter length, and after decimation there would be
                    // nothing left for the blanker to recognise.
                    if engine.state.noise_blanker {
                        engine.nb.process(&mut buf[..n]);
                    }
                    let iq = decimate(engine.decim.as_mut(), &buf[..n], &mut dbuf);
                    engine.process_block(iq, pool.as_ref());
                }
                Err(e) => {
                    let _ = engine.event_tx.send(RadioEvent::ConnectionLost(e.to_string()));
                    return;
                }
            }
        }

        let now = Instant::now();
        // Ahead of both lanes: on a front end with no I/Q the main panadapter is
        // built from the same sweep as the strip, so the sweep has to be in hand
        // before either frame is made.
        engine.poll_wide();
        // The waterfall's own clock, and the reason it is not the frame clock:
        // a row is a few kilobytes appended to a texture, a frame is a repaint.
        // Rows are clocked here at whatever rate the client asked for and ride
        // out in whichever frame comes next, so a screen redrawing sixty times
        // a second can still be handed two hundred lines of band.
        //
        // Caught up rather than accumulated: a row period shorter than the
        // block this loop is processing (a slow front end, a stalled thread)
        // would otherwise build a backlog that never drains. The batch is
        // capped as well — see `MAX_BATCH_ROWS`.
        let row_period = Duration::from_secs_f64(1.0 / f64::from(engine.cfg.rows()));
        if engine.row_sample_clock {
            // The samples are the clock; keep this one parked so handing the
            // job back (a switch to a demod-audio rig) does not fire a burst.
            next_row = now + row_period;
        } else if now >= next_row {
            let behind = now.duration_since(next_row);
            let skipped = (behind.as_secs_f64() / row_period.as_secs_f64()) as u32;
            next_row = now + row_period - behind.min(row_period * skipped.max(1));
            next_row = next_row.max(now);
            engine.push_row();
        }
        if now >= next_frame {
            next_frame = now + Duration::from_secs_f64(1.0 / engine.cfg.fps.max(1) as f64);
            let mut frame = engine.make_spectrum_frame();
            // Nothing measured yet on the lane that drew — see
            // `Engine::drew_empty`. Held back rather than published: the client
            // goes on showing the picture it has, and the batched rows wait for
            // a frame that has something in it. The next tick is scheduled
            // either way, so this costs at most one frame period.
            if !engine.drew_empty() {
                engine.attach_rows(&mut frame);
                // A triple buffer keeps only the newest value, so a client that
                // repaints more slowly than this engine publishes loses whole
                // frames — and every waterfall row that rode in them. The picture
                // then scrolls slower than the time axis beside it says it does,
                // by exactly the fraction dropped, and goes on doing so for ever.
                //
                // `publish` reports the overwrite, and the buffer handed back is
                // the frame nobody fetched, so its rows can be carried into this
                // one. Same rule and the same bound as the network client's
                // `carry_rows_from` — a backlog longer than that is a client that
                // stopped drawing, not a hitch to be made good.
                if dropped_frame {
                    frame.carry_rows_from(spec_in.input_buffer());
                }
                *spec_in.input_buffer_mut() = frame;
                dropped_frame = spec_in.publish();
                if dropped_frame {
                    lane_dropped += 1;
                }
                lane_frames += 1;
            }
        }
        // Once a second, and only where somebody asked for it: this is the
        // measurement that tells a starved front end from a lane that is
        // simply running at its own rate.
        if now.duration_since(lane_at) >= Duration::from_secs(1) {
            let secs = now.duration_since(lane_at).as_secs_f64();
            // Saturating, not wrapping: the analyser is rebuilt whenever the
            // rate, the FFT size or the decimation changes, and a rebuild puts
            // its counter back to zero. Subtracting the old baseline from that
            // wrapped to 1.8e19 and printed it as a rate.
            let ffts = engine.analyzer.transforms();
            let ffts_delta = ffts.saturating_sub(lane_ffts);
            debug!(
                target: "sdroxide::panadapter",
                samples_per_s = lane_samples as f64 / secs,
                fft_per_s = ffts_delta as f64 / secs,
                frames_per_s = lane_frames as f64 / secs,
                // Of those, the ones the client never fetched. Not a fault —
                // publishing faster than a client repaints is ordinary — but
                // their rows are carried forward rather than dropped, and this
                // is how to tell whether that is happening.
                frames_unfetched_per_s = lane_dropped as f64 / secs,
                // The waterfall's real time resolution, which is the number
                // this diagnostic exists to separate from the other two.
                rows_per_s = engine.rows_clocked.saturating_sub(lane_rows) as f64 / secs,
                rows_sent_per_s = engine.rows_sent.saturating_sub(lane_rows_sent) as f64 / secs,
                // Finished sweeps from a front end that computes its own
                // spectrum. Zero on an I/Q receiver; on a rig whose scope is
                // the main panadapter it is the real picture rate, and the
                // number to compare `rows_per_s` against before believing a
                // report that the waterfall looks blocky.
                sweeps_per_s = engine.wide_sweeps.saturating_sub(lane_sweeps) as f64 / secs,
                rate_hz = engine.state.sample_rate,
                fft_size = engine.cfg.fft_size,
                // What the frames are actually being cut into, which is the
                // client's choice and not this engine's — worth naming next to
                // the transform size, because "the FFT is 32768" and "the
                // picture is 2048 columns wide" answer different questions.
                bins = engine.cfg.bins(),
                zoom = engine.zoom.is_some(),
                audio_mode = engine.audio_mode,
                // The window the client asked for, and — on a demod-audio rig
                // — the rig's own passband it is measured against to decide
                // which lane draws. See `Engine::audio_zoom_window`.
                view = ?engine.cfg.viewport,
                audio_band = ?engine.audio_mode.then(|| engine.audio_band()),
                "panadapter rates",
            );
            lane_at = now;
            lane_samples = 0;
            lane_frames = 0;
            lane_dropped = 0;
            lane_rows = engine.rows_clocked;
            lane_rows_sent = engine.rows_sent;
            lane_sweeps = engine.wide_sweeps;
            lane_ffts = ffts;
        }
        if let Some(frame) = engine.make_wide_frame() {
            wide_in.write(frame);
        }
        if now >= next_meters {
            next_meters = now + METER_INTERVAL;
            // How much of the disk the I/Q capture has taken, for the readout
            // beside the button. At 2.4 Msps this climbs by 19 MB a second and
            // an operator wants to see that before the disk fills.
            if let Some(w) = engine.iq_wav.as_ref() {
                let mb = (w.bytes() / (1 << 20)) as u32;
                if engine.state.iq_recording_mb != mb {
                    engine.state.iq_recording_mb = mb;
                    let _ = engine.event_tx.send(RadioEvent::State(engine.state.clone()));
                }
            }
            // The transmitter is on the air whether this engine keyed it or the
            // operator did it at the radio (`rig_tx`). Both put the meter into
            // transmit, and the SWR of an over is worth reading either way —
            // but only an over *we* key is one the guard may unkey, which is
            // why the rails below stay on `tx_active` alone.
            let meters = if engine.tx_active || engine.rig_tx {
                // CAT/TCI rigs report real forward power / SWR; HackRF and other
                // IQ sources have no such sensor and leave both `None` (the meter
                // then falls back to showing drive-side ALC).
                let tele = engine.source.tx_telemetry().unwrap_or_default();
                // The rig's own ALC where it reports one, our modulator's peak
                // otherwise. They are different measurements and the rig's is
                // the one that answers the operator's question: `alc_peak` is
                // what SDRoxide SENDS, and ALC is what the rig does about it.
                // On a CAT rig driving an external radio our figure says
                // nothing useful about whether the audio is too hot for it.
                let alc = tele
                    .alc
                    .unwrap_or_else(|| engine.tx.as_ref().map(|t| t.alc_peak).unwrap_or(0.0));
                // Clients that asked for `tx_sensors` get the same figures.
                if let Some(srv) = engine.tci_srv.as_ref() {
                    srv.push_telemetry(tele);
                }
                // The SWR guard. Placed here because this is the one point in
                // the engine where a fresh SWR reading and `tx_active` are both
                // in hand; it runs before the meters are published so the trip
                // and the reading the operator sees cannot disagree.
                // Which rails apply is decided per sample rather than at
                // key-up, because `tune` can be switched on and off inside one
                // key-down and the reading in hand belongs to whatever the
                // transmitter is doing at this instant. Switching between them
                // restarts the grace period, so a tune begun mid-over gets the
                // full five seconds it would have got from receive rather than
                // inheriting a window that has already expired.
                let tuning = engine.state.tx.tune;
                if tuning != engine.swr_tuning {
                    engine.swr_tuning = tuning;
                    engine.swr_settle = 0;
                    engine.swr_over = 0;
                }
                engine.swr_settle = engine.swr_settle.saturating_add(1);
                let (swr_limit, swr_settle) = swr_rails(tuning, engine.swr_limit);
                // ⛔ `tx_active`, not "on the air". The guard's whole action is
                // to unkey, and the only over it can unkey is one this engine
                // is driving: an operator holding the radio's microphone would
                // simply keep transmitting while sdroxide sent an unkey the rig
                // ignores — or, worse on a rig whose key-down doubles as an
                // audio-source switch, would be cut off mid-word by sdroxide
                // "helping". The radio has its own protection for its own over.
                if engine.tx_active
                    && engine.swr_guard
                    && engine.swr_tripped.is_none()
                    && engine.swr_settle > swr_settle
                {
                    // ⛔ FORWARD POWER IS OPTIONAL AND MUST STAY OPTIONAL. This
                    // read `(Some(swr), Some(fwd))` until 17 August 2026, on the
                    // reasoning that a rig reporting SWR without power "cannot be
                    // vetted". That reasoning cost two failed live tests: the
                    // CI-V driver sends `TxTelemetry { fwd_w: None, swr: Some(v) }`,
                    // so on every Icom the arm could never match and the guard
                    // was dead code. Requiring a figure the most common rig
                    // family never sends is not caution, it is a silent
                    // disablement.
                    match tele.swr {
                        Some(swr)
                            if swr >= swr_limit
                                && tele.fwd_w.map_or(true, |f| f >= SWR_MIN_FWD_W) =>
                        {
                            engine.swr_over = engine.swr_over.saturating_add(1);
                            if engine.swr_over >= SWR_TRIP_SAMPLES {
                                let limit = swr_limit;
                                warn!(swr, limit, tuning, "SWR guard tripped, unkeying");
                                // Latch FIRST, so the transmitter cannot be
                                // re-keyed in the window between unkeying and
                                // the operator seeing the warning.
                                engine.swr_tripped = Some(swr);
                                engine.state.tx.swr_tripped = Some(swr);
                                engine.swr_over = 0;
                                // ⛔ NOT `deny_tx`, and the difference matters.
                                // That helper only sets the state flags, on the
                                // documented assumption that every deny happens
                                // BEFORE the transmitter keyed — true of the
                                // rails in `sync_tx_state`, false here, where
                                // the rig is keyed and on the air right now.
                                // Keying never touches `tx_active` directly; it
                                // goes through `Command::SetPtt` so the guards
                                // and the hardware both follow. Same route the
                                // TCI starvation unkey uses.
                                engine.apply(Command::SetTune(false));
                                engine.apply(Command::SetPtt(false));
                                // Short on purpose. This lands on a banner in
                                // front of an operator who has just had a
                                // transmission cut off, and the two things worth
                                // reading are the figure and what to look at.
                                // Says "tune limit" when that is the figure
                                // being quoted, so the number in front of the
                                // operator is never one they cannot find in
                                // the settings — the tune limit is derived,
                                // not typed.
                                let (what, which) = match tuning {
                                    true => ("Tune stopped", "tune limit"),
                                    false => ("Transmit stopped", "limit"),
                                };
                                engine.notice(&format!(
                                    "{what}. SWR {swr:.1}:1, {which} {limit:.1}:1. \
                                     Check the antenna."
                                ));
                                engine.emit_state();
                            }
                        }
                        // A reading that is fine resets the run. The count is for
                        // CONSECUTIVE samples: an isolated spike is exactly what
                        // the debounce exists to ignore.
                        Some(_) => engine.swr_over = 0,
                        // ⚠️ NO READING IS NOT A GOOD READING. Neither advance
                        // nor reset: the rig has simply not said anything yet.
                        // Resetting here would have been the same class of bug
                        // as the one above, quietly holding the count at zero on
                        // any rig whose telemetry arrives slower than the meter
                        // tick.
                        None => {}
                    }
                }
                let (adc_peak_dbfs, adc_clip) = engine.adc.read();
                Some(Meters {
                    s_dbm: -127.0,
                    adc_peak_dbfs,
                    adc_clip,
                    tx: Some(TxMeters { fwd_w: tele.fwd_w, swr: tele.swr, alc, po: tele.po }),
                    stereo: false,
                    tone: None,
                })
            } else {
                // Not transmitting: both SWR counters belong to an over, so they
                // reset here. Doing it on every receive sample rather than at
                // one un-key site means no path out of transmit can leave a
                // stale count behind to blank or trip the NEXT over.
                // ⚠️ The LATCH is deliberately not touched: it survives until
                // acknowledged, which is the whole point of it.
                engine.swr_over = 0;
                engine.swr_settle = 0;
                engine.swr_tuning = false;
                let stereo = engine.main.as_ref().is_some_and(|c| c.stereo_locked());
                let tone = engine.main.as_ref().and_then(|c| c.sub_tone());
                // Read unconditionally, so the window is consumed whether or not
                // a reading is published — otherwise a front end with no signal
                // report would accumulate one reading over the whole session.
                let (adc_peak_dbfs, adc_clip) = engine.adc.read();
                engine.rx_signal_dbm().map(|s_dbm| Meters {
                    s_dbm,
                    adc_peak_dbfs,
                    adc_clip,
                    tx: None,
                    stereo,
                    tone,
                })
            };
            if let Some(m) = meters {
                engine.last_s_dbm = m.s_dbm;
                let _ = engine.event_tx.send(RadioEvent::Meters(m));
            }
        }
        if now >= next_rds {
            next_rds = now + RDS_INTERVAL;
            // Main receiver only, like the stereo indicator: RDS belongs to the
            // station being listened to, and a sub receiver parked on a second
            // broadcast would have nowhere to show it.
            if let Some(rds) = engine.main.as_mut().and_then(|c| c.take_rds()) {
                let _ = engine.event_tx.send(RadioEvent::Rds(rds));
            }
        }
        if now >= next_drm {
            next_drm = now + DRM_INTERVAL;
            // Main receiver only, like RDS: the broadcast being listened to is
            // the one whose label and text belong on the panel.
            if let Some(drm) = engine.main.as_mut().and_then(|c| c.take_drm()) {
                let _ = engine.event_tx.send(RadioEvent::Drm(drm));
            }
        }
        if now >= next_session {
            next_session = now + SESSION_SAVE_INTERVAL;
            engine.save_session();
            engine.flush_digi_config();
        }
    }
}

/// Allocation-free fingerprint of everything a rigctld client can observe.
///
/// Floats are compared by bit pattern rather than value so the digest derives
/// `Eq`; an exact-equality check is what is wanted here anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RigDigest {
    vfo_a: u64,
    vfo_b: u64,
    active_b: bool,
    split: bool,
    mode: Mode,
    filter_lo: u32,
    filter_hi: u32,
    ptt: bool,
    tune: bool,
    rit: i32,
    xit: i32,
    drive: u32,
    volume: u32,
    mic_gain: u32,
    band: Band,
    muted: bool,
    strength: i32,
    noise_blanker: bool,
    noise_reduction: bool,
    auto_notch: bool,
    ranges: (usize, usize),
}

impl RigDigest {
    fn of(s: &RigState) -> Self {
        RigDigest {
            vfo_a: s.vfo_a_hz.to_bits(),
            vfo_b: s.vfo_b_hz.to_bits(),
            active_b: s.active_vfo == sdroxide_types::Vfo::B,
            split: s.split,
            mode: s.mode,
            filter_lo: s.filter_lo.to_bits(),
            filter_hi: s.filter_hi.to_bits(),
            ptt: s.ptt,
            tune: s.tune,
            rit: s.rit_hz,
            xit: s.xit_hz,
            drive: s.drive.to_bits(),
            volume: s.volume.to_bits(),
            mic_gain: s.mic_gain.to_bits(),
            band: s.band,
            muted: s.muted,
            strength: s.strength_dbm,
            noise_blanker: s.noise_blanker,
            noise_reduction: s.noise_reduction,
            auto_notch: s.auto_notch,
            ranges: (s.rx_ranges.len(), s.tx_ranges.len()),
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Where the radio was left, for the next start. Here as well as on the
        // periodic tick because a clean quit is the common case, and it would
        // otherwise lose up to one tick's worth of tuning.
        self.save_session();
        // And the transmit-audio rail, for the same reason: an operator who
        // trims their level and quits has set it, not been trying it out.
        self.flush_digi_config();
        // Finalize any in-progress recording so the MP3 file is closed cleanly
        // when the engine thread exits (all controllers gone / fatal error).
        if let Some(rec) = self.recorder.take() {
            rec.stop();
        }
        // And the I/Q capture, which needs its header patched or the file
        // reads as empty — every byte of it written and none of it playable.
        self.stop_iq_recording();
        // Store a voice-keyer message that was still being recorded, rather
        // than throwing away what the operator had just said.
        self.voice.stop_record();
        // A reconnect attempt may be halfway through opening a device; let it
        // finish rather than leave it running into process exit, where its
        // teardown would race the device libraries' own exit handlers.
        if let Some(j) = self.retry_join.take() {
            let _ = j.join();
        }
    }
}

impl Engine {
    fn run_audio(&mut self, iq: &[Complex32]) {
        let want_rec_main = self.recorder.is_some() && !self.caps.rx_audio_external;
        let rx0 = self.state.rx[0];
        self.refresh_channel_rate();
        let Some(main) = self.main.as_mut() else { return };
        // Four disjoint fields, so the chain's own borrow and the buffers its
        // output is copied into can be live at once.
        run_chain_block(
            main,
            &rx0,
            iq,
            want_rec_main,
            (
                &mut self.main_play,
                &mut self.main_play_r,
                &mut self.main_play_rec,
                &mut self.main_play_r_rec,
            ),
        );
        self.finish_audio(iq);
    }

    /// One block through every lane that consumes it, on as many cores as the
    /// machine has to spare.
    ///
    /// Three things read these samples and write nothing in common: the
    /// device-wide panadapter analyser, the panadapter's zoom lane, and the
    /// receive chain. One after the other they are a sum — on an RX-888 at
    /// 32.4 Msps, most of a core on a machine with thirty-one idle ones. Side
    /// by side they are a maximum.
    ///
    /// **Which half is sent matters.** The receive chain is not `Send`:
    /// DeepFilterNet's inference plan holds `Rc`s, so a chain cannot cross a
    /// thread boundary at all. The two analysers can, so they are what goes to
    /// the pool while the chain stays here — which is also why this uses
    /// `in_place_scope` rather than `join`, since only the spawned side of a
    /// scope has to be `Send`.
    ///
    /// The fork is taken only for a block that lies wholly inside one waterfall
    /// row. A row is pooled from whichever lane is drawing, which is a question
    /// about the engine's *state* and cannot be asked while the analysers are
    /// away — so a block a row boundary falls inside runs the ordinary
    /// sequential path. On a front end fast enough for this to matter that is a
    /// small minority of blocks (an RX-888 at 32.4 Msps clocks a row every
    /// fifteenth one); on a slow one it is most of them, and there is nothing
    /// there to win anyway.
    fn process_block(&mut self, iq: &[Complex32], pool: Option<&rayon::ThreadPool>) {
        self.capture_iq(iq);
        // Read while the chain is certainly in hand, and on both paths: the
        // frame builder asks for it and must never be answered with the
        // stand-in.
        self.refresh_channel_rate();
        let per_row = self.row_period_samples();
        let one_row = self.row_samples + iq.len() <= per_row;
        let Some(pool) = pool.filter(|_| one_row && !iq.is_empty()) else {
            self.feed_panadapter(iq);
            self.run_audio(iq);
            return;
        };
        self.row_sample_clock = true;
        self.sync_zoom();

        let want_rec_main = self.recorder.is_some() && !self.caps.rx_audio_external;
        let rx0 = self.state.rx[0];
        // Named apart so the compiler can see that the three lanes below borrow
        // disjoint fields of the engine.
        let analyzer = &mut self.analyzer;
        let zoom = self.zoom.as_mut();
        let chain = self.main.as_mut();
        let play = &mut self.main_play;
        let play_r = &mut self.main_play_r;
        let play_rec = &mut self.main_play_rec;
        let play_r_rec = &mut self.main_play_r_rec;

        pool.in_place_scope(|scope| {
            scope.spawn(move |_| analyzer.process(iq));
            if let Some(zoom) = zoom {
                scope.spawn(move |_| zoom.process(iq));
            }
            // This thread's share, and the one that could not have been
            // anywhere else.
            if let Some(chain) = chain {
                run_chain_block(
                    chain,
                    &rx0,
                    iq,
                    want_rec_main,
                    (play, play_r, play_rec, play_r_rec),
                );
            }
        });

        self.row_samples += iq.len();
        if self.row_samples >= per_row {
            self.row_samples = 0;
            self.push_row();
        }
        if self.main.is_some() {
            self.finish_audio(iq);
        }
    }

    /// Write the raw block to whichever captures are running.
    ///
    /// The first thing a block meets, and deliberately not part of the receive
    /// chain: what is written has to be exactly what the receiver delivered,
    /// before any lane takes its own decimation of it. It is also the one place
    /// that must not depend on there being audio — a headless station with no
    /// sound card still has a spectrum worth recording, and both of these used
    /// to sit past the early return `EngineConfig` with no audio takes.
    fn capture_iq(&mut self, iq: &[Complex32]) {
        // The command line's raw stream, read back by `--file`.
        if let Some(w) = self.iq_rec.as_mut() {
            use std::io::Write;
            self.iq_rec_buf.clear();
            self.iq_rec_buf.reserve(iq.len() * 8);
            for z in iq {
                self.iq_rec_buf.extend_from_slice(&z.re.to_le_bytes());
                self.iq_rec_buf.extend_from_slice(&z.im.to_le_bytes());
            }
            // A capture that cannot be written is worth one complaint and then
            // silence: failing per block would fill the log faster than the disk.
            if let Err(e) = w.write_all(&self.iq_rec_buf) {
                warn!("IQ capture write failed, stopping the recording: {e}");
                self.iq_rec = None;
            }
        }
        // …and the operator's own, from the REC popup.
        if let Some(w) = self.iq_wav.as_mut()
            && let Err(e) = w.write(iq)
        {
            warn!("I/Q capture write failed, stopping the recording: {e}");
            self.stop_iq_recording();
            let _ =
                self.event_tx.send(RadioEvent::Notice(Some(format!("I/Q recording stopped: {e}"))));
        }
    }

    /// Everything downstream of the receive chain: the speaker, the decoders,
    /// the recorders and the lanes that take their own decimation of the raw
    /// block. Split from [`Engine::run_audio`] so the chain itself can be
    /// forked — see [`Engine::process_block`].
    fn finish_audio(&mut self, iq: &[Complex32]) {
        let want_rec = self.recorder.is_some();
        // A radio listening to its transceiver while an attached receiver
        // paints the picture. The main chain still runs — the high-resolution
        // channel analyzer reads its DDC output, and the sub receiver is a
        // second DDC on the same I/Q — but nothing demodulated here is heard,
        // so its speaker and recorder taps are dropped and the transceiver's
        // audio takes their place below.
        let ext = self.caps.rx_audio_external;
        let Some(out_rate) = self.main.as_ref().map(|m| m.out_rate) else { return };

        if ext {
            // Everything the operator hears is the transceiver's: the speaker,
            // the recording and the decoders alike. `play_rx_audio` is the same
            // stage a demod-audio rig runs — level, digital-mode tap,
            // auto-notch, noise reduction, volume — and it leaves its result in
            // `audio_play`, from where it stands in for what the main chain
            // demodulated. Only the left ear: the sub receiver is the attached
            // receiver's and keeps the right one, below.
            self.audio_re.clear();
            if let Some(rate) = self.source.rx_audio(&mut self.audio_re)
                && rate > 0.0
            {
                self.ext_audio_rate = rate;
            }
            self.play_rx_audio(self.ext_audio_rate);
            self.main_play.clear();
            self.main_play.extend_from_slice(&self.audio_play);
            self.main_play_r.clear();
            self.main_play_rec.clear();
            self.main_play_rec.extend_from_slice(&self.audio_play_rec);
            self.main_play_r_rec.clear();
        } else {
            // Feed the digital-mode decoder from the clean tap (not the mixed,
            // possibly-muted output).
            if let (Some(digi), Some(main)) = (self.digi.as_mut(), self.main.as_ref()) {
                if main.tap_enabled {
                    digi.on_rx_audio(&main.tap_out);
                }
            }
            // Digital voice: play the decoded speech instead of the demodulated
            // signal. The mode declines while it is out of sync, so the operator
            // still hears the raw audio while tuning — unless they asked for it
            // muted.
            // Monitoring a voice-keyer message takes the speakers for its duration:
            // the operator asked to hear the recording, not the band.
            let block = self.main_play.len();
            if self.take_preview_audio(out_rate, block) {
                self.main_play.clear();
                self.main_play.extend_from_slice(&self.voice_prev_out);
                self.main_play_r.clear();
                // Unscaled in both cases: preview audio was never subject to the
                // AF knob to begin with.
                self.main_play_rec.clear();
                self.main_play_r_rec.clear();
                if want_rec {
                    self.main_play_rec.extend_from_slice(&self.voice_prev_out);
                }
            } else if self.take_voice_audio(out_rate) {
                let rx0 = &self.state.rx[0];
                let vol = if rx0.muted { 0.0 } else { rx0.volume * rx0.volume };
                self.main_play.clear();
                self.main_play.extend(self.voice_play.iter().map(|s| s * vol));
                self.main_play_r.clear();
                // Recorder gets the decoded speech unscaled — same reasoning as
                // the demodulated-audio tap above.
                self.main_play_rec.clear();
                self.main_play_r_rec.clear();
                if want_rec {
                    self.main_play_rec.extend_from_slice(&self.voice_play);
                }
            } else if self.mutes_analog_audio() {
                // Silenced in place rather than dropped: the block still has to
                // reach the mixer to keep the output paced.
                self.main_play.fill(0.0);
                self.main_play_r.fill(0.0);
                self.main_play_rec.fill(0.0);
                self.main_play_r_rec.fill(0.0);
            }
        }

        // Binaural audio, if the operator asked for it: one ear becomes two, with
        // the passband spread across them. Here rather than inside the receive
        // chain because this is where the demodulated path and the transceiver's
        // own audio have met, and because everything the decoders, the recorder
        // and the remote stream tap is upstream of it — turning this on changes
        // what the operator hears and nothing else.
        binaural_split(
            &mut self.binaural,
            &self.state.rx[0],
            out_rate,
            !self.state.sub_rx_enabled,
            &mut self.main_play,
            &mut self.bin_left,
            &mut self.main_play_r,
        );

        // Both taps come out of one borrow: `run`'s own return would keep the
        // chain mutably borrowed, leaving no way to ask for the recorder tap
        // as well.
        let (sub_audio, sub_rec): (Option<&[f32]>, Option<&[f32]>) =
            match (&mut self.sub, self.state.sub_rx_enabled) {
                (Some(sub), true) => {
                    // A silent sub (SPEC) degrades to mono rather than stalling.
                    let has_audio = sub.demod.is_some();
                    sub.run(iq, &self.state.rx[1], want_rec);
                    let sub: &RxChain = sub;
                    (has_audio.then(|| sub.out_audio()), has_audio.then(|| sub.take_rec_audio().0))
                }
                _ => (None, None),
            };

        // Both want the right ear. The sub receiver wins: switching it on is an
        // explicit request for that ear, whereas WFM stereo is automatic — so
        // the broadcast falls back to its mono sum until the sub is switched off.
        let right: Option<&[f32]> = match sub_audio {
            Some(a) => Some(a),
            None if !self.main_play_r.is_empty() => Some(&self.main_play_r),
            None => None,
        };
        // The recorder's right channel follows the same priority, and takes the
        // sub's own pre-volume tap rather than its speaker audio — otherwise a
        // stereo recording would pair a pre-volume left with a post-volume
        // right, and turning the sub down would quietly thin the archive.
        let rec_right: Option<&[f32]> = match sub_rec {
            Some(a) => Some(a),
            None if !self.main_play_r_rec.is_empty() => Some(&self.main_play_r_rec),
            None => None,
        };
        if let Some(mixer) = self.mixer.as_mut() {
            mixer.push(&self.main_play, right, &self.main_play_rec, rec_right);
        }
        // Feed the high-resolution channel spectrum from the DDC output.
        if let (Some(ca), Some(main)) = (self.channel_analyzer.as_mut(), self.main.as_ref()) {
            ca.process(main.channel_iq());
        }
        // Feed the CW skimmer from a dedicated wideband decimation of the raw IQ.
        // `Ddc::process` appends, so clear the scratch buffer each block.
        if let Some(ddc) = self.skim_ddc.as_mut() {
            self.skim_buf.clear();
            ddc.process(iq, &mut self.skim_buf);
            if let Some(sk) = self.skimmer.as_ref() {
                sk.on_rx_iq(&self.skim_buf);
            }
        }
        // ...and the ISM decoder from its own, wider decimation onto the 868 MHz
        // channel plan.
        if let Some(ddc) = self.ism_ddc.as_mut() {
            self.ism_buf.clear();
            ddc.process(iq, &mut self.ism_buf);
            if let Some(d) = self.ism.as_ref() {
                d.on_rx_iq(&self.ism_buf);
            }
        }
        // ...and the ADS-B decoder, from the widest window of the lot. On the
        // commonest receiver for this — an RTL-SDR at its default 2.4 Msps —
        // that decimation is by one and the chain is a mixer, because a
        // megabit-a-second waveform has no slack to give away.
        if let Some(ddc) = self.adsb_ddc.as_mut() {
            self.adsb_buf.clear();
            ddc.process(iq, &mut self.adsb_buf);
            if let Some(d) = self.adsb.as_ref() {
                d.on_rx_iq(&self.adsb_buf);
            }
        }
        // ...and the VDL2 lane, from a third of a megahertz around 136.8 MHz.
        // The fourteen channels are split out inside the worker rather than here:
        // one downconverter per channel is the worker's business, and the engine
        // only has to place the window they all come off.
        if let Some(ddc) = self.vdl2_ddc.as_mut() {
            self.vdl2_buf.clear();
            ddc.process(iq, &mut self.vdl2_buf);
            if let Some(d) = self.vdl2.as_ref() {
                d.on_rx_iq(&self.vdl2_buf);
            }
        }
        // ...and the AIS lane, from 150 kHz around 162.000 MHz. The two
        // channels are split out inside the worker, as VDL2's seven are.
        if let Some(ddc) = self.ais_ddc.as_mut() {
            self.ais_buf.clear();
            ddc.process(iq, &mut self.ais_buf);
            if let Some(d) = self.ais.as_ref() {
                d.on_rx_iq(&self.ais_buf);
            }
        }
        // ...and the QO-100 beacon decoder from its own fixed downconversion
        // onto the beacon frequency.
        if let Some(ddc) = self.qo100_ddc.as_mut() {
            self.qo100_buf.clear();
            ddc.process(iq, &mut self.qo100_buf);
            if let Some(c) = self.qo100.as_ref() {
                c.on_rx_iq(&self.qo100_buf);
            }
        }
        // Feed TCI clients: the same clean tap the digital decoders use (so
        // muting or turning down sdroxide can't silence somebody's decoder),
        // resampled to the 48 kHz TCI mandates.
        if let (Some(srv), Some(main)) = (self.tci_srv.as_ref(), self.main.as_ref()) {
            if main.tap_enabled && srv.wants_audio() {
                let in_rate = main.audio_rate();
                if (in_rate - self.tci_aud_in_rate).abs() > 0.01 {
                    self.tci_aud_in_rate = in_rate;
                    self.tci_aud_rs = MonoResampler::new(in_rate, 48_000.0);
                }
                self.tci_aud_buf.clear();
                match self.tci_aud_rs.as_mut() {
                    Some(r) => r.push(&main.tap_out, &mut self.tci_aud_buf),
                    None => self.tci_aud_buf.extend_from_slice(&main.tap_out),
                }
                srv.on_rx_audio(&self.tci_aud_buf);
            }
        }
        // ...and their wideband IQ, from its own decimation at the rate they
        // asked for (mirroring the skimmer window above).
        if let Some(ddc) = self.tci_iq_ddc.as_mut() {
            self.tci_iq_buf.clear();
            ddc.process(iq, &mut self.tci_iq_buf);
            if let Some(srv) = self.tci_srv.as_ref() {
                self.tci_iq_ilv.clear();
                for c in &self.tci_iq_buf {
                    self.tci_iq_ilv.push(c.re);
                    self.tci_iq_ilv.push(c.im);
                }
                srv.on_rx_iq(&self.tci_iq_ilv, ddc.out_rate() as u32);
            }
        }
    }

    /// What the zoom lane should be, if the display has anything to gain from
    /// one: the centre of the window the client asked for, and the
    /// power-of-two decimation whose output still covers it with margin.
    ///
    /// `None` where there is nothing to gain — no viewport at all (the operator
    /// is looking at the whole window, which is what the device-wide analyser
    /// *is*), a lane that already owns the frame (audio mode, or a digital
    /// mode's channel analyser), or a device-wide FFT that still has a bin for
    /// every column of the display.
    fn wanted_zoom(&self) -> Option<(f64, u32)> {
        if self.audio_mode || self.channel_analyzer.is_some() {
            return None;
        }
        let full = self.state.sample_rate;
        let (lo, hi) = self.cfg.viewport?;
        let span = hi - lo;
        if !span.is_finite() || span <= 0.0 || full <= 0.0 || span >= full {
            return None;
        }
        // One device-wide bin per column of the emitted frame. Below that the
        // pooling in `SpectrumAnalyzer::make_frame` has fewer measurements than
        // it has columns to fill and the trace stair-steps: an RX-888 streaming
        // 8.1 MHz through a 32768-point FFT is 247 Hz a bin, so a 68 kHz window
        // on screen is drawn from 275 numbers.
        //
        // Hysteresis: a lane already up is held until the device-wide analyser
        // has comfortably enough bins again. The two draw the same signal at
        // different bin widths, so they put the noise floor at different levels
        // — a zoom parked on the threshold would otherwise flip the picture
        // between them every time the client resent its window.
        let need = self.cfg.bins() as f64 * if self.zoom.is_some() { 1.5 } else { 1.0 };
        if self.analyzer.fft_size() as f64 * span / full >= need {
            return None;
        }
        // The narrowest output on the ladder that still spans the viewport with
        // room for the decimator's skirt. Shared with the client, which works
        // out the same answer to decide how large a device-wide FFT is worth
        // asking for — see `sdroxide_types::panadapter_fft_ceiling`.
        let decim = sdroxide_types::zoom_lane_decimation(full, span);
        (decim > 1).then_some(((lo + hi) / 2.0, decim))
    }

    /// Keep the zoom lane in step with the window the display is asking for.
    ///
    /// Synced here rather than at each of the half-dozen places a viewport, a
    /// centre or a rate can move: it is a handful of comparisons when nothing
    /// has changed, and a lane that quietly stopped matching its window would
    /// show the operator a picture of somewhere else.
    ///
    /// Separate from feeding it samples because feeding is forked: choosing the
    /// lane needs the whole engine, and running it needs only the lane. Nothing
    /// inside a block can change the answer, so it is settled once at the top of
    /// [`Engine::feed_panadapter`] and the two analysers then run side by side.
    fn sync_zoom(&mut self) {
        let in_rate = self.state.sample_rate;
        // The lane mixes down by an offset from the front end's centre, so the
        // centre it is pointed with has to be the one the *samples* carry —
        // otherwise, on a source whose stream lags its retunes, the lane lands
        // beside the window it labels itself with. Same reasoning as
        // [`Engine::display_center_hz`], which labels the frame.
        let dev_center = self.display_center_hz();
        match self.wanted_zoom() {
            None => {
                self.zoom = None;
                return;
            }
            Some(want) => match self.zoom.as_mut() {
                // The same lane: the window, the front end, or both may have
                // moved under it, and re-pointing is all either needs.
                Some(z) if z.serves(want, in_rate) => z.aim(want.0, dev_center),
                _ => {
                    let fft = zoom_lane_fft(self.cfg.bins());
                    // The window the client is actually looking at, which is
                    // what decides how much signal one transform may cover —
                    // see [`analysis_window`]. `wanted_zoom` only returns a
                    // lane where there is a viewport, so this always has one.
                    let view_span = self
                        .cfg
                        .viewport
                        .map(|(lo, hi)| hi - lo)
                        .unwrap_or(in_rate / f64::from(want.1));
                    let mut lane = ZoomLane::new(
                        in_rate,
                        want.1,
                        want.0,
                        dev_center,
                        self.cfg.avg_tc,
                        fft,
                        view_span,
                        f64::from(self.cfg.rows()),
                    );
                    // Start it on the device-wide analyser's picture of the
                    // same window, so the zoom step that builds it does not
                    // black the waterfall out for as long as the lane takes to
                    // fill — a quarter of a second at the rate a deep zoom runs
                    // at. See [`SpectrumAnalyzer::seed_from`].
                    lane.analyzer.seed_from(
                        &self.analyzer,
                        span_fraction(dev_center, in_rate, lane.center_hz, lane.rate_hz),
                    );
                    debug!(
                        center = lane.center_hz,
                        rate = lane.rate_hz,
                        decim = lane.decim,
                        fft,
                        "panadapter zoom lane built"
                    );
                    self.zoom = Some(lane);
                }
            },
        }
    }

    /// Demod-audio (CAT rig) RX: the source hands us already-demodulated real
    /// audio (packed in the I component). No DDC/demod — FFT it for the narrow
    /// panadapter, play it to the speakers, and feed the digital decoders.
    fn run_audio_mode(&mut self, iq: &[Complex32]) {
        self.audio_re.clear();
        self.audio_re.extend(iq.iter().map(|c| c.re));

        // Panadapter (packed-real FFT — see make_spectrum_frame). No sample
        // clocking here: a demod-audio lane runs at tens of kilohertz, so a
        // block already carries more time than a row does and there is nothing
        // finer to divide.
        self.row_sample_clock = false;
        self.analyzer.process(iq);

        self.play_rx_audio(self.radio_fs);
        // A demod-audio rig has one receiver and no sub, so the second ear is
        // free — and the binaural widener is the one thing that fills it. This
        // is the front end a great many operators are listening on, so it gets
        // the same treatment the demodulated path does.
        self.main_play_r.clear();
        binaural_split(
            &mut self.binaural,
            &self.state.rx[0],
            self.audio_out_rate,
            true,
            &mut self.audio_play,
            &mut self.bin_left,
            &mut self.main_play_r,
        );
        let right = (!self.main_play_r.is_empty()).then_some(&self.main_play_r[..]);
        if let Some(mixer) = self.mixer.as_mut() {
            mixer.push(&self.audio_play, right, &self.audio_play_rec, None);
        }
    }

    /// The receive stage for audio that arrives *as* audio rather than being
    /// demodulated here: level meter, digital-mode tap, auto-notch, noise
    /// reduction, then the speaker and recorder paths, from whatever is already
    /// in `audio_re` at `in_rate`.
    ///
    /// Two arrangements have such a stream, and they share every step of it.
    /// For a demod-audio rig it *is* the receiver, and `in_rate` is the front
    /// end's own rate. For a radio with another radio attached as its
    /// panadapter and [`sdroxide_types::PanadapterAudio::Transceiver`] chosen,
    /// the picture comes from the attached receiver's I/Q and this comes from
    /// the transceiver's sound card — a different clock, hence the parameter.
    fn play_rx_audio(&mut self, in_rate: f64) {
        // The only level reading there is on audio that nothing here
        // demodulated: no DDC and no demodulator ran, so nothing else measured
        // anything. Same one-pole as the real S-meter's, so the scanner's
        // threshold behaves consistently across front ends even though the two
        // are not the same quantity — this one is the rig's audio, after its
        // own AGC and squelch.
        if !self.audio_re.is_empty() {
            let p: f32 =
                self.audio_re.iter().map(|s| s * s).sum::<f32>() / self.audio_re.len() as f32;
            self.audio_level += 0.3 * (p - self.audio_level);
        }

        // FT8/FT4 run directly on the radio audio (before NR, so the decoder
        // always sees the raw signal).
        if let Some(digi) = self.digi.as_mut() {
            digi.on_rx_audio(&self.audio_re);
        }

        // Auto-notch (constant tones) then spectral noise reduction.
        let notch_on = self.state.rx[0].auto_notch;
        if self.audio_notch_on != notch_on {
            if notch_on {
                self.audio_notch.reset();
            }
            self.audio_notch_on = notch_on;
        }
        if self.audio_notch_on {
            self.audio_notch.process(&mut self.audio_re);
        }
        let nr_level = self.state.rx[0].noise_reduction;
        if self.audio_nr_level != nr_level {
            let prev = self.audio_nr_level;
            self.audio_nr_level = nr_level;
            let switched = prev.engine() != nr_level.engine();
            match nr_level.engine() {
                Some(NrEngine::Rnn) => {
                    if switched {
                        self.audio_nnr.reset();
                    }
                    self.audio_nnr.set_mix(nr_level.rnn_mix());
                }
                Some(NrEngine::DeepFilter) => {
                    if switched {
                        ensure_dfnr(&mut self.audio_dfnr, &mut self.audio_dfnr_failed);
                        if let Some(df) = self.audio_dfnr.as_mut() {
                            df.reset();
                        }
                    }
                    if let Some(df) = self.audio_dfnr.as_mut() {
                        df.set_atten_lim_db(nr_level.df_atten_db());
                    }
                }
                Some(NrEngine::SpecBleach) => {
                    if switched {
                        self.audio_sbnr.reset();
                    }
                    let (db, whiten) = nr_level.spec_params();
                    self.audio_sbnr.set_params(db, whiten);
                }
                Some(NrEngine::Spectral) => {
                    if switched {
                        self.audio_nr.reset();
                    }
                    let (over, floor) = nr_level.params();
                    self.audio_nr.set_params(over, floor);
                }
                None => {}
            }
        }
        if self.audio_nr_level.is_on() {
            // Per block, not on the level change: the input rate is reassigned
            // when the interface is reopened, and the rate-aware engines would
            // otherwise keep resampling for the rate the old device had.
            let fs = in_rate;
            match self.audio_nr_level.engine() {
                Some(NrEngine::Rnn) => {
                    self.audio_nnr.set_rate(fs);
                    self.audio_nnr.process(&mut self.audio_re);
                }
                Some(NrEngine::DeepFilter) => match self.audio_dfnr.as_mut() {
                    Some(df) => {
                        df.set_rate(fs);
                        df.process(&mut self.audio_re);
                    }
                    None => {
                        self.audio_nnr.set_rate(fs);
                        self.audio_nnr.process(&mut self.audio_re);
                    }
                },
                Some(NrEngine::SpecBleach) => {
                    self.audio_sbnr.set_rate(fs);
                    self.audio_sbnr.process(&mut self.audio_re);
                }
                Some(NrEngine::Spectral) => self.audio_nr.process(&mut self.audio_re),
                None => {}
            }
            // Suppression lowers the level; boost it back up per NR strength.
            let g = self.audio_nr_level.makeup_gain();
            for s in &mut self.audio_re {
                *s = (*s * g).clamp(-1.0, 1.0);
            }
        }

        // Speaker path: resample in_rate → out_rate, apply volume/mute. Built
        // on demand rather than at construction because either end can move
        // under us — a reopened interface changes the first, an audio-device
        // swap the second — and a resampler left on the old pair retunes the
        // pitch of everything that follows.
        if (in_rate - self.audio_rs_in_rate).abs() > 0.01
            || (self.audio_out_rate - self.audio_rs_out_rate).abs() > 0.01
        {
            self.audio_rs_in_rate = in_rate;
            self.audio_rs_out_rate = self.audio_out_rate;
            self.audio_resampler = MonoResampler::new(in_rate, self.audio_out_rate);
        }
        let rx0 = &self.state.rx[0];
        let vol = if rx0.muted { 0.0 } else { rx0.volume };
        self.audio_play.clear();
        match self.audio_resampler.as_mut() {
            Some(rs) => rs.push(&self.audio_re, &mut self.audio_play),
            None => self.audio_play.extend_from_slice(&self.audio_re),
        }
        // Recorder tap: same signal, without the volume/mute scaling below —
        // see `RxChain::rec_buf` for why, and for why it is skipped entirely
        // when nothing is recording.
        let want_rec = self.recorder.is_some();
        self.audio_play_rec.clear();
        if want_rec {
            self.audio_play_rec.extend_from_slice(&self.audio_play);
        }
        if vol != 1.0 {
            for s in self.audio_play.iter_mut() {
                *s *= vol;
            }
        }
        // A monitored voice-keyer message takes the speakers; otherwise digital
        // voice replaces the rig's audio with what it decoded from it.
        let block = self.audio_play.len();
        if self.take_preview_audio(self.audio_out_rate, block) {
            self.audio_play.clear();
            self.audio_play.extend_from_slice(&self.voice_prev_out);
            self.audio_play_rec.clear();
            if want_rec {
                self.audio_play_rec.extend_from_slice(&self.voice_prev_out);
            }
        } else if self.take_voice_audio(self.audio_out_rate) {
            self.audio_play.clear();
            self.audio_play.extend(self.voice_play.iter().map(|s| s * vol));
            self.audio_play_rec.clear();
            if want_rec {
                self.audio_play_rec.extend_from_slice(&self.voice_play);
            }
        } else if self.mutes_analog_audio() {
            self.audio_play.fill(0.0);
            self.audio_play_rec.fill(0.0);
        }
    }

    /// True when the active digital-voice mode wants the demodulated audio
    /// silenced instead of passed through — asked only after
    /// [`Engine::take_voice_audio`] declined, so decoded speech still plays.
    fn mutes_analog_audio(&self) -> bool {
        self.digi.as_ref().is_some_and(|d| d.mutes_analog_audio())
    }

    /// Pull decoded speech from a digital-voice mode into `voice_play`, at
    /// `out_rate`.
    ///
    /// Returns false when no such mode is active or it has nothing to play, in
    /// which case the caller keeps the demodulated audio.
    fn take_voice_audio(&mut self, out_rate: f64) -> bool {
        let Some(digi) = self.digi.as_mut() else { return false };
        self.voice_buf.clear();
        if !digi.rx_audio_out(&mut self.voice_buf) {
            return false;
        }
        // The mode produces 48 kHz; the speaker may want something else.
        if (out_rate - self.voice_rs_out_rate).abs() > 0.01 {
            self.voice_rs_out_rate = out_rate;
            self.voice_rs = MonoResampler::new(48_000.0, out_rate);
        }
        self.voice_play.clear();
        match self.voice_rs.as_mut() {
            Some(r) => r.push(&self.voice_buf, &mut self.voice_play),
            None => self.voice_play.extend_from_slice(&self.voice_buf),
        }
        true
    }

    /// A change the CAT rig reported (operator moved the dial/mode on the
    /// radio). Reflect it in state WITHOUT re-commanding the rig — that would
    /// feed back through the serial poll.
    fn apply_control(&mut self, update: ControlUpdate) {
        match update {
            ControlUpdate::Freq(hz) => {
                // The rig reports its VFO, which in CW on a radio that keys its
                // own transmitter is a sidetone above our dial — the offset this
                // end put there (`rig_cw_offset_hz`). Taking it back out is what
                // stops the readout climbing by one pitch per poll.
                let hz = hz - self.rig_cw_offset_hz();
                match self.state.active_vfo {
                    Vfo::A => self.state.vfo_a_hz = hz,
                    Vfo::B => self.state.vfo_b_hz = hz,
                }
                self.state.band = Band::containing(hz);
                self.adopt_source_center();
                // A dial the rig moved can land outside what the front end is
                // receiving, and then nothing is demodulated at all. On a rig
                // that carries its own I/Q (TCI) it never does — the window
                // moved with the dial, and `adopt_source_center` above has just
                // taken the new centre — so this is a no-op there. Where the
                // two are *different radios*, though, turning the transceiver's
                // knob moves the dial and leaves the receiver exactly where it
                // was, and something has to bring it along.
                self.keep_vfo_in_span();
                // And then put the receiver on the new dial, which is the part
                // neither of the two calls above necessarily does. On a
                // panadapter pairing — a transceiver on CAT beside an SDR that
                // supplies the spectrum — the window deliberately does *not*
                // follow the rig's knob, so `adopt_source_center` finds nothing
                // to adopt, and a move inside the span gives
                // `keep_vfo_in_span` nothing to do either. Without this the
                // readout and the passband marker follow the rig while the DDC
                // stays on the frequency before the move: flrig retunes from
                // WSJT-X, the picture agrees and the audio does not (issue
                // #206). Every operator-initiated tune already pairs
                // `follow_dial` with this; the rig-initiated one was the odd
                // path out.
                //
                // Not in audio mode, where there is no DDC and the rig's own
                // dial is the tuning: `update_tuning` would command the
                // frequency straight back at the radio that just reported it.
                if !self.audio_mode {
                    self.update_tuning();
                }
                self.update_display_center();
                let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
            }
            // The hardware centre moved out from under this engine — a sibling
            // stream on a shared-LO device retuned it. Adopt it: the span
            // simply is somewhere else now, the receivers are clamped into
            // it, and nothing is commanded back at the hardware — answering
            // an adoption with a correction is how two engines sharing one LO
            // would chase each other forever.
            ControlUpdate::Center(hz) => {
                if (hz - self.state.center_hz).abs() < 0.5 {
                    return;
                }
                self.state.center_hz = hz;
                let (lo, hi) = self.passband();
                let vfo = match self.state.active_vfo {
                    Vfo::A => {
                        self.state.vfo_a_hz = self.state.vfo_a_hz.clamp(lo, hi);
                        self.state.vfo_a_hz
                    }
                    Vfo::B => {
                        self.state.vfo_b_hz = self.state.vfo_b_hz.clamp(lo, hi);
                        self.state.vfo_b_hz
                    }
                };
                self.state.band = Band::containing(vfo);
                // Where the hardware demonstrably is, is by definition a
                // frequency it took.
                self.good_vfo_hz = vfo;
                self.reseat_sub_freq();
                // The windows that are placed against the hardware centre have
                // to be re-placed against the new one, exactly as
                // `adopt_source_center` and `set_center_hz` do — this arm is the
                // third way the centre can move and was the one that did not.
                //
                // The symptom was specific: an RX-888 crossing into its VHF path
                // re-parks its tuner and reports the move through here, so the
                // ISM decoder kept a window on wherever the receiver used to be
                // and reported "no ISM channel is inside the receiver's window"
                // while the dial plainly read 868.88 MHz. Switching the decoder
                // off and on rebuilt it and it worked, which is the tell: the
                // window was stale, not wrong.
                self.sync_skim_window();
                self.sync_ism_window();
                self.sync_adsb_window();
                self.sync_vdl2_window();
                self.sync_ais_window();
                self.sync_qo100_window();
                self.update_tuning();
                let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
            }
            // Power levels the rig reports (the operator moved them on the rig,
            // or these are the levels it came up with). Adopted, not overridden:
            // the rig's own setting is what the operator asked for.
            ControlUpdate::TxDrive(frac) => {
                let frac = frac.clamp(0.0, 1.0);
                if self.state.tx.drive != frac {
                    self.state.tx.drive = frac;
                    let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
                }
            }
            ControlUpdate::TuneDrive(frac) => {
                let frac = frac.clamp(0.0, 1.0);
                if self.state.tx.tune_drive != frac {
                    self.state.tx.tune_drive = frac;
                    let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
                }
            }
            // The squelch the radio is set to, read when its control link
            // opened. Adopted rather than overridden, exactly as the drive
            // above is — the operator set it at the rig, and a remembered level
            // imposed on top would move a gate they can hear.
            ControlUpdate::Squelch(frac) => {
                let frac = frac.clamp(0.0, 1.0);
                if self.state.rig_squelch != frac {
                    self.state.rig_squelch = frac;
                    let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
                }
            }
            // Which socket the radio is receiving on, as the radio itself
            // reports it when its control link opens. Adopted for the same
            // reason the drive above is: the operator set it on the rig, the
            // rig is where it survived the power cycle, and nothing here knew
            // it until it was asked. Remembered as well as shown, so the next
            // open puts the front end back on the port the radio was actually
            // on rather than on one a session file from another day remembers.
            //
            // Ignored, though, where this end has already asserted a port —
            // which [`Engine::restore_antennas`] does at every open, from the
            // command line or the session. There the radio has been *told*, and
            // a report that disagrees is its answer to a read that crossed that
            // command on the wire; adopting it would show a port the radio just
            // left. A preference the front end does not have is no assertion:
            // it belonged to some other interface, nothing was sent, and what
            // the radio says is then the only account there is.
            ControlUpdate::Antenna(name) => {
                let asserted =
                    self.want_antenna.0.as_ref().is_some_and(|w| self.caps.antennas_rx.contains(w));
                if asserted {
                    return;
                }
                if self.state.antenna_rx != name {
                    self.state.antenna_rx = name.to_string();
                    let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
                }
                self.want_antenna.0 = Some(name.to_string());
            }
            ControlUpdate::Ptt(closed) => self.apply_hw_ptt(closed),
            // An over the operator started at the radio. Recorded, never
            // answered: see [`ControlUpdate::RigTx`] for why keying along with
            // it would talk over the person holding the microphone.
            ControlUpdate::RigTx(on) => self.adopt_rig_tx(on, "its own control"),
            ControlUpdate::Mode(m) => {
                let cur = self.state.rx[0].mode;
                // Against the mode we *command*, not the one on screen: SSTV is
                // commanded as plain LSB on 160/80/40 m, CW keyed as audio
                // (MCW) is commanded as the digi sideband, and a rig echoing
                // that back has done exactly as it was told.
                let cw_mcw = cur == Mode::Cw && self.source.cw_audio_keyed();
                let same_class =
                    expected_rig_class(self.control_mode(), cw_mcw, self.state.rx_freq_hz())
                        == rig_mode_class(m);
                if cur.is_digital() || cw_mcw {
                    // Digital modes (FT8/FT4/PSK/RTTY/SSTV) — and CW-as-MCW —
                    // are app-driven and ride the sideband the app chose. Never
                    // leave the mode because of a rig report; if the rig
                    // drifted onto another sideband (e.g. per-band mode memory
                    // switching to LSB on 40/80 m), command it straight back.
                    // Re-commanding just echoes the same sideband, which is
                    // same-class and ignored, so this settles (no feedback).
                    if !same_class {
                        let _ = self.source.set_control_mode(self.control_mode());
                    }
                    return;
                }
                // Non-digital: follow the operator's rig, but only when the
                // underlying rig class actually changed (ignore USB↔DIGU echoes).
                if !same_class {
                    let r = &mut self.state.rx[0];
                    r.mode = m;
                    (r.filter_lo, r.filter_hi) = m.default_filter();
                    let snapshot = *r;
                    // Rebuild the demodulator for the new mode. Sideband is
                    // carried entirely in the sign of the filter edges, so
                    // without this the internal demod (e.g. TCI wideband-IQ RX)
                    // keeps the old sideband while state/UI already show the new
                    // mode — the LSB-shows-but-demodulates-USB desync.
                    if let Some(c) = self.chain_mut(RxId::Main) {
                        c.build_for_mode(&snapshot);
                    }
                    self.update_display_center(); // sideband flip changes the window
                    self.sync_digi_mode();
                    // Into or out of CW the dial and the rig's VFO stop being
                    // the same number — see `reseat_dial_for_cw`, which moves
                    // ours rather than the radio's.
                    self.reseat_dial_for_cw();
                    let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
                }
            }
        }
    }

    /// In audio mode, keep `state.center_hz`/`sample_rate` describing the
    /// displayed RF window (dial ± bw/2, width = bw) so the panadapter axis and
    /// zoom clamp match the audio-band spectrum.
    ///
    /// The window hangs off the *receive* frequency, not the VFO: the rig hands
    /// us audio from wherever its dial is, and with RIT on that dial is the VFO
    /// plus the offset (see [`Self::update_tuning`]). Anchoring on the VFO would
    /// mislabel every bin by the RIT offset.
    /// The rate `analyzer` is actually running at.
    ///
    /// Not [`RadioState::sample_rate`] in audio mode: there the analyser sees
    /// the sound card's own stream, while `sample_rate` describes the
    /// *displayed* window — the audio band, or the radio's scope span where
    /// that is the main panadapter. Rebuilding it against the displayed width
    /// gave the averaging time constant the wrong clock.
    fn analyzer_rate(&self) -> f64 {
        if self.audio_mode { self.radio_fs } else { self.state.sample_rate }
    }

    /// The window the device-wide analyser is drawn in, where that is narrower
    /// than what it is fed — see [`build_analyzer`].
    ///
    /// Audio mode only. There the lane runs at the sound card's rate and the
    /// picture is the rig's passband mapped onto RF, so a transform with a true
    /// bin per column of a wide panel covers more than a second of signal; on
    /// an I/Q front end the same lane draws the whole span and every bin of it
    /// is wanted.
    fn analyzer_view_span(&self) -> Option<f64> {
        if !self.audio_mode {
            return None;
        }
        let view = self
            .cfg
            .viewport
            .map(|(lo, hi)| hi - lo)
            .filter(|s| s.is_finite() && *s > 0.0)
            .unwrap_or(self.audio_bw);
        Some(view.min(self.audio_bw).max(1.0))
    }

    fn update_display_center(&mut self) {
        if !self.audio_mode {
            return;
        }
        // The radio's own scope decides both ends of the window when it is the
        // main lane, and `state` has to say so: the client's zoom clamp and the
        // sub-receiver limits are built on these two numbers, and left
        // describing the audio band they would pin the view to a few kHz of a
        // panadapter that is now hundreds wide.
        if let Some((center, span)) = self.scope_main_window() {
            self.state.center_hz = center;
            self.state.sample_rate = span;
            return;
        }
        let dial = self.state.rx_freq_hz();
        let lsb = self.state.rx[0].mode.is_lower_sideband_at(dial);
        self.state.center_hz =
            if lsb { dial - self.audio_bw / 2.0 } else { dial + self.audio_bw / 2.0 };
        self.state.sample_rate = self.audio_bw;
    }

    /// Follow the front end's centre after it reported a dial move of its own.
    ///
    /// On a rig that *is* the front end — a transceiver feeding its I/Q output
    /// into a sound card, an Icom sending its 12 kHz IF — the dial and the
    /// centre of the baseband we capture are the same synthesiser. Turning the
    /// knob moves the spectrum as surely as it moves the readout, so the window
    /// has to move with it, or it labels new content with the old axis: the
    /// waterfall shows one frequency while the demodulator, tuned to the offset
    /// between a stale centre and the new VFO, hears another.
    ///
    /// A front end whose stream centre is independent of the rig's VFO — TCI's
    /// wideband IQ, a Flex panadapter, an SDR with a CAT rig alongside it —
    /// reports the same centre it had before, so this costs one comparison and
    /// changes nothing. Nothing is commanded back at the hardware either: the
    /// front end is where it says it is, and answering an adoption with a
    /// correction is how the operator's knob and our retune would fight (the
    /// same rule as [`ControlUpdate::Center`]).
    fn adopt_source_center(&mut self) {
        if self.audio_mode {
            return; // no IQ window here — `update_display_center` owns the axis
        }
        let center = self.source.center_hz();
        if (center - self.state.center_hz).abs() < 0.5 {
            return;
        }
        self.state.center_hz = center;
        // A move we did not command, learned about from a report that arrives
        // *after* the rig made it — so the stream is already there, or as near
        // as anything here can tell. Anything still queued in the trail was
        // superseded by it. Compensating a move nobody timed would be guessing
        // in the other direction.
        self.center_trail.clear();
        self.stream_center_hz = center;
        // Where the hardware demonstrably is, is by definition a frequency it
        // took — this is the dial the rig itself just reported.
        self.good_vfo_hz = self.state.active_freq_hz();
        // The skim window is placed against the hardware centre, so it has to be
        // re-placed against the new one — and re-labelled and cleared if it
        // really moved, so no track straddles the old and new axes (as
        // `retune_named` does for a retune we asked for).
        self.sync_skim_window();
        // The ISM window does *not* follow the hardware centre: its channels are
        // at fixed frequencies, so it stays on them for as long as the new span
        // still reaches, and only slides when it has to.
        self.sync_ism_window();
        // The ADS-B window *does* follow the hardware centre, being one fixed
        // target rather than a plan of them: 1090 MHz is where it has to be, and
        // the only question is whether the new span still reaches it.
        self.sync_adsb_window();
        // The VDL2 window is a plan of channels like the ISM one, but unlike it
        // the whole plan fits in a single window, so there is nothing to choose
        // between — it goes where the span can reach the group and follows the
        // hardware centre when it has to.
        self.sync_vdl2_window();
        // The AIS window is the same shape as the VDL2 one and smaller: two
        // channels 50 kHz apart, and a span that reaches only one of them still
        // hears every ship, at half its reporting rate.
        self.sync_ais_window();
        // The QO-100 window, unlike the ISM one, *does* follow the hardware
        // centre — its one target frequency never moves, so re-seating the
        // mixer is all a retune ever needs.
        self.sync_qo100_window();
        // Re-seat the DDCs on the new centre. Without this the main receiver
        // keeps the offset it had against the old one, which is exactly how a
        // rig-initiated retune ends up demodulating somewhere the readout does
        // not claim.
        self.update_tuning();
    }

    /// Hand a slot's decodes to PSK Reporter. Every station we can name and
    /// place is a reception report; free text and unresolved hashed callsigns
    /// name nobody, and our own callsign is not something we heard.
    fn psk_report_decodes(&self, decodes: &[sdroxide_types::Decode], dial_hz: f64) {
        let mode = self.digi.as_ref().map(|d| d.mode().label().to_string()).unwrap_or_default();
        let my_call = self.digi_config.my_call.trim();
        for d in decodes {
            let Some(call) = d.from.as_deref().filter(|c| !c.is_empty()) else { continue };
            if call.eq_ignore_ascii_case(my_call) {
                continue;
            }
            let freq = dial_hz + d.audio_hz as f64;
            if freq <= 0.0 {
                continue;
            }
            self.spots.psk_report(sdroxide_net::PskReport {
                call: call.to_string(),
                grid: d.grid.clone().unwrap_or_default(),
                freq_hz: freq as u32,
                snr_db: d.snr_db.clamp(-128, 127) as i8,
                mode: mode.clone(),
                when_utc: d.slot_utc.max(0) as u32,
            });
        }
    }

    /// Start, retarget or stop the WSJT-X UDP broadcast to match its config.
    fn sync_wsjtx(&mut self) {
        let want = self.wsjtx_cfg.enabled;
        let same = self
            .wsjtx
            .as_ref()
            .is_some_and(|w| w.addr() == self.wsjtx_cfg.addr() && w.id() == self.wsjtx_cfg.id);
        if want && same {
            return;
        }
        if let Some(w) = self.wsjtx.take() {
            w.close(); // tell clients to drop us before the socket goes
        }
        if !want {
            info!("WSJT-X UDP broadcast stopped");
            return;
        }
        match sdroxide_wsjtx::WsjtxUdp::start(&self.wsjtx_cfg) {
            Ok(w) => {
                w.heartbeat(env!("CARGO_PKG_VERSION"));
                w.clear(); // a fresh session starts with an empty decode window
                self.wsjtx_beat = Instant::now();
                self.wsjtx = Some(w);
            }
            Err(e) => {
                warn!("WSJT-X UDP broadcast: {e}");
                let _ = self.event_tx.send(RadioEvent::NetStatus(Some(format!("WSJT-X UDP: {e}"))));
            }
        }
    }

    /// Notice the dial has crossed into another band, and drop what only meant
    /// anything on the one it left.
    ///
    /// Polled here rather than hooked onto the places that move the dial:
    /// `state.band` is recomputed at more than a dozen of them — the band
    /// buttons, a memory recall, the scanner, a satellite lock, and
    /// [`Engine::apply_control`] for the knob on the radio's own front panel —
    /// and none of them is a band-change event. This is the one funnel every
    /// tick goes through whatever moved it, so a QSY made at the rig counts
    /// exactly as one made in the program.
    ///
    /// Band-level rather than a delta on the dial, matching what the clients
    /// themselves do: tuning about within a band is the same opening and
    /// invalidates nothing.
    ///
    /// WSPR is exempt. Its band hopping crosses an edge every couple of minutes
    /// on purpose, and a survey of several bands is what the mode is for. The
    /// band is still recorded, so leaving WSPR on another band is noticed once,
    /// there and then, rather than being missed.
    fn poll_band_change(&mut self) {
        let band = self.state.band;
        if std::mem::replace(&mut self.band_seen, band) == band {
            return;
        }
        // The antenna first, and outside the WSPR exemption below: a beam that
        // hears 2 m and a vertical that hears 40 are not a mode's business, and
        // a receiver left on the wrong socket hears nothing to decode.
        self.follow_band_antenna(band);
        if self.state.rx[0].mode.is_wspr() {
            return;
        }
        // A station marked to be worked carries the audio offset it was heard
        // at, and that offset is only a frequency at all against the dial it
        // was heard on. Left in the queue across a QSY it is not a stale row:
        // the sequencer takes the next one the moment it is free and calls on
        // exactly that offset — a transmission on the wrong frequency to
        // somebody who is not there. Inert in every mode without a queue.
        if let Some(d) = self.digi.as_mut() {
            d.queue_remove("");
        }
        // The clients' decode windows hold what was heard where the dial used
        // to be, and the protocol has one message for saying so. WSJT-X sends
        // it on this same occasion; without it a logger on the far end of the
        // broadcast keeps the mixed list our own window has just been taken
        // out of.
        if let Some(w) = &self.wsjtx {
            w.clear();
        }
    }

    /// Keep the broadcast alive: clients drop a station they stop hearing from.
    fn wsjtx_heartbeat(&mut self) {
        if self.wsjtx.is_some() && self.wsjtx_beat.elapsed() >= Duration::from_secs(15) {
            self.wsjtx_beat = Instant::now();
            if let Some(w) = &self.wsjtx {
                w.heartbeat(env!("CARGO_PKG_VERSION"));
            }
        }
    }

    /// Broadcast a slot's decodes to the WSJT-X clients.
    fn wsjtx_decodes(&self, decodes: &[sdroxide_types::Decode]) {
        let Some(w) = &self.wsjtx else { return };
        let mode = self.digi.as_ref().map(|d| d.mode().label().to_string()).unwrap_or_default();
        for d in decodes {
            w.decode(&sdroxide_wsjtx::msg::DecodeInfo {
                new: true,
                slot_utc: d.slot_utc,
                snr_db: d.snr_db as i32,
                dt: d.dt as f64,
                audio_hz: d.audio_hz.max(0.0) as u32,
                mode: mode.clone(),
                message: d.message.clone(),
            });
        }
    }

    /// Broadcast the station's state as WSJT-X reports it.
    fn wsjtx_status(&self, s: &sdroxide_types::DigiStatus) {
        let Some(w) = &self.wsjtx else { return };
        w.status(&sdroxide_wsjtx::msg::StatusInfo {
            dial_hz: self.state.rx_freq_hz().max(0.0) as u64,
            mode: s.mode.label().to_string(),
            dx_call: s.dx_call.clone().unwrap_or_default(),
            report: String::new(),
            // "Tx enabled" is WSJT-X's auto-sequencing switch: ours is on
            // whenever the QSO machine intends to key.
            tx_enabled: s.tx_next,
            transmitting: s.transmitting,
            decoding: false,
            rx_df_hz: s.audio_hz.max(0.0) as u32,
            tx_df_hz: s.audio_hz.max(0.0) as u32,
            de_call: s.config.my_call.clone(),
            de_grid: s.config.my_grid.clone(),
            dx_grid: s.dx_grid.clone().unwrap_or_default(),
            tx_watchdog: s.tx_watchdog,
            // JS8's period is a runtime setting rather than implied by the
            // mode, so it has to come from the status rather than a constant.
            tr_period_s: match s.mode {
                sdroxide_types::Mode::Ft4 => 7,
                // The WSJT-X UDP Status message carries the period as whole
                // seconds, so FT4's 7.5 goes out as 7 and FT2's 3.75 as 3.
                sdroxide_types::Mode::Ft2 => 3,
                sdroxide_types::Mode::Js8 => s.js8.as_ref().map_or(15, |j| j.speed.slot_s() as u32),
                _ => 15,
            },
            tx_message: s.tx_pending_msg.clone().unwrap_or_default(),
        });
    }

    /// Tick the FT8/FT4 controller and apply its actions (emit events, key/
    /// unkey PTT). Owned actions avoid a `&mut self.digi` / `&mut self` clash.
    /// Apply the transmit offset stored for the band the dial is on, when that
    /// band has changed since the last one applied.
    ///
    /// Called from the poll rather than hooked onto the places that assign
    /// `state.band`, of which there are four: two retune paths, a region change
    /// and a band-plan reload, the last two moving the band under a dial that
    /// never moved. One check at a known point covers all four and cannot be
    /// forgotten by a fifth.
    ///
    /// A band with nothing stored is left alone rather than reset to 1500. The
    /// operator is mid-session on a frequency they chose, and the memory has
    /// nothing better to offer than what is already there.
    ///
    /// So is a mode whose tone offset is a standard rather than a choice —
    /// RTTY's pair is 2125/2295 on every band, and restoring FT8's figure over
    /// it would leave mark and space wherever the last slot hunt happened to
    /// land. Nothing writes an entry for those modes either, so this only
    /// matters where a band already carries one from a slotted mode.
    fn follow_band_tx_offset(&mut self) {
        let band = self.state.band;
        if self.digi_tx_band == Some(band) {
            return;
        }
        self.digi_tx_band = Some(band);
        let Some(hz) = self.digi_config.tx_audio_hz.get(&band).copied() else { return };
        if let Some(d) = self.digi.as_mut().filter(|d| !d.mode().holds_standard_tones()) {
            // `restore_audio_hz`, not `set_audio_hz`: this is the one move that
            // goes through a hold. See the trait method.
            d.restore_audio_hz(hz);
        }
    }

    fn poll_digi(&mut self) {
        // A message the radio was keying itself has run its length: it is off
        // the air, so the station interlock goes back. Not while an ordinary
        // over is running, which holds the gate on its own account.
        if let Some(until) = self.cw_gate_until
            && Instant::now() >= until
        {
            self.cw_gate_until = None;
            if !self.tx_active {
                self.release_tx_gate();
            }
        }
        self.follow_band_tx_offset();
        let Some(digi) = self.digi.as_mut() else { return };
        let dial = self.state.rx_freq_hz();
        let actions = digi.poll(SystemTime::now(), dial);
        for a in actions {
            match a {
                DigiAction::Decodes(d) => {
                    self.psk_report_decodes(&d, dial);
                    self.wsjtx_decodes(&d);
                    let _ = self.event_tx.send(RadioEvent::Ft8Decodes(d));
                }
                DigiAction::Status(s) => {
                    self.wsjtx_status(&s);
                    let _ = self.event_tx.send(RadioEvent::Ft8Status(s));
                }
                DigiAction::QsoLogged(r) => {
                    // A contact logged with a serial we sent is the one thing
                    // that advances the contest count (issue #223) — not the
                    // transmission, which may be repeated, and not the operator,
                    // who would have to remember. `stx` is set only by a contest
                    // exchange, so this is exactly once per contest contact.
                    if let Some(sent) = r.stx {
                        let next = sdroxide_types::next_contest_serial(sent);
                        if self.digi_config.contest_serial != next {
                            self.digi_config.contest_serial = next;
                            self.push_contest_serial();
                        }
                    }
                    if let Some(w) = &self.wsjtx {
                        w.qso_logged(&r);
                    }
                    let _ = self.event_tx.send(RadioEvent::Ft8QsoLogged(r));
                }
                DigiAction::WsprSpots(spots) => {
                    self.spots.wspr_report(&spots, dial, self.digi_config.wspr_tx_percent);
                    let _ = self.event_tx.send(RadioEvent::WsprSpots(spots));
                }
                DigiAction::SetDial(hz) => self.wspr_hop(hz),
                DigiAction::RadeCallsign { call, snr_db, freq_hz } => {
                    // A RADE station identified itself in its End-of-Over
                    // frame: report hearing it. The reporter pairs the report
                    // with the frequency we already told it we are on, so
                    // `freq_hz` is only of interest to the log line.
                    debug!(%call, snr_db, freq_hz, "RADE callsign decoded");
                    self.spots.reporter_rx_report(call, snr_db.round().clamp(-128.0, 127.0) as i32);
                }
                DigiAction::KeyTx => {
                    // Key up via the normal PTT path so the safety rails apply.
                    self.digi_tx = true;
                    self.state.tx.ptt = true;
                    self.sync_tx_state();
                    // If the rails refused, drop the burst so the QSO reverts.
                    if !self.tx_active {
                        self.digi_tx = false;
                        self.state.tx.ptt = false;
                        if let Some(d) = self.digi.as_mut() {
                            d.abort_tx();
                        }
                    }
                    let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
                }
                DigiAction::UnkeyTx => {
                    self.digi_tx = false;
                    self.state.tx.ptt = false;
                    self.sync_tx_state();
                    let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
                }
                DigiAction::SstvLine { image_id, y, rgb } => {
                    let _ = self.event_tx.send(RadioEvent::SstvLine { image_id, y, rgb });
                }
                DigiAction::SstvImage { image_id, mode, w, h, rgb } => {
                    // Encode once: PNG for both the persistent store and the wire.
                    let png = encode_png(&rgb, w, h);
                    if let Some(png) = png.clone() {
                        // The picture goes out whole so the live pane resolves
                        // immediately; the gallery entry follows from the
                        // worker, carrying the name it was filed under.
                        if let Some(name) = save_sstv_rx(&png) {
                            self.gallery.entry(ImageKind::Sstv, name, None);
                        }
                        let _ =
                            self.event_tx.send(RadioEvent::SstvImage { image_id, mode, w, h, png });
                    }
                }
                DigiAction::SstvStatus(s) => {
                    let _ = self.event_tx.send(RadioEvent::SstvStatus(s));
                }
                DigiAction::WefaxLine { image_id, y, gray } => {
                    let _ = self.event_tx.send(RadioEvent::WefaxLine { image_id, y, gray });
                }
                DigiAction::WefaxImage { image_id, w, h, gray } => {
                    // Grayscale all the way to disk: a weather chart is line
                    // art in one channel, and tripling it to RGB would treble
                    // a two-megapixel PNG for nothing.
                    if let Some(png) = encode_png_gray(&gray, w, h) {
                        if let Some(name) = save_wefax_rx(&png, dial) {
                            self.gallery.entry(ImageKind::Wefax, name, None);
                        }
                        let _ = self.event_tx.send(RadioEvent::WefaxImage { image_id, w, h, png });
                    }
                }
                DigiAction::WefaxStatus(s) => {
                    let _ = self.event_tx.send(RadioEvent::WefaxStatus(s));
                }
                DigiAction::RifpRows { image_id, y, w, h, rows } => {
                    let _ = self.event_tx.send(RadioEvent::RifpRows { image_id, y, w, h, rows });
                }
                DigiAction::RifpImage { image_id, meta, w, h, rgb } => {
                    // Same store as SSTV: a received picture is a received
                    // picture, whichever mode carried it.
                    if let Some(png) = encode_png(&rgb, w, h) {
                        // The manifest goes with the entry: it is not in the
                        // PNG and there is nowhere on disk it survives, so this
                        // is the only chance the gallery has to record who sent
                        // the picture and how it was carried.
                        if let Some(name) = save_sstv_rx(&png) {
                            self.gallery.entry(ImageKind::Sstv, name, Some(meta.clone()));
                        }
                        let _ = self.event_tx.send(RadioEvent::RifpImage { image_id, meta, png });
                    }
                }
                DigiAction::RifpStatus(s) => {
                    let _ = self.event_tx.send(RadioEvent::RifpStatus(s));
                }
                DigiAction::DigiImage { w, h, rgb } => {
                    if let Some(png) = encode_png(&rgb, w, h) {
                        let _ = self.event_tx.send(RadioEvent::DigiImage { png });
                    }
                }
                // Forwarded verbatim — no encoding. A Hell column is fourteen
                // bytes and they arrive continuously; compressing them would
                // cost more than it saved.
                DigiAction::HellColumns { seq, rows, cols } => {
                    let _ = self.event_tx.send(RadioEvent::HellColumns { seq, rows, cols });
                }
                // CW the radio keys itself. No PTT and no transmit chain: the
                // rig switches to transmit for the length of the message on its
                // own, which is why nothing here touches `tx_active`.
                //
                // The station interlock still applies — a radio keying itself is
                // a radio on the air — but there is no key-up for the usual
                // rails to hang off, so it is claimed here and held for as long
                // as the message takes.
                DigiAction::SendCw { text, seconds } => {
                    if self.tx_gate.as_ref().is_some_and(|g| !g.try_acquire(self.instance)) {
                        let msg = match self.tx_gate.as_ref().and_then(|g| g.holder()) {
                            Some(id) => format!("radio {} is on the air", id + 1),
                            None => "another radio is on the air".to_string(),
                        };
                        warn!("CW refused: {msg}");
                        self.notice(&format!("transmit refused — {msg}"));
                        // Nothing of this over reached the air, so the panel
                        // must not show it as sent.
                        if let Some(d) = self.digi.as_mut() {
                            d.abort_tx();
                        }
                    } else {
                        self.cw_gate_until =
                            Some(Instant::now() + Duration::from_secs_f32(seconds.max(0.0)));
                        // Assert the drive first, exactly as key-down does for
                        // every other over. Nothing else can: this path never
                        // reaches `sync_tx_state`, so without it the operator's
                        // Drive slider would be the one control that does
                        // nothing in the one mode where it is the *only*
                        // control — the rig keys its own transmitter here and
                        // never looks at the audio we send it.
                        let d = self.tx_drive();
                        self.source.set_tx_drive(d as f64);
                        // And throw the external T/R switch, for the same
                        // reason and with the same lead. `tx_active` never
                        // becomes true on this path, so nothing else in the
                        // engine would ever key the relay for a message the rig
                        // is about to put on the air at full power.
                        self.lead_tr_switch();
                        self.source.send_cw(&text);
                    }
                }
                DigiAction::AbortCw => {
                    self.source.abort_cw();
                    self.cw_gate_until = None;
                    if !self.tx_active {
                        self.release_tx_gate();
                        // The message never finished, so the hold has to start
                        // now rather than at the length it was going to be.
                        // `poll_tr_switch` would get there on the next tick;
                        // this makes the receiver's return prompt instead.
                        if let Some(hub) = self.tr_switch.as_ref() {
                            hub.unkey(self.instance);
                        }
                    }
                }
            }
        }
    }

    /// Build the digital-mode engine for `mode`: the continuous keyboard
    /// controller for PSK/RTTY, else the slotted FT8/FT4 controller.
    fn make_digi(&mut self, mode: Mode, tap_rate: f64) -> Box<dyn DigiEngine> {
        if mode == Mode::Cw {
            // First, because CW is not `is_digital()` and nothing below would
            // catch it: it is an ordinary analog mode that happens to have a
            // decoder and a keyer bolted alongside. See `CwController`.
            //
            // Which way it sends is the radio's answer, not a setting of the
            // mode: a rig that keys itself from text is one whose sound card
            // would swallow a sidetone whole.
            Box::new(CwController::new(
                self.digi_config.clone(),
                tap_rate,
                self.source.cw_text_keying(),
            ))
        } else if mode.is_rade() {
            Box::new(RadeController::new(self.digi_config.clone(), tap_rate))
        } else if mode.is_sstv() {
            // Both SSTV modes, one controller: HF and VHF differ in the radio
            // underneath, not in the picture — the same reason the two packet
            // modes share theirs.
            Box::new(SstvController::new(mode, self.digi_config.clone(), tap_rate))
        } else if mode.is_wefax() {
            Box::new(WefaxController::new(self.digi_config.clone(), tap_rate))
        } else if mode == Mode::Navtex {
            // Ahead of the fall-through: NAVTEX has the shape of a keyboard
            // mode and none of its behaviour — no callsign, no transmitter, and
            // a framing of its own — and nothing further down would notice it
            // had been handed a maritime safety broadcast.
            Box::new(NavtexController::new(self.digi_config.clone(), tap_rate))
        } else if mode.is_rifp() {
            Box::new(RifpController::new(self.digi_config.clone(), tap_rate))
        } else if mode.is_aprs() {
            // Ahead of `is_packet`, which APRS is deliberately not a member
            // of: the two share a modem and nothing else. A controller picked
            // by the packet branch would decode the channel perfectly and show
            // it in a monitor pane with no map, no messages and no beacon.
            Box::new(AprsController::new(self.digi_config.clone(), tap_rate))
        } else if mode.is_packet() {
            // Both packet modes, one controller: HF and VHF differ in the radio
            // underneath, not in the link layer. Ahead of `is_text_modem` and
            // the fall-through for the usual reason — packet is neither, and
            // whichever caught it would decode nothing and say nothing.
            //
            // The engine makes the link port and keeps the handle, rather than
            // the controller handing one out: a mode change destroys the
            // controller, and the port's lifetime has to be something the
            // engine can reason about.
            let mut ctl = PacketController::new(mode, self.digi_config.clone(), tap_rate);
            let call = self.digi_config.packet_mycall.trim();
            match sdroxide_ax25::Addr::new(if call.is_empty() { "N0CALL" } else { call }) {
                Ok(me) => {
                    let (handle, endpoint) = sdroxide_ax25::port_pair(sdroxide_ax25::LinkConfig {
                        me,
                        paclen: self.digi_config.packet_paclen,
                        maxframe: self.digi_config.packet_maxframe,
                    });
                    ctl.attach_port(endpoint);
                    self.packet_port = Some(handle);
                }
                Err(e) => warn!("packet callsign: {e}"),
            }
            Box::new(ctl)
        } else if mode.is_rf_paint() {
            Box::new(RfPaintController::new(self.digi_config.clone(), tap_rate))
        } else if mode.is_fsq() {
            Box::new(FsqController::new(self.digi_config.clone(), tap_rate))
        } else if mode.is_hell() {
            // Ahead of `is_text_modem`, which Hell is deliberately not a member
            // of: it types like a keyboard mode but has nothing to decode.
            Box::new(HellController::new(self.digi_config.clone(), tap_rate))
        } else if mode.is_text_modem() {
            Box::new(TextModemController::new(mode, self.digi_config.clone(), tap_rate))
        } else if mode.is_js8() {
            // Ahead of the fall-through, which is FT8's: JS8 is slotted too, so
            // nothing further down would notice it had been handed the wrong
            // protocol. `make_digi_builds_a_js8_controller_for_js8` guards the
            // ordering.
            Box::new(Js8Controller::new(self.digi_config.clone(), tap_rate))
        } else if mode.is_wspr() {
            // Ahead of the fall-through for the same reason, and the symptom
            // would be quieter still: WSPR is 4-FSK in the same passband, so an
            // FT8 decoder handed its audio finds nothing and says nothing.
            Box::new(WsprController::new(self.digi_config.clone(), tap_rate))
        } else {
            Box::new(DigiController::new(mode, self.digi_config.clone(), tap_rate))
        }
    }

    /// Build the controller for `mode` and put the radio in step with it.
    ///
    /// Only CW has anything to assert: a rig that keys itself sends at *its*
    /// keyer's speed, so the panel's WPM means nothing on the air until the rig
    /// has been told it — including the speed carried over from the last
    /// session, which the operator never touches this time round.
    fn start_digi(&mut self, mode: Mode, tap_rate: f64) {
        // The old controller owns the far end of any existing link, and is
        // about to be dropped. Clear the handle first so nothing hands a
        // session a port whose other end has gone.
        self.packet_port = None;
        self.digi = Some(self.make_digi(mode, tap_rate));
        // The new controller starts at the mode's default 1500 Hz whatever the
        // last one was on, so the band's stored offset has to be applied again.
        // Clearing this leaves that to `follow_band_tx_offset` on the next poll
        // rather than repeating it here.
        self.digi_tx_band = None;
        if mode == Mode::Cw {
            self.source.set_cw_wpm(self.digi_config.cw_wpm);
        }
    }

    /// Keep the CW passband centred on the pitch the panel is copying.
    ///
    /// The decoder is fed the post-filter audio, the same audio the operator
    /// hears, so a cursor moved outside the passband would be a cursor on a
    /// signal that is neither audible nor decodable. Every CW rig moves its
    /// filter with its pitch control for the same reason; the width the
    /// operator has chosen is left alone.
    fn sync_cw_filter(&mut self) {
        if self.state.rx[0].mode != Mode::Cw {
            return;
        }
        let pitch = self.cw_pitch_hz();
        let r = &mut self.state.rx[0];
        let w = (r.filter_hi - r.filter_lo).abs().clamp(50.0, 3000.0);
        let (lo, hi) = (pitch - w / 2.0, pitch + w / 2.0);
        if (r.filter_lo - lo).abs() < 0.5 && (r.filter_hi - hi).abs() < 0.5 {
            return;
        }
        (r.filter_lo, r.filter_hi) = (lo, hi);
        if let Some(d) = self.main.as_mut().and_then(|c| c.demod.as_mut()) {
            d.set_filter(lo, hi);
        }
        self.push_control_filter();
    }

    /// The sidetone pitch the CW panel is copying at.
    ///
    /// The controller's rather than the configuration's, because the operator
    /// moves it by clicking; the stored figure only stands in before there is a
    /// controller to ask.
    fn cw_pitch_hz(&self) -> f32 {
        self.digi.as_ref().map_or(self.digi_config.cw_pitch_hz, |d| d.audio_hz())
    }

    /// How far above our dial a transceiver's own VFO has to sit.
    ///
    /// Zero everywhere but one case, and that case is CW on a radio handing us
    /// raw I/Q around its own VFO. sdroxide's CW dial is a zero-beat — the tone
    /// being copied sits a sidetone pitch above it, which is what
    /// [`Mode::on_air_hz`] answers and where the keyer puts the carrier when
    /// sdroxide makes it. A transceiver put in CW makes its own instead, on its
    /// VFO, whether the key is a paddle in its socket or text handed to its
    /// keyer — so a VFO left on our dial transmits a whole sidetone below the
    /// station being answered, and the station never hears the call. That is
    /// issue #170, reported on an ELAD FDM-DUO with a key in it: the signal was
    /// copied perfectly at 700 Hz and worked nobody.
    ///
    /// So the VFO goes where the contact is and the DDC takes the difference.
    /// Nothing on screen moves: the readout, the passband and the axis are all
    /// exactly where they were, and the radio is on the station.
    ///
    /// Whether the stream comes out on the VFO at all is a property of the
    /// radio rather than of CW, so the front end has to say
    /// ([`IqSource::cw_iq_on_vfo`]): a rig that moves its own I.F. by the
    /// pitch instead — a K3 on `CW WGHT: VFO OFS`, a QMX on I/Q — hands out a
    /// stream that is already a sidetone below its readout, and there the dial
    /// and the VFO are the same number and must stay it.
    ///
    /// Not in demodulated-audio mode, where the rig's own receiver has already
    /// applied the offset — a station on its VFO is what arrives as a tone — and
    /// not for MCW ([`IqSource::cw_audio_keyed`]), where the rig is held on a
    /// sideband and keyed sidetone lands a pitch above the VFO exactly as it
    /// does on an SDR.
    fn rig_cw_offset_hz(&self) -> f64 {
        if self.audio_mode
            || self.state.rx[0].mode != Mode::Cw
            || !self.source.center_is_dial()
            || !self.source.cw_iq_on_vfo()
            || self.source.cw_audio_keyed()
        {
            return 0.0;
        }
        f64::from(self.cw_pitch_hz())
    }

    /// Put the dial back under a VFO whose *mode* the radio changed.
    ///
    /// [`Self::rig_cw_offset_hz`] the other way round. A mode reported by the
    /// rig is somebody's hand on its front panel, and nothing there moved the
    /// VFO — so entering or leaving CW is ours to absorb: the radio keeps the
    /// frequency it is displaying and our dial takes the sidetone step. The
    /// alternative would nudge a stranger's rig 700 Hz for having been switched
    /// to CW, which is not what connecting to it should do.
    ///
    /// Only where the centre *is* the rig's VFO, which is also where that
    /// front end parks no LO of its own, so the two numbers are one.
    fn reseat_dial_for_cw(&mut self) {
        if self.audio_mode || !self.source.center_is_dial() {
            return;
        }
        let want = self.state.center_hz - self.rig_cw_offset_hz();
        if (want - self.state.active_freq_hz()).abs() < 0.5 {
            return;
        }
        match self.state.active_vfo {
            Vfo::A => self.state.vfo_a_hz = want,
            Vfo::B => self.state.vfo_b_hz = want,
        }
        self.state.band = Band::containing(want);
        self.good_vfo_hz = want;
        self.update_tuning();
    }

    /// Keep a self-keying transceiver's VFO where CW says it has to be.
    ///
    /// The pitch is where the contact is ([`Self::rig_cw_offset_hz`]), so a
    /// pitch the operator moved — from the panel, or by clicking a station in
    /// the passband — moves the frequency such a radio belongs on; and a
    /// session restored straight into CW opens with the dial and the VFO on the
    /// same number, which in CW they must not be. A no-op in every other mode,
    /// and on an SDR, where `follow_dial` falls through to the span check.
    fn sync_cw_dial(&mut self) {
        if self.state.rx[0].mode != Mode::Cw {
            return;
        }
        self.follow_dial();
        self.update_tuning();
    }

    /// Hand the main receiver's passband to a radio that filters for us.
    ///
    /// A no-op everywhere but a CAT rig, and a radio listening to one behind an
    /// attached panadapter receiver. There the operator's width control has
    /// nothing on this side to act on — the audio arrives already filtered — so
    /// unless it reaches the radio it is a control that moves and does nothing.
    fn push_control_filter(&mut self) {
        if !self.filters_at_the_rig() {
            return;
        }
        let r = &self.state.rx[0];
        self.source.set_control_filter(r.mode, r.filter_lo as f64, r.filter_hi as f64);
    }

    /// Whether the radio in front of us is the one doing the receiving, so its
    /// mode and its filter are the ones the operator's controls have to reach.
    ///
    /// True for a demod-audio rig, where there is nothing else; and for a radio
    /// listening to its transceiver while an attached receiver paints the
    /// picture, where there *is* a demodulator here but it is not the one being
    /// heard. False where sdroxide demodulates what is heard, however the
    /// transmitter is driven — the rig's receive settings are then its own
    /// business, and its transmit mode is asserted at key-down instead.
    fn filters_at_the_rig(&self) -> bool {
        self.audio_mode || self.caps.rx_audio_external
    }

    /// The mode to command the radio in front of us. The operator's mode, with
    /// one exception.
    ///
    /// Analog SSTV is a phone emission and follows phone practice rather than
    /// the digital modes' fixed USB, so on 160/80/40 m the rig has to be put in
    /// LSB. There is no lower-sideband spelling of `Mode::Sstv` to send — every
    /// family's mode map is a table with no dial in it — so the translation
    /// happens here, at the one place that knows both the mode and where we are
    /// tuned. A rig then echoing LSB back is answered by the same translation
    /// in [`Self::apply_control_update`], so nothing drags it off again.
    ///
    /// Not for a radio that answers [`IqSource::resolves_sstv_sideband`]. A CAT
    /// rig's mode goes through `digi_mode` — plain sideband or the rig's DATA
    /// position — and translating to LSB first spends that choice, which put
    /// the picture on the microphone input (issue #313). That layer has the
    /// dial and works the sideband out itself.
    fn control_mode(&self) -> Mode {
        let mode = self.state.rx[0].mode;
        if mode.is_sstv()
            && mode.is_lower_sideband_at(self.state.rx_freq_hz())
            && !self.source.resolves_sstv_sideband()
        {
            Mode::Lsb
        } else {
            mode
        }
    }

    /// Assert the operator's receive mode (and, where it applies, passband) to
    /// a front end that asked to track it — see [`IqSource::tracks_rx_mode`].
    ///
    /// Called wherever the source is established rather than only when the
    /// operator changes something: a radio comes up in the mode its session
    /// restored, and a pairing whose receiver offset depends on that mode has
    /// no other way to learn it. Deliberately *not* extended to demod-audio
    /// rigs, which have always been left to report their own mode at startup.
    fn push_rx_mode(&mut self) {
        if !self.source.tracks_rx_mode() {
            return;
        }
        let _ = self.source.set_control_mode(self.control_mode());
        self.push_control_filter();
    }

    /// Whether an operator's mode change has to reach the radio in front of us.
    ///
    /// Wider than [`Self::push_rx_mode`]'s test, which is about asserting the
    /// mode at establishment: a demod-audio rig has no other demodulator, a
    /// panadapter pairing's receiver offset can depend on the mode, and an Icom
    /// handing us its 12 kHz IF filters that IF by the mode it is in — all
    /// three have to follow the mode control, even though only the middle one
    /// wants it imposed the moment the source opens.
    fn rig_follows_rx_mode(&self) -> bool {
        self.audio_mode || self.source.tracks_rx_mode() || self.source.commands_rx_mode()
    }

    /// Construct or tear down the digi controller to match the current mode.
    /// The transform, analysis window and overlap the digital modes' channel
    /// analyser should run at on a channel of `ch_rate`.
    ///
    /// This analyser *replaces* the panadapter's whole frame while a slotted
    /// mode is up, and it draws a window a few kilohertz wide out of a channel
    /// tens of kilohertz wide — so both of the numbers that used to be one
    /// fixed 16384 matter, and they pull in opposite directions.
    ///
    /// The **transform** has to be large enough that the visible window has a
    /// point under every column of the display, or the picture stair-steps: at
    /// the old fixed size a 3.7 kHz window off a 48 kHz channel was 1263 points
    /// spread across a 2560-column panel, drawn two columns to a point. That is
    /// the "rough and boxed" of issue #302, and turning the *FFT* setting up did
    /// nothing because this lane never read it.
    ///
    /// The **window** has to be short enough that a row of the waterfall is a
    /// moment rather than a paragraph. Growing the transform to fix the first
    /// problem would have made the second one four times worse — 1.4 seconds of
    /// signal in every row, on a mode whose symbols are 160 ms long — which is
    /// why the two are chosen separately and the difference is padding.
    ///
    /// On the ordinary FT8 setup the window comes out at a third of a second and
    /// 2.9 Hz a bin, which is WSJT-X's own waterfall transform, arrived at from
    /// [`RESOLVED_PER_VIEW`] rather than copied.
    fn channel_analyzer_shape(&self, ch_rate: f64) -> (usize, usize, usize) {
        // What this lane actually draws: the mode's sub-band where it has one,
        // else whatever the client is looking at, bounded by the channel.
        let view = self
            .cfg
            .viewport
            .map(|(lo, hi)| hi - lo)
            .filter(|s| s.is_finite() && *s > 0.0)
            .unwrap_or(DIGI_VIEW_SPAN_HZ)
            .min(ch_rate);
        // A point per display column across that window, which over the whole
        // channel is that many times wider.
        let want = (self.cfg.bins() as f64 * ch_rate / view).ceil().max(1.0);
        let fft = (want as usize).next_power_of_two().clamp(4096, CHANNEL_FFT_MAX);
        let window_len = analysis_window(fft, ch_rate, view);
        (fft, window_len, hop_div_for(ch_rate, window_len, f64::from(self.cfg.rows())))
    }

    /// Re-shape the digital modes' channel analyser when the display it serves
    /// has changed enough to want a different one — see
    /// [`Engine::channel_analyzer_shape`]. A no-op when there is no such lane,
    /// and a no-op when the shape is the one already running.
    fn sync_channel_analyzer(&mut self) {
        let Some(have) = self.channel_analyzer.as_ref() else { return };
        let ch_rate = self.channel_rate_hz;
        let (fft, window_len, hop_div) = self.channel_analyzer_shape(ch_rate);
        if have.fft_size() == fft && have.window_len() == window_len {
            return;
        }
        let mut ca =
            SpectrumAnalyzer::with_window(fft, window_len, ch_rate, CHANNEL_AVG_TC_S, hop_div);
        // Seeded from the lane it replaces, over exactly the same span, so the
        // waterfall does not go black for the length of one transform every
        // time somebody drags a zoom.
        ca.seed_from(have, (0.0, 1.0));
        self.channel_analyzer = Some(ca);
    }

    fn sync_digi_mode(&mut self) {
        let mode = self.state.rx[0].mode;
        // CW joins the digital modes here and nowhere else. It is not one — the
        // rig is in CW, the demodulated tone stays audible, and none of the
        // digital-mode display or band-plan handling applies — but the panel's
        // decoder and keyer need exactly the audio tap and transmit-block seam
        // this builds, so it gets one.
        let want = mode.is_digital() || mode == Mode::Cw;
        let have = self.digi.is_some();
        // Audio mode feeds the decoder the rig's audio directly (run_audio_mode);
        // there's no RxChain tap or high-res channel analyzer. A radio
        // listening to its transceiver behind an attached panadapter receiver
        // feeds it the same way, at the transceiver's sound-card rate — the
        // decoders work on what is heard, not on what the receiver demodulated.
        let tap_rate = if self.audio_mode {
            self.radio_fs
        } else if self.caps.rx_audio_external {
            self.ext_audio_rate
        } else {
            self.main.as_ref().map(|c| c.audio_rate()).unwrap_or(48_000.0)
        };
        // The high-resolution channel analyzer replaces the panadapter's whole
        // frame with a 3.7 kHz window on the dial, which is what a digital mode
        // wants and the opposite of what CW does. A CW operator is working the
        // band: tuning across it, watching the skimmer mark stations either side,
        // choosing where to call. Narrowing the display to the passband would
        // take the band away and leave pan and zoom with nothing to move over,
        // because the frame itself would no longer contain it.
        let want_channel = want && mode.is_digital() && !self.audio_mode;
        match (want_channel, self.channel_analyzer.is_some()) {
            (true, false) => {
                let ch_rate = self.channel_rate_hz;
                let (fft, window_len, hop_div) = self.channel_analyzer_shape(ch_rate);
                let mut ca = SpectrumAnalyzer::with_window(
                    fft,
                    window_len,
                    ch_rate,
                    CHANNEL_AVG_TC_S,
                    hop_div,
                );
                // This window *is* the panadapter while a digital mode is up, so
                // unseeded, entering FT8 blacks the display out for as long as
                // the first transform takes to fill. The device-wide analyser
                // has the same band already.
                let window = span_fraction(
                    self.display_center_hz(),
                    self.state.sample_rate,
                    self.state.rx_freq_hz(),
                    ch_rate,
                );
                ca.seed_from(&self.analyzer, window);
                debug!(
                    rate = ch_rate,
                    fft,
                    window_len,
                    hop_div,
                    bin_hz = ch_rate / window_len as f64,
                    window_s = window_len as f64 / ch_rate,
                    "digital-mode channel analyser built"
                );
                self.channel_analyzer = Some(ca);
            }
            // Covers arriving in CW from a digital mode, where the analyzer is
            // already up and nothing else would take it down.
            (false, true) => self.channel_analyzer = None,
            _ => {}
        }

        self.digi_rate = tap_rate;
        if want && !have {
            self.start_digi(mode, tap_rate);
            self.sync_audio_tap();
            info!(?mode, tap_rate, "digital-mode engine started");
            // CW enters with the operator's saved pitch, which need not be the
            // 700 Hz the mode's default passband is centred on.
            self.sync_cw_filter();
            // Emit the operator config so a client that hasn't seen a digital
            // mode yet (e.g. straight into SSTV) can seed its editable copy.
            self.emit_digi_status();
        } else if want && have {
            // Mode changed between digital modes: rebuild for the new one.
            if self.digi.as_ref().map(|d| d.mode()) != Some(mode) {
                self.start_digi(mode, tap_rate);
            }
        } else if !want && have {
            if let Some(d) = self.digi.as_mut() {
                d.abort();
            }
            // Kill any digi-driven transmission.
            if self.digi_tx || self.state.tx.ptt {
                self.state.tx.ptt = false;
                self.digi_tx = false;
                self.tci_tx = false;
                self.sync_tx_state();
            }
            self.digi = None;
            self.packet_port = None;
            self.channel_analyzer = None;
            self.sync_audio_tap();
            info!("digital-mode engine stopped");
        }
    }

    /// Build the display spectrum frame. In digital modes it comes from the
    /// high-resolution channel analyzer (VFO-centered) while the requested
    /// viewport fits inside the DDC channel; otherwise from the full-rate
    /// device analyzer.
    /// Take whatever sweep the front end has finished into the cache.
    ///
    /// Polled rather than paced: the source decides its own sweep rate, and
    /// asking more often than it produces simply leaves the cache alone. Every
    /// `wide_spectrum_db` implementation returns before it touches `out`, so a
    /// poll that finds nothing keeps the sweep already held.
    fn poll_wide(&mut self) {
        let mut bins = std::mem::take(&mut self.wide_bins);
        let sweep = self.source.wide_spectrum_db(&mut bins).filter(|_| !bins.is_empty());
        self.wide_bins = bins;
        if let Some(window) = sweep {
            self.wide_window = Some(window);
            self.wide_at = Instant::now();
            self.wide_fresh = true;
            self.wide_sweeps = self.wide_sweeps.wrapping_add(1);
        }
        // The scope is the display axis while it is the main lane, so the axis
        // has to follow it both ways: a sweep on a new centre or span moves the
        // window, and a scope that stops sweeping — switched off at the radio,
        // or a session that never had one — hands the window back to the audio
        // band. Clients are told only when something actually moved: at ten
        // sweeps a second, a state broadcast per sweep is traffic for nothing.
        if self.audio_mode {
            let before = (self.state.center_hz, self.state.sample_rate);
            self.update_display_center();
            if (self.state.center_hz, self.state.sample_rate) != before {
                let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
            }
        }
    }

    /// Every FFT lane that can feed the main panadapter, so the peak hold
    /// between waterfall rows can be switched on or read across all of them at
    /// once without naming them four times at each call site.
    fn lanes(&mut self) -> impl Iterator<Item = &mut SpectrumAnalyzer> {
        std::iter::once(&mut self.analyzer)
            .chain(std::iter::once(&mut self.tx_analyzer))
            .chain(self.zoom.as_mut().map(|z| &mut z.analyzer))
            .chain(self.channel_analyzer.as_mut())
    }

    /// Whether the frame just built came out of an analyser holding nothing —
    /// a lane rebuilt a moment ago, or one reset across a transmit — and so
    /// reads the display floor in every column.
    ///
    /// The invariant: **a picture nobody has measured is not published.** A
    /// black frame is a lie about the band, and a black waterfall row is a
    /// permanent one — it scrolls away up the history and stays there. Holding
    /// the last honest picture for the millisecond or two an analyser takes to
    /// fill is what every part of this file's row plumbing already does when
    /// there is nothing new to say.
    ///
    /// Asked afterwards rather than worked out in advance for the same reason
    /// [`SpectrumAnalyzer::took_row`] is: which lane draws is a dozen
    /// conditions deep in [`Engine::make_spectrum_frame`], and a second copy of
    /// that branch here would go wrong the first time either changed. Every
    /// lane is polled, not just until one answers, so no flag is left standing
    /// for the next frame to trip over.
    ///
    /// **Not the channel analyser.** Every other lane is fed straight off the
    /// front end and fills within one transform of it, so waiting is a matter
    /// of milliseconds. That one is fed from the receive chain's DDC output and
    /// there may not be a receive chain — an engine with no audio configured
    /// never feeds it at all — so holding the panadapter until it has something
    /// could hold it for ever. It is seeded instead, where there is a wider
    /// picture to seed it from.
    fn drew_empty(&mut self) -> bool {
        std::iter::once(&mut self.analyzer)
            .chain(std::iter::once(&mut self.tx_analyzer))
            .chain(self.zoom.as_mut().map(|z| &mut z.analyzer))
            .fold(false, |empty, a| a.drew_empty() | empty)
    }

    /// One waterfall row: the strongest thing each column saw since the last
    /// row was taken.
    ///
    /// Deliberately built by calling [`Engine::make_spectrum_frame`] itself
    /// with the lanes switched to read their held peaks. The row and the frame
    /// it will ride in have to agree about which lane is drawing, what the
    /// viewport is and how the bins are pooled, and the only way to be sure of
    /// that is for it to be the same code — a second copy of that branch would
    /// drift from this one on the first change to either.
    ///
    /// `None` where a lane has no peaks to hold: a radio's own sweep arrives
    /// finished a few times a second and has nothing between rows to miss, and
    /// a transmit monitor is a level check rather than a record of the band.
    /// Both leave the frame's rows empty and the client scrolls on its own
    /// clock, which is what every build before this one did everywhere.
    fn make_row(&mut self) -> Option<SpectrumFrame> {
        if !self.clocks_rows() {
            return None;
        }
        // Arm every lane that could draw this row, then let the frame builder
        // pick as it always does and ask afterwards which one it read.
        //
        // **Only that lane keeps the hold.** It costs a compare and a store per
        // bin on every transform, over an array as long as the transform — half
        // a megabyte on a device-wide analyser running a 131072-point window —
        // and a lane nobody is pooling from pays all of it for nothing. That is
        // the whole of what a zoomed panadapter on a 2.4 Msps front end was
        // spending on the device-wide lane while the zoom lane drew the picture
        // (issue #216).
        //
        // A lane that has just become the drawing one answers its first row
        // from the running average — the current spectrum, which is exactly
        // what every build before the row clock drew — and holds from the next.
        self.lanes().for_each(|a| a.set_read_hold(true));
        let frame = self.make_spectrum_frame();
        self.lanes().for_each(|a| {
            a.set_read_hold(false);
            if a.took_row() {
                a.set_row_hold(true);
                a.reset_hold();
            } else {
                a.set_row_hold(false);
            }
        });
        // A row is history: published black it stays black for as long as the
        // operator scrolls back. See [`Engine::drew_empty`].
        (!self.drew_empty()).then_some(frame)
    }

    /// Feed the panadapter lanes a block, clocking waterfall rows off the
    /// samples as they go by.
    ///
    /// The block is handed over in row-sized pieces rather than whole. Nothing
    /// about the analysis changes — an FFT still lands every `hop` samples,
    /// wherever the piece boundaries fall — but a row can now be taken *inside*
    /// a block, so the waterfall's rate is what the operator asked for instead
    /// of however often the front end happens to be read. On a 1.5 Msps source
    /// read 16384 samples at a time that is the difference between 94 rows a
    /// second and 224.
    ///
    /// What still bounds it is the analyser: a row can never show more than the
    /// transforms its interval contained, so asking for rows faster than
    /// `rate / hop` simply repeats them.
    fn feed_panadapter(&mut self, iq: &[Complex32]) {
        self.row_sample_clock = true;
        // Which window the zoom lane covers cannot change inside one block, so
        // it is settled once here rather than per piece.
        self.sync_zoom();
        let per_row = self.row_period_samples();
        let mut off = 0;
        while off < iq.len() {
            let want = per_row.saturating_sub(self.row_samples).max(1);
            let take = want.min(iq.len() - off);
            let chunk = &iq[off..off + take];
            self.analyzer.process(chunk);
            if let Some(zoom) = self.zoom.as_mut() {
                zoom.process(chunk);
            }
            self.row_samples += take;
            off += take;
            if self.row_samples >= per_row {
                self.row_samples = 0;
                self.push_row();
            }
        }
    }

    /// Take the receive chain's channel rate into
    /// [`Engine::channel_rate_hz`](#structfield.channel_rate_hz).
    fn refresh_channel_rate(&mut self) {
        self.channel_rate_hz = self.main.as_ref().map_or(48_000.0, |c| c.channel_rate());
    }

    /// Samples between waterfall rows at the rate the client asked for.
    fn row_period_samples(&self) -> usize {
        (self.state.sample_rate / f64::from(self.cfg.rows())).max(1.0) as usize
    }

    /// Clock one row into the batch the next frame will carry.
    fn push_row(&mut self) {
        let Some(row) = self.make_row() else {
            return;
        };
        let axis = (row.center_hz, row.span_hz, row.bins.len());
        if self.row_axis != Some(axis) && !self.slide_batch(axis) {
            self.row_batch.clear();
            self.row_axis = Some(axis);
        }
        let cols = row.bins.len();
        if cols == 0 {
            return;
        }
        self.rows_clocked = self.rows_clocked.wrapping_add(1);
        while self.row_batch.len() / cols >= MAX_BATCH_ROWS {
            self.row_batch.drain(..cols);
        }
        self.row_batch.extend_from_slice(&row.bins);
    }

    /// Hand the batch to a frame about to go out, if the rows are of that
    /// frame — a lane that switched between the last row and this frame leaves
    /// them behind rather than drawing one picture's history under another's.
    fn attach_rows(&mut self, frame: &mut SpectrumFrame) {
        // Said whether or not there are any rows *this* frame: below the frame
        // rate most frames carry none, and the client has to know that as
        // "wait for the next one" rather than as "scroll this yourself".
        frame.rows_clocked = self.clocks_rows();
        // A lane that has just stopped clocking still has whatever it batched
        // before it stopped — the operator keying up, or zooming the audio
        // scope in. Those rows go nowhere: the client is about to scroll this
        // frame on its own wall clock, and rows handed to it alongside that
        // instruction are a picture drawn twice over. "Does not clock rows"
        // and "carries rows" are contradictory, and the invariant is worth
        // holding at the one place that can break it.
        if !frame.rows_clocked {
            self.row_batch.clear();
            return;
        }
        let cols = frame.bins.len();
        let axis = (frame.center_hz, frame.span_hz, cols);
        if self.row_axis == Some(axis) || self.slide_batch(axis) {
            frame.rows = std::mem::take(&mut self.row_batch);
            if cols > 0 {
                self.rows_sent = self.rows_sent.wrapping_add((frame.rows.len() / cols) as u64);
            }
        } else {
            self.row_batch.clear();
        }
    }

    /// Carry the batched rows onto a window that has moved, and say whether
    /// they could be.
    ///
    /// A centre that has moved is not a different picture, and this is the
    /// common case rather than the exotic one: a panadapter drag with the view
    /// fully zoomed out moves the window once per displayed frame (issue
    /// #133), so between one row and the next — and between the last row and
    /// the frame it belongs to — the axis has usually shifted. Throwing the
    /// batch away each time cost about a third of the rows at the medium
    /// scroll rate, measured, and the waterfall then scrolled slower than its
    /// own time labels for as long as the drag lasted (issue #177).
    fn slide_batch(&mut self, to: (f64, f64, usize)) -> bool {
        let Some(from) = self.row_axis else { return false };
        if !slide_rows(&mut self.row_batch, from, to) {
            return false;
        }
        self.row_axis = Some(to);
        true
    }

    /// Whether the lane about to be published clocks its own waterfall rows.
    ///
    /// The same test [`Engine::make_row`] refuses on, named once so the two
    /// cannot disagree — a lane that said it clocked rows and then never
    /// produced any would freeze the waterfall.
    fn clocks_rows(&self) -> bool {
        // A sweep that arrives finished has nothing between rows to miss, so
        // the client scrolls it on its own clock. The audio analyser is not
        // like that: it runs transforms continuously, and while it is the one
        // drawing ([`Engine::audio_zoom_window`]) the waterfall gets its rows
        // clocked and its peak held like any other lane.
        let scope_draws = self.audio_mode
            && self.scope_main_window().is_some()
            && self.audio_zoom_window().is_none();
        !(self.tx_active || scope_draws)
    }

    /// Build a full-band frame, if a sweep has arrived since the last one.
    ///
    /// The source hands over dBFS bins covering its whole Nyquist band; the
    /// display policy — pooling down to [`WIDE_BINS`] and mapping to the u8
    /// range the client draws — stays here, identical to the main lane, so both
    /// panadapters respond to the same level controls.
    ///
    /// Deliberately the constant and not the client's chosen width: the strip
    /// this feeds is a shallow band-wide overview a thousand pixels across, so
    /// widening it with the main panadapter would spend link bandwidth on
    /// detail that is pooled away again at the far end.
    fn make_wide_frame(&mut self) -> Option<SpectrumFrame> {
        if !self.wide_fresh {
            return None;
        }
        self.wide_fresh = false;
        let (center_hz, span_hz) = self.wide_window?;
        let (floor, ceil) = auto_levels(&self.wide_bins, self.wide_levels);
        self.wide_levels = Some((floor, ceil));
        self.wide_seq = self.wide_seq.wrapping_add(1);
        Some(pool_window_to_frame(
            &self.wide_bins,
            self.wide_seq,
            center_hz,
            span_hz,
            floor,
            ceil,
            WIDE_BINS,
            None,
        ))
    }

    /// The window the radio's own spectrum covers, when that spectrum is what
    /// the *main* panadapter should be showing.
    ///
    /// True for a receive path that carries no I/Q at all but does publish a
    /// finished spectrum — an Icom's LAN session with its audio set to AF, and
    /// its `27 00` scope streaming. There the audio FFT is not a picture of the
    /// band at all: it is a picture of what the radio has already demodulated,
    /// one-sided by construction (a dial with the whole display bandwidth above
    /// it and nothing below), and never wider than the rig's own filter however
    /// wide the display is set. The scope is the only real spectrum such a
    /// session has, it is centred on the dial, and it is what every other client
    /// of these radios draws.
    ///
    /// Except in the digital modes, whose waterfall *is* the audio band: FT8 and
    /// the keyboard modes place signals by their audio offset inside the rig's
    /// passband, and a band-wide scope at a few hundred Hz a bin cannot show one
    /// at all.
    fn scope_main_window(&self) -> Option<(f64, f64)> {
        if !self.audio_mode
            || self.state.rx[0].mode.is_digital()
            || self.wide_at.elapsed() >= SCOPE_MAIN_STALE
        {
            return None;
        }
        self.wide_window
    }

    /// The full-band window, when the viewport has been zoomed out past what
    /// the I/Q covers and that spectrum is the only thing that reaches it.
    ///
    /// [`Engine::scope_main_window`] is the same idea for a front end with no
    /// I/Q at all; this is for one that has some, but not as much as the
    /// operator is asking to see. The test is containment rather than width: a
    /// viewport that has been *panned* off the edge of the passband is as far
    /// outside it as one that is simply wider, and neither has anything for the
    /// device analyser to draw.
    ///
    /// `None` in audio mode — the branch above owns that case — and while the
    /// front end has published nothing wide lately, so a lane that stops
    /// arriving hands the picture back rather than freezing it.
    fn wide_main_window(&self) -> Option<(f64, f64)> {
        if self.audio_mode || self.wide_at.elapsed() >= SCOPE_MAIN_STALE {
            return None;
        }
        let (wide_center, wide_span) = self.wide_window?;
        let (vp_lo, vp_hi) = self.cfg.viewport?;
        if !(wide_span > 0.0 && vp_hi > vp_lo) {
            return None;
        }
        // What the device analyser can actually draw. Its centre is the one the
        // samples in hand were taken at, not the one most recently commanded —
        // on a laggy front end those differ, and the wrong one here would flip
        // the panadapter between the two sources every time the dial moved.
        wide_covers_viewport(
            (vp_lo, vp_hi),
            self.stream_center_now(),
            self.state.sample_rate,
            (wide_center, wide_span),
            self.wide_bins.len(),
        )
        .then_some((wide_center, wide_span))
    }

    /// The RF window the rig's demodulated audio covers: its passband, on the
    /// side of the dial the mode puts it.
    fn audio_band(&self) -> (f64, f64) {
        let dial = self.state.active_freq_hz();
        if self.state.rx[0].mode.is_lower_sideband_at(dial) {
            (dial - self.audio_bw, dial)
        } else {
            (dial, dial + self.audio_bw)
        }
    }

    /// The viewport, when it lies inside the rig's own passband — and so when
    /// the audio it is sending resolves that window far better than its scope.
    ///
    /// A serial CAT rig's scope is a fixed number of points across whatever
    /// span it was told to sweep: an IC-705 sends 475, so at ±250 kHz that is a
    /// kilohertz a point, and an operator zooming in is magnifying rather than
    /// resolving — a CW signal stays one block wide however far they go, and
    /// the block only gets fatter. Measured on that radio: 1053 Hz a point at
    /// ±250 kHz, 105 at ±25 kHz, and about four sweeps a second at every span.
    ///
    /// The same rig is already sending its demodulated audio over the sound
    /// card, and that goes through the panadapter's own analyser — 48 kHz
    /// through a 16384-point window is some three hertz a bin, arriving twenty
    /// times a second. Inside the passband it is a better picture by two orders
    /// of magnitude in both axes.
    ///
    /// Outside it there is nothing to switch to: the audio is not a picture of
    /// the band at all, only of what the rig has already demodulated, so a
    /// wider view stays on the scope. The display *axis* stays the scope's
    /// either way ([`Engine::update_display_center`]), so zooming back out is
    /// the same gesture it always was.
    fn audio_zoom_window(&self) -> Option<(f64, f64)> {
        if !self.audio_mode {
            return None;
        }
        let (lo, hi) = self.cfg.viewport?;
        if !(hi > lo) {
            return None;
        }
        let (band_lo, band_hi) = self.audio_band();
        // What the operator can actually see, which is not what arrives: a
        // client sends its window with slack around it so that panning inside
        // it needs no reconfiguration, and today that slack is double the
        // visible span. So the visible part is the middle of what was asked
        // for, and it is the *visible* part that has to be inside the rig's
        // filter for the audio to be the honest picture. Judging the slack as
        // well would refuse every zoom that reached the passband edge.
        let quarter = (hi - lo) / 4.0;
        if lo + quarter < band_lo || hi - quarter > band_hi {
            return None;
        }
        // Clipped to the filter, so the mirror the other side of the dial is
        // never drawn: the rig sends *real* audio, whose spectrum is symmetric,
        // and only the one side of it is the band.
        Some((lo.max(band_lo), hi.min(band_hi)))
    }

    /// The main panadapter, drawn from the radio's own finished bins.
    ///
    /// Auto-ranged rather than mapped through the operator's dB window: an
    /// Icom's scope is a 0..=160 scale with no documented dB per step, so the
    /// numbers reaching here are a linear guess with an uncalibrated slope and a
    /// fixed floor/ceiling would show either black or white. The levels are the
    /// full-band lane's own, so two lanes drawing the same sweep cannot disagree
    /// about it.
    fn make_scope_frame(&mut self, center_hz: f64, span_hz: f64) -> SpectrumFrame {
        let (floor, ceil) = auto_levels(&self.wide_bins, self.wide_levels);
        self.wide_levels = Some((floor, ceil));
        self.scope_seq = self.scope_seq.wrapping_add(1);
        pool_window_to_frame(
            &self.wide_bins,
            self.scope_seq,
            center_hz,
            span_hz,
            floor,
            ceil,
            self.cfg.bins(),
            self.cfg.viewport,
        )
    }

    /// Remember where the centre went, for a front end whose stream lags the
    /// command that moved it. A no-op — and no allocation — at zero delay.
    fn note_center_change(&mut self, center_hz: f64) {
        if self.source.stream_delay_s() <= 0.0 {
            return;
        }
        self.center_trail.push_back((self.samples_read, center_hz));
    }

    /// Where the samples in hand were taken, for a decision that has to be made
    /// against the stream rather than against the dial.
    ///
    /// [`Engine::display_center_hz`] walks [`Engine::center_trail`] to work
    /// this out, but only a front end that declares a stream delay ever builds
    /// a trail: with no delay the samples in hand *are* at the commanded
    /// centre, that function short-circuits, and
    /// [`Engine::stream_center_hz`] is never written after its seed.
    ///
    /// Reading the field regardless is how the main panadapter came to compare
    /// its viewport against wherever the receiver happened to be when the
    /// engine opened. Every tune after that looked, to
    /// [`Engine::wide_main_window`], like a viewport panned clean off the
    /// passband — so an RX-888 drew the whole of HF from its 64.8 MHz overview
    /// lane at 16 kHz a bin, everywhere except the one window the engine had
    /// started in.
    fn stream_center_now(&self) -> f64 {
        if self.source.stream_delay_s() > 0.0 {
            self.stream_center_hz
        } else {
            self.state.center_hz
        }
    }

    /// The centre the samples now being drawn were actually taken at.
    ///
    /// [`Engine::note_center_change`] logs each retune; this reads back the one
    /// in force `stream_delay_s` ago and drops everything older, so the trail
    /// stays a handful of entries however long a drag runs. A source that
    /// declares no delay never reaches past the first line, and gets the
    /// commanded centre exactly as before.
    fn display_center_hz(&mut self) -> f64 {
        let delay = self.source.stream_delay_s();
        if delay <= 0.0 {
            return self.state.center_hz;
        }
        // The samples the analyser is holding were taken this far back up the
        // stream. Counted at the *device* rate, which is what
        // [`Engine::samples_read`] counts: `state.sample_rate` is what is left
        // after front-end decimation, and using it would under-count the delay
        // by exactly that factor.
        let behind = (delay * self.radio_fs) as u64 + self.last_block / 2;
        let Some(cutoff) = self.samples_read.checked_sub(behind) else {
            return self.state.center_hz;
        };
        // Every entry at or before the cutoff has now reached the analyser; the
        // last of them is the centre these samples were taken at, and anything
        // still ahead of the cutoff is a retune the stream has yet to show.
        let mut center = None;
        while let Some(&(at, hz)) = self.center_trail.front() {
            if at > cutoff {
                break;
            }
            center = Some(hz);
            self.center_trail.pop_front();
        }
        if let Some(hz) = center {
            self.stream_center_hz = hz;
        }
        self.stream_center_hz
    }

    fn make_spectrum_frame(&mut self) -> SpectrumFrame {
        if self.tx_active {
            return self.make_tx_frame();
        }
        let bins = self.cfg.bins();
        if self.audio_mode {
            // Zoomed inside the rig's own passband: its audio resolves that
            // window by two orders of magnitude more than its scope can, so it
            // draws — see [`Engine::audio_zoom_window`]. Ahead of the scope
            // test, because the scope is what owns the picture everywhere else.
            if let Some(vp) = self.audio_zoom_window() {
                return self.analyzer.make_frame(
                    self.state.active_freq_hz(),
                    self.radio_fs,
                    self.cfg.db_floor,
                    self.cfg.db_ceil,
                    bins,
                    Some(vp),
                );
            }
            // The radio's own scope, where this session has one — the only
            // spectrum of the *band* a demod-audio path can show.
            if let Some((center_hz, span_hz)) = self.scope_main_window() {
                return self.make_scope_frame(center_hz, span_hz);
            }
            // The real audio's FFT is symmetric; the dial is audio-DC. USB maps
            // audio f → dial+f (show the positive half); LSB → dial-f (negative
            // half). Both give the correct RF window over `audio_bw`.
            let dial = self.state.active_freq_hz();
            let vp = if self.state.rx[0].mode.is_lower_sideband_at(dial) {
                (dial - self.audio_bw, dial)
            } else {
                (dial, dial + self.audio_bw)
            };
            return self.analyzer.make_frame(
                dial,
                self.radio_fs,
                self.cfg.db_floor,
                self.cfg.db_ceil,
                bins,
                Some(vp),
            );
        }
        if let Some(ca) = self.channel_analyzer.as_mut() {
            let vfo = self.state.rx_freq_hz();
            let ch_rate = self.main.as_ref().map(|c| c.channel_rate()).unwrap_or(48_000.0);
            // The client's viewport — the zoomed/panned digital waterfall,
            // which the UI fits to the mode's sub-band on entry. Without one,
            // the fixed-allocation modes still get their FT8 sub-band (a
            // viewport-less client shows the right frame from the first one),
            // while the free-roaming modes get the full device span — for
            // them None is what a fully zoomed-out view sends, and it must
            // not snap back to the sub-band.
            let mode = self.state.rx[0].mode;
            let full = self.state.sample_rate;
            let (vp_lo, vp_hi) = self.cfg.viewport.unwrap_or_else(|| {
                if mode.is_slotted() || mode.is_wspr() {
                    (vfo - 200.0, vfo + 3500.0)
                } else {
                    (self.state.center_hz - full / 2.0, self.state.center_hz + full / 2.0)
                }
            });
            // The channel analyzer only sees the DDC output (vfo ± ch_rate/2).
            // While the requested window fits inside, serve it at channel
            // resolution; a wider view falls through to the device analyzer,
            // which is coarser but actually contains the rest of the band.
            if vp_lo >= vfo - ch_rate / 2.0 && vp_hi <= vfo + ch_rate / 2.0 {
                return ca.make_frame(
                    vfo,
                    ch_rate,
                    self.cfg.db_floor,
                    self.cfg.db_ceil,
                    bins,
                    Some((vp_lo, vp_hi)),
                );
            }
        }
        // Zoomed out past what the I/Q covers, onto a front end that also
        // publishes a wider spectrum: draw the main panadapter from *that*.
        //
        // The same fallback the channel analyser makes to the device analyser
        // one step up, for the same reason — coarser, but it actually contains
        // the window being asked for. Without it the zoom-out stops at the I/Q
        // span, which on a receiver whose I/Q is a narrow window onto a wide
        // band is a hard floor a long way in: a KiwiSDR sends 12 kHz of I/Q and
        // a picture of the whole 0-30 MHz, and the panadapter could only ever
        // show the 12 kHz.
        //
        // Deliberately all-or-nothing rather than a composite of the two: the
        // I/Q's own bins are finer in the middle, but splicing two spectra with
        // different noise floors and different analysers leaves a visible seam
        // at the join that reads as a signal. The whole window comes from one
        // source, and which one is decided by whether the I/Q still covers it.
        if let Some((wide_center, wide_span)) = self.wide_main_window() {
            return self.make_scope_frame(wide_center, wide_span);
        }
        // The zoomed window at its own resolution, where the device-wide
        // analyser has run out of bins to give it.
        //
        // Only once the lane has a spectrum. A lane just built holds zeros, and
        // zero is the display floor in every column — so the frames and the
        // waterfall rows drawn from it between its construction and its first
        // transform are black, for a quarter of a second at the rate a deep
        // zoom's lane runs at. The device-wide analyser covers the same window
        // the whole time, coarser but live, which is exactly the picture the
        // operator had one zoom step ago.
        if let Some(z) = self.zoom.as_mut().filter(|z| z.analyzer.primed()) {
            return z.analyzer.make_frame(
                z.center_hz,
                z.rate_hz,
                self.cfg.db_floor,
                self.cfg.db_ceil,
                bins,
                self.cfg.viewport,
            );
        }
        let center = self.display_center_hz();
        self.analyzer.make_frame(
            center,
            self.state.sample_rate,
            self.cfg.db_floor,
            self.cfg.db_ceil,
            bins,
            self.cfg.viewport,
        )
    }

    /// TX monitor frame: the operator's own transmitted signal. Wideband IQ
    /// backends show the upconverted TX at its RF position in the full span;
    /// audio-mode (CAT), audio-TX (TCI) and digital modes show a narrow
    /// transmit-sideband scope built from the TX baseband/audio.
    fn make_tx_frame(&mut self) -> SpectrumFrame {
        let dial = self.tx_center_hz;
        let bins = self.cfg.bins();
        let lsb = self.state.rx[0].mode.is_lower_sideband_at(dial);
        let (floor, ceil) = (self.cfg.db_floor, self.cfg.db_ceil);
        // Attenuate the monitor for display by mapping through a window shifted
        // up by `off` dB (equivalent to attenuating the signal), so full-scale TX
        // lands `TX_MON_HEADROOM_DB` below the ceiling instead of clamping to max.
        // Tracks `ceil` so it stays correct after the user retunes the range (FIT).
        let off = TX_MON_HEADROOM_DB - ceil;
        let (mf, mc) = (floor + off, ceil + off);
        // A `tx_audio` rig (TCI) modulates our raw audio and returns no TX IQ, so
        // voice/tune there also drive `tx_analyzer` (packed-real audio) — not the
        // wideband IQ analyzer — even though it isn't `audio_mode` or digital.
        let mut frame = if self.audio_mode || self.caps.tx_audio || self.channel_analyzer.is_some()
        {
            let bw = if self.audio_mode { self.audio_bw } else { 3500.0 };
            let vp = if self.state.rx[0].mode.is_carrier_centered() {
                // RIFP's transmitted signal straddles the dial.
                let half = (self.state.rx[0].filter_hi - self.state.rx[0].filter_lo).abs() as f64
                    * 0.5
                    * 1.2;
                (dial - half, dial + half)
            } else if lsb {
                (dial - bw, dial)
            } else {
                (dial, dial + bw)
            };
            self.tx_analyzer.make_frame(dial, TX_MONITOR_RATE, mf, mc, bins, Some(vp))
        } else {
            // Wideband IQ: the upconverted TX sits at `tx_center_hz` in the full span.
            self.analyzer.make_frame(self.tx_center_hz, self.state.sample_rate, mf, mc, bins, None)
        };
        // Report the real range so the panadapter's dB axis is unchanged; the
        // bins are already dimmed by the shifted window above.
        frame.db_floor = floor;
        frame.db_ceil = ceil;
        frame
    }

    fn apply(&mut self, cmd: Command) {
        use Command::*;
        match cmd {
            // One arm for the dial and for the panadapter's gestures. They
            // used to part company on a rig that tunes with its dial — a click
            // moved only our receiver and left the rig where it was, on the
            // theory that a signal already inside the captured span needs no
            // retune. A field report (a Kenwood on its I/Q output) showed what
            // that leaves behind: the rig's readout disagreeing with ours, its
            // next frequency report snapping ours back, and two transmit paths
            // that never borrow the dial — CW keyed as text through the rig's
            // own keyer, and a microphone keyed at the radio — going on air at
            // the stale frequency the panel no longer showed. So a click
            // follows the dial exactly as the readout does; on an SDR
            // `follow_dial` still keeps the window, so only the
            // rig-as-front-end case moves.
            SetVfo { vfo, hz } | TuneInSpan { vfo, hz } => {
                self.stop_scan_for_operator();
                let hz = hz.max(0.0);
                match vfo {
                    Vfo::A => self.state.vfo_a_hz = hz,
                    Vfo::B => self.state.vfo_b_hz = hz,
                }
                if vfo == self.state.active_vfo {
                    self.state.band = Band::containing(hz);
                    self.follow_dial();
                }
                self.update_tuning();
            }
            TuneWidebandTo(hz) => {
                self.stop_scan_for_operator();
                // The dial that puts the front end's *own centre* on `hz`,
                // which is where a wideband lane's window is placed from. On a
                // receiver with no LO offset the two are the same number and
                // this is an ordinary retune; on a zero-IF one they differ by a
                // quarter of the span, and using the dial is what left the ISM
                // window a quarter-span above the band it was sent to (#310).
                let want = hz.max(0.0);
                let dial = (want - self.lo_offset_hz()).max(0.0);
                let vfo = self.state.active_vfo;
                match vfo {
                    Vfo::A => self.state.vfo_a_hz = dial,
                    Vfo::B => self.state.vfo_b_hz = dial,
                }
                self.state.band = Band::containing(dial);
                if self.audio_mode {
                    // A rig on a sound card has no window to place: the dial is
                    // the whole receiver, and moving it is all there is to do.
                    self.follow_dial();
                } else if (self.state.center_hz - want).abs() >= 0.5 {
                    // Not `follow_dial`, which moves the front end only once the
                    // dial has left its span. This is a request to move it.
                    self.retune_for_vfo(dial);
                }
                self.update_tuning();
            }
            SelectVfo(v) => {
                // The mode goes with the VFO. Shelve what the one being left
                // was listening in, and put the receiver into what the one
                // being taken up was left in (issue #286).
                self.shelve_vfo_mode();
                self.state.active_vfo = v;
                self.recall_vfo_mode();
                self.state.band = Band::containing(self.state.active_freq_hz());
                self.follow_dial();
                self.update_tuning();
            }
            SwapVfos => {
                // A swap exchanges the whole listening position, not just the
                // two numbers: A's mode and passband go to B along with its
                // dial. The active VFO does not change, so what it is now
                // holding is the *other* one's setup, and the receiver has to
                // be put into it.
                self.shelve_vfo_mode();
                std::mem::swap(&mut self.state.vfo_a_hz, &mut self.state.vfo_b_hz);
                self.vfo_memory.swap(0, 1);
                self.recall_vfo_mode();
                self.state.band = Band::containing(self.state.active_freq_hz());
                self.follow_dial();
                self.update_tuning();
            }
            CopyAtoB => {
                self.state.vfo_b_hz = self.state.vfo_a_hz;
                // A=B copies the position, mode included — otherwise the VFO
                // that was just made a duplicate of A would listen to A's
                // frequency in something else.
                self.shelve_vfo_mode();
                self.vfo_memory[Vfo::B.index()] = self.vfo_memory[Vfo::A.index()];
                // A no-op while A is the active VFO, and the whole point of the
                // call while B is: the receiver has just been moved onto A's
                // frequency and has to hear it in A's mode.
                self.recall_vfo_mode();
                self.update_tuning();
            }
            SetSplit(on) => self.state.split = on,
            SetCenter(hz) => {
                // Asking for the centre the front end is already on costs a
                // hardware retune, a skimmer restart and a waterfall remap for
                // nothing — and a panadapter pan held against the end of a
                // band asks for exactly that, once per frame. Same half-hertz
                // as `follow_dial`.
                if (self.state.center_hz - hz).abs() >= 0.5 {
                    self.retune(hz);
                    self.update_tuning();
                }
            }
            SetSampleRate(_) => { /* needs stream re-open; deferred */ }
            SetDecimation(factor) => self.set_decimation(factor),
            SetBand(band) => {
                self.stop_scan_for_operator();
                self.change_band(band);
            }
            SetMode { rx, mode } => {
                // Decided before the mode is applied, because knowing whether
                // this is a change at all means asking what is being left, and
                // applied after it, because the dial push is mode-aware: the
                // sidetone offset a self-keying rig sits on, the sideband the
                // audio-mode window hangs off, the digital lane's own centre.
                let convention = self.conventional_dial_for(rx, mode);
                self.set_rx_mode(rx, mode);
                if let Some(hz) = convention {
                    self.tune_to_conventional_dial(hz);
                }
            }
            SetFilter { rx, lo, hi } => {
                let (lo, hi) = (lo.min(hi), lo.max(hi));
                let r = &mut self.state.rx[rx.index()];
                (r.filter_lo, r.filter_hi) = (lo, hi);
                if let Some(d) = self.chain_mut(rx).and_then(|c| c.demod.as_mut()) {
                    d.set_filter(lo, hi);
                }
                // The main receiver's filter is also the transmit passband.
                // Retune it live, mid-over, rather than at the next PTT. An
                // inverting transponder flips the sideband, which mirrors the
                // filter edges the same way `tx_begin` does.
                if rx == RxId::Main {
                    let inverting = self
                        .sat_lock
                        .as_ref()
                        .is_some_and(|l| l.cfg.uplink.is_some_and(|u| u.inverting));
                    let (lo, hi) = if inverting { (-hi, -lo) } else { (lo, hi) };
                    if let Some(m) = self.tx.as_mut().and_then(|tx| tx.modulator.as_mut()) {
                        m.set_filter(lo, hi);
                    }
                    // The envelope processor filters against the same passband
                    // and has to be told at the same moment: a correction
                    // band-limited to the width the operator has just left
                    // would be one the transmission no longer fits inside.
                    if let Some(c) = self.tx.as_mut().and_then(|tx| tx.cessb.as_mut()) {
                        c.set_filter(lo, hi);
                    }
                    // And on a radio that does its own filtering, the width the
                    // operator just set belongs to the radio.
                    self.push_control_filter();
                }
            }
            SetAgc { rx, agc } => {
                self.state.rx[rx.index()].agc = agc;
                // Switching off hands the audio to the fixed manual gain. Seed
                // it from where the AGC had settled so the level carries over:
                // the operator is turning off the levelling, not asking to go
                // deaf. They can trim it from there.
                if agc == AgcMode::Off {
                    if let Some(db) = self.chain_mut(rx).map(|c| c.agc.gain_db()) {
                        self.state.rx[rx.index()].manual_gain_db =
                            db.clamp(0.0, sdroxide_types::MAX_MANUAL_GAIN_DB);
                    }
                }
                let manual_db = self.state.rx[rx.index()].manual_gain_db;
                if let Some(c) = self.chain_mut(rx) {
                    c.agc.set_manual_gain_db(manual_db);
                    c.agc.set_mode(agc);
                }
            }
            SetAgcMaxGain { rx, db } => {
                self.state.rx[rx.index()].agc_max_gain_db = db;
                if let Some(c) = self.chain_mut(rx) {
                    c.agc.set_max_gain_db(db);
                }
            }
            SetManualGain { rx, db } => {
                let db = db.clamp(0.0, sdroxide_types::MAX_MANUAL_GAIN_DB);
                self.state.rx[rx.index()].manual_gain_db = db;
                if let Some(c) = self.chain_mut(rx) {
                    c.agc.set_manual_gain_db(db);
                }
            }
            SetVolume { rx, v } => self.state.rx[rx.index()].volume = v.clamp(0.0, 1.0),
            SetMute { rx, muted } => self.state.rx[rx.index()].muted = muted,
            SetSquelch { rx, db } => self.state.rx[rx.index()].squelch_db = db,
            // The rig's own squelch, on a front end that has one. Held in the
            // state either way so the rail keeps its position on a source that
            // is not listening, and passed straight down — the radio is what
            // the level means something to.
            SetRigSquelch { frac } => {
                self.state.rig_squelch = frac.clamp(0.0, 1.0);
                self.source.set_squelch(self.state.rig_squelch);
            }
            SetNoiseBlanker(on) => self.state.noise_blanker = on,
            SetNoiseReduction { rx, level } => {
                // A remote client can ask for an engine this host cannot run.
                // Answer with the truth rather than with silence: fall back to
                // the same strength on RNNoise, say so once, and let the state
                // broadcast correct every attached UI's optimistic echo.
                let level = if level.engine() == Some(NrEngine::DeepFilter)
                    && !dfnr_available(&mut self.audio_dfnr, &mut self.audio_dfnr_failed)
                {
                    let fallback = level.with_engine(NrEngine::Rnn);
                    let _ = self.event_tx.send(RadioEvent::Notice(Some(format!(
                        "DeepFilterNet could not be loaded on this host — using NR {} instead",
                        fallback.label()
                    ))));
                    fallback
                } else {
                    level
                };
                self.state.rx[rx.index()].noise_reduction = level;
            }
            SetAutoNotch { rx, on } => self.state.rx[rx.index()].auto_notch = on,
            SetWfmStereo { rx, on } => self.state.rx[rx.index()].wfm_stereo = on,
            SetBinaural { rx, on } => self.state.rx[rx.index()].binaural = on,
            // Main receiver only, like the status it answers: the DRM panel
            // shows the broadcast being listened to.
            SetDrmService { service } => {
                if let Some(c) = self.main.as_mut() {
                    c.set_drm_service(service);
                }
            }
            SetDrmConstellation { channel } => {
                if let Some(c) = self.main.as_mut() {
                    c.set_drm_constellation(channel);
                }
            }
            SetToneSquelch { rx, tone } => self.state.rx[rx.index()].tone_sql = tone,
            SetScannerConfig(mut cfg) => {
                // A different kind of scan, or the same kind gathering its
                // candidates a different way, is a different scan: the plan in
                // flight belongs to the old setting.
                let restart = self.state.scan.running
                    && (cfg.kind != self.scan_cfg.kind || cfg.mem_fast != self.scan_cfg.mem_fast);
                // Retuning the range (or changing the grid under it) retires the
                // channels skipped in the old one — they described *that* band.
                cfg.forget_stale_skips();
                self.scan_cfg = cfg;
                if restart {
                    // A different kind of scan is a different scan; starting it
                    // fresh is clearer than reinterpreting a half-finished pass.
                    self.stop_scan(None);
                    self.set_scanning(true);
                }
                self.save_scanner_config();
            }
            SetScanning(on) => self.set_scanning(on),
            ScanNext => self.scan_skip(false),
            ScanSkip => self.scan_skip(true),
            SetAudioDuck(gain) => {
                if let Some(mixer) = self.mixer.as_mut() {
                    mixer.set_duck(gain);
                }
                self.speech_duck = gain.clamp(0.0, 1.0);
            }
            SetRecording(on) => {
                if on {
                    self.start_recording();
                } else {
                    self.stop_recording();
                }
            }
            SetRecordingMono(on) => self.state.recording_mono = on,
            SetContestSerial(n) => {
                let n = n.clamp(1, sdroxide_types::CONTEST_SERIAL_MAX);
                if self.digi_config.contest_serial != n {
                    self.digi_config.contest_serial = n;
                    self.push_contest_serial();
                }
            }
            SetIqRecording(on) => {
                if on {
                    self.start_iq_recording();
                } else {
                    self.stop_iq_recording();
                }
                let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
            }
            SetSubRx(on) => {
                self.state.sub_rx_enabled = on;
                if on && self.sub.is_none() && self.main.is_some() {
                    self.sub = Some(RxChain::new(
                        self.state.sample_rate,
                        &self.state.rx[1],
                        self.audio_out_rate,
                    ));
                } else if !on {
                    self.sub = None;
                }
                self.update_tuning();
            }
            SetSubRxFreq(hz) => {
                self.state.sub_rx_hz = self.clamp_to_passband(hz);
                self.update_tuning();
            }
            SetRit { enabled, hz } => {
                self.state.rit = sdroxide_types::OffsetState { enabled, hz };
                self.update_tuning();
            }
            SetXit { enabled, hz } => self.state.xit = sdroxide_types::OffsetState { enabled, hz },
            SetRepeater(r) => self.set_repeater(r.clamped()),
            ToneBurst => self.fire_tone_burst(),
            SetPtt(on) => self.set_ptt(on),
            SetTune(on) => {
                // As with PTT, an operator TUNE takes the transmitter back.
                self.end_tci_tx();
                self.cancel_voice_play();
                self.state.tx.tune = on;
                self.sync_tx_state();
                // Toggled mid-over (PTT held): already keyed, so `sync_tx_state`
                // left the rig alone — swap the power level over by hand.
                if self.tx_active {
                    self.source.set_tx_drive(self.tx_power_level() as f64);
                }
            }
            SetSwrGuard { enabled, limit } => {
                // Clamped, not trusted. Below about 1.1:1 no real antenna ever
                // sits, so a low value would refuse every transmission and look
                // like a broken radio; and the field is `0.0` in a default
                // `TxState`, so an early edit from a client that has not yet
                // heard from the engine would otherwise arm exactly that.
                let limit = limit.clamp(SWR_LIMIT_MIN, SWR_LIMIT_MAX);
                self.swr_guard = enabled;
                self.swr_limit = limit;
                self.state.tx.swr_guard = enabled;
                self.state.tx.swr_limit = limit;
                self.swr_over = 0;
                // Disarming clears a standing trip: leaving transmit latched
                // out by a guard that is no longer on would be unexplainable
                // from the screen.
                if !enabled && self.swr_tripped.take().is_some() {
                    self.state.tx.swr_tripped = None;
                    self.clear_notice();
                }
                self.emit_state();
            }
            ClearSwrTrip => {
                if let Some(swr) = self.swr_tripped.take() {
                    info!(swr, "SWR guard acknowledged, transmit re-enabled");
                    self.swr_over = 0;
                    self.state.tx.swr_tripped = None;
                    self.emit_state();
                    // Clears the persistent warning raised by `deny_tx`, so the
                    // banner goes when the condition it reports has been dealt
                    // with, and not before.
                    self.clear_notice();
                }
            }
            SetTxDrive(v) => {
                self.state.tx.drive = v.clamp(0.0, 1.0);
                // CAT/TCI rigs command output power directly; IQ sources ignore
                // this and scale the modulated samples instead. While tuning the
                // rig is holding the tune level, so leave it alone until unkey.
                if !self.state.tx.tune {
                    let d = self.tx_drive();
                    self.source.set_tx_drive(d as f64);
                }
            }
            SetTuneDrive(v) => {
                self.state.tx.tune_drive = v.clamp(0.0, 1.0);
                self.source.set_tune_drive(self.tx_tune_level() as f64);
                // Tuning right now: the rig's power is the tune level, so the
                // slider takes effect without unkeying.
                if self.state.tx.tune {
                    self.source.set_tx_drive(self.tx_tune_level() as f64);
                }
            }
            SetMicGain(v) => self.state.tx.mic_gain = v.clamp(0.0, 1.0),
            SetTxEq(eq) => self.state.tx.eq = eq.clamped(),

            // ── Voice keyer ─────────────────────────────────────────────────
            VoiceRecord(Some(slot)) => {
                // Recording reads the same microphone the transmitter does, so
                // the two can't run at once.
                if self.tx_active || self.digi_tx {
                    warn!("voice keyer: cannot record while transmitting");
                    return;
                }
                if self.mic.is_none() {
                    warn!("voice keyer: no microphone configured");
                    return;
                }
                if !self.voice.start_record(slot as usize) {
                    return;
                }
                // Whatever accumulated in the capture ring while nothing was
                // draining it is stale; the recording starts from now.
                if let Some(mic) = self.mic.as_mut() {
                    while mic.consumer.pop().is_ok() {}
                }
                self.voice_tick = None;
                self.emit_voice_status();
            }
            VoiceRecord(None) => {
                if self.voice.is_recording() {
                    self.voice.stop_record();
                    self.emit_voice_status();
                }
            }
            VoicePlay(Some(slot)) => self.start_voice_play(slot as usize),
            VoicePlay(None) => self.stop_voice_play(),
            VoicePreview(Some(slot)) => {
                // Monitoring rides on the receive audio path, which stands
                // still while transmitting (and does not exist at all without an
                // audio device) — so there would be nothing to listen to.
                if self.tx_active || self.digi_tx {
                    warn!("voice keyer: cannot monitor a message while transmitting");
                    return;
                }
                if self.mixer.is_none() {
                    warn!("voice keyer: no audio output to monitor through");
                    return;
                }
                if self.voice.is_recording() {
                    self.voice.stop_record();
                }
                if self.voice.start_preview(slot as usize) {
                    self.voice_prev_q.clear();
                    self.voice_tick = None;
                    self.emit_voice_status();
                }
            }
            VoicePreview(None) => self.stop_voice_preview(),
            VoiceClear(slot) => {
                self.voice.clear(slot as usize);
                self.emit_voice_status();
            }
            VoiceRename { slot, name } => {
                self.voice.rename(slot as usize, name);
                self.emit_voice_status();
            }
            // Remembered as well as applied, exactly like the antenna below: a
            // reconnect, an interface switch or the next start reopens the
            // device on its driver defaults, and the operator's front-end gains
            // have to survive that (see [`Engine::restore_gains`]).
            SetGain { dir, element, db } => match dir {
                Direction::Rx => {
                    if let Err(e) = self.source.set_gain_element(&element, db) {
                        warn!("set RX gain {element}: {e}");
                    }
                    self.state.gains = self.source.current_gains();
                    self.remember_gain(dir, element, db);
                    // An RSPdx's HDR switch rides a gain element and changes
                    // which LNA table is in force.
                    self.refresh_rx_gains();
                    // …and a KiwiSDR's waterfall zoom rides one and changes how
                    // wide the full-band lane is, which is what bounds the
                    // panadapter's zoom-out. Cheap, and only sent when the
                    // number really moved.
                    let wide = self.source.wide_span_hz();
                    if wide != self.caps.wide_span_hz {
                        self.caps.wide_span_hz = wide;
                        let _ =
                            self.event_tx.send(RadioEvent::CapabilitiesUpdated(self.caps.clone()));
                    }
                }
                Direction::Tx => {
                    if let Err(e) = self.source.set_tx_gain_element(&element, db) {
                        warn!("set TX gain {element}: {e}");
                    }
                    self.state.tx_gains = self.source.current_tx_gains();
                    self.remember_gain(dir, element, db);
                }
            },
            // Remembered as well as applied: a reconnect or an interface switch
            // reopens the device on its driver default, and the operator's
            // choice has to survive that (see [`Engine::restore_antennas`]).
            SetDeviceSetting { key, value } => {
                if let Err(e) = self.source.set_device_setting(&key, &value) {
                    warn!("set {key} = {value}: {e}");
                    self.notice(&format!("The radio would not take {key} = {value}: {e}"));
                }
            }
            // The radio's own power switch. Nothing else here changes: what
            // sdroxide believes about a radio that has just been switched off is
            // the link's business to notice, and the reconnect that follows is
            // what puts a radio switched back on into a known state.
            SetRigPower(on) => {
                if !self.caps.commands_rig_power {
                    return;
                }
                if let Err(e) = self.source.set_rig_power(on) {
                    warn!("switching the radio {}: {e}", if on { "on" } else { "off" });
                }
            }
            SetAntenna { dir, name } => {
                match dir {
                    Direction::Rx => {
                        if let Err(e) = self.source.set_antenna(&name) {
                            warn!("set RX antenna {name}: {e}");
                        }
                        self.state.antenna_rx = self.source.current_antenna();
                        self.want_antenna.0 = Some(name.clone());
                        self.band_antenna.entry(self.state.band).or_default().0 = Some(name);
                        // A Hi-Z port has fewer front-end states than the 50 Ω
                        // one beside it.
                        self.refresh_rx_gains();
                    }
                    Direction::Tx => {
                        if let Err(e) = self.source.set_tx_antenna(&name) {
                            warn!("set TX antenna {name}: {e}");
                        }
                        self.state.antenna_tx = self.source.current_tx_antenna();
                        self.want_antenna.1 = Some(name.clone());
                        self.band_antenna.entry(self.state.band).or_default().1 = Some(name);
                    }
                }
                // The choice belongs to the band it was made on, so the band it
                // is next left on is the one it has to be remembered against —
                // `poll_band_change` puts it back when the dial returns.
                self.band_seen = self.state.band;
            }
            StoreMemory { name } => {
                let id = self.memories.iter().map(|m| m.id).max().unwrap_or(0) + 1;
                let rx = &self.state.rx[0];
                // An RTTY memory carries the modem setup too: the frequency of
                // a 50 baud / 450 Hz reverse broadcast is useless without it.
                let rtty = (rx.mode == Mode::Rtty).then(|| sdroxide_types::RttyMemory {
                    baud: self.digi_config.rtty_baud,
                    shift_hz: self.digi_config.rtty_shift_hz,
                    reverse: self.digi_config.rtty_reverse,
                    afc: self.digi_config.rtty_afc,
                });
                self.memories.push(MemoryChannel {
                    id,
                    name,
                    freq_hz: self.state.active_freq_hz(),
                    mode: rx.mode,
                    filter_lo: rx.filter_lo,
                    filter_hi: rx.filter_hi,
                    folder: None,
                    rtty,
                    // Always captured, even plainly simplex with no tone: a
                    // repeater memory recalled after this one has to be able to
                    // take the shift back off. `None` survives only in channels
                    // stored before this field existed, and a recall reads that
                    // as simplex too — see `RecallMemory` below.
                    repeater: Some(self.state.repeater),
                    // Only where there is a socket to choose. A front end with
                    // one antenna stores `None`, which is what tells a recall
                    // to leave the relay alone — see `MemoryChannel::antenna`.
                    antenna: (self.caps.antennas_rx.len() > 1 && !self.state.antenna_rx.is_empty())
                        .then(|| self.state.antenna_rx.clone()),
                });
                self.save_memories();
            }
            RecallMemory(id) => {
                self.stop_scan_for_operator();
                if let Some(m) = self.memories.iter().find(|m| m.id == id).cloned() {
                    // Into the config *before* the mode switch: entering RTTY
                    // builds the modem from `digi_config`, and it has to be
                    // born with the memory's setup rather than the previous one.
                    if let Some(r) = m.rtty {
                        let c = &mut self.digi_config;
                        (c.rtty_baud, c.rtty_shift_hz) = (r.baud, r.shift_hz);
                        (c.rtty_reverse, c.rtty_afc) = (r.reverse, r.afc);
                    }
                    self.apply_entry(BandStackEntry {
                        freq_hz: m.freq_hz,
                        mode: m.mode,
                        filter_lo: m.filter_lo,
                        filter_hi: m.filter_hi,
                    });
                    // After the dial, not before it: a memory stored with AUTO
                    // on resolves its shift against the frequency it is being
                    // recalled onto, and asking the band plan about the dial we
                    // are leaving would answer for the wrong band.
                    //
                    // A channel stored before the field existed carries no
                    // setup at all, and that is read as plain simplex rather
                    // than as "leave whatever is set alone" (issue #204).
                    // Nothing in the UI can express "no opinion" — the list
                    // draws such a channel exactly like a simplex one — so
                    // treating it as one is what the operator is being shown.
                    // The alternative is the dangerous reading: recalling
                    // 145.500 off a list that says 145.500 and transmitting
                    // 600 kHz down with the last repeater's tone still on.
                    self.set_repeater(m.repeater.unwrap_or_default().clamped());
                    // The antenna the channel was stored on, where this front
                    // end still has that socket. Absent means "leave it alone",
                    // unlike the repeater setup above — see
                    // `MemoryChannel::antenna` for why the two read an empty
                    // field differently.
                    if let Some(name) = m
                        .antenna
                        .filter(|n| self.caps.antennas_rx.contains(n))
                        .filter(|n| !self.source.owns_rx_antenna() && self.state.antenna_rx != *n)
                    {
                        self.apply(SetAntenna { dir: Direction::Rx, name });
                    }
                    if m.rtty.is_some() {
                        // Already in RTTY when recalled: the mode didn't change,
                        // so no rebuild happened and the live modem still holds
                        // the old setup.
                        if let Some(d) = self.digi.as_mut() {
                            d.set_config(self.digi_config.clone());
                        }
                        // Persisted and echoed like any other setup change, so
                        // the panel's controls and the next start agree with
                        // what the modem is now doing.
                        if let Err(e) = sdroxide_config::save_digi_config(&self.digi_config) {
                            warn!("saving digi config: {e}");
                        }
                        self.mark_shared_store_write();
                        self.emit_digi_status();
                    }
                }
            }
            ImportMemories(channels) => {
                // The engine owns the numbering: an id that came in over the
                // wire, or out of somebody's file, is not one this list can
                // trust not to collide with what is already stored.
                let mut next = self.memories.iter().map(|m| m.id).max().unwrap_or(0) + 1;
                let mut added = 0usize;
                for mut m in channels {
                    let name = m.name.trim().to_string();
                    if !m.freq_hz.is_finite() || m.freq_hz <= 0.0 {
                        continue;
                    }
                    // A channel already on the list is not stored twice, so
                    // re-importing an updated directory adds what is new
                    // rather than doubling what is not. Frequency and mode
                    // rather than name: two clubs name the same machine
                    // differently, and nobody wants it twice for that.
                    if self
                        .memories
                        .iter()
                        .any(|o| (o.freq_hz - m.freq_hz).abs() < 1.0 && o.mode == m.mode)
                    {
                        continue;
                    }
                    m.id = next;
                    m.name = name;
                    m.repeater = Some(m.repeater.unwrap_or_default().clamped());
                    next += 1;
                    self.memories.push(m);
                    added += 1;
                }
                if added > 0 {
                    self.save_memories();
                }
            }
            DeleteMemory(id) => {
                self.memories.retain(|m| m.id != id);
                self.save_memories();
            }
            EditMemory { id, name, freq_hz, mode, repeater, antenna } => {
                // A dial that is not a number would be stored, scanned and
                // tuned to; refuse it here rather than in each of those.
                let name = name.trim().to_string();
                if !freq_hz.is_finite() || freq_hz <= 0.0 {
                    warn!("edit memory {id}: {freq_hz} Hz is not a frequency");
                } else if let Some(m) = self.memories.iter_mut().find(|m| m.id == id) {
                    // The stored passband belongs to the mode it was stored in
                    // — a 300 Hz CW filter recalled on USB is a channel nobody
                    // can hear — so a change of mode takes the new mode's
                    // default. So does a move that flips the sideband the mode
                    // rides (SSTV below 30 MHz), where the sign of the edges
                    // *is* the sideband. Otherwise the operator's own filter
                    // survives an edit of the name.
                    let (lo, hi) = mode.default_filter_at(freq_hz);
                    if m.mode != mode || (m.filter_lo < 0.0) != (lo < 0.0) {
                        (m.filter_lo, m.filter_hi) = (lo, hi);
                    }
                    // The RTTY modem setup is only meaningful in RTTY: carried
                    // along while the memory stays there, dropped when it is
                    // edited into another mode. Edited *into* RTTY it stays
                    // `None`, which means what it has always meant — recall on
                    // whatever the modem is already set to — rather than
                    // silently capturing the setup that happens to be live.
                    if mode != Mode::Rtty {
                        m.rtty = None;
                    }
                    if !name.is_empty() {
                        m.name = name;
                    }
                    m.freq_hz = freq_hz;
                    m.mode = mode;
                    // Unlike the RTTY setup above, this is not tied to the
                    // mode: an operator may well store the shift and the tone
                    // on a channel they listen to in another mode, and the
                    // editor is where they say so. Clamped here because the
                    // command is a door a remote client can push anything
                    // through.
                    m.repeater = repeater.map(|r| r.clamped());
                    // Only a socket this front end actually has. The editor
                    // offers nothing else, but the command is a door a remote
                    // client can push anything through, and a name no antenna
                    // answers to is a channel that recalls onto nothing — so it
                    // reads as "leave the antenna alone", which is what an
                    // absent field means anyway.
                    m.antenna = antenna.filter(|n| self.caps.antennas_rx.contains(n));
                    self.save_memories();
                }
            }
            CreateMemoryFolder { name } => {
                let id = self.mem_folders.iter().map(|f| f.id).max().unwrap_or(0) + 1;
                self.mem_folders.push(MemoryFolder { id, name });
                self.save_mem_folders();
            }
            RenameMemoryFolder { id, name } => {
                if let Some(f) = self.mem_folders.iter_mut().find(|f| f.id == id) {
                    f.name = name;
                    self.save_mem_folders();
                }
            }
            DeleteMemoryFolder(id) => {
                // The folder's contents go back to the top level: deleting a
                // folder is never a way to delete a memory.
                self.mem_folders.retain(|f| f.id != id);
                for m in self.memories.iter_mut().filter(|m| m.folder == Some(id)) {
                    m.folder = None;
                }
                self.save_mem_folders();
                // …and out of the scanner's selection with it: its channels are
                // back at the top level, and a scan still naming the folder
                // would be looking for them where they are not.
                if self.scan_cfg.forget_folder(id) {
                    self.save_scanner_config();
                }
                self.save_memories();
            }
            MoveMemoryToFolder { id, folder } => {
                // Refuse a dangling folder id rather than filing the memory
                // under a folder no window would ever show.
                let exists = folder.is_none_or(|f| self.mem_folders.iter().any(|x| x.id == f));
                if exists && let Some(m) = self.memories.iter_mut().find(|m| m.id == id) {
                    m.folder = folder;
                    self.save_memories();
                }
            }
            SetSkimmerView(view) => {
                // Reject a degenerate window rather than skimming nothing at
                // all: a client mid-layout can briefly report a zero-width view.
                let view = view.filter(|(lo, hi)| hi > lo && lo.is_finite() && hi.is_finite());
                if self.skim_view != view {
                    self.skim_view = view;
                    // Not just the gate: the window itself follows the operator's
                    // waterfall, and a pan far enough out of it moves the whole
                    // slice of band the skimmers are reading.
                    self.sync_skim_window();
                }
            }
            SetSpectrumCfg(new_cfg) => {
                // The overlap is chosen from the rate *and* the scroll speed
                // ([`hop_div_for`]), so a change of speed re-sizes the hop too.
                let rebuild =
                    new_cfg.fft_size != self.cfg.fft_size || new_cfg.rows() != self.cfg.rows();
                // The zoom lane's FFT is sized from the display width
                // ([`zoom_lane_fft`]), and `ZoomLane::serves` knows nothing
                // about that — it identifies a lane by its window. So a client
                // that widens its display without moving the window would keep
                // a lane analysing for the old width. Drop it and let
                // `feed_zoom` build the replacement.
                let rewidth = new_cfg.bins() != self.cfg.bins();
                let rate = self.analyzer_rate();
                self.cfg = new_cfg;
                if rewidth {
                    self.zoom = None;
                }
                if rebuild {
                    let mut next = build_analyzer(
                        self.cfg.fft_size as usize,
                        rate,
                        self.cfg.avg_tc,
                        f64::from(self.cfg.rows()),
                        self.analyzer_view_span(),
                    );
                    // The replacement covers the same span, so the picture in
                    // hand is a true picture of it — coarser or finer than what
                    // this one will settle at, but a picture. Started empty it
                    // would answer the display floor until its first transform
                    // instead, which is the waterfall going black; and since
                    // the client grows its FFT as the operator zooms, that is
                    // once per zoom step rather than once a session.
                    next.seed_from(&self.analyzer, (0.0, 1.0));
                    self.analyzer = next;
                    self.tx_analyzer = SpectrumAnalyzer::new(
                        self.cfg.fft_size as usize,
                        TX_MONITOR_RATE,
                        self.cfg.avg_tc,
                    );
                } else {
                    self.analyzer.set_avg_tc(self.cfg.avg_tc, rate);
                    self.tx_analyzer.set_avg_tc(self.cfg.avg_tc, TX_MONITOR_RATE);
                }
                // The digital modes' channel analyser is sized from the display
                // width and the window on screen too, and it *is* the frame
                // while one of those modes is up — so a client that widens its
                // panel or zooms into the sub-band has to be given a lane that
                // resolves what it is now drawing. Rebuilt only when the answer
                // actually changes: every rebuild is a seeded restart, and the
                // viewport arrives on every frame of a drag.
                self.sync_channel_analyzer();
            }

            // Digital modes (FT8/FT4).
            SetDigiConfig(c) => {
                let c = keep_engine_owned(c, &self.digi_config);
                self.digi_config = c.clone();
                if let Some(d) = self.digi.as_mut() {
                    d.set_config(c);
                }
                // A radio that keys itself has its own speed control, and the
                // panel's WPM chip is the operator setting it.
                if self.state.rx[0].mode == Mode::Cw {
                    self.source.set_cw_wpm(self.digi_config.cw_wpm);
                }
                // Applying the setup is how a stood-down band hop is resumed —
                // the operator has said what they want the beacon to do, which
                // settles the argument over the dial that suspended it.
                self.hop_suspended = false;
                self.sync_cw_filter();
                self.sync_cw_dial();
                if let Err(e) = sdroxide_config::save_digi_config(&self.digi_config) {
                    warn!("saving digi config: {e}");
                }
                // This write covers whatever the rail had queued, so the
                // debounce has nothing left to flush.
                self.digi_dirty = false;
                self.mark_shared_store_write();
                // The network features report the same operator identity, so a
                // callsign or grid edit reaches them from here.
                self.spots.set_operator(&self.digi_config.my_call, &self.report_grid());
                // ...and so does the ADS-B lane, which needs the receiver's own
                // position to place an aircraft on the ground.
                self.sync_adsb_home();
                self.emit_digi_status();
            }
            SetDigiAudioFreq(hz) => {
                if let Some(d) = self.digi.as_mut() {
                    d.set_audio_hz(hz);
                }
                self.sync_cw_filter();
                self.sync_cw_dial();
                // Remembered against the band, and only here: this arm is the
                // operator's own route (the offset box, the nudge chips, a click
                // on a decode or the waterfall). The automatic movers never
                // reach it, which is intended — the quietest slot this over is
                // not a preference, and saving it would overwrite the figure
                // that was chosen on purpose.
                //
                // Read back from the controller rather than stored as asked,
                // because `hz` is a request: a hold refuses it outright and the
                // Fox zone floors it, so the value on the air is the only one
                // worth restoring later.
                //
                // Nothing is remembered for a mode whose offset is a standard
                // rather than a choice: RTTY's tone pair is the same everywhere,
                // so there is nothing about it that belongs to a band — and
                // writing it here would hand FT8 a 2210 Hz transmit offset the
                // next time that band came round.
                if let Some(actual) = self
                    .digi
                    .as_ref()
                    .filter(|d| !d.mode().holds_standard_tones())
                    .map(|d| d.audio_hz())
                {
                    let band = self.state.band;
                    if self.digi_config.tx_audio_hz.insert(band, actual) != Some(actual) {
                        // Saved on every change, as every other setting here is.
                        // The nudge chips can write repeatedly, which is a few
                        // hundred bytes of JSON either way; the alternative is a
                        // debounce that loses the last move on a crash.
                        if let Err(e) = sdroxide_config::save_digi_config(&self.digi_config) {
                            warn!("saving digi config: {e}");
                        }
                    }
                }
            }
            SetDigiTxLevel { mode, level } => {
                // Keyed on the mode the command carries, not on the dial: the
                // rail is dragged while transmitting, and a mode change landing
                // between the drag and this arm would write one mode's level
                // onto another's entry (see `Command::SetDigiTxLevel`).
                self.digi_config.set_tx_level(mode, level);
                // The controller keeps its own copy and `DigiStatus.config` is
                // built from it, so without this the echo below carries a stale
                // map and every client seeds from the wrong number.
                if let Some(d) = self.digi.as_mut() {
                    d.set_config(self.digi_config.clone());
                }
                // Applied now, written later. A drag emits one of these per
                // frame, and `save_digi_config` is an atomic write — temp file,
                // fsync, rename — on the thread that is also pacing transmit
                // blocks to real time. The one time an operator moves this
                // control is while transmitting and watching ALC, which is
                // exactly when that cost would land. `SetDigiAudioFreq` saves
                // immediately because nudge chips fire a handful of times; a
                // rail is not a chip.
                self.digi_dirty = true;
                self.emit_digi_status();
            }
            DigiCallCq => {
                if let Some(d) = self.digi.as_mut() {
                    d.call_cq();
                }
            }
            DigiStartQso { from, grid, snr, audio_hz, wait_for_cq } => {
                if let Some(d) = self.digi.as_mut() {
                    d.start_qso(from, grid, snr, audio_hz, wait_for_cq);
                }
            }
            DigiSetStep(step) => {
                if let Some(d) = self.digi.as_mut() {
                    d.set_step(step);
                }
            }
            DigiSendText(text) => {
                if let Some(d) = self.digi.as_mut() {
                    d.send_text(text);
                }
            }
            DigiQueueAdd { from, grid, snr, audio_hz, wait_for_cq } => {
                if let Some(d) = self.digi.as_mut() {
                    d.queue_add(sdroxide_types::QueuedCall {
                        call: from,
                        grid,
                        snr_db: snr,
                        audio_hz,
                        wait_for_cq,
                    });
                }
            }
            DigiQueueRemove(call) => {
                if let Some(d) = self.digi.as_mut() {
                    d.queue_remove(&call);
                }
            }
            DigiStopQso => {
                if let Some(d) = self.digi.as_mut() {
                    d.stop_qso();
                }
            }
            DigiAbortTx => {
                self.cancel_voice_play();
                if let Some(d) = self.digi.as_mut() {
                    d.abort_tx();
                }
                if self.digi_tx || self.state.tx.ptt {
                    self.state.tx.ptt = false;
                    self.digi_tx = false;
                    self.sync_tx_state();
                }
            }
            DigiTxText(text) => {
                if let Some(d) = self.digi.as_mut() {
                    d.set_tx_text(text);
                }
            }
            DigiClearRx => {
                if let Some(d) = self.digi.as_mut() {
                    d.clear_rx();
                    // Said now rather than left to the next poll: on a quiet
                    // channel the controller has nothing to report for seconds
                    // at a time, and a window that does not empty until the
                    // next character arrives reads as a button that missed.
                    self.emit_digi_status();
                }
            }
            DigiTxActive(on) => {
                if let Some(d) = self.digi.as_mut() {
                    d.set_tx_active(on);
                }
                // Leaving TX: if nothing is queued, drop PTT promptly.
                if !on && (self.digi_tx || self.state.tx.ptt) {
                    if self.digi.as_ref().map(|d| d.tx_burst_active()) != Some(true) {
                        self.state.tx.ptt = false;
                        self.digi_tx = false;
                        self.sync_tx_state();
                    }
                }
            }
            SstvSetMode(mode) => {
                if let Some(d) = self.digi.as_mut() {
                    d.set_sstv_mode(mode);
                }
            }
            WefaxStart => {
                if let Some(d) = self.digi.as_mut() {
                    d.wefax_start();
                }
            }
            WefaxStop => {
                if let Some(d) = self.digi.as_mut() {
                    d.wefax_stop();
                }
            }
            WefaxNudge(px) => {
                if let Some(d) = self.digi.as_mut() {
                    d.wefax_nudge(px);
                }
            }
            SstvTx { mode, png } => {
                // Decode the UI-composed PNG to RGB and queue it; the controller
                // keys TX on the next poll.
                if let Some((rgb, w, h)) = decode_png_rgb(&png) {
                    if let Some(d) = self.digi.as_mut() {
                        d.set_sstv_image(mode, rgb, w, h);
                    }
                } else {
                    warn!("SSTV TX: could not decode composed image");
                }
            }
            RifpTx { png } => {
                // Same shape as SSTV: the panel composes and we hand the
                // controller pixels. Encoding, chunking and framing are its job.
                if let Some((rgb, w, h)) = decode_png_rgb(&png) {
                    if let Some(d) = self.digi.as_mut() {
                        d.set_rifp_image(rgb, w, h);
                    }
                } else {
                    warn!("RIFP TX: could not decode composed image");
                }
            }
            RifpDropSession(session) => {
                if let Some(d) = self.digi.as_mut() {
                    d.rifp_drop_session(&session);
                }
            }
            DigiImageTx { png } => {
                // FSQ image: decode + grayscale, then queue it for the controller.
                if let Some((rgb, w, h)) = decode_png_rgb(&png) {
                    let gray: Vec<u8> = rgb
                        .chunks_exact(3)
                        .map(|p| {
                            (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) as u8
                        })
                        .collect();
                    if let Some(d) = self.digi.as_mut() {
                        d.set_image(gray, w, h);
                    }
                } else {
                    warn!("FSQ image TX: could not decode image");
                }
            }

            // Skimmers.
            SetSkimmerConfig(cfg) => {
                self.state.skimmer = cfg;
                // Remember it for the next run (and for a swap back to a
                // wideband source), before `sync_skimmer` may force the live
                // state off on an audio-mode one.
                self.skim_cfg = cfg;
                if let Err(e) = sdroxide_config::save_skimmer_config(&cfg) {
                    warn!("saving skimmer config: {e}");
                }
                // Start/stop the shared skim window, then hand the running
                // worker its per-kind enables and squelches.
                self.sync_skimmer();
                if let Some(sk) = self.skimmer.as_ref() {
                    sk.set_config(cfg);
                }
            }

            // ISM decoder.
            SetIsmConfig(cfg) => {
                let prev_rtl433 = self.state.ism.rtl433;
                self.state.ism = cfg;
                // Remembered before `sync_ism` may force the live state off on an
                // audio-mode source, so a swap back restores what was chosen.
                self.ism_cfg = cfg;
                if let Err(e) = sdroxide_config::save_ism_config(&cfg) {
                    warn!("saving ISM config: {e}");
                }
                let _ = prev_rtl433;
                self.sync_ism();
                if let Some(d) = self.ism.as_ref() {
                    d.set_config(cfg);
                }
            }

            // ADS-B decoder (issue #160).
            SetAdsbConfig(cfg) => {
                let cfg = cfg.sane();
                self.state.adsb = cfg;
                // Remembered before `sync_adsb` may force the live state off,
                // so a source swap back restores what was chosen.
                self.adsb_cfg = cfg;
                if let Err(e) = sdroxide_config::save_adsb_config(&cfg) {
                    warn!("saving ADS-B config: {e}");
                }
                self.sync_adsb();
                if let Some(d) = self.adsb.as_ref() {
                    d.set_config(cfg);
                }
            }

            // VDL Mode 2 decoder.
            SetVdl2Config(cfg) => {
                let cfg = cfg.sane();
                self.state.vdl2 = cfg;
                // Remembered before `sync_vdl2` may force the live state off, so
                // a source swap back restores what was chosen.
                self.vdl2_cfg = cfg;
                if let Err(e) = sdroxide_config::save_vdl2_config(&cfg) {
                    warn!("saving VDL2 config: {e}");
                }
                self.sync_vdl2();
                if let Some(d) = self.vdl2.as_ref() {
                    d.set_config(cfg);
                }
            }

            // AIS decoder.
            SetAisConfig(cfg) => {
                let cfg = cfg.sane();
                self.state.ais = cfg;
                // Remembered before `sync_ais` may force the live state off, so
                // a source swap back restores what was chosen.
                self.ais_cfg = cfg;
                if let Err(e) = sdroxide_config::save_ais_config(&cfg) {
                    warn!("saving AIS config: {e}");
                }
                self.sync_ais();
                if let Some(d) = self.ais.as_ref() {
                    d.set_config(cfg);
                }
            }

            SetQo100Config(cfg) => {
                self.state.qo100 = cfg;
                // Held in step with the live setting for symmetry with the
                // ISM and skimmer configs; see `qo100_cfg`'s own doc for why
                // nothing reads it back and why it is not saved to disk.
                self.qo100_cfg = cfg;
                self.sync_qo100();
                if let Some(c) = self.qo100.as_ref() {
                    c.set_config(cfg);
                }
            }

            ReloadIsmDecoders => {
                // Seeded here as well as at startup, so deleting the file to get
                // the commented example back works without a restart.
                let text = sdroxide_config::load_rtl433_flex();
                let (specs, problems) = ism_flex_parse(&text);
                let n = specs.len();
                if let Some(d) = self.ism.as_ref() {
                    d.set_flex_conf(text);
                }
                // Whatever the file had to say reaches the operator who asked
                // for the reload, not just the log.
                let msg = if problems.is_empty() {
                    format!("rtl433_flex.conf: {n} decoder(s) loaded")
                } else {
                    format!(
                        "rtl433_flex.conf: {n} decoder(s) loaded, {} refused — {}",
                        problems.len(),
                        problems.join("; ")
                    )
                };
                let _ = self.event_tx.send(RadioEvent::Notice(Some(msg)));
                return;
            }

            // Network cockpit (no RadioState change → return before the State
            // emit below).
            TestLogin(target) => self.spots.test_login(target),
            SetNetworkConfig(cfg) => {
                if let Err(e) = sdroxide_config::save_network_config(&cfg) {
                    warn!("saving network config: {e}");
                }
                self.net_cfg = cfg.clone();
                // The mailbox root does not depend on the settings, but the
                // manager may never have opened — a first run with no config
                // directory yet, say — so retry here rather than only at boot.
                match self.winlink.as_mut() {
                    Some(wl) => wl.set_config(cfg.winlink.clone()),
                    None => self.winlink = open_mailbox(&cfg.winlink),
                }
                self.spots.set_config(cfg);
                self.emit_station_config();
                self.emit_winlink_status();
                return;
            }
            SpotDialHint(hz) => {
                self.spots.set_dial(hz);
                return;
            }
            LookupCallsign { call } => {
                self.spots.lookup(call);
                return;
            }
            UploadQso { qso_id, adif, targets } => {
                self.spots.upload(qso_id, adif, targets);
                return;
            }
            SyncConfirmations => {
                self.spots.sync_confirmations();
                return;
            }

            // Built-in rigctld server (no RadioState change → return before
            // the State emit below).
            SetRigctldConfig(cfg) => {
                if let Err(e) = self.store.save_rigctld_config(&cfg) {
                    warn!("saving rigctld config: {e}");
                }
                self.rigctld_cfg = cfg;
                self.sync_rigctld();
                self.emit_station_config();
                return;
            }

            // WSJT-X UDP broadcast (no RadioState change → return before the
            // State emit below).
            SetWsjtxConfig(cfg) => {
                if let Err(e) = self.store.save_wsjtx_config(&cfg) {
                    warn!("saving WSJT-X UDP config: {e}");
                }
                self.wsjtx_cfg = cfg;
                self.sync_wsjtx();
                self.emit_station_config();
                return;
            }

            // Transmit-image presets and the received-picture stores (no
            // RadioState change → return before the State emit below).
            ImageSetSlot { slot, bytes } => {
                if slot as usize >= sdroxide_types::IMAGE_SLOTS {
                    warn!(slot, "picture upload for a slot that does not exist");
                } else if bytes.len() > sdroxide_types::IMAGE_UPLOAD_MAX {
                    // Refused here, before the bytes reach a worker: the point
                    // of the cap is not to spend anything on them at all.
                    warn!(slot, len = bytes.len(), "picture upload refused: too large");
                    let _ = self.event_tx.send(RadioEvent::Notice(Some(format!(
                        "Picture refused: {} MB is over the {} MB limit",
                        bytes.len() / 1_048_576,
                        sdroxide_types::IMAGE_UPLOAD_MAX / 1_048_576,
                    ))));
                } else {
                    self.gallery.normalise(slot, bytes);
                }
                return;
            }
            ImageClearSlot(slot) => {
                if self.images.clear(slot as usize) {
                    self.emit_image_presets();
                }
                return;
            }
            ImageSetMessage { slot, message } => {
                if self.images.set_message(slot as usize, message) {
                    self.emit_image_presets();
                }
                return;
            }
            // Answered inline: the bytes are already in memory and bounded to
            // a thousand pixels, so there is nothing here worth a thread.
            ImageGetSlot(slot) => {
                let (version, png) = self.images.source(slot as usize);
                let _ = self.event_tx.send(RadioEvent::ImageSlotSource { slot, version, png });
                return;
            }
            ImageList { kind, offset, count } => {
                self.gallery.list(kind, offset, count);
                return;
            }
            ImageGet { kind, name } => {
                self.gallery.fetch(kind, name);
                return;
            }
            ImageDelete { kind, name } => {
                self.gallery.delete(kind, name);
                return;
            }

            // ── Winlink radio email ──
            //
            // All of these answer on the event channel and none of them touch
            // `RadioState`, so they return before the State emit below.
            WinlinkAbort => {
                if let Some(wl) = self.winlink.as_mut() {
                    wl.abort();
                }
                self.emit_winlink_status();
                return;
            }
            AprsBeacon => match self.digi.as_mut() {
                Some(d) if self.state.rx[0].mode.is_aprs() => d.aprs_beacon_now(),
                _ => {}
            },
            AprsSendMessage { to, text } => match self.digi.as_mut() {
                Some(d) if self.state.rx[0].mode.is_aprs() => d.aprs_send_message(to, text),
                _ => {}
            },
            PacketBeacon => {
                match self.digi.as_mut() {
                    Some(d) if self.state.rx[0].mode.is_packet() => d.packet_beacon_now(),
                    _ => {
                        let _ = self.event_tx.send(RadioEvent::Notice(Some(
                            "switch the radio to PACKET or PACKET-HF to beacon".into(),
                        )));
                    }
                }
                return;
            }
            // The connected-mode terminal. No transmit gate of its own on any
            // of these: the frames leave through `DigiAction::KeyTx` and the
            // engine's normal PTT path, so the station interlock and the band
            // rails apply exactly as they do to a beacon. Its absence here
            // looks like an oversight, so: it is not one.
            //
            // Every other refusal — no callsign, a bad path, the link busy —
            // is written into the terminal's own transcript by the controller,
            // where the operator is looking. Only "you are not in a packet
            // mode" has to be a Notice, because in that case there is no
            // controller and so no transcript to write into.
            PacketConnect { call, via, ext } => {
                match self.digi.as_mut() {
                    Some(d) if self.state.rx[0].mode.is_packet() => {
                        d.packet_connect(call, via, ext);
                    }
                    _ => {
                        let _ = self.event_tx.send(RadioEvent::Notice(Some(
                            "switch the radio to PACKET or PACKET-HF to connect".into(),
                        )));
                    }
                }
                return;
            }
            PacketSend { text } => {
                if let Some(d) = self.digi.as_mut()
                    && self.state.rx[0].mode.is_packet()
                {
                    d.packet_send_line(text);
                }
                return;
            }
            PacketDisconnect => {
                if let Some(d) = self.digi.as_mut()
                    && self.state.rx[0].mode.is_packet()
                {
                    d.packet_disconnect();
                }
                return;
            }
            PacketTermClear => {
                if let Some(d) = self.digi.as_mut()
                    && self.state.rx[0].mode.is_packet()
                {
                    d.packet_term_clear();
                }
                return;
            }
            WinlinkConnect => {
                // Taken by value so the radio can be set up for the call
                // below: applying the gateway's channel needs the engine, and
                // the mailbox is borrowed from it.
                let Some(c) = self.winlink.as_ref().map(|wl| wl.config().clone()) else {
                    let _ = self.event_tx.send(RadioEvent::Notice(Some(
                        "the Winlink mailbox is unavailable".into(),
                    )));
                    return;
                };
                // The lane is the operator's setting rather than a
                // command parameter, so an existing client that knows
                // nothing about radio lanes still connects by telnet.
                let route = match c.lane {
                    sdroxide_types::WinlinkLane::Telnet => {
                        sdroxide_winlink::WinlinkRoute::Telnet { address: c.cms_address.clone() }
                    }
                    sdroxide_types::WinlinkLane::Packet => {
                        self.apply_gateway_channel(&c);
                        sdroxide_winlink::WinlinkRoute::Packet {
                            gateway: c.gateway.trim().to_uppercase(),
                            via: c.gateway_via.clone(),
                            baud: c.gateway_baud,
                        }
                    }
                };
                if let Some(wl) = self.winlink.as_mut()
                    && let Err(e) = wl.connect(route)
                {
                    let _ = self.event_tx.send(RadioEvent::Notice(Some(e)));
                }
                self.emit_winlink_status();
                return;
            }
            MailList { folder, offset, count } => {
                if let Some(wl) = self.winlink.as_ref() {
                    let listing = wl.list(folder, offset, count);
                    let _ = self.event_tx.send(RadioEvent::MailListing(listing));
                }
                return;
            }
            MailGet { folder, mid } => {
                if let Some(wl) = self.winlink.as_ref()
                    && let Some(msg) = wl.get(folder, &mid)
                {
                    let _ = self.event_tx.send(RadioEvent::MailMessage(Box::new(msg)));
                }
                return;
            }
            MailCompose(draft) => {
                if let Some(wl) = self.winlink.as_mut() {
                    match wl.compose(&draft) {
                        Ok(mid) => {
                            let _ = self.event_tx.send(RadioEvent::MailSaved(mid));
                            self.emit_winlink_status();
                        }
                        Err(e) => {
                            let _ = self.event_tx.send(RadioEvent::Notice(Some(e)));
                        }
                    }
                }
                return;
            }
            MailDelete { folder, mid } => {
                if let Some(wl) = self.winlink.as_mut() {
                    match wl.delete(folder, &mid) {
                        // Told to every attached client: a deleted message is
                        // gone from all of their mailbox views, not just the
                        // one that asked.
                        Ok(()) => {
                            let _ = self.event_tx.send(RadioEvent::MailDeleted { folder, mid });
                            self.emit_winlink_status();
                        }
                        Err(e) => {
                            let _ = self.event_tx.send(RadioEvent::Notice(Some(e)));
                        }
                    }
                }
                return;
            }
            MailMove { from, to, mid } => {
                if let Some(wl) = self.winlink.as_mut() {
                    match wl.move_to(from, to, &mid) {
                        Ok(()) => {
                            let _ =
                                self.event_tx.send(RadioEvent::MailDeleted { folder: from, mid });
                            self.emit_winlink_status();
                        }
                        Err(e) => {
                            let _ = self.event_tx.send(RadioEvent::Notice(Some(e)));
                        }
                    }
                }
                return;
            }

            // Built-in TCI server (no RadioState change → return before the
            // State emit below).
            SetTciServerConfig(cfg) => {
                if let Err(e) = self.store.save_tci_server_config(&cfg) {
                    warn!("saving TCI server config: {e}");
                }
                self.tci_cfg = cfg;
                self.sync_tci_server();
                self.emit_station_config();
                return;
            }

            // The operator's satellite additions (no RadioState change → return
            // before the State emit below).
            SetSatConfig(cfg) => {
                if let Err(e) = sdroxide_config::save_sat_config(&cfg) {
                    warn!("saving satellite config: {e}");
                }
                self.sat_cfg = cfg;
                self.emit_station_config();
                // The subscription list may have gained or lost a row, so the
                // status list it is drawn beside has to be rebuilt. Read from
                // the disk cache — no fetch, which is what UPDATE NOW is for.
                self.emit_tle_sub_status();
                return;
            }
            RefreshTleSubs => {
                self.start_tle_refresh();
                return;
            }

            // The satellite lock answers through its own status stream; the
            // dial move it makes emits `State` from inside the handler.
            SetSatLock(cfg) => {
                match cfg {
                    Some(cfg) => self.start_sat_lock(*cfg),
                    None => self.stop_sat_lock(),
                }
                return;
            }
            SetRotatorConfig(cfg) => {
                if let Err(e) = sdroxide_config::save_rotator_config(&cfg) {
                    warn!("saving rotator config: {e}");
                }
                self.rot_cfg = cfg;
                self.sync_rotator();
                self.emit_station_config();
                return;
            }
            SetRelayConfig(cfg) => {
                if let Err(e) = sdroxide_config::save_relay_config(&cfg) {
                    warn!("saving T/R switch config: {e}");
                }
                self.relay_cfg = *cfg;
                // Whichever engine is told opens the hardware and becomes the
                // switch's owner — *not* only the primary one.
                //
                // The primary opens it at startup because somebody has to and
                // one of them has to be chosen. But a remote client is attached
                // to one radio's session, so its `SetRelayConfig` reaches that
                // radio and no other: gating this on `primary` would mean an
                // operator who set the switch up from their second radio's tab
                // saved a configuration that never opened anything. The hub
                // holds a single driver either way, and `sync_relay` closes the
                // old one before opening the new.
                self.sync_relay();
                self.emit_station_config();
                return;
            }
            TestRelay { channel } => {
                if let Some(hub) = self.tr_switch.as_ref() {
                    hub.test(channel);
                }
                return;
            }
            SetRegion(region) => {
                if let Err(e) = sdroxide_config::save_region(region) {
                    warn!("saving IARU region: {e}");
                }
                sdroxide_types::set_region(region);
                // The band the dial is on can change without the dial moving:
                // 7.250 is 40 m in Region 2 and general coverage everywhere
                // else, and the band stack, the band buttons and the transmit
                // lockout all key off `state.band`.
                self.state.band = Band::containing(self.state.active_freq_hz());
                self.emit_station_config();
            }
            SetCessb(db) => {
                let db = db.clamp(0.0, sdroxide_types::CESSB_MAX_DB);
                if self.state.tx.cessb_db == db {
                    return;
                }
                self.state.tx.cessb_db = db;
                // Live: the processor reads the figure at the top of every
                // transmit block, so an operator can hear the difference on the
                // air rather than having to unkey and key again to try it.
            }
            SetDigiPresets(presets) => {
                // Sorted and de-duplicated on the way in, so the picker never
                // has to and two clients editing the same list converge on the
                // same file. A preset with no frequency is dropped rather than
                // saved as a row that can never be tuned.
                let mut presets: Vec<sdroxide_types::DigiPreset> =
                    presets.into_iter().filter(|p| p.dial_hz > 0.0).collect();
                presets.sort_by(|a, b| {
                    (a.mode as u8).cmp(&(b.mode as u8)).then(a.dial_hz.total_cmp(&b.dial_hz))
                });
                presets.dedup_by(|a, b| a.mode == b.mode && (a.dial_hz - b.dial_hz).abs() < 1.0);
                if let Err(e) = sdroxide_config::save_digi_presets(&presets) {
                    warn!("saving the digital-mode frequency presets: {e}");
                    self.notice(&format!("Could not save your frequency presets: {e}"));
                }
                // Process-wide, like the band plan: every radio at this station
                // offers the same list from the next lookup.
                sdroxide_types::set_digi_presets(presets);
                self.emit_station_config();
            }
            ReloadBandPlan => {
                // A missing file is seeded here as well as at startup, which is
                // what makes "delete it to get the defaults back" work without
                // a restart.
                let plan = sdroxide_config::load_band_plan();
                let custom = !plan.is_default();
                // The plan is process-wide, so a second radio at this station
                // is transmitting under the new edges from this moment too —
                // the lockout reads it at key time. Only its *displayed* band
                // lags, until it next retunes and recomputes.
                sdroxide_types::set_band_plan(plan);
                // Same reason as `SetRegion`: the dial has not moved but the
                // band under it may have.
                self.state.band = Band::containing(self.state.active_freq_hz());
                self.emit_station_config();
                // Whatever the loader had to say — a row it dropped, a file it
                // could not read — reaches the operator who asked for the
                // reload, not just the log.
                let alerts = sdroxide_config::take_load_alerts();
                if alerts.is_empty() {
                    self.notice(if custom {
                        "Band plan reloaded from bandplan.json."
                    } else {
                        "Band plan reloaded — it matches the built-in IARU tables."
                    });
                } else {
                    self.notice(&alerts.join("\n"));
                }
            }

            // This radio's own `radio.json` (no RadioState change → return
            // before the State emit below). The engine is the one that writes
            // it: the file is in *this* machine's config directory, and the
            // screen asking for the change may be in another country.
            SetRadioConfig { cfg, reopen } => {
                if let Err(e) = self.store.save_radio_config(&cfg) {
                    warn!("saving radio config: {e}");
                }
                // The per-band drive calibration is the one part of this file
                // the engine holds a copy of, because it is read on every
                // transmitted block (issue #295). Taken from what was asked
                // for rather than from the store: a save that failed leaves
                // the radio running the configuration on disk, and the trim
                // is corrected on the next reload either way.
                self.drive_trim = cfg.tx_drive_trim.clone();
                let tx_hz = self.tx_freq_told.unwrap_or_else(|| self.state.tx_freq_hz());
                self.refresh_drive_trim(tx_hz);
                // The receive trim is the other half of that: live rather than
                // at the next reopen, because an operator setting it is
                // listening to the radio while they drag the control.
                self.rx_af_gain_db = cfg.rx_audio_gain_db;
                self.refresh_af_trim();
                // Announced from the store rather than echoed from `cfg`, so
                // what every client shows is what was actually written — a
                // failed save leaves them on the configuration the radio is
                // really running, not the one that got away.
                self.emit_radio_config();
                // Where the antenna is lives in this file, and it is what
                // receptions are reported from — so an operator who has just
                // taken a receiver on the other side of the world has moved the
                // square every report goes out under (issue #284).
                self.spots.set_operator(&self.digi_config.my_call, &self.report_grid());
                self.sync_adsb_home();
                if reopen {
                    self.reopen_source();
                }
                return;
            }

            // The same rebuild with nothing to save first: the station has
            // switched this radio on or off in its roster, and the factory
            // reads the roster. No `RadioState` change of its own — what
            // changed is the front end, which announces itself.
            ReopenSource => {
                self.reopen_source();
                return;
            }
        }
        let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
    }

    /// Begin recording both sides of the QSO to a new MP3 file (RX left, TX
    /// right where a second receiver is running, dual mono where none is, or a
    /// single mixed channel if `recording_mono` is set — see `rec_split`). The
    /// filename encodes the UTC date/time, dial frequency and mode; the file
    /// lands in the user's music directory (or the config dir as a fallback).
    /// No-op if already recording; reports a [`RadioEvent::Notice`] if it
    /// can't start.
    fn start_recording(&mut self) {
        if self.recorder.is_some() {
            return;
        }
        if self.mixer.is_none() {
            let _ =
                self.event_tx.send(RadioEvent::Notice(Some("No audio output to record".into())));
            return;
        }
        let dir = match sdroxide_config::recordings_dir() {
            Ok(d) => d,
            Err(e) => {
                let _ = self
                    .event_tx
                    .send(RadioEvent::Notice(Some(format!("Recording: no directory ({e})"))));
                return;
            }
        };
        let name = self.recording_filename();
        let path = dir.join(&name);
        let channels = if self.state.recording_mono {
            RecordingChannels::Mono
        } else {
            RecordingChannels::Stereo
        };
        match Recorder::start(path, self.audio_out_rate, channels) {
            Ok((rec, prod)) => {
                let mixer = self.mixer.as_mut().expect("checked above");
                mixer.rec_tap = Some(prod);
                mixer.rec_mono = self.state.recording_mono;
                // Whatever the last recording's layout was is no evidence
                // about this one's; the first block re-latches it.
                mixer.rec_split = false;
                mixer.tx_rec_rs = MonoResampler::new(TX_MONITOR_RATE, self.audio_out_rate);
                self.recorder = Some(rec);
                self.state.recording = true;
                self.state.recording_file = Some(name);
            }
            Err(e) => {
                let _ =
                    self.event_tx.send(RadioEvent::Notice(Some(format!("Recording failed: {e}"))));
            }
        }
    }

    /// Start the operator's raw I/Q capture.
    ///
    /// Named the way SDR# names one, so the file opens in SDR#, SDRuno, HDSDR
    /// or SDRangel already tuned to where it was made — see
    /// [`crate::iq_wav`].
    fn start_iq_recording(&mut self) {
        if self.iq_wav.is_some() {
            return;
        }
        if self.audio_mode {
            let _ = self.event_tx.send(RadioEvent::Notice(Some(
                "This radio hands over demodulated audio, so there is no I/Q to record".into(),
            )));
            return;
        }
        let dir = match sdroxide_config::recordings_dir() {
            Ok(d) => d,
            Err(e) => {
                let _ = self
                    .event_tx
                    .send(RadioEvent::Notice(Some(format!("I/Q recording: no directory ({e})"))));
                return;
            }
        };
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let rate = self.state.sample_rate.max(1.0) as u32;
        let name = crate::iq_wav::capture_name(unix, self.state.center_hz, rate);
        match crate::iq_wav::IqWavWriter::create(&dir.join(&name), rate, self.state.center_hz) {
            Ok(w) => {
                info!(path = %w.path().display(), rate, "recording I/Q");
                self.iq_wav = Some(w);
                self.state.iq_recording = true;
                self.state.iq_recording_file = Some(name);
                self.state.iq_recording_mb = 0;
            }
            Err(e) => {
                let _ = self
                    .event_tx
                    .send(RadioEvent::Notice(Some(format!("I/Q recording failed: {e}"))));
            }
        }
    }

    /// Close the I/Q capture, patching its header so the file is playable.
    fn stop_iq_recording(&mut self) {
        if let Some(w) = self.iq_wav.take()
            && let Err(e) = w.finish()
        {
            warn!("closing the I/Q capture: {e}");
        }
        self.state.iq_recording = false;
        self.state.iq_recording_file = None;
        self.state.iq_recording_mb = 0;
    }

    /// Stop and finalize any active recording.
    fn stop_recording(&mut self) {
        if let Some(mixer) = self.mixer.as_mut() {
            mixer.rec_tap = None; // stop feeding before the worker drains + closes
            mixer.tx_rec_rs = None;
        }
        if let Some(rec) = self.recorder.take() {
            rec.stop();
        }
        self.state.recording = false;
        self.state.recording_file = None;
    }

    /// `sdroxide_<UTC date>_<UTC time>_<freq>_<mode>.mp3`, filesystem-safe.
    fn recording_filename(&self) -> String {
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let (y, mo, d, h, mi, s) = utc_civil(secs);
        let mhz = self.state.active_freq_hz() / 1_000_000.0;
        let mode = self.state.rx[0].mode.label().replace(['/', ' '], "");
        // All radios share one recordings directory, so two of them starting a
        // recording in the same second must not name the same file. Radio 0
        // keeps the historical name.
        let radio = match self.instance {
            0 => String::new(),
            n => format!("_radio{n}"),
        };
        format!(
            "sdroxide{radio}_{y:04}-{mo:02}-{d:02}_{h:02}-{mi:02}-{s:02}Z_{mhz:.6}MHz_{mode}.mp3"
        )
    }

    /// Construct or tear down the wideband skimmer worker: it runs while at
    /// least one kind (CW / PSK / RTTY) is enabled. The skim window is a
    /// dedicated decimation of the raw IQ, placed on the part of the band the
    /// operator is looking at ([`skim_center_for`]) and kept there by
    /// [`Engine::sync_skim_window`].
    fn sync_skimmer(&mut self) {
        // Wideband-only: an audio-mode source (a CAT rig on a sound card) has
        // only a narrow audio slice, so the skimmers stay off there — and the
        // state is corrected so the UI reflects that rather than a request the
        // engine silently ignored.
        if self.audio_mode {
            self.state.skimmer = sdroxide_types::SkimmerSettings::OFF;
        }
        match (self.state.skimmer.any_enabled(), self.skimmer.is_some()) {
            (true, false) => {
                let (ddc, center) = self.build_skim_window();
                let rate = ddc.out_rate();
                self.skimmer = Some(SkimmerController::new(
                    rate,
                    center,
                    self.state.center_hz - center,
                    self.state.skimmer,
                ));
                self.skim_ddc = Some(ddc);
                self.skim_center_hz = center;
                self.sync_skimmer_view();
                info!(rate, center, "skimmer started");
            }
            (false, true) => {
                self.skimmer = None;
                self.skim_ddc = None;
                self.skim_buf.clear();
                info!("skimmer stopped");
            }
            _ => {}
        }
    }

    /// A down-converter for the skim window as it should be placed *now*,
    /// already mixed onto it, and the absolute frequency it is centred on.
    ///
    /// The one place the chain is built, so the rate it was built from is always
    /// recorded with it.
    fn build_skim_window(&mut self) -> (Ddc, f64) {
        let mut ddc = Ddc::new(self.state.sample_rate, SKIM_TARGET_HZ);
        let center = self.skim_window_center_hz(ddc.out_rate(), None);
        ddc.set_offset_hz(center - self.state.center_hz);
        self.skim_in_rate = self.state.sample_rate;
        (ddc, center)
    }

    /// Where the skim window should sit for a window of `rate`, given where the
    /// operator is looking and where it is now.
    fn skim_window_center_hz(&self, rate: f64, current: Option<f64>) -> f64 {
        skim_center_for(
            self.skim_view,
            self.state.rx_freq_hz(),
            self.state.center_hz,
            self.state.sample_rate,
            rate,
            current,
        )
    }

    /// Re-place the skim window after a retune, a pan or a rate change, and hand
    /// the skimmers the view they are gated to.
    ///
    /// The rate is the one thing that cannot be re-pointed: the detector's bin
    /// width, its frame clock, the WPM it reads off that clock and DeepCW's whole
    /// front end are all built from it, and none can be retuned in place. So a
    /// chain built for a rate the front end has left is rebuilt rather than kept
    /// — the mistake issue #142 was in the ISM lane, where a stale window went
    /// quiet and only switching the decoder off and on put it right.
    ///
    /// Everything else moves in place. The NCO offset is re-seated on every call,
    /// even where the window has not moved in absolute terms: it is measured from
    /// the hardware centre, and on a front end wide enough to keep the view in
    /// sight either side of a retune that centre is exactly what just moved.
    fn sync_skim_window(&mut self) {
        let Some(ddc) = self.skim_ddc.as_ref() else { return };
        let want_rate = Ddc::rate_for(self.state.sample_rate, SKIM_TARGET_HZ);
        if (want_rate - ddc.out_rate()).abs() >= 1.0
            || (self.state.sample_rate - self.skim_in_rate).abs() >= 1.0
        {
            self.skimmer = None;
            self.skim_ddc = None;
            self.skim_buf.clear();
            self.sync_skimmer();
            return;
        }
        let center = self.skim_window_center_hz(want_rate, Some(self.skim_center_hz));
        if let Some(ddc) = self.skim_ddc.as_mut() {
            ddc.set_offset_hz(center - self.state.center_hz);
        }
        let moved = (center - self.skim_center_hz).abs() >= 1.0;
        self.skim_center_hz = center;
        // Sent even when the centre held: the DC spike is at a fixed frequency,
        // so a front end that retuned under a stationary window moved it inside
        // that window. The skimmers ignore a centre that has not changed, so
        // nothing is thrown away by saying so.
        if let Some(sk) = self.skimmer.as_ref() {
            sk.set_window(center, self.state.center_hz - center);
        }
        if moved {
            debug!(center, rate = want_rate, "skim window moved");
        }
        // After the placement, not before: a window that moved has just dropped
        // its tracks, and the view has to be back in place before the new ones
        // are spawned.
        self.sync_skimmer_view();
    }

    /// Hand the running skimmers the window the operator can actually see, so
    /// they only spend decoder time on signals that are on screen.
    fn sync_skimmer_view(&self) {
        if let Some(sk) = self.skimmer.as_ref() {
            sk.set_view(self.skim_view);
        }
    }

    /// Drain skimmer spots and forward them as events.
    fn poll_skimmer(&mut self) {
        let Some(sk) = self.skimmer.as_ref() else { return };
        for action in sk.poll() {
            match action {
                SkimmerAction::Spots(mut spots) => {
                    // CW spots are gated to CW segments here; PSK/RTTY spots are
                    // already gated to their per-band calling sub-bands inside the
                    // digi skimmer.
                    spots.retain(|s| match s.kind {
                        sdroxide_types::SkimmerKind::Cw => sdroxide_types::is_cw_segment(s.freq_hz),
                        _ => true,
                    });
                    let _ = self.event_tx.send(RadioEvent::SkimmerSpots(spots));
                }
            }
        }
    }

    /// Target width of the ISM decoder's window.
    ///
    /// Wide enough to hold the whole 868 MHz channel plan with the unusable band
    /// edges allowed for, and no wider: every extra hertz is decimation the engine
    /// pays for on every block. On a front end that delivers less than this the
    /// `Ddc` decimates by one and the window is simply whatever the receiver gives
    /// — which is the RX-888's VHF case, where the wideband downconverter hands
    /// over 2.025 Msps and the plan fits inside its flat portion.
    fn ism_window_target_hz(&self) -> f64 {
        self.ism_window_plan().map(|p| p.target_rate_hz).unwrap_or_else(|| {
            (sdroxide_ism::span_hz() / sdroxide_ism::USABLE_FRACTION).min(self.state.sample_rate)
        })
    }

    /// The width the window will actually be built at, which is not always the
    /// width that was asked for: a `Ddc` decimates by an integer and rounds.
    ///
    /// Taken from the plan rather than re-derived, because the plan chose that
    /// window by asking whether the channels and bands fit *this* width. Working
    /// it out again here is how the two came to disagree in issue #310.
    fn ism_window_rate_hz(&self) -> f64 {
        self.ism_window_plan()
            .map(|p| p.rate_hz)
            .unwrap_or_else(|| Ddc::rate_for(self.state.sample_rate, self.ism_window_target_hz()))
    }

    /// Where the ISM window wants to sit, given which lanes are switched on.
    ///
    /// Not a constant any more: the native decoders are fixed on the European
    /// 868 MHz channels, but the embedded rtl_433 can be pointed at 315, 345,
    /// 433 or 915 MHz, and a window aimed at 868 would never reach those. Its
    /// width is a choice too, not only the band's own default (issue #141).
    fn ism_window_plan(&self) -> Option<sdroxide_ism::WindowPlan> {
        sdroxide_ism::window_plan(&self.state.ism, self.state.center_hz, self.state.sample_rate)
    }

    /// Where to place the ISM window's centre for a given width.
    fn ism_window_center_hz(&self, rate: f64) -> f64 {
        let want = self
            .ism_window_plan()
            .map(|p| p.want_center_hz)
            .unwrap_or_else(sdroxide_ism::ideal_center_hz);
        sdroxide_ism::window_center_for(want, self.state.center_hz, self.state.sample_rate, rate)
    }

    /// Construct or tear down the ISM decoder, and keep its window on the channel
    /// plan as the front end retunes.
    ///
    /// Unlike the skimmer's window, which sits on the hardware centre and never
    /// moves, this one is placed as close to 868.9 MHz as the receiver's span
    /// allows: the channels are at fixed frequencies, so a window centred
    /// anywhere else reaches fewer of them.
    fn sync_ism(&mut self) {
        // Wideband-only, for the same reason as the skimmers: a CAT rig on a
        // sound card hands over demodulated audio, not a 1.5 MHz span. Correcting
        // the state means the UI says so instead of showing a request the engine
        // quietly dropped.
        if self.audio_mode {
            self.state.ism = sdroxide_types::IsmSettings::OFF;
        }
        let want = self.state.ism.any_enabled();
        match (want, self.ism.is_some()) {
            (true, false) => {
                let (ddc, center) = self.build_ism_window();
                let out_rate = ddc.out_rate();
                // Read from disk here rather than on the worker: the operator's
                // decoder file is config, and config loading lives on this side.
                let flex = sdroxide_config::load_rtl433_flex();
                self.ism = Some(IsmController::new(center, out_rate, self.state.ism, flex));
                self.ism_ddc = Some(ddc);
                self.ism_center_hz = center;
                info!(rate = out_rate, center, "ISM decoder started");
            }
            (false, true) => {
                self.ism = None;
                self.ism_ddc = None;
                self.ism_buf.clear();
                info!("ISM decoder stopped");
            }
            (true, true) => self.sync_ism_window(),
            _ => {}
        }
    }

    /// A down-converter for the window the plan asks for *now*, already mixed
    /// onto it, and the absolute frequency it is centred on.
    ///
    /// The one place the chain is built, so that the rate it was built from is
    /// always recorded with it.
    fn build_ism_window(&mut self) -> (Ddc, f64) {
        let target = self.ism_window_target_hz();
        let mut ddc = Ddc::new(self.state.sample_rate, target);
        let center = self.ism_window_center_hz(ddc.out_rate());
        ddc.set_offset_hz(center - self.state.center_hz);
        self.ism_in_rate = self.state.sample_rate;
        (ddc, center)
    }

    /// Re-place the window after a retune, a band change or a sample-rate change.
    ///
    /// Rebuilding the *decoder* would throw away the device table, which is the
    /// one thing an operator watching a band accumulates over minutes. So the
    /// worker is kept and only its window moves; it rebuilds only its channels.
    ///
    /// The chain that feeds it is another matter. Its width is not a constant
    /// chosen when the decoder started: selecting another band moves both where
    /// the window aims and how wide it has to be — 433 MHz wants 250 kHz where
    /// the 868 plan wants nearly two megahertz — and a device swap moves the rate
    /// being decimated from. Neither can be retuned in place, so a chain built
    /// for the old numbers is rebuilt rather than kept, which is what issue #142
    /// was: after a band change the window stayed the width the previous band
    /// needed, nothing fitted inside it, both lanes went quiet and only switching
    /// the decoder off and on put it right.
    fn sync_ism_window(&mut self) {
        let Some(ddc) = self.ism_ddc.as_ref() else { return };
        let want_rate = self.ism_window_rate_hz();
        if (want_rate - ddc.out_rate()).abs() >= 1.0
            || (self.state.sample_rate - self.ism_in_rate).abs() >= 1.0
        {
            let (ddc, center) = self.build_ism_window();
            let rate = ddc.out_rate();
            self.ism_ddc = Some(ddc);
            self.ism_center_hz = center;
            if let Some(d) = self.ism.as_ref() {
                d.set_window(center, rate);
            }
            info!(rate, center, "ISM window rebuilt");
            return;
        }

        let center = self.ism_window_center_hz(want_rate);
        let Some(ddc) = self.ism_ddc.as_mut() else { return };
        // Re-seated even when the window has not moved in absolute terms: the
        // offset is measured from the hardware centre, and on a front end wide
        // enough to keep the plan in view either side of a retune that centre is
        // exactly what just moved. Leaving the mixer where it was put the window
        // as far off the channels as the dial had travelled. Phase-continuous and
        // filter-free, so there is nothing to save by skipping it.
        ddc.set_offset_hz(center - self.state.center_hz);
        if (center - self.ism_center_hz).abs() < 1.0 {
            return;
        }
        self.ism_center_hz = center;
        if let Some(d) = self.ism.as_ref() {
            d.set_window(center, want_rate);
        }
    }

    /// The QO-100 beacon decoder's downconverter output rate for a given
    /// search width: comfortably oversampled against the 800 chip/s the
    /// beacon transmits at — see `sdroxide_qo100::bpsk`'s module doc for why
    /// the decoder wants that margin — and wide enough that the requested
    /// search actually fits under Nyquist with room to spare, so widening the
    /// window in the UI really does widen what the receiver hands the
    /// decoder rather than asking it to search past the edge of what it can
    /// see. Floored at 16 kHz, and still far short of what the ISM plan or
    /// the skimmer ask for, so it costs the engine almost nothing at the
    /// default width.
    fn qo100_target_rate_hz(&self) -> f64 {
        qo100_rate_for_cfg(&self.state.qo100)
    }

    /// Whether the QO-100 worker should be running at all: the spectral
    /// tracker or the telemetry decoder (or both) is switched on.
    fn qo100_wanted(&self) -> bool {
        self.state.qo100.enabled || self.state.qo100.decode_telemetry
    }

    /// Construct or tear down the QO-100 beacon decoder, mirroring
    /// [`Self::sync_ism`]'s shape. Simpler than the ISM window in one way —
    /// the target frequency ([`sdroxide_types::QO100_BEACON_HZ`]) never moves
    /// with the band — but not in another: the operator's search-width
    /// buttons change how wide a downconversion the decoder needs, the same
    /// reason ISM's own window resizes when the band plan does.
    fn sync_qo100(&mut self) {
        // Wideband-only, for the same reason as the skimmers and ISM: a CAT
        // rig on a sound card hands over demodulated audio, not IQ to mix a
        // downconverter from.
        if self.audio_mode {
            self.state.qo100.enabled = false;
            self.state.qo100.decode_telemetry = false;
        }
        match (self.qo100_wanted(), self.qo100.is_some()) {
            (true, false) => {
                let mut ddc = Ddc::new(self.state.sample_rate, self.qo100_target_rate_hz());
                ddc.set_offset_hz(sdroxide_types::QO100_BEACON_HZ - self.state.center_hz);
                let out_rate = ddc.out_rate();
                self.qo100 = Some(Qo100Controller::new(out_rate, self.state.qo100));
                self.qo100_ddc = Some(ddc);
                self.qo100_in_rate = self.state.sample_rate;
                info!(rate = out_rate, "QO-100 beacon decoder started");
            }
            (false, true) => {
                self.qo100 = None;
                self.qo100_ddc = None;
                self.qo100_buf.clear();
                info!("QO-100 beacon decoder stopped");
            }
            (true, true) => self.sync_qo100_window(),
            _ => {}
        }
    }

    /// Re-seat the downconverter's mixer after a retune, and rebuild it
    /// outright if the sample rate feeding it has changed or the operator has
    /// asked for a different search width — a `Ddc` bakes its input rate and
    /// its decimation chain in at construction and neither can be changed in
    /// place, the same reason `sync_ism_window` rebuilds on a rate change.
    fn sync_qo100_window(&mut self) {
        if self.qo100_ddc.is_none() {
            return;
        }
        let want_rate = self.qo100_target_rate_hz();
        let have_rate = self.qo100_ddc.as_ref().map(|d| d.out_rate()).unwrap_or(0.0);
        // `Ddc` settles for the nearest *achievable* rate, so this compares
        // against what a fresh chain would actually land on rather than the
        // bare request — a request unchanged in effect must not rebuild the
        // chain (and so drop whatever the decoder had buffered) every tick.
        let rebuild = (self.state.sample_rate - self.qo100_in_rate).abs() >= 1.0
            || (Ddc::rate_for(self.state.sample_rate, want_rate) - have_rate).abs() >= 1.0;
        if rebuild {
            let mut ddc = Ddc::new(self.state.sample_rate, want_rate);
            ddc.set_offset_hz(sdroxide_types::QO100_BEACON_HZ - self.state.center_hz);
            let out_rate = ddc.out_rate();
            self.qo100_ddc = Some(ddc);
            self.qo100_in_rate = self.state.sample_rate;
            // The controller bakes its sample rate into the worker's rolling
            // buffer sizing and every `bpsk::acquire` call, so a rate change
            // rebuilds it too rather than trying to reconfigure it in place —
            // simple, and cheap: switching search width is a deliberate,
            // infrequent click, not a hot path.
            self.qo100 = Some(Qo100Controller::new(out_rate, self.state.qo100));
            info!(rate = out_rate, "QO-100 window rebuilt");
            return;
        }
        let Some(ddc) = self.qo100_ddc.as_mut() else { return };
        ddc.set_offset_hz(sdroxide_types::QO100_BEACON_HZ - self.state.center_hz);
    }

    /// Drain the QO-100 beacon decoder's latest status, run the tracker's
    /// closed loop against it, and forward it.
    fn poll_qo100(&mut self) {
        let Some(c) = self.qo100.as_ref() else { return };
        let Some(mut status) = c.poll() else { return };

        let armed = self.state.qo100.enabled && self.state.qo100.auto_apply;
        if !armed {
            // Turning the loop off (or the whole tracker off) starts the
            // running totals fresh next time.
            self.qo100_auto = Qo100Auto::default();
        } else {
            self.qo100_auto_step(&status);
            status.auto_applying = self.state.qo100.auto_apply;
            status.auto_total_hz = self.qo100_auto.run.total_hz;
            status.auto_applies = self.qo100_auto.applies;
            status.auto_last_hz = self.qo100_auto.last_hz;
            status.auto_last_unix = self.qo100_auto.last_unix;
        }

        let _ = self.event_tx.send(RadioEvent::Qo100Status(status));
    }

    /// One pass of the tracker's closed loop: decide whether this estimate is
    /// clean and steady enough to write into the converter offset, and if so
    /// do it. Deadband + rate-limit + a two-cycle agreement check keep it
    /// from yanking a running receiver on a single bad reading — the
    /// "deadband + rare reopen" approach the QO-100 page is built around.
    fn qo100_auto_step(&mut self, status: &sdroxide_types::Qo100Status) {
        // ⛔ Never under an over. A correction *is* a front-end reopen — the
        // device is released and claimed again — and QO-100 is worked full
        // duplex, so on a station whose transmitter and receiver are the same
        // box (a Pluto, a LimeSDR) doing that unasked in the middle of a
        // transmission cuts the operator off mid-word. The manual APPLY button
        // reopens too, but an operator pressing it has chosen the moment; this
        // loop has not. Held rather than dropped: the agreement run is left
        // standing, so the correction goes out on the first cycle after the
        // over instead of having to be earned again.
        if self.on_air() {
            return;
        }

        let out = qo100_auto_correction(
            status.est_offset_hz,
            status.est_null_depth_db,
            status.est_symmetry,
            status.est_snr_db,
            self.qo100_auto.run,
            self.qo100_auto.last_apply.map(|t| t.elapsed()),
        );
        self.qo100_auto.run = out.run;

        if let Some(stop) = out.stop {
            // The loop has decided it is doing harm. Take it off the way the
            // operator would — the chip goes out on the page, because a
            // control that says it is armed while nothing is armed is worse
            // than the runaway it just stopped.
            self.state.qo100.auto_apply = false;
            warn!(?stop, total_hz = self.qo100_auto.run.total_hz, "QO-100 auto-correct disarmed");
            let _ = self.event_tx.send(RadioEvent::Notice(Some(stop.notice().to_string())));
            let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
            return;
        }

        let Some(off) = out.correction else { return };

        // `hardware_hz = dial_hz + offset`, so moving the beacon from where it
        // reads (`target + off`) back onto `target` means adding `off` to the
        // converter offset — the same maths the page's APPLY button uses.
        //
        // Which offset that is, the config decides: a transverter row covering
        // the beacon takes precedence over the single converter offset, and
        // correcting the one the receiver is not reading moves nothing (see
        // `RadioConfig::converter_offset_at_mut`).
        let mut cfg = self.store.load_radio_config();
        *cfg.converter_offset_at_mut(sdroxide_types::QO100_BEACON_HZ) += off;
        if let Err(e) = self.store.save_radio_config(&cfg) {
            warn!("qo100 auto-apply: saving radio config: {e}");
            return;
        }
        self.emit_radio_config();
        self.reopen_source();

        self.qo100_auto.last_apply = Some(Instant::now());
        self.qo100_auto.applies += 1;
        self.qo100_auto.last_hz = off;
        self.qo100_auto.last_unix = unix_now_f64() as i64;
        info!(
            correction_hz = off,
            total_hz = self.qo100_auto.run.total_hz,
            "QO-100 tracker corrected the converter offset"
        );
    }

    /// Drain the ISM decoder's device table and status and forward them.
    fn poll_ism(&mut self) {
        let Some(d) = self.ism.as_ref() else { return };
        for action in d.poll() {
            let ev = match action {
                IsmAction::Reports(r) => RadioEvent::IsmReports(r),
                IsmAction::Status(s) => RadioEvent::IsmStatus(s),
            };
            let _ = self.event_tx.send(ev);
        }
    }

    /// The rate the ADS-B window asks its down-converter for.
    ///
    /// Everything the front end delivers, up to a CPU cap — *not* a preferred
    /// figure the stream is decimated down to. Mode S is a half-microsecond
    /// chip, so samples per chip is the whole game: a receiver handing over
    /// 4 Msps decodes every arrival phase where one handing over 2.4 is merely
    /// good, and decimating to hit a target would give away the only thing that
    /// matters here.
    ///
    /// A receiver below [`sdroxide_types::ADSB_MIN_RATE_HZ`] lands on its own
    /// rate, `sync_adsb` refuses to start, and the panel says why.
    fn adsb_target_rate_hz(&self) -> f64 {
        self.state.sample_rate.min(sdroxide_types::ADSB_MAX_RATE_HZ)
    }

    /// Why the decoder will do badly here even though it can run. `None` when
    /// there is nothing to say.
    ///
    /// A different kind of statement from [`Self::adsb_unavailable`]: the lane
    /// is decoding and aircraft will appear, and the operator would otherwise
    /// have no way of knowing that the ones at the edge of range are being lost
    /// to arithmetic rather than to propagation.
    fn adsb_degraded(&self) -> Option<String> {
        let rate = Ddc::rate_for(self.state.sample_rate, self.adsb_target_rate_hz());
        if rate >= sdroxide_types::ADSB_GOOD_RATE_HZ || self.adsb_unavailable().is_some() {
            return None;
        }
        Some(format!(
            "this stream is {:.3} Msps and a Mode S chip is half a microsecond, so the \
             signal is barely sampled: the strong aircraft decode and the weak ones are \
             lost. {:.1} Msps or more is what it takes — widen the receiver's window if \
             it has the setting.",
            rate / 1e6,
            sdroxide_types::ADSB_GOOD_RATE_HZ / 1e6
        ))
    }

    /// Where the window sits: on 1090 MHz where the span reaches it, and on the
    /// hardware centre where it does not — in which case nothing decodes and
    /// [`Self::adsb_unavailable`] is what the operator is told.
    fn adsb_window_center_hz(&self, rate: f64) -> f64 {
        let want = sdroxide_types::ADSB_FREQ_HZ;
        // The window has to fit inside the stream, and the stream's outer edges
        // are where a front end's own anti-alias filter is rolling off, so the
        // usable span is not the whole of it.
        let slack = (self.state.sample_rate * ADSB_USABLE_FRACTION - rate) / 2.0;
        if slack <= 0.0 {
            return self.state.center_hz;
        }
        want.clamp(self.state.center_hz - slack, self.state.center_hz + slack)
    }

    /// Why the decoder cannot run here, if it cannot. `None` means it can.
    ///
    /// Every sentence names the number it is talking about. "No aircraft" and
    /// "this receiver was never going to hear any" produce the same empty list,
    /// and only this tells them apart.
    fn adsb_unavailable(&self) -> Option<String> {
        if self.audio_mode {
            return Some(
                "this front end hands over demodulated audio; ADS-B needs the raw I/Q stream"
                    .to_string(),
            );
        }
        if self.state.sample_rate < sdroxide_types::ADSB_MIN_RATE_HZ {
            return Some(format!(
                "ADS-B needs at least {:.1} Msps and this stream is {:.3} Msps — \
                 lower the front-end decimation, or raise the device sample rate",
                sdroxide_types::ADSB_MIN_RATE_HZ / 1e6,
                self.state.sample_rate / 1e6
            ));
        }
        // `may_rx_hz`, not `can_rx_hz`: publishing a tuning range is optional
        // and plenty of drivers never do, so an empty list means "this driver
        // didn't say" rather than "this radio tunes nowhere". Reading silence as
        // a refusal here told an SXceiver — which publishes none — that it
        // cannot reach 1090 MHz, and stood the lane down on a receiver that can.
        if !self.caps.may_rx_hz(sdroxide_types::ADSB_FREQ_HZ) {
            return Some("this receiver does not tune to 1090 MHz".to_string());
        }
        let rate = Ddc::rate_for(self.state.sample_rate, self.adsb_target_rate_hz());
        let center = self.adsb_window_center_hz(rate);
        if !sdroxide_adsb::window_covers(center, rate) {
            return Some(format!(
                "1090.000 MHz is outside the receiver's window, which is {:.3} MHz wide \
                 about {:.3} MHz",
                rate / 1e6,
                center / 1e6
            ));
        }
        None
    }

    /// A down-converter for the 1090 MHz window, already mixed onto it, and the
    /// absolute frequency it is centred on.
    ///
    /// The one place the chain is built, so the rate it was built from is always
    /// recorded with it.
    fn build_adsb_window(&mut self) -> (Ddc, f64) {
        let target = self.adsb_target_rate_hz();
        let mut ddc = Ddc::new(self.state.sample_rate, target);
        let center = self.adsb_window_center_hz(ddc.out_rate());
        ddc.set_offset_hz(center - self.state.center_hz);
        self.adsb_in_rate = self.state.sample_rate;
        (ddc, center)
    }

    /// Start or stop the ADS-B lane to match the mode and the front end.
    ///
    /// Unlike the ISM decoder, which the operator switches on and leaves running
    /// under whatever else they are doing, this one follows the *mode*: it needs
    /// the receiver parked on 1090 MHz at two and a half megasamples a second,
    /// and nothing else can be listened to through that.
    fn sync_adsb(&mut self) {
        let want = self.state.rx[0].mode.is_adsb() && self.adsb_unavailable().is_none();
        // The operator's own preference survives being overruled: `state.adsb`
        // is what the panel reads, `adsb_cfg` is what they chose.
        if self.audio_mode {
            self.state.adsb = sdroxide_types::AdsbSettings::OFF;
        } else {
            self.state.adsb = self.adsb_cfg;
        }
        match (want, self.adsb.is_some()) {
            (true, false) => {
                let (ddc, center) = self.build_adsb_window();
                let out_rate = ddc.out_rate();
                let c = AdsbController::new(center, out_rate, self.state.adsb);
                c.set_home(self.adsb_home);
                self.adsb = Some(c);
                self.adsb_ddc = Some(ddc);
                self.adsb_center_hz = center;
                info!(rate = out_rate, center, "ADS-B decoder started");
            }
            (false, true) => {
                self.adsb = None;
                self.adsb_ddc = None;
                self.adsb_buf.clear();
                info!("ADS-B decoder stopped");
            }
            (true, true) => self.sync_adsb_window(),
            _ => {}
        }
    }

    /// Re-place the window after a retune or a rate change.
    ///
    /// The aircraft table survives: a receiver nudged a hundred kilohertz is
    /// still looking at the same sky, and a target list rebuilt from nothing
    /// every time the dial moves would be worse than a second of missed frames.
    /// The chain that feeds it is another matter — a `Ddc` bakes in both its
    /// input rate and its decimation, so a change in either is a rebuild rather
    /// than a retune (the lesson of issue #142, next door).
    fn sync_adsb_window(&mut self) {
        let Some(ddc) = self.adsb_ddc.as_ref() else { return };
        let want_rate = Ddc::rate_for(self.state.sample_rate, self.adsb_target_rate_hz());
        if (want_rate - ddc.out_rate()).abs() >= 1.0
            || (self.state.sample_rate - self.adsb_in_rate).abs() >= 1.0
        {
            let (ddc, center) = self.build_adsb_window();
            let rate = ddc.out_rate();
            self.adsb_ddc = Some(ddc);
            self.adsb_center_hz = center;
            if let Some(d) = self.adsb.as_ref() {
                d.set_window(center, rate);
            }
            info!(rate, center, "ADS-B window rebuilt");
            return;
        }

        let center = self.adsb_window_center_hz(want_rate);
        let Some(ddc) = self.adsb_ddc.as_mut() else { return };
        // Re-seated even when the window has not moved in absolute terms: the
        // offset is measured from the *hardware* centre, and a retune is exactly
        // what moves that. Phase-continuous and filter-free, so there is nothing
        // to save by skipping it.
        ddc.set_offset_hz(center - self.state.center_hz);
        if (center - self.adsb_center_hz).abs() < 1.0 {
            return;
        }
        self.adsb_center_hz = center;
        if let Some(d) = self.adsb.as_ref() {
            d.set_window(center, want_rate);
        }
    }

    /// Tell the ADS-B lane where the station is, when that changes.
    ///
    /// A surface position squitter has no globally-unambiguous decode, so an
    /// aircraft on a taxiway can only be placed against a reference — and until
    /// it has been heard airborne, ours is the only one there is.
    /// The Maidenhead locator this radio's receptions are reported under, and
    /// the position its ADS-B lane places aircraft against.
    ///
    /// The station's own square when the antenna is in the shack, and the
    /// receiver's when it is not — a public KiwiSDR or SpyServer taken in this
    /// tab is somebody else's antenna on another continent, and reporting what
    /// it hears from the operator's square is how issue #284 turned a local
    /// 2 m signal into a Europe-Australia opening.
    ///
    /// Empty means there is nothing honest to say: an online receiver whose
    /// directory published no position at all. Every reporting feed already
    /// treats an empty locator as "do not start" — see
    /// `SpotManager::rebuild_psk_upload` — so nothing has to be refused twice.
    ///
    /// Read from the store rather than cached: it is the same file the settings
    /// dialog writes, and this runs at the handful of moments the identity can
    /// change, never per block.
    fn report_grid(&self) -> String {
        self.store
            .load_radio_config()
            .report_grid(&self.digi_config.my_grid)
            .unwrap_or_default()
            .to_string()
    }

    fn sync_adsb_home(&mut self) {
        let grid = self.report_grid();
        let home =
            (!grid.is_empty()).then(|| sdroxide_types::grid_to_latlon(grid.trim())).flatten();
        if home == self.adsb_home {
            return;
        }
        self.adsb_home = home;
        if let Some(d) = self.adsb.as_ref() {
            d.set_home(home);
        }
    }

    /// Drain the ADS-B decoder's aircraft table and forward it.
    ///
    /// The worker knows what it is decoding but not what the receiver could have
    /// been decoding, so the "why is this empty" fields are filled in here,
    /// where the front end's capabilities are.
    fn poll_adsb(&mut self) {
        let unavailable = self.adsb_unavailable();
        let Some(d) = self.adsb.as_ref() else {
            // Nothing running. On the ADS-B mode that is a fact worth sending —
            // it is the only way the panel can say what is wrong — but off it
            // there is nobody listening.
            //
            // Once, not per block: this runs at the front end's block rate,
            // which on a fast source is hundreds of times a second, and every
            // one of them would go to the UI and to every remote client.
            if self.state.rx[0].mode.is_adsb() && self.adsb_idle_sent.as_ref() != Some(&unavailable)
            {
                self.adsb_idle_sent = Some(unavailable.clone());
                let st = sdroxide_types::AdsbStatus {
                    unavailable,
                    suggest_center_hz: Some(sdroxide_types::ADSB_FREQ_HZ),
                    ..Default::default()
                };
                let _ = self.event_tx.send(RadioEvent::AdsbStatus(Box::new(st)));
            }
            return;
        };
        // Running again: whatever was last said about it being down is stale,
        // so a later stop says it afresh.
        self.adsb_idle_sent = None;
        let degraded = self.adsb_degraded();
        for action in d.poll() {
            let AdsbAction::Status(mut st) = action;
            st.unavailable = unavailable.clone();
            st.degraded = degraded.clone();
            st.suggest_center_hz = unavailable.is_some().then_some(sdroxide_types::ADSB_FREQ_HZ);
            let _ = self.event_tx.send(RadioEvent::AdsbStatus(st));
        }
    }

    /// The rate the VDL2 window asks its down-converter for.
    ///
    /// Capped at what the front end delivers, because a window is a decimation
    /// of that stream and not a second tuner. A receiver too narrow to hold even
    /// one channel therefore lands on its own rate, `sync_vdl2` refuses to
    /// start, and the panel says why.
    fn vdl2_target_rate_hz(&self) -> f64 {
        sdroxide_vdl2::plan::WINDOW_TARGET_RATE_HZ.min(self.state.sample_rate)
    }

    /// Where the window sits: over the channel plan where the span reaches it,
    /// and as close as the span allows otherwise.
    fn vdl2_window_center_hz(&self, rate: f64) -> f64 {
        sdroxide_vdl2::plan::window_center_for(self.state.center_hz, self.state.sample_rate, rate)
    }

    /// Why the decoder cannot run here, if it cannot. `None` means it can.
    ///
    /// Every sentence names the number it is talking about. "Nothing on the air"
    /// and "this receiver was never going to hear any of it" produce the same
    /// empty log, and only this tells them apart.
    fn vdl2_unavailable(&self) -> Option<String> {
        if self.audio_mode {
            return Some(
                "this front end hands over demodulated audio; VDL2 needs the raw I/Q stream"
                    .to_string(),
            );
        }
        if self.state.sample_rate < sdroxide_types::VDL2_MIN_RATE_HZ {
            return Some(format!(
                "VDL2 needs at least {:.0} kHz of stream and this one is {:.1} kHz — \
                 lower the front-end decimation, or raise the device sample rate",
                sdroxide_types::VDL2_MIN_RATE_HZ / 1e3,
                self.state.sample_rate / 1e3
            ));
        }
        // Silence is not a refusal; see the same check in `adsb_unavailable`.
        if !self.caps.may_rx_hz(sdroxide_types::VDL2_PLAN_CENTER_HZ) {
            return Some("this receiver does not tune to 136.8 MHz".to_string());
        }
        let rate = Ddc::rate_for(self.state.sample_rate, self.vdl2_target_rate_hz());
        let center = self.vdl2_window_center_hz(rate);
        if !sdroxide_vdl2::window_covers(center, rate) {
            return Some(format!(
                "no VDL2 channel is inside the receiver's window, which is {:.3} MHz wide \
                 about {:.3} MHz",
                rate / 1e6,
                center / 1e6
            ));
        }
        None
    }

    /// Why the decoder will do less than it could here, even though it runs.
    ///
    /// A different kind of statement from [`Self::vdl2_unavailable`]: the lane is
    /// working, and an operator who sees three channels lit out of fourteen
    /// would otherwise have to guess whether the other eleven are quiet or out
    /// of reach.
    fn vdl2_degraded(&self) -> Option<String> {
        if self.vdl2_unavailable().is_some() {
            return None;
        }
        let rate = Ddc::rate_for(self.state.sample_rate, self.vdl2_target_rate_hz());
        let center = self.vdl2_window_center_hz(rate);
        let reached = sdroxide_vdl2::plan::channels_in_window(center, rate);
        let total = sdroxide_vdl2::plan::CHANNELS.len();
        if reached.len() < total {
            // Named rather than counted, because "which ones" is the question —
            // but the window is centred on the plan, so what it misses is the
            // ends of it, and naming eleven frequencies would say less than
            // naming the two edges of what is left.
            let (lo, hi) = (reached[0], reached[reached.len() - 1]);
            let missing = total - reached.len();
            return Some(format!(
                "this window is {:.0} kHz wide and reaches {} of the {total} channels, \
                 {:.3} to {:.3} MHz — the other {missing} {} outside it",
                rate / 1e3,
                reached.len(),
                sdroxide_vdl2::plan::CHANNELS[lo].center_hz / 1e6,
                sdroxide_vdl2::plan::CHANNELS[hi].center_hz / 1e6,
                if missing == 1 { "is" } else { "are" }
            ));
        }
        let sps = Ddc::rate_for(rate, sdroxide_vdl2::plan::CHANNEL_TARGET_RATE_HZ)
            / sdroxide_types::VDL2_SYMBOL_RATE;
        if sps < sdroxide_types::VDL2_GOOD_SPS {
            return Some(format!(
                "this window leaves only {sps:.1} samples per symbol; below \
                 {:.0} the symbol timing has too little to work with",
                sdroxide_types::VDL2_GOOD_SPS
            ));
        }
        None
    }

    /// A down-converter for the VDL2 window, already mixed onto it, and the
    /// absolute frequency it is centred on.
    fn build_vdl2_window(&mut self) -> (Ddc, f64) {
        let target = self.vdl2_target_rate_hz();
        let mut ddc = Ddc::new(self.state.sample_rate, target);
        let center = self.vdl2_window_center_hz(ddc.out_rate());
        ddc.set_offset_hz(center - self.state.center_hz);
        self.vdl2_in_rate = self.state.sample_rate;
        (ddc, center)
    }

    /// Start or stop the VDL2 lane to match the mode and the front end.
    ///
    /// Follows the *mode*, like the ADS-B lane and unlike the ISM decoder: it
    /// needs the receiver parked on the datalink group, and nothing else can be
    /// listened to through that.
    fn sync_vdl2(&mut self) {
        let want = self.state.rx[0].mode.is_vdl2() && self.vdl2_unavailable().is_none();
        // The operator's own preference survives being overruled: `state.vdl2`
        // is what the panel reads, `vdl2_cfg` is what they chose.
        if self.audio_mode {
            self.state.vdl2 = sdroxide_types::Vdl2Settings::OFF;
        } else {
            self.state.vdl2 = self.vdl2_cfg;
        }
        match (want, self.vdl2.is_some()) {
            (true, false) => {
                let (ddc, center) = self.build_vdl2_window();
                let out_rate = ddc.out_rate();
                self.vdl2 = Some(Vdl2Controller::new(center, out_rate, self.state.vdl2));
                self.vdl2_ddc = Some(ddc);
                self.vdl2_center_hz = center;
                info!(rate = out_rate, center, "VDL2 decoder started");
            }
            (false, true) => {
                self.vdl2 = None;
                self.vdl2_ddc = None;
                self.vdl2_buf.clear();
                info!("VDL2 decoder stopped");
            }
            (true, true) => self.sync_vdl2_window(),
            _ => {}
        }
    }

    /// Re-place the window after a retune or a rate change.
    ///
    /// The log and the station table survive: a receiver nudged a hundred
    /// kilohertz is still listening to the same aeroplanes. The chain that feeds
    /// them is another matter — a `Ddc` bakes in both its input rate and its
    /// decimation, so a change in either is a rebuild rather than a retune.
    fn sync_vdl2_window(&mut self) {
        let Some(ddc) = self.vdl2_ddc.as_ref() else { return };
        let want_rate = Ddc::rate_for(self.state.sample_rate, self.vdl2_target_rate_hz());
        if (want_rate - ddc.out_rate()).abs() >= 1.0
            || (self.state.sample_rate - self.vdl2_in_rate).abs() >= 1.0
        {
            let (ddc, center) = self.build_vdl2_window();
            let rate = ddc.out_rate();
            self.vdl2_ddc = Some(ddc);
            self.vdl2_center_hz = center;
            if let Some(d) = self.vdl2.as_ref() {
                d.set_window(center, rate);
            }
            info!(rate, center, "VDL2 window rebuilt");
            return;
        }

        let center = self.vdl2_window_center_hz(want_rate);
        let Some(ddc) = self.vdl2_ddc.as_mut() else { return };
        // Re-seated even when the window has not moved in absolute terms: the
        // offset is measured from the *hardware* centre, and a retune is exactly
        // what moves that.
        ddc.set_offset_hz(center - self.state.center_hz);
        if (center - self.vdl2_center_hz).abs() < 1.0 {
            return;
        }
        self.vdl2_center_hz = center;
        if let Some(d) = self.vdl2.as_ref() {
            d.set_window(center, want_rate);
        }
    }

    /// Drain the VDL2 decoder's log and station table and forward them.
    ///
    /// The worker knows what it is decoding but not what the receiver could have
    /// been decoding, so the "why is this empty" fields are filled in here,
    /// where the front end's capabilities are.
    fn poll_vdl2(&mut self) {
        let unavailable = self.vdl2_unavailable();
        let Some(d) = self.vdl2.as_ref() else {
            // Nothing running. On the VDL2 mode that is a fact worth sending —
            // it is the only way the panel can say what is wrong — but off it
            // there is nobody listening. Once, not per block.
            if self.state.rx[0].mode.is_vdl2() && self.vdl2_idle_sent.as_ref() != Some(&unavailable)
            {
                self.vdl2_idle_sent = Some(unavailable.clone());
                let st = sdroxide_types::Vdl2Status {
                    unavailable,
                    suggest_center_hz: Some(sdroxide_types::VDL2_PLAN_CENTER_HZ),
                    ..Default::default()
                };
                let _ = self.event_tx.send(RadioEvent::Vdl2Status(Box::new(st)));
            }
            return;
        };
        // Running again: whatever was last said about it being down is stale.
        self.vdl2_idle_sent = None;
        let degraded = self.vdl2_degraded();
        for action in d.poll() {
            let Vdl2Action::Status(mut st) = action;
            st.unavailable = unavailable.clone();
            st.degraded = degraded.clone();
            st.suggest_center_hz = (unavailable.is_some() || degraded.is_some())
                .then_some(sdroxide_types::VDL2_PLAN_CENTER_HZ);
            let _ = self.event_tx.send(RadioEvent::Vdl2Status(st));
        }
    }

    /// The rate the AIS window asks its down-converter for.
    ///
    /// Capped at what the front end delivers, because a window is a decimation
    /// of that stream and not a second tuner. A receiver too narrow to hold even
    /// one channel therefore lands on its own rate, `sync_ais` refuses to start,
    /// and the panel says why.
    fn ais_target_rate_hz(&self) -> f64 {
        sdroxide_ais::plan::WINDOW_TARGET_RATE_HZ.min(self.state.sample_rate)
    }

    /// Where the window sits: over the pair of channels where the span reaches
    /// them, and as close as the span allows otherwise.
    fn ais_window_center_hz(&self, rate: f64) -> f64 {
        sdroxide_ais::plan::window_center_for(self.state.center_hz, self.state.sample_rate, rate)
    }

    /// Why the decoder cannot run here, if it cannot. `None` means it can.
    ///
    /// Every sentence names the number it is talking about. "Nothing on the
    /// water" and "this receiver was never going to hear any of it" produce the
    /// same empty table, and only this tells them apart.
    fn ais_unavailable(&self) -> Option<String> {
        if self.audio_mode {
            return Some(
                "this front end hands over demodulated audio; AIS needs the raw I/Q stream"
                    .to_string(),
            );
        }
        if self.state.sample_rate < sdroxide_types::AIS_MIN_RATE_HZ {
            return Some(format!(
                "AIS needs at least {:.0} kHz of stream and this one is {:.1} kHz — \
                 lower the front-end decimation, or raise the device sample rate",
                sdroxide_types::AIS_MIN_RATE_HZ / 1e3,
                self.state.sample_rate / 1e3
            ));
        }
        // Silence is not a refusal; see the same check in `adsb_unavailable`.
        if !self.caps.may_rx_hz(sdroxide_types::AIS_PLAN_CENTER_HZ) {
            return Some("this receiver does not tune to 162 MHz".to_string());
        }
        let rate = Ddc::rate_for(self.state.sample_rate, self.ais_target_rate_hz());
        let center = self.ais_window_center_hz(rate);
        if !sdroxide_ais::window_covers(center, rate) {
            return Some(format!(
                "neither AIS channel is inside the receiver's window, which is {:.0} kHz \
                 wide about {:.3} MHz",
                rate / 1e3,
                center / 1e6
            ));
        }
        None
    }

    /// Why the decoder will do less than it could here, even though it runs.
    ///
    /// A different kind of statement from [`Self::ais_unavailable`]: the lane is
    /// working, and an operator watching vessels report at half the rate they
    /// should would otherwise have to guess whether the sea is quiet or half
    /// the traffic is out of reach.
    fn ais_degraded(&self) -> Option<String> {
        if self.ais_unavailable().is_some() {
            return None;
        }
        let rate = Ddc::rate_for(self.state.sample_rate, self.ais_target_rate_hz());
        let center = self.ais_window_center_hz(rate);
        let reached = sdroxide_ais::plan::channels_in_window(center, rate);
        let total = sdroxide_ais::plan::CHANNELS.len();
        if reached.len() < total {
            let missing: Vec<String> = (0..total)
                .filter(|i| !reached.contains(i))
                .map(|i| sdroxide_ais::plan::CHANNELS[i].label.to_string())
                .collect();
            return Some(
                format!(
                    "this window is {:.0} kHz wide and reaches AIS {} only — a ship alternates \
                 between the two channels, so it will be heard at half its reporting rate",
                    rate / 1e3,
                    sdroxide_ais::plan::CHANNELS[reached[0]].label,
                ) + &format!(" (AIS {} is outside it)", missing.join(", ")),
            );
        }
        let both = reached.len() == total;
        let sps = sdroxide_ais::plan::channel_rate_for(rate, both) / sdroxide_types::AIS_BIT_RATE;
        if sps < sdroxide_types::AIS_GOOD_SPS {
            return Some(format!(
                "this window leaves only {sps:.1} samples a bit; below {:.0} the bit \
                 timing has too little to work with",
                sdroxide_types::AIS_GOOD_SPS
            ));
        }
        None
    }

    /// A down-converter for the AIS window, already mixed onto it, and the
    /// absolute frequency it is centred on.
    fn build_ais_window(&mut self) -> (Ddc, f64) {
        let target = self.ais_target_rate_hz();
        let mut ddc = Ddc::new(self.state.sample_rate, target);
        let center = self.ais_window_center_hz(ddc.out_rate());
        ddc.set_offset_hz(center - self.state.center_hz);
        self.ais_in_rate = self.state.sample_rate;
        (ddc, center)
    }

    /// Start or stop the AIS lane to match the mode and the front end.
    ///
    /// Follows the *mode*, like the ADS-B and VDL2 lanes and unlike the ISM
    /// decoder: it needs the receiver parked on 162 MHz, and nothing else can be
    /// listened to through that.
    fn sync_ais(&mut self) {
        let want = self.state.rx[0].mode.is_ais() && self.ais_unavailable().is_none();
        // The operator's own preference survives being overruled: `state.ais` is
        // what the panel reads, `ais_cfg` is what they chose.
        if self.audio_mode {
            self.state.ais = sdroxide_types::AisSettings::OFF;
        } else {
            self.state.ais = self.ais_cfg;
        }
        match (want, self.ais.is_some()) {
            (true, false) => {
                let (ddc, center) = self.build_ais_window();
                let out_rate = ddc.out_rate();
                self.ais = Some(AisController::new(center, out_rate, self.state.ais));
                self.ais_ddc = Some(ddc);
                self.ais_center_hz = center;
                info!(rate = out_rate, center, "AIS decoder started");
            }
            (false, true) => {
                self.ais = None;
                self.ais_ddc = None;
                self.ais_buf.clear();
                info!("AIS decoder stopped");
            }
            (true, true) => self.sync_ais_window(),
            _ => {}
        }
    }

    /// Re-place the window after a retune or a rate change.
    ///
    /// The vessel table survives: a receiver nudged a few kilohertz is still
    /// watching the same sea. The chain that feeds it is another matter — a
    /// `Ddc` bakes in both its input rate and its decimation, so a change in
    /// either is a rebuild rather than a retune.
    fn sync_ais_window(&mut self) {
        let Some(ddc) = self.ais_ddc.as_ref() else { return };
        let want_rate = Ddc::rate_for(self.state.sample_rate, self.ais_target_rate_hz());
        if (want_rate - ddc.out_rate()).abs() >= 1.0
            || (self.state.sample_rate - self.ais_in_rate).abs() >= 1.0
        {
            let (ddc, center) = self.build_ais_window();
            let rate = ddc.out_rate();
            self.ais_ddc = Some(ddc);
            self.ais_center_hz = center;
            if let Some(d) = self.ais.as_ref() {
                d.set_window(center, rate);
            }
            info!(rate, center, "AIS window rebuilt");
            return;
        }

        let center = self.ais_window_center_hz(want_rate);
        let Some(ddc) = self.ais_ddc.as_mut() else { return };
        // Re-seated even when the window has not moved in absolute terms: the
        // offset is measured from the *hardware* centre, and a retune is exactly
        // what moves that.
        ddc.set_offset_hz(center - self.state.center_hz);
        if (center - self.ais_center_hz).abs() < 1.0 {
            return;
        }
        self.ais_center_hz = center;
        if let Some(d) = self.ais.as_ref() {
            d.set_window(center, want_rate);
        }
    }

    /// Drain the AIS decoder's vessel table and forward it.
    ///
    /// The worker knows what it is decoding but not what the receiver could have
    /// been decoding, so the "why is this empty" fields are filled in here,
    /// where the front end's capabilities are.
    fn poll_ais(&mut self) {
        let unavailable = self.ais_unavailable();
        let Some(d) = self.ais.as_ref() else {
            // Nothing running. On the AIS mode that is a fact worth sending — it
            // is the only way the panel can say what is wrong — but off it there
            // is nobody listening. Once, not per block.
            if self.state.rx[0].mode.is_ais() && self.ais_idle_sent.as_ref() != Some(&unavailable) {
                self.ais_idle_sent = Some(unavailable.clone());
                let st = sdroxide_types::AisStatus {
                    unavailable,
                    suggest_center_hz: Some(sdroxide_types::AIS_PLAN_CENTER_HZ),
                    ..Default::default()
                };
                let _ = self.event_tx.send(RadioEvent::AisStatus(Box::new(st)));
            }
            return;
        };
        // Running again: whatever was last said about it being down is stale.
        self.ais_idle_sent = None;
        let degraded = self.ais_degraded();
        for action in d.poll() {
            let AisAction::Status(mut st) = action;
            st.unavailable = unavailable.clone();
            st.degraded = degraded.clone();
            st.suggest_center_hz = (unavailable.is_some() || degraded.is_some())
                .then_some(sdroxide_types::AIS_PLAN_CENTER_HZ);
            let _ = self.event_tx.send(RadioEvent::AisStatus(st));
        }
    }

    /// The main receiver's clean audio tap is shared: the digital-mode engine
    /// and the TCI server's RX-audio stream both read `tap_out`. Whoever wants
    /// it turns it on; it switches off only when nobody does. Every decision to
    /// enable or disable the tap goes through here — two owners writing the
    /// flag directly would silently starve one of them.
    fn sync_audio_tap(&mut self) {
        let want = self.digi.is_some() || self.tci_srv.as_ref().is_some_and(|s| s.wants_audio());
        if let Some(c) = self.main.as_mut() {
            if c.tap_enabled != want {
                c.tap_enabled = want;
                if !want {
                    c.tap_out.clear();
                } else {
                    // Re-seed the tap's level from the first block it sees
                    // rather than from wherever the band was when the tap was
                    // last switched off, which may be a session ago.
                    c.tap_level_db = f32::NAN;
                }
            }
        }
    }

    /// Start, stop or rebind the rigctld server to match `rigctld_cfg`.
    fn sync_rigctld(&mut self) {
        match (self.rigctld_cfg.enabled, self.rigctld.is_some()) {
            (true, true) => {
                if self.rigctld.as_ref().map(|s| s.addr()) != Some(self.rigctld_cfg.addr().as_str())
                {
                    // Address changed: drop first so the old port is released
                    // before we try to take the new one.
                    self.rigctld = None;
                    self.start_rigctld();
                } else if let Some(s) = self.rigctld.as_ref() {
                    // Transmit permission and the client limit apply live.
                    s.set_config(self.rigctld_cfg.clone());
                    self.emit_rigctld_status();
                }
            }
            (true, false) => self.start_rigctld(),
            (false, true) => {
                self.rigctld = None;
                self.rigctld_seen = None;
                info!("rigctld server stopped");
                self.emit_rigctld_status();
            }
            (false, false) => self.emit_rigctld_status(),
        }
    }

    fn start_rigctld(&mut self) {
        self.rigctld_err = None;
        let snap = self.rigctld_snapshot();
        match RigctldController::start(&self.rigctld_cfg, snap.clone()) {
            Ok(srv) => {
                info!(addr = %self.rigctld_cfg.addr(), "rigctld server started");
                self.rigctld = Some(srv);
                self.rigctld_seen = Some(RigDigest::of(&snap));
            }
            Err(e) => {
                // By far the most common first-run failure is a real rigctld
                // already holding 4532, so say so rather than leaving the
                // operator with a bare "address in use".
                warn!("rigctld server: {e}");
                let hint = if self.rigctld_cfg.port == 4532 {
                    format!("{e} — a real rigctld may already own this port")
                } else {
                    e
                };
                self.rigctld_err = Some(hint);
            }
        }
        self.emit_rigctld_status();
    }

    fn emit_rigctld_status(&self) {
        let _ = self.event_tx.send(RadioEvent::RigctldStatus {
            running: self.rigctld.is_some(),
            addr: self
                .rigctld
                .as_ref()
                .map(|s| s.addr().to_string())
                .unwrap_or_else(|| self.rigctld_cfg.addr()),
            clients: self.rigctld.as_ref().map(|s| s.clients()).unwrap_or(0),
            error: self.rigctld_err.clone(),
        });
    }

    /// The slice of state rigctld clients see.
    fn rigctld_snapshot(&self) -> RigState {
        let rx = &self.state.rx[0];
        RigState {
            vfo_a_hz: self.state.vfo_a_hz,
            vfo_b_hz: self.state.vfo_b_hz,
            active_vfo: self.state.active_vfo,
            split: self.state.split,
            mode: rx.mode,
            filter_lo: rx.filter_lo,
            filter_hi: rx.filter_hi,
            ptt: self.state.tx.ptt,
            tune: self.state.tx.tune,
            rit_hz: self.state.rit.effective_hz() as i32,
            xit_hz: self.state.xit.effective_hz() as i32,
            drive: self.state.tx.drive,
            volume: rx.volume,
            mic_gain: self.state.tx.mic_gain,
            band: self.state.band,
            muted: rx.muted,
            strength_dbm: self.last_s_dbm.round() as i32,
            noise_blanker: self.state.noise_blanker,
            noise_reduction: rx.noise_reduction.is_on(),
            auto_notch: rx.auto_notch,
            can_tx: self.caps.is_transmit_capable(),
            rx_ranges: self.caps.freq_ranges_rx.clone(),
            tx_ranges: self.caps.freq_ranges_tx.clone(),
        }
    }

    /// Service the rigctld server: carry out what clients asked for and
    /// republish the state they read.
    fn poll_rigctld(&mut self) {
        let Some(srv) = self.rigctld.as_ref() else { return };
        let mut clients_changed = false;
        for req in srv.poll() {
            match req {
                // Everything a client can command goes through `apply`, so the
                // ham-band guard, frequency-range checks and the usual state
                // broadcast to the GUI all apply unchanged.
                sdroxide_rigctld::ServerRequest::Cmd(c) => self.apply(c),
                sdroxide_rigctld::ServerRequest::Clients(_) => clients_changed = true,
            }
        }
        if clients_changed {
            self.emit_rigctld_status();
        }
        // Cheap first: the digest is all Copy scalars, so an idle radio costs
        // a comparison rather than two Vec clones per audio block.
        let digest = self.rigctld_digest();
        if self.rigctld_seen != Some(digest) {
            let snap = self.rigctld_snapshot();
            if let Some(srv) = self.rigctld.as_ref() {
                srv.publish_state(snap);
            }
            self.rigctld_seen = Some(digest);
        }
    }

    fn rigctld_digest(&self) -> RigDigest {
        let rx = &self.state.rx[0];
        RigDigest {
            vfo_a: self.state.vfo_a_hz.to_bits(),
            vfo_b: self.state.vfo_b_hz.to_bits(),
            active_b: self.state.active_vfo == sdroxide_types::Vfo::B,
            split: self.state.split,
            mode: rx.mode,
            filter_lo: rx.filter_lo.to_bits(),
            filter_hi: rx.filter_hi.to_bits(),
            ptt: self.state.tx.ptt,
            tune: self.state.tx.tune,
            rit: self.state.rit.effective_hz() as i32,
            xit: self.state.xit.effective_hz() as i32,
            drive: self.state.tx.drive.to_bits(),
            volume: rx.volume.to_bits(),
            mic_gain: self.state.tx.mic_gain.to_bits(),
            band: self.state.band,
            muted: rx.muted,
            // Quantised to whole dB: the raw reading dithers continuously, and
            // a client can only see integers anyway.
            strength: self.last_s_dbm.round() as i32,
            noise_blanker: self.state.noise_blanker,
            noise_reduction: rx.noise_reduction.is_on(),
            auto_notch: rx.auto_notch,
            ranges: (self.caps.freq_ranges_rx.len(), self.caps.freq_ranges_tx.len()),
        }
    }

    /// Start, stop or rebind the built-in TCI server to match `tci_cfg`.
    /// Mirrors [`Engine::sync_skimmer`].
    fn sync_tci_server(&mut self) {
        match (self.tci_cfg.enabled, self.tci_srv.is_some()) {
            (true, true) => {
                if self.tci_srv.as_ref().map(|s| s.addr()) != Some(self.tci_cfg.addr().as_str()) {
                    // Address changed: drop first so the old port is released
                    // before we try to take the new one (they may be the same
                    // port on a different interface).
                    self.tci_srv = None;
                    self.start_tci_server();
                } else if let Some(s) = self.tci_srv.as_ref() {
                    // Client limit and transmit permission apply live.
                    s.set_config(self.tci_cfg.clone());
                    self.emit_tci_status();
                }
            }
            (true, false) => self.start_tci_server(),
            (false, true) => {
                self.tci_srv = None;
                self.tci_iq_ddc = None;
                self.tci_iq_buf.clear();
                self.tci_aud_rs = None;
                self.tci_aud_in_rate = 0.0;
                self.tci_tx = false;
                self.tci_last_snap = None;
                self.sync_audio_tap();
                info!("TCI server stopped");
                self.emit_tci_status();
            }
            (false, false) => self.emit_tci_status(),
        }
    }

    fn start_tci_server(&mut self) {
        self.tci_srv_err = None;
        // The most likely first-run failure by far: sdroxide is itself a TCI
        // client of a rig on this machine, and that rig already owns the port.
        // Diagnose it rather than leaving the operator with "address in use".
        if let Some(conflict) = self.tci_backend_conflict() {
            self.tci_srv_err = Some(conflict);
            self.emit_tci_status();
            return;
        }
        let snap = self.tci_snapshot();
        match TciServerController::start(&self.tci_cfg, &self.caps, snap.clone()) {
            Ok(srv) => {
                info!(addr = %self.tci_cfg.addr(), "TCI server started");
                self.tci_srv = Some(srv);
                self.tci_last_snap = Some(snap);
            }
            Err(e) => {
                warn!("TCI server: {e}");
                self.tci_srv_err = Some(e);
            }
        }
        self.emit_tci_status();
    }

    /// The address we'd bind, when it is the very rig we are connected to as a
    /// TCI client.
    fn tci_backend_conflict(&self) -> Option<String> {
        // This radio's scope, not the station's: on a station with more than
        // one radio the free function is radio 0's file, so radio 1's server
        // would be diagnosed against radio 0's backend — and either accuse it
        // of a clash it does not have, or miss its own.
        let radio = self.store.load_radio_config();
        if radio.backend != sdroxide_types::Backend::Tci {
            return None;
        }
        // The client address may omit the port (defaulting to 50001) and may
        // name localhost in any of its spellings.
        let (host, port) = match radio.tci.address.trim().rsplit_once(':') {
            Some((h, p)) => (h.trim().to_string(), p.trim().parse::<u16>().unwrap_or(50_001)),
            None => (radio.tci.address.trim().to_string(), 50_001),
        };
        let local = |h: &str| matches!(h, "127.0.0.1" | "localhost" | "::1" | "0.0.0.0" | "");
        if port == self.tci_cfg.port && local(&host) && local(&self.tci_cfg.bind) {
            return Some(format!(
                "port {port} is the TCI radio sdroxide is connected to — pick another port"
            ));
        }
        None
    }

    fn emit_tci_status(&self) {
        let _ = self.event_tx.send(RadioEvent::TciServerStatus {
            running: self.tci_srv.is_some(),
            addr: self.tci_cfg.addr(),
            clients: self.tci_srv.as_ref().map(|s| s.clients()).unwrap_or(0),
            error: self.tci_srv_err.clone(),
        });
    }

    /// The slice of state TCI clients see.
    fn tci_snapshot(&self) -> TciStateSnapshot {
        // An audio-mode source (a CAT rig on a sound card) has no wideband IQ,
        // so a zero rate tells the server not to advertise the stream at all.
        // Otherwise report the live decimation, or — before anyone has
        // subscribed — the rate we *would* deliver, since a client that sees no
        // `iq_samplerate` concludes there is no IQ stream and never asks.
        let iq_rate = if self.audio_mode {
            0
        } else {
            match self.tci_iq_ddc.as_ref() {
                Some(d) => d.out_rate() as u32,
                None => Ddc::rate_for(self.state.sample_rate, TCI_IQ_DEFAULT_HZ) as u32,
            }
        };
        let span = if self.audio_mode { self.audio_bw } else { self.state.sample_rate } / 2.0;
        let (lo, hi) = self
            .caps
            .freq_ranges_rx
            .iter()
            .fold((f64::MAX, 0.0f64), |(lo, hi), &(a, b)| (lo.min(a), hi.max(b)));
        TciStateSnapshot {
            vfo_a_hz: self.state.vfo_a_hz,
            vfo_b_hz: self.state.vfo_b_hz,
            center_hz: self.state.center_hz,
            if_span_hz: span,
            mode: self.state.rx[0].mode,
            split: self.state.split,
            ptt: self.state.tx.ptt,
            tune: self.state.tx.tune,
            drive_pct: (self.state.tx.drive * 100.0).round().clamp(0.0, 100.0) as u32,
            tune_drive_pct: (self.state.tx.tune_drive * 100.0).round().clamp(0.0, 100.0) as u32,
            muted: self.state.rx[0].muted,
            volume_db: TciStateSnapshot::volume_db_from(self.state.rx[0].volume),
            iq_rate,
            vfo_lo_hz: if lo == f64::MAX { 0.0 } else { lo },
            vfo_hi_hz: hi,
            can_tx: self.caps.is_transmit_capable(),
        }
    }

    /// Service the TCI server: carry out what clients asked for, keep the IQ
    /// decimation matched to their subscription, and publish any state change.
    fn poll_tci_server(&mut self) {
        let Some(srv) = self.tci_srv.as_ref() else { return };
        let mut clients_changed = false;
        for req in srv.poll() {
            match req {
                // Everything a client can command goes through `apply`, so the
                // ham-band guard, frequency-range checks and the usual state
                // broadcast to the GUI all apply unchanged.
                ServerRequest::Cmd(c) => self.apply(c),
                ServerRequest::Key(on) => self.tci_key(on),
                ServerRequest::Clients(_) => clients_changed = true,
            }
        }
        if clients_changed {
            self.emit_tci_status();
        }
        // The tail of an over whose client has already unkeyed: end it the
        // moment there is no longer a whole block of its audio left, so the
        // transmitter stops on the client's last sample rather than on its PTT.
        if self.tci_tx_drain && self.mic_fifo.len() < TX_AUDIO_BLOCK {
            self.end_tci_tx();
            self.apply(Command::SetPtt(false));
        }
        // A client that stopped feeding audio without unkeying would otherwise
        // leave us transmitting silence indefinitely.
        if self.tci_tx_starved > TCI_TX_STARVE_LIMIT {
            warn!("TCI client stopped sending TX audio; unkeying");
            self.end_tci_tx();
            self.apply(Command::SetPtt(false));
        }
        self.sync_tci_iq();
        self.sync_audio_tap();
        self.broadcast_tci_state();
    }

    /// Build, retune or drop the IQ decimation feeding TCI clients, following
    /// the rate they asked for. The `Ddc` snaps to an integer decimation of the
    /// device rate, so the rate we report back is rarely the round number they
    /// requested — which is exactly what the `iq_samplerate` echo is for.
    fn sync_tci_iq(&mut self) {
        // Wideband only: an audio-mode source has no IQ to decimate.
        let want =
            if self.audio_mode { None } else { self.tci_srv.as_ref().and_then(|s| s.wants_iq()) };
        match want {
            Some(rate) => {
                // Rebuild only when the snapped result would actually differ —
                // `rate_for` answers that without building the filters.
                let target = Ddc::rate_for(self.state.sample_rate, rate as f64);
                if self.tci_iq_ddc.as_ref().map(|d| d.out_rate()) != Some(target) {
                    let ddc = Ddc::new(self.state.sample_rate, rate as f64);
                    info!(requested = rate, actual = ddc.out_rate(), "TCI server IQ stream");
                    self.tci_iq_ddc = Some(ddc);
                }
            }
            None => {
                if self.tci_iq_ddc.is_some() {
                    self.tci_iq_ddc = None;
                    self.tci_iq_buf.clear();
                    self.tci_iq_ilv.clear();
                }
            }
        }
    }

    /// Hand the transmitter to a TCI client, or take it back.
    ///
    /// Keying never touches `tx_active` directly: it goes through
    /// [`Command::SetPtt`] so `sync_tx_state`'s guards (transmit-capable
    /// device, frequency in range, `tx_ham_only`) decide, exactly as they do
    /// for the operator. A refusal is reflected straight back to the client.
    fn tci_key(&mut self, on: bool) {
        if !on {
            // The client's unkey says it has stopped *sending*, not that what
            // it already handed over may be thrown away: the transmitter runs
            // up to [`TCI_TX_LEAD`] behind the client's own waveform, and
            // cutting there takes the tail off every burst — which on a timed
            // digital mode is the whole decode. Play out what is in hand
            // first; `poll_tci_server` unkeys when it has run dry.
            if self.tci_tx && !self.tci_tx_drain && self.mic_fifo.len() >= TX_AUDIO_BLOCK {
                self.tci_tx_drain = true;
                if let Some(s) = self.tci_srv.as_mut() {
                    s.drain_tx_audio();
                    s.deny_tx();
                }
                return;
            }
            self.end_tci_tx();
            self.apply(Command::SetPtt(false));
            return;
        }
        // Refuse rather than interrupt: the operator, a digital-mode burst and
        // TUNE all outrank a network client.
        if !self.tci_cfg.allow_tx || self.digi_tx || self.state.tx.ptt || self.state.tx.tune {
            if let Some(s) = self.tci_srv.as_ref() {
                s.deny_tx();
            }
            return;
        }
        self.apply(Command::SetPtt(true));
        if self.state.tx.ptt {
            self.tci_tx = true;
            self.tci_tx_starved = 0;
            self.tci_tx_drain = false;
            // Nothing from the last over is owed to this one.
            self.tci_pace.rekey();
            // Start from an empty ring so a previous over's tail can't play.
            if let Some(s) = self.tci_srv.as_mut() {
                s.drain_tx_audio();
            }
        } else if let Some(s) = self.tci_srv.as_ref() {
            // A safety rail refused; tell the client so it stops streaming.
            s.deny_tx();
        }
    }

    /// Say so when an over went out with nothing on the microphone.
    ///
    /// The one failure a transmit meter cannot show, because there is nothing
    /// to show: the modulator was handed silence, so the transmitter made a few
    /// milliwatts of nothing and every other control — the drive slider, the
    /// power register, TUNE — reads exactly as it should. That is issue #215,
    /// reported as "TUNE and FT8 make full power and SSB makes milliwatts",
    /// which is what a microphone opening on the wrong sound card looks like
    /// from the operator's side.
    ///
    /// Voice overs only, and only ones long enough to have said something: a
    /// digital burst has no microphone by design, and neither has a tune.
    fn report_silent_microphone(&mut self) {
        let blocks = std::mem::take(&mut self.voice_blocks);
        let peak = std::mem::take(&mut self.voice_peak);
        // A tenth of a second, so a keyed-and-released PTT is not a complaint.
        if blocks < 10 || peak > SILENT_MIC_PEAK {
            return;
        }
        let what = if self.tci_tx {
            "the TCI client sent none"
        } else if self.mic.is_some() {
            "the microphone sent nothing"
        } else {
            "there is no microphone open"
        };
        warn!(peak, blocks, "voice over with a silent microphone");
        let _ = self.event_tx.send(RadioEvent::Notice(Some(format!(
            "That over went out with no audio — {what}. Pick the right input under \
             Settings → General."
        ))));
    }

    /// End a TCI-driven over: stop sourcing from the client, discard what it
    /// queued, and tell it we are no longer transmitting on its behalf.
    /// Idempotent — every path that could end the over calls it.
    fn end_tci_tx(&mut self) {
        if !self.tci_tx {
            return;
        }
        self.tci_tx = false;
        self.tci_tx_starved = 0;
        self.tci_tx_drain = false;
        self.tci_pace.rekey();
        self.mic_fifo.clear();
        if let Some(s) = self.tci_srv.as_mut() {
            s.drain_tx_audio();
            s.deny_tx();
        }
    }

    /// Publish the current state to TCI clients when it has changed.
    ///
    /// One diff per tick rather than emits scattered through `apply`: this also
    /// catches a CAT rig's dial being turned (`apply_control`), a device swap,
    /// and a transmit request the safety rails refused — none of which any
    /// single command handler sees.
    fn broadcast_tci_state(&mut self) {
        let Some(srv) = self.tci_srv.as_ref() else { return };
        let snap = self.tci_snapshot();
        if self.tci_last_snap.as_ref() != Some(&snap) {
            srv.broadcast_state(snap.clone());
            self.tci_last_snap = Some(snap);
        }
    }

    /// The contest serial has moved: give it to the controller, write it, and
    /// tell every client.
    ///
    /// All three, because the number has three homes that must agree — the
    /// controller composes the next exchange from it, the file is what a
    /// restart mid-contest reads, and `DigiStatus.config` is what a panel's
    /// readout shows. Its own method because both routes that change it (a
    /// contact logged, the operator setting it) need all three.
    fn push_contest_serial(&mut self) {
        if let Some(d) = self.digi.as_mut() {
            d.set_config(self.digi_config.clone());
        }
        if let Err(e) = sdroxide_config::save_digi_config(&self.digi_config) {
            warn!("saving digi config: {e}");
        }
        self.mark_shared_store_write();
        self.emit_digi_status();
    }

    fn emit_digi_status(&self) {
        if let Some(d) = self.digi.as_ref() {
            let _ = self.event_tx.send(RadioEvent::Ft8Status(d.status()));
        }
    }

    fn emit_image_presets(&self) {
        let _ = self.event_tx.send(RadioEvent::ImagePresets(self.images.status()));
    }

    /// Announce everything this station persists, as one snapshot.
    ///
    /// Sent at startup and after every change rather than answered on request:
    /// the settings dialog may be running on another machine, and the files
    /// these describe are only reachable from this one.
    fn emit_station_config(&self) {
        let _ = self.event_tx.send(RadioEvent::StationConfig(Box::new(
            sdroxide_types::StationConfig {
                net: self.net_cfg.clone(),
                rigctld: self.rigctld_cfg.clone(),
                tci_server: self.tci_cfg.clone(),
                wsjtx: self.wsjtx_cfg.clone(),
                sat: self.sat_cfg.clone(),
                rotator: self.rot_cfg.clone(),
                relay: self.relay_cfg.clone(),
                region: sdroxide_types::region(),
                band_plan: sdroxide_types::band_plan().clone(),
                digi_presets: sdroxide_types::digi_presets().to_vec(),
            },
        )));
    }

    /// Announce this radio's `radio.json` — which interface is open and how
    /// every backend is configured.
    ///
    /// Read back from the store rather than kept in memory. The engine does not
    /// otherwise hold this file (the [`ReopenFn`] factory loads it afresh on
    /// every open, and several backends read their own section as they start),
    /// so the store is the only copy there is, and reading it is what makes an
    /// announcement mean "this is what is on disk" rather than "this is what
    /// somebody asked for".
    ///
    /// Sent at startup and after every change, for [`Engine::emit_station_config`]'s
    /// reason: the settings dialog may be running on another machine, and this
    /// file is only reachable from this one.
    fn emit_radio_config(&self) {
        let _ =
            self.event_tx.send(RadioEvent::RadioConfig(Box::new(self.store.load_radio_config())));
    }

    /// Announce what each TLE subscription's cached listing holds. Reads the
    /// disk cache the tracker fetches into — no network, so this is free to
    /// send alongside every config change.
    fn emit_tle_sub_status(&self) {
        let status = sdroxide_solar::tlesub::status_all(&self.sat_cfg.subs);
        let _ = self.event_tx.send(RadioEvent::TleSubStatus(status));
    }

    /// Re-fetch every enabled TLE subscription, off the engine thread.
    ///
    /// One HTTPS round trip per subscription, so it cannot run inline: a dozen
    /// listings behind a slow link would stall the receiver for seconds. A
    /// refresh already in flight is left to finish rather than being stacked
    /// on — the operator pressing UPDATE NOW twice wants one answer, not two.
    fn start_tle_refresh(&mut self) {
        if self.tle_refresh.as_ref().is_some_and(|h| !h.is_finished()) {
            return;
        }
        let subs = self.sat_cfg.subs.clone();
        self.tle_refresh = std::thread::Builder::new()
            .name("sdroxide-tlesub".into())
            .spawn(move || sdroxide_solar::tlesub::refresh_all(&subs))
            .map_err(|e| warn!("could not start the TLE refresh: {e}"))
            .ok();
    }

    /// Publish a finished TLE refresh. Called once per tick; a no-op unless a
    /// refresh thread has just come home.
    fn poll_tle_refresh(&mut self) {
        if !self.tle_refresh.as_ref().is_some_and(|h| h.is_finished()) {
            return;
        }
        let status = match self.tle_refresh.take().expect("checked above").join() {
            Ok(v) => v,
            Err(_) => {
                warn!("the TLE refresh thread panicked");
                return;
            }
        };
        let _ = self.event_tx.send(RadioEvent::TleSubStatus(status));
    }

    // ── Satellite lock ──────────────────────────────────────────────────────

    /// Begin (or re-shape) a satellite lock: resolve the element set and the
    /// observer, put the dial on the nominal downlink, and run the first
    /// computation synchronously so the answer to `SetSatLock` is a live
    /// status rather than a promise of one.
    fn start_sat_lock(&mut self, cfg: sdroxide_types::SatLockConfig) {
        let Some(sat) = self.resolve_sat_tle(&cfg) else {
            let _ = self.event_tx.send(RadioEvent::Notice(Some(format!(
                "No current elements for {} — refresh the TLE subscriptions in Settings ▸ TLE",
                cfg.name
            ))));
            return;
        };
        let observer =
            cfg.observer.or_else(|| sdroxide_types::grid_to_latlon(&self.digi_config.my_grid));
        let Some(observer) = observer else {
            let _ = self.event_tx.send(RadioEvent::Notice(Some(
                "The satellite lock needs your grid locator — set it in Settings ▸ General".into(),
            )));
            return;
        };
        if self.audio_mode && cfg.doppler {
            let _ = self.event_tx.send(RadioEvent::Notice(Some(
                "CAT rig: satellite tracking runs, but Doppler correction needs an IQ front end"
                    .into(),
            )));
        }
        self.stop_scan_for_operator();
        // The dial goes to the nominal downlink through the normal tuning
        // path, so the span check and a possible hardware retune behave
        // exactly as a hand tune would. Only on a *new* lock: the same config
        // arrives again whenever a client flips its doppler or rotator
        // switch, and yanking the dial back to where the lock started would
        // undo the operator's tuning across the passband.
        let same_bird = self.sat_lock.as_ref().is_some_and(|l| l.cfg.norad_id == cfg.norad_id);
        if cfg.downlink_hz > 0.0 && !same_bird {
            self.state.active_vfo = Vfo::A;
            self.state.vfo_a_hz = cfg.downlink_hz;
            self.state.band = Band::containing(cfg.downlink_hz);
            self.keep_vfo_in_span();
        }
        let track = sdroxide_types::SatTrackStatus {
            norad_id: cfg.norad_id,
            name: cfg.name.clone(),
            downlink_hz: self.state.active_freq_hz(),
            ..Default::default()
        };
        self.sat_lock = Some(ActiveSatLock {
            cfg,
            sat,
            observer,
            track,
            // Everything due immediately: the first `poll_sat_track` below
            // computes, applies and emits in one go.
            next_calc: Instant::now(),
            next_emit: Instant::now(),
            next_slow_unix: 0.0,
        });
        self.update_tuning();
        let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
        self.poll_sat_track();
    }

    /// Release the lock: take the corrections out of the signal path and say
    /// so, symmetric with the status stream a live lock produces.
    fn stop_sat_lock(&mut self) {
        if self.sat_lock.take().is_none() {
            return;
        }
        if let Some(tx) = self.tx.as_mut() {
            tx.sat_nco = None;
        }
        self.update_tuning();
        // The antenna stops chasing a satellite nobody is locked to.
        self.drive_rotator();
        let _ = self.event_tx.send(RadioEvent::SatTrack(None));
    }

    /// Find the freshest element set for a lock's catalogue number: an
    /// explicit TLE the client sent, the operator's pasted sets, and every
    /// enabled subscription's cached listing all compete, newest epoch wins.
    /// Newest-wins is what lets a lock recover from stale elements the moment
    /// a subscription refresh lands on disk.
    fn resolve_sat_tle(
        &self,
        cfg: &sdroxide_types::SatLockConfig,
    ) -> Option<sdroxide_solar::Satellite> {
        use sdroxide_solar::satellites::{parse_pasted_tles, parse_subscribed_tles};
        let mut best: Option<sdroxide_solar::Satellite> = None;
        let mut consider = |s: sdroxide_solar::Satellite| {
            if s.norad_id == cfg.norad_id
                && best.as_ref().is_none_or(|b| s.epoch_unix > b.epoch_unix)
            {
                best = Some(s);
            }
        };
        if let Some((l1, l2)) = &cfg.tle {
            for s in parse_pasted_tles(&format!("{}\n{l1}\n{l2}", cfg.name)) {
                consider(s);
            }
        }
        for s in parse_pasted_tles(&self.sat_cfg.tle_text()) {
            consider(s);
        }
        let cache = sdroxide_solar::cache::Cache::open();
        for sub in self.sat_cfg.live_subs().filter(|s| s.wants(cfg.norad_id)) {
            if let Some(text) = sdroxide_solar::tlesub::cached_text(&cache, sub) {
                for s in parse_subscribed_tles(&text) {
                    consider(s);
                }
            }
        }
        best
    }

    /// The tracking loop: recompute the geometry a few times a second, keep
    /// the RX DDC and the TX NCO on the current correction, and stream the
    /// status. Called every engine tick; a cheap no-op until the interval is
    /// due or when no lock is active.
    fn poll_sat_track(&mut self) {
        let now = Instant::now();
        if !self.sat_lock.as_ref().is_some_and(|l| now >= l.next_calc) {
            return;
        }
        // Taken out wholesale so the borrow checker lets the slow lane consult
        // `resolve_sat_tle` (which reads `sat_cfg`) — put back before the
        // tuning update, which reads the lock again.
        let mut lock = self.sat_lock.take().expect("checked above");
        lock.next_calc = now + SAT_CALC_INTERVAL;
        let unix = unix_now_f64();

        // Slow lane: the pass summary, and — once the elements have gone
        // stale — another look at the caches, which is how a lock survives a
        // TLE refresh instead of dying with its element set.
        if unix >= lock.next_slow_unix {
            lock.next_slow_unix = unix + SAT_SLOW_INTERVAL_S;
            if lock.sat.at(unix).is_none() {
                if let Some(fresh) = self.resolve_sat_tle(&lock.cfg) {
                    if fresh.epoch_unix > lock.sat.epoch_unix {
                        lock.sat = fresh;
                    }
                }
            }
            lock.track.next_pass = match lock.sat.next_passes(
                lock.observer.0,
                lock.observer.1,
                unix,
                48.0 * 3600.0,
                1,
            ) {
                sdroxide_solar::PassSearch::Passes(p) => {
                    p.first().map(|p| sdroxide_types::SatPass {
                        rise_unix: p.rise_unix,
                        set_unix: p.set_unix,
                        rise_az: p.rise_az,
                        set_az: p.set_az,
                        max_el: p.max_el,
                        max_el_unix: p.max_el_unix,
                    })
                }
                _ => None,
            };
        }

        let was_active = lock.track.corrections_active;
        match lock.sat.observe(unix, lock.observer.0, lock.observer.1) {
            Some(o) => {
                // The DDC rides `rx_freq_hz` (dial + RIT), so that is the
                // frequency the RX correction scales with; the uplink maps
                // from the dial alone — RIT is a receive trim and must not
                // move the transmitter, that is what XIT is for.
                let f_down = self.state.rx_freq_hz();
                let f_up = lock.cfg.uplink.map(|u| {
                    u.uplink_for(self.state.active_freq_hz()) + self.state.xit.effective_hz()
                });
                let t = &mut lock.track;
                t.az_deg = o.az_deg;
                t.el_deg = o.el_deg;
                t.range_km = o.range_km;
                t.range_rate_km_s = o.range_rate_km_s;
                t.downlink_hz = self.state.active_freq_hz();
                t.uplink_hz = f_up;
                t.visible = o.el_deg > 0.0;
                t.stale_elements = false;
                t.corrections_active = lock.cfg.doppler && !self.audio_mode;
                (t.doppler_rx_hz, t.doppler_tx_hz) = if t.corrections_active {
                    (
                        sdroxide_types::doppler_rx_hz(f_down, o.range_rate_km_s),
                        f_up.map_or(0.0, |f| sdroxide_types::doppler_tx_hz(f, o.range_rate_km_s)),
                    )
                } else {
                    (0.0, 0.0)
                };
            }
            None => {
                // Stale or decayed elements: a kilohertz of yesterday's
                // correction is worse than none, so the corrections drop to
                // zero rather than freezing — and the status says why.
                let t = &mut lock.track;
                t.stale_elements = true;
                t.corrections_active = false;
                t.doppler_rx_hz = 0.0;
                t.doppler_tx_hz = 0.0;
            }
        }

        let apply = lock.track.corrections_active || was_active;
        if now >= lock.next_emit {
            lock.next_emit = now + SAT_EMIT_INTERVAL;
            let _ = self.event_tx.send(RadioEvent::SatTrack(Some(Box::new(lock.track.clone()))));
        }
        self.sat_lock = Some(lock);
        if apply {
            self.update_tuning();
            self.update_sat_tx_nco();
        }
        self.drive_rotator();
    }

    /// (Re)build the rotctld client to match the config. Rebuilding on every
    /// change is fine: the client is a thread and a socket, and a config edit
    /// is a human-speed event.
    fn sync_rotator(&mut self) {
        self.rotator = None;
        self.rot_last_status = None;
        if self.rot_cfg.enabled {
            self.rotator = Some(sdroxide_rotator::RotctldClient::start(self.rot_cfg.clone()));
        } else {
            // Say the client has gone, so a status line does not keep showing
            // the last position of a client that no longer exists.
            let _ = self.event_tx.send(RadioEvent::RotatorStatus {
                connected: false,
                az_deg: 0.0,
                el_deg: 0.0,
                error: None,
            });
        }
    }

    /// Feed the rotator from the lock's geometry: track above the configured
    /// horizon, pre-position onto the rise azimuth in the last minute before
    /// AOS, park otherwise. The client dedups, so calling this at the
    /// tracking rate costs nothing.
    fn drive_rotator(&mut self) {
        let Some(rot) = self.rotator.as_ref() else { return };
        let lock = self.sat_lock.as_ref().filter(|l| l.cfg.rotator);
        let Some(l) = lock else {
            rot.park();
            return;
        };
        let t = &l.track;
        if t.stale_elements {
            rot.park();
            return;
        }
        if t.el_deg >= self.rot_cfg.min_el_deg.max(0.0) {
            rot.set_target(t.az_deg, t.el_deg);
            return;
        }
        // Below the horizon: swing onto the rise azimuth just before AOS so
        // the pass is not spent catching up with the antenna.
        if let Some(p) = &t.next_pass {
            let to_rise = p.rise_unix as f64 - unix_now_f64();
            if (0.0..60.0).contains(&to_rise) {
                rot.set_target(p.rise_az, 0.0);
                return;
            }
        }
        rot.park();
    }

    /// Relay the rotctld client's health to the clients: connection changes
    /// immediately, position movement at a readout rate.
    fn poll_rotator_status(&mut self) {
        let Some(rot) = self.rotator.as_ref() else { return };
        let st = rot.status();
        let now = Instant::now();
        let transition = self
            .rot_last_status
            .as_ref()
            .is_none_or(|p| p.connected != st.connected || p.error != st.error);
        let moved = now >= self.next_rot_emit && self.rot_last_status.as_ref() != Some(&st);
        if transition || moved {
            self.next_rot_emit = now + Duration::from_secs(2);
            self.rot_last_status = Some(st.clone());
            let _ = self.event_tx.send(RadioEvent::RotatorStatus {
                connected: st.connected,
                az_deg: st.az_deg,
                el_deg: st.el_deg,
                error: st.error,
            });
        }
    }

    /// (Re)build the external T/R switch to match the config.
    ///
    /// Refused while anything is on the air. Rebuilding drops the driver, and
    /// dropping it puts every contact back to receive — under live RF that is
    /// an antenna relay thrown mid-over, which is precisely the accident the
    /// whole subsystem exists to prevent. The operator pressing APPLY during
    /// their own transmission is not a rare mistake; it is what happens when
    /// they are trying to fix the thing while testing it.
    fn sync_relay(&mut self) {
        let Some(hub) = self.tr_switch.clone() else { return };
        if hub.busy() {
            warn!("not rebuilding the T/R switch while the station is on the air");
            self.notice("the T/R switch will be reconfigured after this over");
            self.relay_pending = true;
            return;
        }
        self.relay_pending = false;
        self.relay_last_status = None;
        // Close the old one *first*. Its thread owns a serial port or a device
        // node, and the new configuration is very often the same one with a
        // number changed — so opening before closing would have this engine
        // race itself for its own hardware. Also the moment the contacts are
        // put back to receive, which is why this is refused while on the air.
        hub.install(None, &self.relay_cfg);
        match sdroxide_relay::open(&self.relay_cfg) {
            Ok(handle) => {
                if let Some(h) = handle.as_ref() {
                    info!("T/R switch: {}. {}", h.describe(), self.relay_cfg.sequence_note());
                }
                hub.set_open_error(None);
                hub.install(handle, &self.relay_cfg);
            }
            Err(e) => {
                warn!("T/R switch: {e}");
                hub.install(None, &self.relay_cfg);
                hub.set_open_error(Some(e.to_string()));
            }
        }
        self.emit_relay_status();
    }

    /// Whether this radio is on the air by any route.
    ///
    /// Three terms, and the absence of a fourth is deliberate. `hw_ptt` is
    /// **not** here: [`Engine::apply_hw_ptt`] sets it and *then* asks for the
    /// over, so a key-down the rails refuse leaves it standing true with
    /// nothing transmitting — and a relay driven from it would ground the
    /// antenna for an over that never happened and hold it there until the
    /// operator let go of the foot switch. A hardware PTT line reaches the
    /// switch through `tx_active` like every other route.
    fn on_air(&self) -> bool {
        // `cw_gate_until` covers the one over with no key-up of its own: a
        // message the rig's own keyer is sending, where `tx_active` is false
        // throughout and the radio is nonetheless transmitting.
        self.tx_active || self.rig_tx || self.cw_gate_until.is_some()
    }

    /// Throw the contacts and wait for them, immediately before RF.
    ///
    /// Blocking the engine's thread here is deliberate. The relay *command*
    /// does not block it — the driver owns the port on its own thread and this
    /// is a channel send — but the lead is waited out, because on a CAT rig
    /// `tx_begin` **is** the key-down: RF appears the moment the PTT frame goes
    /// out. Anything that did not block could not promise "contacts before RF",
    /// which is the only thing this feature is for. The wait lands in the gap
    /// where the receive path is about to be paused anyway, and at the ten or
    /// twenty milliseconds a relay actually needs it is inside what the output
    /// ring already carries.
    ///
    /// Capped, because an operator who types 500 ms should get a switch that
    /// works and not a radio that stutters.
    fn lead_tr_switch(&mut self) {
        let Some(hub) = self.tr_switch.as_ref() else { return };
        let wait = hub.key(self.instance);
        if wait.is_zero() {
            return;
        }
        if wait > MAX_TR_LEAD {
            if !self.relay_lead_capped {
                self.relay_lead_capped = true;
                warn!(
                    "the T/R switch asks for {} ms before transmit; waiting {} ms — a longer \
                     lead would be an audible hole in the receive audio and a delay between the \
                     operator and their own transmitter",
                    wait.as_millis(),
                    MAX_TR_LEAD.as_millis()
                );
            }
            std::thread::sleep(MAX_TR_LEAD);
        } else {
            std::thread::sleep(wait);
        }
    }

    /// Keep the shared switch in step with this radio, and relay its health.
    ///
    /// Called every tick. In the common case that is one atomic load and a
    /// comparison — see [`crate::TrSwitch::publish`].
    fn poll_tr_switch(&mut self) {
        let Some(hub) = self.tr_switch.clone() else { return };
        hub.publish(self.instance, self.on_air());

        // A transmitter out in the shack keyed itself, and the sense line saw
        // it in milliseconds rather than the few hundred the CAT poll takes.
        // Adopted by exactly one engine — the hub decides which, since on a
        // multi-radio station only one of them is the transceiver with the wire
        // in it and telling the others would mute the wrong receiver and refuse
        // the wrong key-down.
        if let Some(keyed) = hub.take_sense_edge(self.instance) {
            self.adopt_rig_tx(keyed, "the transmit sense line");
        }

        // A configuration that arrived mid-over, applied now that the station
        // is off the air.
        if self.relay_pending && !hub.busy() {
            self.sync_relay();
        }

        if self.primary {
            let st = hub.status();
            if self.relay_last_status.as_ref() != Some(&st) {
                self.relay_last_status = Some(st.clone());
                let _ = self.event_tx.send(RadioEvent::RelayStatus(Box::new(st)));
            }
        }
    }

    fn emit_relay_status(&mut self) {
        let Some(hub) = self.tr_switch.as_ref() else { return };
        let st = hub.status();
        self.relay_last_status = Some(st.clone());
        let _ = self.event_tx.send(RadioEvent::RelayStatus(Box::new(st)));
    }

    /// The radio in front of us started an over of its own — reported over CAT,
    /// or seen on the T/R switch's sense line, which is the same fact arriving
    /// a few hundred milliseconds sooner.
    ///
    /// Recorded, never answered: keying along with it would talk over the
    /// person holding the microphone. See [`ControlUpdate::RigTx`].
    fn adopt_rig_tx(&mut self, on: bool, how: &str) {
        if on == self.rig_tx {
            return;
        }
        self.rig_tx = on;
        info!(
            "the radio is {} under {how}",
            match on {
                true => "transmitting",
                false => "receiving",
            }
        );
        self.follow_rig_tx(on);
        // Straight through to the switch rather than waiting for the next tick.
        // The whole value of hearing about this early is spending none of it.
        if let Some(hub) = self.tr_switch.as_ref() {
            hub.publish(self.instance, self.on_air());
        }
    }

    /// The receive Doppler correction currently in force, Hz — zero unless a
    /// lock is active with corrections running.
    fn sat_rx_doppler_hz(&self) -> f64 {
        match &self.sat_lock {
            Some(l) if l.track.corrections_active => l.track.doppler_rx_hz,
            _ => 0.0,
        }
    }

    /// Keep the transmit-side NCO on the current correction: TX Doppler plus
    /// however far the transponder mapping has moved since key-down (the
    /// operator tuning across the passband mid-over). A no-op between overs —
    /// the chain only exists while transmitting.
    fn update_sat_tx_nco(&mut self) {
        let shift = match self.sat_lock.as_ref() {
            Some(l) if l.track.corrections_active => {
                l.track.doppler_tx_hz + l.track.uplink_hz.map_or(0.0, |f| f - self.tx_center_hz)
            }
            _ => 0.0,
        };
        let Some(tx) = self.tx.as_mut() else { return };
        if shift == 0.0 && tx.sat_nco.is_none() {
            return;
        }
        match tx.sat_nco.as_mut() {
            Some(n) => n.set_freq(shift, tx.tx_rate),
            None => tx.sat_nco = Some(Nco::new(shift, tx.tx_rate)),
        }
    }

    /// Drain the picture worker: a normalised upload lands in a slot, a listing
    /// or a fetched file goes straight out, a rejection becomes an operator
    /// notice.
    fn poll_images(&mut self) {
        use crate::image_store::GalleryEvent;
        for ev in self.gallery.poll() {
            match ev {
                GalleryEvent::Normalised { slot, png, w, h } => {
                    if self.images.adopt(slot as usize, png, w, h) {
                        self.emit_image_presets();
                    }
                }
                GalleryEvent::Rejected(msg) => {
                    let _ = self.event_tx.send(RadioEvent::Notice(Some(msg)));
                }
                GalleryEvent::Listing(l) => {
                    let _ = self.event_tx.send(RadioEvent::ImageListing(l));
                }
                GalleryEvent::File { kind, name, png } => {
                    let _ = self.event_tx.send(RadioEvent::ImageFile { kind, name, png });
                }
                GalleryEvent::Saved(e) => {
                    let _ = self.event_tx.send(RadioEvent::ImageSaved(e));
                }
                GalleryEvent::Deleted { kind, name } => {
                    let _ = self.event_tx.send(RadioEvent::ImageDeleted { kind, name });
                }
            }
        }
    }

    /// Keep the spot manager's band context current and forward any spots,
    /// lookup/upload results, confirmations, or status lines it produced.
    /// Drain the Winlink worker. Non-blocking, like every other feed the
    /// engine owns: a forwarding session takes tens of seconds and none of it
    /// happens here.
    fn poll_winlink(&mut self) {
        // Keep the manager's view of the radio link current. A mode change
        // destroys the controller that owns the far end, so this both hands the
        // port over when a packet mode starts and takes it away when it stops —
        // taking it away is what turns "the operator changed mode mid-session"
        // into a session that fails with a transcript instead of blocking for
        // ever on a link that no longer exists.
        let port = self.packet_port.clone();
        if let Some(wl) = self.winlink.as_mut() {
            // Compared by identity, not by "is there one".
            //
            // `start_digi` clears the handle and `make_digi` puts a new one
            // back inside the same call, so a change from PACKET to PACKET-HF
            // reads as `true != true` here and the manager keeps a handle whose
            // other end was dropped with the old controller. A session started
            // on it then blocks against a link that no longer exists.
            let stale = match (port.as_ref(), wl.packet_port()) {
                (Some(new), Some(held)) => !new.same_link(held),
                (a, b) => a.is_some() != b.is_some(),
            };
            if stale {
                wl.set_packet_port(port);
            }
        }
        let Some(wl) = self.winlink.as_mut() else { return };
        if wl.poll() {
            let _ = self.event_tx.send(RadioEvent::WinlinkStatus(wl.status().clone()));
        }
    }

    /// Start, stop and pump the KISS TNC server.
    ///
    /// Only runs in a packet mode: the server offers *this* modem, and offering
    /// it while the radio is on FT8 would be a socket that accepts frames and
    /// silently never sends them. Stopping it on the way out is what makes a
    /// mode change a clean disconnect for any attached host rather than a
    /// hang.
    fn poll_kiss_server(&mut self) {
        let want = self.state.rx[0].mode.is_packet() && self.digi_config.packet_kiss_server;
        match (want, self.kiss.is_some()) {
            (true, false) => {
                let addr = format!("127.0.0.1:{}", self.digi_config.packet_kiss_port);
                match sdroxide_kiss::KissServer::start(&addr) {
                    Ok(s) => {
                        info!(addr = %s.addr(), "KISS server started");
                        self.kiss = Some(s);
                    }
                    Err(e) => {
                        // Say so once and turn the setting off, rather than
                        // retrying every tick and filling the log: a port
                        // clash does not resolve itself.
                        warn!("KISS server: {e}");
                        self.digi_config.packet_kiss_server = false;
                        let _ = self
                            .event_tx
                            .send(RadioEvent::Notice(Some(format!("KISS server: {e}"))));
                    }
                }
                return;
            }
            (false, true) => {
                self.kiss = None;
                info!("KISS server stopped");
                return;
            }
            _ => {}
        }
        let Some(srv) = self.kiss.as_ref() else { return };

        // Host → air. Everything a host sends goes through CSMA like our own
        // traffic; a KISS client can ask for the channel but cannot take it.
        let reqs = srv.poll();
        if let Some(digi) = self.digi.as_mut() {
            for r in reqs {
                match r {
                    sdroxide_kiss::KissRequest::Send(frame) => digi.packet_send_frame(frame),
                    sdroxide_kiss::KissRequest::Parameter(cmd, v) => {
                        // Reported, not applied — TXDELAY and friends are the
                        // operator's settings here, and a host overriding them
                        // invisibly would be a mystery to debug.
                        debug!(
                            ?cmd,
                            v, "KISS host set a parameter; ignoring in favour of the operator's"
                        );
                    }
                    sdroxide_kiss::KissRequest::Clients(_) => {}
                }
            }
            // Air → host.
            for frame in digi.packet_take_air_frames() {
                srv.broadcast(&frame);
            }
        }
    }

    /// Put the radio on the selected RMS gateway's channel and speed.
    ///
    /// Both belong to the gateway rather than to the operator: a 2 m RMS runs
    /// 1200 or 9600 because of how it was built, and no rule about the band
    /// predicts which. So they are applied here, at the moment of calling,
    /// rather than when the gateway was picked in settings — a settings dialog
    /// that moved the dial under a listening operator would be worse than the
    /// problem it solves.
    ///
    /// Both changes are announced. A speed change is invisible on the air and
    /// its failure mode is silence, so an operator who is not told would have
    /// no way to tell a retuned modem from a dead gateway.
    fn apply_gateway_channel(&mut self, cfg: &sdroxide_types::WinlinkConfig) {
        // Only VHF/UHF has a choice: HF packet is 300 baud and the controller
        // clamps to it, so touching the config on 40 m would write a setting
        // that does nothing and leave the Setup panel describing a modem the
        // operator does not have.
        if self.state.rx[0].mode == Mode::Packet && self.digi_config.packet_baud != cfg.gateway_baud
        {
            self.digi_config.packet_baud = cfg.gateway_baud;
            // The same fan-out SetDigiConfig does: the Setup panel's speed
            // chip reads this field, and a stale one would show 1200 while
            // the modem ran 9600.
            if let Some(d) = self.digi.as_mut() {
                d.set_config(self.digi_config.clone());
            }
            if let Err(e) = sdroxide_config::save_digi_config(&self.digi_config) {
                warn!("saving digi config: {e}");
            }
            self.mark_shared_store_write();
            self.emit_digi_status();
            let _ = self.event_tx.send(RadioEvent::Notice(Some(format!(
                "packet speed set to {} baud for {}",
                cfg.gateway_baud.label(),
                cfg.gateway.trim().to_uppercase(),
            ))));
        }

        // Zero means "wherever the radio already is", which is what an
        // operator parked on one channel wants.
        if cfg.gateway_freq_hz > 0.0
            && (self.state.active_freq_hz() - cfg.gateway_freq_hz).abs() >= 1.0
        {
            self.stop_scan_for_operator();
            match self.state.active_vfo {
                Vfo::A => self.state.vfo_a_hz = cfg.gateway_freq_hz,
                Vfo::B => self.state.vfo_b_hz = cfg.gateway_freq_hz,
            }
            self.state.band = Band::containing(cfg.gateway_freq_hz);
            self.keep_vfo_in_span();
            self.update_tuning();
            let _ = self.event_tx.send(RadioEvent::Notice(Some(format!(
                "tuned to {:.4} MHz for {}",
                cfg.gateway_freq_hz / 1e6,
                cfg.gateway.trim().to_uppercase(),
            ))));
        }
    }

    /// Publish the current Winlink status, so a client that has just attached
    /// or just changed a setting sees the folder counts without connecting.
    fn emit_winlink_status(&mut self) {
        if let Some(wl) = self.winlink.as_ref() {
            let _ = self.event_tx.send(RadioEvent::WinlinkStatus(wl.status().clone()));
        }
    }

    fn poll_spots(&mut self) {
        self.spots.set_dial(self.state.active_freq_hz());

        // FreeDV Reporter. Pushed unconditionally every tick and deduplicated
        // on the reporter thread, so this one place also catches a CAT rig's
        // dial being turned, a mode change made on the radio itself, and a
        // transmit request the safety rails refused.
        //
        // `tx_freq_hz` (not `rx_freq_hz`) because the reporter shows where a
        // station transmits, and `tx_active` (not `state.tx.ptt`) because a
        // refused key must never be reported as being on the air.
        //
        // On a station with more than one radio the answer is not this engine's
        // own mode: the session lives on the primary engine, and the radio in
        // RADE may be any of them. Every engine publishes its own state to the
        // shared `RadeWatch`; what the reporter is told is whichever radio is
        // actually on FreeDV. `None` is a single-radio start, where this engine
        // *is* the station.
        let in_rade = self.state.rx[0].mode.is_rade();
        let tx_freq = self.state.tx_freq_hz().round().max(0.0) as u64;
        let (freq, visible, tx) = match self.rade_watch.as_ref() {
            Some(w) => {
                w.publish(self.instance, in_rade, tx_freq, self.tx_active);
                match w.reported() {
                    // Nobody on FreeDV: hidden, and the frequency stops
                    // mattering — keep pushing our own so a station that comes
                    // back to RADE has something current to show.
                    None => (tx_freq, false, self.tx_active),
                    Some((f, t)) => (f, true, t),
                }
            }
            None => (tx_freq, in_rade, self.tx_active),
        };
        self.spots.set_reporter_freq(freq);
        self.spots.set_reporter_visible(visible);
        self.spots.set_reporter_tx(tx);

        for ev in self.spots.poll() {
            let re = match ev {
                sdroxide_net::NetEvent::Spots(s) => RadioEvent::Spots(s),
                sdroxide_net::NetEvent::Status(s) => RadioEvent::NetStatus(s),
                sdroxide_net::NetEvent::Callsign(c) => RadioEvent::CallsignResult(c),
                sdroxide_net::NetEvent::Upload(r) => RadioEvent::Upload(r),
                sdroxide_net::NetEvent::LoginTest(r) => RadioEvent::LoginTest(r),
                sdroxide_net::NetEvent::Confirmations(r) => RadioEvent::Confirmations(r),
                sdroxide_net::NetEvent::WsprSpots(s) => RadioEvent::WsprSpots(s),
                sdroxide_net::NetEvent::PropPaths(p) => RadioEvent::PropPaths(p),
            };
            let _ = self.event_tx.send(re);
        }
    }

    fn chain_mut(&mut self, rx: RxId) -> Option<&mut RxChain> {
        match rx {
            RxId::Main => self.main.as_mut(),
            RxId::Sub => self.sub.as_mut(),
        }
    }

    /// The conventional-dial rule for the slot-based modes: selecting FT8, FT4,
    /// FT2, JS8 or WSPR asks for the frequency that band's convention agrees
    /// on. `None` when the dial is to be left exactly where it is.
    ///
    /// These modes are worked on one dial per band and decoded in lockstep with
    /// everyone else on it. A receiver a few kilohertz off is not off-centre,
    /// it is deaf — and each of them keeps its *own* dial (20 m is 14.074 for
    /// FT8, 14.080 for FT4, 14.084 for FT2, 14.078 for JS8 and 14.095600 for
    /// WSPR), so arriving from a neighbouring mode is a move too. Selecting the
    /// mode is therefore worth acting on, exactly as APRS, ADS-B and VDL2
    /// already are in [`Self::set_rx_mode`].
    ///
    /// Deliberately *not* part of `set_rx_mode`. A band-stack recall, a memory
    /// channel and a scanner step each carry a dial of their own and set the
    /// mode as part of applying it: a memory stored on an off-plan FT8 net has
    /// to come back on the frequency it was stored with, not on the one the
    /// convention would have picked. This is asked on the `SetMode` command
    /// alone — the mode buttons, the keyboard shortcut, a remote client,
    /// rigctld — and on nothing else.
    ///
    /// Four guards beyond that:
    ///
    /// * the main receiver only, because the sub receiver does not own the dial;
    /// * a real mode change, so re-selecting the mode you are already in is not
    ///   a way to lose the frequency you tuned;
    /// * a dial not already on one of *this* mode's conventional frequencies,
    ///   so an operator sitting in FT8's DXpedition window keeps it, and so does
    ///   anyone who moves off frequency and back within the mode;
    /// * a band this mode has a convention in at all. Selecting FT8 on 11 m or
    ///   inside a broadcast band leaves the dial where it is: the rule moves a
    ///   receiver to the right spot in the band it is already on, it does not
    ///   decide which band the operator wanted.
    fn conventional_dial_for(&self, rx: RxId, mode: Mode) -> Option<f64> {
        if rx != RxId::Main || self.state.rx[0].mode == mode {
            return None;
        }
        // `is_slotted` is FT8/FT4/FT2/JS8. WSPR is slotted too and is kept out
        // of that predicate for reasons of its own (see its docs), but it is a
        // one-frequency-per-band mode by exactly the same argument.
        if !(mode.is_slotted() || mode.is_wspr()) {
            return None;
        }
        let dial = self.state.active_freq_hz();
        if sdroxide_types::digi_channels(mode).iter().any(|c| (c.dial_hz - dial).abs() < 1.0) {
            return None;
        }
        let here = sdroxide_types::digi_channels_in(mode, Band::containing(dial));
        // The plain calling frequency, which is the published one with no note
        // — the same choice the band buttons already make. Not simply the
        // lowest: FT8's DXpedition dial is *below* the calling one on five
        // bands, and dropping an operator into a Fox/Hound window would be a
        // trap. And not one the operator saved themselves either: a dial that
        // is already on one of those is left alone by the check above, which is
        // the whole of what a saved frequency should do to a rule that moves
        // the radio without being asked.
        let hz = here
            .iter()
            .find(|c| !c.mine && c.note.is_empty())
            .or_else(|| here.iter().find(|c| !c.mine))?
            .dial_hz;
        // `may_rx_hz`, not `can_rx_hz`: a driver that publishes no tuning range
        // has said nothing about where it tunes, not "nowhere".
        self.caps.may_rx_hz(hz).then_some(hz)
    }

    /// Put the active VFO on `hz` — the frequency [`Self::conventional_dial_for`]
    /// asked for, applied once the mode it belongs to is in place.
    ///
    /// Said out loud rather than done silently: along with the APRS, ADS-B and
    /// VDL2 rules this is one of the few times a mode selection moves the dial,
    /// and an operator who did not expect it should be able to find out why.
    fn tune_to_conventional_dial(&mut self, hz: f64) {
        info!(
            from = self.state.active_freq_hz(),
            to = hz,
            mode = self.state.rx[0].mode.label(),
            "the mode has one agreed dial in this band; tuning to it"
        );
        match self.state.active_vfo {
            Vfo::A => self.state.vfo_a_hz = hz,
            Vfo::B => self.state.vfo_b_hz = hz,
        }
        self.state.band = Band::containing(hz);
        // `follow_dial` moves an IQ front end's LO if the new dial left the
        // span; `update_tuning` is what carries a dial to a CAT rig, which
        // `follow_dial` deliberately does not do.
        self.follow_dial();
        self.update_tuning();
    }

    /// Record what the active VFO is listening in, so taking up the other one
    /// can leave it there.
    ///
    /// Called on the way *out* of a VFO rather than on every mode change: the
    /// only thing that has to be true is that the shelf is current at the
    /// moment it is read, and one call at the switch beats instrumenting every
    /// path that can change a mode or a filter width — the mode buttons, a band
    /// stack recall, a memory, the scanner, rigctld, a remote client, the rig's
    /// own knob.
    fn shelve_vfo_mode(&mut self) {
        self.vfo_memory[self.state.active_vfo.index()] = VfoMemory::of(&self.state.rx[0]);
    }

    /// Put the main receiver into the mode and passband the now-active VFO was
    /// left in.
    ///
    /// Goes through [`Self::set_rx_mode`] so a mode change here is the same
    /// mode change as any other: the CAT rig is commanded, the digital lane
    /// follows, a keyer message in flight is stopped. The filter is restored
    /// afterwards because selecting a mode installs that mode's default width,
    /// which would otherwise throw away a passband the operator had narrowed.
    fn recall_vfo_mode(&mut self) {
        let want = self.vfo_memory[self.state.active_vfo.index()];
        if want.mode != self.state.rx[0].mode {
            self.set_rx_mode(RxId::Main, want.mode);
        }
        let r = &mut self.state.rx[0];
        if (r.filter_lo, r.filter_hi) == (want.filter_lo, want.filter_hi) {
            return;
        }
        (r.filter_lo, r.filter_hi) = (want.filter_lo, want.filter_hi);
        let snapshot = *r;
        if let Some(d) = self.main.as_mut().and_then(|c| c.demod.as_mut()) {
            d.set_filter(snapshot.filter_lo, snapshot.filter_hi);
        }
        // The rig does the filtering on a CAT link, so its width has to follow
        // too — exactly as `set_rx_mode` does for the mode it commands.
        // Self-guarded, and a no-op on every front end that filters here.
        self.push_control_filter();
    }

    fn set_rx_mode(&mut self, rx: RxId, mode: Mode) {
        // APRS is a channel, not a band, and which channel is a property of
        // the operator's region: 144.800 in Region 1, 144.390 in the Americas,
        // 145.175 in Australia and New Zealand. A receiver anywhere else is
        // not receiving APRS at all, so selecting the mode tunes there.
        //
        // Every other mode leaves the dial alone, and this is careful not to
        // become an exception to that: it only fires on the *main* receiver,
        // only when the mode is actually changing, and only when the dial is
        // not already on one of the channels — so a traveller who has tuned
        // Japan's 144.640 by hand keeps it, and so does anyone who moves off
        // frequency and back within the mode.
        if rx == RxId::Main
            && mode.is_aprs()
            && !self.state.rx[0].mode.is_aprs()
            && !sdroxide_types::is_aprs_channel(self.state.active_freq_hz())
        {
            let hz = sdroxide_types::aprs_dial();
            match self.state.active_vfo {
                Vfo::A => self.state.vfo_a_hz = hz,
                Vfo::B => self.state.vfo_b_hz = hz,
            }
            self.state.band = Band::containing(hz);
            self.follow_dial();
            // `follow_dial` moves an IQ front end's LO and stops there; a
            // transceiver on a sound card is only ever told its dial by
            // `update_tuning`, and without this the state moved to the channel
            // while the radio stayed on the frequency it was already on.
            self.update_tuning();
        }
        // ADS-B is a channel too, and a far more absolute one: there is exactly
        // one worldwide, the receiver has to be on it, and nothing else can be
        // heard through a 2.4 Msps window parked there. Same three guards as
        // APRS above — main receiver, a real mode change, and only when 1090 is
        // not already inside the window — plus one more: a receiver that cannot
        // reach 1090 MHz at all is left where it is, because retuning it would
        // take away the band the operator was on and give nothing back. The
        // panel says why instead.
        if rx == RxId::Main
            && mode.is_adsb()
            && !self.state.rx[0].mode.is_adsb()
            && self.caps.may_rx_hz(sdroxide_types::ADSB_FREQ_HZ)
            && !sdroxide_adsb::window_covers(self.state.center_hz, self.state.sample_rate)
        {
            let hz = sdroxide_types::ADSB_FREQ_HZ;
            match self.state.active_vfo {
                Vfo::A => self.state.vfo_a_hz = hz,
                Vfo::B => self.state.vfo_b_hz = hz,
            }
            self.state.band = Band::containing(hz);
            self.follow_dial();
            // See the APRS block above for why `follow_dial` is not enough.
            self.update_tuning();
        }
        // VDL2 is the same argument with one difference: the channels are a
        // group rather than a point, so "already inside the window" is asked of
        // the whole plan, and a receiver that reaches only part of it is left
        // where it is and told so rather than dragged across the band.
        if rx == RxId::Main
            && mode.is_vdl2()
            && !self.state.rx[0].mode.is_vdl2()
            && self.caps.may_rx_hz(sdroxide_types::VDL2_PLAN_CENTER_HZ)
            && !sdroxide_vdl2::window_covers(self.state.center_hz, self.state.sample_rate)
        {
            let hz = sdroxide_types::VDL2_PLAN_CENTER_HZ;
            match self.state.active_vfo {
                Vfo::A => self.state.vfo_a_hz = hz,
                Vfo::B => self.state.vfo_b_hz = hz,
            }
            self.state.band = Band::containing(hz);
            self.follow_dial();
            // See the APRS block above for why `follow_dial` is not enough.
            self.update_tuning();
        }
        // AIS is the same argument as VDL2 with a shorter reach: two channels
        // 50 kHz apart, and a window over either of them is worth having,
        // because a ship alternates between the two and is heard on whichever
        // one is being listened to.
        if rx == RxId::Main
            && mode.is_ais()
            && !self.state.rx[0].mode.is_ais()
            && self.caps.may_rx_hz(sdroxide_types::AIS_PLAN_CENTER_HZ)
            && !sdroxide_ais::window_covers(self.state.center_hz, self.state.sample_rate)
        {
            let hz = sdroxide_types::AIS_PLAN_CENTER_HZ;
            match self.state.active_vfo {
                Vfo::A => self.state.vfo_a_hz = hz,
                Vfo::B => self.state.vfo_b_hz = hz,
            }
            self.state.band = Band::containing(hz);
            self.follow_dial();
            // See the APRS block above for why `follow_dial` is not enough.
            self.update_tuning();
        }
        // Changing modes under a running keyer message would leave it playing
        // into a transmit chain that has just been rebuilt (or into a digital
        // mode that has no use for it).
        if rx == RxId::Main && self.state.rx[0].mode != mode {
            self.stop_voice_play();
            self.stop_voice_preview();
            if self.voice.is_recording() {
                self.voice.stop_record();
                self.emit_voice_status();
            }
        }
        // At this receiver's own dial, not in the abstract: SSTV's sideband
        // depends on the band it is being used on.
        let dial = match rx {
            RxId::Main => self.state.rx_freq_hz(),
            RxId::Sub => self.state.sub_rx_hz,
        };
        let r = &mut self.state.rx[rx.index()];
        r.mode = mode;
        let (lo, hi) = mode.default_filter_at(dial);
        (r.filter_lo, r.filter_hi) = (lo, hi);
        let snapshot = *r;
        if let Some(c) = self.chain_mut(rx) {
            c.build_for_mode(&snapshot);
        }
        // A CAT rig: command its mode (subject to the mode policy) and, since
        // the sideband flips which half of the audio band is RF, re-center.
        // A radio listening to its transceiver behind an attached panadapter
        // receiver commands the same two things, and for the same reason — the
        // rig is doing the receiving — but its display is the receiver's I/Q
        // and a sideband flip does not move it. An Icom on its LAN port
        // commands the mode alone: we demodulate its 12 kHz IF, but that IF
        // comes through the filter its mode picks.
        if rx == RxId::Main && self.rig_follows_rx_mode() {
            let _ = self.source.set_control_mode(self.control_mode());
            // After the mode, never before: every family expresses a filter in
            // terms of the mode the rig is in, so a width sent ahead of the
            // mode is a width measured against the old one. Self-guarded: a
            // pairing whose audio comes from the *receiver* leaves the rig's
            // own filter alone.
            self.push_control_filter();
            if self.audio_mode {
                self.update_display_center();
            }
        }
        // The main receiver's mode drives the digital-mode engine; entering
        // or leaving Ft8/Ft4 starts/stops it (and aborts any in-flight QSO).
        if rx == RxId::Main {
            self.sync_digi_mode();
            // ...and the ADS-B lane, which unlike the other wideband decoders
            // runs only while its mode is selected.
            self.sync_adsb();
            self.sync_vdl2();
            self.sync_ais();
            self.emit_digi_status();
            // A wider channel needs a wider berth from the LO: switching a
            // narrow mode that was happily sitting 30 kHz off the LO into WFM
            // hands the discriminator a 250 kHz channel with the DC spike
            // inside it. Re-check the clearance and move the LO if it grew.
            if !self.audio_mode {
                // `follow_dial` rather than the span check alone: entering or
                // leaving CW moves a self-keying transceiver's VFO by a whole
                // sidetone (`rig_cw_offset_hz`), and that is a move no span
                // check would ever ask for — the VFO is sitting on the centre.
                self.follow_dial();
                self.update_tuning();
            }
        }
    }

    /// PowerSDR-style band button: same band = cycle the stack; different
    /// band = save the current entry, recall the target's top.
    fn change_band(&mut self, band: Band) {
        let cur_band = self.state.band;
        let rx = self.state.rx[0];
        let cur_entry = BandStackEntry {
            freq_hz: self.state.active_freq_hz(),
            mode: rx.mode,
            filter_lo: rx.filter_lo,
            filter_hi: rx.filter_hi,
        };

        if band == cur_band {
            if let Some(stack) = self.stacks.get_mut(&band) {
                if stack.len() > 1 {
                    stack.rotate_left(1);
                }
            }
        } else {
            let stack = self.stacks.entry(cur_band).or_default();
            match stack.iter().position(|e| (e.freq_hz - cur_entry.freq_hz).abs() < 1.0) {
                Some(i) => stack[i] = cur_entry,
                None => {
                    stack.insert(0, cur_entry);
                    stack.truncate(3);
                }
            }
        }

        let entry = self.stacks.get(&band).and_then(|s| s.first().copied()).unwrap_or_else(|| {
            let (freq_hz, mode) = band.default_entry();
            let (filter_lo, filter_hi) = mode.default_filter_at(freq_hz);
            BandStackEntry { freq_hz, mode, filter_lo, filter_hi }
        });

        self.state.band = band;
        self.apply_entry(entry);
        if let Err(e) = sdroxide_config::save_bandstacks(&self.stacks) {
            warn!("saving band stacks: {e}");
        }
        self.mark_shared_store_write();
    }

    /// What the S-meter should read, in dBm, or `None` when nothing here
    /// measures a signal at all.
    ///
    /// Three front ends, in the order of how much the reading is worth:
    ///
    /// * A rig with its own S-meter (a CAT rig answering the meter read) — the
    ///   manufacturer calibrated that against a signal generator, and it is
    ///   already in dBm, so the dBFS→dBm offset must *not* go on top of it.
    /// * An IQ front end — the receive chain measures its own passband, in
    ///   dBFS, which `cal_offset_db` turns into dBm for this hardware. A front
    ///   end that reports its own gain ([`IqSource::rx_gain_db`]) has that
    ///   taken off first: dBFS is a level at the converter, and the level at
    ///   the antenna is that level minus whatever the front end put in front
    ///   of it. Without the subtraction the reading follows the gain — every
    ///   step of an attenuator, and on hardware with its own AGC the loop's
    ///   whole range, lands on the meter as if the band had done it.
    /// * A rig on a sound card whose family has no meter read: all that is left
    ///   is the level of the audio it sends, after its own AGC and squelch.
    ///   Not the same quantity, but it moves with the signal, and a meter that
    ///   moves is worth more than one that never leaves its stop — which is
    ///   what a demod-audio source used to show, having no `RxChain` to ask.
    fn rx_signal_dbm(&mut self) -> Option<f32> {
        if let Some(dbm) = self.source.rx_signal_dbm() {
            return Some(dbm);
        }
        // Listening to the transceiver behind an attached panadapter receiver:
        // the main chain is still running, but on the *other* radio's antenna,
        // and a meter reading a signal nobody is listening to is worse than no
        // meter. The audio actually being heard is the only honest measurement
        // left once the rig has declined to report its own.
        if self.caps.rx_audio_external {
            return Some(self.audio_level_dbfs() + self.cal_offset_db);
        }
        if let Some(p) = self.main.as_ref().and_then(|c| c.power_dbfs()) {
            let gain = self.source.rx_gain_db().unwrap_or(0.0);
            return Some(p - gain + self.cal_offset_db);
        }
        self.audio_mode.then(|| self.audio_level_dbfs() + self.cal_offset_db)
    }

    /// The smoothed level of audio that arrived as audio — a demod-audio
    /// source, or the transceiver behind an attached panadapter receiver — in
    /// dBFS. Meaningless (and not measured) on any other kind of front end.
    fn audio_level_dbfs(&self) -> f32 {
        10.0 * (self.audio_level + 1e-20).log10()
    }

    // ---- Scanning ----------------------------------------------------------

    /// What the level on the current channel is, in dBFS on the same scale as
    /// the receiver's own squelch — so "stop where the audio would open" is
    /// true by construction when the operator asks for it.
    fn scan_level_dbfs(&mut self) -> Option<f32> {
        // A CAT rig on a sound card: no DDC, no demodulator, no measurement of
        // the signal itself — the audio it sends is the only evidence there is.
        // Deliberately not the rig's own S-meter, even where one is available:
        // a scan compares this against the *squelch* threshold, which is a
        // level in the audio, and the two scales do not meet. A radio listening
        // to its transceiver behind an attached receiver is in the same
        // position, and ahead of the chain below for the same reason as in
        // `rx_signal_dbm`: that chain is measuring the other antenna.
        if self.audio_mode || self.caps.rx_audio_external {
            return Some(self.audio_level_dbfs());
        }
        if let Some(p) = self.main.as_ref().and_then(|c| c.power_dbfs()) {
            return Some(p);
        }
        // No audio chain at all — a headless engine started without a sound
        // device. The panadapter's FFT is still running, and channel power read
        // off it is the same quantity the demodulator would have measured, so a
        // scan still works with nothing to listen on. It inherits the display's
        // own averaging, though, so with `avg_tc` turned up a channel takes
        // longer to read as free than the real meter would have taken.
        let (flo, fhi) = self.state.rx[0].mode.default_filter();
        self.analyzer.spectrum_db(&mut self.scan_db);
        crate::scanner::channel_power_db(
            &self.scan_db,
            self.state.center_hz,
            self.state.sample_rate,
            self.state.rx_freq_hz(),
            (fhi - flo).abs().max(1.0) as f64,
        )
    }

    fn scan_threshold_db(&self) -> f32 {
        if self.scan_cfg.follow_squelch {
            self.state.rx[0].squelch_db
        } else {
            self.scan_cfg.threshold_db
        }
    }

    fn scan_is_busy(&mut self) -> bool {
        self.scan_level_dbfs().is_some_and(|level| level >= self.scan_threshold_db())
    }

    /// The stored channels a memory scan would visit, in the order the store
    /// holds them: not skipped, and filed under a folder the operator has the
    /// scan set to (issue #236).
    ///
    /// A channel whose folder has gone from under it counts as unfiled — the
    /// same reading the memory list draws it with, so what the scan runs over
    /// is what the operator can see.
    fn scannable_memories(&self) -> impl Iterator<Item = &sdroxide_types::MemoryChannel> {
        let cfg = &self.scan_cfg;
        let folders = &self.mem_folders;
        self.memories.iter().filter(move |m| {
            let filed = m.folder.filter(|id| folders.iter().any(|f| f.id == *id));
            !cfg.skip.contains(&m.id) && cfg.scans_folder(filed)
        })
    }

    /// Whether this front end can search a whole span at once. A demod-audio
    /// source has no span to search, so it walks channels instead.
    fn scan_can_sweep(&self) -> bool {
        !self.audio_mode && self.state.sample_rate > 0.0
    }

    /// How long to give the front end after it moves. The meter's own one-pole
    /// takes about a tenth of a second to be worth reading, and a hardware
    /// retune has to finish before that even starts.
    fn scan_settle(&self) -> Duration {
        Duration::from_millis(self.scan_cfg.dwell_ms.max(40) as u64)
    }

    /// Start or stop scanning, saying why if it cannot start.
    fn set_scanning(&mut self, on: bool) {
        if !on {
            self.stop_scan(None);
            return;
        }
        if self.state.scan.running {
            return;
        }
        if self.tx_active {
            self.notice("cannot scan while transmitting");
            return;
        }
        let usable = match self.scan_cfg.kind {
            ScanKind::Memories => {
                let any = self.scannable_memories().next().is_some();
                if !any {
                    self.notice(
                        "nothing to scan: no memory channels in the chosen folders, or all of \
                         them skipped",
                    );
                }
                any
            }
            ScanKind::Range => {
                let ok = self.scan_cfg.range_is_usable();
                if !ok {
                    self.notice("the scan range is empty — check the low and high frequencies");
                }
                ok
            }
        };
        if !usable {
            return;
        }
        let now = Instant::now();
        self.scan = Some(Scan {
            phase: ScanPhase::Probing(now),
            queue: std::collections::VecDeque::new(),
            slices: Vec::new(),
            slice: usize::MAX, // rolls over to 0 on the first refill
            stepped: Vec::new(),
            step_at: 0,
            mem_slices: Vec::new(),
            mem_slice: usize::MAX, // as `slice` above
        });
        self.state.scan = sdroxide_types::ScanState { running: true, holding: false };
        // Pick the first target now rather than waiting a poll for it, so
        // nothing has to distinguish "just started" from "just finished a
        // dwell" later on.
        self.scan_advance(now);
        self.emit_state();
    }

    /// Stop a running scan wherever it happens to be, which leaves the operator
    /// on that channel — the same as pressing stop on a handheld.
    fn stop_scan(&mut self, why: Option<&str>) {
        if self.scan.take().is_none() && !self.state.scan.running {
            return;
        }
        self.state.scan = sdroxide_types::ScanState::default();
        // A sweep drives the hardware centre itself, dial and all, so a scan
        // that stopped on a CW signal leaves a self-keying transceiver's VFO on
        // our zero-beat rather than on the station. Put it back before the
        // operator reaches for the paddle. See `sync_cw_dial`.
        self.sync_cw_dial();
        if let Some(why) = why {
            self.notice(why);
        }
        self.emit_state();
    }

    /// Called from every command that moves the dial by hand. Touching the
    /// tuning stops the scanner, as it does on every radio that has one; the
    /// alternative is the engine and the operator fighting over the VFO.
    fn stop_scan_for_operator(&mut self) {
        if self.state.scan.running {
            self.stop_scan(None);
        }
        self.stop_hop_for_operator();
    }

    /// The same bargain for WSPR band hopping: a hand on the VFO wins.
    ///
    /// Stood down rather than switched off, so the setting the operator chose is
    /// still the setting they chose — applying the WSPR setup again resumes it.
    /// Silent when hopping was not running, and said once when it was: a beacon
    /// that stopped moving with no explanation reads as the feature being
    /// broken.
    fn stop_hop_for_operator(&mut self) {
        if self.hop_suspended || !self.digi_config.wspr_hop {
            return;
        }
        if self.state.rx[0].mode != Mode::Wspr {
            return;
        }
        self.hop_suspended = true;
        self.notice(
            "WSPR band hopping paused — you moved the dial. Apply the WSPR setup to resume.",
        );
    }

    /// Move the dial because the WSPR beacon asked to hop bands.
    ///
    /// The controller proposes and this disposes. Refused while transmitting
    /// (moving the dial under a carrier is never right), while the operator has
    /// the tuning stood down, and for a frequency the front end cannot reach —
    /// the last of those *silently*, because a device without 160 m would
    /// otherwise post the same complaint every time the cycle came round to it.
    fn wspr_hop(&mut self, hz: f64) {
        if self.hop_suspended || self.tx_active || self.state.tx.ptt || self.digi_tx {
            return;
        }
        if hz <= 0.0 {
            return;
        }
        // Ask about the centre the front end would actually sit on, not the
        // dial: on a radio that parks its LO clear of the VFO they differ, and
        // it is the LO that has to be in range.
        let offset = self.lo_offset_hz();
        if ![hz + offset, hz - offset, hz].into_iter().any(|c| self.can_tune(c)) {
            debug!(hz, "WSPR hop skipped: outside the receive range");
            return;
        }
        match self.state.active_vfo {
            Vfo::A => self.state.vfo_a_hz = hz,
            Vfo::B => self.state.vfo_b_hz = hz,
        }
        self.state.band = Band::containing(hz);
        self.keep_vfo_in_span();
        self.update_tuning();
        let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
    }

    fn poll_scanner(&mut self) {
        if !self.state.scan.running {
            return;
        }
        if self.tx_active {
            self.stop_scan(Some("scan stopped: transmitting"));
            return;
        }
        let now = Instant::now();
        let Some(phase) = self.scan.as_ref().map(|s| s.phase) else { return };
        match phase {
            ScanPhase::Settling(until) => {
                if now >= until {
                    self.scan_read_slice(now);
                }
            }
            ScanPhase::Probing(until) => {
                if now >= until {
                    self.scan_decide(now);
                }
            }
            ScanPhase::Holding { since, last_busy } => self.scan_hold(now, since, last_busy),
        }
    }

    /// The front end has settled on a sweep slice: read the whole span at once
    /// and queue everything busy in it.
    fn scan_read_slice(&mut self, now: Instant) {
        self.analyzer.spectrum_db(&mut self.scan_db);
        if self.scan_cfg.kind == ScanKind::Memories {
            return self.scan_read_mem_slice(now);
        }
        let (lo, hi) = self.scan_cfg.range();
        let (flo, fhi) = self.scan_cfg.mode.default_filter();
        let found = crate::scanner::busy_channels(
            &self.scan_db,
            self.state.center_hz,
            self.state.sample_rate,
            lo,
            hi,
            (fhi - flo).abs().max(1.0) as f64,
            self.scan_cfg.step_hz.max(1.0),
            self.scan_threshold_db(),
        );
        // Dropped here rather than at the stop: a skipped channel then costs
        // nothing at all, instead of a dwell each time round to rediscover that
        // the operator does not want it.
        let cfg = &self.scan_cfg;
        let keep: Vec<ScanTarget> =
            found.into_iter().filter(|&f| !cfg.skips_freq(f)).map(ScanTarget::Freq).collect();
        if let Some(sc) = self.scan.as_mut() {
            sc.queue.extend(keep);
        }
        self.scan_advance(now);
    }

    /// The dwell on a candidate is up: stop here, or carry on.
    fn scan_decide(&mut self, now: Instant) {
        if self.scan_is_busy() {
            if let Some(sc) = self.scan.as_mut() {
                sc.phase = ScanPhase::Holding { since: now, last_busy: now };
            }
            self.state.scan.holding = true;
            self.emit_state();
        } else {
            self.scan_advance(now);
        }
    }

    /// Stopped on a channel: decide whether it is time to move on.
    fn scan_hold(&mut self, now: Instant, since: Instant, last_busy: Instant) {
        let busy = self.scan_is_busy();
        if busy {
            if let Some(sc) = self.scan.as_mut() {
                sc.phase = ScanPhase::Holding { since, last_busy: now };
            }
        }
        let grace = Duration::from_millis(self.scan_cfg.resume_ms as u64);
        let done = match self.scan_cfg.resume {
            ScanResume::Manual => false,
            ScanResume::Timed => now.duration_since(since) >= grace,
            // A gap between overs is not the end of a conversation, so the
            // grace period runs from when the signal actually stopped.
            ScanResume::Carrier => !busy && now.duration_since(last_busy) >= grace,
        };
        if done {
            self.scan_advance(now);
        }
    }

    /// Move on to the next thing to look at, refilling the queue as needed.
    fn scan_advance(&mut self, now: Instant) {
        self.state.scan.holding = false;
        // Bounded: one refill to top the queue up, and a second attempt only if
        // that refill produced nothing to visit. Anything further waits for the
        // next poll, so no arrangement of settings can spin the DSP thread.
        for _ in 0..2 {
            if let Some(target) = self.scan.as_mut().and_then(|s| s.queue.pop_front()) {
                self.scan_goto(target, now);
                return;
            }
            match self.scan_refill(now) {
                Refill::Queued => continue,
                // Either the front end is moving and the next poll picks it up,
                // or the scan is over.
                Refill::Waiting | Refill::Stopped => return,
            }
        }
    }

    /// Put the next batch of candidates in the queue, or set the front end
    /// moving towards them.
    fn scan_refill(&mut self, now: Instant) -> Refill {
        match self.scan_cfg.kind {
            ScanKind::Memories if self.scan_cfg.mem_fast && self.scan_can_sweep() => {
                self.scan_next_mem_slice(now)
            }
            ScanKind::Memories => {
                let targets: Vec<ScanTarget> =
                    self.scannable_memories().map(|m| ScanTarget::Memory(m.id)).collect();
                if targets.is_empty() {
                    self.stop_scan(Some(
                        "scan stopped: every memory channel in the chosen folders is skipped",
                    ));
                    return Refill::Stopped;
                }
                if let Some(sc) = self.scan.as_mut() {
                    sc.queue.extend(targets);
                }
                Refill::Queued
            }
            ScanKind::Range if self.scan_can_sweep() => self.scan_next_slice(now),
            ScanKind::Range => self.scan_next_stepped(),
        }
    }

    /// Move the front end to the next slice of the range. The spectrum is read
    /// once it has settled, which is a poll or two later.
    fn scan_next_slice(&mut self, now: Instant) -> Refill {
        let (lo, hi) = self.scan_cfg.range();
        let span = self.state.sample_rate;
        let settle = self.scan_settle();
        let Some(sc) = self.scan.as_mut() else { return Refill::Stopped };
        if sc.slices.is_empty() {
            sc.slices = crate::scanner::slice_centers(lo, hi, span);
            if sc.slices.is_empty() {
                self.stop_scan(Some("scan stopped: the range is not a usable one"));
                return Refill::Stopped;
            }
        }
        sc.slice = sc.slice.wrapping_add(1);
        if sc.slice >= sc.slices.len() {
            sc.slice = 0; // round again: a scanner runs until it is stopped
        }
        let center = sc.slices[sc.slice];
        sc.phase = ScanPhase::Settling(now + settle);

        // Park the dial on the slice as well, so the operator sees where the
        // scan is looking rather than a dial left behind on the last candidate.
        match self.state.active_vfo {
            Vfo::A => self.state.vfo_a_hz = center,
            Vfo::B => self.state.vfo_b_hz = center,
        }
        self.state.band = Band::containing(center);
        if !self.retune(center) {
            // Outside the front end's range: `tune_refused` has already put the
            // dial back and said so, and the next slice may still be reachable.
            return Refill::Waiting;
        }
        // The running average still holds the previous slice's samples, and
        // they are from a different part of the band entirely.
        self.analyzer.reset();
        self.zoom = None;
        self.update_tuning();
        self.emit_state();
        Refill::Waiting
    }

    /// Move the front end to the next window of a fast memory scan's plan,
    /// building the plan first if there is none.
    ///
    /// The memory twin of [`Engine::scan_next_slice`], and the same shape: the
    /// spectrum is read once the front end has settled, a poll or two later.
    /// What differs is what the windows are for. A range sweep tiles a band it
    /// has to search; this one covers a *list*, so its windows are placed on
    /// where the channels are and nowhere else, and a whole band's worth of
    /// them is one tune (issue #228).
    fn scan_next_mem_slice(&mut self, now: Instant) -> Refill {
        let span = self.state.sample_rate;
        let settle = self.scan_settle();
        if self.scan.as_ref().is_some_and(|sc| sc.mem_slices.is_empty()) {
            let chans: Vec<(u32, f64)> =
                self.scannable_memories().map(|m| (m.id, m.freq_hz)).collect();
            let freqs: Vec<f64> = chans.iter().map(|(_, hz)| *hz).collect();
            let plan: Vec<(f64, Vec<u32>)> = crate::scanner::memory_slices(&freqs, span)
                .into_iter()
                .map(|(c, group)| (c, group.into_iter().map(|i| chans[i].0).collect()))
                .collect();
            if plan.is_empty() {
                self.stop_scan(Some(
                    "scan stopped: every memory channel in the chosen folders is skipped",
                ));
                return Refill::Stopped;
            }
            if let Some(sc) = self.scan.as_mut() {
                sc.mem_slices = plan;
            }
        }
        let Some(sc) = self.scan.as_mut() else { return Refill::Stopped };
        sc.mem_slice = sc.mem_slice.wrapping_add(1);
        if sc.mem_slice >= sc.mem_slices.len() {
            // Round again on a plan built from the store as it stands now: a
            // channel stored, deleted or re-filed during the last lap belongs
            // in this one. Back to `usize::MAX` rather than to 0, because the
            // rebuild below goes through the same `wrapping_add` and 0 would
            // start the new lap on the second window.
            sc.mem_slice = usize::MAX;
            sc.mem_slices.clear();
            return Refill::Queued;
        }
        let center = sc.mem_slices[sc.mem_slice].0;
        sc.phase = ScanPhase::Settling(now + settle);

        // Whether this is a move at all. A list that fits in one window is the
        // common case and has exactly one of these, so a lap comes back to a
        // centre the front end is already on — and throwing the analyser away
        // there would blank the panadapter several times a second for a scan
        // that never moved the radio.
        let moved = (self.state.center_hz - center).abs() > 0.5;
        // The dial goes with it, as it does on a range sweep: the operator
        // should see where the scan is looking rather than a readout left
        // behind on the last channel it stopped at.
        match self.state.active_vfo {
            Vfo::A => self.state.vfo_a_hz = center,
            Vfo::B => self.state.vfo_b_hz = center,
        }
        self.state.band = Band::containing(center);
        if !self.retune(center) {
            // Out of the front end's range — `tune_refused` has put the dial
            // back and said so, and the next window may still be reachable.
            return Refill::Waiting;
        }
        if moved {
            // The running average is still holding the last window's samples,
            // and they are from another part of the band entirely.
            self.analyzer.reset();
            self.zoom = None;
        }
        self.update_tuning();
        self.emit_state();
        Refill::Waiting
    }

    /// The front end has settled on a fast memory scan's window: measure every
    /// channel it can see, and queue the ones something is on.
    ///
    /// The dwell still confirms each of them through the receiver itself, so
    /// this decides only what is worth a visit — which is why a channel the
    /// transform cannot honestly answer for is queued rather than dropped (see
    /// [`crate::scanner::busy_memories`]).
    fn scan_read_mem_slice(&mut self, now: Instant) {
        let ids = self
            .scan
            .as_ref()
            .and_then(|sc| sc.mem_slices.get(sc.mem_slice))
            .map(|(_, ids)| ids.clone())
            .unwrap_or_default();
        // Resolved against the store rather than remembered with the plan: a
        // channel edited or deleted between the tune and this read is one this
        // window no longer knows anything about.
        let here: Vec<(u32, f64, f64)> = ids
            .iter()
            .filter_map(|id| self.memories.iter().find(|m| m.id == *id))
            .map(|m| (m.id, m.freq_hz, (m.filter_hi - m.filter_lo).abs() as f64))
            .collect();
        let freqs: Vec<f64> = here.iter().map(|(_, hz, _)| *hz).collect();
        let widths: Vec<f64> = here.iter().map(|(_, _, bw)| *bw).collect();
        let busy = crate::scanner::busy_memories(
            &self.scan_db,
            self.state.center_hz,
            self.state.sample_rate,
            &freqs,
            &widths,
            self.scan_threshold_db(),
        );
        if let Some(sc) = self.scan.as_mut() {
            sc.queue.extend(busy.into_iter().map(|i| ScanTarget::Memory(here[i].0)));
        }
        self.scan_advance(now);
    }

    /// One channel at a time along the range's grid — for a front end with no
    /// span of its own to search.
    fn scan_next_stepped(&mut self) -> Refill {
        let (lo, hi) = self.scan_cfg.range();
        let step = self.scan_cfg.step_hz.max(1.0);
        let mut picked = None;
        {
            let cfg = &self.scan_cfg;
            let Some(sc) = self.scan.as_mut() else { return Refill::Stopped };
            if sc.stepped.is_empty() {
                // Capped: a whole band on a 5 kHz grid is a few thousand
                // channels, and asking a CAT rig to visit a hundred thousand is
                // not a scan.
                const MAX: usize = 8192;
                let n = (((hi - lo) / step).floor() as usize + 1).min(MAX);
                sc.stepped = (0..n).map(|i| lo + i as f64 * step).collect();
            }
            // One channel per refill: every one on this path has to be listened
            // to, so queueing the whole band up front would buy nothing.
            //
            // Bounded by one lap round the grid, because a skipped channel is
            // passed over here rather than dwelt on: with every channel in the
            // range skipped there is nothing left to visit, and hunting for
            // ever would spin the DSP thread instead of saying so.
            for _ in 0..sc.stepped.len() {
                if sc.step_at >= sc.stepped.len() {
                    sc.step_at = 0;
                }
                let f = sc.stepped[sc.step_at];
                sc.step_at += 1;
                if !cfg.skips_freq(f) {
                    picked = Some(f);
                    break;
                }
            }
            if let Some(f) = picked {
                sc.queue.push_back(ScanTarget::Freq(f));
            }
        }
        if picked.is_some() {
            return Refill::Queued;
        }
        // Nothing to visit at all — either the range yielded no grid, or every
        // channel on it is skipped. Both are dead ends, and both have to be
        // said: a scanner that is running and never stopping anywhere looks
        // exactly like one that is broken.
        let had_grid = self.scan.as_ref().is_some_and(|sc| !sc.stepped.is_empty());
        self.stop_scan(Some(if had_grid {
            "scan stopped: every channel in the range is skipped"
        } else {
            "scan stopped: the range is not a usable one"
        }));
        Refill::Stopped
    }

    /// Park the receiver on a candidate and start listening to it.
    fn scan_goto(&mut self, target: ScanTarget, now: Instant) {
        match target {
            ScanTarget::Memory(id) => {
                let Some(m) = self.memories.iter().find(|m| m.id == id) else { return };
                let entry = BandStackEntry {
                    freq_hz: m.freq_hz,
                    mode: m.mode,
                    filter_lo: m.filter_lo,
                    filter_hi: m.filter_hi,
                };
                // The setup the channel was stored with, read exactly as
                // `RecallMemory` reads it — an absent one as plain simplex with
                // no tone (issue #204). A scan stops on a channel to be worked,
                // and the operator who answers the call reaches for the PTT
                // rather than for the shift: without this the over goes out on
                // the last repeater's shift and tone, whichever channel the
                // scan is actually sitting on (issue #264).
                let repeater = m.repeater.unwrap_or_default().clamped();
                self.place_entry_in_span(entry);
                // After the dial, not before it: a channel stored with AUTO on
                // resolves its shift against the frequency it lands on, and the
                // one being left is the wrong band to ask about.
                self.set_repeater(repeater);
            }
            ScanTarget::Freq(hz) => {
                if self.state.rx[0].mode != self.scan_cfg.mode {
                    self.set_rx_mode(RxId::Main, self.scan_cfg.mode);
                }
                match self.state.active_vfo {
                    Vfo::A => self.state.vfo_a_hz = hz,
                    Vfo::B => self.state.vfo_b_hz = hz,
                }
                self.state.band = Band::containing(hz);
                self.keep_vfo_in_span();
                self.update_tuning();
            }
        }
        let settle = self.scan_settle();
        if let Some(sc) = self.scan.as_mut() {
            sc.phase = ScanPhase::Probing(now + settle);
        }
        self.emit_state();
    }

    /// Leave the channel the scan is holding on, optionally never to stop there
    /// again.
    fn scan_skip(&mut self, forever: bool) {
        if !self.state.scan.running {
            return;
        }
        // Whichever channel the dial is actually sitting on, rather than
        // whichever one the queue last dealt: a recall can be refused, and the
        // operator means "this one, the one I am listening to".
        let here = self.state.active_freq_hz();
        match (forever, self.scan_cfg.kind) {
            (false, _) => {}
            (true, ScanKind::Memories) => {
                if let Some(id) =
                    self.memories.iter().find(|m| (m.freq_hz - here).abs() < 1.0).map(|m| m.id)
                    && !self.scan_cfg.skip.contains(&id)
                {
                    self.scan_cfg.skip.push(id);
                    self.save_scanner_config();
                }
            }
            // A range scan has no channel to name, so the frequency itself is
            // what is remembered — and it is persisted, so the pager that fires
            // every three minutes is dismissed once rather than every pass.
            (true, ScanKind::Range) => {
                if !self.scan_cfg.skips_freq(here) {
                    self.scan_cfg.skip_freq(here);
                    self.save_scanner_config();
                }
            }
        }
        self.scan_advance(Instant::now());
    }

    fn save_scanner_config(&mut self) {
        if let Err(e) = self.store.save_scanner_config(&self.scan_cfg) {
            warn!("saving scanner config: {e}");
        }
        let _ = self.event_tx.send(RadioEvent::Scanner(self.scan_cfg.clone()));
    }

    /// Refuse a transmit request, and *say so*.
    ///
    /// This used to be a `warn!` and nothing else, which meant an unattended
    /// beacon whose rails refused every slot sat there looking like it was
    /// working: the mode said it would transmit, the log said why it did not,
    /// and the two never met. A refusal the operator cannot see is
    /// indistinguishable from a bug in whatever asked to key.
    fn deny_tx(&mut self, reason: &str) {
        warn!("TX refused: {reason}");
        self.state.tx.ptt = false;
        self.state.tx.tune = false;
        // Every deny happens before the transmitter keyed, so a gate claimed
        // moments ago by the rail sequence has to be handed back — and if this
        // radio never held it, releasing is a no-op by contract.
        self.release_tx_gate();
        self.notice(&format!("transmit refused — {reason}"));
    }

    /// Hand back the station's transmit interlock, if this engine holds it,
    /// and reconcile the external T/R switch with what is actually happening.
    ///
    /// Reconciled rather than released, and the difference matters: this is
    /// called from [`Engine::deny_tx`], which runs on refusals that have
    /// nothing to do with whether the station is on the air — a key-down
    /// refused *because the rig is transmitting on its own PTT* being the case
    /// in point. Dropping the contacts there would open an antenna relay under
    /// somebody else's live RF and throw it straight back on the next tick.
    fn release_tx_gate(&self) {
        if let Some(gate) = self.tx_gate.as_ref() {
            gate.release(self.instance);
        }
        if let Some(hub) = self.tr_switch.as_ref() {
            hub.publish(self.instance, self.on_air());
        }
    }

    fn notice(&self, text: &str) {
        let _ = self.event_tx.send(RadioEvent::Notice(Some(text.to_string())));
    }

    /// Withdraw the persistent notice. `Notice(None)` is the documented clear.
    fn clear_notice(&self) {
        let _ = self.event_tx.send(RadioEvent::Notice(None));
    }

    fn emit_state(&self) {
        let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
    }

    /// Tune + set mode/filter from a band-stack entry or memory channel.
    fn apply_entry(&mut self, entry: BandStackEntry) {
        self.place_entry(entry, true);
    }

    /// [`Self::apply_entry`], but leaving the front end where it is when the
    /// target is already inside the span it is receiving.
    ///
    /// For the scanner, where a run of memories on the same band costs a DDC
    /// shift each instead of a hardware retune each — the difference between
    /// stepping channels in microseconds and stepping them in tens of
    /// milliseconds. A band change still moves the LO, via `keep_vfo_in_span`.
    fn place_entry_in_span(&mut self, entry: BandStackEntry) {
        self.place_entry(entry, false);
    }

    fn place_entry(&mut self, entry: BandStackEntry, force_retune: bool) {
        match self.state.active_vfo {
            Vfo::A => self.state.vfo_a_hz = entry.freq_hz,
            Vfo::B => self.state.vfo_b_hz = entry.freq_hz,
        }
        self.state.band = Band::containing(entry.freq_hz);
        self.set_rx_mode(RxId::Main, entry.mode);
        let r = &mut self.state.rx[0];
        (r.filter_lo, r.filter_hi) = (entry.filter_lo, entry.filter_hi);
        let snapshot = *r;
        if let Some(d) = self.main.as_mut().and_then(|c| c.demod.as_mut()) {
            d.set_filter(snapshot.filter_lo, snapshot.filter_hi);
        }
        if force_retune {
            self.retune_for_vfo(entry.freq_hz);
        } else {
            self.keep_vfo_in_span();
        }
        self.update_tuning();
    }

    /// Put the front end back on the antenna ports the operator asked for.
    ///
    /// Called on every source that reaches this engine — the one it started
    /// with, and every one a reconnect or an interface switch brings in — because
    /// a freshly opened device is on whatever port its driver defaults to.
    ///
    /// A name the device does not list is skipped rather than attempted: after
    /// an interface switch the preference usually belongs to the *other* radio,
    /// and a driver asked for a port it has never heard of logs an error that
    /// says nothing useful. The state is refreshed from the hardware either way,
    /// so the UI shows the port in use rather than the one that was wanted.
    fn restore_antennas(&mut self) {
        let (want_rx, want_tx) = self.want_antenna.clone();
        // Not where the source owns the receive port. A LimeSDR with a LimeRFE
        // in front of it listens on the socket the front end is cabled to, and
        // a port some earlier run happened to record would put it back on an
        // empty connector at every start — see [`IqSource::owns_rx_antenna`].
        let want_rx = want_rx.filter(|_| !self.source.owns_rx_antenna());
        if let Some(name) = want_rx.filter(|n| self.caps.antennas_rx.contains(n))
            && self.state.antenna_rx != name
        {
            if let Err(e) = self.source.set_antenna(&name) {
                warn!("restoring RX antenna {name}: {e}");
            }
            self.state.antenna_rx = self.source.current_antenna();
        }
        if let Some(name) = want_tx.filter(|n| self.caps.antennas_tx.contains(n))
            && self.state.antenna_tx != name
        {
            if let Err(e) = self.source.set_tx_antenna(&name) {
                warn!("restoring TX antenna {name}: {e}");
            }
            self.state.antenna_tx = self.source.current_tx_antenna();
        }
    }

    /// Put the socket the operator last chose on `band` back, as the dial
    /// crosses into it.
    ///
    /// A station with more than one antenna has one per band rather than one
    /// altogether, and a radio with a selector is expected to know that: an
    /// Icom's band stacking register carries the socket beside the frequency,
    /// and an SDRplay operator switching between an HF wire and a VHF discone
    /// should not have to reach for the control every time (issues #235, #238).
    ///
    /// Only bands the operator has actually chosen on are remembered, so a band
    /// never worked leaves the front end exactly where it was — the same
    /// "no preference means no assertion" rule [`Engine::restore_antennas`]
    /// follows, and for the same reason.
    ///
    /// The socket is *asserted*, not compared: on a band change the radio may
    /// already have moved it under us. An Icom's band stacking register carries
    /// a socket of its own, so crossing into 40 m puts the rig on whatever was
    /// last used there — and if that happens to match what sdroxide last read,
    /// a comparison against the cached socket sends nothing and the wrong
    /// aerial stays connected. That is why 30 m → 40 m left an IC-7610 on ANT1
    /// while 20 m → 40 m switched it correctly: the same preference, and only
    /// the *cached* value differed (issue #258).
    fn follow_band_antenna(&mut self, band: Band) {
        // Not where the source owns the receive port: a LimeRFE listens on the
        // socket it is cabled to whatever the band. Same exemption as
        // `restore_antennas`.
        let Some((rx, tx)) = self.band_antenna.get(&band).cloned() else { return };
        let before = (self.state.antenna_rx.clone(), self.state.antenna_tx.clone());
        if let Some(name) =
            rx.filter(|n| !self.source.owns_rx_antenna() && self.caps.antennas_rx.contains(n))
        {
            if let Err(e) = self.source.set_antenna(&name) {
                warn!("switching to RX antenna {name} for {band:?}: {e}");
            }
            self.state.antenna_rx = self.source.current_antenna();
            self.want_antenna.0 = Some(name);
        }
        if let Some(name) = tx.filter(|n| self.caps.antennas_tx.contains(n)) {
            if let Err(e) = self.source.set_tx_antenna(&name) {
                warn!("switching to TX antenna {name} for {band:?}: {e}");
            }
            self.state.antenna_tx = self.source.current_tx_antenna();
            self.want_antenna.1 = Some(name);
        }
        if before != (self.state.antenna_rx.clone(), self.state.antenna_tx.clone()) {
            let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
        }
    }

    /// Re-publish the antenna list when the *radio* has since said what it is.
    ///
    /// The same shape as [`Engine::refresh_center_is_dial`], and there for the
    /// same reason: a control link answers a round trip after the device was
    /// built, and the capabilities went out before it did. Every CI-V rig
    /// speaks one dialect, so whether this one has an antenna selector is not
    /// known until it answers — or NAKs — the read the link sends on opening.
    /// A front end whose ports are a fact about the hardware says nothing here
    /// and nothing happens.
    fn refresh_antennas(&mut self) {
        let Some(list) = self.source.learned_antennas() else { return };
        // Compared without building anything: this runs on every pass of the
        // loop and the answer changes once a session at most.
        if list.len() == self.caps.antennas_rx.len()
            && list.iter().zip(&self.caps.antennas_rx).all(|(a, b)| a == b)
        {
            return;
        }
        self.caps.antennas_rx = list.iter().map(|a| (*a).to_string()).collect();
        let _ = self.event_tx.send(RadioEvent::CapabilitiesUpdated(self.caps.clone()));
        // A port the session remembered may only now be one this radio offers.
        self.restore_antennas();
        self.follow_band_antenna(self.state.band);
        let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
    }

    /// Re-publish the RX gain ranges when what they depend on has moved.
    ///
    /// The same shape as [`Engine::refresh_antennas`], and there for a
    /// neighbouring reason: an RSP's RF gain is a step into a table whose
    /// length belongs to the *band*, so a range published once when the device
    /// opened is wrong for most of the spectrum. Called after a retune, a port
    /// change and a gain command — the three things that can move it — rather
    /// than from the loop, because building the answer allocates and the loop
    /// runs per block.
    ///
    /// Also re-reads the values. The driver clamps a state the new band does
    /// not have and reports the one it kept, and a slider left sitting above
    /// that would be showing a setting the hardware never took.
    fn refresh_rx_gains(&mut self) {
        let Some(mut gains) = self.source.learned_rx_gains() else { return };
        // Receive stages only, as the name says: what the source hands back
        // replaces those and nothing else. A wholesale assignment would take a
        // transmitter's gain stages off the panel the first time its
        // receiver's ladder moved — which no source here would do today, the
        // one that answers being receive-only, and which is exactly the sort
        // of thing that goes unnoticed until the second one arrives.
        gains.extend(self.caps.gains.iter().filter(|g| g.direction == Direction::Tx).cloned());
        if gains != self.caps.gains {
            self.caps.gains = gains;
            let _ = self.event_tx.send(RadioEvent::CapabilitiesUpdated(self.caps.clone()));
        }
        let now = self.source.current_gains();
        if now != self.state.gains {
            self.state.gains = now;
            let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
        }
    }

    /// Note down a gain stage the operator has set, so it can be put back on
    /// the next open — and, through `session.json`, on the next start.
    ///
    /// Recorded as the device reads it back rather than as it was asked for
    /// where the two differ: a driver that quantises 33.7 dB to its 1 dB grid
    /// should be remembered on the step it actually went to, or every restart
    /// would re-send a figure it never held.
    fn remember_gain(&mut self, dir: Direction, element: String, db: f64) {
        let (want, actual) = match dir {
            Direction::Rx => (&mut self.want_gains.0, &self.state.gains),
            Direction::Tx => (&mut self.want_gains.1, &self.state.tx_gains),
        };
        let db = actual.iter().find(|(n, _)| *n == element).map_or(db, |(_, d)| *d);
        match want.iter_mut().find(|(n, _)| *n == element) {
            Some(slot) => slot.1 = db,
            None => want.push((element, db)),
        }
    }

    /// Re-apply the remembered front-end gain stages, for the same reason
    /// [`Engine::restore_antennas`] re-applies the antenna: a device that has
    /// just been opened sits on its driver's defaults, and the LNA and
    /// attenuator settings an operator arrived at by listening to the noise
    /// floor are not something to make them find again on every start.
    ///
    /// Elements the current front end does not have are skipped — the same
    /// check the antenna gets — and a value out of its range is clamped rather
    /// than dropped, so a figure carried over from another device lands on the
    /// nearest thing this one can do instead of on nothing.
    ///
    /// Several native backends (Pluto, RX-888, Airspy HF+, HPSDR, SDRplay)
    /// already keep their gain stages in `radio.json` and apply them as they
    /// open, so for those this runs *after* and the session's value wins. The
    /// two agree in practice — the panels that write that config push the same
    /// figure through `SetGain`, which is what is remembered here — and the
    /// backends that have no config home for a gain, the SoapySDR ones, are the
    /// reason this exists at all.
    fn restore_gains(&mut self) {
        let (want_rx, want_tx) = self.want_gains.clone();
        // Copied out of `caps` before the source is touched: `set_gain_element`
        // needs `self` mutably, and the range is all that is wanted from it.
        let range = |caps: &DeviceCaps, dir, name: &str| {
            caps.gains
                .iter()
                .find(|g| g.direction == dir && g.name == name)
                .map(|g| (g.min_db, g.max_db))
        };
        let mut touched_rx = false;
        for (name, db) in want_rx {
            let Some((min, max)) = range(&self.caps, Direction::Rx, &name) else { continue };
            if let Err(e) = self.source.set_gain_element(&name, db.clamp(min, max)) {
                warn!("restoring RX gain {name}: {e}");
            }
            touched_rx = true;
        }
        if touched_rx {
            self.state.gains = self.source.current_gains();
        }
        let mut touched_tx = false;
        for (name, db) in want_tx {
            let Some((min, max)) = range(&self.caps, Direction::Tx, &name) else { continue };
            if let Err(e) = self.source.set_tx_gain_element(&name, db.clamp(min, max)) {
                warn!("restoring TX gain {name}: {e}");
            }
            touched_tx = true;
        }
        if touched_tx {
            self.state.tx_gains = self.source.current_tx_gains();
        }
    }

    /// Write `digi.json` if the transmit-audio rail has moved since the last
    /// write. A no-op otherwise, which is every tick that is not during or just
    /// after a drag.
    ///
    /// Deferred rather than written in the command arm for the reason
    /// `digi_dirty` gives, and flushed from the same two places the session is —
    /// the periodic tick and `Drop` — for the reason the session's own comment
    /// gives: a clean quit is the common case, and without it a drag that ended
    /// in the last few seconds would be lost.
    ///
    /// `digi.json` is a shared store, so the generation is bumped as
    /// `SetDigiConfig` does: without it this engine reloads its own write the
    /// next time another one touches the directory.
    fn flush_digi_config(&mut self) {
        if !std::mem::take(&mut self.digi_dirty) {
            return;
        }
        if let Err(e) = sdroxide_config::save_digi_config(&self.digi_config) {
            warn!("saving digi config: {e}");
        }
        self.mark_shared_store_write();
    }

    /// Write the dial, mode, antennas and levels to `session.json` if any of
    /// them has moved since the last write, so the next start comes up here
    /// rather than on the default frequency and levels. A no-op on an engine
    /// that isn't remembering its session.
    ///
    /// Compared before writing rather than written on every change: the dial
    /// moves continuously while it is being spun, and none of those hundreds of
    /// intermediate frequencies is worth a file write.
    fn save_session(&mut self) {
        let Some(saved) = self.session.as_ref() else { return };
        // What the hardware reports, not what was asked for, and dropped when it
        // is empty: a front end with no antenna to choose (a CAT rig, a file)
        // must not erase the port a real radio was left on.
        let keep = |name: &str| (!name.is_empty()).then(|| name.to_string());
        // Both dials and which one was in use, so a station left listening on
        // B — or split, with the other VFO on the DX's transmit frequency —
        // comes back set up the way it was rather than with B collapsed onto A.
        let now = sdroxide_config::Session {
            freq_hz: self.state.vfo_a_hz,
            vfo_b_hz: Some(self.state.vfo_b_hz),
            active_vfo: self.state.active_vfo,
            mode: self.state.rx[0].mode,
            // The shelf is only current for the VFO that is *not* in use, so
            // the live mode is written into the active slot on the way past
            // rather than shelved here — `save_session` is called from a timer
            // and from `Drop`, and neither is a VFO change.
            vfo_modes: Some({
                let mut m = [self.vfo_memory[0].mode, self.vfo_memory[1].mode];
                m[self.state.active_vfo.index()] = self.state.rx[0].mode;
                m
            }),
            antenna_rx: keep(&self.state.antenna_rx).or_else(|| saved.antenna_rx.clone()),
            antenna_tx: keep(&self.state.antenna_tx).or_else(|| saved.antenna_tx.clone()),
            volume: self.state.rx[0].volume,
            muted: self.state.rx[0].muted,
            rx_gain_db: self.state.rx[0].manual_gain_db,
            agc: self.state.rx[0].agc,
            drive: self.state.tx.drive,
            tune_drive: self.state.tx.tune_drive,
            mic_gain: self.state.tx.mic_gain,
            cessb_db: self.state.tx.cessb_db,
            tx_eq: self.state.tx.eq,
            squelch_db: self.state.rx[0].squelch_db,
            noise_reduction: self.state.rx[0].noise_reduction,
            binaural: self.state.rx[0].binaural,
            // The standing choice again, not what the front end of the moment
            // could do with it: a session written while the radio was switched
            // off would otherwise put 1 on disk and lose it for good.
            decimation: self.want_decimation,
            repeater: self.state.repeater,
            // What the operator asked for rather than what the device currently
            // reports, for the antennas' reason again: a front end with no gain
            // to set — a CAT rig, a file — must not erase the stages a real
            // receiver was left on, and a driver that moves a gain by itself
            // (an AGC riding the IF) is not the operator changing their mind.
            gains: self.want_gains.0.clone(),
            tx_gains: self.want_gains.1.clone(),
            recording_mono: self.state.recording_mono,
            band_antenna: self.band_antenna.clone(),
        };
        if now == *saved {
            return;
        }
        match self.store.save_session(&now) {
            Ok(()) => self.session = Some(now),
            // Don't latch the new value on failure, so the next tick retries.
            Err(e) => warn!("saving the session (dial + mode + antennas + levels): {e}"),
        }
    }

    /// Write the memory list out, and say so on screen if it could not be
    /// written.
    ///
    /// A log line is not enough here. The operator has just stored, edited or
    /// filed a channel and the list in front of them shows the change, so a
    /// refused save is a lie they only find out about at the next start. The
    /// commonest refusal is the deliberate one — the file could not be read at
    /// startup, so sdroxide will not write the empty list it had to run on over
    /// it (issue #269) — and that is precisely the case they have to be told
    /// about, because the channels are still there and this screen says they
    /// are not.
    fn save_memories(&mut self) {
        if let Err(e) = sdroxide_config::save_memories(&self.memories) {
            warn!("saving memories: {e}");
            let _ = self
                .event_tx
                .send(RadioEvent::Notice(Some(format!("the memory list was not saved: {e}"))));
        }
        self.mark_shared_store_write();
        let _ = self.event_tx.send(RadioEvent::Memories(self.memories.clone()));
    }

    fn save_mem_folders(&mut self) {
        if let Err(e) = sdroxide_config::save_memory_folders(&self.mem_folders) {
            warn!("saving memory folders: {e}");
            let _ = self
                .event_tx
                .send(RadioEvent::Notice(Some(format!("the memory folders were not saved: {e}"))));
        }
        self.mark_shared_store_write();
        let _ = self.event_tx.send(RadioEvent::MemoryFolders(self.mem_folders.clone()));
    }

    /// Announce a completed write to a station-shared store, recording the new
    /// generation as already-seen so this engine does not reload its own save.
    fn mark_shared_store_write(&mut self) {
        if let Some(sync) = self.store_sync.as_ref() {
            self.shared_gen_seen = sync.bump();
        }
    }

    /// Reload the shared stores when another engine announced a write.
    fn poll_shared_stores(&mut self) {
        let Some(sync) = self.store_sync.as_ref() else { return };
        let g = sync.generation();
        if g != self.shared_gen_seen {
            self.shared_gen_seen = g;
            self.reload_shared_stores();
        }
    }

    /// Re-read the station-shared stores after another engine in this process
    /// saved one — see [`EngineSwap::ReloadSharedStores`]. Only what changed on
    /// disk moves; nothing is written back from here, so two engines nudging
    /// each other cannot ping-pong.
    fn reload_shared_stores(&mut self) {
        // Ours first. A level still sitting in `digi_config` waiting for the
        // tick is not in the file another engine just wrote, so reading that
        // file over it would discard the operator's last drag.
        self.flush_digi_config();
        let memories = sdroxide_config::load_memories();
        if memories != self.memories {
            self.memories = memories;
            let _ = self.event_tx.send(RadioEvent::Memories(self.memories.clone()));
        }
        let folders = sdroxide_config::load_memory_folders();
        if folders != self.mem_folders {
            self.mem_folders = folders;
            let _ = self.event_tx.send(RadioEvent::MemoryFolders(self.mem_folders.clone()));
        }
        self.stacks = sdroxide_config::load_bandstacks();
        let digi_config = sdroxide_config::load_digi_config();
        if digi_config != self.digi_config {
            // The same fan-out a SetDigiConfig does, minus the save: the other
            // engine already wrote the file.
            self.digi_config = digi_config;
            if let Some(d) = self.digi.as_mut() {
                d.set_config(self.digi_config.clone());
            }
            self.spots.set_operator(&self.digi_config.my_call, &self.report_grid());
            self.emit_digi_status();
        }
    }

    /// The frequency window the receivers can reach: the device passband. Both
    /// DDCs tap the same IQ stream, so anything outside it simply isn't there.
    fn passband(&self) -> (f64, f64) {
        let half = self.state.sample_rate / 2.0;
        (self.state.center_hz - half, self.state.center_hz + half)
    }

    fn clamp_to_passband(&self, hz: f64) -> f64 {
        let (lo, hi) = self.passband();
        hz.clamp(lo, hi)
    }

    /// Park the sub receiver on the inactive VFO when it has never been placed
    /// (zero), or when its frequency has fallen outside the device passband —
    /// a band change, a retune, or a sample-rate change moving the hardware out
    /// from under it. Without this the sub's DDC would sit at an offset beyond
    /// the IQ it is fed and the operator would hear silence with no indication
    /// why.
    ///
    /// The inactive VFO is the seed (rather than the dial) because that is
    /// where the sub used to live unconditionally, so switching it on for the
    /// first time still lands where it always did.
    fn reseat_sub_freq(&mut self) {
        // Nothing to park while the sub is off — and inventing a frequency for
        // it then would consume the "never placed" zero during startup, so the
        // operator's first SUB would land on a stale dial instead of on the
        // VFO they had just set up as the other place to listen.
        if !self.state.sub_rx_enabled {
            return;
        }
        let (lo, hi) = self.passband();
        if self.state.sub_rx_hz > 0.0 && (lo..=hi).contains(&self.state.sub_rx_hz) {
            return;
        }
        let inactive = match self.state.active_vfo {
            Vfo::A => self.state.vfo_b_hz,
            Vfo::B => self.state.vfo_a_hz,
        };
        // The inactive VFO can be off-passband too (split across bands): fall
        // back to the dial, which is in range by construction.
        self.state.sub_rx_hz = if (lo..=hi).contains(&inactive) {
            inactive
        } else {
            self.state.rx_freq_hz().clamp(lo, hi)
        };
    }

    /// Point the main-RX DDC at the active VFO (+RIT) and the sub-RX DDC at
    /// its own parked frequency.
    /// Swap the audio output sink at runtime (frontend changed sound devices).
    /// Rebuilds the RX chains for the new device rate; the digi tap and DDC
    /// offsets are re-armed on the fresh chains.
    fn set_audio_output(&mut self, audio: Option<AudioParams>) {
        // The recorder feeds off the mixer we're about to replace; finalize it
        // rather than leave a half-written file with a dangling feed.
        let was_recording = self.recorder.is_some();
        self.stop_recording();
        match audio {
            Some(a) => {
                self.main =
                    Some(RxChain::new(self.state.sample_rate, &self.state.rx[0], a.out_rate));
                let mut mixer = StereoMixer::new(a.producer);
                // A swap mid-announcement must not come back at full volume.
                mixer.set_duck(self.speech_duck);
                // Nor may it come back without the operator's trim: a rig whose
                // codec is quiet is quiet on every sound card.
                mixer.set_trim_db(self.rx_af_gain_db);
                self.mixer = Some(mixer);
                self.audio_out_rate = a.out_rate;
                self.sub = self
                    .state
                    .sub_rx_enabled
                    .then(|| RxChain::new(self.state.sample_rate, &self.state.rx[1], a.out_rate));
                self.sync_audio_tap();
                info!(out_rate = a.out_rate, "audio output swapped");
            }
            None => {
                self.main = None;
                self.sub = None;
                self.mixer = None;
                info!("audio output removed; running silent");
            }
        }
        self.update_tuning();
        if was_recording {
            // Reflect the auto-stop to clients (this path doesn't run `apply`).
            let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
        }
    }

    /// Swap the microphone feed at runtime.
    fn set_audio_input(&mut self, mic: Option<MicParams>) {
        self.mic_resampler = match &mic {
            Some(m) => MonoResampler::new(m.rate, 48_000.0),
            None => None,
        };
        self.mic_fifo.clear();
        match &mic {
            Some(m) => info!(rate = m.rate, "mic input swapped"),
            None => info!("mic input removed; TX carries silence"),
        }
        self.mic = mic;
    }

    /// Set how far the raw IQ is decimated before the receiver sees it.
    ///
    /// The hardware is left streaming exactly as it was: this is not a rate the
    /// device is ever told about, so there is no stream restart, no gap and no
    /// risk of a front end refusing the new setting. What does change is the
    /// rate every piece of DSP downstream was built for, and none of that can
    /// be retuned in place — so the same set a device swap rebuilds is rebuilt
    /// here, for the same reason.
    ///
    /// A request the device cannot carry (or any request at all in audio mode,
    /// where there is no IQ) comes back from [`decimation_for`] as the nearest
    /// factor that works, so the state the operator is shown is always the one
    /// the receiver is actually running.
    fn set_decimation(&mut self, factor: u32) {
        // Noted before the clamp, and whether or not anything moves: this is
        // the operator's standing choice, and it outlives the front end that
        // happens to be open when they make it.
        self.want_decimation = wanted_decimation(factor);
        let factor = decimation_for(factor, self.radio_fs, self.audio_mode);
        if factor == self.state.decimation {
            return;
        }
        self.state.decimation = factor;
        self.decim = (factor > 1).then(|| Decimator::new(factor));
        self.state.sample_rate = self.radio_fs / factor as f64;

        self.analyzer = build_analyzer(
            self.cfg.fft_size as usize,
            self.state.sample_rate,
            self.cfg.avg_tc,
            f64::from(self.cfg.rows()),
            self.analyzer_view_span(),
        );
        if self.mixer.is_some() {
            self.main =
                Some(RxChain::new(self.state.sample_rate, &self.state.rx[0], self.audio_out_rate));
            self.sub = self.state.sub_rx_enabled.then(|| {
                RxChain::new(self.state.sample_rate, &self.state.rx[1], self.audio_out_rate)
            });
        }
        // The digital-mode controller and its high-resolution waterfall are fed
        // at the main chain's channel rate, which the rebuild above just moved.
        self.digi = None;
        self.channel_analyzer = None;
        self.skimmer = None;
        self.skim_ddc = None;
        self.skim_buf.clear();
        // The ISM decoder's window is a decimation of a rate that has just
        // changed, so its chain goes too — but the device table it has built up
        // over the last few minutes is worth more than the rebuild costs, so
        // `sync_ism` is what puts it back rather than a teardown here.
        self.ism_ddc = None;
        self.ism = None;
        self.ism_buf.clear();
        // The ADS-B lane goes the same way and for the same reason, except that
        // decimating the front end is the one thing most likely to take it below
        // the two megasamples a second it cannot work without — which `sync_adsb`
        // will then say out loud rather than restarting a decoder that can only
        // find nothing.
        self.adsb_ddc = None;
        self.adsb = None;
        self.adsb_buf.clear();
        // The VDL2 lane goes the same way. Decimating the front end below about
        // 440 kHz is what takes the plan's outer channels out of reach, and
        // `sync_vdl2` says which ones are left rather than restarting a decoder
        // that will quietly hear a third of the traffic.
        self.vdl2_ddc = None;
        self.vdl2 = None;
        self.vdl2_buf.clear();
        // The AIS lane the same way: below about 100 kHz of stream the window
        // stops holding both channels, and `sync_ais` says which one is left
        // rather than restarting a decoder that will quietly hear half the
        // shipping.
        self.ais_ddc = None;
        self.ais = None;
        self.ais_buf.clear();
        // Dropped outright rather than left to `sync_tci_iq`'s own comparison:
        // two device rates can snap to the same client rate, and it would then
        // keep a decimation chain built for the rate we have just left.
        self.tci_iq_ddc = None;
        self.tci_iq_buf.clear();
        self.tci_iq_ilv.clear();
        self.sync_digi_mode();
        self.sync_skimmer();
        self.sync_ism();
        self.sync_adsb();
        self.sync_vdl2();
        self.sync_ais();
        self.sync_qo100();
        self.sync_audio_tap();
        self.sync_tci_iq();
        info!(factor, rate = self.state.sample_rate, "front-end decimation");

        // A narrower span may not reach where the sub receiver was parked, and
        // a zero-IF front end's LO offset shrinks with the span (see
        // [`Self::lo_offset_hz`]) — so both the sub and the hardware centre are
        // re-derived before the next block arrives.
        self.reseat_sub_freq();
        self.keep_vfo_in_span();
        self.update_tuning();
        self.emit_state();
        self.save_session();
    }

    /// Rebuild the IQ front-end at runtime (backend / CAT audio / HPSDR-TCI
    /// address changed). Opens the new source via the [`ReopenFn`] factory and
    /// only swaps on success, so a bad config leaves the current interface
    /// running with an on-screen error instead of going dark — for every source
    /// that can coexist with its own replacement. One that cannot (see
    /// [`IqSource::release`]) is stood down first and takes the failure case
    /// with it: it goes dark, and [`Engine::poll_reconnect`] picks it up.
    fn reopen_source(&mut self) {
        let center = self.state.active_freq_hz();
        let Some(factory) = self.reopen.clone() else {
            warn!("runtime interface switching unavailable in this build");
            return;
        };
        // Before the factory runs, not after it fails: an exclusively-claimed
        // device is the one thing standing between itself and its replacement.
        self.source.release();
        // A background attempt may hold the factory; the operator's own change
        // wins as soon as that one finishes.
        let opened = {
            let mut reopen = factory.lock().unwrap_or_else(|e| e.into_inner());
            // Holding the factory means any attempt that was already in flight
            // has finished. Its answer is stale — it was opening whatever the
            // configuration said a moment ago — so it is let go of here rather
            // than collected later, where `poll_reconnect` would adopt it on
            // top of what the operator just asked for. That is what used to
            // put a radio's interface straight back after it was switched off:
            // the switch released the device, and the attempt already running
            // claimed it again a moment later, with the switch reading OFF.
            self.abandon_retry();
            reopen(center)
        };
        // Whatever the operator just chose starts the retry schedule over.
        self.retry_at = None;
        self.retry_every = RETRY_FIRST;
        match opened {
            Ok((source, caps)) => self.adopt_source(source, caps),
            Err(e) => {
                warn!("interface change failed: {e}");
                let _ = self
                    .event_tx
                    .send(RadioEvent::Notice(Some(format!("Interface change failed: {e}"))));
            }
        }
    }

    /// Throw away a background reconnect attempt, standing down whatever it
    /// opened.
    ///
    /// Called only with the factory lock held, which is what bounds the wait:
    /// the worker holds that lock across its whole open, so by the time this
    /// runs the attempt has finished and its answer is at most one instruction
    /// away. A source it did open is *released* rather than merely dropped —
    /// an exclusively-claimed device has to be let go before its replacement
    /// can have it, and that is the method that says so.
    fn abandon_retry(&mut self) {
        let Some(rx) = self.retry.take() else { return };
        if let Ok(Ok((mut source, _))) = rx.recv_timeout(ABANDON_WAIT) {
            debug!(source = %source.describe(), "dropping a reconnect the operator overtook");
            source.release();
        }
        if let Some(j) = self.retry_join.take() {
            let _ = j.join();
        }
    }

    /// Keep trying the configured interface while the front-end is only a
    /// stand-in (see [`IqSource::needs_reopen`]): the rig wasn't there when we
    /// started — a network rig like TCI is commonly still coming up, or the app
    /// launched first — or its link has since dropped. Attaching on our own is
    /// what the operator expects; hunting for Settings → Radio → Apply is not.
    ///
    /// The attempt itself runs on a worker thread: opening a backend can block
    /// for seconds (a TCP connect to a host that never answers), and the engine
    /// loop still has to serve commands and the built-in servers meanwhile.
    fn poll_reconnect(&mut self) {
        // Collect an attempt that has finished.
        if let Some(rx) = &self.retry {
            let outcome = rx.try_recv();
            if matches!(outcome, Err(TryRecvError::Empty)) {
                return; // still connecting
            }
            self.retry = None;
            if let Some(j) = self.retry_join.take() {
                let _ = j.join(); // it has answered; this returns at once
            }
            match outcome {
                Ok(Ok((source, caps))) => {
                    self.retry_every = RETRY_FIRST;
                    self.retry_at = None;
                    info!(source = %source.describe(), "radio interface connected");
                    self.adopt_source(source, caps);
                }
                Ok(Err(e)) => {
                    // Back off: a rig that isn't there yet is the normal case
                    // here, so this must not turn into a busy retry loop.
                    self.retry_every = (self.retry_every * 2).min(RETRY_MAX);
                    self.retry_at = Some(Instant::now() + self.retry_every);
                    debug!("radio interface still unavailable: {e}");
                }
                // The worker died without answering (a panic inside a backend's
                // open). Count it as a failed attempt; the backoff keeps that
                // from becoming a thread-spawning loop.
                Err(_) => {
                    warn!("reconnect attempt died before answering");
                    self.retry_every = (self.retry_every * 2).min(RETRY_MAX);
                    self.retry_at = Some(Instant::now() + self.retry_every);
                }
            }
        }

        if !self.source.needs_reopen() {
            self.retry_at = None;
            self.retry_every = RETRY_FIRST;
            return;
        }
        let Some(factory) = self.reopen.clone() else { return };
        let now = Instant::now();
        match self.retry_at {
            // First time we notice: give the interface a moment, and say so —
            // unless the source already carries an on-screen reason (the
            // "no radio" placeholder does).
            None => {
                self.retry_at = Some(now + self.retry_every);
                if self.source.open_status().is_none() {
                    let _ = self.event_tx.send(RadioEvent::Notice(Some(
                        "Radio disconnected — reconnecting…".into(),
                    )));
                }
                return;
            }
            Some(at) if now < at => return,
            Some(_) => {}
        }

        // Same reason as in `reopen_source`, and it matters most here: a dongle
        // that has stopped delivering without dying still holds its USB
        // interface, so every attempt to replace it would be refused as busy
        // and the stream could never recover on its own.
        self.source.release();

        let center = self.state.active_freq_hz();
        let (tx, rx) = crossbeam_channel::bounded(1);
        let spawned =
            std::thread::Builder::new().name("sdroxide-reconnect".into()).spawn(move || {
                let opened = {
                    let mut reopen = factory.lock().unwrap_or_else(|e| e.into_inner());
                    reopen(center)
                };
                let _ = tx.send(opened);
            });
        match spawned {
            Ok(join) => {
                self.retry = Some(rx);
                self.retry_join = Some(join);
            }
            Err(e) => {
                warn!("could not spawn the reconnect thread: {e}");
                self.retry_every = (self.retry_every * 2).min(RETRY_MAX);
                self.retry_at = Some(now + self.retry_every);
            }
        }
    }

    /// Replace the live IQ source and rebuild every rate-dependent stage,
    /// re-initialising tuning exactly as at a cold start on the new front-end.
    /// The operator's speaker/mic (mixer + mic feed) are untouched — only the
    /// radio interface swaps.
    fn adopt_source(&mut self, source: Box<dyn IqSource>, caps: DeviceCaps) {
        // Never carry a keyed transmit across the swap.
        if self.tx_active {
            let _ = self.source.tx_end();
        }
        self.source = source;
        // The old front end's pipeline went with it.
        self.center_trail.clear();
        self.stream_center_hz = self.source.center_hz();
        self.caps = caps;
        self.caps.center_is_dial = self.source.center_is_dial();
        self.caps.cw_audio_keyed = self.source.cw_audio_keyed();
        self.caps.commands_squelch = self.source.commands_squelch();
        self.caps.commands_rig_power = self.source.commands_rig_power();
        self.caps.wide_span_hz = self.source.wide_span_hz();
        self.audio_mode = self.caps.audio_mode;
        self.radio_fs = self.source.sample_rate();
        self.audio_bw = self.source.display_bandwidth().unwrap_or(self.radio_fs / 2.0);
        // The decimation is the operator's, so it carries across the swap — but
        // re-asked of the new front end, which may not have the bandwidth to
        // spare for it (or, in audio mode, any IQ to decimate at all).
        let decimation = decimation_for(self.want_decimation, self.radio_fs, self.audio_mode);
        self.decim = (decimation > 1).then(|| Decimator::new(decimation));

        // A swap changes the front end, not the operating position. The mode,
        // filters, AGC, audio levels, transmit levels, RIT/XIT, split and the
        // sub receiver are the operator's settings and stay theirs; only what
        // genuinely describes the new hardware is taken from it.
        //
        // Starting from `RadioState::default()` here is what used to reset the
        // radio to 20 m USB on every Apply — including an Apply that changed
        // nothing, and including the *automatic* reconnect after a link drop,
        // where a radio quietly retuning and changing mode by itself is worse
        // still.
        let mut state = RadioState {
            center_hz: self.source.center_hz(),
            sample_rate: self.radio_fs / decimation as f64,
            decimation,
            gains: self.source.current_gains(),
            tx_gains: self.source.current_tx_gains(),
            antenna_rx: self.source.current_antenna(),
            antenna_tx: self.source.current_tx_antenna(),
            // The levels carry over; a keyed transmit never does.
            tx: sdroxide_types::TxState { ptt: false, tune: false, ..self.state.tx },
            // The scan was walking the old front end's span.
            scan: sdroxide_types::ScanState::default(),
            skimmer: if self.audio_mode {
                sdroxide_types::SkimmerSettings::OFF // wideband-only feature
            } else {
                self.skim_cfg // the operator's choice survives the swap
            },
            ..self.state.clone()
        };
        // `reopen` asked the new front end for the frequency we were on, so
        // normally it is exactly there and both VFOs carry over untouched. One
        // that could not go there — a swap to hardware that does not cover this
        // band — has landed somewhere else, and then its own answer is the
        // honest thing to show rather than a dial the radio is not on.
        if (state.center_hz - self.state.active_freq_hz()).abs() > 1.0 {
            state.vfo_a_hz = state.center_hz;
            state.vfo_b_hz = state.center_hz;
        }
        // …and a front end that cannot hear where the dial is takes it with it.
        // Not politeness: the dial is what every tune is judged against, so one
        // left outside the new receive range refuses every move the operator
        // makes *and* falls back to itself, which is a radio that has to be
        // typed a frequency out of thin air before it works again. Several back
        // ends put us here by answering with the frequency they were handed
        // rather than the one they clamped to, and a converter switched on
        // under a running radio moves the whole range at once.
        let mut moved = None;
        if let Some(hz) = nearest_rx_hz(&self.caps, state.active_freq_hz()) {
            let why = format!(
                "{:.6} MHz is outside this radio's receive range ({}) — the dial moved to \
                 {:.6} MHz",
                state.active_freq_hz() / 1e6,
                describe_ranges(&self.caps.freq_ranges_rx),
                hz / 1e6,
            );
            warn!("{why}");
            moved = Some(why);
            state.vfo_a_hz = hz;
            state.vfo_b_hz = hz;
        }
        state.band = Band::containing(state.active_freq_hz());
        self.state = state;
        // The frequency a refused tune goes back to belongs to the front end
        // that is on the air now: the old one's dial may be nowhere this one can
        // go, and `keep_vfo_in_span` below is about to judge exactly that.
        self.good_vfo_hz = self.state.active_freq_hz();
        // The tuning is fresh, but the antenna is a property of the station's
        // coax rather than of the front end: a radio that dropped out and came
        // back has to return to the port it was receiving on. The gain stages
        // come back for the same reason — the reopened device is on its driver
        // defaults, and only the ones this front end actually has are applied.
        self.restore_antennas();
        self.restore_gains();

        // Rebuild the device analyzer for the new rate — the decimated one off
        // an IQ front end, since that is what it is fed and what `make_frame`
        // measures its bins against; the card rate in audio mode, where the
        // analyzer FFTs the rig's audio directly.
        let analyzer_rate = if self.audio_mode { self.radio_fs } else { self.state.sample_rate };
        self.analyzer = build_analyzer(
            self.cfg.fft_size as usize,
            analyzer_rate,
            self.cfg.avg_tc,
            f64::from(self.cfg.rows()),
            self.analyzer_view_span(),
        );

        // Drop rate-dependent / stateful DSP so it rebuilds for the new source.
        self.tx = None;
        self.tx_active = false;
        // The new front end has its own PTT line and reports it from scratch.
        // Carrying "still held" across the swap would swallow the next press,
        // since only an edge keys.
        self.hw_ptt = false;
        // Likewise it has been told no transmit frequency yet, whatever the old
        // one knew.
        self.tx_freq_told = None;
        self.release_tx_gate();
        self.tx_pace = None;
        self.digi = None;
        self.digi_tx = false;
        // The keyer's recordings survive a device swap; an over in flight — or a
        // monitor running through the old audio rate — does not.
        self.voice.stop_play();
        self.voice.stop_preview();
        self.voice_prev_q.clear();
        self.voice_prev_rs = None;
        self.voice_prev_rate = 0.0;
        self.voice_tx = false;
        self.sub = None;
        self.channel_analyzer = None;
        self.skimmer = None;
        self.skim_ddc = None;
        self.skim_buf.clear();
        // The TCI server itself survives the swap — clients stay connected
        // across a device change — but everything derived from the old device
        // rate is rebuilt below by `sync_tci_iq` / `sync_audio_tap`.
        self.tci_iq_ddc = None;
        self.tci_iq_buf.clear();
        self.tci_iq_ilv.clear();
        self.tci_aud_rs = None;
        self.tci_aud_in_rate = 0.0;
        self.tci_tx = false;
        self.tci_last_snap = None;
        self.audio_re.clear();
        self.audio_play.clear();

        // Rebuild the RX / speaker path around the (unchanged) mixer.
        if self.mixer.is_some() {
            if self.audio_mode {
                self.main = None;
                self.audio_resampler = MonoResampler::new(self.radio_fs, self.audio_out_rate);
            } else {
                self.main = Some(RxChain::new(
                    self.state.sample_rate,
                    &self.state.rx[0],
                    self.audio_out_rate,
                ));
                self.audio_resampler = None;
            }
        } else {
            self.main = None;
            self.audio_resampler = None;
        }

        info!(source = %self.source.describe(), audio_mode = self.audio_mode, "radio source swapped at runtime");
        let _ = self.event_tx.send(RadioEvent::Capabilities(self.caps.clone()));
        let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
        // Surface any open warning (radio audio unavailable, …) and any config
        // file reset to defaults by the reload, or clear a stale notice.
        let mut notes: Vec<String> = self.source.open_status().into_iter().collect();
        notes.extend(moved);
        notes.extend(sdroxide_config::take_load_alerts());
        let _ =
            self.event_tx.send(RadioEvent::Notice((!notes.is_empty()).then(|| notes.join("\n"))));

        // Re-establish mode-dependent chains for the fresh state.
        self.sync_digi_mode();
        if !self.audio_mode {
            self.sync_skimmer();
            self.sync_ism();
            self.sync_adsb();
            self.sync_vdl2();
            self.sync_ais();
            self.sync_qo100();
        }
        // Re-derive the TCI streams at the new device rate and push a fresh
        // state burst, so connected clients follow the swap.
        self.sync_tci_iq();
        self.sync_audio_tap();
        self.broadcast_tci_state();
        // Same as a cold start: the fresh VFO sits on the new front end's LO,
        // which is where zero-IF hardware must not be tuned.
        self.push_rx_mode();
        self.keep_vfo_in_span();
        self.update_tuning();
        // Again, because both of those can move the centre — a dial that had to
        // be brought into the new front end's range always does — and the state
        // sent above was the one from before they ran.
        self.emit_state();
    }

    /// Keep a band-dependent sideband following the dial.
    ///
    /// Only analog SSTV has one — LSB on 160/80/40 m, USB above — so tuning
    /// across that boundary has to mirror the passband (sideband lives in the
    /// sign of the filter edges, which is what puts the demodulator on the
    /// right side and, at key-down, the modulator), and tell a rig that is
    /// doing the receiving. Every other mode's sideband is fixed, so this
    /// leaves them alone.
    fn follow_sideband(&mut self) {
        let mode = self.state.rx[0].mode;
        // `Mode::Sstv` by name rather than [`Mode::is_sstv`]: its VHF twin
        // rides an FM carrier, which has no sideband to swap.
        if mode != Mode::Sstv {
            return;
        }
        let want = mode.default_filter_at(self.state.rx_freq_hz());
        let r = &mut self.state.rx[0];
        if (r.filter_lo, r.filter_hi) == want {
            return;
        }
        (r.filter_lo, r.filter_hi) = want;
        let (lo, hi) = want;
        if let Some(d) = self.main.as_mut().and_then(|c| c.demod.as_mut()) {
            d.set_filter(lo, hi);
        }
        if self.rig_follows_rx_mode() {
            let _ = self.source.set_control_mode(self.control_mode());
            self.push_control_filter();
        }
        let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
    }

    fn update_tuning(&mut self) {
        // Before anything reads the passband below (the audio-mode window is
        // drawn on the sideband), and before the dial reaches the rig.
        self.follow_sideband();
        if self.audio_mode {
            // The rig's dial IS the VFO — command it over CAT (no DDC offset).
            // RIT has no DDC to ride on either, so it goes on the dial too: the
            // source is told the VFO and the offset separately so it can take
            // the offset back out of what the rig reports. XIT and split are
            // the same trick on the transmit side, applied by `tx_begin`, which
            // already receives `tx_freq_hz`.
            self.source.set_rit_hz(self.state.rit.effective_hz());
            let dial = self.state.active_freq_hz();
            if self.source.set_center_hz(dial).is_ok() {
                // The rig took the dial: this is where a refused tune goes back
                // to. Tracked here rather than in `keep_vfo_in_span`, which a
                // CAT rig never reaches.
                self.good_vfo_hz = dial;
            }
            self.update_display_center();
            return;
        }
        let mut main_offset = self.state.rx_freq_hz() - self.state.center_hz;
        // A satellite lock rides its Doppler correction on the DDC rather than
        // the dial: the operator keeps reading the published frequency while
        // the NCO follows the moving signal. Clamped so a dial parked at the
        // very edge of the span cannot be pushed out of the usable bandwidth.
        let doppler = self.sat_rx_doppler_hz();
        if doppler != 0.0 {
            let lim = 0.48 * self.state.sample_rate;
            main_offset = (main_offset + doppler).clamp(-lim, lim);
        }
        self.reseat_sub_freq();
        let sub_offset = self.state.sub_rx_hz - self.state.center_hz;
        // A retune is the one thing the demod cannot see for itself: the DDC
        // ahead of it absorbs the move and the composite carries on looking like
        // a station. Judged on the dial rather than on the DDC offset, which also
        // moves when the front end recentres or a satellite lock tracks Doppler
        // — neither of which is a different station.
        let dial = self.state.rx_freq_hz();
        if (dial - self.rds_dial_hz).abs() > RDS_RETUNE_HZ {
            self.rds_dial_hz = dial;
            if let Some(c) = self.main.as_mut() {
                c.reset_rds();
            }
        }
        if (dial - self.drm_dial_hz).abs() > DRM_RETUNE_HZ {
            self.drm_dial_hz = dial;
            if let Some(c) = self.main.as_mut() {
                c.reset_drm();
            }
        }
        if let Some(c) = self.main.as_mut() {
            c.set_offset_hz(main_offset);
        }
        if let Some(c) = self.sub.as_mut() {
            c.set_offset_hz(sub_offset);
        }
        // Keep a wideband-IQ rig's own VFO on our dial (TCI); no-op elsewhere. This
        // way returning from TX doesn't snap the rig back to the IQ centre.
        self.source.set_if_offset(main_offset);
        // On a screen wider than the skim window the dial decides which part of
        // it is skimmed ([`skim_center_for`]), and tuning inside the span moves
        // the dial without moving the front end or the client's view — so this
        // is the only place that would notice. A handful of comparisons, and the
        // dead band means ordinary tuning changes nothing.
        self.sync_skim_window();
    }

    /// Tell the source where we would transmit, for the band-switching hardware
    /// that has to know before the operator keys (see
    /// [`IqSource::set_tx_freq_hz`]).
    ///
    /// Polled from the engine loop rather than pushed from the places that move
    /// the transmit frequency. There are a lot of those — the dial, VFO select,
    /// split, XIT, a band or memory recall, a satellite uplink, a scanner step,
    /// rigctld — and several change it without going near `update_tuning`, so
    /// instrumenting them all is a list that would silently fall out of date.
    /// Deriving it costs one comparison per iteration.
    fn push_tx_freq(&mut self) {
        let hz = match self.sat_lock.as_ref().and_then(|l| l.cfg.uplink) {
            Some(u) => u.uplink_for(self.state.active_freq_hz()) + self.state.xit.effective_hz(),
            None => self.state.tx_freq_hz(),
        };
        if self.tx_freq_told != Some(hz) {
            self.tx_freq_told = Some(hz);
            self.source.set_tx_freq_hz(hz);
            // The band the amplifier is about to work has changed with it, and
            // with the band its calibration (issue #295). Same reason this is
            // derived here rather than pushed from the dozen places that move
            // the transmit frequency.
            self.refresh_drive_trim(hz);
        }
    }

    // ── Repeater operation ──────────────────────────────────────────────────

    /// Take a new repeater setting and make the transmit chain agree with it.
    fn set_repeater(&mut self, r: RepeaterState) {
        let was = self.state.repeater;
        self.state.repeater = r;
        // AUTO may have just been switched on, or its offset overridden by
        // hand: ask the plan again rather than waiting for the dial to move.
        self.auto_shift_dial = None;
        self.refresh_auto_shift();
        // Rebuilt rather than retuned, because a generator holds the phase of
        // the tone it is part-way through and there is no sense in carrying
        // that across to a different one.
        if self.state.repeater.tx_tone() != was.tx_tone() {
            self.rebuild_sub_tone();
        }
        self.emit_state();
    }

    /// Build (or drop) the sub-audible generator from the current setting.
    fn rebuild_sub_tone(&mut self) {
        self.sub_tone = self.state.repeater.tx_tone().map(|t| SubToneGen::new(t, TX_MONITOR_RATE));
    }

    /// Follow the band plan's repeater shift as the dial moves.
    ///
    /// The plan's answer is *resolved* into the state's own `shift` and
    /// `offset_hz` rather than being left as a rule to re-run: that way the
    /// figure on every screen, the one a memory stores and the one
    /// [`RadioState::tx_freq_hz`] transmits on are the same figure, and only
    /// one of them had to consult a table.
    ///
    /// Where the plan says nothing the shift goes to simplex — the safe
    /// answer, and the honest one: outside a repeater sub-band there is no
    /// standard shift to follow. The offset magnitude is left alone, so
    /// switching AUTO off gives back the figure that was set by hand.
    ///
    /// Polled from the engine loop rather than pushed from everything that
    /// moves the dial, for the same reason [`Engine::push_tx_freq`] is: there
    /// are a lot of those and instrumenting them all is a list that would
    /// silently fall out of date. It costs one comparison per iteration.
    fn refresh_auto_shift(&mut self) {
        if !self.state.repeater.auto {
            self.auto_shift_dial = None;
            return;
        }
        let dial = self.state.active_freq_hz();
        if self.auto_shift_dial == Some(dial) {
            return;
        }
        self.auto_shift_dial = Some(dial);
        let (shift, offset_hz) = sdroxide_types::standard_shift(dial)
            .unwrap_or((sdroxide_types::Shift::Simplex, self.state.repeater.offset_hz));
        let r = &mut self.state.repeater;
        if (r.shift, r.offset_hz) == (shift, offset_hz) {
            return;
        }
        (r.shift, r.offset_hz) = (shift, offset_hz);
        self.emit_state();
    }

    /// Send the 1750 Hz burst that opens a carrier-access repeater.
    ///
    /// Mid-over it plays over the microphone. From receive it keys the
    /// transmitter, sends the burst and unkeys again — a whole over of its own,
    /// which is what the burst button on a European mobile is. The key-down
    /// goes through [`Engine::set_ptt`] like any other, so every transmit rail
    /// applies and a refused one leaves nothing behind.
    fn fire_tone_burst(&mut self) {
        if self.state.rx[0].mode != Mode::Nfm {
            return self.notice(
                "the 1750 Hz burst is an FM repeater's door-opener — switch to NFM to send one",
            );
        }
        let keyed = self.tx_active;
        if !keyed {
            self.set_ptt(true);
            if !self.tx_active {
                return; // refused; `deny_tx` has already said why
            }
        }
        // After the key-down, so it outlives the burst `sync_tx_state` arms for
        // an over that starts on one.
        self.burst = Some(ToneBurst::new(self.state.repeater.burst_ms, TX_MONITOR_RATE));
        self.burst_unkeys = !keyed;
    }

    /// End an over that existed only to carry a burst. Called when the burst
    /// finishes inside a transmit block; a no-op for one fired mid-over, whose
    /// over belongs to the operator.
    fn end_burst_over(&mut self) {
        if !std::mem::take(&mut self.burst_unkeys) {
            return;
        }
        // Let the tone reach the air before PTT drops — a burst chopped in half
        // opens nothing, the same reason an FT8 burst drains first.
        self.source.tx_drain();
        self.tx_pace = None;
        self.state.tx.ptt = false;
        self.sync_tx_state();
        self.emit_state();
    }

    /// The output power to command on a rig that has its own power control:
    /// the TUNE level while tuning, the drive level otherwise. We tune by
    /// transmitting a carrier through the normal TX path rather than through the
    /// rig's own TUNE function, so without this the tune level would never reach
    /// the rig and a tune would go out at the (typically much lower) voice
    /// drive.
    fn tx_power_level(&self) -> f32 {
        if self.state.tx.tune { self.tx_tune_level() } else { self.tx_drive() }
    }

    /// The drive actually used: the operator's setting, calibrated for the band
    /// it is going out on and then held under whatever ceiling the converter in
    /// front of the radio imposes.
    ///
    /// Both corrections in one place, in that order, because they answer
    /// different questions and only one of them is a limit. The band trim is a
    /// calibration — every amplifier has a different gain on every band, and
    /// this is what makes one Drive setting mean one output power across all of
    /// them (issue #295). The ceiling is a hard limit and therefore last: a
    /// transverter's I.F. input takes milliwatts, and the drive that is right
    /// for the radio's own bands destroys it, so no calibration may lift the
    /// drive back over it (issue #278).
    ///
    /// Neither moves the operator's slider. The number they set for HF is still
    /// there when the dial leaves the transverter's band, and the trim is a
    /// property of the station rather than of the setting.
    fn calibrated(&self, want: f32) -> f32 {
        self.under_ceiling(want * self.drive_trim()).clamp(0.0, 1.0)
    }

    fn under_ceiling(&self, want: f32) -> f32 {
        match self.source.tx_drive_ceiling() {
            Some(c) => want.min(c.clamp(0.0, 1.0)),
            None => want,
        }
    }

    /// The multiplier this band's calibration puts on the drive — decibels of
    /// output power converted for whichever kind of drive control this source
    /// has (see [`sdroxide_types::BandDriveTrim::factor_for`]).
    fn drive_trim(&self) -> f32 {
        sdroxide_types::BandDriveTrim::factor_for(
            self.drive_trim_db,
            self.source.commands_tx_power(),
        )
    }

    /// Push the operator's receive-audio trim into the live mixer.
    ///
    /// Silent when there is no mixer: a radio started with no audio sink still
    /// holds the figure, and the mixer that is eventually built for it is seeded
    /// from the same field.
    fn refresh_af_trim(&mut self) {
        let db = self.rx_af_gain_db;
        if let Some(mixer) = self.mixer.as_mut() {
            mixer.set_trim_db(db);
        }
    }

    /// Work out this band's drive calibration afresh — after a retune, and
    /// after the operator edits the table.
    ///
    /// Free on a station that has not made one: an empty table is the
    /// overwhelming majority, and it never reaches the band-plan lookup.
    fn refresh_drive_trim(&mut self, tx_dial_hz: f64) {
        let (band, db) = if self.drive_trim.is_empty() {
            (sdroxide_types::Band::Gen, 0.0)
        } else {
            let band = sdroxide_types::Band::containing(tx_dial_hz);
            let db = self
                .drive_trim
                .iter()
                .find(|t| t.band == band)
                .map(sdroxide_types::BandDriveTrim::db)
                .unwrap_or(0.0);
            (band, db)
        };
        if db == self.drive_trim_db {
            return;
        }
        debug!("TX drive calibration: {db:+.1} dB on {} ({tx_dial_hz:.0} Hz)", band.label());
        self.drive_trim_db = db;
        // A rig that holds its own power setting has already been told the old
        // one; it is not keyed (this only runs when the transmit frequency
        // moves or the table is edited), so correct it now rather than leaving
        // the next over to open at the previous band's level.
        if self.source.commands_tx_power() && !self.tx_active {
            let level = self.tx_power_level() as f64;
            self.source.set_tx_drive(level);
        }
    }

    /// [`Self::calibrated`] applied to the transmit drive, which is what scales
    /// the modulated I/Q.
    fn tx_drive(&self) -> f32 {
        self.calibrated(self.state.tx.drive)
    }

    /// …and to the TUNE level, which is the drive for as long as TUNE holds the
    /// transmitter. The same two corrections apply: a tune is RF out of the
    /// same amplifier and into the same transverter as an over, and a tune that
    /// ignored the converter's ceiling would be the one transmission most
    /// likely to destroy its I.F. input — it is a carrier, held.
    fn tx_tune_level(&self) -> f32 {
        self.calibrated(self.state.tx.tune_drive)
    }

    /// Key or unkey from an operator PTT — the on-screen button, a MIDI or
    /// keyboard binding, or the radio's own PTT line ([`Self::apply_hw_ptt`]).
    /// Every route lands here so they cannot drift apart.
    fn set_ptt(&mut self, on: bool) {
        // …and over a burst-only over. A hand on PTT while the 1750 Hz burst is
        // still playing means the operator is about to speak, so the over
        // becomes theirs and the end of the burst must not take it away from
        // them. Before `fire_tone_burst` arms its own, which sets this again
        // for the over it is keying.
        self.burst_unkeys = false;
        // The operator takes precedence over a TCI client: keying up
        // locally mid-over takes the transmitter back rather than
        // swapping the on-air audio out from under whoever is talking.
        self.end_tci_tx();
        // Same rule for the voice keyer: a hand on PTT ends the
        // recorded message rather than talking over it. Releasing PTT
        // stops it too — that is the natural "shut up" gesture.
        self.cancel_voice_play();
        // A digital-voice mode owns its own over: it has to build the
        // first modem frame before there is anything to send, and it
        // has to append the end-of-over frame afterwards. Route PTT
        // through the mode so the main button and the panel's transmit
        // button do the same thing.
        if self.state.rx[0].mode.is_rade() {
            if let Some(d) = self.digi.as_mut() {
                d.set_tx_active(on);
                self.emit_digi_status();
                return;
            }
        }
        self.state.tx.ptt = on;
        self.sync_tx_state();
    }

    /// The transceiver has keyed or unkeyed itself, and the receiver on this
    /// side is still running: give the loudspeaker the same treatment an over
    /// of our own would.
    ///
    /// Nothing here drives that over — see [`ControlUpdate::RigTx`] — so the
    /// engine never enters the transmit path for it, and the receive read at
    /// the top of the engine loop keeps going. On an arrangement where the
    /// receiver is a different device from the transmitter, that means it is
    /// listening to the station's own transmitter for the length of the over:
    /// an RTL-SDR watching the antenna a KX3 is transmitting into (issue #244),
    /// or a dongle on the rig's I.F. output. "Mute on transmit" was already
    /// answered for overs sdroxide keys; an over the operator keys with the
    /// microphone in their hand is the one they are most likely to be holding
    /// a headphone to their ear through.
    ///
    /// Whether to do it at all is the front end's to say
    /// ([`IqSource::mutes_rx_audio_on_rig_tx`]) — for a paired receiver it is
    /// the operator's own **Mute on transmit**, and everywhere else an over is
    /// an over.
    ///
    /// The picture is deliberately left running. A receiver that is still
    /// receiving has something to show, and an operator on the microphone
    /// watching their own signal go out is a reason to have paired one at all;
    /// **Blank on transmit** answers for the overs sdroxide keys, where there
    /// is nothing coming in to draw.
    ///
    /// The recording tap goes quiet with the speaker, as it does on our own
    /// overs: what it would capture is not a band.
    fn follow_rig_tx(&mut self, on: bool) {
        // Our own over owns the mixer while it lasts, and its unkey is what
        // clears these. A rig report must never be the thing that un-mutes one.
        // (The CAT thread already drops the rig's transmit reads across our own
        // PTT edges, so this is the belt to those braces.)
        if self.tx_active {
            return;
        }
        if !self.source.mutes_rx_audio_on_rig_tx() {
            return;
        }
        if let Some(mixer) = self.mixer.as_mut() {
            mixer.tx_muted = on;
            mixer.rx_rec_enabled = !on;
        }
    }

    /// The radio's own PTT line changed state — a foot switch, a mic button, or
    /// whatever is wired to the board's PTT input.
    ///
    /// A key-down is only taken when nothing else already owns the
    /// transmitter, and the matching key-up is then only honoured if this line
    /// is what started the over. Both halves of that rule exist for the same
    /// reason: several boards report their PTT pin and their own MOX state on
    /// one line, so an over *we* started (TUNE, an FT8 burst, the voice keyer,
    /// a TCI client, the on-screen button) comes straight back as "the operator
    /// is holding PTT". Adopting that would be harmless on its own — we are
    /// already transmitting — but the state would outlive the over that caused
    /// it and hold the transmitter down after the real owner let go.
    fn apply_hw_ptt(&mut self, closed: bool) {
        if closed == self.hw_ptt {
            return; // a level, re-reported; only edges do anything
        }
        if closed && (self.tx_active || self.state.tx.tune || self.digi_tx) {
            return; // someone else owns this over — and owns its key-up too
        }
        self.hw_ptt = closed;
        info!("radio PTT line {}", if closed { "closed" } else { "open" });
        self.set_ptt(closed);
    }

    /// Reconcile the TX hardware state with `ptt || tune`, enforcing the
    /// safety rails on key-down.
    fn sync_tx_state(&mut self) {
        let want_tx = self.state.tx.ptt || self.state.tx.tune;
        if want_tx == self.tx_active {
            return;
        }
        if want_tx {
            // The SWR latch comes before every other rail, including the
            // station interlock. It is a pure state check with no side effects
            // to unwind, and it is the one refusal that says the antenna system
            // itself is suspect: there is no point acquiring the transmit gate
            // to key into a fault.
            if let Some(swr) = self.swr_tripped {
                return self.deny_tx(&format!(
                    "SWR guard tripped at {swr:.1}:1. Acknowledge it to transmit again"
                ));
            }
            // The radio is already transmitting under its own control —
            // somebody has their hand on its microphone. Keying here would put
            // sdroxide's audio into an over that is not its to drive, and on a
            // rig whose key-down doubles as an audio-source switch it would cut
            // the operator off mid-word. Placed with the other pure state
            // checks, before anything acquires or tears down.
            if self.rig_tx {
                return self.deny_tx("the radio is transmitting on its own PTT");
            }
            // The station's transmit interlock: one radio on the air at a
            // time. First among the rails, before anything with side effects,
            // so a refused key-up on this radio leaves it exactly as it was.
            if self.tx_gate.as_ref().is_some_and(|g| !g.try_acquire(self.instance)) {
                let msg = match self.tx_gate.as_ref().and_then(|g| g.holder()) {
                    Some(id) => format!("radio {} is on the air", id + 1),
                    None => "another radio is on the air".to_string(),
                };
                return self.deny_tx(&msg);
            }
            // A recording reads the same microphone the transmitter does, so
            // keying up — by any route: PTT, TUNE, a digital burst, a TCI
            // client — ends it and stores what was captured. The local monitor
            // goes too: it rides the receive path, which stands still during a
            // (half-duplex) over.
            if self.voice.is_recording() {
                self.voice.stop_record();
                self.emit_voice_status();
            }
            self.stop_voice_preview();
            // While a satellite lock is active the transmit frequency is the
            // transponder's answer to the dial, not the dial itself — split
            // and VFO B are ignored rather than rewritten. XIT survives as
            // the operator's trim on the mapped uplink. The safety rails
            // below then vet the real uplink, which is the point: an
            // out-of-band transponder pairing is caught here.
            let txf = match self.sat_lock.as_ref() {
                Some(l) => match l.cfg.uplink {
                    Some(u) => {
                        u.uplink_for(self.state.active_freq_hz()) + self.state.xit.effective_hz()
                    }
                    None => {
                        return self.deny_tx(
                            "this satellite lock is receive-only — the chosen link has no uplink",
                        );
                    }
                },
                None => self.state.tx_freq_hz(),
            };
            if !self.caps.is_transmit_capable() {
                return self.deny_tx("this device cannot transmit");
            }
            if !self.caps.may_tx_hz(txf) {
                return self.deny_tx(&format!(
                    "{:.6} MHz is outside this radio's transmit range",
                    txf / 1e6
                ));
            }
            if self.tx_ham_only && Band::containing(txf) == Band::Gen {
                return self.deny_tx(
                    "outside amateur bands (set tx_ham_only = false in config.toml, or pass \
                     --oob-tx, if you are licensed to transmit here)",
                );
            }
            // The dial is not what goes out. A digital mode transmits at the
            // dial PLUS its audio offset, so every check above — this radio's
            // range, the amateur-band rail — has been vetting a frequency we do
            // not radiate. On most bands the gap does not matter, because the
            // conventional dial sits kilohertz below the edge. Where a licence
            // is narrower than the band plan it matters entirely.
            //
            // UK 60 m is the case that earned it: on a 5357 kHz dial the
            // allocation ends at 5358.0, an audio offset over 1 kHz is out of
            // band, and `Band::containing` says 60 m throughout because the
            // built-in table carries the WRC-15 allocation (5351.5–5366.5) that
            // most of Region 1 actually has.
            //
            // So this is checked against the SUB-SEGMENTS rather than the band
            // edges, because `bandplan.json` holds one range per band and eleven
            // would be needed. An operator whose licence is narrower says so by
            // narrowing the segments in that file; nothing here is UK-specific.
            //
            // It fails OPEN where the table says nothing: unless the dial is
            // inside a listed segment, no opinion is offered. A band plan with a
            // gap in it must not become a transmit lockout for the operator who
            // is legitimately in that gap.
            if self.tx_ham_only
                && let Some(d) = self.digi.as_ref()
                && let Some(bw) = d.mode().occupied_bw_hz()
                && sdroxide_types::segment_kind_at(txf).is_some()
            {
                // Upper sideband, and the offset is the signal's LOWEST tone —
                // the figure the waterfall shows and the one WSJT-X's Tx Freq
                // means — so the emission runs from there up by its bandwidth.
                let lo = txf + f64::from(d.audio_hz());
                let hi = lo + f64::from(bw);
                if !sdroxide_types::span_within_segment(lo, hi) {
                    return self.deny_tx(&format!(
                        "{:.3} kHz dial + {:.0} Hz offset puts {} from {:.3} to {:.3} kHz, which \
                         leaves the band plan's segment — lower the transmit offset, or widen \
                         the segment in bandplan.json if your licence allows it",
                        txf / 1e3,
                        d.audio_hz(),
                        d.mode().label(),
                        lo / 1e3,
                        hi / 1e3,
                    ));
                }
            }
            // Assert the app's current mode and power levels to the rig before
            // keying, so a CAT/TCI rig transmits in the right modulation at the
            // right drive even when the operator hasn't touched those controls
            // this session — otherwise the rig keeps its own (e.g. 0 % drive → no
            // output, or a stale/empty modulation). No-ops for IQ sources, which
            // apply mode and drive in the modulator chain instead.
            let _ = self.source.set_control_mode(self.control_mode());
            self.source.set_tx_drive(self.tx_power_level() as f64);
            self.source.set_tune_drive(self.tx_tune_level() as f64);
            // In audio mode `tx_begin` just asserts CAT PTT; there is no
            // modulator/DUC (the rig modulates the audio we feed its sound card).
            let begin_rate = if self.audio_mode { self.radio_fs } else { self.state.sample_rate };
            // On a radio that makes its own CW carrier the VFO *is* the transmit
            // frequency, and the contact sits a sidetone above the dial — the
            // same offset the receive window already rides on.
            let rig_txf = txf + self.rig_cw_offset_hz();
            // Every rail has passed and the station is about to be on the air.
            // The external T/R switch leads the RF — contacts first, drive
            // second, always — which is the one guarantee that whole subsystem
            // exists to make. Placed after the rails on purpose, so a refused
            // key-down never touches the hardware and there is nothing to
            // unwind.
            if let Some(hub) = self.tr_switch.as_ref() {
                if let Some(why) = hub.refusal() {
                    return self.deny_tx(&why);
                }
            }
            self.lead_tr_switch();
            match self.source.tx_begin(rig_txf, begin_rate) {
                Ok(tx_rate) => {
                    // Rate-match the digital modes to whatever this radio
                    // actually plays.
                    //
                    // A `DigiEngine` keeps one clock for both directions and
                    // synthesises at the *receive* tap's rate; on some radios
                    // that is not the rate the transmit stream runs at. An Icom
                    // on its 12 kHz IF output is the case that found it (issue
                    // #150): the IF arrives decimated to 24 kHz while transmit
                    // audio goes back at the session's 48, so a packet burst
                    // went out at exactly twice its baud rate — structurally
                    // perfect, half as long, and undecodable by anything.
                    //
                    // The target differs by path. Where the radio modulates the
                    // audio we hand it, that is the rate it consumes. Where we
                    // modulate it ourselves, `TxChain`'s upconverter is built
                    // for 48 kHz and the device rate is downstream of it.
                    let want =
                        if self.audio_mode || self.caps.tx_audio { tx_rate } else { 48_000.0 };
                    self.digi_tx_fifo.clear();
                    self.digi_tx_done = false;
                    self.digi_tx_rs = if (want - self.digi_rate).abs() < 0.5 {
                        None
                    } else {
                        info!(
                            from = self.digi_rate,
                            to = want,
                            "digital transmit audio is being rate-matched to the radio"
                        );
                        MonoResampler::new(self.digi_rate, want)
                    };
                    // No modulator/DUC when the device transmits raw audio (a CAT
                    // rig, or a TCI rig with wideband-IQ RX + audio TX).
                    if !self.audio_mode && !self.caps.tx_audio {
                        // Across an inverting transponder the sideband flips:
                        // transmit LSB to come out of the downlink as the USB
                        // it is being listened to on.
                        let rx0 = &self.state.rx[0];
                        let mut mode = rx0.mode;
                        let inverting = self
                            .sat_lock
                            .as_ref()
                            .is_some_and(|l| l.cfg.uplink.is_some_and(|u| u.inverting));
                        if inverting {
                            mode = match mode {
                                Mode::Usb => Mode::Lsb,
                                Mode::Lsb => Mode::Usb,
                                m => m,
                            };
                        }
                        // A flipped sideband also mirrors the filter edges:
                        // USB's positive Hz-from-carrier range becomes LSB's
                        // negative one (and back), keeping the same width.
                        let passband = if inverting {
                            (-rx0.filter_hi, -rx0.filter_lo)
                        } else {
                            (rx0.filter_lo, rx0.filter_hi)
                        };
                        self.tx = Some(TxChain::new(mode, tx_rate, passband));
                    }
                    self.tx_center_hz = txf;
                    // Seed the TX-side Doppler NCO before the first block goes
                    // out, so even a short over starts corrected.
                    self.update_sat_tx_nco();
                    self.tx_active = true;
                    // From here to the unkey below, the receive read at the top
                    // of this loop is skipped for a half-duplex source. A
                    // receiver that is not the transmitter keeps streaming into
                    // a buffer nobody drains, so tell it that what it is about
                    // to throw away is an over and not an overrun. Full duplex
                    // is left alone: it is still being read, so anything it
                    // drops there it really did drop.
                    if !self.caps.full_duplex {
                        self.source.set_rx_paused(true);
                    }
                    // A front end that keeps receiving through the over on a
                    // *different* radio from the one keyed (an attached
                    // panadapter receiver) hears our own transmitter; silence
                    // the speakers for its length without stopping it.
                    let tx_mute = self.source.mutes_rx_audio_on_tx();
                    if let Some(mixer) = self.mixer.as_mut() {
                        mixer.rx_rec_enabled = false;
                        mixer.tx_muted = tx_mute;
                    }
                    // Start the TX monitor + the real-time pacer clean (no residue
                    // from a prior burst/over) and drop any stale mic audio so the
                    // feed can't start already behind.
                    self.tx_analyzer.reset();
                    self.tx_pace = None;
                    self.mic_fifo.clear();
                    // Same reason: the EQ's delay line still holds the last
                    // over's audio, and an over should not open on its ring-out.
                    self.tx_eq.reset();
                    // The repeater's signalling starts clean with the over: a
                    // tone generator carries the phase of the tone it was
                    // part-way through, and a DCS word its place in the word.
                    self.rebuild_sub_tone();
                    // A repeater that opens on a 1750 Hz burst wants it before
                    // the first word. Not for a tune, which is a carrier and
                    // not an over, and not for a digital burst, which owns its
                    // own audio from the first sample.
                    if self.state.repeater.burst_auto
                        && self.state.rx[0].mode == Mode::Nfm
                        && !self.state.tx.tune
                        && !self.digi_tx
                    {
                        self.burst =
                            Some(ToneBurst::new(self.state.repeater.burst_ms, TX_MONITOR_RATE));
                        self.burst_unkeys = false;
                    }
                }
                Err(e) => {
                    // The contacts were thrown a moment ago and nothing came of
                    // it. Dropped with no hold: there was never anything on the
                    // air to protect, and a receiver held off for it would be
                    // deaf for no reason.
                    if let Some(hub) = self.tr_switch.as_ref() {
                        hub.abort(self.instance);
                    }
                    self.deny_tx(&format!("the radio refused to key: {e}"))
                }
            }
        } else {
            if let Err(e) = self.source.tx_end() {
                warn!("tx_end: {e}");
            }
            // Immediately after the radio has been told to stop, so the hold
            // times are measured from when the RF actually ends rather than
            // from whenever the next loop tick happens to come round. Returns
            // at once — the holds are the driver's business and this thread has
            // a receiver to get back to.
            if let Some(hub) = self.tr_switch.as_ref() {
                hub.unkey(self.instance);
            }
            // Give a rig with its own power control its operating level back.
            // TUNE holds it at the (deliberately low) tune level for the length
            // of the tune, and a radio left there afterwards is one that answers
            // the next call — from its own hand microphone, where nothing here
            // is in the way — at a few watts.
            if self.source.commands_tx_power() {
                let d = self.tx_drive();
                self.source.set_tx_drive(d as f64);
            }
            // The radio kept streaming RX for the whole over; PTT only
            // stopped us polling it. Drop the backlog instead of replaying
            // it as fresh RX on the next read.
            self.source.discard_pending_rx();
            // Strictly after the drain, which is what makes a backlog latch
            // unnecessary: the buffer is empty again before the backend is told
            // to resume counting, so the discards either side of the unkey land
            // on the right side of the line without anyone having to guess.
            if !self.caps.full_duplex {
                self.source.set_rx_paused(false);
            }
            self.report_silent_microphone();
            self.tx = None;
            self.tx_active = false;
            // A burst belongs to the over that was carrying it; an over cut
            // short must not leave one armed to play into the next.
            self.burst = None;
            self.burst_unkeys = false;
            // The run of over-limit readings belongs to the over that just
            // ended. The LATCH deliberately survives: that is what makes it a
            // latch rather than a per-over warning.
            self.swr_over = 0;
            self.release_tx_gate();
            if let Some(mixer) = self.mixer.as_mut() {
                mixer.rx_rec_enabled = true;
                mixer.tx_muted = false;
            }
            self.tx_pace = None;
            // Drop the transmit residue so the first receive frames aren't a
            // blend of TX samples and fresh RX. The zoom lane held its average
            // through the over without being fed, so its residue is older
            // still — and its decimator holds filter state from before the key
            // went down. Dropped whole; the next block rebuilds it.
            self.analyzer.reset();
            self.zoom = None;
        }
    }

    // ── Voice keyer ─────────────────────────────────────────────────────────

    fn emit_voice_status(&mut self) {
        let _ = self.event_tx.send(RadioEvent::VoiceStatus(self.voice.status()));
    }

    /// Replace one block of speaker audio with the message being monitored.
    ///
    /// `n` is the block length the receive path just produced, so taking
    /// exactly that many samples paces the monitor to real time — the same
    /// trick the digital-voice substitution uses. Returns true when
    /// `voice_prev_out` holds the block to play, false when nothing is being
    /// monitored or the message has just played out.
    fn take_preview_audio(&mut self, out_rate: f64, n: usize) -> bool {
        if !self.voice.is_previewing() || n == 0 {
            return false;
        }
        if (out_rate - self.voice_prev_rate).abs() > 0.01 {
            self.voice_prev_rate = out_rate;
            self.voice_prev_rs = MonoResampler::new(crate::voice::VOICE_RATE, out_rate);
            self.voice_prev_q.clear();
        }
        while self.voice_prev_q.len() < n {
            let mut block = [0.0f32; TX_AUDIO_BLOCK];
            let got = self.voice.fill_preview(&mut block);
            if got == 0 {
                break; // end of the message
            }
            match self.voice_prev_rs.as_mut() {
                Some(r) => r.push(&block[..got], &mut self.voice_prev_q),
                None => self.voice_prev_q.extend_from_slice(&block[..got]),
            }
        }
        if self.voice_prev_q.is_empty() {
            self.stop_voice_preview();
            return false;
        }
        let rx0 = &self.state.rx[0];
        // The operator's own volume control, as for any other received audio.
        let vol = if rx0.muted { 0.0 } else { rx0.volume * rx0.volume };
        let take = self.voice_prev_q.len().min(n);
        self.voice_prev_out.clear();
        self.voice_prev_out.extend(self.voice_prev_q.drain(..take).map(|s| s * vol));
        // The tail of the last block: silence, so the output stays paced.
        self.voice_prev_out.resize(n, 0.0);
        true
    }

    fn stop_voice_preview(&mut self) {
        if !self.voice.is_previewing() {
            return;
        }
        self.voice.stop_preview();
        self.voice_prev_q.clear();
        self.voice_tick = None;
        self.emit_voice_status();
    }

    /// Feed a recording from the microphone, and end a keyer over once its
    /// message has played out. Called once per engine iteration.
    fn poll_voice(&mut self) {
        if self.voice.is_recording() {
            self.record_voice_block();
        }
        // An over that never reached the air — the transmit rails refused, or a
        // digital-voice burst was aborted — must release the keyer rather than
        // leave it playing into nothing with the button lit.
        if self.voice_tx
            && !self.tx_active
            && !self.digi_tx
            && self.voice_started.is_some_and(|t| t.elapsed() > Duration::from_secs(1))
        {
            warn!("voice keyer: transmit never started; message cancelled");
            self.cancel_voice_play();
            return;
        }
        // `play_finished` only means the message has been read *out of* the
        // keyer; the last blocks are still in the transmit FIFO, and unkeying
        // on it alone would chop the tail.
        if self.voice_tx && self.voice.play_finished() && self.mic_fifo.len() < TX_AUDIO_BLOCK {
            // The message is out. Unkey through the mode's own path, then let
            // go of the transmitter once the over has actually ended — a
            // digital-voice mode still has its end-of-over frame to send, and
            // holding `voice_tx` keeps the live mic out of it.
            self.release_voice_tx();
            if !self.tx_active && !self.digi_tx {
                self.voice_tx = false;
                self.voice.stop_play();
                self.mic_fifo.clear();
                self.voice_tick = None;
                self.emit_voice_status();
                return;
            }
        }
        // Publish the moving position a few times a second while something runs.
        if self.voice.is_recording() || self.voice.is_playing() || self.voice.is_previewing() {
            let now = Instant::now();
            let due = self.voice_tick.is_none_or(|t| now.duration_since(t).as_millis() >= 200);
            if due {
                self.voice_tick = Some(now);
                self.emit_voice_status();
            }
        }
    }

    /// Drain the microphone into the running recording, stopping at the cap.
    fn record_voice_block(&mut self) {
        self.voice_rec_buf.clear();
        if let Some(mic) = self.mic.as_mut() {
            let mut raw = Vec::with_capacity(mic.consumer.slots());
            while let Ok(s) = mic.consumer.pop() {
                raw.push(s);
            }
            match &mut self.mic_resampler {
                Some(r) => r.push(&raw, &mut self.voice_rec_buf),
                None => self.voice_rec_buf.extend_from_slice(&raw),
            }
        }
        if self.voice_rec_buf.is_empty() {
            return;
        }
        let buf = std::mem::take(&mut self.voice_rec_buf);
        let room = self.voice.push_mic(&buf);
        self.voice_rec_buf = buf;
        if !room {
            info!("voice keyer: length cap reached; recording stored");
            self.voice.stop_record();
            self.emit_voice_status();
        }
    }

    /// Transmit slot `slot`, keying up the same way the operator's PTT does.
    fn start_voice_play(&mut self, slot: usize) {
        if !self.state.rx[0].mode.allows_voice_keyer() {
            warn!("voice keyer: not available in {}", self.state.rx[0].mode.label());
            return;
        }
        if self.state.tx.tune {
            warn!("voice keyer: TUNE is active; turn it off first");
            return;
        }
        if self.voice.is_recording() {
            self.voice.stop_record();
        }
        if self.state.rx[0].mode.is_rade() && self.digi.is_none() {
            warn!("voice keyer: the digital-voice modem is not running");
            return;
        }
        if !self.voice.start_play(slot) {
            // An empty slot is a no-op, not a keyed transmitter with nothing to
            // say. This is what makes the shipped numpad bindings harmless on a
            // fresh installation.
            return;
        }
        // A local message takes the transmitter back from a TCI client, exactly
        // as an operator PTT does.
        self.end_tci_tx();
        self.voice_tx = true;
        self.voice_tick = None;
        self.voice_started = Some(Instant::now());
        self.mic_fifo.clear();
        if self.state.rx[0].mode.is_rade() {
            // Digital voice owns its own over (it has to build the first modem
            // frame before there is anything to send, and append an end-of-over
            // frame afterwards), so key through the mode as PTT does.
            if let Some(d) = self.digi.as_mut() {
                d.set_tx_active(true);
            }
            self.emit_digi_status();
        } else {
            self.state.tx.ptt = true;
            self.sync_tx_state();
            // The transmit rails (band limits, device capability) may have
            // refused; don't leave a message playing into nothing.
            if !self.tx_active {
                self.voice_tx = false;
                self.voice.stop_play();
            }
        }
        let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
        self.emit_voice_status();
    }

    /// Stop a message and end the over (the operator pressed stop, or an
    /// external control asked us to).
    fn stop_voice_play(&mut self) {
        if !self.voice_tx && !self.voice.is_playing() {
            return;
        }
        self.voice.stop_play();
        self.release_voice_tx();
        self.voice_tx = false;
        self.mic_fifo.clear();
        self.voice_tick = None;
        self.emit_voice_status();
    }

    /// Unkey a keyer over. RADE closes its own over (end-of-over frame, then
    /// the burst finishes on its own, announcing the state change itself);
    /// everything else drops PTT here.
    fn release_voice_tx(&mut self) {
        if self.state.rx[0].mode.is_rade() {
            if let Some(d) = self.digi.as_mut() {
                d.set_tx_active(false);
            }
        } else if self.state.tx.ptt {
            self.state.tx.ptt = false;
            self.sync_tx_state();
            let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
        }
    }

    /// Drop any keyer playback without touching the key state — the caller is
    /// about to set it (operator PTT/TUNE, or an abort).
    fn cancel_voice_play(&mut self) {
        if !self.voice_tx && !self.voice.is_playing() {
            return;
        }
        self.voice.stop_play();
        self.voice_tx = false;
        self.mic_fifo.clear();
        self.voice_tick = None;
        self.emit_voice_status();
    }

    /// The operator's transmit-audio level for the over that is on the air, on a
    /// radio that modulates what we send it.
    ///
    /// This mode's own level if the operator has set one, else the level for
    /// the carrier it goes out on — see
    /// [`sdroxide_types::DigiConfig::tx_audio_levels`] for why an absent entry
    /// inherits, and [`sdroxide_types::DigiConfig::tx_audio_level_fm`] for what
    /// the two carrier defaults mean. On FM it is the deviation; on sideband it
    /// is drive into the modulator, and the thing that keeps a constant-envelope
    /// mode out of the rig's ALC.
    ///
    /// Keyed on the mode on the air rather than on whichever panel set it, so a
    /// deviation set for 1200 baud never lands on FT8 and an FT8 level never
    /// lands on RTTY.
    fn digi_tx_audio_level(&self) -> f32 {
        self.digi_config.tx_level_for(self.state.rx[0].mode)
    }

    /// One block of transmit audio from the digital mode, at the rate the radio
    /// consumes and with the modem's own headroom already divided out.
    ///
    /// Returns true when the over is finished. Where a rate matcher is in play
    /// that means the modem has finished *and* its resampled audio has drained:
    /// unkeying on the modem alone would cut off whatever is still queued, and
    /// the tail of a packet frame is its check sequence.
    fn fill_digi_tx_block(&mut self, out: &mut [f32]) -> bool {
        let gain = self.digi.as_ref().map_or(1.0, |d| digi_tx_gain(d.tx_peak()));
        let done = match self.digi_tx_rs.take() {
            // The usual case: the modem already speaks the radio's rate.
            None => self.digi.as_mut().is_none_or(|d| d.fill_tx_block(out)),
            Some(mut rs) => {
                // Pull whole modem blocks, resample into a queue, and hand out
                // exactly what the radio asked for.
                while self.digi_tx_fifo.len() < out.len() && !self.digi_tx_done {
                    let mut src = vec![0.0f32; out.len()];
                    let finished = self.digi.as_mut().is_none_or(|d| d.fill_tx_block(&mut src));
                    self.digi_tx_scratch.clear();
                    rs.push(&src, &mut self.digi_tx_scratch);
                    self.digi_tx_fifo.extend_from_slice(&self.digi_tx_scratch);
                    self.digi_tx_done = finished;
                }
                let take = self.digi_tx_fifo.len().min(out.len());
                out[..take].copy_from_slice(&self.digi_tx_fifo[..take]);
                out[take..].fill(0.0);
                self.digi_tx_fifo.drain(..take);
                let done = self.digi_tx_done && self.digi_tx_fifo.is_empty();
                self.digi_tx_rs = Some(rs);
                done
            }
        };
        // The modem's own headroom, divided back out (`DigiEngine::tx_peak`) so
        // the radio is handed a full-scale modulating signal and a full Drive is
        // a full transmitter (issue #131).
        //
        // ...and then the operator's own level, but only where the *radio*
        // modulates what we send it. Where we modulate it ourselves the
        // modulator and Drive already own the level, so this stays out of it.
        let level =
            if self.audio_mode || self.caps.tx_audio { self.digi_tx_audio_level() } else { 1.0 };
        let gain = gain * level;
        if (gain - 1.0).abs() > f32::EPSILON {
            for a in out.iter_mut() {
                *a = (*a * gain).clamp(-1.0, 1.0);
            }
        }
        done
    }

    /// Top up the 48 kHz TX FIFO from whichever source owns this over — the
    /// voice keyer, a TCI client's audio stream, or the local microphone — and
    /// bound the queue.
    ///
    /// Returns `false` while a TCI over is still building its cushion: network
    /// audio arrives in bursts, so transmitting the first block that turns up
    /// would underrun a moment later.
    fn fill_tx_audio_fifo(&mut self) -> bool {
        self.fill_tx_audio_fifo_depth(TX_AUDIO_BLOCK * 2)
    }

    /// [`Self::fill_tx_audio_fifo`], with the depth the voice keyer queues ahead.
    ///
    /// The modulator paths take one block per call and want a block of slack;
    /// the digital-voice path hands the *whole* FIFO to the codec each block, so
    /// it asks for exactly one — queueing more there would feed the vocoder
    /// faster than real time.
    fn fill_tx_audio_fifo_depth(&mut self, voice_depth: usize) -> bool {
        // A recorded message owns this over: the real microphone is drained and
        // discarded so it cannot leak in alongside, and the stored audio is
        // metered out a block at a time. Once the message has been read out the
        // FIFO is left to drain, which is what tells `poll_voice` the over can
        // end without chopping the tail.
        if self.voice_tx {
            if let Some(mic) = self.mic.as_mut() {
                while mic.consumer.pop().is_ok() {}
            }
            while !self.voice.play_finished() && self.mic_fifo.len() < voice_depth {
                let mut block = [0.0f32; TX_AUDIO_BLOCK];
                self.voice.fill_tx_block(&mut block);
                self.mic_fifo.extend_from_slice(&block);
            }
            return true;
        }
        if !self.tci_tx {
            if let Some(mic) = self.mic.as_mut() {
                let mut raw = Vec::with_capacity(mic.consumer.slots());
                while let Ok(s) = mic.consumer.pop() {
                    raw.push(s);
                }
                match &mut self.mic_resampler {
                    Some(r) => r.push(&raw, &mut self.mic_fifo),
                    None => self.mic_fifo.extend_from_slice(&raw),
                }
            }
            // Latency bound: keep at most 100 ms queued.
            if self.mic_fifo.len() > 4_800 {
                let cut = self.mic_fifo.len() - 4_800;
                self.mic_fifo.drain(..cut);
            }
            return true;
        }

        // A TCI client owns this over. Drain and discard the real microphone so
        // it can't leak in alongside.
        if let Some(mic) = self.mic.as_mut() {
            while mic.consumer.pop().is_ok() {}
        }
        // Playing out the tail of an over the client has already ended: it is
        // not sending any more and must not be asked to.
        if self.tci_tx_drain {
            return true;
        }
        // `tx_pace` is unset until the first block goes out, which marks the
        // pre-roll: this function is called from a loop that spins far faster
        // than the transmit clock until then, and counting those calls as
        // consumption would run the demand away before a frame had been sent.
        let playing = self.tx_pace.is_some();
        if let Some(srv) = self.tci_srv.as_mut() {
            let mut block = [0.0f32; TX_AUDIO_BLOCK];
            let mut got = 0usize;
            loop {
                let n = srv.read_tx_audio(&mut block);
                self.mic_fifo.extend_from_slice(&block[..n]);
                got += n;
                if n < TX_AUDIO_BLOCK {
                    break;
                }
            }
            // Closed-loop pacing against the transmit clock: ask for everything
            // this over will have consumed plus a fixed lead, less what has
            // already been asked for. A client that honours chronos then tracks
            // real time whatever cadence it answers on; one that self-paces (as
            // sdroxide's own client does when a rig never chronos) fills the
            // queue past the lead and is not asked at all. See `TciTxPace`.
            let asked = self.tci_pace.request(self.mic_fifo.len(), got, playing);
            if let Some(frames) = asked {
                srv.request_chrono(frames);
            }
            // The whole of the pacing loop in one line, once a block. This is
            // the only view there is of a third-party client's cadence — see
            // issue #202, where the numbers behind two different failures
            // (a chopped over and a silent one) look identical from outside.
            trace!(
                target: "sdroxide::tcitx",
                got,
                queued = self.mic_fifo.len(),
                asked = asked.unwrap_or(0),
                requested = self.tci_pace.asked,
                supplied = self.tci_pace.supplied,
                played = self.tci_pace.played,
                "TCI TX pacing"
            );
        }
        if self.mic_fifo.len() > TCI_TX_FIFO_CAP {
            let cut = self.mic_fifo.len() - TCI_TX_FIFO_CAP;
            self.mic_fifo.drain(..cut);
        }
        // `tx_pace` is unset until the first block goes out, marking the pre-roll.
        if self.tx_pace.is_none() {
            return self.mic_fifo.len() >= TX_AUDIO_BLOCK * 3;
        }
        // Short of a full block is an underrun: pad with silence rather than
        // chop the over, but count it so a dead client is eventually unkeyed.
        if self.mic_fifo.len() < TX_AUDIO_BLOCK {
            self.tci_tx_starved += 1;
        } else {
            self.tci_tx_starved = 0;
        }
        true
    }

    /// Route the microphone for a digital-mode transmission.
    ///
    /// Synthesised-burst modes (FT8, SSTV, the keyboard modems) don't want it,
    /// and it is drained and discarded so it can't back up or leak into the
    /// burst. Digital *voice* is the exception: the mic is the payload, so it
    /// is resampled to 48 kHz and handed to the mode.
    fn feed_digi_mic(&mut self) {
        if self.digi.as_ref().is_some_and(|d| d.wants_mic()) {
            self.fill_tx_audio_fifo_depth(TX_AUDIO_BLOCK);
            if !self.mic_fifo.is_empty() {
                // Mic gain, on the one path where the microphone is the
                // payload. It was missing here: both places that applied it are
                // voice branches this never reaches, so the Mic slider did
                // nothing at all to a digital-voice over and the vocoder was
                // fed whatever the sound card delivered. Same 50 %-is-unity
                // convention as the voice paths, so one setting means one thing
                // whichever mode is transmitting.
                let gain = self.state.tx.mic_gain * 2.0;
                if (gain - 1.0).abs() > f32::EPSILON {
                    for a in self.mic_fifo.iter_mut() {
                        *a = (*a * gain).clamp(-1.0, 1.0);
                    }
                }
                if let Some(d) = self.digi.as_mut() {
                    d.on_tx_mic(&self.mic_fifo);
                }
                self.mic_fifo.clear();
            }
            return;
        }
        if let Some(mic) = self.mic.as_mut() {
            while mic.consumer.pop().is_ok() {}
        }
    }

    /// One ~10 ms transmit block: mic → modulator → drive → DUC → device.
    fn tx_block(&mut self) -> crate::Result<()> {
        // A CAT/TCI rig modulates itself; we just route raw 48 kHz TX audio to
        // it (`tx_write_audio`) instead of building modulated IQ.
        if self.audio_mode || self.caps.tx_audio {
            return self.tx_block_audio();
        }
        // Digital-mode burst: the FT8/FT4 controller supplies the audio; the
        // real mic is drained and discarded so it can't leak into the burst.
        if self.digi_tx {
            return self.tx_block_digi();
        }

        let fm = self.state.rx[0].mode == Mode::Nfm;
        // A 1750 Hz burst owns the whole block: it replaces the microphone
        // anyway, and one fired from receive may have no microphone behind it
        // at all — so it runs ahead of the wait for mic audio, which would
        // otherwise stall it forever.
        //
        // Only while the mode is still the FM the burst was armed in. A mode
        // changed mid-burst leaves the burst unplayed, and without this that
        // unplayed burst would go on claiming every block — a microphone that
        // had gone dead for the rest of the over.
        let bursting = fm && self.burst.is_some();
        // Fill the 48 kHz FIFO from whoever owns this over (mic or TCI client).
        if !bursting && !self.fill_tx_audio_fifo() {
            std::thread::sleep(Duration::from_millis(2));
            return Ok(());
        }
        let tci_tx = self.tci_tx;
        // Read before the chain is borrowed: it asks the *source* what ceiling
        // the converter in front of the radio imposes.
        let drive = self.tx_drive();
        let tune_level = self.tx_tune_level();
        let cessb_db = self.state.tx.cessb_db;
        let Some(tx) = self.tx.as_mut() else { return Ok(()) };

        tx.mod_buf.clear();
        let mut burst_done = false;
        if self.state.tx.tune || tx.modulator.is_none() {
            // Steady carrier at the tune level (also CW until the keyer exists).
            let level = tune_level;
            tx.mod_buf.resize(TX_AUDIO_BLOCK, Complex32::new(level, 0.0));
            self.mic_fifo.clear();
            // The recording tap has no other source of TX audio during tune —
            // without this the tap goes quiet for the tune's duration and
            // drifts out of sync with RX, worse with every tune. Same audible
            // tone `tx_block_audio`'s tune branch already generates.
            if let Some(mixer) = self.mixer.as_mut() {
                let mut tone = [0.0f32; TX_AUDIO_BLOCK];
                let inc = std::f32::consts::TAU * 1000.0 / TX_MONITOR_RATE as f32;
                for a in &mut tone {
                    *a = self.tune_phase.cos() * level;
                    self.tune_phase += inc;
                    if self.tune_phase > std::f32::consts::TAU {
                        self.tune_phase -= std::f32::consts::TAU;
                    }
                }
                mixer.push_tx(&tone);
            }
        } else {
            let mut audio = [0.0f32; TX_AUDIO_BLOCK];
            if bursting {
                // The microphone is muted for the length of the burst; drop
                // what arrived rather than letting it queue up behind the tone
                // and go out a moment late.
                self.mic_fifo.clear();
            } else {
                let take = self.mic_fifo.len().min(TX_AUDIO_BLOCK);
                audio[..take].copy_from_slice(&self.mic_fifo[..take]);
                self.mic_fifo.drain(..take);
                // What the microphone actually delivered this over, before any
                // gain: a transmitter making milliwatts of voice while TUNE
                // makes full power is a silent microphone every time, and
                // nothing on screen said so (issue #215).
                self.voice_peak =
                    self.voice_peak.max(audio[..take].iter().fold(0.0f32, |m, s| m.max(s.abs())));
                self.voice_blocks += 1;

                // Mic gain is the operator's microphone control; a TCI client sets
                // its own level and uses `drive` for power, so it is left alone.
                let mic_gain = if tci_tx { 1.0 } else { self.state.tx.mic_gain * 2.0 };
                for a in &mut audio {
                    *a = tx.dc.run(*a) * mic_gain;
                }
                // Voice-only parametric EQ, ahead of the modulator.
                apply_tx_eq(&mut self.tx_eq, &mut self.tx_eq_cfg, &self.state.tx.eq, &mut audio);
            }
            // The repeater's own signalling, last of all: the tone has to reach
            // the modulator at the level it was built at, and the EQ above
            // exists to shape a voice rather than a 100 Hz sine.
            if fm {
                burst_done = apply_tx_signalling(&mut self.sub_tone, &mut self.burst, &mut audio);
            }
            if let Some(mixer) = self.mixer.as_mut() {
                mixer.push_tx(&audio);
            }
            let modulator = tx.modulator.as_mut().expect("checked above");
            modulator.process(&audio, &mut tx.mod_buf);
            // Controlled-envelope SSB, between the modulator and the drive
            // control. Here because it works on the *envelope* — the thing the
            // amplifier runs out of — which does not exist until the sideband
            // has been made, and because what it hands on is already held at
            // full scale, so the operator's drive setting still means what it
            // said. Only on the two voice sidebands, and only when they asked
            // for it: `cessb` is `None` on every other mode.
            if let Some(c) = tx.cessb.as_mut() {
                c.set_compression_db(cessb_db);
                if c.active() {
                    c.process(&mut tx.mod_buf);
                }
            }
            for z in &mut tx.mod_buf {
                *z *= drive;
                // Hard limiter: digital full scale is the ceiling.
                let mag = z.norm();
                if mag > 1.0 {
                    *z /= mag;
                }
            }
        }

        let peak = tx.mod_buf.iter().fold(0.0f32, |a, z| a.max(z.norm()));
        tx.alc_peak = peak.max(tx.alc_peak * 0.85);

        // TX monitor: the 48 kHz analytic modulator output is exactly the signal
        // going on the air (one sideband, at the audio offset from the dial) —
        // used for the narrow digital-mode scope.
        self.tx_analyzer.process(&tx.mod_buf);

        tx.tx_buf.clear();
        tx.duc.process(&tx.mod_buf, &mut tx.tx_buf);
        // Satellite lock: shift the over by the TX Doppler correction (plus
        // any transponder drift since key-down). Before the analyzer tap, so
        // the wideband display shows the signal where it actually goes out.
        if let Some(nco) = tx.sat_nco.as_mut() {
            nco.mix_in_place(&mut tx.tx_buf);
        }
        if !tx.tx_buf.is_empty() {
            self.source.tx_write(&tx.tx_buf)?;
            // The upconverted IQ feeds the wideband display at its RF position.
            self.analyzer.process(&tx.tx_buf);
        }
        // Keep the device/network TX ring near-empty (HPSDR ≈ 0.5 s, SoapySDR
        // varies) rather than letting a fast loop fill it and delay the signal.
        pace_tx_block(&mut self.tx_pace, self.source.tx_pace_cushion_ms());
        if burst_done {
            self.end_burst_over();
        }
        Ok(())
    }

    /// One TX block driven by the FT8/FT4 burst player: pull 10 ms of the
    /// synthesized burst, USB-modulate it (same SsbMod path as voice), and
    /// write it out. Unkeys and advances the QSO when the burst finishes.
    fn tx_block_digi(&mut self) -> crate::Result<()> {
        self.feed_digi_mic();
        if self.tx.is_none() {
            return Ok(());
        }
        // The headroom is divided out inside `fill_digi_tx_block`, before the
        // modulator rather than on the modulated samples below: on a sideband
        // mode the two are the same thing, but on FM they are not — there the
        // level is the deviation, and scaling the modulator's constant envelope
        // would raise the power while leaving a packet over half as wide as the
        // channel expects it to be.
        //
        // Ahead of borrowing the transmit chain, because filling the block
        // needs the engine itself.
        let mut audio = [0.0f32; TX_AUDIO_BLOCK];
        let done = self.fill_digi_tx_block(&mut audio);
        if let Some(mixer) = self.mixer.as_mut() {
            mixer.push_tx(&audio);
        }
        // Read before the chain is borrowed, as above.
        let drive = self.tx_drive();
        let Some(tx) = self.tx.as_mut() else { return Ok(()) };

        tx.mod_buf.clear();
        // Every mode that reaches here rides single sideband, so the chain has
        // a modulator — except CW, where the chain deliberately has none so
        // that a manual PTT keys a carrier rather than modulating the mic. The
        // keyer's sidetone does want one: put through the same USB path as
        // everything else it lands on the air at dial + pitch, which is exactly
        // where the waterfall cursor says it should.
        let modulator = match tx.modulator.as_mut() {
            Some(m) => m,
            None => {
                let (lo, hi) = Mode::Usb.default_filter();
                tx.modulator.insert(Box::new(sdroxide_dsp::SsbMod::new(48_000.0, lo, hi)))
            }
        };
        modulator.process(&audio, &mut tx.mod_buf);
        for z in &mut tx.mod_buf {
            *z *= drive;
            let mag = z.norm();
            if mag > 1.0 {
                *z /= mag;
            }
        }
        let peak = tx.mod_buf.iter().fold(0.0f32, |a, z| a.max(z.norm()));
        tx.alc_peak = peak.max(tx.alc_peak * 0.85);

        self.tx_analyzer.process(&tx.mod_buf); // TX monitor (narrow digital scope)

        tx.tx_buf.clear();
        tx.duc.process(&tx.mod_buf, &mut tx.tx_buf);
        // Satellite lock: same TX Doppler shift as the voice path — an FT8
        // burst through a transponder needs it just as much.
        if let Some(nco) = tx.sat_nco.as_mut() {
            nco.mix_in_place(&mut tx.tx_buf);
        }
        if !tx.tx_buf.is_empty() {
            self.source.tx_write(&tx.tx_buf)?;
            self.analyzer.process(&tx.tx_buf); // wideband RF display
        }
        // Pace the burst to real time so it isn't raced into the device ring
        // (which would drop PTT early — the tail matters for FT8 decode).
        pace_tx_block(&mut self.tx_pace, self.source.tx_pace_cushion_ms());

        if done {
            // Burst finished: drain any queued audio, then unkey and let the QSO
            // machine advance.
            self.source.tx_drain();
            self.tx_pace = None;
            self.digi_tx = false;
            self.state.tx.ptt = false;
            self.sync_tx_state();
            if let Some(d) = self.digi.as_mut() {
                d.on_burst_done();
            }
            let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
            self.emit_digi_status();
        }
        Ok(())
    }

    /// One ~10 ms TX block for a CAT rig: gather 48 kHz mono audio (mic voice or
    /// an FT8/FT4 burst) and hand it to the rig's sound card — the radio does
    /// its own modulation. PTT is asserted separately by `sync_tx_state`.
    fn tx_block_audio(&mut self) -> crate::Result<()> {
        let mut audio = [0.0f32; TX_AUDIO_BLOCK];
        // The digital mode's burst has played out, and the 1750 Hz burst has —
        // two different bursts, both of which end an over, so they are named
        // apart rather than sharing one flag.
        let mut digi_done = false;
        let mut burst_done = false;
        // Whether this block carries repeater signalling — FM only, and never
        // over a digital burst or a tune, both of which own their whole block.
        let mut signalling = false;

        if self.digi_tx {
            self.feed_digi_mic();
            // Full scale into the rig, as the tune tone below goes out on a
            // radio with its own power control — and for the same reason. That
            // power control cannot make up 6 dB the audio never had: a
            // half-scale burst asks for a quarter of the power a TUNE at the
            // same slider setting gets out of the same radio, and the Drive
            // slider looks dead, because what the output is riding on is the
            // audio and not the power register (issue #131). `tx_peak` is what
            // divides it back out, inside `fill_digi_tx_block`.
            digi_done = self.fill_digi_tx_block(&mut audio);
        } else if self.state.tx.tune {
            // An audio-modulated rig (CAT/TCI) needs a tone to produce a carrier;
            // silence would key up with no output. On a rig with its own power
            // control the tone is just the modulating signal and goes out at full
            // scale — `tx_power_level` already commanded the tune level, and
            // attenuating here as well would scale the carrier twice. Elsewhere
            // (a CAT rig's sound card) the tone amplitude is the only tune-level
            // control there is.
            let amp = if self.source.commands_tx_power() {
                1.0
            } else {
                self.state.tx.tune_drive.clamp(0.05, 1.0)
            };
            let inc = std::f32::consts::TAU * 1000.0 / TX_MONITOR_RATE as f32;
            for a in &mut audio {
                *a = self.tune_phase.cos() * amp;
                self.tune_phase += inc;
                if self.tune_phase > std::f32::consts::TAU {
                    self.tune_phase -= std::f32::consts::TAU;
                }
            }
        } else if self.burst.is_some() && self.state.rx[0].mode == Mode::Nfm {
            // A 1750 Hz burst owns the whole block. Ahead of the voice branch
            // because it replaces the microphone anyway, and because a burst
            // fired from receive may have no microphone behind it at all —
            // waiting for mic audio would stall it forever.
            self.mic_fifo.clear();
            signalling = true;
        } else {
            // Voice: mic (or a TCI client's stream) → 48 kHz FIFO → this block.
            if !self.fill_tx_audio_fifo() {
                std::thread::sleep(Duration::from_millis(2));
                return Ok(());
            }
            // On a real-time-paced network rig (TCI), build a small cushion before
            // the first block so the mic's bursty delivery can't underrun the
            // steady 48 kHz feed into choppy silence. `tx_pace` is unset until the
            // first block goes out, marking the pre-roll.
            // The voice keyer plays from memory and can never arrive late, so
            // it needs no cushion — and a message shorter than the cushion
            // would never satisfy this at all.
            if self.caps.tx_audio
                && !self.voice_tx
                && self.tx_pace.is_none()
                && self.mic_fifo.len() < TX_AUDIO_BLOCK * 2
            {
                std::thread::sleep(Duration::from_millis(2));
                return Ok(());
            }
            let take = self.mic_fifo.len().min(TX_AUDIO_BLOCK);
            audio[..take].copy_from_slice(&self.mic_fifo[..take]);
            self.mic_fifo.drain(..take);
            // See the modulating path's copy of this: an over with nothing on
            // the microphone is worth saying out loud (issue #215).
            self.voice_peak =
                self.voice_peak.max(audio[..take].iter().fold(0.0f32, |m, s| m.max(s.abs())));
            self.voice_blocks += 1;
            // A TCI client sets its own audio level; the mic-gain control is for
            // the operator's microphone and would double-scale it.
            let gain = if self.tci_tx { 1.0 } else { self.state.tx.mic_gain * 2.0 };
            for a in &mut audio {
                *a *= gain;
            }
            // Voice-only parametric EQ. The rig modulates this audio itself, so
            // handing it to the sound card is the same point in the chain that
            // handing it to the modulator is on the I/Q path — the last one we
            // own. Before the clamp, so a boosted band is held to full scale
            // rather than leaving here above it.
            apply_tx_eq(&mut self.tx_eq, &mut self.tx_eq_cfg, &self.state.tx.eq, &mut audio);
            for a in &mut audio {
                *a = a.clamp(-1.0, 1.0);
            }
            signalling = self.state.rx[0].mode == Mode::Nfm;
        }

        // The repeater's own signalling.
        //
        // ⚠️ A rig that modulates its own audio is the one place this may not
        // reach the air: a sub-audible tone at 67-250 Hz has to survive the
        // radio's microphone input, and most of those high-pass the audio to
        // keep exactly this sort of thing out of it. The 1750 Hz burst is
        // in-band and passes; a CTCSS tone may not, in which case the answer is
        // the rig's own encoder, set at the radio. Sent regardless, because
        // where it does pass — a data/line input, or a rig fed at baseband —
        // it works, and because the alternative is to decide on the operator's
        // behalf that their radio cannot do it.
        if signalling {
            burst_done = apply_tx_signalling(&mut self.sub_tone, &mut self.burst, &mut audio);
        }

        // TX monitor: the rig modulates its own audio, so approximate the on-air
        // spectrum by FFTing the outgoing audio (packed real; the display shows
        // just the transmit sideband).
        self.tx_mon_buf.clear();
        self.tx_mon_buf.extend(audio.iter().map(|&a| Complex32::new(a, 0.0)));
        self.tx_analyzer.process(&self.tx_mon_buf);

        if let Some(mixer) = self.mixer.as_mut() {
            mixer.push_tx(&audio);
        }

        self.source.tx_write_audio(&audio)?;

        // Wall-clock pace the audio feed to real time. Without this the loop
        // spins far faster than 48 kHz and floods the downstream buffer: an FT8
        // burst raced to the end and dropped PTT early (~5 s instead of 12.6 s),
        // a TCI voice over piled up >1 s of latency in the rig's TX ring while
        // starving the mic FIFO (choppy audio), and a CAT rig buffered its ~1 s
        // output ring before the sound card's own backpressure engaged (voice
        // delayed by ~1 s). Pacing keeps every backend's ring near-empty.
        pace_tx_block(&mut self.tx_pace, self.source.tx_pace_cushion_ms());

        if burst_done {
            self.end_burst_over();
        }

        if digi_done {
            // Let any queued audio play out before dropping PTT, so the rig
            // transmits the whole burst (FT8 needs every symbol).
            self.source.tx_drain();
            self.tx_pace = None;
            self.digi_tx = false;
            self.state.tx.ptt = false;
            self.sync_tx_state();
            if let Some(d) = self.digi.as_mut() {
                d.on_burst_done();
            }
            let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
            self.emit_digi_status();
        }
        Ok(())
    }

    /// The operator set the dial — the readout, a keypad entry, a memory, VFO
    /// A/B, an external controller. Put the front end where that dial is.
    ///
    /// On almost every radio that is [`Self::keep_vfo_in_span`]: the window is
    /// a resource worth keeping, the VFO tunes inside it with a DDC, and the
    /// hardware only moves when the span can no longer reach. On a rig that
    /// tunes *with* the dial — a transceiver whose I/Q output feeds a sound
    /// card, an Icom sending its 12 kHz IF — there is no such window to keep:
    /// its one synthesiser decides both where we listen and what we capture.
    /// Tuning the DDC away from it there would leave the radio's readout and
    /// ours showing different frequencies, with nothing to reconcile them until
    /// the next thing the rig reported snapped ours back to its — so the dial
    /// the operator asked for is commanded at the radio, and the window follows
    /// it.
    ///
    /// The panadapter's gestures ([`sdroxide_types::Command::TuneInSpan`])
    /// arrive here too. A click used to be exempt — the signal is already in
    /// the baseband, so only our receiver moved — until a field report showed
    /// the price of the exemption: the two readouts disagree, and an over the
    /// engine does not key itself (CW through the rig's own keyer, a mic keyed
    /// at the radio) transmits on the dial that was left behind.
    ///
    /// All of which needs a dial that answers. A rig sending I/Q down a sound
    /// card with no control cable on it does not have one, says so
    /// ([`Self::refresh_center_is_dial`]), and is tuned inside its span like
    /// any SDR — otherwise a click relabels the span around spectrum that is
    /// not arriving, which is a receiver that cannot change station at all
    /// (issue #155).
    fn follow_dial(&mut self) {
        if self.audio_mode || !self.source.center_is_dial() {
            self.keep_vfo_in_span();
            return;
        }
        let dial = self.state.active_freq_hz();
        // Not always the dial: in CW a radio that keys its own transmitter has
        // to sit on the contact, a sidetone above it (`rig_cw_offset_hz`).
        let vfo = dial + self.rig_cw_offset_hz();
        // Asking for a centre the front end is already on costs a skimmer
        // restart and a CAT write for nothing.
        if (self.state.center_hz - (vfo + self.lo_offset_hz())).abs() >= 0.5 {
            if self.retune_for_vfo(vfo) {
                // What a refused tune goes back to is a dial, not a VFO.
                self.good_vfo_hz = dial;
            }
        } else {
            self.good_vfo_hz = dial;
        }
    }

    /// Re-ask the front end whether its centre is the dial, and tell the UIs
    /// when the answer has changed.
    ///
    /// Almost every source answers this once and for ever — an SDR's LO is an
    /// LO whatever else happens. A transceiver on a sound card does not: its
    /// dial is ours to command only for as long as something answers on the
    /// control port, and a radio switched on (or a cable plugged in) after
    /// sdroxide started hands one back mid-session. `follow_dial` above is the
    /// whole reason it matters — a stale "yes" leaves every panadapter gesture
    /// commanding a dial nothing is listening to, and the operator watching the
    /// span relabel itself around spectrum the radio is not sending.
    fn refresh_center_is_dial(&mut self) {
        let is_dial = self.source.center_is_dial();
        if is_dial != self.caps.center_is_dial {
            self.caps.center_is_dial = is_dial;
            let _ = self.event_tx.send(RadioEvent::CapabilitiesUpdated(self.caps.clone()));
        }
    }

    /// Retune hardware center if the active VFO left the usable span — or, on a
    /// front end that has to keep clear of its own LO, came too close to it.
    fn keep_vfo_in_span(&mut self) {
        if self.audio_mode {
            return; // the dial is the VFO; update_tuning drives CAT directly
        }
        let span = self.state.sample_rate;
        let usable = span * 0.45; // keep VFO out of the outer 5% roll-off
        let vfo = self.state.active_freq_hz();
        let from_lo = (vfo - self.state.center_hz).abs();
        if from_lo > usable || from_lo < self.lo_guard_hz() {
            self.retune_for_vfo(vfo);
        } else {
            // In span on the centre the hardware is already on, so this dial is
            // one the front end is known to be able to receive.
            self.good_vfo_hz = vfo;
        }
    }

    /// How far above the VFO this front end's LO is parked, in the span the
    /// receiver actually sees.
    ///
    /// A zero-IF device asks for a quarter of *its* span, which is a quarter of
    /// the decimated one divided by the factor — the offset has to shrink with
    /// the span, or the LO would be parked outside the bandwidth the decimator
    /// keeps and the VFO would land on a piece of spectrum that is no longer
    /// there. Scaling it keeps the same fraction of the visible span, so a
    /// decimated receiver sits exactly as far off DC, relatively, as an
    /// undecimated one.
    fn lo_offset_hz(&self) -> f64 {
        self.source.lo_offset_hz() / self.state.decimation as f64
    }

    /// How far the active VFO has to stay from the hardware LO.
    ///
    /// Zero on a front end whose LO is clean (`lo_offset_hz` == 0), so its
    /// tuning behaviour is untouched. Otherwise 1.2× the DDC channel's
    /// half-width, which is the whole point of the offset: keep DC outside the
    /// channel the demodulator actually sees, with a margin. Capped below the
    /// offset itself, because a guard a retune could not satisfy would make
    /// [`Self::keep_vfo_in_span`] retune on every single call.
    fn lo_guard_hz(&self) -> f64 {
        let offset = self.lo_offset_hz();
        if offset <= 0.0 {
            return 0.0;
        }
        let channel = self.main.as_ref().map(|c| c.channel_rate()).unwrap_or(48_000.0);
        (channel * 0.6).min(offset * 0.8)
    }

    /// Put the hardware where this VFO wants it: on the VFO for a front end with
    /// a clean LO, [`IqSource::lo_offset_hz`] away from it for one without.
    /// Reports whether the front end took it.
    ///
    /// The offset is normally *above* the VFO, which is where band activity
    /// sits. At the top of a tuning range that is the one place the LO cannot
    /// go, so the mirror position is tried next, and tuning the LO straight to
    /// the VFO last: a DC spike inside the passband is a poorer receiver, but a
    /// receiver, which "outside the tuning range" is not.
    fn retune_for_vfo(&mut self, vfo_hz: f64) -> bool {
        let offset = self.lo_offset_hz();
        let center = [vfo_hz + offset, vfo_hz - offset, vfo_hz]
            .into_iter()
            .find(|&c| self.can_tune(c))
            // Nothing is reachable: ask for the natural place anyway, so the
            // refusal below reports the range rather than inventing a reason.
            .unwrap_or(vfo_hz + offset);
        if !self.retune_named(center, vfo_hz) {
            return false;
        }
        self.good_vfo_hz = vfo_hz;
        true
    }

    /// Whether the front end says it can put its LO here. A front end that
    /// publishes no ranges at all is taken at its word and asked directly.
    fn can_tune(&self, center_hz: f64) -> bool {
        self.caps.may_rx_hz(center_hz)
    }

    /// Move the hardware centre, and report whether the front end took it.
    ///
    /// A front end that publishes its tuning range is never asked for anything
    /// outside it. That is not politeness towards the driver: a driver can fail
    /// a tune *part-way* and leave the hardware unusable. A LimeSDR asked for a
    /// frequency below the LMS7002M's range answers
    /// "SoapyLMS7::setFrequency() failed", having already torn down its
    /// interface clock, and receives nothing further until it is set up again —
    /// so the cheapest cure is not to make the call. A request that gets past
    /// this and fails anyway is a hardware fault rather than an operator error:
    /// the source restarts itself on the last frequency that worked (and asks
    /// to be reopened if it cannot), and the dial goes back to match.
    fn retune(&mut self, center_hz: f64) -> bool {
        self.retune_named(center_hz, center_hz)
    }

    /// [`Self::retune`], naming `dial_hz` if the tune is refused: on a front end
    /// that parks its LO clear of the VFO the two differ, and the operator
    /// asked for the dial, not for the LO behind it.
    fn retune_named(&mut self, center_hz: f64, dial_hz: f64) -> bool {
        if !self.can_tune(center_hz) {
            self.tune_refused(format!(
                "{:.6} MHz is outside this radio's receive range ({})",
                dial_hz / 1e6,
                describe_ranges(&self.caps.freq_ranges_rx)
            ));
            return false;
        }
        match self.source.set_center_hz(center_hz) {
            Ok(()) => {
                self.state.center_hz = center_hz;
                self.note_center_change(center_hz);
                // The LNA table an RSP offers belongs to the band, and the band
                // has just moved.
                self.refresh_rx_gains();
                // Re-place the skim window inside the span that has just moved;
                // one that really moves re-labels its spots and clears its
                // tracks, so none straddles the old and new axis.
                self.sync_skim_window();
                self.sync_ism_window();
                self.sync_adsb_window();
                self.sync_vdl2_window();
                self.sync_ais_window();
                self.sync_qo100_window();
                true
            }
            Err(e) => {
                self.tune_refused(format!("the radio refused to tune: {e}"));
                false
            }
        }
    }

    /// A tune the front end would not or did not take: put the dial back on the
    /// last frequency it accepted and say why.
    ///
    /// The centre is untouched by a refused tune, so the dial that went with it
    /// is still one this front end can receive — going back there leaves a
    /// working radio rather than a receiver pointed somewhere it cannot hear.
    /// This is a notice and not a [`RadioEvent::ConnectionLost`]: the session is
    /// intact, and a fatal-looking error would leave every attached UI showing
    /// a dead radio that is in fact still streaming.
    fn tune_refused(&mut self, why: String) {
        let good = self.good_vfo_hz;
        warn!("{why}; dial back to {:.6} MHz", good / 1e6);
        match self.state.active_vfo {
            Vfo::A => self.state.vfo_a_hz = good,
            Vfo::B => self.state.vfo_b_hz = good,
        }
        self.state.band = Band::containing(good);
        self.update_tuning();
        let _ = self
            .event_tx
            .send(RadioEvent::Notice(Some(format!("{why} — back to {:.6} MHz", good / 1e6))));
        let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
    }
}

/// The frequency inside a device's published receive ranges nearest `hz`, or
/// `None` when there is nothing to do: `hz` is already reachable, or the device
/// publishes no ranges at all and is taken at its word (see
/// [`sdroxide_types::DeviceCaps::may_rx_hz`]).
///
/// The nearest edge rather than a band centre or a default: it is the closest
/// this radio can come to where the operator was, and on the common cause — a
/// converter switched on or off under a running radio — it is a short step from
/// where they wanted to be.
fn nearest_rx_hz(caps: &DeviceCaps, hz: f64) -> Option<f64> {
    if caps.may_rx_hz(hz) {
        return None;
    }
    caps.freq_ranges_rx
        .iter()
        .map(|&(lo, hi)| hz.clamp(lo, hi))
        .min_by(|a, b| (a - hz).abs().total_cmp(&(b - hz).abs()))
}

/// Tunable ranges as an operator would read them.
fn describe_ranges(ranges: &[(f64, f64)]) -> String {
    ranges
        .iter()
        .map(|&(lo, hi)| format!("{:.3}–{:.3} MHz", lo / 1e6, hi / 1e6))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The underlying rig mode class a `Mode` commands over CAT/TCI (USB/LSB/CW/
/// AM/FM). Digital/data modes ride on a sideband, so a rig reporting that plain
/// sideband must not be mistaken for the operator leaving the digital mode.
fn rig_mode_class(m: Mode) -> u8 {
    match m {
        Mode::Lsb | Mode::Digl => 0,
        Mode::Usb
        | Mode::Digu
        | Mode::Ft8
        | Mode::Ft4
        | Mode::Ft2
        | Mode::Js8
        | Mode::Wspr
        | Mode::Psk
        | Mode::Rtty
        | Mode::Sstv
        | Mode::Wefax
        | Mode::Navtex
        | Mode::Olivia
        | Mode::Thor
        | Mode::Fsq
        | Mode::Hell
        | Mode::RfPaint
        | Mode::Rade
        | Mode::PacketHf
        | Mode::Spec => 1,
        // DRM sits on the dial in a channel about as wide as AM's, and a
        // rig has no DRM setting to report back — see `to_hamlib_mode`.
        Mode::Am | Mode::Sam | Mode::Dsb | Mode::Isb | Mode::Drm => 2,
        Mode::Cw => 3,
        // RIFP, VHF packet, APRS, VHF SSTV and VHF RTTY are data on an FM
        // carrier, so a rig reporting plain FM is still where we left it.
        Mode::Nfm
        | Mode::Wfm
        | Mode::Rifp
        | Mode::Packet
        | Mode::Aprs
        | Mode::SstvFm
        | Mode::RttyFm
        // ADS-B is not a mode any rig has, and no rig will ever be in it: the
        // dial is at 1090 MHz. Grouped with FM so an echo is never read as the
        // operator having left the mode.
        | Mode::Adsb
        | Mode::Vdl2
        | Mode::Ais => 5,
    }
}

/// The rig-mode class an echo should be compared against: CW keyed as audio
/// (MCW) is commanded to the rig as the digital modes' sideband, so the class
/// expected back is the sideband's, not CW's.
///
/// At `dial_hz` because analog SSTV's is the one sideband that follows the
/// band, and on a radio that resolves that itself ([`IqSource::
/// resolves_sstv_sideband`]) the mode arrives here still called SSTV. Reading
/// it off the mode alone would expect USB back on 40 m for ever, and answer
/// every one of the rig's LSB reports by commanding the mode again.
fn expected_rig_class(commanded: Mode, cw_mcw: bool, dial_hz: f64) -> u8 {
    let m = if cw_mcw { Mode::Digu } else { commanded };
    if m.is_lower_sideband_at(dial_hz) { rig_mode_class(Mode::Lsb) } else { rig_mode_class(m) }
}

/// Encode an interleaved-RGB image (`w*h*3` bytes) to PNG.
fn encode_png(rgb: &[u8], w: u16, h: u16) -> Option<Vec<u8>> {
    let img = image::RgbImage::from_raw(w as u32, h as u32, rgb.to_vec())?;
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img).write_to(&mut buf, image::ImageFormat::Png).ok()?;
    Some(buf.into_inner())
}

/// Decode PNG bytes to interleaved RGB plus dimensions.
fn decode_png_rgb(png: &[u8]) -> Option<(Vec<u8>, u16, u16)> {
    let img = image::load_from_memory(png).ok()?.to_rgb8();
    let (w, h) = (img.width() as u16, img.height() as u16);
    Some((img.into_raw(), w, h))
}

/// Persist a received SSTV image (PNG) under the config `sstv_rx` directory,
/// returning the name it was filed under.
fn save_sstv_rx(png: &[u8]) -> Option<String> {
    save_image_rx("sstv", png)
}

/// Persist a received picture under the store its mode keeps.
///
/// `kind` is both the directory (`<kind>_rx`) and the file-name prefix, so a
/// weather chart and an SSTV picture never land in the same gallery — they are
/// browsed for completely different reasons and a fifteen-minute chart would
/// bury a session's SSTV.
///
/// The name is returned rather than kept quiet: it is what a gallery lists the
/// picture under and the key every later fetch names it by.
/// Open the Winlink mailbox under the config directory.
///
/// Returns `None` rather than failing the engine: a mailbox that cannot be
/// opened is a reason for the mail window to complain, not for the radio to
/// refuse to start.
/// Pull the decoder specs out of the operator's `rtl433_flex.conf`.
///
/// Problems are logged here rather than raised: a spec that does not parse costs
/// its own decoder and nothing else, and the ISM window shows the same list
/// again with the line numbers, which is where somebody editing the file will be
/// looking.
#[cfg(feature = "rtl433")]
fn ism_flex_parse(text: &str) -> (Vec<String>, Vec<String>) {
    let (specs, problems) = sdroxide_ism::rtl433::flex::parse_conf(text);
    let mut said = Vec::new();
    for p in &problems {
        warn!("rtl433_flex.conf line {}: {}", p.line, p.message);
        said.push(format!("line {}: {}", p.line, p.message));
    }
    (specs.into_iter().map(|s| s.spec).collect(), said)
}

#[cfg(not(feature = "rtl433"))]
fn ism_flex_parse(_text: &str) -> (Vec<String>, Vec<String>) {
    (Vec::new(), Vec::new())
}

fn open_mailbox(cfg: &sdroxide_types::WinlinkConfig) -> Option<sdroxide_winlink::WinlinkManager> {
    let dir = match sdroxide_config::config_dir() {
        Ok(d) => d.join("winlink"),
        Err(e) => {
            warn!("winlink mailbox: {e}");
            return None;
        }
    };
    match sdroxide_winlink::WinlinkManager::new(dir, cfg.clone()) {
        Ok(m) => Some(m),
        Err(e) => {
            warn!("opening the winlink mailbox: {e}");
            None
        }
    }
}

fn save_image_rx(kind: &str, png: &[u8]) -> Option<String> {
    let dir = match sdroxide_config::image_rx_dir(kind) {
        Ok(d) => d,
        Err(e) => {
            warn!("{kind}_rx dir: {e}");
            return None;
        }
    };
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let name = format!("{kind}-{ts}.png");
    let path = dir.join(&name);
    match std::fs::write(&path, png) {
        Ok(()) => Some(name),
        Err(e) => {
            warn!("saving {kind} image {}: {e}", path.display());
            None
        }
    }
}

/// Persist a received weather chart under the pictures directory, named for
/// when it was received and the dial it came in on.
///
/// Its own store and its own naming rather than `save_image_rx`'s: charts live
/// where the operator's other pictures live, and the name is the only thing
/// that will ever say which of a station's dozen daily products this one is.
/// The name is built by `sdroxide-types` so that the panel — which has to label
/// charts it reads back off disk — reads exactly what is written here.
fn save_wefax_rx(png: &[u8], dial_hz: f64) -> Option<String> {
    let dir = match sdroxide_config::wefax_rx_dir() {
        Ok(d) => d,
        Err(e) => {
            warn!("wefax chart dir: {e}");
            return None;
        }
    };
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let meta = sdroxide_types::WefaxChartMeta {
        unix,
        // A dial of zero means nothing is tuned, which is not a frequency worth
        // recording — better an unlabelled chart than a mislabelled one.
        dial_hz: (dial_hz > 0.0).then_some(dial_hz),
    };
    let name = meta.file_name();
    let path = dir.join(&name);
    match std::fs::write(&path, png) {
        Ok(()) => Some(name),
        Err(e) => {
            warn!("saving wefax chart {}: {e}", path.display());
            None
        }
    }
}

/// Encode a single-channel raster as a grayscale PNG.
fn encode_png_gray(gray: &[u8], w: u16, h: u16) -> Option<Vec<u8>> {
    let img = image::GrayImage::from_raw(w as u32, h as u32, gray.to_vec())?;
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageLuma8(img).write_to(&mut buf, image::ImageFormat::Png).ok()?;
    Some(buf.into_inner())
}

/// Replace the fields of an incoming [`DigiConfig`] that a client's copy cannot
/// be trusted to carry, leaving every genuine setting as sent.
///
/// The property they share is not who *owns* the setting — it is that each has a
/// write route of its own, outside `SetDigiConfig`. A panel seeds its editable
/// copy from the first status and owns it from then on, so anything written by
/// another route is missing from every copy seeded before that write. An
/// incoming config is therefore always a stale snapshot of these two fields, and
/// taking it discards everything learned since that client started.
///
/// - `tx_audio_hz`, the per-band transmit offsets, written by
///   `Command::SetDigiAudioFreq`.
/// - `tx_audio_levels`, the per-mode transmit-audio levels, written by
///   `Command::SetDigiTxLevel`.
/// - `contest_serial`, the next contest exchange's serial number, advanced by
///   the engine itself as each contact is logged and set by hand through
///   `Command::SetContestSerial`. The staleness here is not hypothetical: a
///   panel seeded at the start of a contest holds serial 1 for the whole
///   weekend, and any setting applied from it would send the count back there.
///
/// Found the hard way, minutes after the offsets went in. 60 m's was set,
/// recorded and saved; ticking Hold TX a moment later sent a copy seeded before
/// it existed, and the empty map went over the good file. Nothing reported a
/// fault, because the write succeeded. A merge here cannot be defeated by a
/// client that means no harm, where asking clients to send the map back
/// faithfully could be defeated by any of them.
fn keep_engine_owned(mut incoming: DigiConfig, current: &DigiConfig) -> DigiConfig {
    incoming.tx_audio_hz = current.tx_audio_hz.clone();
    incoming.tx_audio_levels = current.tx_audio_levels.clone();
    incoming.contest_serial = current.contest_serial;
    incoming
}

#[cfg(test)]
mod rig_mode_class_tests {
    use super::*;

    /// Issue #119: CW keyed as audio (MCW) is commanded to the rig as the digi
    /// sideband, so the rig echoing USB (or DATA-U) back is it doing exactly
    /// as told — not the operator leaving CW — and must be same-class.
    #[test]
    fn a_rig_on_the_sideband_is_where_cw_as_mcw_left_it() {
        let expected = expected_rig_class(Mode::Cw, true, 7_030_000.0);
        assert_eq!(expected, rig_mode_class(Mode::Usb));
        assert_eq!(expected, rig_mode_class(Mode::Digu));
        assert_ne!(expected, rig_mode_class(Mode::Cw));
        // And drifting onto the other sideband is still a class change, so the
        // handler commands the rig straight back.
        assert_ne!(expected, rig_mode_class(Mode::Lsb));
    }

    /// With the rig's own keyer sending, the rig really is put in CW and its
    /// echo is compared against CW, exactly as before.
    #[test]
    fn cw_from_the_rig_keyer_still_expects_cw_back() {
        assert_eq!(expected_rig_class(Mode::Cw, false, 7_030_000.0), rig_mode_class(Mode::Cw));
    }

    /// Analog SSTV reaches a radio that works its own sideband out still called
    /// SSTV (issue #313), so what is expected back has to come from the dial:
    /// on 40 m the rig will report LSB, and reading the mode alone would call
    /// that a class change for ever and re-command the mode on every report.
    #[test]
    fn sstvs_expected_class_follows_the_band() {
        assert_eq!(expected_rig_class(Mode::Sstv, false, 14_230_000.0), rig_mode_class(Mode::Usb));
        for dial in [1_890_000.0, 3_730_000.0, 7_171_000.0] {
            assert_eq!(
                expected_rig_class(Mode::Sstv, false, dial),
                rig_mode_class(Mode::Lsb),
                "at {:.3} MHz",
                dial / 1e6
            );
        }
        // The FM twin has no sideband to follow the band with.
        assert_eq!(expected_rig_class(Mode::SstvFm, false, 7_171_000.0), rig_mode_class(Mode::Nfm));
    }
}

#[cfg(test)]
mod digi_config_tests {
    use super::*;

    #[test]
    fn a_client_config_cannot_wipe_the_band_offsets() {
        use sdroxide_types::Band;
        // What the engine has learned: an offset the operator set on 60 m.
        let mut current = DigiConfig::default();
        current.tx_audio_hz.insert(Band::M60, 370.0);

        // What a panel sends when a chip is toggled: its own copy, seeded
        // before that offset existed, carrying an empty map and one real edit.
        let incoming = DigiConfig { hold_tx_freq: true, ..DigiConfig::default() };

        let merged = keep_engine_owned(incoming, &current);
        assert_eq!(
            merged.tx_audio_hz.get(&Band::M60).copied(),
            Some(370.0),
            "a chip toggle wiped the band offsets"
        );
        assert!(merged.hold_tx_freq, "and the edit the client actually made was lost");
    }

    /// The same trap, one map along (issue #186). The transmit-audio rail has
    /// its own command, so a client seeded before the operator touched it sends
    /// an empty map — and an hour of per-mode levels would go over the good
    /// file on the next squelch nudge.
    #[test]
    fn a_client_config_cannot_wipe_the_per_mode_tx_levels() {
        let mut current = DigiConfig::default();
        current.set_tx_level(Mode::Ft8, 0.25);
        current.set_tx_level(Mode::Rtty, 0.4);

        let incoming = DigiConfig { digi_squelch: 3.0, ..DigiConfig::default() };

        let merged = keep_engine_owned(incoming, &current);
        assert_eq!(merged.tx_level_for(Mode::Ft8), 0.25, "a squelch nudge wiped the TX levels");
        assert_eq!(merged.tx_level_for(Mode::Rtty), 0.4);
        assert_eq!(merged.digi_squelch, 3.0, "and the edit the client actually made was lost");
    }
}

#[cfg(test)]
mod recording_tap_tests {
    use super::*;

    fn drained(cons: &mut rtrb::Consumer<f32>) -> Vec<f32> {
        std::iter::from_fn(|| cons.pop().ok()).collect()
    }

    /// A stereo recording splits RX and TX into an ear each only when there is
    /// a second receiver holding the right channel open. With one receiver and
    /// a mono signal there is nothing to separate, so both sides go to both
    /// channels rather than leaving the file playing out of one ear.
    #[test]
    fn one_receiver_records_dual_mono() {
        let (out, _speaker) = rtrb::RingBuffer::<f32>::new(64);
        let (tap, mut rec) = rtrb::RingBuffer::<f32>::new(64);
        let mut mixer = StereoMixer::new(out);
        mixer.rec_tap = Some(tap);

        // Receive, no sub: the same sample in both channels.
        mixer.push(&[0.5, 0.25], None, &[0.5, 0.25], None);
        assert_eq!(drained(&mut rec), vec![0.5, 0.5, 0.25, 0.25]);

        // The over that follows is centred the same way — a file that is dual
        // mono on receive must not go one-sided the moment the operator keys.
        mixer.rx_rec_enabled = false;
        mixer.push_tx(&[0.75]);
        assert_eq!(drained(&mut rec), vec![0.75, 0.75]);

        // Switch the sub receiver on and the split is back: it owns the right
        // channel, and the over keeps to that side so the two ends of the QSO
        // stay apart.
        mixer.rx_rec_enabled = true;
        mixer.push(&[0.5], Some(&[0.1]), &[0.5], Some(&[0.1]));
        assert_eq!(drained(&mut rec), vec![0.5, 0.1]);
        mixer.rx_rec_enabled = false;
        mixer.push_tx(&[0.75]);
        assert_eq!(drained(&mut rec), vec![0.0, 0.75]);
    }

    /// The operator's receive-audio trim is the one gain in the path that can
    /// make a radio *louder*, which a rig with a quiet USB codec needs (issue
    /// #315) — and it must reach the speakers without reaching the recording,
    /// which is the whole point of the recorder's tap arriving separately.
    ///
    /// The limit is not decoration either: past full scale the conversion to a
    /// card's own sample format is free to wrap, and a wrap is a click at the
    /// loudest the card can go.
    #[test]
    fn the_receive_trim_lifts_the_speakers_and_not_the_recording() {
        let (out, mut speaker) = rtrb::RingBuffer::<f32>::new(64);
        let (tap, mut rec) = rtrb::RingBuffer::<f32>::new(64);
        let mut mixer = StereoMixer::new(out);
        mixer.rec_tap = Some(tap);
        mixer.rec_mono = true;

        // Exactly double, which +6 dB is only nearly.
        mixer.set_trim_db(20.0 * 2f32.log10());
        mixer.push(&[0.25], None, &[0.25], None);
        // One receiver, so the speaker path is the same sample in both ears.
        let heard = drained(&mut speaker);
        assert_eq!(heard.len(), 2, "{heard:?}");
        assert!(heard.iter().all(|s| (s - 0.5).abs() < 1e-3), "{heard:?}");
        assert_eq!(drained(&mut rec), vec![0.25], "the trim must not reach the archive");

        // Loud enough to leave the rails, and held at them.
        mixer.set_trim_db(30.0);
        mixer.push(&[0.5, -0.5], None, &[0.5, -0.5], None);
        let heard = drained(&mut speaker);
        assert_eq!(heard, vec![1.0, 1.0, -1.0, -1.0]);
        assert_eq!(drained(&mut rec), vec![0.5, -0.5]);

        // A figure nobody should be able to reach, however `radio.json` was
        // edited — and the default is off.
        mixer.set_trim_db(200.0);
        let capped = mixer.trim;
        mixer.set_trim_db(30.0);
        assert_eq!(capped, mixer.trim);
        mixer.set_trim_db(0.0);
        assert_eq!(mixer.trim, 1.0);
    }

    /// The mono recording is one channel however many receivers are running —
    /// dual mono is a stereo-file layout, not a downmix.
    #[test]
    fn mono_recording_stays_one_channel() {
        let (out, _speaker) = rtrb::RingBuffer::<f32>::new(64);
        let (tap, mut rec) = rtrb::RingBuffer::<f32>::new(64);
        let mut mixer = StereoMixer::new(out);
        mixer.rec_tap = Some(tap);
        mixer.rec_mono = true;

        mixer.push(&[0.5, 0.25], None, &[0.5, 0.25], None);
        assert_eq!(drained(&mut rec), vec![0.5, 0.25]);
        mixer.rx_rec_enabled = false;
        mixer.push_tx(&[0.75]);
        assert_eq!(drained(&mut rec), vec![0.75]);
    }
}

#[cfg(test)]
mod stereo_tests {
    use super::*;

    /// Device-rate IQ carrying an FM stereo multiplex, hard-panned left.
    fn wfm_stereo_iq(dev_rate: f64, secs: f64) -> Vec<Complex32> {
        let n = (dev_rate * secs) as usize;
        let mut phase = 0.0f64;
        (0..n)
            .map(|i| {
                let t = i as f64 / dev_rate;
                let (l, r) = (0.8 * (std::f64::consts::TAU * 1_000.0 * t).sin(), 0.0);
                let (m, s) = ((l + r) / 2.0, (l - r) / 2.0);
                // Sine phase, as the broadcast standard specifies.
                let mpx = 0.9 * (m + s * (std::f64::consts::TAU * 38_000.0 * t).sin())
                    + 0.1 * (std::f64::consts::TAU * 19_000.0 * t).sin();
                phase += std::f64::consts::TAU * 75_000.0 * mpx / dev_rate;
                Complex32::new(phase.cos() as f32, phase.sin() as f32)
            })
            .collect()
    }

    fn goertzel(x: &[f32], freq: f64, rate: f64) -> f64 {
        let w = std::f64::consts::TAU * freq / rate;
        let coeff = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for &v in x {
            let s0 = v as f64 + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        (s1 * s1 + s2 * s2 - coeff * s1 * s2) / (x.len() as f64 * x.len() as f64 / 4.0)
    }

    /// The whole receive chain, not just the demodulator: DDC, AGC, squelch,
    /// volume, the L/R matrix and the stereo resampler, exactly as the engine
    /// runs them.
    #[test]
    fn rx_chain_delivers_separated_stereo() {
        let dev_rate = 1_536_000.0;
        let out_rate = 48_000.0;
        let mut rx = RxState::with_mode(Mode::Wfm);
        rx.volume = 1.0;
        let mut chain = RxChain::new(dev_rate, &rx, out_rate);

        let iq = wfm_stereo_iq(dev_rate, 6.0);
        let (mut left, mut right) = (Vec::new(), Vec::new());
        let (mut rec_left, mut rec_right) = (Vec::new(), Vec::new());
        for block in iq.chunks(16_384) {
            let (l, r) = chain.run(block, &rx, true);
            // Once stereo is up the chain must deliver both ears every block.
            // A block can be stereo and still carry no samples, the resampler
            // not having filled a chunk yet; `run` reports that as `Some(&[])`
            // where the recorder tap reports `None`. Both mean "nothing here",
            // so there is nothing to compare either.
            let stereo_block = match r {
                Some(r) if !r.is_empty() => {
                    assert_eq!(l.len(), r.len(), "L/R block lengths diverged");
                    left.extend_from_slice(l);
                    right.extend_from_slice(r);
                    true
                }
                _ => false,
            };
            if stereo_block {
                let (rl, rr) = chain.take_rec_audio();
                rec_left.extend_from_slice(rl);
                rec_right.extend_from_slice(rr.expect("a stereo block must tap both ears"));
            }
        }
        assert!(chain.stereo_locked(), "pilot never locked through the chain");
        assert!(!left.is_empty(), "chain never produced a stereo block");

        // The speaker path and the recorder tap are built in separate passes
        // over the same matrixed block, so they can drift apart without anything
        // else noticing. At volume 1.0 the only difference between them is the
        // scaling that isn't happening, so they must agree sample for sample.
        assert_eq!(rec_left, left, "recorder tap diverged from the left speaker");
        assert_eq!(rec_right, right, "recorder tap diverged from the right speaker");

        let tail = left.len() * 3 / 4;
        let pl = goertzel(&left[tail..], 1_000.0, out_rate);
        let pr = goertzel(&right[tail..], 1_000.0, out_rate);
        let sep = 10.0 * (pl / pr.max(1e-30)).log10();
        assert!(sep >= 20.0, "separation only {sep:.1} dB out of the full chain");
    }

    /// Noise reduction and the auto-notch delay the sum by a whole frame; the
    /// matrix cannot survive that, so the chain must fall back to mono.
    #[test]
    fn noise_reduction_forces_mono() {
        let dev_rate = 1_536_000.0;
        let mut rx = RxState::with_mode(Mode::Wfm);
        rx.volume = 1.0;
        let mut chain = RxChain::new(dev_rate, &rx, 48_000.0);
        let iq = wfm_stereo_iq(dev_rate, 3.0);
        for block in iq.chunks(16_384) {
            let _ = chain.run(block, &rx, false);
        }
        assert!(chain.stereo_locked());

        // The fade is deliberate (200 ms), so what matters is that it *reaches*
        // mono and stays there, not that it switches on the first block.
        rx.noise_reduction = NrLevel::Medium;
        let mut last_stereo = true;
        for block in iq.chunks(16_384) {
            let (_, r) = chain.run(block, &rx, false);
            last_stereo = r.is_some();
        }
        assert!(!last_stereo, "still decoding stereo after NR had been on for 3 s");
    }

    /// Every engine, through the real chain. The two NR call sites are near
    /// duplicates, so a wiring mistake in one is invisible in the other; this
    /// at least pins that each engine runs, keeps the block length, and hands
    /// back finite audio. One chain for all twelve levels, so the
    /// DeepFilterNet model is unpacked once.
    #[test]
    fn every_nr_engine_preserves_the_block() {
        let dev_rate = 1_536_000.0;
        let mut rx = RxState::with_mode(Mode::Usb);
        rx.volume = 1.0;
        let mut chain = RxChain::new(dev_rate, &rx, 48_000.0);
        // Broadband noise is the honest input here: it is what NR is pointed at.
        let mut seed = 0x5EEDu64;
        let iq: Vec<Complex32> = (0..16_384 * 8)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let r = ((seed >> 40) as i32) as f32 / (1 << 23) as f32 - 1.0;
                Complex32::new(r * 0.2, r * 0.2)
            })
            .collect();

        for level in NrLevel::ALL {
            rx.noise_reduction = level;
            for block in iq.chunks(16_384) {
                let (out, _) = chain.run(block, &rx, false);
                assert!(out.iter().all(|s| s.is_finite()), "{level:?} produced non-finite audio");
            }
        }
    }
}

/// Slide a batch of pooled waterfall rows from one window onto another, and
/// say whether it could be done at all.
///
/// Only a move: the span and the column count have to match, and then the bins
/// are the same width and the move is a whole number of them. Each row keeps
/// whatever of the band is still inside the window, and the strip that has
/// just come into view is filled with the floor — the same "nothing here yet"
/// a client's own history remap leaves along the edge it has just uncovered.
///
/// False for a zoom, a width change, or a move further than the window is
/// wide: there is nothing of the old picture left to carry, and the caller
/// starts the batch again.
fn slide_rows(batch: &mut [u8], from: (f64, f64, usize), to: (f64, f64, usize)) -> bool {
    let (to_center, span, cols) = to;
    if cols == 0 || from.2 != cols || span <= 0.0 || (from.1 - span).abs() > span * 1e-6 {
        return false;
    }
    let d = ((to_center - from.0) / (span / cols as f64)).round();
    if d.abs() >= cols as f64 {
        return false;
    }
    let d = d as isize;
    if d != 0 {
        // Column `j` on the new axis is where column `j + d` was on the old.
        for row in batch.chunks_exact_mut(cols) {
            if d > 0 {
                let d = d as usize;
                row.copy_within(d.., 0);
                row[cols - d..].fill(0);
            } else {
                let d = d.unsigned_abs();
                row.copy_within(..cols - d, d);
                row[..d].fill(0);
            }
        }
    }
    true
}

#[cfg(test)]
mod binaural_tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    fn cw(binaural: bool) -> RxState {
        RxState { binaural, ..RxState::with_mode(Mode::Cw) }
    }

    /// A 700 Hz note in the middle of the CW passband, at a level the clamp
    /// never reaches.
    fn note() -> Vec<f32> {
        (0..4_096)
            .map(|i| 0.3 * (std::f64::consts::TAU * 700.0 * f64::from(i) / RATE).cos() as f32)
            .collect()
    }

    /// `binaural_split` against one block: what came back in each ear, and
    /// whether the widener was kept.
    fn split(rx: &RxState, right_free: bool, right_in: &[f32]) -> (Vec<f32>, Vec<f32>, bool) {
        let mut slot = None;
        let (mut mono, mut scratch) = (note(), Vec::new());
        let mut right = right_in.to_vec();
        binaural_split(&mut slot, rx, RATE, right_free, &mut mono, &mut scratch, &mut right);
        (mono, right, slot.is_some())
    }

    /// With it on in CW, the second ear is filled and the two are no longer
    /// the same signal — but they still sum back to the audio that went in.
    #[test]
    fn cw_fills_the_second_ear() {
        let (left, right, kept) = split(&cw(true), true, &[]);
        assert!(kept, "the widener should be kept while it is running");
        assert_eq!(left.len(), 4_096);
        assert_eq!(right.len(), 4_096);
        assert!(left != right, "the two ears are the same signal");
        // Past the latency, where both ears carry real audio rather than the
        // priming zeros.
        let mono = note();
        let far = 2_048;
        assert!(
            (far..4_096).any(|i| (left[i] - mono[i]).abs() > 0.05),
            "the left ear is unchanged, so nothing was spread"
        );
        assert!(right[far..].iter().any(|s| s.abs() > 0.05), "the right ear is silent");
    }

    /// SSB is spread too — the same treatment as CW, which is what the modes
    /// the chip is drawn in ask for.
    #[test]
    fn ssb_is_spread_as_well() {
        let rx = RxState { binaural: true, ..RxState::with_mode(Mode::Usb) };
        let (left, right, kept) = split(&rx, true, &[]);
        assert!(kept);
        assert_eq!(right.len(), 4_096);
        assert!(left != right, "the two ears are the same signal");
    }

    /// Every mode that is *not* offered it is left exactly as it was: no second
    /// ear, no widener, and the audio untouched.
    #[test]
    fn other_modes_are_left_mono() {
        for mode in [Mode::Am, Mode::Nfm, Mode::Ft8, Mode::Digu] {
            let rx = RxState { binaural: true, ..RxState::with_mode(mode) };
            let (left, right, kept) = split(&rx, true, &[]);
            assert_eq!(left, note(), "{mode:?} was widened");
            assert!(right.is_empty(), "{mode:?} filled the right ear");
            assert!(!kept);
        }
    }

    /// …and so is CW with it switched off, which is also where the filter
    /// state is dropped rather than left to go stale.
    #[test]
    fn switching_it_off_leaves_the_audio_alone() {
        let (left, right, kept) = split(&cw(false), true, &[]);
        assert_eq!(left, note());
        assert!(right.is_empty());
        assert!(!kept);

        let mut slot = None;
        let (mut mono, mut scratch, mut right) = (note(), Vec::new(), Vec::new());
        binaural_split(&mut slot, &cw(true), RATE, true, &mut mono, &mut scratch, &mut right);
        assert!(slot.is_some());
        right.clear();
        binaural_split(&mut slot, &cw(false), RATE, true, &mut mono, &mut scratch, &mut right);
        assert!(slot.is_none(), "the widener outlived the setting that asked for it");
    }

    /// The sub receiver owns the right ear when it is running: an explicit
    /// second receiver outranks an effect on the first.
    #[test]
    fn the_sub_receiver_keeps_the_right_ear() {
        let (left, right, kept) = split(&cw(true), false, &[]);
        assert_eq!(left, note());
        assert!(right.is_empty());
        assert!(!kept);
    }

    /// So does audio that is already stereo — nothing here overwrites an ear
    /// something else has filled.
    #[test]
    fn an_ear_that_is_already_filled_is_not_taken() {
        let filled = vec![0.1f32; 4_096];
        let (left, right, kept) = split(&cw(true), true, &filled);
        assert_eq!(left, note());
        assert_eq!(right, filled);
        assert!(!kept);
    }
}

#[cfg(test)]
mod slide_rows_tests {
    use super::slide_rows;

    /// 8 columns over 800 Hz centred on 1000: one column per 100 Hz.
    fn axis(center: f64) -> (f64, f64, usize) {
        (center, 800.0, 8)
    }

    /// The window moves up by two columns, so the picture moves down by two
    /// and the top two columns are band nobody has heard yet.
    #[test]
    fn a_window_that_moved_up_carries_its_rows_down() {
        let mut rows = vec![1, 2, 3, 4, 5, 6, 7, 8, 11, 12, 13, 14, 15, 16, 17, 18];
        assert!(slide_rows(&mut rows, axis(1000.0), axis(1200.0)));
        assert_eq!(rows[..8], [3, 4, 5, 6, 7, 8, 0, 0]);
        assert_eq!(rows[8..], [13, 14, 15, 16, 17, 18, 0, 0]);
    }

    /// And the other way.
    #[test]
    fn a_window_that_moved_down_carries_them_up() {
        let mut rows = vec![1, 2, 3, 4, 5, 6, 7, 8];
        assert!(slide_rows(&mut rows, axis(1000.0), axis(900.0)));
        assert_eq!(rows, [0, 1, 2, 3, 4, 5, 6, 7]);
    }

    /// Less than half a column is no move at all — a fractional shift applied
    /// every frame would blur the batch away for nothing.
    #[test]
    fn a_move_shorter_than_a_column_leaves_the_rows_alone() {
        let mut rows = vec![1, 2, 3, 4, 5, 6, 7, 8];
        assert!(slide_rows(&mut rows, axis(1000.0), axis(1040.0)));
        assert_eq!(rows, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    /// A zoom is not a move, and neither is a width change: nothing lines up
    /// column for column, so the caller has to start again.
    #[test]
    fn a_zoom_or_a_width_change_cannot_be_slid() {
        let mut rows = vec![1, 2, 3, 4, 5, 6, 7, 8];
        assert!(!slide_rows(&mut rows, axis(1000.0), (1000.0, 400.0, 8)));
        assert!(!slide_rows(&mut rows, axis(1000.0), (1000.0, 800.0, 4)));
    }

    /// Past the width of the window there is nothing left to carry.
    #[test]
    fn a_move_clear_of_the_window_keeps_nothing() {
        let mut rows = vec![1, 2, 3, 4, 5, 6, 7, 8];
        assert!(!slide_rows(&mut rows, axis(1000.0), axis(1900.0)));
    }
}

/// Max-pool dB bins down to `out_bins` and map them onto the u8 range clients
/// draw, producing a frame with the axis the caller describes.
///
/// Max rather than mean on purpose: a narrow carrier occupying one bin of
/// several thousand must survive being squeezed into one pixel, and averaging
/// it against its neighbours is exactly how a panadapter loses weak signals as
/// the operator zooms out.
///
/// `viewport` extracts a sub-span, as [`sdroxide_dsp::SpectrumAnalyzer::make_frame`]
/// does for bins this engine computed itself, and the returned frame's axis then
/// describes the viewport. The full-band strip never zooms and passes `None`;
/// the main lane does, and pre-computed bins have to honour it the same way or
/// zooming a scope-fed panadapter would move the axis without moving the trace.
fn pool_window_to_frame(
    db: &[f32],
    seq: u32,
    center_hz: f64,
    span_hz: f64,
    db_floor: f32,
    db_ceil: f32,
    out_bins: usize,
    viewport: Option<(f64, f64)>,
) -> SpectrumFrame {
    let scale = 255.0 / (db_ceil - db_floor).max(1e-6);
    let n = db.len();
    if n == 0 || out_bins == 0 {
        return SpectrumFrame {
            seq,
            center_hz,
            span_hz,
            db_floor,
            db_ceil,
            bins: vec![0; out_bins],
            rows: Vec::new(),
            rows_clocked: false,
        };
    }
    let (frac_lo, frac_hi, out_center, out_span) = match viewport {
        Some((lo, hi)) if hi > lo && span_hz > 0.0 => {
            let full_lo = center_hz - span_hz / 2.0;
            let flo = ((lo - full_lo) / span_hz).clamp(0.0, 0.998);
            let fhi = ((hi - full_lo) / span_hz).clamp(flo + 0.002, 1.0);
            (flo, fhi, full_lo + (flo + fhi) / 2.0 * span_hz, (fhi - flo) * span_hz)
        }
        _ => (0.0, 1.0, center_hz, span_hz),
    };
    let lo_bin = frac_lo * n as f64;
    let bin_range = (frac_hi - frac_lo) * n as f64;
    let mut bins = Vec::with_capacity(out_bins);
    if bin_range < out_bins as f64 {
        // Stretching, not pooling: the window holds fewer measurements than
        // there are columns to fill, so each one has to cover several.
        //
        // Reading between them rather than repeating them. A rig's own scope is
        // a fixed number of points however wide the panadapter is drawn — an
        // IC-705 sends 475 across its whole span, so a view of a fifth of that
        // span is 85 numbers spread over a couple of thousand pixels — and
        // repeating each one seventeen times is the wall of hard blocks that
        // gets reported as a broken waterfall. It is also self-inflicted: the
        // waterfall's own sampler is linear and would have drawn exactly this
        // gradient, had the frame not arrived pre-blocked.
        //
        // Only in this direction. Coarsening stays the peak below, because a
        // carrier one bin wide has to survive being pooled into a column, and
        // averaging or sampling it away is how a signal disappears from a
        // zoomed-out panadapter.
        for i in 0..out_bins {
            // Centre of this column, in source-bin coordinates, with the half
            // bin taken off so bin centres land on column centres.
            let at = lo_bin + (i as f64 + 0.5) * bin_range / out_bins as f64 - 0.5;
            let k = at.floor().clamp(0.0, (n - 1) as f64) as usize;
            let t = (at - k as f64).clamp(0.0, 1.0) as f32;
            let (a, b) = (db[k], db[(k + 1).min(n - 1)]);
            bins.push(((a + (b - a) * t - db_floor) * scale).clamp(0.0, 255.0) as u8);
        }
    } else {
        for i in 0..out_bins {
            let lo = ((lo_bin + i as f64 * bin_range / out_bins as f64) as usize).min(n - 1);
            let hi =
                ((lo_bin + (i + 1) as f64 * bin_range / out_bins as f64) as usize).clamp(lo + 1, n);
            let peak = db[lo..hi].iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            bins.push(((peak - db_floor) * scale).clamp(0.0, 255.0) as u8);
        }
    }
    SpectrumFrame {
        seq,
        center_hz: out_center,
        span_hz: out_span,
        db_floor,
        db_ceil,
        bins,
        rows: Vec::new(),
        rows_clocked: false,
    }
}

#[cfg(test)]
mod skim_window_tests {
    use super::{SKIM_TARGET_HZ, skim_center_for};

    /// An RX-888 handed the whole half-spectrum: 32.4 MHz of it, centred on
    /// 16.2 MHz because that is the only place a window that wide can sit.
    const WIDE: f64 = 32_400_000.0;
    const WIDE_CENTER: f64 = 16_200_000.0;
    /// Rounded off the decimation ladder from that rate; the exact figure only
    /// matters to the arithmetic, not to the rule.
    const WIN: f64 = 202_500.0;

    /// The bug as reported: the skimmers on a wide front end decoded nothing at
    /// all, because their window sat in the middle of the sampled span while the
    /// operator watched a band 2 MHz away.
    #[test]
    fn the_window_goes_where_the_operator_is_looking() {
        let view = (14_000_000.0, 14_100_000.0);
        let c = skim_center_for(Some(view), 14_030_000.0, WIDE_CENTER, WIDE, WIN, None);
        assert!(
            c - WIN / 2.0 <= view.0 && c + WIN / 2.0 >= view.1,
            "window at {c} does not cover {view:?}"
        );
    }

    /// Nobody watching — a headless server, or a client that has not said what
    /// it is showing yet — and the window starts where it always did.
    #[test]
    fn no_view_starts_on_the_hardware_centre() {
        assert_eq!(skim_center_for(None, 14_030_000.0, WIDE_CENTER, WIDE, WIN, None), WIDE_CENTER);
    }

    /// A view the window already covers does not move it. Every move costs each
    /// track in the window and each callsign half-read, and a drag reports a new
    /// view several times a second.
    #[test]
    fn a_pan_inside_the_window_holds_it_still() {
        let cur = 14_050_000.0;
        for lo in [14_020_000.0, 14_000_000.0, 14_080_000.0] {
            let view = Some((lo, lo + 20_000.0));
            let c = skim_center_for(view, lo + 10_000.0, WIDE_CENTER, WIDE, WIN, Some(cur));
            assert_eq!(c, cur, "view {view:?} moved the window");
        }
    }

    /// ...and a pan that leaves it does, onto the view it can no longer see.
    #[test]
    fn a_pan_out_of_the_window_moves_it() {
        let cur = 14_050_000.0;
        let view = (14_200_000.0, 14_240_000.0);
        let c = skim_center_for(Some(view), 14_220_000.0, WIDE_CENTER, WIDE, WIN, Some(cur));
        assert_eq!(c, (view.0 + view.1) / 2.0);
    }

    /// A screen wider than the window can only ever be part-covered, so the part
    /// is the one around the dial — a band-wide view of 20 m has its CW at one
    /// end and its phone at the other, and the operator is in one of them.
    #[test]
    fn a_wide_view_keeps_the_dial_in_the_window() {
        let view = Some((14_000_000.0, 14_350_000.0));
        let dial = 14_025_000.0;
        let c = skim_center_for(view, dial, WIDE_CENTER, WIDE, WIN, None);
        assert!((c - dial).abs() < WIN / 2.0, "dial {dial} outside the window at {c}");
        // ...without the window hanging off the end of the screen, where half of
        // it would be skimming spectrum nobody is looking at.
        assert!(c - WIN / 2.0 >= 14_000_000.0 - 0.5, "window at {c} runs past the view");
    }

    /// Tuning across that view does not re-cut the window every few kilohertz.
    /// It has to be the dial that moves it, and only once the dial is well out
    /// towards the edge, or a scrolled VFO would keep throwing the decoders away.
    #[test]
    fn ordinary_tuning_does_not_re_cut_a_wide_view() {
        let view = Some((14_000_000.0, 14_350_000.0));
        let cur = 14_101_250.0;
        for dial in [14_090_000.0, 14_101_250.0, 14_120_000.0, 14_150_000.0] {
            assert_eq!(
                skim_center_for(view, dial, WIDE_CENTER, WIDE, WIN, Some(cur)),
                cur,
                "dial {dial} moved the window"
            );
        }
        let far = 14_300_000.0;
        assert_ne!(skim_center_for(view, far, WIDE_CENTER, WIDE, WIN, Some(cur)), cur);
    }

    /// The window is an NCO offset inside the stream, so both its edges stay
    /// inside what was sampled however close to the end of the span the operator
    /// is looking — including the bottom of 160 m on a front end that starts at
    /// zero.
    #[test]
    fn the_window_stays_inside_the_span() {
        for view in [(1_800_000.0, 1_840_000.0), (32_000_000.0, 32_400_000.0)] {
            let c = skim_center_for(Some(view), view.0, WIDE_CENTER, WIDE, WIN, None);
            assert!(c - WIN / 2.0 >= -0.5, "window at {c} starts below the span");
            assert!(c + WIN / 2.0 <= WIDE + 0.5, "window at {c} ends above the span");
        }
    }

    /// A front end no wider than the window has nothing to place: it is all
    /// window, wherever the operator looks. This is every narrow SDR and every
    /// I/Q rig, and the behaviour there must not change at all.
    #[test]
    fn a_narrow_front_end_is_all_window() {
        let center = 14_100_000.0;
        for span in [48_000.0, 192_000.0, SKIM_TARGET_HZ] {
            let view = Some((center - 5_000.0, center + 5_000.0));
            assert_eq!(skim_center_for(view, center, center, span, SKIM_TARGET_HZ, None), center);
        }
    }

    /// A front end that retuned out from under a stationary window takes the
    /// window with it rather than leaving it hanging outside the new span.
    #[test]
    fn a_retune_drags_a_window_left_outside_the_span() {
        // 2 Msps on 14.1 MHz, then the dial jumps a band with the view.
        let (span, cur) = (2_000_000.0, 14_050_000.0);
        let view = Some((21_020_000.0, 21_060_000.0));
        let c = skim_center_for(view, 21_030_000.0, 21_040_000.0, span, WIN, Some(cur));
        assert!((c - 21_040_000.0).abs() < span / 2.0, "window at {c} is outside the new span");
    }

    /// A degenerate view — a client mid-layout, or one whose window has drifted
    /// off the end of the span entirely — leaves the window where it is rather
    /// than parking it somewhere arbitrary.
    #[test]
    fn a_view_the_front_end_cannot_reach_is_ignored() {
        let cur = 14_050_000.0;
        let view = Some((88_000_000.0, 88_200_000.0));
        assert_eq!(skim_center_for(view, 88_100_000.0, WIDE_CENTER, WIDE, WIN, Some(cur)), cur);
    }
}

#[cfg(test)]
mod qo100_rate_tests {
    use super::{qo100_capture_rate_for, qo100_rate_for_cfg};
    use sdroxide_types::Qo100Settings;

    use super::{
        QO100_AUTO_AGREE_RUN, QO100_AUTO_MAX_STALLED, QO100_AUTO_MIN_INTERVAL,
        QO100_AUTO_TOTAL_MAX_HZ, Qo100AutoRun, Qo100AutoStop, qo100_auto_correction,
    };
    use std::time::Duration;

    // A twin-lobe shape clean enough for the loop to act on.
    const CLEAN: (f32, f32, f32) = (7.0, 0.85, 6.0);

    /// Feed `off` through as a run of agreeing clean cycles from `prev` and
    /// return the final outcome — `since_last` says how long ago the last
    /// correction was.
    fn run_from(
        prev: Qo100AutoRun,
        off: f64,
        cycles: u8,
        since_last: Option<Duration>,
    ) -> super::Qo100AutoOutcome {
        let (n, s, r) = CLEAN;
        let mut out = qo100_auto_correction(Some(off), n, s, r, prev, since_last);
        for _ in 1..cycles {
            out = qo100_auto_correction(Some(off), n, s, r, out.run, since_last);
        }
        out
    }

    fn run(off: f64, cycles: u8, since_last: Option<Duration>) -> super::Qo100AutoOutcome {
        run_from(Qo100AutoRun::default(), off, cycles, since_last)
    }

    #[test]
    fn the_loop_needs_a_run_of_agreeing_clean_cycles_before_it_corrects() {
        // One short of the run: still nothing.
        let held = run(400.0, QO100_AUTO_AGREE_RUN - 1, None);
        assert_eq!(held.correction, None);
        // The run completed: correction goes out and the run resets.
        let fired = run(400.0, QO100_AUTO_AGREE_RUN, None);
        assert_eq!(fired.correction, Some(400.0));
        assert_eq!(fired.run.agree_run, 0);
        assert_eq!(fired.run.agree_ref_hz, None);
        assert_eq!(fired.run.total_hz, 400.0);
    }

    #[test]
    fn a_marginal_shape_breaks_the_run() {
        let (_, s, r) = CLEAN;
        // A full run's worth of agreeing estimates, but the shape is marginal.
        let prev = Qo100AutoRun { agree_ref_hz: Some(400.0), agree_run: 9, ..Default::default() };
        let out = qo100_auto_correction(Some(400.0), 3.0, s, r, prev, None);
        assert_eq!(out.correction, None);
        assert_eq!(out.run.agree_run, 0, "a bad cycle resets the run");
    }

    #[test]
    fn an_estimate_that_flips_sign_restarts_the_run() {
        let (n, s, r) = CLEAN;
        let prev = Qo100AutoRun { agree_ref_hz: Some(-400.0), agree_run: 9, ..Default::default() };
        let out = qo100_auto_correction(Some(400.0), n, s, r, prev, None);
        assert_eq!(out.correction, None);
        assert_eq!(out.run.agree_run, 1, "an oscillation never builds a run");
    }

    #[test]
    fn inside_the_deadband_nothing_is_written() {
        let out = run(120.0, QO100_AUTO_AGREE_RUN + 2, None);
        assert_eq!(out.correction, None);
    }

    /// The loop checking its own work: a correction that does not move the
    /// beacon is the signature of a write landing somewhere the receiver does
    /// not read it (a transverter row covering the beacon, a front end that
    /// will not reopen). Repeating it forever is the runaway; this stops.
    ///
    /// Driven the way the engine drives it — a stream of *estimates*, with the
    /// beacon staying obstinately where it was however often it is corrected.
    #[test]
    fn a_correction_that_does_not_move_the_beacon_takes_the_loop_off() {
        let settled = Some(QO100_AUTO_MIN_INTERVAL + Duration::from_secs(1));
        let (n, sy, r) = CLEAN;
        let mut state = Qo100AutoRun::default();
        let mut corrections = 0;

        // The beacon reads 3 kHz out and never moves, whatever is written.
        for _ in 0..40 {
            let out = qo100_auto_correction(Some(3_000.0), n, sy, r, state, settled);
            state = out.run;
            if out.correction.is_some() {
                corrections += 1;
            }
            if let Some(stop) = out.stop {
                assert_eq!(stop, Qo100AutoStop::NotConverging);
                assert!(
                    corrections <= QO100_AUTO_MAX_STALLED as usize + 1,
                    "gave up only after {corrections} fruitless corrections"
                );
                // …and it stays off: nothing more is written after the stop.
                let after = qo100_auto_correction(Some(3_000.0), n, sy, r, state, settled);
                assert_eq!(after.correction, None);
                return;
            }
        }
        panic!("the loop corrected {corrections} times and never gave up");
    }

    /// A correction that *is* taking effect leaves the beacon back on target,
    /// and that resets the patience above — a slowly warming LNB corrected
    /// over and over across an afternoon must never look like a runaway, even
    /// though every one of its corrections is much the same size.
    #[test]
    fn a_beacon_that_comes_back_to_target_is_a_working_loop() {
        let settled = Some(QO100_AUTO_MIN_INTERVAL + Duration::from_secs(1));
        let (n, sy, r) = CLEAN;
        let mut state = Qo100AutoRun::default();

        for round in 0..8 {
            // The LNB has drifted another ~300 Hz: three agreeing cycles, then
            // the correction goes out.
            let mut out = qo100_auto_correction(Some(300.0), n, sy, r, state, settled);
            for _ in 1..QO100_AUTO_AGREE_RUN {
                out = qo100_auto_correction(Some(300.0), n, sy, r, out.run, settled);
            }
            assert_eq!(out.correction, Some(300.0), "round {round}");
            assert_eq!(out.stop, None, "round {round}");

            // …and it worked: the beacon is back on target.
            let back = qo100_auto_correction(Some(10.0), n, sy, r, out.run, settled);
            assert_eq!(back.run.stalled, 0, "round {round}: a working correction clears the count");
            state = back.run;
        }
    }

    /// The backstop under the convergence check: however it got there, an
    /// offset this far from where it started is not a calibration any more.
    #[test]
    fn a_running_total_past_any_real_lnb_takes_the_loop_off() {
        let settled = Some(QO100_AUTO_MIN_INTERVAL + Duration::from_secs(1));
        let state =
            Qo100AutoRun { total_hz: QO100_AUTO_TOTAL_MAX_HZ - 100.0, ..Default::default() };
        let out = run_from(state, 4_000.0, QO100_AUTO_AGREE_RUN, settled);
        assert_eq!(out.correction, None);
        assert_eq!(out.stop, Some(Qo100AutoStop::Runaway));
    }

    #[test]
    fn a_recent_correction_holds_the_next_one_off() {
        assert_eq!(
            run(400.0, QO100_AUTO_AGREE_RUN + 2, Some(Duration::from_secs(2))).correction,
            None
        );
        let settled = Some(QO100_AUTO_MIN_INTERVAL + Duration::from_secs(1));
        assert_eq!(run(400.0, QO100_AUTO_AGREE_RUN, settled).correction, Some(400.0));
    }

    #[test]
    fn the_tracker_only_capture_just_covers_the_parking_window() {
        // Decoder off: with the tracker off too, the floor.
        let off = Qo100Settings {
            enabled: false,
            decode_telemetry: false,
            park_hi_hz: 12_000.0,
            ..Default::default()
        };
        assert_eq!(qo100_rate_for_cfg(&off), 16_000.0);

        // Tracker on, decoder off: enough to hold the top of the parking
        // window under Nyquist, and much less than the decoder's 2.5x.
        let tracking = Qo100Settings { enabled: true, search_half_width_hz: 25_000.0, ..off };
        let r = qo100_rate_for_cfg(&tracking);
        assert!(r / 2.0 > 12_000.0 + 800.0, "beacon at +12 kHz must be under Nyquist: {r}");
        assert!(
            r < 0.6 * qo100_capture_rate_for(25_000.0),
            "much lighter than what the decoder would ask for the same station: {r}"
        );
    }

    #[test]
    fn the_decoder_path_still_gets_its_full_oversampling() {
        let decode = Qo100Settings {
            enabled: true,
            decode_telemetry: true,
            search_half_width_hz: 25_000.0,
            park_hi_hz: 12_000.0,
            ..Default::default()
        };
        assert_eq!(qo100_rate_for_cfg(&decode), qo100_capture_rate_for(25_000.0));
    }

    /// The narrowest width (±5 kHz) and anything down to it sit on the 16 kHz
    /// floor — ×2.5 does not reach it until ±6.4 kHz.
    #[test]
    fn the_narrowest_widths_sit_on_the_16_khz_floor() {
        assert_eq!(qo100_capture_rate_for(5_000.0), 16_000.0);
        assert_eq!(qo100_capture_rate_for(0.0), 16_000.0);
        assert_eq!(qo100_capture_rate_for(6_400.0), 16_000.0);
    }

    /// Past the knee the capture tracks the search width, so widening the
    /// window in the UI really does hand the decoder a wider span.
    #[test]
    fn a_wider_search_asks_for_a_proportionally_wider_capture() {
        assert_eq!(qo100_capture_rate_for(10_000.0), 25_000.0);
        assert_eq!(qo100_capture_rate_for(25_000.0), 62_500.0);
        // ±50 kHz is the UI's MAX_HALF_WIDTH_HZ.
        assert_eq!(qo100_capture_rate_for(50_000.0), 125_000.0);
    }

    #[test]
    fn the_capture_rate_is_monotonic_and_never_below_the_floor() {
        let mut prev = 0.0;
        for hw in [0.0, 2_500.0, 5_000.0, 10_000.0, 20_000.0, 50_000.0] {
            let r = qo100_capture_rate_for(hw);
            assert!(r >= prev, "rate dropped at half-width {hw}");
            assert!(r >= 16_000.0, "below the floor at half-width {hw}");
            prev = r;
        }
    }
}

#[cfg(test)]
mod zoom_lane_fft_tests {
    use super::zoom_lane_fft;

    /// Enough transform to put a bin under every column *on screen*, which is
    /// eight times the columns: the lane's output is up to 2.8 times the
    /// viewport, and the viewport is about twice the window actually being
    /// looked at. Two dilutions, both paid for.
    #[test]
    fn a_column_on_screen_gets_a_bin_of_its_own() {
        assert_eq!(zoom_lane_fft(2048), 16_384);
        // It used to be twice the columns, which counted only the first
        // dilution and left this lane resolving half a bin per column — fine
        // while the client answered a zoom by growing the device-wide FFT
        // instead, and not fine now that this lane is what draws the window
        // (issue #195).
        assert_ne!(zoom_lane_fft(2048), 4096);
    }

    /// A wider panadapter asks for more, until the ceiling: past there a single
    /// transform covers more signal than the lane's hop can hide.
    #[test]
    fn a_wider_panadapter_gets_a_wider_transform() {
        assert_eq!(zoom_lane_fft(4096), 32_768);
        assert_eq!(zoom_lane_fft(8192), 32_768);
    }

    /// Held at both ends: nothing below the lane's own floor, and nothing past
    /// the point where one transform covers more signal than its hop can hide.
    #[test]
    fn it_is_a_power_of_two_inside_the_bounds() {
        for w in [0usize, 1, 1000, 2048, 3000, 4096, 8192, 100_000] {
            let n = zoom_lane_fft(w);
            assert!((4096..=32_768).contains(&n), "{w} gave {n}");
            assert!(n.is_power_of_two(), "{w} gave {n}");
        }
    }
}

#[cfg(test)]
mod tap_level_tests {
    use super::{TAP_GAIN_MAX, TAP_GAIN_MIN, TAP_TARGET_RMS, tap_gain_for};

    /// One block of a tone at `amp`.
    fn block(amp: f32, n: usize) -> Vec<f32> {
        (0..n).map(|k| amp * std::f32::consts::SQRT_2 * (k as f32 * 0.37).sin()).collect()
    }

    /// The decoder's tap has to be levelled — a weak signal decoded from a
    /// couple of dozen sixteen-bit steps is a signal thrown away — and it has
    /// to settle where the controller's own scaling expects it.
    fn settle(amp: f32) -> f32 {
        let (mut level, mut gain) = (f32::NAN, 1.0f32);
        let b = block(amp, 480);
        // Ten seconds of it, at a hundredth of a second a block.
        for _ in 0..1_000 {
            tap_gain_for(&mut level, &mut gain, &b, 48_000.0);
        }
        gain * amp
    }

    #[test]
    fn the_tap_settles_at_the_level_the_decoder_wants() {
        for amp in [1e-4f32, 1e-3, 0.01, 0.1, 0.5] {
            let out = settle(amp);
            assert!(
                (out - TAP_TARGET_RMS).abs() < TAP_TARGET_RMS * 0.05,
                "{amp} levelled to {out}"
            );
        }
    }

    /// And it must not move on the time scale a decoder cares about. That is
    /// the whole reason it is not the AGC: a gain that changes inside a
    /// transmission amplitude-modulates every tone in the passband and spreads
    /// each of them across its neighbours, which cost ten of thirteen decodes
    /// and about sixteen decibels of reported SNR in issue #307.
    #[test]
    fn the_level_barely_moves_across_a_symbol() {
        let (mut level, mut gain) = (f32::NAN, 1.0f32);
        let quiet = block(0.01, 480);
        for _ in 0..2_000 {
            tap_gain_for(&mut level, &mut gain, &quiet, 48_000.0);
        }
        // A station twenty decibels above the band opens up — a step far larger
        // than anything a real band takes in one go.
        let start = gain;
        let loud = block(0.1, 480);
        let mut after_symbol = 0.0f32;
        let mut after_half_second = 0.0f32;
        for k in 0..50 {
            let g = tap_gain_for(&mut level, &mut gain, &loud, 48_000.0);
            // An FT8 symbol is 160 ms; a block here is 10.
            if k == 15 {
                after_symbol = (20.0 * (g / start).log10()).abs();
            }
            after_half_second = (20.0 * (g / start).log10()).abs();
        }
        assert!(after_symbol < 1.0, "the tap gain moved {after_symbol:.2} dB across a symbol");
        assert!(
            after_half_second < 3.0,
            "the tap gain moved {after_half_second:.2} dB in half a second"
        );
    }

    /// On a band that is not changing — which is what a band is, minute to
    /// minute — the gain is simply a constant, and a slot of audio through it
    /// is the slot of audio the antenna delivered.
    #[test]
    fn a_steady_band_gives_a_constant() {
        let (mut level, mut gain) = (f32::NAN, 1.0f32);
        let b = block(0.02, 480);
        for _ in 0..3_000 {
            tap_gain_for(&mut level, &mut gain, &b, 48_000.0);
        }
        let settled = gain;
        let mut worst = 0.0f32;
        for _ in 0..(15 * 100) {
            let g = tap_gain_for(&mut level, &mut gain, &b, 48_000.0);
            worst = worst.max((20.0 * (g / settled).log10()).abs());
        }
        assert!(worst < 0.05, "the tap gain wandered {worst:.3} dB on a steady band");
    }

    /// It does follow a band that really has changed, though — over a few
    /// slots, not inside one. A decoder left on last hour's gain would be
    /// clipping or quantising instead.
    #[test]
    fn it_follows_a_band_that_has_actually_changed() {
        let (mut level, mut gain) = (f32::NAN, 1.0f32);
        let quiet = block(0.001, 480);
        for _ in 0..5_000 {
            tap_gain_for(&mut level, &mut gain, &quiet, 48_000.0);
        }
        let loud = block(0.1, 480);
        for _ in 0..(30 * 100) {
            tap_gain_for(&mut level, &mut gain, &loud, 48_000.0);
        }
        let out = gain * 0.1;
        assert!(
            (out - TAP_TARGET_RMS).abs() < TAP_TARGET_RMS * 0.05,
            "thirty seconds after the band came up the tap is still at {out}"
        );
    }

    /// Nothing a front end can hand over makes this produce a gain a decoder
    /// cannot use.
    #[test]
    fn the_gain_is_always_usable() {
        for amp in [0.0f32, 1e-30, 1e12, f32::NAN, f32::INFINITY] {
            let (mut level, mut gain) = (f32::NAN, 1.0f32);
            let b = block(amp, 128);
            for _ in 0..10 {
                let g = tap_gain_for(&mut level, &mut gain, &b, 48_000.0);
                assert!(g.is_finite(), "{amp} gave {g}");
                assert!((TAP_GAIN_MIN..=TAP_GAIN_MAX).contains(&g), "{amp} gave {g}");
            }
        }
        // An empty block asks nothing and changes nothing.
        let (mut level, mut gain) = (f32::NAN, 3.0f32);
        assert_eq!(tap_gain_for(&mut level, &mut gain, &[], 48_000.0), 3.0);
    }
}

#[cfg(test)]
mod analysis_window_tests {
    use super::{analysis_window, zoom_lane_fft};

    /// How much signal one transform covers, in seconds.
    fn secs(fft: usize, rate: f64, view: f64) -> f64 {
        analysis_window(fft, rate, view) as f64 / rate
    }

    /// The picture issue #302 was reported about: an FT8 sub-band on a wide
    /// panel. Before this, the transform was sized from the *columns* — 32768
    /// points at the few kilohertz a zoomed lane runs at, which is five and a
    /// half seconds of band in every row of the waterfall, on a mode whose
    /// symbols are 160 ms long. Every transmission drew as one unbroken bar.
    #[test]
    fn an_ft8_sub_band_is_a_third_of_a_second_not_five() {
        // A 48 kHz front end zoomed to the 3.7 kHz sub-band: the ladder settles
        // on a 6 kHz lane, and a 2560-column panel asks for a 32768-point
        // transform.
        let (rate, view) = (6_000.0, 3_700.0);
        let fft = zoom_lane_fft(2560);
        assert_eq!(fft, 32_768);
        assert!(
            (5.4..5.5).contains(&(fft as f64 / rate)),
            "the unpadded transform used to cover {:.2} s",
            fft as f64 / rate
        );
        let s = secs(fft, rate, view);
        assert!((0.2..0.4).contains(&s), "a row now covers {s:.3} s");
        // Which is 2.9 Hz a bin — WSJT-X's own waterfall transform, arrived at
        // from the resolution rule rather than copied from it.
        let bin = rate / analysis_window(fft, rate, view) as f64;
        assert!((2.0..4.0).contains(&bin), "{bin:.2} Hz a bin");
    }

    /// A deep zoom is a request for resolution and must still get it: the rule
    /// is "enough to resolve what is on screen", not a fixed number of
    /// milliseconds. A 200 Hz window on a carrier keeps every point it had.
    #[test]
    fn a_deep_zoom_keeps_its_resolution() {
        let fft = zoom_lane_fft(2560);
        // 48 kHz down the ladder to 375 Hz for a 200 Hz view.
        let n = analysis_window(fft, 375.0, 200.0);
        assert!(n >= 1_024, "a 200 Hz view was cut to {n} samples");
        let bin = 375.0 / n as f64;
        assert!(bin < 0.4, "{bin:.3} Hz a bin is coarser than the view deserves");
    }

    /// And a lane that draws everything it is fed is left exactly as it was: a
    /// device-wide panadapter resolves the whole span, and every bin of it is
    /// being looked at.
    #[test]
    fn a_lane_drawing_its_whole_span_is_untouched() {
        for (fft, rate) in [(4096usize, 32_400_000.0), (65_536, 2_400_000.0), (1024, 48_000.0)] {
            assert_eq!(analysis_window(fft, rate, rate), fft, "{fft} at {rate}");
        }
    }

    /// Nothing a client can send makes this return something a transform cannot
    /// use.
    #[test]
    fn it_is_always_a_usable_window() {
        for view in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e-9, 1e12] {
            for rate in [0.0, f64::NAN, 375.0, 48_000.0, 32_400_000.0] {
                let n = analysis_window(4096, rate, view);
                assert!(n > 0 && n <= 4096, "rate {rate} view {view} gave {n}");
            }
        }
    }
}

/// Whether the full-band lane, rather than the device analyser, is what covers
/// `viewport`.
///
/// True once the viewport reaches outside the passband — by being wider than
/// it, or by having been panned off its edge; the device analyser has nothing
/// to draw there either way — and only while the wide lane really is the wider
/// of the two.
///
/// `dev_center` is the centre the samples in hand were taken at rather than the
/// one most recently commanded. On a front end whose stream lags its tuning the
/// two differ, and the commanded one would flip the panadapter between the two
/// sources on every dial move.
fn wide_covers_viewport(
    (vp_lo, vp_hi): (f64, f64),
    dev_center: f64,
    dev_span: f64,
    (_wide_center, wide_span): (f64, f64),
    wide_bins: usize,
) -> bool {
    if !(dev_span > 0.0 && wide_span > dev_span) || wide_bins == 0 {
        return false;
    }
    // Never for a window this lane cannot resolve. Its bins are the whole band
    // divided a thousand-odd ways — 29 kHz apiece on a KiwiSDR — so a viewport
    // a few kilohertz wide pools down into one of them and draws as a flat line
    // straight across the panadapter. Whatever the I/Q has is worth more than
    // that, even where it does not cover the whole window; and a window this
    // narrow sitting outside the passband is a retune waiting to happen rather
    // than a picture to be drawn (`follow_view_into_the_band`).
    const MIN_BINS_ACROSS: f64 = 8.0;
    if (vp_hi - vp_lo) < wide_span / wide_bins as f64 * MIN_BINS_ACROSS {
        return false;
    }
    let (dev_lo, dev_hi) = (dev_center - dev_span / 2.0, dev_center + dev_span / 2.0);
    // A hair of tolerance: a viewport fitted to the passband is built as
    // `centre ± span/2` in one place and clamped in another, and the two round
    // differently — which would otherwise put a fully-zoomed-out view
    // permanently on the coarser lane.
    let eps = dev_span * 1e-6;
    vp_lo < dev_lo - eps || vp_hi > dev_hi + eps
}

#[cfg(test)]
mod wide_main_window_tests {

    /// A KiwiSDR: 12 kHz of I/Q at 10 MHz, and a 1024-bin picture of the whole
    /// 0-30 MHz — 29.3 kHz a bin.
    const KIWI_DEV: (f64, f64) = (10_000_000.0, 12_000.0);
    const KIWI_WIDE: (f64, f64) = (15_000_000.0, 30_000_000.0);
    const KIWI_WIDE_BINS: usize = 1024;

    /// [`wide_covers_viewport`] with this file's KiwiSDR figures.
    fn covers(vp: (f64, f64), dev: (f64, f64)) -> bool {
        super::wide_covers_viewport(vp, dev.0, dev.1, KIWI_WIDE, KIWI_WIDE_BINS)
    }

    /// Zoomed in, or fitted to the passband: the I/Q is finer and covers it.
    #[test]
    fn the_passband_keeps_its_own_picture() {
        let (c, span) = KIWI_DEV;
        assert!(!covers((c - 1_000.0, c + 1_000.0), (c, span)));
        assert!(
            !covers((c - span / 2.0, c + span / 2.0), (c, span)),
            "a view fitted exactly to the passband must not fall to the coarser lane"
        );
    }

    /// The rounding this tolerance exists for: `centre ± span/2` computed in
    /// two places, differing in the last bit.
    #[test]
    fn a_viewport_a_whisker_outside_still_counts_as_inside() {
        let (c, span) = KIWI_DEV;
        let nudge = span * 1e-9;
        assert!(!covers((c - span / 2.0 - nudge, c + span / 2.0 + nudge), (c, span)));
    }

    /// Zoomed out past the I/Q — the case the whole thing exists for.
    #[test]
    fn zooming_out_past_the_iq_reaches_the_wide_lane() {
        let (c, span) = KIWI_DEV;
        assert!(covers((c - 500_000.0, c + 500_000.0), (c, span)));
        assert!(covers((0.0, 30_000_000.0), (c, span)));
    }

    /// Panned off the edge rather than widened: the device analyser has nothing
    /// there either, so the test is containment and not width. Wide enough for
    /// the lane to resolve, which a window of a few kilohertz is not — see
    /// `a_window_too_narrow_for_the_lane_is_left_to_the_iq`.
    #[test]
    fn a_viewport_panned_off_the_edge_reaches_it_too() {
        let (c, span) = KIWI_DEV;
        let lo = c + 5e6;
        assert!(covers((lo, lo + 1e6), (c, span)));
    }

    /// The flat line this cost before it was caught: the receiver restates its
    /// sample rate every session — a KiwiSDR's arrives as 11998.876277 one
    /// time and 11998.876241 the next — so a view fitted to one of those is a
    /// fraction of a hertz wider than the other, and the client asked for a
    /// window twice the passband. The lane's bins are 29 kHz apiece, so a
    /// 24 kHz window pooled from them is a single bin drawn straight across the
    /// panadapter. Refused on resolution: whatever the I/Q has beats that.
    #[test]
    fn a_window_too_narrow_for_the_lane_is_left_to_the_iq() {
        let (c, span) = KIWI_DEV;
        // Twice the passband, centred on it — outside it at both ends.
        assert!(!covers((c - span, c + span), (c, span)));
        // Even far away from the passband, where the I/Q covers none of it.
        assert!(!covers((c + 5e6, c + 5e6 + span), (c, span)));
        // Eight of the lane's own bins is enough to draw.
        let bin = KIWI_WIDE.1 / KIWI_WIDE_BINS as f64;
        assert!(covers((c + 5e6, c + 5e6 + bin * 8.5), (c, span)));
    }

    /// A front end whose "wide" lane is no bigger than its I/Q has nothing to
    /// add, and switching to it would cost resolution for no reach.
    #[test]
    fn a_wide_lane_no_wider_than_the_iq_is_never_used() {
        let (c, span) = (10_000_000.0, 2_000_000.0);
        let narrow_lane = |wide: (f64, f64)| {
            super::wide_covers_viewport((c - 5e6, c + 5e6), c, span, wide, KIWI_WIDE_BINS)
        };
        assert!(!narrow_lane((c, 100_000.0)));
        assert!(!narrow_lane((c, span)));
    }

    /// A front end still coming up, before it has said what it streams.
    #[test]
    fn nothing_happens_before_the_passband_is_known() {
        assert!(!covers((0.0, 1e6), (0.0, 0.0)));
    }

    /// The lag case: the centre the samples were taken at is the one that
    /// decides, so a dial that has moved ahead of the stream does not flip the
    /// panadapter between the two sources.
    #[test]
    fn the_stream_centre_decides_not_the_commanded_one() {
        let span = 12_000.0;
        let stream_center = 10_000_000.0;
        // A viewport still sitting on the samples in hand.
        assert!(!covers((stream_center - 4_000.0, stream_center + 4_000.0), (stream_center, span)));
    }
}

#[cfg(test)]
mod wide_frame_tests {
    use super::*;

    #[test]
    fn pooling_preserves_a_narrow_carrier() {
        // One hot bin in four thousand must still be visible after being pooled
        // into two thousand — this is the whole reason for max-pooling.
        let mut db = vec![-120.0f32; 4096];
        db[1234] = -20.0;
        let f = pool_window_to_frame(&db, 1, 16.2e6, 32.4e6, -120.0, -20.0, WIDE_BINS, None);
        assert_eq!(f.bins.len(), WIDE_BINS);
        assert_eq!(f.bins[1234 * WIDE_BINS / 4096], 255);
        assert_eq!(f.bins[0], 0);
    }

    /// A rig's own scope is a fixed number of points, and a panadapter drawn
    /// wider than that is stretching them. Repeating each one is what made an
    /// IC-705's waterfall a wall of blocks; the ramp between two neighbours has
    /// to actually be a ramp.
    #[test]
    fn a_stretched_sweep_is_a_gradient_and_not_a_staircase() {
        // Eight points climbing evenly, drawn across 256 columns.
        let db: Vec<f32> = (0..8).map(|i| -120.0 + i as f32 * 10.0).collect();
        let f = pool_window_to_frame(&db, 1, 1e6, 1e6, -120.0, -40.0, 256, None);
        assert_eq!(f.bins.len(), 256);

        // Monotone, as the input is.
        assert!(f.bins.windows(2).all(|w| w[1] >= w[0]), "the ramp went backwards");

        // And it climbs continuously rather than in eight steps: between the
        // first and last source points every column differs from its neighbour
        // by at most a couple of levels, where replication would jump ~32 at
        // each of seven boundaries.
        let biggest = f.bins.windows(2).map(|w| w[1] as i32 - w[0] as i32).max().unwrap_or(0);
        assert!(biggest <= 3, "the largest step between columns was {biggest}");
    }

    /// The other direction is untouched: coarsening still keeps the peak, which
    /// is what stops a one-bin carrier vanishing from a zoomed-out view.
    #[test]
    fn stretching_does_not_change_how_a_wide_window_is_pooled() {
        let mut db = vec![-120.0f32; 4096];
        db[77] = -20.0;
        let f = pool_window_to_frame(&db, 1, 1e6, 1e6, -120.0, -20.0, 512, None);
        assert_eq!(f.bins[77 * 512 / 4096], 255);
    }

    #[test]
    fn the_frame_carries_the_axis_it_was_given() {
        let db = vec![-60.0f32; 1024];
        let f = pool_window_to_frame(&db, 7, 16.2e6, 32.4e6, -120.0, -20.0, 256, None);
        assert_eq!(f.seq, 7);
        assert_eq!(f.center_hz, 16.2e6);
        assert_eq!(f.span_hz, 32.4e6);
        // freq_at_bin should then map the ends onto DC and Nyquist.
        assert!(f.freq_at_bin(0) >= 0.0);
        assert!(f.freq_at_bin(255) <= 32.4e6);
    }

    #[test]
    fn levels_outside_the_window_clamp_rather_than_wrap() {
        let db = vec![40.0f32; 64];
        let hot = pool_window_to_frame(&db, 1, 0.0, 1.0, -120.0, -20.0, 8, None);
        assert!(hot.bins.iter().all(|b| *b == 255));
        let db = vec![-400.0f32; 64];
        let cold = pool_window_to_frame(&db, 1, 0.0, 1.0, -120.0, -20.0, 8, None);
        assert!(cold.bins.iter().all(|b| *b == 0));
    }

    #[test]
    fn fewer_input_bins_than_output_bins_still_produces_a_full_frame() {
        let db = vec![-50.0f32; 100];
        let f = pool_window_to_frame(&db, 1, 0.0, 1.0, -120.0, -20.0, WIDE_BINS, None);
        assert_eq!(f.bins.len(), WIDE_BINS);
        assert!(f.bins.iter().all(|b| *b > 0));
    }

    /// Zooming a scope-fed panadapter has to move the trace, not just relabel
    /// the axis: the frame describes the viewport, and the carrier inside it
    /// lands at the fraction of the *viewport* it occupies.
    #[test]
    fn a_viewport_extracts_the_sub_span_it_names() {
        // 100 kHz across 1000 bins, so 100 Hz a bin, with a carrier at +25 kHz.
        let mut db = vec![-120.0f32; 1000];
        db[750] = -20.0;
        let f = pool_window_to_frame(
            &db,
            1,
            14_000_000.0,
            100_000.0,
            -120.0,
            -20.0,
            256,
            Some((14_020_000.0, 14_030_000.0)),
        );
        assert!((f.center_hz - 14_025_000.0).abs() < 1.0, "centre: {}", f.center_hz);
        assert!((f.span_hz - 10_000.0).abs() < 1.0, "span: {}", f.span_hz);
        // Half way into a viewport running 14.020–14.030 MHz.
        let peak = f.bins.iter().enumerate().max_by_key(|(_, v)| **v).map(|(i, _)| i).unwrap();
        assert!(peak.abs_diff(128) <= 4, "carrier at bin {peak} of 256");
    }

    /// A source that has published no sweep yet must not take the frame builder
    /// down with it.
    #[test]
    fn an_empty_sweep_still_produces_a_frame() {
        let f = pool_window_to_frame(&[], 1, 0.0, 1.0, -120.0, -20.0, 8, Some((0.0, 0.5)));
        assert_eq!(f.bins, vec![0u8; 8]);
    }
}

/// Choose the dB window that shows everything in a full-band frame.
///
/// The main panadapter's window is set by the operator, which is right for a
/// slice they are working in. It is wrong for a strip covering the whole of HF:
/// the difference between a quiet 10 m and a crowded broadcast band is 60 dB or
/// more, and any fixed pair of numbers leaves one end of the band either black
/// or saturated.
///
/// The floor tracks a low percentile rather than the minimum, so a handful of
/// dead bins cannot drag it down. The ceiling tracks the strongest signal,
/// because a scale that clips the loudest carrier in the band is not showing
/// "all signal strengths" — but the *DC region is excluded first*, since a
/// direct-sampling ADC parks a large offset spike at bin zero and letting that
/// set the scale would push the entire band to black.
///
/// Both ends are then smoothed towards their new values: jumping straight there
/// makes the display flicker every time a signal keys up.
fn auto_levels(db: &[f32], prev: Option<(f32, f32)>) -> (f32, f32) {
    const FALLBACK: (f32, f32) = (-120.0, -20.0);
    /// Widest window worth showing; beyond this the interesting part of the
    /// band gets too few levels to distinguish.
    const MAX_RANGE: f32 = 120.0;

    // Drop the DC region: on a direct-sampling front end bin 0 carries the
    // ADC's offset, which is tens of dB above everything else and is not a
    // signal.
    let skip = (db.len() / 512).max(1);
    let usable = db.get(skip..).unwrap_or(&[]);
    let mut v: Vec<f32> = usable.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return prev.unwrap_or(FALLBACK);
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // A few dB below the noise floor, so the floor itself reads as texture
    // rather than as flat black.
    let mut floor = v[((v.len() - 1) as f64 * 0.10).round() as usize] - 4.0;
    let mut ceil = v[v.len() - 1] + 6.0;

    // Never let the window collapse: a degenerate range maps everything to one
    // colour and looks like a dead receiver.
    if ceil - floor < 24.0 {
        let mid = 0.5 * (ceil + floor);
        floor = mid - 12.0;
        ceil = mid + 12.0;
    }
    if ceil - floor > MAX_RANGE {
        floor = ceil - MAX_RANGE;
    }

    match prev {
        None => (floor, ceil),
        Some((pf, pc)) => {
            // Rise quickly so a band opening is not clipped, fall slowly so the
            // scale does not pump between frames.
            let smooth = |old: f32, new: f32| {
                let a = if new > old { 0.35 } else { 0.08 };
                old + (new - old) * a
            };
            (smooth(pf, floor), smooth(pc, ceil))
        }
    }
}

#[cfg(test)]
mod auto_level_tests {
    use super::*;

    fn band(noise: f32, signals: &[(usize, f32)], n: usize) -> Vec<f32> {
        let mut v = vec![noise; n];
        for (i, db) in signals {
            v[*i] = *db;
        }
        v
    }

    #[test]
    fn the_window_brackets_the_signals_present() {
        let db = band(-110.0, &[(100, -30.0), (500, -45.0)], 4096);
        let (floor, ceil) = auto_levels(&db, None);
        assert!(floor < -110.0, "floor {floor} should sit below the noise");
        assert!(ceil > -30.0, "ceiling {ceil} should clear the strongest signal");
    }

    #[test]
    fn one_dead_bin_does_not_drag_the_floor_down() {
        let mut db = band(-100.0, &[(7, -20.0)], 4096);
        db[0] = -400.0; // a single pathological bin
        let (floor, _) = auto_levels(&db, None);
        assert!(floor > -130.0, "a single dead bin moved the floor to {floor}");
    }

    #[test]
    fn the_ceiling_accommodates_the_loudest_real_signal() {
        // A 0 dBFS carrier is a real signal, and a scale that clips it is not
        // showing all signal strengths.
        let db = band(-100.0, &[(2000, 0.0)], 4096);
        let (_, ceil) = auto_levels(&db, None);
        assert!(ceil >= 0.0, "ceiling {ceil} clips the strongest carrier");
    }

    /// The DC bin on a direct-sampling ADC carries the converter's offset, not
    /// a signal. Letting it set the ceiling pushes the whole band to black —
    /// this receiver really does sit ~65 counts off zero.
    #[test]
    fn the_dc_offset_spike_does_not_set_the_scale() {
        let mut db = band(-100.0, &[(3000, -40.0)], 4096);
        db[0] = 0.0;
        db[1] = -10.0;
        let (_, ceil) = auto_levels(&db, None);
        assert!(ceil < -20.0, "the DC spike set the ceiling to {ceil}");
    }

    #[test]
    fn the_window_never_grows_past_what_is_readable() {
        let db = band(-200.0, &[(9, 0.0)], 4096);
        let (floor, ceil) = auto_levels(&db, None);
        assert!(ceil - floor <= 120.0 + 1.0, "window is {:.0} dB wide", ceil - floor);
    }

    #[test]
    fn a_flat_band_still_gets_a_usable_window() {
        let db = vec![-95.0f32; 4096];
        let (floor, ceil) = auto_levels(&db, None);
        assert!(ceil - floor >= 24.0, "window collapsed to {:.1} dB", ceil - floor);
    }

    #[test]
    fn levels_are_smoothed_rather_than_jumping() {
        let quiet = vec![-120.0f32; 4096];
        let loud = band(-60.0, &[(10, -10.0)], 4096);
        let first = auto_levels(&quiet, None);
        let second = auto_levels(&loud, Some(first));
        // It moves towards the new scale but does not arrive in one frame.
        let target = auto_levels(&loud, None);
        assert!(second.1 > first.1, "ceiling did not rise");
        assert!(second.1 < target.1, "ceiling jumped straight to the target");
    }

    #[test]
    fn an_empty_frame_keeps_the_previous_window() {
        let prev = (-115.0, -25.0);
        assert_eq!(auto_levels(&[], Some(prev)), prev);
        assert_eq!(auto_levels(&[f32::NAN, f32::NAN], Some(prev)), prev);
    }
}

#[cfg(test)]
mod hop_div_tests {
    use super::{MAX_HOP_DIV, hop_div_for};

    /// The case this rule exists for: a wide front end runs far more transforms
    /// than any waterfall can draw, and half-overlapping them doubles that for
    /// nothing. An RX-888 at 8.1 MHz through a 4096-point window makes 3955 a
    /// second at the customary half-hop; a waterfall wants at most a couple of
    /// hundred.
    #[test]
    fn a_wide_front_end_stops_overlapping() {
        assert_eq!(hop_div_for(8_100_000.0, 4096, 28.0), 1);
        assert_eq!(hop_div_for(8_100_000.0, 4096, 224.0), 1);
    }

    /// It is a rule, not a switch: a rate that produces only a handful of
    /// transforms per row keeps its overlap. 2 Msps through a 32768-point
    /// window is 122 transforms a second, which at 28 rows is already about
    /// the four per row that is wanted — so nothing is taken away.
    #[test]
    fn a_lane_with_transforms_to_spare_only_just_keeps_its_overlap() {
        assert_eq!(hop_div_for(2_000_000.0, 32_768, 28.0), 2);
        // Scroll faster on the same lane and the overlap has to come back.
        assert_eq!(hop_div_for(2_000_000.0, 32_768, 224.0), 8);
    }

    /// And the case it must not break: a zoomed lane at a few kilohertz fills a
    /// window barely twice a second, and its eighth-hop is what puts rows on
    /// the waterfall at all. The rule has to hand that back unchanged.
    #[test]
    fn a_zoomed_lane_keeps_its_fine_overlap() {
        // The lane a 10 kHz view on an 8.1 Msps front end builds: 8.1e6/512.
        assert_eq!(hop_div_for(15_820.0, 4096, 28.0), MAX_HOP_DIV);
        assert_eq!(hop_div_for(48_000.0, 4096, 56.0), MAX_HOP_DIV);
    }

    /// Never coarser than one window per hop. Past that the analyser would skip
    /// samples outright, and a signal shorter than a window could land entirely
    /// in the gap — which is exactly what the peak hold between rows exists to
    /// prevent.
    #[test]
    fn it_never_skips_samples() {
        for rate in [48_000.0, 1_536_000.0, 8_100_000.0, 64_000_000.0] {
            for fft in [1024usize, 4096, 32_768, 131_072] {
                for rows in [5.0, 28.0, 224.0] {
                    let d = hop_div_for(rate, fft, rows);
                    assert!((1..=MAX_HOP_DIV).contains(&d), "{rate}/{fft}/{rows} gave {d}");
                }
            }
        }
    }

    /// A rate or a scroll speed nobody has filled in yet falls back to the
    /// half-overlap every build before this one used.
    #[test]
    fn an_unknown_rate_keeps_the_old_behaviour() {
        assert_eq!(hop_div_for(0.0, 4096, 28.0), 2);
        assert_eq!(hop_div_for(f64::NAN, 4096, 28.0), 2);
        assert_eq!(hop_div_for(8_100_000.0, 4096, 0.0), 2);
    }
}

#[cfg(test)]
mod collapse_tests {
    use super::collapse_superseded;
    use sdroxide_types::{Command, Vfo};

    fn kinds(batch: &[Command]) -> Vec<String> {
        batch
            .iter()
            .map(|c| match c {
                Command::SetCenter(hz) => format!("C{hz}"),
                Command::SetVfo { vfo, hz } => format!("V{vfo:?}{hz}"),
                other => format!("{other:?}"),
            })
            .collect()
    }

    fn collapse(mut batch: Vec<Command>) -> Vec<String> {
        collapse_superseded(&mut batch);
        kinds(&batch)
    }

    /// What a drag delivers: one centre and one dial per frame of the UI, of
    /// which only the last pair is a state anything ever sees. Sixty retunes a
    /// second is what issue #188's remote client was asking a Pi for.
    #[test]
    fn a_drags_worth_of_frames_costs_one_retune() {
        let drag: Vec<Command> = (0..4)
            .flat_map(|i| {
                let hz = 14_100_000.0 + f64::from(i) * 1000.0;
                [Command::SetCenter(hz), Command::SetVfo { vfo: Vfo::A, hz }]
            })
            .collect();
        assert_eq!(collapse(drag), ["C14103000", "VA14103000"]);
    }

    /// The two VFOs are separate settings, and each keeps its own last word.
    #[test]
    fn each_vfo_keeps_its_own_last_value() {
        assert_eq!(
            collapse(vec![
                Command::SetVfo { vfo: Vfo::A, hz: 1.0 },
                Command::SetVfo { vfo: Vfo::B, hz: 2.0 },
                Command::SetVfo { vfo: Vfo::A, hz: 3.0 },
            ]),
            ["VB2", "VA3"]
        );
    }

    /// The rule that keeps this honest: a command that could *read* the centre
    /// or the dial is a wall, and nothing before it is dropped across it.
    /// `CopyAtoB` is the one that proves it — collapsing the two `SetVfo`s
    /// around it would copy a VFO that never held the value being copied.
    #[test]
    fn a_command_that_reads_the_dial_is_a_wall() {
        assert_eq!(
            collapse(vec![
                Command::SetVfo { vfo: Vfo::A, hz: 100.0 },
                Command::CopyAtoB,
                Command::SetVfo { vfo: Vfo::A, hz: 200.0 },
            ]),
            ["VA100", "CopyAtoB", "VA200"]
        );
        // ...and the setters *after* the wall still collapse among themselves.
        assert_eq!(
            collapse(vec![
                Command::SetCenter(1.0),
                Command::SwapVfos,
                Command::SetCenter(2.0),
                Command::SetCenter(3.0),
            ]),
            ["C1", "SwapVfos", "C3"]
        );
    }

    /// Nothing to collapse must change nothing — including the ordinary case of
    /// a single command arriving on its own.
    #[test]
    fn a_batch_with_no_repeats_is_left_alone() {
        assert_eq!(collapse(vec![]), Vec::<String>::new());
        assert_eq!(collapse(vec![Command::SetCenter(1.0)]), ["C1"]);
        assert_eq!(
            collapse(vec![Command::SetPtt(true), Command::SetCenter(1.0)]),
            ["SetPtt(true)", "C1"]
        );
    }
}

#[cfg(test)]
mod tci_pace_tests {
    use super::{TCI_TX_ASK_TIMEOUT_BLOCKS, TCI_TX_LEAD, TX_AUDIO_BLOCK, TciTxPace};

    /// A client that answers every chrono at once is asked for exactly what the
    /// transmitter consumes, and never twice for the same audio.
    #[test]
    fn a_prompt_client_is_asked_for_what_the_transmitter_consumes() {
        let mut p = TciTxPace::default();
        // Key-down on an empty queue: the lead, plus the block this one is
        // about to consume, once.
        let first = TCI_TX_LEAD + TX_AUDIO_BLOCK;
        assert_eq!(p.request(0, 0, true), Some(first as u32));
        // The next block, with nothing yet arrived, asks for one block — the
        // one it will consume — and not for the lead again.
        assert_eq!(p.request(0, 0, true), Some(TX_AUDIO_BLOCK as u32));
        // It all lands and two blocks have been played, so the queue stands at
        // the lead; one block is asked for, once per block, from here on.
        assert_eq!(
            p.request(TCI_TX_LEAD, first + TX_AUDIO_BLOCK, true),
            Some(TX_AUDIO_BLOCK as u32),
        );
    }

    /// The bug behind the second half of issue #202: pacing towards a standing
    /// queue depth pins the ask rate to the arrival rate, so a client that has
    /// once fallen behind is asked for exactly what it manages to send and can
    /// never catch up. Measured against WSJT-X: 55 % of a 13.6 s FT8 slot.
    ///
    /// The transmit clock is what fixes it — a dry queue asks for *more*.
    #[test]
    fn a_client_that_has_fallen_behind_is_asked_for_real_time_not_for_what_it_sent() {
        let mut p = TciTxPace::default();
        // Half-rate client: one block of audio for every two blocks played.
        let mut asked = 0usize;
        let mut supplied = 0usize;
        for i in 0..200 {
            let got = if i % 2 == 0 { TX_AUDIO_BLOCK } else { 0 };
            supplied += got;
            // The queue never builds: everything that arrives goes straight out.
            asked += p.request(0, got, true).unwrap_or(0) as usize;
        }
        // Over 200 blocks the transmitter consumed 200 blocks, so that — plus
        // the standing lead — is what the client must have been asked for. A
        // loop pinned to the arrival rate would have asked for half of it.
        let want = 200 * TX_AUDIO_BLOCK + TCI_TX_LEAD;
        assert_eq!(asked, want, "asked for {asked} frames of a {want}-frame over");
        assert!(asked > supplied * 2 - TX_AUDIO_BLOCK, "the shortfall must be asked for again");
    }

    /// …and a client that keeps up is never asked for more than the queue can
    /// hold: the running total is bounded by what the transmitter has used.
    #[test]
    fn a_client_that_keeps_up_is_never_asked_for_more_than_the_lead() {
        let mut p = TciTxPace::default();
        let mut queued = 0usize;
        // A client that answers every chrono in full, one block later.
        let mut in_flight = 0usize;
        for _ in 0..500 {
            let got = std::mem::take(&mut in_flight);
            queued += got;
            in_flight = p.request(queued, got, true).unwrap_or(0) as usize;
            queued = queued.saturating_sub(TX_AUDIO_BLOCK);
            assert!(
                queued <= TCI_TX_LEAD + TX_AUDIO_BLOCK,
                "the queue reached {queued}, past the {TCI_TX_LEAD}-frame lead"
            );
        }
        assert!(queued >= TCI_TX_LEAD - TX_AUDIO_BLOCK, "the queue drained to {queued}");
    }

    /// A chrono the client simply dropped must not silence the rest of the
    /// over: after a spell of complete silence what is outstanding is written
    /// off and asked for again.
    #[test]
    fn an_unanswered_chrono_is_eventually_asked_for_again() {
        let mut p = TciTxPace::default();
        assert_eq!(p.request(0, 0, false), Some(TCI_TX_LEAD as u32));
        // Nothing arrived on the block that made the request either, so that
        // one counts towards the silence as well. `playing` is false so the
        // transmit clock cannot be what asks again — only the write-off can.
        let mut quiet_blocks = 1;
        let again = loop {
            quiet_blocks += 1;
            assert!(quiet_blocks < 500, "the outstanding request was never written off");
            if let Some(frames) = p.request(0, 0, false) {
                break frames;
            }
        };
        assert_eq!(again, TCI_TX_LEAD as u32);
        assert_eq!(
            quiet_blocks, TCI_TX_ASK_TIMEOUT_BLOCKS,
            "a client that has said nothing at all is asked again, but only after \
             longer than any answer could take"
        );
    }

    /// The pre-roll spins far faster than the transmit clock, so its calls must
    /// not count as consumption — a hundred of them before the first block goes
    /// out would otherwise ask for a second of audio that nothing has used.
    #[test]
    fn the_pre_roll_does_not_run_the_demand_away() {
        let mut p = TciTxPace::default();
        let mut asked = p.request(0, 0, false).unwrap_or(0) as usize;
        for _ in 0..(TCI_TX_ASK_TIMEOUT_BLOCKS - 2) {
            asked += p.request(0, 0, false).unwrap_or(0) as usize;
        }
        assert_eq!(asked, TCI_TX_LEAD, "the pre-roll asked for {asked} frames, not the lead");
    }

    /// A client that streams on its own clock and ignores chronos fills the
    /// queue by itself; there is nothing to ask it for.
    #[test]
    fn a_self_pacing_client_is_left_alone() {
        let mut p = TciTxPace::default();
        assert_eq!(p.request(TCI_TX_LEAD * 3, TCI_TX_LEAD * 3, true), None);
    }

    /// A new over starts from nothing: last over's counters would make the
    /// first block of this one look like a client hundreds of frames in debt.
    #[test]
    fn a_new_over_starts_from_nothing() {
        let mut p = TciTxPace::default();
        for _ in 0..100 {
            p.request(0, TX_AUDIO_BLOCK, true);
        }
        p.rekey();
        assert_eq!(p.request(0, 0, true), Some((TCI_TX_LEAD + TX_AUDIO_BLOCK) as u32));
    }
}
