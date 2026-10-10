// SPDX-License-Identifier: Apache-2.0
//! Tier 2: Cgroup Discovery & Proc Fallback Boundary Cases (R5).

use crate::harness::{
    compute_cpu_seconds, parse_cgroup_v2_path, parse_proc_stat_cpu_ticks,
};

#[test]
fn test_cgroup_malformed_no_colons() {
    let content = "just some random garbage without colons\n";
    let path = parse_cgroup_v2_path(content);
    assert!(path.is_none(), "Malformed cgroup file must return None");
}

#[test]
fn test_cgroup_empty_file() {
    let content = "";
    let path = parse_cgroup_v2_path(content);
    assert!(path.is_none(), "Empty cgroup file must return None");
}

#[test]
fn test_proc_stat_truncated_fields() {
    // Fewer than 14 fields
    let line = "123 (comm) S 1 2 3";
    let ticks = parse_proc_stat_cpu_ticks(line);
    assert!(ticks.is_none(), "Truncated /proc/self/stat must return None");
}

#[test]
fn test_proc_stat_non_numeric_ticks() {
    let line = "123 (comm) S 1 123 123 0 0 0 0 0 0 0 not_a_number not_a_number 0 0";
    let ticks = parse_proc_stat_cpu_ticks(line);
    assert!(ticks.is_none(), "Corrupted tick values must return None safely");
}

#[test]
fn test_proc_stat_nested_parentheses_comm() {
    let line = "999 (worker (sub) [1]) S 1 999 999 0 0 0 0 0 0 0 100 25 0 0 20 0 1 0 0 0 0";
    let ticks = parse_proc_stat_cpu_ticks(line);
    assert_eq!(
        ticks,
        Some((100, 25)),
        "Complex command string with multiple parentheses must parse correctly"
    );
}

#[test]
fn test_compute_cpu_seconds_zero_clk_tck_safe() {
    // If sysconf(_SC_CLK_TCK) returns 0 or fails, fallback to 100 Hz must prevent divide-by-zero
    let secs = compute_cpu_seconds(200, 100, 0);
    assert_eq!(secs, 3.0, "Zero clk_tck must fall back to 100 Hz safely");
}
