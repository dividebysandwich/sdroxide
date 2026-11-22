//! Precipitation radar for the ADS-B map: NOAA's NEXRAD mosaic for the US
//! (see [`Source`]), and the Deutscher Wetterdienst's
//! composite (`dwd:Niederschlagsradar`, the RV product, 1 km, every
//! 5 minutes), fetched from its public WMS.
//!
//! Licence: DWD geodata and geodata services are reusable under CC BY 4.0
//! with a source credit (dwd.de/copyright), so the map names the source
//! while the layer is on. Chosen over the radar layers of other trackers
//! (RainViewer and the like), whose terms are narrower (HDZ 2026-11-17:
//! „nix einbauen, was unklar ist“).
//!
//! One image covers the whole composite in plain latitude/longitude
//! (EPSG:4326), which is the ADS-B map's own projection, so it is drawn as
//! one stretched texture with no reprojection. Asked for with transparency,
//! the WMS leaves "no rain" clear, veils the area outside the radars in grey
//! and draws the coverage edge magenta; the veil is dropped and the edge kept
//! faint so the map shows through.

use eframe::egui;

use crate::i18n::tr;

/// Where the radar comes from. Each is one image in plain latitude/longitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Deutscher Wetterdienst, RV composite: Germany and its neighbours.
    /// CC BY 4.0 with a source credit.
    Dwd,
    /// NOAA/NWS NEXRAD base reflectivity mosaic of the contiguous US, served
    /// by the Iowa Environmental Mesonet. US government data, public domain;
    /// IEM asks to be named.
    Nexrad,
    /// Environment and Climate Change Canada, GeoMet radar composite (1 km,
    /// rain rate). Open Government Licence – Canada, with attribution.
    Canada,
    /// NASA GPM IMERG precipitation rate from satellites, 60° N to 60° S,
    /// through NASA GIBS: coarser than radar and some hours late, but it
    /// covers Africa, Australia and the oceans, where no free radar is.
    /// NASA data, free of restriction.
    World,
}

impl Source {
    /// The setting: 0 by home, 1 DWD, 2 NEXRAD, 3 Canada, 4 world. By home:
    /// the radar whose area holds it (Canada from 49° N in America), else the
    /// satellite estimate; no home, DWD.
    // Why: HDZ 2026-11-17: „gibt's noch andere freie Quellen, Afrika,
    //      Australien? … da wo klar ist, bau ein“. Australia's BoM radar is
    //      licensed for personal use only, so it is not among them.
    pub fn pick(setting: u8, home: Option<(f64, f64)>) -> Source {
        let inside = |s: Source, (lat, lon): (f64, f64)| {
            let (w, so, e, n) = s.bbox();
            (w..=e).contains(&lon) && (so..=n).contains(&lat)
        };
        match (setting, home) {
            (1, _) => Source::Dwd,
            (2, _) => Source::Nexrad,
            (3, _) => Source::Canada,
            (4, _) => Source::World,
            (_, None) => Source::Dwd,
            (_, Some(h)) if inside(Source::Dwd, h) => Source::Dwd,
            (_, Some(h)) if h.0 >= 49.0 && inside(Source::Canada, h) => Source::Canada,
            (_, Some(h)) if inside(Source::Nexrad, h) => Source::Nexrad,
            _ => Source::World,
        }
    }

    /// The image's extent, degrees: west, south, east, north.
    pub fn bbox(self) -> (f64, f64, f64, f64) {
        match self {
            Source::Dwd => (1.46, 45.68, 18.73, 56.22),
            Source::Nexrad => (-126.0, 24.0, -66.0, 50.0),
            Source::Canada => (-142.0, 41.0, -52.0, 70.0),
            Source::World => (-180.0, -60.0, 180.0, 60.0),
        }
    }

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    fn url(self) -> String {
        let (w, s, e, n) = self.bbox();
        let (base, layer) = match self {
            Source::Dwd => ("https://maps.dwd.de/geoserver/dwd/wms", "dwd:Niederschlagsradar"),
            Source::Nexrad => (
                "https://mesonet.agron.iastate.edu/cgi-bin/wms/nexrad/n0q.cgi",
                "nexrad-n0q-900913",
            ),
            Source::Canada => ("https://geo.weather.gc.ca/geomet", "RADAR_1KM_RRAI"),
            Source::World => (
                "https://gibs.earthdata.nasa.gov/wms/epsg4326/best/wms.cgi",
                "IMERG_Precipitation_Rate",
            ),
        };
        // 1400 px across (the world 2048), the height in the same degrees
        // per pixel.
        let width = if self == Source::World { 2048.0 } else { 1400.0 };
        let h = (width * (n - s) / (e - w)).round() as u32;
        format!(
            "{base}?service=WMS&version=1.1.1&request=GetMap&layers={layer}&styles=\
             &srs=EPSG:4326&bbox={w},{s},{e},{n}&width={width}&height={h}\
             &format=image/png&transparent=true"
        )
    }

