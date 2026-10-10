// SPDX-License-Identifier: Apache-2.0
//! Tier 2: Watchdog & Suspend Detector Boundary & Corner Cases (R2).

use crate::harness::{ClockSample, detect_suspend_divergence};

#[test]
fn test_negative_divergence_backward_skew() {
    // Under clock jitter or virtualized guests, monotonic could report 1ns ahead of boottime
    let prev = ClockSample {
        boottime_ns: 10_000_000_000,
        monotonic_ns: 10_000_000_000,
    };
    let curr = ClockSample {
        boottime_ns: 10_010_000_000,
        monotonic_ns: 10_010_000_005, // 5ns ahead
    };

    let (is_suspend, div) = detect_suspend_divergence(prev, curr, 1_000_000_000);
    assert!(!is_suspend);
    assert_eq!(div, 0, "Saturating sub must clamp negative divergence to 0");
}

#[test]
fn test_exact_threshold_boundary() {
    let prev = ClockSample {
        boottime_ns: 10_000_000_000,
        monotonic_ns: 10_000_000_000,
    };
    let curr = ClockSample {
        boottime_ns: 11_010_000_000, // +1.010s
        monotonic_ns: 10_010_000_000, // +0.010s (divergence = exactly 1.000s)
    };

    let (is_suspend, div) = detect_suspend_divergence(prev, curr, 1_000_000_000);
    assert!(is_suspend, "Exact 1.000s divergence must trigger suspend");
    assert_eq!(div, 1_000_000_000);
}

#[test]
fn test_just_below_threshold_divergence() {
    let prev = ClockSample {
        boottime_ns: 10_000_000_000,
        monotonic_ns: 10_000_000_000,
    };
    let curr = ClockSample {
        boottime_ns: 11_009_999_999,
        monotonic_ns: 10_010_000_000, // divergence = 999,999,999 ns
    };

    let (is_suspend, div) = detect_suspend_divergence(prev, curr, 1_000_000_000);
    assert!(!is_suspend, "999,999,999 ns must NOT trigger 1s threshold");
    assert_eq!(div, 999_999_999);
}

#[test]
fn test_monotonic_u64_saturation() {
    let prev = ClockSample {
        boottime_ns: u64::MAX - 2_000_000_000,
        monotonic_ns: u64::MAX - 2_000_000_000,
    };
    let curr = ClockSample {
        boottime_ns: u64::MAX,
        monotonic_ns: u64::MAX - 1_500_000_000,
    };

    let (is_suspend, div) = detect_suspend_divergence(prev, curr, 1_000_000_000);
    assert!(is_suspend, "Extreme clock values must not overflow");
    assert_eq!(div, 1_500_000_000);
}

#[test]
fn test_rapid_repeated_heartbeats() {
    let mut last = 1_000_000_000u64;
    for offset in 1..=1000 {
        let now = 1_000_000_000u64 + offset; // 1ns increments
        let delta = now.saturating_sub(last);
        assert_eq!(delta, 1);
        last = now;
    }
}

#[test]
fn test_extended_suspend_duration() {
    let prev = ClockSample {
        boottime_ns: 1_000_000_000,
        monotonic_ns: 1_000_000_000,
    };
    // 7 days of sleep: 7 * 86400 * 1e9 ns
    let seven_days_ns = 7 * 86400 * 1_000_000_000u64;
    let curr = ClockSample {
        boottime_ns: 1_000_000_000 + seven_days_ns,
        monotonic_ns: 1_000_000_000 + 50_000_000,
    };

    let (is_suspend, div) = detect_suspend_divergence(prev, curr, 1_000_000_000);
    assert!(is_suspend);
    assert!(div >= seven_days_ns - 50_000_000);
}
