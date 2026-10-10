use crate::cosmos::Cosmos;
use crate::cosmos::physics::to_screen_fast;
use crate::runner::TerminalCell;

pub fn draw_big_bang_shell(effect: &Cosmos, grid: &mut [TerminalCell], cols: usize, rows: usize) {
    let max_t = 1.6;
    let progress = effect.state_timer / max_t;
    let r_universe = effect.state_timer * 26.0;
    let shell_thickness = 3.5f32;
    let r_screen = r_universe * effect.zoom;
    let band = r_screen + shell_thickness * effect.zoom + 2.0;
    let bang_ux = effect.universe_cx;
    let bang_uy = effect.universe_cy;
    let (ucx_screen, ucy_screen) = to_screen_fast(
        bang_ux,
        bang_uy,
        effect.universe_cx,
        effect.universe_cy,
        effect.screen_cx,
        effect.screen_cy,
        effect.zoom,
    );

    let y_min = ((ucy_screen as f32 - band).floor() as isize).max(0) as usize;
    let y_max = ((ucy_screen as f32 + band).ceil() as isize)
        .min(rows as isize - 1)
        .max(0) as usize;
    let x_min = ((ucx_screen as f32 - band * 2.0).floor() as isize).max(0) as usize;
    let x_max = ((ucx_screen as f32 + band * 2.0).ceil() as isize)
        .min(cols as isize - 1)
        .max(0) as usize;

    for gy in y_min..=y_max {
        for gx in x_min..=x_max {
            let dx = gx as i32 - ucx_screen;
            let dy = gy as i32 - ucy_screen;
            let rx_f = (dx as f32 / 2.0) / effect.zoom;
            let ry_f = (dy as f32) / effect.zoom;
            let dist = (rx_f * rx_f + ry_f * ry_f).sqrt();

            if dist < r_universe && dist > (r_universe - shell_thickness).max(0.0) {
                let idx = gy * cols + gx;
                if idx < grid.len() {
                    let shell_rel = (dist - (r_universe - shell_thickness)) / shell_thickness;

                    let color = if progress < 0.25 {
                        (255, 255, 255)
                    } else if shell_rel > 0.8 {
                        (100, 240, 255)
                    } else if shell_rel > 0.4 {
                        (255, 140, 50)
                    } else {
                        (180, 50, 220)
                    };

                    let ch = if progress < 0.3 {
                        if shell_rel > 0.5 { '█' } else { '▓' }
                    } else if progress < 0.6 {
                        if shell_rel > 0.5 { '▓' } else { '░' }
                    } else if progress < 0.9 {
                        if shell_rel > 0.6 { '░' } else { '·' }
                    } else if shell_rel > 0.8 {
                        '·'
                    } else {
                        ' '
                    };

                    if ch != ' ' {
                        grid[idx] = TerminalCell {
                            ch,
                            fg: color,
                            bg: (0, 0, 0),
                            bold: progress < 0.7,
                        };
                    }
                }
            }
        }
    }
}
