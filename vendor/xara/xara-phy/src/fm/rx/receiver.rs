use super::ctrl::{CtrlDetection, CtrlDetector};
use super::frontend::FrontEnd;
use super::ofdm::{CP_MID, Detection, FrameRx, HOP, MIN_COHERENCE, OfdmEngine, Stft};
use super::{CP, DEC, FS6, SYMLEN, to48};
use crate::dsp::Track;
use crate::fm::code::{Codec, phasors, record_key, tables};
use crate::fm::ctrl::{Codebook, Side};
use crate::fm::level::{FmBandwidth, MAX_SPF, Record};
use crate::receiver::{Assemble, Assembly, Muted, Pending, continues};
use num_complex::Complex64;
use std::collections::BTreeMap;

pub use crate::receiver::DecodeStats;

const BUF_SECONDS: f64 = 12.0;
const GRACE: i64 = 2 * SYMLEN;
const CONTINUE_AT: i64 = 2 * SYMLEN;
const NOISE_BLOCK: usize = 60;
const PROBE_ROWS: usize = 6;

#[derive(Clone, Debug)]
pub struct CtrlEvent {
    pub det: CtrlDetection,
    pub side: Side,
}

impl CtrlEvent {
    pub fn t_end48(&self) -> i64 {
        to48(self.det.t_end)
    }
}

impl crate::receiver::Frame for FrameRx {
    type Record = Record;

    fn ok(&self) -> bool {
        self.ok
    }

    fn t(&self) -> i64 {
        self.t
    }

    fn t_end(&self) -> i64 {
        self.t_end
    }
}

pub type Transmission = crate::receiver::Transmission<FrameRx>;

impl Transmission {
    pub fn t_end48(&self) -> i64 {
        to48(self.t_end)
    }
}

pub type Event = crate::receiver::Event<CtrlEvent, FrameRx>;

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub decode: DecodeStats,
    pub discarded: usize,
    pub tx: usize,
    pub unexpected_frame2: usize,
}

pub struct Receiver {
    pub bw: FmBandwidth,
    key: u32,
    fe: FrontEnd,
    buf: Track<Complex64>,
    stft: Stft,
    engines: BTreeMap<Record, OfdmEngine>,
    asm: Assembly<FrameRx, ()>,
    muted: Muted,
    ctrl: [Option<CtrlDetector>; 2],
    codec: Codec,
    probed: Option<i64>,
    pub stats: Stats,
}

impl Receiver {
    pub fn new(bw: FmBandwidth, key: u32, records: &[Record], nframes: usize) -> Self {
        let mut rx = Self {
            bw,
            key,
            fe: FrontEnd::new(),
            buf: Track::new((BUF_SECONDS * FS6) as usize),
            stft: Stft::new(MAX_SPF as usize + 1),
            engines: BTreeMap::new(),
            asm: Assembly::new(nframes),
            muted: Muted::default(),
            ctrl: [None, None],
            codec: Codec::new(bw),
            probed: None,
            stats: Stats::default(),
        };
        rx.expect(records, Some(nframes));
        rx
    }

    pub fn key(&self) -> u32 {
        self.key
    }

    fn engine(&self, record: Record) -> OfdmEngine {
        let l = self.bw.level(record);
        let mut e = OfdmEngine::new(l, &tables(self.bw)[record.index()].other, &phasors(record_key(record, self.key)));
        let span_hops = e.span / HOP;
        e.start_at(self.stft.diff.base.max(self.stft.diff.end() - span_hops - (FS6 as i64) / HOP));
        e
    }

    pub fn expect(&mut self, records: &[Record], nframes: Option<usize>) {
        for &r in records {
            if !self.engines.contains_key(&r) {
                let e = self.engine(r);
                self.engines.insert(r, e);
            }
        }
        self.engines.retain(|r, _| records.contains(r));
        if let Some(n) = nframes {
            self.asm.nframes = n;
        }
    }

    pub fn expected(&self) -> Vec<Record> {
        self.engines.keys().copied().collect()
    }

    pub fn set_key(&mut self, key: u32) {
        if key == self.key {
            return;
        }
        self.key = key;
        let keyed: Vec<Record> = self.engines.keys().copied().filter(|r| r.keyed()).collect();
        for r in keyed {
            let e = self.engine(r);
            self.engines.insert(r, e);
        }
    }

    pub fn set_bandwidth(&mut self, bw: FmBandwidth) {
        if bw == self.bw {
            return;
        }
        self.bw = bw;
        self.codec = Codec::new(bw);
        let recs = self.expected();
        self.engines.clear();
        self.expect(&recs, None);
    }

    pub fn set_ctrl(&mut self, side: Side, cb: Option<Codebook>, codes: &[u32]) {
        let start = self.buf.base.max(self.buf.end() - 40 * 128);
        let slot = &mut self.ctrl[side.idx()];
        match (cb, slot.as_mut()) {
            (None, _) => *slot = None,
            (Some(cb), Some(c)) => {
                if c.codebook() != &cb {
                    c.set_codebook(cb);
                }
                c.set_enabled(codes);
            }
            (Some(cb), None) => {
                let mut c = CtrlDetector::new(cb, codes);
                c.start_at(start);
                *slot = Some(c);
            }
        }
    }

    pub fn mute(&mut self, a48: i64, b48: i64) {
        self.muted.mute(&self.buf, FS6, a48 / DEC, b48 / DEC);
    }

