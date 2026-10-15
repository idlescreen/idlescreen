// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Lists available screensavers across the built-in 50-scene 16-bit arcade engine.

use crate::ascii::effects::EffectKind;

pub fn execute() -> Result<(), Box<dyn std::error::Error>> {
    println!("Available Screensavers:");
    println!("  ascii             [built-in] All 50 16-bit Arcade Scenes (Shuffled)");
    println!("  tour              [built-in] All 50 16-bit Arcade Scenes in Order (1/50 to 50/50)");
    if crate::ascii::sys_info::is_superfedora_machine() {
        println!("  superfedora       [unlocked] 16-bit SuperFedora pixel art & 50-scene rotation");
    } else {
        println!(
            "  superfedora       [locked] Requires SuperFedora workstation (superfedora.github.io)"
        );
    }
    println!();
    println!("Pinned 16-Bit Arcade Scenes (50 built-in):");
    for chunk in EffectKind::ALL.chunks(5) {
        let row: Vec<String> = chunk.iter().map(|k| format!("{:<16}", k.name())).collect();
        println!("  {}", row.join(" ").trim_end());
    }
    Ok(())
}
