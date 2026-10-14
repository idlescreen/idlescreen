// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Desktop environment detection and delta-based system telemetry.

use std::fs;
use std::sync::Mutex;

/// Detects Desktop Environment name, cleaned to crisp uppercase brand words (e.g. "GNOME", "KDE").
pub fn detect_desktop_env() -> String {
    if let Ok(v) = std::env::var("IDLE_DESKTOP_ENV").or_else(|_| std::env::var("IDLE_DE_NAME")) {
        let t = v.trim();
        if !t.is_empty() {
            return clean_desktop_env(t);
        }
    }

    for key in &[
        "XDG_CURRENT_DESKTOP",
        "DESKTOP_SESSION",
        "GDMSESSION",
        "XDG_SESSION_DESKTOP",
    ] {
        if let Ok(v) = std::env::var(key) {
            let t = v.trim();
            if !t.is_empty() {
                let clean = clean_desktop_env(t);
                if clean != "DESKTOP" && clean != "UNKNOWN" {
                    return clean;
                }
            }
        }
    }
    "DESKTOP".to_string()
}

/// Cleans raw desktop environment string to uppercase single brand word.
pub fn clean_desktop_env(raw: &str) -> String {
    let t = raw.split(':').next_back().unwrap_or(raw).trim();
    let upper = t.to_ascii_uppercase();

    let words: Vec<&str> = upper
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();

    if words.contains(&"GNOME") || words.contains(&"MUTTER") {
        return "GNOME".to_string();
    }
    if words.contains(&"KDE") || words.contains(&"PLASMA") || words.contains(&"KWIN") {
        return "KDE".to_string();
    }
    for &brand in &[
        "COSMIC", "HYPRLAND", "SWAY", "WAYFIRE", "RIVER", "NIRI", "LABWC", "WESTON", "CINNAMON",
        "MATE", "XFCE", "I3", "LXQT", "LXDE", "BUDGIE", "PANTHEON", "DEEPIN", "OPENBOX", "BSPWM",
        "AWESOME", "QTILE", "DWM",
    ] {
        if words.contains(&brand) {
            return brand.to_string();
        }
    }

    if let Some(&first) = words.first() {
        if first.is_empty() || matches!(first, "UNKNOWN" | "DEFAULT" | "NONE" | "STANDARD") {
            "DESKTOP".to_string()
        } else {
            first.to_string()
        }
    } else {
        "DESKTOP".to_string()
    }
}

/// Detects Linux Kernel version string, standardized to single uppercase brand word "LINUX".
pub fn detect_kernel_version() -> String {
    if let Ok(v) = std::env::var("IDLE_KERNEL_VERSION") {
        let t = v.trim();
        if !t.is_empty() {
            return clean_kernel_version(t);
        }
    }
    "LINUX".to_string()
}

/// Cleans kernel version into single uppercase brand word "LINUX".
pub fn clean_kernel_version(_raw: &str) -> String {
    "LINUX".to_string()
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
    SystemTelemetry {
        cpu_percent: read_cpu_percent().unwrap_or(0.0),
        ram_percent: read_ram_percent().unwrap_or(0.0),
        uptime_secs: read_uptime().unwrap_or(0),
        kernel: detect_kernel_version(),
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

static PREV_CPU_SAMPLE: Mutex<Option<(u64, u64)>> = Mutex::new(None);

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
    let idle = parts[3].saturating_add(parts.get(4).copied().unwrap_or(0));
    let total: u64 = parts.iter().sum();
    if total == 0 {
        return None;
    }
    let busy = total.saturating_sub(idle);

    if let Ok(mut guard) = PREV_CPU_SAMPLE.lock() {
        let pct = if let Some((prev_busy, prev_total)) = *guard {
            let d_total = total.saturating_sub(prev_total);
            let d_busy = busy.saturating_sub(prev_busy);
            if d_total > 0 {
                (d_busy as f32 / d_total as f32) * 100.0
            } else {
                (busy as f32 / total as f32) * 100.0
            }
        } else {
            (busy as f32 / total as f32) * 100.0
        };
        *guard = Some((busy, total));
        return Some(pct.clamp(0.0, 100.0));
    }

    Some(((busy as f32 / total as f32) * 100.0).clamp(0.0, 100.0))
}
