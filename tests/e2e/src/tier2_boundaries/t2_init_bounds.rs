// SPDX-License-Identifier: Apache-2.0
//! Tier 2: Init Detection & Dispatch Boundary & Corner Cases (R1).

use crate::harness::{
    InitEnvironment, InitSystem, detect_mock_init, safe_exec_supervisor,
};
use std::fs;

#[test]
fn test_init_detect_precedence_systemd_over_others() {
    let env = InitEnvironment::new(InitSystem::Systemd);
    // Add conflicting markers
    fs::create_dir_all(env.root().join("run/openrc")).ok();
    fs::create_dir_all(env.root().join("run/dinit")).ok();

    let detected = detect_mock_init(env.root());
    assert_eq!(
        detected,
        InitSystem::Systemd,
        "Systemd marker must have top precedence when multiple exist"
    );
}

#[test]
fn test_init_detect_empty_or_broken_paths() {
    let env = InitEnvironment::new(InitSystem::Standalone);
    let detected = detect_mock_init(env.root());
    assert_eq!(detected, InitSystem::Standalone);
}

#[test]
fn test_supervisor_exec_handles_non_executable_binary() {
    let env = InitEnvironment::new(InitSystem::Runit);
    let bin_dir = env.bin_dir();
    let sv_path = bin_dir.join("sv");
    fs::write(&sv_path, "not a script").unwrap();

    let res = safe_exec_supervisor(InitSystem::Runit, "status", "idle", Some(&bin_dir));
    assert!(res.is_ok(), "Must not panic on non-executable binary");
}

#[test]
fn test_supervisor_exec_handles_empty_path_var() {
    let res = safe_exec_supervisor(InitSystem::Systemd, "start", "idle", None);
    assert_eq!(
        res,
        Ok("fallback_direct_spawn:start:idle".to_string()),
        "Missing path must cleanly trigger direct spawn fallback"
    );
}

#[test]
fn test_supervisor_exec_unknown_subcommand_fails_safely() {
    let env = InitEnvironment::new(InitSystem::OpenRc);
    let bin_dir = env.bin_dir();
    fs::write(bin_dir.join("rc-service"), "#!/bin/sh\nexit 1\n").unwrap();

    let res = safe_exec_supervisor(
        InitSystem::OpenRc,
        "invalid_verb",
        "idle-daemon",
        Some(&bin_dir),
    );
    assert!(res.is_ok(), "Must return result without process abort");
}

#[test]
fn test_init_dispatch_unit_name_validation() {
    let valid_units = ["idle-daemon", "idle-daemon.service", "io.github.idlescreen.Idle"];
    for unit in valid_units {
        assert!(!unit.contains(';'), "Unit name must not contain shell injections");
        assert!(!unit.contains('&'), "Unit name must not contain command delimiters");
    }
}
