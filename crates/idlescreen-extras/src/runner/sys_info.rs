// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! System information and environment queries for extras savers.

use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os: String,
    pub logo_text: String,
    pub kernel: String,
    pub hostname: String,
    pub cpu: String,
    pub uptime_secs: u64,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub mem_used_pct: f32,
    pub cpu_usage_pct: f32,
    pub power_status: String,
    pub disk_summary: String,
    pub gpus: String,
    pub monitors: String,
}

impl Default for SystemInfo {
    fn default() -> Self {
        let os = detect_host_os().unwrap_or_else(|| "Linux".to_string());
        let kernel = detect_kernel().unwrap_or_else(|| "Linux".to_string());
        let power_status = detect_power_status();
        Self {
            logo_text: os.clone(),
            os,
            kernel,
            hostname: "localhost".to_string(),
            cpu: "CPU".to_string(),
            uptime_secs: 3600,
            mem_used_mb: 2048,
            mem_total_mb: 8192,
            mem_used_pct: 25.0,
            cpu_usage_pct: 5.0,
            power_status,
            disk_summary: "OK".to_string(),
            gpus: "GPU".to_string(),
            monitors: "1".to_string(),
        }
    }
}

pub fn get_system_info() -> SystemInfo {
    SystemInfo::default()
}

pub fn detect_power_status() -> String {
    if let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") {
        let mut saw_battery = false;
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(kind) = std::fs::read_to_string(path.join("type")) {
                let kind_trimmed = kind.trim();
                if kind_trimmed.eq_ignore_ascii_case("Mains")
                    || kind_trimmed.eq_ignore_ascii_case("USB")
                {
                    if let Ok(online) = std::fs::read_to_string(path.join("online"))
                        && online.trim() == "1"
                    {
                        return "AC".to_string();
                    }
                } else if kind_trimmed.eq_ignore_ascii_case("Battery") {
                    saw_battery = true;
                    if let Ok(status) = std::fs::read_to_string(path.join("status"))
                        && status.trim().eq_ignore_ascii_case("Discharging")
                    {
                        return "Battery".to_string();
                    }
                }
            }
        }
        if saw_battery {
            return "Battery".to_string();
        }
    }
    "AC".to_string()
}

pub fn detect_host_os() -> Option<String> {
    if let Ok(v) = std::env::var("IDLE_OS_NAME") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    static CACHED_OS: OnceLock<Option<String>> = OnceLock::new();
    CACHED_OS
        .get_or_init(|| {
            for path in ["/etc/os-release", "/usr/lib/os-release"] {
                if let Ok(text) = std::fs::read_to_string(path) {
                    for line in text.lines() {
                        let trimmed = line.trim();
                        if let Some(v) = trimmed.strip_prefix("PRETTY_NAME=") {
                            let s = v.trim().trim_matches('"').trim_matches('\'').trim();
                            if !s.is_empty() {
                                return Some(s.to_string());
                            }
                        }
                    }
                    for line in text.lines() {
                        let trimmed = line.trim();
                        if let Some(v) = trimmed.strip_prefix("NAME=") {
                            let s = v.trim().trim_matches('"').trim_matches('\'').trim();
                            if !s.is_empty() {
                                return Some(s.to_string());
                            }
                        }
                    }
                }
            }
            None
        })
        .clone()
}

pub fn detect_desktop_environment() -> Option<String> {
    for key in [
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_DESKTOP",
        "DESKTOP_SESSION",
    ] {
        if let Ok(v) = std::env::var(key) {
            let t = v.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

pub fn detect_kernel() -> Option<String> {
    if let Ok(v) = std::env::var("IDLE_KERNEL_VERSION") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    static CACHED_KERNEL: OnceLock<Option<String>> = OnceLock::new();
    CACHED_KERNEL
        .get_or_init(|| {
            if let Ok(text) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
                let t = text.trim();
                if !t.is_empty() {
                    return Some(t.to_string());
                }
            }
            None
        })
        .clone()
}

pub fn publish_caption(_text: &str) {}

pub fn env_is_set(keys: &[&str]) -> bool {
    keys.iter().any(|k| std::env::var_os(k).is_some())
}

pub fn param(name: &str) -> Option<String> {
    let key = name.replace(['.', '-'], "_").to_ascii_uppercase();
    std::env::var(format!("IDLE_{key}")).ok()
}

pub fn param_f32(name: &str) -> Option<f32> {
    param(name).and_then(|v| v.parse::<f32>().ok())
}
