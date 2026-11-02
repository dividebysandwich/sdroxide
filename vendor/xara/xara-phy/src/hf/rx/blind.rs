use super::ctrl::{HB_HZ, HOP, NBLK, NFFT, NS, ROWS, Stft, clamp_bin};
use crate::dsp::Track;
use crate::hf::ctrl::code::{self, BREAK, CONNECT_REPLY_2750, CONNECT_REQ_2300, KISS_L1, KISS_L2, KISS_L4};
use crate::hf::ctrl::{
    COMB, Codebook, Fmt, Role, SEED_OFF, SEED_OFF_LEVEL, SEED_OFF_REPLY_2750, book_off, bw_family, connect_off, family,
    family_comb, gen_code, req_seed_off,
};
use crate::hf::level::Bandwidth;
use crate::rnd::VbRnd;
use num_complex::Complex64;
use std::sync::OnceLock;

const OFFSETS: i64 = 10;
const STATES: u32 = 1 << 24;
const MAX_SEED_STEPS: u32 = 513;
const SESSION_CODES: u32 = 25;
const ANCHORS: [(u32, u32, u32); 3] = [
    (1, SEED_OFF, CONNECT_REQ_2300 - 1),
    (CONNECT_REPLY_2750, SEED_OFF_REPLY_2750, KISS_L2 - 1),
    (code::start_level_code(1), SEED_OFF_LEVEL, KISS_L4 - 1),
];
const TAU: usize = 1;

struct Tuning {
    fmt: Fmt,
    gate: f64,
    min_hits: usize,
    strong_hits: usize,
}

const TUNING: [Tuning; 2] = [
    Tuning { fmt: Fmt::F0, gate: 0.42, min_hits: 18, strong_hits: 22 },
    Tuning { fmt: Fmt::F1, gate: 0.5, min_hits: 11, strong_hits: 13 },
];

#[derive(Clone, Debug)]
pub struct Candidate {
    pub fmt: Fmt,
    pub min_hits: usize,
    pub strong_hits: usize,
    pub family: u32,
    pub t_end: i64,
    pub df_hz: f64,
    pub tones: Vec<u32>,
}

impl Candidate {
    pub fn t_start(&self) -> i64 {
        self.t_end - self.fmt.params().nsym as i64 * NS as i64
    }
}

struct Gate {
    tuning: &'static Tuning,
    family: u32,
    hold: i64,
    best: Option<(i64, i64, f64)>,
    block: i64,
    next: Option<i64>,
}

impl Gate {
    fn push(&mut self, j: i64, metric: f64, o: i64) -> Option<(i64, i64)> {
        let out = self.flush_before(j);
        if j < self.block || metric < self.tuning.gate {
            return out;
        }
        match &mut self.best {
            Some(b) if metric > b.2 => *b = (j, o, metric),
            Some(b) if metric >= b.2 - 0.5 / (self.tuning.fmt.params().nsym - 1) as f64 => *b = (j, o, b.2),
            Some(_) => {}
            None => self.best = Some((j, o, metric)),
        }
        out
    }

    fn flush_before(&mut self, j: i64) -> Option<(i64, i64)> {
        match self.best {
            Some((jj, o, _)) if j - jj > self.hold => {
                self.best = None;
                self.block = jj + ROWS * (self.tuning.fmt.params().nsym as i64 - 1);
                Some((jj, o))
            }
            _ => None,
        }
    }
}

pub struct BlindDetector {
    stft: Stft,
    gates: Vec<Gate>,
}

fn comb(v: u32) -> Vec<u32> {
    let (n, off) = family_comb(v);
    (0..=n).map(|i| i * COMB + off).collect()
}

fn best_tone(row: &super::ctrl::Spectra, bases: &[u32], par: u32, o: i64) -> (u32, f64) {
    let mut best = (0, -1.0f32, 0usize);
    for &b in bases {
        let hb = clamp_bin(2 * (b as i64) + o, NBLK);
        for i in 0..7 {
            let t = b + par + 2 * i;
            let v = row.power[clamp_bin(2 * t as i64 + o, NFFT)];
            if v > best.1 {
                best = (t, v, hb);
            }
        }
    }
    (best.0, f64::from(best.1 / (row.comb_sum[best.2] + 1e-30)))
}

