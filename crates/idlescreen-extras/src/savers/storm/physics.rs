//! Physics submodules and core resize checks/color generation helpers.

pub mod animals;
pub mod bird;
pub mod drops;
pub mod lightning;
pub mod puddle;
pub mod scenery;

use crate::runner::LcgRng;

use crate::runner::get_system_info;

use crate::storm::Storm;
use crate::storm::types::{BirdState, LogoCell, Phase, Star};

impl Storm {
    pub(crate) fn cold_rain_color(rng: &mut LcgRng) -> (u8, u8, u8) {
        let r = rng.next_range(0.0, 1.0);
        if r < 0.60 {
            let brightness = rng.next_range(60.0, 120.0);
            (
                (brightness * 0.8) as u8,
                (brightness * 0.9) as u8,
                brightness as u8,
            )
        } else if r < 0.90 {
            let b = rng.next_range(110.0, 180.0);
            let g = b * rng.next_range(0.6, 0.85);
            let r = g * rng.next_range(0.5, 0.7);
            (r as u8, g as u8, b as u8)
        } else {
            let val = rng.next_range(180.0, 230.0);
            ((val * 0.9) as u8, (val * 0.95) as u8, val as u8)
        }
    }

    pub fn check_resize(&mut self, cols: usize, rows: usize) {
        if cols != self.last_cols || rows != self.last_rows {
            self.intro_fade = 0.0;
            self.logo_cells.clear();
            self.splashes.clear();
            self.drops.clear();
            self.puddle = vec![0.0f32; cols];
            self.puddle_color = vec![(0u8, 0u8, 0u8); cols];

            self.stars.clear();
            let target_stars = (((cols * rows / 20).clamp(15, 60)) as f32
                * self.quality_scale
                * (if self.on_battery { 0.55 } else { 1.0 }))
                as usize;
            for _ in 0..target_stars {
                self.stars.push(Star {
                    x: self.rng.next_f32(),
                    y: self.rng.next_f32() * 0.65, // restrict to upper 65% of screen (the sky)
                    phase: self.rng.next_f32() * std::f32::consts::TAU,
                    ch: if self.rng.next_bool(0.08) {
                        '✦'
                    } else if self.rng.next_bool(0.25) {
                        '+'
                    } else {
                        '.'
                    },
                });
            }

            if let Some(logo) =
                crate::runner::place_centered_logo(cols, rows, &get_system_info().logo_text, None)
            {
                for (r_offset, line) in logo.lines.iter().enumerate() {
                    for (c_offset, ch) in line.chars().enumerate() {
                        if ch != ' ' {
                            self.logo_cells.push(LogoCell {
                                x: logo.x + c_offset,
                                y: logo.y + r_offset,
                                ch,
                                active: true,
                                glow: 0.0,
                                water: 0.0,
                            });
                        }
                    }
                }
            }

            self.phase = Phase::Complete;
            self.phase_timer = 0.0;
            self.last_cols = cols;
            self.last_rows = rows;
            let (bg, mid, fg) = Self::generate_scenery(&mut self.rng, cols, rows);
            self.bg_cells = bg;
            self.mid_scenery = mid;
            self.fg_scenery = fg;

            // Populate all perch points (Big Tree branch + top of logo cells)
            let mut perch_points = Vec::new();
            if !crate::runner::is_secondary_monitor() {
                let primary = crate::runner::get_primary_monitor_bounds(cols, rows);
                let tree_x = primary.start_col + 8;
                perch_points.push((tree_x + 2, primary.end_row.saturating_sub(5)));
                for cell in &self.logo_cells {
                    let has_above = self
                        .logo_cells
                        .iter()
                        .any(|c| c.x == cell.x && c.y == cell.y.saturating_sub(1));
                    if !has_above && cell.y > 0 {
                        perch_points.push((cell.x, cell.y - 1));
                    }
                }
            }
            self.perch_points = perch_points;

            // Choose starting perch point
            if self.perch_points.is_empty() {
                self.bird_perch_x = 0.0;
                self.bird_perch_y = 0.0;
            } else {
                let p_idx = self.rng.next_usize(self.perch_points.len());
                self.bird_perch_x = self.perch_points[p_idx].0 as f32;
                self.bird_perch_y = self.perch_points[p_idx].1 as f32;
            }
            self.bird_x = self.bird_perch_x;
            self.bird_y = self.bird_perch_y;
            self.bird_state = BirdState::Sitting;
            self.bird_timer = self.rng.next_range(5.0, 15.0);
            self.bird_wing_flap = false;
        }
    }
}
