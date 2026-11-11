//! An [`IqSource`] for a receiver published by a SpyServer.
//!
//! Two ways in, one source: [`SpyServerSource::connect_wideband`] asks for one
//! I/Q stream at a stage of the server's ladder, the way every other SDR here
//! delivers; [`SpyServerSource::connect_vfo`] asks for a *narrow* one that
//! follows the dial, and leans on the server's own FFT for the band view.
//! Everything below the constructors is shared, because it is the same server
//! and the same samples — only how much of the band arrives as I/Q differs.
//!
//! # Two lanes, on purpose
//!
//! The FFT stream reaches the engine through
//! [`IqSource::wide_spectrum_db`] — the same seam the RX-888's full band and
//! an Icom's own scope already use — and *not* through `sample_rate`. That
//! matters: `sample_rate` is what the DDC, the sub-receiver clamp and
//! `keep_vfo_in_span` are all built on, and a value describing a span the
//! receiver cannot actually demodulate would break each of them in a different
//! way. So the main panadapter is what the I/Q covers, honestly, and the strip
//! above it is what the server drew.
//!
//! The consequence in the VFO interface is deliberate and worth knowing about:
//! the main panadapter is a few tens of kHz wide, and the whole band lives in
//! the strip. Tuning across the strip works because the engine notices the new
//! frequency is outside the received slice and retunes — which here means the
//! server slides its narrow window along.
//!
//! Receive only: the trait's transmit methods already default to errors, which
//! is the correct answer for a receiver on the other side of a network.

use std::time::Duration;

use sdroxide_dsp::IqCorrect;
use sdroxide_radio::{Complex32, ControlUpdate, DC_BLOCK_HZ, IqSource, Result};
use sdroxide_spyserver::SpyServerHandle;
use sdroxide_types::SpyServerConfig;

/// How long the server may deliver nothing before the link counts as dead.
/// Matches the `rtl_tcp` backend's network window: long enough that a
/// congested WiFi hop is not read as a dead server and reconnected out from
/// under a stream that was about to recover.
const SILENCE_BEFORE_RECONNECT: Duration = Duration::from_secs(5);

pub struct SpyServerSource {
    handle: SpyServerHandle,
    /// The centre this end has asked for and the server accepted — see
    /// [`IqSource::center_hz`] below for why this rather than the readback.
    center: f64,
    rx_scratch: Vec<f32>,
    label: String,
    /// Whether this is the narrow-I/Q interface. Only affects what is said to
    /// the operator; the samples arrive the same way either side.
    vfo: bool,
    gain_index: u32,
    auto_digital_gain: bool,
    digital_gain_db: f64,
    fft_enabled: bool,
    /// The sync counter at the last poll, so a device move by whoever owns a
    /// shared server is noticed exactly once.
    last_sync_seq: u64,
    /// Front-end DC and image correction, applied to every block as it
    /// arrives. `None` while the operator has it switched off. See
    /// [`IqCorrect`].
    iq_correct: Option<IqCorrect>,
}

impl SpyServerSource {
    /// A wideband I/Q stream, at a stage of the server's own rate ladder.
    pub fn connect_wideband(cfg: &SpyServerConfig, center_hz: f64) -> anyhow::Result<Self> {
        let handle = SpyServerHandle::connect_wideband(cfg, center_hz)?;
        Ok(Self::from_handle(handle, cfg, center_hz, false))
    }

    /// A narrow I/Q stream that follows the dial, plus the server's FFT of the
    /// whole band for the full-band strip.
    pub fn connect_vfo(cfg: &SpyServerConfig, center_hz: f64) -> anyhow::Result<Self> {
        let handle = SpyServerHandle::connect_vfo(cfg, center_hz)?;
        Ok(Self::from_handle(handle, cfg, center_hz, true))
    }

