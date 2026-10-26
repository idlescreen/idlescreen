// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Minimal JSON string escaping.
//!
//! The daemon and CLI hand-build their JSON payloads, so any value that
//! reaches them from user configuration (a saver name, a raw daemon
//! response) has to be escaped before it is interpolated into a string
//! literal — otherwise one stray quote breaks the whole document.

/// Escapes `s` for use inside a JSON string literal (without quotes).
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}
