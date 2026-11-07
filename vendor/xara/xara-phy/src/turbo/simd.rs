pub(crate) trait Lanes: Copy + Send {
    fn load(v: [i16; 8]) -> Self;
    fn store(self) -> [i16; 8];
    fn gamma(a: i16, b: i16) -> Self;
    fn adds(self, o: Self) -> Self;
    fn subs(self, o: Self) -> Self;
    fn max(self, o: Self) -> Self;
    fn zip_lo(self) -> Self;
    fn zip_hi(self) -> Self;
    fn pairmax(self, o: Self) -> Self;
    fn norm(self) -> Self;
    fn llr(v0: Self, v1: Self) -> i32;
}

#[cfg_attr(any(target_arch = "x86_64", target_arch = "aarch64", arm_simd32), allow(dead_code))]
pub(crate) const SIGN_A: [i16; 8] = [1, -1, 0, 0, 0, 0, -1, 1];
#[cfg_attr(any(target_arch = "x86_64", target_arch = "aarch64", arm_simd32), allow(dead_code))]
pub(crate) const SIGN_B: [i16; 8] = [0, 0, 1, -1, -1, 1, 0, 0];
pub(crate) const U1: [bool; 8] = [false, true, false, true, true, false, true, false];

#[cfg_attr(any(target_arch = "x86_64", target_arch = "aarch64", arm_simd32), allow(dead_code))]
#[derive(Clone, Copy)]
pub(crate) struct Scalar([i16; 8]);

#[cfg_attr(any(target_arch = "x86_64", target_arch = "aarch64", arm_simd32), allow(dead_code))]
impl Scalar {
    fn map2(self, o: Self, f: impl Fn(i16, i16) -> i16) -> Self {
        Self(std::array::from_fn(|i| f(self.0[i], o.0[i])))
    }
}

impl Lanes for Scalar {
    fn load(v: [i16; 8]) -> Self {
        Self(v)
    }

    fn store(self) -> [i16; 8] {
        self.0
    }

    fn gamma(a: i16, b: i16) -> Self {
        Self(std::array::from_fn(|i| SIGN_A[i] * a + SIGN_B[i] * b))
    }

    fn adds(self, o: Self) -> Self {
        self.map2(o, i16::saturating_add)
    }

    fn subs(self, o: Self) -> Self {
        self.map2(o, i16::saturating_sub)
    }

    fn max(self, o: Self) -> Self {
        self.map2(o, i16::max)
    }

    fn zip_lo(self) -> Self {
        Self(std::array::from_fn(|i| self.0[i / 2]))
    }

    fn zip_hi(self) -> Self {
        Self(std::array::from_fn(|i| self.0[4 + i / 2]))
    }

    fn pairmax(self, o: Self) -> Self {
        let v =
            |j: usize| if j < 4 { self.0[2 * j].max(self.0[2 * j + 1]) } else { o.0[2 * j - 8].max(o.0[2 * j - 7]) };
        Self(std::array::from_fn(v))
    }

    fn norm(self) -> Self {
        Self(self.0.map(|v| v.saturating_sub(self.0[0])))
    }

    fn llr(v0: Self, v1: Self) -> i32 {
        let (mut p, mut q) = (i16::MIN, i16::MIN);
        for i in 0..8 {
            let (a, b) = if U1[i] { (v1.0[i], v0.0[i]) } else { (v0.0[i], v1.0[i]) };
            p = p.max(a);
            q = q.max(b);
        }
        i32::from(p) - i32::from(q)
    }
}

#[cfg(target_arch = "x86_64")]
mod sse2 {
    use super::{Lanes, U1};
    use core::arch::x86_64::*;

    #[derive(Clone, Copy)]
    pub(crate) struct Sse2(__m128i);

    // SAFETY: SSE2 is part of the x86_64 baseline.
    impl Lanes for Sse2 {
        #[inline(always)]
        fn load(v: [i16; 8]) -> Self {
            Self(unsafe { _mm_loadu_si128(v.as_ptr().cast()) })
        }

        #[inline(always)]
        fn store(self) -> [i16; 8] {
            let mut o = [0i16; 8];
            unsafe { _mm_storeu_si128(o.as_mut_ptr().cast(), self.0) };
            o
        }

