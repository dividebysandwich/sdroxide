//! How the station is doing, over time: messages a minute, aircraft heard and
//! placed, the farthest position — and, when the frames come from `readsb` on
//! another machine, that machine's signal, noise, strong-signal share,
//! temperature and WiFi level.
//!
//! The log's PLOTS answer *what* was heard. These answer *how well*: a range
//! that sinks at dusk is traffic, a range that sinks while the noise climbs is
//! the antenna or the amplifier, and a receiver box on a balcony that runs hot
//! or loses its supply shows it here before it shows in the table.
// Why: HDZ 2026-11-26: „wollen wir neben plots noch eine "graphs" seite
//      dazustellen?“ — after an evening of gain series on a Raspberry Pi on the
//      balcony, where every comparison had to be measured by hand in two-minute
//      windows; „die temperatur hätte ich da gerne dran. der rechner bleibt
//      draussen“.
//
//! Everything local is sampled from the status the engine already sends, so the
//! graphs work with this receiver's own decoder as well as with a Beast feed.
//! The remote figures come from two files over HTTP on the feed's host:
//! `tar1090/data/stats.json`, which every `readsb` writes and `tar1090` serves,
//! and `station/station.json`, an optional file a station can publish with its
//! temperature, supply flags and WiFi level. Either may be missing; its curves
//! are then simply not drawn.

use std::collections::VecDeque;

use eframe::egui::{self, Align2, Color32, FontId, Sense, Stroke};
use sdroxide_types::{AdsbFeed, AdsbStatus};

/// One sample every ten seconds: `readsb` rewrites its files at that pace too,
/// and a day of it is 8,640 points per curve — nothing to hold in memory.
pub const STEP_S: i64 = 10;
/// How much history is kept: the longest window offered.
pub const KEEP_S: i64 = 24 * 3600;
/// A remote reading older than this is not put into a sample: a host that
/// stopped answering shows as a gap, not as a flat line of its last value.
const REMOTE_FRESH_S: i64 = 3 * STEP_S;

/// The windows offered over the graphs, seconds.
pub const SPANS: [i64; 4] = [15 * 60, 3600, 6 * 3600, 24 * 3600];

pub fn span_label(s: i64) -> String {
    if s < 3600 { format!("{} min", s / 60) } else { format!("{} h", s / 3600) }
}

/// One moment of the station. `NAN` where the figure was not available.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub t: i64,
    /// Accepted frames a minute, from the decoder's own count.
    pub msgs_min: f32,
    /// Aircraft heard in the last minute.
    pub aircraft: f32,
    /// Of those, aircraft whose position moved in the last minute.
    pub placed: f32,
    /// Farthest of those positions from home, km. `NAN` without a home.
    pub max_km: f32,
    /// From the feed's `readsb`: mean signal and noise, dBFS, last minute.
    pub signal: f32,
    pub noise: f32,
    /// Messages above −3 dBFS, percent of those accepted: the overload gauge.
    pub strong_pct: f32,
    /// From the feed's `station.json`.
    pub temp_c: f32,
    pub wifi_dbm: f32,
    /// Supply or temperature limit active right now (low voltage, frequency
    /// capped, throttled, soft temperature limit).
    pub limited: bool,
}

/// What the feed's host last said about itself.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Remote {
    pub at: i64,
    pub signal: Option<f32>,
    pub noise: Option<f32>,
    pub strong_pct: Option<f32>,
    pub temp_c: Option<f32>,
    pub wifi_dbm: Option<f32>,
    pub limited: bool,
}

#[derive(Default)]
pub struct Graphs {
    pub samples: VecDeque<Sample>,
    /// Time and frame count of the last sample, for the rate.
    last: Option<(i64, u64)>,
    #[cfg(not(target_arch = "wasm32"))]
    remote: std::sync::Arc<std::sync::Mutex<Option<Remote>>>,
    #[cfg(not(target_arch = "wasm32"))]
    busy: std::sync::Arc<std::sync::atomic::AtomicBool>,
    #[cfg(not(target_arch = "wasm32"))]
    asked_at: i64,
}

