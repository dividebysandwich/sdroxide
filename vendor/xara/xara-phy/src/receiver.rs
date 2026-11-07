use crate::dsp::Track;
use num_complex::Complex64;

#[derive(Clone, Debug, Default)]
pub struct DecodeStats {
    pub decodes: usize,
    pub crc_ok: usize,
}

impl DecodeStats {
    pub(crate) fn count(&mut self, ok: bool) {
        self.decodes += 1;
        self.crc_ok += usize::from(ok);
    }
}

fn mean_power(buf: &Track<Complex64>, (a, b): (i64, i64)) -> f64 {
    let v = buf.get(a, b);
    v.iter().map(|z| z.norm_sqr()).sum::<f64>() / v.len().max(1) as f64
}

pub(crate) fn continues(buf: &Track<Complex64>, after: (i64, i64), frame: (i64, i64), noise: f64) -> bool {
    let (after, frame) = (mean_power(buf, after), mean_power(buf, frame));
    after - noise > 0.3 * (frame - noise).max(1e-30) && frame > 4.0 * noise
}

#[derive(Default)]
pub(crate) struct Muted(Vec<(i64, i64)>);

impl Muted {
    pub fn mute(&mut self, buf: &Track<Complex64>, fs: f64, a: i64, b: i64) {
        let keep = buf.end() - 8 * fs as i64;
        self.0.retain(|&(_, e)| e > keep);
        self.0.push((a - 64, b + 64));
    }

    fn overlaps(&self, a: i64, b: i64) -> bool {
        self.0.iter().any(|&(ma, mb)| a < mb && b > ma)
    }

    pub fn quiet_blocks(&self, buf: &Track<Complex64>, fs: f64, block: usize) -> Option<Vec<f64>> {
        let from = buf.base.max(buf.end() - 6 * fs as i64);
        let nb = block as i64;
        let mut p: Vec<f64> = buf
            .get(from, buf.end())
            .chunks_exact(block)
            .enumerate()
            .filter(|&(i, _)| !self.overlaps(from + nb * i as i64, from + nb * (i as i64 + 1)))
            .map(|(_, c)| c.iter().map(|z| z.norm_sqr()).sum::<f64>() / block as f64)
            .collect();
        if p.len() < 4 {
            return None;
        }
        p.sort_by(f64::total_cmp);
        Some(p)
    }
}

pub trait Frame: Clone {
    type Record: Copy + PartialEq + std::fmt::Debug;
    fn ok(&self) -> bool;
    fn t(&self) -> i64;
    fn t_end(&self) -> i64;
}

#[derive(Clone, Debug)]
pub struct Transmission<F: Frame> {
    pub record: F::Record,
    pub expected: usize,
    pub complete: bool,
    pub t_end: i64,
    pub frames: Vec<F>,
}

impl<F: Frame> Transmission<F> {
    pub fn ok(&self) -> Vec<bool> {
        self.frames.iter().map(F::ok).collect()
    }
}

#[derive(Clone, Debug)]
pub enum Event<C, F: Frame> {
    Ctrl(C),
    Frame(F),
    Tx(Transmission<F>),
}

pub(crate) struct Pending<F: Frame, W> {
    pub record: F::Record,
    pub t: i64,
    pub frames: Vec<F>,
    pub two: bool,
    pub wait: W,
}

pub(crate) struct Assembly<F: Frame, W> {
    pub nframes: usize,
    pub cur: Option<Pending<F, W>>,
    pub done_until: i64,
    pub tx_end48: Option<i64>,
}

impl<F: Frame, W> Assembly<F, W> {
    pub fn new(nframes: usize) -> Self {
        Self { nframes, cur: None, done_until: i64::MIN / 2, tx_end48: None }
    }
}

type Cur<A> = Pending<<A as Assemble>::Frame, <A as Assemble>::Wait>;
type Out<A> = Vec<Event<<A as Assemble>::Ctrl, <A as Assemble>::Frame>>;

pub(crate) trait Assemble: Sized {
    type Record: Copy + PartialEq + std::fmt::Debug;
    type Frame: Frame<Record = Self::Record>;
    type Hint: Copy;
    type Ctrl;
    type Wait;
    const PAIR_TOL: i64;
    const EMIT_BAD: bool;
    const CLOSE_ON_MISS: bool;
    const LOST_NEEDS_OK: bool;

    fn asm(&self) -> &Assembly<Self::Frame, Self::Wait>;
    fn asm_mut(&mut self) -> &mut Assembly<Self::Frame, Self::Wait>;
    fn track(&self) -> &Track<Complex64>;
    fn has(&self, record: Self::Record) -> bool;
    fn span(&self, record: Self::Record) -> i64;
    fn decode(&mut self, record: Self::Record, t: i64, index: usize, hint: Self::Hint) -> Option<Self::Frame>;
    fn pairs(&self, cur: &Cur<Self>, record: Self::Record) -> bool;
    fn frame1_end(&self, cur: &Cur<Self>) -> i64;
    fn lost(&self, record: Self::Record, back: i64, t: i64, span: i64) -> Option<Self::Frame>;
    fn wait(&self, record: Self::Record, r: &Self::Frame, hint: Self::Hint) -> Self::Wait;
    fn expected_frames(&self, cur: &Cur<Self>) -> usize;
    fn finished(&mut self, frames: &[Self::Frame]);

