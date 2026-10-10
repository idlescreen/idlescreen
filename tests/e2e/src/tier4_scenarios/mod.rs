// SPDX-License-Identifier: Apache-2.0
//! Tier 4: Real-World Application Scenarios test suite.
//!
//! Validates realistic end-to-end multi-step system workflows:
//! - t4_systemd_workflow: Native systemd desktop session lifecycle
//! - t4_s6_workflow: s6 supervised appliance service and logging
//! - t4_openrc_workflow: OpenRC lightweight desktop session and syslog
//! - t4_suspend_resume_cycle: Laptop deep sleep and display wake cycle
//! - t4_container_fallback: Rootless container standalone fallback
//! - t4_install_non_systemd: Fresh package installation on non-systemd hosts
//!
//! Ensures end-to-end coherence across real operating environment scenarios.

pub mod t4_systemd_workflow;
pub mod t4_s6_workflow;
pub mod t4_openrc_workflow;
pub mod t4_suspend_resume_cycle;
pub mod t4_container_fallback;
pub mod t4_install_non_systemd;
