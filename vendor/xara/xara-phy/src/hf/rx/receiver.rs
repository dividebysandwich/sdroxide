use super::ctrl::{CtrlDetection, CtrlDetector, CtrlEntry};
use super::frontend::FrontEnd;
use super::fsk::FskEngine;
use super::ofdm::{HYPOTHESES, OfdmEngine, Stft};
use super::psk::PskEngine;
use super::{DEC, FS3};
use crate::dat::RecordTables;
use crate::decode::FrameDecoder;
use crate::dsp::Track;
use crate::hf;
use crate::hf::level::{Bandwidth, Record, Waveform};
use crate::receiver::{Assemble, Assembly, Muted, Pending, continues};
use num_complex::Complex64;
use std::collections::BTreeMap;
use std::f64::consts::PI;

pub use crate::receiver::DecodeStats;

const BUF_SECONDS: f64 = 24.0;
const SEARCH_HZ_LINKED: f64 = 30.0;
const SEARCH_HZ_UNLINKED: f64 = 117.0;
const GRACE: i64 = (0.04 * FS3) as i64;
const CONFIRM_GAP: i64 = 8;
const CONFIRM_GAP_PSK: i64 = 48;
const CONFIRM_LEN: i64 = 60;
const CONFIRM_WINDOWS: i64 = 10;
const FRAME2_LOOKAHEAD: i64 = 3 * 32 + 16;

pub(crate) struct Ctx<'a> {
    pub buf: &'a Track<Complex64>,
    pub linked: bool,
    pub search_hz: f64,
    pub ofdm: &'a [Stft],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Detection {
    pub record: Record,
    pub t: i64,
    pub hint: Hint,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Hint {
    Fsk(i64),
    Psk,
    Ofdm(f64),
}

#[derive(Clone, Debug, Default)]
pub struct Decoded {
    pub record: Record,
    pub index: usize,
    pub t: i64,
    pub t_end: i64,
    pub ok: bool,
    pub bytes: Vec<u8>,
    pub raw_snr_db: f64,
    pub snr_db: f64,
    pub snr_avg_db: Option<f64>,
    pub afc_res: f64,
    pub iterations: usize,
    pub metric: f64,
}

#[derive(Clone, Debug)]
pub struct FrameRx {
    pub frame: Decoded,
    pub afc_hz: f64,
    pub missing: bool,
}

impl FrameRx {
    pub fn t_end48(&self) -> i64 {
        to48(self.frame.t_end)
    }
}

impl crate::receiver::Frame for FrameRx {
    type Record = Record;

    fn ok(&self) -> bool {
        self.frame.ok
    }

    fn t(&self) -> i64 {
        self.frame.t
    }

    fn t_end(&self) -> i64 {
        self.frame.t_end
    }
}

pub type Transmission = crate::receiver::Transmission<FrameRx>;

impl Transmission {
    pub fn t_end48(&self) -> i64 {
        to48(self.t_end)
    }
}

#[derive(Clone, Debug)]
pub struct CtrlEvent {
    pub det: CtrlDetection,
    pub afc_hz: f64,
}

impl CtrlEvent {
    pub fn t_end48(&self) -> i64 {
        to48(self.det.t_end)
    }
}

pub type Event = crate::receiver::Event<CtrlEvent, FrameRx>;

pub fn to48(m: i64) -> i64 {
    m * DEC
}

pub(crate) enum Engine {
    Fsk(FskEngine),
    Psk(PskEngine),
    Ofdm(OfdmEngine),
}

impl Engine {
    fn new(bw: Bandwidth, record: Record, t: &RecordTables, psk: &[Vec<Complex64>], ofdm: &[Vec<Complex64>]) -> Self {
        match bw.level(record).waveform {
            Waveform::Fsk { .. } => Engine::Fsk(FskEngine::new(bw, record, &t.other)),
            Waveform::Psk { .. } => Engine::Psk(PskEngine::new(bw, record, &t.other, psk)),
            Waveform::Ofdm { .. } => Engine::Ofdm(OfdmEngine::new(bw, record, &t.other, ofdm)),
        }
    }

    pub fn span(&self) -> i64 {
        match self {
            Engine::Fsk(e) => e.span,
            Engine::Psk(e) => e.span,
            Engine::Ofdm(e) => e.span,
        }
    }

    pub fn tail(&self) -> i64 {
        match self {
            Engine::Psk(e) => e.tail,
            _ => 0,
        }
    }

    fn ofdm(&self) -> Option<&OfdmEngine> {
        if let Engine::Ofdm(e) = self { Some(e) } else { None }
    }

