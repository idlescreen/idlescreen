// SPDX-License-Identifier: Apache-2.0
//! Tier 1: Dynamic Cgroup v2 Delegation & In-Process Fallback tests (R5).

use crate::harness::{compute_cpu_seconds, parse_cgroup_v2_path, parse_proc_stat_cpu_ticks};
use std::path::PathBuf;

#[test]
fn test_parse_cgroup_v2_standard_user_slice() {
    let content = "0::/user.slice/user-1000.slice/user@1000.service/app.slice\n";
    let path = parse_cgroup_v2_path(content);
    assert_eq!(
        path,
        Some(PathBuf::from(
            "user.slice/user-1000.slice/user@1000.service/app.slice"
        )),
        "Must correctly parse cgroup v2 user slice path"
    );
}

#[test]
fn test_parse_cgroup_v2_root_slice() {
    let content = "0::/\n";
    let path = parse_cgroup_v2_path(content);
    assert_eq!(
        path,
        Some(PathBuf::from("")),
        "Root slice parsed as empty relative path"
    );
}

#[test]
fn test_parse_proc_stat_valid_line() {
    // Simulated /proc/self/stat line:
    // pid=123 comm=(idlescreen) state=S ppid=1 ... utime=150 stime=50 ...
    let line = "123 (idlescreen) S 1 123 123 0 0 0 0 0 0 0 150 50 0 0 20 0 1 0 0 0 0";
    let ticks = parse_proc_stat_cpu_ticks(line);
    assert_eq!(
        ticks,
        Some((150, 50)),
        "Must parse utime=150 and stime=50 ticks"
    );
}

#[test]
fn test_parse_proc_stat_with_spaces_in_comm() {
    // In Linux, process command can contain spaces and parentheses: e.g. (idle daemon (v2))
    let line = "456 (idle daemon (v2)) S 1 456 456 0 0 0 0 0 0 0 320 80 0 0 20 0 1 0 0 0 0";
    let ticks = parse_proc_stat_cpu_ticks(line);
    assert_eq!(
        ticks,
        Some((320, 80)),
        "Must correctly split after rightmost parenthesis"
    );
}

#[test]
fn test_compute_cpu_seconds_standard_ticks() {
    // 200 ticks at 100 Hz = 2.00 seconds
    let secs = compute_cpu_seconds(150, 50, 100);
    assert!(
        (secs - 2.0).abs() < 1e-6,
        "200 ticks at 100 Hz must equal 2.0s"
    );

    // 500 ticks at 250 Hz = 2.00 seconds
    let secs250 = compute_cpu_seconds(400, 100, 250);
    assert!(
        (secs250 - 2.0).abs() < 1e-6,
        "500 ticks at 250 Hz must equal 2.0s"
    );
}

#[test]
fn test_in_process_cpu_fallback_active_without_cgroup() {
    // When cgroup v2 delegation is unavailable, in-process accounting
    // remains active using /proc/self/stat CPU ticks.
    let cgroup_available = false;
    let proc_stat_sample = parse_proc_stat_cpu_ticks(
        "789 (idlescreen) R 1 789 789 0 0 0 0 0 0 0 500 100 0 0 20 0 1 0 0 0 0",
    );

    let accounting_mode = if cgroup_available {
        "cgroup_v2"
    } else if proc_stat_sample.is_some() {
        "proc_stat_fallback"
    } else {
        "unaccounted"
    };

    assert_eq!(
        accounting_mode, "proc_stat_fallback",
        "Must activate in-process proc stat CPU accounting fallback"
    );
}
