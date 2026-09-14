//! The background data feed: one worker thread that fetches DONKI, NOAA SWPC
//! and SDO, and publishes a snapshot the UI reads under a short lock.
//!
//! Rules this is built around:
//!
//! * **The UI thread never touches the network.** Not a blocking call, not a
//!   JPEG decode — a 2048² decode is 150 ms, which is ten dropped frames.
//! * **The lock is never held across I/O.** The worker fetches and decodes
//!   first, then takes the lock only to swap a field.
//! * **Network happens only while the window is open.** The thread is started
//!   on first open and stops when the handle is dropped; a session that never
//!   opens the view makes no outbound connection at all.
//! * **Offline is a supported state, not an error.** The disk cache is
//!   published before the first request, and a failing source keeps serving its
//!   cached value with an honest age next to it.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};

use crate::aurora::{self, AuroraOval, HemisphericPower, KpPoint};
use crate::cache::{Cache, Validators};
use crate::clouds;
use crate::data::{SolarData, Source, SourceStatus, TLE_PERIOD_S};
use crate::donki::{self, CmeEvent, FlareEvent};
use crate::imagery::{self, SdoChannel, SunImage};
use crate::indices::{self};
use crate::satellites::{self, Satellite};
use crate::swpc::{self, ActiveRegion};

/// How far back events are requested, days.
const WINDOW_DAYS: i64 = 30;
/// Body size caps. The 4096 solar images are ~2 MB; the rest are far smaller.
const JSON_LIMIT: u64 = 16 * 1024 * 1024;
const IMAGE_LIMIT: u64 = 24 * 1024 * 1024;
/// The cloud mosaics are 280 kB each at the size asked for. This leaves room
/// for the service to change its mind about the encoding without leaving room
/// for a runaway body.
const CLOUD_LIMIT: u64 = 8 * 1024 * 1024;
/// How long a request may take in total.
const TIMEOUT: Duration = Duration::from_secs(25);
/// Worker wake interval; individual sources have their own, longer, cadences.
const TICK: Duration = Duration::from_secs(15);

pub enum FeedCmd {
    RefreshAll,
    SetChannel(SdoChannel),
    SetResolution(u32),
    /// Replace the operator's own element sets, as one three-line listing.
    ///
    /// Whole-set replacement rather than add/remove, because the settings
    /// dialog owns the list: sending what it now holds cannot drift out of step
    /// with it the way an incremental protocol can.
    SetCustomTles(String),
    /// Replace the subscribed element-set listings. Cached bodies are published
    /// at once and refetched on the element-set cadence.
    SetTleSubs(Vec<sdroxide_types::TleSubscription>),
}

/// A payload in the form it arrived in, for a relay that has to hand the same
/// bytes to somebody else.
///
/// [`SolarData`] holds *decoded* products — RGBA pixels and SGP4 constants —
/// which are both far larger than the original and not serialisable. The
/// server's browser relay needs the original JPEG and the original element set
/// instead, so the worker offers them here rather than making anyone re-encode
/// or re-fetch. Only sources whose wire form is the raw bytes appear.
pub enum RawUpdate {
    Sun {
        channel: SdoChannel,
        fetched_unix: i64,
        jpeg: Vec<u8>,
    },
    /// An element set: `geo` distinguishes QO-100 from the amateur list.
    Tle {
        geo: bool,
        text: String,
    },
    /// One channel of the cloud mosaic, as the PNG it arrived as. The decoded
    /// field is a megabyte of planes; the picture it came from is a quarter of
    /// that, and the receiver has the same decoder.
    Clouds {
        band: clouds::Band,
        frame_unix: i64,
        fetched_unix: i64,
        png: Vec<u8>,
    },
}

/// Handle to the worker. Dropping it stops the thread, which is what confines
/// network activity to the window's lifetime.
pub struct SolarFeed {
    shared: Arc<Mutex<SolarData>>,
    tx: Sender<FeedCmd>,
}

impl SolarFeed {
    /// Start the worker. `wake` is called after every published change — the UI
    /// passes a closure that repaints the solar viewport.
    pub fn start(
        channel: SdoChannel,
        resolution: u32,
        wake: impl Fn() + Send + 'static,
    ) -> SolarFeed {
        SolarFeed::start_with_raw(channel, resolution, wake, None)
    }

