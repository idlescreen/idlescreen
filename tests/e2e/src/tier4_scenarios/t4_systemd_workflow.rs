// SPDX-License-Identifier: Apache-2.0
//! Tier 4 Scenario 1: Systemd native service lifecycle with abstract NOTIFY_SOCKET.

use crate::harness::{
    InitEnvironment, InitSystem, NotifySocketProbe, detect_mock_init, emit_notify_socket,
};

#[test]
fn test_scenario_systemd_full_service_lifecycle() {
    // 1. Initialize host environment with systemd
    let env = InitEnvironment::new(InitSystem::Systemd);
    assert_eq!(detect_mock_init(env.root()), InitSystem::Systemd);

    // 2. Set up NOTIFY_SOCKET listener
    let notify = NotifySocketProbe::new();

    // 3. Daemon starts up and notifies readiness
    assert!(emit_notify_socket(notify.socket_path(), "READY=1\n").is_ok());
    assert_eq!(notify.recv_message(), Some("READY=1\n".to_string()));

    // 4. Daemon sends periodic watchdog heartbeats
    for _ in 0..3 {
        assert!(emit_notify_socket(notify.socket_path(), "WATCHDOG=1\n").is_ok());
        assert_eq!(notify.recv_message(), Some("WATCHDOG=1\n".to_string()));
    }

    // 5. System triggers shutdown; daemon notifies STOPPING=1
    assert!(emit_notify_socket(notify.socket_path(), "STOPPING=1\n").is_ok());
    assert_eq!(notify.recv_message(), Some("STOPPING=1\n".to_string()));
}
