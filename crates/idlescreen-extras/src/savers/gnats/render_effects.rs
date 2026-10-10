use super::types::{Firefly, Star};
use crate::runner::TerminalCell;

pub fn draw_stars(
    stars: &[Star],
    time_elapsed: f32,
    cols: usize,
    rows: usize,
    accent: (u8, u8, u8),
    intro_fade: f32,
    grid: &mut [TerminalCell],
) {
    let fade = intro_fade.clamp(0.0, 1.0);
    for star in stars {
        let sx = (star.x * cols as f32) as usize;
        let sy = (star.y * rows as f32) as usize;
        if sx < cols && sy < rows {
            let sparkle = f32::midpoint((time_elapsed * 2.5 + star.phase).sin(), 1.0);
            let brightness = (sparkle * 0.35 + star.excitation * 0.65).min(1.0) * fade;

            let (star_r, star_g, star_b) = if star.excitation > 0.05 {
                // Blend excited star color with registry accent color
                let blend = star.excitation.min(1.0);
                (
                    ((160.0 * (1.0 - blend) + accent.0 as f32 * blend) * fade).min(255.0) as u8,
                    ((180.0 * (1.0 - blend) + accent.1 as f32 * blend) * fade).min(255.0) as u8,
                    ((220.0 * (1.0 - blend) + accent.2 as f32 * blend) * fade).min(255.0) as u8,
                )
            } else {
                // Soft blue-ish white stars twinkling
                (
                    (110.0 * fade + brightness * 70.0) as u8,
                    (120.0 * fade + brightness * 75.0) as u8,
                    (140.0 * fade + brightness * 80.0) as u8,
                )
            };

            let ch = if star.excitation > 0.8 {
                '✹'
            } else if star.excitation > 0.4 {
                '✦'
            } else {
                star.ch
            };

            grid[sy * cols + sx] = TerminalCell {
                ch,
                fg: (star_r, star_g, star_b),
                bg: (0, 0, 0),
                bold: star.excitation > 0.4 || brightness > 0.7,
            };

            // Draw anamorphic lens flares on excited stars
            if star.excitation > 0.6 {
                let flare_intensity = ((star.excitation - 0.6) / 0.4).min(1.0);

                // Horizontal lens flare streak
                let h_len = 6;
                for dx in 1..h_len {
                    let alpha = (90.0 * flare_intensity) as u8;
                    let fade = alpha.saturating_sub((dx * (80 / h_len)) as u8);
                    if fade > 5 {
                        let f_color = (
                            (star_r as f32 * 0.5 + fade as f32 * 0.5).min(255.0) as u8,
                            (star_g as f32 * 0.5 + fade as f32 * 0.5).min(255.0) as u8,
                            (star_b as f32 * 0.5 + fade as f32 * 0.5).min(255.0) as u8,
                        );
                        if sx + dx < cols {
                            let c = &mut grid[sy * cols + (sx + dx)];
                            if c.ch == ' ' {
                                c.ch = '─';
                                c.fg = f_color;
                                c.bold = false;
                            }
                        }
                        if sx >= dx {
                            let c = &mut grid[sy * cols + (sx - dx)];
                            if c.ch == ' ' {
                                c.ch = '─';
                                c.fg = f_color;
                                c.bold = false;
                            }
                        }
                    }
                }

                // Vertical lens flare streak
                let v_len = 3;
                for dy in 1..v_len {
                    let alpha = (70.0 * flare_intensity) as u8;
                    let fade = alpha.saturating_sub((dy * (60 / v_len)) as u8);
                    if fade > 5 {
                        let f_color = (
                            (star_r as f32 * 0.5 + fade as f32 * 0.5).min(255.0) as u8,
                            (star_g as f32 * 0.5 + fade as f32 * 0.5).min(255.0) as u8,
                            (star_b as f32 * 0.5 + fade as f32 * 0.5).min(255.0) as u8,
                        );
                        if sy + dy < rows {
                            let c = &mut grid[(sy + dy) * cols + sx];
                            if c.ch == ' ' {
                                c.ch = '│';
                                c.fg = f_color;
                                c.bold = false;
                            }
                        }
                        if sy >= dy {
                            let c = &mut grid[(sy - dy) * cols + sx];
                            if c.ch == ' ' {
                                c.ch = '│';
                                c.fg = f_color;
                                c.bold = false;
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn draw_connectors(
    fireflies: &[Firefly],
    cols: usize,
    rows: usize,
    intro_fade: f32,
    grid: &mut [TerminalCell],
) {
    let fade = intro_fade.clamp(0.0, 1.0);
    let max_connect_dist = 11.5f32;
    let num_fireflies = fireflies.len();
    for i in 0..num_fireflies {
        for j in (i + 1)..num_fireflies {
            let dx = fireflies[j].x - fireflies[i].x;
            let dy = fireflies[j].y - fireflies[i].y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < max_connect_dist * max_connect_dist {
                let dist = dist_sq.sqrt();
                let x1 = fireflies[i].x;
                let y1 = fireflies[i].y;
                let x2 = fireflies[j].x;
                let y2 = fireflies[j].y;
                let ldx = x2 - x1;
                let ldy = y2 - y1;
                let steps = ldx.abs().max(ldy.abs()).round() as i32;

                if steps > 1 {
                    let x_step = ldx / steps as f32;
                    let y_step = ldy / steps as f32;

                    let adx = ldx.abs();
                    let ady = ldy.abs();
                    let ch = if ady < 0.4142 * adx {
                        '─'
                    } else if ady > 2.4142 * adx {
                        '│'
                    } else if ldx * ldy > 0.0 {
                        '╲'
                    } else {
                        '╱'
                    };

                    let color1 = fireflies[i].color;
                    let color2 = fireflies[j].color;
                    // Depth: closer pairs brighter; distant links sit further back
                    let nearness = (1.0 - dist / max_connect_dist).clamp(0.0, 1.0);
                    let size_depth =
                        f32::midpoint(fireflies[i].size as f32, fireflies[j].size as f32) / 3.0;
                    let intensity = nearness * (0.22 + 0.38 * size_depth) * fade;

                    let line_r = (f32::midpoint(color1.0 as f32, color2.0 as f32) * intensity) as u8;
                    let line_g = (f32::midpoint(color1.1 as f32, color2.1 as f32) * intensity) as u8;
                    let line_b = (f32::midpoint(color1.2 as f32, color2.2 as f32) * intensity) as u8;

                    for k in 1..steps {
                        let lx = (x1 + k as f32 * x_step).round() as i32;
                        let ly = (y1 + k as f32 * y_step).round() as i32;
                        if lx >= 0 && lx < cols as i32 && ly >= 0 && ly < rows as i32 {
                            let idx = ly as usize * cols + lx as usize;
                            if grid[idx].ch == ' ' {
                                grid[idx] = TerminalCell {
                                    ch,
                                    fg: (line_r, line_g, line_b),
                                    bg: (0, 0, 0),
                                    bold: intensity > 0.28,
                                };
                            }
                        }
                    }
                }
            }
        }
    }
}
