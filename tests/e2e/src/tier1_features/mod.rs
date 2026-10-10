// SPDX-License-Identifier: Apache-2.0
//! Tier 1: Feature Coverage test suite.
//!
//! Covers primary functionality (happy path) for all features in ORIGINAL_REQUEST.md:
//! - t1_init_detect: Init detection & supervisor command dispatch (R1)
//! - t1_watchdog_clock: Monotonic watchdog heartbeat & suspend detection (R2)
//! - t1_readiness: Abstract readiness sockets, pipes, and bus names (R3)
//! - t1_logging: Syslog RFC 3164 and atomic rolling state log sinks (R4)
//! - t1_cgroup_cpu: Cgroup v2 discovery and proc stat CPU fallback (R5)
//! - t1_templates_pkg: Service definitions and packaging automation (R6)
//!
//! Each feature includes at least 5 distinct test cases verifying valid
//! inputs and expected behavior.

pub mod t1_cgroup_cpu;
pub mod t1_init_detect;
pub mod t1_logging;
pub mod t1_readiness;
pub mod t1_templates_pkg;
pub mod t1_watchdog_clock;
