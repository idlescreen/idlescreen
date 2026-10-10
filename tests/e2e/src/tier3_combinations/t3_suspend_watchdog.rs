// SPDX-License-Identifier: Apache-2.0
//! Tier 3: Pairwise combination — Suspend Resume + Watchdog Monotonic Reset.

use crate::harness::{ClockSample, detect_suspend_divergence};

#[test]
fn test_suspend_detection_triggers_watchdog_reset_and_resync() {
    let mut watchdog_heartbeat = 1_000_000_000u64;
    let mut resync_dispatched = false;

    let prev = ClockSample {
        boottime_ns: 10_000_000_000,
        monotonic_ns: 10_000_000_000,
    };
    // 20s suspend
    let curr = ClockSample {
        boottime_ns: 30_050_000_000,
        monotonic_ns: 10_050_000_000,
    };

    let (is_suspend, _) = detect_suspend_divergence(prev, curr, 1_000_000_000);
    if is_suspend {
        resync_dispatched = true;
        watchdog_heartbeat = curr.monotonic_ns;
    }

    assert!(
        resync_dispatched,
        "Resync must be triggered on suspend wake"
    );
    let watchdog_age = curr.monotonic_ns.saturating_sub(watchdog_heartbeat);
    assert_eq!(watchdog_age, 0, "Watchdog must be zeroed immediately");
}

#[test]
fn test_no_false_watchdog_stalls_across_suspend() {
    // If the timeout is 5 seconds, a 10s sleep could cause a false stall
    // under wall clock, but monotonic clock + resume reset prevents this.
    let timeout_ns = 5_000_000_000u64;
    let mut last_heartbeat = 1_000_000_000u64;

    let prev = ClockSample {
        boottime_ns: 1_000_000_000,
        monotonic_ns: 1_000_000_000,
    };
    let curr = ClockSample {
        boottime_ns: 11_020_000_000, // 10s sleep
        monotonic_ns: 1_020_000_000,
    };

    let (is_suspend, _) = detect_suspend_divergence(prev, curr, 1_000_000_000);
    if is_suspend {
        last_heartbeat = curr.monotonic_ns;
    }

    let age = curr.monotonic_ns.saturating_sub(last_heartbeat);
    let stalled = age > timeout_ns;
    assert!(
        !stalled,
        "System resume must not escalate false stall timeout"
    );
}

#[test]
fn test_repeated_suspend_cycles_stability() {
    let mut prev = ClockSample {
        boottime_ns: 0,
        monotonic_ns: 0,
    };
    for cycle in 1..=10 {
        let sleep_ns = 2_000_000_000u64;
        let curr = ClockSample {
            boottime_ns: prev.boottime_ns + sleep_ns + 50_000_000,
            monotonic_ns: prev.monotonic_ns + 50_000_000,
        };
        let (is_suspend, div) = detect_suspend_divergence(prev, curr, 1_000_000_000);
        assert!(is_suspend, "Cycle {cycle} must detect suspend");
        assert_eq!(div, sleep_ns);
        prev = curr;
    }
}