    pub fn mark_tx_end(&mut self, n48: i64) {
        self.asm.tx_end48 = Some(n48);
    }

    pub fn feed(&mut self, x: &[f64]) -> Vec<Event> {
        let (m0, y) = self.fe.process(x);
        if !y.is_empty() {
            self.buf.append(m0, y);
        }
        let mut ev = Vec::new();
        for (side, c) in Side::BOTH.into_iter().zip(&mut self.ctrl) {
            if let Some(c) = c {
                ev.extend(c.step(&self.buf).into_iter().map(|det| Event::Ctrl(CtrlEvent { det, side })));
            }
        }
        self.stft.update(&self.buf);
        let mut dets: Vec<Detection> = self.engines.values_mut().flat_map(|e| e.step(&self.stft)).collect();
        dets.sort_by(|a, b| a.t.cmp(&b.t).then(b.metric.total_cmp(&a.metric)));
        for d in dets {
            ev.extend(self.on_det(d.record, d.t, ()));
        }
        ev.extend(self.pending());
        ev
    }

    pub fn flush(&mut self) -> Vec<Event> {
        self.finalize()
    }

    pub fn noise_floor(&self) -> f64 {
        self.muted.quiet_blocks(&self.buf, FS6, NOISE_BLOCK).map_or(0.0, |p| p[p.len() / 100])
    }

    fn continues(&self, end: i64) -> bool {
        continues(&self.buf, (end + CP, end + CONTINUE_AT), (end - 2 * SYMLEN, end - CP), self.noise_floor())
    }

    fn pending(&mut self) -> Vec<Event> {
        let Some(mut cur) = self.asm.cur.take() else { return Vec::new() };
        if !self.engines.contains_key(&cur.record) {
            return self.finish(cur);
        }
        let span = self.span(cur.record);
        let end1 = cur.t - CP_MID + span;
        if !cur.two {
            if self.buf.end() < end1 + CONTINUE_AT {
                self.asm.cur = Some(cur);
                return Vec::new();
            }
            if !(cur.frames[0].ok && cur.record != Record::Broadcast && self.continues(end1)) {
                return self.finish(cur);
            }
            self.stats.unexpected_frame2 += 1;
            cur.two = true;
        }
        let (t2, record) = (cur.t + span, cur.record);
        let guessed = self.asm.nframes != 2;
        if guessed && self.probed != Some(t2) {
            match self.engines[&record].early_coherence(&self.buf, t2, PROBE_ROWS) {
                None => {
                    self.asm.cur = Some(cur);
                    return Vec::new();
                }
                Some(c) if c < MIN_COHERENCE => return self.finish(cur),
                Some(_) => self.probed = Some(t2),
            }
        }
        self.asm.cur = Some(cur);
        if self.buf.end() < t2 + span + GRACE {
            return Vec::new();
        }
        match self.decode(record, t2, 1, ()) {
            Some(r) if !guessed || r.ok || r.coherence >= MIN_COHERENCE => self.close(r, false, Vec::new()),
            _ => self.finalize(),
        }
    }
}

impl Assemble for Receiver {
    type Record = Record;
    type Frame = FrameRx;
    type Hint = ();
    type Ctrl = CtrlEvent;
    type Wait = ();
    const PAIR_TOL: i64 = SYMLEN / 2;
    const EMIT_BAD: bool = false;
    const CLOSE_ON_MISS: bool = true;
    const LOST_NEEDS_OK: bool = true;

    fn asm(&self) -> &Assembly<FrameRx, ()> {
        &self.asm
    }

    fn asm_mut(&mut self) -> &mut Assembly<FrameRx, ()> {
        &mut self.asm
    }

    fn track(&self) -> &Track<Complex64> {
        &self.buf
    }

    fn has(&self, record: Record) -> bool {
        self.engines.contains_key(&record)
    }

    fn span(&self, record: Record) -> i64 {
        self.bw.level(record).nsym() as i64 * SYMLEN
    }

    fn decode(&mut self, record: Record, t: i64, index: usize, _: ()) -> Option<FrameRx> {
        let e = self.engines.get(&record)?;
        let r = e.decode(&self.buf, &self.codec, t, index)?;
        self.stats.decode.count(r.ok);
        Some(r)
    }

    fn pairs(&self, _: &Pending<FrameRx, ()>, record: Record) -> bool {
        record != Record::Broadcast
    }

    fn frame1_end(&self, cur: &Pending<FrameRx, ()>) -> i64 {
        cur.t + self.span(cur.record)
    }

    fn lost(&self, record: Record, back: i64, _: i64, span: i64) -> Option<FrameRx> {
        let te = self.asm.tx_end48? / DEC;
        (back - CP_MID >= te).then(|| FrameRx {
            record,
            index: 0,
            t: back,
            t_end: back - CP_MID + span,
            missing: true,
            ..Default::default()
        })
    }

    fn keep(&mut self, r: &FrameRx) -> bool {
        let k = r.ok || r.coherence >= MIN_COHERENCE;
        if !k {
            self.stats.discarded += 1;
        }
        k
    }

    fn wait(&self, _: Record, _: &FrameRx, _: ()) {}

    fn expected_frames(&self, _: &Pending<FrameRx, ()>) -> usize {
        self.asm.nframes.max(1)
    }

    fn finished(&mut self, _: &[FrameRx]) {
        self.stats.tx += 1;
    }
}
