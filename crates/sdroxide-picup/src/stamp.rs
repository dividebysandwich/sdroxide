//! The uploaded copy of a picture: QSSTV's caption bar, its size limit, and
//! the encoding the operator picked. Also the `checknew.jpg` heartbeat.
//!
//! The picture on disk is never touched — only the copy that goes up.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::{DynamicImage, ImageFormat, Rgb, RgbImage, imageops::FilterType};
use sdroxide_types::UploadFormat;

/// QSSTV's limit on what it uploads.
pub const MAX_W: u32 = 960;
pub const MAX_H: u32 = 768;

/// The bundled UI font (OFL), so the caption looks like the rest of the
/// program and does not depend on what is installed on the station.
fn font() -> Option<FontRef<'static>> {
    const RAW: &[u8] = include_bytes!("../../sdroxide-ui/assets/fonts/ChakraPetch-SemiBold.ttf");
    FontRef::try_from_slice(RAW).ok()
}

/// What the caption says about a picture.
#[derive(Debug, Clone, Default)]
pub struct Caption {
    /// When it was received, Unix seconds.
    pub unix: i64,
    /// The dial frequency, or 0 when nothing is tuned.
    pub dial_hz: f64,
    /// "Martin 1", "RIFP", "WEFAX" …
    pub mode: String,
    /// Who sent it, when an FSK ID or a manifest said.
    pub call: Option<String>,
}

impl Caption {
    /// QSSTV's wording: `hh:mm ddd MMM d, yyyy UTC  <kHz> kHz  <mode> de <call>`.
    pub fn text(&self) -> String {
        let when = chrono::DateTime::from_timestamp(self.unix, 0)
            .map(|t| t.format("%H:%M %a %b %-d, %Y UTC").to_string())
            .unwrap_or_default();
        let mut s = when;
        if self.dial_hz > 0.0 {
            s.push_str(&format!("  {:.0} kHz", self.dial_hz / 1000.0));
        }
        if !self.mode.is_empty() {
            s.push_str("  ");
            s.push_str(&self.mode);
        }
        if let Some(call) = self.call.as_deref().filter(|c| !c.trim().is_empty()) {
            s.push_str(" de ");
            s.push_str(call.trim());
        }
        s
    }
}

/// Make the copy that is uploaded.
pub fn prepare(
    png: &[u8],
    caption: Option<&Caption>,
    downscale: bool,
    format: UploadFormat,
) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(png).map_err(|e| format!("reading the picture: {e}"))?;
    let img = if downscale && (img.width() > MAX_W || img.height() > MAX_H) {
        img.resize(MAX_W, MAX_H, FilterType::Lanczos3)
    } else {
        img
    };
    let mut rgb = img.to_rgb8();
    if let Some(c) = caption {
        draw_bar(&mut rgb, &c.text());
    }
    encode(DynamicImage::ImageRgb8(rgb), format)
}

fn encode(img: DynamicImage, format: UploadFormat) -> Result<Vec<u8>, String> {
    let mut out = std::io::Cursor::new(Vec::new());
    match format {
        UploadFormat::Png => img.write_to(&mut out, ImageFormat::Png),
        UploadFormat::Jpeg => {
            let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85);
            img.write_with_encoder(enc)
        }
    }
    .map_err(|e| format!("encoding the upload: {e}"))?;
    Ok(out.into_inner())
}

/// White text on a black bar in the top-left corner, sized to the picture the
/// way QSSTV sizes it: 9 px per 320 px of width, never under 8.
fn draw_bar(img: &mut RgbImage, text: &str) {
    let Some(font) = font() else { return };
    let px = (9.0 * img.width() as f32 / 320.0).max(8.0);
    let scaled = font.as_scaled(PxScale::from(px));
    let width: f32 = text.chars().map(|c| scaled.h_advance(scaled.glyph_id(c))).sum();
    let bar_w = ((width + 6.0).ceil() as u32).min(img.width());
    let bar_h = ((scaled.height() + 2.0).ceil() as u32).min(img.height());
    for y in 0..bar_h {
        for x in 0..bar_w {
            img.put_pixel(x, y, Rgb([0, 0, 0]));
        }
    }
    let baseline = 1.0 + scaled.ascent();
    let mut x = 2.0;
    for c in text.chars() {
        let id = scaled.glyph_id(c);
        let glyph = id.with_scale_and_position(PxScale::from(px), ab_glyph::point(x, baseline));
        x += scaled.h_advance(id);
        let Some(outline) = font.outline_glyph(glyph) else { continue };
        let b = outline.px_bounds();
        outline.draw(|gx, gy, cov| {
            let (px, py) = (b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32);
            if px < 0 || py < 0 || px as u32 >= bar_w || py as u32 >= bar_h {
                return;
            }
            let v = (cov.clamp(0.0, 1.0) * 255.0) as u8;
            let p = img.get_pixel_mut(px as u32, py as u32);
            let v = p.0[0].max(v);
            *p = Rgb([v, v, v]);
        });
    }
}

