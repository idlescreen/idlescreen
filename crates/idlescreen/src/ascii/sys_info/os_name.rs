// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Brand-word normalisation for the detected host OS name.

/// Cleans raw OS name to crisp uppercase single brand words (e.g. "FEDORA", "ARCH", "UBUNTU", "LINUX").
pub fn clean_os_name(raw: &str) -> String {
    let mut s = raw.trim();
    if let Some(idx) = s.find('(') {
        s = s[..idx].trim();
    }

    let upper = s.to_ascii_uppercase();
    if upper.is_empty() || upper == "UNKNOWN" {
        return "IDLESCREEN".to_string();
    }

    let words: Vec<&str> = upper
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();

    if words.contains(&"SUPERFEDORA") || (words.contains(&"SUPER") && words.contains(&"FEDORA")) {
        return "SUPERFEDORA".to_string();
    }
    for &w in &words {
        match w {
            "POP" | "POPOS" => return "POP".to_string(),
            "ARCO" | "ARCOLINUX" => return "ARCO".to_string(),
            "FEDORA" | "ARCH" | "UBUNTU" | "DEBIAN" | "ALPINE" | "OPENSUSE" | "SLES" | "VOID"
            | "GENTOO" | "MANJARO" | "NIXOS" | "RHEL" | "CENTOS" | "ROCKY" | "ALMA" | "SOLUS"
            | "PARCH" | "ARCHCRAFT" | "CACHYOS" | "ENDEAVOUROS" | "GARUDA" | "ARTIX"
            | "CHIMERA" | "SLACKWARE" | "MINT" | "ZORIN" | "ELEMENTARY" | "KALI" | "NOBARA"
            | "BAZZITE" => return w.to_string(),
            _ => {}
        }
    }
    if words.contains(&"SUSE") {
        return if words.contains(&"ENTERPRISE") {
            "SLES"
        } else {
            "SUSE"
        }
        .to_string();
    }
    if words.contains(&"RED") && words.contains(&"HAT") {
        return "RHEL".to_string();
    }
    if words.contains(&"KDE") || words.contains(&"NEON") {
        return "KDE".to_string();
    }
    let ignore = [
        "LINUX",
        "GNU",
        "SERVER",
        "WORKSTATION",
        "DESKTOP",
        "ENTERPRISE",
        "EDITION",
        "LTS",
        "RELEASE",
        "THE",
        "PROJECT",
    ];
    for w in &words {
        if !ignore.contains(w) {
            return (*w).to_string();
        }
    }
    if words.contains(&"LINUX") {
        return "LINUX".to_string();
    }
    "IDLESCREEN".to_string()
}
