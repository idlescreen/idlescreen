// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 5x5 block glyph logo rendering and centering helpers.

use super::{get_primary_monitor_bounds, is_secondary_monitor};

#[derive(Debug, Clone)]
pub struct CenteredLogo {
    pub lines: Vec<String>,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

fn get_5x5_pattern(ch: char) -> Option<[&'static str; 5]> {
    let u = ch.to_ascii_uppercase();
    match u {
        'A' => Some([" █▀▀█ ", "█   █", "█████", "█   █", "█   █"]),
        'B' => Some(["████  ", "█   █ ", "████  ", "█   █ ", "████  "]),
        'C' => Some([" ████", "█    ", "█    ", "█    ", " ████"]),
        'D' => Some(["████  ", "█   █ ", "█   █ ", "█   █ ", "████  "]),
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
        '5' => Some(["█████", "█    ", "████ ", "    █", "████"]),
        '6' => Some([" ███ ", "█    ", "████ ", "█   █", " ███ "]),
        '7' => Some(["█████", "    █", "   █ ", "  █  ", "  █  "]),
        '8' => Some([" ███ ", "█   █", " ███ ", "█   █", " ███ "]),
        '9' => Some([" ███ ", "█   █", " ████", "    █", " ███ "]),
        '_' => Some(["     ", "     ", "     ", "     ", "█████"]),
        '!' => Some(["  █  ", "  █  ", "  █  ", "     ", "  █  "]),
        ' ' => Some(["     ", "     ", "     ", "     ", "     "]),
        '.' => Some(["     ", "     ", "     ", "     ", "  █  "]),
        '-' => Some(["     ", "     ", " ███ ", "     ", "     "]),
        _ => Some([" ███ ", "█   █", "█   █", "█   █", " ███ "]),
    }
}

pub fn render_logo_block(text: &str, sub_text: Option<&str>) -> Vec<String> {
    let mut rows: Vec<String> = vec![String::new(); 5];
    for ch in text.chars() {
        let pattern = get_5x5_pattern(ch).unwrap_or(["     "; 5]);
        for (i, line) in pattern.iter().enumerate() {
            rows[i].push_str(line);
            rows[i].push(' ');
        }
    }
    if let Some(sub) = sub_text {
        rows.push(String::new());
        rows.push(sub.to_string());
    }
    rows
}

pub fn place_centered_logo(
    cols: usize,
    rows: usize,
    text: &str,
    sub_text: Option<&str>,
) -> Option<CenteredLogo> {
    if is_secondary_monitor() {
        return None;
    }
    let primary = get_primary_monitor_bounds(cols, rows);
    let budget = primary.width() / 6;
    if budget == 0 {
        return None;
    }
    let trimmed: String = text.trim().chars().take(budget).collect();
    let trimmed_sub = sub_text.map(|t| t.trim().chars().take(budget).collect::<String>());
    let lines = render_logo_block(&trimmed, trimmed_sub.as_deref());
    let logo_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let logo_h = lines.len();
    if logo_w == 0 || logo_h == 0 {
        return None;
    }
    let x = primary.start_col + primary.width().saturating_sub(logo_w) / 2;
    let y = primary.start_row + primary.height().saturating_sub(logo_h) / 2;
    Some(CenteredLogo {
        lines,
        x,
        y,
        width: logo_w,
        height: logo_h,
    })
}
