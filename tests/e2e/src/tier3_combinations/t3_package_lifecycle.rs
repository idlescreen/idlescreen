// SPDX-License-Identifier: Apache-2.0
//! Tier 3: Pairwise combination — Package Detection + Service Template Deployment.

use crate::harness::{InitEnvironment, InitSystem, detect_mock_init};

#[test]
fn test_openrc_detection_selects_openrc_template() {
    let env = InitEnvironment::new(InitSystem::OpenRc);
    let init = detect_mock_init(env.root());
    let template_dir = match init {
        InitSystem::OpenRc => "assets/services/openrc",
        _ => "unknown",
    };
    assert_eq!(template_dir, "assets/services/openrc");
}

#[test]
fn test_s6_detection_selects_s6_template_and_notification_fd() {
    let env = InitEnvironment::new(InitSystem::S6);
    let init = detect_mock_init(env.root());
    let (template_dir, fd_file) = match init {
        InitSystem::S6 => ("assets/services/s6", "assets/services/s6/notification-fd"),
        _ => ("unknown", ""),
    };
    assert_eq!(template_dir, "assets/services/s6");
    assert!(fd_file.ends_with("notification-fd"));
}

#[test]
fn test_runit_detection_selects_runit_template() {
    let env = InitEnvironment::new(InitSystem::Runit);
    let init = detect_mock_init(env.root());
    let template_dir = match init {
        InitSystem::Runit => "assets/services/runit",
        _ => "unknown",
    };
    assert_eq!(template_dir, "assets/services/runit");
}

#[test]
fn test_dinit_detection_selects_dinit_template() {
    let env = InitEnvironment::new(InitSystem::Dinit);
    let init = detect_mock_init(env.root());
    let template_dir = match init {
        InitSystem::Dinit => "assets/services/dinit",
        _ => "unknown",
    };
    assert_eq!(template_dir, "assets/services/dinit");
}
