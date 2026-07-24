//! "RF Paint" (Spectrum Painting) compositing for transmit.
//!
//! Both the text and the image areas reduce to the same thing: a grayscale
//! intensity bitmap (`w` = frequency columns, `h` = time rows, row 0 = top of
//! the picture) which is PNG-encoded and sent through the ordinary image
//! transmit path. The engine grayscales it again and hands it to the
//! `RfPaintController`, which paints it onto the waterfall.
//!
//! Pure-Rust (`image` + `ab_glyph`) so it runs identically native and on wasm.
//! Dimensions are bounded here to stay inside the synthesizer's tone/time caps
//! and to keep the tones legible on a zoomed waterfall.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use eframe::egui::{self, Color32};

/// Frequency resolution: bitmap width → number of tone bins across the 3 kHz
/// band (≈ 15 Hz/bin at 200, easily resolved on the zoomed waterfall).
pub const FREQ_BINS: usize = 200;
/// Image time rows (transmit is `rows × ROW_SECS` ≈ 7 s at 128).
pub const IMG_MAX_ROWS: usize = 128;
/// Text-banner height → post-rotation frequency bins for painted text. This
/// sets the (constant) glyph size and the frequency detail; still coarse enough
/// to read on the zoomed waterfall (3 kHz / 60 ≈ 50 Hz per bin).
pub const TEXT_ROWS: usize = 60;
/// Cap on the text banner's width → post-rotation time rows. The glyph size is
/// constant, so a longer message simply produces a wider banner (a longer
/// transmit); this bounds the very longest message (≈ 90 s) rather than shrinking
/// the font. Kept ≤ the synthesizer's `MAX_ROWS`.
pub const TEXT_MAX_WIDTH: usize = 1600;

/// The bundled UI font (also used by SSTV), rasterised to a mono coverage mask.
fn font() -> Option<FontRef<'static>> {
    const RAW: &[u8] = include_bytes!("../assets/fonts/ChakraPetch-SemiBold.ttf");
    FontRef::try_from_slice(RAW).ok()
}

fn text_advance(text: &str, font: &FontRef<'static>, scale: PxScale) -> f32 {
    let scaled = font.as_scaled(scale);
    let mut width = 0.0;
    let mut prev = None;
    for ch in text.chars() {
        let g = font.glyph_id(ch);
        if let Some(p) = prev {
            width += scaled.kern(p, g);
        }
        width += scaled.h_advance(g);
        prev = Some(g);
    }
    width
}

/// Rasterise a single line of `text` into a grayscale bitmap for painting.
///
/// The line is drawn as a horizontal banner (`TEXT_ROWS` tall, constant glyph
/// size) and then rotated 90° **counter-clockwise**, so it reads correctly on
/// the waterfall — the banner's height becomes the frequency span and its length
/// becomes transmit time. Returns `None` for empty text.
pub fn text_bitmap(text: &str) -> Option<(Vec<u8>, u16, u16)> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let font = font()?;
    let h = TEXT_ROWS;
    let pad = 3.0;
    // Constant glyph size — a longer message widens the banner (longer transmit)
    // rather than shrinking the font.
    let px = h as f32 * 0.92;
    let scale = PxScale::from(px);
    let adv = text_advance(text, &font, scale);
    let w = ((adv + 2.0 * pad).ceil() as usize).clamp(8, TEXT_MAX_WIDTH);
    let mut banner = vec![0u8; w * h];
    // Baseline low enough to keep descenders (g, Q) inside the banner.
    let baseline = h as f32 * 0.74;
    draw_text_gray(&mut banner, w, h, pad, baseline, text, &font, scale);
    let (rot, rw, rh) = rotate_ccw(&banner, w, h);
    Some((rot, rw as u16, rh as u16))
}