    fn from_handle(
        handle: SpyServerHandle,
        cfg: &SpyServerConfig,
        center_hz: f64,
        vfo: bool,
    ) -> Self {
        let label = handle.label.clone();
        tracing::info!("SpyServer source ready: {label}, center {center_hz:.0} Hz");
        SpyServerSource {
            center: center_hz,
            rx_scratch: Vec::new(),
            label,
            vfo,
            // Whatever the handshake settled on, which is not always what was
            // configured: on a server another client owns, the gain is theirs
            // and this end never sent one.
            gain_index: handle.gain_index(),
            auto_digital_gain: cfg.auto_digital_gain,
            digital_gain_db: cfg.digital_gain_db,
            fft_enabled: cfg.fft_enabled && handle.fft_span_hz > 0.0,
            last_sync_seq: handle.sync_seq(),
            iq_correct: cfg
                .iq_correction
                .then(|| IqCorrect::new(DC_BLOCK_HZ, handle.sample_rate_hz)),
            handle,
        }
    }

    /// What the server said about the receiver behind it, for the capability
    /// block.
    pub fn info(&self) -> &sdroxide_spyserver::DeviceInfo {
        &self.handle.info
    }

    /// Whether this client may move the receiver itself, as opposed to only
    /// its own window inside what another client is already receiving.
    pub fn can_control(&self) -> bool {
        self.handle.can_control()
    }

    /// The window this client can tune within, in Hz — the whole receiver when
    /// it has control, and only the slack around another client's centre when
    /// it does not.
    pub fn tuning_window(&self) -> (f64, f64) {
        self.handle.tuning_window()
    }

    /// The I/Q rates this server offers on this interface.
    pub fn available_rates(&self) -> Vec<f64> {
        self.handle.info.iq_rates(self.vfo).into_iter().map(|(_, r)| r).collect()
    }
}

impl IqSource for SpyServerSource {
    /// The engine is transmitting and has stopped reading, or has started
    /// again. Passed straight through to the stream thread, which keeps
    /// receiving either way — this only decides whether a full ring is
    /// reported as an overrun or as the ordinary cost of an over. See
    /// [`IqSource::set_rx_paused`].
    fn set_rx_paused(&mut self, paused: bool) {
        self.handle.set_rx_paused(paused);
    }

    /// The I/Q rate, in both interfaces — never the FFT span.
    ///
    /// The full-band strip's own widget says why: this number is read as the
    /// real receiver bandwidth by `keep_vfo_in_span`, which would then never
    /// retune, and by the sub-receiver clamp, which would let a sub be placed
    /// where the DDC cannot reach.
    fn sample_rate(&self) -> f64 {
        self.handle.sample_rate_hz
    }

    /// What was asked for and accepted, not what the server last reported.
    ///
    /// The engine sets its own `center_hz` to whatever it requested the moment
    /// [`Self::set_center_hz`] answers `Ok`, and never reads this back. So an
    /// `Ok` is a promise that the received slice is where the engine thinks it
    /// is; a refusal has to be an `Err` (below) rather than a quiet clamp, or
    /// the VFO ends up outside the delivered span with the arithmetic
    /// insisting it is inside — a receiver hearing nothing, that never retunes
    /// because by its own reckoning there is nothing to fix.
    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        let reachable = self.handle.reachable_center(hz);
        if (reachable - hz).abs() > 1.0 {
            let (lo, hi) = self.handle.tuning_window();
            return Err(sdroxide_radio::RadioError::Msg(if self.handle.can_control() {
                format!(
                    "this receiver tunes {:.3}–{:.3} MHz",
                    f64::from(self.handle.info.minimum_frequency) / 1e6,
                    f64::from(self.handle.info.maximum_frequency) / 1e6,
                )
            } else {
                format!(
                    "another client owns this SpyServer and has it on {:.6} MHz — until it \
                     moves, only {:.6}–{:.6} MHz is reachable from here",
                    self.handle.device_center_hz() / 1e6,
                    lo / 1e6,
                    hi / 1e6,
                )
            }));
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
        if let Some(iq) = self.iq_correct.as_mut() {
            iq.process(&mut buf[..pairs]);
        }
        Ok(pairs)
    }

