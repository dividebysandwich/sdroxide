//! The band plan as data: band edges and sub-segments per IARU region, either
//! the built-in tables or whatever the operator has put in `bandplan.json`.
//!
//! ## Why it is a file
//!
//! The tables in [`crate::band`] and [`crate::band_segments`] are the *regional*
//! allocations, which is the most a program can honestly ship: national
//! administrations grant less than the region in places and more in others, a
//! licence class narrows it again, and the sub-band conventions inside a
//! country drift faster than a release cycle. An operator who knows their own
//! plan should be able to say so without waiting for anybody, and without
//! editing Rust.
//!
//! So the built-in tables are the *seed*. On first start the engine writes them
//! to `bandplan.json` in the config directory; from then on that file is the
//! authority, and every band edge, transmit lockout, waterfall strip and
//! skimmer window in the program follows it.
//!
//! ## Shape
//!
//! One [`RegionPlan`] per region, each holding the bands, the sub-segments and
//! the two skimmer windows. The runtime form indexes bands by
//! [`Band::ALL`] position and keeps segments sorted, so the lookups stay cheap;
//! the *file* form is a plain list of rows in **megahertz**, because that is how
//! band plans are published and how anyone editing one thinks.
//!
//! ## Why a global
//!
//! Same reason as [`crate::region`]: the plan is read from three dozen places
//! that have no business carrying it as a parameter, a station has exactly one,
//! and it does not change under you. Installed once at startup and again on an
//! explicit reload. Every entry point that reads the global also has a method
//! on [`BandPlan`] taking it explicitly, which is what the tests use.

use std::sync::{LazyLock, RwLock};

use serde::{Deserialize, Serialize};

use crate::band_segments::{Segment, SegmentKind};
use crate::{Band, Region};

/// One region's band plan.
#[derive(Clone, Debug, PartialEq)]
pub struct RegionPlan {
    /// Edges in Hz by [`Band::ALL`] index. `None` where this region has no such
    /// band — and always `None` for [`Band::Gen`], which is the absence of one.
    edges: [Option<(f64, f64)>; Band::ALL.len()],
    /// Sub-segments, sorted by lower edge.
    segments: Vec<Segment>,
    /// Where the PSK31 skimmer listens, in Hz.
    psk: Vec<(f64, f64)>,
    /// Where the RTTY skimmer listens, in Hz.
    rtty: Vec<(f64, f64)>,
}

impl RegionPlan {
    /// Edges in Hz for `band`, or `None` for a band this region does not have.
    pub fn edges(&self, band: Band) -> Option<(f64, f64)> {
        Band::ALL.iter().position(|b| *b == band).and_then(|i| self.edges[i])
    }

    /// The band containing `hz`, or [`Band::Gen`] if none does.
    pub fn containing(&self, hz: f64) -> Band {
        Band::ALL
            .iter()
            .zip(self.edges.iter())
            .find(|(_, e)| e.is_some_and(|(lo, hi)| hz >= lo && hz <= hi))
            .map(|(b, _)| *b)
            .unwrap_or(Band::Gen)
    }

    /// The sub-segments, ascending.
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Where PSK31 clusters, in Hz.
    pub fn psk_ranges(&self) -> &[(f64, f64)] {
        &self.psk
    }

    /// Where RTTY clusters, in Hz.
    pub fn rtty_ranges(&self) -> &[(f64, f64)] {
        &self.rtty
    }
}

/// The band plan for all three regions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "PlanFile", into = "PlanFile")]
pub struct BandPlan {
    r1: RegionPlan,
    r2: RegionPlan,
    r3: RegionPlan,
    /// What was wrong with the file this was read from, in the operator's
    /// terms. Empty for the built-in tables.
    ///
    /// Carried rather than logged at the point of discovery because this crate
    /// does no I/O: the loader prints them, and a bad row is dropped rather
    /// than allowed to take the whole file down with it.
    problems: Vec<String>,
}

impl Default for BandPlan {
    fn default() -> Self {
        BandPlan::iaru_defaults()
    }
}

impl BandPlan {
    /// The built-in IARU tables — the seed for a fresh `bandplan.json`, and
    /// what everything falls back to when there is no file or it cannot be
    /// read.
    pub fn iaru_defaults() -> BandPlan {
        BandPlan {
            r1: RegionPlan::iaru_default(Region::R1),
            r2: RegionPlan::iaru_default(Region::R2),
            r3: RegionPlan::iaru_default(Region::R3),
            problems: Vec::new(),
        }
    }

    pub fn region(&self, region: Region) -> &RegionPlan {
        match region {
            Region::R1 => &self.r1,
            Region::R2 => &self.r2,
            Region::R3 => &self.r3,
        }
    }

