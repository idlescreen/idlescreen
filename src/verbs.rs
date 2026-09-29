// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Router-native verbs: `components`, `install`, `remove`, `versions`.
//! Everything else falls through to `idle-cli` (see main.rs).

use std::process::{Command, ExitCode};

use crate::catalog::{self, COMPONENTS};

/// Print the component catalog with install status.
pub fn components() -> ExitCode {
    println!(
        "{:<8} {:<14} {:<10} DESCRIPTION",
        "NAME", "PACKAGE", "STATE"
    );
    for c in COMPONENTS {
        let state = if catalog::installed(c) {
            "installed"
        } else {
            "missing"
        };
        println!("{:<8} {:<14} {:<10} {}", c.name, c.package, state, c.desc);
    }
    println!("\nRun a component:   idlescreen <name> [args]   e.g. idlescreen tui");
    println!("Daemon commands:   idlescreen <verb>          e.g. idlescreen preview beams");
    ExitCode::SUCCESS
}

/// Install components via the system package manager.
pub fn install(args: &[String]) -> ExitCode {
    let pkgs = resolve_packages(args);
    if pkgs.is_empty() {
        eprintln!("usage: idlescreen install <component>... | idlescreen install all");
        eprintln!("components: {}", names().join(", "));
        return ExitCode::FAILURE;
    }
    run_pkg_manager("install", &pkgs)
}

/// Remove components via the system package manager.
pub fn remove(args: &[String]) -> ExitCode {
    let pkgs = resolve_packages(args);
    if pkgs.is_empty() {
        eprintln!("usage: idlescreen remove <component>...");
        return ExitCode::FAILURE;
    }
    run_pkg_manager("remove", &pkgs)
}

/// Print installed versions of every component.
pub fn versions() -> ExitCode {
    println!("idlescreen {}", env!("CARGO_PKG_VERSION"));
    for c in COMPONENTS {
        let v = match c.binary {
            Some(bin) if catalog::binary_installed(bin) => bin_version(bin),
            _ => catalog::package_version(c.package).unwrap_or_else(|| "not installed".to_string()),
        };
        println!("{:<8} {}", c.name, v);
    }
    ExitCode::SUCCESS
}

fn names() -> Vec<&'static str> {
    COMPONENTS.iter().map(|c| c.name).collect()
}

/// Map component names to package names; `all` expands to the full set.
fn resolve_packages(args: &[String]) -> Vec<String> {
    let mut pkgs = Vec::new();
    for arg in args {
        if arg == "all" {
            pkgs.extend(COMPONENTS.iter().map(|c| c.package.to_string()));
            continue;
        }
        match catalog::lookup(arg) {
            Some(c) => pkgs.push(c.package.to_string()),
            None => eprintln!("idlescreen: unknown component '{arg}'"),
        }
    }
    pkgs.sort();
    pkgs.dedup();
    pkgs
}

/// `dnf install`/`apt install` — whichever exists. Escalates via sudo.
fn run_pkg_manager(verb: &str, pkgs: &[String]) -> ExitCode {
    let mgr = if which("dnf") {
        "dnf"
    } else if which("apt") {
        "apt"
    } else {
        eprintln!("idlescreen: no supported package manager (dnf/apt) found");
        return ExitCode::FAILURE;
    };
    let status = Command::new("sudo")
        .arg(mgr)
        .arg(verb)
        .args(pkgs)
        .arg("-y")
        .status();
    match status {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(u8::try_from(s.code().unwrap_or(1)).unwrap_or(1)),
        Err(e) => {
            eprintln!("idlescreen: failed to run sudo {mgr}: {e}");
            ExitCode::FAILURE
        }
    }
}

fn which(bin: &str) -> bool {
    catalog::binary_installed(bin)
}

/// `bin --version`, first line.
fn bin_version(bin: &str) -> String {
    Command::new(bin)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .map(str::to_string)
        })
        .unwrap_or_else(|| "installed".to_string())
}
