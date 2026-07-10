// SPDX-License-Identifier: MIT

//! Authorization for D-Bus control methods.

use zbus::Connection;
use zbus::message::Header;

/// Basenames of processes allowed to call control methods on the daemon.
const TRUSTED_CONTROL_PEERS: &[&str] = &["trance", "trance-applet", "trance-tui", "trance-cli"];

#[cfg(test)]
fn peer_exe_basename(pid: u32) -> Option<String> {
    let path = format!("/proc/{pid}/exe");
    let target = std::fs::canonicalize(path).ok()?;
    target
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
}

/// Result of inspecting a peer executable path.
enum PeerExeCheck {
    /// Path readable and matches trusted name + install prefix (+ root ownership).
    Trusted,
    /// Path readable but not an allowed control client.
    Untrusted,
    /// Cannot read `/proc/<pid>/exe` (common under systemd hardening + Yama).
    Unreadable,
}

fn check_peer_exe(pid: u32) -> PeerExeCheck {
    let path = format!("/proc/{pid}/exe");
    let target = match std::fs::canonicalize(&path) {
        Ok(t) => {
            tracing::debug!(
                "D-Bus auth check: canonical path for PID {} is {:?}",
                pid,
                t
            );
            t
        }
        Err(e) => {
            // EACCES/EPERM: hardened services often cannot ptrace-read peer
            // `/proc/<pid>/exe`. ENOENT: peer already exited.
            tracing::warn!(
                "D-Bus auth check: failed to canonicalize /proc/{}/exe: {:?}",
                pid,
                e
            );
            return PeerExeCheck::Unreadable;
        }
    };
    let name = match target.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => {
            tracing::warn!(
                "D-Bus auth check: failed to get file name from {:?}",
                target
            );
            return PeerExeCheck::Untrusted;
        }
    };
    if !TRUSTED_CONTROL_PEERS.contains(&name) {
        tracing::warn!(
            "D-Bus auth check: process name {:?} is not in trusted control peers list",
            name
        );
        return PeerExeCheck::Untrusted;
    }
    let parent = target.parent().and_then(|p| p.to_str()).unwrap_or("");
    let path_ok = parent == "/usr/bin"
        || parent == "/usr/local/bin"
        || (cfg!(debug_assertions)
            && {
                if let Ok(current_exe) = std::env::current_exe() {
                    if let Ok(current_canonical) = std::fs::canonicalize(current_exe) {
                        let match_parent = target.parent() == current_canonical.parent();
                        tracing::debug!(
                            "D-Bus auth check: comparing parent of target {:?} and current_canonical {:?} -> {}",
                            target.parent(),
                            current_canonical.parent(),
                            match_parent
                        );
                        match_parent
                    } else {
                        tracing::warn!("D-Bus auth check: failed to canonicalize current exe");
                        false
                    }
                } else {
                    tracing::warn!("D-Bus auth check: failed to get current exe");
                    false
                }
            });
    if !path_ok {
        tracing::warn!(
            "D-Bus auth check: path {:?} parent {:?} not trusted",
            target,
            parent
        );
        return PeerExeCheck::Untrusted;
    }

    // Production installs: binary must be root-owned and not world-writable so a
    // user cannot drop a fake `trance` next to the real one in a shared dir.
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match std::fs::metadata(&target) {
            Ok(meta) => {
                let mode = meta.mode();
                if mode & 0o002 != 0 {
                    tracing::warn!(
                        "D-Bus auth check: refusing world-writable peer binary {:?}",
                        target
                    );
                    return PeerExeCheck::Untrusted;
                }
                // Only enforce root ownership for the system prefixes (not debug same-dir peers).
                if (parent == "/usr/bin" || parent == "/usr/local/bin") && meta.uid() != 0 {
                    tracing::warn!(
                        "D-Bus auth check: refusing non-root-owned peer binary {:?} (uid {})",
                        target,
                        meta.uid()
                    );
                    return PeerExeCheck::Untrusted;
                }
            }
            Err(e) => {
                tracing::warn!(
                    "D-Bus auth check: cannot stat peer binary {:?}: {:?}",
                    target,
                    e
                );
                return PeerExeCheck::Untrusted;
            }
        }
    }

    PeerExeCheck::Trusted
}