    /// What was wrong with the file this came from — rows dropped, edges the
    /// wrong way round, bands that overlap. Empty when there was nothing to
    /// complain about.
    pub fn problems(&self) -> &[String] {
        &self.problems
    }

    /// True where this is exactly the built-in table, so a caller can say
    /// whether the station is on the shipped plan or the operator's own.
    pub fn is_default(&self) -> bool {
        *self == *DEFAULTS
    }

    /// This plan as the text of `bandplan.json`: megahertz, one row per line,
    /// and the `readme` that stands in for the comments JSON cannot carry.
    ///
    /// Here rather than in the loader because it is the *format* — units,
    /// layout, self-documentation — and the loader's business is bytes and
    /// paths. `serde_json::to_string_pretty` would give the same data across
    /// nine hundred lines with every field on its own; this file exists to be
    /// read down and edited, so its rows stay on one line each.
    pub fn to_json_document(&self) -> String {
        let file = PlanFile { readme: readme(), ..PlanFile::from(self.clone()) };
        let mut out = Vec::new();
        let mut ser = serde_json::Serializer::with_formatter(&mut out, RowPerLine::default());
        // Infallible: writing to a `Vec` cannot fail, and every type in
        // `PlanFile` is a plain struct of numbers, strings and unit enums.
        file.serialize(&mut ser).expect("a band plan always serialises");
        let mut text = String::from_utf8(out).expect("serde_json emits UTF-8");
        text.push('\n');
        text
    }
}

/// `BandPlan` equality is about the *plan*, not about what the file it came
/// from got wrong: two stations running identical band edges are running the
/// same plan whether or not one of them had a typo above a dropped row.
///
/// This is what lets a client compare the plan a station just announced against
/// the one it already has — the announced copy has been through the wire, which
/// drops the problems, so a derived `PartialEq` would call every announcement a
/// change and leak a plan on each one.
impl PartialEq for BandPlan {
    fn eq(&self, other: &BandPlan) -> bool {
        self.r1 == other.r1 && self.r2 == other.r2 && self.r3 == other.r3
    }
}

impl RegionPlan {
    fn iaru_default(region: Region) -> RegionPlan {
        let mut edges = [None; Band::ALL.len()];
        for (i, band) in Band::ALL.iter().enumerate() {
            edges[i] = band.iaru_default_edges_in(region);
        }
        RegionPlan {
            edges,
            segments: crate::band_segments::default_segments_in(region).to_vec(),
            psk: crate::band_segments::default_psk_ranges_in(region).to_vec(),
            rtty: crate::band_segments::default_rtty_ranges_in(region).to_vec(),
        }
    }
}

// ── The file form ───────────────────────────────────────────────────────────
//
// Megahertz and plain rows, because this file exists to be edited by hand. The
// conversion to the runtime form rounds to the nearest hertz — 1.8385 MHz is
// not exactly representable, and no band plan has ever needed a fraction of a
// hertz.

fn mhz_to_hz(mhz: f64) -> f64 {
    (mhz * 1e6).round()
}

fn hz_to_mhz(hz: f64) -> f64 {
    hz / 1e6
}

/// One band's edges, in MHz.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct BandRow {
    /// Which band. `M160` … `M10`, `M6`, `M4`, `M2`, `M125`, `M70`, `Cm33`,
    /// `Cm23`, `Cm13`, `Cm9`, `Cm6`, `Cm3`. Leave a band out and this region
    /// simply does not have it — which is how the built-in tables give Regions 2
    /// and 3 no 4 m.
    band: Band,
    lo_mhz: f64,
    hi_mhz: f64,
}

/// One sub-segment, in MHz.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SegRow {
    lo_mhz: f64,
    hi_mhz: f64,
    /// `Cw`, `Digi`, `Phone`, `Beacon` or `All`.
    kind: SegmentKind,
}

/// One skimmer window, in MHz.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct WindowRow {
    lo_mhz: f64,
    hi_mhz: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct RegionFile {
    bands: Vec<BandRow>,
    segments: Vec<SegRow>,
    psk_windows: Vec<WindowRow>,
    rtty_windows: Vec<WindowRow>,
}

/// `bandplan.json` as it sits on disk.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct PlanFile {
    /// Ignored on load; written on seed so the file explains itself. JSON has
    /// no comments, so this is the next best thing.
    readme: Vec<String>,
    region1: RegionFile,
    region2: RegionFile,
    region3: RegionFile,
}

