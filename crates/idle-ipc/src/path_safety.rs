// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Path / name validation for IPC sockets and POSIX SHM objects.

/// POSIX SHM object names we create look like `/trance-shm-<pid>-<idx>`.
/// Reject anything else so a compromised arg vector cannot open arbitrary objects.
pub fn is_valid_shm_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("/trance-shm-") else {
        return false;
    };
    if rest.is_empty() || rest.len() > 64 {
        return false;
    }
    rest.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// UDS control paths must be absolute, end in `.sock`, stay under path limits,
/// and must not contain nulls or `..` segments.
pub fn is_plausible_socket_path(path: &str) -> bool {
    if path.is_empty() || path.len() >= 108 {
        return false;
    }
    if path.contains('\0') || !path.starts_with('/') || !path.ends_with(".sock") {
        return false;
    }
    if path.split('/').any(|seg| seg == "..") {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shm_name_accepts_daemon_format() {
        assert!(is_valid_shm_name("/trance-shm-1234-0"));
        assert!(is_valid_shm_name("/trance-shm-1-99"));
        assert!(is_valid_shm_name("/trance-shm-test_name-0"));
        assert!(is_valid_shm_name(&format!(
            "/trance-shm-{}",
            "a".repeat(64)
        )));
    }

    #[test]
    fn shm_name_rejects_traversal_and_oddities() {
        assert!(!is_valid_shm_name("trance-shm-1-0"));
        assert!(!is_valid_shm_name("/other-1-0"));
        assert!(!is_valid_shm_name("/trance-shm-../etc"));
        assert!(!is_valid_shm_name("/trance-shm-"));
        assert!(!is_valid_shm_name("/trance-shm-a/b"));
        assert!(!is_valid_shm_name("/trance-shm-a b"));
        assert!(!is_valid_shm_name("/trance-shm-a;b"));
        assert!(!is_valid_shm_name("/TRANCE-SHM-1-0"));
        assert!(!is_valid_shm_name(&format!(
            "/trance-shm-{}",
            "x".repeat(80)
        )));
        assert!(!is_valid_shm_name(&format!(
            "/trance-shm-{}",
            "x".repeat(65)
        )));
    }

    #[test]
    fn socket_path_rejects_relative_and_dots() {
        assert!(is_plausible_socket_path(
            "/run/user/1000/trance-uds-1-0.sock"
        ));
        assert!(!is_plausible_socket_path("relative.sock"));
        assert!(!is_plausible_socket_path("/tmp/../etc/passwd.sock"));
        assert!(!is_plausible_socket_path("/tmp/foo"));
        assert!(!is_plausible_socket_path(""));
        assert!(!is_plausible_socket_path("/tmp/foo.sock\0x"));
        assert!(!is_plausible_socket_path(&format!(
            "/{}.sock",
            "a".repeat(108)
        )));
        assert!(!is_plausible_socket_path(&"x".repeat(108)));
    }

    #[test]
    fn socket_path_accepts_tmp_and_runtime() {
        assert!(is_plausible_socket_path("/tmp/trance-uds-1-0.sock"));
        assert!(is_plausible_socket_path(
            "/run/user/1000/idle-uds-42-1.sock"
        ));
        // sun_path is 108 bytes — length 107 must remain accepted.
        let ok_107 = format!("/{}.sock", "a".repeat(101));
        assert_eq!(ok_107.len(), 107);
        assert!(is_plausible_socket_path(&ok_107));
        let bad_108 = format!("/{}.sock", "a".repeat(102));
        assert_eq!(bad_108.len(), 108);
        assert!(!is_plausible_socket_path(&bad_108));
    }

    #[test]
    fn socket_path_rejects_dotdot_middle_segment() {
        assert!(!is_plausible_socket_path("/run/../user/x.sock"));
        assert!(!is_plausible_socket_path("/a/b/../c.sock"));
    }
}