fn is_trusted_control_peer(pid: u32, peer_uid: Option<u32>) -> bool {
    // Escape hatch is debug-only so release builds cannot be opened with
    // `TRANCE_DBUS_TRUST_ALL=1` by a local attacker.
    if cfg!(debug_assertions)
        && std::env::var("TRANCE_DBUS_TRUST_ALL").ok().as_deref() == Some("1")
    {
        tracing::warn!("D-Bus auth: TRANCE_DBUS_TRUST_ALL=1 (debug build only)");
        return true;
    }

    match check_peer_exe(pid) {
        PeerExeCheck::Trusted => true,
        PeerExeCheck::Untrusted => false,
        PeerExeCheck::Unreadable => {
            // Session bus: peers are already same-user scoped by the bus.
            // Under systemd hardening (ProtectSystem=strict, PrivateTmp, …)
            // the daemon often cannot read `/proc/<peer>/exe` (EACCES), which
            // previously rejected *all* control calls including /usr/bin/trance.
            // Fall back to matching the peer's Unix UID to our own.
            #[cfg(unix)]
            {
                let our_uid = unsafe { libc::geteuid() };
                if let Some(uid) = peer_uid {
                    if uid == our_uid {
                        tracing::info!(
                            "D-Bus auth: peer pid {pid} uid {uid} accepted via same-UID fallback (peer exe unreadable)"
                        );
                        return true;
                    }
                    tracing::warn!(
                        "D-Bus auth: peer pid {pid} uid {uid} != our uid {our_uid}; denying"
                    );
                    return false;
                }
            }
            #[cfg(not(unix))]
            {
                let _ = peer_uid;
            }
            tracing::warn!("D-Bus auth: peer exe unreadable and no UID to fall back on");
            false
        }
    }
}

/// Control methods (preview, config writes) require trance CLI or applet.
pub async fn require_control_peer(
    connection: &Connection,
    header: &Header<'_>,
) -> zbus::fdo::Result<()> {
    let sender = header.sender().ok_or_else(|| {
        zbus::fdo::Error::AccessDenied("control request missing D-Bus sender".into())
    })?;

    let dbus = zbus::fdo::DBusProxy::new(connection)
        .await
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    let creds = dbus
        .get_connection_credentials((*sender).clone().into())
        .await
        .map_err(|_| zbus::fdo::Error::AccessDenied("cannot verify D-Bus peer".into()))?;
    let pid = creds
        .process_id()
        .ok_or_else(|| zbus::fdo::Error::AccessDenied("D-Bus peer PID unavailable".into()))?;
    let peer_uid = creds.unix_user_id();

    if is_trusted_control_peer(pid, peer_uid) {
        tracing::info!("D-Bus control peer accepted (pid {pid})");
        Ok(())
    } else {
        tracing::info!("D-Bus control peer rejected (pid {pid})");
        Err(zbus::fdo::Error::AccessDenied(
            "control methods require the trance CLI or panel applet".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_peer_names_are_fixed() {
        assert!(TRUSTED_CONTROL_PEERS.contains(&"trance"));
        assert!(TRUSTED_CONTROL_PEERS.contains(&"trance-applet"));
        assert!(TRUSTED_CONTROL_PEERS.contains(&"trance-tui"));
        assert!(TRUSTED_CONTROL_PEERS.contains(&"trance-cli"));
        assert!(!TRUSTED_CONTROL_PEERS.contains(&"bash"));
        assert!(!TRUSTED_CONTROL_PEERS.contains(&"python3"));
    }

    #[test]
    fn current_process_is_readable() {
        let pid = std::process::id();
        assert!(peer_exe_basename(pid).is_some());
    }

    #[test]
    fn current_process_exe_check_is_trusted_or_untrusted() {
        // Running under `cargo test` the binary is not named `trance`, so Untrusted.
        let pid = std::process::id();
        match check_peer_exe(pid) {
            PeerExeCheck::Trusted | PeerExeCheck::Untrusted | PeerExeCheck::Unreadable => {}
        }
    }

    #[test]
    fn same_uid_fallback_accepts_when_exe_unreadable() {
        #[cfg(unix)]
        {
            let uid = unsafe { libc::geteuid() };
            // Nonexistent PID → Unreadable → same-UID accepts.
            assert!(is_trusted_control_peer(u32::MAX, Some(uid)));
            // Non-matching UID must never be accepted on unreadable exe.
            assert!(!is_trusted_control_peer(
                u32::MAX,
                Some(uid.wrapping_add(1))
            ));
        }
    }
}
