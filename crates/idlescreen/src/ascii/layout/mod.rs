// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 3-tier progressive layout and portrait telemetry adaptation.

pub mod cache;
pub mod fonts;

pub use cache::LruFontAtlasCache;
pub use fonts::render_superfedora_banner;

use super::sys_info::SystemTelemetry;
use super::unicode::{non_latin_fallback, normalize_to_ascii_fold};
use fonts::{append_subtitle, get_3x3, get_5x5};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutTier {
    /// cols >= 60: Full 5x5 block banner with centered subtitle
    Wide,
    /// 25 <= cols < 60: Condensed 3x3 banner or bordered text with ellipsis
    Medium,
    /// cols < 25: Single centered glyph or vertical character stack
    Narrow,
}

impl LayoutTier {
    pub fn for_dimensions(cols: usize, _rows: usize) -> Self {
        if cols >= 60 {
            LayoutTier::Wide
        } else if cols >= 25 {
            LayoutTier::Medium
        } else {
            LayoutTier::Narrow
        }
    }
}

pub struct RenderedBanner {
    pub lines: Vec<String>,
    pub width: usize,
    pub height: usize,
}

/// Renders banner lines using the 3-tier progressive layout.
pub fn render_banner(
    raw_text: &str,
    subtitle: Option<&str>,
    cols: usize,
    rows: usize,
) -> RenderedBanner {
    let tier = LayoutTier::for_dimensions(cols, rows);
    let folded = normalize_to_ascii_fold(raw_text);
    let effective_text = if folded
        .chars()
        .all(|c| c.is_ascii() || c.is_ascii_whitespace())
    {
        folded
    } else if let Some(fb) = non_latin_fallback(raw_text) {
        fb.to_string()
    } else {
        folded
    };

    let trimmed_eff = effective_text.trim();
    if trimmed_eff.eq_ignore_ascii_case("SUPERFEDORA")
        || trimmed_eff.eq_ignore_ascii_case("SUPER FEDORA")
    {
        return render_superfedora_banner(tier, subtitle, cols, rows);
    }

    match tier {
        LayoutTier::Wide => {
            let (scale, display_text) = determine_scale(&effective_text, subtitle, cols, rows);
            let mut lines = if scale == 1 {
                let budget = (cols.saturating_sub(4) / 6).max(1);
                let text: String = display_text.chars().take(budget).collect();
                let chars_count = text.chars().count();
                let mut lines = vec![String::new(); 5];
                for (ci, ch) in text.chars().enumerate() {
                    let pattern = get_5x5(ch).unwrap_or(["     "; 5]);
                    for (i, row) in pattern.iter().enumerate() {
                        lines[i].push_str(row);
                        if ci + 1 < chars_count {
                            lines[i].push(' ');
                        }
                    }
                }
                lines
            } else {
                let spacing_str = " ".repeat(if scale >= 4 { 3 } else { 2 });
                let mut lines = vec![String::new(); 5 * scale];
                let chars_count = display_text.chars().count();
                for (ci, ch) in display_text.chars().enumerate() {
                    let pattern = get_5x5(ch).unwrap_or(["     "; 5]);
                    let max_w = pattern.iter().map(|r| r.chars().count()).max().unwrap_or(5);
                    for (r, base_row) in pattern.iter().enumerate() {
                        let mut scaled_row = String::with_capacity(max_w * scale);
                        for c in base_row.chars() {
                            let fill = if matches!(c, '█' | '▀' | '▄') {
                                '█'
                            } else {
                                ' '
                            };
                            scaled_row.extend(std::iter::repeat_n(fill, scale));
                        }
                        let pad = (max_w * scale).saturating_sub(scaled_row.chars().count());
                        scaled_row.extend(std::iter::repeat_n(' ', pad));
                        if ci + 1 < chars_count {
                            scaled_row.push_str(&spacing_str);
                        }
                        for sr in 0..scale {
                            lines[r * scale + sr].push_str(&scaled_row);
                        }
                    }
                }
                lines
            };
            append_subtitle(&mut lines, subtitle, cols.saturating_sub(4), true);
            let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            let height = lines.len();
            RenderedBanner {
                lines,
                width,
                height,
            }
        }
        LayoutTier::Medium => {
            let budget = (cols.saturating_sub(2) / 4).max(1);
            let chars: Vec<char> = effective_text.trim().chars().collect();
            let display_chars: Vec<char> = if chars.len() > budget {
                let mut v: Vec<char> = chars.into_iter().take(budget.saturating_sub(1)).collect();
                v.push('.');
                v
            } else {
                chars
            };
            let mut lines = vec![String::new(); 3];
            for ch in &display_chars {
                let pattern = get_3x3(*ch).unwrap_or(["   "; 3]);
                for (i, row) in pattern.iter().enumerate() {
                    lines[i].push_str(row);
                    lines[i].push(' ');
                }
            }
            append_subtitle(&mut lines, subtitle, cols.saturating_sub(2), false);
            let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            let height = lines.len();
            RenderedBanner {
                lines,
                width,
                height,
            }
        }
        LayoutTier::Narrow => {
            let max_vertical = rows.saturating_sub(4).max(1);
            let lines: Vec<String> = effective_text
                .trim()
                .chars()
                .take(max_vertical)
                .map(|ch| ch.to_string())
                .collect();
            let height = lines.len();
            RenderedBanner {
                lines,
                width: 1,
                height,
            }
        }
    }
}