    /// As [`SolarFeed::start`], but also publishing raw payloads to `raw_tx`.
    ///
    /// The channel must be unbounded or drained promptly: the worker never
    /// blocks on it, and a send that fails is simply dropped, so a stalled
    /// consumer costs freshness rather than the whole feed.
    pub fn start_with_raw(
        channel: SdoChannel,
        resolution: u32,
        wake: impl Fn() + Send + 'static,
        raw_tx: Option<Sender<RawUpdate>>,
    ) -> SolarFeed {
        let shared = Arc::new(Mutex::new(SolarData::default()));
        let (tx, rx) = crossbeam_channel::unbounded();
        let worker_shared = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("solar-feed".into())
            .spawn(move || worker(worker_shared, rx, channel, resolution, wake, raw_tx))
            .map_err(|e| tracing::error!("could not start the solar feed thread: {e}"))
            .ok();
        SolarFeed { shared, tx }
    }

    pub fn data(&self) -> std::sync::MutexGuard<'_, SolarData> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A second handle on the snapshot, for code that cannot borrow the feed
    /// (the window's deferred render closure, which outlives any borrow).
    pub fn shared(&self) -> Arc<Mutex<SolarData>> {
        Arc::clone(&self.shared)
    }

    pub fn send(&self, cmd: FeedCmd) {
        // A full or closed channel means the worker is gone; nothing to do.
        let _ = self.tx.send(cmd);
    }
}

