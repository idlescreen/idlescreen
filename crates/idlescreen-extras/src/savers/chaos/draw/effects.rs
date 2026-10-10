use crate::chaos::{Chaos, ExplosionType, Phase};
use crate::runner::TerminalCell;

impl Chaos {
    pub(crate) fn draw_special_effects(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        max_possible_dist: f32,
        center_x: f32,
        center_y: f32,
    ) {
        if self.phase != Phase::Chaos {
            return;
        }

        match self.explosion_type {
            ExplosionType::Shockwave => {
                let ring_radius = ((self.phase_timer * 28.0) % (max_possible_dist * 1.2)) as i32;
                let ring_thickness = 2;
                for r in (ring_radius - ring_thickness)..=(ring_radius + ring_thickness) {
                    if r < 2 {
                        continue;
                    }
                    for angle_step in 0..36 {
                        let angle = (angle_step as f32) * 10.0 * std::f32::consts::PI / 180.0;
                        let rx = (center_x + r as f32 * angle.cos()).round() as i32;
                        let ry = (center_y + r as f32 * angle.sin() * 0.48).round() as i32;
                        if rx >= 0 && rx < cols as i32 && ry >= 0 && ry < rows as i32 {
                            let idx = (ry as usize) * cols + (rx as usize);
                            let cell = &mut grid[idx];
                            if cell.ch == ' ' || cell.ch == '.' || cell.ch == '•' {
                                let use_block = ((r + angle_step) % 3) == 0;
                                cell.ch = if use_block { '▓' } else { '░' };
                                let intensity =
                                    (180.0 + (r as f32 - ring_radius as f32).abs() * 20.0)
                                        .min(255.0) as u8;
                                cell.fg = (
                                    intensity,
                                    (intensity as f32 * 0.7) as u8,
                                    intensity.saturating_sub(30),
                                );
                                cell.bold = true;
                            }
                        }
                    }
                }
            }
            ExplosionType::Entropy => {
                for p in &self.particles {
                    if !p.snapped {
                        let px = p.x.round() as i32;
                        let py = p.y.round() as i32;
                        let seed =
                            ((self.phase_timer * 17.0 + px as f32 * 0.7 + py as f32) as i32) % 17;
                        if seed % 7 < 2 {
                            for d in 0..3 {
                                let ox = (seed + d) % 7 - 3;
                                let oy = (seed * 3 + d) % 5 - 2;
                                let rx = px + ox;
                                let ry = py + oy;
                                if rx >= 0 && rx < cols as i32 && ry >= 0 && ry < rows as i32 {
                                    let idx = (ry as usize) * cols + (rx as usize);
                                    let cell = &mut grid[idx];
                                    if cell.ch == ' ' || cell.ch == '.' {
                                        cell.ch =
                                            ['░', '▒', '▓', '?', '#'][(seed + d) as usize % 5];
                                        cell.fg = (80, 60, 40);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ExplosionType::Resonance => {
                let hum = ((self.phase_timer * 25.0).sin() * 0.5 + 0.5).min(1.0);
                for p in &self.particles {
                    if p.snapped {
                        let px = p.x.round() as i32;
                        let py = p.y.round() as i32;
                        if px >= 0 && px < cols as i32 && py >= 0 && py < rows as i32 {
                            let idx = (py as usize) * cols + (px as usize);
                            let cell = &mut grid[idx];
                            if cell.ch != ' ' {
                                let boost = (hum * 40.0) as u8;
                                cell.fg = (
                                    cell.fg.0.saturating_add(boost),
                                    cell.fg.1.saturating_add(boost / 2),
                                    cell.fg.2.saturating_add(boost / 3),
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