        #[inline(always)]
        fn gamma(a: i16, b: i16) -> Self {
            unsafe {
                let ab = _mm_set1_epi32((u32::from(b as u16) << 16 | u32::from(a as u16)) as i32);
                let v = _mm_shufflehi_epi16::<0b00_00_01_01>(_mm_shufflelo_epi16::<0b01_01_00_00>(ab));
                Self(_mm_mullo_epi16(v, _mm_setr_epi16(1, -1, 1, -1, -1, 1, -1, 1)))
            }
        }

        #[inline(always)]
        fn adds(self, o: Self) -> Self {
            Self(unsafe { _mm_adds_epi16(self.0, o.0) })
        }

        #[inline(always)]
        fn subs(self, o: Self) -> Self {
            Self(unsafe { _mm_subs_epi16(self.0, o.0) })
        }

        #[inline(always)]
        fn max(self, o: Self) -> Self {
            Self(unsafe { _mm_max_epi16(self.0, o.0) })
        }

        #[inline(always)]
        fn zip_lo(self) -> Self {
            Self(unsafe { _mm_unpacklo_epi16(self.0, self.0) })
        }

        #[inline(always)]
        fn zip_hi(self) -> Self {
            Self(unsafe { _mm_unpackhi_epi16(self.0, self.0) })
        }

        #[inline(always)]
        fn pairmax(self, o: Self) -> Self {
            unsafe {
                let m =
                    |v: __m128i| _mm_max_epi16(_mm_srai_epi32::<16>(_mm_slli_epi32::<16>(v)), _mm_srai_epi32::<16>(v));
                Self(_mm_packs_epi32(m(self.0), m(o.0)))
            }
        }

        #[inline(always)]
        fn norm(self) -> Self {
            unsafe {
                let z = _mm_shuffle_epi32::<0>(_mm_shufflelo_epi16::<0>(self.0));
                Self(_mm_subs_epi16(self.0, z))
            }
        }

        #[inline(always)]
        fn llr(v0: Self, v1: Self) -> i32 {
            unsafe {
                let m = _mm_setr_epi16(
                    -i16::from(U1[0]),
                    -i16::from(U1[1]),
                    -i16::from(U1[2]),
                    -i16::from(U1[3]),
                    -i16::from(U1[4]),
                    -i16::from(U1[5]),
                    -i16::from(U1[6]),
                    -i16::from(U1[7]),
                );
                let d = _mm_and_si128(_mm_xor_si128(v0.0, v1.0), m);
                let (p, q) = (_mm_xor_si128(v0.0, d), _mm_xor_si128(v1.0, d));
                let h = _mm_max_epi16(_mm_unpacklo_epi64(p, q), _mm_unpackhi_epi64(p, q));
                let h = _mm_max_epi16(h, _mm_shuffle_epi32::<0b10_11_00_01>(h));
                let h = _mm_max_epi16(h, _mm_srli_epi32::<16>(h));
                i32::from(_mm_extract_epi16::<0>(h) as i16) - i32::from(_mm_extract_epi16::<4>(h) as i16)
            }
        }
    }
}

#[cfg(target_arch = "aarch64")]
mod neon {
    use super::{Lanes, SIGN_A, SIGN_B, U1};
    use core::arch::aarch64::*;

    #[derive(Clone, Copy)]
    pub(crate) struct Neon(int16x8_t);

    const MASK: [u16; 8] = {
        let mut m = [0u16; 8];
        let mut i = 0;
        while i < 8 {
            m[i] = if U1[i] { 0xffff } else { 0 };
            i += 1;
        }
        m
    };

    // SAFETY: NEON is part of the aarch64 baseline.
    impl Lanes for Neon {
        #[inline(always)]
        fn load(v: [i16; 8]) -> Self {
            Self(unsafe { vld1q_s16(v.as_ptr()) })
        }

        #[inline(always)]
        fn store(self) -> [i16; 8] {
            let mut o = [0i16; 8];
            unsafe { vst1q_s16(o.as_mut_ptr(), self.0) };
            o
        }

        #[inline(always)]
        fn gamma(a: i16, b: i16) -> Self {
            unsafe { Self(vmlaq_n_s16(vmulq_n_s16(vld1q_s16(SIGN_A.as_ptr()), a), vld1q_s16(SIGN_B.as_ptr()), b)) }
        }

        #[inline(always)]
        fn adds(self, o: Self) -> Self {
            Self(unsafe { vqaddq_s16(self.0, o.0) })
        }

