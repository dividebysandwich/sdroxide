//! Whole-app layout tests for the PLAYBACK, WEATHER SATELLITES and RADIOSONDES
//! windows and the chips that open them: each window filled with what a real
//! session puts in it, at the screen sizes and layout tiers people run, in
//! English and in German (the longest strings), at its opening size and dragged
//! down to the smallest the operator can make it. Nothing may be laid out past
//! the edge that clips it, the window has to stay on the screen, and every
//! control in it has to be on show or a scroll away.
//!
//! Ignored in the normal run, as the other whole-app tests in `frame.rs` are:
//! the app sets the language, the look and the tier process-wide. They hold a
//! lock against each other, so `--ignored` with a filter that matches several
//! is safe. Run one with, for example,
//! `cargo test -p sdroxide-ui --lib the_sonde_window_fits -- --ignored`.

use eframe::egui;
use sdroxide_types::{LayoutMode, Mode, UiLanguage};

use super::SdroxideApp;

/// One at a time: the language and the tier are process-wide.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A controller with nothing behind it: enough to draw the whole app.
struct NullCtrl;
impl sdroxide_types::RadioController for NullCtrl {
    fn send(&mut self, _cmd: sdroxide_types::Command) {}
    fn poll_event(&mut self) -> Option<sdroxide_types::RadioEvent> {
        None
    }
}

/// The screens: the four common desktop sizes on whatever tier they pick, a
/// small laptop on the SMALL layout, and a tablet and a phone for the browser.
const SCREENS: [(f32, f32, LayoutMode); 7] = [
    (1024.0, 600.0, LayoutMode::Auto),
    (1280.0, 720.0, LayoutMode::Auto),
    (1366.0, 768.0, LayoutMode::Auto),
    (1366.0, 768.0, LayoutMode::Small),
    (1920.0, 1080.0, LayoutMode::Auto),
    (820.0, 1180.0, LayoutMode::Tablet),
    (390.0, 844.0, LayoutMode::Phone),
];

const LANGUAGES: [&str; 2] = ["en", "de"];

fn new_app(ctx: &egui::Context, layout: LayoutMode, tag: &str) -> SdroxideApp {
    let mut app = SdroxideApp::new_tab(ctx, None, None, Box::new(NullCtrl), 0, false);
    app.ui_settings.layout = layout;
    app.ui_settings.language = UiLanguage::from_tag(tag).unwrap();
    app.state.rx[0].mode = Mode::Nfm;
    app.digi_cfg_edit.my_grid = "JO62qm".into();
    app
}

/// Draw `frames` frames, `events(i)` fed into frame `i`. Returns the shapes of
/// the last one.
fn run(
    ctx: &egui::Context,
    app: &mut SdroxideApp,
    frame: &mut eframe::Frame,
    screen: egui::Rect,
    t0: &mut f64,
    frames: usize,
    mut events: impl FnMut(usize) -> Vec<egui::Event>,
) -> Vec<egui::epaint::ClippedShape> {
    let mut shapes = Vec::new();
    for i in 0..frames {
        *t0 += 0.1;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(*t0),
                predicted_dt: 0.1,
                events: events(i),
                ..Default::default()
            },
            |ui| eframe::App::ui(app, ui, frame),
        );
        out.textures_delta.clear();
        shapes = out.shapes;
    }
    shapes
}

