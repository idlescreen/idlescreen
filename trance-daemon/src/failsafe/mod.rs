// SPDX-License-Identifier: MIT

use std::io::Write;

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

fn which_bin(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
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

    let term_bin = term_emulators.into_iter().find(|&term| which_bin(term));

    let Some(term) = term_bin else {
        return Err("No terminal emulator found".to_string());
    };

    loop {
        tracing::info!("Spawning failsafe locker using {}...", term);
        let mut cmd = std::process::Command::new(term);
        if term == "gnome-terminal" || term == "konsole" {
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
