//! Threading wrapper around the decoder, mirroring
//! [`sdroxide_ism::IsmController`]: the realtime engine thread ships window-rate
//! I/Q over a bounded channel (dropping on backpressure) and drains results
//! non-blocking through [`AdsbController::poll`]. The correlator, the slicer and
//! the aircraft table all run on the worker.
//!
//! Control traffic rides a separate unbounded channel for the same reason it
//! does in the skimmer and the ISM decoder: a retune or an off-switch must never
//! be dropped behind a backed-up I/Q queue, which is exactly when it is most
//! likely to be sent.
//!
//! # Why the whole table goes out every time
//!
//! Two hundred and forty samples a second reach this decoder per aircraft in
//! view, and a status message every half second carries all of them at once.
//! Forwarding decodes individually would be a message per squitter — several
//! hundred a second on a busy sector — each of which the panel would have to
//! fold into a table it is already keeping. The snapshot does that folding once,
//! on the worker, and has the property every snapshot has: a dropped one costs
//! nothing, because the next carries the same information.
//!
//! # Beast in and out
//!
//! Every frame the demodulator accepts can also go out as Beast (see
//! [`crate::beast_net`]), and the table can be fed from somebody else's Beast
//! stream instead of from I/Q. The two are kept apart by where they attach:
//! the output is written in [`Worker::process`], which only I/Q reaches, and
//! the input arrives in [`Worker::absorb_net`], which never writes out. A
//! station whose frames come back to it through an aggregator therefore shows
//! them and does not send them round again.
//!
//! The status goes out on its own clock rather than on the I/Q's, because a
//! Beast feed needs no receiver: with nothing arriving from a front end there
//! would otherwise be nothing to say, and the panel would wait for ever.
//!
//! # The stream clock
//!
//! The worker counts samples rather than reading the wall clock for anything
//! that has to be monotonic and fine-grained — the CPR pairing window and the
//! derived turn rate. At a known sample rate that count *is* a clock, it cannot
//! jump, it does not care how the engine is scheduling blocks, and it makes the
//! decoder's arithmetic exactly reproducible from a recording. A Beast feed
//! has no samples to count, and takes the worker's monotonic clock instead —
//! the network's jitter is milliseconds against a ten-second pairing window.
//! The table is cleared when the feed changes, so the two clocks never meet in
//! one aircraft.

use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, Sender, bounded, select, tick, unbounded};
use sdroxide_dsp::Complex32 as C32;
use sdroxide_types::{AdsbFeed, AdsbSettings, AdsbStatus};
use tracing::{debug, info};

use crate::beast::{self, BeastFrame};
use crate::beast_net::{BeastClient, BeastOut, FeedState};
use crate::demod::Demod;
use crate::frame::{self, Rejected};
use crate::track::Tracker;

/// How often a snapshot goes out.
///
/// Twice a second: an airliner's position updates about that often, so a faster
/// tick would re-send a table that had not moved, and a slower one would make a
/// target visibly step rather than track.
const EMIT_INTERVAL: Duration = Duration::from_millis(500);

/// What the engine drains each tick.
pub enum AdsbAction {
    Status(Box<AdsbStatus>),
}

/// Realtime data, dropped on backpressure.
struct Iq(Vec<C32>);

/// Control traffic, never dropped.
enum Ctl {
    Window {
        center_hz: f64,
        rate_hz: f64,
    },
    Config(AdsbSettings),
    /// Where the receiver is, for the surface-position reference.
    Home(Option<(f64, f64)>),
    /// Where the demodulator's frames also go, as Beast.
    Out(Option<BeastOut>),
    /// Where the table's frames come from, and the Beast address if a stream.
    Feed(AdsbFeed, Option<String>),
    /// Where the station's learned range is kept, see [`crate::prior`].
    Prior(Option<std::path::PathBuf>),
    Stop,
}

pub struct AdsbController {
    iq_tx: Sender<Iq>,
    ctl_tx: Sender<Ctl>,
    res_rx: Receiver<AdsbAction>,
    worker: Option<JoinHandle<()>>,
}

