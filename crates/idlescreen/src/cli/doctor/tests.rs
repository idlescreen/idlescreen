// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::inspect::{DoctorReport, check_systemd_status, collect_diagnostics, test_shm_writable};
use super::view::print_help;
use super::{execute, execute_args};

#[test]
fn test_collect_diagnostics() {
    let report = collect_diagnostics();
    assert!(!report.active_saver.is_empty());
    assert!(report.timeout_secs > 0);
    assert!(report.dwell_secs > 0.0);
    assert_eq!(report.input_debounce_ms, 500);
    assert!(!report.wayland_presenter_note.is_empty());
}

#[test]
fn test_execute_modes() {
    assert!(execute_args(&["--help".to_string()]).is_ok());
    assert!(execute_args(&["-h".to_string()]).is_ok());
    assert!(execute().is_ok());
    assert!(execute_args(&[]).is_ok());
    assert!(execute_args(&["-v".to_string()]).is_ok());
    assert!(execute_args(&["--verbose".to_string()]).is_ok());
    assert!(execute_args(&["--all".to_string()]).is_ok());
    assert!(execute_args(&["--json".to_string()]).is_ok());
}

#[test]
fn test_shm_probe() {
    let _ = test_shm_writable();
}

#[test]
fn test_systemd_probe() {
    let (_active, desc) = check_systemd_status();
    assert!(!desc.is_empty());
}

#[test]
fn test_print_help() {
    print_help();
}

#[test]
fn test_report_fields() {
    let report = DoctorReport {
        wayland_display: Some("wayland-0".to_string()),
        desktop_env: Some("GNOME".to_string()),
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        runtime_dir_ok: true,
        daemon_pid: Some(12345),
        daemon_running: true,
        control_sock_path: None,
        control_sock_active: true,
        session_bus_connected: true,
        dbus_service_active: true,
        systemd_active: true,
        systemd_desc: "idlescreen.service (active)".to_string(),
        inhibited: false,
        active_saver: "ascii".to_string(),
        timeout_secs: 300,
        dwell_secs: 5.0,
        mode_desc: "cycle",
        config_path: None,
        config_is_toml: true,
        shm_writable: true,
        input_debounce_ms: 500,
        wayland_presenter_ok: true,
        wayland_presenter_note: "layer-shell + wl_shm ready, 500ms debounce active".to_string(),
    };
    assert_eq!(report.active_saver, "ascii");
    assert!(report.daemon_running);
    assert!(report.dbus_service_active);
    assert_eq!(report.input_debounce_ms, 500);
    assert!(report.wayland_presenter_ok);
}
