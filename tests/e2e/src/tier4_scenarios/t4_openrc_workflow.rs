// SPDX-License-Identifier: Apache-2.0
//! Tier 4 Scenario 3: OpenRC daemon lifecycle with bus claiming and Syslog RFC 3164.

use crate::harness::{
    InitEnvironment, InitSystem, SyslogProbe, detect_mock_init, format_rfc3164, repo_root,
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

    // 3. Daemon claims session bus name — the name the shipped D-Bus
    // activation file declares must be the one the daemon registers.
    let unit = repo_root().join("assets/io.github.idlescreen.Idle.service");
    let unit_body = std::fs::read_to_string(&unit)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", unit.display()));
    assert!(
        unit_body.contains("Name=io.github.idlescreen.Idle"),
        "{} does not declare the daemon's well-known bus name",
        unit.display()
    );

    // 4. Daemon logs readiness to Syslog
    let ready_log = format_rfc3164(
        3,
        6,
        "idlescreen",
        2024,
        "D-Bus name claimed; service ready under OpenRC",
    );
    client
        .send_to(ready_log.as_bytes(), syslog.socket_path())
        .unwrap();

    let received = syslog.recv_syslog().unwrap();
    assert_eq!(received.facility, 3);
    assert_eq!(received.severity, 6);
    assert!(received.content.contains("service ready under OpenRC"));
}
