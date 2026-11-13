//! The device state machine: the open sequence, tuning, and the front end.
//!
//! # The open sequence is load-bearing
//!
//! On the FDM-DUO the FIFO must be stopped, then initialised, then *configured*,
//! then started — and the frequency and front-end settings have to go out in
//! the window between the initialise and the start. ELAD's own module does it
//! in that order and nothing says what happens if it is not; a driver written
//! against a device nobody has is not the place to find out. [`Device::open`]
//! is therefore one function rather than a set of setters the caller sequences.
//!
//! The samplers differ in one place and it is the last step: their FIFO is
//! started by [`Device::start_pending`] from the stream thread, once the bulk
//! transfers are already queued. That is the order of the only driver anybody
//! has streamed an FDM-S2 with, and the reverse of `gr-elad`'s. See issue #178.

use sdroxide_types::EladConfig;

use crate::error::{Error, Result};
use crate::protocol::{
    CAT_FRAME_LEN, CAT_REPLY_LEN, Calibration, DuoSub, DuoTuning, FRONT_END_ATTENUATOR,
    FRONT_END_FILTER, Model, Request, STATUS_CAT_BUSY, STATUS_CAT_REPLY, TUNE_READ_LEN,
    decode_cat_reply, decode_tune_read, eeprom, eeprom_f32, eeprom_i32, s2_front_end_code,
    tune_request, tuning_word,
};
use crate::trace::Trace;
use crate::usb::UsbDev;

/// How many times the DUO's CAT busy flag is polled before giving up, and how
/// long between tries. Ten milliseconds by two hundred is two seconds, which is
/// `gr-elad`'s own patience and far longer than a front panel needs.
const CAT_POLL_TRIES: usize = 200;
const CAT_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);

/// An opened, configured ELAD.
pub struct Device {
    usb: UsbDev,
    model: Model,
    cal: Calibration,
    center_hz: f64,
    rate_hz: u32,
    attenuator: bool,
    preselector: bool,
    /// Serial and hardware version out of the EEPROM, for the log and the UI.
    pub serial: Option<String>,
    pub hw_version: Option<(u8, u8)>,
    pub firmware: Option<(u8, u8)>,
    /// Warnings gathered at open that the operator should see rather than have
    /// to find in a log — a slow USB port, a calibration that would not read.
    pub warnings: Vec<String>,
    /// Whether the FIFO has been started yet. See [`Device::start_pending`].
    started: bool,
    /// An FDM-DUO running a wide image this open uploaded, out of stand-alone.
    /// See [`crate::remote`].
    remote: bool,
    trace: Trace,
}

