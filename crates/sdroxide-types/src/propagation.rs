//! What got through, where, and on what — the evidence behind the propagation
//! map and the observed-MUF floor.
//!
//! Every mode that decodes a distant station has measured a path through the
//! ionosphere. WSPR does it deliberately; FT8, FT4 and JS8 do it as a side
//! effect of working people; a logged QSO did it before anybody thought to
//! write it down. This module is where those become one kind of statement, so a
//! map can be drawn from all of them at once.
//!
//! ## Two things it is careful about
//!
//! **Signal reports are not comparable across modes.** WSPR, FT8, FT4 and JS8
//! all quote SNR in a 2500 Hz bandwidth, so the numbers share units — but their
//! decode floors are ten decibels apart. A raw comparison would paint the most
//! sensitive mode on the band as the worst propagation on it. What is stored is
//! therefore the **margin above that mode's own floor**, power-corrected where
//! the transmit power is known. See [`PropObservation::margin_db`].
//!
//! **An RST is not an SNR.** A logged QSO contributes a path and no margin at
//! all, rather than a number invented from `59`. It counts toward how busy a
//! cell is and is excluded from the mean.
//!
//! ## What a cell means
//!
//! Observations are deposited at the **ionospheric control points** of the path
//! — the midpoints of each hop — not at the remote station. The map that
//! results answers "where is the ionosphere supporting this band", which is a
//! property of a place in the sky; a map of remote stations would answer "where
//! do radio amateurs live", which is already known. It is also the only
//! binning from which a MUF can honestly be read, because a MUF belongs to a
//! patch of ionosphere and not to a callsign.

use serde::{Deserialize, Serialize};

use crate::Band;

/// Where an observation came from.
///
/// Not decoration: the decode floor differs by ten decibels across these, and
/// whether the transmit power is known differs too. Both feed
/// [`PropObservation::margin_db`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropSource {
    /// A WSPR beacon this station decoded.
    Wspr,
    /// A WSPRnet report of somebody decoding *this* station.
    WsprHeardUs,
    Ft8,
    Ft4,
    Js8,
    /// A contact in the logbook. Carries a path and no signal report worth the
    /// name.
    Logged,
    /// A Reverse Beacon Network skimmer spot: somebody else's receiver heard
    /// somebody else's transmitter.
    ///
    /// The only source here whose paths do not touch this station at all, and
    /// the only one whose endpoints are not locators. An RBN line carries two
    /// callsigns and no grids, so both ends are placed at their DXCC entity's
    /// nominal centre — see [`crate::resolve_place`]. That is accurate enough
    /// for a small country and badly wrong for a large one, which is why this
    /// is a separate source with its own switch rather than more of the same
    /// evidence.
    ///
    /// Appended last deliberately: [`PropSources`](crate::PropSources) is a
    /// persisted bitmask over `ALL`, so a settings file written before this
    /// existed loads with RBN off, which is the right default for a source
    /// this coarse.
    Rbn,
    /// FT2 — FT4's waveform at double the symbol rate.
    ///
    /// Appended last for the same reason as [`PropSource::Rbn`]: the bitmask
    /// is positional over [`PropSource::ALL`], so inserting next to `Ft4`
    /// where it belongs logically would silently re-point every saved bit
    /// above it at the wrong source.
    Ft2,
}

impl PropSource {
    /// Roughly where this mode stops decoding, in dB referenced to 2500 Hz.
    ///
    /// The published figures for each protocol, which is what makes a −25 dB
    /// WSPR report and a −18 dB FT8 report comparable at all: the first has
    /// four decibels of margin and the second three.
    pub fn decode_floor_db(self) -> f32 {
        match self {
            PropSource::Wspr | PropSource::WsprHeardUs => -29.0,
            PropSource::Js8 => -24.0,
            PropSource::Ft8 => -21.0,
            PropSource::Ft4 => -17.0,
            // FT2 sends the same 174 coded bits as FT4 in half the time, so
            // it collects half the energy: 3 dB worse, and the on-air reports
            // from the first months (QSOs down to about -12) sit right there.
            // The mode's own publicity claims -23; that is the figure for
            // FT8's 15-second frame and cannot apply to a 2.52 s one.
            PropSource::Ft2 => -14.0,
            // A CW skimmer decodes down to a few dB S/N in its own narrow
            // bandwidth — a few hundred hertz, not the 2500 the other figures
            // are referenced to. Restating that floor in 2500 Hz costs about
            // 7 dB, which puts it here. Unlike the others this is an estimate
            // rather than a published protocol threshold: skimmer bandwidth
            // varies with the decoder, and RBN does not say which was used.
            PropSource::Rbn => -4.0,
            // Nothing to compare against: see `margin_db`.
            PropSource::Logged => 0.0,
        }
    }

    /// Whether the transmit power at the far end is known.
    ///
    /// True only for WSPR, whose message carries it — and that is the whole
    /// reason the correction exists. A 200 mW beacon heard at −25 dB describes
    /// a far better path than a 5 W one heard at the same level, and a map that
    /// ranked them equal would be actively misleading about which bands are
    /// open.
    pub fn power_known(self) -> bool {
        matches!(self, PropSource::Wspr | PropSource::WsprHeardUs)
    }

    pub fn label(self) -> &'static str {
        match self {
            PropSource::Wspr => "WSPR",
            PropSource::WsprHeardUs => "WSPR rx",
            PropSource::Ft8 => "FT8",
            PropSource::Ft4 => "FT4",
            PropSource::Ft2 => "FT2",
            PropSource::Js8 => "JS8",
            PropSource::Logged => "Log",
            PropSource::Rbn => "RBN",
        }
    }

    /// Every source, for the filter chips.
    pub const ALL: [PropSource; 8] = [
        PropSource::Wspr,
        PropSource::WsprHeardUs,
        PropSource::Ft8,
        PropSource::Ft4,
        PropSource::Js8,
        PropSource::Logged,
        PropSource::Rbn,
        PropSource::Ft2,
    ];
}

/// The transmit power margins are normalised to: 5 W, in dBm.
///
/// Chosen because it is what most stations run on the digital modes and what
/// WSPR's message can express exactly, so the correction is usually zero and
/// the numbers stay recognisable.
pub const REF_TX_DBM: f32 = 37.0;

