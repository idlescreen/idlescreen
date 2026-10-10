// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! System information and multi-probe OS/DE/Kernel detection.

use std::fs;
use std::path::Path;

/// Multi-probe detection for Host OS name.
///
/// Hierarchy:
/// 1. `$IDLE_OS_NAME` environment override
/// 2. `/etc/os-release` and `/usr/lib/os-release` (`PRETTY_NAME`, fallback to `NAME`)
/// 3. `/etc/lsb-release` (`DISTRIB_DESCRIPTION`, fallback to `DISTRIB_ID`)
/// 4. `/etc/debian_version`, `/etc/redhat-release`, `/etc/alpine-release`
/// 5. `/etc/issue`
/// 6. `libc::uname(2)`
/// 7. Default fallback `"IdleScreen"`
pub fn detect_os_name() -> String {
    // 1. Environment override
    if let Ok(v) = std::env::var("IDLE_OS_NAME") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return clean_os_name(trimmed);
        }
    }

    // 2. /etc/os-release or /usr/lib/os-release
    for path in &["/etc/os-release", "/usr/lib/os-release"] {
        if let Some(name) = parse_os_release(path) {
            return clean_os_name(&name);
        }
    }

    // 3. /etc/lsb-release
    if let Some(name) = parse_lsb_release("/etc/lsb-release") {
        return clean_os_name(&name);
    }

    // 4. /etc/debian_version, /etc/redhat-release, /etc/alpine-release
    if let Ok(content) = fs::read_to_string("/etc/redhat-release") {
        let first = content.lines().next().unwrap_or("").trim();
        if !first.is_empty() {
            return clean_os_name(first);
        }
    }
    if let Ok(content) = fs::read_to_string("/etc/alpine-release") {
        let first = content.lines().next().unwrap_or("").trim();
        if !first.is_empty() {
            return format!("Alpine Linux {first}");
        }
    }
    if let Ok(content) = fs::read_to_string("/etc/debian_version") {
        let first = content.lines().next().unwrap_or("").trim();
        if !first.is_empty() {
            return format!("Debian {first}");
        }
    }

    // 5. /etc/issue
    if let Ok(content) = fs::read_to_string("/etc/issue") {
        let first = content.lines().next().unwrap_or("").trim();
        let cleaned = first
            .replace("\\n", "")
            .replace("\\l", "")
            .replace("\\r", "")
            .replace("\\m", "")
            .trim()
            .to_string();
        if !cleaned.is_empty() {
            return clean_os_name(&cleaned);
        }
    }

    // 6. uname(2) sysname
    if let Some(sysname) = uname_sysname()
        && !sysname.is_empty() {
            return clean_os_name(&sysname);
        }

    // 7. Default fallback
    "IdleScreen".to_string()
}

/// Parses `PRETTY_NAME` or `NAME` from an os-release file.
fn parse_os_release(path: impl AsRef<Path>) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut pretty_name = None;

    for line in content.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
            pretty_name = Some(val.trim_matches('"').trim_matches('\'').to_string());
        } else if let Some(val) = line.strip_prefix("NAME=") {
            name = Some(val.trim_matches('"').trim_matches('\'').to_string());
        }
    }

    pretty_name.or(name).filter(|s| !s.trim().is_empty())
}

/// Parses `DISTRIB_DESCRIPTION` or `DISTRIB_ID` from lsb-release.
fn parse_lsb_release(path: impl AsRef<Path>) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let mut desc = None;
    let mut id = None;

    for line in content.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("DISTRIB_DESCRIPTION=") {
            desc = Some(val.trim_matches('"').trim_matches('\'').to_string());
        } else if let Some(val) = line.strip_prefix("DISTRIB_ID=") {
            id = Some(val.trim_matches('"').trim_matches('\'').to_string());
        }
    }

    desc.or(id).filter(|s| !s.trim().is_empty())
}

/// Calls `libc::uname` to get system name.
fn uname_sysname() -> Option<String> {
    let mut uts: libc::utsname = unsafe { std::mem::zeroed() };
    if unsafe { libc::uname(&raw mut uts) } == 0 {
        let c_str = unsafe { std::ffi::CStr::from_ptr(uts.sysname.as_ptr()) };
        c_str.to_str().ok().map(std::string::ToString::to_string)
    } else {
        None
    }
}

/// Calls `libc::uname` to get kernel release.
fn uname_release() -> Option<String> {
    let mut uts: libc::utsname = unsafe { std::mem::zeroed() };
    if unsafe { libc::uname(&raw mut uts) } == 0 {
        let c_str = unsafe { std::ffi::CStr::from_ptr(uts.release.as_ptr()) };
        c_str.to_str().ok().map(std::string::ToString::to_string)
    } else {
        None
    }
}