impl Graphs {
    /// Called with every status the engine sends; takes a sample when ten
    /// seconds have passed, and asks the feed's host for its figures.
    pub fn note(
        &mut self,
        st: &AdsbStatus,
        home: Option<(f64, f64)>,
        feed_host: Option<String>,
        now: i64,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        if st.feed == AdsbFeed::Beast
            && let Some(host) = feed_host
        {
            self.ask(host, now);
        }
        #[cfg(target_arch = "wasm32")]
        let _ = feed_host;
        let Some((t0, f0)) = self.last else {
            self.last = Some((now, st.frames));
            return;
        };
        if now - t0 < STEP_S {
            return;
        }
        // A restarted decoder starts counting again from zero.
        let df = st.frames.saturating_sub(f0);
        let dt = (now - t0) as f32;
        self.last = Some((now, st.frames));
        let heard: Vec<_> = st.aircraft.iter().filter(|a| now - a.last_at <= 60).collect();
        let placed: Vec<(f64, f64)> = heard
            .iter()
            .filter(|a| a.last_pos_at > 0 && now - a.last_pos_at <= 60)
            .filter_map(|a| Some((a.lat?, a.lon?)))
            .collect();
        let max_km = home
            .map(|h| {
                placed
                    .iter()
                    .map(|p| sdroxide_types::distance_km(h, *p) as f32)
                    .fold(f32::NAN, f32::max)
            })
            .unwrap_or(f32::NAN);
        let r = self.remote_now(now);
        let v = |o: Option<f32>| o.unwrap_or(f32::NAN);
        self.push(Sample {
            t: now,
            msgs_min: df as f32 * 60.0 / dt,
            aircraft: heard.len() as f32,
            placed: placed.len() as f32,
            max_km,
            signal: v(r.and_then(|r| r.signal)),
            noise: v(r.and_then(|r| r.noise)),
            strong_pct: v(r.and_then(|r| r.strong_pct)),
            temp_c: v(r.and_then(|r| r.temp_c)),
            wifi_dbm: v(r.and_then(|r| r.wifi_dbm)),
            limited: r.is_some_and(|r| r.limited),
        });
    }