impl AdsbController {
    /// `window_rate_hz` is the rate of the I/Q the engine will feed, and
    /// `window_center_hz` the absolute RF frequency it is centred on.
    pub fn new(window_center_hz: f64, window_rate_hz: f64, cfg: AdsbSettings) -> AdsbController {
        let (iq_tx, iq_rx) = bounded::<Iq>(64);
        let (ctl_tx, ctl_rx) = unbounded::<Ctl>();
        let (res_tx, res_rx) = unbounded::<AdsbAction>();

        let worker = std::thread::Builder::new()
            .name("sdroxide-adsb".into())
            .spawn(move || {
                let (net_tx, net_rx) = bounded::<Vec<BeastFrame>>(256);
                let mut w = Worker::new(window_center_hz, window_rate_hz, cfg);
                w.net_tx = Some(net_tx);
                let emit = tick(EMIT_INTERVAL);
                loop {
                    select! {
                        recv(ctl_rx) -> msg => match msg {
                            Ok(Ctl::Window { center_hz, rate_hz }) => {
                                w.set_window(center_hz, rate_hz);
                            }
                            Ok(Ctl::Config(next)) => w.set_config(next),
                            Ok(Ctl::Home(h)) => w.set_home(h),
                            Ok(Ctl::Out(o)) => w.out = o,
                            Ok(Ctl::Feed(f, addr)) => w.set_feed(f, addr),
                            Ok(Ctl::Prior(path)) => w.set_prior(path),
                            Ok(Ctl::Stop) | Err(_) => break,
                        },
                        recv(iq_rx) -> msg => match msg {
                            Ok(Iq(iq)) => w.process(&iq),
                            Err(_) => break,
                        },
                        recv(net_rx) -> msg => {
                            if let Ok(frames) = msg {
                                w.absorb_net(&frames);
                            }
                        },
                        recv(emit) -> _ => {
                            w.save_prior(false);
                            if res_tx.send(AdsbAction::Status(Box::new(w.status()))).is_err() {
                                break;
                            }
                        },
                    }
                }
                w.save_prior(true);
            })
            .expect("spawn adsb worker");

        AdsbController { iq_tx, ctl_tx, res_rx, worker: Some(worker) }
    }

    /// Realtime path: hand a block of window-rate I/Q to the worker.
    /// Non-blocking; drops the block if the worker is behind.
    pub fn on_rx_iq(&self, iq: &[C32]) {
        let _ = self.iq_tx.try_send(Iq(iq.to_vec()));
    }

    /// The window moved — the front end retuned, or changed rate.
    ///
    /// The aircraft table survives: a receiver nudged a hundred kilohertz is
    /// still looking at the same sky, and throwing away every target for it
    /// would be a worse answer than a second of missed frames.
    pub fn set_window(&self, center_hz: f64, rate_hz: f64) {
        let _ = self.ctl_tx.send(Ctl::Window { center_hz, rate_hz });
    }

    pub fn set_config(&self, cfg: AdsbSettings) {
        let _ = self.ctl_tx.send(Ctl::Config(cfg));
    }

    /// Tell the decoder where the receiver is, so a surface position has a
    /// reference to be decoded against.
    pub fn set_home(&self, home: Option<(f64, f64)>) {
        let _ = self.ctl_tx.send(Ctl::Home(home));
    }

    /// Judge first fixes by the station's learned range kept in `path`, and
    /// teach it; `None`, every first fix is taken. Every decoder given the
    /// same path shares one prior.
    pub fn set_prior_file(&self, path: Option<std::path::PathBuf>) {
        let _ = self.ctl_tx.send(Ctl::Prior(path));
    }

    /// Also send every frame the demodulator accepts to `out`, as Beast; `None`
    /// stops. Frames from a Beast feed are never sent — see the module notes.
    pub fn set_out(&self, out: Option<BeastOut>) {
        let _ = self.ctl_tx.send(Ctl::Out(out));
    }

