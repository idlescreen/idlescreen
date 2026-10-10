//! Chaos screensaver drawing: stars, lens flares, particles, and phase effects.

mod effects;
mod spike;
mod stars;

use super::{Chaos, Phase};
use crate::runner::TerminalCell;

fn apply_intro_fade(grid: &mut [TerminalCell], intro_fade: f32) {
    let fade = intro_fade.clamp(0.0, 1.0);
    if fade >= 0.999 {
        return;
    }
    for cell in grid.iter_mut() {
        cell.fg = (
            (cell.fg.0 as f32 * fade) as u8,
            (cell.fg.1 as f32 * fade) as u8,
            (cell.fg.2 as f32 * fade) as u8,
        );
    }
}

impl Chaos {
    pub fn draw_impl(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        grid.fill(TerminalCell::default());

        let accent = self.cached_accent;
        let intro = self.intro_fade.clamp(0.0, 1.0);

        self.draw_stars(grid, cols, rows, accent);

        let center_x = self.center_x;
        let center_y = self.center_y;
        let max_possible_dist = self.max_possible_dist;

        self.draw_special_effects(grid, cols, rows, max_possible_dist, center_x, center_y);

        // Soft chromatic: continuous strength, no hard on/off.
        let chroma = self.chromatic_strength.clamp(0.0, 1.0);
        let shift = if chroma > 0.08 {
            // Cap shift so we never full-screen flash.
            ((chroma * 2.2).round() as i32).clamp(1, 2)
        } else {
            0
        };
        // Channel ghost intensity scales with chroma (softer than solid RGB ghosts).
        let ghost_a = (chroma * 0.72).clamp(0.0, 0.72);

        let inv_max_possible_dist = self.inv_max_possible_dist;

        for p in &self.particles {
            let px = p.x.round() as i32;
            let py = p.y.round() as i32;

            if px >= 0 && px < cols as i32 && py >= 0 && py < rows as i32 {
                let color = if self.phase == Phase::Assembled {
                    // Accent-tinted readable logo particles.
                    let glow_factor = p.glow.min(1.2);
                    if glow_factor > 0.9 {
                        let t = ((glow_factor - 0.9) / 0.3).min(1.0);
                        let r = (accent.0 as f32 * (1.0 - t) + 220.0 * t).min(255.0) as u8;
                        let g = (accent.1 as f32 * (1.0 - t) + 220.0 * t).min(255.0) as u8;
                        let b = (accent.2 as f32 * (1.0 - t) + 230.0 * t).min(255.0) as u8;
                        (r, g, b)
                    } else {
                        let base = 0.55 + 0.45 * glow_factor;
                        let r = (accent.0 as f32 * base).min(255.0) as u8;
                        let g = (accent.1 as f32 * base).min(255.0) as u8;
                        let b = (accent.2 as f32 * base).min(255.0) as u8;
                        (r, g, b)
                    }
                } else {
                    // Accent holds the near field; warm chaos only at the edges.
                    let dx = p.x - center_x;
                    let dy = p.y - center_y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let ratio = (dist * inv_max_possible_dist).min(1.0);
                    let r = (255.0 * ratio * 0.85 + (accent.0 as f32) * (1.0 - ratio * 0.85)) as u8;
                    let g = (110.0 * ratio * 0.75 + (accent.1 as f32) * (1.0 - ratio * 0.75)) as u8;
                    let b = ((accent.2 as f32) * (1.0 - ratio * 0.5) + 40.0 * ratio) as u8;
                    (r, g, b)
                };

                let idx = py as usize * cols + px as usize;

                if shift > 0 && ghost_a > 0.05 {
                    let rx = px - shift;
                    if rx >= 0 && rx < cols as i32 {
                        let r_idx = py as usize * cols + rx as usize;
                        let gr = (230.0 * ghost_a) as u8;
                        let gg = (10.0 * ghost_a) as u8;
                        let gb = (50.0 * ghost_a) as u8;
                        grid[r_idx] = TerminalCell {
                            ch: p.ch,
                            fg: (gr, gg, gb),
                            bg: grid[r_idx].bg,
                            bold: false,
                        };
                    }
                    let bx = px + shift;
                    if bx >= 0 && bx < cols as i32 {
                        let b_idx = py as usize * cols + bx as usize;
                        let br = (0.0 * ghost_a) as u8;
                        let bg = (120.0 * ghost_a) as u8;
                        let bb = (255.0 * ghost_a) as u8;
                        grid[b_idx] = TerminalCell {
                            ch: p.ch,
                            fg: (br, bg, bb),
                            bg: grid[b_idx].bg,
                            bold: false,
                        };
                    }
                }

                let center_fg = if shift > 0 && ghost_a > 0.2 {
                    // Soft green mid-channel, mixed toward true color as chroma decays.
                    let mix = (ghost_a - 0.2) / 0.52;
                    (
                        (10.0 * mix + color.0 as f32 * (1.0 - mix)) as u8,
                        (230.0 * mix + color.1 as f32 * (1.0 - mix)) as u8,
                        (80.0 * mix + color.2 as f32 * (1.0 - mix)) as u8,
                    )
                } else {
                    color
                };

                grid[idx] = TerminalCell {
                    ch: p.ch,
                    fg: center_fg,
                    bg: grid[idx].bg,
                    bold: self.phase == Phase::Assembled || p.glow > 0.8,
                };
            }
        }

        apply_intro_fade(grid, intro);
    }
}
