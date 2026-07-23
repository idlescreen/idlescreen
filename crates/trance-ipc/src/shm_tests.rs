// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn shm_name_accepts_daemon_format() {
    assert!(is_valid_shm_name("/trance-shm-1234-0"));
    assert!(is_valid_shm_name("/trance-shm-1-99"));
}

#[test]
fn shm_name_rejects_traversal_and_oddities() {
    assert!(!is_valid_shm_name("trance-shm-1-0"));
    assert!(!is_valid_shm_name("/other-1-0"));
    assert!(!is_valid_shm_name("/trance-shm-../etc"));
    assert!(!is_valid_shm_name("/trance-shm-"));
    assert!(!is_valid_shm_name("/trance-shm-a/b"));
    assert!(!is_valid_shm_name(&format!("/trance-shm-{}", "x".repeat(80))));
}

#[test]
fn socket_path_rejects_relative_and_dots() {
    assert!(is_plausible_socket_path("/run/user/1000/trance-uds-1-0.sock"));
    assert!(!is_plausible_socket_path("relative.sock"));
    assert!(!is_plausible_socket_path("/tmp/../etc/passwd.sock"));
    assert!(!is_plausible_socket_path("/tmp/foo"));
    assert!(!is_plausible_socket_path(""));
}

#[test]
fn create_rejects_bad_name() {
    match SharedMemory::create("/evil", 4096) {
        Err(err) => assert!(err.contains("invalid shm name")),
        Ok(_) => panic!("expected invalid name error"),
    }
}

#[test]
fn create_rejects_tiny_size() {
    match SharedMemory::create("/trance-shm-1-0", 1) {
        Err(err) => assert!(err.contains("size")),
        Ok(_) => panic!("expected size error"),
    }
}
