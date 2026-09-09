//! Ham-band sub-segment data (CW / digimode / phone / beacon ranges), shared so
//! the engine can gate skimmers by segment and the UI can draw the band plan.
//!
//! One table per IARU region, picked by the station's [`crate::Region`]
//! setting. Frequencies are absolute Hz. Only HF amateur bands are covered — a
//! frequency outside every listed segment returns `None`, which is what
//! everything above 30 MHz does.
//!
//! ## What is a segment and what is a convention
//!
//! Two different kinds of fact live here and they are kept apart on purpose.
//!
//! The **segments** are the region's band plan: which part of a band is for CW,
//! for narrow data, for phone. They differ by region and that is the whole
//! reason this module is parameterised.
//!
//! The **dial tables** — FT8, FT4, JS8, WSPR and the rest — are what the modes'
//! own communities settled on, and most of those are global: WSJT-X ships one
//! frequency list for the whole world. Only where a mode really does operate in
//! two places, like PSK31 and RTTY on 40 m, is a dial tagged with the regions
//! it belongs to.

use serde::{Deserialize, Serialize};

use crate::Region;
use crate::region::mask;

/// The operating category of a band sub-segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentKind {
    /// CW / Morse sub-band.
    Cw,
    /// Narrow-band data / digimode sub-band (RTTY, PSK, FT8, …).
    Digi,
    /// SSB / phone sub-band.
    Phone,
    /// Beacon sub-band.
    Beacon,
    /// All modes. The Region 2 and 3 band plans hand whole stretches of a band
    /// to "all modes" rather than splitting phone from data the way Region 1
    /// does, and calling those phone would be wrong in both directions: it
    /// would flag a perfectly ordinary data frequency as out of plan, and it
    /// would tell the operator the segment is narrower than it is.
    ///
    /// Appended last: `SegmentKind` is serialised, and postcard numbers
    /// variants by declaration order.
    All,
}

/// One band sub-segment: `[lo, hi)` in Hz with its operating category.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub lo: f64,
    pub hi: f64,
    pub kind: SegmentKind,
}

const fn seg(lo: f64, hi: f64, kind: SegmentKind) -> Segment {
    Segment { lo, hi, kind }
}

const M: f64 = 1_000_000.0;

use SegmentKind::{All, Beacon, Cw, Digi, Phone};

/// HF sub-segments for IARU Region 1, sorted by frequency.
pub const SEGMENTS_R1: &[Segment] = &[
    // 160m
    seg(1.810 * M, 1.838 * M, Cw),
    seg(1.838 * M, 1.843 * M, Digi),
    seg(1.843 * M, 2.000 * M, Phone),
    // 80m
    seg(3.500 * M, 3.570 * M, Cw),
    seg(3.570 * M, 3.600 * M, Digi),
    seg(3.600 * M, 3.800 * M, Phone),
    // 60m — the WRC-15 allocation is 15 kHz of all-modes everywhere.
    seg(5.3515 * M, 5.3665 * M, All),
    // 40m
    seg(7.000 * M, 7.040 * M, Cw),
    seg(7.040 * M, 7.100 * M, Digi),
    seg(7.100 * M, 7.200 * M, Phone),
    // 30m (no phone)
    seg(10.100 * M, 10.130 * M, Cw),
    seg(10.130 * M, 10.150 * M, Digi),
    // 20m
    seg(14.000 * M, 14.070 * M, Cw),
    seg(14.070 * M, 14.099 * M, Digi),
    seg(14.099 * M, 14.101 * M, Beacon),
    seg(14.101 * M, 14.350 * M, Phone),
    // 17m
    seg(18.068 * M, 18.095 * M, Cw),
    seg(18.095 * M, 18.109 * M, Digi),
    seg(18.109 * M, 18.111 * M, Beacon),
    seg(18.111 * M, 18.168 * M, Phone),
    // 15m
    seg(21.000 * M, 21.070 * M, Cw),
    seg(21.070 * M, 21.150 * M, Digi),
    seg(21.150 * M, 21.450 * M, Phone),
    // 12m
    seg(24.890 * M, 24.915 * M, Cw),
    seg(24.915 * M, 24.930 * M, Digi),
    seg(24.930 * M, 24.990 * M, Phone),
    // 11m — the CB band, which is all-modes by regulation rather than a ham
    // segment: no amateur plan splits it, so one `All` span covers the forty
    // channels and the digimode cluster above them.
    seg(26.965 * M, 27.710 * M, All),
    // 10m
    seg(28.000 * M, 28.070 * M, Cw),
    seg(28.070 * M, 28.190 * M, Digi),
    seg(28.190 * M, 28.300 * M, Beacon),
    seg(28.300 * M, 29.700 * M, Phone),
];

/// HF sub-segments for IARU Region 2 (the Americas), sorted by frequency.
///
/// Three bands are genuinely different rather than merely relabelled: 160 m
/// starts 10 kHz lower and opens with a digimode window, 80 m runs to 4.000 and
/// 40 m to 7.300. Above 30 m the split points are Region 1's, because the
/// modes that define them are the same modes — but the top of each band is "all
/// modes" here rather than a phone sub-band.
pub const SEGMENTS_R2: &[Segment] = &[
    // 160m — the extra 1.800–1.810 is digimodes in this region's plan.
    seg(1.800 * M, 1.810 * M, Digi),
    seg(1.810 * M, 1.838 * M, Cw),
    seg(1.838 * M, 1.843 * M, Digi),
    seg(1.843 * M, 2.000 * M, All),
    // 80m — 500 kHz wide, and the top 200 kHz is where the phone activity is.
    seg(3.500 * M, 3.570 * M, Cw),
    seg(3.570 * M, 3.600 * M, Digi),
    seg(3.600 * M, 4.000 * M, All),
    // 60m
    seg(5.3515 * M, 5.3665 * M, All),
    // 40m — the CW/data boundary is 5 kHz lower than Region 1's, and the band
    // runs 100 kHz further up.
    seg(7.000 * M, 7.035 * M, Cw),
    seg(7.035 * M, 7.100 * M, Digi),
    seg(7.100 * M, 7.300 * M, All),
    // 30m
    seg(10.100 * M, 10.130 * M, Cw),
    seg(10.130 * M, 10.150 * M, Digi),
    // 20m
    seg(14.000 * M, 14.070 * M, Cw),
    seg(14.070 * M, 14.099 * M, Digi),
    seg(14.099 * M, 14.101 * M, Beacon),
    seg(14.101 * M, 14.350 * M, All),
    // 17m
    seg(18.068 * M, 18.095 * M, Cw),
    seg(18.095 * M, 18.109 * M, Digi),
    seg(18.109 * M, 18.111 * M, Beacon),
    seg(18.111 * M, 18.168 * M, All),
    // 15m
    seg(21.000 * M, 21.070 * M, Cw),
    seg(21.070 * M, 21.150 * M, Digi),
    seg(21.150 * M, 21.450 * M, All),
    // 12m
    seg(24.890 * M, 24.915 * M, Cw),
    seg(24.915 * M, 24.930 * M, Digi),
    seg(24.930 * M, 24.990 * M, All),
    // 11m — the CB band, all-modes, and identical in every region for the same
    // reason it is permissive in Region 2's plan: nothing here is amateur and
    // nothing is split.
    seg(26.965 * M, 27.710 * M, All),
    // 10m
    seg(28.000 * M, 28.070 * M, Cw),
    seg(28.070 * M, 28.190 * M, Digi),
    seg(28.190 * M, 28.300 * M, Beacon),
    seg(28.300 * M, 29.700 * M, All),
];

