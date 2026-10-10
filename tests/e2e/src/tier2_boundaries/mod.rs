// SPDX-License-Identifier: Apache-2.0
//! Tier 2: Boundary & Corner Cases test suite.
//!
//! Covers adversarial edge cases, error conditions, and resource boundaries:
//! - t2_init_bounds: Missing binaries, empty PATH, permission errors (R1)
//! - t2_watchdog_bounds: Clock jitter, exact 1s boundary, extreme suspend (R2)
//! - t2_readiness_bounds: Missing envs, buffer limits, double close (R3)
//! - t2_logging_bounds: Missing /dev/log, corrupted packets, non-UTF8 (R4)
//! - t2_cgroup_bounds: Malformed cgroup lines, truncated stat fields (R5)
//! - t2_package_bounds: Missing templates, unknown inits, restart race (R6)
//!
//! Each feature includes at least 5 distinct corner cases testing resilience.

pub mod t2_cgroup_bounds;
pub mod t2_init_bounds;
pub mod t2_logging_bounds;
pub mod t2_package_bounds;
pub mod t2_readiness_bounds;
pub mod t2_watchdog_bounds;