impl BlindDetector {
    pub fn new(bw: Bandwidth) -> Self {
        let families: &[u32] = match bw {
            Bandwidth::Hz500 => &[1],
            Bandwidth::Hz2300 => &[2],
            Bandwidth::Hz2750 => &[2, 3],
        };
        let gates = families
            .iter()
            .flat_map(|&v| {
                TUNING.iter().map(move |tuning| Gate {
                    tuning,
                    family: v,
                    hold: 4 * (tuning.fmt.params().nsym as i64 - 1),
                    best: None,
                    block: i64::MIN,
                    next: None,
                })
            })
            .collect();
        Self { stft: Stft::new(ROWS as usize * 80), gates }
    }

    pub fn start_at(&mut self, start: i64) {
        self.stft.start_at(start);
        for g in &mut self.gates {
            g.next = None;
        }
    }

    fn tones(&self, fmt: Fmt, v: u32, j: i64, o: i64) -> (Vec<u32>, f64) {
        let nd = fmt.params().nsym as usize - 1;
        let bases = comb(v);
        let mut sum = 0.0;
        let tau = TAU as i64;
        let tones = (0..nd as i64 + 2 * tau)
            .map(|k| {
                let row = self.stft.spec.at(j - ROWS * (nd as i64 - 1 + tau - k));
                let par = u32::from((k - tau) % 2 == 0 && v != 1);
                let (t, r) = best_tone(row, &bases, par, o);
                if (tau..nd as i64 + tau).contains(&k) {
                    sum += r;
                }
                t
            })
            .collect();
        (tones, sum / nd as f64)
    }

    pub fn step(&mut self, buf: &Track<Complex64>) -> Vec<Candidate> {
        self.stft.update(buf);
        let mut out = Vec::new();
        let (base, end) = (self.stft.spec.base, self.stft.spec.end());
        for gi in 0..self.gates.len() {
            let (tuning, v) = (self.gates[gi].tuning, self.gates[gi].family);
            let fmt = tuning.fmt;
            let nd = fmt.params().nsym as i64 - 1;
            let lo = self.gates[gi].next.unwrap_or(base + ROWS * (nd + 1)).max(base + ROWS * (nd + 1));
            for j in lo..end - 16 {
                let best = (-OFFSETS..=OFFSETS)
                    .map(|o| (self.tones(fmt, v, j, o).1, o))
                    .max_by(|a, b| a.0.total_cmp(&b.0))
                    .unwrap();
                if let Some((jj, o)) = self.gates[gi].push(j, best.0, best.1) {
                    let (tones, _) = self.tones(fmt, v, jj, o);
                    out.push(Candidate {
                        fmt,
                        min_hits: tuning.min_hits,
                        strong_hits: tuning.strong_hits,
                        family: v,
                        t_end: jj * HOP + NS as i64,
                        df_hz: o as f64 * HB_HZ,
                        tones,
                    });
                }
            }
            self.gates[gi].next = Some(lo.max(end - 16));
        }
        out
    }
}

const SHIFT: i64 = 5;
const NSHIFT: usize = 2 * SHIFT as usize + 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Found {
    pub state: u32,
    pub hits: usize,
    pub dt_sym: i64,
    pub df_bins: i64,
}

fn hits_from(state: u32, tones: &[u32], nd: usize, v: u32, max_miss: usize) -> Option<Found> {
    let (n, off) = family_comb(v);
    let mut r = VbRnd::from_state(state);
    let mut hits = [[0u8; NSHIFT]; 2 * TAU + 1];
    let mut best = 0usize;
    for k in 0..nd {
        let b = r.rand_int(n) * COMB + off;
        let g = (b + u32::from(k % 2 == 0 && v != 1) + 2 * r.rand_int(6)) as i64;
        for (tau, h) in hits.iter_mut().enumerate() {
            let d = tones[k + tau] as i64 - g + SHIFT;
            if (0..NSHIFT as i64).contains(&d) {
                let c = &mut h[d as usize];
                *c += 1;
                best = best.max(usize::from(*c));
            }
        }
        if k + 1 - best > max_miss {
            return None;
        }
    }
    let (tau, d, h) = (0..2 * TAU + 1)
        .flat_map(|t| (0..NSHIFT).map(move |d| (t, d)))
        .map(|(t, d)| (t, d, usize::from(hits[t][d])))
        .max_by_key(|x| x.2)
        .unwrap();
    Some(Found { state, hits: h, dt_sym: tau as i64 - TAU as i64, df_bins: d as i64 - SHIFT })
}

