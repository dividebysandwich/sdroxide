#[derive(Clone, Debug)]
pub struct XorShift(u64);

impl XorShift {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn upto(&mut self, n: u64) -> u64 {
        self.next_u64() % (n + 1)
    }

    pub fn below(&mut self, n: u32) -> u32 {
        ((self.next_u64() >> 11) % u64::from(n.max(1))) as u32
    }

    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}
