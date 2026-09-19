//! SSTV image compositing for transmit: crop/scale a source picture to the
//! selected mode's dimensions, stamp the operator's banner strip across the
//! top of it, and overlay the slot's multi-line message (one bundled font, bold
//! with a black outline for readability, with the first line drawn at double
//! size).
//!
//! The banner is what the station puts its name on, so its two texts, its two
//! colours and its height come from [`sdroxide_types::DigiConfig`] rather than
//! from here — see [`Banner`]. The stock settings compose exactly the strip
//! this module used to hard-wire: the callsign at the left, `SDRoxide vX.Y.Z`
//! at the right, white on red fading to black.
//!
//! Pure-Rust (`image` + `ab_glyph`) so it runs identically in the native app and
//! the wasm browser client — the composed buffer is both the live preview and,
//! PNG-encoded, the transmit payload.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use eframe::egui;

/// The banner strip drawn across the top of a transmitted picture, with the
/// operator's templates already resolved against the station identity.
///
/// Resolved rather than carried as templates because the same banner is drawn
/// several times for one picture — the live preview on every keystroke, then
/// the transmit copy — and because it keeps the drawing code free of any
/// notion of what a `{call}` is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Banner {
    /// Height of the strip in picture pixels. The text is sized from it.
    pub height: u16,
    /// Text printed at the left end.
    pub left: String,
    /// Text printed at the right end, right-aligned.
    pub right: String,
    /// Colour at the top of the strip. It fades to [`Self::fill2`], or to black
    /// when that is `None`.
    pub fill: [u8; 3],
    /// The colour the strip fades to at its bottom, when the operator asked for
    /// a gradient. `None` fades to black, which is the original look.
    pub fill2: Option<[u8; 3]>,
    /// Colour both texts are printed in.
    pub ink: [u8; 3],
    /// The colour the text fades to across the strip, when the operator asked
    /// for a text gradient. `None` prints it in one colour.
    pub ink2: Option<[u8; 3]>,
    /// Override both texts with a horizontal rainbow, whatever the colours
    /// above say.
    pub rainbow: bool,
    /// Colour the texts are outlined in, when the operator asked for an
    /// outline. `None` prints them plain.
    pub outline: Option<[u8; 3]>,
}

impl Banner {
    /// The banner the station's digital-mode config asks for, or `None` when
    /// the operator has switched it off.
    ///
    /// A banner whose two texts both resolve to nothing still draws its strip:
    /// blanking the text is not the same request as turning the banner off,
    /// and a strip that vanished when a callsign had not been entered yet
    /// would look like a bug rather than like an empty field.
    pub fn from_config(cfg: &sdroxide_types::DigiConfig) -> Option<Banner> {
        cfg.sstv_banner.then(|| Banner {
            // A zero height would be an invisible banner that still pushed the
            // message down by nothing; one pixel is the smallest honest strip.
            height: cfg.sstv_banner_height.max(1),
            left: expand(&cfg.sstv_banner_left, &cfg.my_call, &cfg.my_grid),
            right: expand(&cfg.sstv_banner_right, &cfg.my_call, &cfg.my_grid),
            fill: cfg.sstv_banner_fill,
            fill2: cfg.sstv_style.banner_gradient.then_some(cfg.sstv_style.banner_fill2),
            ink: cfg.sstv_banner_ink,
            ink2: cfg.sstv_style.banner_ink_gradient.then_some(cfg.sstv_style.banner_ink2),
            rainbow: cfg.sstv_style.rainbow_text,
            outline: cfg.sstv_style.banner_outline.then_some(cfg.sstv_style.banner_outline_ink),
        })
    }
}