/// Rotate a grayscale bitmap 90° counter-clockwise. The result is `h` wide × `w`
/// tall.
fn rotate_ccw(src: &[u8], w: usize, h: usize) -> (Vec<u8>, usize, usize) {
    let (dw, dh) = (h, w);
    let mut dst = vec![0u8; dw * dh];
    for y in 0..h {
        for x in 0..w {
            // Counter-clockwise: source (x, y) → destination (y, w-1-x).
            dst[(w - 1 - x) * dw + y] = src[y * w + x];
        }
    }
    (dst, dw, dh)
}

/// Decode an image file and reduce it to a grayscale bitmap fitted within
/// `FREQ_BINS × IMG_MAX_ROWS` (aspect preserved), with a light contrast stretch
/// so the picture reads clearly on the waterfall. Returns `(gray, w, h)`.
pub fn image_bitmap(bytes: &[u8]) -> Option<(Vec<u8>, u16, u16)> {
    let img = image::load_from_memory(bytes).ok()?;
    let img = img
        .resize(
            FREQ_BINS as u32,
            IMG_MAX_ROWS as u32,
            image::imageops::FilterType::Triangle,
        )
        .to_luma8();
    let (w, h) = (img.width() as u16, img.height() as u16);
    let mut gray = img.into_raw();
    contrast_stretch(&mut gray);
    Some((gray, w, h))
}

/// Stretch the intensity range to full 0..255 (min→0, max→255) so faint images
/// still paint a bright, legible picture.
fn contrast_stretch(gray: &mut [u8]) {
    let (mut lo, mut hi) = (255u8, 0u8);
    for &v in gray.iter() {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if hi <= lo {
        return;
    }
    let span = (hi - lo) as f32;
    for v in gray.iter_mut() {
        *v = (((*v - lo) as f32 / span) * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

/// PNG-encode a grayscale bitmap (as RGB, since the engine decodes PNG→RGB→gray).
pub fn gray_to_png(gray: &[u8], w: u16, h: u16) -> Option<Vec<u8>> {
    let mut rgb = Vec::with_capacity(gray.len() * 3);
    for &v in gray {
        rgb.extend_from_slice(&[v, v, v]);
    }
    let img = image::RgbImage::from_raw(w as u32, h as u32, rgb)?;
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut buf, image::ImageFormat::Png)
        .ok()?;
    Some(buf.into_inner())
}

/// Build a grayscale preview of a bitmap — exactly how the picture will look on
/// the waterfall (row 0 = top). A few black guard rows top and bottom give the
/// looping "waterfall" scroll a clean seam between repeats.
pub fn preview_gray_image(gray: &[u8], w: u16, h: u16) -> egui::ColorImage {
    let guard = 4usize;
    let (w, h) = ((w as usize).max(1), h as usize);
    let hh = h + guard * 2;
    let mut ci = egui::ColorImage::new([w, hh], vec![Color32::BLACK; w * hh]);
    for y in 0..h {
        for x in 0..w {
            let v = gray[y * w + x];
            ci.pixels[(y + guard) * w + x] = Color32::from_gray(v);
        }
    }
    ci
}

#[allow(clippy::too_many_arguments)]
fn draw_text_gray(
    gray: &mut [u8],
    w: usize,
    h: usize,
    x: f32,
    baseline: f32,
    text: &str,
    font: &FontRef<'static>,
    scale: PxScale,
) {
    let scaled = font.as_scaled(scale);
    let mut caret = x;
    let mut prev = None;
    for ch in text.chars() {
        let gid = font.glyph_id(ch);
        if let Some(p) = prev {
            caret += scaled.kern(p, gid);
        }
        let glyph = gid.with_scale_and_position(scale, point(caret, baseline));
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, cov| {
                let px = bounds.min.x as i32 + gx as i32;
                let py = bounds.min.y as i32 + gy as i32;
                if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h {
                    let i = py as usize * w + px as usize;
                    let val = (cov * 255.0).round().clamp(0.0, 255.0) as u8;
                    gray[i] = gray[i].max(val);
                }
            });
        }
        caret += scaled.h_advance(gid);
        prev = Some(gid);
    }
}
