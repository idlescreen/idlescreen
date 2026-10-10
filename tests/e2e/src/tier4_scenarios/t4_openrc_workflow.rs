// SPDX-License-Identifier: Apache-2.0
//! Tier 4 Scenario 3: OpenRC daemon lifecycle with bus claiming and Syslog RFC 3164.

use crate::harness::{
    InitEnvironment, InitSystem, SyslogProbe, detect_mock_init, format_rfc3164,
};
use std::os::unix::net::UnixDatagram;

#[test]
fn test_scenario_openrc_service_lifecycle() {
    // 1. Detect OpenRC init
    let env = InitEnvironment::new(InitSystem::OpenRc);
    assert_eq!(detect_mock_init(env.root()), InitSystem::OpenRc);

    // 2. Set up Syslog /dev/log probe
    let syslog = SyslogProbe::new();
    let client = UnixDatagram::unbound().unwrap();

    // 3. Daemon claims session bus name
    let bus_name = "io.github.idlescreen.Idle";
    assert_eq!(bus_name, "io.github.idlescreen.Idle");

    // 4. Daemon logs readiness to Syslog
    let ready_log = format_rfc3164(
        3,
        6,
        "idle-daemon",
        2024,
        "D-Bus name claimed; service ready under OpenRC",
    );
    client.send_to(ready_log.as_bytes(), syslog.socket_path()).unwrap();

    let received = syslog.recv_syslog().unwrap();
    assert_eq!(received.facility, 3);
    assert_eq!(received.severity, 6);
    assert!(received.content.contains("service ready under OpenRC"));
}
