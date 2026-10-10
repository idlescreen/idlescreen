//! Drawing and rendering implementation for the storm screensaver.

pub mod entities;
pub mod rain_lightning;
pub mod scenery;

use crate::runner::TerminalCell;
use crate::storm::Storm;

impl Storm {
    pub fn draw_impl(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        let accent = self.cached_accent;
        let in_flash = self.lightning_flash > 0.0;
        // Soft flash intensity (peaks bright, long dim tail)
        let flash_t = (self.lightning_flash / 0.32).clamp(0.0, 1.0);
        // Brief accent tint on lightning sky
        let bg_color = if in_flash {
            let base_r = 30.0 + accent.0 as f32 * 0.18 * flash_t;
            let base_g = 36.0 + accent.1 as f32 * 0.18 * flash_t;
            let base_b = 48.0 + accent.2 as f32 * 0.22 * flash_t;
            (
                (base_r * flash_t).min(70.0) as u8,
                (base_g * flash_t).min(80.0) as u8,
                (base_b * flash_t).min(95.0) as u8,
            )
        } else {
            (0, 0, 0)
        };

        // Initialize grid cells to clear space and apply flash background color
        let primary = crate::runner::get_primary_monitor_bounds(cols, rows);
        for y in 0..rows {
            for x in 0..cols {
                let cell = &mut grid[y * cols + x];
                cell.ch = ' ';
                cell.fg = (0, 0, 0);
                cell.bg = if in_flash && primary.contains(x, y) {
                    bg_color
                } else {
                    (0, 0, 0)
                };
                cell.bold = false;
            }
        }

        if !in_flash {
            self.draw_stars(grid, cols, rows, bg_color);
        }

        // 0. Distant mountains & background trees (bg_cells)
        self.draw_bg_cells(grid, cols, rows, bg_color);

        // 1. Background rain drops (is_background is true)
        let rain_char = if self.wind > 2.5 {
            '/'
        } else if self.wind < -2.5 {
            '\\'
        } else {
            '|'
        };

        self.draw_rain(grid, cols, rows, bg_color, rain_char, true);

        // 1b. Background floor splashes (is_background is true)
        self.draw_splashes(grid, cols, rows, bg_color, true);

        // 1c. Background lightning forks (lightning_is_background is true) using thin chars
        self.draw_lightning_bolts(grid, cols, rows, bg_color, in_flash, true);

        // 2. Midground scenery trees (mid_scenery)
        self.draw_midground_scenery(grid, cols, rows, bg_color);

        // 2b. Midground animals (draw Bigfoot as a 3-cell high entity)
        self.draw_midground_animals(grid, cols, rows, bg_color);

        // 3. Foreground trees and Big Pine Tree (fg_scenery)
        self.draw_foreground_scenery(grid, cols, rows, bg_color);

        // Desaturate and cool down the accent color for a cold, miserable feel
        let cool_r = (accent.0 as f32 * 0.25 + 90.0 * 0.75) as u8;
        let cool_g = (accent.1 as f32 * 0.25 + 110.0 * 0.75) as u8;
        let cool_b = (accent.2 as f32 * 0.25 + 135.0 * 0.75) as u8;
        let cold_accent = (cool_r, cool_g, cool_b);

        // 4. Persistent logo cells (logo_cells) with dynamic rippling rain glows and puddle accumulation
        self.draw_logo_cells(grid, cols, rows, bg_color, cold_accent);

        // 5. Foreground animals (draw Bear as 2-cell high entity, Deer as 2-cell high entity)
        self.draw_foreground_animals(grid, cols, rows, bg_color);

        // 6. Bird rendering (render Sitting, Scared, Flying, or Explores wing flaps)
        self.draw_bird(grid, cols, rows, bg_color);

        // 7. Floor puddles (puddle and puddle_color)
        self.draw_puddles(grid, cols, rows, bg_color);

        // 8. Foreground rain drops (is_background is false)
        self.draw_rain(grid, cols, rows, bg_color, rain_char, false);

        // 8b. Foreground splashes/sparks (is_background is false)
        self.draw_splashes(grid, cols, rows, bg_color, false);

        // 8c. Foreground lightning forks (lightning_is_background is false) using thin chars
        self.draw_lightning_bolts(grid, cols, rows, bg_color, in_flash, false);

        // 9. Subtitles drawn centered below the logo on primary monitor
        self.draw_subtitles(grid, cols, rows);
    }
}