pub fn match_state(state: u32, tones: &[u32], fmt: Fmt, v: u32, min_hits: usize) -> Option<Found> {
    let nd = fmt.params().nsym as usize - 1;
    hits_from(state, tones, nd, v, nd.saturating_sub(min_hits))
}

pub fn search_states(tones: &[u32], fmt: Fmt, v: u32, min_hits: usize) -> Vec<Found> {
    let nd = fmt.params().nsym as usize - 1;
    assert_eq!(tones.len(), nd + 2 * TAU, "tones span TAU extra symbols each side");
    let max_miss = nd.saturating_sub(min_hits);
    let threads = crate::dsp::cores() as u32;
    let chunk = STATES.div_ceil(threads);
    let mut found: Vec<Found> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..threads)
            .map(|i| {
                sc.spawn(move || {
                    let (a, b) = (i * chunk, ((i + 1) * chunk).min(STATES));
                    (a..b).filter_map(|s| hits_from(s, tones, nd, v, max_miss)).collect::<Vec<_>>()
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    found.sort_by_key(|f| (std::cmp::Reverse(f.hits), f.state));
    let near = |a: u32, b: u32| {
        let (mut x, mut y) = (VbRnd::from_state(a), VbRnd::from_state(a));
        (0..10).any(|_| {
            x.advance();
            y.retreat();
            x.state() == b || y.state() == b
        })
    };
    let mut keep: Vec<Found> = Vec::new();
    for f in found {
        if !keep.iter().any(|k| near(k.state, f.state)) {
            keep.push(f);
        }
    }
    keep
}

fn seeded_states() -> &'static [(u32, u32)] {
    static TAB: OnceLock<Vec<(u32, u32)>> = OnceLock::new();
    TAB.get_or_init(|| {
        let mut t: Vec<(u32, u32)> = (0..32768u32).map(|r| (VbRnd::stream(f64::from(r)).state(), r)).collect();
        t.sort_unstable();
        t
    })
}

pub fn seed_preimages(targets: &[u32]) -> Vec<(usize, u32)> {
    let tab = seeded_states();
    let mut out = Vec::new();
    for (i, &t) in targets.iter().enumerate() {
        let mut g = VbRnd::from_state(t);
        for k in 0..MAX_SEED_STEPS {
            g.retreat();
            let s = g.state();
            let lo = tab.partition_point(|e| e.0 < s);
            out.extend(tab[lo..].iter().take_while(|e| e.0 == s).map(|&(_, r)| (i, r + 32768 * k)));
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    Answerer(u32),
    Caller(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hypothesis {
    pub code: u32,
    pub sender: Role,
    pub key: Key,
}

fn draws_per_code(fmt: Fmt) -> u32 {
    2 * (fmt.params().nsym - 1)
}

pub fn hypotheses(state: u32, fmt: Fmt, v: u32, bw: Bandwidth) -> Vec<Hypothesis> {
    if fmt.params().ntones != 1 {
        return Vec::new();
    }
    let mut targets = Vec::new();
    let mut r = VbRnd::from_state(state);
    for _ in 0..SESSION_CODES {
        targets.push(r.state());
        for _ in 0..draws_per_code(fmt) {
            r.retreat();
        }
    }
    let f = fmt.index() as u32;
    let mut hs = Vec::new();
    for (k, x) in seed_preimages(&targets) {
        let k = k as u32;
        let mut add = |code: u32, sender: Role, key: Option<Key>| {
            if let Some(key) = key {
                hs.push(Hypothesis { code, sender, key });
            }
        };
        if k == 0 && fmt == Fmt::F0 {
            for b in [Bandwidth::Hz2300, Bandwidth::Hz500, Bandwidth::Hz2750] {
                let code = code::connect_req(b, 0);
                if family(code, bw) == v {
                    let d = book_off(Role::Caller) + req_seed_off(b);
                    add(code, Role::Caller, x.checked_sub(d).map(Key::Answerer));
                }
            }
        }
        if k <= 1 && v == bw_family(bw) {
            for sender in [Role::Caller, Role::Answerer] {
                add(BREAK + k, sender, x.checked_sub(f + connect_off(sender)).map(Key::Caller));
            }
        }
        for (c0, seed_add, last) in ANCHORS {
            let code = c0 + k;
            if code <= last && code != KISS_L1 && family(code, bw) == v {
                for sender in [Role::Caller, Role::Answerer] {
                    add(code, sender, x.checked_sub(seed_add + f + book_off(sender)).map(Key::Answerer));
                }
            }
        }
    }
    let want = gen_code(&mut VbRnd::from_state(state), fmt, v);
    hs.retain(|h| codeword(h, fmt, bw) == want);
    hs.sort_by_key(|h| (h.key, h.code));
    hs.dedup();
    hs
}

pub fn codeword(h: &Hypothesis, fmt: Fmt, bw: Bandwidth) -> crate::hf::ctrl::Codeword {
    let book = match h.key {
        Key::Answerer(a) => Codebook::new(a, [0; 5], h.sender, bw),
        Key::Caller(c) => {
            let mut b = Codebook::new(0, [0; 5], h.sender, bw);
            b.set_connect_codes(c, h.sender);
            b
        }
    };
    book.tx(fmt, h.code).clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call::call_hash;

    fn observe(w: &[crate::hf::ctrl::Symbol], df: i64, dt: i64) -> Vec<u32> {
        let mut t: Vec<u32> = w.iter().map(|s| (s[0] as i64 + df) as u32).collect();
        t.insert(0, 40);
        t.push(60);
        match dt {
            1 => {
                t.insert(0, 50);
                t.pop();
            }
            -1 => {
                t.remove(0);
                t.push(70);
            }
            _ => {}
        }
        t
    }

    #[test]
    fn connect_request_round_trip() {
        let bw = Bandwidth::Hz2300;
        let a = call_hash(b"N1CALL");
        let cb = Codebook::new(a, [0; 5], Role::Caller, bw);
        let mut noisy = observe(cb.tx(Fmt::F0, 26), 2, 0);
        for t in noisy.iter_mut().step_by(3) {
            *t += 2;
        }
        let found = search_states(&noisy, Fmt::F0, 2, TUNING[0].min_hits);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].dt_sym, found[0].df_bins), (0, 2));
        let hs = hypotheses(found[0].state, Fmt::F0, 2, bw);
        assert!(hs.contains(&Hypothesis { code: 26, sender: Role::Caller, key: Key::Answerer(a) }), "{hs:?}");
    }

    #[test]
    fn session_and_connect_codes() {
        let bw = Bandwidth::Hz2300;
        let (a, c) = (call_hash(b"N1CALL"), call_hash(b"N0CALL"));
        let ans = Codebook::new(a, [0; 5], Role::Answerer, bw);
        let f = search_states(&observe(ans.tx(Fmt::F1, 18), 0, 1), Fmt::F1, 2, TUNING[1].min_hits)[0];
        assert_eq!((f.dt_sym, f.df_bins), (1, 0));
        let s = f.state;
        assert!(hypotheses(s, Fmt::F1, 2, bw).contains(&Hypothesis {
            code: 18,
            sender: Role::Answerer,
            key: Key::Answerer(a)
        }));
        let mut cal = Codebook::new(a, [0; 5], Role::Caller, bw);
        cal.set_connect_codes(c, Role::Caller);
        for code in [49, 50] {
            let s = search_states(&observe(cal.tx(Fmt::F0, code), 0, -1), Fmt::F0, 2, TUNING[0].min_hits)[0].state;
            assert!(hypotheses(s, Fmt::F0, 2, bw).contains(&Hypothesis {
                code,
                sender: Role::Caller,
                key: Key::Caller(c)
            }));
        }
    }

    #[test]
    fn seed_preimages_match_forward_scan() {
        let mut targets: Vec<u32> = [0u64, 1, 292, 32767, 32768, 50 + 41 * 32768, 1_000_000, 32768 * 513 - 1]
            .iter()
            .map(|&x| {
                let mut g = VbRnd::new();
                g.seed_rnd_wide(x);
                g.state()
            })
            .collect();
        targets.extend([0x000000, 0x123456, 0xFFFFFF, 0x395886]);
        let mut want = Vec::new();
        for r in 0..32768u32 {
            let mut g = VbRnd::stream(f64::from(r));
            for k in 0..MAX_SEED_STEPS {
                g.advance();
                for (i, _) in targets.iter().enumerate().filter(|&(_, &t)| t == g.state()) {
                    want.push((i, r + 32768 * k));
                }
            }
        }
        let mut got = seed_preimages(&targets);
        got.sort_unstable();
        want.sort_unstable();
        assert!(want.len() >= 8);
        assert_eq!(got, want);
    }
}
