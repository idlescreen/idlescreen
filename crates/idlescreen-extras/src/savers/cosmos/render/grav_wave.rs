use crate::cosmos::Cosmos;
use crate::cosmos::physics::{TERMINAL_ASPECT_Y, to_screen_fast};
use crate::runner::TerminalCell;

pub fn draw_grav_wave(effect: &Cosmos, grid: &mut [TerminalCell], cols: usize, rows: usize) {
    let max_life = effect.grav_wave_max.max(0.1);
    let age = max_life - effect.grav_wave_timer;
    let r_universe = age * 42.0;
    let r_screen = r_universe * effect.zoom;
    let thickness = 2.4f32;
    let band = r_screen + thickness + 2.0;
    let (gw_sx, gw_sy) = to_screen_fast(
        effect.grav_wave_cx,
        effect.grav_wave_cy,
        effect.universe_cx,
        effect.universe_cy,
        effect.screen_cx,
        effect.screen_cy,
        effect.zoom,
    );

    // Soft life curve: ease-out so the ring dissolves gently.
    let life = (effect.grav_wave_timer / max_life).clamp(0.0, 1.0);
    let life_soft = life * life * (3.0 - 2.0 * life);
    let accent = effect.cached_accent;

    let y_min = ((gw_sy as f32 - band).floor() as isize).max(0) as usize;
    let y_max = ((gw_sy as f32 + band).ceil() as isize)
        .min(rows as isize - 1)
        .max(0) as usize;
    let x_min = ((gw_sx as f32 - band * 2.0).floor() as isize).max(0) as usize;
    let x_max = ((gw_sx as f32 + band * 2.0).ceil() as isize)
        .min(cols as isize - 1)
        .max(0) as usize;

    for y in y_min..=y_max {
        for x in x_min..=x_max {
            let dx = x as f32 - gw_sx as f32;
            let dy = (y as f32 - gw_sy as f32) * TERMINAL_ASPECT_Y;
            let dist = (dx * dx + dy * dy).sqrt();

            if (dist - r_screen).abs() < thickness {
                let idx = y * cols + x;
                let ring = (1.0 - (dist - r_screen).abs() / thickness).clamp(0.0, 1.0);
                let intensity = ring * life_soft * 0.85;

                let r = ((120.0 + accent.0 as f32 * 0.25) * intensity) as u8;
                let g = ((70.0 + accent.1 as f32 * 0.2) * intensity
                    + 90.0 * (1.0 - intensity) * life_soft) as u8;
                let b = ((220.0 + accent.2 as f32 * 0.15) * intensity.min(1.0)).min(255.0) as u8;

                if grid[idx].ch == ' ' {
                    if intensity > 0.28 {
                        grid[idx] = TerminalCell {
                            ch: if intensity > 0.7 {
                                '≈'
                            } else if intensity > 0.45 {
                                '~'
                            } else {
                                '·'
                            },
                            fg: (r, g, b),
                            bg: (0, 0, 0),
                            bold: intensity > 0.65,
                        };
                    }
                } else if intensity > 0.35 {
                    let blend = |c_old: u8, c_new: u8| {
                        (c_old as f32 * (1.0 - intensity * 0.7) + c_new as f32 * intensity * 0.7)
                            .clamp(0.0, 255.0) as u8
                    };
                    grid[idx].fg = (
                        blend(grid[idx].fg.0, r),
                        blend(grid[idx].fg.1, g),
                        blend(grid[idx].fg.2, b),
                    );
                    if grid[idx].ch == '═' || grid[idx].ch == '─' {
                        grid[idx].ch = '≈';
                    } else if grid[idx].ch == '☼' || grid[idx].ch == '❂' {
                        grid[idx].ch = '╬';
                    }
                }
            }
        }
    }
}
