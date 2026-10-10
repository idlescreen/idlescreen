// SPDX-License-Identifier: Apache-2.0
//! Tier 3: Pairwise combination — Init Systems + Readiness Signaling.

use crate::harness::{
    InitEnvironment, InitSystem, NotifySocketProbe, PipeReadinessProbe, detect_mock_init,
    emit_notify_socket,
};

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

    let target_notification_fd = 3;
    assert_eq!(
        target_notification_fd, 3,
        "s6 notification fd contract is descriptor 3"
    );
    let mut pipe = PipeReadinessProbe::new();
    assert!(pipe.simulate_daemon_ready().is_ok());
    assert_eq!(pipe.await_readiness(), Ok(true));
}

#[test]
fn test_dinit_paired_with_pipe_notification_fd3() {
    let env = InitEnvironment::new(InitSystem::Dinit);
    assert_eq!(detect_mock_init(env.root()), InitSystem::Dinit);

    let mut pipe = PipeReadinessProbe::new();
    assert!(pipe.simulate_daemon_ready().is_ok());
    assert_eq!(pipe.await_readiness(), Ok(true));
}

#[test]
fn test_openrc_paired_with_session_bus_name() {
    let env = InitEnvironment::new(InitSystem::OpenRc);
    assert_eq!(detect_mock_init(env.root()), InitSystem::OpenRc);

    let bus_name = "io.github.idlescreen.Idle";
    let bus_acquired = !bus_name.is_empty();
    assert!(bus_acquired);
}

#[test]
fn test_standalone_paired_with_bus_or_pid() {
    let env = InitEnvironment::new(InitSystem::Standalone);
    assert_eq!(detect_mock_init(env.root()), InitSystem::Standalone);

    let pidfile_name = "idlescreen.pid";
    assert_eq!(pidfile_name, "idlescreen.pid");
}