/// QSSTV's `checknew.jpg`: a black JPEG whose height is the current minute,
/// which a web page polls to notice a new picture without downloading it.
/// Width 1 while running, 2 for "offline". Minute 0 would make an empty image
/// (QSSTV's does), so the height never drops below 1.
pub fn checknew(width: u32, unix: i64) -> Vec<u8> {
    let minute = ((unix.rem_euclid(3600)) / 60) as u32;
    let img = RgbImage::new(width, minute.max(1));
    encode(DynamicImage::ImageRgb8(img), UploadFormat::Jpeg).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = RgbImage::from_pixel(w, h, Rgb([200, 30, 30]));
        encode(DynamicImage::ImageRgb8(img), UploadFormat::Png).unwrap()
    }

    #[test]
    fn caption_reads_like_qsstv() {
        let c = Caption {
            unix: 1_700_000_000, // 2023-11-14 22:13 UTC, a Tuesday
            dial_hz: 14_230_000.0,
            mode: "Martin 1".into(),
            call: Some("EA4XYZ".into()),
        };
        assert_eq!(c.text(), "22:13 Tue Nov 14, 2023 UTC  14230 kHz  Martin 1 de EA4XYZ");
        let bare = Caption { unix: 1_700_000_000, ..Default::default() };
        assert_eq!(bare.text(), "22:13 Tue Nov 14, 2023 UTC");
    }

    #[test]
    fn large_pictures_are_shrunk_and_small_ones_left() {
        let out = prepare(&png(1920, 1080), None, true, UploadFormat::Png).unwrap();
        let img = image::load_from_memory(&out).unwrap();
        assert!(img.width() <= MAX_W && img.height() <= MAX_H);
        let out = prepare(&png(320, 256), None, true, UploadFormat::Png).unwrap();
        let img = image::load_from_memory(&out).unwrap();
        assert_eq!((img.width(), img.height()), (320, 256));
    }

    #[test]
    fn the_bar_is_black_with_white_text() {
        let c = Caption { unix: 1_700_000_000, mode: "Scottie 1".into(), ..Default::default() };
        let out = prepare(&png(320, 256), Some(&c), false, UploadFormat::Png).unwrap();
        let img = image::load_from_memory(&out).unwrap().to_rgb8();
        assert_eq!(img.get_pixel(0, 0), &Rgb([0, 0, 0]), "the bar starts at the corner");
        let mut bar = (0..8).flat_map(|y| (0..80).map(move |x| (x, y)));
        assert!(bar.clone().any(|(x, y)| img.get_pixel(x, y).0[0] > 200), "there is text");
        let coloured = bar.find(|&(x, y)| {
            let p = img.get_pixel(x, y).0;
            p[0] != p[1] || p[1] != p[2]
        });
        assert_eq!(coloured, None, "the bar is grey-scale only");
        assert_eq!(img.get_pixel(319, 255), &Rgb([200, 30, 30]), "the rest is untouched");
    }

    #[test]
    fn jpeg_comes_out_as_jpeg() {
        let out = prepare(&png(64, 64), None, false, UploadFormat::Jpeg).unwrap();
        assert_eq!(image::guess_format(&out).unwrap(), ImageFormat::Jpeg);
    }

    #[test]
    fn checknew_is_as_tall_as_the_minute() {
        let at = |m: i64| image::load_from_memory(&checknew(1, 3600 * 5 + m * 60)).unwrap();
        assert_eq!((at(37).width(), at(37).height()), (1, 37));
        assert_eq!(at(0).height(), 1);
        let off = image::load_from_memory(&checknew(2, 3600 + 12 * 60)).unwrap();
        assert_eq!((off.width(), off.height()), (2, 12));
    }
}
