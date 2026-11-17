//! Cloud and thunderstorms for the ADS-B map, from the same source and the
//! same code as the 3D globe: NOAA/NESDIS's global geostationary mosaic via
//! nowCOAST (see `sdroxide_solar::clouds`), infrared for cloud-top height and
//! visible for low cloud by day, with deep convection picked out as storm
//! cells.
//!
//! Fetched here rather than borrowed from the globe's feed, which only runs
//! while the globe is open and brings the Sun, satellites and space weather
//! with it. Two PNGs of about 280 kB an hour.
//! Why: HDZ 2026-11-17: „1. machen“ (WOLKEN and GEWITTER on the map).

use eframe::egui;
#[cfg(not(target_arch = "wasm32"))]
use sdroxide_solar::clouds::Band;
use sdroxide_solar::clouds::{self, CloudField, ConvCell};

/// The mosaic is hourly and an hour behind; asked again after this long.
const REFRESH_S: i64 = 1800;
const RETRY_S: i64 = 60;

type Fetched = Result<(egui::ColorImage, Vec<ConvCell>, i64), String>;

/// The latest cloud picture and storm cells.
pub struct CloudLayer {
    tex: Option<egui::TextureHandle>,
    pub cells: Vec<ConvCell>,
    /// The hour the picture is of, Unix seconds.
    pub frame_unix: i64,
    asked_at: i64,
    pending: bool,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    tx: std::sync::mpsc::Sender<Fetched>,
    rx: std::sync::mpsc::Receiver<Fetched>,
}

impl Default for CloudLayer {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        CloudLayer {
            tex: None,
            cells: Vec::new(),
            frame_unix: 0,
            asked_at: i64::MIN / 2,
            pending: false,
            tx,
            rx,
        }
    }
}

impl CloudLayer {
    /// Take in what has arrived and fetch when due. Once a frame while
    /// CLOUDS or STORMS is on.
    pub fn poll(&mut self, ctx: &egui::Context, now: i64) {
        while let Ok(res) = self.rx.try_recv() {
            self.pending = false;
            match res {
                Ok((img, cells, frame)) => {
                    self.tex = Some(ctx.load_texture("adsb-clouds", img, Default::default()));
                    self.cells = cells;
                    self.frame_unix = frame;
                }
                Err(e) => tracing::debug!("cloud mosaic: {e}"),
            }
        }
        let due = if self.tex.is_some() { REFRESH_S } else { RETRY_S };
        if !self.pending && now - self.asked_at >= due {
            self.asked_at = now;
            self.spawn(ctx, now);
        }
    }

    pub fn texture(&self) -> Option<egui::TextureId> {
        self.tex.as_ref().map(|t| t.id())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn spawn(&mut self, ctx: &egui::Context, now: i64) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        self.pending = std::thread::Builder::new()
            .name("sdroxide-adsb-clouds".into())
            .spawn(move || {
                let _ = tx.send(fetch(now));
                ctx.request_repaint();
            })
            .is_ok();
    }

    #[cfg(target_arch = "wasm32")]
    fn spawn(&mut self, _ctx: &egui::Context, _now: i64) {}
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_band(agent: &ureq::Agent, band: Band, now: i64) -> Result<clouds::Plane, String> {
    let mut r = agent.get(&band.url()).call().map_err(|e| e.to_string())?;
    let frame = r
        .headers()
        .get("warning")
        .and_then(|w| w.to_str().ok())
        .and_then(clouds::frame_unix)
        .unwrap_or_else(|| clouds::assumed_frame_unix(now));
    let bytes = r
        .body_mut()
        .with_config()
        .limit(8 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    clouds::parse_plane(band, &bytes, frame, now).ok_or_else(|| "unreadable mosaic".into())
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch(now: i64) -> Fetched {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(10)))
        .timeout_global(Some(std::time::Duration::from_secs(90)))
        .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let ir = fetch_band(&agent, Band::Longwave, now)?;
    // The visible channel only adds low cloud by day; without it the picture
    // is infrared only, which is still the backbone.
    let vis = fetch_band(&agent, Band::Visible, now).ok();
    let field = clouds::combine(&ir, vis.as_ref());
    Ok((image_of(&field), field.cells.clone(), field.frame_unix))
}

/// The field as a texture: white-grey, as opaque as the cloud is thick, a
/// little whiter where the tops stand high, at most half-covering so the map
/// and the traffic stay readable.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn image_of(f: &CloudField) -> egui::ColorImage {
    let mut px = Vec::with_capacity(clouds::GRID_W * clouds::GRID_H * 4);
    for (o, t) in f.opacity.iter().zip(&f.top) {
        let a = (u16::from(*o) * 140 / 255) as u8;
        let v = 195 + (u16::from(*t) * 55 / 255) as u8;
        px.extend_from_slice(&[v, v, v.saturating_add(5), a]);
    }
    egui::ColorImage::from_rgba_unmultiplied([clouds::GRID_W, clouds::GRID_H], &px)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thick_high_cloud_is_whiter_and_more_opaque_than_thin_low_cloud() {
        let n = clouds::GRID_W * clouds::GRID_H;
        let mut f = CloudField {
            frame_unix: 0,
            opacity: vec![0; n],
            top: vec![0; n],
            cells: Vec::new(),
            has_visible: false,
        };
        f.opacity[0] = 255;
        f.top[0] = 255;
        f.opacity[1] = 40;
        let img = image_of(&f);
        let (thick, thin) = (img.pixels[0], img.pixels[1]);
        assert!(thick.a() > thin.a() && thin.a() > 0);
        assert_eq!(img.pixels[2].a(), 0, "clear sky");
        assert_eq!(img.size, [clouds::GRID_W, clouds::GRID_H]);
    }

    /// Live check against nowCOAST; run by hand with `--ignored`.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[ignore]
    fn the_mosaic_arrives_with_cloud_and_storms() {
        let now = crate::time::now_unix();
        let (img, cells, frame) = fetch(now).expect("fetch");
        let cloudy = img.pixels.iter().filter(|p| p.a() > 0).count();
        eprintln!(
            "{:?}, {cloudy} cloudy px, {} storms, frame {}s old",
            img.size,
            cells.len(),
            now - frame
        );
        assert!(cloudy > 1000);
    }
}
