use crate::runner::TerminalCell;

const STAR_REF_MASS: f32 = 3.0;

const EMPTY: TerminalCell = TerminalCell {
    ch: ' ',
    fg: (0, 0, 0),
    bg: (0, 0, 0),
    bold: false,
};

// render helper with many positional inputs; refactor to RenderContext tracked for Sprint-03.
#[allow(clippy::too_many_arguments)]
fn plot_cell(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    x: i32,
    y: i32,
    ch: char,
    fg: (u8, u8, u8),
    bold: bool,
) {
    if x < 0 || y < 0 || x >= cols as i32 || y >= rows as i32 {
        return;
    }
    let idx = y as usize * cols + x as usize;
    if idx >= grid.len() {
        return;
    }
    if ch == ' ' {
        grid[idx] = EMPTY;
        return;
    }
    if matches!(grid[idx].ch, ' ' | '·' | '.' | '\'') || bold {
        grid[idx] = TerminalCell {
            ch,
            fg,
            bg: (0, 0, 0),
            bold,
        };
    }
}

fn star_core_glyph(mass: f32, fade_in: f32) -> char {
    if fade_in < 0.6 {
        '·'
    } else if mass >= 6.0 {
        '☼'
    } else {
        '*'
    }
}

/// Multi-cell starburst — not a single glyph; rays + particle corona on the terminal grid.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_star_corona(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    sx: i32,
    sy: i32,
    color: (u8, u8, u8),
    mass: f32,
    fade_in: f32,
    time: f32,
    zoom: f32,
    accent: (u8, u8, u8),
) {
    if fade_in <= 0.01 {
        return;
    }

    let pulse = (time * 1.4 + mass * 0.15).sin() * 0.5 + 0.5;
    let arm_len = ((mass / STAR_REF_MASS).sqrt() * (3.0 + pulse) / zoom).round() as i32;
    let arm_len = arm_len.clamp(2, 14);
    let corona_r = ((mass / STAR_REF_MASS).sqrt() * (2.0 + pulse * 0.5) / zoom).round() as i32;
    let corona_r = corona_r.clamp(1, 8);

    // Slight accent pull on the corona so stars sit in the system theme.
    let at = 0.16;
    let tint = |c: u8, a: u8| c as f32 * (1.0 - at) + a as f32 * at;
    let c0 = tint(color.0, accent.0);
    let c1 = tint(color.1, accent.1);
    let c2 = tint(color.2, accent.2);

    let core_fg = (
        ((c0 * fade_in) + 90.0 * pulse).min(255.0) as u8,
        ((c1 * fade_in) + 70.0 * pulse).min(255.0) as u8,
        ((c2 * fade_in) + 40.0 * pulse).min(255.0) as u8,
    );
    let ray_fg = (
        (c0 * fade_in * 0.75) as u8,
        (c1 * fade_in * 0.75) as u8,
        (c2 * fade_in * 0.75) as u8,
    );
    let haze_fg = (
        (c0 * fade_in * 0.35) as u8,
        (c1 * fade_in * 0.35) as u8,
        (c2 * fade_in * 0.35) as u8,
    );

    let core = star_core_glyph(mass, fade_in);
    plot_cell(grid, cols, rows, sx, sy, core, core_fg, true);

    for d in 1..=arm_len {
        let t = 1.0 - d as f32 / arm_len as f32;
        let fg = (
            ((ray_fg.0 as f32) * t) as u8,
            ((ray_fg.1 as f32) * t) as u8,
            ((ray_fg.2 as f32) * t) as u8,
        );
        let ch = if d == 1 {
            '+'
        } else if t > 0.55 {
            '·'
        } else {
            '.'
        };
        plot_cell(grid, cols, rows, sx, sy - d, ch, fg, d <= 2);
        plot_cell(grid, cols, rows, sx, sy + d, ch, fg, d <= 2);
        for dx in 1..=(d * 2) {
            plot_cell(grid, cols, rows, sx - dx, sy, ch, fg, d <= 2);
            plot_cell(grid, cols, rows, sx + dx, sy, ch, fg, d <= 2);
        }
    }

    if mass >= 4.0 {
        let diag = (arm_len / 2).max(1);
        for d in 1..=diag {
            let t = 1.0 - d as f32 / diag as f32;
            let fg = (
                ((haze_fg.0 as f32) * t) as u8,
                ((haze_fg.1 as f32) * t) as u8,
                ((haze_fg.2 as f32) * t) as u8,
            );
            let hx = d * 2;
            plot_cell(grid, cols, rows, sx - hx, sy - d, '·', fg, false);
            plot_cell(grid, cols, rows, sx + hx, sy - d, '·', fg, false);
            plot_cell(grid, cols, rows, sx - hx, sy + d, '·', fg, false);
            plot_cell(grid, cols, rows, sx + hx, sy + d, '·', fg, false);
        }
    }

    for dy in -corona_r..=corona_r {
        for dx in -corona_r * 2..=corona_r * 2 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let rx = dx as f32 / 2.0;
            let dist = (rx * rx + dy as f32 * dy as f32).sqrt();
            if dist > corona_r as f32 || dist < 0.4 {
                continue;
            }
            let shimmer = (time * 3.0 + dist * 1.7 + rx * 0.4).sin() * 0.5 + 0.5;
            if shimmer < 0.35 {
                continue;
            }
            let t = (1.0 - dist / corona_r as f32) * fade_in * shimmer;
            let fg = (
                (c0 * t * 0.4) as u8,
                (c1 * t * 0.4) as u8,
                (c2 * t * 0.4) as u8,
            );
            plot_cell(grid, cols, rows, sx + dx, sy + dy, '·', fg, false);
        }
    }
}
