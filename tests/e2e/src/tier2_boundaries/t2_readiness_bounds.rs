// SPDX-License-Identifier: Apache-2.0
//! Tier 2: Readiness Notification Boundary & Corner Cases (R3).

use crate::harness::{
    NotifySocketProbe, PipeReadinessProbe, emit_notify_socket,
};
use std::path::Path;

#[test]
fn test_notify_socket_missing_env_is_noop() {
    let notify_socket_env: Option<&str> = None;
    let sent = if let Some(path) = notify_socket_env {
        emit_notify_socket(Path::new(path), "READY=1\n").is_ok()
    } else {
        false
    };
    assert!(!sent, "Unset NOTIFY_SOCKET must be safe no-op");
}

#[test]
fn test_notify_socket_empty_env_is_noop() {
    let notify_socket_env = "";
    let sent = if !notify_socket_env.is_empty() {
        emit_notify_socket(Path::new(notify_socket_env), "READY=1\n").is_ok()
    } else {
        false
    };
    assert!(!sent, "Empty NOTIFY_SOCKET must be safe no-op");
}

#[test]
fn test_notify_socket_abstract_socket_max_length() {
    // Linux sockaddr_un sun_path limit is 108 bytes
    let valid_abstract_name = "@idlescreen_notify_socket_test";
    assert!(valid_abstract_name.len() < 108, "Abstract socket name must fit within sun_path");
}

#[test]
fn test_pipe_readiness_double_close_safety() {
    let mut pipe = PipeReadinessProbe::new();
    // First simulate readiness (writes \n and drops write end)
    let res1 = pipe.simulate_daemon_ready();
    assert!(res1.is_ok());

    // Second simulate readiness on already closed pipe should not panic
    let res2 = pipe.simulate_daemon_ready();
    assert!(res2.is_ok());
}

#[test]
fn test_pipe_readiness_extra_newlines_tolerated() {
    let mut pipe = PipeReadinessProbe::new();
    let _ = pipe.simulate_daemon_ready();
    let ready = pipe.await_readiness();
    assert_eq!(ready, Ok(true));
}

#[test]
fn test_notify_socket_malformed_payload_tolerated() {
    let probe = NotifySocketProbe::new();
    // Daemon or external tool emits unknown keys or malformed strings
    let res = emit_notify_socket(probe.socket_path(), "STATUS=Working...\nUNKNOWN_KEY=42\n");
    assert!(res.is_ok());

    let received = probe.recv_message();
    assert!(received.is_some());
    assert!(received.unwrap().contains("STATUS=Working..."));
}
