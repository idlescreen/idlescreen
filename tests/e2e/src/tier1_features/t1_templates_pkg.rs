// SPDX-License-Identifier: Apache-2.0
//! Tier 1: Service Templates & Packaging Automation tests (R6).
//!
//! These assert against the templates the package actually ships, not
//! against inline copies of them: a literal that is compared with itself
//! proves nothing about the shipped artefact.

use crate::harness::{InitSystem, repo_root};
use std::fs;

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

#[test]
fn test_openrc_service_template_contract() {
    let template = read("assets/services/idlescreen.openrc");
    assert!(
        template.starts_with("#!/sbin/openrc-run"),
        "the OpenRC template must carry the openrc-run shebang"
    );
    assert!(
        template.contains(r#"command="/usr/bin/idlescreen""#),
        "the OpenRC template must launch the packaged binary"
    );
    assert!(
        template.contains(r#"command_args="daemon""#),
        "the OpenRC template must start the daemon subcommand"
    );
    assert!(
        !template.contains("/run/idlescreen.pid"),
        "the OpenRC template must not point at the legacy root-owned pidfile"
    );
}

#[test]
fn test_runit_service_template_contract() {
    let template = read("assets/services/idlescreen.runit");
    assert!(
        template.starts_with("#!/bin/sh"),
        "the runit template must carry a POSIX shell shebang"
    );
    assert!(
        template.contains("exec /usr/bin/idlescreen daemon"),
        "the runit template must exec the daemon as the service process"
    );
    let log = read("assets/services/idlescreen.runit-log");
    assert!(
        log.contains("svlogd"),
        "the runit log service must hand output to svlogd"
    );
}

#[test]
fn test_dinit_service_template_contract() {
    let template = read("assets/services/idlescreen.dinit");
    assert!(
        template.contains("type = process"),
        "the dinit template must declare a process-type service"
    );
    assert!(
        template.contains("command = /usr/bin/idlescreen daemon"),
        "the dinit template must start the daemon subcommand"
    );
    assert!(
        template.contains("restart = true"),
        "the dinit template must restart the daemon"
    );
}

#[test]
fn test_s6_service_template_contract() {
    let run_script = read("assets/services/idlescreen.s6-run");
    assert!(
        run_script.starts_with("#!/usr/bin/execlineb -P"),
        "the s6 template must carry the execline shebang"
    );
    assert!(
        run_script.contains("idlescreen daemon"),
        "the s6 template must start the daemon subcommand"
    );
    let service_type = read("assets/services/idlescreen.s6-type");
    assert_eq!(
        service_type.trim(),
        "longrun",
        "s6-overlay needs a longrun type file next to the run script"
    );
}

#[test]
fn test_xdg_desktop_autostart_contract() {
    let desktop_entry = read("assets/idlescreen.desktop");
    assert!(
        desktop_entry.contains("[Desktop Entry]"),
        "the autostart entry needs a Desktop Entry group"
    );
    assert!(
        desktop_entry.contains("Type=Application"),
        "the autostart entry must be an Application"
    );
    assert!(
        desktop_entry.contains("Exec=idlescreen daemon"),
        "the autostart entry must start the daemon subcommand"
    );
    assert!(
        desktop_entry.contains("X-GNOME-HiddenUnderSystemd=true"),
        "the autostart entry must stay out of the way of the systemd user unit"
    );
    assert!(
        desktop_entry.contains("X-systemd-skip=true"),
        "the autostart entry must not start a second daemon under systemd"
    );
}

#[test]
fn test_detect_sh_init_system_contract() {
    // Every init system the installer advertises must have a shipped
    // template and a distinct supervisor binary.
    let root = repo_root();
    for (variant, rel) in [
        (InitSystem::Systemd, "assets/idlescreen.service"),
        (InitSystem::OpenRc, "assets/services/idlescreen.openrc"),
        (InitSystem::Runit, "assets/services/idlescreen.runit"),
        (InitSystem::Dinit, "assets/services/idlescreen.dinit"),
        (InitSystem::S6, "assets/services/idlescreen.s6-run"),
    ] {
        assert!(
            root.join(rel).exists(),
            "{variant:?} is advertised but {rel} is not shipped"
        );
    }
}