        #[inline(always)]
        fn subs(self, o: Self) -> Self {
            Self(unsafe { vqsubq_s16(self.0, o.0) })
        }

        #[inline(always)]
        fn max(self, o: Self) -> Self {
            Self(unsafe { vmaxq_s16(self.0, o.0) })
        }

        #[inline(always)]
        fn zip_lo(self) -> Self {
            Self(unsafe { vzip1q_s16(self.0, self.0) })
        }

        #[inline(always)]
        fn zip_hi(self) -> Self {
            Self(unsafe { vzip2q_s16(self.0, self.0) })
        }

        #[inline(always)]
        fn pairmax(self, o: Self) -> Self {
            Self(unsafe { vpmaxq_s16(self.0, o.0) })
        }

        #[inline(always)]
        fn norm(self) -> Self {
            Self(unsafe { vqsubq_s16(self.0, vdupq_laneq_s16::<0>(self.0)) })
        }

        #[inline(always)]
        fn llr(v0: Self, v1: Self) -> i32 {
            unsafe {
                let m = vld1q_u16(MASK.as_ptr());
                let (p, q) = (vbslq_s16(m, v1.0, v0.0), vbslq_s16(m, v0.0, v1.0));
                i32::from(vmaxvq_s16(p)) - i32::from(vmaxvq_s16(q))
            }
        }
    }
}

#[cfg(arm_simd32)]
mod simd32 {
    use super::{Lanes, SIGN_A, SIGN_B, U1};
    use core::arch::asm;

    #[derive(Clone, Copy)]
    pub(crate) struct Simd32([u32; 4]);

    const MASK: [u32; 4] = {
        let mut m = [0u32; 4];
        let mut i = 0;
        while i < 8 {
            if U1[i] {
                m[i / 2] |= 0xffff << (16 * (i % 2));
            }
            i += 1;
        }
        m
    };

    #[inline(always)]
    fn qadd16(a: u32, b: u32) -> u32 {
        let r;
        unsafe {
            asm!("qadd16 {}, {}, {}", lateout(reg) r, in(reg) a, in(reg) b, options(pure, nomem, nostack, preserves_flags))
        };
        r
    }

    #[inline(always)]
    fn qsub16(a: u32, b: u32) -> u32 {
        let r;
        unsafe {
            asm!("qsub16 {}, {}, {}", lateout(reg) r, in(reg) a, in(reg) b, options(pure, nomem, nostack, preserves_flags))
        };
        r
    }

    #[inline(always)]
    fn max16(a: u32, b: u32) -> u32 {
        let r;
        unsafe {
            asm!(
                "ssub16 {t}, {a}, {b}",
                "sel {r}, {a}, {b}",
                a = in(reg) a,
                b = in(reg) b,
                t = out(reg) _,
                r = lateout(reg) r,
                options(pure, nomem, nostack),
            )
        };
        r
    }

    #[inline(always)]
    fn dup_lo(w: u32) -> u32 {
        (w & 0xffff) | (w << 16)
    }

    #[inline(always)]
    fn dup_hi(w: u32) -> u32 {
        (w >> 16) | (w & 0xffff_0000)
    }

    #[inline(always)]
    fn pmax(x: u32, y: u32) -> u32 {
        max16((x & 0xffff) | (y << 16), (x >> 16) | (y & 0xffff_0000))
    }

    #[inline(always)]
    fn hmax(w: u32) -> i16 {
        (w as i16).max((w >> 16) as i16)
    }

    impl Simd32 {
        #[inline(always)]
        fn map2(self, o: Self, f: impl Fn(u32, u32) -> u32) -> Self {
            Self(std::array::from_fn(|i| f(self.0[i], o.0[i])))
        }
    }

    // SAFETY: the ARMv6 SIMD instructions are available on every target that sets arm_simd32.
    impl Lanes for Simd32 {
        #[inline(always)]
        fn load(v: [i16; 8]) -> Self {
            Self(std::array::from_fn(|i| u32::from(v[2 * i] as u16) | u32::from(v[2 * i + 1] as u16) << 16))
        }

        #[inline(always)]
        fn store(self) -> [i16; 8] {
            std::array::from_fn(|i| (self.0[i / 2] >> (16 * (i % 2))) as i16)
        }

        #[inline(always)]
        fn gamma(a: i16, b: i16) -> Self {
            Self::load(std::array::from_fn(|i| SIGN_A[i] * a + SIGN_B[i] * b))
        }

