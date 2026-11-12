//! Shared test helpers. The generator is seeded: every run gives the same cases.
#![allow(dead_code)]

/// SplitMix64. One seeded stream for each test; no global random source.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..bound` (`bound` above 0).
    pub fn below(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound
    }

    /// A value in `low..=high`.
    pub fn range(&mut self, low: u64, high: u64) -> u64 {
        low + self.below(high - low + 1)
    }

    pub fn chance(&mut self, one_in: u64) -> bool {
        self.below(one_in) == 0
    }

    /// A `u64` with a random number of significant bits, so small and huge values both occur.
    pub fn wide(&mut self) -> u64 {
        let bits = self.below(65) as u32;
        if bits == 0 {
            0
        } else {
            self.next_u64() >> (64 - bits)
        }
    }

    /// A multiple of 0.25 in `low..=high` (both multiples of 0.25). Exact in `f32`.
    pub fn quarter(&mut self, low: f32, high: f32) -> f32 {
        let steps = ((high - low) * 4.0) as u64;
        low + self.below(steps + 1) as f32 * 0.25
    }
}