/// The margin above a mode's decode floor, normalised to [`REF_TX_DBM`].
///
/// `tx_dbm` is the far end's transmit power where it is known. A beacon running
/// less than the reference gets *credited* the difference: it achieved the same
/// report with less power, which means a better path.
pub fn margin_db(source: PropSource, snr_db: f32, tx_dbm: Option<f32>) -> f32 {
    let power_correction = match tx_dbm.filter(|_| source.power_known()) {
        Some(dbm) => REF_TX_DBM - dbm,
        None => 0.0,
    };
    snr_db - source.decode_floor_db() + power_correction
}

/// One demonstrated path through the ionosphere: somebody transmitted, somebody
/// else decoded it, at a known frequency and time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PropObservation {
    pub tx: (f64, f64),
    pub rx: (f64, f64),
    pub freq_hz: f64,
    pub band: Band,
    /// Unix seconds at the slot it was heard in, not when we noticed.
    pub at_utc: i64,
    pub source: PropSource,
    /// Margin above the mode's decode floor — see [`margin_db`]. `None` where
    /// the source reports nothing that is an SNR.
    pub margin_db: Option<f32>,
    /// Great-circle length in km. Cached because every consumer wants it and
    /// the haversine is the expensive part of depositing one.
    pub path_km: f32,
}

impl PropObservation {
    /// Build an observation from two positions, filling in the band and the
    /// path length.
    ///
    /// `None` when the frequency is in no band the map can fold — the broadcast
    /// services, which are bands of their own now but are not amateur ones and
    /// have no plane to go in, and general coverage. (11 m *is* folded: the map
    /// carries CB skip for the operators who run it.)
    pub fn new(
        tx: (f64, f64),
        rx: (f64, f64),
        freq_hz: f64,
        at_utc: i64,
        source: PropSource,
        margin_db: Option<f32>,
    ) -> Option<Self> {
        let band = Band::containing(freq_hz);
        if matches!(band, Band::Gen | Band::Lw | Band::Mw | Band::Sw | Band::Fm) {
            return None;
        }
        Some(PropObservation {
            tx,
            rx,
            freq_hz,
            band,
            at_utc,
            source,
            margin_db,
            path_km: crate::distance_km(tx, rx) as f32,
        })
    }

    /// How many F2 hops this path most likely took.
    ///
    /// One hop reaches about 3000 km; past that the signal came down and went
    /// up again, and the ionosphere was involved once per hop. Capped at six,
    /// because past ~18 000 km the path is more than half way round the world
    /// and which way it went is a guess.
    pub fn hops(&self) -> u8 {
        ((self.path_km / MAX_HOP_KM).ceil() as u8).clamp(1, MAX_HOPS)
    }

    /// The ionospheric control points: the midpoint of each hop.
    ///
    /// For one hop that is the path midpoint. For three it is a third of the
    /// way along, then half, then five sixths — the places the signal actually
    /// touched the F layer, as far as a single reception report can say.
    pub fn control_points(&self) -> Vec<(f64, f64)> {
        let hops = self.hops() as usize;
        // Sampled off the great circle rather than interpolated in lat/lon,
        // which would put the points in the wrong place on any path that is not
        // due east-west. `great_circle_points` takes a *segment* count and
        // returns one more point than that, so `2·hops` segments give the
        // `2·hops + 1` samples whose odd indices are the hop midpoints.
        let along = crate::great_circle_points(self.tx, self.rx, 2 * hops);
        (0..hops).filter_map(|k| along.get(2 * k + 1).copied()).collect()
    }
}

/// One path somebody else's network demonstrated, ready to be folded in.
///
/// The wire and channel form of an observation whose endpoints were resolved
/// somewhere other than the propagation store — today that means the Reverse
/// Beacon Network, where a reader thread turns two callsigns into two positions
/// and hands the result on. Deliberately *not* a [`PropObservation`]: the band,
/// the path length and the margin all depend on things the store owns (which
/// sources are enabled, what the mode's floor is), so this carries only what
/// the reader actually knew.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropPath {
    /// Transmitter position, degrees.
    pub tx: (f64, f64),
    /// Receiver position, degrees.
    pub rx: (f64, f64),
    pub freq_hz: f64,
    /// Unix seconds the reception happened at.
    pub at_utc: i64,
    /// Signal-to-noise as the network reported it, before any floor
    /// correction. `None` where the report carries nothing that is an SNR.
    pub snr_db: Option<f32>,
    /// What makes this path distinct, for de-duplication — normally
    /// `"SPOTTER>DX"`. A firehose repeats the same path many times a minute
    /// and each repeat must be folded once.
    pub key: String,
}

/// The longest ground range a single F2 hop covers, in km.
pub const MAX_HOP_KM: f32 = 3000.0;
/// Hops past which the path is guesswork rather than geometry.
pub const MAX_HOPS: u8 = 6;

/// Nominal F2 reflection height, km. Used where no ionosonde is close enough to
/// say better.
pub const DEFAULT_HM_KM: f64 = 300.0;

/// Earth radius in km, matching [`crate::distance_km`].
const EARTH_R_KM: f64 = 6371.0;

/// Paths shorter than this contribute heat but no MUF bound.
///
/// Under a few hundred kilometres a signal may never have touched the
/// ionosphere at all — ground wave and line of sight both reach that far on
/// HF — and a bound inferred from a path that did not go up is not a bound.
pub const MIN_MUF_PATH_KM: f32 = 300.0;

/// Obliquity factor M(d): how much a hop of ground range `d_km` lifts the
/// vertical critical frequency.
///
/// The secant law on a curved Earth. The hop geometry fixes the elevation
/// angle, the elevation angle fixes the angle of incidence at the layer, and
/// the usable frequency is `foF2 · sec φ`. About 3.3 at 3000 km with the
/// default height, and 1 at zero range where the signal goes straight up.
pub fn obliquity_factor(d_km: f64, hm_km: f64) -> f64 {
    let hm = hm_km.max(80.0);
    let theta = (d_km.max(0.0) / 2.0) / EARTH_R_KM; // geocentric half-angle
    let ratio = EARTH_R_KM / (EARTH_R_KM + hm);
    // Elevation angle of the ray at the ground.
    let elev = ((theta.cos() - ratio) / theta.sin().max(1e-9)).atan();
    // Angle of incidence at the layer.
    let sin_phi = (ratio * elev.cos()).clamp(-1.0, 1.0);
    let cos_phi = (1.0 - sin_phi * sin_phi).sqrt().max(1e-6);
    1.0 / cos_phi
}

