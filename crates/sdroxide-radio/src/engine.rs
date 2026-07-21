//! `RadioCore`: the engine thread that owns the IQ source, all DSP, and the
//! authoritative [`RadioState`].
//!
//! M4 scope: main + sub receiver chains mixed to stereo (main left, sub
//! right), all demodulators, band-stack registers, memory channels
//! (persisted engine-side), hardware gain/antenna control, and
//! viewport-aware spectrum frames. TX arrives in M5.

use std::time::{Duration, Instant, SystemTime};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use tracing::{info, warn};

use sdroxide_config::BandStacks;
use sdroxide_digi::{DigiAction, DigiController};
use sdroxide_skimmer::{SkimmerAction, SkimmerController};
use sdroxide_dsp::{
    Agc, DcBlock, Ddc, Demodulator, Duc, Modulator, MonoResampler, NoiseBlanker,
    SpectrumAnalyzer, channel_target, make_demod, make_modulator,
};
use sdroxide_types::{
    Band, BandStackEntry, Command, DeviceCaps, DigiConfig, Direction, MemoryChannel, Meters,
    Mode, RadioEvent, RadioState, RxId, RxState, SpectrumConfig, SpectrumFrame, TxMeters, Vfo,
};

use crate::{Complex32, IqSource};

/// Number of bins in emitted display frames (matches the waterfall texture width).
pub const DISPLAY_BINS: usize = 2048;

pub struct EngineHandles {
    pub cmd_tx: Sender<Command>,
    pub event_rx: Receiver<RadioEvent>,
    pub spectrum_out: triple_buffer::Output<SpectrumFrame>,
    /// Join before process exit so device teardown (SoapySDR/libusb) can't
    /// race the C libraries' own exit handlers.
    pub thread: Option<std::thread::JoinHandle<()>>,
}

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
    /// Refuse to key up outside amateur bands.
    pub tx_ham_only: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            audio: None,
            mic: None,
            cal_offset_db: 0.0,
            initial_mode: None,
            tx_ham_only: true,
        }
    }
}

/// Spawn the engine thread. It runs until the last command sender is dropped
/// or the source fails.
pub fn start(source: Box<dyn IqSource>, caps: DeviceCaps, cfg: EngineConfig) -> EngineHandles {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    let empty = SpectrumFrame {
        seq: 0,
        center_hz: 0.0,
        span_hz: 0.0,
        db_floor: 0.0,
        db_ceil: 0.0,
        bins: Vec::new(),
    };
    let (spec_in, spectrum_out) = triple_buffer::triple_buffer(&empty);

    let thread = std::thread::Builder::new()
        .name("sdroxide-dsp".into())
        .spawn(move || engine_thread(source, caps, cfg, cmd_rx, event_tx, spec_in))
        .expect("spawn dsp thread");

    EngineHandles { cmd_tx, event_rx, spectrum_out, thread: Some(thread) }
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
    /// When true, `tap_out` receives a copy of the post-AGC, pre-volume audio
    /// for the digital-mode decoder (independent of mute/volume/squelch).
    tap_enabled: bool,
    tap_out: Vec<f32>,
    channel_buf: Vec<Complex32>,
    audio_buf: Vec<f32>,
    out_buf: Vec<f32>,
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
            tap_enabled: false,
            tap_out: Vec::new(),
            channel_buf: Vec::new(),
            audio_buf: Vec::new(),
            out_buf: Vec::new(),
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
        self.demod = make_demod(rx.mode, self.ddc.out_rate());
        if let Some(d) = self.demod.as_mut() {
            d.set_filter(rx.filter_lo, rx.filter_hi);
        }
        let audio_rate = self
            .demod
            .as_ref()
            .map(|d| d.audio_rate())
            .unwrap_or_else(|| self.ddc.out_rate());
        self.agc = Agc::new(audio_rate);
        self.agc.set_mode(rx.agc);
        self.agc.set_max_gain_db(rx.agc_max_gain_db);
        self.resampler = MonoResampler::new(audio_rate, self.out_rate);
    }

    fn set_offset_hz(&mut self, hz: f64) {
        self.offset_hz = hz;
        self.ddc.set_offset_hz(hz);
    }

    /// Process a device-rate block; the returned slice is audio at
    /// `out_rate` (empty when this chain produces no audio, e.g. SPEC).
    fn run(&mut self, iq: &[Complex32], rx: &RxState) -> &[f32] {
        self.out_buf.clear();
        let Some(demod) = self.demod.as_mut() else {
            return &self.out_buf;
        };

        self.channel_buf.clear();
        self.ddc.process(iq, &mut self.channel_buf);

        self.audio_buf.clear();
        demod.process(&self.channel_buf, &mut self.audio_buf);
        self.agc.process(&mut self.audio_buf);

        // Tap the clean, post-AGC audio before volume/mute/squelch so the
        // FT8/FT4 decoder isn't starved by the operator's listening choices.
        if self.tap_enabled {
            self.tap_out.clear();
            self.tap_out.extend_from_slice(&self.audio_buf);
        }

        // Squelch: gate on post-filter (pre-AGC) power, smoothed ~10 ms so
        // opening and closing don't click.
        let open = demod.power_dbfs() >= rx.squelch_db;
        let sq_target = if open { 1.0 } else { 0.0 };
        let vol = if rx.muted { 0.0 } else { rx.volume * rx.volume };
        for s in &mut self.audio_buf {
            self.sq_gain += (sq_target - self.sq_gain) * 0.002;
            *s *= vol * self.sq_gain;
        }

        match &mut self.resampler {
            Some(r) => r.push(&self.audio_buf, &mut self.out_buf),
            None => self.out_buf.extend_from_slice(&self.audio_buf),
        }
        // Clamp after resampling so interpolation overshoot can't escape.
        for s in &mut self.out_buf {
            *s = s.clamp(-1.0, 1.0);
        }
        &self.out_buf
    }

    fn power_dbfs(&self) -> Option<f32> {
        self.demod.as_ref().map(|d| d.power_dbfs())
    }
}