        #[inline(always)]
        fn adds(self, o: Self) -> Self {
            self.map2(o, qadd16)
        }

        #[inline(always)]
        fn subs(self, o: Self) -> Self {
            self.map2(o, qsub16)
        }

        #[inline(always)]
        fn max(self, o: Self) -> Self {
            self.map2(o, max16)
        }

        #[inline(always)]
        fn zip_lo(self) -> Self {
            let [w0, w1, _, _] = self.0;
            Self([dup_lo(w0), dup_hi(w0), dup_lo(w1), dup_hi(w1)])
        }

        #[inline(always)]
        fn zip_hi(self) -> Self {
            let [_, _, w2, w3] = self.0;
            Self([dup_lo(w2), dup_hi(w2), dup_lo(w3), dup_hi(w3)])
        }

        #[inline(always)]
        fn pairmax(self, o: Self) -> Self {
            let ([a0, a1, a2, a3], [b0, b1, b2, b3]) = (self.0, o.0);
            Self([pmax(a0, a1), pmax(a2, a3), pmax(b0, b1), pmax(b2, b3)])
        }

        #[inline(always)]
        fn norm(self) -> Self {
            let z = dup_lo(self.0[0]);
            Self(self.0.map(|w| qsub16(w, z)))
        }

        #[inline(always)]
        fn llr(v0: Self, v1: Self) -> i32 {
            let p: [u32; 4] = std::array::from_fn(|i| (v0.0[i] & !MASK[i]) | (v1.0[i] & MASK[i]));
            let q: [u32; 4] = std::array::from_fn(|i| (v1.0[i] & !MASK[i]) | (v0.0[i] & MASK[i]));
            let m = |v: [u32; 4]| hmax(max16(max16(v[0], v[1]), max16(v[2], v[3])));
            i32::from(m(p)) - i32::from(m(q))
        }
    }
}

#[cfg(target_arch = "x86_64")]
pub(crate) type Native = sse2::Sse2;
#[cfg(target_arch = "aarch64")]
pub(crate) type Native = neon::Neon;
#[cfg(arm_simd32)]
pub(crate) type Native = simd32::Simd32;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", arm_simd32)))]
pub(crate) type Native = Scalar;

#[cfg(test)]
mod tests {
    use super::{Lanes, Native, Scalar};

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn value(&mut self) -> i16 {
            const EDGE: [i16; 9] = [i16::MIN, i16::MIN + 1, -16384, -1, 0, 1, 16384, i16::MAX - 1, i16::MAX];
            if self.next() % 3 == 0 { EDGE[(self.next() % 9) as usize] } else { self.next() as i16 }
        }

        fn lanes(&mut self) -> [i16; 8] {
            std::array::from_fn(|_| self.value())
        }
    }

    #[test]
    fn native_lanes_match_scalar() {
        let mut r = Rng(0x9e37_79b9_7f4a_7c15);
        for _ in 0..200_000 {
            let (x, y) = (r.lanes(), r.lanes());
            let (sx, sy, nx, ny) = (Scalar::load(x), Scalar::load(y), Native::load(x), Native::load(y));
            assert_eq!(nx.store(), x);
            assert_eq!(nx.adds(ny).store(), sx.adds(sy).store(), "adds {x:?} {y:?}");
            assert_eq!(nx.subs(ny).store(), sx.subs(sy).store(), "subs {x:?} {y:?}");
            assert_eq!(nx.max(ny).store(), sx.max(sy).store(), "max {x:?} {y:?}");
            assert_eq!(nx.zip_lo().store(), sx.zip_lo().store(), "zip_lo {x:?}");
            assert_eq!(nx.zip_hi().store(), sx.zip_hi().store(), "zip_hi {x:?}");
            assert_eq!(nx.pairmax(ny).store(), sx.pairmax(sy).store(), "pairmax {x:?} {y:?}");
            assert_eq!(nx.norm().store(), sx.norm().store(), "norm {x:?}");
            assert_eq!(Native::llr(nx, ny), Scalar::llr(sx, sy), "llr {x:?} {y:?}");
            let (a, b) = (r.value().max(-i16::MAX), r.value().max(-i16::MAX));
            assert_eq!(Native::gamma(a, b).store(), Scalar::gamma(a, b).store(), "gamma {a} {b}");
        }
    }
}