/// The lower bound on foF2 at the control point implied by `f_mhz` getting
/// through a hop of `d_km`.
///
/// Never an estimate of foF2: the signal got through, so the critical frequency
/// was **at least** this. How much more is exactly what a reception report
/// cannot say — nobody transmits on the frequencies that would have failed.
pub fn fof2_floor_mhz(f_mhz: f64, d_km: f64, hm_km: f64) -> f64 {
    f_mhz / obliquity_factor(d_km, hm_km)
}

/// The same bound expressed the way operators and ionosondes both quote it: as
/// a floor under MUF(3000), the maximum usable frequency for a 3000 km path.
///
/// This is what makes observations on different path lengths comparable. Twenty
/// metres over five hundred kilometres is a far hotter ionosphere than twenty
/// metres over three thousand, and only after normalising can the two be put on
/// one map.
pub fn muf3000_floor_mhz(f_mhz: f64, d_km: f64, hm_km: f64) -> f64 {
    let hops = (d_km / MAX_HOP_KM as f64).ceil().clamp(1.0, MAX_HOPS as f64);
    let hop = (d_km / hops).max(MIN_MUF_PATH_KM as f64);
    fof2_floor_mhz(f_mhz, hop, hm_km) * obliquity_factor(MAX_HOP_KM as f64, hm_km)
}

// ── The binned field ────────────────────────────────────────────────────────

/// Grid columns: 2.5° of longitude each, column 0 starting at 180° W.
pub const GRID_W: usize = 144;
/// Grid rows: 2.5° of latitude each, row 0 at the north pole.
pub const GRID_H: usize = 72;
/// Cells per plane.
pub const GRID_CELLS: usize = GRID_W * GRID_H;

/// Half-width of the deposit kernel, in km.
///
/// A reception report places the control point to within a few hundred
/// kilometres at best — the reflection height is assumed, the path is assumed
/// great-circle, and the ionosphere is not flat. Smearing by about that much is
/// honest rather than decorative, and it is also what makes the map read as a
/// field instead of a scatter of cells.
pub const SPLAT_SIGMA_KM: f64 = 400.0;
/// Where the kernel stops, and — see [`splat_kernel`] — reaches zero.
pub const SPLAT_CUTOFF_KM: f64 = 2.0 * SPLAT_SIGMA_KM;

/// The deposit kernel at great-circle distance `d_km`, peaking at 1.
///
/// A Gaussian lowered by its own value at the cutoff and rescaled back to a
/// peak of one, so that it *arrives* at zero rather than being chopped off
/// there. A plain truncated Gaussian still stands 13 % tall where it stops, and
/// on a 2.5° grid that step draws a hard rim around every splat — a circle
/// quantised to whole cells, which is exactly the polygon a smooth field should
/// never show. Fading it out costs one subtraction and removes the edge
/// outright, which no amount of filtering downstream can do.
pub fn splat_kernel(d_km: f64) -> f64 {
    if d_km >= SPLAT_CUTOFF_KM {
        return 0.0;
    }
    let g = |d: f64| (-(d * d) / (2.0 * SPLAT_SIGMA_KM * SPLAT_SIGMA_KM)).exp();
    let pedestal = g(SPLAT_CUTOFF_KM);
    ((g(d_km) - pedestal) / (1.0 - pedestal)).max(0.0)
}

/// Weight below which a cell is treated as empty and cleared.
const EPS: f32 = 1e-3;

/// Independent paths a cell needs before it will assert a MUF floor.
///
/// One path can be a mis-decode, a mis-typed grid, or a station whose locator
/// is wrong. Two that agree are evidence.
pub const MIN_MUF_PATHS: f32 = 2.0;

/// One band's accumulated evidence, as an equirectangular field.
///
/// Row 0 is the north pole and column 0 starts at 180° W, so the sphere's own
/// UV coordinates index it directly and the texture upload is a memcpy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BandPlane {
    /// Decayed, splatted evidence. Unbounded above; the display normalises.
    pub weight: Vec<f32>,
    /// Weight-weighted mean decode margin, dB. Meaningless where `margin_w` is
    /// zero, which is why that is kept rather than inferred from `weight`:
    /// logged QSOs add weight and no margin.
    pub margin: Vec<f32>,
    /// The weight behind `margin`.
    pub margin_w: Vec<f32>,
    /// The highest MUF(3000) floor any observation in this band implied through
    /// this cell, in MHz — the best this band did here inside the memory
    /// window. A maximum rather than a mean: the highest frequency that got
    /// through is the tightest bound, and averaging would only loosen it.
    pub muf_floor_mhz: Vec<f32>,
    /// Decayed count of contributing paths.
    pub paths: Vec<f32>,
    /// Wall clock this plane has been decayed to.
    pub decayed_utc: i64,
}

impl BandPlane {
    fn new(now_utc: i64) -> Self {
        BandPlane {
            weight: vec![0.0; GRID_CELLS],
            margin: vec![0.0; GRID_CELLS],
            margin_w: vec![0.0; GRID_CELLS],
            muf_floor_mhz: vec![0.0; GRID_CELLS],
            paths: vec![0.0; GRID_CELLS],
            decayed_utc: now_utc,
        }
    }

    /// Age the whole plane to `now_utc`.
    ///
    /// Lazy — applied when the plane is next read or written rather than on a
    /// timer — so a band nobody is looking at costs nothing at all until it is.
    fn decay_to(&mut self, now_utc: i64, halflife_s: f64) {
        let dt = (now_utc - self.decayed_utc) as f64;
        if dt <= 0.0 {
            return;
        }
        self.decayed_utc = now_utc;
        let k = 0.5f64.powf(dt / halflife_s.max(1.0)) as f32;
        if k >= 1.0 {
            return;
        }
        for i in 0..GRID_CELLS {
            self.weight[i] *= k;
            self.margin_w[i] *= k;
            self.paths[i] *= k;
            if self.weight[i] < EPS {
                // Cleared rather than merely faded: `muf_floor_mhz` is a
                // maximum and cannot decay smoothly, so a band that has gone
                // shut has to stop asserting the opening it had an hour ago.
                self.weight[i] = 0.0;
                self.margin[i] = 0.0;
                self.margin_w[i] = 0.0;
                self.muf_floor_mhz[i] = 0.0;
                self.paths[i] = 0.0;
            }
        }
    }

