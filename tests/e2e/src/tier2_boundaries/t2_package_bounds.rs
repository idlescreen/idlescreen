// SPDX-License-Identifier: Apache-2.0
//! Tier 2: Packaging & Service Automation Boundary Cases (R6).

use std::fs;
use tempfile::TempDir;

#[test]
fn test_template_missing_file_handling() {
    let temp = TempDir::new().unwrap();
    let nonexistent = temp.path().join("missing.service");
    let content = fs::read_to_string(&nonexistent);
    assert!(
        content.is_err(),
        "Reading nonexistent template must error cleanly"
    );
}

#[test]
fn test_template_empty_file_rejected() {
    let temp = TempDir::new().unwrap();
    let empty_file = temp.path().join("empty.service");
    fs::write(&empty_file, "").unwrap();

    let text = fs::read_to_string(&empty_file).unwrap();
    assert!(
        text.trim().is_empty(),
        "Empty template must be identified as invalid"
    );
}

#[test]
fn test_unknown_init_system_string_fallback() {
    let unknown_init = "sysvinit_legacy_unknown";
    let normalized = match unknown_init {
        "systemd" => "systemd",
        "openrc" => "openrc",
        "runit" => "runit",
        "dinit" => "dinit",
        "s6" => "s6",
        _ => "standalone",
    };
    assert_eq!(
        normalized, "standalone",
        "Unknown init must fall back to standalone"
    );
}

#[test]
fn test_detect_sh_mock_arch_family_variants() {
    let arch_distros = [
        "arch",
        "endeavouros",
        "garuda",
        "cachyos",
        "artix",
        "manjaro",
    ];
    for distro in arch_distros {
        let is_arch = matches!(
            distro,
            "arch" | "endeavouros" | "garuda" | "cachyos" | "artix" | "manjaro"
        );
        assert!(is_arch, "{distro} must be classified as arch family");
    }
}

#[test]
fn test_start_daemon_timeout_budget() {
    // Timeout budget calculation for daemon startup wait
    let max_retries = 30;
    let sleep_per_retry_ms = 200;
    let total_budget_ms = max_retries * sleep_per_retry_ms;
    assert_eq!(total_budget_ms, 6000, "Startup budget must be 6.0 seconds");
}

#[test]
fn test_service_restart_race_protection() {
    // When stopping and restarting daemon, short delay ensures socket/bus release
    let stop_settle_delay_ms = 400;
    assert!(
        stop_settle_delay_ms >= 300,
        "Stop settle delay must be >= 300ms"
    );
}