    fn is_psk(&self) -> bool {
        matches!(self, Engine::Psk(_))
    }

    fn step(&mut self, ctx: &Ctx) -> Vec<Detection> {
        match self {
            Engine::Fsk(e) => e.step(ctx),
            Engine::Psk(e) => e.step(ctx),
            Engine::Ofdm(e) => e.step(ctx),
        }
    }

    fn start_at(&mut self, start: i64) {
        match self {
            Engine::Fsk(e) => e.start_at(start),
            Engine::Psk(e) => e.start_at(start),
            Engine::Ofdm(e) => e.start_at(start),
        }
    }

    fn decode(&self, buf: &Track<Complex64>, dec: &FrameDecoder, t: i64, index: usize, k: Hint) -> Option<Decoded> {
        match self {
            Engine::Fsk(e) => e.decode(buf, dec, t, if let Hint::Fsk(d) = k { d } else { 0 }, index),
            Engine::Psk(e) => e.decode(buf, dec, t, index),
            Engine::Ofdm(e) => e.decode(buf, dec, t, if let Hint::Ofdm(fh) = k { fh } else { 0.0 }, index),
        }
    }
}

pub(crate) struct Wait {
    t1_end: i64,
    hyp: Hint,
    deadline: i64,
    confirm: Option<(i64, i64)>,
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub decode: DecodeStats,
    pub tx: usize,
    pub unexpected_frame2: usize,
}

pub struct Receiver {
    pub bw: Bandwidth,
    fe: FrontEnd,
    buf: Track<Complex64>,
    pub afc_hz: f64,
    ph: f64,
    pub linked: bool,
    pub ctrl_afc: bool,
    pub confirm_end: bool,
    engines: BTreeMap<Record, Engine>,
    asm: Assembly<FrameRx, Wait>,
    muted: Muted,
    ctrl: Option<CtrlDetector>,
    dec: FrameDecoder,
    ofdm_stfts: Vec<Stft>,
    pub stats: Stats,
}

impl Receiver {
    pub fn idle(bw: Bandwidth) -> Self {
        Self::new(bw, &[], 1, None)
    }

    pub fn new(bw: Bandwidth, records: &[Record], nframes: usize, ctrl: Option<&[CtrlEntry]>) -> Self {
        let mut rx = Self {
            bw,
            fe: FrontEnd::new(),
            buf: Track::new((BUF_SECONDS * FS3) as usize),
            afc_hz: 0.0,
            ph: 0.0,
            linked: false,
            ctrl_afc: true,
            confirm_end: true,
            engines: BTreeMap::new(),
            asm: Assembly::new(nframes),
            muted: Muted::default(),
            ctrl: ctrl.map(|e| CtrlDetector::new(bw, e)),
            dec: FrameDecoder::new(bw),
            ofdm_stfts: Stft::hypotheses(),
            stats: Stats::default(),
        };
        rx.expect(records, Some(nframes));
        rx
    }

    pub fn search_hz(&self) -> f64 {
        if self.linked { SEARCH_HZ_LINKED } else { SEARCH_HZ_UNLINKED }
    }

    pub fn expect(&mut self, records: &[Record], nframes: Option<usize>) {
        for &r in records {
            if !self.engines.contains_key(&r) {
                let mut e = Engine::new(self.bw, r, &self.dec.tables[r.index()], hf::psk_phasors(), hf::ofdm_phasors());
                let back = (self.buf.end() - e.span() - FS3 as i64).max(self.asm.done_until - 64 - FS3 as i64 / 4);
                e.start_at(self.buf.base.max(back));
                self.engines.insert(r, e);
            }
        }
        self.engines.retain(|r, _| records.contains(r));
        if let Some(n) = nframes {
            self.asm.nframes = n;
        }
    }

    pub fn baseband(&self) -> &Track<Complex64> {
        &self.buf
    }

    pub fn expected(&self) -> Vec<Record> {
        self.engines.keys().copied().collect()
    }

    pub fn set_ctrl_entries(&mut self, entries: Option<&[CtrlEntry]>) {
        match (entries, &mut self.ctrl) {
            (None, _) => self.ctrl = None,
            (Some(e), Some(c)) => c.set_entries(e),
            (Some(e), None) => {
                let mut c = CtrlDetector::new(self.bw, e);
                c.start_at(self.buf.base.max(self.buf.end() - 40 * 128));
                self.ctrl = Some(c);
            }
        }
    }

    pub fn mute(&mut self, a48: i64, b48: i64) {
        self.muted.mute(&self.buf, FS3, a48 / DEC, b48 / DEC);
    }

