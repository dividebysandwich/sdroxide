#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bits(pub Vec<bool>);

impl Bits {
    pub fn push(&mut self, v: u64, n: usize) {
        self.0.extend((0..n).rev().map(|i| (v >> i) & 1 == 1));
    }

    pub fn push_bfi(&mut self, v: u64, n: usize) {
        let w = (64 - v.leading_zeros() as usize).max(1);
        if w > n { self.push(v >> (w - n), n) } else { self.push(v, n) }
    }

    pub fn push_text7(&mut self, data: &[u8]) {
        for &c in data {
            self.push(u64::from(c), 7);
        }
        let pad = self.last() != Some(true);
        self.bit(pad);
        while self.len() % 8 != 0 {
            self.bit(pad);
        }
    }

    pub fn bit(&mut self, b: bool) {
        self.0.push(b);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn last(&self) -> Option<bool> {
        self.0.last().copied()
    }

    pub fn pack(&self) -> Vec<u8> {
        self.0.chunks(8).map(|c| c.iter().enumerate().fold(0u8, |a, (i, &b)| a | (u8::from(b) << (7 - i)))).collect()
    }

    pub fn unpack(bytes: &[u8]) -> Self {
        Self(bytes.iter().flat_map(|&b| (0..8).rev().map(move |i| (b >> i) & 1 == 1)).collect())
    }
}

pub struct Reader<'a> {
    pub bits: &'a [bool],
    pub pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bits: &'a [bool]) -> Self {
        Self { bits, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.bits.len().saturating_sub(self.pos)
    }

    pub fn bit(&mut self) -> Option<bool> {
        let b = *self.bits.get(self.pos)?;
        self.pos += 1;
        Some(b)
    }

    pub fn take(&mut self, n: usize) -> Option<u64> {
        if self.remaining() < n {
            return None;
        }
        let v = self.bits[self.pos..self.pos + n].iter().fold(0, |a, &b| (a << 1) | u64::from(b));
        self.pos += n;
        Some(v)
    }

    pub fn text7(&mut self) -> Option<Vec<u8>> {
        let pad = *self.bits.last()?;
        let end = self.bits.iter().rposition(|&b| b != pad).map_or(0, |i| i + 1);
        let mut out = Vec::new();
        while self.pos + 7 <= end {
            out.push(self.take(7)? as u8);
        }
        Some(out)
    }

    pub fn text(&mut self, n: usize) -> Option<String> {
        if self.remaining() < n {
            return None;
        }
        let s = self.bits[self.pos..self.pos + n].iter().map(|&b| if b { '1' } else { '0' }).collect();
        self.pos += n;
        Some(s)
    }
}

pub fn vb_val(s: &[u8]) -> u64 {
    s.iter()
        .filter(|&&c| c != b' ')
        .take_while(|c| c.is_ascii_digit())
        .fold(0u64, |a, &c| a.saturating_mul(10).saturating_add(u64::from(c - b'0')))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text7_round_trip() {
        for data in [&b""[..], b"A", b"hello world", b"\x7f\x7f\x7f", b"\x00\x01"] {
            let mut b = Bits::default();
            b.push(0b101, 3);
            b.push_text7(data);
            assert_eq!(b.len() % 8, 0);
            let mut r = Reader::new(&b.0);
            r.take(3);
            assert_eq!(r.text7().as_deref(), Some(data));
        }
        assert_eq!(Reader::new(&[]).text7(), None);
    }
}