impl Device {
    /// Open the device named by `cfg`, configure it, and leave it streaming.
    pub fn open(cfg: &EladConfig, center_hz: f64, trace: &Trace) -> Result<Device> {
        let mut usb = UsbDev::open(&cfg.serial, trace)?;

        // A sampler's down-converter does not exist until its FPGA is loaded,
        // and nothing below would notice: the bridge answers out of its EEPROM,
        // so the whole sequence that follows succeeds against an empty FPGA and
        // the bulk endpoint then stays silent for ever (issue #178). The image
        // also *is* the sample rate, which is why this is the one place
        // `sample_rate_hz` is a command rather than a description.
        //
        // The device is claimed first and let go again rather than the other way
        // round: the model decides whether any of this applies, and it is the
        // product id of the device the configured serial actually picked — not
        // of whatever else is on the bus.
        let mut fpga_warning = None;
        match crate::fpga::wanted(usb.model(), cfg.sample_rate_hz) {
            crate::fpga::Load::NotNeeded => {}
            crate::fpga::Load::Unavailable(w) => fpga_warning = Some(w),
            crate::fpga::Load::Run(run) => {
                // ELAD's loader claims the interface itself, so ours has to be
                // gone before it starts.
                drop(usb);
                if let Err(w) = run.execute(trace) {
                    fpga_warning = Some(w);
                }
                usb = crate::fpga::reclaim(&cfg.serial, trace)?;
            }
        }
        let model = usb.model();

        let mut dev = Device {
            usb,
            model,
            cal: Calibration::default(),
            center_hz,
            rate_hz: cfg.sample_rate_hz,
            attenuator: cfg.attenuator,
            preselector: cfg.preselector,
            serial: None,
            hw_version: None,
            firmware: None,
            warnings: Vec::new(),
            started: false,
            remote: false,
            trace: trace.clone(),
        };

        dev.identify();
        dev.read_calibration();

        // An FDM-DUO streams 192 kHz in stand-alone; anything wider is an image
        // uploaded now (see `crate::remote`). Without one the stream is read at
        // what it really is.
        if model == Model::Duo && crate::remote::wanted(dev.rate_hz) {
            match crate::remote::upload_for(dev.rate_hz) {
                Ok(upload) => {
                    dev.load_remote(&upload)?;
                    dev.remote = true;
                    crate::remote::note_remote();
                    crate::remote::note_loaded(dev.rate_hz);
                }
                Err(w) => {
                    dev.rate_hz = crate::remote::STANDALONE_RATE_HZ;
                    tracing::warn!("{w}");
                    dev.trace.note(&w);
                    dev.warnings.push(w);
                }
            }
        } else if model == Model::Duo && crate::remote::went_remote() {
            // Back to 192 kHz from a wide image: have the DUO reconfigure from
            // its own image, as FDM-SW2 does. Loading the stand-alone image from
            // the DLL instead left it in remote streaming zeros (AJ3G, 25 Oct
            // 2026); this put it back in STD ALONE.
            match dev.restore_standalone() {
                Ok(()) => crate::remote::forget_remote(),
                Err(e @ Error::Access(_)) => return Err(e),
                Err(e) => {
                    let w = format!("the FDM-DUO did not go back to stand-alone: {e}");
                    tracing::warn!("{w}");
                    dev.trace.note(&w);
                    dev.warnings.push(w);
                }
            }
        }

        if let Some(w) = fpga_warning {
            tracing::warn!("{w}");
            dev.trace.note(&w);
            dev.warnings.push(w);
        }

        if let Some(w) = dev.usb.link_too_slow_for(dev.rate_hz) {
            tracing::warn!("{w}");
            dev.trace.note(&w);
            dev.warnings.push(w);
        }

        // The order below is the whole reason this is one function. See the
        // module header.
        if model == Model::Duo {
            dev.duo_fifo(DuoSub::FifoRun, 0, "stop FIFO")?;
            dev.duo_fifo(DuoSub::FifoInit, 0, "init FIFO")?;
        }
        dev.retune()?;
        dev.apply_front_end()?;
        // SDR Console sends `IQ1;` here. It is also the DUO's I/Q transmit
        // mode, and the FIFO stop and init above are what make a wide image
        // stream, so `IQ0;` makes sure it is off and the centre is tuned again
        // with the remote payload (see `crate::remote`).
        if dev.remote {
            dev.cat_write("IQ0;")?;
            std::thread::sleep(crate::remote::SETTLE_BEFORE_START);
            dev.tune_word()?;
        }
        // On the FDM-DUO the start belongs here, at the end of the sequence
        // `gr-elad` performs. On a sampler it is deliberately left until the
        // host is already reading — see `start_pending`.
        if model == Model::Duo {
            dev.start()?;
            dev.started = true;
        }

        tracing::info!(
            "opened {} (usb {:04x}:{:04x}, serial {}, {}) at {} Hz",
            model.name(),
            crate::protocol::VID,
            model.pid(),
            dev.serial.as_deref().unwrap_or("unknown"),
            dev.usb.speed_name(),
            dev.rate_hz,
        );
        Ok(dev)
    }

    /// Read the identity fields. Best-effort throughout: none of them is needed
    /// to receive, and a device whose EEPROM will not answer is still a device.
    fn identify(&mut self) {
        if let Ok(v) = self.usb.control_in(Request::Version, "fw version", 0, 0, 2)
            && v.len() >= 2
        {
            self.firmware = Some((v[0], v[1]));
        }
        let (addr, len) = eeprom::HW_VERSION;
        if let Ok(v) = self.usb.eeprom(addr, len)
            && v.len() >= 2
        {
            self.hw_version = Some((v[0], v[1]));
        }
        self.serial = self.usb.read_serial();

        let summary = format!(
            "{} — serial {}, hardware {}, firmware {}, {}",
            self.model.name(),
            self.serial.as_deref().unwrap_or("unknown"),
            self.hw_version.map(|(a, b)| format!("{a}.{b}")).unwrap_or_else(|| "unknown".into()),
            self.firmware.map(|(a, b)| format!("{a}.{b}")).unwrap_or_else(|| "unknown".into()),
            self.usb.speed_name(),
        );
        self.trace.set_identity(&summary);
    }

