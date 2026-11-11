//! The decoder lanes the engine runs for the radiosonde band, behind one
//! interface: the engine hands the wide I/Q to [`SondeBank::on_wide_iq`] and
//! drains one merged [`SondeStatus`] from [`SondeBank::poll`].
//!
//! Without scan there is one lane, on the channel the operator chose. With
//! scan, the bank looks at the spectrum of the window every couple of seconds
//! ([`crate::scan`]), starts a lane on each sonde carrier it finds (a few at a
//! time) and drops a lane whose carrier has gone, keeping the sondes that lane
//! heard in the merged list.

use sdroxide_dsp::{Complex32, ComplexResampler, Ddc};
use sdroxide_types::{SondeSettings, SondeStatus, SondeTrack};

use crate::SONDE_LANE_RATE_HZ;
use crate::controller::SondeController;
use crate::scan::{Accumulator, LaneBook, merge};

/// How often the window's spectrum is looked at for new sondes, seconds.
pub const SCAN_EVERY_S: i64 = 2;
/// Blocks that must have been averaged before a look is trusted.
const MIN_BLOCKS: u32 = 8;
/// A block of spectrum is taken after this many seconds of I/Q.
const BLOCK_EVERY_S: f64 = 0.03;

/// One channel being decoded.
struct Lane {
    freq_hz: f64,
    ddc: Ddc,
    rs: Option<ComplexResampler>,
    buf: Vec<Complex32>,
    res_buf: Vec<Complex32>,
    ctl: SondeController,
    /// Frames this lane's decoder has counted so far.
    frames: u64,
    level_dbfs: f32,
}

impl Lane {
    fn new(in_rate: f64, center_hz: f64, freq_hz: f64, cfg: SondeSettings) -> Self {
        let mut ddc = Ddc::new(in_rate, SONDE_LANE_RATE_HZ);
        ddc.set_offset_hz(freq_hz - center_hz);
        let rs = ComplexResampler::new(ddc.out_rate(), SONDE_LANE_RATE_HZ);
        Self {
            freq_hz,
            ddc,
            rs,
            buf: Vec::new(),
            res_buf: Vec::new(),
            ctl: SondeController::new(SondeSettings { frequency_hz: freq_hz, ..cfg }),
            frames: 0,
            level_dbfs: -120.0,
        }
    }

    fn feed(&mut self, iq: &[Complex32]) {
        self.buf.clear();
        self.ddc.process(iq, &mut self.buf);
        let lane = match self.rs.as_mut() {
            Some(rs) => {
                self.res_buf.clear();
                rs.push(&self.buf, &mut self.res_buf);
                &self.res_buf
            }
            None => &self.buf,
        };
        self.ctl.on_rx_iq(lane);
    }
}

/// The spectrum watcher and the channels it has started.
struct Scanner {
    acc: Accumulator,
    book: LaneBook,
    /// Samples of I/Q to let pass before taking the next block.
    gap: usize,
    /// A block being collected.
    cap: Vec<Complex32>,
    last_eval: i64,
}

pub struct SondeBank {
    in_rate: f64,
    center_hz: f64,
    cfg: SondeSettings,
    lanes: Vec<Lane>,
    scanner: Option<Scanner>,
    /// Every sonde heard, merged across lanes, including lanes since dropped.
    tracks: Vec<SondeTrack>,
    /// Frames counted by lanes that have since been dropped.
    frames_gone: u64,
    last_emit: i64,
    /// The merged list changed without a lane reporting (a lane was dropped or
    /// the mode changed): publish it at the next poll.
    dirty: bool,
}

impl SondeBank {
    pub fn new(in_rate: f64, center_hz: f64, cfg: SondeSettings) -> Self {
        let mut b = Self {
            in_rate,
            center_hz,
            cfg,
            lanes: Vec::new(),
            scanner: None,
            tracks: Vec::new(),
            frames_gone: 0,
            last_emit: i64::MIN,
            dirty: false,
        };
        b.apply_mode();
        b
    }

    /// The channels now being decoded, in Hz.
    pub fn channels(&self) -> Vec<f64> {
        self.lanes.iter().map(|l| l.freq_hz).collect()
    }

