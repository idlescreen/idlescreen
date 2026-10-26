// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Tests for the DPMS power-management controller.

use super::{DpmsController, OutputPowerMode};
use std::time::Duration;

#[test]
fn test_dpms_state_transitions_and_wake() {
    let mut dpms = DpmsController::new();
    assert!(!dpms.is_sleeping());
    assert_eq!(dpms.timeout_secs(), 0);
    assert!(dpms.is_active_and_stable());

    dpms.set_timeout_secs(300);
    assert_eq!(dpms.timeout_secs(), 300);
    // Without bound Wayland output_powers, check_idle_sleep is a safe no-op
    assert!(!dpms.check_idle_sleep(Duration::from_mins(10)));

    dpms.set_power_mode(OutputPowerMode::Off);
    assert!(dpms.is_sleeping());
    assert!(!dpms.is_active_and_stable());

    let woke = dpms.wake_outputs();
    assert!(woke);
    assert!(!dpms.is_sleeping());
    assert!(!dpms.wake_outputs());
}

#[test]
fn test_dpms_destroy_and_live_disable_wake() {
    let mut dpms = DpmsController::new();
    dpms.set_timeout_secs(300);

    // A compositor-initiated power-off (lid close, compositor OSD) is not
    // the daemon's to undo: the idle timer must leave it alone.
    dpms.on_compositor_mode(OutputPowerMode::Off);
    assert!(dpms.is_sleeping());
    assert!(!dpms.check_idle_sleep(Duration::from_mins(2)));
    assert!(dpms.is_sleeping());

    // A sleep the daemon requested itself is woken when the timeout is
    // extended past the elapsed presentation time.
    dpms.set_power_mode(OutputPowerMode::On);
    dpms.set_power_mode(OutputPowerMode::Off);
    assert!(dpms.is_sleeping());
    assert!(dpms.check_idle_sleep(Duration::from_mins(2)));
    assert!(!dpms.is_sleeping());

    // Disabling timeout (0) while sleeping wakes outputs
    dpms.set_power_mode(OutputPowerMode::Off);
    dpms.set_timeout_secs(0);
    assert!(dpms.check_idle_sleep(Duration::from_mins(15)));
    assert!(!dpms.is_sleeping());

    dpms.set_power_mode(OutputPowerMode::Off);
    assert!(dpms.is_sleeping());
    dpms.destroy();
    assert!(!dpms.is_sleeping());
}