    pub fn mark_tx_end(&mut self, n48: i64) {
        self.asm.tx_end48 = Some(n48);
    }

    pub fn feed(&mut self, x: &[f64]) -> Vec<Event> {
        let (m0, b3) = self.fe.process(x);
        if !b3.is_empty() {
            let w = -2.0 * PI * self.afc_hz / FS3;
            let ph0 = self.ph;
            let v: Vec<Complex64> =
                b3.iter().enumerate().map(|(n, &b)| b * Complex64::from_polar(1.0, ph0 + w * n as f64)).collect();
            self.ph = (ph0 + w * b3.len() as f64).rem_euclid(2.0 * PI);
            self.buf.append(m0, v);
        }
        let mut ev = Vec::new();
        let linked = self.linked;
        if let Some(c) = &mut self.ctrl {
            for d in c.step(&self.buf, linked) {
                if self.ctrl_afc {
                    self.afc_hz += if linked { d.df_hz / 2.0 } else { d.df_hz };
                }
                ev.push(Event::Ctrl(CtrlEvent { det: d, afc_hz: self.afc_hz }));
            }
        }
        let nh = if linked { 1 } else { HYPOTHESES.len() };
        for (i, st) in self.ofdm_stfts.iter_mut().enumerate().take(nh) {
            let need = self.engines.values().filter_map(|e| e.ofdm()?.rows_needed(i, &self.buf));
            if let Some(r) = need.reduce(|a, b| (a.0.min(b.0), a.1.max(b.1))) {
                st.update(&self.buf, r);
            }
        }
        let ctx = Ctx { buf: &self.buf, linked, search_hz: self.search_hz(), ofdm: &self.ofdm_stfts };
        let mut dets: Vec<Detection> = self.engines.values_mut().flat_map(|e| e.step(&ctx)).collect();
        for (i, st) in self.ofdm_stfts.iter_mut().enumerate() {
            st.keep_from(self.engines.values().filter_map(|e| Some(e.ofdm()?.rows_kept(i))).min());
        }
        dets.sort_by_key(|d| d.t);
        for d in dets {
            ev.extend(self.on_det(d.record, d.t, d.hint));
        }
        ev.extend(self.pending());
        ev
    }

    pub fn flush(&mut self) -> Vec<Event> {
        self.finalize()
    }

    fn lead_samples(&self, record: Record) -> f64 {
        let l = self.bw.level(record);
        f64::from(l.lead_in()) * l.symbol_period() * FS3
    }

    fn max_extra(&self, record: Record) -> i64 {
        let e0 = &self.engines[&record];
        let kind = |r: Record| std::mem::discriminant(&self.bw.level(r).waveform);
        let mut ext = 0;
        for (&r, e) in &self.engines {
            let lead = if kind(r) == kind(record) {
                self.lead_samples(r) - self.lead_samples(record)
            } else {
                self.lead_samples(r)
            };
            ext = ext.max((e.span() as f64 + e.tail() as f64 + lead - e0.span() as f64 - e0.tail() as f64) as i64);
        }
        ext
    }

    pub fn noise_floor(&self) -> f64 {
        self.muted.quiet_blocks(&self.buf, FS3, 30).map_or(0.0, |p| percentile_sorted(&p, 5.0))
    }

    fn continues(&self, cur: &Pending<FrameRx, Wait>, (a, b): (i64, i64)) -> bool {
        let e = cur.frames[0].frame.t_end;
        continues(&self.buf, (a, b), (e - 300, e - 8), self.noise_floor())
    }

    fn pending(&mut self) -> Vec<Event> {
        let Some(mut cur) = self.asm.cur.take() else { return Vec::new() };
        let Some(e) = self.engines.get(&cur.record) else { return self.finish(cur) };
        let (span, tail) = (e.span(), e.tail());
        if let Some((mut a, b)) = cur.wait.confirm {
            while a < b && a + CONFIRM_LEN <= self.buf.end() {
                if !self.continues(&cur, (a, a + CONFIRM_LEN)) {
                    return self.finish(cur);
                }
                a += CONFIRM_LEN;
            }
            if a < b {
                cur.wait.confirm = Some((a, b));
                self.asm.cur = Some(cur);
                return Vec::new();
            }
            cur.wait.confirm = None;
            cur.two = true;
            cur.wait.deadline = cur.t + 2 * span + tail + GRACE;
            self.stats.unexpected_frame2 += 1;
        }
        if self.asm.nframes == 2 || cur.two {
            let t2 = cur.t + span;
            if self.buf.end() >= t2 + span + tail + FRAME2_LOOKAHEAD {
                return match self.decode(cur.record, t2, 1, cur.wait.hyp) {
                    Some(r) if r.frame.ok || self.asm.nframes == 2 => {
                        self.asm.cur = Some(cur);
                        self.close(r, true, Vec::new())
                    }
                    _ => self.finish(cur),
                };
            }
        }
        if self.buf.end() >= cur.wait.deadline {
            return self.finish(cur);
        }
        self.asm.cur = Some(cur);
        Vec::new()
    }
}

