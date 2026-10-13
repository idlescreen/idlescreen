// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 3-tier progressive layout and portrait telemetry adaptation.

use super::sys_info::SystemTelemetry;
use super::unicode::{non_latin_fallback, normalize_to_ascii_fold};

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

/// 5x5 font pattern for uppercase ASCII characters.
fn get_5x5(ch: char) -> Option<[&'static str; 5]> {
    let u = ch.to_ascii_uppercase();
    match u {
        'A' => Some([" ███ ", "█   █", "█████", "█   █", "█   █"]),
        'B' => Some(["████ ", "█   █", "████ ", "█   █", "████ "]),
        'C' => Some([" ████", "█    ", "█    ", "█    ", " ████"]),
        'D' => Some(["████ ", "█   █", "█   █", "█   █", "████ "]),
        'E' => Some(["█████", "█    ", "████ ", "█    ", "█████"]),
        'F' => Some(["█████", "█    ", "████ ", "█    ", "█    "]),
        'G' => Some([" ████", "█    ", "█ ███", "█   █", " ████"]),
        'H' => Some(["█   █", "█   █", "█████", "█   █", "█   █"]),
        'I' => Some(["█████", "  █  ", "  █  ", "  █  ", "█████"]),
        'J' => Some(["    █", "    █", "    █", "█   █", " ███ "]),
        'K' => Some(["█   █", "█  █ ", "███  ", "█  █ ", "█   █"]),
        'L' => Some(["█    ", "█    ", "█    ", "█    ", "█████"]),
        'M' => Some(["█   █", "██ ██", "█ █ █", "█   █", "█   █"]),
        'N' => Some(["█   █", "██  █", "█ █ █", "█  ██", "█   █"]),
        'O' => Some([" ███ ", "█   █", "█   █", "█   █", " ███ "]),
        'P' => Some(["████ ", "█   █", "████ ", "█    ", "█    "]),
        'Q' => Some([" ███ ", "█   █", "█ █ █", "█  █ ", " ████"]),
        'R' => Some(["████ ", "█   █", "████ ", "█  █ ", "█   █"]),
        'S' => Some([" ████", "█    ", " ███ ", "    █", "████ "]),
        'T' => Some(["█████", "  █  ", "  █  ", "  █  ", "  █  "]),
        'U' => Some(["█   █", "█   █", "█   █", "█   █", " ███ "]),
        'V' => Some(["█   █", "█   █", " █ █ ", " █ █ ", "  █  "]),
        'W' => Some(["█   █", "█   █", "█ █ █", "██ ██", "█   █"]),
        'X' => Some(["█   █", " █ █ ", "  █  ", " █ █ ", "█   █"]),
        'Y' => Some(["█   █", " █ █ ", "  █  ", "  █  ", "  █  "]),
        'Z' => Some(["█████", "   █ ", "  █  ", " █   ", "█████"]),
        '0' => Some([" ███ ", "█  ██", "█ █ █", "██  █", " ███ "]),
        '1' => Some(["  █  ", " ██  ", "  █  ", "  █  ", "█████"]),
        '2' => Some([" ███ ", "█   █", "   █ ", " █   ", "█████"]),
        '3' => Some(["████ ", "    █", " ███ ", "    █", "████ "]),
        '4' => Some(["█   █", "█   █", "█████", "    █", "    █"]),
        '5' => Some(["█████", "█    ", "████ ", "    █", "████ "]),
        '6' => Some([" ███ ", "█    ", "████ ", "█   █", " ███ "]),
        '7' => Some(["█████", "    █", "   █ ", "  █  ", "  █  "]),
        '8' => Some([" ███ ", "█   █", " ███ ", "█   █", " ███ "]),
        '9' => Some([" ███ ", "█   █", " ████", "    █", " ███ "]),
        '_' => Some(["     ", "     ", "     ", "     ", "█████"]),
        '!' => Some(["  █  ", "  █  ", "  █  ", "     ", "  █  "]),
        '.' => Some(["     ", "     ", "     ", "     ", "  █  "]),
        '-' => Some(["     ", "     ", " ███ ", "     ", "     "]),
        ' ' => Some(["     ", "     ", "     ", "     ", "     "]),
        _ => Some([" ███ ", "█   █", "█   █", "█   █", " ███ "]),
    }
}

