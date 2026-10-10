// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! RAII terminal guard for raw mode, alternate screen, and autowrap state.

use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::style::ResetColor;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use std::io::{Write, stdout};

pub struct TerminalGuard {
    pub raw_enabled: bool,
    pub alt_enabled: bool,
}

impl TerminalGuard {
    pub fn enter() -> Self {
        let mut out = stdout();
        let raw_enabled = enable_raw_mode().is_ok();
        let alt_enabled = execute!(out, EnterAlternateScreen, Hide, Clear(ClearType::All)).is_ok();
        // Disable autowrap (\x1b[?7l) to prevent unwanted scrolling on wide displays
        let _ = write!(out, "\x1b[?7l");
        let _ = out.flush();
        Self {
            raw_enabled,
            alt_enabled,
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut out = stdout();
        // Re-enable autowrap (\x1b[?7h)
        let _ = write!(out, "\x1b[?7h");
        let _ = execute!(out, Show, ResetColor);
        if self.alt_enabled {
            let _ = execute!(out, LeaveAlternateScreen);
        }
        let _ = out.flush();
        if self.raw_enabled {
            let _ = disable_raw_mode();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_guard_struct() {
        let g = TerminalGuard {
            raw_enabled: false,
            alt_enabled: false,
        };
        assert!(!g.raw_enabled);
        assert!(!g.alt_enabled);
    }
}
