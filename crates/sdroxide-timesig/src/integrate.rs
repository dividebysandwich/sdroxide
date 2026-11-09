//! Minutes too weak to read one at a time, read several at a time.
//!
//! A minute decodes only if every one of its fifty-nine seconds is read
//! right, so a signal whose bits are each misread one time in twenty loses
//! nearly every minute — while it is still being heard perfectly well. Below
//! about −12 dB that is where DCF77 and ALS162 sit: the seconds are locked,
//! most bits read, and no minute ever passes its checks.
//!
//! But most of what a minute says, the next one says again. The date is the
//! same all day; the time of day moves on by exactly one minute; the zone
//! is the same for months. So instead of deciding each bit and then checking
//! the minute, this keeps — for every value each field could have — how well
//! the seconds heard so far agree with that value, and lets the evidence for
//! each one add up from minute to minute. A field is decided when one value
//! stands clearly ahead of every other.
//!
//! # The evidence
//!
//! Each second arrives as two log-likelihood ratios, in nats: how much likelier
//! a 1 is than a 0, and how much likelier the minute marker is than either.
//! A value's score for one minute is half the sum of its bits' ratios, signed
//! by the bit — its log-likelihood, up to a constant every value shares. Each
//! ratio is clipped to [`CLIP`] first, so a burst of static that makes one
//! second look certain cannot outvote a minute of honest ones.
//!
//! # The fields
//!
//! In DCF77's layout, which ALS162 shares from second 15 on:
//!
//! - *The minute*: where in the minute each second is, from the marker. Each
//!   second's marker ratio is added into one of sixty bins by its count
//!   modulo sixty; the marker's bin climbs, the rest sink.
//! - *The clock*: the UTC time of day and the zone together, 2 × 1440 values,
//!   scored on seconds 21–35 (the local minute and hour, with their parities)
//!   and 17–18 (CEST, CET). Every minute each value's score moves on to the
//!   next minute along, since the minute that read 14:36 is followed by the
//!   one reading 14:37. Counted in UTC, the clock goes on smoothly through a
//!   change of summer time, where the local time jumps an hour and the zone
//!   bits swap; a local clock would carry an hour's worth of evidence for
//!   the wrong hour across the change, and might for a while name a minute
//!   whose wrong hour and wrong zone agree with each other.
//! - *The date*: seconds 36–58, day, weekday, month, year and their parity —
//!   one value per day from 2000 to 2099, so only real dates with the right
//!   weekday are candidates. A minute's date depends on which side of
//!   midnight it fell, which is only known once the clock is decided; so each
//!   minute's ratios are kept, and summed into "today" and "yesterday" when
//!   it is. A value's score is linear in the ratios, so the two sums are all
//!   it takes.
//!
//! Older evidence counts for less — each minute keeps [`KEEP`] of the last —
//! so a field that was wrong-footed recovers by itself.
//!
//! # Deciding
//!
//! A field's lead is its best value's score less the next best's: the log of
//! how much likelier the one is than the other. The clock and the date each
//! need [`DECIDE_LEAD`] — the best value at least twenty thousand times
//! likelier than any other — and the decided time is then put back through
//! the station's own encoder, which has to send exactly the bits decided:
//! the summer-time rule agrees with the zone, the weekday is the date's. Only
//! then is the minute named.
//!
//! The nearest rival of any value differs from it in at least two bits — the
//! parity bits see to that, and the zone bits come in pairs — so on a clean
//! signal one minute is enough, and the integration costs nothing where a
//! minute would have read anyway.

use std::collections::VecDeque;

use sdroxide_types::ymd_hms_to_unix;

use crate::format::Format;
use crate::format::dcf77::{DATE_FROM, TIME_FROM, ZONE, date_field, time_field};