    fn push(&mut self, s: Sample) {
        self.samples.push_back(s);
        while self.samples.front().is_some_and(|f| s.t - f.t > KEEP_S) {
            self.samples.pop_front();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn remote_now(&self, now: i64) -> Option<Remote> {
        self.remote.lock().ok().and_then(|g| *g).filter(|r| now - r.at <= REMOTE_FRESH_S)
    }
    #[cfg(target_arch = "wasm32")]
    fn remote_now(&self, _now: i64) -> Option<Remote> {
        None
    }

    /// One request pair at a time, at most every ten seconds, off the UI
    /// thread: a host that has gone away costs a timeout in the background,
    /// never a stalled frame.
    #[cfg(not(target_arch = "wasm32"))]
    fn ask(&mut self, host: String, now: i64) {
        use std::sync::atomic::Ordering;
        if now - self.asked_at < STEP_S || self.busy.swap(true, Ordering::AcqRel) {
            return;
        }
        self.asked_at = now;
        let slot = self.remote.clone();
        let busy = self.busy.clone();
        std::thread::spawn(move || {
            let r = fetch(&host, now);
            if let Ok(mut g) = slot.lock() {
                *g = r;
            }
            busy.store(false, Ordering::Release);
        });
    }

    /// Whether any remote figure arrived in the window — decides whether the
    /// station's graphs are drawn at all.
    pub fn has_remote(&self, since: i64) -> bool {
        self.samples.iter().any(|s| s.t >= since && (!s.signal.is_nan() || !s.temp_c.is_nan()))
    }
}

/// The host part of a `host:port` Beast address, bracketed IPv6 included.
pub fn host_of(addr: &str) -> Option<String> {
    let a = addr.trim();
    let h = match a.rsplit_once(':') {
        Some((h, p)) if p.parse::<u16>().is_ok() && !h.ends_with(':') => h,
        _ => a,
    };
    (!h.is_empty()).then(|| h.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch(host: &str, now: i64) -> Option<Remote> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(3)))
        .timeout_global(Some(std::time::Duration::from_secs(5)))
        .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let get = |path: &str| -> Option<serde_json::Value> {
        let body = agent
            .get(&format!("http://{host}/{path}"))
            .call()
            .ok()?
            .body_mut()
            .read_to_string()
            .ok()?;
        serde_json::from_str(&body).ok()
    };
    let stats = get("tar1090/data/stats.json");
    let station = get("station/station.json");
    if stats.is_none() && station.is_none() {
        return None;
    }
    let mut r = Remote { at: now, ..Remote::default() };
    if let Some(s) = stats {
        read_stats(&s, &mut r);
    }
    if let Some(s) = station {
        read_station(&s, &mut r);
    }
    Some(r)
}

/// `readsb`'s last-minute block: mean signal and noise, and the share of
/// strong messages among those accepted.
pub fn read_stats(v: &serde_json::Value, r: &mut Remote) {
    let m = &v["last1min"];
    let l = &m["local"];
    let f = |x: &serde_json::Value| x.as_f64().map(|x| x as f32);
    r.signal = f(&l["signal"]);
    r.noise = f(&l["noise"]);
    let accepted: f64 =
        l["accepted"].as_array().map(|a| a.iter().filter_map(|x| x.as_f64()).sum()).unwrap_or(0.0);
    if accepted > 0.0
        && let Some(s) = l["strong_signals"].as_f64()
    {
        r.strong_pct = Some((s / accepted * 100.0) as f32);
    }
}

/// The station's own file: temperature, WiFi, and the supply flags as
/// `vcgencmd get_throttled` prints them.
pub fn read_station(v: &serde_json::Value, r: &mut Remote) {
    let f = |x: &serde_json::Value| x.as_f64().map(|x| x as f32);
    r.temp_c = f(&v["cpu_temp_c"]);
    r.wifi_dbm = f(&v["wifi_dbm"]);
    // Bits 0–3 are "now": under-voltage, frequency capped, throttled, soft
    // temperature limit. The high bits only say it happened since boot.
    r.limited = v["throttled"]
        .as_str()
        .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .is_some_and(|b| b & 0xF != 0);
}

/// One curve: a label, its colour and how to read it from a sample.
pub struct Curve {
    pub label: String,
    pub color: Color32,
    pub get: fn(&Sample) -> f32,
}

/// One panel: a title, its unit, and one or two curves sharing an axis.
pub struct Panel {
    pub title: String,
    pub unit: &'static str,
    pub curves: Vec<Curve>,
    /// Decimals of the value shown beside the title.
    pub decimals: usize,
}

/// Draws the panels stacked, the last `span` seconds up to `now`, in `size`.
pub fn draw(
    ui: &mut egui::Ui,
    g: &Graphs,
    panels: &[Panel],
    span: i64,
    now: i64,
    size: egui::Vec2,
    ink: Color32,
) {
    let (r, resp) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter_at(r);
    let font = FontId::monospace(9.5);
    let since = now - span;
    let pts: Vec<&Sample> = g.samples.iter().filter(|s| s.t >= since).collect();
    if panels.is_empty() {
        return;
    }
    let gap = 6.0;
    let ph = ((r.height() - gap * (panels.len() as f32 - 1.0)) / panels.len() as f32).max(30.0);
    let x_of = |t: i64| r.left() + 44.0 + (r.width() - 50.0) * ((t - since) as f32 / span as f32);
    let pointer = resp.hover_pos();
    // The sample under the pointer, the same for every panel.
    let at = pointer.and_then(|p| {
        pts.iter()
            .min_by(|a, b| (x_of(a.t) - p.x).abs().total_cmp(&(x_of(b.t) - p.x).abs()))
            .copied()
    });
    for (i, p) in panels.iter().enumerate() {
        let top = r.top() + i as f32 * (ph + gap);
        let plot = egui::Rect::from_min_max(
            egui::pos2(r.left() + 44.0, top + 12.0),
            egui::pos2(r.right() - 6.0, top + ph - 2.0),
        );
        // The axis: from the data, a little padded, never flat.
        let vals: Vec<f32> = pts
            .iter()
            .flat_map(|s| p.curves.iter().map(move |c| (c.get)(s)))
            .filter(|v| v.is_finite())
            .collect();
        let (mut lo, mut hi) = vals
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
        if !lo.is_finite() {
            (lo, hi) = (0.0, 1.0);
        }
        if hi - lo < 1e-3 {
            hi += 1.0;
            lo -= 1.0;
        }
        let pad = (hi - lo) * 0.08;
        let (lo, hi) = (lo - pad, hi + pad);
        let y_of = |v: f32| plot.bottom() - (v - lo) / (hi - lo) * plot.height();
        painter.rect_stroke(
            plot,
            0.0,
            Stroke::new(1.0, ink.gamma_multiply(0.25)),
            egui::StrokeKind::Inside,
        );
        painter.text(
            egui::pos2(plot.left() - 4.0, plot.top()),
            Align2::RIGHT_TOP,
            format!("{hi:.0}"),
            font.clone(),
            ink.gamma_multiply(0.7),
        );
        painter.text(
            egui::pos2(plot.left() - 4.0, plot.bottom()),
            Align2::RIGHT_BOTTOM,
            format!("{lo:.0}"),
            font.clone(),
            ink.gamma_multiply(0.7),
        );
        // Title, with the latest value of each curve.
        let mut head = p.title.clone();
        for c in &p.curves {
            if let Some(v) = pts.iter().rev().map(|s| (c.get)(s)).find(|v| v.is_finite()) {
                head.push_str(&format!("   {} {:.*} {}", c.label, p.decimals, v, p.unit));
            }
        }
        painter.text(egui::pos2(plot.left(), top), Align2::LEFT_TOP, head, font.clone(), ink);
        for c in &p.curves {
            // Broken where a value is missing or samples are more than two
            // steps apart: a gap is information, a bridge across it is not.
            let mut run: Vec<egui::Pos2> = Vec::new();
            let mut prev_t = i64::MIN;
            for s in &pts {
                let v = (c.get)(s);
                if !v.is_finite() || s.t - prev_t > 2 * STEP_S {
                    if run.len() > 1 {
                        painter.add(egui::Shape::line(
                            std::mem::take(&mut run),
                            Stroke::new(1.3, c.color),
                        ));
                    }
                    run.clear();
                }
                if v.is_finite() {
                    run.push(egui::pos2(x_of(s.t), y_of(v)));
                }
                prev_t = s.t;
            }
            if run.len() > 1 {
                painter.add(egui::Shape::line(run, Stroke::new(1.3, c.color)));
            } else if let Some(q) = run.first() {
                painter.circle_filled(*q, 1.5, c.color);
            }
        }
        // Supply or temperature limit: a red tick under the panel where it held.
        if p.unit == "°C" {
            for s in pts.iter().filter(|s| s.limited) {
                let x = x_of(s.t);
                painter.line_segment(
                    [egui::pos2(x, plot.bottom() - 4.0), egui::pos2(x, plot.bottom())],
                    Stroke::new(2.0, Color32::from_rgb(230, 70, 60)),
                );
            }
        }
        if let Some(s) = at {
            let x = x_of(s.t);
            painter.line_segment(
                [egui::pos2(x, plot.top()), egui::pos2(x, plot.bottom())],
                Stroke::new(1.0, ink.gamma_multiply(0.4)),
            );
            for c in &p.curves {
                let v = (c.get)(s);
                if v.is_finite() {
                    painter.circle_filled(egui::pos2(x, y_of(v)), 2.5, c.color);
                }
            }
        }
    }
    // Time along the bottom: start, middle, now.
    let fmt = |t: i64| {
        let l = t + crate::time::local_offset_at(t);
        format!("{:02}:{:02}", (l.rem_euclid(86_400)) / 3600, (l.rem_euclid(3600)) / 60)
    };
    for t in [since, since + span / 2, now] {
        painter.text(
            egui::pos2(x_of(t).clamp(r.left() + 60.0, r.right() - 20.0), r.bottom()),
            Align2::CENTER_BOTTOM,
            fmt(t),
            font.clone(),
            ink.gamma_multiply(0.7),
        );
    }
    // The values under the pointer, beside it.
    if let (Some(s), Some(p)) = (at, pointer) {
        let mut lines = vec![fmt(s.t)];
        for pn in panels {
            for c in &pn.curves {
                let v = (c.get)(s);
                if v.is_finite() {
                    lines.push(format!("{} {:.*} {}", c.label, pn.decimals, v, pn.unit));
                }
            }
        }
        let galley = painter.layout_no_wrap(lines.join("\n"), font.clone(), ink);
        let at = egui::pos2(
            (p.x + 12.0).min(r.right() - galley.size().x - 8.0),
            (p.y - galley.size().y - 10.0).max(r.top()),
        );
        let frame = egui::Rect::from_min_size(at, galley.size()).expand(4.0);
        painter.rect_filled(frame, 2.0, Color32::from_black_alpha(220));
        painter.galley(at, galley, ink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(frames: u64) -> AdsbStatus {
        AdsbStatus { frames, ..AdsbStatus::default() }
    }

    #[test]
    fn a_sample_every_ten_seconds_with_the_rate_per_minute() {
        let mut g = Graphs::default();
        g.note(&status(0), None, None, 1000);
        g.note(&status(50), None, None, 1005);
        assert!(g.samples.is_empty(), "five seconds is not a step");
        g.note(&status(100), None, None, 1010);
        assert_eq!(g.samples.len(), 1);
        assert_eq!(g.samples[0].msgs_min, 600.0, "100 frames in 10 s");
        assert!(g.samples[0].max_km.is_nan(), "no home, no range");
    }

    #[test]
    fn a_restarted_decoder_counts_as_nothing_rather_than_a_huge_jump() {
        let mut g = Graphs::default();
        g.note(&status(10_000), None, None, 0);
        g.note(&status(20), None, None, 10);
        assert_eq!(g.samples[0].msgs_min, 0.0);
    }

    #[test]
    fn history_stops_at_a_day() {
        let mut g = Graphs::default();
        g.note(&status(0), None, None, 0);
        for i in 1..=(KEEP_S / STEP_S + 20) {
            g.note(&status(0), None, None, i * STEP_S);
        }
        let span = g.samples.back().unwrap().t - g.samples.front().unwrap().t;
        assert!(span <= KEEP_S);
    }

    #[test]
    fn readsb_stats_give_signal_noise_and_the_strong_share() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"last1min":{"local":{"signal":-17.4,"noise":-31.9,"strong_signals":50,"accepted":[4911,89]}}}"#,
        )
        .unwrap();
        let mut r = Remote::default();
        read_stats(&v, &mut r);
        assert_eq!((r.signal, r.noise), (Some(-17.4), Some(-31.9)));
        assert_eq!(r.strong_pct, Some(1.0), "50 of 5000");
    }

    #[test]
    fn the_station_file_gives_temperature_and_only_current_limits() {
        let mut r = Remote::default();
        let ok: serde_json::Value =
            serde_json::from_str(r#"{"cpu_temp_c":10.2,"wifi_dbm":-65,"throttled":"0x50000"}"#)
                .unwrap();
        read_station(&ok, &mut r);
        assert_eq!((r.temp_c, r.wifi_dbm), (Some(10.2), Some(-65.0)));
        assert!(!r.limited, "0x50000 only says it happened since boot");
        let now: serde_json::Value = serde_json::from_str(r#"{"throttled":"0x50005"}"#).unwrap();
        read_station(&now, &mut r);
        assert!(r.limited);
    }

    #[test]
    fn the_host_is_the_address_without_its_port() {
        assert_eq!(host_of("192.168.178.126:30005").as_deref(), Some("192.168.178.126"));
        assert_eq!(host_of("pimetal").as_deref(), Some("pimetal"));
        assert_eq!(host_of("[fe80::1]:30005").as_deref(), Some("[fe80::1]"));
        assert_eq!(host_of("  "), None);
    }
}
