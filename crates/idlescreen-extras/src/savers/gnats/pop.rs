// SPDX-License-Identifier: MIT

use super::types::Attractor;
use crate::runner::hsl_to_rgb;

pub fn create_attractors(
    attractors: &mut Vec<Attractor>,
    cols: usize,
    rows: usize,
    accent: (u8, u8, u8),
) {
    attractors.clear();
    let (acc_h, _acc_s, _acc_l) = crate::runner::rgb_to_hsl(accent.0, accent.1, accent.2);
    let (cx, cy) = if crate::runner::is_secondary_monitor() {
        (cols as f32 / 2.0, rows as f32 / 2.0)
    } else {
        let primary = crate::runner::get_primary_monitor_bounds(cols, rows);
        (
            (primary.start_col + primary.width() / 2) as f32,
            (primary.start_row + primary.height() / 2) as f32,
        )
    };

    attractors.push(Attractor {
        x: cx,
        y: cy,
        color: accent,
        phase: 0.0,
        speed: 0.6,
    });
    attractors.push(Attractor {
        x: cx,
        y: cy,
        color: hsl_to_rgb((acc_h + 120.0).rem_euclid(360.0), 0.95, 0.60),
        phase: 2.0,
        speed: 0.45,
    });
    attractors.push(Attractor {
        x: cx,
        y: cy,
        color: hsl_to_rgb((acc_h - 120.0).rem_euclid(360.0), 0.95, 0.60),
        phase: 4.0,
        speed: 0.75,
    });
}