/// 3x3 condensed font pattern for uppercase ASCII characters.
fn get_3x3(ch: char) -> Option<[&'static str; 3]> {
    let u = ch.to_ascii_uppercase();
    match u {
        'A' => Some(["█▀█", "█▀█", "▀ ▀"]),
        'B' => Some(["█▀▄", "█▀▄", "▀▀ "]),
        'C' => Some(["█▀▀", "█  ", "▀▀▀"]),
        'D' => Some(["█▀▄", "█ █", "▀▀ "]),
        'E' => Some(["█▀▀", "█▀ ", "▀▀▀"]),
        'F' => Some(["█▀▀", "█▀ ", "▀  "]),
        'G' => Some(["█▀▀", "█ █", "▀▀▀"]),
        'H' => Some(["█ █", "█▀█", "▀ ▀"]),
        'I' => Some(["▀█▀", " █ ", "▀▀▀"]),
        'J' => Some(["  █", "  █", "▀▀ "]),
        'K' => Some(["█ █", "██ ", "▀ ▀"]),
        'L' => Some(["█  ", "█  ", "▀▀▀"]),
        'M' => Some(["█▀█", "█ █", "▀ ▀"]),
        'N' => Some(["█▀█", "█ █", "▀ ▀"]),
        'O' => Some(["█▀█", "█ █", "▀▀▀"]),
        'P' => Some(["█▀█", "█▀▀", "▀  "]),
        'Q' => Some(["█▀█", "█▄█", "▀▀█"]),
        'R' => Some(["█▀█", "█▀▄", "▀ ▀"]),
        'S' => Some(["█▀▀", "▀▀█", "▀▀▀"]),
        'T' => Some(["▀█▀", " █ ", " ▀ "]),
        'U' => Some(["█ █", "█ █", "▀▀▀"]),
        'V' => Some(["█ █", "█ █", " ▀ "]),
        'W' => Some(["█ █", "█▀█", "▀ ▀"]),
        'X' => Some(["▀▄▀", " █ ", "▀ ▀"]),
        'Y' => Some(["█ █", " █ ", " ▀ "]),
        'Z' => Some(["▀▀█", " █ ", "█▀▀"]),
        '0' => Some(["█▀█", "█ █", "▀▀▀"]),
        '1' => Some([" █ ", " █ ", " ▀ "]),
        '2' => Some(["▀▀█", "█▀▀", "▀▀▀"]),
        '3' => Some(["▀▀█", " ▀█", "▀▀▀"]),
        '4' => Some(["█ █", "▀▀█", "  ▀"]),
        '5' => Some(["█▀▀", "▀▀█", "▀▀▀"]),
        '6' => Some(["█▀▀", "█▀█", "▀▀▀"]),
        '7' => Some(["▀▀█", "  █", "  ▀"]),
        '8' => Some(["█▀█", "█▀█", "▀▀▀"]),
        '9' => Some(["█▀█", "▀▀█", "  ▀"]),
        '.' => Some(["   ", "   ", " ▄ "]),
        '-' => Some(["   ", " ▀ ", "   "]),
        ' ' => Some(["   ", "   ", "   "]),
        _ => Some(["█▀█", "█ █", "▀▀▀"]),
    }
}