/// HF sub-segments for IARU Region 3 (Asia-Pacific), sorted by frequency.
///
/// Between the other two: 160 m starts at 1.800 as in Region 2 but has no
/// digimode window at the bottom, 80 m stops at 3.900, and 40 m stops at 7.200
/// as in Region 1 while the data conventions on it are Region 2's.
pub const SEGMENTS_R3: &[Segment] = &[
    // 160m
    seg(1.800 * M, 1.838 * M, Cw),
    seg(1.838 * M, 1.843 * M, Digi),
    seg(1.843 * M, 2.000 * M, All),
    // 80m
    seg(3.500 * M, 3.570 * M, Cw),
    seg(3.570 * M, 3.600 * M, Digi),
    seg(3.600 * M, 3.900 * M, All),
    // 60m
    seg(5.3515 * M, 5.3665 * M, All),
    // 40m
    seg(7.000 * M, 7.035 * M, Cw),
    seg(7.035 * M, 7.100 * M, Digi),
    seg(7.100 * M, 7.200 * M, All),
    // 30m
    seg(10.100 * M, 10.130 * M, Cw),
    seg(10.130 * M, 10.150 * M, Digi),
    // 20m
    seg(14.000 * M, 14.070 * M, Cw),
    seg(14.070 * M, 14.099 * M, Digi),
    seg(14.099 * M, 14.101 * M, Beacon),
    seg(14.101 * M, 14.350 * M, All),
    // 17m
    seg(18.068 * M, 18.095 * M, Cw),
    seg(18.095 * M, 18.109 * M, Digi),
    seg(18.109 * M, 18.111 * M, Beacon),
    seg(18.111 * M, 18.168 * M, All),
    // 15m
    seg(21.000 * M, 21.070 * M, Cw),
    seg(21.070 * M, 21.150 * M, Digi),
    seg(21.150 * M, 21.450 * M, All),
    // 12m
    seg(24.890 * M, 24.915 * M, Cw),
    seg(24.915 * M, 24.930 * M, Digi),
    seg(24.930 * M, 24.990 * M, All),
    // 11m — the CB band, all-modes, and identical in every region for the same
    // reason it is permissive in Region 2's plan: nothing here is amateur and
    // nothing is split.
    seg(26.965 * M, 27.710 * M, All),
    // 10m
    seg(28.000 * M, 28.070 * M, Cw),
    seg(28.070 * M, 28.190 * M, Digi),
    seg(28.190 * M, 28.300 * M, Beacon),
    seg(28.300 * M, 29.700 * M, All),
];

/// The built-in HF sub-segments for `region` — the seed for a fresh
/// `bandplan.json`, and what is used until one is loaded.
pub(crate) fn default_segments_in(region: Region) -> &'static [Segment] {
    match region {
        Region::R1 => SEGMENTS_R1,
        Region::R2 => SEGMENTS_R2,
        Region::R3 => SEGMENTS_R3,
    }
}

/// The HF sub-segments of `region`'s band plan, sorted by frequency — from the
/// station's [`crate::BandPlan`].
pub fn segments_in(region: Region) -> &'static [Segment] {
    crate::band_plan().region(region).segments()
}

/// The HF sub-segments of the station's configured region.
pub fn segments() -> &'static [Segment] {
    segments_in(crate::region())
}

/// The operating category at `hz` in `region`, or `None` outside every listed
/// HF segment.
pub fn segment_kind_at_in(hz: f64, region: Region) -> Option<SegmentKind> {
    segments_in(region).iter().find(|s| hz >= s.lo && hz < s.hi).map(|s| s.kind)
}

/// The operating category at `hz`, or `None` outside every listed HF segment.
pub fn segment_kind_at(hz: f64) -> Option<SegmentKind> {
    segment_kind_at_in(hz, crate::region())
}

/// True when the whole span `lo..=hi` fits inside one sub-segment of `region`'s
/// band plan.
///
/// Unlike [`segment_kind_at_in`], which asks about a point, this asks about an
/// emission: a signal has width, and the question at a band edge is whether all
/// of it is inside, not whether its carrier is.
///
/// The upper bound is INCLUSIVE, deliberately, where `segment_kind_at_in`'s is
/// not. A segment ending at 5358.0 kHz permits a transmission reaching exactly
/// 5358.0; the exclusive test would refuse it, and the operator who did the
/// arithmetic correctly would be the one told no.
///
/// Touching segments are MERGED first, so a span crossing from one into the next
/// is accepted where they meet. This asks a question about legality, not about
/// band-plan etiquette: a signal reaching out of the digital sub-segment and
/// into the phone one is in poor taste and is not out of band, and this is
/// consulted by the transmit lockout, which must refuse only the latter.
///
/// Region 1's 160 m is the case that forced it. The plan runs 1.838–1.843 Digi
/// and 1.843–2.000 Phone, so an FT8 signal on the conventional 1.840 dial leaves
/// the digital segment at a 2950 Hz offset. Without the merge every offset above
/// that was refused, on a band edge that is not one: 550 Hz of the mode's normal
/// range, denied because two rows in a table happen to meet there.
///
/// A real GAP still stops it, which is the whole point. The UK's 60 m is eleven
/// discrete slivers, and 5.358–5.362 is not an allocation at all, so a signal
/// running past 5358.0 is not merged into anything and is correctly refused.
pub fn span_within_segment_in(lo: f64, hi: f64, region: Region) -> bool {
    let segs = segments_in(region);
    // `segments_in` is sorted by lower edge and non-overlapping (the loader
    // sorts it and complains about overlaps), so one pass finds the run.
    let mut i = 0;
    while i < segs.len() {
        let start = segs[i].lo;
        let mut end = segs[i].hi;
        // Extend while the next segment begins exactly where this run ends.
        // Sub-hertz slack because the edges come from MHz in `bandplan.json`.
        while i + 1 < segs.len() && (segs[i + 1].lo - end).abs() < 0.5 {
            i += 1;
            end = segs[i].hi;
        }
        if lo >= start && hi <= end {
            return true;
        }
        i += 1;
    }
    false
}

/// True when `lo..=hi` fits inside one sub-segment of the station's region.
pub fn span_within_segment(lo: f64, hi: f64) -> bool {
    span_within_segment_in(lo, hi, crate::region())
}

/// True if `hz` falls in a CW sub-segment.
pub fn is_cw_segment(hz: f64) -> bool {
    segment_kind_at(hz) == Some(SegmentKind::Cw)
}

/// True if `hz` falls in a digimode sub-segment.
pub fn is_digi_segment(hz: f64) -> bool {
    segment_kind_at(hz) == Some(SegmentKind::Digi)
}

/// FT8 dial frequencies (Hz); each mode occupies ~3 kHz of USB audio above it.
pub const FT8_DIALS: &[f64] = &[
    1_840_000.0,
    3_573_000.0,
    7_074_000.0,
    10_136_000.0,
    14_074_000.0,
    18_100_000.0,
    21_074_000.0,
    24_915_000.0,
    // 11 m: the 27.265 channel 26 digimode spot.
    27_265_000.0,
    28_074_000.0,
];
/// FT4 dial frequencies (Hz).
pub const FT4_DIALS: &[f64] = &[
    3_575_000.0,
    7_047_500.0,
    10_140_000.0,
    14_080_000.0,
    18_104_000.0,
    21_140_000.0,
    24_919_000.0,
    28_180_000.0,
];
/// FT2 dial frequencies (Hz), as shipped by the reference client (Decodium's
/// `models/FrequencyList.cpp`). HF only, matching the FT4 list above; the
/// reference also lists 50.316, 70.157, 144.177, 432.177 and 1296.177 MHz.
///
/// 80m and 12m land within a couple of hundred hertz of the JS8 dials — that
/// is the mode author's choice of segment, not a transcription slip.
pub const FT2_DIALS: &[f64] = &[
    1_843_000.0,
    3_578_000.0,
    5_360_000.0,
    7_062_000.0,
    10_144_000.0,
    14_084_000.0,
    18_108_000.0,
    21_144_000.0,
    24_923_000.0,
    28_184_000.0,
];
/// JS8Call dial frequencies (Hz); occupies ~3 kHz of USB audio above each.
pub const JS8_DIALS: &[f64] = &[
    1_842_000.0,
    3_578_000.0,
    7_078_000.0,
    10_130_000.0,
    14_078_000.0,
    18_104_000.0,
    21_078_000.0,
    24_922_000.0,
    // 11 m: the JS8 call channel on channel 25.
    27_245_000.0,
    28_078_000.0,
];
/// WSPR dial frequencies (Hz). The 200 Hz WSPR window sits ~1400–1600 Hz above
/// each dial; slow-CW (QRSS/MEPT) beacons cluster just below it (~1000–1400 Hz).
pub const WSPR_DIALS: &[f64] = &[
    1_836_600.0,
    3_568_600.0,
    7_038_600.0,
    10_138_700.0,
    14_095_600.0,
    18_104_600.0,
    21_094_600.0,
    24_924_600.0,
    // 11 m shares channel 25 with SSTV; listed because the dial is published,
    // while the hop bands deliberately omit the band (see `wspr_default_hop_bands`).
    27_255_000.0,
    28_124_600.0,
];