/// Interleaves main/sub audio into the stereo ring. When the sub receiver is
/// active, main goes left and sub right; otherwise main goes to both ears.
struct StereoMixer {
    out: rtrb::Producer<f32>,
    main_q: Vec<f32>,
    sub_q: Vec<f32>,
    dropped: u64,
}

/// Bound on per-channel queueing (≈¼ s at 48 kHz) so a stalled side can't
/// grow the other without limit.
const MIXER_CAP: usize = 12_000;

impl StereoMixer {
    fn new(out: rtrb::Producer<f32>) -> Self {
        StereoMixer { out, main_q: Vec::new(), sub_q: Vec::new(), dropped: 0 }
    }

    fn push(&mut self, main: &[f32], sub: Option<&[f32]>) {
        self.main_q.extend_from_slice(main);
        let dual = match sub {
            Some(s) => {
                self.sub_q.extend_from_slice(s);
                true
            }
            None => {
                self.sub_q.clear();
                false
            }
        };

        let n = if dual { self.main_q.len().min(self.sub_q.len()) } else { self.main_q.len() };
        if n > 0 {
            if self.out.slots() >= n * 2 {
                for i in 0..n {
                    let l = self.main_q[i];
                    let r = if dual { self.sub_q[i] } else { l };
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
    }
}

/// The transmit chain: mic 48 k → modulator → drive → DUC → device.
struct TxChain {
    modulator: Option<Box<dyn Modulator>>,
    dc: DcBlock,
    duc: Duc,
    mod_buf: Vec<Complex32>,
    tx_buf: Vec<Complex32>,
    alc_peak: f32,
}

/// 10 ms of TX audio per iteration.
const TX_AUDIO_BLOCK: usize = 480;

impl TxChain {
    fn new(mode: Mode, tx_rate: f64) -> Self {
        TxChain {
            modulator: make_modulator(mode, 48_000.0),
            dc: DcBlock::new(100.0, 48_000.0),
            duc: Duc::new(48_000.0, tx_rate),
            mod_buf: Vec::new(),
            tx_buf: Vec::new(),
            alc_peak: 0.0,
        }
    }
}

struct Engine {
    source: Box<dyn IqSource>,
    caps: DeviceCaps,
    state: RadioState,
    cfg: SpectrumConfig,
    analyzer: SpectrumAnalyzer,
    event_tx: Sender<RadioEvent>,
    main: Option<RxChain>,
    sub: Option<RxChain>,
    mixer: Option<StereoMixer>,
    audio_out_rate: f64,
    cal_offset_db: f32,
    stacks: BandStacks,
    memories: Vec<MemoryChannel>,
    mic: Option<MicParams>,
    mic_resampler: Option<MonoResampler>,
    mic_fifo: Vec<f32>,
    tx: Option<TxChain>,
    tx_active: bool,
    tx_center_hz: f64,
    tx_ham_only: bool,
    nb: NoiseBlanker,
    /// FT8/FT4 controller, present only while a digital mode is active.
    digi: Option<DigiController>,
    digi_config: DigiConfig,
    /// True while the current TX burst is driven by the digi engine.
    digi_tx: bool,
    /// High-resolution spectrum over the VFO channel (digital modes only):
    /// fed the decimated channel IQ so an FFT gives ~3 Hz/bin resolution.
    channel_analyzer: Option<SpectrumAnalyzer>,
    /// CW skimmer: a dedicated wideband decimator off the raw IQ plus a
    /// worker-thread decoder, present only while the skimmer is enabled.
    skim_ddc: Option<Ddc>,
    skimmer: Option<SkimmerController>,
    skim_buf: Vec<Complex32>,
}

/// Target width of the CW skimmer window (Hz); the Ddc snaps to the nearest
/// integer decimation of the device rate.
const SKIM_TARGET_HZ: f64 = 192_000.0;

fn engine_thread(
    source: Box<dyn IqSource>,
    caps: DeviceCaps,
    engine_cfg: EngineConfig,
    cmd_rx: Receiver<Command>,
    event_tx: Sender<RadioEvent>,
    mut spec_in: triple_buffer::Input<SpectrumFrame>,
) {
    let mut state = RadioState::default();
    state.center_hz = source.center_hz();
    state.sample_rate = source.sample_rate();
    state.vfo_a_hz = source.center_hz();
    state.vfo_b_hz = source.center_hz();
    state.band = Band::containing(state.vfo_a_hz);
    state.gains = source.current_gains();
    state.tx_gains = source.current_tx_gains();
    state.antenna_rx = source.current_antenna();
    if let Some(mode) = engine_cfg.initial_mode {
        for rx in &mut state.rx {
            *rx = RxState::with_mode(mode);
        }
    }

    let cfg = SpectrumConfig::default();
    let analyzer = SpectrumAnalyzer::new(cfg.fft_size as usize, state.sample_rate, cfg.avg_tc);

    let (main, mixer, audio_out_rate) = match engine_cfg.audio {
        Some(audio) => {
            let chain = RxChain::new(state.sample_rate, &state.rx[0], audio.out_rate);
            info!(channel_rate = chain.ddc.out_rate(), out_rate = audio.out_rate, "audio chain up");
            (Some(chain), Some(StereoMixer::new(audio.producer)), audio.out_rate)
        }
        None => (None, None, 48_000.0),
    };

    let memories = sdroxide_config::load_memories();
    let stacks = sdroxide_config::load_bandstacks();
    let digi_config = sdroxide_config::load_digi_config();

    info!(source = %source.describe(), "engine started");
    let _ = event_tx.send(RadioEvent::Capabilities(caps.clone()));
    let _ = event_tx.send(RadioEvent::State(state.clone()));
    let _ = event_tx.send(RadioEvent::Memories(memories.clone()));

    let mut engine = Engine {
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
        cal_offset_db: engine_cfg.cal_offset_db,
        stacks,
        memories,
        mic: engine_cfg.mic,
        mic_resampler: None,
        mic_fifo: Vec::new(),
        tx: None,
        tx_active: false,
        tx_center_hz: 0.0,
        tx_ham_only: engine_cfg.tx_ham_only,
        nb: NoiseBlanker::new(),
        digi: None,
        digi_config,
        digi_tx: false,
        channel_analyzer: None,
        skim_ddc: None,
        skimmer: None,
        skim_buf: Vec::new(),
    };
    if let Some(mic) = &engine.mic {
        engine.mic_resampler = MonoResampler::new(mic.rate, 48_000.0);
    }
    // If we start up already in a digital mode, spin up the controller.
    engine.sync_digi_mode();
    engine.update_tuning();

    let mut buf = vec![Complex32::default(); 16_384];
    let mut next_frame = Instant::now();
    let mut next_meters = Instant::now();

    loop {
        loop {
            match cmd_rx.try_recv() {
                Ok(cmd) => engine.apply(cmd),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if engine.tx_active {
                        let _ = engine.source.tx_end();
                    }
                    info!("all controllers gone; engine stopping");
                    return;
                }
            }
        }

        // Drive the FT8/FT4 slot machine (runs in both RX and TX). Returns
        // owned actions to avoid borrowing `engine.digi` and `engine` at once.
        engine.poll_digi();
        engine.poll_skimmer();

        if engine.tx_active {
            // Blocking TX write paces this loop at ~10 ms per block.
            if let Err(e) = engine.tx_block() {
                let _ = engine.event_tx.send(RadioEvent::ConnectionLost(e.to_string()));
                return;
            }
            // Full-duplex hardware keeps receiving during TX.
            if engine.caps.full_duplex {
                if let Ok(n @ 1..) = engine.source.read(&mut buf) {
                    engine.run_audio(&buf[..n]);
                }
            }
        } else {
            match engine.source.read(&mut buf) {
                Ok(0) => continue, // timeout
                Ok(n) => {
                    if engine.state.noise_blanker {
                        engine.nb.process(&mut buf[..n]);
                    }
                    engine.analyzer.process(&buf[..n]);
                    engine.run_audio(&buf[..n]);
                }
                Err(e) => {
                    let _ = engine.event_tx.send(RadioEvent::ConnectionLost(e.to_string()));
                    return;
                }
            }
        }

        let now = Instant::now();
        if now >= next_frame {
            next_frame = now + Duration::from_secs_f64(1.0 / engine.cfg.fps.max(1) as f64);
            spec_in.write(engine.make_spectrum_frame());
        }
        if now >= next_meters {
            next_meters = now + Duration::from_millis(100);
            let meters = if engine.tx_active {
                let alc = engine.tx.as_ref().map(|t| t.alc_peak).unwrap_or(0.0);
                Some(Meters {
                    s_dbm: -127.0,
                    adc_peak_dbfs: 0.0,
                    // No power/SWR sensors on HackRF; expose drive-side ALC.
                    tx: Some(TxMeters { fwd_w: None, swr: None, alc }),
                })
            } else {
                engine.main.as_ref().and_then(|c| c.power_dbfs()).map(|p| Meters {
                    s_dbm: p + engine.cal_offset_db,
                    adc_peak_dbfs: 0.0,
                    tx: None,
                })
            };
            if let Some(m) = meters {
                let _ = engine.event_tx.send(RadioEvent::Meters(m));
            }
        }
    }
}

impl Engine {
    fn run_audio(&mut self, iq: &[Complex32]) {
        let Some(main) = self.main.as_mut() else { return };
        let main_audio = main.run(iq, &self.state.rx[0]);

        let sub_audio: Option<&[f32]> = match (&mut self.sub, self.state.sub_rx_enabled) {
            (Some(sub), true) => {
                // A silent sub (SPEC) degrades to mono rather than stalling.
                let has_audio = sub.demod.is_some();
                let a = sub.run(iq, &self.state.rx[1]);
                has_audio.then_some(a)
            }
            _ => None,
        };

        if let Some(mixer) = self.mixer.as_mut() {
            mixer.push(main_audio, sub_audio);
        }

        // Feed the digital-mode decoder from the clean tap (not the mixed,
        // possibly-muted output).
        if let (Some(digi), Some(main)) = (self.digi.as_mut(), self.main.as_ref()) {
            if main.tap_enabled {
                digi.on_rx_audio(&main.tap_out);
            }
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
    }

    /// Tick the FT8/FT4 controller and apply its actions (emit events, key/
    /// unkey PTT). Owned actions avoid a `&mut self.digi` / `&mut self` clash.
    fn poll_digi(&mut self) {
        let Some(digi) = self.digi.as_mut() else { return };
        let dial = self.state.rx_freq_hz();
        let actions = digi.poll(SystemTime::now(), dial);
        for a in actions {
            match a {
                DigiAction::Decodes(d) => {
                    let _ = self.event_tx.send(RadioEvent::Ft8Decodes(d));
                }
                DigiAction::Status(s) => {
                    let _ = self.event_tx.send(RadioEvent::Ft8Status(s));
                }
                DigiAction::QsoLogged(r) => {
                    let _ = self.event_tx.send(RadioEvent::Ft8QsoLogged(r));
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
            }
        }
    }

    /// Construct or tear down the digi controller to match the current mode.
    fn sync_digi_mode(&mut self) {
        let mode = self.state.rx[0].mode;
        let want = mode.is_digital();
        let have = self.digi.is_some();
        if want && !have {
            let tap_rate = self.main.as_ref().map(|c| c.audio_rate()).unwrap_or(48_000.0);
            self.digi = Some(DigiController::new(mode, self.digi_config.clone(), tap_rate));
            if let Some(c) = self.main.as_mut() {
                c.tap_enabled = true;
            }
            // High-resolution channel spectrum: 16k-point FFT over the ~50 kHz
            // channel ≈ 3 Hz/bin, enough to resolve 6.25 Hz FT8 tones.
            let ch_rate = self.main.as_ref().map(|c| c.channel_rate()).unwrap_or(48_000.0);
            self.channel_analyzer = Some(SpectrumAnalyzer::new(16_384, ch_rate, 0.10));
            info!(?mode, ch_rate, "FT8/FT4 engine started");
        } else if want && have {
            // Mode changed between Ft8/Ft4: rebuild for the new one.
            if self.digi.as_ref().map(|d| d.mode()) != Some(mode) {
                let tap_rate = self.main.as_ref().map(|c| c.audio_rate()).unwrap_or(48_000.0);
                self.digi = Some(DigiController::new(mode, self.digi_config.clone(), tap_rate));
            }
        } else if !want && have {
            if let Some(d) = self.digi.as_mut() {
                d.abort();
            }
            // Kill any digi-driven transmission.
            if self.digi_tx || self.state.tx.ptt {
                self.state.tx.ptt = false;
                self.digi_tx = false;
                self.sync_tx_state();
            }
            self.digi = None;
            self.channel_analyzer = None;
            if let Some(c) = self.main.as_mut() {
                c.tap_enabled = false;
                c.tap_out.clear();
            }
            info!("FT8/FT4 engine stopped");
        }
    }

    /// Build the display spectrum frame. In digital modes it comes from the
    /// high-resolution channel analyzer (VFO-centered), zoomed to the FT8
    /// audio passband; otherwise from the full-rate device analyzer.
    fn make_spectrum_frame(&mut self) -> SpectrumFrame {
        if let Some(ca) = self.channel_analyzer.as_mut() {
            let vfo = self.state.rx_freq_hz();
            let ch_rate = self.main.as_ref().map(|c| c.channel_rate()).unwrap_or(48_000.0);
            // Show the FT8 sub-band (dial-200 .. dial+3500 Hz) at full res.
            let viewport = Some((vfo - 200.0, vfo + 3500.0));
            return ca.make_frame(
                vfo,
                ch_rate,
                self.cfg.db_floor,
                self.cfg.db_ceil,
                DISPLAY_BINS,
                viewport,
            );
        }
        let center = if self.tx_active { self.tx_center_hz } else { self.state.center_hz };
        self.analyzer.make_frame(
            center,
            self.state.sample_rate,
            self.cfg.db_floor,
            self.cfg.db_ceil,
            DISPLAY_BINS,
            if self.tx_active { None } else { self.cfg.viewport },
        )
    }

    fn apply(&mut self, cmd: Command) {
        use Command::*;
        match cmd {
            SetVfo { vfo, hz } => {
                let hz = hz.max(0.0);
                match vfo {
                    Vfo::A => self.state.vfo_a_hz = hz,
                    Vfo::B => self.state.vfo_b_hz = hz,
                }
                if vfo == self.state.active_vfo {
                    self.state.band = Band::containing(hz);
                    self.keep_vfo_in_span();
                }
                self.update_tuning();
            }
            SelectVfo(v) => {
                self.state.active_vfo = v;
                self.state.band = Band::containing(self.state.active_freq_hz());
                self.keep_vfo_in_span();
                self.update_tuning();
            }
            SwapVfos => {
                std::mem::swap(&mut self.state.vfo_a_hz, &mut self.state.vfo_b_hz);
                self.state.band = Band::containing(self.state.active_freq_hz());
                self.keep_vfo_in_span();
                self.update_tuning();
            }
            CopyAtoB => {
                self.state.vfo_b_hz = self.state.vfo_a_hz;
                self.update_tuning();
            }
            SetSplit(on) => self.state.split = on,
            SetCenter(hz) => {
                self.retune(hz);
                self.update_tuning();
            }
            SetSampleRate(_) => { /* needs stream re-open; deferred */ }
            SetBand(band) => self.change_band(band),
            SetMode { rx, mode } => self.set_rx_mode(rx, mode),
            SetFilter { rx, lo, hi } => {
                let (lo, hi) = (lo.min(hi), lo.max(hi));
                let r = &mut self.state.rx[rx.index()];
                (r.filter_lo, r.filter_hi) = (lo, hi);
                if let Some(d) = self.chain_mut(rx).and_then(|c| c.demod.as_mut()) {
                    d.set_filter(lo, hi);
                }
            }
            SetAgc { rx, agc } => {
                self.state.rx[rx.index()].agc = agc;
                if let Some(c) = self.chain_mut(rx) {
                    c.agc.set_mode(agc);
                }
            }
            SetAgcMaxGain { rx, db } => {
                self.state.rx[rx.index()].agc_max_gain_db = db;
                if let Some(c) = self.chain_mut(rx) {
                    c.agc.set_max_gain_db(db);
                }
            }
            SetVolume { rx, v } => self.state.rx[rx.index()].volume = v.clamp(0.0, 1.0),
            SetMute { rx, muted } => self.state.rx[rx.index()].muted = muted,
            SetSquelch { rx, db } => self.state.rx[rx.index()].squelch_db = db,
            SetNoiseBlanker(on) => self.state.noise_blanker = on,
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
            SetRit { enabled, hz } => {
                self.state.rit = sdroxide_types::OffsetState { enabled, hz };
                self.update_tuning();
            }
            SetXit { enabled, hz } => self.state.xit = sdroxide_types::OffsetState { enabled, hz },
            SetPtt(on) => {
                self.state.tx.ptt = on;
                self.sync_tx_state();
            }
            SetTune(on) => {
                self.state.tx.tune = on;
                self.sync_tx_state();
            }
            SetTxDrive(v) => self.state.tx.drive = v.clamp(0.0, 1.0),
            SetTuneDrive(v) => self.state.tx.tune_drive = v.clamp(0.0, 1.0),
            SetMicGain(v) => self.state.tx.mic_gain = v.clamp(0.0, 1.0),
            SetGain { dir, element, db } => match dir {
                Direction::Rx => {
                    if let Err(e) = self.source.set_gain_element(&element, db) {
                        warn!("set RX gain {element}: {e}");
                    }
                    self.state.gains = self.source.current_gains();
                }
                Direction::Tx => {
                    if let Err(e) = self.source.set_tx_gain_element(&element, db) {
                        warn!("set TX gain {element}: {e}");
                    }
                    self.state.tx_gains = self.source.current_tx_gains();
                }
            },
            SetAntenna { dir, name } => {
                if dir == sdroxide_types::Direction::Rx {
                    if let Err(e) = self.source.set_antenna(&name) {
                        warn!("set antenna {name}: {e}");
                    }
                    self.state.antenna_rx = self.source.current_antenna();
                } else {
                    self.state.antenna_tx = name; // applied when TX exists (M5)
                }
            }
            StoreMemory { name } => {
                let id = self.memories.iter().map(|m| m.id).max().unwrap_or(0) + 1;
                let rx = &self.state.rx[0];
                self.memories.push(MemoryChannel {
                    id,
                    name,
                    freq_hz: self.state.active_freq_hz(),
                    mode: rx.mode,
                    filter_lo: rx.filter_lo,
                    filter_hi: rx.filter_hi,
                });
                self.save_memories();
            }
            RecallMemory(id) => {
                if let Some(m) = self.memories.iter().find(|m| m.id == id).cloned() {
                    self.apply_entry(BandStackEntry {
                        freq_hz: m.freq_hz,
                        mode: m.mode,
                        filter_lo: m.filter_lo,
                        filter_hi: m.filter_hi,
                    });
                }
            }
            DeleteMemory(id) => {
                self.memories.retain(|m| m.id != id);
                self.save_memories();
            }
            SetSpectrumCfg(new_cfg) => {
                let rebuild = new_cfg.fft_size != self.cfg.fft_size;
                self.cfg = new_cfg;
                if rebuild {
                    self.analyzer = SpectrumAnalyzer::new(
                        self.cfg.fft_size as usize,
                        self.state.sample_rate,
                        self.cfg.avg_tc,
                    );
                } else {
                    self.analyzer.set_avg_tc(self.cfg.avg_tc, self.state.sample_rate);
                }
            }

            // Digital modes (FT8/FT4).
            SetDigiConfig(c) => {
                self.digi_config = c.clone();
                if let Some(d) = self.digi.as_mut() {
                    d.set_config(c);
                }
                if let Err(e) = sdroxide_config::save_digi_config(&self.digi_config) {
                    warn!("saving digi config: {e}");
                }
                self.emit_digi_status();
            }
            SetDigiAudioFreq(hz) => {
                if let Some(d) = self.digi.as_mut() {
                    d.set_audio_hz(hz);
                }
            }
            DigiCallCq => {
                if let Some(d) = self.digi.as_mut() {
                    d.call_cq();
                }
            }
            DigiStartQso { from, grid, snr, audio_hz } => {
                if let Some(d) = self.digi.as_mut() {
                    d.start_qso(from, grid, snr, audio_hz);
                }
            }
            DigiStopQso => {
                if let Some(d) = self.digi.as_mut() {
                    d.stop_qso();
                }
            }
            DigiAbortTx => {
                if let Some(d) = self.digi.as_mut() {
                    d.abort_tx();
                }
                if self.digi_tx || self.state.tx.ptt {
                    self.state.tx.ptt = false;
                    self.digi_tx = false;
                    self.sync_tx_state();
                }
            }

            // Skimmers.
            SetSkimmerEnabled(on) => {
                self.state.skimmer_enabled = on;
                self.sync_skimmer();
            }
        }
        let _ = self.event_tx.send(RadioEvent::State(self.state.clone()));
    }

    /// Construct or tear down the wideband CW skimmer to match
    /// `skimmer_enabled`. The skim window is a dedicated decimation of the raw
    /// IQ centered on the device center (offset 0), so tuning the VFO within the
    /// span doesn't disturb the streaming decoders.
    fn sync_skimmer(&mut self) {
        match (self.state.skimmer_enabled, self.skimmer.is_some()) {
            (true, false) => {
                let ddc = Ddc::new(self.state.sample_rate, SKIM_TARGET_HZ);
                let rate = ddc.out_rate();
                self.skimmer = Some(SkimmerController::new(rate, self.state.center_hz));
                self.skim_ddc = Some(ddc);
                info!(rate, "CW skimmer started");
            }
            (false, true) => {
                self.skimmer = None;
                self.skim_ddc = None;
                self.skim_buf.clear();
                info!("CW skimmer stopped");
            }
            _ => {}
        }
    }

    /// Drain skimmer spots and forward them as events.
    fn poll_skimmer(&mut self) {
        let Some(sk) = self.skimmer.as_ref() else { return };
        for action in sk.poll() {
            match action {
                SkimmerAction::Spots(spots) => {
                    let _ = self.event_tx.send(RadioEvent::SkimmerSpots(spots));
                }
            }
        }
    }

    fn emit_digi_status(&self) {
        if let Some(d) = self.digi.as_ref() {
            let _ = self.event_tx.send(RadioEvent::Ft8Status(d.status()));
        }
    }

    fn chain_mut(&mut self, rx: RxId) -> Option<&mut RxChain> {
        match rx {
            RxId::Main => self.main.as_mut(),
            RxId::Sub => self.sub.as_mut(),
        }
    }

    fn set_rx_mode(&mut self, rx: RxId, mode: Mode) {
        let r = &mut self.state.rx[rx.index()];
        r.mode = mode;
        let (lo, hi) = mode.default_filter();
        (r.filter_lo, r.filter_hi) = (lo, hi);
        let snapshot = *r;
        if let Some(c) = self.chain_mut(rx) {
            c.build_for_mode(&snapshot);
        }
        // The main receiver's mode drives the digital-mode engine; entering
        // or leaving Ft8/Ft4 starts/stops it (and aborts any in-flight QSO).
        if rx == RxId::Main {
            self.sync_digi_mode();
            self.emit_digi_status();
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

        let entry = self
            .stacks
            .get(&band)
            .and_then(|s| s.first().copied())
            .unwrap_or_else(|| {
                let (freq_hz, mode) = band.default_entry();
                let (filter_lo, filter_hi) = mode.default_filter();
                BandStackEntry { freq_hz, mode, filter_lo, filter_hi }
            });

        self.state.band = band;
        self.apply_entry(entry);
        if let Err(e) = sdroxide_config::save_bandstacks(&self.stacks) {
            warn!("saving band stacks: {e}");
        }
    }

    /// Tune + set mode/filter from a band-stack entry or memory channel.
    fn apply_entry(&mut self, entry: BandStackEntry) {
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
        self.retune(entry.freq_hz);
        self.update_tuning();
    }

    fn save_memories(&mut self) {
        if let Err(e) = sdroxide_config::save_memories(&self.memories) {
            warn!("saving memories: {e}");
        }
        let _ = self.event_tx.send(RadioEvent::Memories(self.memories.clone()));
    }

    /// Point the main-RX DDC at the active VFO (+RIT) and the sub-RX DDC at
    /// the inactive VFO.
    fn update_tuning(&mut self) {
        let main_offset = self.state.rx_freq_hz() - self.state.center_hz;
        let inactive = match self.state.active_vfo {
            Vfo::A => self.state.vfo_b_hz,
            Vfo::B => self.state.vfo_a_hz,
        };
        let sub_offset = inactive - self.state.center_hz;
        if let Some(c) = self.main.as_mut() {
            c.set_offset_hz(main_offset);
        }
        if let Some(c) = self.sub.as_mut() {
            c.set_offset_hz(sub_offset);
        }
    }

    /// Reconcile the TX hardware state with `ptt || tune`, enforcing the
    /// safety rails on key-down.
    fn sync_tx_state(&mut self) {
        let want_tx = self.state.tx.ptt || self.state.tx.tune;
        if want_tx == self.tx_active {
            return;
        }
        if want_tx {
            let txf = self.state.tx_freq_hz();
            let deny = |reason: &str, state: &mut RadioState| {
                warn!("TX refused: {reason}");
                state.tx.ptt = false;
                state.tx.tune = false;
            };
            if !self.caps.is_transmit_capable() {
                return deny("device is not transmit capable", &mut self.state);
            }
            if !self.caps.can_tx_hz(txf) {
                return deny("frequency outside the device TX range", &mut self.state);
            }
            if self.tx_ham_only && Band::containing(txf) == Band::Gen {
                return deny(
                    "outside amateur bands (tx_ham_only is set in config.toml)",
                    &mut self.state,
                );
            }
            match self.source.tx_begin(txf, self.state.sample_rate) {
                Ok(tx_rate) => {
                    self.tx = Some(TxChain::new(self.state.rx[0].mode, tx_rate));
                    self.tx_center_hz = txf;
                    self.tx_active = true;
                }
                Err(e) => deny(&format!("tx_begin failed: {e}"), &mut self.state),
            }
        } else {
            if let Err(e) = self.source.tx_end() {
                warn!("tx_end: {e}");
            }
            self.tx = None;
            self.tx_active = false;
            // Drop the transmit residue so the first receive frames aren't a
            // blend of TX samples and fresh RX.
            self.analyzer.reset();
        }
    }

    /// One ~10 ms transmit block: mic → modulator → drive → DUC → device.
    fn tx_block(&mut self) -> crate::Result<()> {
        // Digital-mode burst: the FT8/FT4 controller supplies the audio; the
        // real mic is drained and discarded so it can't leak into the burst.
        if self.digi_tx {
            return self.tx_block_digi();
        }

        let Some(tx) = self.tx.as_mut() else { return Ok(()) };

        // Drain the mic into a 48 kHz FIFO.
        if let Some(mic) = self.mic.as_mut() {
            let mut raw = Vec::with_capacity(mic.consumer.slots());
            while let Ok(s) = mic.consumer.pop() {
                raw.push(s);
            }
            match &mut self.mic_resampler {
                Some(r) => r.push(&raw, &mut self.mic_fifo),
                None => self.mic_fifo.extend_from_slice(&raw),
            }
            // Latency bound: keep at most 100 ms queued.
            if self.mic_fifo.len() > 4_800 {
                let cut = self.mic_fifo.len() - 4_800;
                self.mic_fifo.drain(..cut);
            }
        }

        tx.mod_buf.clear();
        if self.state.tx.tune || tx.modulator.is_none() {
            // Steady carrier at the tune level (also CW until the keyer exists).
            let level = self.state.tx.tune_drive.clamp(0.0, 1.0);
            tx.mod_buf.resize(TX_AUDIO_BLOCK, Complex32::new(level, 0.0));
            self.mic_fifo.clear();
        } else {
            let mut audio = [0.0f32; TX_AUDIO_BLOCK];
            let take = self.mic_fifo.len().min(TX_AUDIO_BLOCK);
            audio[..take].copy_from_slice(&self.mic_fifo[..take]);
            self.mic_fifo.drain(..take);

            let mic_gain = self.state.tx.mic_gain * 2.0;
            for a in &mut audio {
                *a = tx.dc.run(*a) * mic_gain;
            }
            let modulator = tx.modulator.as_mut().expect("checked above");
            modulator.process(&audio, &mut tx.mod_buf);
            let drive = self.state.tx.drive;
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

        tx.tx_buf.clear();
        tx.duc.process(&tx.mod_buf, &mut tx.tx_buf);
        if !tx.tx_buf.is_empty() {
            self.source.tx_write(&tx.tx_buf)?;
            // Show the operator their own TX spectrum.
            self.analyzer.process(&tx.tx_buf);
        }
        Ok(())
    }

    /// One TX block driven by the FT8/FT4 burst player: pull 10 ms of the
    /// synthesized burst, USB-modulate it (same SsbMod path as voice), and
    /// write it out. Unkeys and advances the QSO when the burst finishes.
    fn tx_block_digi(&mut self) -> crate::Result<()> {
        // Discard any real mic so it can't back up or leak into the burst.
        if let Some(mic) = self.mic.as_mut() {
            while mic.consumer.pop().is_ok() {}
        }
        let Some(tx) = self.tx.as_mut() else { return Ok(()) };

        let mut audio = [0.0f32; TX_AUDIO_BLOCK];
        let done = match self.digi.as_mut() {
            Some(d) => d.fill_tx_block(&mut audio),
            None => true,
        };

        tx.mod_buf.clear();
        let modulator = tx.modulator.as_mut().expect("SsbMod for Ft8/Ft4");
        modulator.process(&audio, &mut tx.mod_buf);
        let drive = self.state.tx.drive;
        for z in &mut tx.mod_buf {
            *z *= drive;
            let mag = z.norm();
            if mag > 1.0 {
                *z /= mag;
            }
        }
        let peak = tx.mod_buf.iter().fold(0.0f32, |a, z| a.max(z.norm()));
        tx.alc_peak = peak.max(tx.alc_peak * 0.85);

        tx.tx_buf.clear();
        tx.duc.process(&tx.mod_buf, &mut tx.tx_buf);
        if !tx.tx_buf.is_empty() {
            self.source.tx_write(&tx.tx_buf)?;
            self.analyzer.process(&tx.tx_buf);
        }

        if done {
            // Burst finished: unkey and let the QSO machine advance.
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

    /// Retune hardware center if the active VFO left the usable span.
    fn keep_vfo_in_span(&mut self) {
        let span = self.state.sample_rate;
        let usable = span * 0.45; // keep VFO out of the outer 5% roll-off
        let vfo = self.state.active_freq_hz();
        if (vfo - self.state.center_hz).abs() > usable {
            self.retune(vfo);
        }
    }

    fn retune(&mut self, center_hz: f64) {
        match self.source.set_center_hz(center_hz) {
            Ok(()) => {
                self.state.center_hz = center_hz;
                // The skim window follows the hardware center; re-label spots
                // and clear tracks so nothing straddles the old/new axis.
                if let Some(sk) = self.skimmer.as_ref() {
                    sk.set_center(center_hz);
                }
            }
            Err(e) => {
                let _ = self
                    .event_tx
                    .send(RadioEvent::ConnectionLost(format!("retune failed: {e}")));
            }
        }
    }
}