    /// Read the per-unit calibration.
    ///
    /// A field that will not read leaves its term at zero rather than failing
    /// the open: the result is a receiver whose absolute level is out by a
    /// decibel or two, which is a great deal better than no receiver. It is
    /// said out loud, though — a level that is quietly wrong is the sort of
    /// thing that gets reported as a hardware fault years later.
    fn read_calibration(&mut self) {
        let mut missing = Vec::new();
        let mut read_f32 = |usb: &UsbDev, (addr, len): (u16, u16), what: &str| -> f32 {
            match usb.eeprom(addr, len).ok().and_then(|b| eeprom_f32(&b)) {
                Some(v) => v,
                None => {
                    missing.push(what.to_string());
                    0.0
                }
            }
        };
        self.cal.global_db = read_f32(&self.usb, eeprom::GLOBAL_OFFSET_DB, "global gain");
        self.cal.lp_db = read_f32(&self.usb, eeprom::LP_OFFSET_DB, "filter gain");
        self.cal.att_db = read_f32(&self.usb, eeprom::ATT_OFFSET_DB, "attenuator gain");

        let (addr, len) = eeprom::RATE_CORRECTION;
        self.cal.rate_correction_hz =
            match self.usb.eeprom(addr, len).ok().and_then(|b| eeprom_i32(&b)) {
                Some(v) => v,
                None => {
                    missing.push("clock correction".to_string());
                    0
                }
            };

        self.trace.note(format!(
            "calibration: global {:+.2} dB, lp {:+.2} dB, att {:+.2} dB, clock {:+} Hz",
            self.cal.global_db, self.cal.lp_db, self.cal.att_db, self.cal.rate_correction_hz
        ));
        if !missing.is_empty() {
            let w = format!(
                "{} did not return its {} calibration; those corrections are \
                 taken as zero, so absolute levels and the dial may be slightly out",
                self.model.name(),
                missing.join(", ")
            );
            tracing::warn!("{w}");
            self.warnings.push(w);
        }
    }

    /// The ADC clock this device actually runs at, correction included.
    fn clock_hz(&self) -> f64 {
        self.model.clock_hz() + self.cal.rate_correction_hz as f64
    }

    /// Upload a remote image into an FDM-DUO, in the order SDR Console does.
    ///
    /// Not while the radio transmits — refused before the first request, and
    /// stopped between any two if a transmission begins. The radio
    /// reconfigures its FPGA here, and its transmitter goes through that FPGA
    /// too.
    fn load_remote(&mut self, upload: &[u8]) -> Result<()> {
        let waiting = || {
            Error::Access(
                "Waiting for the radio to stop transmitting to load the FDM-DUO's wide image."
                    .into(),
            )
        };
        if crate::remote::transmitting() {
            return Err(waiting());
        }
        for attempt in 1..=crate::remote::LOAD_ATTEMPTS {
            if self.load_once(upload, attempt)? {
                std::thread::sleep(crate::remote::SETTLE_BEFORE_TUNE);
                return Ok(());
            }
        }
        Err(Error::Access(format!(
            "The FDM-DUO did not accept its {} kHz image ({} tries). It now has no working \
             receiver: power-cycle the DUO, then open it again.",
            self.rate_hz / 1000,
            crate::remote::LOAD_ATTEMPTS
        )))
    }

