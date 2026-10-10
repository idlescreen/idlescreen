use crate::runner::LcgRng;
use crate::storm::Storm;
use crate::storm::types::SceneryCell;

impl Storm {
    pub(crate) fn generate_scenery(
        rng: &mut LcgRng,
        cols: usize,
        rows: usize,
    ) -> (Vec<SceneryCell>, Vec<SceneryCell>, Vec<SceneryCell>) {
        let mut bg = Vec::new();
        let mut mid = Vec::new();
        let mut fg = Vec::new();
        if rows < 10 || crate::runner::is_secondary_monitor() {
            return (bg, mid, fg);
        }

        let primary = crate::runner::get_primary_monitor_bounds(cols, rows);
        let primary_width = primary.width();
        let primary_height = primary.height();
        if primary_height < 10 || primary_width == 0 {
            return (bg, mid, fg);
        }

        let mountain_color = (18, 22, 28);
        let snow_color = (65, 75, 85);

        let mut mountain_heights = vec![0; primary_width];
        for (lx, height) in mountain_heights.iter_mut().enumerate() {
            let x = primary.start_col + lx;
            let h = (primary_height as f32 * 0.20)
                + (x as f32 * 0.04).sin() * (primary_height as f32 * 0.08)
                + (x as f32 * 0.10).cos() * (primary_height as f32 * 0.03);
            *height = h.clamp(2.0, primary_height as f32 * 0.45) as usize;
        }

        for (lx, &m_h) in mountain_heights.iter().enumerate() {
            let x = primary.start_col + lx;
            let peak_y = primary.end_row.saturating_sub(m_h + 3);

            for y in peak_y..primary.end_row.saturating_sub(2) {
                let ch = if y == peak_y {
                    if rng.next_bool(0.5) { '^' } else { '/' }
                } else if y == peak_y + 1 {
                    '.'
                } else {
                    ' '
                };

                let col = if y == peak_y {
                    snow_color
                } else {
                    mountain_color
                };
                if ch != ' ' {
                    bg.push((x, y, ch, col));
                }
            }
        }

        let mut bx = primary.start_col + 4;
        while bx < primary.end_col.saturating_sub(4) {
            let base_y = primary.end_row.saturating_sub(3);
            let bg_h = rng.next_range(1.0, 3.0) as usize;
            let bg_tree_color = (
                rng.next_range(16.0, 24.0) as u8,
                rng.next_range(24.0, 32.0) as u8,
                rng.next_range(20.0, 28.0) as u8,
            );
            let bg_trunk_color = (24, 20, 16);

            bg.push((bx, base_y, '|', bg_trunk_color));
            if bg_h > 1 {
                bg.push((bx, base_y.saturating_sub(1), '|', bg_trunk_color));
            }

            let foliage_top = base_y.saturating_sub(bg_h);
            bg.push((bx, foliage_top, '▲', bg_tree_color));
            if bg_h > 1 {
                bg.push((bx - 1, foliage_top + 1, '▲', bg_tree_color));
                bg.push((bx + 1, foliage_top + 1, '▲', bg_tree_color));
            }
            bx += rng.next_range(6.0, 14.0) as usize;
        }

        let mid_tree_color = (25, 38, 28);
        let mid_trunk_color = (32, 28, 24);

        let mut mx = primary.start_col + 12;
        while mx < primary.end_col.saturating_sub(8) {
            let tree_h = rng.next_range(2.0, 3.5) as usize;
            let base_y = primary.end_row.saturating_sub(3);

            mid.push((mx, base_y, '║', mid_trunk_color));
            for h_offset in 1..=tree_h {
                let foliage_y = base_y.saturating_sub(h_offset);
                mid.push((mx, foliage_y, '▲', mid_tree_color));
                if h_offset > 1 {
                    if mx > 0 {
                        mid.push((mx - 1, foliage_y, '▲', mid_tree_color));
                    }
                    if mx + 1 < cols {
                        mid.push((mx + 1, foliage_y, '▲', mid_tree_color));
                    }
                }
            }
            mx += rng.next_range(8.0, 15.0) as usize;
        }

        let fg_tree_color = (35, 55, 40);
        let trunk_color = (48, 42, 36);

        let mut fx = primary.start_col + primary_width.saturating_sub(22);
        while fx < primary.end_col.saturating_sub(3) {
            let tree_h = rng.next_range(2.0, 4.0) as usize;
            let base_y = primary.end_row.saturating_sub(3);

            fg.push((fx, base_y, '║', trunk_color));

            for h_offset in 1..=tree_h {
                let foliage_y = base_y.saturating_sub(h_offset);
                fg.push((fx, foliage_y, '▲', fg_tree_color));
                if h_offset > 1 {
                    if fx > 0 {
                        fg.push((fx - 1, foliage_y, '▲', fg_tree_color));
                    }
                    if fx + 1 < cols {
                        fg.push((fx + 1, foliage_y, '▲', fg_tree_color));
                    }
                }
            }
            fx += rng.next_range(7.0, 12.0) as usize;
        }

        let tree_x = primary.start_col + 8;
        if primary_width > 20 {
            let base_y = primary.end_row.saturating_sub(3);
            let trunk_top = base_y.saturating_sub(4);
            for y in trunk_top..=base_y {
                fg.push((tree_x, y, '║', trunk_color));
            }
            let branch_y = base_y.saturating_sub(2);
            fg.push((tree_x + 1, branch_y, '═', trunk_color));
            fg.push((tree_x + 2, branch_y, '═', trunk_color));

            let foliage_base = base_y.saturating_sub(4);
            fg.push((tree_x, foliage_base - 2, '▲', fg_tree_color));
            for dx in -1..=1 {
                fg.push((
                    (tree_x as i32 + dx) as usize,
                    foliage_base - 1,
                    '▲',
                    fg_tree_color,
                ));
            }
            for dx in -2..=2 {
                fg.push((
                    (tree_x as i32 + dx) as usize,
                    foliage_base,
                    '▲',
                    fg_tree_color,
                ));
            }
        }

        (bg, mid, fg)
    }
}