impl Assemble for Receiver {
    type Record = Record;
    type Frame = FrameRx;
    type Hint = Hint;
    type Ctrl = CtrlEvent;
    type Wait = Wait;
    const PAIR_TOL: i64 = 8;
    const EMIT_BAD: bool = true;
    const CLOSE_ON_MISS: bool = false;
    const LOST_NEEDS_OK: bool = false;

    fn asm(&self) -> &Assembly<FrameRx, Wait> {
        &self.asm
    }

    fn asm_mut(&mut self) -> &mut Assembly<FrameRx, Wait> {
        &mut self.asm
    }

    fn track(&self) -> &Track<Complex64> {
        &self.buf
    }

    fn has(&self, record: Record) -> bool {
        self.engines.contains_key(&record)
    }

    fn span(&self, record: Record) -> i64 {
        self.engines.get(&record).map_or(0, Engine::span)
    }

    fn decode(&mut self, record: Record, t: i64, index: usize, k: Hint) -> Option<FrameRx> {
        let e = self.engines.get(&record)?;
        let r = e.decode(&self.buf, &self.dec, t, index, k)?;
        self.stats.decode.count(r.ok);
        Some(FrameRx { frame: r, afc_hz: self.afc_hz, missing: false })
    }

    fn pairs(&self, cur: &Pending<FrameRx, Wait>, _: Record) -> bool {
        cur.frames[0].frame.ok || self.asm.nframes == 2 || cur.two
    }

    fn frame1_end(&self, cur: &Pending<FrameRx, Wait>) -> i64 {
        cur.wait.t1_end
    }

    fn holds(&self, cur: &Pending<FrameRx, Wait>, t: i64) -> bool {
        self.buf.end() < cur.wait.deadline && t < cur.wait.deadline
    }

    fn lost(&self, record: Record, back: i64, t: i64, _: i64) -> Option<FrameRx> {
        let te = self.asm.tx_end48? as f64 / DEC as f64;
        (back as f64 >= te + self.lead_samples(record) - 16.0).then(|| FrameRx {
            frame: Decoded { record, index: 0, t: back, t_end: t, ..Default::default() },
            afc_hz: self.afc_hz,
            missing: true,
        })
    }

    fn settled(&self, r: &FrameRx) -> bool {
        r.frame.ok && self.asm.nframes == 1 && !self.confirm_end
    }

    fn wait(&self, record: Record, r: &FrameRx, k: Hint) -> Wait {
        let e = &self.engines[&record];
        let (span, tail, psk) = (e.span(), e.tail(), e.is_psk());
        let t = r.frame.t;
        let nframes = self.asm.nframes;
        let body = if nframes == 2 { 2 * span } else { span };
        let deadline = t + body + tail + GRACE + if r.frame.ok { 0 } else { self.max_extra(record) };
        let confirm = (r.frame.ok && nframes == 1 && self.confirm_end).then(|| {
            let g = if psk { CONFIRM_GAP_PSK } else { CONFIRM_GAP };
            (r.frame.t_end + g, r.frame.t_end + g + CONFIRM_WINDOWS * CONFIRM_LEN)
        });
        Wait { t1_end: t + span, hyp: k, deadline, confirm }
    }

    fn expected_frames(&self, _: &Pending<FrameRx, Wait>) -> usize {
        self.asm.nframes
    }

    fn finished(&mut self, frames: &[FrameRx]) {
        let good: Vec<f64> = frames.iter().filter(|f| f.frame.ok).map(|f| f.frame.afc_res).collect();
        if !good.is_empty() {
            let res = good.iter().sum::<f64>() / good.len() as f64;
            self.afc_hz += if self.linked { res / 2.0 } else { res };
        }
        self.stats.tx += 1;
    }
}

fn percentile_sorted(p: &[f64], q: f64) -> f64 {
    let pos = q / 100.0 * (p.len() - 1) as f64;
    let i = pos.floor() as usize;
    let f = pos - i as f64;
    if i + 1 < p.len() { p[i] * (1.0 - f) + p[i + 1] * f } else { p[i] }
}
