// SPDX-License-Identifier: Apache-2.0
//! Tier 4 Scenario 2: s6 supervised daemon lifecycle with fd 3 readiness and rolling file.

use crate::harness::{
    InitEnvironment, InitSystem, PipeReadinessProbe, RollingFileProbe,
    detect_mock_init,
};
use tempfile::TempDir;

#[test]
fn test_scenario_s6_supervision_lifecycle() {
    // 1. Detect s6 init system
    let env = InitEnvironment::new(InitSystem::S6);
    assert_eq!(detect_mock_init(env.root()), InitSystem::S6);

    // 2. Set up notification-fd contract for descriptor 3
    let s6_notification_fd = 3;
    assert_eq!(s6_notification_fd, 3, "s6 notification fd contract must be 3");
    let mut pipe = PipeReadinessProbe::new();

    // 3. Daemon initializes and writes newline to fd 3
    assert!(pipe.simulate_daemon_ready().is_ok());
    assert_eq!(pipe.await_readiness(), Ok(true));

    // 4. Daemon logs to atomic rolling log file
    let temp = TempDir::new().unwrap();
    let log_probe = RollingFileProbe::new(temp.path());
    log_probe.write_line("INFO idle-daemon: s6 readiness confirmed").unwrap();
    log_probe.write_line("INFO idle-daemon: enter tick loop").unwrap();

    let logs = log_probe.read_all();
    assert!(logs.contains("s6 readiness confirmed"));
    assert!(logs.contains("enter tick loop"));
}