    /// Put an FDM-DUO back on its own stand-alone image, out of remote mode.
    ///
    /// `E3` with [`crate::remote::RESTORE_BEGIN`] and no upload, then `E5`
    /// every half second until it answers `E5 10`, then `F9`: what FDM-SW2
    /// sends going back to 192 kHz.
    ///
    /// Not while the radio transmits: this reconfigures the FPGA as a load
    /// does.
    fn restore_standalone(&mut self) -> Result<()> {
        if crate::remote::transmitting() {
            return Err(Error::Access(
                "Waiting for the radio to stop transmitting to put the FDM-DUO back in \
                 stand-alone."
                    .into(),
            ));
        }
        let t0 = std::time::Instant::now();
        self.trace.note("putting the DUO back on its stand-alone image");
        let gw = Request::DuoGateway;
        let r = self.usb.control_in(
            gw,
            "restore begin",
            crate::remote::RESTORE_BEGIN,
            DuoSub::LoadBegin.index(),
            1,
        )?;
        expect_ack("beginning the stand-alone restore", DuoSub::LoadBegin as u8, &r)?;
        loop {
            let r = self.usb.control_in(gw, "restore end", 0, DuoSub::LoadEnd.index(), 2)?;
            expect_ack("finishing the stand-alone restore", DuoSub::LoadEnd as u8, &r)?;
            if crate::remote::load_done(&r) {
                break;
            }
            if t0.elapsed() > crate::remote::RESTORE_TIMEOUT {
                return Err(Error::Access(format!(
                    "the FDM-DUO did not finish going back to stand-alone in {:.0} s (last \
                     answer {r:02X?}). Power-cycle the DUO, then open it again.",
                    crate::remote::RESTORE_TIMEOUT.as_secs_f32()
                )));
            }
            std::thread::sleep(crate::remote::RESTORE_POLL);
        }
        let b = self.usb.control_in(gw, "post-restore F9", 0, DuoSub::PreLoadB.index(), 2)?;
        self.trace.note(format!(
            "back on the stand-alone image in {:.1} s, F9 {b:02X?}",
            t0.elapsed().as_secs_f32()
        ));
        self.split_off();
        Ok(())
    }

    /// Take the DUO out of the split that remote mode left on.
    ///
    /// In remote the DUO keeps VFO A on the I/Q centre and works on VFO B, and
    /// after the restore it was in STD ALONE but still showed SP, with its band
    /// changes stuck (AJ3G, 25 Oct, cbd28cf). `SP0;` is split off, `FR0;` and
    /// `FT0;` put receive and transmit on VFO A. Best-effort: the answers
    /// before and after go in the trace, and a failure doesn't stop the open.
    fn split_off(&self) {
        let read = |c: &str| self.cat_query(c).unwrap_or_else(|e| format!("({e})"));
        let before = [read("SP;"), read("FR;"), read("FT;")];
        for cmd in ["SP0;", "FR0;", "FT0;"] {
            if let Err(e) = self.cat_write(cmd) {
                self.trace.note(format!("{cmd} not sent: {e}"));
            }
        }
        let after = [read("SP;"), read("FR;"), read("FT;")];
        self.trace.note(format!("split before {before:?}, after SP0 FR0 FT0 {after:?}"));
    }

    /// One upload. `Ok(false)` when the radio's end-of-load answer says the
    /// image did not configure.
    fn load_once(&mut self, upload: &[u8], attempt: u32) -> Result<bool> {
        let waiting = || {
            Error::Access(
                "Waiting for the radio to stop transmitting to load the FDM-DUO's wide image."
                    .into(),
            )
        };
        let t0 = std::time::Instant::now();
        self.trace.note(format!(
            "loading the {} kHz image ({} bytes), try {attempt}",
            self.rate_hz / 1000,
            upload.len()
        ));
        let gw = Request::DuoGateway;
        let a = self.usb.control_in(gw, "pre-load F6", 0, DuoSub::PreLoadA.index(), 1)?;
        let b = self.usb.control_in(gw, "pre-load F9", 0, DuoSub::PreLoadB.index(), 2)?;
        self.trace.note(format!("pre-load answers {a:02X?} {b:02X?}"));
        let r = self.usb.control_in(gw, "load begin", 1, DuoSub::LoadBegin.index(), 1)?;
        expect_ack("beginning the image load", DuoSub::LoadBegin as u8, &r)?;
        // The FPGA clears itself before it takes data; SDR Console waits
        // about 0.2 s here every time, and without the wait the radio
        // answered the end of the load with E5 00, not E5 10 (AJ3G, 25 Oct).
        std::thread::sleep(crate::remote::ERASE_WAIT);
        for (n, chunk) in crate::remote::chunks(upload).enumerate() {
            if crate::remote::transmitting() {
                self.trace.note(format!("image load stopped at request {n}: transmitting"));
                return Err(waiting());
            }
            let r =
                self.usb.control_out_quiet(gw, chunk.len() as u16, DuoSub::LoadData.index(), chunk);
            if let Err(e) = r {
                self.trace.note(format!("image load failed at request {n}: {e}"));
                return Err(e);
            }
        }
        let r = self.usb.control_in(gw, "load end", 0, DuoSub::LoadEnd.index(), 2)?;
        expect_ack("finishing the image load", DuoSub::LoadEnd as u8, &r)?;
        let done = crate::remote::load_done(&r);
        let v = self.usb.control_in(gw, "post-load FD", 0, DuoSub::PostLoad.index(), 3)?;
        self.trace.note(format!(
            "image {} in {:.1} s: end {r:02X?}, after {v:02X?}",
            if done { "loaded" } else { "NOT accepted" },
            t0.elapsed().as_secs_f32()
        ));
        Ok(done)
    }

