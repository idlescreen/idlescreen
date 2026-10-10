// Test files legitimately panic; suppress the lint at file scope.
#![allow(clippy::panic)]
// SPDX-License-Identifier: Apache-2.0

//! Makes `physics.rs`'s cost claim enforceable.
//!
//! `wave_displace` walks the live ring slice once per displaced cell, so
//! it is the innermost loop of the saver's render path. Its label
//! promises no allocation on the steady path. That promise is currently
//! prose: nothing fails if someone rewrites the inner loop to collect
//! displaced points into a `Vec` first. These tests make it a build
//! failure instead.

use perf_test_support::{assert_no_alloc, count_allocs};

use super::wave_displace;
use crate::ripple::types::Ring;

fn sample_rings() -> Vec<Ring> {
    // A non-empty ring set: with no rings the loop body never runs and
    // the test would pass for the wrong reason.
    (0..3)
        .map(|i| Ring {
            x: 0.25 + i as f32 * 0.2,
            y: 0.5,
            r: 0.1 + i as f32 * 0.05,
            max_r: 0.3,
            life: 1.0,
            strength: 1.0,
            age: 0.0,
        })
        .collect()
}

#[test]
fn wave_displace_does_not_allocate() {
    let rings = sample_rings();
    let (dx, dy, amp) = assert_no_alloc("wave_displace", || wave_displace(0.5, 0.5, &rings));
    // The instrument must also be shown to work on this exact shape of
    // code, not just in the abstract.
    let (_, seen) = count_allocs(|| {
        let mut out: Vec<(f32, f32, f32)> = Vec::with_capacity(8);
        for _ in 0..8 {
            out.push(wave_displace(0.5, 0.5, &rings));
        }
        out.len()
    });
    assert!(
        seen >= 1,
        "a Vec::with_capacity must be counted, saw {seen}"
    );
    let _ = (dx, dy, amp);
}

#[test]
fn an_empty_ring_set_is_also_free() {
    let empty: Vec<Ring> = Vec::new();
    let _ = assert_no_alloc("wave_displace (no rings)", || {
        wave_displace(0.5, 0.5, &empty)
    });
}
