//! SSTV image compositing for transmit: crop/scale a source picture to the
//! selected mode's dimensions, stamp a red→black header strip with the program
//! name + version, and overlay the operator's multi-line message (one bundled
//! font, bold with a black outline for readability, with the first line drawn at
//! double size).
//!
//! Pure-Rust (`image` + `ab_glyph`) so it runs identically in the native app and
//! the wasm browser client — the composed buffer is both the live preview and,
//! PNG-encoded, the transmit payload.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use eframe::egui;
use sdroxide_types::SstvMode;

/// Height of the header strip, in pixels.
const HEADER_H: usize = 16;

/// The single font used for the header and the message overlay
/// (ChakraPetch-SemiBold, already bundled OFL for the UI's own text).
fn message_font() -> Option<FontRef<'static>> {
    const RAW: &[u8] = include_bytes!("../assets/fonts/ChakraPetch-SemiBold.ttf");
    FontRef::try_from_slice(RAW).ok()
}

/// Decode arbitrary image file bytes (PNG/JPEG) to interleaved RGB + size.
pub fn decode_image(bytes: &[u8]) -> Option<(Vec<u8>, u16, u16)> {
    let img = image::load_from_memory(bytes).ok()?.to_rgb8();
    let (w, h) = (img.width() as u16, img.height() as u16);
    Some((img.into_raw(), w, h))
}

/// Decode an image file and downscale it so neither side exceeds `max` pixels
/// (keeping aspect ratio), returning interleaved RGB + size. Bounds the memory
/// held per transmit slot.
pub fn load_source_bounded(bytes: &[u8], max: u16) -> Option<(Vec<u8>, u16, u16)> {
    let img = image::load_from_memory(bytes).ok()?;
    let img = if img.width() > max as u32 || img.height() > max as u32 {
        img.resize(max as u32, max as u32, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let rgb = img.to_rgb8();
    let (w, h) = (rgb.width() as u16, rgb.height() as u16);
    Some((rgb.into_raw(), w, h))
}

/// Encode interleaved RGB to PNG.
pub fn encode_png(rgb: &[u8], w: u16, h: u16) -> Option<Vec<u8>> {
    let img = image::RgbImage::from_raw(w as u32, h as u32, rgb.to_vec())?;
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img).write_to(&mut buf, image::ImageFormat::Png).ok()?;
    Some(buf.into_inner())
}

/// Crop-and-scale a source image to exactly `(w, h)`, filling the frame
/// (centre-crop, preserving aspect ratio).
pub fn crop_scale(src_rgb: &[u8], sw: u16, sh: u16, w: u16, h: u16) -> Vec<u8> {
    let Some(src) = image::RgbImage::from_raw(sw as u32, sh as u32, src_rgb.to_vec()) else {
        return vec![0u8; w as usize * h as usize * 3];
    };
    image::DynamicImage::ImageRgb8(src)
        .resize_to_fill(w as u32, h as u32, image::imageops::FilterType::Triangle)
        .to_rgb8()
        .into_raw()
}

/// Build the final transmit image: crop/scale the source to the mode size, add
/// the header strip, then the message overlay. Returns `(rgb, w, h)`.
pub fn compose(
    mode: SstvMode,
    src_rgb: &[u8],
    sw: u16,
    sh: u16,
    message: &str,
    callsign: &str,
) -> (Vec<u8>, u16, u16) {
    let (w, h) = mode.dimensions();
    let mut img = crop_scale(src_rgb, sw, sh, w, h);
    draw_header(&mut img, w as usize, h as usize, callsign);
    draw_message(&mut img, w as usize, h as usize, message);
    (img, w, h)
}

/// Convert interleaved RGB to an egui image for a texture.
pub fn color_image(rgb: &[u8], w: u16, h: u16) -> egui::ColorImage {
    egui::ColorImage::from_rgb([w as usize, h as usize], rgb)
}

fn put(img: &mut [u8], w: usize, h: usize, x: i32, y: i32, r: u8, g: u8, b: u8) {
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return;
    }
    let i = (y as usize * w + x as usize) * 3;
    img[i] = r;
    img[i + 1] = g;
    img[i + 2] = b;
}

