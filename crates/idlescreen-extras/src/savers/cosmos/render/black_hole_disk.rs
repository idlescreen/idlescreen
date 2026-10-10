use crate::cosmos::Cosmos;
use crate::runner::TerminalCell;

const BH_REF_MASS: f32 = 8.0;

pub fn bh_radius_from_mass(mass: f32) -> f32 {
    5.0 * (mass / BH_REF_MASS).sqrt().clamp(1.0, 1.85)
}

/// Shared accretion-disk renderer for seeds and singularity/collapse views.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_black_hole_disk(
    effect: &Cosmos,
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    bh_x: i32,
    bh_y: i32,
    fade_in: f32,
    seed_color: (u8, u8, u8),
    mass: f32,
    spin_rate: f32,
) {
    let r_universe = bh_radius_from_mass(mass);
    let band = r_universe / 5.0;
    let r_scaled = (r_universe * effect.zoom) as i32;
    let dir = if effect.spin_clockwise { 1.0 } else { -1.0 };

    for dy in -r_scaled..=r_scaled {
        for dx in -r_scaled * 2..=r_scaled * 2 {
            let rx_f = (dx as f32 / 2.0) / effect.zoom;
            let ry_f = (dy as f32) / effect.zoom;
            let dist = (rx_f * rx_f + ry_f * ry_f).sqrt();
            let gx = bh_x + dx;
            let gy = bh_y + dy;
            if gx >= 0 && gx < cols as i32 && gy >= 0 && gy < rows as i32 {
                let idx = gy as usize * cols + gx as usize;
                if dist < 1.3 * band * fade_in {
                    grid[idx] = TerminalCell {
                        ch: ' ',
                        fg: (0, 0, 0),
                        bg: (0, 0, 0),
                        bold: false,
                    };
                } else if dist < 4.8 * band * fade_in {
                    let angle = ry_f.atan2(rx_f);
                    let wave = (angle - effect.time_elapsed * spin_rate * dir + dist * 1.2).sin();
                    if wave > -0.3 {
                        // Subtle theme accent pull on the accretion disk.
                        let accent = effect.cached_accent;
                        let at = 0.22;
                        let (fg, ch) = if dist < 2.2 * band * fade_in {
                            (
                                (
                                    ((180.0 * (1.0 - at) + accent.0 as f32 * at) * fade_in) as u8,
                                    ((240.0 * (1.0 - at) + accent.1 as f32 * at) * fade_in) as u8,
                                    ((255.0 * (1.0 - at) + accent.2 as f32 * at) * fade_in) as u8,
                                ),
                                '╬',
                            )
                        } else if dist < 3.5 * band * fade_in {
                            let t = 0.18;
                            (
                                (
                                    ((seed_color.0 as f32 * (1.0 - t) + accent.0 as f32 * t)
                                        * fade_in) as u8,
                                    ((seed_color.1 as f32 * (1.0 - t) + accent.1 as f32 * t)
                                        * fade_in) as u8,
                                    ((seed_color.2 as f32 * (1.0 - t) + accent.2 as f32 * t)
                                        * fade_in) as u8,
                                ),
                                if wave > 0.3 { '═' } else { '─' },
                            )
                        } else {
                            (
                                (
                                    ((seed_color.0.saturating_sub(50)) as f32 * fade_in) as u8,
                                    ((seed_color.1.saturating_sub(20)) as f32 * fade_in) as u8,
                                    ((seed_color.2.saturating_sub(80)) as f32 * fade_in) as u8,
                                ),
                                if wave > 0.4 { '~' } else { '·' },
                            )
                        };
                        grid[idx] = TerminalCell {
                            ch,
                            fg,
                            bg: (0, 0, 0),
                            bold: dist < 3.2 * band * fade_in,
                        };
                    }
                }
            }
        }
    }
}