/// Every piece of text drawn, with where it is and the rectangle it is clipped to.
fn texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<(String, egui::Rect, egui::Rect)> {
    fn walk(s: &egui::Shape, clip: egui::Rect, out: &mut Vec<(String, egui::Rect, egui::Rect)>) {
        match s {
            egui::Shape::Text(t) => {
                out.push((t.galley.text().to_string(), t.visual_bounding_rect(), clip))
            }
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, clip, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for c in shapes {
        walk(&c.shape, c.clip_rect, &mut out);
    }
    out
}

/// The interactive widgets of a layer, as the pass just drawn placed them —
/// less the window's own resize edges and corners, which straddle its border
/// by design.
fn widgets(ctx: &egui::Context, layer: egui::LayerId) -> Vec<egui::WidgetRect> {
    let edge = egui::Id::new(layer).with("edge_drag");
    let edges: Vec<egui::Id> =
        ["right", "left", "bottom", "top", "right_bottom", "right_top", "left_bottom", "left_top"]
            .iter()
            .map(|s| edge.with(*s))
            .collect();
    let corner = layer.id.with("resize").with("__resize_corner");
    ctx.viewport(|v| {
        v.this_pass
            .widgets
            .get_layer(layer)
            .filter(|w| w.sense.senses_click() || w.sense.senses_drag())
            .filter(|w| !edges.contains(&w.id) && w.id != corner)
            .copied()
            .collect()
    })
}

/// What can be wrong with one window as drawn.
fn check_window(
    ctx: &egui::Context,
    shapes: &[egui::epaint::ClippedShape],
    win_id: egui::Id,
    screen: egui::Rect,
    exempt: &[egui::Rect],
) -> Vec<String> {
    let mut bad = Vec::new();
    let Some(win) = ctx.memory(|m| m.area_rect(win_id)) else {
        return vec!["window not drawn".into()];
    };
    if !screen.expand(1.0).contains_rect(win) {
        bad.push(format!(
            "window {:.0},{:.0}..{:.0},{:.0} leaves the {:.0}x{:.0} screen",
            win.left(),
            win.top(),
            win.right(),
            win.bottom(),
            screen.width(),
            screen.height()
        ));
    }
    // Text laid out wider than what clips it, inside the window. Text clipped
    // by a map (its labels run off its edge by design) is left alone.
    for (t, r, clip) in texts(shapes) {
        if !r.is_finite()
            || !win.expand(2.0).contains_rect(clip)
            || exempt.iter().any(|e| {
                e.expand(0.5).contains_rect(clip)
                    || (r.top() >= e.top() - 0.5 && r.bottom() <= e.bottom() + 0.5)
            })
        {
            continue;
        }
        let over = (r.right() - clip.right()).max(clip.left() - r.left());
        if over > 1.0 && r.top() < clip.bottom() && r.bottom() > clip.top() {
            bad.push(format!("text {t:?} cut off by {over:.1} pt"));
        }
    }
    // Controls narrower on screen than they were laid out: cut at the side.
    let layer = egui::LayerId::new(egui::Order::Middle, win_id);
    for w in widgets(ctx, layer) {
        // Inside a sideways scroll, past the side is a scroll away.
        if exempt
            .iter()
            .any(|e| w.rect.top() >= e.top() - 0.5 && w.rect.bottom() <= e.bottom() + 0.5)
        {
            continue;
        }
        let over = (w.rect.right() - win.right()).max(win.left() - w.rect.left());
        if over > 1.0 {
            bad.push(format!(
                "a control at x {:.0}..{:.0} is {over:.1} pt past the side",
                w.rect.left(),
                w.rect.right()
            ));
        }
    }
    bad
}

/// The controls of a window that are cut off — below its edge or the
/// screen's, or hidden by what clips them — which no scroll brings into view.
/// Scrolls with the wheel, a notch at a time, at points down the window, as
/// an operator would look for them.
fn unreachable(
    ctx: &egui::Context,
    app: &mut SdroxideApp,
    frame: &mut eframe::Frame,
    screen: egui::Rect,
    t0: &mut f64,
    win_id: egui::Id,
) -> Vec<String> {
    let layer = egui::LayerId::new(egui::Order::Middle, win_id);
    let Some(win) = ctx.memory(|m| m.area_rect(win_id)) else { return Vec::new() };
    let seen = win.intersect(screen);
    let in_view = |n: &egui::WidgetRect| {
        n.rect.bottom() <= seen.bottom() + 1.0
            && n.rect.top() >= seen.top() - 1.0
            && n.interact_rect.height() >= n.rect.height() * 0.5
    };
    let mut pending: Vec<egui::WidgetRect> =
        widgets(ctx, layer).into_iter().filter(|w| w.rect.height() > 0.0 && !in_view(w)).collect();
    if pending.is_empty() {
        return Vec::new();
    }
    // Steps under 8 points each: egui takes those whole, in the frame they
    // arrive, where a larger step is spread over the frames after it.
    let wheel = |at: egui::Pos2, dy: f32| {
        let n = (dy.abs() / 7.5).ceil().max(1.0) as usize;
        let mut ev = vec![egui::Event::PointerMoved(at)];
        ev.extend((0..n).map(|_| egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, dy / n as f32),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        }));
        ev
    };
    let xs = [seen.left() + 0.25 * seen.width(), seen.left() + 0.75 * seen.width()];
    let mut y = seen.top() + 30.0;
    while y < seen.bottom() - 8.0 && !pending.is_empty() {
        for x in xs {
            let at = egui::pos2(x, y);
            let mut last: Vec<egui::Rect> = Vec::new();
            for _ in 0..120 {
                run(ctx, app, frame, screen, t0, 2, |i| {
                    if i == 0 { wheel(at, -40.0) } else { Vec::new() }
                });
                let now: Vec<Option<egui::WidgetRect>> = ctx.viewport(|v| {
                    pending.iter().map(|w| v.this_pass.widgets.get(w.id).copied()).collect()
                });
                let rects: Vec<egui::Rect> =
                    now.iter().map(|n| n.map_or(egui::Rect::NOTHING, |n| n.rect)).collect();
                let reached: Vec<bool> =
                    now.iter().map(|n| n.is_some_and(|n| in_view(&n))).collect();
                let moved = rects != last;
                last = rects;
                let mut k = 0;
                pending.retain(|_| {
                    k += 1;
                    !reached[k - 1]
                });
                last =
                    last.into_iter().zip(&reached).filter(|(_, r)| !**r).map(|(x, _)| x).collect();
                if pending.is_empty() || !moved {
                    break;
                }
            }
            // Back to the top for the next place.
            run(ctx, app, frame, screen, t0, 3, |_| wheel(at, 3_000.0));
        }
        y += 40.0;
    }
    run(ctx, app, frame, screen, t0, 2, |_| vec![egui::Event::PointerGone]);
    pending
        .iter()
        .map(|w| {
            format!(
                "a control at x {:.0}..{:.0}, y {:.0}..{:.0} is cut off (window shows y {:.0}..{:.0}) and no scroll brings it into view",
                w.rect.left(),
                w.rect.right(),
                w.rect.top(),
                w.rect.bottom(),
                seen.top(),
                seen.bottom()
            )
        })
        .collect()
}

