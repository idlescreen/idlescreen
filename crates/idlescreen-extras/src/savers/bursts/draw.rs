//! Drawing implementation for Bursts.

use super::Bursts;
use super::drawing;
use super::physics;
use super::types::ActiveExplosion;
use crate::runner::TerminalCell;

/// Soft ease-out for particle/explosion life so colors fade gently.
fn soft_life(life: f32, max_life: f32) -> f32 {
    let t = (life / max_life.max(1e-4)).clamp(0.0, 1.0);
    // Smoothstep-ish: holds a bit then eases down.
    t * t * (3.0 - 2.0 * t)
}

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
        cell.bg = (
            (cell.bg.0 as f32 * fade) as u8,
            (cell.bg.1 as f32 * fade) as u8,
            (cell.bg.2 as f32 * fade) as u8,
        );
    }
}

impl Bursts {
    pub fn draw_impl(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        // Clear grid to avoid lingering elements/trails from previous frames
        for cell in grid.iter_mut() {
            cell.ch = ' ';
            cell.fg = (0, 0, 0);
            cell.bg = (0, 0, 0);
            cell.bold = false;
        }

        let reach = crate::runner::span_reach_scale(cols, rows);
        let intro = self.intro_fade.clamp(0.0, 1.0);

        // Collect active explosions to light up the logo and buildings
        let mut active_explosions = Vec::new();
        for exp in &self.explosions {
            let pct = soft_life(exp.life, exp.max_life);
            active_explosions.push(ActiveExplosion {
                x: exp.x,
                y: exp.y,
                radius: 18.0 * pct * reach,
                color: exp.color,
                intensity: pct,
            });
        }

        // 1. Draw background stars
        drawing::draw_stars(
            &self.stars,
            &self.skyline,
            &active_explosions,
            self.time_elapsed,
            grid,
            cols,
            rows,
        );

        // 2. Draw centered logo
        drawing::draw_logo(
            &self.logo_text,
            &active_explosions,
            grid,
            cols,
            rows,
            self.accent,
        );

        // 3. Draw rising rockets
        for rocket in &self.rockets {
            let cx = rocket.x as usize;
            let cy = rocket.y as usize;
            if cx < cols && cy < rows {
                grid[cy * cols + cx] = TerminalCell {
                    ch: '^',
                    fg: (255, 255, 255),
                    bg: (0, 0, 0),
                    bold: true,
                };
            }
        }

        // 4. Draw explosion particles (soft life curve)
        for p in &self.particles {
            let cx = p.x as usize;
            let cy = p.y as usize;
            if cx < cols && cy < rows {
                let is_smoke = p.color == (100, 100, 100);
                let height_from_bottom = rows.saturating_sub(1).saturating_sub(cy);
                let height_fade = if is_smoke {
                    1.0
                } else {
                    let dist = height_from_bottom.saturating_sub(self.skyline[cx]);
                    (dist as f32 / 5.0).clamp(0.0, 1.0)
                };
                let pct = soft_life(p.life, p.max_life) * height_fade;

                if pct > 0.04 {
                    let color = (
                        (p.color.0 as f32 * pct) as u8,
                        (p.color.1 as f32 * pct) as u8,
                        (p.color.2 as f32 * pct) as u8,
                    );

                    // Only draw if skyward of the skyline profile (except smoke)
                    if height_from_bottom >= self.skyline[cx] || is_smoke {
                        // Only overwrite empty space or flares (except if smoke)
                        let current_ch = grid[cy * cols + cx].ch;
                        if is_smoke
                            || current_ch == ' '
                            || current_ch == '─'
                            || current_ch == '│'
                            || current_ch == '/'
                            || current_ch == '\\'
                        {
                            grid[cy * cols + cx] = TerminalCell {
                                ch: p.ch,
                                fg: color,
                                bg: (0, 0, 0),
                                bold: pct > 0.5,
                            };
                        }
                    }
                }
            }
        }

        // 5. Draw city skyline (with building windows reacting to nearby explosions)
        drawing::draw_skyline(
            &self.skyline,
            &self.skyline_windows,
            &active_explosions,
            self.time_elapsed,
            grid,
            cols,
            rows,
        );

        // 6. Draw overlay cinematic lens flares and starbursts centered at active explosion origins
        for exp in &self.explosions {
            let pct = soft_life(exp.life, exp.max_life);
            let ex = exp.x;
            let ey = exp.y;
            let sx = ex as usize;
            let sy = ey as usize;
            if sx < cols && sy < rows {
                let flare_intensity = pct;
                let (er, eg, eb) = exp.color;

                let center_idx = sy * cols + sx;
                let ch = if flare_intensity > 0.6 {
                    '✸'
                } else if flare_intensity > 0.3 {
                    '✦'
                } else {
                    '•'
                };
                let starburst_color = (
                    (f32::midpoint(er as f32, 255.0) * flare_intensity) as u8,
                    (f32::midpoint(eg as f32, 255.0) * flare_intensity) as u8,
                    (f32::midpoint(eb as f32, 255.0) * flare_intensity) as u8,
                );

                grid[center_idx] = TerminalCell {
                    ch,
                    fg: starburst_color,
                    bg: grid[center_idx].bg,
                    bold: flare_intensity > 0.4,
                };

                // Draw horizontal streak (subtle anamorphic horizontal glow)
                let h_len = (6.0 * reach).ceil() as usize;
                for dx in 1..h_len {
                    let alpha = (160.0 * flare_intensity) as u8;
                    let fade = alpha.saturating_sub((dx * (150 / h_len.max(1))) as u8);
                    if fade > 15 {
                        if sx + dx < cols {
                            let cell = &mut grid[sy * cols + (sx + dx)];
                            cell.fg =
                                physics::blend_explosion_flare_h_color(cell.fg, fade, (er, eg, eb));
                            if cell.ch == ' ' {
                                cell.ch = '─';
                            }
                        }
                        if sx >= dx {
                            let cell = &mut grid[sy * cols + (sx - dx)];
                            cell.fg =
                                physics::blend_explosion_flare_h_color(cell.fg, fade, (er, eg, eb));
                            if cell.ch == ' ' {
                                cell.ch = '─';
                            }
                        }
                    }
                }
            }
        }

        apply_intro_fade(grid, intro);
    }
}
