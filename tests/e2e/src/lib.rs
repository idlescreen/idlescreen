// SPDX-License-Identifier: Apache-2.0
//! IdleScreen End-to-End Opaque-Box Test Suite.
//!
//! Provides comprehensive requirement-driven verification across 4 tiers:
//! - Tier 1: Feature Coverage (>=5 tests per feature)
//! - Tier 2: Boundary & Corner Cases (>=5 tests per feature)
//! - Tier 3: Cross-Feature Combinations (pairwise combinations)
//! - Tier 4: Real-World Application Scenarios (realistic workflows)

#![deny(unsafe_code)]

pub mod harness;

#[cfg(test)]
pub mod tier1_features;
#[cfg(test)]
pub mod tier2_boundaries;
#[cfg(test)]
pub mod tier3_combinations;
#[cfg(test)]
pub mod tier4_scenarios;

pub use harness::{
    ClockSample, InitEnvironment, InitSystem, NotifySocketProbe, ParsedSyslog, PipeReadinessProbe,
    RollingFileProbe, SyslogProbe, compute_cpu_seconds, detect_mock_init,
    detect_suspend_divergence, emit_notify_socket, format_rfc3164, parse_cgroup_v2_path,
    parse_proc_stat_cpu_ticks, parse_rfc3164_datagram, read_real_clocks, safe_exec_supervisor,
};