    /// Who serves it, under what licence, and where both are written down:
    /// for the chips' tooltips and their right-click link (HDZ 2026-11-17:
    /// „in den Tooltips die Quelle … Link“).
    pub fn info(self) -> (&'static str, String, &'static str) {
        match self {
            Source::Dwd => {
                ("Deutscher Wetterdienst", tr!("adsb-lic-ccby"), "https://www.dwd.de/copyright")
            }
            Source::Nexrad => (
                "NOAA NWS NEXRAD, Iowa Environmental Mesonet",
                tr!("adsb-lic-pd"),
                "https://mesonet.agron.iastate.edu/",
            ),
            Source::Canada => (
                "Environment and Climate Change Canada",
                tr!("adsb-lic-ogl"),
                "https://eccc-msc.github.io/open-data/licence/readme_en/",
            ),
            Source::World => {
                ("NASA GPM IMERG, GIBS", tr!("adsb-lic-nasa"), "https://gpm.nasa.gov/data/imerg")
            }
        }
    }

    /// The credit the map shows, with the image's time.
    pub fn credit(self, time: &str) -> String {
        match self {
            Source::Dwd => tr!("adsbmap-wx-credit", time = time),
            Source::Nexrad => tr!("adsbmap-wx-credit-us", time = time),
            Source::Canada => tr!("adsbmap-wx-credit-ca", time = time),
            Source::World => tr!("adsbmap-wx-credit-world", time = time),
        }
    }
}

/// The radar updates every five minutes; asked again after this long.
const REFRESH_S: i64 = 300;
/// After a failed fetch. Short: the first request of a fresh image can take
/// the server half a minute to render (seen 17.11.), and the layer should not
/// sit empty for long after that.
const RETRY_S: i64 = 20;

/// A picture on disk is shown at start for this long after it was fetched:
/// the radar sits on screen at once instead of after the server's half minute,
/// and older weather is not passed off as today's.
/// Why: HDZ 2026-11-17: „2h Cache, sonst wartet man beim Programmstart jedes
///      Mal dumm rum“.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const CACHE_MAX_S: i64 = 2 * 3600;

/// A decoded picture and when it was fetched, or why none came, with the
/// generation it was asked in (see [`WxRadar::reload`]).
type Fetched = (u32, Source, Result<(egui::ColorImage, i64), String>);

/// The latest radar image and the fetch under way.
pub struct WxRadar {
    tex: Option<egui::TextureHandle>,
    source: Option<Source>,
    /// When the shown image was fetched, Unix seconds.
    pub fetched_at: i64,
    asked_at: i64,
    pending: bool,
    /// Bumped by [`WxRadar::reload`]; a fetch asked before it is dropped.
    generation: u32,
    /// The next fetch skips the kept picture: the operator asked for a new one.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    skip_cache: bool,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    tx: std::sync::mpsc::Sender<Fetched>,
    rx: std::sync::mpsc::Receiver<Fetched>,
}

impl Default for WxRadar {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        WxRadar {
            tex: None,
            source: None,
            fetched_at: 0,
            asked_at: i64::MIN / 2,
            pending: false,
            generation: 0,
            skip_cache: false,
            tx,
            rx,
        }
    }
}

impl WxRadar {
    /// Drop the picture on the map and fetch a new one from the server now,
    /// past the kept one; a fetch still under way is ignored when it lands.
    /// Why: HDZ 2026-11-17: ein Knopf, „damit alte Wetterkarten einfach aktiv
    ///      verschwinden“.
    pub fn reload(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.tex = None;
        self.fetched_at = 0;
        self.asked_at = i64::MIN / 2;
        self.pending = false;
        self.skip_cache = true;
    }

