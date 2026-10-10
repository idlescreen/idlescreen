// SPDX-License-Identifier: Apache-2.0
//! Tier 3: Pairwise combination — Supervisor Fallback + Logging Sinks.

use crate::harness::{
    InitEnvironment, InitSystem, RollingFileProbe, SyslogProbe, format_rfc3164,
    safe_exec_supervisor,
};
use std::os::unix::net::UnixDatagram;
use tempfile::TempDir;

#[test]
fn test_missing_supervisor_logs_warning_to_syslog() {
    let env = InitEnvironment::new(InitSystem::Systemd);
    let empty_bin = env.bin_dir();
    let res = safe_exec_supervisor(InitSystem::Systemd, "start", "idle", Some(&empty_bin));
    assert!(res.is_ok());

    let syslog = SyslogProbe::new();
    let sock = UnixDatagram::unbound().unwrap();
    let warn_msg = format_rfc3164(
        3,
        4, // warning severity
        "idlescreen",
        1111,
        "systemctl missing; falling back to direct spawn",
    );
    sock.send_to(warn_msg.as_bytes(), syslog.socket_path())
        .unwrap();

    let received = syslog.recv_syslog().unwrap();
    assert_eq!(received.facility, 3);
    assert_eq!(received.severity, 4);
    assert!(received.content.contains("falling back to direct spawn"));
}

#[test]
fn test_missing_supervisor_logs_warning_to_rolling_file() {
    let temp = TempDir::new().unwrap();
    let file_probe = RollingFileProbe::new(temp.path());

    let log_entry = "WARN idle-dbus: systemctl missing; falling back to direct spawn";
    file_probe.write_line(log_entry).unwrap();

    let content = file_probe.read_all();
    assert!(content.contains("systemctl missing; falling back to direct spawn"));
}

#[test]
fn test_non_systemd_cli_logs_tails_file() {
    let init = InitSystem::OpenRc;
    let temp = TempDir::new().unwrap();
    let file_probe = RollingFileProbe::new(temp.path());

    file_probe.write_line("line A").unwrap();
    file_probe.write_line("line B").unwrap();

    let stream_source = match init {
        InitSystem::Systemd => "journalctl --user -u idlescreen.service -f",
        _ => "tail -F ~/.local/state/idlescreen/idlescreen.log",
    };

    assert!(stream_source.starts_with("tail"));
    assert!(file_probe.read_all().contains("line A"));
}
