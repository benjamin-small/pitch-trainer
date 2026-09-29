//! Small deterministic PRNG (SplitMix64). Seeded from JS at startup; fixed seeds in tests.

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform integer in `lo..=hi`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        assert!(lo <= hi, "empty range {lo}..={hi}");
        let span = hi as u64 - lo as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }

    pub fn coin(&mut self) -> bool {
        self.next_u64() >> 63 == 1
    }

    /// Fisher–Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.range(0, i as u32) as usize;
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn range_is_inclusive_and_covers_all_values() {
        let mut rng = Rng::new(1);
        let mut seen = [false; 5];
        for _ in 0..1000 {
            let v = rng.range(3, 7);
            assert!((3..=7).contains(&v));
            seen[(v - 3) as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn range_single_value() {
        let mut rng = Rng::new(2);
        assert_eq!(rng.range(4, 4), 4);
    }

    #[test]
    fn coin_gives_both_sides() {
        let mut rng = Rng::new(3);
        let heads = (0..1000).filter(|_| rng.coin()).count();
        assert!(heads > 400 && heads < 600, "heads = {heads}");
    }

    #[test]
    fn shuffle_keeps_elements() {
        let mut rng = Rng::new(4);
        let mut items = [1, 2, 3, 4, 5, 6, 7, 8];
        rng.shuffle(&mut items);
        let mut sorted = items;
        sorted.sort();
        assert_eq!(sorted, [1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