pub(crate) fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn worker(
    shared: Arc<Mutex<SolarData>>,
    rx: Receiver<FeedCmd>,
    mut channel: SdoChannel,
    mut resolution: u32,
    wake: impl Fn(),
    raw_tx: Option<Sender<RawUpdate>>,
) {
    // Never blocks the fetch loop: a relay that has stopped draining costs
    // freshness, not the feed.
    let raw = |u: RawUpdate| {
        if let Some(tx) = &raw_tx {
            let _ = tx.try_send(u);
        }
    };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let mut cache = Cache::open();
    // The operator's pasted sets and their subscriptions, kept apart so a
    // refresh of one does not disturb the other. The pasted ones are held as
    // the text they arrived as: a `Satellite` owns SGP4 constants and cannot be
    // cloned, and re-parsing a handful of them is nothing next to a fetch.
    let mut pasted_text = String::new();
    let mut subs: Vec<sdroxide_types::TleSubscription> = Vec::new();
    let mut subs_due_unix: i64 = 0;

    // Publish whatever is on disk before touching the network, so the window
    // has content the moment it opens — including with no connection at all.
    load_cached(&shared, &cache, channel, resolution, &raw);
    wake();

    loop {
        let now = now_unix();
        let mut changed = false;
        // Subscriptions ride the element-set cadence: TLEs are reissued a few
        // times a day at most, and a fresher fetch would buy nothing.
        if !subs.is_empty() && now >= subs_due_unix {
            subs_due_unix = now + TLE_PERIOD_S;
            let mut sats = satellites::parse_pasted_tles(&pasted_text);
            let mut status = Vec::with_capacity(subs.len());
            for sub in subs.iter() {
                let (v, st) = crate::tlesub::refresh(&agent, &mut cache, sub, now);
                sats.extend(v);
                status.push(st);
            }
            let mut d = shared.lock().unwrap_or_else(|e| e.into_inner());
            d.sats_custom = dedup_by_norad(sats);
            d.tle_subs = status;
            drop(d);
            raw(RawUpdate::Tle { geo: false, text: custom_tle_text(&cache, &pasted_text, &subs) });
            changed = true;
        }
        for src in Source::ALL {
            let due = {
                let d = shared.lock().unwrap_or_else(|e| e.into_inner());
                d.status(src).is_due(src, now)
            };
            if due {
                changed |= refresh(src, &agent, &mut cache, &shared, channel, resolution, &raw);
            }
        }
        if changed {
            wake();
        }

        match rx.recv_timeout(TICK) {
            Ok(FeedCmd::RefreshAll) => {
                let mut d = shared.lock().unwrap_or_else(|e| e.into_inner());
                // Clear the schedule so every source is due on the next
                // pass, but keep `last_ok_unix` so the age readouts stay honest
                // until the new data actually lands.
                for s in &mut d.status {
                    s.attempt_unix = None;
                    s.consecutive_failures = 0;
                }
            }
            Ok(FeedCmd::SetChannel(c)) => {
                channel = c;
                // Serve the cached image for the new channel immediately, then
                // let the normal cadence refresh it.
                load_cached_sun(&shared, &cache, channel, resolution, &raw);
                let mut d = shared.lock().unwrap_or_else(|e| e.into_inner());
                d.status[Source::Sun.index()] = SourceStatus::default();
                drop(d);
                wake();
            }
            Ok(FeedCmd::SetResolution(r)) => {
                resolution = r.clamp(512, 4096);
                let mut d = shared.lock().unwrap_or_else(|e| e.into_inner());
                d.status[Source::Sun.index()] = SourceStatus::default();
            }
            Ok(FeedCmd::SetCustomTles(text)) => {
                // Parse outside the lock; SGP4 constants for a long list are
                // not free, and the UI reads this mutex every frame.
                pasted_text = text;
                publish_custom(&shared, &cache, &pasted_text, &subs);
                raw(RawUpdate::Tle {
                    geo: false,
                    text: custom_tle_text(&cache, &pasted_text, &subs),
                });
                wake();
            }
            Ok(FeedCmd::SetTleSubs(v)) => {
                subs = v.into_iter().filter(|s| s.enabled && s.is_valid()).collect();
                // Serve the cached listings straight away — a subscription the
                // operator has had for weeks should not blank the sky while a
                // fetch it does not need runs.
                publish_custom(&shared, &cache, &pasted_text, &subs);
                raw(RawUpdate::Tle {
                    geo: false,
                    text: custom_tle_text(&cache, &pasted_text, &subs),
                });
                // ...then let the next loop pass decide whether to refetch,
                // from when each listing was last actually fetched.
                subs_due_unix =
                    subs.iter().map(|s| cache.fetched_at(&s.url) + TLE_PERIOD_S).min().unwrap_or(0);
                wake();
            }
            Err(RecvTimeoutError::Timeout) => {}
            // The handle was dropped: the window closed, so stop fetching.
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    tracing::debug!("solar feed thread stopped");
}

/// Publish the pasted sets plus whatever the subscriptions have on disk.
///
/// No network: this is the startup and settings-change path, and it must stay
/// usable with no connection at all.
fn publish_custom(
    shared: &Mutex<SolarData>,
    cache: &Cache,
    pasted_text: &str,
    subs: &[sdroxide_types::TleSubscription],
) {
    let mut sats = satellites::parse_pasted_tles(pasted_text);
    for sub in subs {
        sats.extend(crate::tlesub::cached(cache, sub));
    }
    let status = subs.iter().map(|s| crate::tlesub::cached_status(cache, s)).collect();
    let mut d = shared.lock().unwrap_or_else(|e| e.into_inner());
    d.sats_custom = dedup_by_norad(sats);
    d.tle_subs = status;
}

/// The operator's element sets — pasted and subscribed — as one listing, for
/// the browser relay.
///
/// The browser has no subscription machinery — it is fed decoded products over
/// the WebSocket — so it gets the union as one element-set listing and puts it
/// where the amateur group used to go. Concatenating three-line listings is
/// valid, and [`satellites::parse_tles`] drops the duplicates that overlapping
/// groups produce. The pasted sets go first, so they win those duplicates the
/// way they do in [`publish_custom`].
fn custom_tle_text(
    cache: &Cache,
    pasted_text: &str,
    subs: &[sdroxide_types::TleSubscription],
) -> String {
    let mut out = String::new();
    if !pasted_text.trim().is_empty() {
        out.push_str(pasted_text.trim_end());
        out.push('\n');
    }
    for sub in subs {
        if let Some(text) = crate::tlesub::cached_text(cache, sub) {
            out.push_str(text.trim_end());
            out.push('\n');
        }
    }
    out
}

/// Keep the first entry for each catalogue number.
///
/// Subscriptions overlap — the stations group and the cubesat group both carry
/// plenty of the same satellites — and a duplicate would be a second marker
/// drawn a few metres from the first. Pasted sets are put in first, so they win.
fn dedup_by_norad(mut sats: Vec<Satellite>) -> Vec<Satellite> {
    let mut seen = std::collections::HashSet::new();
    sats.retain(|s| seen.insert(s.norad_id));
    sats
}

fn load_cached(
    shared: &Mutex<SolarData>,
    cache: &Cache,
    channel: SdoChannel,
    resolution: u32,
    raw: &dyn Fn(RawUpdate),
) {
    let cmes = cache.read_string("cme.json").and_then(|s| donki::parse_cmes(&s).ok());
    let flares = cache.read_string("flr.json").and_then(|s| donki::parse_flares(&s).ok());
    let regions = cache.read_string("regions.json").and_then(|s| swpc::parse_regions(&s).ok());
    let flux = cache.read_string("flux.json").and_then(|s| indices::parse_flux(&s));
    let kp = cache.read_string("kp.json").and_then(|s| indices::parse_kp(&s));
    let xray = cache.read_string("xray.json").and_then(|s| indices::parse_xray(&s));
    let sondes = cache.read_string("ionosondes.json").map(|s| indices::parse_ionosondes(&s));
    // Keep the element set in its original form as well as parsed: a relay
    // forwards the text, and re-serialising SGP4 constants is not possible.
    let sats_geo_txt = cache.read_string("qo100.txt");
    let sats_geo = sats_geo_txt.as_deref().map(satellites::parse_tles);
    let oval = cache.read_string("ovation.json").and_then(|s| aurora::parse_ovation(&s));
    let power =
        cache.read_string("hemipower.txt").and_then(|s| aurora::parse_hemispheric_power(&s));
    let kp_forecast = cache.read_string("kpforecast.json").map(|s| aurora::parse_kp_forecast(&s));
    {
        let mut d = shared.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(v) = cmes {
            d.cmes = v;
        }
        if let Some(v) = flares {
            d.flares = v;
        }
        if let Some(v) = regions {
            d.regions = v;
        }
        d.weather.flux = flux;
        d.weather.geomagnetic = kp;
        d.weather.xray = xray;
        if let Some(v) = sondes {
            d.weather.ionosondes = v;
        }
        if let Some(v) = sats_geo {
            d.sats_geo = v;
        }
        if let Some(v) = oval {
            d.aurora = Some(Arc::new(v));
            d.aurora_gen += 1;
        }
        d.aurora_power = power;
        if let Some(v) = kp_forecast {
            d.kp_forecast = v;
        }
    }
    if let Some(text) = sats_geo_txt {
        raw(RawUpdate::Tle { geo: true, text });
    }
    load_cached_clouds(shared, cache, raw);
    load_cached_sun(shared, cache, channel, resolution, raw);
}

/// Put yesterday's weather on the globe before today's is asked for.
///
/// Stale cloud is worth showing — the overlay says how old it is — because the
/// alternative is a bare planet for the first few seconds of every launch, and
/// for the whole session when there is no network.
fn load_cached_clouds(shared: &Mutex<SolarData>, cache: &Cache, raw: &dyn Fn(RawUpdate)) {
    let mut any = false;
    for band in [clouds::Band::Longwave, clouds::Band::Visible] {
        let Some(bytes) = cache.read(band.cache_name()) else { continue };
        let fetched_unix = cache.fetched_at(&band.url());
        // The header that carried the frame's own hour is long gone, so it is
        // assumed — see `assumed_frame_unix`, which never guesses newer.
        let frame_unix = clouds::assumed_frame_unix(fetched_unix);
        // Decoding and the background estimate both happen out here, before the
        // lock: this is the expensive step.
        let Some(plane) = clouds::parse_plane(band, &bytes, frame_unix, fetched_unix) else {
            continue;
        };
        {
            let mut d = shared.lock().unwrap_or_else(|e| e.into_inner());
            match band {
                clouds::Band::Longwave => d.cloud_ir = Some(Arc::new(plane)),
                clouds::Band::Visible => d.cloud_vis = Some(Arc::new(plane)),
            }
        }
        raw(RawUpdate::Clouds { band, frame_unix, fetched_unix, png: bytes });
        any = true;
    }
    if any {
        shared.lock().unwrap_or_else(|e| e.into_inner()).rebuild_clouds();
    }
}

fn load_cached_sun(
    shared: &Mutex<SolarData>,
    cache: &Cache,
    channel: SdoChannel,
    resolution: u32,
    raw: &dyn Fn(RawUpdate),
) {
    let name = channel.cache_name(resolution);
    let Some(bytes) = cache.read(&name) else { return };
    let fetched_unix = cache.fetched_at(&channel.url(resolution));
    // Decode outside the lock: this is the expensive step.
    let Some(img) = imagery::decode(&bytes, channel, fetched_unix) else {
        return;
    };
    {
        let mut d = shared.lock().unwrap_or_else(|e| e.into_inner());
        d.sun = Some(Arc::new(img));
        d.sun_gen += 1;
    }
    raw(RawUpdate::Sun { channel, fetched_unix, jpeg: bytes });
}

/// Fetch one source. Returns whether anything the UI shows changed.
fn refresh(
    src: Source,
    agent: &ureq::Agent,
    cache: &mut Cache,
    shared: &Mutex<SolarData>,
    channel: SdoChannel,
    resolution: u32,
    raw: &dyn Fn(RawUpdate),
) -> bool {
    let now = now_unix();
    let (url, name, limit) = match src {
        Source::Sun => (channel.url(resolution), channel.cache_name(resolution), IMAGE_LIMIT),
        Source::Cme => (
            donki::cme_url(now - WINDOW_DAYS * 86_400, now + 86_400),
            "cme.json".to_string(),
            JSON_LIMIT,
        ),
        Source::Flare => (
            donki::flare_url(now - WINDOW_DAYS * 86_400, now + 86_400),
            "flr.json".to_string(),
            JSON_LIMIT,
        ),
        Source::Regions => (swpc::REGIONS_URL.to_string(), "regions.json".to_string(), JSON_LIMIT),
        Source::Flux => (indices::FLUX_URL.to_string(), "flux.json".to_string(), JSON_LIMIT),
        Source::Kp => (indices::KP_URL.to_string(), "kp.json".to_string(), JSON_LIMIT),
        Source::Xray => (indices::XRAY_URL.to_string(), "xray.json".to_string(), JSON_LIMIT),
        Source::Muf => {
            (indices::IONOSONDE_URL.to_string(), "ionosondes.json".to_string(), JSON_LIMIT)
        }
        Source::SatGeo => (satellites::QO100_URL.to_string(), "qo100.txt".to_string(), JSON_LIMIT),
        Source::Aurora => (aurora::OVATION_URL.to_string(), "ovation.json".to_string(), JSON_LIMIT),
        Source::AuroraPower => {
            (aurora::HEMI_POWER_URL.to_string(), "hemipower.txt".to_string(), JSON_LIMIT)
        }
        Source::KpForecast => {
            (aurora::KP_FORECAST_URL.to_string(), "kpforecast.json".to_string(), JSON_LIMIT)
        }
        Source::Clouds => (
            clouds::Band::Longwave.url(),
            clouds::Band::Longwave.cache_name().to_string(),
            CLOUD_LIMIT,
        ),
        Source::CloudsVis => (
            clouds::Band::Visible.url(),
            clouds::Band::Visible.cache_name().to_string(),
            CLOUD_LIMIT,
        ),
        Source::BandConditions => (
            indices::BAND_CONDITIONS_URL.to_string(),
            BAND_CONDITIONS_CACHE.to_string(),
            JSON_LIMIT,
        ),
    };

    match http_get(agent, &url, &cache.validators(&url), limit) {
        Ok(None) => {
            // 304: what we already have is current.
            cache.touch(&url, now);
            let mut d = shared.lock().unwrap_or_else(|e| e.into_inner());
            d.status[src.index()].record_ok(now);
            false
        }
        Ok(Some((bytes, validators, warning))) => {
            // What hour the picture is of, where the source says so. Only the
            // cloud mosaic does; for everything else the fetch time is the
            // observation time to within its own cadence.
            let frame_unix = warning.as_deref().and_then(clouds::frame_unix).unwrap_or(now);
            // Parse and decode before taking the lock.
            let parsed = parse(src, &bytes, channel, now, frame_unix);
            let ok = match &parsed {
                Parsed::None => false,
                _ => true,
            };
            if ok {
                cache.write(&name, &url, &bytes, Validators { fetched_unix: now, ..validators });
                // Forward the original bytes for the sources whose wire form is
                // the payload itself, before `bytes` is dropped.
                match src {
                    Source::Sun => {
                        raw(RawUpdate::Sun { channel, fetched_unix: now, jpeg: bytes.clone() })
                    }
                    Source::SatGeo => {
                        if let Ok(text) = std::str::from_utf8(&bytes) {
                            raw(RawUpdate::Tle { geo: true, text: text.to_string() });
                        }
                    }
                    Source::Clouds => raw(RawUpdate::Clouds {
                        band: clouds::Band::Longwave,
                        frame_unix,
                        fetched_unix: now,
                        png: bytes.clone(),
                    }),
                    Source::CloudsVis => raw(RawUpdate::Clouds {
                        band: clouds::Band::Visible,
                        frame_unix,
                        fetched_unix: now,
                        png: bytes.clone(),
                    }),
                    _ => {}
                }
            }
            let mut d = shared.lock().unwrap_or_else(|e| e.into_inner());
            match parsed {
                Parsed::Cmes(v) => d.cmes = v,
                Parsed::Flares(v) => d.flares = v,
                Parsed::Regions(v) => d.regions = v,
                Parsed::Sun(img) => {
                    d.sun = Some(Arc::new(img));
                    d.sun_gen += 1;
                }
                Parsed::Flux(v) => d.weather.flux = Some(v),
                Parsed::Kp(v) => d.weather.geomagnetic = Some(v),
                Parsed::Xray(v) => d.weather.xray = Some(v),
                Parsed::Ionosondes(v) => d.weather.ionosondes = v,
                Parsed::BandConditions(v) => d.weather.band_conditions = Some(v),
                Parsed::SatGeo(v) => d.sats_geo = v,
                Parsed::Aurora(v) => {
                    d.aurora = Some(Arc::new(v));
                    d.aurora_gen += 1;
                }
                Parsed::AuroraPower(v) => d.aurora_power = Some(v),
                Parsed::KpForecast(v) => d.kp_forecast = v,
                Parsed::Cloud(plane) => {
                    match plane.band {
                        clouds::Band::Longwave => d.cloud_ir = Some(Arc::new(plane)),
                        clouds::Band::Visible => d.cloud_vis = Some(Arc::new(plane)),
                    }
                    d.rebuild_clouds();
                }
                Parsed::None => {
                    d.status[src.index()]
                        .record_err(now, format!("{} returned unusable data", src.label()));
                    return false;
                }
            }
            d.status[src.index()].record_ok(now);
            true
        }
        Err(e) => {
            tracing::warn!("solar feed: {} fetch failed: {e}", src.label());
            let mut d = shared.lock().unwrap_or_else(|e| e.into_inner());
            d.status[src.index()].record_err(now, e);
            // The status line changed even though the data did not.
            true
        }
    }
}

enum Parsed {
    Cmes(Vec<CmeEvent>),
    Flares(Vec<FlareEvent>),
    Regions(Vec<ActiveRegion>),
    Sun(SunImage),
    Flux(indices::SolarFlux),
    Kp(indices::GeomagneticIndex),
    Xray(indices::XrayLevel),
    Ionosondes(Vec<indices::Ionosonde>),
    SatGeo(Vec<Satellite>),
    Aurora(AuroraOval),
    AuroraPower(HemisphericPower),
    KpForecast(Vec<KpPoint>),
    Cloud(clouds::Plane),
    BandConditions(indices::BandConditions),
    None,
}

fn parse(src: Source, bytes: &[u8], channel: SdoChannel, now: i64, frame_unix: i64) -> Parsed {
    match src {
        Source::Sun => match imagery::decode(bytes, channel, now) {
            Some(img) => Parsed::Sun(img),
            None => Parsed::None,
        },
        Source::Clouds | Source::CloudsVis => {
            let band =
                if src == Source::Clouds { clouds::Band::Longwave } else { clouds::Band::Visible };
            clouds::parse_plane(band, bytes, frame_unix, now).map_or(Parsed::None, Parsed::Cloud)
        }
        _ => {
            let Ok(text) = std::str::from_utf8(bytes) else { return Parsed::None };
            let parsed = match src {
                Source::Cme => donki::parse_cmes(text).map(Parsed::Cmes),
                Source::Flare => donki::parse_flares(text).map(Parsed::Flares),
                Source::Regions => swpc::parse_regions(text).map(Parsed::Regions),
                // These four return an Option rather than a Result: a feed that
                // is momentarily empty is normal, not an error.
                Source::Flux => {
                    return indices::parse_flux(text).map_or(Parsed::None, Parsed::Flux);
                }
                Source::Kp => return indices::parse_kp(text).map_or(Parsed::None, Parsed::Kp),
                Source::Xray => {
                    return indices::parse_xray(text).map_or(Parsed::None, Parsed::Xray);
                }
                Source::Muf => {
                    let v = indices::parse_ionosondes(text);
                    return if v.is_empty() { Parsed::None } else { Parsed::Ionosondes(v) };
                }
                Source::SatGeo => {
                    let v = satellites::parse_tles(text);
                    return if v.is_empty() { Parsed::None } else { Parsed::SatGeo(v) };
                }
                Source::Aurora => {
                    return aurora::parse_ovation(text).map_or(Parsed::None, Parsed::Aurora);
                }
                Source::AuroraPower => {
                    return aurora::parse_hemispheric_power(text)
                        .map_or(Parsed::None, Parsed::AuroraPower);
                }
                Source::KpForecast => {
                    let v = aurora::parse_kp_forecast(text);
                    return if v.is_empty() { Parsed::None } else { Parsed::KpForecast(v) };
                }
                Source::BandConditions => {
                    return indices::parse_band_conditions(text)
                        .map_or(Parsed::None, Parsed::BandConditions);
                }
                Source::Sun | Source::Clouds | Source::CloudsVis => unreachable!(),
            };
            parsed.unwrap_or_else(|e| {
                tracing::warn!("solar feed: {} parse failed: {e}", src.label());
                Parsed::None
            })
        }
    }
}

/// Cached name for the band-conditions document, shared by the feed's own
/// fetch and the standalone one below so the two cannot end up with separate
/// copies of the same file.
const BAND_CONDITIONS_CACHE: &str = "hamqsl.xml";

/// The published band conditions, fetched if the cached copy has expired.
///
/// Blocking, and meant to be called from a worker thread. Standalone rather
/// than one more [`Source`] on the feed because of *where it is read*: the
/// verdicts colour the band menu in the main window, which is on screen for the
/// whole session, so unlike everything else in this module they cannot wait for
/// the 3D view to be opened. Everything else here is looked at only inside that
/// window and stops when it closes.
///
/// It shares the disk cache and the validators with the feed's copy, so with
/// the 3D view open the second of the two requests in an hour is a conditional
/// GET that comes back 304 — and with it closed there is exactly one. That
/// matters here more than elsewhere: this is a single operator's server and its
/// published request is hourly polling at most.
///
/// Returns the cached document when the network is unreachable, and `None` only
/// when there is no usable copy at all.
pub fn band_conditions_cached() -> Option<indices::BandConditions> {
    fn cached(cache: &Cache) -> Option<indices::BandConditions> {
        cache.read_string(BAND_CONDITIONS_CACHE).as_deref().and_then(parse_bc)
    }

    let url = indices::BAND_CONDITIONS_URL;
    let mut cache = Cache::open();

    // The publisher's interval. Nothing is gained by asking sooner: the flux
    // figures behind these verdicts are recomputed hourly.
    //
    // Conditional on there actually being something to serve. The index and the
    // document are two files, and a copy that was deleted, truncated or half
    // written leaves the first saying "fresh" about a second that cannot be
    // read — in which case the interval is protecting nothing and skipping the
    // fetch would blank the band menu for an hour.
    if now_unix() - cache.fetched_at(url) < Source::BandConditions.period()
        && let Some(c) = cached(&cache)
    {
        return Some(c);
    }

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    match http_get(&agent, url, &cache.validators(url), JSON_LIMIT) {
        // 304: what is on disk is current.
        Ok(None) => {
            cache.touch(url, now_unix());
            cached(&cache)
        }
        Ok(Some((bytes, validators, _))) => {
            let parsed = std::str::from_utf8(&bytes).ok().and_then(parse_bc);
            // Only cache what parsed. An error page written over a good
            // document would survive the restart that a failed fetch does not.
            if parsed.is_some() {
                cache.write(BAND_CONDITIONS_CACHE, url, &bytes, validators);
            }
            parsed.or_else(|| cached(&cache))
        }
        Err(e) => {
            tracing::warn!("band conditions fetch failed: {e}");
            cached(&cache)
        }
    }
}

fn parse_bc(text: &str) -> Option<indices::BandConditions> {
    indices::parse_band_conditions(text)
}

/// Cached name for the global WSPR activity document.
const BAND_ACTIVITY_CACHE: &str = "wspr-activity.json";

/// How often the global WSPR activity is refetched, seconds.
///
/// The query's window is fifteen minutes, so ten keeps the figure moving
/// without polling a free public database harder than it needs to.
pub const BAND_ACTIVITY_PERIOD_S: i64 = 600;

/// The global WSPR activity per band, fetched if the cached copy has expired.
///
/// Blocking, and meant to be called from a worker thread — a standalone fetch
/// like [`band_conditions_cached`], for the same reason: the column it fills is
/// in the BANDS window, which the operator expects to be populated whenever
/// they open it, not only once the 3D view has been up. Returns the cached
/// copy when the database is unreachable, and `None` only when there is no
/// usable copy at all.
pub fn band_activity_cached() -> Option<indices::BandActivityTable> {
    let url = indices::band_activity_url();
    let mut cache = Cache::open();
    let read = |cache: &Cache| {
        cache
            .read_string(BAND_ACTIVITY_CACHE)
            .and_then(|t| indices::parse_band_activity(&t, cache.fetched_at(&url)))
    };
    if now_unix() - cache.fetched_at(&url) < BAND_ACTIVITY_PERIOD_S
        && let Some(t) = read(&cache)
    {
        return Some(t);
    }

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    match http_get(&agent, &url, &cache.validators(&url), JSON_LIMIT) {
        Ok(Some((bytes, validators, _))) => {
            let now = now_unix();
            let parsed = std::str::from_utf8(&bytes)
                .ok()
                .and_then(|t| indices::parse_band_activity(t, now));
            // Only cache what parsed; an error page over a good document would
            // outlive the failed fetch.
            if parsed.is_some() {
                cache.write(BAND_ACTIVITY_CACHE, &url, &bytes, validators);
            }
            parsed.or_else(|| read(&cache))
        }
        Ok(None) => {
            cache.touch(&url, now_unix());
            read(&cache)
        }
        Err(e) => {
            tracing::warn!("WSPR activity fetch failed: {e}");
            read(&cache)
        }
    }
}

/// Conditional GET. `Ok(None)` means 304 Not Modified.
///
/// The third member of the tuple is the response's `Warning` header, which is
/// where GeoServer puts the timestamp of the frame it decided to serve — the
/// only place the cloud mosaic says what hour it is a picture of. Everything
/// else ignores it.
pub(crate) fn http_get(
    agent: &ureq::Agent,
    url: &str,
    validators: &Validators,
    limit: u64,
) -> Result<Option<(Vec<u8>, Validators, Option<String>)>, String> {
    let mut req = agent.get(url);
    if let Some(etag) = &validators.etag {
        req = req.header("If-None-Match", etag);
    }
    if let Some(lm) = &validators.last_modified {
        req = req.header("If-Modified-Since", lm);
    }

    let mut resp = req.call().map_err(|e| e.to_string())?;
    if resp.status() == 304 {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let header =
        |name: &str| resp.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
    let next = Validators {
        etag: header("etag"),
        last_modified: header("last-modified"),
        fetched_unix: 0,
    };
    let warning = header("warning");
    let bytes =
        resp.body_mut().with_config().limit(limit).read_to_vec().map_err(|e| e.to_string())?;
    Ok(Some((bytes, next, warning)))
}
