// SPDX-License-Identifier: Apache-2.0
//! Tier 1: Multi-Sink Logging & Log Viewer tests (R4).

use crate::harness::{
    RollingFileProbe, SyslogProbe, format_rfc3164,
};
use std::os::unix::net::UnixDatagram;
use tempfile::TempDir;

#[test]
fn test_syslog_rfc3164_format_daemon_info() {
    // Facility daemon (3), Severity info (6) -> PRI = 3 * 8 + 6 = 30
    let formatted = format_rfc3164(3, 6, "idle-daemon", 1234, "daemon started");
    assert!(formatted.starts_with("<30>"), "Expected <30> priority prefix");
    assert!(formatted.contains("idle-daemon[1234]: daemon started"));
}

#[test]
fn test_syslog_rfc3164_format_daemon_err() {
    // Facility daemon (3), Severity err (3) -> PRI = 3 * 8 + 3 = 27
    let formatted = format_rfc3164(3, 3, "idle-daemon", 1234, "unrecoverable error");
    assert!(formatted.starts_with("<27>"), "Expected <27> priority prefix");
    assert!(formatted.contains("idle-daemon[1234]: unrecoverable error"));
}

#[test]
fn test_syslog_datagram_emission_and_reception() {
    let probe = SyslogProbe::new();
    let sock = UnixDatagram::unbound().expect("create client datagram socket");
    let payload = format_rfc3164(3, 6, "idle-daemon", 5555, "render loop active");
    sock.send_to(payload.as_bytes(), probe.socket_path())
        .expect("send syslog packet");

    let received = probe.recv_syslog();
    assert!(received.is_some(), "Syslog packet must be received");
    let parsed = received.unwrap();
    assert_eq!(parsed.priority, 30);
    assert_eq!(parsed.facility, 3);
    assert_eq!(parsed.severity, 6);
    assert!(parsed.content.contains("idle-daemon[5555]: render loop active"));
}

#[test]
fn test_rolling_log_file_creation_under_state_dir() {
    let temp = TempDir::new().expect("create temp dir");
    let probe = RollingFileProbe::new(temp.path());

    assert!(
        probe.path().ends_with(".local/state/idlescreen/idle-daemon.log"),
        "Log file path must follow canonical state directory convention"
    );

    let res = probe.write_line("INFO idle-daemon: initialized successfully");
    assert!(res.is_ok(), "Writing log line must succeed");
    assert!(probe.path().exists(), "Log file must exist on disk");
}

#[test]
fn test_rolling_log_file_append_lines() {
    let temp = TempDir::new().expect("create temp dir");
    let probe = RollingFileProbe::new(temp.path());

    probe.write_line("line 1").unwrap();
    probe.write_line("line 2").unwrap();
    probe.write_line("line 3").unwrap();

    let content = probe.read_all();
    assert!(content.contains("line 1\nline 2\nline 3\n"));
}

#[test]
fn test_rolling_log_file_rotation_on_size() {
    let temp = TempDir::new().expect("create temp dir");
    let probe = RollingFileProbe::new(temp.path());

    for _ in 0..100 {
        probe.write_line("This is a rolling test line padding bytes").unwrap();
    }

    let rotated = probe.rotate(500); // 500 byte limit
    assert!(rotated, "Log file exceeding threshold must rotate");
    let backup = probe.path().with_extension("log.1");
    assert!(backup.exists(), "Rotated backup file must exist on disk");
}