    /// A centre of its own — the server's centre, moved by a command (a server that
    /// refuses tuning answers once per resting place) — so the engine may centre
    /// the I/Q on a resting dial (`Engine::recentre_after_dwell`).
    fn recentre_on_dwell(&self) -> bool {
        true
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    /// The server's FFT of the whole band, as the full-band strip.
    ///
    /// These are finished dBFS bins the server quantised into the window this
    /// client asked for; the centre and span travel with them because the
    /// window moves. Nothing here can be demodulated — it is a picture, like
    /// an Icom's scope sweep — and the engine's own auto-levelling decides how
    /// it is drawn.
    /// The span the server's FFT covers, which is a stage of its own ladder and
    /// settled at connect — so how far the panadapter may be zoomed out is
    /// known before a single frame has arrived.
    fn wide_span_hz(&self) -> f64 {
        if self.fft_enabled { self.handle.fft_span_hz } else { 0.0 }
    }

    fn wide_spectrum_db(&mut self, out: &mut Vec<f32>) -> Option<(f64, f64)> {
        let frame = self.handle.take_fft()?;
        out.clear();
        out.extend_from_slice(&frame.bins);
        Some((frame.center_hz, frame.span_hz))
    }

    /// Whoever owns a shared server moved its receiver.
    ///
    /// Reported rather than corrected: the span simply is somewhere else now,
    /// and answering with a retune would have two clients pushing one receiver
    /// back and forth forever. The engine adopts such moves for exactly this
    /// reason.
    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        let seq = self.handle.sync_seq();
        if seq == self.last_sync_seq {
            return Vec::new();
        }
        self.last_sync_seq = seq;
        // Only meaningful while another client owns the tuning. When this end
        // has control, a moved device centre is the server carrying out our own
        // retune, and reporting it back would be an echo.
        if self.handle.can_control() {
            return Vec::new();
        }
        let iq = self.handle.iq_center_hz();
        if (iq - self.center).abs() < 1.0 {
            return Vec::new();
        }
        self.center = iq;
        vec![ControlUpdate::Center(iq)]
    }

