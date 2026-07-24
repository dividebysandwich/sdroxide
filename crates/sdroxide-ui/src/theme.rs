//! The sdroxide look: dark navy panels, cyan accents, hot-pink strokes,
//! sharp corners, Chakra Petch type (SIL OFL — see assets/fonts/OFL.txt).

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};

// Palette.
pub const BG_DEEP: Color32 = Color32::from_rgb(0x05, 0x08, 0x10);
pub const PANEL: Color32 = Color32::from_rgb(0x0b, 0x11, 0x1e);
pub const INPUT_BG: Color32 = Color32::from_rgb(0x04, 0x07, 0x0e);
pub const FILL: Color32 = Color32::from_rgb(0x10, 0x1a, 0x2c);
pub const FILL_HOVER: Color32 = Color32::from_rgb(0x17, 0x24, 0x3c);
pub const FILL_ACTIVE: Color32 = Color32::from_rgb(0x1d, 0x2f, 0x4d);
pub const LINE: Color32 = Color32::from_rgb(0x1a, 0x27, 0x40);
pub const LINE_LIT: Color32 = Color32::from_rgb(0x2a, 0x4a, 0x66);
pub const TEXT: Color32 = Color32::from_rgb(0xb4, 0xc6, 0xda);
pub const TEXT_STRONG: Color32 = Color32::from_rgb(0xe8, 0xf4, 0xff);
pub const CYAN: Color32 = Color32::from_rgb(0x00, 0xd0, 0xf4);
pub const CYAN_DIM: Color32 = Color32::from_rgb(0x1d, 0x9c, 0xbe);
pub const PINK: Color32 = Color32::from_rgb(0xff, 0x2a, 0x55);
pub const YELLOW: Color32 = Color32::from_rgb(0xff, 0xd2, 0x3f);
pub const GREEN: Color32 = Color32::from_rgb(0x46, 0xe0, 0x7d);
/// Dark ink used on top of cyan fills.
pub const INK_ON_CYAN: Color32 = Color32::from_rgb(0x02, 0x10, 0x19);
// Red-accent chrome (cyberpunk box borders / list rows).
pub const RED_DEEP: Color32 = Color32::from_rgb(0x6e, 0x18, 0x2c);
pub const CQ_BG: Color32 = Color32::from_rgb(0x24, 0x0c, 0x15);
/// Background for a decode addressed to our own station (warm gold, stands out).
pub const TOME_BG: Color32 = Color32::from_rgb(0x2c, 0x24, 0x06);
pub const ROW_BG: Color32 = Color32::from_rgb(0x0a, 0x10, 0x1b);
pub const ROW_HOVER: Color32 = Color32::from_rgb(0x14, 0x1e, 0x2e);

pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);

    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| {

    style.text_styles = [
        (TextStyle::Heading, FontId::new(16.0, FontFamily::Name("chakra-bold".into()))),
        (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
    ]
    .into();

    style.spacing.item_spacing = egui::vec2(7.0, 5.0);
    style.spacing.button_padding = egui::vec2(7.0, 3.0);
    // Fixed slider width: otherwise sliders expand to fill the row, so a
    // module with a slider balloons and pushes later modules off-screen
    // instead of letting `horizontal_wrapped` wrap them.
    style.spacing.slider_width = 84.0;
    style.spacing.combo_width = 84.0;

    let v = &mut style.visuals;
    v.dark_mode = true;
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = INPUT_BG;
    v.faint_bg_color = Color32::from_rgb(0x0e, 0x16, 0x26);
    v.code_bg_color = INPUT_BG;

    v.window_stroke = Stroke::new(1.0, PINK);
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;

    v.selection.bg_fill = CYAN;
    v.selection.stroke = Stroke::new(1.0, INK_ON_CYAN);
    v.hyperlink_color = CYAN;
    v.warn_fg_color = YELLOW;
    v.error_fg_color = PINK;
    v.slider_trailing_fill = true;

    let sharp = CornerRadius::ZERO;
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.noninteractive.weak_bg_fill = PANEL;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.noninteractive.corner_radius = sharp;

    v.widgets.inactive.bg_fill = FILL;
    v.widgets.inactive.weak_bg_fill = FILL;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, LINE_LIT);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.inactive.corner_radius = sharp;

    v.widgets.hovered.bg_fill = FILL_HOVER;
    v.widgets.hovered.weak_bg_fill = FILL_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, CYAN_DIM);
    v.widgets.hovered.fg_stroke = Stroke::new(1.2, TEXT_STRONG);
    v.widgets.hovered.corner_radius = sharp;

    v.widgets.active.bg_fill = FILL_ACTIVE;
    v.widgets.active.weak_bg_fill = FILL_ACTIVE;
    v.widgets.active.bg_stroke = Stroke::new(1.0, CYAN);
    v.widgets.active.fg_stroke = Stroke::new(1.2, CYAN);
    v.widgets.active.corner_radius = sharp;

    v.widgets.open.bg_fill = FILL_ACTIVE;
    v.widgets.open.weak_bg_fill = FILL_ACTIVE;
    v.widgets.open.bg_stroke = Stroke::new(1.0, CYAN_DIM);
    v.widgets.open.fg_stroke = Stroke::new(1.0, TEXT_STRONG);
    v.widgets.open.corner_radius = sharp;

    });
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "chakra".into(),
        Arc::new(FontData::from_static(include_bytes!("../assets/fonts/ChakraPetch-Regular.ttf"))),
    );
    fonts.font_data.insert(
        "chakra-bold".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/ChakraPetch-SemiBold.ttf"
        ))),
    );
    // Angular techno monospace for the FT8 decode list (Share Tech Mono, OFL).
    fonts.font_data.insert(
        "cyber-mono".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/ShareTechMono-Regular.ttf"
        ))),
    );

    if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
        prop.insert(0, "chakra".into());
    }
    // Make Share Tech Mono the primary monospace (used by the decode list,
    // frequency readout, and meters).
    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono.insert(0, "cyber-mono".into());
    }
    fonts
        .families
        .insert(FontFamily::Name("cyber-mono".into()), vec!["cyber-mono".to_string()]);
    // Bold family for headings, falling back through the proportional stack.
    let mut bold_stack = vec!["chakra-bold".to_string()];
    if let Some(prop) = fonts.families.get(&FontFamily::Proportional) {
        bold_stack.extend(prop.iter().cloned());
    }
    fonts.families.insert(FontFamily::Name("chakra-bold".into()), bold_stack);

    ctx.set_fonts(fonts);
}