    fn holds(&self, _cur: &Cur<Self>, _t: i64) -> bool {
        false
    }

    fn keep(&mut self, _r: &Self::Frame) -> bool {
        true
    }

    fn settled(&self, _r: &Self::Frame) -> bool {
        false
    }

    fn on_det(&mut self, record: Self::Record, t: i64, hint: Self::Hint) -> Out<Self> {
        if !self.has(record) || t < self.asm().done_until - 64 {
            return Vec::new();
        }
        let span = self.span(record);
        let mut ev = Vec::new();
        if let Some(cur) = &self.asm().cur {
            if cur.record == record && (t - (cur.t + span)).abs() <= Self::PAIR_TOL && self.pairs(cur, record) {
                return self.frame2(record, t, hint, ev);
            }
            if t < self.frame1_end(cur) - 64 {
                if cur.frames[0].ok() {
                    return ev;
                }
                let Some(r) = self.decode(record, t, 0, hint) else { return ev };
                if r.ok() || Self::EMIT_BAD {
                    ev.push(Event::Frame(r.clone()));
                }
                if r.ok() {
                    self.begin(record, r, hint, &mut ev);
                }
                return ev;
            }
            if self.holds(cur, t) {
                return ev;
            }
            ev.extend(self.finalize());
        }
        if self.asm().nframes == 2 {
            let back = t - span;
            if back - 8 >= self.track().base && t > self.asm().done_until {
                if let Some(rb) = self.decode(record, back, 0, hint).filter(|r| r.ok()) {
                    ev.push(Event::Frame(rb.clone()));
                    self.start(record, rb, hint);
                    return self.frame2(record, t, hint, ev);
                }
            }
            if let Some(lost) = self.lost(record, back, t, span) {
                match self.decode(record, t, 1, hint) {
                    Some(r) if r.ok() || !Self::LOST_NEEDS_OK => {
                        self.start(record, lost, hint);
                        return self.close(r, true, ev);
                    }
                    _ if !Self::LOST_NEEDS_OK => return ev,
                    _ => {}
                }
            }
        }
        let Some(r) = self.decode(record, t, 0, hint) else { return ev };
        if !self.keep(&r) {
            return ev;
        }
        if r.ok() || Self::EMIT_BAD {
            ev.push(Event::Frame(r.clone()));
        }
        self.begin(record, r, hint, &mut ev);
        ev
    }

    fn begin(&mut self, record: Self::Record, r: Self::Frame, hint: Self::Hint, ev: &mut Out<Self>) {
        let settled = self.settled(&r);
        self.start(record, r, hint);
        if settled {
            ev.extend(self.finalize());
        }
    }

    fn start(&mut self, record: Self::Record, r: Self::Frame, hint: Self::Hint) {
        let wait = self.wait(record, &r, hint);
        let a = self.asm_mut();
        a.cur = Some(Pending { record, t: r.t(), frames: vec![r], two: a.nframes == 2, wait });
    }

    fn frame2(&mut self, record: Self::Record, t: i64, hint: Self::Hint, mut ev: Out<Self>) -> Out<Self> {
        match self.decode(record, t, 1, hint) {
            Some(r) => self.close(r, true, ev),
            None if Self::CLOSE_ON_MISS => {
                ev.extend(self.finalize());
                ev
            }
            None => ev,
        }
    }

    fn close(&mut self, r: Self::Frame, emit_bad: bool, mut ev: Out<Self>) -> Out<Self> {
        let Some(mut cur) = self.asm_mut().cur.take() else { return ev };
        if r.ok() || emit_bad {
            ev.push(Event::Frame(r.clone()));
        }
        cur.frames.push(r);
        ev.extend(self.finish(cur));
        ev
    }

    fn finalize(&mut self) -> Out<Self> {
        match self.asm_mut().cur.take() {
            Some(cur) => self.finish(cur),
            None => Vec::new(),
        }
    }

    fn finish(&mut self, cur: Cur<Self>) -> Out<Self> {
        let span = self.span(cur.record);
        let last = &cur.frames[cur.frames.len() - 1];
        self.asm_mut().done_until = last.t() + span - 32;
        let t_end = last.t_end();
        let expected = self.expected_frames(&cur);
        let complete = cur.frames.iter().all(|f| f.ok()) && cur.frames.len() >= expected;
        self.finished(&cur.frames);
        vec![Event::Tx(Transmission { record: cur.record, expected, complete, t_end, frames: cur.frames })]
    }
}
