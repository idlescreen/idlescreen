use super::Glyphs;
use super::physics;
use crate::runner::TerminalCell;
impl Glyphs {
    pub fn draw_impl(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        grid.fill(TerminalCell::default());

        let accent = self.cached_accent;
        let fade = self.intro_fade.clamp(0.0, 1.0);

        for d in &self.drops {
            let x = d.x as i32;
            if x < 0 || x as usize >= cols {
                continue;
            }
            for k in 0..d.length {
                let y = d.y as i32 - k as i32;
                if y < 0 || y as usize >= rows {
                    continue;
                }
                let idx = (y as usize) * cols + x as usize;
                if idx >= grid.len() {
                    break;
                }
                // Heads prefer live system chars; near-trail also samples them often.
                let use_live = !self.live_system_chars.is_empty()
                    && (k == 0 || (d.layer == 1 && k < 3 && (d.char_rot + k) % 3 == 0));
                let pool = if use_live {
                    &self.live_system_chars
                } else {
                    &self.char_pool
                };
                let ch = pool[(d.char_rot + k) % pool.len().max(1)];
                let (mut r, mut g, mut b) =
                    physics::calculate_glyph_color(accent, k, d.length, d.layer);
                // Accent-colored bright heads (full theme accent at k==0 near layer).
                if k == 0 && d.layer == 1 {
                    r = accent.0;
                    g = accent.1;
                    b = accent.2;
                }
                r = (r as f32 * fade) as u8;
                g = (g as f32 * fade) as u8;
                b = (b as f32 * fade) as u8;
                grid[idx] = TerminalCell {
                    ch,
                    fg: (r, g, b),
                    bg: (0, 0, 0),
                    bold: k == 0 && d.layer == 1,
                };
            }
        }

        if let Some(logo) = crate::runner::place_centered_logo(cols, rows, &self.logo_text, None) {
            let dim = (
                (accent.0 as f32 * 0.35 * fade) as u8,
                (accent.1 as f32 * 0.35 * fade) as u8,
                (accent.2 as f32 * 0.35 * fade) as u8,
            );
            for (r_offset, line) in logo.lines.iter().enumerate() {
                let gy = logo.y + r_offset;
                if gy >= rows {
                    continue;
                }
                for (c_offset, ch) in line.chars().enumerate() {
                    let gx = logo.x + c_offset;
                    if gx >= cols || ch == ' ' {
                        continue;
                    }
                    grid[gy * cols + gx] = TerminalCell {
                        ch,
                        fg: dim,
                        bg: (0, 0, 0),
                        bold: true,
                    };
                }
            }
        }
    }
}
