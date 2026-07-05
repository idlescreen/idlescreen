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