/// Official 16-bit SuperFedora website pixel art banner matching superfedora.github.io intro stage (`WM_ROWS`).
/// "SUPER" is positioned in the upper-left directly over the "FEDORA" wordmark.
const SUPERFEDORA_WEBSITE_ART: [&str; 22] = [
    "   █████ ██  ██ █████  ██████ █████",
    "  ██     ██  ██ ██  ██ ██     ██  ██",
    "  █████  ██  ██ ██████ █████  ██████",
    "      ██ ██  ██ ██     ██     ██ ██",
    "      ██ ██  ██ ██     ██     ██  ██",
    "  █████   ████  ██     ██████ ██  ██",
    "",
    "████████████   ████████████   █████████         ████████      █████████           ████",
    "████████████   ████████████   ███████████      ██████████     ███████████       ████████",
    "████████████   ████████████   ████████████    ████████████    ███████████       ████████",
    "███            ███            ███      ███    ███      ███    ███     ███     █████  █████",
    "███            ███            ███      ███    ███      ███    ███     ███     ███      ███",
    "███            ███            ███      ███    ███      ███    ███████████     ███      ███",
    "█████████      █████████      ███      ███    ███      ███    █████████       ███      ███",
    "█████████      █████████      ███      ███    ███      ███    █████████       ████████████",
    "█████████      █████████      ███      ███    ███      ███    ███   ███       ████████████",
    "███            ███            ███      ███    ███      ███    ███     ███     ████████████",
    "███            ███            ███      ███    ███      ███    ███     ███     ███      ███",
    "███            ███            ███      ███    ███      ███    ███     ███     ███      ███",
    "███            ████████████   ████████████    ████████████    ███       ███   ███      ███",
    "███            ████████████   ███████████      ██████████     ███       ███   ███      ███",
    "███            ████████████   █████████         ████████      ███       ███   ███      ███",
];

/// Renders a single word using 3x3 condensed ASCII block patterns.
fn render_word_3x3(word: &str) -> (Vec<String>, usize) {
    let count = word.chars().count();
    let mut lines = vec![String::new(); 3];
    for (ci, ch) in word.chars().enumerate() {
        let pattern = get_3x3(ch).unwrap_or(["   "; 3]);
        for (i, row) in pattern.iter().enumerate() {
            lines[i].push_str(row);
            if ci + 1 < count {
                lines[i].push(' ');
            }
        }
    }
    let width = lines.first().map_or(0, |l| l.chars().count());
    (lines, width)
}

/// Renders the 16-bit SuperFedora stacked pixel art banner ("SUPER" stacked over "FEDORA").
pub fn render_superfedora_banner(
    tier: LayoutTier,
    subtitle: Option<&str>,
    cols: usize,
    rows: usize,
) -> RenderedBanner {
    match tier {
        LayoutTier::Wide => {
            let mut lines: Vec<String> = SUPERFEDORA_WEBSITE_ART
                .iter()
                .map(|&s| s.to_string())
                .collect();

            if let Some(sub) = subtitle {
                let sub_budget = cols.saturating_sub(4);
                let sub_trimmed: String = sub.trim().chars().take(sub_budget).collect();
                if !sub_trimmed.is_empty() {
                    lines.push(String::new());
                    let banner_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
                    let sub_len = sub_trimmed.chars().count();
                    if banner_w > sub_len {
                        let pad = (banner_w - sub_len) / 2;
                        let mut centered = " ".repeat(pad);
                        centered.push_str(&sub_trimmed);
                        lines.push(centered);
                    } else {
                        lines.push(sub_trimmed);
                    }
                }
            }

            let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            let height = lines.len();
            RenderedBanner {
                lines,
                width,
                height,
            }
        }
        LayoutTier::Medium => {
            let (super_lines, _) = render_word_3x3("SUPER");
            let (fedora_lines, _) = render_word_3x3("FEDORA");

            // Left-aligned at upper-left of FEDORA matching website styling
            let mut lines = super_lines;
            lines.push(String::new());
            lines.extend(fedora_lines);

            if let Some(sub) = subtitle {
                let sub_budget = cols.saturating_sub(2);
                let sub_trimmed: String = sub.trim().chars().take(sub_budget).collect();
                if !sub_trimmed.is_empty() {
                    lines.push(String::new());
                    lines.push(sub_trimmed);
                }
            }

            let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            let height = lines.len();
            RenderedBanner {
                lines,
                width,
                height,
            }
        }
        LayoutTier::Narrow => {
            let chars = ["S", "U", "P", "E", "R", " ", "F", "E", "D", "O", "R", "A"];
            let max_v = rows.saturating_sub(4).max(1);
            let mut lines = Vec::new();
            for &c in chars.iter().take(max_v) {
                lines.push(c.to_string());
            }
            let width = 1;
            let height = lines.len();
            RenderedBanner {
                lines,
                width,
                height,
            }
        }
    }
}

