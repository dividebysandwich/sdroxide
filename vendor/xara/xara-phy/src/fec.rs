pub fn frame_bits(frame: &[u8], k: usize) -> Vec<u8> {
    let mut bits: Vec<u8> = frame.iter().flat_map(|&b| (0..8).rev().map(move |i| (b >> i) & 1)).collect();
    bits.resize(k, 0);
    bits
}

pub fn whiten(bits: &mut [u8], w: &[u8]) {
    for (b, &x) in bits.iter_mut().zip(w) {
        *b ^= x;
    }
}

fn rsc(input: impl Iterator<Item = u8>, k: usize) -> (Vec<u8>, Vec<u8>) {
    let (mut x, mut p) = (Vec::with_capacity(k + 3), Vec::with_capacity(k + 3));
    let (mut r0, mut r1, mut r2) = (0u8, 0u8, 0u8);
    for u in input.map(Some).chain([None; 3]) {
        let u = u.unwrap_or(r0 ^ r1);
        let a = u ^ r0 ^ r1;
        x.push(u);
        p.push(a ^ r2 ^ r0);
        (r0, r1, r2) = (r1, r2, a);
    }
    (x, p)
}

pub fn turbo_encode(info: &[u8], perm: &[u32], p: u32, n: usize) -> Vec<u8> {
    let k = info.len();
    let (x1, p1) = rsc(info.iter().copied(), k);
    let (x2, p2) = rsc(perm.iter().map(|&i| info[i as usize]), k);
    let p = p as usize;
    let mut out = Vec::with_capacity(n);
    for t in 0..k {
        out.push(x1[t]);
        if p == 1 || t % p == 0 {
            out.push(p1[t]);
        }
        if p == 1 || t % p == p / 2 {
            out.push(p2[t]);
        }
    }
    for t in k..k + 3 {
        out.extend([x1[t], p1[t], x2[t], p2[t]]);
    }
    assert!(out.len() <= n, "turbo output {} exceeds N {n}", out.len());
    out.resize(n, 0);
    out
}

pub fn interleave(coded: &[u8], chan: &[u32]) -> Vec<u8> {
    chan.iter().map(|&i| coded[i as usize]).collect()
}

pub const fn puncture_period(rate: u32) -> u32 {
    match rate {
        2 => 1,
        3 => 2,
        4 => 4,
        5 => 6,
        6 => 8,
        7 => 10,
        _ => panic!("codeRateIndex"),
    }
}

pub const fn punctured_len(k: u32, p: u32) -> u32 {
    if p == 1 {
        return 3 * k + 12;
    }
    let p1 = k.div_ceil(p);
    let half = p / 2;
    let p2 = if k > half { (k - half - 1) / p + 1 } else { 0 };
    k + p1 + p2 + 12
}