/// Analog-SSTV frequencies (Hz), with the note the picker shows and the regions
/// each belongs to. A picture occupies ~2.7 kHz of the sideband in use — above
/// the dial on 20 m and up, below it on 80 m and 40 m, where SSTV follows phone
/// practice and runs LSB (see [`crate::Mode::is_lower_sideband_at`]).
///
/// The secondaries matter more here than in most modes: a picture takes two
/// minutes, so a single frequency per band is occupied a lot of the time and
/// the convention is simply to move up.
///
/// 80 m and 40 m split by region and always have — 3.730 and 7.165 in Region 1,
/// 3.845 and 7.171 in the Americas and the Asia-Pacific. 3.845 is outside the
/// Region 1 allocation altogether, which is exactly the sort of entry the
/// region filter exists to keep out of a European operator's picker.
pub const SSTV_DIALS: &[(f64, &str, u8)] = &[
    (3_730_000.0, "", mask::R1),
    (3_735_000.0, "secondary", mask::R1),
    (3_845_000.0, "", mask::R23),
    (7_165_000.0, "", mask::R1),
    (7_171_000.0, "", mask::R23),
    (14_230_000.0, "", mask::ALL),
    (14_233_000.0, "secondary", mask::ALL),
    (21_340_000.0, "", mask::ALL),
    // 11 m — SSTV rides the digimode cluster above the CB channels. 27.700 is
    // the established calling frequency; 27.255 shares channel 25 with JS8,
    // 27.375 rides channel 37 and 27.710 is the top of the cluster.
    (27_255_000.0, "secondary", mask::ALL),
    (27_375_000.0, "secondary", mask::ALL),
    (27_700_000.0, "", mask::ALL),
    (27_710_000.0, "secondary", mask::ALL),
    (28_680_000.0, "", mask::ALL),
    (28_690_000.0, "secondary", mask::ALL),
];

/// The VHF/UHF image channels, where analog SSTV rides an FM carrier
/// ([`crate::Mode::SstvFm`]) rather than a sideband.
///
/// ⚠️ Sparse on purpose, like the repeater plan: only channels a published plan
/// actually names are here. The three Region 1 entries are the IARU R1
/// VHF/UHF bandplan's own — 50.510 "SSTV" and 144.500 "Image mode centre
/// (SSTV, Fax…)" in all-mode segments, and 433.400 "SSTV (FM/AFSK)", which
/// names the modulation outright. Region 2's plan designates no image channel
/// at all: the ARRL 2 m plan calls 145.50–145.80 "miscellaneous and
/// experimental modes", and 145.500 inside it is the frequency North American
/// FM SSTV has settled on in practice rather than one the plan appoints.
/// Region 3 names none, so none is offered.
pub const SSTV_FM_DIALS: &[(f64, &str, u8)] = &[
    (50_510_000.0, "", mask::R1),
    (144_500_000.0, "", mask::R1),
    (145_500_000.0, "common practice, not a plan entry", mask::R2),
    (433_400_000.0, "", mask::R1),
];

/// The analog-SSTV frequencies in use in `region`, ascending — [`SSTV_DIALS`]
/// with the region filter applied, for callers that want the frequencies rather
/// than the picker's annotated channels.
pub fn sstv_dials_in(region: Region) -> Vec<f64> {
    SSTV_DIALS.iter().filter(|&&(_, _, m)| region.in_mask(m)).map(|&(hz, _, _)| hz).collect()
}

/// RIFP centre frequencies (Hz): the calling frequency the draft names, plus a
/// spot in each segment below it where a wide channel fits (10 m FM, the 6 m
/// and 2 m all-modes parts). These are centres, not lower edges — the CPFSK
/// signal straddles the dial, ±12.5 kHz.
pub const RIFP_CALLING: &[f64] =
    &[29_600_000.0, 51_250_000.0, 144_700_000.0, crate::RIFP_CALLING_HZ];

/// True where the *automatic* / beacon digital modes live and the PSK/RTTY
/// skimmers must not run — their DSP would only produce garbage from these
/// signals. Covers FT8, FT4 and JS8 (dial → +3 kHz), plus the WSPR window and
/// the QRSS/MEPT beacons just below it (~1000–1700 Hz above the WSPR dial).
///
/// Region-independent, because these dial lists are: the WSJT modes ship one
/// frequency table for the whole world.
pub fn is_auto_digi(hz: f64) -> bool {
    FT8_DIALS.iter().any(|&f| (f - 100.0..=f + 3100.0).contains(&hz))
        || FT4_DIALS.iter().any(|&f| (f - 100.0..=f + 3100.0).contains(&hz))
        || FT2_DIALS.iter().any(|&f| (f - 100.0..=f + 3100.0).contains(&hz))
        || JS8_DIALS.iter().any(|&f| (f - 100.0..=f + 3100.0).contains(&hz))
        || WSPR_DIALS.iter().any(|&f| {
            // Dial reference, plus the QRSS + WSPR beacon window above it.
            (f - 100.0..=f + 400.0).contains(&hz) || (f + 1000.0..=f + 1700.0).contains(&hz)
        })
}

/// The narrow sub-bands where PSK31 activity clusters in Region 1. The PSK
/// skimmer runs only here, not across the whole digi segment.
pub const PSK_RANGES_R1: &[(f64, f64)] = &[
    (1_838_000.0, 1_840_000.0),   // 160m
    (3_580_000.0, 3_583_000.0),   // 80m
    (7_038_000.0, 7_042_000.0),   // 40m
    (10_139_000.0, 10_142_000.0), // 30m
    (14_070_000.0, 14_073_000.0), // 20m (below FT8 @ 14.074)
    (18_097_000.0, 18_100_000.0), // 17m (below FT8 @ 18.100)
    (21_070_000.0, 21_073_000.0), // 15m
    (24_920_000.0, 24_923_000.0), // 12m
    (27_498_000.0, 27_502_000.0), // 11m (PSK31 at 27.500, ±2 kHz like 10 m)
    (28_118_000.0, 28_122_000.0), // 10m
];

/// The same for Regions 2 and 3, which differ only on 40 m: the activity is
/// around 7.070, not 7.040.
pub const PSK_RANGES_R23: &[(f64, f64)] = &[
    (1_838_000.0, 1_840_000.0),   // 160m
    (3_580_000.0, 3_583_000.0),   // 80m
    (7_068_000.0, 7_072_000.0),   // 40m
    (10_139_000.0, 10_142_000.0), // 30m
    (14_070_000.0, 14_073_000.0), // 20m
    (18_097_000.0, 18_100_000.0), // 17m
    (21_070_000.0, 21_073_000.0), // 15m
    (24_920_000.0, 24_923_000.0), // 12m
    (27_498_000.0, 27_502_000.0), // 11m (PSK31 at 27.500)
    (28_118_000.0, 28_122_000.0), // 10m
];

/// The sub-bands where RTTY activity clusters in Region 1.
pub const RTTY_RANGES_R1: &[(f64, f64)] = &[
    (3_580_000.0, 3_600_000.0),   // 80m
    (7_040_000.0, 7_050_000.0),   // 40m
    (10_140_000.0, 10_150_000.0), // 30m
    (14_083_000.0, 14_099_000.0), // 20m (above FT4 @ 14.080)
    (18_101_000.0, 18_109_000.0), // 17m
    (21_080_000.0, 21_120_000.0), // 15m
    (24_921_000.0, 24_930_000.0), // 12m
    (27_498_000.0, 27_506_000.0), // 11m (RTTY at 27.500)
    (28_083_000.0, 28_120_000.0), // 10m
];

/// The same for Regions 2 and 3: 40 m RTTY sits from 7.075 up, around the
/// 7.080 calling frequency, rather than in Region 1's 7.040–7.050.
pub const RTTY_RANGES_R23: &[(f64, f64)] = &[
    (3_580_000.0, 3_600_000.0),   // 80m
    (7_075_000.0, 7_100_000.0),   // 40m
    (10_140_000.0, 10_150_000.0), // 30m
    (14_083_000.0, 14_099_000.0), // 20m
    (18_101_000.0, 18_109_000.0), // 17m
    (21_080_000.0, 21_120_000.0), // 15m
    (24_921_000.0, 24_930_000.0), // 12m
    (27_498_000.0, 27_506_000.0), // 11m (RTTY at 27.500)
    (28_083_000.0, 28_120_000.0), // 10m
];

/// The built-in PSK31 windows for `region`, before any `bandplan.json`.
pub(crate) fn default_psk_ranges_in(region: Region) -> &'static [(f64, f64)] {
    match region {
        Region::R1 => PSK_RANGES_R1,
        Region::R2 | Region::R3 => PSK_RANGES_R23,
    }
}

/// Where PSK31 clusters in `region`, from the station's [`crate::BandPlan`].
pub fn psk_ranges_in(region: Region) -> &'static [(f64, f64)] {
    crate::band_plan().region(region).psk_ranges()
}

/// Where PSK31 clusters in the station's configured region.
pub fn psk_ranges() -> &'static [(f64, f64)] {
    psk_ranges_in(crate::region())
}

/// The built-in RTTY windows for `region`, before any `bandplan.json`.
pub(crate) fn default_rtty_ranges_in(region: Region) -> &'static [(f64, f64)] {
    match region {
        Region::R1 => RTTY_RANGES_R1,
        Region::R2 | Region::R3 => RTTY_RANGES_R23,
    }
}

