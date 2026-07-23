/// Environment keys for deterministic offline export (`idle-render`).
pub const SEED_ENV_KEYS: &[&str] = &["IDLE_RENDER_SEED", "TRANCE_SEED"];

/// Parse a seed from the process environment, if set and valid.
pub fn seed_from_env() -> Option<u64> {
    for key in SEED_ENV_KEYS {
        if let Ok(raw) = std::env::var(key) {
            let s = raw.trim();
            if s.is_empty() {
                continue;
            }
            if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
                if let Ok(v) = u64::from_str_radix(hex, 16) {
                    return Some(v);
                }
            } else if let Ok(v) = s.parse::<u64>() {
                return Some(v);
            }
        }
    }
    None
}

/// Linear Congruential Generator. Deterministic, lock-free.
///
/// # Example
///
/// ```
/// use trance_api::LcgRng;
/// let mut rng = LcgRng::new(42);
/// let n = rng.next_range(0.0, 10.0);
/// assert!(n >= 0.0 && n <= 10.0);
/// ```
#[derive(Clone, Debug)]
pub struct LcgRng(u64);

impl LcgRng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn new_random() -> Self {
        use std::time::SystemTime;
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        Self::new(seed)
    }

    /// Prefer `IDLE_RENDER_SEED` or `TRANCE_SEED` (decimal or 0x-hex); else random.
    pub fn from_env_or_random() -> Self {
        match seed_from_env() {
            Some(s) => Self::new(s),
            None => Self::new_random(),
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    pub fn next_f32(&mut self) -> f32 {
        let val = (self.next_u64() >> 40) as u32;
        (val as f32) * (1.0 / (1u32 << 24) as f32)
    }

    pub fn next_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }

    pub fn next_usize(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.next_u64() % max as u64) as usize
    }

    pub fn next_bool(&mut self, prob: f32) -> bool {
        self.next_f32() < prob
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_u64_changes_state() {
        let mut rng = LcgRng::new(42);
        let first = rng.next_u64();
        let second = rng.next_u64();
        assert_ne!(first, second);
    }

    #[test]
    fn next_range_within_bounds() {
        let mut rng = LcgRng::new(42);
        for _ in 0..1000 {
            let n = rng.next_range(0.0, 10.0);
            assert!((0.0..=10.0).contains(&n));
        }
    }

    #[test]
    fn next_range_degenerate_returns_constant() {
        let mut rng = LcgRng::new(7);
        for _ in 0..10 {
            assert_eq!(rng.next_range(5.0, 5.0), 5.0);
        }
    }

    #[test]
    fn next_bool_returns_both() {
        let mut rng = LcgRng::new(123);
        let mut true_count = 0;
        let mut false_count = 0;
        for _ in 0..200 {
            if rng.next_bool(0.5) {
                true_count += 1;
            } else {
                false_count += 1;
            }
        }
        assert!(true_count > 0);
        assert!(false_count > 0);
    }

    #[test]
    fn next_usize_within_bounds() {
        let mut rng = LcgRng::new(99);
        for _ in 0..500 {
            let n = rng.next_usize(10);
            assert!(n < 10);
        }
    }

    #[test]
    fn next_usize_zero_max_returns_zero() {
        let mut rng = LcgRng::new(99);
        assert_eq!(rng.next_usize(0), 0);
    }

    #[test]
    fn rng_is_deterministic_for_same_seed() {
        let mut a = LcgRng::new(0xABCD);
        let mut b = LcgRng::new(0xABCD);
        for _ in 0..50 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        /// Same seed always yields the same sequence (determinism).
        #[test]
        fn same_seed_same_stream(seed: u64, steps in 1usize..64) {
            let mut a = LcgRng::new(seed);
            let mut b = LcgRng::new(seed);
            for _ in 0..steps {
                prop_assert_eq!(a.next_u64(), b.next_u64());
            }
        }

        /// `next_usize(max)` is always in `0..max` when max > 0.
        #[test]
        fn next_usize_in_range(seed: u64, max in 1usize..=10_000) {
            let mut rng = LcgRng::new(seed);
            for _ in 0..32 {
                let n = rng.next_usize(max);
                prop_assert!(n < max, "n={n} max={max}");
            }
        }

        /// `next_f32` stays in [0, 1).
        #[test]
        fn next_f32_unit_interval(seed: u64) {
            let mut rng = LcgRng::new(seed);
            for _ in 0..32 {
                let f = rng.next_f32();
                prop_assert!((0.0..1.0).contains(&f), "f={f}");
            }
        }
    }
}

#[cfg(test)]
mod seed_env_tests {
    use super::*;

    #[test]
    fn seed_from_env_decimal() {
        unsafe {
            std::env::set_var("IDLE_RENDER_SEED", "12345");
        }
        assert_eq!(seed_from_env(), Some(12345));
        unsafe {
            std::env::remove_var("IDLE_RENDER_SEED");
        }
    }

    #[test]
    fn seed_from_env_hex() {
        unsafe {
            std::env::set_var("TRANCE_SEED", "0x10");
        }
        assert_eq!(seed_from_env(), Some(16));
        unsafe {
            std::env::remove_var("TRANCE_SEED");
        }
    }
}