/// Drag the window's lower right corner up and to the left, as far as the
/// operator can, to its smallest.
fn shrink(
    ctx: &egui::Context,
    app: &mut SdroxideApp,
    frame: &mut eframe::Frame,
    screen: egui::Rect,
    t0: &mut f64,
    win_id: egui::Id,
) {
    let Some(win) = ctx.memory(|m| m.area_rect(win_id)) else { return };
    let from = win.right_bottom() - egui::vec2(2.0, 2.0);
    let to = win.left_top() + egui::vec2(60.0, 60.0);
    let steps = 6;
    run(ctx, app, frame, screen, t0, steps + 6, |i| {
        let at = |k: usize| from + (to - from) * (k as f32 / steps as f32);
        let press = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        match i {
            0 => vec![egui::Event::PointerMoved(from)],
            1 => vec![press(from, true)],
            i if i <= steps + 1 => vec![egui::Event::PointerMoved(at(i - 1))],
            i if i == steps + 2 => vec![press(to, false)],
            _ => vec![egui::Event::PointerGone],
        }
    });
}

/// A window to open, and how it is filled.
#[derive(Clone, Copy, Debug)]
enum Fill {
    Playback,
    WxsatApt,
    WxsatLrpt,
    Sonde,
    SondeCardWindow,
}

fn long_name(i: usize) -> String {
    let modes = ["SSTV", "RTTY", "FT8", "CW", "USB", "PSK", "OLIVIA", "NAVTEX"];
    format!(
        "sdroxide_radio{}_2026-10-{:02}_1{}-00-05Z_{}.230000MHz_{}.mp3",
        i % 3,
        1 + i % 28,
        i % 10,
        14 + i,
        modes[i % modes.len()]
    )
}

