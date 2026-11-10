//! A tiny deterministic generator (SplitMix64) so exploration is reproducible per person,
//! day and session, and testable without a random-number dependency.

#[derive(Debug, Clone)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        SplitMix64(seed)
    }

    /// Seed from ids and a day number, so the same request ranks the same way twice.
    pub fn from_parts(parts: &[u128]) -> Self {
        let mut h: u64 = 0x9E37_79B9_7F4A_7C15;
        for p in parts {
            h ^= (*p as u64) ^ ((*p >> 64) as u64);
            h = h.rotate_left(27).wrapping_mul(0x94D0_49BB_1331_11EB);
        }
        SplitMix64(h)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence_and_values_in_range() {
        let mut a = SplitMix64::from_parts(&[1, 2, 3]);
        let mut b = SplitMix64::from_parts(&[1, 2, 3]);
        for _ in 0..100 {
            let x = a.next_f32();
            assert_eq!(x, b.next_f32());
            assert!((0.0..1.0).contains(&x));
        }
        let mut c = SplitMix64::from_parts(&[1, 2, 4]);
        assert_ne!(SplitMix64::from_parts(&[1, 2, 3]).next_u64(), c.next_u64());
    }
}
