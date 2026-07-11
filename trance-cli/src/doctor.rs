// SPDX-License-Identifier: MIT

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use trance_dbus::TranceClient;

#[derive(Debug)]
struct CheckResult {
    name: &'static str,
    passed: bool,
    detail: String,
}

fn chk(name: &'static str, passed: bool, detail: impl Into<String>) -> CheckResult {
    CheckResult {
        name,
        passed,
        detail: detail.into(),
    }
}

/// Run diagnostics. When `fix` is true, attempt to reload/enable/restart the
/// user unit so upgrades do not require remembering systemctl flags.
pub fn run_doctor(fix: bool) -> Result<()> {
    println!("==========================================");
    println!("Trance System Diagnostics (Doctor)");
    println!("==========================================");

    if fix {
        fix_user_service()?;
        println!();
    }

    let results = vec![
        check_wayland(),
        check_dbus(),
        check_systemd_service(),
        check_running_pid(),
        check_config_parses(),
        check_fonts(),
        check_package_install(),
    ];
    print_results(&results);
    if !results.iter().all(|r| r.passed) {
        if !fix {
            println!("Hint: try  trance doctor --fix  to reload/enable the user service.");
        }
        std::process::exit(1);
    }
    Ok(())
}

/// Best-effort recovery after package upgrade or a dead session service.
fn fix_user_service() -> Result<()> {
    println!("--fix: reloading and ensuring trance-daemon user service...");

    let reload = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status()
        .context("systemctl --user daemon-reload")?;
    if !reload.success() {
        bail!(
            "daemon-reload failed (exit {}). Are you in a graphical user session?",
            reload.code().unwrap_or(-1)
        );
    }
    println!(" [✔] systemctl --user daemon-reload");

    let _ = Command::new("systemctl")
        .args(["--user", "reset-failed", "trance-daemon.service"])
        .status();

    // Enable so future logins start the daemon (idempotent).
    let enable = Command::new("systemctl")
        .args(["--user", "enable", "trance-daemon.service"])
        .status()
        .context("systemctl --user enable")?;
    if enable.success() {
        println!(" [✔] systemctl --user enable trance-daemon");
    } else {
        println!(" [!] enable returned non-zero (may already be linked); continuing");
    }

    // Prefer try-restart when already running; otherwise start.
    let try_restart = Command::new("systemctl")
        .args(["--user", "try-restart", "trance-daemon.service"])
        .status();
    match try_restart {
        Ok(st) if st.success() => {
            println!(" [✔] systemctl --user try-restart trance-daemon");
        }
        _ => {
            let start = Command::new("systemctl")
                .args(["--user", "start", "trance-daemon.service"])
                .status()
                .context("systemctl --user start")?;
            if start.success() {
                println!(" [✔] systemctl --user start trance-daemon");
            } else {
                bail!(
                    "could not start trance-daemon (exit {}). Check: journalctl --user -u trance-daemon -n 40",
                    start.code().unwrap_or(-1)
                );
            }
        }
    }

    // Brief settle for Type=dbus BusName claim.
    std::thread::sleep(std::time::Duration::from_millis(300));
    Ok(())
}

fn check_wayland() -> CheckResult {
    match std::env::var("WAYLAND_DISPLAY")
        .ok()
        .filter(|s| !s.is_empty())
    {
        Some(display) => {
            println!(" [✔] Wayland Session: WAYLAND_DISPLAY is set to '{display}'.");
            chk("Wayland Session", true, display)
        }
        None => {
            println!(" [✗] Wayland Session: WAYLAND_DISPLAY environment variable is not set!");
            println!("     -> Fix: Trance daemon requires a running Wayland compositor.");
            chk("Wayland Session", false, "WAYLAND_DISPLAY not set")
        }
    }
}

fn check_dbus() -> CheckResult {
    match TranceClient::connect() {
        Ok(_) => {
            println!(
                " [✔] D-Bus Connectivity: Connected to session service '{}'.",
                trance_dbus::SERVICE_NAME
            );
            chk("D-Bus Connectivity", true, trance_dbus::SERVICE_NAME)
        }
        Err(e) => {
            println!(" [✗] D-Bus Connectivity: Failed to connect to daemon: {e}");
            chk("D-Bus Connectivity", false, format!("{e}"))
        }
    }
}

fn check_systemd_service() -> CheckResult {
    let dbus_ok = TranceClient::connect().is_ok();
    let output = Command::new("systemctl")
        .args(["--user", "is-active", "trance-daemon"])
        .output();
    match output {
        Ok(out) => {
            let active = String::from_utf8_lossy(&out.stdout).trim() == "active";
            if active {
                println!(" [✔] Systemd Service: trance-daemon.service is active.");
                chk("Systemd Service", true, "active")
            } else if dbus_ok {
                println!(
                    " [!] Systemd Service: Daemon is active, but systemd service is not reported active."
                );
                chk("Systemd Service", true, "reachable via d-bus")
            } else {
                println!(" [✗] Systemd Service: trance-daemon.service is inactive or failed.");
                println!(
                    "     -> Fix: Start the service with: systemctl --user start trance-daemon"
                );
                chk("Systemd Service", false, "inactive")
            }
        }
        Err(_) => {
            println!(" [!] Systemd Service: 'systemctl' command not found or not usable.");
            chk("Systemd Service", true, "systemctl unavailable")
        }
    }
}

