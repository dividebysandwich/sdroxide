use super::{CrcCheck, Decoded, Split};
use crate::dsp::cores;
use std::borrow::Cow;
use std::ops::Range;
use std::sync::Barrier;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

pub(crate) const LEN: usize = 1024;
pub(crate) const SEGMENTS: usize = 4;

pub(crate) trait Kernel: Sync {
    type V: Copy + Default + PartialOrd + Send + Sync;
    type S: Copy + Send + Sync;
    type C: Copy + Send;
    fn term(&self) -> Self::S;
    fn zero(&self) -> Self::S;
    fn cell(&self) -> Self::C;
    fn quant<'a>(&self, v: &'a [f64]) -> Cow<'a, [Self::V]>;
    fn add(&self, a: Self::V, b: Self::V) -> Self::V;
    fn alpha(&self, x: &[Self::V], p: &[Self::V], a: Self::S, cells: &mut [Self::C]) -> Self::S;
    fn beta(&self, x: &[Self::V], p: &[Self::V], b: Self::S, cells: &[Self::C], out: &mut [Self::V]) -> Self::S;
}

struct Sh<T>(*mut T, usize);

// SAFETY: workers only touch disjoint ranges, or read, between barriers.
unsafe impl<T: Send + Sync> Sync for Sh<T> {}

impl<T> Sh<T> {
    fn new(v: &mut [T]) -> Self {
        Self(v.as_mut_ptr(), v.len())
    }

    unsafe fn all(&self) -> &[T] {
        // SAFETY: the caller keeps the slice alive and overlaps no part() it holds.
        unsafe { std::slice::from_raw_parts(self.0, self.1) }
    }

    #[allow(clippy::mut_from_ref)]
    unsafe fn part(&self, r: Range<usize>) -> &mut [T] {
        assert!(r.start <= r.end && r.end <= self.1);
        // SAFETY: r is in bounds and the caller keeps it unaliased for the borrow.
        unsafe { std::slice::from_raw_parts_mut(self.0.add(r.start), r.len()) }
    }
}

#[allow(clippy::too_many_arguments)]
fn segment<K: Kernel>(
    kern: &K,
    k: usize,
    t_len: usize,
    (s0, s1): (usize, usize),
    x: &[K::V],
    lp: &[K::V],
    mut a: K::S,
    pb_old: &[K::S],
    pb_new: &mut [K::S],
    out: &mut [K::V],
    cells: &mut [K::C],
) -> K::S {
    let mut s = s0;
    while s < s1 {
        let e = (s + LEN).min(s1);
        let (xw, pw, cw) = (&x[s - s0..e - s0], &lp[s - s0..e - s0], &mut cells[..e - s]);
        let next = kern.alpha(xw, pw, a, cw);
        let b = if e == t_len { kern.term() } else { pb_old[e / LEN] };
        let o = (s - s0).min(out.len());
        let kk = k.min(e).saturating_sub(s);
        pb_new[(s - s0) / LEN] = kern.beta(xw, pw, b, cw, &mut out[o..o + kk]);
        a = next;
        s = e;
    }
    a
}