pub fn is_portrait(cols: usize, rows: usize) -> bool {
    cols > 0 && (rows as f32 / cols as f32) > 1.4
}

pub fn format_portrait_telemetry(
    telemetry: &SystemTelemetry,
    cols: usize,
) -> (Vec<String>, Vec<String>) {
    let top = vec![
        center_text(&format!("CPU: {:.1}%", telemetry.cpu_percent), cols),
        center_text(&format!("RAM: {:.1}%", telemetry.ram_percent), cols),
    ];
    let (hours, mins) = (
        telemetry.uptime_secs / 3600,
        (telemetry.uptime_secs % 3600) / 60,
    );
    let bottom = vec![
        center_text(&telemetry.kernel, cols),
        center_text(&format!("Up: {hours}h {mins}m"), cols),
    ];
    (top, bottom)
}

fn center_text(text: &str, cols: usize) -> String {
    let len = text.chars().count();
    if len >= cols {
        text.chars().take(cols).collect()
    } else {
        format!("{}{text}", " ".repeat((cols - len) / 2))
    }
}

pub fn calculate_banner_scale(text: &str, cols: usize, rows: usize) -> usize {
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("SUPERFEDORA")
        || trimmed.eq_ignore_ascii_case("SUPER FEDORA")
    {
        return 1;
    }
    let effective_rows = if is_portrait(cols, rows) {
        rows.saturating_sub(8)
    } else {
        rows
    };
    let count = trimmed.chars().count();
    for &(scale, min_rows, spacing) in &[(4, 40, 3), (3, 30, 2), (2, 26, 2)] {
        if effective_rows < min_rows {
            continue;
        }
        let mut text_width = 0;
        for (ci, ch) in trimmed.chars().enumerate() {
            let pattern = get_5x5(ch).unwrap_or(["     "; 5]);
            let max_w = pattern.iter().map(|r| r.chars().count()).max().unwrap_or(5);
            text_width += max_w * scale + if ci + 1 < count { spacing } else { 0 };
        }
        if text_width + 4 <= cols {
            return scale;
        }
    }
    1
}

fn determine_scale(
    text: &str,
    _subtitle: Option<&str>,
    cols: usize,
    rows: usize,
) -> (usize, String) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return (1, String::new());
    }
    let scale = calculate_banner_scale(trimmed, cols, rows);
    if scale > 1 {
        (scale, trimmed.to_string())
    } else {
        let budget = (cols.saturating_sub(4) / 6).max(1);
        (1, trimmed.chars().take(budget).collect())
    }
}