    /// The largest weight in the plane, for normalising a display.
    pub fn peak(&self) -> f32 {
        self.weight.iter().copied().fold(0.0, f32::max)
    }

    /// The share of the world this band is demonstrably getting through, from 0
    /// to 1.
    ///
    /// A cell counts wherever any weight survives — which is to say within the
    /// deposit kernel's reach of a control point, and that kernel is the
    /// positional uncertainty of a control point rather than decoration. So
    /// this is "the patch of ionosphere this band worked through, to the
    /// accuracy a reception report can place it".
    ///
    /// Saturating, which is the whole difference between it and a path count:
    /// forty contacts through one patch of sky say the band is open in one
    /// direction, and a count would call that forty times as open as one
    /// contact. It answers *how widely* rather than *how much*.
    ///
    /// Area-weighted by `cos(lat)`: on an equirectangular grid a polar cell is
    /// a sliver and an equatorial one is not, and counting cells would make the
    /// Arctic the largest place on Earth.
    pub fn reach(&self) -> f32 {
        let (mut lit, mut all) = (0.0f64, 0.0f64);
        for row in 0..GRID_H {
            let (lat, _) = cell_center(row, 0);
            let area = lat.to_radians().cos().max(0.0);
            all += area * GRID_W as f64;
            for col in 0..GRID_W {
                if self.weight[row * GRID_W + col] > 0.0 {
                    lit += area;
                }
            }
        }
        if all <= 0.0 { 0.0 } else { (lit / all) as f32 }
    }

    /// True where nothing survives.
    pub fn is_empty(&self) -> bool {
        self.peak() <= 0.0
    }

    /// How much evidence there is, as a decayed count of contributing paths.
    ///
    /// The companion to [`BandPlane::reach`] and not a substitute for it: this
    /// says *how much* got through and that says *how widely*. A pile-up on one
    /// bearing scores high here and low there, and reading only one of the two
    /// is how a busy contest corner comes to look like an open band.
    ///
    /// Fractional because it decays: a path counted an hour ago is worth less
    /// than half a path now.
    pub fn total_paths(&self) -> f32 {
        self.paths.iter().sum()
    }

    /// The best decode margin anywhere in the plane, dB above the mode's own
    /// floor.
    ///
    /// `None` where nothing carrying a margin has been deposited — a plane
    /// built only from logged contacts has paths and no signal reports, and
    /// inventing a zero for it would rank it alongside a band that was
    /// genuinely marginal.
    pub fn best_margin_db(&self) -> Option<f32> {
        (0..GRID_CELLS)
            .filter(|&i| self.margin_w[i] > 0.0)
            .map(|i| self.margin[i])
            .fold(None, |best: Option<f32>, m| Some(best.map_or(m, |b| b.max(m))))
    }
}

/// Every band's evidence, plus how fast it is forgotten.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropField {
    /// One plane per index into [`Band::ALL`], allocated on first use — a
    /// session spent on 20 m carries one plane, not fourteen.
    pub planes: Vec<Option<BandPlane>>,
    /// Bumped on every mutation, so a GPU upload can skip an unchanged field.
    pub generation: u64,
    /// How long an observation's contribution takes to halve, in seconds.
    pub halflife_s: f64,
}

/// Default memory: 45 minutes to halve.
///
/// The ionosphere's own memory is short. An opening two hours old should not be
/// arguing with a reception from two minutes ago, and a map that averaged the
/// whole evening would show the evening rather than the band.
pub const DEFAULT_HALFLIFE_S: f64 = 45.0 * 60.0;

impl Default for PropField {
    fn default() -> Self {
        PropField {
            planes: (0..Band::ALL.len()).map(|_| None).collect(),
            generation: 0,
            halflife_s: DEFAULT_HALFLIFE_S,
        }
    }
}

/// Grid cell containing a position. Row 0 is the north pole.
pub fn cell_of(lat: f64, lon: f64) -> (usize, usize) {
    let row = (((90.0 - lat) / 2.5).floor() as i64).clamp(0, GRID_H as i64 - 1) as usize;
    let col = (((lon + 180.0) / 2.5).floor() as i64).rem_euclid(GRID_W as i64) as usize;
    (row, col)
}

/// The centre of a grid cell.
pub fn cell_center(row: usize, col: usize) -> (f64, f64) {
    (90.0 - (row as f64 + 0.5) * 2.5, -180.0 + (col as f64 + 0.5) * 2.5)
}

impl PropField {
    /// Index of `band` into [`Self::planes`].
    fn plane_index(band: Band) -> Option<usize> {
        Band::ALL.iter().position(|b| *b == band)
    }

    /// The plane for `band`, aged to `now_utc`, if it has ever been written.
    pub fn plane(&self, band: Band) -> Option<&BandPlane> {
        Self::plane_index(band).and_then(|i| self.planes[i].as_ref())
    }

    /// Bands with anything in them, in [`Band::ALL`] order.
    pub fn live_bands(&self) -> Vec<Band> {
        Band::ALL
            .iter()
            .copied()
            .filter(|b| self.plane(*b).is_some_and(|p| !p.is_empty()))
            .collect()
    }

    /// Age every allocated plane to `now_utc`, dropping the ones that emptied.
    pub fn decay(&mut self, now_utc: i64) {
        let hl = self.halflife_s;
        let mut changed = false;
        for slot in self.planes.iter_mut() {
            if let Some(p) = slot.as_mut() {
                let before = p.decayed_utc;
                p.decay_to(now_utc, hl);
                if p.decayed_utc != before {
                    changed = true;
                }
                if p.is_empty() {
                    *slot = None;
                }
            }
        }
        if changed {
            self.generation = self.generation.wrapping_add(1);
        }
    }

