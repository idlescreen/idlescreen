// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Unicode normalization and fullwidth CJK width handling.

use unicode_normalization::UnicodeNormalization;
use unicode_width::UnicodeWidthChar;

/// Normalizes text using Unicode NFKD decomposition, folding diacritics / accents
/// (e.g., "É" -> "E", "ü" -> "u") so they can be matched into block glyphs.
pub fn normalize_to_ascii_fold(text: &str) -> String {
    text.nfkd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect()
}

/// Fallback translation for known non-Latin distribution names to ASCII IDs.
pub fn non_latin_fallback(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    if lower.contains("ред ос") || lower.contains("red os") {
        Some("RED OS")
    } else if lower.contains("альт") || lower.contains("alt") {
        Some("ALT Linux")
    } else if lower.contains("астра") || lower.contains("astra") {
        Some("Astra Linux")
    } else if lower.contains("深度") || lower.contains("deepin") {
        Some("Deepin")
    } else if lower.contains("统信") || lower.contains("uos") {
        Some("UOS")
    } else if lower.contains("麒麟") || lower.contains("kylin") {
        Some("Kylin")
    } else {
        None
    }
}

/// Computes the visual column width of a character string according to unicode-width.
pub fn string_width(s: &str) -> usize {
    s.chars().map(|c| c.width().unwrap_or(1)).sum()
}

/// Structure describing a character placement in a row grid with CJK width awareness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridCellChar {
    /// Regular character occupying 1 column
    Single(char),
    /// Wide character occupying 2 columns (leading cell)
    WideLeading(char),
    /// Trailing continuation cell for wide character (prevents column misalignment)
    WideContinuation,
}

/// Formats text into grid cells, handling fullwidth characters properly.
pub fn place_unicode_str(s: &str, max_cols: usize) -> Vec<GridCellChar> {
    let mut cells = Vec::with_capacity(max_cols);
    for ch in s.chars() {
        let w = ch.width().unwrap_or(1);
        if w >= 2 {
            if cells.len() + 2 > max_cols {
                break;
            }
            cells.push(GridCellChar::WideLeading(ch));
            cells.push(GridCellChar::WideContinuation);
        } else {
            if cells.len() + 1 > max_cols {
                break;
            }
            cells.push(GridCellChar::Single(ch));
        }
    }
    cells
}
