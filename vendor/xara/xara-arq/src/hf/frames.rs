use crate::hf::adapt::{SN_NORM_DB, step_thr};
use crate::link;
use crate::num::NO_SNR;
use crate::textcomp::fits_frames;
use xara_phy::hf::ctrl::Fmt;
use xara_phy::hf::frame::{Control, TERMINATOR};
use xara_phy::hf::level::{Bandwidth, Record, qam_limit, start_level_max};

pub const HINT_MAX: u8 = 6;
pub(crate) const UNREGISTERED_MAX_LEVEL: Record = Record::new(4).unwrap();

pub fn ctrl_thr(fmt: Fmt) -> f64 {
    let f = fmt.params();
    f64::from(f.snr_db) - (SN_NORM_DB - f64::from(f.k_db))
}

pub fn format_for_level(bw: Bandwidth, level: Record) -> Fmt {
    let thr = step_thr(bw.level(level));
    Fmt::ALL.into_iter().filter(|&f| ctrl_thr(f) <= thr).max().unwrap_or(Fmt::F0)
}

#[derive(Clone, Copy, Debug)]
pub struct ReplyFormat {
    prev_snr: f32,
    prev_missed: usize,
}

impl Default for ReplyFormat {
    fn default() -> Self {
        Self { prev_snr: NO_SNR, prev_missed: 0 }
    }
}

impl ReplyFormat {
    pub fn update(&mut self, snr_norm: f64, hits: usize, rx: Fmt) -> (f32, Fmt) {
        let norm = snr_norm as f32;
        let snr = norm.min(self.prev_snr);
        self.prev_snr = norm;
        let m = ((rx.params().nsym as usize - 1) * rx.params().ntones).saturating_sub(hits);
        let missed = m.max(self.prev_missed);
        self.prev_missed = m;
        (snr, choose_reply_format(snr, missed, rx))
    }
}

pub fn choose_reply_format(snr: f32, missed: usize, rx: Fmt) -> Fmt {
    const MAX_MISSED: [f64; 3] = [31.0, 1.0, 0.0];
    let tones = |f: Fmt| f.params().ntones as f64;
    let mut out = Fmt::F0;
    for (f, max) in Fmt::ALL.into_iter().zip(MAX_MISSED) {
        if max < tones(f) / tones(rx) * missed as f64 || snr <= ctrl_thr(f) as f32 {
            break;
        }
        out = f;
    }
    out
}

pub fn payload_len(bw: Bandwidth, level: Record) -> usize {
    bw.level(level).payload() as usize
}

pub fn queue_hint(bw: Bandwidth, level: Record, queued: usize) -> u8 {
    queue_hint_by(bw, level, |l, k| queued < k * payload_len(bw, l))
}

pub fn queue_hint_text(bw: Bandwidth, level: Record, queued: &[u8], state: usize) -> u8 {
    queue_hint_by(bw, level, |l, k| {
        let p = payload_len(bw, l);
        fits_frames(queued, k, p, p - 1, state)
    })
}

fn queue_hint_by(bw: Bandwidth, level: Record, fits: impl Fn(Record, usize) -> bool) -> u8 {
    const UP: [(usize, u8); 4] = [(1, 3), (2, 4), (3, 5), (4, 6)];
    let top = bw.top_level();
    link::queue_hint_by(level.get(), top.get(), [(0, 2, &UP), (0, 1, &UP)], HINT_MAX, |l, k| {
        Record::new(l).is_some_and(|l| l <= top && fits(l, k))
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parsed {
    pub ctrl: Control,
    pub last: bool,
    pub full: bool,
    pub next_level: Option<u8>,
    pub data: Option<Vec<u8>>,
}

pub fn parse(frame: &[u8], marker: u8) -> Option<Parsed> {
    let at = frame.len().checked_sub(3)?;
    let (ctrl, last, full) = Control::from_byte(frame[at]);
    let Some(end) = at.checked_sub(usize::from(last)) else {
        return Some(Parsed { ctrl, last, full, next_level: None, data: None });
    };
    let field = &frame[..end];
    let next_level = last.then(|| frame[end]);
    let data = if full {
        Some(field.to_vec())
    } else {
        field.iter().rposition(|&b| b == TERMINATOR).and_then(|i| {
            let after = field.get(i + 1).copied();
            (after.is_none() || after == Some(marker) || after == Some(0)).then(|| field[..i].to_vec())
        })
    };
    Some(Parsed { ctrl, last, full, next_level, data })
}

#[derive(Clone, Copy, Debug)]
pub struct StartCap {
    pub registered: bool,
    pub fs: bool,
    pub text: bool,
    pub header_sent: bool,
}

pub fn start_level_cap(bw: Bandwidth, c: StartCap) -> Record {
    match (c.registered, c.fs && !c.header_sent) {
        (false, _) => UNREGISTERED_MAX_LEVEL,
        (true, true) => qam_limit(bw),
        (true, false) => text_cap(start_level_max(bw), c.text),
    }
}

pub(crate) fn text_cap(max: Record, text: bool) -> Record {
    if text { max.down() } else { max }
}

pub fn hint_table_minimum(bw: Bandwidth, level: Record, text: bool) -> u8 {
    let t: &[u8] = match (bw, text) {
        (Bandwidth::Hz500, false) => &[6, 6, 6, 5, 3],
        (Bandwidth::Hz500, true) => &[6, 6, 5, 3],
        (_, false) => &[6, 6, 4, 2],
        (_, true) => &[6, 5, 3],
    };
    t.get(level.index()).copied().unwrap_or(0)
}