/// The most one second's ratio may count for, nats.
const CLIP: f32 = 6.0;
/// How much of every field's evidence is kept from one minute to the next:
/// a memory of about twenty minutes. Its noise averages down as if from
/// forty, which is about what a signal 8 dB below the single-minute limit
/// needs.
const KEEP: f32 = 0.95;
/// The lead the marker's bin needs over every other before the minute is
/// known, nats.
const SYNC_LEAD: f32 = 8.0;
/// The lead the clock and the date each need before they are decided, nats.
const DECIDE_LEAD: f32 = 10.0;
/// A gap longer than this, minutes, and the evidence is started again: a
/// sample clock off by a hundred parts per million has slipped a fifth of a
/// second in that time, and whatever else happened in it is unknown.
const MAX_GAP_MIN: i64 = 30;
/// Minutes of date evidence kept: what is older counts for less than
/// [`KEEP`]^300, a ten-millionth.
const DATE_KEPT: usize = 300;
/// Days from 2000-01-01 to 2099-12-31.
const DAYS: usize = 36_525;
const TIMES: usize = 1440;
/// Each zone's offset from UTC, minutes: CEST, CET — the order of [`ZONE`].
const ZONE_MIN: [usize; 2] = [120, 60];

/// What one second said, as log-likelihood ratios in nats.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Soft {
    /// A 1 against a 0.
    pub bit: f32,
    /// The marker against the likelier of a 0 and a 1.
    pub marker: f32,
}

/// The minute being gathered: its count since the first second, and each
/// second's bit ratio.
struct Minute {
    id: i64,
    llr: [f32; 60],
}

pub(crate) struct Integrator {
    fmt: &'static Format,
    rate: f64,
    /// The last second seen, and its count.
    last: Option<(f64, i64)>,
    marker: [f32; 60],
    /// Which bin of [`Self::marker`] the minute marker is in.
    sync: Option<usize>,
    minute: Option<Minute>,
    /// The minute last added to the fields.
    folded: Option<i64>,
    /// Each clock value's score: zone `z`'s UTC minute of the day `u` at
    /// `z * TIMES + u`.
    clock: Vec<f32>,
    /// Each minute's date ratios, newest last, with its count.
    dates: VecDeque<(i64, [f32; 23])>,
    /// Seconds 20 (always 1) and 0 (always 0): a minute gathered at the
    /// wrong second shows in them.
    start_bit: f32,
    zero_bit: f32,
    tod_bits: Vec<u16>,
    date_bits: Vec<u32>,
}

impl Integrator {
    pub fn new(fmt: &'static Format, rate: f64) -> Self {
        let epoch = ymd_hms_to_unix(2000, 1, 1, 0, 0, 0);
        Integrator {
            fmt,
            rate,
            last: None,
            marker: [0.0; 60],
            sync: None,
            minute: None,
            folded: None,
            clock: vec![0.0; 2 * TIMES],
            dates: VecDeque::new(),
            start_bit: 0.0,
            zero_bit: 0.0,
            tod_bits: (0..TIMES as u32).map(|v| time_field(v / 60, v % 60)).collect(),
            date_bits: (0..DAYS as i64).map(|d| date_field(epoch + d * 86_400)).collect(),
        }
    }

    /// Forgets everything: a new carrier, or a new station.
    pub fn reset(&mut self) {
        self.last = None;
        self.marker = [0.0; 60];
        self.sync = None;
        self.clear_fields();
    }

    fn clear_fields(&mut self) {
        self.minute = None;
        self.folded = None;
        self.clock.iter_mut().for_each(|v| *v = 0.0);
        self.dates.clear();
        self.start_bit = 0.0;
        self.zero_bit = 0.0;
    }

    /// One second, starting at input sample `pos`, and what it said if it
    /// could be measured at all. Returns the UTC minute that starts one
    /// second after it — when it is a minute's marker, and every field of
    /// that minute is decided.
    pub fn push(&mut self, pos: f64, soft: Option<Soft>) -> Option<i64> {
        // Counted from the last second rather than from the first, so a
        // sample clock's drift never adds up to a miscount.
        let n = match self.last {
            Some((p, n)) => n + ((pos - p) / self.rate).round() as i64,
            None => 0,
        };
        self.last = Some((pos, n));
        if let Some(s) = soft {
            let i = n.rem_euclid(60) as usize;
            self.marker[i] = KEEP * self.marker[i] + s.marker.clamp(-CLIP, CLIP);
            self.resync();
        }
        let sync = self.sync?;
        let k = n - sync as i64 - 1;
        let (id, sec) = (k.div_euclid(60), k.rem_euclid(60) as usize);
        // A second of a later minute: the one gathered so far is as complete
        // as it will get.
        if let Some(m) = self.minute.take_if(|m| m.id != id) {
            self.fold(&m);
        }
        let m = self.minute.get_or_insert(Minute { id, llr: [0.0; 60] });
        if let Some(s) = soft {
            m.llr[sec] = s.bit.clamp(-CLIP, CLIP);
        }
        if sec < 59 {
            return None;
        }
        let m = self.minute.take().expect("just gathered");
        self.fold(&m);
        self.decide()
    }

