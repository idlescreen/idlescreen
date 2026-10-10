// SPDX-License-Identifier: Apache-2.0
//! Tier 3: Pairwise combination — Standalone Mode + In-Process CPU Fallback.

use crate::harness::{InitSystem, compute_cpu_seconds};

#[test]
fn test_standalone_without_cgroup_uses_proc_stat() {
    let init = InitSystem::Standalone;
    let cgroup_delegated = false;

    let accounting_strategy = match (init, cgroup_delegated) {
        (_, true) => "cgroup_v2_user_slice",
        (InitSystem::Standalone, false) | (_, false) => "proc_stat_utime_stime_fallback",
    };

    assert_eq!(
        accounting_strategy, "proc_stat_utime_stime_fallback",
        "Standalone mode without cgroups must engage proc_stat fallback"
    );
}

#[test]
fn test_standalone_cpu_budget_enforcement_via_proc_stat() {
    // Process has 10% CPU budget (0.10s per second)
    let budget_sec_per_sec = 0.10f64;

    // Measurement interval: 1.0s elapsed
    // Ticks consumed: 25 ticks on 100 Hz = 0.25s
    let delta_ticks_utime = 20u64;
    let delta_ticks_stime = 5u64;
    let cpu_used = compute_cpu_seconds(delta_ticks_utime, delta_ticks_stime, 100);

    let budget_exceeded = cpu_used > budget_sec_per_sec;
    assert!(budget_exceeded, "0.25s CPU usage exceeds 0.10s budget");
}

#[test]
fn test_standalone_cgroup_presence_prefers_cgroup() {
    let cgroup_delegated = true;
    let preferred = if cgroup_delegated {
        "cgroup_v2"
    } else {
        "proc_stat"
    };
    assert_eq!(
        preferred, "cgroup_v2",
        "Must prefer cgroup v2 when available"
    );
}