/// Where RTTY clusters in `region`, from the station's [`crate::BandPlan`].
pub fn rtty_ranges_in(region: Region) -> &'static [(f64, f64)] {
    crate::band_plan().region(region).rtty_ranges()
}

/// Where RTTY clusters in the station's configured region.
pub fn rtty_ranges() -> &'static [(f64, f64)] {
    rtty_ranges_in(crate::region())
}

/// FT8 DXpedition (Fox/Hound) dial frequencies (Hz).
///
/// A separate set from [`FT8_DIALS`] on purpose: a DXpedition running Fox mode
/// fills its whole 3 kHz window with hounds, so it is kept off the ordinary
/// calling frequency rather than swamping it. These are WSJT-X's own defaults,
/// which is what every station chasing the expedition will be tuned to.
pub const FT8_DXPED_DIALS: &[f64] = &[
    1_845_000.0,
    3_567_000.0,
    7_056_000.0,
    10_131_000.0,
    14_090_000.0,
    18_095_000.0,
    21_091_000.0,
    24_911_000.0,
    28_091_000.0,
];

/// FT8 dial frequencies above HF (Hz), tagged with the regions each belongs to.
/// 6 m has two: 50.313 is the calling frequency and 50.323 is where the DX goes
/// when it is busy, which during a sporadic-E opening is most of the time.
///
/// 4 m carries a region tag because the *band* does: 70 MHz is an amateur
/// allocation in Region 1 alone, so 70.174 is offered there and nowhere else.
/// The rest follow 144.174's pattern of x.174.
///
/// ## The microwave bands
///
/// 23 cm and 13 cm reach this table; 9 cm and 6 cm do not, because no FT8
/// convention has settled on them — a dial invented here would send an operator
/// to call on an empty frequency, which is worse than offering nothing and
/// letting them tune to where the activity actually is.
///
/// 13 cm has two, and which one a band button lands on has to follow the region:
/// the narrow-band segment of that band is at 2320 under the Region 1 plan and
/// at 2304 in the Americas, so 2320.174 and 2304.174 are both in real use.
/// Both are offered everywhere — a station with a 2304 transverter in Europe is
/// not a station with nothing to work — but the *unannotated* one, which is what
/// [`crate::Mode::Ft8`]'s band button tunes to, is the one its own region's plan
/// puts narrow-band modes on. Region 3 is given 2320 with less confidence than
/// the other two: its national plans put narrow-band 13 cm work in more than one
/// place, and an operator whose does otherwise picks 2304 from the same list.
///
/// 23 cm's 1296.500 is a secondary rather than a rival: 1296.174 is the calling
/// frequency, and where a national plan puts something else across it the
/// activity moves up.
pub const FT8_VHF_DIALS: &[(f64, &str, u8)] = &[
    (50_313_000.0, "", mask::ALL),
    (50_323_000.0, "", mask::ALL),
    (70_174_000.0, "", mask::R1),
    (144_174_000.0, "", mask::ALL),
    (432_174_000.0, "", mask::ALL),
    (1_296_174_000.0, "", mask::ALL),
    (1_296_500_000.0, "secondary", mask::ALL),
    (2_304_174_000.0, "", mask::R2),
    (2_304_174_000.0, "2304 segment", mask::R13),
    (2_320_174_000.0, "", mask::R13),
    (2_320_174_000.0, "2320 segment", mask::R2),
];

/// PSK31 dial frequencies (Hz), tagged with the regions each belongs to.
///
/// 40 m carries the region split that the rest of the table does not need:
/// 7.040 is where Region 1 activity sits — the bottom of the digimode segment
/// since the 2009 rebandplan, the older 7.035 having been left in the
/// narrow-band part — and 7.070 is the Region 2 and 3 convention.
pub const PSK_DIALS: &[(f64, &str, u8)] = &[
    (1_838_000.0, "", mask::ALL),
    (3_580_000.0, "", mask::ALL),
    (7_040_000.0, "", mask::R1),
    (7_070_000.0, "", mask::R23),
    (10_142_000.0, "", mask::ALL),
    (14_070_000.0, "", mask::ALL),
    (18_097_000.0, "", mask::ALL),
    (21_070_000.0, "", mask::ALL),
    (24_920_000.0, "", mask::ALL),
    (27_500_000.0, "", mask::ALL),
    (28_120_000.0, "", mask::ALL),
];

/// RTTY dial frequencies (Hz) — the DX calling spots, plus the 40 m split
/// between Region 1's 7.040 and the 7.080 the other two regions use.
pub const RTTY_DIALS: &[(f64, &str, u8)] = &[
    (3_580_000.0, "", mask::ALL),
    (3_590_000.0, "DX calling", mask::ALL),
    (7_040_000.0, "", mask::R1),
    (7_080_000.0, "", mask::R23),
    (10_142_000.0, "", mask::ALL),
    (14_080_000.0, "", mask::ALL),
    (14_083_000.0, "DX calling", mask::ALL),
    (18_105_000.0, "", mask::ALL),
    (21_080_000.0, "", mask::ALL),
    (24_925_000.0, "", mask::ALL),
    (27_500_000.0, "", mask::ALL),
    (28_080_000.0, "", mask::ALL),
];

/// FSQCall dial frequencies (Hz), as the mode's own documentation publishes
/// them. The signal sits in the audio passband above each.
pub const FSQ_DIALS: &[f64] = &[
    1_842_000.0,
    3_588_000.0,
    7_105_000.0,
    10_144_000.0,
    14_105_000.0,
    18_106_000.0,
    21_105_000.0,
    24_925_000.0,
    28_105_000.0,
];

/// APRS channels (Hz), tagged with the regions each belongs to.
///
/// The one mode here with no worldwide frequency at all: APRS is a single
/// shared channel, and which channel is a regional decision that predates any
/// attempt to harmonise it. A station tuned to the wrong one hears nothing and
/// is heard by nobody, so the region's own channel is what a band button lands
/// on and the neighbours are offered beside it with a note saying where they
/// are used.
///
/// - **144.800** — Region 1, everywhere: the IARU Region 1 channel.
/// - **144.390** — the Americas, and also the working channel across much of
///   South-East Asia, which is why Region 3 is offered it as well.
/// - **144.640** — Japan (and China), which sit on their own channel.
/// - **145.175** — Australia and New Zealand, the unannotated Region 3 entry.
/// - **432.500 / 445.925** — the 70 cm channels of Region 1 and Region 2.
///   There is no Region 3 UHF convention settled enough to put here, and
///   inventing one would send an operator to beacon on an empty frequency.
pub const APRS_DIALS: &[(f64, &str, u8)] = &[
    (144_390_000.0, "", mask::R2),
    (144_390_000.0, "South-East Asia", mask::R3),
    (144_390_000.0, "the Americas", mask::R1),
    (144_640_000.0, "Japan, China", mask::R3),
    (144_800_000.0, "", mask::R1),
    (144_800_000.0, "Region 1", mask::R23),
    (145_175_000.0, "", mask::R3),
    (145_175_000.0, "Australia, New Zealand", mask::R12),
    (432_500_000.0, "70 cm", mask::R1),
    (445_925_000.0, "70 cm", mask::R2),
];

/// The APRS channel for the station's configured region — the frequency an
/// APRS station is expected to be sitting on.
///
/// Every other digital mode leaves the dial where the operator put it, because
/// every other digital mode is worked across a segment. APRS is a channel: a
/// receiver 10 kHz away from it is not receiving APRS at all, so selecting the
/// mode is worth acting on.
#[must_use]
pub fn aprs_dial() -> f64 {
    aprs_dial_in(crate::region())
}

/// The same for `region`.
#[must_use]
pub fn aprs_dial_in(region: Region) -> f64 {
    APRS_DIALS
        .iter()
        .find(|&&(_, note, m)| note.is_empty() && region.in_mask(m))
        .map_or(144_800_000.0, |&(hz, _, _)| hz)
}

/// True when `hz` is one of the APRS channels — any region's, not just the
/// configured one. A traveller who has tuned Japan's 144.640 by hand is on an
/// APRS channel and must not be moved off it.
#[must_use]
pub fn is_aprs_channel(hz: f64) -> bool {
    APRS_DIALS.iter().any(|&(f, _, _)| (f - hz).abs() < 500.0)
}

