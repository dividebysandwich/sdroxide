use crate::bits::Bits;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Clone, Debug)]
struct Entry {
    phrase: Vec<u8>,
    code: Vec<bool>,
    next: usize,
}

#[derive(Clone, Debug)]
enum Miss {
    Escape(Vec<bool>, usize),
    Byte(Box<[usize; 256]>),
}

#[derive(Clone, Copy, Debug)]
enum Hit {
    Entry(usize),
    Escape(usize),
}

#[derive(Clone, Debug)]
struct StateTab {
    start: Vec<bool>,
    entries: Vec<Entry>,
    miss: Miss,
    decode: HashMap<(usize, u64), Hit>,
}

fn bits(s: &str) -> Vec<bool> {
    s.bytes().map(|b| b == b'1').collect()
}

fn key(code: &[bool]) -> (usize, u64) {
    (code.len(), code.iter().fold(0u64, |a, &b| a << 1 | u64::from(b)))
}

type Parsed = (Vec<bool>, Vec<Entry>, Option<(Vec<bool>, usize)>);

fn tables() -> &'static [StateTab; 8] {
    static T: OnceLock<[StateTab; 8]> = OnceLock::new();
    T.get_or_init(|| {
        let mut raw: [Parsed; 8] = Default::default();
        let mut cur = 0;
        for line in include_str!("textcomp.dict").lines() {
            let f: Vec<&str> = line.split(' ').collect();
            match f[0] {
                "S" => {
                    cur = f[1].parse::<usize>().unwrap() - 1;
                    raw[cur].0 = bits(f[2]);
                    raw[cur].2 = (f[3] != "-").then(|| (bits(f[3]), f[4].parse().unwrap()));
                }
                _ => {
                    let phrase = if f[1] == "-" { Vec::new() } else { hex(f[1]) };
                    raw[cur].1.push(Entry { phrase, code: bits(f[2]), next: f[3].parse().unwrap() });
                }
            }
        }
        raw.map(|(start, entries, escape)| {
            let miss = match escape {
                Some((code, next)) => Miss::Escape(code, next),
                None => Miss::Byte(Box::new(std::array::from_fn(|b| {
                    entries
                        .iter()
                        .position(|e| e.phrase == [b as u8])
                        .expect("textcomp.dict: state without escape lacks a byte")
                }))),
            };
            let mut decode = HashMap::new();
            for (i, e) in entries.iter().enumerate() {
                decode.entry(key(&e.code)).or_insert(Hit::Entry(i));
            }
            if let Miss::Escape(c, next) = &miss {
                decode.entry(key(c)).or_insert(Hit::Escape(*next));
            }
            StateTab { start, entries, miss, decode }
        })
    })
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

pub const START_STATE: usize = 1;
pub(crate) const MAX_PHRASE: usize = 39;
pub(crate) const TRIAL_BYTES: usize = 150;
const MAX_CODE: usize = 10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encoded {
    pub bytes: Vec<u8>,
    pub consumed: usize,
    pub end_state: usize,
}

pub fn compress(data: &[u8], cap: usize, state: usize) -> Encoded {
    let t = tables();
    let mut state = if (1..=t.len()).contains(&state) { state } else { START_STATE };
    let mut out = t[state - 1].start.clone();
    let (mut consumed, mut pos) = (0, 0);
    let mut last: Option<(usize, usize, usize)> = None;
    while pos < data.len() {
        let st = &t[state - 1];
        let hit = st.entries.iter().find_map(|e| {
            let p = &e.phrase;
            (!p.is_empty() && data[pos..].starts_with(p)).then_some((e, p.len()))
        });
        let (code, m, next) = match hit {
            Some((e, m)) => (&e.code, m, e.next),
            None => match &st.miss {
                Miss::Escape(c, n) => (c, 0, *n),
                Miss::Byte(by) => {
                    let e = &st.entries[by[usize::from(data[pos])]];
                    (&e.code, 1, e.next)
                }
            },
        };
        let full = out.len() + code.len() > cap * 8;
        if !full {
            consumed += m;
            out.extend(code);
            last = Some((code.len(), m, state));
        }
        if next != 0 {
            state = next;
        }
        if full {
            break;
        }
        pos += m;
    }
    pad(&mut out);
    if out.len() > cap * 8 {
        if let Some((len, m, st)) = last {
            out.truncate(out.len() - 8 - len);
            consumed -= m;
            state = st;
            pad(&mut out);
        }
    }
    Encoded { bytes: Bits(out).pack(), consumed, end_state: state }
}

pub fn turn_compresses(queued: &[u8]) -> bool {
    let n = queued.len().min(TRIAL_BYTES);
    let c = compress(&queued[..n], n, START_STATE);
    c.bytes.len() < c.consumed
}

pub fn input_window(room: usize) -> usize {
    3 * room + MAX_PHRASE
}

pub fn fill_frame(queued: &[u8], room: usize, state: usize) -> Encoded {
    compress(&queued[..queued.len().min(input_window(room))], room, state)
}

pub fn fits_frames(queued: &[u8], frames: usize, room: usize, last_room: usize, state: usize) -> bool {
    let q = queued.len();
    q <= (frames - 1) * room + last_room
        || match frames {
            1 => q <= input_window(room) && compress(queued, last_room, state).consumed == q,
            _ => {
                q <= input_window(2 * room) && {
                    let c = compress(queued, room, state);
                    let r = q - c.consumed;
                    r == 0 || compress(&queued[c.consumed..], r.min(last_room), c.end_state).consumed == r
                }
            }
        }
}

fn pad(b: &mut Vec<bool>) {
    let p = !*b.last().unwrap_or(&true);
    let n = if b.len() % 8 == 0 { 8 } else { 8 - b.len() % 8 };
    b.extend(std::iter::repeat_n(p, n));
}

pub fn decompress(payload: &[u8]) -> Option<(Vec<u8>, usize)> {
    let t = tables();
    let mut b = Bits::unpack(payload).0;
    let p = *b.last()?;
    while b.last() == Some(&p) {
        b.pop();
    }
    if b.len() < 3 {
        return None;
    }
    let mut state = key(&b[..3]).1 as usize + 1;
    let (mut i, mut out) = (3, Vec::new());
    while i < b.len() {
        let st = &t[state - 1];
        let (len, hit) =
            (1..=MAX_CODE.min(b.len() - i)).find_map(|l| st.decode.get(&key(&b[i..i + l])).map(|h| (l, *h)))?;
        i += len;
        match hit {
            Hit::Entry(e) => {
                out.extend(&st.entries[e].phrase);
                if st.entries[e].next != 0 {
                    state = st.entries[e].next;
                }
            }
            Hit::Escape(next) => state = next,
        }
    }
    Some((out, state))
}
