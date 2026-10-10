// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Terminal screensaver preview using crossterm for headless and SSH sessions.

use std::io::{Write, stdout};
use std::time::{Duration, Instant};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::style::ResetColor;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode, size,
};

use crate::ascii::Ascii;
use crate::cell::TerminalCell;
use crate::extras_ipc::ExtrasSession;

pub fn execute(saver_arg: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = stdout();
    enable_raw_mode()?;
    execute!(out, EnterAlternateScreen, Hide, Clear(ClearType::All))?;

    let res = run_preview_loop(&mut out, saver_arg);

    let _ = execute!(out, Show, ResetColor, LeaveAlternateScreen);
    let _ = disable_raw_mode();

    res
}

fn run_preview_loop(
    out: &mut std::io::Stdout,
    saver_arg: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let saver_name = saver_arg.unwrap_or("ascii");

    let (init_cols, init_rows) = size().unwrap_or((80, 24));
    let mut cols = init_cols as usize;
    let mut rows = init_rows as usize;

    let mut is_extras = saver_name != "ascii" && ExtrasSession::is_available();
    let mut extras_session = if is_extras {
        ExtrasSession::spawn(saver_name, cols, rows).ok()
    } else {
        None
    };

    if extras_session.is_none() {
        is_extras = false;
    }

    let mut ascii_saver = Ascii::new();
    ascii_saver.init(cols, rows);

    let mut grid = vec![TerminalCell::default(); cols * rows];
    let frame_dur = Duration::from_millis(16); // ~60fps
    let mut last_frame = Instant::now();

    loop {
        // Poll for keyboard input
        if event::poll(Duration::from_millis(5))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q' | 'Q') | KeyCode::Esc => break,
                        _ => {}
                    }
                }
            } else if let Event::Resize(new_cols, new_rows) = event::read()? {
                cols = new_cols as usize;
                rows = new_rows as usize;
                grid.resize(cols * rows, TerminalCell::default());
                if is_extras {
                    if let Some(ref mut session) = extras_session {
                        let _ = session.init_grid(cols, rows);
                    }
                } else {
                    ascii_saver.init(cols, rows);
                }
            }
        }

        let now = Instant::now();
        let dt = now.duration_since(last_frame);
        if dt >= frame_dur {
            last_frame = now;

            // Render next frame
            if is_extras
                && let Some(ref mut session) = extras_session
                && session.tick_and_draw(dt, &mut grid, cols, rows).is_err()
            {
                // Fall back to built-in ASCII
                is_extras = false;
                ascii_saver.init(cols, rows);
            }

            if !is_extras {
                ascii_saver.update(dt, cols, rows);
                ascii_saver.draw(&mut grid, cols, rows);
            }

            // Draw to terminal
            execute!(out, MoveTo(0, 0))?;
            let mut buf_str = String::with_capacity(cols * rows * 2);
            for y in 0..rows {
                let row_offset = y * cols;
                for x in 0..cols {
                    let idx = row_offset + x;
                    if idx < grid.len() {
                        let cell = grid[idx];
                        buf_str.push(cell.ch);
                    }
                }
                if y + 1 < rows {
                    buf_str.push('\n');
                }
            }
            write!(out, "{buf_str}")?;
            out.flush()?;
        }
    }

    Ok(())
}
