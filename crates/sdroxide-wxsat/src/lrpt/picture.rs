//! The picture the segments make: one image per channel, 1568 pixels wide,
//! eight lines per row of segments. A row is told by its time, not by a
//! counter: the segments of a row share a time stamp, rows are about 1.231 s
//! apart, and a lost row leaves a gap of the right size instead of shifting
//! everything after it.

use std::collections::{BTreeMap, HashSet};

use super::jpeg::{MCU_PER_LINE, MCU_PER_SEGMENT, PICTURE_WIDTH, Segment};

/// Time between two rows of segments, milliseconds (a mean: 1228 and 1232
/// alternate on the pass the decoder was made on).
const ROW_MS: f64 = 1231.0;
/// Segments within this many ms of a known row's time belong to it.
const SAME_ROW_MS: i64 = 400;
/// A new row this many rows from the last one is a new pass, not a gap.
const MAX_GAP_ROWS: i64 = 400;
/// Lines in a row.
pub const ROW_LINES: usize = 8;

/// Rows of one channel, by row number.
#[derive(Default)]
struct Channel {
    rows: BTreeMap<i64, Vec<u8>>,
}

#[derive(Default)]
pub struct LrptPicture {
    channels: BTreeMap<u16, Channel>,
    /// Recent rows: time and number.
    known: Vec<(i64, i64)>,
    /// Number of the first row of this picture, so that the height is counted
    /// from it.
    first_row: Option<i64>,
    /// Rows already handed out by [`LrptPicture::take_finished`].
    sent: HashSet<(u16, i64)>,
}

/// A row of one channel, finished: `ROW_LINES` lines of `PICTURE_WIDTH` pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedRow {
    pub apid: u16,
    /// Row number from the top of the picture.
    pub row: u32,
    pub lines: Vec<u8>,
}

impl LrptPicture {
    pub fn new() -> Self {
        Self::default()
    }

    /// The row a segment of time `ms` belongs to.
    fn row_for(&mut self, ms: i64) -> i64 {
        if let Some(&(_, r)) = self.known.iter().find(|&&(t, _)| (t - ms).abs() <= SAME_ROW_MS) {
            return r;
        }
        let row = match self.known.last() {
            None => 0,
            Some(&(t, r)) => r + ((ms - t) as f64 / ROW_MS).round() as i64,
        };
        self.known.push((ms, row));
        if self.known.len() > 12 {
            self.known.remove(0);
        }
        row
    }

    /// Put a segment in its place. A segment far from the others in time (the
    /// next pass, or a clock that jumped) starts a new picture.
    pub fn push(&mut self, seg: &Segment) {
        let ms = i64::from(seg.ms);
        if let Some(&(t, _)) = self.known.last()
            && ((ms - t) as f64 / ROW_MS).abs() > MAX_GAP_ROWS as f64
        {
            *self = Self::default();
        }
        let row = self.row_for(ms);
        let first = *self.first_row.get_or_insert(row);
        if row < first {
            self.first_row = Some(row);
        }
        let ch = self.channels.entry(seg.apid).or_default();
        let r = ch.rows.entry(row).or_insert_with(|| vec![0u8; PICTURE_WIDTH * ROW_LINES]);
        let x0 = usize::from(seg.mcu_id) * 8;
        for (b, block) in seg.pixels.chunks(64).enumerate().take(MCU_PER_SEGMENT) {
            let x = x0 + b * 8;
            if x + 8 > MCU_PER_LINE * 8 {
                break;
            }
            for line in 0..8 {
                r[line * PICTURE_WIDTH + x..line * PICTURE_WIDTH + x + 8]
                    .copy_from_slice(&block[line * 8..line * 8 + 8]);
            }
        }
    }

    /// The rows nobody has taken yet that are done: every row older than the
    /// newest one seen, or every row when `flush` says the pass has gone quiet.
    /// Each row is handed out once, so a link can carry a row when it is whole
    /// rather than each time one more of its 14 segments arrives.
    pub fn take_finished(&mut self, flush: bool) -> Vec<FinishedRow> {
        let Some(first) = self.first_row else { return Vec::new() };
        let newest = self
            .channels
            .values()
            .filter_map(|c| c.rows.keys().next_back())
            .max()
            .copied()
            .unwrap_or(first);
        let mut out = Vec::new();
        for (&apid, ch) in &self.channels {
            for (&r, lines) in &ch.rows {
                if (r < newest || flush) && self.sent.insert((apid, r)) {
                    out.push(FinishedRow {
                        apid,
                        row: (r - first).max(0) as u32,
                        lines: lines.clone(),
                    });
                }
            }
        }
        // Rows long gone need not be remembered.
        self.sent.retain(|&(_, r)| r + 64 >= newest);
        out.sort_by_key(|f| (f.row, f.apid));
        out
    }