    /// Fill the table from this receiver, or from the Beast output at `addr`.
    /// A change of feed clears the table.
    pub fn set_feed(&self, feed: AdsbFeed, addr: Option<String>) {
        let _ = self.ctl_tx.send(Ctl::Feed(feed, addr));
    }

    /// Drain whatever the worker has produced since the last poll. Non-blocking.
    pub fn poll(&self) -> Vec<AdsbAction> {
        let mut out = Vec::new();
        while let Ok(a) = self.res_rx.try_recv() {
            out.push(a);
        }
        out
    }
}

impl Drop for AdsbController {
    fn drop(&mut self) {
        let _ = self.ctl_tx.send(Ctl::Stop);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }
}

struct Worker {
    demod: Demod,
    tracker: Tracker,
    /// The station's learned range and the file it is kept in.
    prior: Option<(crate::prior::SharedPrior, std::path::PathBuf)>,
    /// When the prior was last written.
    prior_saved: Instant,
    center_hz: f64,
    rate_hz: f64,
    /// Samples seen, which at a known rate is the stream clock.
    samples: u64,
    frames: u64,
    bad_crc: u64,
    unmatched: u64,
    /// Frames accepted after a one-bit repair (counted in `frames` too).
    repaired: u64,
    /// Scratch, kept so a block does not allocate.
    cand: Vec<crate::demod::Candidate>,
    cfg: AdsbSettings,
    home: Option<(f64, f64)>,
    feed: AdsbFeed,
    /// The demodulator's frames, as Beast, to whoever is listening.
    out: Option<BeastOut>,
    /// The Beast stream the table is filled from, when the feed is Beast.
    client: Option<BeastClient>,
    /// Where a [`BeastClient`] delivers; the worker loop holds the other end.
    net_tx: Option<Sender<Vec<BeastFrame>>>,
    /// The clock a Beast feed is timed by.
    t0: Instant,
}

impl Worker {
    fn new(center_hz: f64, rate_hz: f64, cfg: AdsbSettings) -> Worker {
        info!(rate = rate_hz, center = center_hz, "ADS-B decoder started");
        Worker {
            demod: Demod::new(rate_hz),
            tracker: Tracker::new(cfg),
            center_hz,
            rate_hz,
            samples: 0,
            frames: 0,
            bad_crc: 0,
            unmatched: 0,
            repaired: 0,
            cand: Vec::new(),
            cfg,
            home: None,
            feed: AdsbFeed::Radio,
            out: None,
            client: None,
            net_tx: None,
            t0: Instant::now(),
            prior: None,
            prior_saved: Instant::now(),
        }
    }

    /// Switch where the table is filled from. A new feed starts a new table
    /// and new counts: the two feeds keep time by different clocks, and the
    /// counts describe one source or the other, never a sum.
    fn set_feed(&mut self, feed: AdsbFeed, addr: Option<String>) {
        if feed != self.feed {
            self.feed = feed;
            self.tracker = Tracker::new(self.cfg);
            self.tracker.set_home(self.home);
            self.tracker.set_prior(self.prior.as_ref().map(|(p, _)| p.clone()));
            (self.frames, self.bad_crc, self.unmatched, self.repaired) = (0, 0, 0, 0);
            info!(?feed, "ADS-B feed");
        }
        let want = if feed == AdsbFeed::Beast { addr } else { None };
        if self.client.as_ref().map(|c| c.addr().to_string()) != want {
            self.client = None;
            if let (Some(a), Some(tx)) = (want, self.net_tx.clone()) {
                self.client = Some(BeastClient::start(&a, tx));
            }
        }
    }