fn blend(img: &mut [u8], w: usize, h: usize, x: i32, y: i32, r: u8, g: u8, b: u8, a: f32) {
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return;
    }
    let a = a.clamp(0.0, 1.0);
    let i = (y as usize * w + x as usize) * 3;
    let mix = |o: u8, n: u8| (o as f32 * (1.0 - a) + n as f32 * a).round().clamp(0.0, 255.0) as u8;
    img[i] = mix(img[i], r);
    img[i + 1] = mix(img[i + 1], g);
    img[i + 2] = mix(img[i + 2], b);
}

/// The red→black gradient strip: operator callsign on the left, program name +
/// version on the right ("SDRoxide vX.Y.Z").
fn draw_header(img: &mut [u8], w: usize, h: usize, callsign: &str) {
    let strip = HEADER_H.min(h);
    for y in 0..strip {
        // Red at the top fading to black at the bottom of the strip.
        let t = 1.0 - (y as f32 / strip.max(1) as f32);
        let r = (170.0 * t) as u8;
        for x in 0..w {
            put(img, w, h, x as i32, y as i32, r, 0, 0);
        }
    }
    if let Some(font) = message_font() {
        let scale = PxScale::from(11.0);
        let baseline = (strip as f32 * 0.72).round();
        // Left: operator callsign (uppercased).
        let call = callsign.trim().to_uppercase();
        if !call.is_empty() {
            draw_text(img, w, h, 4.0, baseline, &call, &font, scale, (255, 255, 255), 1.0);
        }
        // Right: program name + version.
        let brand = format!("SDRoxide v{}", env!("CARGO_PKG_VERSION"));
        let tw = text_width(&brand, &font, scale);
        draw_text(img, w, h, w as f32 - tw - 4.0, baseline, &brand, &font, scale, (235, 235, 235), 1.0);
    }
}

/// Overlay the message in a single font, white with a black outline, starting
/// just below the header. The first line is drawn at double the size of the
/// rest (a title line), with its outline thickened to match.
fn draw_message(img: &mut [u8], w: usize, h: usize, message: &str) {
    let Some(font) = message_font() else {
        return;
    };
    let base_px = 30.0_f32;
    let mut baseline = HEADER_H as f32;
    for (i, line) in message.lines().enumerate() {
        // First line twice as large; the line height and outline scale with it.
        let px = if i == 0 { base_px * 1.5 } else { base_px };
        let line_h = px * 1.2;
        baseline += line_h;
        if line.trim().is_empty() {
            continue;
        }
        let scale = PxScale::from(px);
        let outline = px / base_px * 1.5;
        // Black outline: draw the glyphs offset in eight directions.
        for (ox, oy) in [
            (-outline, 0.0),
            (outline, 0.0),
            (0.0, -outline),
            (0.0, outline),
            (-outline, -outline),
            (outline, -outline),
            (-outline, outline),
            (outline, outline),
        ] {
            draw_text(img, w, h, 6.0 + ox, baseline + oy, line, &font, scale, (0, 0, 0), 1.0);
        }
        draw_text(img, w, h, 6.0, baseline, line, &font, scale, (255, 255, 255), 1.0);
        if baseline as usize >= h {
            break;
        }
    }
}

fn text_width(text: &str, font: &FontRef<'static>, scale: PxScale) -> f32 {
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

fn draw_text(
    img: &mut [u8],
    w: usize,
    h: usize,
    x: f32,
    baseline: f32,
    text: &str,
    font: &FontRef<'static>,
    scale: PxScale,
    color: (u8, u8, u8),
    alpha: f32,
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
                blend(img, w, h, px, py, color.0, color.1, color.2, cov * alpha);
            });
        }
        caret += scaled.h_advance(gid);
        prev = Some(gid);
    }
}