    /// The channels that have anything, by APID.
    pub fn channels(&self) -> Vec<u16> {
        self.channels.keys().copied().collect()
    }

    /// Rows from the first one seen to the last, gaps included.
    pub fn rows(&self) -> usize {
        let Some(first) = self.first_row else { return 0 };
        let last = self
            .channels
            .values()
            .filter_map(|c| c.rows.keys().next_back())
            .max()
            .copied()
            .unwrap_or(first);
        (last - first + 1).max(0) as usize
    }

    /// One channel as grey pixels, `PICTURE_WIDTH` wide, a row of lines per row
    /// of segments; a row that never arrived is black.
    pub fn gray(&self, apid: u16) -> Option<(usize, usize, Vec<u8>)> {
        let ch = self.channels.get(&apid)?;
        let first = self.first_row?;
        let rows = self.rows();
        let mut out = vec![0u8; rows * ROW_LINES * PICTURE_WIDTH];
        for (&r, data) in &ch.rows {
            let at = (r - first) as usize * ROW_LINES * PICTURE_WIDTH;
            if at + data.len() <= out.len() {
                out[at..at + data.len()].copy_from_slice(data);
            }
        }
        Some((PICTURE_WIDTH, rows * ROW_LINES, out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(apid: u16, ms: u32, mcu_id: u8, v: u8) -> Segment {
        Segment { apid, ms, mcu_id, q: 90, pixels: vec![v; MCU_PER_SEGMENT * 64] }
    }

    #[test]
    fn segments_of_a_row_go_side_by_side_and_rows_stack() {
        let mut p = LrptPicture::new();
        p.push(&seg(64, 1_000_000, 0, 10));
        p.push(&seg(64, 1_000_003, 14, 20));
        p.push(&seg(65, 1_000_000, 0, 99));
        p.push(&seg(64, 1_001_231, 0, 30));
        assert_eq!(p.channels(), vec![64, 65]);
        assert_eq!(p.rows(), 2);
        let (w, h, g) = p.gray(64).unwrap();
        assert_eq!((w, h), (1568, 16));
        assert_eq!(g[0], 10);
        assert_eq!(g[14 * 8], 20);
        assert_eq!(g[8 * 1568], 30);
        // The other channel is on the same rows.
        assert_eq!(p.gray(65).unwrap().2[0], 99);
    }

    #[test]
    fn a_lost_row_leaves_a_gap_of_its_own_size() {
        let mut p = LrptPicture::new();
        p.push(&seg(64, 5_000_000, 0, 1));
        p.push(&seg(64, 5_000_000 + 3 * 1231 + 2, 0, 2)); // two rows lost
        assert_eq!(p.rows(), 4);
        let (_, _, g) = p.gray(64).unwrap();
        assert_eq!(g[0], 1);
        assert_eq!(g[8 * 1568], 0, "the lost rows are black");
        assert_eq!(g[3 * 8 * 1568], 2);
    }

    #[test]
    fn a_late_segment_of_an_earlier_row_finds_its_row() {
        let mut p = LrptPicture::new();
        p.push(&seg(64, 9_000_000, 0, 1));
        p.push(&seg(64, 9_001_231, 0, 2));
        p.push(&seg(64, 9_000_004, 14, 7)); // a late one for the first row
        let (_, _, g) = p.gray(64).unwrap();
        assert_eq!(g[14 * 8], 7);
        assert_eq!(p.rows(), 2);
    }

    #[test]
    fn a_row_is_handed_out_once_when_a_newer_one_has_begun() {
        let mut p = LrptPicture::new();
        p.push(&seg(64, 1_000_000, 0, 10));
        p.push(&seg(65, 1_000_000, 0, 11));
        assert!(p.take_finished(false).is_empty(), "the only row may still grow");
        p.push(&seg(64, 1_001_231, 0, 12));
        let done = p.take_finished(false);
        assert_eq!(
            done.iter().map(|f| (f.apid, f.row)).collect::<Vec<_>>(),
            vec![(64, 0), (65, 0)]
        );
        assert_eq!(done[0].lines.len(), PICTURE_WIDTH * ROW_LINES);
        assert_eq!(done[0].lines[0], 10);
        assert!(p.take_finished(false).is_empty(), "each row once");
        // The pass goes quiet: the last row is let go.
        let last = p.take_finished(true);
        assert_eq!(last.iter().map(|f| (f.apid, f.row)).collect::<Vec<_>>(), vec![(64, 1)]);
    }

    #[test]
    fn a_far_jump_in_time_starts_a_new_picture() {
        let mut p = LrptPicture::new();
        p.push(&seg(64, 1_000_000, 0, 1));
        p.push(&seg(64, 80_000_000, 0, 2));
        assert_eq!(p.rows(), 1);
    }
}