/// Renders banner lines using the 3-tier progressive layout.
pub fn render_banner(
    raw_text: &str,
    subtitle: Option<&str>,
    cols: usize,
    rows: usize,
) -> RenderedBanner {
    let tier = LayoutTier::for_dimensions(cols, rows);

    // Normalize input text
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
            if scale == 1 {
                // Budget for characters: each is 6 cols (5 + 1 space). Max budget = (cols - 4) / 6
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
                if let Some(sub) = subtitle {
                    let sub_budget = cols.saturating_sub(4);
                    let sub_trimmed: String = sub.trim().chars().take(sub_budget).collect();
                    if !sub_trimmed.is_empty() {
                        lines.push(String::new());
                        let banner_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
                        let sub_len = sub_trimmed.chars().count();
                        if banner_w > sub_len {
                            let pad = (banner_w - sub_len) / 2;
                            let mut centered = " ".repeat(pad);
                            centered.push_str(&sub_trimmed);
                            lines.push(centered);
                        } else {
                            lines.push(sub_trimmed);
                        }
                    }
                }
                let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
                let height = lines.len();
                RenderedBanner {
                    lines,
                    width,
                    height,
                }
            } else {
                let spacing = match scale {
                    2 => 2,
                    3 => 2,
                    _ => 3,
                };
                let spacing_str = " ".repeat(spacing);
                let mut lines = vec![String::new(); 5 * scale];
                let chars_count = display_text.chars().count();

                for (ci, ch) in display_text.chars().enumerate() {
                    let pattern = get_5x5(ch).unwrap_or(["     "; 5]);
                    let max_w = pattern.iter().map(|r| r.chars().count()).max().unwrap_or(5);

                    for (r, base_row) in pattern.iter().enumerate() {
                        let mut scaled_row = String::with_capacity(max_w * scale);
                        for c in base_row.chars() {
                            match c {
                                '█' | '▀' | '▄' => {
                                    for _ in 0..scale {
                                        scaled_row.push('█');
                                    }
                                }
                                _ => {
                                    for _ in 0..scale {
                                        scaled_row.push(' ');
                                    }
                                }
                            }
                        }
                        let cur_len = scaled_row.chars().count();
                        if cur_len < max_w * scale {
                            for _ in 0..(max_w * scale - cur_len) {
                                scaled_row.push(' ');
                            }
                        }
                        if ci + 1 < chars_count {
                            scaled_row.push_str(&spacing_str);
                        }

                        for sr in 0..scale {
                            lines[r * scale + sr].push_str(&scaled_row);
                        }
                    }
                }

                if let Some(sub) = subtitle {
                    let sub_budget = cols.saturating_sub(4);
                    let sub_trimmed: String = sub.trim().chars().take(sub_budget).collect();
                    if !sub_trimmed.is_empty() {
                        lines.push(String::new());
                        let banner_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
                        let sub_len = sub_trimmed.chars().count();
                        if banner_w > sub_len {
                            let pad = (banner_w - sub_len) / 2;
                            let mut centered = " ".repeat(pad);
                            centered.push_str(&sub_trimmed);
                            lines.push(centered);
                        } else {
                            lines.push(sub_trimmed);
                        }
                    }
                }

                let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
                let height = lines.len();
                RenderedBanner {
                    lines,
                    width,
                    height,
                }
            }
        }
        LayoutTier::Medium => {
            // Budget for 3x3 characters: each is 4 cols (3 + 1 space)
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
            if let Some(sub) = subtitle {
                let sub_budget = cols.saturating_sub(2);
                let sub_trimmed: String = sub.trim().chars().take(sub_budget).collect();
                if !sub_trimmed.is_empty() {
                    lines.push(String::new());
                    lines.push(sub_trimmed);
                }
            }
            let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            let height = lines.len();
            RenderedBanner {
                lines,
                width,
                height,
            }
        }
        LayoutTier::Narrow => {
            // cols < 25: Single glyph or vertical character stack
            let mut lines = Vec::new();
            let max_vertical = rows.saturating_sub(4).max(1);
            for ch in effective_text.trim().chars().take(max_vertical) {
                lines.push(ch.to_string());
            }
            let width = 1;
            let height = lines.len();
            RenderedBanner {
                lines,
                width,
                height,
            }
        }
    }
}

