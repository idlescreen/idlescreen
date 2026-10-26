// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! RAII terminal guard, raw mode termios control, window size, and buffered ANSI key polling.

#![allow(unsafe_code)]

use std::io::{Write, stdin, stdout};
use std::os::fd::AsRawFd;
use std::sync::Mutex;
use std::time::Duration;

#[rustfmt::skip]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCode {
    Backspace, Enter, Left, Right, Up, Down, Home, End,
    PageUp, PageDown, Tab, BackTab, Delete, Char(char), Esc,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyModifiers(u8);

#[rustfmt::skip]
impl KeyModifiers {
    pub const NONE: Self = Self(0);
    pub const SHIFT: Self = Self(1);
    pub const CONTROL: Self = Self(2);
    pub const ALT: Self = Self(4);
    #[inline] pub const fn contains(self, other: Self) -> bool { (self.0 & other.0) == other.0 }
}

#[rustfmt::skip]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent { pub code: KeyCode, pub modifiers: KeyModifiers }

#[rustfmt::skip]
impl KeyEvent {
    #[inline] pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self { Self { code, modifiers } }
}

#[rustfmt::skip]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TermEvent { Key(KeyEvent), Resize(u16, u16) }

static PENDING_BYTES: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Latched once stdin reaches EOF. Sources such as /dev/null are always
/// reported as readable by poll() while read() keeps returning 0 bytes, so
/// without this latch every poll would look like pending input forever.
static STDIN_EOF: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn terminal_size() -> Option<(u16, u16)> {
    for fd in [stdout().as_raw_fd(), stdin().as_raw_fd(), 2] {
        // SAFETY: `ws` is zero-initialized and passed to `ioctl(TIOCGWINSZ)`.
        unsafe {
            let mut ws: libc::winsize = std::mem::zeroed();
            if libc::ioctl(fd, libc::TIOCGWINSZ, &raw mut ws) == 0 && ws.ws_col > 0 && ws.ws_row > 0
            {
                return Some((ws.ws_col, ws.ws_row));
            }
        }
    }
    None
}

#[rustfmt::skip]
fn poll_stdin_ms(ms: i32) -> bool {
    let mut pfd = libc::pollfd { fd: libc::STDIN_FILENO, events: libc::POLLIN, revents: 0 };
    // SAFETY: `pfd` is a valid single-element `pollfd` array watching `STDIN_FILENO`.
    unsafe { libc::poll(&raw mut pfd, 1, ms) > 0 && (pfd.revents & libc::POLLIN) != 0 }
}

pub fn poll_input(timeout: Duration) -> bool {
    if STDIN_EOF.load(std::sync::atomic::Ordering::Relaxed) {
        return false;
    }
    PENDING_BYTES.lock().is_ok_and(|q| !q.is_empty())
        || poll_stdin_ms(i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX))
}

fn read_stdin(dst: &mut [u8]) -> isize {
    // SAFETY: `dst` is a valid mutable byte slice passed to `libc::read` on `STDIN_FILENO`.
    let n = unsafe { libc::read(libc::STDIN_FILENO, dst.as_mut_ptr().cast(), dst.len()) };
    if n == 0 {
        // A genuine end-of-file: latch it so poll() no longer reports the
        // always-readable descriptor as pending input.
        STDIN_EOF.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    n
}

pub fn read_event() -> std::io::Result<TermEvent> {
    if PENDING_BYTES.lock().map_or(true, |q| q.is_empty()) {
        let mut buf = [0u8; 64];
        let n = read_stdin(&mut buf);
        if n <= 0 {
            return Err(std::io::Error::from_raw_os_error(libc::EIO));
        }
        let mut len = n as usize;
        if buf[0] == 0x1b && len < 6 && poll_stdin_ms(5) {
            let n2 = read_stdin(&mut buf[len..]);
            if n2 > 0 {
                len += n2 as usize;
            }
        }
        if let Ok(mut g) = PENDING_BYTES.lock() {
            g.extend_from_slice(&buf[..len]);
        }
    }
    let mut g = PENDING_BYTES
        .lock()
        .map_err(|_| std::io::Error::other("input lock"))?;
    let (ev, consumed) = consume_one_key(&g);
    let n_drain = consumed.min(g.len());
    g.drain(..n_drain);
    Ok(TermEvent::Key(ev))
}

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

pub struct TerminalGuard {
    pub raw_enabled: bool,
    pub alt_enabled: bool,
    saved_termios: Option<libc::termios>,
}

impl TerminalGuard {
    pub fn enter() -> Self {
        use std::io::IsTerminal;
        let (mut out, saved_termios) = (stdout(), enable_raw_termios());
        let (raw_enabled, alt_enabled) = (
            saved_termios.is_some(),
            out.is_terminal() && write!(out, "\x1b[?1049h\x1b[?25l\x1b[2J\x1b[?7l").is_ok(),
        );
        let _ = out.flush();
        Self {
            raw_enabled,
            alt_enabled,
            saved_termios,
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.alt_enabled {
            let mut out = stdout();
            let _ = write!(out, "\x1b[?7h\x1b[?25h\x1b[0m\x1b[?1049l");
            let _ = out.flush();
        }
        if let Some(ref orig) = self.saved_termios {
            // SAFETY: Restores the original termios struct captured on `STDIN_FILENO` in `enter()`.
            unsafe {
                let _ = libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw const *orig);
            }
        }
    }
}

fn enable_raw_termios() -> Option<libc::termios> {
    // SAFETY: Zero-initializing `libc::termios` and reading via `tcgetattr` on `STDIN_FILENO`.
    unsafe {
        let mut orig: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(libc::STDIN_FILENO, &raw mut orig) != 0 {
            return None;
        }
        let mut raw = orig;
        libc::cfmakeraw(&raw mut raw);
        raw.c_cc[libc::VMIN] = 1;
        raw.c_cc[libc::VTIME] = 0;
        (libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw const raw) == 0).then_some(orig)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_guard_and_buffered_key_parser() {
        let g = TerminalGuard {
            raw_enabled: false,
            alt_enabled: false,
            saved_termios: None,
        };
        assert!(!g.raw_enabled && !g.alt_enabled);
        assert_eq!(consume_one_key(b"\x1b[A\x1b[B").0.code, KeyCode::Up);
        assert_eq!(consume_one_key(b"\x1b[<0;10;20Mq").1, 11);
        assert_eq!(consume_one_key(b"\x1bx").0.modifiers, KeyModifiers::ALT);
    }
}
