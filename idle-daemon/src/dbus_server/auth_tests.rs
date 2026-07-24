// SPDX-License-Identifier: Apache-2.0

use super::auth_peer::{
    PeerExeCheck, TRUSTED_CONTROL_PEERS, check_peer_exe, comm_matches_trusted, peer_comm,
    peer_exe_basename,
};
use super::*;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn clear_trust_all_env() {
    // Other tests may set these; deny-policy tests must start clean.
    unsafe {
        std::env::remove_var("IDLE_DBUS_TRUST_ALL");
        std::env::remove_var("TRANCE_DBUS_TRUST_ALL");
    }
}

#[test]
fn trusted_peer_names_are_fixed() {
    assert!(TRUSTED_CONTROL_PEERS.contains(&"idle"));
    assert!(TRUSTED_CONTROL_PEERS.contains(&"trance"));
    assert!(TRUSTED_CONTROL_PEERS.contains(&"trance-applet"));
    assert!(TRUSTED_CONTROL_PEERS.contains(&"trance-tui"));
    assert!(TRUSTED_CONTROL_PEERS.contains(&"idle-cli"));
    assert!(TRUSTED_CONTROL_PEERS.contains(&"idle-tui"));
    assert!(TRUSTED_CONTROL_PEERS.contains(&"idle-applet"));
    assert!(TRUSTED_CONTROL_PEERS.contains(&"idlescreen"));
    assert!(!TRUSTED_CONTROL_PEERS.contains(&"bash"));
    assert!(!TRUSTED_CONTROL_PEERS.contains(&"python3"));
    assert!(!TRUSTED_CONTROL_PEERS.contains(&"idle-daemon"));
}

#[test]
fn trusted_peer_names_fit_linux_comm() {
    // Kernel task comm is 15 visible chars; list must stay matchable without truncation bugs.
    for name in TRUSTED_CONTROL_PEERS {
        assert!(
            name.len() <= 15,
            "{name:?} exceeds COMM_MAX; update comm_matches_trusted"
        );
        assert!(comm_matches_trusted(name));
    }
}

#[test]
fn current_process_is_readable() {
    let pid = std::process::id();
    assert!(peer_exe_basename(pid).is_some());
}

#[test]
fn current_process_exe_check_is_trusted_or_untrusted() {
    let pid = std::process::id();
    match check_peer_exe(pid) {
        PeerExeCheck::Trusted | PeerExeCheck::Untrusted | PeerExeCheck::Unreadable => {}
    }
}

#[test]
fn same_uid_alone_insufficient_when_exe_and_comm_unreadable() {
    #[cfg(unix)]
    {
        let uid = unsafe { libc::geteuid() };
        // Nonexistent PID → Unreadable exe and unreadable comm.
        // Pure same-UID fallback must NOT accept (would allow any same-user client).
        assert!(!is_trusted_control_peer(u32::MAX, Some(uid), ":1.42"));
        assert!(!is_trusted_control_peer(
            u32::MAX.saturating_sub(1),
            Some(uid),
            ":1.43"
        ));
        assert!(!is_trusted_control_peer(
            u32::MAX,
            Some(uid.wrapping_add(1)),
            ":1.42"
        ));
    }
}

#[test]
fn missing_peer_uid_is_denied() {
    let _guard = env_lock();
    clear_trust_all_env();
    // Policy: deny when Unix UID credential is unavailable (cross-user / incomplete).
    assert!(!is_trusted_control_peer(std::process::id(), None, ":1.1"));
    assert!(!is_trusted_control_peer(u32::MAX, None, ":1.missing"));
}

#[test]
fn missing_uid_denied_even_for_self_pid() {
    let _guard = env_lock();
    clear_trust_all_env();
    // Same process, missing UID still deny — UID is mandatory, not optional.
    let pid = std::process::id();
    assert!(!is_trusted_control_peer(pid, None, ":1.self"));
}

#[test]
fn comm_matches_trusted_exact_and_truncated() {
    assert!(comm_matches_trusted("idle"));
    assert!(comm_matches_trusted("idle-cli"));
    assert!(comm_matches_trusted("trance-applet"));
    assert!(comm_matches_trusted("  idle  "));
    assert!(!comm_matches_trusted("bash"));
    assert!(!comm_matches_trusted("python3"));
    assert!(!comm_matches_trusted(""));
    assert!(!comm_matches_trusted("   "));
    // All allowlisted names are ≤15 chars; longer names would use prefix match.
    assert!(!comm_matches_trusted("idle-cli-extra-long-name"));
    assert!(!comm_matches_trusted("idlescreen-extra"));
}

#[test]
fn peer_comm_of_self_is_readable() {
    let pid = std::process::id();
    let comm = peer_comm(pid);
    assert!(comm.is_some(), "expected /proc/self/comm to be readable");
    assert!(!comm.unwrap_or_default().is_empty());
}

#[test]
fn peer_comm_of_missing_pid_is_none() {
    assert!(peer_comm(u32::MAX).is_none());
    assert!(peer_exe_basename(u32::MAX).is_none());
    assert!(matches!(check_peer_exe(u32::MAX), PeerExeCheck::Unreadable));
}

#[test]
fn wrong_uid_denied_even_if_process_exists() {
    #[cfg(unix)]
    {
        let pid = std::process::id();
        let our = unsafe { libc::geteuid() };
        // Synthetic other uid — must deny before path checks matter.
        assert!(!is_trusted_control_peer(
            pid,
            Some(our.wrapping_add(12345).max(1)),
            ":1.99"
        ));
    }
}

#[test]
fn dbus_trust_all_env_only_in_debug_builds() {
    let _guard = env_lock();
    clear_trust_all_env();
    // In release, env escape hatch is hard-disabled; in debug it may open.
    let prior_idle = std::env::var("IDLE_DBUS_TRUST_ALL").ok();
    let prior_trance = std::env::var("TRANCE_DBUS_TRUST_ALL").ok();
    unsafe {
        std::env::set_var("IDLE_DBUS_TRUST_ALL", "1");
        std::env::remove_var("TRANCE_DBUS_TRUST_ALL");
    }
    let accepted = is_trusted_control_peer(u32::MAX, None, ":1.trust");
    if cfg!(debug_assertions) {
        assert!(accepted, "debug build should honor IDLE_DBUS_TRUST_ALL=1");
    } else {
        assert!(!accepted, "release must ignore *_DBUS_TRUST_ALL");
    }
    match prior_idle {
        Some(v) => unsafe {
            std::env::set_var("IDLE_DBUS_TRUST_ALL", v);
        },
        None => unsafe {
            std::env::remove_var("IDLE_DBUS_TRUST_ALL");
        },
    }
    match prior_trance {
        Some(v) => unsafe {
            std::env::set_var("TRANCE_DBUS_TRUST_ALL", v);
        },
        None => unsafe {
            std::env::remove_var("TRANCE_DBUS_TRUST_ALL");
        },
    }
}

#[test]
fn untrusted_basename_never_matches_comm_policy() {
    for bad in ["sh", "curl", "systemd", "idle-daemon", "idle_daemon"] {
        assert!(!comm_matches_trusted(bad), "{bad} must not be trusted");
    }
}