/// One conventional operating frequency for a digital mode.
///
/// "Conventional" rather than "legal": these are the spots the mode's own
/// community has settled on, which is what decides whether anybody hears you.
/// The band edges are a separate matter and are enforced separately.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DigiChannel {
    /// Dial frequency in Hz. Every mode in this table sits on the dial with its
    /// signal in the audio passband — above it on USB, which is all of them bar
    /// analog SSTV on 80 m and 40 m, where the passband is below the dial.
    pub dial_hz: f64,
    /// What distinguishes it from the band's other entries — "DXpedition",
    /// "secondary", "DX calling". Empty for the plain calling frequency, which
    /// is the first entry in a band.
    pub note: &'static str,
    /// True for one of the operator's own [`DigiPreset`]s rather than a
    /// published convention.
    ///
    /// The distinction is not decoration. Only the operator's own entries can
    /// be deleted, and only a published convention is a frequency the rest of
    /// the world is listening on — so a rule that moves the dial by itself is
    /// entitled to pick one of those and not one of these.
    pub mine: bool,
}

impl DigiChannel {
    /// True when this frequency is not in a data or all-modes sub-segment of
    /// the station's configured region's band plan.
    pub fn outside_data_segment(&self, mode: crate::Mode) -> bool {
        self.outside_data_segment_in(mode, crate::region())
    }

    /// True when this frequency is not in a data or all-modes sub-segment of
    /// `region`'s band plan.
    ///
    /// Several genuinely conventional frequencies are not: the WSJT-X
    /// DXpedition set is a *global* convention chosen around the Region 2 band
    /// plan, and three of its entries land in Region 1's CW and phone segments.
    ///
    /// Worth showing rather than hiding: an operator should know before they
    /// key that the frequency the DX is working on is one their own band plan
    /// does not put data in.
    ///
    /// The wideband modes are exempt — analog SSTV is a phone emission and
    /// RIFP's CPFSK channel is 25 kHz wide, so both belong in the SSB and
    /// all-mode parts of the band by design and flagging them would be noise.
    pub fn outside_data_segment_in(&self, mode: crate::Mode, region: Region) -> bool {
        if wideband_by_design(mode) {
            return false;
        }
        // Above HF there are no segments in the table to check against.
        match segment_kind_at_in(self.dial_hz, region) {
            Some(kind) => !matches!(kind, SegmentKind::Digi | SegmentKind::All),
            None => false,
        }
    }
}

/// True for the modes whose signal is too wide for a narrow-data sub-segment,
/// and which therefore belong in the phone / all-mode parts of the band.
fn wideband_by_design(mode: crate::Mode) -> bool {
    // VHF packet is a 25 kHz FM channel and belongs nowhere near a narrow-data
    // sub-segment. HF packet is the opposite — 300 baud in a few hundred hertz
    // is narrow data by any measure — so it is deliberately not listed.
    matches!(
        mode,
        crate::Mode::Sstv
            | crate::Mode::SstvFm
            | crate::Mode::Rifp
            | crate::Mode::Packet
            | crate::Mode::Aprs
    )
}

/// The conventional dial frequencies for `mode` in the station's configured
/// region, ascending.
pub fn digi_channels(mode: crate::Mode) -> Vec<DigiChannel> {
    digi_channels_for(mode, crate::region())
}

/// One frequency the operator added to a mode's list themselves.
///
/// The shipped tables are what the mode's community settled on globally; this
/// is the local net that meets on 3.585 every Tuesday, the club's own SSTV
/// frequency, the corner of 30 m a group actually uses. sdroxide cannot know
/// those and should not pretend to — but having tuned one once, an operator
/// should not have to remember the number again (issue #268).
///
/// Kept beside the built-in tables rather than replacing them: both appear in
/// the same picker, and the operator's own entries are marked so it is always
/// clear which is which.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DigiPreset {
    /// The mode it belongs to. A frequency is only ever offered for the mode it
    /// was saved under — 14.100 means something quite different in CW and in
    /// PSK31.
    pub mode: crate::Mode,
    /// Dial frequency in Hz, on the same terms as [`DigiChannel::dial_hz`].
    pub dial_hz: f64,
    /// What the operator called it. Shown beside the frequency in the picker,
    /// which is the whole reason to save one rather than type it again.
    #[serde(default)]
    pub note: String,
}

/// The operator's own presets, once they have been installed; empty until then.
static INSTALLED_PRESETS: std::sync::RwLock<Option<&'static [DigiPreset]>> =
    std::sync::RwLock::new(None);

/// The frequencies the operator has added to the mode tables themselves.
///
/// Leaked on install for the same reason the band plan is: the lookups hand out
/// `&'static` data, installs happen at startup and when the list is edited, and
/// neither is in a loop.
#[must_use]
pub fn digi_presets() -> &'static [DigiPreset] {
    match INSTALLED_PRESETS.read() {
        Ok(g) => g.unwrap_or(&[]),
        // A poisoned lock means a writer panicked mid-install. No presets is a
        // safe answer, and far better than joining the panic from a draw loop.
        Err(_) => &[],
    }
}

/// Adopt `presets` as the operator's own frequency list.
///
/// Called by the engine at startup and whenever the list is edited, and by a
/// remote client when the station announces its own — the presets belong to the
/// station, like the band plan, so every screen attached to a radio offers the
/// same ones.
pub fn set_digi_presets(presets: Vec<DigiPreset>) {
    let leaked: &'static [DigiPreset] = Box::leak(presets.into_boxed_slice());
    if let Ok(mut g) = INSTALLED_PRESETS.write() {
        *g = Some(leaked);
    }
}

