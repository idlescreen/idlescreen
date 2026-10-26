// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Keyboard escape-sequence decoding for the preview terminal.

use super::{KeyCode, KeyEvent, KeyModifiers};

pub fn consume_one_key(b: &[u8]) -> (KeyEvent, usize) {
    if b.starts_with(b"\x1b[1;2C") {
        return (KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT), 6);
    }
    if b.starts_with(b"\x1b[1;2D") {
        return (KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT), 6);
    }
    if let Some(&[0x1b, b'[', d, b'~']) = b.get(..4) {
        let code = match d {
            b'1' | b'7' => Some(KeyCode::Home),
            b'4' | b'8' => Some(KeyCode::End),
            b'3' => Some(KeyCode::Delete),
            b'5' => Some(KeyCode::PageUp),
            b'6' => Some(KeyCode::PageDown),
            _ => None,
        };
        if let Some(c) = code {
            return (KeyEvent::new(c, KeyModifiers::NONE), 4);
        }
    }
    if let Some(&[0x1b, p @ (b'[' | b'O'), k]) = b.get(..3) {
        let map = match k {
            b'A' => Some((KeyCode::Up, KeyModifiers::NONE)),
            b'B' => Some((KeyCode::Down, KeyModifiers::NONE)),
            b'C' => Some((KeyCode::Right, KeyModifiers::NONE)),
            b'D' => Some((KeyCode::Left, KeyModifiers::NONE)),
            b'H' => Some((KeyCode::Home, KeyModifiers::NONE)),
            b'F' => Some((KeyCode::End, KeyModifiers::NONE)),
            b'Z' if p == b'[' => Some((KeyCode::BackTab, KeyModifiers::SHIFT)),
            _ => None,
        };
        if let Some((c, m)) = map {
            return (KeyEvent::new(c, m), 3);
        }
    }
    if b.starts_with(b"\x1b[") {
        let end = b[2..]
            .iter()
            .position(|&x| (0x40..=0x7E).contains(&x))
            .map_or(b.len(), |i| 3 + i);
        return (KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), end);
    }
    if let Some(&[0x1b, ch @ 0x20..=0x7E]) = b.get(..2) {
        let ev = KeyEvent::new(KeyCode::Char(ch as char), KeyModifiers::ALT);
        return (ev, 2);
    }
    match b.first().copied().unwrap_or(0x1b) {
        0x1b => (KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1),
        b'\r' | b'\n' => (KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 1),
        b'\t' => (KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), 1),
        0x7f | 0x08 => (KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE), 1),
        c @ 1..=26 => (
            KeyEvent::new(KeyCode::Char((c - 1 + b'a') as char), KeyModifiers::CONTROL),
            1,
        ),
        _ => {
            for len in 1..=b.len().min(4) {
                if let Ok(s) = std::str::from_utf8(&b[..len])
                    && let Some(ch) = s.chars().next()
                {
                    let m = if ch.is_ascii_uppercase() {
                        KeyModifiers::SHIFT
                    } else {
                        KeyModifiers::NONE
                    };
                    return (KeyEvent::new(KeyCode::Char(ch), m), ch.len_utf8());
                }
            }
            (KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1)
        }
    }
}

pub fn parse_key_bytes(b: &[u8]) -> KeyEvent {
    consume_one_key(b).0
}