    /// Deposit one observation at its ionospheric control points.
    ///
    /// Each hop's midpoint is splatted with a great-circle Gaussian, weighted
    /// `1/hops` so a long path does not out-vote a short one simply by touching
    /// the ionosphere more times.
    pub fn deposit(&mut self, obs: &PropObservation, now_utc: i64, hm_km: f64) {
        let Some(idx) = Self::plane_index(obs.band) else { return };
        let hl = self.halflife_s;
        let plane = self.planes[idx].get_or_insert_with(|| BandPlane::new(now_utc));
        plane.decay_to(now_utc, hl);

        let points = obs.control_points();
        if points.is_empty() {
            return;
        }
        let share = 1.0 / points.len() as f32;
        // The bound this observation implies, if it implies one at all.
        let muf = (obs.path_km >= MIN_MUF_PATH_KM)
            .then(|| muf3000_floor_mhz(obs.freq_hz / 1e6, obs.path_km as f64, hm_km) as f32)
            .unwrap_or(0.0);

        for (lat, lon) in points {
            splat(plane, lat, lon, share, obs.margin_db, muf);
        }
        self.generation = self.generation.wrapping_add(1);
    }

    /// What the observations bound the ionosphere to at `(lat, lon)`.
    ///
    /// `None` where no band has enough evidence there. Always a floor: see
    /// [`PropMuf`].
    pub fn muf_at(&self, lat: f64, lon: f64) -> Option<PropMuf> {
        let (row, col) = cell_of(lat, lon);
        let i = row * GRID_W + col;
        let mut best: Option<PropMuf> = None;
        for band in Band::ALL {
            let Some(p) = self.plane(band) else { continue };
            if p.paths[i] < MIN_MUF_PATHS || p.muf_floor_mhz[i] <= 0.0 {
                continue;
            }
            let cand = PropMuf {
                floor_mhz: p.muf_floor_mhz[i] as f64,
                band,
                paths: p.paths[i],
                newest_unix: p.decayed_utc,
            };
            if best.as_ref().is_none_or(|b| cand.floor_mhz > b.floor_mhz) {
                best = Some(cand);
            }
        }
        best
    }
}

/// Add one control point's contribution to a plane.
fn splat(plane: &mut BandPlane, lat: f64, lon: f64, share: f32, margin: Option<f32>, muf: f32) {
    // Latitude reach is fixed; longitude reach grows as 1/cos(lat), which is
    // exactly the area correction an equirectangular grid needs and comes free
    // from measuring the kernel in great-circle distance.
    let dlat = SPLAT_CUTOFF_KM / 111.19;
    let (r0, _) = cell_of((lat + dlat).min(90.0), lon);
    let (r1, _) = cell_of((lat - dlat).max(-90.0), lon);
    for row in r0..=r1.min(GRID_H - 1) {
        let (clat, _) = cell_center(row, 0);
        let cos = clat.to_radians().cos().abs().max(1e-3);
        let dlon = (SPLAT_CUTOFF_KM / (111.19 * cos)).min(180.0);
        let span = ((dlon / 2.5).ceil() as i64).min(GRID_W as i64 / 2);
        let (_, c0) = cell_of(lat, lon);
        for dc in -span..=span {
            let col = ((c0 as i64 + dc).rem_euclid(GRID_W as i64)) as usize;
            let (cl, cn) = cell_center(row, col);
            let d = crate::distance_km((lat, lon), (cl, cn));
            let w = share * splat_kernel(d) as f32;
            if w <= 0.0 {
                continue;
            }
            let i = row * GRID_W + col;
            plane.weight[i] += w;
            // The path count and the MUF claim are recorded only in the kernel's
            // core, at full share, and not out in its skirts.
            //
            // Two different uncertainties are in play. `weight` is a display
            // smear: the map should read as a field rather than a scatter of
            // cells. The count is evidence, and "how many hop midpoints landed
            // here" has to mean the same thing everywhere — weighting it by the
            // display kernel would make the two-path gate need four paths where
            // a control point happened to fall near a cell corner, and two
            // where it fell in the middle.
            if d <= SPLAT_SIGMA_KM {
                plane.paths[i] += share;
                if muf > plane.muf_floor_mhz[i] {
                    plane.muf_floor_mhz[i] = muf;
                }
            }
            if let Some(m) = margin {
                // Running weighted mean, so the whole history need not be kept.
                let total = plane.margin_w[i] + w;
                plane.margin[i] = (plane.margin[i] * plane.margin_w[i] + m * w) / total;
                plane.margin_w[i] = total;
            }
        }
    }
}

/// What the observations bound the ionosphere to at one place.
///
/// Named a floor and printed as one. Nothing downstream may present it as a
/// MUF: [`crate::Band`]-level evidence says the ionosphere was *at least* this
/// good, and how much better is precisely what a reception report cannot say —
/// nobody transmits on the frequencies that would have failed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropMuf {
    /// The bound, in MHz, expressed as a floor under MUF(3000).
    pub floor_mhz: f64,
    /// The band that produced it — the highest one still open through here.
    pub band: Band,
    /// Decayed count of paths supporting it.
    pub paths: f32,
    /// When the supporting plane was last touched.
    pub newest_unix: i64,
}

