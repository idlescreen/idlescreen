// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Lists available screensavers across built-in core and extras.

use crate::extras_ipc::resolve_extras_binary;
use std::process::Command;

pub fn execute() -> Result<(), Box<dyn std::error::Error>> {
    println!("Available Screensavers:");
    println!("  ascii             [built-in] Dynamic Host OS / DE / Kernel mutation");
    if crate::ascii::sys_info::is_superfedora_machine() {
        println!("  superfedora       [unlocked] 16-bit SuperFedora pixel art & spotlight");
    } else {
        println!(
            "  superfedora       [locked] Requires SuperFedora workstation (superfedora.github.io)"
        );
    }

    if let Some(extras_bin) = resolve_extras_binary() {
        if let Ok(output) = Command::new(&extras_bin).arg("--list").output()
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let s = line.trim();
                if !s.is_empty() {
                    println!("  {s:<17} [extras] Mathematical screensaver");
                }
            }
        }
    } else {
        println!();
        println!("Note: Install 'idlescreen-extras' for 11 additional mathematical screensavers.");
    }

    Ok(())
}
