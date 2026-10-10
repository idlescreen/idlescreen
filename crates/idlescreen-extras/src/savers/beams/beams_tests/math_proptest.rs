//! Property tests for pure beams math (smoothstep + frame pacing).
//! Deterministic std-only replacement for the previous proptest version.
// SPDX-License-Identifier: Apache-2.0

use crate::beams::pacing::{FramePacing, update_frame_time};
use crate::beams::types::smoothstep;
use std::time::Duration;

/// xorshift64* — deterministic case generation without proptest.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform f32 in `lo..hi`.
    fn f32(&mut self, lo: f32, hi: f32) -> f32 {
        let t = (self.next() >> 40) as f32 / (1u64 << 24) as f32;
        lo + t * (hi - lo)
    }

    fn u64(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo + 1)
    }

    fn bool(&mut self) -> bool {
        self.next() & 1 == 1
    }
}

const CASES: usize = 2048;

/// smoothstep output is always in [0, 1] for finite inputs with edge1 > edge0.
#[test]
fn prop_smoothstep_in_unit_interval() {
    let mut rng = Rng::new(0x5EED_0001);
    for _ in 0..CASES {
        let edge0 = rng.f32(-1000.0, 1000.0);
        let edge1 = edge0 + rng.f32(0.001, 1000.0);
        let x = rng.f32(-2000.0, 2000.0);
        let y = smoothstep(edge0, edge1, x);
        assert!(y.is_finite());
        assert!((0.0..=1.0).contains(&y), "y={y}");
    }
}

/// smoothstep is non-decreasing in x for fixed edges.
#[test]
fn prop_smoothstep_monotone_in_x() {
    let mut rng = Rng::new(0x5EED_0002);
    for _ in 0..CASES {
        let edge0 = rng.f32(-100.0, 100.0);
        let edge1 = edge0 + rng.f32(0.01, 50.0);
        let x0 = rng.f32(-200.0, 200.0);
        let dx = rng.f32(0.0, 50.0);
        let y0 = smoothstep(edge0, edge1, x0);
        let y1 = smoothstep(edge0, edge1, x0 + dx);
        assert!(y1 + 1e-5 >= y0, "y0={y0} y1={y1}");
    }
}

/// Quality scale stays clamped to [0.20, 1.0] under any pacing inputs.
#[test]
fn prop_quality_scale_clamped() {
    let mut rng = Rng::new(0x5EED_0003);
    for _ in 0..CASES {
        let mut pacing = FramePacing {
            frame_time_ema: rng.f32(0.001, 0.5),
            target_frame_time: rng.f32(0.004, 0.05),
            quality_scale: rng.f32(0.20, 1.0),
        };
        update_frame_time(
            &mut pacing,
            rng.f32(0.0, 60.0),
            rng.bool(),
            Duration::from_millis(rng.u64(1, 200)),
        );
        assert!(pacing.quality_scale.is_finite());
        assert!(
            (0.20..=1.0).contains(&pacing.quality_scale),
            "quality_scale={}",
            pacing.quality_scale
        );
        assert!(pacing.frame_time_ema.is_finite());
        assert!(pacing.frame_time_ema >= 0.0);
        assert!(pacing.target_frame_time > 0.0);
    }
}
