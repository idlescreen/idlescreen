// SPDX-License-Identifier: MIT

//! Check whether a newer *system package* is available.
//!
//! Detection order matters: many Fedora systems also ship `apt-cache` for
//! foreign tooling. Prefer whatever package manager **actually installed**
//! `trance`, not whichever tool happens to be on PATH.

use std::process::Command;

use anyhow::Result;

const PKG: &str = "trance";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Apt,
    Dnf,
}

fn command_ok(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn stdout_trim(cmd: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(cmd).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// How was trance installed on this machine?
fn detect_backend() -> Option<Backend> {
    // 1) What actually owns the package?
    if command_ok("rpm", &["-q", PKG]) {
        return Some(Backend::Dnf);
    }
    if command_ok("dpkg-query", &["-W", "-f=${Status}", PKG])
        || command_ok("dpkg", &["-s", PKG])
    {
        // Confirm "installed" status when possible.
        if let Some(status) = stdout_trim("dpkg-query", &["-W", "-f=${Status}", PKG]) {
            if status.contains("install ok installed") {
                return Some(Backend::Apt);
            }
        } else {
            return Some(Backend::Apt);
        }
    }

    // 2) OS family when the package name is not registered yet.
    if let Ok(os) = std::fs::read_to_string("/etc/os-release") {
        let id = os
            .lines()
            .find_map(|l| l.strip_prefix("ID="))
            .unwrap_or("")
            .trim_matches('"');
        let like = os
            .lines()
            .find_map(|l| l.strip_prefix("ID_LIKE="))
            .unwrap_or("")
            .trim_matches('"');
        if id == "fedora"
            || id == "rhel"
            || id == "centos"
            || id == "rocky"
            || id == "almalinux"
            || like.split_whitespace().any(|t| {
                matches!(t, "fedora" | "rhel" | "centos")
            })
        {
            if which("dnf") || which("rpm") {
                return Some(Backend::Dnf);
            }
        }
        if id == "debian"
            || id == "ubuntu"
            || id == "pop"
            || like.split_whitespace().any(|t| {
                matches!(t, "debian" | "ubuntu")
            })
        {
            if which("apt-cache") || which("apt") {
                return Some(Backend::Apt);
            }
        }
    }

    // 3) Last resort: PATH (dnf before apt — apt-cache often exists on Fedora).
    if which("dnf") {
        return Some(Backend::Dnf);
    }
    if which("apt-cache") {
        return Some(Backend::Apt);
    }
    None
}

fn which(cmd: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| {
            std::env::split_paths(&p).any(|dir| dir.join(cmd).is_file())
        })
        .unwrap_or(false)
}

fn rpm_installed_version(pkg: &str) -> Option<String> {
    stdout_trim("rpm", &["-q", pkg, "--qf", "%{VERSION}-%{RELEASE}"])
}

fn dnf_available_version(pkg: &str) -> Option<String> {
    // dnf5-friendly; refreshes metadata when needed.
    stdout_trim(
        "dnf",
        &[
            "repoquery",
            "--available",
            "--latest-limit=1",
            "--qf",
            "%{version}-%{release}",
            pkg,
        ],
    )
    .or_else(|| {
        // Older dnf / no network: parse list output.
        let out = Command::new("dnf")
            .args(["list", "--available", pkg])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        parse_dnf_list_version(&text, /*available_section*/ true)
    })
}

/// Parse `dnf list` lines like: `trance.x86_64 0.3.33-1 crateria`
fn parse_dnf_list_version(text: &str, want_available: bool) -> Option<String> {
    let mut section = "";
    let mut last = None;
    for line in text.lines() {
        let line = line.trim();
        if line.contains("Installed Packages") || line == "Installed packages" {
            section = "installed";
            continue;
        }
        if line.contains("Available Packages") || line == "Available packages" {
            section = "available";
            continue;
        }
        if !(line.starts_with("trance.") || line.starts_with("trance ")) {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let ver = parts[1].to_string();
        if want_available && (section == "available" || section.is_empty()) {
            last = Some(ver);
        } else if !want_available && section == "installed" {
            last = Some(ver);
        }
    }
    last
}

fn handle_dnf_update() -> Result<()> {
    println!("Checking for updates with DNF/RPM...");

    let installed = rpm_installed_version(PKG);
    let available = dnf_available_version(PKG);

    match (installed, available) {
        (Some(inst), Some(cand)) => {
            if versions_equalish(&inst, &cand) {
                println!(" [✔] Trance is already up to date (version {inst}).");
                println!("     -> Upgrade anytime with: sudo dnf update");
            } else {
                println!(" [!] A new version is available: {inst} → {cand}");
                println!("     -> Run: sudo dnf update");
            }
        }
        (Some(inst), None) => {
            println!(" [✔] Installed version: {inst}");
            println!(" [!] Could not query the latest package from the repo.");
            println!("     -> Try: sudo dnf clean all && sudo dnf update");
            println!("     -> Confirm the crateria repo is in /etc/yum.repos.d/");
        }
        (None, Some(cand)) => {
            println!(" [!] Trance is not installed as an RPM (latest in repo: {cand}).");
            println!("     -> Install with: sudo dnf install trance");
        }
        (None, None) => {
            println!(" [!] Could not find the 'trance' package via RPM/DNF.");
            println!("     -> Register the repo, then: sudo dnf install trance");
            println!("     -> curl -fsSL https://crateria.github.io/packages/rpm/crateria.repo \\");
            println!("          | sudo tee /etc/yum.repos.d/crateria.repo");
        }
    }
    Ok(())
}

fn apt_policy_versions(pkg: &str) -> Option<(String, String)> {
    let output = Command::new("apt-cache")
        .args(["policy", pkg])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut installed = None;
    let mut candidate = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(s) = line.strip_prefix("Installed:") {
            installed = Some(s.trim().to_string());
        } else if let Some(s) = line.strip_prefix("Candidate:") {
            candidate = Some(s.trim().to_string());
        }
    }
    Some((installed?, candidate?))
}

fn handle_apt_update() -> Result<()> {
    println!("Checking for updates with APT...");

    match apt_policy_versions(PKG) {
        Some((inst, cand)) => {
            if inst == "(none)" {
                println!(" [!] Trance is not installed as a DEB package.");
                println!("     -> Install with: sudo apt install trance");
            } else if inst != cand {
                println!(" [!] A new version is available: {inst} → {cand}");
                println!("     -> Run: sudo apt update && sudo apt upgrade");
            } else {
                println!(" [✔] Trance is already up to date (version {inst}).");
                println!("     -> Upgrade anytime with: sudo apt update && sudo apt upgrade");
            }
        }
        None => {
            // Fall back to dpkg version only.
            if let Some(inst) = stdout_trim("dpkg-query", &["-W", "-f=${Version}", PKG]) {
                println!(" [✔] Installed version: {inst}");
                println!(" [!] Could not read APT candidate (is the crateria repo configured?).");
                println!("     -> sudo apt update && sudo apt upgrade");
            } else {
                println!(" [!] Could not determine package status for 'trance'.");
                println!("     -> Ensure the crateria APT repo is registered, then:");
                println!("     -> sudo apt update && sudo apt install trance");
            }
        }
    }
    Ok(())
}

/// Compare RPM-style `0.3.33-1` loosely (ignore arch suffix noise).
fn versions_equalish(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        s.trim()
            .trim_end_matches(".x86_64")
            .trim_end_matches(".noarch")
            .to_string()
    };
    norm(a) == norm(b)
}

#[tracing::instrument]
pub fn handle_self_update() -> Result<()> {
    match detect_backend() {
        Some(Backend::Dnf) => handle_dnf_update(),
        Some(Backend::Apt) => handle_apt_update(),
        None => {
            println!(" [!] No supported package manager detected (need DNF/RPM or APT).");
            println!("     -> Fedora: sudo dnf update");
            println!("     -> Debian/Ubuntu: sudo apt update && sudo apt upgrade");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dnf5_list_available() {
        let text = "\
Installed packages
trance.x86_64 0.3.32-1 crateria

Available packages
trance.x86_64 0.3.29-1 crateria
trance.x86_64 0.3.33-1 crateria
";
        assert_eq!(
            parse_dnf_list_version(text, true).as_deref(),
            Some("0.3.33-1")
        );
        assert_eq!(
            parse_dnf_list_version(text, false).as_deref(),
            Some("0.3.32-1")
        );
    }

    #[test]
    fn versions_equalish_ignores_arch() {
        assert!(versions_equalish("0.3.33-1", "0.3.33-1"));
        assert!(versions_equalish("0.3.33-1.x86_64", "0.3.33-1"));
        assert!(!versions_equalish("0.3.32-1", "0.3.33-1"));
    }
}
