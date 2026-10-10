// SPDX-License-Identifier: Apache-2.0
//! Tier 3: Cross-Feature Combinations test suite.
//!
//! Evaluates pairwise integration across diverse subsystem boundaries:
//! - t3_init_readiness: Pairwise matrix of init systems and readiness signaling
//! - t3_suspend_watchdog: Divergence detection coupled with watchdog zeroing
//! - t3_logging_fallback: Supervisor command failures logged to syslog & file
//! - t3_standalone_budget: Standalone mode with in-process CPU budget fallback
//! - t3_package_lifecycle: Init detection coupled with service template selection
//!
//! Verifies cross-cutting interactions between runtime components.

pub mod t3_init_readiness;
pub mod t3_suspend_watchdog;
pub mod t3_logging_fallback;
pub mod t3_standalone_budget;
pub mod t3_package_lifecycle;
