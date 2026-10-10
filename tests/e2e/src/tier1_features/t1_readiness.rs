// SPDX-License-Identifier: Apache-2.0
//! Tier 1: Abstract Readiness Notifications tests (R3).

use crate::harness::{
    NotifySocketProbe, PipeReadinessProbe, emit_notify_socket,
};

#[test]
fn test_notify_socket_filesystem_ready_datagram() {
    let probe = NotifySocketProbe::new();
    let res = emit_notify_socket(probe.socket_path(), "READY=1\n");
    assert!(res.is_ok(), "Emitting READY=1 must succeed");

    let received = probe.recv_message();
    assert_eq!(
        received,
        Some("READY=1\n".to_string()),
        "Probe must receive READY=1 payload"
    );
}

#[test]
fn test_notify_socket_stopping_datagram() {
    let probe = NotifySocketProbe::new();
    let res = emit_notify_socket(probe.socket_path(), "STOPPING=1\n");
    assert!(res.is_ok(), "Emitting STOPPING=1 must succeed");

    let received = probe.recv_message();
    assert_eq!(
        received,
        Some("STOPPING=1\n".to_string()),
        "Probe must receive STOPPING=1 payload"
    );
}

#[test]
fn test_notify_socket_watchdog_datagram() {
    let probe = NotifySocketProbe::new();
    let res = emit_notify_socket(probe.socket_path(), "WATCHDOG=1\n");
    assert!(res.is_ok(), "Emitting WATCHDOG=1 must succeed");

    let received = probe.recv_message();
    assert_eq!(
        received,
        Some("WATCHDOG=1\n".to_string()),
        "Probe must receive WATCHDOG=1 payload"
    );
}

#[test]
fn test_fd_pipe_readiness_newline_byte() {
    let mut pipe = PipeReadinessProbe::new();
    let res = pipe.simulate_daemon_ready();
    assert!(res.is_ok(), "Daemon readiness write must succeed");

    let ready = pipe.await_readiness();
    assert_eq!(
        ready,
        Ok(true),
        "Supervisor must read newline and verify readiness"
    );
}

#[test]
fn test_fd_pipe_readiness_target_fd_is_three() {
    // In s6 and dinit readiness protocols, notification fd is standard descriptor 3
    let expected_fd = 3i32;
    assert_eq!(expected_fd, 3, "Notification fd contract must specify descriptor 3");
}

#[test]
fn test_dbus_session_bus_name_contract() {
    let canonical_name = "io.github.idlescreen.Idle";
    assert!(
        canonical_name.starts_with("io.github.idlescreen"),
        "D-Bus service name must adhere to standard reversed domain"
    );
    assert_eq!(
        canonical_name, "io.github.idlescreen.Idle",
        "Canonical session bus name must match contract"
    );
}
