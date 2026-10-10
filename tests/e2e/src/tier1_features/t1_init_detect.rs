// SPDX-License-Identifier: Apache-2.0
//! Tier 1: Multi-Init Detection & Service Dispatch tests (R1).

use crate::harness::{InitEnvironment, InitSystem, detect_mock_init, safe_exec_supervisor};
use std::fs;

#[test]
fn test_detect_systemd_when_run_systemd_present() {
    let env = InitEnvironment::new(InitSystem::Systemd);
    let detected = detect_mock_init(env.root());
    assert_eq!(detected, InitSystem::Systemd, "Expected systemd detection");
}

#[test]
fn test_detect_openrc_when_run_openrc_present() {
    let env = InitEnvironment::new(InitSystem::OpenRc);
    let detected = detect_mock_init(env.root());
    assert_eq!(detected, InitSystem::OpenRc, "Expected OpenRC detection");
}

#[test]
fn test_detect_runit_when_runit_present() {
    let env = InitEnvironment::new(InitSystem::Runit);
    let detected = detect_mock_init(env.root());
    assert_eq!(detected, InitSystem::Runit, "Expected runit detection");
}

#[test]
fn test_detect_dinit_when_dinit_present() {
    let env = InitEnvironment::new(InitSystem::Dinit);
    let detected = detect_mock_init(env.root());
    assert_eq!(detected, InitSystem::Dinit, "Expected dinit detection");
}

#[test]
fn test_detect_s6_when_service_present() {
    let env = InitEnvironment::new(InitSystem::S6);
    let detected = detect_mock_init(env.root());
    assert_eq!(detected, InitSystem::S6, "Expected s6 detection");
}

#[test]
fn test_detect_standalone_when_no_supervisor() {
    let env = InitEnvironment::new(InitSystem::Standalone);
    let detected = detect_mock_init(env.root());
    assert_eq!(
        detected,
        InitSystem::Standalone,
        "Expected standalone fallback"
    );
}

#[test]
fn test_supervisor_safe_exec_with_mock_binary() {
    let env = InitEnvironment::new(InitSystem::OpenRc);
    let bin_dir = env.bin_dir();
    let rc_service = bin_dir.join("rc-service");
    fs::write(&rc_service, "#!/bin/sh\nexit 0\n").expect("write fake rc-service");

    let result = safe_exec_supervisor(InitSystem::OpenRc, "start", "idle-daemon", Some(&bin_dir));
    assert_eq!(
        result,
        Ok("success:rc-service:start:idle-daemon".to_string()),
        "rc-service execution should succeed"
    );
}

#[test]
fn test_supervisor_safe_exec_fallback_when_binary_missing() {
    let env = InitEnvironment::new(InitSystem::Systemd);
    let empty_bin_dir = env.bin_dir();

    // Supervisor binary 'systemctl' is missing; must fallback gracefully
    let result = safe_exec_supervisor(
        InitSystem::Systemd,
        "start",
        "idle-daemon",
        Some(&empty_bin_dir),
    );
    assert_eq!(
        result,
        Ok("fallback_direct_spawn:start:idle-daemon".to_string()),
        "Missing binary must fall back to direct spawn without io::ErrorKind::NotFound abort"
    );
}