/// Checks if screen is in portrait orientation (rows / cols > 1.4).
pub fn is_portrait(cols: usize, rows: usize) -> bool {
    if cols == 0 {
        return false;
    }
    (rows as f32 / cols as f32) > 1.4
}

/// Generates portrait telemetry lines (CPU, RAM, Kernel, Uptime).
pub fn format_portrait_telemetry(
    telemetry: &SystemTelemetry,
    cols: usize,
) -> (Vec<String>, Vec<String>) {
    let top = vec![
        center_text(&format!("CPU: {:.1}%", telemetry.cpu_percent), cols),
        center_text(&format!("RAM: {:.1}%", telemetry.ram_percent), cols),
    ];

    let hours = telemetry.uptime_secs / 3600;
    let mins = (telemetry.uptime_secs % 3600) / 60;
    let bottom = vec![
        center_text(&telemetry.kernel, cols),
        center_text(&format!("Up: {hours}h {mins}m"), cols),
    ];

    (top, bottom)
}

fn center_text(text: &str, cols: usize) -> String {
    let len = text.chars().count();
    if len >= cols {
        return text.chars().take(cols).collect();
    }
    let pad = (cols - len) / 2;
    let mut s = String::with_capacity(cols);
    for _ in 0..pad {
        s.push(' ');
    }
    s.push_str(text);
    s
}

/// Computes the responsive scale factor (1..=4) for the WideTier banner based on screen dimensions and text length.
pub fn calculate_banner_scale(text: &str, cols: usize, rows: usize) -> usize {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 1;
    }

    if trimmed.eq_ignore_ascii_case("SUPERFEDORA") || trimmed.eq_ignore_ascii_case("SUPER FEDORA") {
        return 1;
    }

    let effective_rows = if is_portrait(cols, rows) {
        rows.saturating_sub(8)
    } else {
        rows
    };

    let count = trimmed.chars().count();

    // Candidates in descending order of scale: 4 (400%), 3 (300%), 2 (200%)
    for &scale in &[4, 3, 2] {
        let min_rows = match scale {
            4 => 40,
            3 => 30,
            2 => 26,
            _ => 10,
        };

        if effective_rows < min_rows {
            continue;
        }

        let spacing = match scale {
            2 => 2,
            3 => 2,
            _ => 3,
        };

        // Calculate total width of trimmed text at this scale
        let mut text_width = 0;
        for (ci, ch) in trimmed.chars().enumerate() {
            let pattern = get_5x5(ch).unwrap_or(["     "; 5]);
            let max_w = pattern.iter().map(|r| r.chars().count()).max().unwrap_or(5);
            text_width += max_w * scale;
            if ci + 1 < count {
                text_width += spacing;
            }
        }

        // Must leave at least 4 columns margin
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
        let display_text: String = trimmed.chars().take(budget).collect();
        (1, display_text)
    }
}