/// Substitute the banner placeholders in `template`.
///
/// `{call}` is uppercased — a callsign belongs in capitals on the air and the
/// header always printed it that way — but the template around it is left as
/// the operator typed it, which is what stops `SDRoxide` becoming `SDROXIDE`.
/// An unrecognised `{…}` is copied through untouched so a typo is visible in
/// the preview rather than silently printing nothing.
pub fn expand(template: &str, call: &str, grid: &str) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        rest = &rest[open..];
        let Some(close) = rest.find('}') else {
            // An unclosed brace is just text.
            break;
        };
        match rest[1..close].to_ascii_lowercase().as_str() {
            "call" => out.push_str(call.trim().to_uppercase().as_str()),
            "grid" => out.push_str(grid.trim()),
            "version" => out.push_str(env!("CARGO_PKG_VERSION")),
            _ => out.push_str(&rest[..=close]),
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

/// The placeholders [`expand`] knows, with a word on each, for the hint text
/// beside the two fields that take them.
pub const BANNER_PLACEHOLDERS: [(&str, &str); 3] = [
    ("{call}", "your callsign, in capitals"),
    ("{grid}", "your locator"),
    ("{version}", "the sdroxide version"),
];

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

/// Build the final transmit image: crop/scale the source to `(w, h)`, add the
/// banner strip, then the message overlay. Returns `(rgb, w, h)`.
///
/// The size is the caller's: SSTV takes it from the line format, RIFP from the
/// operator, since the protocol fixes none of its own. `banner` is `None` when
/// the operator has switched the strip off, and the message then starts at the
/// top of the picture instead of below it.
pub fn compose(
    w: u16,
    h: u16,
    src_rgb: &[u8],
    sw: u16,
    sh: u16,
    message: &str,
    banner: Option<&Banner>,
    style: &sdroxide_types::SstvStyle,
) -> (Vec<u8>, u16, u16) {
    let mut img = crop_scale(src_rgb, sw, sh, w, h);
    let strip = match banner {
        Some(b) => draw_banner(&mut img, w as usize, h as usize, b),
        None => 0,
    };
    draw_message(&mut img, w as usize, h as usize, message, strip, style);
    (img, w, h)
}

/// Convert interleaved RGB to an egui image for a texture.
pub fn color_image(rgb: &[u8], w: u16, h: u16) -> egui::ColorImage {
    egui::ColorImage::from_rgb([w as usize, h as usize], rgb)
}

// ── Overlay messages: who owns the text ──
//
// The presets live in the engine, so the message for a slot arrives as part of
// its status and can change while the operator is looking at it — another
// screen attached to the same radio, or the echo of this one's own write coming
// back a round trip later. Adopting every echo would put the cursor back where
// it was when the write went out; ignoring them all would mean an edit made
// elsewhere never showed up.
//
// So exactly one slot is client-owned at a time: whichever the operator is
// typing in. Every other slot takes the engine's word. The same arrangement the
// voice keyer's slot labels use, kept here rather than in the panel so it can be
// tested without an `egui::Context`.

/// The text to show for `slot`: what is being typed if it is this slot,
/// otherwise what the engine says.
pub fn message_shown<'a>(
    edit: &'a Option<(usize, String)>,
    presets: &'a sdroxide_types::ImagePresets,
    slot: usize,
) -> &'a str {
    match edit {
        Some((i, text)) if *i == slot => text,
        _ => presets.slots.get(slot).map_or("", |s| s.message.as_str()),
    }
}

/// Give a claimed slot back to the engine, returning the command to send when
/// the text actually changed.
///
/// `None` when nothing was claimed or nothing was edited, so an unfocused click
/// does not spray writes at the engine.
pub fn commit_message(
    edit: &mut Option<(usize, String)>,
    presets: &sdroxide_types::ImagePresets,
) -> Option<sdroxide_types::Command> {
    let (slot, text) = edit.take()?;
    (presets.slots.get(slot).map(|s| s.message.as_str()) != Some(text.as_str()))
        .then(|| sdroxide_types::Command::ImageSetMessage { slot: slot as u8, message: text })
}