    /// Write the DDC tuning word for the current centre.
    fn retune(&mut self) -> Result<()> {
        self.tune_word()?;

        // Tell the transceiver's own front panel where its receiver has gone.
        // Best-effort: it changes nothing about the samples, and a DUO whose
        // display disagrees with the panadapter is a cosmetic fault rather than
        // a reason to refuse to receive.
        if self.model == Model::Duo {
            let hz = self.center_hz.round().clamp(0.0, 99_999_999_999.0) as u64;
            if let Err(e) = self.cat_write(&format!("CF{hz:011};")) {
                self.trace.note(format!("front-panel frequency not updated: {e}"));
            }
        }
        Ok(())
    }

    /// The tune request alone.
    fn tune_word(&self) -> Result<()> {
        let word = tuning_word(self.center_hz, self.clock_hz());
        let (value, index, data) = tune_request(self.model, word, self.remote);
        self.trace.note(format!(
            "tune {:.0} Hz: clock {:.0} Hz, word 0x{word:08X} → val 0x{value:04x} idx 0x{index:04x} \
             data [{:02X} {:02X}]",
            self.center_hz,
            self.clock_hz(),
            data[0],
            data[1],
        ));
        let req = match self.model {
            Model::Duo => Request::DuoGateway,
            Model::S1 | Model::S2 => Request::SamplerTune,
        };
        self.usb.control_out(req, "tune", value, index, &data)
    }

    /// Apply the attenuator and the pre-selection filters.
    ///
    /// Three different mechanisms behind one idea, which is why this is a match
    /// and not a pair of setters: the S1 has a register per switch, the S2 packs
    /// both into one code that also depends on where the receiver is tuned, and
    /// the DUO has neither — it takes the same `AT` and `LP` commands its CAT
    /// port takes, through the USB gateway.
    fn apply_front_end(&mut self) -> Result<()> {
        match self.model {
            Model::S1 => {
                self.sampler_front_end(FRONT_END_FILTER, u16::from(self.preselector))?;
                self.sampler_front_end(FRONT_END_ATTENUATOR, u16::from(self.attenuator))?;
            }
            Model::S2 => {
                // One code carries the band, the bypass and the attenuator, so
                // it has to be recomputed whenever any of the three moves —
                // including on a retune, which is why `set_center_hz` calls this.
                let code = s2_front_end_code(self.center_hz, self.preselector, self.attenuator);
                self.sampler_front_end(FRONT_END_FILTER, code)?;
            }
            Model::Duo => {
                self.cat_write(&format!("AT{};", u8::from(self.attenuator)))?;
                self.cat_write(&format!("LP{};", u8::from(self.preselector)))?;
            }
        }
        Ok(())
    }

    /// One S1/S2 front-end register. The device echoes the request code back.
    fn sampler_front_end(&self, index: u16, code: u16) -> Result<()> {
        let reply = self.usb.control_in(
            Request::SamplerFrontEnd,
            if index == FRONT_END_FILTER { "filter" } else { "attenuator" },
            code,
            index,
            1,
        )?;
        expect_ack("front-end setting", Request::SamplerFrontEnd.code(), &reply)
    }

