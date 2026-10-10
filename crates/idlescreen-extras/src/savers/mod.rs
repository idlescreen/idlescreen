// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Consolidates all 11 legacy mathematical screensavers into a unified module.

pub mod aurora;
pub mod beams;
pub mod bursts;
pub mod chaos;
pub mod cosmos;
pub mod glyphs;
pub mod gnats;
pub mod hearth;
pub mod radar;
pub mod ripple;
pub mod storm;

use crate::runner::Screensaver;

pub const ALL_SAVERS: [&str; 11] = [
    "aurora", "beams", "bursts", "chaos", "cosmos", "glyphs", "gnats", "hearth", "radar", "ripple",
    "storm",
];

pub fn create_saver(name: &str) -> Option<Box<dyn Screensaver>> {
    let clean = name.trim().to_ascii_lowercase().replace(['-', '_'], "");
    match clean.as_str() {
        "aurora" => Some(Box::new(aurora::Aurora::new())),
        "beams" => Some(Box::new(beams::Beams::new())),
        "bursts" => Some(Box::new(bursts::Bursts::new())),
        "chaos" => Some(Box::new(chaos::Chaos::new())),
        "cosmos" => Some(Box::new(cosmos::Cosmos::new())),
        "glyphs" | "matrix" => Some(Box::new(glyphs::Glyphs::new())),
        "gnats" => Some(Box::new(gnats::Gnats::new())),
        "hearth" => Some(Box::new(hearth::Hearth::new())),
        "radar" => Some(Box::new(radar::Radar::new())),
        "ripple" => Some(Box::new(ripple::Ripple::new())),
        "storm" => Some(Box::new(storm::Storm::new())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::TerminalCell;
    use std::time::Duration;

    #[test]
    fn test_all_11_savers_instantiate_and_render() {
        let cols = 80;
        let rows = 24;
        for &name in &ALL_SAVERS {
            let mut saver = create_saver(name).unwrap_or_else(|| panic!("saver {name} must exist"));
            saver.init(cols, rows);
            saver.update(Duration::from_millis(16), cols, rows);
            let mut grid = vec![TerminalCell::default(); cols * rows];
            saver.draw(&mut grid, cols, rows);
            assert_eq!(grid.len(), cols * rows);
        }
    }
}
