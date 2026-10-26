// SPDX-License-Identifier: Apache-2.0
//! Tier 3: Pairwise combination — Init Systems + Readiness Signaling.

use crate::harness::{
    InitEnvironment, InitSystem, NotifySocketProbe, PipeReadinessProbe, detect_mock_init,
    emit_notify_socket, repo_root,
};
use std::fs;

#[test]
fn test_systemd_paired_with_notify_socket() {
    let env = InitEnvironment::new(InitSystem::Systemd);
    assert_eq!(detect_mock_init(env.root()), InitSystem::Systemd);

    let notify = NotifySocketProbe::new();
    let res = emit_notify_socket(notify.socket_path(), "READY=1\n");
    assert!(res.is_ok());
    assert_eq!(notify.recv_message(), Some("READY=1\n".to_string()));
}

#[test]
fn test_s6_paired_with_pipe_notification_fd3() {
    let env = InitEnvironment::new(InitSystem::S6);
    assert_eq!(detect_mock_init(env.root()), InitSystem::S6);

    let mut pipe = PipeReadinessProbe::new();
    // The s6 contract is a notification file descriptor: the supervisor
    // passes the write end and waits for the readiness byte followed by EOF.
    assert!(pipe.take_write_fd() >= 0, "notification fd must be valid");
    assert!(pipe.simulate_daemon_ready().is_ok());
    assert_eq!(pipe.await_readiness(), Ok(true));
}

#[test]
fn test_dinit_paired_with_pipe_notification_fd3() {
    let env = InitEnvironment::new(InitSystem::Dinit);
    assert_eq!(detect_mock_init(env.root()), InitSystem::Dinit);

    let mut pipe = PipeReadinessProbe::new();
    assert!(pipe.take_write_fd() >= 0, "notification fd must be valid");
    assert!(pipe.simulate_daemon_ready().is_ok());
    assert_eq!(pipe.await_readiness(), Ok(true));
}

#[test]
fn test_openrc_paired_with_session_bus_name() {
    let env = InitEnvironment::new(InitSystem::OpenRc);
    assert_eq!(detect_mock_init(env.root()), InitSystem::OpenRc);

    // The D-Bus activation file that ships in the package must claim the
    // same well-known name the daemon registers, or the session bus never
    // routes to IdleScreen.
    let unit = repo_root().join("assets/io.github.idlescreen.Idle.service");
    let body =
        fs::read_to_string(&unit).unwrap_or_else(|e| panic!("cannot read {}: {e}", unit.display()));
    assert!(
        body.contains("Name=io.github.idlescreen.Idle"),
        "{} does not declare the daemon's well-known bus name",
        unit.display()
    );
    assert!(
        body.contains("Exec="),
        "{} must carry an Exec= line so the bus can activate the daemon",
        unit.display()
    );
}

#[test]
fn test_standalone_paired_with_bus_or_pid() {
    let env = InitEnvironment::new(InitSystem::Standalone);
    assert_eq!(detect_mock_init(env.root()), InitSystem::Standalone);

    // The pidfile name is a contract between the daemon and every service
    // template. The daemon writes it under its runtime directory; a template
    // that points at a different path would supervise a process it can
    // never find.
    let root = repo_root();
    let pidfile_src = root.join("crates/idlescreen/src/daemon/pidfile.rs");
    let body = fs::read_to_string(&pidfile_src)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", pidfile_src.display()));
    assert!(
        body.contains("\"idlescreen.pid\""),
        "the daemon no longer writes idlescreen.pid"
    );
    for rel in [
        "assets/services/idlescreen.openrc",
        "assets/services/idlescreen.runit",
        "assets/services/idlescreen.dinit",
        "assets/services/idlescreen.s6-run",
    ] {
        let path = root.join(rel);
        let body = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert!(
            !body.contains("/run/idlescreen.pid"),
            "{rel} points at the legacy root-owned pidfile path"
        );
    }
}