    /// Start the FIFO if the open did not, and say nothing if it did.
    ///
    /// The samplers are started here, from the stream thread, *after* the bulk
    /// transfers are already queued — which is the order the one driver
    /// verified against a real FDM-S2 uses, and the opposite of the order
    /// `gr-elad` uses. Starting first leaves the device pushing into a FIFO
    /// nobody is emptying for as long as it takes to submit sixteen transfers;
    /// at 6144 kHz that is longer than the bridge's own buffering, and there is
    /// no evidence about what ELAD's FPGA does when it overruns on its very
    /// first block.
    ///
    /// The transceiver is not changed: it initialises its FIFO explicitly, its
    /// sequence is the one `gr-elad` documents, and it is the only model anybody
    /// has streamed from here.
    pub fn start_pending(&mut self) -> Result<()> {
        if self.started {
            return Ok(());
        }
        self.start()?;
        self.started = true;
        Ok(())
    }

    /// Start the sample FIFO. The device echoes `0xE9`.
    fn start(&self) -> Result<()> {
        match self.model {
            Model::Duo => self.duo_fifo(DuoSub::FifoRun, 1, "start FIFO"),
            Model::S1 | Model::S2 => {
                let reply = self.usb.control_in(Request::SamplerStart, "start FIFO", 1, 0, 1)?;
                expect_ack("starting the stream", Request::SamplerStart.code(), &reply)
            }
        }
    }

    /// One of the FDM-DUO's FIFO gateway commands.
    ///
    /// Only the start is checked for its acknowledgement. The stop and the
    /// initialise are documented by usage alone — `gr-elad` logs whether they
    /// answered and carries on either way — and refusing to open a device
    /// because its *stop* did not echo would be inventing a requirement.
    fn duo_fifo(&self, sub: DuoSub, value: u16, what: &'static str) -> Result<()> {
        let reply = self.usb.control_in(Request::DuoGateway, what, value, sub.index(), 1)?;
        if sub == DuoSub::FifoRun && value == 1 {
            return expect_ack("starting the stream", DuoSub::FifoRun as u8, &reply);
        }
        Ok(())
    }

    /// Send one ASCII CAT command through the FDM-DUO's USB gateway.
    ///
    /// The same commands the rig's CAT serial port takes, on the interface that
    /// is already open — which is what lets an operator use the wideband
    /// receiver with no serial cable plugged in at all.
    ///
    /// Always sixteen bytes: the request carries a fixed-length payload, and
    /// the radio reads up to the `;`.
    pub fn cat_write(&self, cmd: &str) -> Result<()> {
        debug_assert!(self.model == Model::Duo, "the CAT gateway is the transceiver's only");
        if cmd.len() > CAT_FRAME_LEN {
            return Err(Error::Access(format!(
                "CAT command {cmd:?} is longer than the {CAT_FRAME_LEN}-byte gateway frame"
            )));
        }
        self.wait_cat_idle()?;
        let mut buf = [0u8; CAT_FRAME_LEN];
        buf[..cmd.len()].copy_from_slice(cmd.as_bytes());
        self.usb.control_out(
            Request::DuoGateway,
            &format!("cat {cmd:?}"),
            CAT_FRAME_LEN as u16,
            DuoSub::CatWrite.index(),
            &buf,
        )
    }

    /// Ask the radio a CAT question (`SP;`) and return its answer, up to the
    /// `;`, as SDR Console reads them: the question, then the status until it
    /// says an answer is waiting, then [`DuoSub::CatRead`].
    ///
    /// Commands such as `RX;` leave an answer of their own waiting, and a read
    /// takes whatever is there: on 946026d the `GI;` after `RX;` read `RX0;`
    /// (AJ3G, 25 Oct). So an answer already waiting is read off first, and
    /// only an answer that starts with the question's own letters is taken.
    pub fn cat_query(&self, cmd: &str) -> Result<String> {
        for _ in 0..4 {
            match self.take_cat_answer()? {
                Some(stale) => {
                    self.trace.note(format!("CAT answer {stale:?} read off before {cmd:?}"))
                }
                None => break,
            }
        }
        let head = &cmd[..cmd.len().min(2)];
        self.cat_write(cmd)?;
        for _ in 0..CAT_POLL_TRIES {
            match self.take_cat_answer()? {
                Some(a) if a.starts_with(head) => return Ok(a),
                Some(other) => {
                    self.trace.note(format!("CAT answer {other:?} skipped, waiting for {cmd:?}"))
                }
                None => std::thread::sleep(CAT_POLL_INTERVAL),
            }
        }
        Err(Error::Access(format!("the FDM-DUO did not answer {cmd:?} within two seconds")))
    }

