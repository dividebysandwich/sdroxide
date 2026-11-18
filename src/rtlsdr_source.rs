//! An [`IqSource`] for an RTL2832U dongle driven by the native driver in
//! `sdroxide-rtlsdr` — no SoapySDR, no libusb.
//!
//! Two ways in, one source: [`RtlSdrSource::open`] takes a dongle on this
//! machine's USB bus, [`RtlSdrSource::connect`] one published over the network
//! by an `rtl_tcp` server. Everything below the constructors is shared,
//! because it is the same radio and the same samples — only the distance to
//! its registers differs.
//!
//! Receive only: the trait's transmit methods already default to errors, which
//! is the correct answer for this hardware.

use std::time::Duration;

use sdroxide_dsp::IqCorrect;
use sdroxide_radio::{Complex32, DC_BLOCK_HZ, IqSource, Result};
use sdroxide_rtlsdr::tcp::proto::RspCmd;
use sdroxide_rtlsdr::{RtlSdrHandle, Stage, stage_db};
use sdroxide_types::{RtlSdrAgc, RtlSdrConfig, RtlSdrHfMode, RtlTcpConfig};

/// How long the dongle may deliver nothing before the connection counts as
/// dead. Shorter than the HPSDR backend's five seconds: this is a local USB
/// device, so there is no network to be briefly slow.
const SILENCE_BEFORE_REOPEN: Duration = Duration::from_secs(3);

/// The same, for an `rtl_tcp` link — where there *is* a network to be briefly
/// slow. Matches the HPSDR backend's window, and is long enough that a
/// congested WiFi hop does not get read as a dead server and reconnected out
/// from under a stream that was about to recover.
const SILENCE_BEFORE_RECONNECT: Duration = Duration::from_secs(5);

pub struct RtlSdrSource {
    handle: RtlSdrHandle,
    center: f64,
    label: String,
    /// Last gain the operator asked for, and what the hardware snapped it to.
    gain_db: f64,
    /// LNA and mixer in dB, the IF VGA in steps, as last set — one by one
    /// or, for LNA and mixer, through the combined ladder.
    stages: [f64; 3],
    agc: RtlSdrAgc,
    bias_tee: bool,
    /// How long a silence has to last before this source asks to be reopened.
    silence: Duration,
    /// Whether the dongle is on the other end of a network link. Only the
    /// operator-facing wording depends on it — a bias tee somewhere else is a
    /// different warning from one on the desk.
    remote: bool,
    /// Front-end DC and image correction, applied to every block as it arrives
    /// so the panadapter, the demodulators and the skimmers all see the same
    /// clean stream. `None` while the operator has it switched off — the
    /// samples then reach the engine exactly as the dongle produced them.
    /// See [`IqCorrect`].
    iq_correct: Option<IqCorrect>,
}

impl RtlSdrSource {
    /// A dongle on this machine's USB bus.
    pub fn open(cfg: &RtlSdrConfig, center_hz: f64) -> anyhow::Result<Self> {
        let handle = RtlSdrHandle::open(cfg, center_hz)?;
        Ok(Self::from_handle(
            handle,
            Settings {
                gain_db: cfg.tuner_gain_db,
                agc: cfg.agc,
                bias_tee: cfg.bias_tee,
                iq_correction: cfg.iq_correction,
                silence: SILENCE_BEFORE_REOPEN,
                remote: false,
            },
            center_hz,
        ))
    }

    /// A dongle published over the network by an `rtl_tcp` server.
    ///
    /// The far end owns the hardware, so everything here is a request rather
    /// than a readback — including the sample rate and the gain the stream is
    /// actually running at. That is a property of the protocol, not of this
    /// connection; see [`RtlSdrHandle::connect`].
    pub fn connect(cfg: &RtlTcpConfig, center_hz: f64) -> anyhow::Result<Self> {
        let handle = RtlSdrHandle::connect(cfg, center_hz)?;
        Ok(Self::from_handle(
            handle,
            Settings {
                gain_db: cfg.tuner_gain_db,
                agc: cfg.agc,
                bias_tee: cfg.bias_tee,
                iq_correction: cfg.iq_correction,
                silence: SILENCE_BEFORE_RECONNECT,
                remote: true,
            },
            center_hz,
        ))
    }

    fn from_handle(handle: RtlSdrHandle, s: Settings, center_hz: f64) -> Self {
        let label = format!("{} @ {:.3} Msps", handle.label, handle.sample_rate_hz / 1e6);
        tracing::info!("RTL-SDR source ready: {label}, center {center_hz:.0} Hz");
        RtlSdrSource {
            center: center_hz,
            label,
            gain_db: s.gain_db,
            stages: {
                let (l, m) = sdroxide_rtlsdr::ladder_indices(s.gain_db);
                // The VGA's code under manual gain until it is set: 0x08.
                [stage_db(Stage::Lna, l), stage_db(Stage::Mixer, m), 8.0]
            },
            agc: s.agc,
            bias_tee: s.bias_tee,
            silence: s.silence,
            remote: s.remote,
            iq_correct: s.iq_correction.then(|| IqCorrect::new(DC_BLOCK_HZ, handle.sample_rate_hz)),
            handle,
        }
    }