    /// Frames from a Beast stream, into the same table the demodulator fills.
    ///
    /// They are checked as the demodulator's are — a surveillance reply is
    /// believed only for an aircraft already heard — and they go nowhere else:
    /// above all not to [`Self::out`], or a station subscribed to an
    /// aggregator that merges its own output would send its frames round for
    /// ever.
    fn absorb_net(&mut self, frames: &[BeastFrame]) {
        if self.feed != AdsbFeed::Beast {
            return;
        }
        let now = unix_now();
        let mono = self.t0.elapsed().as_secs_f64();
        for f in frames {
            if f.bytes.len() == 2 {
                continue; // Mode A/C: no address, nothing for the table.
            }
            match frame::accept(&f.bytes, |icao| self.tracker.knows(icao, now)) {
                Ok(acc) => {
                    let mut bytes = f.bytes.clone();
                    if let frame::Accepted::Repaired { bit, .. } = &acc {
                        bytes[*bit / 8] ^= 0x80 >> (*bit % 8);
                        self.repaired += 1;
                    }
                    self.frames += 1;
                    // `readsb` always reports a level; a sender that does not
                    // gets the floor of the scale rather than a made-up one.
                    let rssi = beast::signal_to_dbfs(f.signal).unwrap_or(-48.1);
                    self.tracker.absorb(&acc, &bytes, rssi, None, now, mono);
                }
                Err(Rejected::BadCrc | Rejected::Malformed) => self.bad_crc += 1,
                Err(Rejected::Unmatched) => self.unmatched += 1,
                Err(Rejected::Unsupported) => {}
            }
        }
    }

    fn set_window(&mut self, center_hz: f64, rate_hz: f64) {
        self.center_hz = center_hz;
        if (rate_hz - self.rate_hz).abs() > 1.0 {
            // The demodulator's whole geometry is derived from the rate, so a
            // new rate is a new demodulator. The aircraft table is not touched.
            self.demod = Demod::new(rate_hz);
            self.rate_hz = rate_hz;
            info!(rate = rate_hz, center = center_hz, "ADS-B window rebuilt");
        }
    }

    fn set_config(&mut self, cfg: AdsbSettings) {
        self.cfg = cfg;
        self.tracker.set_config(cfg);
    }

    fn set_home(&mut self, home: Option<(f64, f64)>) {
        self.home = home;
        self.tracker.set_home(home);
    }

    fn set_prior(&mut self, path: Option<std::path::PathBuf>) {
        self.save_prior(true);
        self.prior = path.map(|f| (crate::prior::open(&f, unix_now()), f));
        self.tracker.set_prior(self.prior.as_ref().map(|(p, _)| p.clone()));
    }

    /// Write the prior every five minutes, or now if `now` — a crash then
    /// costs at most five minutes of learning.
    fn save_prior(&mut self, now: bool) {
        if !now && self.prior_saved.elapsed() < Duration::from_secs(300) {
            return;
        }
        self.prior_saved = Instant::now();
        if let Some((p, path)) = &self.prior
            && let Ok(g) = p.lock()
            && let Err(e) = crate::prior::save(&g, path)
        {
            tracing::warn!(path = %path.display(), "saving the ADS-B range prior: {e}");
        }
    }