    /// Follows the marker's bin, once it stands clear of the rest. A clear
    /// winner somewhere new means everything gathered was gathered at the
    /// wrong second of the minute.
    fn resync(&mut self) {
        let (best, lead) = top2(&self.marker);
        if lead < SYNC_LEAD || self.marker[best] <= 0.0 || self.sync == Some(best) {
            return;
        }
        self.sync = Some(best);
        self.clear_fields();
    }

    fn fold(&mut self, m: &Minute) {
        let mut delta = self.folded.map_or(1, |f| m.id - f);
        if delta <= 0 {
            return;
        }
        if delta > MAX_GAP_MIN {
            self.clear_fields();
            delta = 1;
        }
        self.folded = Some(m.id);
        let keep = KEEP.powi(delta as i32);

        let time = &m.llr[TIME_FROM..TIME_FROM + 15];
        let (cest, cet) = (m.llr[ZONE[0]], m.llr[ZONE[1]]);
        let zone = [0.5 * (cest - cet), 0.5 * (cet - cest)];
        let tod: Vec<f32> =
            self.tod_bits.iter().map(|&bits| half_dot(u32::from(bits), time)).collect();
        for (z, block) in self.clock.chunks_mut(TIMES).enumerate() {
            block.rotate_right(delta as usize % TIMES);
            for (u, score) in block.iter_mut().enumerate() {
                *score = keep * *score + tod[(u + ZONE_MIN[z]) % TIMES] + zone[z];
            }
        }

        let mut date = [0.0f32; 23];
        date.copy_from_slice(&m.llr[DATE_FROM..DATE_FROM + 23]);
        self.dates.push_back((m.id, date));
        if self.dates.len() > DATE_KEPT {
            self.dates.pop_front();
        }
        self.start_bit = keep * self.start_bit + m.llr[20];
        self.zero_bit = keep * self.zero_bit + m.llr[0];
    }

    /// The UTC minute the fields name, if each is decided and the station's
    /// own encoder sends exactly those bits for it.
    fn decide(&self) -> Option<i64> {
        let (c, c_lead) = top2(&self.clock);
        if c_lead < DECIDE_LEAD {
            return None;
        }
        // The framing bits are not voted on, but a minute gathered at the
        // wrong second would show in them.
        if self.start_bit <= 0.0 || self.zero_bit >= 0.0 {
            return None;
        }
        let (z, u) = (c / TIMES, c % TIMES);
        let local = (u + ZONE_MIN[z]) % TIMES;
        // Each minute's date ratios, summed by which side of the last
        // midnight it named: `local` minutes ago it named 00:00.
        let newest = self.folded?;
        let (mut today, mut yesterday) = ([0.0f32; 23], [0.0f32; 23]);
        for (id, llr) in &self.dates {
            let age = newest - id;
            let w = KEEP.powi(age as i32);
            let sum = if age <= local as i64 { &mut today } else { &mut yesterday };
            for (s, l) in sum.iter_mut().zip(llr) {
                *s += w * l;
            }
        }
        let scores: Vec<f32> = (0..DAYS)
            .map(|d| {
                let before = if d > 0 { half_dot(self.date_bits[d - 1], &yesterday) } else { 0.0 };
                half_dot(self.date_bits[d], &today) + before
            })
            .collect();
        let (d, d_lead) = top2(&scores);
        if d_lead < DECIDE_LEAD {
            return None;
        }
        let local_unix =
            ymd_hms_to_unix(2000, 1, 1, 0, 0, 0) + d as i64 * 86_400 + local as i64 * 60;
        let named = local_unix - ZONE_MIN[z] as i64 * 60;
        // The minute that names `named` is sent in the minute before it.
        let sent = (self.fmt.encode)(named - 60);
        let field = |from: usize, len: usize| {
            sent[from..from + len]
                .iter()
                .enumerate()
                .fold(0u32, |acc, (i, s)| acc | (u32::from(s.a) << i))
        };
        let agrees = field(TIME_FROM, 15) == u32::from(self.tod_bits[local])
            && field(DATE_FROM, 23) == self.date_bits[d]
            && sent[ZONE[0]].a == (z == 0)
            && sent[ZONE[1]].a == (z == 1);
        agrees.then_some(named)
    }
}