/// Cleans raw OS name by stripping edition markers (LTS, Server Edition, Workstation, GNU/Linux).
pub fn clean_os_name(raw: &str) -> String {
    let mut s = raw.split(" (").next().unwrap_or(raw).trim();
    // Strip trailing parenthesis content if any remained
    if let Some(idx) = s.find('(') {
        s = s[..idx].trim();
    }

    // Strip edition markers
    let markers = [
        " Server Edition",
        " Workstation Edition",
        " Desktop Edition",
        " Enterprise Linux",
        " GNU/Linux",
        " LTS",
    ];

    let mut cleaned = s.to_string();
    for marker in &markers {
        if let Some(pos) = cleaned.to_ascii_lowercase().find(&marker.to_ascii_lowercase()) {
            cleaned = format!("{}{}", &cleaned[..pos], &cleaned[pos + marker.len()..]);
        }
    }

    // If string ends with version number like "Fedora 40" or "Ubuntu 24.04", keep distro or clean
    let trimmed = cleaned.trim();
    if let Some(pos) = trimmed.rfind(' ') {
        let suffix = &trimmed[pos + 1..];
        if suffix.chars().all(|c| c.is_ascii_digit() || c == '.') && !trimmed[..pos].trim().is_empty() {
            return trimmed[..pos].trim().to_string();
        }
    }

    if trimmed.is_empty() {
        "IdleScreen".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Detects Desktop Environment name.
pub fn detect_desktop_env() -> String {
    for key in &[
        "XDG_CURRENT_DESKTOP",
        "DESKTOP_SESSION",
        "GDMSESSION",
        "XDG_SESSION_DESKTOP",
    ] {
        if let Ok(v) = std::env::var(key) {
            let t = v.trim();
            if !t.is_empty() {
                // Often XDG_CURRENT_DESKTOP is e.g. "ubuntu:GNOME" or "pop:GNOME"
                let clean = t.split(':').next_back().unwrap_or(t).trim();
                if !clean.is_empty() {
                    return match clean.to_ascii_uppercase().as_str() {
                        "GNOME" => "GNOME",
                        "KDE" | "KDE PLASMA" | "PLASMA" => "KDE Plasma",
                        "COSMIC" => "COSMIC",
                        "X-CINNAMON" | "CINNAMON" => "Cinnamon",
                        "MATE" => "MATE",
                        "XFCE" => "XFCE",
                        "SWAY" => "Sway",
                        "HYPRLAND" => "Hyprland",
                        "WAYFIRE" => "Wayfire",
                        "I3" => "i3",
                        other => return other.to_string(),
                    }
                    .to_string();
                }
            }
        }
    }
    "Desktop".to_string()
}

/// Detects Linux Kernel version string (e.g. "Linux 6.13.5").
pub fn detect_kernel_version() -> String {
    if let Ok(v) = std::env::var("IDLE_KERNEL_VERSION") {
        let t = v.trim();
        if !t.is_empty() {
            return clean_kernel_version(t);
        }
    }

    if let Ok(content) = fs::read_to_string("/proc/sys/kernel/osrelease") {
        let t = content.trim();
        if !t.is_empty() {
            return clean_kernel_version(t);
        }
    }

    if let Some(release) = uname_release() {
        return clean_kernel_version(&release);
    }

    "Linux".to_string()
}

/// Cleans kernel version into format "Linux X.Y.Z".
pub fn clean_kernel_version(raw: &str) -> String {
    let trimmed = raw.trim();
    let stripped = trimmed
        .strip_prefix("Linux ")
        .or_else(|| trimmed.strip_prefix("linux "))
        .unwrap_or(trimmed);
    let ver = stripped.split('-').next().unwrap_or(stripped).trim();
    if ver.is_empty() || ver == "unknown" {
        "Linux".to_string()
    } else {
        format!("Linux {ver}")
    }
}

/// System telemetry for portrait telemetry display.
#[derive(Clone, Debug, Default)]
pub struct SystemTelemetry {
    pub cpu_percent: f32,
    pub ram_percent: f32,
    pub uptime_secs: u64,
    pub kernel: String,
}

pub fn query_telemetry() -> SystemTelemetry {
    let kernel = detect_kernel_version();
    let uptime_secs = read_uptime().unwrap_or(0);
    let ram_percent = read_ram_percent().unwrap_or(0.0);
    let cpu_percent = read_cpu_percent().unwrap_or(0.0);

    SystemTelemetry {
        cpu_percent,
        ram_percent,
        uptime_secs,
        kernel,
    }
}

fn read_uptime() -> Option<u64> {
    let content = fs::read_to_string("/proc/uptime").ok()?;
    let first = content.split_whitespace().next()?;
    let secs_f: f64 = first.parse().ok()?;
    Some(secs_f as u64)
}

fn read_ram_percent() -> Option<f32> {
    let content = fs::read_to_string("/proc/meminfo").ok()?;
    let mut total: Option<u64> = None;
    let mut avail: Option<u64> = None;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            total = line.split_whitespace().nth(1).and_then(|v| v.parse().ok());
        } else if line.starts_with("MemAvailable:") {
            avail = line.split_whitespace().nth(1).and_then(|v| v.parse().ok());
        }
        if total.is_some() && avail.is_some() {
            break;
        }
    }

    match (total, avail) {
        (Some(t), Some(a)) if t > 0 => {
            let used = t.saturating_sub(a);
            Some((used as f32 / t as f32) * 100.0)
        }
        _ => None,
    }
}

fn read_cpu_percent() -> Option<f32> {
    let content = fs::read_to_string("/proc/stat").ok()?;
    let first = content.lines().next()?;
    if !first.starts_with("cpu ") {
        return None;
    }
    let parts: Vec<u64> = first
        .split_whitespace()
        .skip(1)
        .filter_map(|p| p.parse().ok())
        .collect();
    if parts.len() < 4 {
        return None;
    }
    let idle = parts[3];
    let total: u64 = parts.iter().sum();
    if total == 0 {
        return None;
    }
    let busy = total.saturating_sub(idle);
    Some((busy as f32 / total as f32) * 100.0)
}