    pub fn sample_rate_hz(&self) -> f64 {
        self.handle.sample_rate_hz
    }

    /// Pass one `rsp_tcp` control through, and only over the network. Over USB
    /// there is no server to send it to, and the element names are shared
    /// because both interfaces are the same `IqSource`.
    fn set_rsp(&self, cmd: RspCmd, arg: u32) {
        if self.remote {
            self.handle.set_rsp(cmd, arg);
        }
    }

    pub fn tuner(&self) -> &str {
        &self.handle.tuner
    }

    pub fn is_blog_v4(&self) -> bool {
        self.handle.is_blog_v4
    }

    /// Whether this dongle is configured to reach below the tuner's 24 MHz
    /// floor, through an upconverter or direct sampling.
    pub fn hf_capable(&self) -> bool {
        self.handle.hf_capable
    }
}

/// The settings the two constructors have in common — the same knobs in
/// [`RtlSdrConfig`] and [`RtlTcpConfig`], plus what the link implies.
struct Settings {
    gain_db: f64,
    agc: RtlSdrAgc,
    bias_tee: bool,
    iq_correction: bool,
    silence: Duration,
    remote: bool,
}

impl IqSource for RtlSdrSource {
    /// The engine is transmitting and has stopped reading, or has started
    /// again. Passed straight through to the stream thread, which keeps
    /// receiving either way — this only decides whether a full ring is
    /// reported as an overrun or as the ordinary cost of an over. See
    /// [`IqSource::set_rx_paused`].
    fn set_rx_paused(&mut self, paused: bool) {
        self.handle.set_rx_paused(paused);
    }

    fn sample_rate(&self) -> f64 {
        self.handle.sample_rate_hz
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        self.handle.set_center_hz(hz);
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // Straight into the caller's block: the ring's interleaved floats and a
        // complex buffer are the same bytes, so there is nothing to unpack —
        // see `sdroxide_dsp::as_interleaved_mut`. The scratch vector and the
        // loop that copied out of it were a second pass over the whole stream
        // to change nothing but the type.
        let n = self.handle.rx_read(sdroxide_dsp::as_interleaved_mut(buf));
        let pairs = n / 2;
        if pairs == 0 {
            // Nothing yet — brief nap so the DSP loop doesn't spin hot.
            std::thread::sleep(Duration::from_millis(2));
            return Ok(0);
        }
        if let Some(iq) = self.iq_correct.as_mut() {
            iq.process(&mut buf[..pairs]);
        }
        Ok(pairs)
    }