/// Half the sum of `llr`, each signed by its bit in `bits`: the
/// log-likelihood of that pattern, up to a constant.
fn half_dot(bits: u32, llr: &[f32]) -> f32 {
    llr.iter().enumerate().map(|(i, &l)| if bits & (1 << i) != 0 { l } else { -l }).sum::<f32>()
        * 0.5
}

/// The index of the largest score, and its lead over the next largest.
fn top2(v: &[f32]) -> (usize, f32) {
    let (mut best, mut first, mut second) = (0, f32::MIN, f32::MIN);
    for (i, &x) in v.iter().enumerate() {
        if x > first {
            second = first;
            first = x;
            best = i;
        } else if x > second {
            second = x;
        }
    }
    (best, first - second)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::Format;
    use sdroxide_types::TimeStation;

    /// The bits a station sends, as confident ratios, one second at a time,
    /// with every `flip`th second's bit turned over.
    fn feed(
        it: &mut Integrator,
        fmt: &Format,
        first_minute: i64,
        minutes: i64,
        flip: Option<usize>,
    ) -> Vec<(i64, i64)> {
        let mut named = Vec::new();
        let mut count = 0usize;
        for m in 0..minutes {
            let sent = (fmt.encode)(first_minute + m * 60);
            for (s, sym) in sent.iter().enumerate() {
                count += 1;
                let mut bit = if sym.a { 2.0 } else { -2.0 };
                if flip.is_some_and(|f| count.is_multiple_of(f)) {
                    bit = -bit;
                }
                let soft = Soft { bit, marker: if sym.marker { 2.0 } else { -2.0 } };
                let pos = (count as f64) * 12_000.0;
                if let Some(utc) = it.push(pos, Some(soft)) {
                    named.push((first_minute + m * 60 + s as i64 + 1, utc));
                }
            }
        }
        named
    }

    #[test]
    fn a_clean_minute_stream_is_named_minute_by_minute() {
        for station in [TimeStation::Dcf77, TimeStation::Als162] {
            let fmt = Format::for_station(station);
            let mut it = Integrator::new(fmt, 12_000.0);
            let start = ymd_hms_to_unix(2026, 10, 18, 12, 30, 0);
            let named = feed(&mut it, fmt, start, 12, None);
            assert!(named.len() >= 6, "{station:?}: {named:?}");
            for (want, got) in named {
                assert_eq!(got, want, "{station:?}");
            }
        }
    }

    /// One bit in seven turned over — no minute would pass its parity — and
    /// still every minute named is the right one.
    #[test]
    fn minutes_that_never_read_are_named_from_several() {
        let fmt = Format::for_station(TimeStation::Dcf77);
        let mut it = Integrator::new(fmt, 12_000.0);
        let start = ymd_hms_to_unix(2026, 10, 18, 12, 30, 0);
        let named = feed(&mut it, fmt, start, 30, Some(7));
        assert!(named.len() >= 10, "{named:?}");
        for (want, got) in named {
            assert_eq!(got, want);
        }
    }

    /// Across midnight, the end of a month, and a change of summer time:
    /// whatever is named is right.
    #[test]
    fn midnight_and_summer_time_never_name_a_wrong_minute() {
        let fmt = Format::for_station(TimeStation::Dcf77);
        for start in [
            // 23:40 CET on 31 January.
            ymd_hms_to_unix(2026, 1, 31, 22, 40, 0),
            // CEST to CET at 01:00 UTC on 25 October.
            ymd_hms_to_unix(2026, 10, 25, 0, 30, 0),
            // CET to CEST on 29 March.
            ymd_hms_to_unix(2026, 3, 29, 0, 30, 0),
            // New Year's Eve, a leap year.
            ymd_hms_to_unix(2024, 12, 31, 22, 40, 0),
        ] {
            let mut it = Integrator::new(fmt, 12_000.0);
            let named = feed(&mut it, fmt, start, 60, Some(9));
            assert!(named.len() >= 20, "{start}: only {} named", named.len());
            for (want, got) in named {
                assert_eq!(got, want, "starting {start}");
            }
        }
    }
}