    fn process(&mut self, iq: &[C32]) {
        if self.feed != AdsbFeed::Radio {
            return;
        }
        self.cand.clear();
        self.demod.push(iq, &mut self.cand);
        let now = unix_now();
        let mono = self.samples as f64 / self.rate_hz.max(1.0);
        // One timestamp for the block: the demodulator does not say where in
        // it a frame began, and nothing that reads this output needs finer.
        let stamp = beast::timestamp_from_samples(self.samples, self.rate_hz);
        self.samples += iq.len() as u64;
        let mut wire = Vec::new();

        // `cand` is taken so the tracker can be borrowed mutably alongside it;
        // it goes straight back, so the buffer is still reused.
        let cand = std::mem::take(&mut self.cand);
        // What this block has already been believed to say. A strong burst is
        // resolved from more than one alignment, and the demodulator drops
        // only the copies that came out bit-identical — a repaired copy of a
        // burst another alignment read cleanly differs in that one bit, and
        // is the same message.
        let mut believed: Vec<Vec<u8>> = Vec::new();
        for c in &cand {
            match frame::accept(&c.bytes, |icao| self.tracker.knows(icao, now)) {
                Ok(acc) => {
                    // The table keeps the frame as it was believed, so a
                    // repaired one is shown with its bit put right.
                    let mut bytes = c.bytes.clone();
                    if let frame::Accepted::Repaired { bit, .. } = &acc {
                        bytes[*bit / 8] ^= 0x80 >> (*bit % 8);
                    }
                    if believed.contains(&bytes) {
                        continue;
                    }
                    self.frames += 1;
                    if matches!(acc, frame::Accepted::Repaired { .. }) {
                        self.repaired += 1;
                    }
                    let carrier =
                        self.demod.carrier_of(c).map(|off| self.center_hz + f64::from(off));
                    self.tracker.absorb(&acc, &bytes, c.rssi_dbfs, carrier, now, mono);
                    if self.out.is_some() {
                        let sig = beast::dbfs_to_signal(c.rssi_dbfs);
                        beast::encode(&mut wire, stamp, sig, &bytes);
                    }
                    believed.push(bytes);
                }
                Err(Rejected::BadCrc | Rejected::Malformed) => self.bad_crc += 1,
                Err(Rejected::Unmatched) => self.unmatched += 1,
                Err(Rejected::Unsupported) => {}
            }
        }
        self.cand = cand;
        if let Some(out) = self.out.as_ref()
            && !wire.is_empty()
        {
            out.send(wire);
        }
    }