/// The conventional dial frequencies for `mode` in `region`, ascending.
///
/// Empty for a mode with no convention of its own — CW, SSB, and the modes
/// whose operating frequency is a property of what they are pointed at rather
/// than of the mode (RF Paint, RADE) — unless the operator has saved one of
/// their own, which is exactly how a mode with no global convention gets a
/// list.
pub fn digi_channels_for(mode: crate::Mode, region: Region) -> Vec<DigiChannel> {
    use crate::Mode;
    let plain = |v: &[f64]| -> Vec<DigiChannel> {
        v.iter().map(|&dial_hz| DigiChannel { dial_hz, note: "", mine: false }).collect()
    };
    // The region-tagged tables: an entry only appears where its convention
    // actually holds, so a Region 1 operator is never offered 7.070 for PSK31.
    let tagged = |v: &[(f64, &'static str, u8)]| -> Vec<DigiChannel> {
        v.iter()
            .filter(|&&(_, _, m)| region.in_mask(m))
            .map(|&(dial_hz, note, _)| DigiChannel { dial_hz, note, mine: false })
            .collect()
    };

    let mut v = match mode {
        Mode::Js8 => plain(JS8_DIALS),
        Mode::Wspr => plain(WSPR_DIALS),
        Mode::Ft8 => {
            let mut v = plain(FT8_DIALS);
            v.extend(FT8_DXPED_DIALS.iter().map(|&dial_hz| DigiChannel {
                dial_hz,
                note: "DXpedition (Fox/Hound)",
                mine: false,
            }));
            v.extend(tagged(FT8_VHF_DIALS));
            v
        }
        Mode::Ft4 => plain(FT4_DIALS),
        Mode::Ft2 => plain(FT2_DIALS),
        Mode::Psk => tagged(PSK_DIALS),
        Mode::Rtty => tagged(RTTY_DIALS),
        Mode::Fsq => plain(FSQ_DIALS),
        Mode::Sstv => tagged(SSTV_DIALS),
        Mode::SstvFm => tagged(SSTV_FM_DIALS),
        Mode::Rifp => plain(RIFP_CALLING),
        Mode::Aprs => tagged(APRS_DIALS),
        _ => Vec::new(),
    };
    // The operator's own, in every region: these are their frequencies, not a
    // continent's convention, and a station that travels takes them along.
    // A preset on a frequency the tables already carry is dropped rather than
    // listed twice — the built-in entry says the same thing and says what it is.
    for p in digi_presets().iter().filter(|p| p.mode == mode) {
        if v.iter().any(|c| (c.dial_hz - p.dial_hz).abs() < 1.0) {
            continue;
        }
        v.push(DigiChannel { dial_hz: p.dial_hz, note: p.note.as_str(), mine: true });
    }
    v.sort_by(|a, b| a.dial_hz.total_cmp(&b.dial_hz));
    v
}

/// The conventional dial frequencies for `mode` that fall inside `band`, in the
/// station's configured region.
///
/// The caller shows a picker when this returns more than one — a band with a
/// single convention has nothing to choose between.
pub fn digi_channels_in(mode: crate::Mode, band: crate::Band) -> Vec<DigiChannel> {
    digi_channels_in_region(mode, band, crate::region())
}

/// The conventional dial frequencies for `mode` inside `band`, in `region`.
pub fn digi_channels_in_region(
    mode: crate::Mode,
    band: crate::Band,
    region: Region,
) -> Vec<DigiChannel> {
    digi_channels_for(mode, region)
        .into_iter()
        .filter(|c| crate::Band::containing_in(c.dial_hz, region) == band)
        .collect()
}

/// True in a PSK31 calling sub-band (and clear of the automatic modes).
pub fn is_psk_segment(hz: f64) -> bool {
    is_psk_segment_in(hz, crate::region())
}

/// The same for `region`.
pub fn is_psk_segment_in(hz: f64, region: Region) -> bool {
    !is_auto_digi(hz) && psk_ranges_in(region).iter().any(|&(lo, hi)| (lo..=hi).contains(&hz))
}

/// True in an RTTY sub-band (and clear of the automatic modes).
pub fn is_rtty_segment(hz: f64) -> bool {
    is_rtty_segment_in(hz, crate::region())
}

/// The same for `region`.
pub fn is_rtty_segment_in(hz: f64, region: Region) -> bool {
    !is_auto_digi(hz) && rtty_ranges_in(region).iter().any(|&(lo, hi)| (lo..=hi).contains(&hz))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Band;

    /// Issue #268: a frequency the operator saved appears in the mode's list,
    /// marked as theirs, and is offered whatever region the station is in — it
    /// is their net, not a continent's convention.
    ///
    /// The install is process-wide, so this test owns the list for its whole
    /// length and puts it back; nothing else in this module reads it.
    #[test]
    fn a_saved_frequency_joins_the_modes_own_list() {
        set_digi_presets(vec![
            DigiPreset {
                mode: crate::Mode::Psk,
                dial_hz: 3_585_000.0,
                note: "Tuesday net".to_string(),
            },
            // A duplicate of a published entry is dropped rather than listed
            // twice: 14.070 is already PSK31's calling frequency.
            DigiPreset { mode: crate::Mode::Psk, dial_hz: 14_070_000.0, note: String::new() },
            // A mode the shipped tables have nothing at all for still gets a
            // list, which is the only way an operator can ever save one.
            DigiPreset { mode: crate::Mode::Olivia, dial_hz: 14_106_500.0, note: String::new() },
        ]);

        let psk = digi_channels_for(crate::Mode::Psk, Region::R1);
        let mine: Vec<_> = psk.iter().filter(|c| c.mine).collect();
        assert_eq!(mine.len(), 1, "the duplicate of a published entry should have been dropped");
        assert_eq!(mine[0].dial_hz, 3_585_000.0);
        assert_eq!(mine[0].note, "Tuesday net");
        assert_eq!(
            psk.iter().filter(|c| (c.dial_hz - 14_070_000.0).abs() < 1.0).count(),
            1,
            "14.070 must appear once, as the published convention"
        );
        // Ascending, presets included — the picker groups by band and relies on it.
        assert!(psk.windows(2).all(|w| w[0].dial_hz <= w[1].dial_hz));
        // Their frequencies, not a region's: the same entry in every region.
        for r in [Region::R1, Region::R2, Region::R3] {
            assert!(
                digi_channels_for(crate::Mode::Psk, r).iter().any(|c| c.mine),
                "{r:?} should still offer the operator's own"
            );
        }
        let olivia = digi_channels_for(crate::Mode::Olivia, Region::R1);
        assert_eq!(olivia.len(), 1);
        assert!(olivia[0].mine);
        assert_eq!(digi_channels_in(crate::Mode::Olivia, Band::M20).len(), 1);

        set_digi_presets(Vec::new());
        assert!(digi_channels_for(crate::Mode::Olivia, Region::R1).is_empty());
        assert!(!digi_channels_for(crate::Mode::Psk, Region::R1).iter().any(|c| c.mine));
    }

    /// The one mode with no worldwide frequency: selecting APRS tunes to the
    /// region's own channel, and getting that wrong means a receiver that
    /// hears nothing and a transmitter nobody hears.
    #[test]
    fn each_region_gets_its_own_aprs_channel() {
        assert_eq!(aprs_dial_in(Region::R1), 144_800_000.0);
        assert_eq!(aprs_dial_in(Region::R2), 144_390_000.0);
        assert_eq!(aprs_dial_in(Region::R3), 145_175_000.0);
    }

    /// A traveller who has tuned another region's channel by hand is on an
    /// APRS channel and must not be moved off it — which is what
    /// `is_aprs_channel` is for, so it has to accept every region's.
    #[test]
    fn every_regions_channel_counts_as_an_aprs_channel() {
        for &(hz, _, _) in APRS_DIALS {
            assert!(is_aprs_channel(hz), "{hz} is in the table but not recognised");
        }
        assert!(!is_aprs_channel(145_500_000.0), "the 2 m calling frequency is not APRS");
    }

    /// Each region's picker offers exactly one unannotated entry — the one a
    /// band button lands on — and the neighbours with a note saying where they
    /// are used.
    #[test]
    fn every_region_has_exactly_one_default_aprs_channel() {
        for region in Region::ALL {
            let plain = APRS_DIALS
                .iter()
                .filter(|&&(_, note, m)| note.is_empty() && region.in_mask(m))
                .count();
            assert_eq!(plain, 1, "{} has {plain} unannotated APRS channels", region.label());
            let offered = digi_channels_for(crate::Mode::Aprs, region);
            assert!(offered.len() > 1, "{} is offered no alternatives", region.label());
        }
    }

    #[test]
    fn classification() {
        assert_eq!(segment_kind_at_in(14_030_000.0, Region::R1), Some(Cw));
        assert_eq!(segment_kind_at_in(14_074_000.0, Region::R1), Some(Digi)); // FT8
        assert_eq!(segment_kind_at_in(14_200_000.0, Region::R1), Some(Phone));
        assert_eq!(segment_kind_at_in(7_020_000.0, Region::R1), Some(Cw));
        assert_eq!(segment_kind_at_in(7_074_000.0, Region::R1), Some(Digi));
        // Outside any HF ham segment.
        assert_eq!(segment_kind_at_in(15_000_000.0, Region::R1), None);
    }

    /// An emission has width, so the transmit lockout asks about a span rather
    /// than a point. Two things have to hold at once, and they pull opposite
    /// ways: a signal must be free to cross where the band plan merely changes
    /// its mind about usage, and must be stopped where the allocation stops.
    #[test]
    fn a_span_may_cross_a_usage_boundary_but_not_a_gap() {
        // Region 1's 160 m: Digi ends and Phone begins at 1.843, and nothing
        // about that is a band edge. FT8 on the conventional 1.840 dial reaches
        // past it at any offset over 2950 Hz, and refusing that would deny
        // 550 Hz of the mode's normal range on a boundary that is not one.
        let dial = 1_840_000.0;
        let ft8 = 50.0;
        for off in [200.0, 2950.0, 3000.0, 3500.0] {
            assert!(
                span_within_segment_in(dial + off, dial + off + ft8, Region::R1),
                "160 m FT8 at {off} Hz should be allowed; it is inside the band throughout"
            );
        }
        // Above the band itself is still refused. 2.000 is where Region 1's
        // 160 m ends, and nothing is merged past it.
        assert!(!span_within_segment_in(1_999_990.0, 2_000_050.0, Region::R1));

        // The built-in Region 1 60 m is one all-modes block, so a signal
        // anywhere inside it is fine. This is what a UK operator narrows by
        // hand, and the narrowing is what the lockout then enforces; the
        // built-in tables are the IARU allocation, not any one licence.
        assert!(span_within_segment_in(5_357_950.0, 5_358_000.0, Region::R1));

        // A gap is not merged away, which is the half that matters. Region 1
        // has nothing between 2.000 and 3.500, so a span reaching into it is
        // refused however close the neighbours are.
        assert!(!span_within_segment_in(3_499_950.0, 3_500_050.0, Region::R1));

        // The upper bound is inclusive: a transmission finishing exactly on a
        // segment's top edge is permitted. Refusing it would fail the operator
        // who did the arithmetic correctly.
        assert!(span_within_segment_in(1_842_000.0, 1_843_000.0, Region::R1));
    }

    /// The places the three plans genuinely disagree — the point of the
    /// setting, so spelled out rather than left to the tables to imply.
    #[test]
    fn the_regions_disagree_where_they_should() {
        // 1.805: below Region 1's band entirely, digimodes in Region 2, CW in
        // Region 3.
        assert_eq!(segment_kind_at_in(1_805_000.0, Region::R1), None);
        assert_eq!(segment_kind_at_in(1_805_000.0, Region::R2), Some(Digi));
        assert_eq!(segment_kind_at_in(1_805_000.0, Region::R3), Some(Cw));
        // 7.037: data in Regions 2 and 3, still CW in Region 1.
        assert_eq!(segment_kind_at_in(7_037_000.0, Region::R1), Some(Cw));
        assert_eq!(segment_kind_at_in(7_037_000.0, Region::R2), Some(Digi));
        assert_eq!(segment_kind_at_in(7_037_000.0, Region::R3), Some(Digi));
        // 7.250: Region 2 only.
        assert_eq!(segment_kind_at_in(7_250_000.0, Region::R1), None);
        assert_eq!(segment_kind_at_in(7_250_000.0, Region::R2), Some(All));
        assert_eq!(segment_kind_at_in(7_250_000.0, Region::R3), None);
        // 3.850: Region 1 stops at 3.800.
        assert_eq!(segment_kind_at_in(3_850_000.0, Region::R1), None);
        assert_eq!(segment_kind_at_in(3_850_000.0, Region::R2), Some(All));
        assert_eq!(segment_kind_at_in(3_850_000.0, Region::R3), Some(All));
        // The top of 20 m is a phone sub-band in Region 1 and all-modes in the
        // other two — which is what keeps an ordinary data frequency there from
        // being flagged as out of plan.
        assert_eq!(segment_kind_at_in(14_200_000.0, Region::R2), Some(All));
        assert_eq!(segment_kind_at_in(14_200_000.0, Region::R3), Some(All));
    }

    /// 20 m, where all three regions agree — so this one is about the skimmer
    /// windows dodging the automatic modes rather than about the region, and it
    /// is asserted for all three to say so.
    #[test]
    fn psk_rtty_subbands() {
        for r in Region::ALL {
            // 20m PSK area, clear of FT8 (14.074).
            assert!(is_psk_segment_in(14_072_000.0, r), "{r:?}");
            assert!(!is_psk_segment_in(14_074_000.0, r)); // FT8
            assert!(!is_psk_segment_in(14_090_000.0, r)); // RTTY area, not PSK
            assert!(!is_psk_segment_in(14_200_000.0, r)); // phone
            // 20m RTTY area, clear of FT4 (14.080) and WSPR/QRSS (around 14.097).
            assert!(is_rtty_segment_in(14_090_000.0, r), "{r:?}");
            assert!(!is_rtty_segment_in(14_081_000.0, r)); // FT4
            assert!(!is_rtty_segment_in(14_095_600.0, r)); // WSPR dial
            assert!(!is_rtty_segment_in(14_097_000.0, r)); // WSPR window (dial + 1400)
            assert!(!is_rtty_segment_in(14_096_800.0, r)); // QRSS window (dial + 1200)
            assert!(!is_rtty_segment_in(14_072_000.0, r)); // PSK area, not RTTY
        }
    }

    /// 40 m is where the skimmers have to move with the region: pointing the
    /// PSK skimmer at 7.040 in the Americas would skim an empty stretch of
    /// band and miss the actual activity 30 kHz up.
    #[test]
    fn the_skimmer_windows_follow_the_region_on_40m() {
        let in_range =
            |ranges: &[(f64, f64)], hz: f64| ranges.iter().any(|&(lo, hi)| (lo..=hi).contains(&hz));
        assert!(in_range(psk_ranges_in(Region::R1), 7_040_000.0));
        assert!(!in_range(psk_ranges_in(Region::R1), 7_070_000.0));
        for r in [Region::R2, Region::R3] {
            assert!(in_range(psk_ranges_in(r), 7_070_000.0), "{r:?}");
            assert!(!in_range(psk_ranges_in(r), 7_040_000.0), "{r:?}");
            assert!(in_range(rtty_ranges_in(r), 7_080_000.0), "{r:?}");
        }
        assert!(in_range(rtty_ranges_in(Region::R1), 7_045_000.0));
        assert!(!in_range(rtty_ranges_in(Region::R1), 7_080_000.0));
    }

    #[test]
    fn wspr_and_qrss_excluded() {
        // The WSPR window (dial + 1400–1600) and the QRSS beacons just below it
        // are excluded from PSK/RTTY skimming on every band.
        for &dial in WSPR_DIALS {
            assert!(is_auto_digi(dial + 1500.0), "WSPR window at {dial}");
            assert!(is_auto_digi(dial + 1200.0), "QRSS window at {dial}");
        }
    }

    /// Every conventional frequency has to be inside an amateur band *of the
    /// region it is offered in*, and the out-of-plan flag has to agree with
    /// that region's segment table.
    ///
    /// There is deliberately no assertion that the frequencies sit in a data
    /// segment. Several genuinely do not: the WSJT-X DXpedition set and the
    /// FSQCall set are both global conventions built around the Region 2 band
    /// plan, and Region 1 puts CW or phone where they land. Pretending
    /// otherwise would mean either dropping frequencies people really use or
    /// asserting something false. What must hold is that every such frequency
    /// is *flagged*, so the picker can say so instead of recommending it
    /// silently.
    #[test]
    fn every_conventional_frequency_is_in_a_band_and_flagged_honestly() {
        use crate::Mode;
        for region in Region::ALL {
            for mode in Mode::ALL {
                for c in digi_channels_for(mode, region) {
                    let band = Band::containing_in(c.dial_hz, region);
                    assert_ne!(
                        band,
                        Band::Gen,
                        "{mode:?} {} is outside every band in {region:?}",
                        c.dial_hz
                    );
                    let flagged = c.outside_data_segment_in(mode, region);
                    match segment_kind_at_in(c.dial_hz, region) {
                        // Above HF there are no sub-segments to check against.
                        None => assert!(!flagged, "{mode:?} {} flagged above HF", c.dial_hz),
                        Some(kind) => assert_eq!(
                            flagged,
                            !wideband_by_design(mode) && !matches!(kind, Digi | All),
                            "{mode:?} {} is in a {kind:?} segment and flagged {flagged}",
                            c.dial_hz
                        ),
                    }
                }
            }
        }
    }

    /// Exactly which frequencies Region 1 does not put narrow data on.
    ///
    /// Spelled out rather than counted, so adding a frequency that lands in the
    /// CW or phone part of a Region 1 band shows up here as a decision to make
    /// rather than slipping into the picker unnoticed.
    #[test]
    fn the_region_1_mismatches_are_exactly_these() {
        use crate::Mode;
        let flagged = |mode: Mode| -> Vec<f64> {
            digi_channels_for(mode, Region::R1)
                .into_iter()
                .filter(|c| c.outside_data_segment_in(mode, Region::R1))
                .map(|c| c.dial_hz)
                .collect()
        };
        // The DXpedition set: 1.845 lands in the Region 1 phone segment, 3.567
        // and 24.911 in CW ones.
        assert_eq!(flagged(Mode::Ft8), vec![1_845_000.0, 3_567_000.0, 24_911_000.0]);
        // Two of FSQCall's published frequencies sit above the Region 1 data
        // segment on their band; the rest happen to fall inside one.
        assert_eq!(flagged(Mode::Fsq), vec![7_105_000.0, 14_105_000.0]);
        // FT2's 160 m dial is 1.843, which is exactly where the Region 1 data
        // segment ends and phone begins — so the whole 2.5 kHz of it lands in
        // the phone segment. Flagged on purpose: the mode's frequency list
        // comes from its author, and an operator in Region 1 needs to see that
        // this one is not usable as published.
        assert_eq!(flagged(Mode::Ft2), vec![1_843_000.0]);
        // The everyday modes are clean.
        for m in [Mode::Ft4, Mode::Psk, Mode::Rtty] {
            assert!(flagged(m).is_empty(), "{m:?}: {:?}", flagged(m));
        }
        // The wideband modes are never flagged: they belong where they are.
        for m in [Mode::Sstv, Mode::Rifp] {
            assert!(flagged(m).is_empty(), "{m:?} must not be flagged");
        }
        // ...and the ordinary FT8 calling frequencies never are either.
        for &f in FT8_DIALS {
            let c = DigiChannel { dial_hz: f, note: "", mine: false };
            assert!(!c.outside_data_segment_in(Mode::Ft8, Region::R1));
        }
    }

    /// The out-of-plan flag has to be regional, not a fixed list: most of what
    /// Region 1 flags is flagged only because it was chosen around a band plan
    /// that hands the top of each band to all modes.
    #[test]
    fn what_region_1_flags_is_mostly_in_plan_elsewhere() {
        use crate::Mode;
        let flagged = |hz: f64, mode, region| {
            DigiChannel { dial_hz: hz, note: "", mine: false }.outside_data_segment_in(mode, region)
        };
        // FT8's 160 m DXpedition frequency, FSQCall's 40 m and 20 m, and FT2's
        // 160 m dial all land in Region 1 phone segments and in Region 2 and 3
        // all-modes ones.
        for (hz, mode) in [
            (1_845_000.0, Mode::Ft8),
            (7_105_000.0, Mode::Fsq),
            (14_105_000.0, Mode::Fsq),
            (1_843_000.0, Mode::Ft2),
        ] {
            assert!(flagged(hz, mode, Region::R1), "{hz} should be flagged in R1");
            for r in [Region::R2, Region::R3] {
                assert!(!flagged(hz, mode, r), "{hz} should not be flagged in {r:?}");
            }
        }
        // The two that sit in a CW sub-band are flagged in every region: every
        // plan puts CW there, and moving to the Americas does not change that.
        for hz in [3_567_000.0, 24_911_000.0] {
            for r in Region::ALL {
                assert!(flagged(hz, Mode::Ft8, r), "{hz} should be flagged in {r:?}");
            }
        }
    }

    /// The picker only appears where there is a choice, so the modes that are
    /// meant to offer one have to actually have several in a band, and no mode
    /// may list the same frequency twice.
    #[test]
    fn bands_with_a_choice_have_one_and_the_rest_do_not() {
        use crate::Mode;
        // FT8 on 20 m: the calling frequency and the DXpedition one.
        let ft8_20 = digi_channels_in_region(Mode::Ft8, Band::M20, Region::R1);
        assert_eq!(ft8_20.len(), 2, "{ft8_20:?}");
        assert_eq!(ft8_20[0].dial_hz, 14_074_000.0);
        assert!(ft8_20[0].note.is_empty(), "the calling frequency leads and is unannotated");
        assert!(ft8_20[1].note.contains("DXpedition"));
        // 6 m has two FT8 frequencies; 2 m has one, so it shows no picker.
        assert_eq!(digi_channels_in_region(Mode::Ft8, Band::M6, Region::R1).len(), 2);
        assert_eq!(digi_channels_in_region(Mode::Ft8, Band::M2, Region::R1).len(), 1);
        // 4 m has one, in the one region that has the band at all.
        let ft8_4 = digi_channels_in_region(Mode::Ft8, Band::M4, Region::R1);
        assert_eq!(ft8_4.len(), 1, "{ft8_4:?}");
        assert_eq!(ft8_4[0].dial_hz, 70_174_000.0);
        for r in [Region::R2, Region::R3] {
            assert!(digi_channels_in_region(Mode::Ft8, Band::M4, r).is_empty(), "{r:?}");
            assert!(
                !digi_channels_for(Mode::Ft8, r).iter().any(|c| c.dial_hz == 70_174_000.0),
                "{r:?} was offered 70.174, which is not an amateur band there"
            );
        }
        // FT4 and FT2 have a single convention everywhere, in every region.
        for region in Region::ALL {
            for b in Band::ALL {
                assert!(digi_channels_in_region(Mode::Ft4, b, region).len() <= 1, "FT4 {b:?}");
                assert!(digi_channels_in_region(Mode::Ft2, b, region).len() <= 1, "FT2 {b:?}");
            }
        }
        // 40 m PSK used to carry the region split as a two-entry picker. Now
        // that the station says where it is, each region is offered only its
        // own frequency and the picker stays out of the way.
        for region in Region::ALL {
            let psk40 = digi_channels_in_region(Mode::Psk, Band::M40, region);
            assert_eq!(psk40.len(), 1, "{region:?}: {psk40:?}");
        }
        assert_eq!(
            digi_channels_in_region(Mode::Psk, Band::M40, Region::R1)[0].dial_hz,
            7_040_000.0
        );
        assert_eq!(
            digi_channels_in_region(Mode::Psk, Band::M40, Region::R2)[0].dial_hz,
            7_070_000.0
        );
        // SSTV on 20 m: the calling frequency and the one to move up to.
        assert_eq!(digi_channels_in_region(Mode::Sstv, Band::M20, Region::R1).len(), 2);
        // Modes with no convention of their own offer nothing rather than
        // something invented.
        for m in [Mode::Usb, Mode::Cw, Mode::Rade, Mode::RfPaint, Mode::Hell] {
            assert!(digi_channels_for(m, Region::R1).is_empty(), "{m:?}");
        }
        // No duplicates, and ascending within every mode and region.
        for region in Region::ALL {
            for m in Mode::ALL {
                let v = digi_channels_for(m, region);
                for w in v.windows(2) {
                    assert!(
                        w[0].dial_hz < w[1].dial_hz,
                        "{m:?} in {region:?}: {:?} then {:?}",
                        w[0],
                        w[1]
                    );
                }
            }
        }
    }

    /// The microwave FT8 frequencies, and the one place they split by region:
    /// 13 cm narrow-band work sits at 2320 under the Region 1 plan and at 2304
    /// in the Americas, and a band button has to land on the operator's own.
    #[test]
    fn the_microwave_ft8_frequencies_follow_the_region_on_13cm() {
        use crate::Mode;
        let dials = |band, region| -> Vec<(f64, &'static str)> {
            digi_channels_in_region(Mode::Ft8, band, region)
                .into_iter()
                .map(|c| (c.dial_hz, c.note))
                .collect()
        };
        for r in Region::ALL {
            // 23 cm: the calling frequency, then the one activity moves up to.
            assert_eq!(
                dials(Band::Cm23, r),
                vec![(1_296_174_000.0, ""), (1_296_500_000.0, "secondary")],
                "{r:?}"
            );
            // 13 cm: both frequencies everywhere, exactly one of them plain —
            // the plain one is what the band button uses, so there must be
            // exactly one or the choice would be made by sort order.
            let d = dials(Band::Cm13, r);
            assert_eq!(d.len(), 2, "{r:?}: {d:?}");
            assert_eq!(d.iter().filter(|(_, n)| n.is_empty()).count(), 1, "{r:?}: {d:?}");
        }
        let plain = |region| -> f64 {
            digi_channels_in_region(Mode::Ft8, Band::Cm13, region)
                .into_iter()
                .find(|c| c.note.is_empty())
                .expect("13 cm has a plain calling frequency in every region")
                .dial_hz
        };
        assert_eq!(plain(Region::R2), 2_304_174_000.0);
        assert_eq!(plain(Region::R1), 2_320_174_000.0);
        assert_eq!(plain(Region::R3), 2_320_174_000.0);
        // The bands with no convention are offered none rather than an invented
        // one — 9 cm and 6 cm have no FT8 frequency anybody has settled on, and
        // 1.25 m and 33 cm none in this table.
        for r in Region::ALL {
            for b in [Band::Cm9, Band::Cm6, Band::M125, Band::Cm33] {
                assert!(digi_channels_in_region(Mode::Ft8, b, r).is_empty(), "{b:?} in {r:?}");
            }
        }
    }

    /// SSTV's 80 m and 40 m calling frequencies are region conventions, and
    /// 3.845 is outside the Region 1 allocation altogether — offering it to a
    /// European operator would be offering an out-of-band transmission.
    #[test]
    fn sstv_offers_each_region_its_own_calling_frequency() {
        use crate::Mode;
        let dials = |region| -> Vec<f64> {
            digi_channels_for(Mode::Sstv, region).into_iter().map(|c| c.dial_hz).collect()
        };
        assert!(dials(Region::R1).contains(&3_730_000.0));
        assert!(!dials(Region::R1).contains(&3_845_000.0));
        assert!(dials(Region::R2).contains(&3_845_000.0));
        assert!(!dials(Region::R2).contains(&3_730_000.0));
        assert!(dials(Region::R1).contains(&7_165_000.0));
        assert!(dials(Region::R3).contains(&7_171_000.0));
    }

    #[test]
    fn segments_sorted_and_non_overlapping() {
        for region in Region::ALL {
            for w in segments_in(region).windows(2) {
                assert!(w[0].hi <= w[1].lo, "{region:?} overlap: {:?} then {:?}", w[0], w[1]);
            }
        }
    }

    /// A sub-segment that runs past its own band's edge would colour the
    /// waterfall outside the allocation and tell the skimmers to work there.
    #[test]
    fn every_segment_is_inside_a_band_of_its_own_region() {
        for region in Region::ALL {
            for s in segments_in(region) {
                let band = Band::containing_in(s.lo, region);
                assert_ne!(band, Band::Gen, "{region:?}: {s:?} starts outside every band");
                let (_, hi) = band.edges_in(region).expect("a named band has edges");
                assert!(s.hi <= hi, "{region:?}: {s:?} runs past {band:?}'s edge at {hi}");
            }
        }
    }
}

#[cfg(test)]
mod js8_tests {
    use super::*;

    #[test]
    fn js8_has_conventional_channels() {
        let ch = digi_channels_for(crate::Mode::Js8, Region::R1);
        assert_eq!(ch.len(), JS8_DIALS.len());
        assert!(ch.iter().any(|c| c.dial_hz == 14_078_000.0), "20 m JS8 missing");
    }

    #[test]
    fn the_js8_subbands_are_off_limits_to_the_skimmers() {
        // Pre-existing gap: the PSK and RTTY skimmers were running across the
        // JS8 sub-bands, where their DSP can only produce garbage.
        for &dial in JS8_DIALS {
            assert!(is_auto_digi(dial + 1500.0), "{dial} + 1500 Hz should be off limits");
        }
    }
}