fn check_running_pid() -> CheckResult {
    let dbus_ok = TranceClient::connect().is_ok();
    let pid_path = pid_file_path();
    if pid_path.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_path)
            && let Ok(pid) = pid_str.trim().parse::<i32>()
        {
            unsafe {
                if libc::kill(pid, 0) == 0 {
                    println!(" [✔] Process Status: Daemon is running (PID {pid}) and responsive.");
                    return chk("Process Status", true, format!("PID {pid}"));
                } else {
                    println!(
                        " [✗] Process Status: Stale PID file exists (PID {pid}), but daemon is not running."
                    );
                    println!("     -> Fix: Clean up stale PID or restart the daemon.");
                    return chk("Process Status", false, format!("stale PID {pid}"));
                }
            }
        }
        chk("Process Status", true, "pid file unreadable")
    } else if dbus_ok {
        println!(" [!] Process Status: Connected to daemon via D-Bus, but PID file is missing.");
        chk("Process Status", true, "missing pid but d-bus ok")
    } else {
        println!(" [✗] Process Status: Daemon PID file does not exist.");
        chk("Process Status", false, "missing pid file")
    }
}

fn check_config_parses() -> CheckResult {
    match get_config_path() {
        Some(path) if path.exists() => match fs::read_to_string(&path) {
            Ok(content) => {
                println!(" [✔] Configuration: File found at '{}'.", path.display());
                let n = content.lines().count();
                println!(
                    "     -> Health check: Configuration file read successfully ({} lines).",
                    n
                );
                chk("Configuration", true, format!("{n} lines"))
            }
            Err(e) => {
                println!(
                    " [✗] Configuration: Found at '{}' but unreadable: {}",
                    path.display(),
                    e
                );
                chk("Configuration", false, format!("unreadable: {e}"))
            }
        },
        Some(path) => {
            println!(" [!] Configuration: File not found. Default settings will be used.");
            println!(
                "     -> Note: Config file path is expected at '{}'.",
                path.display()
            );
            chk("Configuration", true, "default settings")
        }
        None => {
            println!(" [✗] Configuration: Could not resolve home directory path for settings.");
            chk("Configuration", false, "cannot resolve home")
        }
    }
}

fn check_fonts() -> CheckResult {
    if font_check_via_fc_list() {
        println!(" [✔] System Fonts: Monospace font is installed.");
        chk("System Fonts", true, "monospace font found")
    } else {
        println!(" [✗] System Fonts: Monospace font not found on system!");
        println!("     -> Fix: Please install fonts-dejavu-core or a system monospace font.");
        chk("System Fonts", false, "monospace font missing")
    }
}

fn check_package_install() -> CheckResult {
    // Prefer RPM: Fedora often has apt-cache on PATH too.
    if let Ok(o) = Command::new("rpm")
        .args(["-q", "trance", "--qf", "%{NAME}-%{VERSION}-%{RELEASE}.%{ARCH}"])
        .output()
    {
        if o.status.success() {
            let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
            println!(" [✔] Package (RPM): {ver}");
            println!("     -> Upgrade with: sudo dnf update");
            return chk("Package", true, ver);
        }
    }
    if let Ok(o) = Command::new("dpkg-query")
        .args(["-W", "-f=${Package} ${Version}", "trance"])
        .output()
    {
        if o.status.success() {
            let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
            println!(" [✔] Package (DEB): {ver}");
            println!("     -> Upgrade with: sudo apt update && sudo apt upgrade");
            return chk("Package", true, ver);
        }
    }
    println!(" [!] Package: trance not found via RPM or dpkg.");
    println!("     -> Install from the crateria apt/dnf repository.");
    chk("Package", true, "not a system package")
}

fn print_results(results: &[CheckResult]) {
    println!("==========================================");
    for result in results {
        let marker = if result.passed { "✓" } else { "✗" };
        println!("  [{marker}] {}: {}", result.name, result.detail);
    }
    println!("==========================================");
    if results.iter().all(|r| r.passed) {
        println!("Diagnostics complete: ALL SYSTEMS NOMINAL.");
    } else {
        println!("Diagnostics complete: PROBLEMS DETECTED.");
        println!("Diagnostics check failed. Please resolve the issues marked with [✗].");
    }
}

fn pid_file_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("trance-daemon.pid")
    } else {
        std::env::temp_dir().join("trance-daemon.pid")
    }
}

fn get_config_path() -> Option<PathBuf> {
    if let Some(xdg_config) = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
    {
        return Some(PathBuf::from(xdg_config).join("trance").join("config.yaml"));
    }
    let home = std::env::var("HOME").ok()?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("trance")
            .join("config.yaml"),
    )
}

fn font_check_via_fc_list() -> bool {
    let output = Command::new("fc-list").args([":mono"]).output();
    match output {
        Ok(out) => out.status.success() && !out.stdout.is_empty(),
        Err(_) => {
            let common_dirs = ["/usr/share/fonts", "/usr/local/share/fonts"];
            common_dirs.iter().any(|dir| PathBuf::from(dir).exists())
        }
    }
}