fn fill(app: &mut SdroxideApp, what: Fill) {
    let now = crate::time::now_unix();
    match what {
        Fill::Playback => {
            let name = long_name(0);
            app.state.playback = Some(sdroxide_types::PlaybackStatus {
                name: name.clone(),
                position_s: 42.0,
                duration_s: Some(4017.0),
                paused: true,
                looping: true,
                ended: false,
                rate_hz: 48_000,
                channels: 2,
                stamp: sdroxide_types::playback::parse_recording_name(&name),
                fault: Some("the file ended in the middle of a frame (truncated MP3)".into()),
            });
            let mut files: Vec<sdroxide_types::RecordingFile> = (0..14)
                .map(|i| sdroxide_types::RecordingFile {
                    name: long_name(i),
                    bytes: 1_234_567 * (i as u64 + 1),
                    modified_unix: now - 3600 * i as i64,
                    iq: false,
                })
                .collect();
            files.push(sdroxide_types::RecordingFile {
                name: "an operator's own file with a very long name and no stamp in it at all.wav"
                    .into(),
                bytes: 99_999_999,
                modified_unix: now,
                iq: false,
            });
            files.push(sdroxide_types::RecordingFile {
                name: "sdroxide_iq_2026-10-06_10-00-05Z_14.230000MHz.wav".into(),
                bytes: 4_000_000_000,
                modified_unix: now,
                iq: true,
            });
            app.playback.accept(sdroxide_types::RecordingListing {
                dir: "/home/operator/Documents/a rather long folder name/sdroxide/recordings"
                    .into(),
                files,
                error: None,
            });
            app.playback.show = true;
        }
        Fill::WxsatApt | Fill::WxsatLrpt => {
            let lrpt = matches!(what, Fill::WxsatLrpt);
            let kind =
                if lrpt { sdroxide_types::WxsatKind::Lrpt } else { sdroxide_types::WxsatKind::Apt };
            app.state.wxsat = sdroxide_types::WxsatSettings {
                enabled: true,
                frequency_hz: if lrpt { 137_900_000.0 } else { 137_100_000.0 },
                kind,
            };
            // Far from 137 MHz, so the "outside the window" line is drawn too.
            app.state.center_hz = 14_200_000.0;
            app.state.sample_rate = 2_400_000.0;
            let mut st = sdroxide_types::WxsatStatus {
                running: true,
                level_dbfs: -42.0,
                locked: true,
                quality: 0.8,
                pass: 1,
                total_lines: 900,
                first_line: 0,
                kind,
                frames_ok: 1234,
                ..Default::default()
            };
            if lrpt {
                let px = sdroxide_types::WXSAT_LRPT_WIDTH * sdroxide_types::WXSAT_LRPT_ROW_LINES;
                for apid in 64..=69u16 {
                    for row in 0..60u32 {
                        st.rows.push(sdroxide_types::WxsatRow {
                            apid,
                            row,
                            lines: vec![(row * 3 + apid as u32) as u8; px],
                        });
                    }
                }
            } else {
                st.new_lines = (0..900)
                    .map(|i| vec![(i % 251) as u8; sdroxide_types::WXSAT_LINE_PIXELS])
                    .collect();
            }
            app.wxsat.accept(Box::new(st));
            app.wxsat.show = true;
        }
        Fill::Sonde | Fill::SondeCardWindow => {
            app.state.sonde = sdroxide_types::SondeSettings {
                enabled: true,
                frequency_hz: 403_000_000.0,
                scan: true,
            };
            app.state.center_hz = 145_000_000.0;
            app.state.sample_rate = 2_400_000.0;
            let models = ["RS41", "DFM", "M10", "M20", "iMet-54", "Meisei", "C34/C50"];
            let serials = [
                "W1234567",
                "DFM-17-123456789",
                "M10-0123456789-ABCDEF",
                "M20-123-4-56789",
                "IMET54-12345678",
                "MEISEI-IMS100-1234567",
                "C50-0123456789012345",
            ];
            let sondes: Vec<sdroxide_types::SondeTrack> = (0..7)
                .map(|i| {
                    let lat = 51.5 + i as f64 * 0.12;
                    let lon = 13.2 + i as f64 * 0.2;
                    sdroxide_types::SondeTrack {
                        model: models[i].into(),
                        serial: serials[i].into(),
                        freq_hz: 402_000_000.0 + i as f64 * 500_000.0,
                        frame: 1000 + i as u32,
                        lat: Some(lat),
                        lon: Some(lon),
                        alt_m: Some(12_345.0 + i as f32 * 1000.0),
                        speed_ms: Some(23.4),
                        course_deg: Some(87.0),
                        climb_ms: Some(if i % 2 == 0 { 5.2 } else { -12.7 }),
                        num_sv: Some(9),
                        battery_v: Some(2.9),
                        temp_c: Some(-54.3),
                        humidity_pct: Some(12.0),
                        pressure_hpa: Some(187.0),
                        last_unix: now - 3 - 40 * i as i64,
                        frames: 2345,
                        track: (0..200)
                            .map(|k| sdroxide_types::SondePoint {
                                lat: lat - k as f64 * 0.001,
                                lon: lon - k as f64 * 0.002,
                                alt_m: 10.0 * k as f32,
                                unix: now - 200 + k,
                                temp_c: None,
                                humidity_pct: None,
                                pressure_hpa: None,
                            })
                            .collect(),
                    }
                })
                .collect();
            app.sonde.accept(Box::new(sdroxide_types::SondeStatus {
                running: true,
                level_dbfs: -61.0,
                frames: 123_456,
                sondes,
            }));
            let hub: Vec<_> = (0..12)
                .map(|i| crate::sonde_online::OnlineSonde {
                    serial: format!("V{:07}", 4_400_000 + i),
                    model: "RS41".into(),
                    lat: 51.0 + i as f64 * 0.15,
                    lon: 12.5 + i as f64 * 0.17,
                    alt_m: Some(8000.0 + 400.0 * i as f64),
                    climb_ms: Some(5.0),
                    speed_ms: Some(12.0),
                    freq_mhz: Some(403.0),
                    unix: now - 20,
                    manufacturer: Some("Vaisala".into()),
                    subtype: Some("RS41-SGP".into()),
                    heading_deg: Some(270.0),
                    temp_c: Some(-30.0),
                    humidity_pct: Some(40.0),
                    pressure_hpa: Some(350.0),
                    battery_v: Some(2.8),
                    sats: Some(10),
                    snr_db: Some(12.5),
                    station: Some("DL0ABC-a-rather-long-station-name".into()),
                    station_pos: Some((51.1, 12.9)),
                    antenna: Some("1/4 wave monopole on the garage roof, 6 m".into()),
                    software: Some("radiosonde_auto_rx-1.8.2-beta5".into()),
                    received_unix: Some(now - 19),
                })
                .collect();
            // The selected one is the radio's and SondeHub's both: the card
            // with the most rows.
            let mut both = hub[0].clone();
            both.serial = "M10-0123456789-ABCDEF".into();
            let mut hub = hub;
            hub.push(both);
            app.sonde.seed(
                hub,
                Some("M10-0123456789-ABCDEF"),
                (0..50).map(|k| (51.6 - k as f64 * 0.01, 13.6 - k as f64 * 0.01)).collect(),
                matches!(what, Fill::SondeCardWindow),
            );
            app.sonde.show = true;
        }
    }
}