    /// The texture of `source` to draw, fetching a fresh one when due. Once a
    /// frame while the layer is on. A change of source drops the old image.
    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        now: i64,
        source: Source,
    ) -> Option<egui::TextureId> {
        if self.source != Some(source) {
            self.source = Some(source);
            self.tex = None;
            self.fetched_at = 0;
            self.asked_at = i64::MIN / 2;
            // The old source's fetch is no longer wanted; do not wait for it.
            self.generation = self.generation.wrapping_add(1);
            self.pending = false;
        }
        while let Ok((generation, from, res)) = self.rx.try_recv() {
            if generation != self.generation {
                continue;
            }
            self.pending = false;
            match res {
                Ok((img, at)) if from == source && at >= self.fetched_at => {
                    self.tex = Some(ctx.load_texture("adsb-wx-radar", img, Default::default()));
                    self.fetched_at = at;
                }
                Ok(_) => {}
                Err(e) => tracing::debug!("weather radar {from:?}: {e}"),
            }
        }
        let due = if self.tex.is_some() { REFRESH_S } else { RETRY_S };
        if !self.pending && now - self.asked_at >= due {
            self.asked_at = now;
            self.spawn(ctx, source);
        }
        self.tex.as_ref().map(|t| t.id())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn spawn(&mut self, ctx: &egui::Context, source: Source) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let generation = self.generation;
        let use_cache = !std::mem::take(&mut self.skip_cache);
        self.pending = std::thread::Builder::new()
            .name("sdroxide-wx-radar".into())
            .spawn(move || {
                // The kept picture first, if there is a recent one, then the
                // server's; the receiver keeps whichever is newer.
                let dir = cache_dir();
                if use_cache
                    && let Some(bytes) =
                        dir.as_deref().and_then(|d| read_cache(d, source, unix_now()))
                    && let Ok(img) = decode(source, &bytes.0)
                {
                    let _ = tx.send((generation, source, Ok((img, bytes.1))));
                    ctx.request_repaint();
                }
                let res = fetch_png(source).and_then(|bytes| {
                    let img = decode(source, &bytes)?;
                    if let Some(d) = dir.as_deref() {
                        write_cache(d, source, &bytes);
                    }
                    Ok((img, unix_now()))
                });
                let _ = tx.send((generation, source, res));
                ctx.request_repaint();
            })
            .is_ok();
    }

    #[cfg(target_arch = "wasm32")]
    fn spawn(&mut self, _ctx: &egui::Context, _source: Source) {}
}

#[cfg(not(target_arch = "wasm32"))]
fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Where the last picture of each source is kept.
#[cfg(not(target_arch = "wasm32"))]
fn cache_dir() -> Option<std::path::PathBuf> {
    Some(sdroxide_config::config_dir().ok()?.join("adsb-wx"))
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn cache_file(dir: &std::path::Path, source: Source) -> std::path::PathBuf {
    let name = match source {
        Source::Dwd => "dwd",
        Source::Nexrad => "usa",
        Source::Canada => "canada",
        Source::World => "world",
    };
    dir.join(format!("{name}.png"))
}

/// Keep the PNG as the server sent it; written aside and renamed, so a reader
/// never sees half a file.
#[cfg(not(target_arch = "wasm32"))]
fn write_cache(dir: &std::path::Path, source: Source, png: &[u8]) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let path = cache_file(dir, source);
    let tmp = path.with_extension("png.part");
    if std::fs::write(&tmp, png).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

/// The kept PNG and when it was written, if that is no more than
/// [`CACHE_MAX_S`] before `now`.
#[cfg(not(target_arch = "wasm32"))]
fn read_cache(dir: &std::path::Path, source: Source, now: i64) -> Option<(Vec<u8>, i64)> {
    let path = cache_file(dir, source);
    let at = std::fs::metadata(&path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    if now - at > CACHE_MAX_S {
        return None;
    }
    Some((std::fs::read(&path).ok()?, at))
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_png(source: Source) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(10)))
        .timeout_global(Some(std::time::Duration::from_secs(90)))
        .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let mut r = agent.get(&source.url()).call().map_err(|e| e.to_string())?;
    r.body_mut().with_config().limit(8 * 1024 * 1024).read_to_vec().map_err(|e| e.to_string())
}

