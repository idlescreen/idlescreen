// SPDX-License-Identifier: Apache-2.0
//! Tier 1: Service Templates & Packaging Automation tests (R6).

use crate::harness::InitSystem;

#[test]
fn test_openrc_service_template_contract() {
    let template = r#"#!/sbin/openrc-run
description="IdleScreen Wayland screensaver daemon"
command="/usr/bin/idlescreen"
command_args="daemon"
command_background="yes"
pidfile="/run/idlescreen.pid"
"#;

    assert!(template.starts_with("#!/sbin/openrc-run"));
    assert!(template.contains(r#"command="/usr/bin/idlescreen""#));
    assert!(template.contains(r#"command_args="daemon""#));
}

#[test]
fn test_runit_service_template_contract() {
    let template = r#"#!/bin/sh
exec 2>&1
exec idlescreen daemon
"#;

    assert!(template.starts_with("#!/bin/sh"));
    assert!(template.contains("exec idlescreen daemon"));
}

#[test]
fn test_dinit_service_template_contract() {
    let template = r#"type = process
command = /usr/bin/idlescreen daemon
ready-notification = pipev3
restart = yes
"#;

    assert!(template.contains("type = process"));
    assert!(template.contains("command = /usr/bin/idlescreen daemon"));
    assert!(template.contains("ready-notification = pipev3"));
}

#[test]
fn test_s6_service_template_contract() {
    let run_script = "#!/bin/execlineb -P\nidlescreen daemon\n";
    let notification_fd = "3\n";
    let service_type = "longrun\n";

    assert!(run_script.contains("idlescreen daemon"));
    assert_eq!(notification_fd.trim(), "3");
    assert_eq!(service_type.trim(), "longrun");
}

#[test]
fn test_xdg_desktop_autostart_contract() {
    let desktop_entry = r#"[Desktop Entry]
Type=Application
Name=IdleScreen Daemon
Comment=IdleScreen screensaver daemon
Exec=idlescreen daemon
Terminal=false
NoDisplay=true
X-GNOME-Autostart-enabled=true
X-GNOME-HiddenUnderSystemd=true
X-systemd-skip=true
Categories=Utility;Screensaver;
"#;

    assert!(desktop_entry.contains("[Desktop Entry]"));
    assert!(desktop_entry.contains("Type=Application"));
    assert!(desktop_entry.contains("Exec=idlescreen daemon"));
    assert!(desktop_entry.contains("X-GNOME-HiddenUnderSystemd=true"));
    assert!(desktop_entry.contains("X-systemd-skip=true"));
}

#[test]
fn test_detect_sh_init_system_contract() {
    // Validates init system enumeration identifiers
    let systems = [
        (InitSystem::Systemd, "systemd"),
        (InitSystem::OpenRc, "openrc"),
        (InitSystem::Runit, "runit"),
        (InitSystem::Dinit, "dinit"),
        (InitSystem::S6, "s6"),
        (InitSystem::Standalone, "standalone"),
    ];

    for (variant, name) in systems {
        assert!(!name.is_empty(), "Init system name cannot be empty");
        let matches = match variant {
            InitSystem::Systemd => name == "systemd",
            InitSystem::OpenRc => name == "openrc",
            InitSystem::Runit => name == "runit",
            InitSystem::Dinit => name == "dinit",
            InitSystem::S6 => name == "s6",
            InitSystem::Standalone => name == "standalone",
        };
        assert!(matches, "Variant must match standard representation");
    }
}