    /// Start or stop what the settings call for: one lane on the chosen channel,
    /// or a scanner and no lane until it finds one.
    fn apply_mode(&mut self) {
        self.drop_all_lanes();
        if self.cfg.scan {
            self.scanner = Some(Scanner {
                acc: Accumulator::new(self.in_rate),
                book: LaneBook::default(),
                gap: 0,
                cap: Vec::new(),
                last_eval: i64::MIN,
            });
        } else {
            self.scanner = None;
            self.lanes.push(Lane::new(
                self.in_rate,
                self.center_hz,
                self.cfg.frequency_hz,
                self.cfg,
            ));
        }
    }

    fn drop_all_lanes(&mut self) {
        self.absorb();
        self.dirty = true;
        for l in self.lanes.drain(..) {
            self.frames_gone += l.frames;
        }
    }

    /// Take what every lane has reported into the merged list.
    fn absorb(&mut self) -> bool {
        let mut any = false;
        for l in &mut self.lanes {
            if let Some(st) = l.ctl.poll() {
                any = true;
                l.frames = st.frames;
                l.level_dbfs = st.level_dbfs;
                merge(&mut self.tracks, st.sondes);
            }
        }
        any
    }

    /// New settings: a change of mode or channel restarts what it must.
    pub fn set_config(&mut self, cfg: SondeSettings) {
        let was = self.cfg;
        self.cfg = cfg;
        if was.scan != cfg.scan {
            self.apply_mode();
        } else if !cfg.scan && (was.frequency_hz - cfg.frequency_hz).abs() >= 1.0 {
            if let Some(l) = self.lanes.first_mut() {
                l.freq_hz = cfg.frequency_hz;
                l.ddc.set_offset_hz(cfg.frequency_hz - self.center_hz);
                l.ctl.set_config(cfg);
            }
        } else {
            for l in &self.lanes {
                l.ctl.set_config(SondeSettings { frequency_hz: l.freq_hz, ..cfg });
            }
        }
    }

    /// The window moved: re-seat every mixer. In scan mode the lanes follow
    /// their carrier, so a moved window starts the look over.
    pub fn set_center(&mut self, center_hz: f64) {
        if (center_hz - self.center_hz).abs() < 1.0 {
            return;
        }
        self.center_hz = center_hz;
        if self.cfg.scan {
            self.apply_mode();
        } else if let Some(l) = self.lanes.first_mut() {
            l.ddc.set_offset_hz(l.freq_hz - center_hz);
        }
    }

    /// Realtime path: the wide I/Q, `now` in unix seconds.
    pub fn on_wide_iq(&mut self, iq: &[Complex32], now: i64) {
        // Nothing to decode, and nothing worth scanning, while the window is
        // parked off the sonde band (1090 MHz for ADS-B, the shortwave for HFDL).
        if !sdroxide_types::sonde_window_reaches_band(self.center_hz, self.in_rate) {
            return;
        }
        for l in &mut self.lanes {
            l.feed(iq);
        }
        let Some(sc) = self.scanner.as_mut() else { return };
        let n = sc.acc.block_len();
        // Collect one block now and then, not every sample: a look every couple
        // of seconds needs a few dozen blocks, not the whole stream.
        if sc.cap.is_empty() {
            if sc.gap > iq.len() {
                sc.gap -= iq.len();
            } else {
                sc.gap = 0;
                sc.cap.extend_from_slice(&iq[sc.gap.min(iq.len())..]);
            }
        } else {
            sc.cap.extend_from_slice(iq);
        }
        if sc.cap.len() >= n {
            sc.acc.push(&sc.cap[..n]);
            sc.cap.clear();
            sc.gap = (self.in_rate * BLOCK_EVERY_S) as usize;
        }
        if sc.last_eval == i64::MIN {
            sc.last_eval = now;
        }
        if now - sc.last_eval >= SCAN_EVERY_S {
            sc.last_eval = now;
            if sc.acc.blocks() >= MIN_BLOCKS {
                let carriers = sc.acc.carriers(self.in_rate, self.center_hz);
                let change = sc.book.update(&carriers, now);
                for f in change.remove {
                    if let Some(i) = self.lanes.iter().position(|l| l.freq_hz == f) {
                        // Keep what it heard before it goes.
                        if let Some(st) = self.lanes[i].ctl.poll() {
                            merge(&mut self.tracks, st.sondes);
                            self.lanes[i].frames = st.frames;
                        }
                        self.frames_gone += self.lanes[i].frames;
                        self.lanes.remove(i);
                        self.dirty = true;
                    }
                }
                for f in change.add {
                    self.lanes.push(Lane::new(
                        self.in_rate,
                        self.center_hz,
                        f,
                        SondeSettings { frequency_hz: f, ..self.cfg },
                    ));
                }
            }
        }
    }

