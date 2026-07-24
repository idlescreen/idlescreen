// SPDX-License-Identifier: Apache-2.0

//! Peer executable / comm inspection for D-Bus control auth.

/// Basenames of processes allowed to call control methods on the daemon.
pub(super) const TRUSTED_CONTROL_PEERS: &[&str] = &[
    "idle",
    "idle-cli",
    "idle-tui",
    "idle-applet",
    "trance",
    "trance-applet",
    "trance-tui",
    "idlescreen",
];

/// Linux `TASK_COMM_LEN` is 16 bytes including NUL → 15 visible chars in `/proc/pid/comm`.
const COMM_MAX: usize = 15;

/// Result of inspecting a peer executable path.
#[derive(Debug)]
pub(super) enum PeerExeCheck {
    /// Path readable and matches trusted name + install prefix (+ root ownership).
    Trusted,
    /// Path readable but not an allowed control client.
    Untrusted,
    /// Cannot read `/proc/<pid>/exe` (common under systemd hardening + Yama).
    Unreadable,
}

#[cfg(test)]
pub(super) fn peer_exe_basename(pid: u32) -> Option<String> {
    let path = format!("/proc/{pid}/exe");
    let target = std::fs::canonicalize(path).ok()?;
    target
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
}

pub(super) fn check_peer_exe(pid: u32) -> PeerExeCheck {
    let path = format!("/proc/{pid}/exe");
    let target = match std::fs::canonicalize(&path) {
        Ok(t) => t,
        Err(e) => {
            // EACCES/EPERM: hardened services often cannot ptrace-read peer
            // `/proc/<pid>/exe`. ENOENT: peer already exited.
            tracing::warn!("D-Bus auth check: failed to canonicalize /proc/{pid}/exe: {e:?}");
            return PeerExeCheck::Unreadable;
        }
    };
    let name = match target.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => {
            tracing::warn!("D-Bus auth check: failed to get file name from {target:?}");
            return PeerExeCheck::Untrusted;
        }
    };
    if !TRUSTED_CONTROL_PEERS.contains(&name) {
        tracing::warn!(
            "D-Bus auth check: process name {name:?} is not in trusted control peers list"
        );
        return PeerExeCheck::Untrusted;
    }
    let parent = target.parent().and_then(|p| p.to_str()).unwrap_or("");
    let path_ok = parent == "/usr/bin"
        || parent == "/usr/local/bin"
        || (cfg!(debug_assertions) && same_dir_as_current_exe(&target));
    if !path_ok {
        tracing::warn!("D-Bus auth check: path {target:?} parent {parent:?} not trusted");
        return PeerExeCheck::Untrusted;
    }

    // Production: root-owned, not world-writable for system prefixes.
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match std::fs::metadata(&target) {
            Ok(meta) => {
                if meta.mode() & 0o002 != 0 {
                    tracing::warn!(
                        "D-Bus auth check: refusing world-writable peer binary {target:?}"
                    );
                    return PeerExeCheck::Untrusted;
                }
                if (parent == "/usr/bin" || parent == "/usr/local/bin")
                    && meta.uid() != 0
                    && meta.uid() != 65534
                {
                    tracing::warn!(
                        "D-Bus auth check: refusing non-root-owned peer binary {target:?} (uid {})",
                        meta.uid()
                    );
                    return PeerExeCheck::Untrusted;
                }
            }
            Err(e) => {
                tracing::warn!("D-Bus auth check: cannot stat peer binary {target:?}: {e:?}");
                return PeerExeCheck::Untrusted;
            }
        }
    }

    PeerExeCheck::Trusted
}

fn same_dir_as_current_exe(target: &std::path::Path) -> bool {
    let Ok(current_exe) = std::env::current_exe() else {
        return false;
    };
    let Ok(current_canonical) = std::fs::canonicalize(current_exe) else {
        return false;
    };
    target.parent() == current_canonical.parent()
}

/// Read `/proc/<pid>/comm` (usually world-readable even when `exe` is not).
pub(super) fn peer_comm(pid: u32) -> Option<String> {
    let raw = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    let name = raw.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Match a `/proc/pid/comm` value against trusted peer basenames (15-char truncation).
pub(super) fn comm_matches_trusted(comm: &str) -> bool {
    let c = comm.trim();
    TRUSTED_CONTROL_PEERS.iter().any(|name| {
        if *name == c {
            return true;
        }
        // Kernel truncates task comm to COMM_MAX chars.
        name.len() > COMM_MAX && name.as_bytes().get(..COMM_MAX) == Some(c.as_bytes())
    })
}

pub(super) fn our_euid() -> Option<u32> {
    #[cfg(unix)]
    {
        Some(unsafe { libc::geteuid() })
    }
    #[cfg(not(unix))]
    {
        None
    }
}
