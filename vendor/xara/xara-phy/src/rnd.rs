const MASK: u32 = 0xFF_FFFF;
const MUL: u32 = 0xFD43FD;
const INC: u32 = 0xC39EC3;
const MUL_INV: u32 = inverse(MUL);

const fn inverse(m: u32) -> u32 {
    let mut x = m;
    let mut i = 0;
    while i < 5 {
        x = x.wrapping_mul(2u32.wrapping_sub(m.wrapping_mul(x)));
        i += 1;
    }
    x & MASK
}

pub const START_STATE: u32 = 0x05_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VbRnd {
    state: u32,
}

impl Default for VbRnd {
    fn default() -> Self {
        Self::new()
    }
}

impl VbRnd {
    pub const fn new() -> Self {
        Self { state: START_STATE }
    }

    pub const fn from_state(state: u32) -> Self {
        Self { state: state & MASK }
    }

    pub fn stream(seed: f64) -> Self {
        let mut r = Self::new();
        r.rnd(-1.0);
        r.randomize(seed);
        r
    }

    pub const fn state(&self) -> u32 {
        self.state
    }

    pub fn rnd(&mut self, x: f32) -> f32 {
        if x < 0.0 {
            let b = x.to_bits();
            self.state = b.wrapping_add(b >> 24) & MASK;
        }
        if x != 0.0 {
            self.advance();
        }
        self.value()
    }

    pub fn next_f32(&mut self) -> f32 {
        self.advance();
        self.value()
    }

    pub fn randomize(&mut self, d: f64) {
        let hi = (d.to_bits() >> 32) as u32;
        let mid = ((hi & 0xFFFF) ^ (hi >> 16)) & 0xFFFF;
        self.state = (self.state & 0xFF00_00FF) | (mid << 8);
    }

    pub fn draw(&mut self, m: u32) -> u32 {
        (self.next_f32() * m as f32) as u32
    }

    pub fn rand_int(&mut self, n: u32) -> u32 {
        self.draw(n + 1)
    }

    pub fn seed_rnd(&mut self, x: u32) {
        self.seed_rnd_wide(u64::from(x));
    }

    pub(crate) fn seed_rnd_wide(&mut self, x: u64) {
        self.rnd(-1.0);
        self.randomize((x % 32768) as f64);
        for _ in 0..=x / 32768 {
            self.advance();
        }
    }

    pub fn advance(&mut self) {
        self.state = self.state.wrapping_mul(MUL).wrapping_add(INC) & MASK;
    }

    pub fn retreat(&mut self) {
        self.state = self.state.wrapping_sub(INC).wrapping_mul(MUL_INV) & MASK;
    }

    fn value(&self) -> f32 {
        self.state as f32 / 16_777_216.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_examples() {
        let mut r = VbRnd::new();
        assert_eq!(r.next_f32(), 0.705_547_5);
        assert_eq!(r.state(), 0xB49EC3);
        let mut r = VbRnd::new();
        r.rnd(-1.0);
        assert_eq!(r.state(), 0x395886);
        let mut t = VbRnd::stream(292.0);
        let first: Vec<u32> = (0..16).map(|_| t.rand_int(128)).collect();
        assert_eq!(first, [78, 83, 81, 38, 79, 22, 91, 52, 72, 100, 103, 82, 71, 125, 123, 26]);
        assert_eq!(VbRnd::stream(43.0).state(), 0xC04586);
    }

    #[test]
    fn retreat_inverts_advance() {
        let mut r = VbRnd::from_state(0x123456);
        r.advance();
        r.retreat();
        assert_eq!(r.state(), 0x123456);
    }

    #[test]
    fn single_precision_edge() {
        let mut r = VbRnd::from_state(0xF5907A);
        assert_eq!(r.rand_int(5), 5);
    }
}