    /// The CAT answer waiting in the radio, if the status says there is one.
    /// Reading it clears the status bit.
    fn take_cat_answer(&self) -> Result<Option<String>> {
        let status =
            self.usb.control_in(Request::DuoGateway, "cat status", 0, DuoSub::Status.index(), 3)?;
        if !status.get(2).is_some_and(|b| b & STATUS_CAT_REPLY != 0) {
            return Ok(None);
        }
        let reply = self.usb.control_in(
            Request::DuoGateway,
            "cat reply",
            CAT_FRAME_LEN as u16,
            DuoSub::CatRead.index(),
            CAT_REPLY_LEN,
        )?;
        Ok(Some(decode_cat_reply(&reply)))
    }

    /// What the radio says it is tuned to, or `None` on anything but a DUO.
    ///
    /// The FDM-DUO's receive window rides on its VFO, so the front-panel knob
    /// moves the samples out from under us. With a CAT serial port there is a
    /// report to read; with only the streaming cable there was nothing, and the
    /// frequency axis quietly went stale by however far the operator turned —
    /// the samples stayed good and every label on them was wrong. This is the
    /// answer to that, on the interface the samples are already arriving on.
    pub fn read_tuned(&self) -> Result<Option<DuoTuning>> {
        if self.model != Model::Duo {
            return Ok(None);
        }
        // Polled, so only a failure is traced — see `UsbDev::control_in_polled`.
        // The stream thread notes the first answer itself.
        let reply = self.usb.control_in_polled(
            Request::DuoGateway,
            "read tune",
            0,
            DuoSub::TuneRead.index(),
            TUNE_READ_LEN,
        )?;
        Ok(decode_tune_read(&reply))
    }

    /// Block until the radio's CAT buffer has finished the previous command.
    ///
    /// Not politeness: writing while the flag is set loses the command, and
    /// loses it silently — there is no error and no reply to miss.
    fn wait_cat_idle(&self) -> Result<()> {
        for _ in 0..CAT_POLL_TRIES {
            let status = self.usb.control_in(
                Request::DuoGateway,
                "cat status",
                0,
                DuoSub::Status.index(),
                3,
            )?;
            match status.get(2) {
                Some(b) if b & STATUS_CAT_BUSY == 0 => return Ok(()),
                Some(_) => std::thread::sleep(CAT_POLL_INTERVAL),
                // Too short to hold the busy flag. Not something to spin on.
                None => {
                    return Err(Error::ShortRead {
                        request: Request::DuoGateway.code(),
                        want: 3,
                        got: status.len(),
                    });
                }
            }
        }
        Err(Error::Access(
            "the FDM-DUO's CAT buffer stayed busy for two seconds — the radio is \
             not accepting commands over USB"
                .to_string(),
        ))
    }

    // ---- what the stream thread drives ----------------------------------

    pub fn usb(&self) -> &UsbDev {
        &self.usb
    }

    pub fn model(&self) -> Model {
        self.model
    }

    pub fn rate_hz(&self) -> u32 {
        self.rate_hz
    }

    pub fn center_hz(&self) -> f64 {
        self.center_hz
    }

    /// The scale from a wire sample to a unit-full-scale float, for the state
    /// the device is in now.
    pub fn scale(&self) -> f32 {
        self.cal.scale(self.model, self.rate_hz, self.preselector, self.attenuator)
    }

