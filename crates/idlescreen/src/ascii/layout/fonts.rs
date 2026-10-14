// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 5x5 and 3x3 block font glyph tables and SuperFedora stacked website banner art.

use super::{LayoutTier, RenderedBanner};

pub fn get_5x5(ch: char) -> Option<[&'static str; 5]> {
    match ch.to_ascii_uppercase() {
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

pub fn get_3x3(ch: char) -> Option<[&'static str; 3]> {
    match ch.to_ascii_uppercase() {
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
        'M' | 'N' => Some(["█▀█", "█ █", "▀ ▀"]),
        'O' => Some(["█▀█", "█ █", "▀▀▀"]),
        'P' => Some(["█▀█", "█▀▀", "▀  "]),
        'Q' => Some(["█▀█", "█▄█", "▀▀█"]),
        'R' => Some(["█▀█", "█▀▄", "▀ ▀"]),
        'S' | '5' => Some(["█▀▀", "▀▀█", "▀▀▀"]),
        'T' => Some(["▀█▀", " █ ", " ▀ "]),
        'U' | '0' => Some(["█ █", "█ █", "▀▀▀"]),
        'V' => Some(["█ █", "█ █", " ▀ "]),
        'W' => Some(["█ █", "█▀█", "▀ ▀"]),
        'X' => Some(["▀▄▀", " █ ", "▀ ▀"]),
        'Y' => Some(["█ █", " █ ", " ▀ "]),
        'Z' => Some(["▀▀█", " █ ", "█▀▀"]),
        '1' => Some([" █ ", " █ ", " ▀ "]),
        '2' => Some(["▀▀█", "█▀▀", "▀▀▀"]),
        '3' => Some(["▀▀█", " ▀█", "▀▀▀"]),
        '4' => Some(["█ █", "▀▀█", "  ▀"]),
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

pub fn render_superfedora_banner(
    tier: LayoutTier,
    subtitle: Option<&str>,
    cols: usize,
    rows: usize,
) -> RenderedBanner {
    let wide_min_rows = if subtitle.is_some() { 24 } else { 22 };
    let med_min_rows = if subtitle.is_some() { 9 } else { 7 };

    let effective_tier = match tier {
        LayoutTier::Wide if cols >= 90 && rows >= wide_min_rows => LayoutTier::Wide,
        LayoutTier::Wide | LayoutTier::Medium if cols >= 23 && rows >= med_min_rows => {
            LayoutTier::Medium
        }
        _ => LayoutTier::Narrow,
    };

    match effective_tier {
        LayoutTier::Wide => {
            let mut lines: Vec<String> = SUPERFEDORA_WEBSITE_ART
                .iter()
                .map(|&s| s.to_string())
                .collect();
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
            let (mut lines, _) = render_word_3x3("SUPER");
            let (fedora_lines, _) = render_word_3x3("FEDORA");
            lines.push(String::new());
            lines.extend(fedora_lines);
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
            let chars = ["S", "U", "P", "E", "R", " ", "F", "E", "D", "O", "R", "A"];
            let max_v = rows.saturating_sub(4).max(1);
            let lines: Vec<String> = chars.iter().take(max_v).map(|&c| c.to_string()).collect();
            let height = lines.len();
            RenderedBanner {
                lines,
                width: 1,
                height,
            }
        }
    }
}

pub(crate) fn append_subtitle(
    lines: &mut Vec<String>,
    subtitle: Option<&str>,
    budget: usize,
    center: bool,
) {
    if let Some(sub) = subtitle {
        let sub_trimmed: String = sub.trim().chars().take(budget).collect();
        if !sub_trimmed.is_empty() {
            lines.push(String::new());
            if center {
                let banner_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
                let sub_len = sub_trimmed.chars().count();
                if banner_w > sub_len {
                    let pad = (banner_w - sub_len) / 2;
                    let mut centered = " ".repeat(pad);
                    centered.push_str(&sub_trimmed);
                    lines.push(centered);
                    return;
                }
            }
            lines.push(sub_trimmed);
        }
    }
}
