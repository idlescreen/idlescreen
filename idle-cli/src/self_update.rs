// SPDX-License-Identifier: MIT

//! Check whether a newer *system package* is available.

use anyhow::Result;
use std::process::Command;

use super::self_update_backend::{Backend, detect_backend, stdout_trim};

const PKG: &str = "trance";

fn rpm_installed_version(pkg: &str) -> Option<String> {
    stdout_trim("rpm", &["-q", pkg, "--qf", "%{VERSION}-%{RELEASE}"])
}

fn dnf_available_version(pkg: &str) -> Option<String> {
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
        let out = Command::new("dnf")
            .args(["list", "--available", pkg])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        parse_dnf_list_version(&text, true)
    })
}

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
        if !(line.starts_with("idle.")
            || line.starts_with("idle ")
            || line.starts_with("trance.")
            || line.starts_with("trance "))
        {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let ver = parts[1].to_string();
        if (want_available && (section == "available" || section.is_empty()))
            || (!want_available && section == "installed")
        {
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
            println!("     -> Confirm the idlescreen repo is in /etc/yum.repos.d/");
        }
        (None, Some(cand)) => {
            println!(" [!] IdleScreen is not installed as an RPM (latest in repo: {cand}).");
            println!("     -> Install with: sudo dnf install idlescreen");
        }
        (None, None) => {
            println!(" [!] Could not find the 'idlescreen' package via RPM/DNF.");
            println!("     -> Register the repo, then: sudo dnf install idlescreen");
            println!(
                "     -> curl -fsSL https://idlescreen.github.io/packages/rpm/idlescreen.repo \\"
            );
            println!("          | sudo tee /etc/yum.repos.d/idlescreen.repo");
        }
    }
    Ok(())
}

fn apt_policy_versions(pkg: &str) -> Option<(String, String)> {
    let out = stdout_trim("apt-cache", &["policy", pkg])?;
    let mut inst = None;
    let mut cand = None;
    for line in out.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Installed:") {
            inst = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("Candidate:") {
            cand = Some(rest.trim().to_string());
        }
    }
    match (inst, cand) {
        (Some(i), Some(c)) if i != "(none)" && c != "(none)" => Some((i, c)),
        (None, Some(c)) if c != "(none)" => Some(("(none)".to_string(), c)),
        _ => None,
    }
}

fn self_update_apt() -> Result<(), Box<dyn std::error::Error>> {
    println!(" Checking APT package status for '{PKG}'...");
    match apt_policy_versions(PKG) {
        Some((inst, cand)) => {
            println!(" [✔] Installed version: {inst}");
            println!(" [✔] Repository version: {cand}");

            if inst == "(none)" {
                println!(" [!] Package is not currently installed.");
                println!("     -> Run: sudo apt update && sudo apt install idlescreen");
            } else if !versions_equalish(&inst, &cand) {
                println!(" [!] Upgrade available: {inst} -> {cand}");
                println!("     -> Run: sudo apt update && sudo apt install --only-upgrade idlescreen");
            } else {
                println!(" [✔] IdleScreen is up to date.");
                println!("     -> Upgrade anytime with: sudo apt update && sudo apt upgrade");
            }
        }
        None => {
            if let Some(inst) = stdout_trim("dpkg-query", &["-W", "-f=${Version}", PKG]) {
                println!(" [✔] Installed version: {inst}");
                println!(" [!] Could not read APT candidate (is the idlescreen repo configured?).");
                println!("     -> sudo apt update && sudo apt upgrade");
            } else {
                println!(" [!] Could not determine package status for 'idlescreen'.");
                println!("     -> Ensure the idlescreen APT repo is registered, then:");
                println!("     -> sudo apt update && sudo apt install idlescreen");
            }
        }
    }
    Ok(())
}

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
#[path = "self_update_tests.rs"]
mod tests;
