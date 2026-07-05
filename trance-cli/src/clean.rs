// SPDX-License-Identifier: MIT

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use trance_dbus::daemon_available;

#[tracing::instrument]
pub fn handle_clean() -> Result<()> {
    println!("Cleaning Trance workspace files...");

    // 1. PID File Cleanup
    let pid_path = if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("trance-daemon.pid")
    } else {
        std::env::temp_dir().join("trance-daemon.pid")
    };

    if pid_path.exists() {
        if daemon_available() {
            println!(" [!] Daemon is currently running. Sticking to active PID file.");
        } else {
            fs::remove_file(&pid_path).with_context(|| {
                format!("failed to delete stale PID file: {}", pid_path.display())
            })?;
            println!(" [✔] Removed stale PID file: '{}'", pid_path.display());
        }
    } else {
        println!(" [✔] No stale PID files found.");
    }

    // 2. User Cache Directory Cleanup
    if let Ok(home) = std::env::var("HOME") {
        let cache_dir = PathBuf::from(home).join(".cache").join("trance");
        if cache_dir.exists() {
            match fs::remove_dir_all(&cache_dir) {
                Ok(()) => println!(" [✔] Cleared cache directory: '{}'", cache_dir.display()),
                Err(e) => println!(" [!] Warning: Failed to clear cache directory: {e}"),
            }
        } else {
            println!(" [✔] Cache directory is already clean.");
        }
    }

    println!("Cleanup completed successfully.");
    Ok(())
}