    /// The server's gain index, plus the pseudo-elements for everything else
    /// this interface can change while it runs.
    ///
    /// Routing them through `SetGain` rather than adding `Command` variants
    /// keeps `Command`, `DeviceCaps` and the engine untouched for settings only
    /// this backend has — the pattern the RTL-SDR's switches already use. The
    /// element names live in [`SpyServerConfig`] so the wasm settings UI, which
    /// cannot see the driver crate, can build the same commands.
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            SpyServerConfig::GAIN_ELEMENT => {
                // An index into the far end's gain table, not a number of dB.
                let max = f64::from(self.handle.info.maximum_gain_index);
                self.gain_index = db.round().clamp(0.0, max) as u32;
                self.handle.set_gain_index(self.gain_index);
            }
            SpyServerConfig::AUTO_DIGITAL_GAIN_ELEMENT => {
                self.auto_digital_gain = db >= 0.5;
                self.handle.set_auto_digital_gain(self.auto_digital_gain);
            }
            SpyServerConfig::DIGITAL_GAIN_ELEMENT => {
                self.digital_gain_db = db.clamp(0.0, 60.0);
                self.handle.set_digital_gain_db(self.digital_gain_db);
            }
            SpyServerConfig::FFT_ENABLED_ELEMENT => {
                self.fft_enabled = db >= 0.5;
                self.handle.set_fft_enabled(self.fft_enabled);
            }
            SpyServerConfig::FFT_DECIMATION_ELEMENT => {
                self.handle.set_fft_decimation(db.round().clamp(0.0, 31.0) as u32);
            }
            SpyServerConfig::FFT_DB_OFFSET_ELEMENT => {
                self.handle.set_fft_db_offset(db);
            }
            SpyServerConfig::FFT_DB_RANGE_ELEMENT => {
                self.handle.set_fft_db_range(db);
            }
            // Purely a host-side correction, so it switches on and off between
            // one block and the next with nothing to reconfigure on the server.
            // Switching it on starts from a fresh estimate rather than whatever
            // the loop was left holding, which may be from another band.
            SpyServerConfig::IQ_CORRECTION_ELEMENT => {
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
            (SpyServerConfig::GAIN_ELEMENT.to_string(), f64::from(self.handle.gain_index())),
            (
                SpyServerConfig::AUTO_DIGITAL_GAIN_ELEMENT.to_string(),
                f64::from(u8::from(self.auto_digital_gain)),
            ),
            (SpyServerConfig::DIGITAL_GAIN_ELEMENT.to_string(), self.digital_gain_db),
        ]
    }

    /// A server that was restarted, or that dropped this client for falling
    /// behind, is reported as needing a reopen so the engine reconnects on its
    /// own rather than waiting for Apply.
    fn needs_reopen(&self) -> bool {
        !self.handle.is_alive() || self.handle.silent_for() >= SILENCE_BEFORE_RECONNECT
    }

    fn release(&mut self) {
        self.handle.release();
    }

    /// What an operator needs to know but cannot see from the panadapter.
    ///
    /// Two things qualify. A shared server whose receiver belongs to somebody
    /// else limits where this end can tune, and the symptom without a notice is
    /// a dial that refuses to move for no visible reason. And in the VFO
    /// interface the main panadapter is deliberately a narrow slice, which
    /// looks exactly like a receiver that has lost its bandwidth unless it is
    /// said out loud.
    fn open_status(&self) -> Option<String> {
        if !self.handle.can_control() {
            let (lo, hi) = self.handle.tuning_window();
            return Some(format!(
                "{}: another client owns this server — tuning is limited to {:.3}–{:.3} MHz, \
                 and the gain is theirs",
                self.label,
                lo / 1e6,
                hi / 1e6,
            ));
        }
        if self.vfo {
            let span = self.handle.fft_span_hz;
            return Some(if span > 0.0 {
                format!(
                    "{}: the panadapter is the {:.1} kHz being received; the band view is the \
                     server's {:.3} MHz FFT in the strip above it",
                    self.label,
                    self.handle.sample_rate_hz / 1e3,
                    span / 1e6,
                )
            } else {
                format!(
                    "{}: the panadapter is the {:.1} kHz being received, and the server's FFT \
                     is switched off — there is no band view",
                    self.label,
                    self.handle.sample_rate_hz / 1e3,
                )
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Just enough of a server to get a source open and hand it one FFT frame.
    /// The protocol itself is exercised properly in `sdroxide-spyserver`'s own
    /// tests; what is under test here is the thin layer above it, where the
    /// two lanes are separated and a refusal is decided.
    fn fake(can_control: u32) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        std::thread::spawn(move || {
            let Ok((mut sock, _)) = listener.accept() else { return };
            let msg = |kind: u16, body: &[u8]| {
                let mut out = Vec::new();
                out.extend_from_slice(&((2u32 << 24) | 1700).to_le_bytes());
                out.extend_from_slice(&u32::from(kind).to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&(body.len() as u32).to_le_bytes());
                out.extend_from_slice(body);
                out
            };
            let words =
                |ws: &[u32]| -> Vec<u8> { ws.iter().flat_map(|w| w.to_le_bytes()).collect() };

            let mut buf = [0u8; 4096];
            if sock.read(&mut buf).is_err() {
                return;
            }
            // An RTL-SDR at 2.4 Msps, parked on 100 MHz.
            let info =
                words(&[3, 1, 2_400_000, 2_400_000, 6, 1, 28, 24_000_000, 1_766_000_000, 8, 0, 0]);
            let sync = words(&[can_control, 0, 100_000_000, 100_000_000, 100_000_000, 0, 0, 0, 0]);
            if sock.write_all(&msg(0, &info)).is_err() || sock.write_all(&msg(1, &sync)).is_err() {
                return;
            }
            // One 8-bit FFT frame, then hold the connection open.
            std::thread::sleep(Duration::from_millis(50));
            let bins: Vec<u8> = (0..64u32).map(|i| (i * 4) as u8).collect();
            let _ = sock.write_all(&msg(301, &bins));
            std::thread::sleep(Duration::from_secs(3));
        });
        addr
    }

    fn cfg(addr: &str) -> SpyServerConfig {
        SpyServerConfig { address: addr.into(), ..SpyServerConfig::default() }
    }

    fn wait<T>(mut f: impl FnMut() -> Option<T>) -> Option<T> {
        for _ in 0..200 {
            if let Some(v) = f() {
                return Some(v);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        None
    }

    /// The band view arrives on the wide lane, labelled with its own centre and
    /// span — and `sample_rate` keeps reporting the *I/Q*, which is what the
    /// DDC and `keep_vfo_in_span` are built on.
    #[test]
    fn the_band_view_arrives_on_the_wide_lane_and_not_as_the_sample_rate() {
        let addr = fake(1);
        let mut src = SpyServerSource::connect_vfo(&cfg(&addr), 100_000_000.0).expect("connect");

        // 2.4 Msps ladder, VFO target 96 kHz: 2.4M/2^5 = 75 kHz.
        assert_eq!(src.sample_rate(), 75_000.0);

        let mut bins = Vec::new();
        let (center, span) = wait(|| src.wide_spectrum_db(&mut bins)).expect("a full-band frame");
        assert_eq!(bins.len(), 64);
        assert_eq!(span, 2_400_000.0, "the FFT covers the whole receiver, not the I/Q");
        assert_eq!(center, 100_000_000.0);
        assert_ne!(src.sample_rate(), span, "the wide span must never become the I/Q rate");

        // Taken once, gone: `None` is how the engine knows there is nothing new,
        // and a frame served twice would be drawn as a fresh row of history.
        assert_eq!(src.wide_spectrum_db(&mut bins), None);
    }

    /// The VFO interface says out loud that its panadapter is a narrow slice.
    /// Without it, a receiver working exactly as designed looks like one that
    /// has lost its bandwidth.
    #[test]
    fn the_vfo_interface_explains_its_narrow_panadapter() {
        let addr = fake(1);
        let src = SpyServerSource::connect_vfo(&cfg(&addr), 100_000_000.0).expect("connect");
        let status = src.open_status().expect("a notice");
        assert!(status.contains("75.0 kHz"), "{status}");
        assert!(status.contains("strip"), "{status}");

        // The wideband interface needs no such notice: its panadapter is the
        // ordinary thing.
        let addr = fake(1);
        let src = SpyServerSource::connect_wideband(&cfg(&addr), 100_000_000.0).expect("connect");
        assert_eq!(src.open_status(), None);
    }

    /// A tune the server cannot honour must come back as an error, not as a
    /// quiet clamp.
    ///
    /// This is the subtle one. The engine sets its own centre to whatever it
    /// asked for the moment this returns `Ok`, and never reads `center_hz`
    /// back — so clamping and answering `Ok` would leave the VFO outside the
    /// delivered span with the arithmetic insisting it is inside: a receiver
    /// hearing nothing, which never retunes because by its own reckoning there
    /// is nothing to fix.
    #[test]
    fn a_tune_outside_the_reachable_window_is_refused_rather_than_clamped() {
        let addr = fake(0);
        let mut src =
            SpyServerSource::connect_wideband(&cfg(&addr), 100_000_000.0).expect("connect");
        assert!(!src.can_control());

        // 600 kHz of window inside 2.4 MHz of receiver: 900 kHz of slack.
        assert_eq!(src.tuning_window(), (99_100_000.0, 100_900_000.0));

        // Inside: accepted, and the reported centre is what was asked for.
        src.set_center_hz(100_500_000.0).expect("inside the window");
        assert_eq!(src.center_hz(), 100_500_000.0);

        // Outside: refused, with the reachable window named — and the centre is
        // left where it was rather than moved somewhere the samples are not.
        let e = src.set_center_hz(102_000_000.0).expect_err("outside the window");
        let s = e.to_string();
        assert!(s.contains("another client owns"), "{s}");
        assert!(s.contains("100.900000"), "the reachable edge is named: {s}");
        assert_eq!(src.center_hz(), 100_500_000.0, "a refused tune must not move the centre");
    }

    /// A shared server's limits are stated on screen, so a dial that will not
    /// move is never a mystery.
    #[test]
    fn a_shared_server_says_what_it_will_not_let_this_client_do() {
        let addr = fake(0);
        let src = SpyServerSource::connect_wideband(&cfg(&addr), 100_000_000.0).expect("connect");
        let status = src.open_status().expect("a notice");
        assert!(status.contains("another client owns"), "{status}");
        assert!(status.contains("99.100"), "{status}");
        assert!(status.contains("gain is theirs"), "{status}");
    }
}