/// The self-documentation written into a freshly seeded file.
fn readme() -> Vec<String> {
    [
        "sdroxide band plan. Every band edge, transmit lockout, waterfall band strip \
         and skimmer window follows this file.",
        "All frequencies are in MHz. Edges are inclusive; a sub-segment covers \
         [lo_mhz, hi_mhz).",
        "bands: the amateur allocations. Omit a band and this region does not have it; \
         narrow one to your own licence and sdroxide will refuse to transmit outside it \
         (with tx_ham_only set, which is the default).",
        "A band sdroxide adds in a later version — 4 m (M4) was the first, the microwave bands \
         (Cm33 through Cm6) the latest — is not in a file written before it existed, so it is \
         filled in from the built-in tables when this file names it in no region at all. Give it \
         a row in any region and this file decides it everywhere, as it does for every other \
         band.",
        "segments: the sub-band plan drawn on the waterfall. kind is Cw, Digi, Phone, \
         Beacon or All.",
        "psk_windows / rtty_windows: where the wideband skimmers listen. Narrow windows \
         cost less CPU than wide ones.",
        "Which region applies is the IARU region setting on the General tab \
         (region in config.toml).",
        "Edit, save, then press RELOAD BAND PLAN on the General tab — no restart. Delete \
         this file to get the built-in IARU tables back; a fresh one is written on the \
         next reload or start.",
        "A file sdroxide cannot parse is left exactly as it is and the built-in tables are \
         used for that session, so a typo costs nothing. A single row that says something \
         impossible is dropped and named in the notice bar; the rest of the file still \
         applies.",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

impl From<BandPlan> for PlanFile {
    fn from(p: BandPlan) -> PlanFile {
        // No readme here: this conversion is also what the plan rides the wire
        // as, in the `StationConfig` bundle sent to every client on connect and
        // on every change, and a kilobyte of English prose has no business
        // going down a socket over and over. [`BandPlan::to_json_document`]
        // adds it for the one destination that wants it.
        PlanFile {
            readme: Vec::new(),
            region1: (&p.r1).into(),
            region2: (&p.r2).into(),
            region3: (&p.r3).into(),
        }
    }
}

impl From<&RegionPlan> for RegionFile {
    fn from(r: &RegionPlan) -> RegionFile {
        RegionFile {
            bands: Band::ALL
                .iter()
                .zip(r.edges.iter())
                .filter_map(|(band, e)| {
                    e.map(|(lo, hi)| BandRow {
                        band: *band,
                        lo_mhz: hz_to_mhz(lo),
                        hi_mhz: hz_to_mhz(hi),
                    })
                })
                .collect(),
            segments: r
                .segments
                .iter()
                .map(|s| SegRow { lo_mhz: hz_to_mhz(s.lo), hi_mhz: hz_to_mhz(s.hi), kind: s.kind })
                .collect(),
            psk_windows: r.psk.iter().map(|&(lo, hi)| WindowRow::from_hz(lo, hi)).collect(),
            rtty_windows: r.rtty.iter().map(|&(lo, hi)| WindowRow::from_hz(lo, hi)).collect(),
        }
    }
}

impl WindowRow {
    fn from_hz(lo: f64, hi: f64) -> WindowRow {
        WindowRow { lo_mhz: hz_to_mhz(lo), hi_mhz: hz_to_mhz(hi) }
    }
}

/// A file that parsed as JSON but says something impossible.
///
/// Only raised for the one thing that cannot be salvaged — a region with no
/// bands at all, which would leave the operator unable to transmit anywhere and
/// is far more likely to be a mistake than an intention. Everything else is
/// repaired row by row and reported through [`BandPlan::problems`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandPlanError(String);

// Written out rather than derived: this crate has no `thiserror`, and gaining a
// dependency for one message would be a poor trade — it compiles for wasm too.
impl std::fmt::Display for BandPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BandPlanError {}

/// Bands sdroxide gained *after* `bandplan.json` became the authority.
///
/// Omitting a band from a region means that region does not have it — that is
/// the file's whole contract, and it is not weakened here. But a file written
/// before a band existed is not omitting it; it has never heard of it, and every
/// installation that already had a file would otherwise be the one place the new
/// band never appears. So a band on this list that the file names in *no* region
/// is filled in from the built-in tables, exactly as a freshly seeded file would
/// have it. Name it in any region — even to give it different edges there — and
/// the file is the authority again, in all three.
///
/// New entries are appended as bands are added. Old ones stay: a file that
/// predates 4 m predates everything after it too.
const BANDS_ADDED_SINCE_THE_FILE: &[Band] = &[
    Band::M4,
    Band::M125,
    Band::Cm33,
    Band::Cm23,
    Band::Cm13,
    Band::Cm9,
    Band::Cm6,
    Band::Cm3,
];

impl TryFrom<PlanFile> for BandPlan {
    type Error = BandPlanError;

    fn try_from(f: PlanFile) -> Result<BandPlan, BandPlanError> {
        let mut problems = Vec::new();
        let mut r1 = RegionPlan::from_file(&f.region1, Region::R1, &mut problems)?;
        let mut r2 = RegionPlan::from_file(&f.region2, Region::R2, &mut problems)?;
        let mut r3 = RegionPlan::from_file(&f.region3, Region::R3, &mut problems)?;
        for band in BANDS_ADDED_SINCE_THE_FILE {
            let named = [&f.region1, &f.region2, &f.region3]
                .iter()
                .any(|r| r.bands.iter().any(|row| row.band == *band));
            if named {
                continue;
            }
            let Some(i) = Band::ALL.iter().position(|b| b == band) else { continue };
            for (plan, region) in
                [(&mut r1, Region::R1), (&mut r2, Region::R2), (&mut r3, Region::R3)]
            {
                plan.edges[i] = band.iaru_default_edges_in(region);
            }
        }
        Ok(BandPlan { r1, r2, r3, problems })
    }
}

impl RegionPlan {
    fn from_file(
        f: &RegionFile,
        region: Region,
        problems: &mut Vec<String>,
    ) -> Result<RegionPlan, BandPlanError> {
        let r = region.number();
        let mut edges: [Option<(f64, f64)>; Band::ALL.len()] = [None; Band::ALL.len()];
        for row in &f.bands {
            let (lo, hi) = (mhz_to_hz(row.lo_mhz), mhz_to_hz(row.hi_mhz));
            if row.band == Band::Gen {
                problems.push(format!(
                    "region {r}: GEN is not a band — it is what a frequency outside every \
                     band reports as. Row ignored."
                ));
                continue;
            }
            if !(lo.is_finite() && hi.is_finite()) || lo >= hi {
                problems.push(format!(
                    "region {r}: {} has lo_mhz {} and hi_mhz {}, which is not a band. \
                     Row ignored.",
                    row.band.label_in(region),
                    row.lo_mhz,
                    row.hi_mhz
                ));
                continue;
            }
            let i = Band::ALL.iter().position(|b| *b == row.band).expect("Band::ALL is complete");
            if edges[i].is_some() {
                problems.push(format!(
                    "region {r}: {} is listed twice; the later row wins.",
                    row.band.label_in(region)
                ));
            }
            edges[i] = Some((lo, hi));
        }
        if edges.iter().all(|e| e.is_none()) {
            return Err(BandPlanError(format!(
                "region {r} has no bands at all — that would leave every frequency out of \
                 band and every transmission refused"
            )));
        }

        // Overlapping bands are not rejected: `containing` takes the first
        // match, which is a defined answer, and an operator splitting a band
        // into licence classes may well want them to touch. But they are worth
        // saying out loud, because the *other* band then never appears.
        let mut spans: Vec<(Band, (f64, f64))> =
            Band::ALL.iter().zip(edges.iter()).filter_map(|(b, e)| e.map(|e| (*b, e))).collect();
        spans.sort_by(|a, b| a.1.0.total_cmp(&b.1.0));
        for w in spans.windows(2) {
            if w[0].1.1 >= w[1].1.0 {
                problems.push(format!(
                    "region {r}: {} ({}–{} MHz) overlaps {} ({}–{} MHz); a frequency in both \
                     reports as the first.",
                    w[0].0.label_in(region),
                    hz_to_mhz(w[0].1.0),
                    hz_to_mhz(w[0].1.1),
                    w[1].0.label_in(region),
                    hz_to_mhz(w[1].1.0),
                    hz_to_mhz(w[1].1.1),
                ));
            }
        }

        let mut segments: Vec<Segment> = Vec::with_capacity(f.segments.len());
        for row in &f.segments {
            let (lo, hi) = (mhz_to_hz(row.lo_mhz), mhz_to_hz(row.hi_mhz));
            if !(lo.is_finite() && hi.is_finite()) || lo >= hi {
                problems.push(format!(
                    "region {r}: segment {}–{} MHz is not a range. Row ignored.",
                    row.lo_mhz, row.hi_mhz
                ));
                continue;
            }
            segments.push(Segment { lo, hi, kind: row.kind });
        }
        // Sorted here rather than demanded of the file: the lookups walk this in
        // order and an operator adding a segment should not have to find the
        // right line to put it on.
        segments.sort_by(|a, b| a.lo.total_cmp(&b.lo));
        for w in segments.windows(2) {
            if w[0].hi > w[1].lo {
                problems.push(format!(
                    "region {r}: segments {}–{} MHz and {}–{} MHz overlap; a frequency in \
                     both is read as the lower one.",
                    hz_to_mhz(w[0].lo),
                    hz_to_mhz(w[0].hi),
                    hz_to_mhz(w[1].lo),
                    hz_to_mhz(w[1].hi),
                ));
            }
        }

        let windows = |rows: &[WindowRow], what: &str, problems: &mut Vec<String>| {
            rows.iter()
                .filter_map(|row| {
                    let (lo, hi) = (mhz_to_hz(row.lo_mhz), mhz_to_hz(row.hi_mhz));
                    if !(lo.is_finite() && hi.is_finite()) || lo >= hi {
                        problems.push(format!(
                            "region {r}: {what} window {}–{} MHz is not a range. Row ignored.",
                            row.lo_mhz, row.hi_mhz
                        ));
                        return None;
                    }
                    Some((lo, hi))
                })
                .collect()
        };
        let psk = windows(&f.psk_windows, "PSK", problems);
        let rtty = windows(&f.rtty_windows, "RTTY", problems);

        Ok(RegionPlan { edges, segments, psk, rtty })
    }
}

// ── One row per line ────────────────────────────────────────────────────────

/// A `serde_json` formatter that indents like the pretty one until it is deep
/// enough to be inside a row, then writes the rest of that row inline.
///
/// Whitespace only — every decision here is about where a newline goes, so it
/// cannot emit anything but the JSON the pretty formatter would have.
#[derive(Default)]
struct RowPerLine {
    /// One entry per open container, recording whether anything has been
    /// written into it — so an empty list closes as `[]` rather than as a
    /// bracket alone on the next line.
    open: Vec<bool>,
}

impl RowPerLine {
    /// Four containers deep is inside a row: root object → region object → row
    /// array → the row itself. Its fields go on one line; the array of rows
    /// still gets one row per line.
    const INLINE_FROM: usize = 4;

    fn inline(&self) -> bool {
        self.open.len() >= Self::INLINE_FROM
    }

    fn newline<W: ?Sized + std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_all(b"\n")?;
        for _ in 0..self.open.len() {
            w.write_all(b"  ")?;
        }
        Ok(())
    }

    fn begin<W: ?Sized + std::io::Write>(&mut self, w: &mut W, open: &[u8]) -> std::io::Result<()> {
        self.open.push(false);
        w.write_all(open)
    }

    /// Before an item: a comma if it is not the first, then either a space
    /// (inline) or a line break at this container's indent.
    fn separate<W: ?Sized + std::io::Write>(
        &mut self,
        w: &mut W,
        first: bool,
    ) -> std::io::Result<()> {
        if let Some(has) = self.open.last_mut() {
            *has = true;
        }
        if !first {
            w.write_all(b",")?;
        }
        if self.inline() {
            if !first {
                w.write_all(b" ")?;
            }
            Ok(())
        } else {
            self.newline(w)
        }
    }

    fn end<W: ?Sized + std::io::Write>(&mut self, w: &mut W, close: &[u8]) -> std::io::Result<()> {
        // Asked before the pop — whether this container's *contents* were
        // inline is what decides if its closing bracket needs a line of its
        // own. After the pop, `newline` indents to the container's own level,
        // which is where that bracket belongs.
        let was_inline = self.inline();
        let had_content = self.open.pop().unwrap_or(false);
        if had_content && !was_inline {
            self.newline(w)?;
        }
        w.write_all(close)
    }
}

impl serde_json::ser::Formatter for RowPerLine {
    fn begin_array<W: ?Sized + std::io::Write>(&mut self, w: &mut W) -> std::io::Result<()> {
        self.begin(w, b"[")
    }

    fn end_array<W: ?Sized + std::io::Write>(&mut self, w: &mut W) -> std::io::Result<()> {
        self.end(w, b"]")
    }

    fn begin_array_value<W: ?Sized + std::io::Write>(
        &mut self,
        w: &mut W,
        first: bool,
    ) -> std::io::Result<()> {
        self.separate(w, first)
    }

    fn begin_object<W: ?Sized + std::io::Write>(&mut self, w: &mut W) -> std::io::Result<()> {
        self.begin(w, b"{")
    }

    fn end_object<W: ?Sized + std::io::Write>(&mut self, w: &mut W) -> std::io::Result<()> {
        self.end(w, b"}")
    }

    fn begin_object_key<W: ?Sized + std::io::Write>(
        &mut self,
        w: &mut W,
        first: bool,
    ) -> std::io::Result<()> {
        self.separate(w, first)
    }

    fn begin_object_value<W: ?Sized + std::io::Write>(&mut self, w: &mut W) -> std::io::Result<()> {
        w.write_all(b": ")
    }
}

// ── The installed plan ──────────────────────────────────────────────────────

static DEFAULTS: LazyLock<BandPlan> = LazyLock::new(BandPlan::iaru_defaults);

/// The operator's plan once it has been loaded, leaked so the lookups can hand
/// out `&'static` slices without an `Arc` in every signature. One small
/// allocation per install, and installs happen at startup and on an explicit
/// reload — not in any loop.
static INSTALLED: RwLock<Option<&'static BandPlan>> = RwLock::new(None);

/// The band plan every lookup uses: the operator's `bandplan.json` once it has
/// been installed, the built-in IARU tables until then.
pub fn band_plan() -> &'static BandPlan {
    match INSTALLED.read() {
        Ok(g) => g.unwrap_or(&DEFAULTS),
        // A poisoned lock means a writer panicked mid-install. The built-in
        // tables are a safe answer and a great deal better than joining the
        // panic from inside a draw loop.
        Err(_) => &DEFAULTS,
    }
}