fn window_id(ctx: &egui::Context, what: Fill) -> egui::Id {
    crate::layout::salted_id(
        ctx,
        match what {
            Fill::Playback => "PLAYBACK",
            Fill::WxsatApt | Fill::WxsatLrpt => "WXSAT",
            Fill::Sonde | Fill::SondeCardWindow => "SONDE",
        },
    )
}

/// Open the window filled, on every screen and in both languages, at its
/// opening size and shrunk to its smallest, and collect what does not fit.
fn fits(what: Fill) -> Vec<String> {
    let mut bad = Vec::new();
    for (w, h, layout) in SCREENS {
        for tag in LANGUAGES {
            for shrunk in [false, true] {
                let ctx = egui::Context::default();
                let mut app = new_app(&ctx, layout, tag);
                fill(&mut app, what);
                let mut frame = eframe::Frame::_new_kittest();
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h));
                let mut t0 = 0.0;
                let mut shapes =
                    run(&ctx, &mut app, &mut frame, screen, &mut t0, 30, |_| Vec::new());
                let win_id = window_id(&ctx, what);
                if shrunk {
                    shrink(&ctx, &mut app, &mut frame, screen, &mut t0, win_id);
                    shapes = run(&ctx, &mut app, &mut frame, screen, &mut t0, 15, |_| Vec::new());
                }
                let size =
                    ctx.memory(|m| m.area_rect(win_id)).map_or(egui::Vec2::ZERO, |r| r.size());
                let tag_line = format!(
                    "{what:?} {w:.0}x{h:.0} {layout:?} {tag}{} (window {:.0}x{:.0})",
                    if shrunk { " shrunk" } else { "" },
                    size.x,
                    size.y
                );
                let mut exempt: Vec<egui::Rect> = Vec::new();
                exempt.extend(app.sonde.map_bounds());
                exempt.extend(app.sonde.table_view());
                let mut found = check_window(&ctx, &shapes, win_id, screen, &exempt);
                if matches!(what, Fill::Sonde | Fill::SondeCardWindow) {
                    found.extend(check_card(&ctx, &shapes, &app, screen, what));
                }
                found.extend(unreachable(&ctx, &mut app, &mut frame, screen, &mut t0, win_id));
                bad.extend(found.into_iter().map(|f| format!("{tag_line}: {f}")));
            }
        }
    }
    bad
}