pub(crate) fn run<K: Kernel>(
    kern: &K,
    sp: &Split,
    perm: &[u32],
    iters: usize,
    early: Option<(usize, &CrcCheck)>,
) -> Decoded {
    let k = perm.len();
    let t_len = k + 3;
    let ls = [kern.quant(&sp.ls), kern.quant(&sp.ls2)];
    let lp = [kern.quant(&sp.lp1), kern.quant(&sp.lp2)];
    let nwin = t_len.div_ceil(LEN);
    let seg = nwin.div_ceil(SEGMENTS) * LEN;
    let nseg = t_len.div_ceil(seg);
    let nthr = nseg.min(cores());
    let mut inv = vec![0u32; k];
    for (i, &j) in perm.iter().enumerate() {
        inv[j as usize] = i as u32;
    }
    let zero = K::V::default();
    let mut ext = [vec![zero; k], vec![zero; k]];
    let mut le21 = vec![zero; k];
    let mut bits = vec![0u8; k];
    let mut pb: [[Vec<K::S>; 2]; 2] = std::array::from_fn(|_| std::array::from_fn(|_| vec![kern.zero(); nwin + 1]));
    let mut ap: [[Vec<K::S>; 2]; 2] = std::array::from_fn(|_| std::array::from_fn(|_| vec![kern.zero(); nseg + 1]));
    let sh_ext = ext.each_mut().map(|v| Sh::new(v));
    let sh_le = Sh::new(&mut le21);
    let sh_bits = Sh::new(&mut bits);
    let sh_pb = pb.each_mut().map(|p| p.each_mut().map(|v| Sh::new(v)));
    let sh_ap = ap.each_mut().map(|p| p.each_mut().map(|v| Sh::new(v)));
    let barrier = Barrier::new(nthr);
    let (stop, diff, ran) = (AtomicBool::new(false), AtomicBool::new(false), AtomicUsize::new(0));
    let worker = |w: usize| {
        let mut x = vec![zero; seg];
        let mut cells = vec![kern.cell(); LEN];
        for it in 1..=iters {
            let (old, new) = ((it + 1) % 2, it % 2);
            for d in 0..2 {
                for i in (w..nseg).step_by(nthr) {
                    let (s0, s1) = (i * seg, ((i + 1) * seg).min(t_len));
                    let xs = &mut x[..s1 - s0];
                    // SAFETY: segment i owns ext[d][s0..s1], pb[d][new] windows s0..s1 and ap[d][new][i + 1].
                    unsafe {
                        let src = if d == 0 { sh_le.all() } else { sh_ext[0].all() };
                        for (t, v) in (s0..s1).zip(xs.iter_mut()) {
                            *v = if t >= k {
                                ls[d][t]
                            } else if d == 0 {
                                kern.add(ls[0][t], src[t])
                            } else {
                                kern.add(ls[1][t], src[perm[t] as usize])
                            };
                        }
                        let a0 = if i == 0 { kern.term() } else { sh_ap[d][old].all()[i] };
                        let a = segment(
                            kern,
                            k,
                            t_len,
                            (s0, s1),
                            xs,
                            &lp[d][s0..s1],
                            a0,
                            sh_pb[d][old].all(),
                            sh_pb[d][new].part(s0 / LEN..s1.div_ceil(LEN)),
                            sh_ext[d].part(s0.min(k)..s1.min(k)),
                            &mut cells,
                        );
                        if i + 1 < nseg {
                            sh_ap[d][new].part(i + 1..i + 2)[0] = a;
                        }
                    }
                }
                barrier.wait();
            }
            let r = w * k / nthr..(w + 1) * k / nthr;
            // SAFETY: worker w owns le21[r] and bits[r]; ext is read-only here.
            unsafe {
                let (e1, e2) = (sh_ext[0].all(), sh_ext[1].all());
                let (le, bt) = (sh_le.part(r.clone()), sh_bits.part(r.clone()));
                let mut changed = false;
                for (o, j) in r.enumerate() {
                    le[o] = e2[inv[j] as usize];
                    let b = u8::from(kern.add(kern.add(ls[0][j], e1[j]), le[o]) < zero);
                    changed |= b != bt[o];
                    bt[o] = b;
                }
                if changed {
                    diff.store(true, Relaxed);
                }
            }
            barrier.wait();
            if w == 0 {
                ran.store(it, Relaxed);
                let same = !diff.swap(false, Relaxed);
                // SAFETY: no worker writes bits until the next barrier.
                let done = early
                    .is_some_and(|(min, check)| it >= min && it < iters && same && check.ok(unsafe { sh_bits.all() }));
                stop.store(done, Relaxed);
            }
            barrier.wait();
            if stop.load(Relaxed) {
                break;
            }
        }
    };
    std::thread::scope(|s| {
        for w in 1..nthr {
            s.spawn(move || worker(w));
        }
        worker(0);
    });
    Decoded { bits, iterations: ran.into_inner() }
}
