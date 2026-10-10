// SPDX-License-Identifier: Apache-2.0
//! Tier 1: Monotonic Watchdog & Universal Suspend Detector tests (R2).

use crate::harness::{ClockSample, detect_suspend_divergence, read_real_clocks};

#[test]
fn test_monotonic_heartbeat_advancement() {
    let t0 = 10_000_000_000u64; // 10s
    let t1 = 12_500_000_000u64; // 12.5s
    let age = t1.saturating_sub(t0);
    assert_eq!(
        age, 2_500_000_000,
        "Heartbeat age must advance monotonically"
    );
}

#[test]
fn test_monotonic_heartbeat_immune_to_wallclock() {
    // If wall clock jumps backwards (e.g. NTP sync) or forwards,
    // CLOCK_MONOTONIC remains unaffected.
    let mono_t0 = 5_000_000_000u64;
    let mono_t1 = 5_100_000_000u64;
    let age_mono = mono_t1.saturating_sub(mono_t0);

    // Wall clock could jump backwards by an hour
    let wall_t0 = 100_000_000_000u64;
    let wall_t1 = 96_400_000_000u64; // -3600s
    let age_wall = wall_t1.saturating_sub(wall_t0);

    assert_eq!(age_mono, 100_000_000, "Monotonic age must stay positive");
    assert_eq!(
        age_wall, 0,
        "Wall clock underflow yields 0 but causes false stalls if used"
    );
}

#[test]
fn test_steady_state_no_divergence() {
    let prev = ClockSample {
        boottime_ns: 100_000_000_000,
        monotonic_ns: 90_000_000_000,
    };
    // 50ms tick interval in normal running system
    let curr = ClockSample {
        boottime_ns: 100_050_000_000,
        monotonic_ns: 90_050_000_000,
    };

    let (is_suspend, div) = detect_suspend_divergence(prev, curr, 1_000_000_000);
    assert!(!is_suspend, "Normal tick should not trigger suspend");
    assert_eq!(div, 0, "Divergence in running system should be zero");
}

#[test]
fn test_suspend_divergence_detection_above_threshold() {
    let prev = ClockSample {
        boottime_ns: 100_000_000_000,
        monotonic_ns: 90_000_000_000,
    };
    // System suspended for 5 seconds:
    // CLOCK_BOOTTIME advanced by 5.05s, while CLOCK_MONOTONIC only advanced by 50ms
    let curr = ClockSample {
        boottime_ns: 105_050_000_000,
        monotonic_ns: 90_050_000_000,
    };

    let (is_suspend, div) = detect_suspend_divergence(prev, curr, 1_000_000_000);
    assert!(is_suspend, "5 second sleep must trigger suspend detection");
    assert_eq!(div, 5_000_000_000, "Divergence must match suspend duration");
}

#[test]
fn test_suspend_triggers_display_resync_flag() {
    let mut display_resync_required = false;
    let prev = ClockSample {
        boottime_ns: 10_000_000_000,
        monotonic_ns: 10_000_000_000,
    };
    let curr = ClockSample {
        boottime_ns: 12_500_000_000,
        monotonic_ns: 10_050_000_000,
    };

    let (is_suspend, _) = detect_suspend_divergence(prev, curr, 1_000_000_000);
    if is_suspend {
        display_resync_required = true;
    }

    assert!(
        display_resync_required,
        "Display resync must be requested on resume"
    );
}

#[test]
fn test_watchdog_reset_on_resume() {
    let mut last_watchdog_heartbeat = 10_000_000_000u64;
    let prev = ClockSample {
        boottime_ns: 10_000_000_000,
        monotonic_ns: 10_000_000_000,
    };
    let curr = ClockSample {
        boottime_ns: 25_000_000_000, // 15s sleep
        monotonic_ns: 10_020_000_000,
    };

    let (is_suspend, _) = detect_suspend_divergence(prev, curr, 1_000_000_000);
    if is_suspend {
        // Reset watchdog to prevent spurious timeout
        last_watchdog_heartbeat = curr.monotonic_ns;
    }

    let age = curr.monotonic_ns.saturating_sub(last_watchdog_heartbeat);
    assert_eq!(
        age, 0,
        "Watchdog must be reset to zero age immediately after resume"
    );
}

#[test]
fn test_real_clocks_readable() {
    let sample = read_real_clocks();
    assert!(sample.boottime_ns > 0, "CLOCK_BOOTTIME must be positive");
    assert!(sample.monotonic_ns > 0, "CLOCK_MONOTONIC must be positive");
    assert!(
        sample.boottime_ns >= sample.monotonic_ns,
        "CLOCK_BOOTTIME must be >= CLOCK_MONOTONIC"
    );
}
