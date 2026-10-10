use crate::runner::TerminalCell;

// render helper with many positional inputs (grid, dimensions, params); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_spike(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    x: i32,
    y: i32,
    ch: char,
    fade: u8,
    green_mult: f32,
    blue_add: u8,
    accent: (u8, u8, u8),
) {
    if x >= 0 && x < cols as i32 && y >= 0 && y < rows as i32 {
        let cell = &mut grid[y as usize * cols + x as usize];
        if cell.ch == ' ' || cell.ch == ch {
            let fg_r = (fade as f32 * 0.5 + accent.0 as f32 * 0.5).min(255.0) as u8;
            let fg_g = ((fade as f32 * green_mult) * 0.5 + accent.1 as f32 * 0.5).min(255.0) as u8;
            let fg_b = (fade.saturating_add(blue_add) as f32 * 0.5 + accent.2 as f32 * 0.5)
                .min(255.0) as u8;
            cell.ch = ch;
            cell.fg = (fg_r, fg_g, fg_b);
        }
    }
}
