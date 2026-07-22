//! Ham-band sub-segment data (CW / digimode / phone / beacon ranges), shared so
//! the engine can gate skimmers by segment and the UI can draw the band plan.
//!
//! IARU Region 1 HF ranges, mirrored from the UI band-plan overlay. Frequencies
//! are absolute Hz. Only HF amateur bands are covered; a frequency outside every
//! listed segment returns `None`.

use serde::{Deserialize, Serialize};

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
}

/// One band sub-segment: `[lo, hi)` in Hz with its operating category.
#[derive(Debug, Clone, Copy)]
pub struct Segment {
    pub lo: f64,
    pub hi: f64,
    pub kind: SegmentKind,
}

const fn seg(lo: f64, hi: f64, kind: SegmentKind) -> Segment {
    Segment { lo, hi, kind }
}

const M: f64 = 1_000_000.0;

/// HF CW / digi / phone / beacon segments (IARU Region 1), sorted by frequency.
pub const SEGMENTS: &[Segment] = &[
    // 160m
    seg(1.810 * M, 1.838 * M, SegmentKind::Cw),
    seg(1.838 * M, 1.843 * M, SegmentKind::Digi),
    seg(1.843 * M, 2.000 * M, SegmentKind::Phone),
    // 80m
    seg(3.500 * M, 3.570 * M, SegmentKind::Cw),
    seg(3.570 * M, 3.600 * M, SegmentKind::Digi),
    seg(3.600 * M, 3.800 * M, SegmentKind::Phone),
    // 40m
    seg(7.000 * M, 7.040 * M, SegmentKind::Cw),
    seg(7.040 * M, 7.100 * M, SegmentKind::Digi),
    seg(7.100 * M, 7.200 * M, SegmentKind::Phone),
    // 30m (no phone)
    seg(10.100 * M, 10.130 * M, SegmentKind::Cw),
    seg(10.130 * M, 10.150 * M, SegmentKind::Digi),
    // 20m
    seg(14.000 * M, 14.070 * M, SegmentKind::Cw),
    seg(14.070 * M, 14.099 * M, SegmentKind::Digi),
    seg(14.099 * M, 14.101 * M, SegmentKind::Beacon),
    seg(14.101 * M, 14.350 * M, SegmentKind::Phone),
    // 17m
    seg(18.068 * M, 18.095 * M, SegmentKind::Cw),
    seg(18.095 * M, 18.109 * M, SegmentKind::Digi),
    seg(18.109 * M, 18.111 * M, SegmentKind::Beacon),
    seg(18.111 * M, 18.168 * M, SegmentKind::Phone),
    // 15m
    seg(21.000 * M, 21.070 * M, SegmentKind::Cw),
    seg(21.070 * M, 21.150 * M, SegmentKind::Digi),
    seg(21.150 * M, 21.450 * M, SegmentKind::Phone),
    // 12m
    seg(24.890 * M, 24.915 * M, SegmentKind::Cw),
    seg(24.915 * M, 24.930 * M, SegmentKind::Digi),
    seg(24.930 * M, 24.990 * M, SegmentKind::Phone),
    // 10m
    seg(28.000 * M, 28.070 * M, SegmentKind::Cw),
    seg(28.070 * M, 28.190 * M, SegmentKind::Digi),
    seg(28.190 * M, 28.300 * M, SegmentKind::Beacon),
    seg(28.300 * M, 29.700 * M, SegmentKind::Phone),
];

/// The operating category at `hz`, or `None` outside every listed HF segment.
pub fn segment_kind_at(hz: f64) -> Option<SegmentKind> {
    SEGMENTS.iter().find(|s| hz >= s.lo && hz < s.hi).map(|s| s.kind)
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
const FT8_DIALS: &[f64] = &[
    1_840_000.0, 3_573_000.0, 7_074_000.0, 10_136_000.0, 14_074_000.0, 18_100_000.0,
    21_074_000.0, 24_915_000.0, 28_074_000.0,
];
/// FT4 dial frequencies (Hz).
const FT4_DIALS: &[f64] = &[
    3_575_000.0, 7_047_500.0, 10_140_000.0, 14_080_000.0, 18_104_000.0, 21_140_000.0,
    24_919_000.0, 28_180_000.0,
];
/// WSPR dial frequencies (Hz); a ~200 Hz slice above each.
const WSPR_DIALS: &[f64] = &[
    1_836_600.0, 3_568_600.0, 7_038_600.0, 10_138_700.0, 14_095_600.0, 18_104_600.0,
    21_094_600.0, 24_924_600.0, 28_124_600.0,
];

/// True where the *automatic* digital modes (FT8/FT4/WSPR) live. The PSK/RTTY
/// skimmers skip these sub-slices of the digi segment — their DSP would only
/// produce garbage from FT8/FT4/WSPR signals.
pub fn is_auto_digi(hz: f64) -> bool {
    FT8_DIALS.iter().any(|&f| (f - 100.0..=f + 3100.0).contains(&hz))
        || FT4_DIALS.iter().any(|&f| (f - 100.0..=f + 3100.0).contains(&hz))
        || WSPR_DIALS.iter().any(|&f| (f - 100.0..=f + 400.0).contains(&hz))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification() {
        assert_eq!(segment_kind_at(14_030_000.0), Some(SegmentKind::Cw));
        assert_eq!(segment_kind_at(14_074_000.0), Some(SegmentKind::Digi)); // FT8
        assert_eq!(segment_kind_at(14_200_000.0), Some(SegmentKind::Phone));
        assert!(is_cw_segment(7_020_000.0));
        assert!(!is_cw_segment(7_074_000.0));
        assert!(is_digi_segment(7_074_000.0));
        // Outside any HF ham segment.
        assert_eq!(segment_kind_at(15_000_000.0), None);
        assert!(!is_cw_segment(15_000_000.0));
    }

    #[test]
    fn segments_sorted_and_non_overlapping() {
        for w in SEGMENTS.windows(2) {
            assert!(w[0].hi <= w[1].lo, "overlap: {:?} then {:?}", w[0], w[1]);
        }
    }
}