impl PropMuf {
    /// How much to trust it, in the same voice — and with the same honesty —
    /// as the ionosonde estimate it sits beside.
    pub fn confidence(&self) -> &'static str {
        if self.paths >= 6.0 {
            "well sampled"
        } else if self.paths >= 3.0 {
            "a few paths"
        } else {
            "thin — two paths"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(tx: (f64, f64), rx: (f64, f64), mhz: f64, src: PropSource) -> PropObservation {
        PropObservation::new(tx, rx, mhz * 1e6, 1_800_000_000, src, Some(10.0))
            .expect("an amateur band")
    }

    // ── margins ────────────────────────────────────────────────────────────

    /// The whole reason margins exist rather than raw SNRs: WSPR decodes ten
    /// decibels deeper than FT4, so the same number means very different things.
    #[test]
    fn the_same_snr_is_a_different_margin_in_a_more_sensitive_mode() {
        let wspr = margin_db(PropSource::Wspr, -20.0, Some(REF_TX_DBM));
        let ft4 = margin_db(PropSource::Ft4, -20.0, None);
        assert!((wspr - 9.0).abs() < 0.01, "wspr margin {wspr}");
        assert!((ft4 - -3.0).abs() < 0.01, "ft4 margin {ft4}");
        assert!(wspr > ft4, "the more sensitive mode came out worse");
    }

    /// A beacon that achieved the same report on less power found a better path.
    #[test]
    fn a_low_power_beacon_is_credited_for_the_power_it_did_not_use() {
        let five_w = margin_db(PropSource::Wspr, -25.0, Some(37.0));
        let two_hundred_mw = margin_db(PropSource::Wspr, -25.0, Some(23.0));
        assert!((two_hundred_mw - five_w - 14.0).abs() < 0.01, "{two_hundred_mw} vs {five_w}");
    }

    /// FT8 does not say what power it was sent with, so nothing is invented.
    #[test]
    fn power_is_only_corrected_where_the_mode_actually_reports_it() {
        assert_eq!(
            margin_db(PropSource::Ft8, -15.0, Some(20.0)),
            margin_db(PropSource::Ft8, -15.0, None),
        );
    }

    // ── geometry ───────────────────────────────────────────────────────────

    #[test]
    fn a_short_path_takes_one_hop_and_a_long_one_takes_several() {
        // London to Paris: one hop.
        let short = obs((51.5, -0.1), (48.9, 2.3), 14.1, PropSource::Ft8);
        assert_eq!(short.hops(), 1);
        assert_eq!(short.control_points().len(), 1);
        // London to Sydney: about 17 000 km.
        let long = obs((51.5, -0.1), (-33.9, 151.2), 14.1, PropSource::Ft8);
        assert!(long.path_km > 16_000.0, "{} km", long.path_km);
        assert_eq!(long.hops(), 6);
        assert_eq!(long.control_points().len(), 6);
    }

    /// The single-hop control point is the path midpoint, and it must be on the
    /// great circle rather than half way between the latitudes and longitudes.
    #[test]
    fn the_control_point_of_one_hop_is_the_great_circle_midpoint() {
        // London to Moscow: about 2500 km, comfortably one hop.
        let (a, b) = ((51.5, -0.1), (55.75, 37.6));
        let o = obs(a, b, 14.1, PropSource::Ft8);
        assert_eq!(o.hops(), 1, "{} km should be one hop", o.path_km);
        let mid = o.control_points()[0];
        // Equidistant from both ends, which is what "midpoint" has to mean.
        let da = crate::distance_km(a, mid);
        let db = crate::distance_km(b, mid);
        assert!((da - db).abs() < 20.0, "{da} km vs {db} km");
        // And poleward of the average of the two latitudes, which is what makes
        // it a *great-circle* midpoint rather than the mean of two coordinates:
        // the short way between two northern points bows north.
        assert!(mid.0 > (a.0 + b.0) / 2.0 + 1.0, "midpoint {mid:?} did not bow poleward");
    }

    /// A two-hop path has two control points, a quarter and three quarters
    /// along — the places the signal actually touched the layer.
    #[test]
    fn a_two_hop_path_puts_a_control_point_in_each_half() {
        let (a, b) = ((51.5, -0.1), (40.7, -74.0));
        let o = obs(a, b, 14.1, PropSource::Ft8);
        assert_eq!(o.hops(), 2, "{} km should be two hops", o.path_km);
        let pts = o.control_points();
        assert_eq!(pts.len(), 2);
        let d0 = crate::distance_km(a, pts[0]);
        let d1 = crate::distance_km(a, pts[1]);
        assert!((d0 / o.path_km as f64 - 0.25).abs() < 0.02, "first at {d0} km");
        assert!((d1 / o.path_km as f64 - 0.75).abs() < 0.02, "second at {d1} km");
        // Neither is the midpoint: a two-hop path never touched the middle.
        let midpoint = crate::great_circle_points(a, b, 2)[1];
        assert!(crate::distance_km(midpoint, pts[0]) > 1000.0);
        assert!(crate::distance_km(midpoint, pts[1]) > 1000.0);
    }

    #[test]
    fn a_frequency_outside_the_amateur_bands_has_no_plane_to_go_in() {
        assert!(
            PropObservation::new(
                (0.0, 0.0),
                (10.0, 10.0),
                9_420_000.0,
                0,
                PropSource::Logged,
                None
            )
            .is_none(),
            "a shortwave broadcast was accepted as a propagation observation"
        );
    }

    // ── the MUF inference ──────────────────────────────────────────────────

    #[test]
    fn the_obliquity_factor_matches_the_textbook_shape() {
        // Straight up: no obliquity at all.
        assert!((obliquity_factor(0.0, DEFAULT_HM_KM) - 1.0).abs() < 0.01);
        // The standard 3000 km reference lands in the range ionosondes report.
        let m3000 = obliquity_factor(3000.0, DEFAULT_HM_KM);
        assert!((3.0..3.6).contains(&m3000), "M(3000) = {m3000}");
        // Monotonic in range.
        assert!(obliquity_factor(1000.0, DEFAULT_HM_KM) < m3000);
    }

    /// The bound is a bound: it says the ionosphere was at least this good.
    #[test]
    fn a_signal_over_three_thousand_kilometres_bounds_muf3000_at_its_own_frequency() {
        let f = 14.1;
        let got = muf3000_floor_mhz(f, 3000.0, DEFAULT_HM_KM);
        assert!((got - f).abs() < 0.1, "{got} should be about {f}");
    }

    /// The point of normalising: the same frequency over a short path means a
    /// far hotter ionosphere, and the map has to be able to say so.
    #[test]
    fn the_same_band_over_a_short_path_implies_a_much_higher_muf() {
        let short = muf3000_floor_mhz(14.1, 600.0, DEFAULT_HM_KM);
        let long = muf3000_floor_mhz(14.1, 3000.0, DEFAULT_HM_KM);
        assert!(short > long * 2.0, "short {short} vs long {long}");
    }

    /// A multi-hop path bounds MUF(3000) at roughly its own frequency, because
    /// each hop is about the reference length. It must *not* be scaled up as
    /// though the whole distance were one hop.
    #[test]
    fn a_multi_hop_path_is_not_mistaken_for_one_enormous_hop() {
        let got = muf3000_floor_mhz(14.1, 12_000.0, DEFAULT_HM_KM);
        assert!((got - 14.1).abs() < 1.0, "{got} should be about 14.1");
    }

    // ── the field ──────────────────────────────────────────────────────────

    #[test]
    fn a_position_maps_to_the_cell_that_contains_it_and_back_again() {
        for (lat, lon) in [(0.0, 0.0), (51.5, -0.1), (-33.9, 151.2), (89.9, 179.9), (-89.9, -179.9)]
        {
            let (r, c) = cell_of(lat, lon);
            let (cl, cn) = cell_center(r, c);
            assert!((cl - lat).abs() <= 2.5, "{lat} landed in a cell centred at {cl}");
            assert!((cn - lon).abs() <= 2.5, "{lon} landed in a cell centred at {cn}");
        }
        // The antimeridian wraps rather than clamping.
        assert_eq!(cell_of(0.0, 180.0).1, cell_of(0.0, -180.0).1);
    }

    #[test]
    fn depositing_heats_the_control_point_and_not_the_endpoints() {
        let mut f = PropField::default();
        let o = obs((51.5, -0.1), (40.7, -74.0), 14.1, PropSource::Ft8);
        f.deposit(&o, 0, DEFAULT_HM_KM);
        let p = f.plane(Band::M20).expect("20 m plane");
        let mid = o.control_points()[0];
        let (mr, mc) = cell_of(mid.0, mid.1);
        assert!(p.weight[mr * GRID_W + mc] > 0.0, "the control point is cold");
        // London itself is 2000 km from the midpoint — well past the kernel.
        let (lr, lc) = cell_of(51.5, -0.1);
        assert_eq!(p.weight[lr * GRID_W + lc], 0.0, "the transmitter's own cell was heated");
    }

    #[test]
    fn a_band_with_no_observations_has_no_plane_at_all() {
        let mut f = PropField::default();
        f.deposit(&obs((51.5, -0.1), (40.7, -74.0), 14.1, PropSource::Ft8), 0, DEFAULT_HM_KM);
        assert!(f.plane(Band::M20).is_some());
        assert!(f.plane(Band::M10).is_none(), "an untouched band allocated a plane");
        assert_eq!(f.live_bands(), vec![Band::M20]);
    }

    #[test]
    fn heat_halves_over_the_half_life_and_then_expires() {
        let mut f = PropField::default();
        let o = obs((51.5, -0.1), (40.7, -74.0), 14.1, PropSource::Ft8);
        f.deposit(&o, 0, DEFAULT_HM_KM);
        let peak0 = f.plane(Band::M20).unwrap().peak();

        f.decay(DEFAULT_HALFLIFE_S as i64);
        let peak1 = f.plane(Band::M20).unwrap().peak();
        assert!((peak1 / peak0 - 0.5).abs() < 0.01, "{peak1} is not half of {peak0}");

        // Long enough and the plane is gone entirely, not merely faint: the MUF
        // floor is a maximum and cannot fade, so it has to be cleared.
        f.decay(DEFAULT_HALFLIFE_S as i64 * 40);
        assert!(f.plane(Band::M20).is_none(), "an expired band left a plane behind");
    }

    /// A logged QSO is a path and nothing more. It must not be able to move the
    /// margin, or somebody's habitual 59 would rewrite the map.
    #[test]
    fn a_logged_qso_lights_a_cell_but_does_not_move_the_margin() {
        let mut f = PropField::default();
        let mut ft8 = obs((51.5, -0.1), (40.7, -74.0), 14.1, PropSource::Ft8);
        ft8.margin_db = Some(12.0);
        f.deposit(&ft8, 0, DEFAULT_HM_KM);

        let mid = ft8.control_points()[0];
        let (r, c) = cell_of(mid.0, mid.1);
        let i = r * GRID_W + c;
        let before = f.plane(Band::M20).unwrap().margin[i];
        let weight_before = f.plane(Band::M20).unwrap().weight[i];

        let mut logged = ft8;
        logged.source = PropSource::Logged;
        logged.margin_db = None;
        f.deposit(&logged, 0, DEFAULT_HM_KM);

        let p = f.plane(Band::M20).unwrap();
        assert_eq!(p.margin[i], before, "an RST moved the margin");
        assert!(p.weight[i] > weight_before, "the path did not count at all");
    }

    /// Near the pole a fixed-radius kernel has to cover many more longitude
    /// cells, or one spot would light a sliver where it should light a patch.
    #[test]
    fn a_polar_splat_covers_more_longitude_cells_than_an_equatorial_one() {
        let count = |lat: f64| {
            let mut plane = BandPlane::new(0);
            splat(&mut plane, lat, 0.0, 1.0, Some(5.0), 0.0);
            plane.weight.iter().filter(|w| **w > 0.0).count()
        };
        let equator = count(0.0);
        let polar = count(80.0);
        assert!(polar > equator * 2, "equator {equator}, 80 °N {polar}");
        // And it stays bounded rather than running away at the pole itself.
        assert!(count(89.0) < GRID_CELLS, "the kernel covered the whole globe");
    }

    /// The kernel has to reach zero where it stops. A truncated Gaussian is
    /// still 13 % tall at 2σ, and that step is a hard rim around every splat.
    #[test]
    fn the_deposit_kernel_fades_out_rather_than_stopping_dead() {
        assert!((splat_kernel(0.0) - 1.0).abs() < 1e-6, "the peak moved");
        assert_eq!(splat_kernel(SPLAT_CUTOFF_KM), 0.0);
        assert_eq!(splat_kernel(SPLAT_CUTOFF_KM * 2.0), 0.0);
        // Approaching the cutoff from inside, it is already negligible — which
        // is what "no edge" means.
        let last = splat_kernel(SPLAT_CUTOFF_KM - 1.0);
        assert!(last > 0.0 && last < 0.002, "the kernel steps off a cliff at {last}");
        // Still monotonically falling, so it is a splat and not a ring.
        let mut prev = f64::INFINITY;
        for i in 0..=40 {
            let k = splat_kernel(SPLAT_CUTOFF_KM * i as f64 / 40.0);
            assert!(k <= prev, "the kernel rose at {i}");
            prev = k;
        }
    }

    // ── reach ──────────────────────────────────────────────────────────────

    /// One report lights the sky it could have come through and no more: a disc
    /// the size of the kernel, which is a fraction of a per cent of the world.
    #[test]
    fn one_observation_reaches_about_the_kernels_own_footprint() {
        let mut f = PropField::default();
        f.deposit(&obs((51.5, -0.1), (55.75, 37.6), 14.1, PropSource::Ft8), 0, DEFAULT_HM_KM);
        let reach = f.plane(Band::M20).unwrap().reach();
        // π·800 km² against the Earth's 5.1·10⁸ km² is about 0.4 %.
        assert!(
            (0.002..0.008).contains(&reach),
            "one splat reached {:.3} % of the world",
            reach * 100.0
        );
    }

    /// Reach saturates where a path count would not. Twenty contacts into the
    /// same patch of sky are one direction, however busy the evening was.
    #[test]
    fn working_the_same_place_repeatedly_does_not_widen_the_reach() {
        let mut f = PropField::default();
        let same = obs((51.5, -0.1), (55.75, 37.6), 14.1, PropSource::Ft8);
        f.deposit(&same, 0, DEFAULT_HM_KM);
        let one = f.plane(Band::M20).unwrap().reach();
        for _ in 0..20 {
            f.deposit(&same, 0, DEFAULT_HM_KM);
        }
        let twenty = f.plane(Band::M20).unwrap().reach();
        assert!((twenty - one).abs() < 1e-6, "{one} became {twenty}");

        // A path in the other direction does widen it, which is the point.
        f.deposit(&obs((51.5, -0.1), (-33.9, 151.2), 14.1, PropSource::Ft8), 0, DEFAULT_HM_KM);
        assert!(
            f.plane(Band::M20).unwrap().reach() > twenty * 2.0,
            "a second direction added nothing"
        );
    }

    /// The reach is measured on a sphere, not on the map it is stored as: a
    /// polar splat covers many more cells and no more world.
    #[test]
    fn a_polar_splat_does_not_reach_further_than_an_equatorial_one() {
        let reach_at = |lat: f64| {
            let mut plane = BandPlane::new(0);
            splat(&mut plane, lat, 0.0, 1.0, Some(5.0), 0.0);
            plane.reach()
        };
        let equator = reach_at(0.0);
        let polar = reach_at(80.0);
        assert!(polar < equator * 1.6, "equator {equator}, 80 °N {polar}");
        assert!(polar > equator * 0.4, "equator {equator}, 80 °N {polar}");
    }

    /// Reach is only as old as the field is: it is read off the same decayed
    /// weights, so a band that has gone quiet stops claiming anything.
    #[test]
    fn reach_expires_with_the_evidence_behind_it() {
        let mut f = PropField::default();
        f.deposit(&obs((51.5, -0.1), (55.75, 37.6), 14.1, PropSource::Ft8), 0, DEFAULT_HM_KM);
        assert!(f.plane(Band::M20).unwrap().reach() > 0.0);
        f.decay(DEFAULT_HALFLIFE_S as i64 * 40);
        assert!(f.plane(Band::M20).is_none(), "the plane outlived its evidence");
    }

    /// One path is not evidence — a mis-typed locator would otherwise assert an
    /// opening that never happened.
    #[test]
    fn a_single_path_does_not_assert_a_muf_floor() {
        let mut f = PropField::default();
        // One hop, so each deposit is one whole path at its control point.
        let o = obs((51.5, -0.1), (55.75, 37.6), 14.1, PropSource::Ft8);
        assert_eq!(o.hops(), 1);
        f.deposit(&o, 0, DEFAULT_HM_KM);
        let mid = o.control_points()[0];
        assert!(f.muf_at(mid.0, mid.1).is_none(), "one path claimed a MUF");

        // A second, independent path through the same place does.
        f.deposit(&o, 0, DEFAULT_HM_KM);
        let m = f.muf_at(mid.0, mid.1).expect("two paths is evidence");
        assert_eq!(m.band, Band::M20);
        assert!(m.floor_mhz > 10.0, "{}", m.floor_mhz);
    }

    /// A six-hop path's control points are guesses, so each is a sixth of a
    /// path rather than a whole one — one such report must not clear the gate
    /// that two real observations exist to pass.
    #[test]
    fn a_long_multi_hop_report_counts_for_less_than_a_single_hop_one() {
        let mut f = PropField::default();
        let long = obs((51.5, -0.1), (-33.9, 151.2), 14.1, PropSource::Ft8);
        assert_eq!(long.hops(), 6);
        for _ in 0..6 {
            f.deposit(&long, 0, DEFAULT_HM_KM);
        }
        let p = long.control_points()[0];
        // Six reports × one sixth each = one path's worth. Still not evidence.
        assert!(f.muf_at(p.0, p.1).is_none(), "six assumed hop points claimed a MUF");
    }

    /// The floor takes the highest band that got through, because that is the
    /// tightest bound. A lower band still open says less.
    #[test]
    fn the_floor_comes_from_the_highest_band_still_open() {
        let mut f = PropField::default();
        let low = obs((51.5, -0.1), (55.75, 37.6), 7.04, PropSource::Ft8);
        let high = obs((51.5, -0.1), (55.75, 37.6), 28.07, PropSource::Ft8);
        for _ in 0..3 {
            f.deposit(&low, 0, DEFAULT_HM_KM);
            f.deposit(&high, 0, DEFAULT_HM_KM);
        }
        let mid = low.control_points()[0];
        let m = f.muf_at(mid.0, mid.1).expect("evidence on both bands");
        assert_eq!(m.band, Band::M10, "the 40 m evidence out-voted the 10 m evidence");
    }

    /// A path short enough to have been ground wave contributes heat but must
    /// not be allowed to claim the ionosphere did anything.
    #[test]
    fn a_very_short_path_heats_the_map_without_claiming_a_muf() {
        let mut f = PropField::default();
        // ~150 km.
        let o = obs((51.5, -0.1), (52.5, 1.0), 14.1, PropSource::Ft8);
        assert!(o.path_km < MIN_MUF_PATH_KM, "{} km", o.path_km);
        for _ in 0..4 {
            f.deposit(&o, 0, DEFAULT_HM_KM);
        }
        let mid = o.control_points()[0];
        let (r, c) = cell_of(mid.0, mid.1);
        assert!(f.plane(Band::M20).unwrap().weight[r * GRID_W + c] > 0.0, "no heat");
        assert!(f.muf_at(mid.0, mid.1).is_none(), "a 150 km path claimed a MUF");
    }
}