    fn status(&mut self) -> AdsbStatus {
        let now = unix_now();
        debug!(frames = self.frames, repaired = self.repaired, "ADS-B frames");
        self.tracker.expire(now);
        self.tracker.clock(self.samples as f64 / self.rate_hz.max(1.0));
        AdsbStatus {
            aircraft: self.tracker.snapshot(),
            unavailable: None,
            degraded: None,
            suggest_center_hz: None,
            window_center_hz: self.center_hz,
            window_rate_hz: self.rate_hz,
            preambles: self.demod.preambles,
            frames: self.frames,
            bad_crc: self.bad_crc,
            unmatched: self.unmatched,
            feed: self.feed,
            feed_state: (self.feed == AdsbFeed::Beast).then(|| match self.client.as_ref() {
                None => "no Beast address set".to_string(),
                Some(c) => match c.state() {
                    FeedState::Connecting => format!("{} · connecting", c.addr()),
                    FeedState::Connected => format!("{} · connected", c.addr()),
                    FeedState::Down(why) => format!("{} · down: {why}", c.addr()),
                },
            }),
        }
    }
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap()).collect()
    }

    /// A repaired frame counts once, and only for an aircraft already heard:
    /// before the first clean squitter the damaged one is a bad frame; after
    /// it, the damaged one is repaired; and a damaged copy beside a clean copy
    /// of the same message in one block is the same message, not a second one.
    #[test]
    fn a_repaired_frame_counts_once_and_only_for_a_known_aircraft() {
        let rate = 2_400_000.0;
        let even = hex("8D40621D58C382D690C8AC2863A7");
        let odd = hex("8D40621D58C386435CC412692AD6");
        let mut odd_bad = odd.clone();
        odd_bad[9] ^= 0x04;
        let burst = |m: &[u8], seed| crate::demod::modulate(m, rate, 0.5, 0.0, seed);
        let mut w = Worker::new(1_090_000_000.0, rate, AdsbSettings::default());
        w.process(&vec![C32::new(0.0, 0.0); 4096]);

        w.process(&burst(&odd_bad, 1));
        assert_eq!((w.frames, w.repaired, w.bad_crc > 0), (0, 0, true), "nobody known yet");

        w.process(&burst(&even, 2));
        assert_eq!((w.frames, w.repaired), (1, 0));

        w.process(&burst(&odd_bad, 3));
        assert_eq!((w.frames, w.repaired), (2, 1), "repaired for a known aircraft");

        let mut both = burst(&odd, 4);
        both.extend(burst(&odd_bad, 5));
        w.process(&both);
        assert_eq!((w.frames, w.repaired), (3, 1), "a damaged copy of a clean frame is no frame");
    }

    const EVEN: &str = "8D40621D58C382D690C8AC2863A7";
    const ODD: &str = "8D40621D58C386435CC412692AD6";

    fn beast_frame(m: &str) -> BeastFrame {
        BeastFrame { timestamp: 0, signal: 128, bytes: hex(m), receiver: None }
    }

    fn sent(rx: &Receiver<Vec<u8>>) -> Vec<BeastFrame> {
        let mut p = beast::Parser::new();
        rx.try_iter().flat_map(|b| p.push(&b)).collect()
    }

    /// The loop barrier. Frames from a Beast feed fill the table and are not
    /// sent out, so a station subscribed to an aggregator that merges its own
    /// output shows its frames once and does not send them round again.
    #[test]
    fn frames_from_a_beast_feed_fill_the_table_and_never_go_out() {
        let (out, rx) = BeastOut::capture();
        let mut w = Worker::new(1_090_000_000.0, 2_400_000.0, AdsbSettings::default());
        w.out = Some(out);
        w.set_feed(AdsbFeed::Beast, None);
        w.absorb_net(&[beast_frame(EVEN), beast_frame(ODD)]);
        assert_eq!(w.frames, 2);
        let ac = w.tracker.snapshot();
        assert_eq!(ac.len(), 1);
        assert!(ac[0].lat.is_some(), "an even/odd pair over Beast gives a position");
        assert!(sent(&rx).is_empty(), "nothing that came in went out");
    }

    /// What the demodulator accepts goes out as Beast — a repaired frame with
    /// its bit put right, so the reader's own check passes — and a Beast
    /// reader gets back the message that was on the air.
    #[test]
    fn the_demodulators_frames_go_out_as_beast_repaired_ones_put_right() {
        let rate = 2_400_000.0;
        let (out, rx) = BeastOut::capture();
        let mut w = Worker::new(1_090_000_000.0, rate, AdsbSettings::default());
        w.out = Some(out);
        let burst = |m: &[u8], seed| crate::demod::modulate(m, rate, 0.5, 0.0, seed);
        let mut odd_bad = hex(ODD);
        odd_bad[9] ^= 0x04;
        w.process(&vec![C32::new(0.0, 0.0); 4096]);
        w.process(&burst(&hex(EVEN), 1));
        w.process(&burst(&odd_bad, 2));
        let got = sent(&rx);
        assert_eq!(
            got.iter().map(|f| f.bytes.clone()).collect::<Vec<_>>(),
            vec![hex(EVEN), hex(ODD)]
        );
        assert!(got[0].signal > 0);
        assert!(got[1].timestamp > got[0].timestamp, "the block clock moves on");
    }

    /// A Beast feed needs no receiver: with no I/Q at all, the status still
    /// comes on its clock and carries the aircraft and the link's state.
    #[test]
    fn a_beast_feed_reports_without_any_iq() {
        use std::io::Write;
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        let c = AdsbController::new(1_090_000_000.0, 2_400_000.0, AdsbSettings::default());
        c.set_feed(AdsbFeed::Beast, Some(addr.clone()));
        let (mut s, _) = l.accept().unwrap();
        let mut wire = Vec::new();
        beast::encode(&mut wire, 1, 100, &hex(EVEN));
        beast::encode(&mut wire, 2, 100, &hex(ODD));
        s.write_all(&wire).unwrap();

        let t0 = Instant::now();
        let mut last = None;
        while t0.elapsed() < Duration::from_secs(5) {
            for AdsbAction::Status(st) in c.poll() {
                last = Some(st);
            }
            if last.as_ref().is_some_and(|st| !st.aircraft.is_empty()) {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let st = last.expect("a status came without any I/Q");
        assert_eq!(st.feed, AdsbFeed::Beast);
        assert_eq!(st.aircraft.len(), 1);
        assert_eq!(st.feed_state.as_deref(), Some(format!("{addr} · connected").as_str()));
    }
}
