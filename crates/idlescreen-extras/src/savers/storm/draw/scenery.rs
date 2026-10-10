use crate::runner::TerminalCell;
use crate::storm::Storm;

impl Storm {
    pub(crate) fn draw_stars(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        bg_color: (u8, u8, u8),
    ) {
        let fade = self.intro_fade.clamp(0.0, 1.0);
        for star in &self.stars {
            let sx = (star.x * cols as f32) as usize;
            let sy = (star.y * rows as f32) as usize;
            if sx < cols && sy < rows {
                let sparkle = f32::midpoint((self.time_elapsed * 2.5 + star.phase).sin(), 1.0);
                let brightness = (sparkle * 0.4 + 0.6) * fade;
                // Dim stars slightly to match cold dark storm atmosphere
                let r = (110.0 * brightness) as u8;
                let g = (120.0 * brightness) as u8;
                let b = (140.0 * brightness) as u8;

                grid[sy * cols + sx] = TerminalCell {
                    ch: star.ch,
                    fg: (r, g, b),
                    bg: bg_color,
                    bold: sparkle > 0.6,
                };
            }
        }
    }

    pub(crate) fn draw_bg_cells(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        bg_color: (u8, u8, u8),
    ) {
        for &(bx, by, bch, bcol) in &self.bg_cells {
            if bx < cols && by < rows {
                grid[by * cols + bx] = TerminalCell {
                    ch: bch,
                    fg: bcol,
                    bg: bg_color,
                    bold: false,
                };
            }
        }
    }

    pub(crate) fn draw_midground_scenery(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        bg_color: (u8, u8, u8),
    ) {
        for &(mx, my, mch, mcol) in &self.mid_scenery {
            if mx < cols && my < rows {
                grid[my * cols + mx] = TerminalCell {
                    ch: mch,
                    fg: mcol,
                    bg: bg_color,
                    bold: false,
                };
            }
        }
    }

    pub(crate) fn draw_foreground_scenery(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        bg_color: (u8, u8, u8),
    ) {
        for &(fx, fy, fch, fcol) in &self.fg_scenery {
            if fx < cols && fy < rows {
                grid[fy * cols + fx] = TerminalCell {
                    ch: fch,
                    fg: fcol,
                    bg: bg_color,
                    bold: false,
                };
            }
        }
    }

    pub(crate) fn draw_puddles(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        bg_color: (u8, u8, u8),
    ) {
        for x in 0..cols {
            if x < self.puddle.len() && self.puddle[x] > 0.05 {
                let p_level = self.puddle[x];
                let ch = if p_level > 1.8 {
                    '█'
                } else if p_level > 0.9 {
                    '▄'
                } else {
                    '_'
                };

                let col = self.puddle_color[x];
                let intensity = (p_level / 2.0).min(1.0);
                let fg = (
                    (col.0 as f32 * intensity) as u8,
                    (col.1 as f32 * intensity) as u8,
                    (col.2 as f32 * intensity) as u8,
                );

                let y = rows.saturating_sub(1);
                grid[y * cols + x] = TerminalCell {
                    ch,
                    fg,
                    bg: bg_color,
                    bold: p_level > 1.0,
                };
            }
        }
    }

    pub(crate) fn draw_subtitles(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        // Clear the native-resolution overlay subtitle
        crate::runner::publish_caption("");

        let display = format_subtitle_for_display(&self.subtitle);
        if display.is_empty() {
            return;
        }

        if crate::runner::is_secondary_monitor() {
            return;
        }

        let primary = crate::runner::get_primary_monitor_bounds(cols, rows);
        let logo_bottom_y = self
            .logo_cells
            .iter()
            .map(|c| c.y)
            .max()
            .unwrap_or_else(|| primary.start_row + primary.height() / 2);

        let y = (logo_bottom_y + 2).min(primary.end_row.saturating_sub(3));
        if y >= rows {
            return;
        }

        let text_chars: Vec<char> = display.chars().collect();
        let start_x = primary.start_col + (primary.width().saturating_sub(text_chars.len())) / 2;

        for (i, &ch) in text_chars.iter().enumerate() {
            let x = start_x + i;
            if x < cols {
                grid[y * cols + x] = TerminalCell {
                    ch,
                    fg: (170, 180, 190), // soft cold grey-blue
                    bg: (0, 0, 0),
                    bold: true,
                };
            }
        }
    }
}

fn format_subtitle_for_display(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    if text.is_empty() {
        return text;
    }

    if let Some(inner) = text.strip_prefix("[Subtitles: ") {
        text = inner.trim_end_matches(']').trim().to_string();
    } else if text.starts_with('[') && text.ends_with(']') && text.len() > 2 {
        text = text[1..text.len() - 1].trim().to_string();
    }

    text
}
