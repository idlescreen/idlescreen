// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Zero-dependency Unicode diacritic folding and East Asian / CJK display width handling.

/// Folds precomposed Latin characters with diacritics/accents to their ASCII base
/// character, and strips combining diacritical marks (`U+0300..=U+036F`).
pub fn normalize_to_ascii_fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if is_combining_mark(ch) {
            continue;
        }
        if ch == '\u{3000}' {
            out.push(' ');
            continue;
        }
        if let Some(u) = (ch as u32)
            .checked_sub(0xFF01)
            .filter(|&d| d <= 0x5D)
            .and_then(|d| char::from_u32(d + 0x21))
        {
            out.push(u);
            continue;
        }
        match fold_latin_diacritic(ch) {
            Some(ascii) => out.push_str(ascii),
            None => out.push(ch),
        }
    }
    out
}

#[inline]
const fn is_combining_mark(c: char) -> bool {
    matches!(
        c as u32,
        0x0300..=0x036F
            | 0x0591..=0x05C7
            | 0x064B..=0x065F
            | 0x1AB0..=0x1AFF
            | 0x1DC0..=0x1DFF
            | 0x20D0..=0x20FF
            | 0xFE20..=0xFE2F
    )
}

#[rustfmt::skip]
const fn fold_latin_diacritic(c: char) -> Option<&'static str> {
    match c {
        'À'..='Å' | 'Ā' | 'Ă' | 'Ą' | 'Ǎ' | 'Ạ' | 'Ả' | 'Ấ' | 'Ầ' | 'Ẩ' | 'Ẫ' | 'Ậ' | 'Ắ' | 'Ằ' | 'Ẳ' | 'Ẵ' | 'Ặ' => Some("A"),
        'à'..='å' | 'ā' | 'ă' | 'ą' | 'ǎ' | 'ạ' | 'ả' | 'ấ' | 'ầ' | 'ẩ' | 'ẫ' | 'ậ' | 'ắ' | 'ằ' | 'ẳ' | 'ẵ' | 'ặ' => Some("a"),
        'Æ' => Some("AE"), 'æ' => Some("ae"), 'Œ' => Some("OE"), 'œ' => Some("oe"), 'ß' => Some("ss"), 'ẞ' => Some("SS"),
        'Ç' | 'Ć' | 'Ĉ' | 'Ċ' | 'Č' => Some("C"),
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => Some("c"),
        'Ď' | 'Đ' | 'Ð' => Some("D"),
        'ď' | 'đ' | 'ð' => Some("d"),
        'È'..='Ë' | 'Ē' | 'Ĕ' | 'Ė' | 'Ę' | 'Ě' | 'Ẹ' | 'Ẻ' | 'Ẽ' | 'Ế' | 'Ề' | 'Ể' | 'Ễ' | 'Ệ' => Some("E"),
        'è'..='ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' | 'ẹ' | 'ẻ' | 'ẽ' | 'ế' | 'ề' | 'ể' | 'ễ' | 'ệ' => Some("e"),
        'Ĝ' | 'Ğ' | 'Ġ' | 'Ģ' => Some("G"),
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => Some("g"),
        'Ĥ' | 'Ħ' => Some("H"),
        'ĥ' | 'ħ' => Some("h"),
        'Ì'..='Ï' | 'Ĩ' | 'Ī' | 'Ĭ' | 'Į' | 'İ' | 'Ǐ' | 'Ỉ' | 'Ị' => Some("I"),
        'ì'..='ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' | 'ǐ' | 'ỉ' | 'ị' => Some("i"),
        'Ĵ' => Some("J"), 'ĵ' => Some("j"),
        'Ķ' => Some("K"), 'ķ' | 'ĸ' => Some("k"),
        'Ĺ' | 'Ļ' | 'Ľ' | 'Ŀ' | 'Ł' => Some("L"),
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => Some("l"),
        'Ñ' | 'Ń' | 'Ņ' | 'Ň' | 'Ŋ' => Some("N"),
        'ñ' | 'ń' | 'ņ' | 'ň' | 'ŋ' => Some("n"),
        'Ò'..='Ö' | 'Ø' | 'Ō' | 'Ŏ' | 'Ő' | 'Ǒ' | 'Ọ' | 'Ỏ' | 'Ố' | 'Ồ' | 'Ổ' | 'Ỗ' | 'Ộ' | 'Ớ' | 'Ờ' | 'Ở' | 'Ỡ' | 'Ợ' | 'Ơ' => Some("O"),
        'ò'..='ö' | 'ø' | 'ō' | 'ŏ' | 'ő' | 'ǒ' | 'ọ' | 'ỏ' | 'ố' | 'ồ' | 'ổ' | 'ỗ' | 'ộ' | 'ớ' | 'ờ' | 'ở' | 'ỡ' | 'ợ' | 'ơ' => Some("o"),
        'Ŕ' | 'Ŗ' | 'Ř' => Some("R"), 'ŕ' | 'ŗ' | 'ř' => Some("r"),
        'Ś' | 'Ŝ' | 'Ş' | 'Š' | 'Ș' => Some("S"),
        'ś' | 'ŝ' | 'ş' | 'š' | 'ș' => Some("s"),
        'Ţ' | 'Ť' | 'Ŧ' | 'Ț' => Some("T"), 'ţ' | 'ť' | 'ŧ' | 'ț' => Some("t"),
        'Ù'..='Ü' | 'Ũ' | 'Ū' | 'Ŭ' | 'Ů' | 'Ű' | 'Ų' | 'Ǔ' | 'Ụ' | 'Ủ' | 'Ứ' | 'Ừ' | 'Ử' | 'Ữ' | 'Ự' | 'Ư' => Some("U"),
        'ù'..='ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' | 'ǔ' | 'ụ' | 'ủ' | 'ứ' | 'ừ' | 'ử' | 'ữ' | 'ự' | 'ư' => Some("u"),
        'Ŵ' => Some("W"), 'ŵ' => Some("w"),
        'Ý' | 'Ŷ' | 'Ÿ' | 'Ỳ' | 'Ỵ' | 'Ỷ' | 'Ỹ' => Some("Y"),
        'ý' | 'ÿ' | 'ŷ' | 'ỳ' | 'ỵ' | 'ỷ' | 'ỹ' => Some("y"),
        'Ź' | 'Ż' | 'Ž' => Some("Z"), 'ź' | 'ż' | 'ž' => Some("z"),
        _ => None,
    }
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

/// Computes the visual terminal column width of a single Unicode scalar value (`0`, `1`, or `2`).
pub const fn char_width(c: char) -> usize {
    let u = c as u32;
    if u == 0
        || u < 0x20
        || (u >= 0x7F && u < 0xA0)
        || (u >= 0x1160 && u <= 0x11FF)
        || (u >= 0x200B && u <= 0x200F)
        || (u >= 0xFE00 && u <= 0xFE0F)
        || u == 0xFEFF
        || is_combining_mark(c)
    {
        return 0;
    }
    if matches!(
        u,
        0x1100..=0x115F
            | 0x231A..=0x231B
            | 0x2329..=0x232A
            | 0x23E9..=0x23FA
            | 0x2E80..=0x303E
            | 0x3040..=0x318F
            | 0x31A0..=0x3247
            | 0x3250..=0x4DBF
            | 0x4E00..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F1E6..=0x1F1FF
            | 0x1F300..=0x1F6FF
            | 0x1F900..=0x1FAFF
            | 0x20000..=0x3FFFD
    ) {
        2
    } else {
        1
    }
}

/// Computes the visual column width of a character string.
pub fn string_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
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
        let w = char_width(ch).max(1);
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