/// The PNG as the map draws it: see [`alpha_of`].
#[cfg(not(target_arch = "wasm32"))]
fn decode(source: Source, bytes: &[u8]) -> Result<egui::ColorImage, String> {
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut px = img.into_raw();
    for p in px.as_chunks_mut::<4>().0 {
        p[3] = match source {
            Source::Dwd => alpha_of(p[0], p[1], p[2], p[3]),
            // IEM sends only the echoes, in the NWS colours, some of them
            // grey; nothing to drop.
            Source::Nexrad | Source::Canada | Source::World => p[3].min(150),
        };
    }
    Ok(egui::ColorImage::from_rgba_unmultiplied([w, h], &px))
}

/// How opaque a WMS pixel is drawn: what the WMS leaves clear stays clear,
/// the grey veil outside the radars goes, the magenta coverage edge stays
/// faint, rain is drawn at most half-opaque so land and traffic stay readable.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn alpha_of(r: u8, g: u8, b: u8, a: u8) -> u8 {
    let (ri, gi, bi) = (i16::from(r), i16::from(g), i16::from(b));
    if a == 0 || ((ri - gi).abs() < 8 && (gi - bi).abs() < 8) {
        0
    } else if r > 200 && b > 200 && g < 80 {
        a.min(70)
    } else {
        a.min(150)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kept_picture_is_shown_for_two_hours_and_not_after() {
        let dir = std::env::temp_dir().join(format!("sdx-wx-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(read_cache(&dir, Source::Dwd, unix_now()).is_none());
        write_cache(&dir, Source::Dwd, b"png");
        let now = unix_now();
        let (bytes, at) = read_cache(&dir, Source::Dwd, now).expect("fresh");
        assert_eq!(bytes, b"png");
        assert!((now - at).abs() < 5);
        assert!(read_cache(&dir, Source::Dwd, at + CACHE_MAX_S).is_some());
        assert!(read_cache(&dir, Source::Dwd, at + CACHE_MAX_S + 1).is_none());
        assert!(read_cache(&dir, Source::World, now).is_none(), "one file per source");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_clear_and_the_veil_vanish_and_rain_stays() {
        // Colours as the DWD WMS sends them (sampled 17.11.).
        assert_eq!(alpha_of(0, 0, 0, 0), 0, "no rain");
        assert_eq!(alpha_of(124, 124, 124, 76), 0, "outside the radars");
        assert_eq!(alpha_of(246, 3, 250, 177), 70, "coverage edge");
        assert_eq!(alpha_of(2, 153, 51, 254), 150, "light rain");
        assert_eq!(alpha_of(249, 251, 1, 254), 150, "heavier rain");
    }

    #[test]
    fn the_radar_follows_home_unless_one_is_chosen() {
        let dortmund = Some((51.5, 7.5));
        let denver = Some((39.7, -105.0));
        assert_eq!(Source::pick(0, dortmund), Source::Dwd);
        assert_eq!(Source::pick(0, denver), Source::Nexrad);
        assert_eq!(Source::pick(0, None), Source::Dwd);
        assert_eq!(Source::pick(2, dortmund), Source::Nexrad);
        assert_eq!(Source::pick(1, denver), Source::Dwd);
        assert_eq!(Source::pick(0, Some((53.5, -113.5))), Source::Canada, "Edmonton");
        assert_eq!(Source::pick(0, Some((-33.9, 151.2))), Source::World, "Sydney");
        assert_eq!(Source::pick(0, Some((-1.3, 36.8))), Source::World, "Nairobi");
        assert_eq!(Source::pick(4, dortmund), Source::World);
        assert!(Source::World.url().contains("width=2048&height=683"));
        assert!(Source::Nexrad.url().contains("bbox=-126,24,-66,50&width=1400&height=607"));
    }

    /// Live check against the DWD service; run by hand with `--ignored`.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[ignore]
    fn both_services_answer_with_an_image() {
        for src in [Source::Dwd, Source::Nexrad, Source::Canada, Source::World] {
            let img = decode(src, &fetch_png(src).expect("fetch")).expect("decode");
            let rain = img.pixels.iter().filter(|p| p.a() > 0).count();
            eprintln!("{src:?}: {:?} px, {rain} with something drawn", img.size);
            assert!(img.size[0] >= 1400);
        }
    }
}
