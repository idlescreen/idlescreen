// SPDX-License-Identifier: MIT

use super::doctor_checks::{CheckResult, chk};
use std::path::PathBuf;
use std::process::Command;

pub fn check_fonts() -> CheckResult {
    if font_check_via_fc_list() {
        println!(" [✔] System Fonts: Monospace font is installed.");
        chk("System Fonts", true, "monospace font found")
    } else {
        println!(" [✗] System Fonts: Monospace font not found on system!");
        println!("     -> Fix: Please install fonts-dejavu-core or a system monospace font.");
        chk("System Fonts", false, "monospace font missing")
    }
}

pub fn check_package_install() -> CheckResult {
    for pkg in ["idle-daemon", "idle-cli", "cosmic-idle", "idlescreen", "idle", "trance"] {
        if let Ok(o) = Command::new("rpm")
            .args(["-q", pkg])
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
            .args(["-W", "-f=${Package} ${Version}", pkg])
            .output()
        {
            if o.status.success() {
                let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
                println!(" [✔] Package (DEB): {ver}");
                println!("     -> Upgrade with: sudo apt update && sudo apt upgrade");
                return chk("Package", true, ver);
            }
        }
    }
    println!(" [!] Package not detected via RPM or DEB query.");
    chk("Package", true, "not a system package")
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
