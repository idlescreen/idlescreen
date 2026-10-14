// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Binary path resolution and unit tests for idlescreen-extras IPC.

use std::path::{Path, PathBuf};

pub const DEFAULT_EXTRAS_BINARY: &str = "/usr/libexec/idlescreen/idlescreen-extras";

/// Resolves path to idlescreen-extras binary across Linux distributions and dev environments.
pub fn resolve_extras_binary() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("IDLESCREEN_EXTRAS_BIN") {
        let p = PathBuf::from(env_path);
        if is_executable(&p) {
            return Some(p);
        }
    }

    for loc in &[
        DEFAULT_EXTRAS_BINARY,
        "/usr/lib/idlescreen/idlescreen-extras",
        "/usr/local/libexec/idlescreen/idlescreen-extras",
        "/usr/local/lib/idlescreen/idlescreen-extras",
    ] {
        let p = Path::new(loc);
        if is_executable(p) {
            return Some(p.to_path_buf());
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let user_bin = Path::new(&home).join(".local/bin/idlescreen-extras");
        if is_executable(&user_bin) {
            return Some(user_bin);
        }
    }

    if let Ok(current) = std::env::current_exe()
        && let Some(dir) = current.parent()
    {
        for candidate in &[
            dir.join("idlescreen-extras"),
            dir.join("../release/idlescreen-extras"),
            dir.join("../debug/idlescreen-extras"),
        ] {
            if is_executable(candidate) {
                return Some(candidate.clone());
            }
        }
    }

    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("idlescreen-extras");
            if is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = path.metadata() {
        meta.is_file() && (meta.permissions().mode() & 0o111 != 0)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::super::breaker::{
        is_circuit_breaker_tripped, record_circuit_breaker_failure, reset_circuit_breaker,
    };
    use super::super::client::ExtrasSession;

    #[test]
    fn test_spawn_oversized_grid_rejected_immediately() {
        let res = ExtrasSession::spawn("storm", 600, 300);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn test_spawn_zero_dimension_grid_rejected_immediately() {
        let res = ExtrasSession::spawn("storm", 0, 100);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().kind(), std::io::ErrorKind::InvalidInput);

        let res2 = ExtrasSession::spawn("storm", 100, 0);
        assert!(res2.is_err());
        assert_eq!(res2.err().unwrap().kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn test_circuit_breaker_tripping_blocks_availability_and_spawn() {
        reset_circuit_breaker();
        assert!(!is_circuit_breaker_tripped());

        record_circuit_breaker_failure();
        assert!(!is_circuit_breaker_tripped());

        record_circuit_breaker_failure();
        assert!(!is_circuit_breaker_tripped());

        record_circuit_breaker_failure();
        assert!(is_circuit_breaker_tripped());
        assert!(!ExtrasSession::is_available());

        let res = ExtrasSession::spawn("storm", 80, 24);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().kind(), std::io::ErrorKind::BrokenPipe);

        reset_circuit_breaker();
        assert!(!is_circuit_breaker_tripped());
    }
}
