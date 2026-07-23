// SPDX-License-Identifier: MIT

use std::io::Write;
use std::path::{Path, PathBuf};

mod pam;

pub use pam::{authenticate, read_password};

pub fn run_failsafe_lock() -> anyhow::Result<()> {
    let username = unsafe {
        let uid = libc::getuid();
        let pwd = libc::getpwuid(uid);
        if !pwd.is_null() && !(*pwd).pw_name.is_null() {
            std::ffi::CStr::from_ptr((*pwd).pw_name)
                .to_string_lossy()
                .into_owned()
        } else {
            std::env::var("USER").unwrap_or_else(|_| "user".to_string())
        }
    };
    println!("\x1b[2J\x1b[H");
    println!("============================================================");
    println!("trance: SCREENSAVER RUNNER CRASHED / EXITED UNEXPECTEDLY!");
    println!("SESSION IS LOCKED FOR SECURITY.");
    println!("============================================================");
    println!();

    loop {
        print!("Password for {}: ", username);
        std::io::stdout().flush()?;

        let password = read_password().unwrap_or_default();
        if authenticate(&username, &password) {
            println!("Authentication successful. Session unlocked.");
            break;
        } else {
            println!("Authentication failed. Please try again.");
            println!();
        }
    }
    Ok(())
}

/// Resolve a terminal basename only under fixed system prefixes.
/// Never consults `$PATH` (PATH hijack would unlock a fake terminal).
fn resolve_term_bin(name: &str) -> Option<PathBuf> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains('\0') {
        return None;
    }
    for dir in ["/usr/bin", "/usr/local/bin"] {
        let p = Path::new(dir).join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

pub fn spawn_failsafe_locker() -> Result<(), String> {
    let term_emulators = [
        "xterm",
        "foot",
        "gnome-terminal",
        "konsole",
        "kitty",
        "alacritty",
        "wezterm",
        "weston-terminal",
    ];

    let current_exe =
        std::env::current_exe().map_err(|e| format!("failed to get current exe path: {e}"))?;

    let term_bin = term_emulators.into_iter().find_map(resolve_term_bin);

    let Some(term) = term_bin else {
        return Err("No terminal emulator found under /usr/bin or /usr/local/bin".to_string());
    };
    let term_name = term
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("terminal");

    loop {
        tracing::info!("Spawning failsafe locker using {}...", term.display());
        let mut cmd = std::process::Command::new(&term);
        if term_name == "gnome-terminal" || term_name == "konsole" {
            cmd.arg("--").arg(&current_exe).arg("failsafe-lock");
        } else {
            cmd.arg("-e").arg(&current_exe).arg("failsafe-lock");
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn terminal: {e}"))?;
        let status = child
            .wait()
            .map_err(|e| format!("failed to wait for locker: {e}"))?;

        if status.success() {
            tracing::info!("Failsafe locker successfully authenticated user.");
            break;
        } else {
            tracing::warn!("Failsafe locker exited without success. respawning...");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_term_rejects_path_injection() {
        assert!(resolve_term_bin("../bin/xterm").is_none());
        assert!(resolve_term_bin("/usr/bin/xterm").is_none());
        assert!(resolve_term_bin("").is_none());
        assert!(resolve_term_bin("xterm\0evil").is_none());
    }

    #[test]
    fn resolve_term_looks_only_under_system_prefixes() {
        // Presence depends on the host; we only assert that a found path is under
        // the allowlisted prefixes (no PATH-relative resolution).
        if let Some(p) = resolve_term_bin("bash") {
            let s = p.to_string_lossy();
            assert!(
                s.starts_with("/usr/bin/") || s.starts_with("/usr/local/bin/"),
                "unexpected path {s}"
            );
        }
    }
}
