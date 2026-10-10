use crate::cosmos::Cosmos;
use crate::cosmos::physics::to_screen_fast;
use crate::runner::TerminalCell;

pub fn draw_particles_and_trails(
    effect: &Cosmos,
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    dim: f32,
) {
    if dim <= 0.001 {
        return;
    }

    let draw_trails = effect.quality_scale >= 0.50;
    let step = if effect.particles.len() > 380 { 2 } else { 1 };

    if draw_trails {
        for p in effect.particles.iter().step_by(step) {
            let hist_len = p.history.len();
            for (k, &(hx, hy)) in p.history.iter().enumerate() {
                let (sx, sy) = to_screen_fast(
                    hx as f32,
                    hy as f32,
                    effect.universe_cx,
                    effect.universe_cy,
                    effect.screen_cx,
                    effect.screen_cy,
                    effect.zoom,
                );
                if sx >= 0 && sx < cols as i32 && sy >= 0 && sy < rows as i32 {
                    let idx = sy as usize * cols + sx as usize;
                    if grid[idx].ch == ' ' {
                        let intensity = ((k + 1) as f32 / (hist_len + 1) as f32) * 0.35 * dim;
                        let fg = (
                            (p.color.0 as f32 * intensity) as u8,
                            (p.color.1 as f32 * intensity) as u8,
                            (p.color.2 as f32 * intensity) as u8,
                        );
                        grid[idx] = TerminalCell {
                            ch: '·',
                            fg,
                            bg: (0, 0, 0),
                            bold: false,
                        };
                    }
                }
            }
        }
    }

    for p in effect.particles.iter().step_by(step) {
        let (sx, sy) = to_screen_fast(
            p.x,
            p.y,
            effect.universe_cx,
            effect.universe_cy,
            effect.screen_cx,
            effect.screen_cy,
            effect.zoom,
        );
        if sx >= 0 && sx < cols as i32 && sy >= 0 && sy < rows as i32 {
            let idx = sy as usize * cols + sx as usize;
            if grid[idx].ch == ' ' || grid[idx].ch == '·' {
                let fg = (
                    (p.color.0 as f32 * dim) as u8,
                    (p.color.1 as f32 * dim) as u8,
                    (p.color.2 as f32 * dim) as u8,
                );
                grid[idx] = TerminalCell {
                    ch: p.ch,
                    fg,
                    bg: (0, 0, 0),
                    bold: dim > 0.35,
                };
            }
        }
    }
}
