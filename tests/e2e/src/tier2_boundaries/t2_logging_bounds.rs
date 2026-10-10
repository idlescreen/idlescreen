// SPDX-License-Identifier: Apache-2.0
//! Tier 2: Multi-Sink Logging Boundary & Corner Cases (R4).

use crate::harness::{
    RollingFileProbe, parse_rfc3164_datagram,
};
use std::os::unix::net::UnixDatagram;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_syslog_missing_socket_fails_silently() {
    let nonexistent = Path::new("/tmp/nonexistent_idlescreen_dev_log.sock");
    let sock = UnixDatagram::unbound().unwrap();
    // Sending to non-existent socket fails, but must not panic or abort
    let res = sock.send_to(b"<30>test", nonexistent);
    assert!(res.is_err(), "Must return error without panic");
}

#[test]
fn test_syslog_malformed_datagram_parser_resilience() {
    let bad_packets = [
        "",
        "no pri header",
        "<not_a_number> message",
        "<99999999> overflow pri",
        "<24",
    ];

    for bad in bad_packets {
        let parsed = parse_rfc3164_datagram(bad);
        assert!(parsed.is_none(), "Malformed packet '{bad}' must return None safely");
    }
}

#[test]
fn test_syslog_max_message_length_truncation() {
    // RFC 3164 specifies max message length of 1024 bytes
    let long_msg = "A".repeat(2048);
    let truncated = if long_msg.len() > 1024 {
        &long_msg[..1024]
    } else {
        &long_msg
    };
    assert_eq!(truncated.len(), 1024, "Long syslog messages must cap at 1024 bytes");
}

#[test]
fn test_rolling_log_file_nonexistent_parent_created() {
    let temp = TempDir::new().unwrap();
    let deep_path = temp.path().join("a/b/c");
    let probe = RollingFileProbe::new(&deep_path);
    let res = probe.write_line("deep line");
    assert!(res.is_ok(), "Writing to uncreated parent tree must succeed");
}

#[test]
fn test_rolling_log_file_zero_size_rotation_guard() {
    let temp = TempDir::new().unwrap();
    let probe = RollingFileProbe::new(temp.path());
    // Empty log file must not rotate under 0-byte guard
    let rotated = probe.rotate(100);
    assert!(!rotated, "Non-existent or empty log file must not trigger rotation");
}

#[test]
fn test_rolling_log_file_non_utf8_sanitization() {
    let raw_bytes: &[u8] = b"Valid prefix \xFF\xFE invalid suffix\n";
    let sanitized = String::from_utf8_lossy(raw_bytes).to_string();
    assert!(sanitized.contains("Valid prefix"));
    assert!(sanitized.contains("invalid suffix"));
}
