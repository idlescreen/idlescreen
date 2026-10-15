// SPDX-License-Identifier: Apache-2.0
//! Cgroup v2 slice discovery, proc stat parser, and monotonic clock probe.

#![allow(unsafe_code)]

use std::path::PathBuf;

pub fn parse_cgroup_v2_path(content: &str) -> Option<PathBuf> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(slice) = trimmed.strip_prefix("0::") {
            let clean = slice.trim_start_matches('/');
            return Some(PathBuf::from(clean));
        }
    }
    None
}

pub fn parse_proc_stat_cpu_ticks(content: &str) -> Option<(u64, u64)> {
    let rparen = content.rfind(')')?;
    let rest = content.get(rparen + 1..)?.trim();
    let fields: Vec<&str> = rest.split_whitespace().collect();

    // In /proc/pid/stat:
    // field 1: pid
    // field 2: (comm)
    // rest starts at field 3 (state) -> index 0
    // utime is field 14 -> index 11
    // stime is field 15 -> index 12
    if fields.len() > 12 {
        let utime = fields[11].parse::<u64>().ok()?;
        let stime = fields[12].parse::<u64>().ok()?;
        Some((utime, stime))
    } else {
        None
    }
}

pub fn compute_cpu_seconds(utime: u64, stime: u64, clk_tck: u64) -> f64 {
    let tck = if clk_tck == 0 { 100 } else { clk_tck };
    (utime + stime) as f64 / tck as f64
}

#[derive(Debug, Clone, Copy)]
pub struct ClockSample {
    pub boottime_ns: u64,
    pub monotonic_ns: u64,
}

pub fn detect_suspend_divergence(
    prev: ClockSample,
    curr: ClockSample,
    threshold_ns: u64,
) -> (bool, u64) {
    let delta_boot = curr.boottime_ns.saturating_sub(prev.boottime_ns);
    let delta_mono = curr.monotonic_ns.saturating_sub(prev.monotonic_ns);
    let divergence = delta_boot.saturating_sub(delta_mono);

    (divergence >= threshold_ns, divergence)
}

pub fn read_real_clocks() -> ClockSample {
    let mut ts_boot = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let mut ts_mono = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    // SAFETY: `ts_mono` and `ts_boot` are valid initialized `timespec` structs on stack.
    unsafe {
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts_mono);
        libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut ts_boot);
    }

    let boottime_ns = (ts_boot.tv_sec as u64) * 1_000_000_000 + (ts_boot.tv_nsec as u64);
    let monotonic_ns = (ts_mono.tv_sec as u64) * 1_000_000_000 + (ts_mono.tv_nsec as u64);

    ClockSample {
        boottime_ns,
        monotonic_ns,
    }
}
