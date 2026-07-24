// SPDX-License-Identifier: Apache-2.0

//! Authorization for D-Bus control methods.

#[path = "auth_peer.rs"]
mod auth_peer;

use auth_peer::{PeerExeCheck, check_peer_exe, comm_matches_trusted, our_euid, peer_comm};

use zbus::Connection;
use zbus::message::Header;

/// Require peer Unix UID to match our euid (session-bus same-user defense in depth).
fn peer_uid_matches_ours(peer_uid: Option<u32>) -> bool {
    let Some(our_uid) = our_euid() else {
        return peer_uid.is_none();
    };
    match peer_uid {
        Some(uid) if uid == our_uid => true,
        Some(uid) => {
            tracing::warn!("D-Bus auth: peer uid {uid} != our uid {our_uid}; denying");
            false
        }
        None => {
            tracing::warn!("D-Bus auth: peer UID unavailable; denying");
            false
        }
    }
}

fn dbus_trust_all_enabled() -> bool {
    if !cfg!(debug_assertions) {
        return false;
    }
    for key in ["IDLE_DBUS_TRUST_ALL", "TRANCE_DBUS_TRUST_ALL"] {
        if std::env::var(key).ok().as_deref() == Some("1") {
            return true;
        }
    }
    false
}

fn is_trusted_control_peer(pid: u32, peer_uid: Option<u32>, peer_name: &str) -> bool {
    // Escape hatch is debug-only so release builds cannot be opened with
    // `TRANCE_DBUS_TRUST_ALL=1` / `IDLE_DBUS_TRUST_ALL=1` by a local attacker.
    if dbus_trust_all_enabled() {
        tracing::warn!("D-Bus auth: *_DBUS_TRUST_ALL=1 (debug build only)");
        return true;
    }

    // Always require same-UID before trusting path or comm (closes cross-user
    // edge cases if the service is ever bound on a broader bus).
    if !peer_uid_matches_ours(peer_uid) {
        return false;
    }

    match check_peer_exe(pid) {
        PeerExeCheck::Trusted => {
            // Narrow TOCTOU window: re-check exe still resolves to a trusted peer.
            match check_peer_exe(pid) {
                PeerExeCheck::Trusted => true,
                other => {
                    tracing::warn!(
                        "D-Bus auth: peer {peer_name} (pid {pid}) failed re-check after Trusted ({other:?})"
                    );
                    false
                }
            }
        }
        PeerExeCheck::Untrusted => false,
        PeerExeCheck::Unreadable => {
            // Do **not** accept pure same-UID: any compromised same-user process
            // could otherwise call control methods. Prefer `/proc/pid/comm`, which
            // remains readable under typical Yama/systemd hardening when `exe` is not.
            match peer_comm(pid) {
                Some(comm) if comm_matches_trusted(&comm) => {
                    tracing::warn!(
                        "D-Bus auth: peer {peer_name} (pid {pid}, comm {comm}) accepted via same-UID + trusted comm (peer exe unreadable)"
                    );
                    true
                }
                Some(comm) => {
                    tracing::warn!(
                        "D-Bus auth: peer pid {pid} comm {comm:?} not trusted; denying (exe unreadable)"
                    );
                    false
                }
                None => {
                    tracing::warn!("D-Bus auth: peer pid {pid} exe and comm unreadable; denying");
                    false
                }
            }
        }
    }
}

/// Control methods (preview, config writes) require idle/trance CLI or applet.
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

    if is_trusted_control_peer(pid, peer_uid, sender.as_str()) {
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
#[path = "auth_tests.rs"]
mod tests;
