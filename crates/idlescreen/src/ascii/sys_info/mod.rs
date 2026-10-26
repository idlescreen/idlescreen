// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! System information and multi-probe OS/DE/Kernel detection.

#![allow(unsafe_code)]

mod de_telemetry;
pub mod os_name;
pub mod theme_probe;

pub use de_telemetry::{
    SystemTelemetry, clean_desktop_env, clean_kernel_version, detect_desktop_env,
    detect_kernel_version, query_telemetry,
};
pub use theme_probe::{
    SystemTheme, ThemeColorScheme, detect_system_theme, parse_color_str, resolve_system_theme_with,
};
// The glyph style is a rasterizer concern; it lives in `raster/` so the
// raster tree stays independent of this probe. Re-exported here because the
// theme resolution returns it.
pub use crate::raster::glyph_style::GlyphFontStyle;

use std::{fs, path::Path, sync::OnceLock};

pub use os_name::clean_os_name;

/// Multi-probe detection for Host OS name.
pub fn detect_os_name() -> String {
    if let Ok(v) = std::env::var("IDLE_OS_NAME") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return clean_os_name(trimmed);
        }
    }

    if is_superfedora_machine() {
        return "SUPERFEDORA".to_string();
    }

    for path in &["/etc/os-release", "/usr/lib/os-release"] {
        if let Some(name) = parse_os_release(path) {
            return clean_os_name(&name);
        }
    }

    if let Some(name) = parse_lsb_release("/etc/lsb-release") {
        return clean_os_name(&name);
    }

    for (path, fallback) in [
        ("/etc/redhat-release", None),
        ("/etc/alpine-release", Some("Alpine")),
        ("/etc/debian_version", Some("Debian")),
    ] {
        if let Ok(content) = fs::read_to_string(path) {
            let first = content.lines().next().unwrap_or("").trim();
            if !first.is_empty() {
                return clean_os_name(fallback.unwrap_or(first));
            }
        }
    }

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

    if let Some(sysname) = uname_sysname()
        && !sysname.is_empty()
    {
        return clean_os_name(&sysname);
    }

    "IdleScreen".to_string()
}

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

fn uname_sysname() -> Option<String> {
    // SAFETY: `uts` is zeroed memory passed to `libc::uname`; `uts.sysname` is null-terminated on success.
    let mut uts: libc::utsname = unsafe { std::mem::zeroed() };
    // SAFETY: `uts` points to valid stack storage.
    if unsafe { libc::uname(&raw mut uts) } == 0 {
        // SAFETY: `uts.sysname` is guaranteed null-terminated by `uname`.
        let c_str = unsafe { std::ffi::CStr::from_ptr(uts.sysname.as_ptr()) };
        c_str.to_str().ok().map(ToString::to_string)
    } else {
        None
    }
}

thread_local! {
    /// Per-thread override for the SuperFedora gate.
    ///
    /// Production code leaves this unset, so the environment and the
    /// filesystem probe decide. Tests set it on their own thread: a
    /// process-global environment variable would otherwise be observed by
    /// every test running in parallel, which made unrelated renders
    /// non-deterministic.
    static SUPERFEDORA_OVERRIDE: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Forces the SuperFedora gate for the current thread only.
///
/// `None` restores environment/filesystem detection.
pub fn set_superfedora_override(value: Option<bool>) {
    SUPERFEDORA_OVERRIDE.with(|c| c.set(value));
}

/// Returns the current thread's SuperFedora override, if any.
pub fn superfedora_override() -> Option<bool> {
    SUPERFEDORA_OVERRIDE.with(std::cell::Cell::get)
}

/// Probes system markers to determine if IdleScreen is installed and run on a SuperFedora workstation.
/// Caches the filesystem probe in `OnceLock` and excludes generic `sf` binaries (B4, §3.2 #5).
pub fn is_superfedora_machine() -> bool {
    if let Some(forced) = superfedora_override() {
        return forced;
    }
    for key in ["SUPERFEDORA", "IDLE_SUPERFEDORA"] {
        if let Ok(v) = std::env::var(key) {
            let t = v.trim();
            if t == "1" || t.eq_ignore_ascii_case("true") {
                return true;
            }
            if t == "0" || t.eq_ignore_ascii_case("false") {
                return false;
            }
        }
    }
    if let Ok(v) = std::env::var("IDLE_OS_NAME") {
        let t = v.trim();
        if t.eq_ignore_ascii_case("superfedora") {
            return true;
        }
    }

    static CACHED_SF: OnceLock<bool> = OnceLock::new();
    *CACHED_SF.get_or_init(|| {
        for p in &[
            "/etc/superfedora/release",
            "/etc/superfedora/version",
            "/etc/superfedora-release",
            "/usr/share/superfedora/release",
            "/etc/superfedora",
        ] {
            if Path::new(p).exists() {
                return true;
            }
        }
        if let Ok(os_rel) = fs::read_to_string("/etc/os-release")
            && os_rel.to_ascii_lowercase().contains("superfedora")
        {
            return true;
        }
        if let Ok(home) = std::env::var("HOME") {
            let home_p = Path::new(&home);
            if home_p.join(".local/share/superfedora").exists()
                || home_p.join(".config/superfedora").exists()
                || home_p.join(".local/bin/superfedora").exists()
            {
                return true;
            }
        }
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                if dir.join("superfedora").is_file() {
                    return true;
                }
            }
        }
        false
    })
}
