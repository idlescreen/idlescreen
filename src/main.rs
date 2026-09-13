// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! `idlescreen` — the IdleScreen router binary.
//!
//! One front door for the whole product line:
//!   idlescreen <component> [args]   exec the component's binary
//!   idlescreen <verb>               daemon/protocol verbs → idle-cli
//!   idlescreen install <comp>...    install components via dnf/apt
//!   idlescreen components|versions  catalog introspection
//!
//! Unknown first arguments fall through to `idle-cli` so every existing
//! `idlescreen <verb>` invocation keeps working unchanged.

mod catalog;
mod verbs;

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("help" | "-h" | "--help") => usage(),
        Some("-V" | "--version") => {
            println!("idlescreen {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("components") => verbs::components(),
        Some("install") => verbs::install(&args[1..]),
        Some("remove") => verbs::remove(&args[1..]),
        Some("versions") => verbs::versions(),
        Some(name) => route(name, &args[1..]),
    }
}

/// Dispatch a non-verb first argument: component binary if it names a
/// runnable component, managed-component info if not, `idle-cli`
/// passthrough if it isn't a component at all.
fn route(name: &str, rest: &[String]) -> ExitCode {
    match catalog::lookup(name) {
        Some(c) => match c.binary {
            Some(bin) => exec_or_hint(bin, rest, c.package),
            None => managed_info(c),
        },
        None => exec_or_hint("idle-cli", &prepend(name, rest), "idle-cli"),
    }
}

fn prepend(head: &str, rest: &[String]) -> Vec<String> {
    std::iter::once(head.to_string())
        .chain(rest.iter().cloned())
        .collect()
}

/// Run `bin` in the foreground, forwarding its exit status. A missing
/// binary prints the package that provides it.
fn exec_or_hint(bin: &str, args: &[String], pkg: &str) -> ExitCode {
    match Command::new(bin).args(args).status() {
        Ok(s) => ExitCode::from(u8::try_from(s.code().unwrap_or(1)).unwrap_or(1)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("idlescreen: '{bin}' is not installed.");
            eprintln!(
                "  install it:  sudo dnf install {pkg}   (or: idlescreen install {})",
                pkg.trim_start_matches("idle-")
            );
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("idlescreen: failed to launch {bin}: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Managed components have no user-facing binary — report their state.
fn managed_info(c: &catalog::Component) -> ExitCode {
    let state = if catalog::installed(c) {
        catalog::package_version(c.package)
            .map_or_else(|| "installed".to_string(), |v| format!("installed ({v})"))
    } else {
        "not installed".to_string()
    };
    println!("{}: {} — {}", c.name, state, c.desc);
    if c.name == "savers" {
        let dir = std::path::Path::new("/usr/libexec/idle/screensavers");
        if let Ok(rd) = std::fs::read_dir(dir) {
            let mut names: Vec<String> = rd
                .filter_map(Result::ok)
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.ends_with(".idleplugin.toml"))
                .map(|n| n.trim_end_matches(".idleplugin.toml").to_string())
                .map(|n| n.trim_start_matches("libscreensaver_").to_string())
                .collect();
            names.sort();
            println!("plugins: {}", names.join(", "));
        }
    }
    if c.name == "runtime" {
        let active = Command::new("systemctl")
            .args(["--user", "is-active", "--quiet", "idle-daemon.service"])
            .status()
            .is_ok_and(|s| s.success());
        println!("daemon: {}", if active { "running" } else { "not running" });
    }
    ExitCode::SUCCESS
}

fn usage() -> ExitCode {
    println!(
        "idlescreen — the IdleScreen front door\n\
         \n\
         USAGE:\n    \
         idlescreen <component> [args]   run a component (cli, tui, studio, cosmic)\n    \
         idlescreen <verb> [args]        daemon commands → idle-cli (preview, stop, list, …)\n    \
         idlescreen components           show the component catalog\n    \
         idlescreen install <comp>...    install components ('all' for everything)\n    \
         idlescreen remove <comp>...     remove components\n    \
         idlescreen versions             installed versions of every component\n    \
         idlescreen -V | --version       router version\n\
         \n\
         Anything unrecognized is forwarded to idle-cli — existing\n\
         `idlescreen <verb>` invocations keep working."
    );
    ExitCode::SUCCESS
}