/// Adopt `plan` as the station's band plan.
///
/// Called from the binary and the engine at startup, from the engine on a
/// reload, and on a remote client when the station announces its own.
/// Everything downstream reads it on the next lookup, so nothing has to be
/// rebuilt — but the waterfall's band strip caches per region, so a client that
/// wants the strip to follow a *reload* has to clear that cache too.
pub fn set_band_plan(plan: BandPlan) {
    let leaked: &'static BandPlan = Box::leak(Box::new(plan));
    if let Ok(mut g) = INSTALLED.write() {
        *g = Some(leaked);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-tripping through the file form has to change nothing. If it did,
    /// the first thing sdroxide ever wrote to `bandplan.json` would be a
    /// different band plan from the one it had just been running.
    #[test]
    fn the_defaults_survive_a_write_and_a_read() {
        let plan = BandPlan::iaru_defaults();
        let text = serde_json::to_string_pretty(&plan).expect("serialise");
        let back: BandPlan = serde_json::from_str(&text).expect("parse");
        assert!(back.problems().is_empty(), "{:?}", back.problems());
        for region in Region::ALL {
            assert_eq!(back.region(region), plan.region(region), "{region:?}");
        }
        assert!(back.is_default());
    }

    /// The seeded file is in megahertz, one row per line, and names its bands,
    /// because it exists to be edited by hand.
    #[test]
    fn the_file_is_written_to_be_edited() {
        let text = BandPlan::iaru_defaults().to_json_document();
        assert!(text.contains(r#"{"band": "M160", "lo_mhz": 1.81, "hi_mhz": 2.0}"#), "{text}");
        assert!(text.contains(r#"{"band": "M70", "lo_mhz": 430.0, "hi_mhz": 440.0}"#), "{text}");
        // 4 m is written into the one region that has it, and nowhere else — and
        // so are 1.25 m and 33 cm, the two the Americas have to themselves.
        for row in [
            r#"{"band": "M4", "lo_mhz": 70.0, "hi_mhz": 70.5}"#,
            r#"{"band": "M125", "lo_mhz": 220.0, "hi_mhz": 225.0}"#,
            r#"{"band": "Cm33", "lo_mhz": 902.0, "hi_mhz": 928.0}"#,
        ] {
            assert_eq!(text.matches(row).count(), 1, "{row} in {text}");
        }
        // 23 cm is the same in all three, so it appears three times.
        assert_eq!(
            text.matches(r#"{"band": "Cm23", "lo_mhz": 1240.0, "hi_mhz": 1300.0}"#).count(),
            3,
            "{text}"
        );
        assert!(!text.contains("1810000"), "hertz should not appear in the file");
        // And it explains itself, since JSON cannot carry a comment.
        assert!(text.contains("\"readme\""));
        assert!(text.contains("MHz"), "the units have to be stated in the file");
        // Still JSON, and still the same plan when it comes back.
        let back: BandPlan = serde_json::from_str(&text).expect("parse");
        assert_eq!(back, BandPlan::iaru_defaults());
    }

    /// The readme is for the file, not for the socket: the plan rides in every
    /// `StationConfig` announcement, and a kilobyte of prose per announcement
    /// would be a kilobyte wasted every time a password changed.
    #[test]
    fn the_readme_does_not_ride_the_wire() {
        let wire = serde_json::to_string(&BandPlan::iaru_defaults()).expect("serialise");
        assert!(wire.contains("\"readme\":[]"), "{wire:.200}");
        assert!(!wire.contains("bandplan"), "no prose on the wire");
        assert!(BandPlan::iaru_defaults().to_json_document().contains("waterfall"));
    }

    /// Two stations on the same edges are on the same plan, whatever their
    /// files got wrong on the way — a client compares the announced plan
    /// against its own to decide whether to adopt it, and the announced copy
    /// has been through a wire that does not carry the complaints.
    #[test]
    fn equality_is_about_the_plan_not_the_typos() {
        let clean = BandPlan::iaru_defaults();
        let mut noisy = clean.clone();
        noisy.problems.push("something was dropped".into());
        assert_eq!(clean, noisy);
        assert!(noisy.is_default());
    }

    fn parse(json: &str) -> BandPlan {
        serde_json::from_str(json).expect("parse")
    }

    /// A band the file leaves out is a band the region does not have — which is
    /// how an operator says "no 60 m here".
    #[test]
    fn a_band_left_out_is_a_band_that_is_not_there() {
        let p = parse(
            r#"{"region1":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]},
                "region2":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]},
                "region3":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]}}"#,
        );
        let r1 = p.region(Region::R1);
        assert_eq!(r1.edges(Band::M20), Some((14_000_000.0, 14_350_000.0)));
        assert_eq!(r1.edges(Band::M60), None);
        assert_eq!(r1.containing(5_357_000.0), Band::Gen);
        assert_eq!(r1.containing(14_200_000.0), Band::M20);
        assert!(!p.is_default());
    }

    /// A file written before sdroxide had 4 m is not saying "no 4 m here" — it
    /// has never heard of the band. So it fills in from the built-in tables,
    /// which is what keeps every installation that already has a file from being
    /// the one place a new band never appears. Name it anywhere and the file is
    /// the authority again, in all three regions.
    #[test]
    fn a_band_added_after_the_file_fills_itself_in() {
        let older = parse(
            r#"{"region1":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]},
                "region2":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]},
                "region3":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]}}"#,
        );
        assert_eq!(older.region(Region::R1).edges(Band::M4), Some((70_000_000.0, 70_500_000.0)));
        assert_eq!(older.region(Region::R1).containing(70_174_000.0), Band::M4);
        // And only into the region that has the band at all.
        assert_eq!(older.region(Region::R2).edges(Band::M4), None);
        assert_eq!(older.region(Region::R3).edges(Band::M4), None);
        // The same for every band added since, one at a time: a file written
        // when 4 m was the newest band still gains the microwave ones.
        assert_eq!(
            older.region(Region::R1).edges(Band::Cm23),
            Some((1_240_000_000.0, 1_300_000_000.0))
        );
        assert_eq!(older.region(Region::R1).containing(1_296_174_000.0), Band::Cm23);
        assert_eq!(
            older.region(Region::R2).edges(Band::M125),
            Some((220_000_000.0, 225_000_000.0))
        );
        assert_eq!(older.region(Region::R1).edges(Band::M125), None);
        assert_eq!(
            older.region(Region::R1).edges(Band::Cm9),
            Some((3_400_000_000.0, 3_475_000_000.0))
        );
        assert_eq!(
            older.region(Region::R3).edges(Band::Cm9),
            Some((3_300_000_000.0, 3_500_000_000.0))
        );

        // Named in one region: the file decides 4 m everywhere from then on, so
        // leaving it out of Region 1 means Region 1 has not got it.
        let named = parse(
            r#"{"region1":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]},
                "region2":{"bands":[{"band":"M4","lo_mhz":70.0,"hi_mhz":70.5}]},
                "region3":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]}}"#,
        );
        assert_eq!(named.region(Region::R1).edges(Band::M4), None);
        assert_eq!(named.region(Region::R2).edges(Band::M4), Some((70_000_000.0, 70_500_000.0)));

        // A narrowed 4 m is obeyed like any other row — Germany's 50 kHz of it.
        let narrowed = parse(
            r#"{"region1":{"bands":[{"band":"M4","lo_mhz":70.15,"hi_mhz":70.2}]},
                "region2":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]},
                "region3":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]}}"#,
        );
        assert_eq!(narrowed.region(Region::R1).edges(Band::M4), Some((70_150_000.0, 70_200_000.0)));
        assert_eq!(narrowed.region(Region::R1).containing(70_174_000.0), Band::M4);
        assert_eq!(narrowed.region(Region::R1).containing(70_300_000.0), Band::Gen);
    }

    /// A narrowed band is the whole point: an operator whose licence stops at
    /// 144.4 should be locked out above it.
    #[test]
    fn a_narrowed_band_narrows_the_lockout() {
        let p = parse(
            r#"{"region1":{"bands":[{"band":"M2","lo_mhz":144.0,"hi_mhz":144.4}]},
                "region2":{"bands":[{"band":"M2","lo_mhz":144.0,"hi_mhz":148.0}]},
                "region3":{"bands":[{"band":"M2","lo_mhz":144.0,"hi_mhz":148.0}]}}"#,
        );
        assert_eq!(p.region(Region::R1).containing(145_500_000.0), Band::Gen);
        assert_eq!(p.region(Region::R2).containing(145_500_000.0), Band::M2);
    }

    /// A row that cannot mean anything is dropped and named, not obeyed and not
    /// fatal — one typo must not cost the operator the other twelve bands.
    #[test]
    fn a_nonsense_row_is_dropped_and_reported() {
        let p = parse(
            r#"{"region1":{"bands":[
                    {"band":"M20","lo_mhz":14.35,"hi_mhz":14.0},
                    {"band":"M40","lo_mhz":7.0,"hi_mhz":7.2},
                    {"band":"Gen","lo_mhz":0.0,"hi_mhz":30.0}],
                 "segments":[{"lo_mhz":7.1,"hi_mhz":7.0,"kind":"Phone"}],
                 "psk_windows":[{"lo_mhz":7.04,"hi_mhz":7.04}]},
                "region2":{"bands":[{"band":"M40","lo_mhz":7.0,"hi_mhz":7.3}]},
                "region3":{"bands":[{"band":"M40","lo_mhz":7.0,"hi_mhz":7.2}]}}"#,
        );
        let r1 = p.region(Region::R1);
        assert_eq!(r1.edges(Band::M20), None, "the reversed band was dropped");
        assert_eq!(r1.edges(Band::M40), Some((7_000_000.0, 7_200_000.0)), "the good row survived");
        assert!(r1.segments().is_empty(), "the reversed segment was dropped");
        assert!(r1.psk_ranges().is_empty(), "the empty window was dropped");
        let said = p.problems().join("\n");
        assert!(said.contains("20M"), "{said}");
        assert!(said.contains("GEN"), "{said}");
        assert!(said.contains("segment"), "{said}");
        assert!(said.contains("PSK"), "{said}");
    }

    /// Overlaps are kept — an operator may mean them — but never silently.
    #[test]
    fn overlaps_are_kept_and_named() {
        let p = parse(
            r#"{"region1":{"bands":[
                    {"band":"M40","lo_mhz":7.0,"hi_mhz":7.3},
                    {"band":"M30","lo_mhz":7.2,"hi_mhz":10.15}]},
                "region2":{"bands":[{"band":"M40","lo_mhz":7.0,"hi_mhz":7.3}]},
                "region3":{"bands":[{"band":"M40","lo_mhz":7.0,"hi_mhz":7.2}]}}"#,
        );
        assert_eq!(p.region(Region::R1).edges(Band::M30), Some((7_200_000.0, 10_150_000.0)));
        assert!(p.problems().iter().any(|s| s.contains("overlaps")), "{:?}", p.problems());
        // First match wins, and `Band::ALL` order decides which that is.
        assert_eq!(p.region(Region::R1).containing(7_250_000.0), Band::M40);
    }

    /// Segments are sorted on load, so an operator can append a row anywhere.
    #[test]
    fn segments_are_sorted_however_they_were_typed() {
        let p = parse(
            r#"{"region1":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}],
                 "segments":[
                    {"lo_mhz":14.1,"hi_mhz":14.35,"kind":"Phone"},
                    {"lo_mhz":14.0,"hi_mhz":14.07,"kind":"Cw"}]},
                "region2":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]},
                "region3":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]}}"#,
        );
        let segs = p.region(Region::R1).segments();
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].kind, SegmentKind::Cw);
        assert_eq!(segs[1].kind, SegmentKind::Phone);
        assert!(p.problems().is_empty(), "{:?}", p.problems());
    }

    /// A region with nothing in it is the one thing worth refusing: it would
    /// lock the operator out of every frequency they have.
    #[test]
    fn a_region_with_no_bands_is_refused() {
        let e = serde_json::from_str::<BandPlan>(
            r#"{"region1":{"bands":[]},
                "region2":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]},
                "region3":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]}}"#,
        )
        .expect_err("an empty region must not load");
        assert!(e.to_string().contains("region 1"), "{e}");
    }

    /// Megahertz in, hertz out, with no float dust: 5.3515 MHz has to come back
    /// as exactly 5_351_500 Hz or the 60 m edges drift by fractions of a hertz
    /// every time the file is rewritten.
    #[test]
    fn megahertz_convert_exactly() {
        for (mhz, hz) in
            [(1.81, 1_810_000.0), (5.3515, 5_351_500.0), (7.0475, 7_047_500.0), (0.153, 153_000.0)]
        {
            assert_eq!(mhz_to_hz(mhz), hz, "{mhz} MHz");
            assert_eq!(mhz_to_hz(hz_to_mhz(hz)), hz, "{hz} Hz round trip");
        }
    }

    /// Nothing installed means the built-in tables, which is what every other
    /// test in this crate relies on.
    #[test]
    fn the_uninstalled_plan_is_the_built_in_one() {
        assert!(band_plan().is_default());
    }
}