/// The sonde's card: pinned, it stays on the map; as a window, on the screen
/// with nothing cut off at its side.
fn check_card(
    ctx: &egui::Context,
    shapes: &[egui::epaint::ClippedShape],
    app: &SdroxideApp,
    screen: egui::Rect,
    what: Fill,
) -> Vec<String> {
    let mut bad = Vec::new();
    if matches!(what, Fill::SondeCardWindow) {
        let id = crate::layout::salted_id(ctx, "sonde-card-window");
        bad.extend(
            check_window(ctx, shapes, id, screen, &[])
                .into_iter()
                .map(|b| format!("card window: {b}")),
        );
        return bad;
    }
    let id = crate::layout::salted_id(ctx, "sonde-pinned-card");
    let (Some(card), Some(map)) = (ctx.memory(|m| m.area_rect(id)), app.sonde.map_bounds()) else {
        bad.push("pinned card or map not drawn".into());
        return bad;
    };
    let room = map.intersect(screen);
    if !room.expand(1.0).contains_rect(card) {
        bad.push(format!(
            "pinned card {:.0}x{:.0} at {:.0},{:.0} leaves the visible map {:.0},{:.0}..{:.0},{:.0}",
            card.width(),
            card.height(),
            card.left(),
            card.top(),
            room.left(),
            room.top(),
            room.right(),
            room.bottom()
        ));
    }
    // What is in it scrolls, both ways, inside that: a card with SondeHub's
    // rows is taller and wider than a small window's map.
    bad
}

#[test]
#[ignore = "draws the whole app, which changes process-wide settings"]
fn the_playback_window_fits() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let bad = fits(Fill::Playback);
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
#[ignore = "draws the whole app, which changes process-wide settings"]
fn the_weather_satellite_window_fits() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let mut bad = fits(Fill::WxsatApt);
    bad.extend(fits(Fill::WxsatLrpt));
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
#[ignore = "draws the whole app, which changes process-wide settings"]
fn the_sonde_window_fits() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let mut bad = fits(Fill::Sonde);
    bad.extend(fits(Fill::SondeCardWindow));
    assert!(bad.is_empty(), "{bad:#?}");
}

/// Whether `want` is drawn somewhere, whole: inside `within` and not cut by
/// its clip.
fn drawn_whole(
    shapes: &[egui::epaint::ClippedShape],
    want: &str,
    within: egui::Rect,
) -> Result<(), String> {
    let all = texts(shapes);
    let hits: Vec<_> = all.iter().filter(|(t, _, _)| t.trim() == want.trim()).collect();
    if hits.is_empty() {
        return Err(format!("{want:?} not drawn"));
    }
    for (_, r, clip) in &hits {
        if within.expand(1.0).contains_rect(*r) && clip.expand(1.0).contains_rect(*r) {
            return Ok(());
        }
    }
    let (_, r, clip) = hits[0];
    Err(format!(
        "{want:?} at {:.0},{:.0}..{:.0},{:.0}, clipped to {:.0},{:.0}..{:.0},{:.0}, outside {:.0},{:.0}..{:.0},{:.0}",
        r.left(),
        r.top(),
        r.right(),
        r.bottom(),
        clip.left(),
        clip.top(),
        clip.right(),
        clip.bottom(),
        within.left(),
        within.top(),
        within.right(),
        within.bottom()
    ))
}

