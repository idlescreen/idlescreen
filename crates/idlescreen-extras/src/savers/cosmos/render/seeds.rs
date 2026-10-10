use super::black_hole_disk::draw_black_hole_disk;
use super::star_corona::draw_star_corona;
use crate::cosmos::Cosmos;
use crate::cosmos::physics::to_screen_fast;
use crate::runner::TerminalCell;

pub fn draw_seeds(effect: &Cosmos, grid: &mut [TerminalCell], cols: usize, rows: usize) {
    for seed in &effect.seeds {
        if !seed.active {
            continue;
        }
        let (sx, sy) = to_screen_fast(
            seed.x,
            seed.y,
            effect.universe_cx,
            effect.universe_cy,
            effect.screen_cx,
            effect.screen_cy,
            effect.zoom,
        );
        if seed.is_black_hole {
            let fade_in = (seed.birth_timer / 2.0).min(1.0);
            draw_black_hole_disk(
                effect, grid, cols, rows, sx, sy, fade_in, seed.color, seed.mass, 8.0,
            );
        } else if sx >= 0 && sx < cols as i32 && sy >= 0 && sy < rows as i32 {
            let fade_in = (seed.birth_timer / 1.5).min(1.0);
            draw_star_corona(
                grid,
                cols,
                rows,
                sx,
                sy,
                seed.color,
                seed.mass,
                fade_in,
                effect.time_elapsed,
                effect.zoom,
                effect.cached_accent,
            );
        }
    }
}