    /// The merged status, when something is new (or about once a second while
    /// there are no lanes, so the panel still shows the decoder is up).
    pub fn poll(&mut self, now: i64) -> Option<SondeStatus> {
        let fresh = self.absorb();
        if !fresh && !self.dirty && !(self.lanes.is_empty() && now != self.last_emit) {
            return None;
        }
        self.dirty = false;
        self.last_emit = now;
        let level = self.lanes.iter().map(|l| l.level_dbfs).fold(-120.0f32, f32::max);
        let frames = self.frames_gone + self.lanes.iter().map(|l| l.frames).sum::<u64>();
        Some(SondeStatus {
            running: self.cfg.enabled,
            level_dbfs: level,
            frames,
            sondes: self.tracks.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xng_mode_sonde::{modulate, whitening};

    const FRAME1: &str = "8635f44093df1a602c87e0fa0521e8943d9cef4c7a67393f6d39fb546461f2111b6447ab79a746c80350cda5344157f8c0c12234f46902220f792816174b313933303239331a00000300000a00002f0007322ce53e31991abf12dada3eb68468c16755d51c7a2a15310216060245f302000d08a31607821e08bb210219060243f302000000000000000000000000000000220d7c1e0807d03cdc071fd81ddb19d70a8d0eb602b60cb518d40692ff00ff00ff001c277d59b8d83301ff0f881f0f38f4fe18b283038735ff000000003eb8ff4947201e6e3aff55415f13fc6e005440440cf100009e9f7406f85800832b631719d70010bebc172a8b00000000000000000000000000000000000000000000a48b7b15366181193ef05d07e1245b1be0f721f801f60804107b0b76110000000000000000000000000000000000ecc7";

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    /// Two sondes in one 240 kS/s window, plus a little noise.
    fn window(rate: f64, offsets: &[f64]) -> Vec<Complex32> {
        let mut on_air = hex(FRAME1);
        whitening::xor_mask(&mut on_air, 0);
        let len = (rate * 0.8) as usize;
        let mut x = 0x9e37_79b9u32;
        let mut mix: Vec<Complex32> = (0..len)
            .map(|_| {
                let mut r = || {
                    x ^= x << 13;
                    x ^= x >> 17;
                    x ^= x << 5;
                    (x as f32 / u32::MAX as f32 - 0.5) * 0.02
                };
                Complex32::new(r(), r())
            })
            .collect();
        for &off in offsets {
            for (m, b) in mix.iter_mut().zip(modulate::burst_iq(&on_air, rate, off, 0.4)) {
                *m += b;
            }
        }
        mix
    }

    fn feed(b: &mut SondeBank, iq: &[Complex32], now: i64) {
        for chunk in iq.chunks(4096) {
            b.on_wide_iq(chunk, now);
        }
    }

    fn wait_for(
        b: &mut SondeBank,
        now: i64,
        pred: impl Fn(&SondeStatus) -> bool,
    ) -> Option<SondeStatus> {
        let mut last = None;
        for _ in 0..500 {
            if let Some(s) = b.poll(now) {
                let hit = pred(&s);
                last = Some(s);
                if hit {
                    return last;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        last
    }

    #[test]
    fn without_scan_there_is_one_lane_on_the_chosen_channel() {
        let cfg = SondeSettings { enabled: true, frequency_hz: 403_030_000.0, scan: false };
        let rate = 240_000.0;
        let mut b = SondeBank::new(rate, 403_000_000.0, cfg);
        assert_eq!(b.channels(), vec![403_030_000.0]);
        feed(&mut b, &window(rate, &[30_000.0]), 0);
        let s = wait_for(&mut b, 0, |s| s.frames >= 1).expect("a status");
        assert_eq!(s.sondes[0].serial, "K1930293");
        // Retune the channel: the same lane follows.
        b.set_config(SondeSettings { frequency_hz: 403_050_000.0, ..cfg });
        assert_eq!(b.channels(), vec![403_050_000.0]);
    }

    #[test]
    fn scan_finds_two_sondes_and_starts_a_lane_on_each() {
        let cfg = SondeSettings { enabled: true, frequency_hz: 403_000_000.0, scan: true };
        let rate = 240_000.0;
        let mut b = SondeBank::new(rate, 403_000_000.0, cfg);
        assert!(b.channels().is_empty(), "no lane until a carrier is found");
        let w = window(rate, &[30_000.0, -50_000.0]);
        // The look takes a couple of seconds' worth of the clock.
        for second in 0..4 {
            feed(&mut b, &w, second);
        }
        let ch = b.channels();
        assert_eq!(ch.len(), 2, "{ch:?}");
        assert!(ch.iter().any(|f| (f - 403_030_000.0).abs() < 1_500.0), "{ch:?}");
        assert!(ch.iter().any(|f| (f - 402_950_000.0).abs() < 1_500.0), "{ch:?}");
        // Now that lanes exist, the next burst is decoded by them.
        feed(&mut b, &w, 5);
        let s = wait_for(&mut b, 5, |s| s.frames >= 2).expect("a status");
        assert!(s.frames >= 2, "one frame per lane, got {}", s.frames);
        assert_eq!(s.sondes.len(), 1, "the same serial on both lanes is one sonde");
    }

    #[test]
    fn leaving_scan_returns_to_one_lane_and_keeps_what_was_heard() {
        let on = SondeSettings { enabled: true, frequency_hz: 403_030_000.0, scan: true };
        let rate = 240_000.0;
        let mut b = SondeBank::new(rate, 403_000_000.0, on);
        let w = window(rate, &[30_000.0]);
        for second in 0..4 {
            feed(&mut b, &w, second);
        }
        feed(&mut b, &w, 5);
        wait_for(&mut b, 5, |s| s.frames >= 1).expect("a status");
        b.set_config(SondeSettings { scan: false, ..on });
        assert_eq!(b.channels(), vec![403_030_000.0]);
        let s = b.poll(6).expect("a status");
        assert_eq!(s.sondes.len(), 1, "the sonde heard in scan mode is still listed");
        assert!(s.frames >= 1, "and its frames are still counted");
    }

    #[test]
    fn a_window_off_the_sonde_band_is_not_scanned() {
        let cfg = SondeSettings { enabled: true, frequency_hz: 403e6, scan: true };
        let w = window(240_000.0, &[30_000.0]);
        // Parked on 1090 MHz (ADS-B): the spectrum is not even averaged.
        let mut off = SondeBank::new(240_000.0, 1_090e6, cfg);
        feed(&mut off, &w, 0);
        assert_eq!(off.scanner.as_ref().map(|s| s.acc.blocks()), Some(0));
        // On the band it is.
        let mut on = SondeBank::new(240_000.0, 403e6, cfg);
        feed(&mut on, &w, 0);
        assert!(on.scanner.as_ref().is_some_and(|s| s.acc.blocks() > 0));
    }

    #[test]
    fn with_no_lane_a_status_still_comes_about_once_a_second() {
        let cfg = SondeSettings { enabled: true, frequency_hz: 403e6, scan: true };
        let mut b = SondeBank::new(240_000.0, 403e6, cfg);
        assert!(b.poll(10).is_some_and(|s| s.running && s.sondes.is_empty()));
        assert!(b.poll(10).is_none(), "once per second");
        assert!(b.poll(11).is_some());
    }
}