    /// A centre of its own — the tuner's LO — so the engine may centre the I/Q on a
    /// resting dial (`Engine::recentre_after_dwell`).
    fn recentre_on_dwell(&self) -> bool {
        true
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    /// The tuner gain, plus a pseudo-element carrying the AGC mode.
    ///
    /// Routing AGC through `SetGain` rather than adding a `Command` variant
    /// keeps `Command`, `DeviceCaps` and the engine untouched for a setting
    /// only this backend has. The encoding lives with
    /// [`RtlSdrAgc::code`] so the two ends cannot drift apart.
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            RtlSdrConfig::TUNER_GAIN_ELEMENT => {
                self.gain_db = db;
                self.handle.set_gain_db(db);
                // The ladder moves LNA and mixer together; their rails follow.
                let (l, m) = sdroxide_rtlsdr::ladder_indices(db);
                self.stages[0] = stage_db(Stage::Lna, l);
                self.stages[1] = stage_db(Stage::Mixer, m);
            }
            RtlSdrConfig::LNA_GAIN_ELEMENT
            | RtlSdrConfig::MIX_GAIN_ELEMENT
            | RtlSdrConfig::IF_GAIN_ELEMENT => {
                let (i, stage) = match name {
                    RtlSdrConfig::LNA_GAIN_ELEMENT => (0, Stage::Lna),
                    RtlSdrConfig::MIX_GAIN_ELEMENT => (1, Stage::Mixer),
                    _ => (2, Stage::Vga),
                };
                // Reported at the step the tuner will take, so the rail
                // settles somewhere real.
                self.stages[i] = stage_db(stage, sdroxide_rtlsdr::stage_index_for(stage, db));
                self.handle.set_stage(stage, db);
            }
            RtlSdrConfig::AGC_ELEMENT => {
                self.agc = RtlSdrAgc::from_code(db.round().clamp(0.0, 3.0) as u8);
                self.handle.set_agc(self.agc);
            }
            RtlSdrConfig::PPM_ELEMENT => {
                self.handle.set_ppm(db.round().clamp(-1000.0, 1000.0) as i32);
            }
            RtlSdrConfig::HF_MODE_ELEMENT => {
                self.handle.set_hf_mode(RtlSdrHfMode::from_code(db.round().clamp(0.0, 2.0) as u8));
            }
            RtlSdrConfig::BIAS_TEE_ELEMENT => {
                self.bias_tee = db >= 0.5;
                self.handle.set_bias_tee(self.bias_tee);
            }
            // The `rsp_tcp` controls. Reachable only on the network interface,
            // and only against an SDRplay server — a plain `rtl_tcp` one does
            // not know these opcodes and ignores them in silence, which is what
            // this protocol does with everything it does not recognise. The
            // settings tab says so rather than pretending to have detected it.
            RtlTcpConfig::RSP_ANTENNA_ELEMENT => {
                self.set_rsp(RspCmd::SetAntenna, db.clamp(0.0, 2.0) as u32)
            }
            RtlTcpConfig::RSP_LNA_STATE_ELEMENT => {
                self.set_rsp(RspCmd::SetLnaState, db.clamp(0.0, 9.0) as u32)
            }
            RtlTcpConfig::RSP_IFGR_ELEMENT => {
                self.set_rsp(RspCmd::SetIfGainR, db.clamp(20.0, 59.0) as u32)
            }
            RtlTcpConfig::RSP_AGC_ELEMENT => self.set_rsp(RspCmd::SetAgc, u32::from(db >= 0.5)),
            RtlTcpConfig::RSP_AGC_SETPOINT_ELEMENT => {
                // The protocol carries an unsigned argument and the set point
                // is negative dBfs, so it goes out as its two's-complement bit
                // pattern — the same way ppm and gain already do.
                self.set_rsp(RspCmd::SetAgcSetPoint, db.clamp(-72.0, 0.0) as i32 as u32)
            }
            RtlTcpConfig::RSP_NOTCH_ELEMENT => {
                self.set_rsp(RspCmd::SetNotch, db.clamp(0.0, 15.0) as u32)
            }
            RtlTcpConfig::RSP_REF_OUT_ELEMENT => {
                self.set_rsp(RspCmd::SetRefOut, u32::from(db >= 0.5))
            }
            // Purely a host-side correction, so it switches on and off between
            // one block and the next with nothing to reconfigure on the dongle.
            // Switching it on starts from a fresh estimate rather than whatever
            // the loop was left holding, which may be from another band.
            RtlSdrConfig::IQ_CORRECTION_ELEMENT => {
                if db >= 0.5 {
                    match self.iq_correct.as_mut() {
                        Some(iq) => iq.reset(),
                        None => {
                            self.iq_correct =
                                Some(IqCorrect::new(DC_BLOCK_HZ, self.handle.sample_rate_hz));
                        }
                    }
                } else {
                    self.iq_correct = None;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        vec![
            (RtlSdrConfig::TUNER_GAIN_ELEMENT.to_string(), self.gain_db),
            (RtlSdrConfig::LNA_GAIN_ELEMENT.to_string(), self.stages[0]),
            (RtlSdrConfig::MIX_GAIN_ELEMENT.to_string(), self.stages[1]),
            (RtlSdrConfig::IF_GAIN_ELEMENT.to_string(), self.stages[2]),
            (RtlSdrConfig::AGC_ELEMENT.to_string(), self.agc.code() as f64),
        ]
    }

    /// A dongle that has been unplugged, or whose thread has died, is reported
    /// as needing a reopen so the engine reconnects on its own — which is what
    /// makes replugging one Just Work rather than needing Apply pressed. Over
    /// `rtl_tcp` the same rule reconnects to a server that was restarted, or
    /// that dropped this client for falling behind.
    fn needs_reopen(&self) -> bool {
        !self.handle.is_alive() || self.handle.silent_for() >= self.silence
    }

    /// Hand the dongle back before the engine opens its replacement. Without
    /// this, changing anything in Settings → Radio on a running RTL-SDR fails
    /// with "held by another program" — the other program being us.
    fn release(&mut self) {
        self.handle.release();
    }

    /// Surface what an operator needs to know but cannot see. A bias tee
    /// putting DC on the feedline is worth a standing on-screen reminder — the
    /// setting is persisted, so it survives a restart with nothing else to
    /// indicate it. More so over `rtl_tcp`, where the coax it is feeding is
    /// wherever the server is and may not be in the room.
    fn open_status(&self) -> Option<String> {
        self.bias_tee.then(|| {
            let where_ = if self.remote { " at the far end" } else { "" };
            format!("{}: bias tee is ON — ~4.5 V DC on the antenna coax{where_}", self.label)
        })
    }
}