    pub fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        let (lo, hi) = self.model.rx_range_hz();
        let hz = hz.clamp(0.0, self.clock_hz());
        if self.center_hz == hz {
            return Ok(());
        }
        let crossed_band = self.model == Model::S2
            && s2_front_end_code(self.center_hz, self.preselector, self.attenuator)
                != s2_front_end_code(hz, self.preselector, self.attenuator);
        self.center_hz = hz;
        if !(lo..=hi).contains(&hz) {
            self.trace
                .note(format!("tuned to {hz:.0} Hz, outside the published {lo:.0}-{hi:.0} Hz"));
        }
        self.retune()?;
        // The S2's filter bank is selected by band, so a big enough move has to
        // reselect it or the receiver stays behind the wrong filter.
        if crossed_band {
            self.apply_front_end()?;
        }
        Ok(())
    }

    pub fn set_attenuator(&mut self, on: bool) -> Result<()> {
        if self.attenuator == on {
            return Ok(());
        }
        self.attenuator = on;
        self.apply_front_end()
    }

    pub fn set_preselector(&mut self, on: bool) -> Result<()> {
        if self.preselector == on {
            return Ok(());
        }
        self.preselector = on;
        self.apply_front_end()
    }

    pub fn attenuator(&self) -> bool {
        self.attenuator
    }

    pub fn preselector(&self) -> bool {
        self.preselector
    }

    /// Stop the FIFO on the way out, so the device is not left pushing samples
    /// at a host that has stopped reading them.
    /// Stop the stream. `leaving` is a close for good — the program exiting or
    /// the radio's tab closing — rather than a reopen, and then a DUO this
    /// process took out of stand-alone is put back on its own 192 kHz image,
    /// or no other program can use it until it is power-cycled (AJ3G, 11 Nov
    /// 2026). A reopen leaves that to the next open, which knows the rate.
    pub fn shutdown(&mut self, leaving: bool) {
        let r = match self.model {
            Model::Duo => self.duo_fifo(DuoSub::FifoRun, 0, "stop FIFO"),
            Model::S1 | Model::S2 => {
                self.usb.control_in(Request::SamplerStart, "stop FIFO", 0, 0, 1).map(|_| ())
            }
        };
        if let Err(e) = r {
            self.trace.note(format!("stopping the stream failed: {e}"));
        }
        // `IQ0;` on the way out, as SDR Console does.
        if self.remote
            && let Err(e) = self.cat_write("IQ0;")
        {
            self.trace.note(format!("IQ0 not sent: {e}"));
        }
        if self.model == Model::Duo && leaving && (self.remote || crate::remote::went_remote()) {
            match self.restore_standalone() {
                Ok(()) => {
                    crate::remote::forget_remote();
                    self.remote = false;
                    tracing::info!("ELAD: FDM-DUO put back in stand-alone (192 kHz) on close");
                }
                Err(e) => {
                    tracing::warn!("ELAD: the FDM-DUO did not go back to stand-alone: {e}");
                    self.trace.note(format!("stand-alone on close failed: {e}"));
                }
            }
        }
    }

    /// Whether this is an FDM-DUO running a wide image, out of stand-alone.
    pub fn remote(&self) -> bool {
        self.remote
    }

    pub fn describe(&self) -> String {
        match &self.serial {
            Some(s) => format!("{} (serial {s})", self.model.name()),
            None => self.model.name().to_string(),
        }
    }
}

/// Check a one-byte acknowledgement that is documented to echo the request
/// code.
fn expect_ack(what: &'static str, want: u8, reply: &[u8]) -> Result<()> {
    match reply.first() {
        Some(&b) if b == want => Ok(()),
        other => Err(Error::NotAcknowledged {
            what,
            want,
            got: match other {
                Some(b) => format!("0x{b:02X}"),
                None => "an empty reply".to_string(),
            },
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_acknowledgement_has_to_be_the_code_the_device_echoes() {
        assert!(expect_ack("x", 0xE9, &[0xE9]).is_ok());
        // Trailing bytes are the device's business, not ours.
        assert!(expect_ack("x", 0xE9, &[0xE9, 0x00]).is_ok());
        assert!(expect_ack("x", 0xE9, &[0x00]).is_err());
        assert!(expect_ack("x", 0xE9, &[]).is_err());
        // The message names what was expected, because "not acknowledged" on
        // its own sends a reader to the wrong place.
        let e = expect_ack("starting the stream", 0xE9, &[0x12]).unwrap_err();
        let s = e.to_string();
        assert!(s.contains("starting the stream"), "{s}");
        assert!(s.contains("0xE9"), "{s}");
        assert!(s.contains("0x12"), "{s}");
    }
}
