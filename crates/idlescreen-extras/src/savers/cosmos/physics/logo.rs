use crate::cosmos::Cosmos;
use crate::cosmos::physics::screen_to_logo_universe;
use crate::cosmos::types::LogoPixel;
use crate::runner::{get_primary_monitor_bounds, get_system_info, is_secondary_monitor};

/// Target width of the OS caption on the primary monitor.
const LOGO_SCREEN_FRACTION: f32 = 0.22;

fn truncate_to_width(text: &str, max_cols: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_cols {
        return text.to_string();
    }
    if max_cols == 0 {
        return String::new();
    }
    if max_cols == 1 {
        return "…".to_string();
    }
    let mut out: String = chars[..max_cols - 1].iter().collect();
    out.push('…');
    out
}

fn build_text_lines(logo_text: &str, kernel: &str, max_cols: usize) -> Vec<String> {
    if kernel.is_empty() || kernel == "unknown" || kernel == "export" {
        vec![truncate_to_width(logo_text, max_cols)]
    } else {
        vec![
            truncate_to_width(logo_text, max_cols),
            truncate_to_width(kernel, max_cols),
        ]
    }
}

pub fn rebuild_logo_pixels(eff: &mut Cosmos, cols: usize, rows: usize) {
    eff.logo_pixels.clear();

    if is_secondary_monitor() {
        eff.logo_scale = 1.0;
        return;
    }

    let sys = get_system_info();
    let primary = get_primary_monitor_bounds(cols, rows);
    let pw = primary.width().max(1);
    let max_cols = ((pw as f32 * LOGO_SCREEN_FRACTION).round() as usize).max(16);

    let lines = build_text_lines(&sys.logo_text, &sys.kernel, max_cols);
    let logo_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let logo_h = lines.len();
    if logo_w == 0 || logo_h == 0 {
        eff.logo_scale = 1.0;
        return;
    }

    eff.logo_scale = 1.0;

    let x = primary.start_col + primary.width().saturating_sub(logo_w) / 2;
    let y = primary.start_row + primary.height().saturating_sub(logo_h) / 2;

    for (r_offset, line) in lines.iter().enumerate() {
        let gy = y + r_offset;
        for (c_offset, ch) in line.chars().enumerate() {
            let gx = x + c_offset;
            let (ux, uy) = screen_to_logo_universe(
                gx as f32,
                gy as f32,
                eff.universe_cx,
                eff.universe_cy,
                eff.screen_cx,
                eff.screen_cy,
            );
            eff.logo_pixels.push(LogoPixel {
                x: ux,
                y: uy,
                vx: 0.0,
                vy: 0.0,
                origin_x: ux,
                origin_y: uy,
                ch,
                exc: 0.0,
                active: false,
                dissolved: false,
                shards_pending: 0,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_adds_ellipsis_when_needed() {
        assert_eq!(truncate_to_width("hello", 10), "hello");
        assert_eq!(truncate_to_width("hello world", 8), "hello w…");
    }

    #[test]
    fn build_text_lines_keeps_two_rows() {
        let lines = build_text_lines("Pop!_OS 22.04", "6.17.9-generic", 40);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Pop!_OS 22.04");
        assert_eq!(lines[1], "6.17.9-generic");
    }

    #[test]
    fn build_text_lines_omits_unknown_kernel() {
        let lines = build_text_lines("IDLESCREEN", "unknown", 40);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "IDLESCREEN");
    }
}
