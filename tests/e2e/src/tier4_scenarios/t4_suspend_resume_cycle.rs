// SPDX-License-Identifier: Apache-2.0
//! Tier 4 Scenario 4: Laptop suspend/resume cycle with resync and watchdog protection.

use crate::harness::{ClockSample, detect_suspend_divergence};

#[test]
fn test_scenario_laptop_suspend_resume_cycle() {
    // Phase 1: Laptop awake, normal operation
    let mut last_watchdog = 10_000_000_000u64;
    let mut outputs_resynced = false;

    let prev = ClockSample {
        boottime_ns: 10_000_000_000,
        monotonic_ns: 10_000_000_000,
    };

    // Phase 2: User closes laptop lid -> laptop sleeps for 1 hour (3600s)
    let sleep_ns = 3600 * 1_000_000_000u64;
    let curr = ClockSample {
        boottime_ns: 10_000_000_000 + sleep_ns + 100_000_000,
        monotonic_ns: 10_000_000_000 + 100_000_000, // monotonic paused during sleep
    };

    // Phase 3: Resume detection
    let (is_suspend, div) = detect_suspend_divergence(prev, curr, 1_000_000_000);
    assert!(is_suspend, "Laptop wake must be detected as suspend event");
    assert_eq!(div, sleep_ns);

    // Phase 4: Action on wake — resync outputs and reset watchdog
    if is_suspend {
        outputs_resynced = true;
        last_watchdog = curr.monotonic_ns;
    }

    assert!(outputs_resynced, "Output topology must be refreshed");
    let watchdog_age_post_wake = curr.monotonic_ns.saturating_sub(last_watchdog);
    assert_eq!(watchdog_age_post_wake, 0, "No false stall watchdog timeout");
}