/// Claim `slot` for editing, committing whatever was claimed before.
pub fn claim_message(
    edit: &mut Option<(usize, String)>,
    presets: &sdroxide_types::ImagePresets,
    slot: usize,
) -> Option<sdroxide_types::Command> {
    if matches!(edit, Some((i, _)) if *i == slot) {
        return None;
    }
    let cmd = commit_message(edit, presets);
    *edit = Some((slot, message_shown(&None, presets, slot).to_string()));
    cmd
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

/// The banner: a strip fading from the operator's colour at the top to black
/// at the bottom, their left text at the left and their right text at the
/// right. Returns how many rows of the picture it actually covered, which is
/// where the message overlay starts.
///
/// The type sizes off the height rather than being a setting of its own — the
/// eleven points the strip was drawn at were eleven points *because* it was
/// sixteen pixels tall, and two controls that have to be turned together are
/// worse than one.
fn draw_banner(img: &mut [u8], w: usize, h: usize, banner: &Banner) -> usize {
    let strip = usize::from(banner.height).min(h);
    if strip == 0 {
        return 0;
    }
    let [fr, fg, fb] = banner.fill;
    for y in 0..strip {
        // `t` runs 0 at the top edge to 1 at the bottom. With a second colour
        // the strip is a straight two-stop gradient; without one it keeps the
        // original fade to black.
        let t = y as f32 / strip as f32;
        let (r, g, b) = match banner.fill2 {
            Some([r2, g2, b2]) => {
                let lerp = |a: u8, c: u8| (f32::from(a) + (f32::from(c) - f32::from(a)) * t) as u8;
                (lerp(fr, r2), lerp(fg, g2), lerp(fb, b2))
            }
            None => {
                let shade = 1.0 - t;
                (
                    (f32::from(fr) * shade) as u8,
                    (f32::from(fg) * shade) as u8,
                    (f32::from(fb) * shade) as u8,
                )
            }
        };
        for x in 0..w {
            put(img, w, h, x as i32, y as i32, r, g, b);
        }
    }
    let Some(font) = message_font() else {
        return strip;
    };
    let ink = if banner.rainbow {
        Ink::Rainbow
    } else if let Some(c2) = banner.ink2 {
        Ink::Gradient(banner.ink, c2)
    } else {
        Ink::Solid(banner.ink)
    };
    let scale = PxScale::from(strip as f32 * 11.0 / 16.0);
    let baseline = (strip as f32 * 0.72).round();
    // The inset scales with the strip too, so a tall banner does not print
    // hard against the edge of the picture.
    let pad = (strip as f32 * 4.0 / 16.0).max(1.0);
    // An outline, in eight directions, ahead of the ink — the message overlay's
    // technique, sized to the strip so a taller banner gets a bolder edge.
    let outline_off = (strip as f32 * 1.0 / 16.0).max(1.0);
    let draw_one = |img: &mut [u8], text: &str, x: f32| {
        if let Some(oc) = banner.outline {
            for (ox, oy) in [
                (-outline_off, 0.0),
                (outline_off, 0.0),
                (0.0, -outline_off),
                (0.0, outline_off),
                (-outline_off, -outline_off),
                (outline_off, -outline_off),
                (-outline_off, outline_off),
                (outline_off, outline_off),
            ] {
                draw_text(
                    img,
                    w,
                    h,
                    x + ox,
                    baseline + oy,
                    text,
                    &font,
                    scale,
                    Ink::Solid(oc),
                    1.0,
                );
            }
        }
        draw_text(img, w, h, x, baseline, text, &font, scale, ink, 1.0);
    };
    if !banner.left.is_empty() {
        draw_one(img, &banner.left, pad);
    }
    if !banner.right.is_empty() {
        let tw = text_width(&banner.right, &font, scale);
        draw_one(img, &banner.right, w as f32 - tw - pad);
    }
    strip
}

/// Overlay the message in a single font, its colour and outline from
/// `style`, starting just below the banner — or at the top of the picture when
/// there is none. The first line is drawn at double the size of the rest (a
/// title line), with its outline thickened to match.
fn draw_message(
    img: &mut [u8],
    w: usize,
    h: usize,
    message: &str,
    top: usize,
    style: &sdroxide_types::SstvStyle,
) {
    let Some(font) = message_font() else {
        return;
    };
    let ink = if style.rainbow_text { Ink::Rainbow } else { Ink::Solid(style.message_ink) };
    let base_px = 30.0_f32;
    let mut baseline = top as f32;
    for (i, line) in message.lines().enumerate() {
        // First line twice as large; the line height and outline scale with it.
        let px = if i == 0 { base_px * 1.5 } else { base_px };
        let line_h = px * 1.2;
        baseline += line_h;
        if line.trim().is_empty() {
            continue;
        }
        let scale = PxScale::from(px);
        if style.message_outline {
            let oc = style.message_outline_ink;
            let outline = px / base_px * 1.5;
            // Outline: draw the glyphs offset in eight directions.
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
                draw_text(
                    img,
                    w,
                    h,
                    6.0 + ox,
                    baseline + oy,
                    line,
                    &font,
                    scale,
                    Ink::Solid(oc),
                    1.0,
                );
            }
        }
        draw_text(img, w, h, 6.0, baseline, line, &font, scale, ink, 1.0);
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

/// How text is coloured as it is drawn: one colour, a gradient across the
/// picture, or a rainbow. `x` is the pixel's own position and `span` the width
/// the gradient or rainbow runs over (the picture's), so the same call draws
/// every variant.
#[derive(Clone, Copy)]
enum Ink {
    Solid([u8; 3]),
    Gradient([u8; 3], [u8; 3]),
    Rainbow,
}

impl Ink {
    /// The colour at pixel column `x`, over a run `span` wide.
    fn at(self, x: f32, span: f32) -> (u8, u8, u8) {
        let span = span.max(1.0);
        match self {
            Ink::Solid(c) => (c[0], c[1], c[2]),
            Ink::Gradient(a, b) => {
                let t = (x / span).clamp(0.0, 1.0);
                let l = |p: u8, q: u8| (f32::from(p) + (f32::from(q) - f32::from(p)) * t) as u8;
                (l(a[0], b[0]), l(a[1], b[1]), l(a[2], b[2]))
            }
            Ink::Rainbow => {
                // Hue sweeps a full turn across the picture's width. Saturation
                // and value are full, which is what makes it read as a rainbow
                // rather than a pastel.
                let hue = (x / span).rem_euclid(1.0) * 360.0;
                hsv_to_rgb(hue, 1.0, 1.0)
            }
        }
    }
}

/// HSV (h 0..360, s/v 0..1) to 8-bit RGB.
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let to = |f: f32| ((f + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    (to(r), to(g), to(b))
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
    ink: Ink,
    alpha: f32,
) {
    let span = w as f32;
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
                let (r, g, b) = ink.at(px as f32, span);
                blend(img, w, h, px, py, r, g, b, cov * alpha);
            });
        }
        caret += scaled.h_advance(gid);
        prev = Some(gid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::{Command, DigiConfig, ImagePresets, ImageSlotInfo, SstvStyle};

    fn presets(messages: &[&str]) -> ImagePresets {
        ImagePresets {
            slots: messages
                .iter()
                .map(|m| ImageSlotInfo { message: (*m).into(), ..ImageSlotInfo::default() })
                .collect(),
        }
    }

    /// The echo of our own write comes back a round trip after it went out.
    /// Adopting it would drop the cursor back to where it was when the operator
    /// started the sentence.
    #[test]
    fn a_server_echo_does_not_clobber_the_slot_being_typed_in() {
        let p = presets(&["a", "b", "old", "d", "e"]);
        let edit = Some((2, "CQ CQ de OE1TEST".to_string()));
        assert_eq!(message_shown(&edit, &p, 2), "CQ CQ de OE1TEST");
    }

    /// An edit made on another screen still has to appear — that is the whole
    /// point of the presets living in the engine.
    #[test]
    fn an_edit_made_elsewhere_reaches_every_slot_but_the_one_in_hand() {
        let p = presets(&["a", "b", "old", "d", "e"]);
        let edit = Some((2, "typing".to_string()));
        assert_eq!(message_shown(&edit, &p, 0), "a");
        assert_eq!(message_shown(&edit, &p, 3), "d");
        // With nothing claimed, every slot follows the engine.
        assert_eq!(message_shown(&None, &p, 2), "old");
        // A slot the engine has never mentioned reads empty rather than panicking.
        assert_eq!(message_shown(&None, &p, 99), "");
    }

    #[test]
    fn leaving_a_slot_commits_it_and_gives_it_back() {
        let p = presets(&["a", "b", "old", "d", "e"]);
        let mut edit = Some((2, "new".to_string()));
        let cmd = commit_message(&mut edit, &p);
        assert_eq!(cmd, Some(Command::ImageSetMessage { slot: 2, message: "new".into() }));
        assert!(edit.is_none(), "the claim is released");

        // Clicking away without having changed anything must not write.
        let mut edit = Some((2, "old".to_string()));
        assert_eq!(commit_message(&mut edit, &p), None);
        assert!(edit.is_none());
        // And with nothing claimed there is nothing to commit.
        assert_eq!(commit_message(&mut None, &p), None);
    }

    #[test]
    fn switching_slots_takes_the_engines_text_for_the_new_one() {
        let p = presets(&["a", "b", "old", "d", "e"]);
        let mut edit = Some((2, "half-typed".to_string()));
        // Moving to slot 4 flushes slot 2 and picks up slot 4's stored text.
        let cmd = claim_message(&mut edit, &p, 4);
        assert_eq!(cmd, Some(Command::ImageSetMessage { slot: 2, message: "half-typed".into() }));
        assert_eq!(edit, Some((4, "e".to_string())));
        // Re-claiming the slot already in hand keeps what is being typed.
        edit = Some((4, "e and more".to_string()));
        assert_eq!(claim_message(&mut edit, &p, 4), None);
        assert_eq!(edit, Some((4, "e and more".to_string())));
    }

    /// The stock settings have to compose the strip the header was hard-wired
    /// to draw before it could be edited, or every existing station's pictures
    /// change appearance on upgrade.
    #[test]
    fn the_stock_banner_is_the_header_that_was_hard_wired() {
        let cfg = DigiConfig { my_call: "oe1test".into(), ..DigiConfig::default() };
        let b = Banner::from_config(&cfg).expect("on by default");
        assert_eq!(b.left, "OE1TEST");
        assert_eq!(b.right, format!("SDRoxide v{}", env!("CARGO_PKG_VERSION")));
        assert_eq!(b.height, 16);
        assert_eq!(b.fill, [170, 0, 0]);
    }

    #[test]
    fn switching_the_banner_off_leaves_no_strip_to_draw() {
        let cfg = DigiConfig { sstv_banner: false, ..DigiConfig::default() };
        assert_eq!(Banner::from_config(&cfg), None);
    }

    /// The callsign goes up in capitals, but only the callsign: uppercasing the
    /// whole template would turn `SDRoxide` into `SDROXIDE`.
    #[test]
    fn only_the_callsign_is_uppercased() {
        assert_eq!(expand("de {call} ", " oe1test ", ""), "de OE1TEST ");
        assert_eq!(expand("SDRoxide {grid}", "", "jn88"), "SDRoxide jn88");
        assert_eq!(expand("v{version}", "", ""), format!("v{}", env!("CARGO_PKG_VERSION")));
        // Case-insensitive, so {CALL} works as well as {call}.
        assert_eq!(expand("{CALL}", "oe1test", ""), "OE1TEST");
    }

    /// A typo has to reach the preview as itself. Swallowing it would leave the
    /// operator staring at a gap with nothing to tell them what went wrong.
    #[test]
    fn an_unknown_placeholder_survives_untouched() {
        assert_eq!(expand("{callsign} {call}", "oe1test", ""), "{callsign} OE1TEST");
        assert_eq!(expand("100% {open", "", ""), "100% {open");
        assert_eq!(expand("}{}{", "", ""), "}{}{");
    }

    /// The message used to start at a fixed 16 rows whether or not a strip was
    /// there. With the banner off, the picture is the operator's from the top.
    #[test]
    fn the_message_starts_below_whatever_the_banner_actually_covered() {
        // A three-pixel-tall picture cannot hold a 16-pixel banner; the strip
        // must report what it covered, not what it was asked for.
        let mut img = vec![0u8; 8 * 3 * 3];
        let tall = Banner { height: 16, ..Banner::default() };
        assert_eq!(draw_banner(&mut img, 8, 3, &tall), 3);
        let none = Banner { height: 0, ..Banner::default() };
        assert_eq!(draw_banner(&mut img, 8, 3, &none), 0);
    }

    /// Composing with no banner must leave the top row of the picture alone —
    /// it is the check that "off" means off rather than "a black strip".
    #[test]
    fn composing_without_a_banner_leaves_the_top_row_of_the_picture() {
        let src = vec![200u8; 4 * 4 * 3];
        let (with, _, _) = compose(
            4,
            4,
            &src,
            4,
            4,
            "",
            Some(&Banner { height: 2, fill: [170, 0, 0], ..Banner::default() }),
            &sdroxide_types::SstvStyle::default(),
        );
        let (without, _, _) =
            compose(4, 4, &src, 4, 4, "", None, &sdroxide_types::SstvStyle::default());
        assert_eq!(with[0], 170);
        assert_eq!(&without[..12], &[200u8; 12]);
    }

    /// The gradient option: the strip runs between the two colours instead of
    /// fading to black.
    #[test]
    fn a_gradient_strip_runs_between_its_two_colours() {
        let mut img = vec![0u8; 8 * 8 * 3];
        let b =
            Banner { height: 8, fill: [200, 0, 0], fill2: Some([0, 0, 200]), ..Banner::default() };
        assert_eq!(draw_banner(&mut img, 8, 8, &b), 8);
        assert_eq!(&img[0..3], &[200, 0, 0], "top row should be the first colour");
        let last = &img[7 * 8 * 3..7 * 8 * 3 + 3];
        assert!(last[2] > 150 && last[0] < 60, "bottom row should be mostly the second: {last:?}");
    }

    /// The outline option: the chosen outline colour reaches the strip, around
    /// the ink.
    #[test]
    fn a_text_outline_puts_the_outline_colour_in_the_strip() {
        let mut img = vec![0u8; 96 * 28 * 3];
        let b = Banner {
            height: 28,
            left: "OO".to_string(),
            fill: [0, 160, 0],
            ink: [255, 255, 255],
            outline: Some([255, 0, 0]),
            ..Banner::default()
        };
        draw_banner(&mut img, 96, 28, &b);
        let has = |c: [u8; 3]| {
            img.chunks_exact(3).any(|p| {
                (i32::from(p[0]) - i32::from(c[0])).abs() < 40
                    && (i32::from(p[1]) - i32::from(c[1])).abs() < 40
                    && (i32::from(p[2]) - i32::from(c[2])).abs() < 40
            })
        };
        assert!(has([255, 255, 255]), "the white ink is missing");
        assert!(has([255, 0, 0]), "the red outline is missing");
    }

    /// The text gradient: the ink moves from the first colour to the second
    /// across the width, so red pixels sit left of blue ones.
    #[test]
    fn a_text_gradient_runs_across_the_strip() {
        let (w, h) = (240usize, 28usize);
        let mut img = vec![0u8; w * h * 3];
        let b = Banner {
            height: 28,
            left: "IIIIIIIIII".to_string(),
            fill: [0, 0, 0],
            ink: [255, 0, 0],
            ink2: Some([0, 0, 255]),
            ..Banner::default()
        };
        draw_banner(&mut img, w, h, &b);
        // Compare the leftmost and rightmost inked pixels: a gradient moves the
        // colour from the first toward the second across the text, whatever the
        // text's own extent within the strip.
        let at = |x: usize, y: usize| {
            let i = (y * w + x) * 3;
            (i32::from(img[i]), i32::from(img[i + 1]), i32::from(img[i + 2]))
        };
        let (mut left, mut right) = (None, None);
        for y in 0..h {
            for x in 0..w {
                let p = at(x, y);
                if p.0.max(p.1).max(p.2) > 30 {
                    if left.is_none() {
                        left = Some((x, y));
                    }
                    right = Some((x, y));
                }
            }
        }
        let (lx, ly) = left.expect("some text should be drawn");
        let (rx, ry) = right.expect("some text should be drawn");
        let (lr, _, lb) = at(lx, ly);
        let (rr, _, rb) = at(rx, ry);
        assert!(rx > lx, "the text should span some width");
        assert!(lr > rr && rb > lb, "left {lr},{lb} should be redder than right {rr},{rb}");
    }

    /// The rainbow override covers the spectrum, not one hue.
    #[test]
    fn rainbow_text_covers_the_spectrum() {
        let (w, h) = (320usize, 120usize);
        let mut img = vec![0u8; w * h * 3];
        let style =
            SstvStyle { rainbow_text: true, message_outline: false, ..SstvStyle::default() };
        draw_message(&mut img, w, h, "WWWWWWWWWWWWWWWWWWWWWWWW", 0, &style);
        let (mut reds, mut greens, mut blues) = (0, 0, 0);
        for p in img.chunks_exact(3) {
            let (r, g, b) = (i32::from(p[0]), i32::from(p[1]), i32::from(p[2]));
            if r > 150 && g < 80 && b < 80 {
                reds += 1;
            }
            if g > 150 && r < 80 && b < 80 {
                greens += 1;
            }
            if b > 150 && r < 80 && g < 80 {
                blues += 1;
            }
        }
        assert!(reds > 0 && greens > 0 && blues > 0, "r{reds} g{greens} b{blues}");
    }
}