/// The ways in: the RADIOSONDES chip in the ISM window's header, the WEATHER
/// SATELLITES chip in the SATELLITE window's tab bar and the PICTURE chip on
/// an APT link, the transport strip above the waterfall while a file plays,
/// and ▶ RECORDINGS… in the REC popup — each drawn whole, on the screen.
#[test]
#[ignore = "draws the whole app, which changes process-wide settings"]
fn the_ways_into_the_new_windows_fit() {
    use crate::i18n::tr;
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let mut bad = Vec::new();
    let screens = [
        (1024.0f32, 600.0f32, LayoutMode::Auto),
        (1024.0, 768.0, LayoutMode::Desktop),
        (1366.0, 768.0, LayoutMode::Auto),
        (1366.0, 768.0, LayoutMode::Desktop),
        (1920.0, 1080.0, LayoutMode::Auto),
        (390.0, 844.0, LayoutMode::Phone),
    ];
    for (w, h, layout) in screens {
        for tag in LANGUAGES {
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h));
            let at = format!("{w:.0}x{h:.0} {layout:?} {tag}");

            // ISM and SATELLITE side by side, and a file playing.
            let ctx = egui::Context::default();
            let mut app = new_app(&ctx, layout, tag);
            app.show_ism = true;
            app.show_sat = true;
            app.sat_cfg = std::sync::Arc::new(sdroxide_types::SatConfig {
                tles: vec![sdroxide_types::CustomTle {
                    name: "NOAA 19".into(),
                    line1: "1 33591U 09005A   26031.51268519  .00000271  00000-0  16472-3 0  9997"
                        .into(),
                    line2: "2 33591  99.0361 121.3384 0013431 262.5195  97.4595 14.13096410877263"
                        .into(),
                    enabled: true,
                }],
                ..Default::default()
            });
            app.sat_win.selected = Some(33591);
            fill(&mut app, Fill::Playback);
            app.playback.show = false;
            let mut frame = eframe::Frame::_new_kittest();
            let mut t0 = 0.0;
            let shapes = run(&ctx, &mut app, &mut frame, screen, &mut t0, 30, |_| Vec::new());
            for (id, wants) in [
                ("Ism", vec![tr!("ism-sonde-chip")]),
                ("SATELLITE", vec![tr!("sat-wxsat"), tr!("sat-picture")]),
            ] {
                let win_id = crate::layout::salted_id(&ctx, id);
                let Some(win) = ctx.memory(|m| m.area_rect(win_id)) else {
                    bad.push(format!("{at}: {id} window not drawn"));
                    continue;
                };
                for want in wants {
                    if let Err(e) = drawn_whole(&shapes, &want, win.intersect(screen)) {
                        bad.push(format!("{at}: {id}: {e}"));
                    }
                }
                // The SATELLITE window's link table is wider than a phone
                // with or without the chip (its four columns keep a 70-point
                // floor each); that is the table's, not the chip's.
                if id == "SATELLITE" && layout == LayoutMode::Phone {
                    continue;
                }
                bad.extend(
                    check_window(&ctx, &shapes, win_id, screen, &[])
                        .into_iter()
                        .map(|e| format!("{at}: {id}: {e}")),
                );
            }
            for want in [
                tr!("playback-strip-label"),
                tr!("playback-not-live"),
                tr!("playback-play"),
                tr!("playback-again"),
                tr!("playback-loop"),
                tr!("playback-stop"),
                tr!("playback-files"),
            ] {
                if let Err(e) = drawn_whole(&shapes, &want, screen) {
                    bad.push(format!("{at}: strip: {e}"));
                }
            }

            // The REC popup, opened with a click on REC where the strip has it.
            if layout != LayoutMode::Desktop {
                continue;
            }
            let ctx = egui::Context::default();
            let mut app = new_app(&ctx, layout, tag);
            let mut frame = eframe::Frame::_new_kittest();
            let mut t0 = 0.0;
            let shapes = run(&ctx, &mut app, &mut frame, screen, &mut t0, 20, |_| Vec::new());
            let Some((_, rec, _)) = texts(&shapes).into_iter().find(|(t, _, _)| t == "REC") else {
                bad.push(format!("{at}: no REC chip"));
                continue;
            };
            let c = rec.center();
            let press = |pressed| egui::Event::PointerButton {
                pos: c,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            let shapes = run(&ctx, &mut app, &mut frame, screen, &mut t0, 6, |i| match i {
                0 => vec![egui::Event::PointerMoved(c)],
                1 => vec![press(true)],
                2 => vec![press(false)],
                _ => Vec::new(),
            });
            if let Err(e) = drawn_whole(&shapes, &tr!("topbar-recordings-chip"), screen) {
                bad.push(format!("{at}: REC popup: {e}"));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}
