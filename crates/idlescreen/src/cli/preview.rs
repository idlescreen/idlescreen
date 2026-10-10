// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Terminal screensaver preview using crossterm for headless and SSH sessions.

use std::fmt::Write as FmtWrite;
use std::io::{IsTerminal, Write, stdout};
use std::time::{Duration, Instant};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::style::ResetColor;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode, size,
};

use crate::ascii::Ascii;
use crate::ascii::effects::EffectKind;
use crate::cell::TerminalCell;
use crate::daemon::config::Config;
use crate::extras_ipc::ExtrasSession;

/// Parsed options for `idlescreen preview`.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewOptions {
    pub saver: Option<String>,
    pub timeout: Option<Duration>,
}

/// Parses CLI arguments for the preview subcommand.
pub fn parse_preview_args(
    args: &[String],
) -> Result<Option<PreviewOptions>, Box<dyn std::error::Error>> {
    let mut saver = None;
    let mut timeout = None;
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("idlescreen preview — Preview screensavers in terminal");
                println!();
                println!("Usage:");
                println!("  idlescreen preview [saver] [options]");
                println!();
                println!("Options:");
                println!("  -t, --timeout <secs>   Automatically exit after <secs> seconds");
                println!("  -h, --help             Show this help message");
                println!();
                println!("Examples:");
                println!(
                    "  idlescreen preview                 # Preview active configured screensaver"
                );
                println!(
                    "  idlescreen preview storm           # Preview 'storm' extras screensaver"
                );
                println!("  idlescreen preview matrix          # Preview 'matrix' ASCII effect");
                println!("  idlescreen preview --timeout 5     # Run preview for 5 seconds");
                return Ok(None);
            }
            "-t" | "--timeout" => {
                let val = iter
                    .next()
                    .ok_or("preview: --timeout requires duration in seconds")?;
                let secs: f64 = val
                    .parse()
                    .map_err(|_| "preview: invalid duration for --timeout")?;
                timeout = Some(Duration::from_secs_f64(secs.max(0.0)));
            }
            s if s.starts_with("--timeout=") => {
                let val = &s["--timeout=".len()..];
                let secs: f64 = val
                    .parse()
                    .map_err(|_| "preview: invalid duration for --timeout")?;
                timeout = Some(Duration::from_secs_f64(secs.max(0.0)));
            }
            s if !s.starts_with('-') => {
                if saver.is_none() {
                    saver = Some(s.to_string());
                }
            }
            other => {
                return Err(format!("preview: unknown option '{other}'").into());
            }
        }
    }

    Ok(Some(PreviewOptions { saver, timeout }))
}

struct TerminalGuard {
    raw_enabled: bool,
    alt_enabled: bool,
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut out = stdout();
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

pub fn execute(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let Some(opts) = parse_preview_args(args)? else {
        return Ok(());
    };

    let config = Config::load_or_migrate();
    let saver_name = opts
        .saver
        .filter(|s| !s.trim().is_empty())
        .or_else(|| config.saver.active.clone().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| "ascii".to_string());

    let is_interactive = std::io::stdin().is_terminal();
    let timeout = if !is_interactive && opts.timeout.is_none() {
        Some(Duration::from_millis(500))
    } else {
        opts.timeout
    };

    let mut out = stdout();
    let raw_enabled = enable_raw_mode().is_ok();
    let alt_enabled = execute!(out, EnterAlternateScreen, Hide, Clear(ClearType::All)).is_ok();
    let _guard = TerminalGuard {
        raw_enabled,
        alt_enabled,
    };

    run_preview_loop(&mut out, &saver_name, timeout)
}

fn run_preview_loop(
    out: &mut std::io::Stdout,
    saver_name: &str,
    timeout: Option<Duration>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (init_cols, init_rows) = size().unwrap_or((80, 24));
    let mut cols = (init_cols as usize).max(1);
    let mut rows = (init_rows as usize).max(1);

    let effect_kind = EffectKind::parse(saver_name);
    let is_extras_candidate = effect_kind.is_none() && saver_name != "ascii";

    let mut is_extras = is_extras_candidate && ExtrasSession::is_available();
    let mut extras_session = if is_extras {
        ExtrasSession::spawn(saver_name, cols, rows).ok()
    } else {
        None
    };

    if extras_session.is_none() {
        is_extras = false;
    }

    let mut ascii_saver = Ascii::new();
    if let Some(effect) = effect_kind {
        ascii_saver.pin_effect(effect);
    }
    ascii_saver.init(cols, rows);

    let mut grid = vec![TerminalCell::default(); cols * rows];
    let frame_dur = Duration::from_millis(16); // ~60fps
    let start_time = Instant::now();
    let mut last_frame = Instant::now();

    loop {
        if let Some(limit) = timeout
            && start_time.elapsed() >= limit
        {
            break;
        }

        let elapsed = last_frame.elapsed();
        let poll_dur = frame_dur.saturating_sub(elapsed);

        if poll_dur > Duration::from_millis(0) {
            match event::poll(poll_dur) {
                Ok(true) => match event::read() {
                    Ok(Event::Key(key)) => {
                        if key.kind == KeyEventKind::Press {
                            break;
                        }
                    }
                    Ok(Event::Resize(new_cols, new_rows)) => {
                        cols = (new_cols as usize).max(1);
                        rows = (new_rows as usize).max(1);
                        grid.resize(cols * rows, TerminalCell::default());
                        if is_extras {
                            if let Some(ref mut session) = extras_session {
                                let _ = session.init_grid(cols, rows);
                            }
                        } else {
                            ascii_saver.init(cols, rows);
                        }
                    }
                    _ => {}
                },
                Ok(false) => {}
                Err(_) => {
                    std::thread::sleep(poll_dur);
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

            // Draw to terminal with truecolor ANSI
            render_grid_to_terminal(out, &grid, cols, rows)?;
        }
    }

    Ok(())
}

fn render_grid_to_terminal(
    out: &mut std::io::Stdout,
    grid: &[TerminalCell],
    cols: usize,
    rows: usize,
) -> std::io::Result<()> {
    execute!(out, MoveTo(0, 0))?;
    let mut buf = String::with_capacity(cols * rows * 4);

    let mut cur_fg: Option<(u8, u8, u8)> = None;
    let mut cur_bg: Option<(u8, u8, u8)> = None;
    let mut cur_bold: Option<bool> = None;

    for y in 0..rows {
        let row_offset = y * cols;
        for x in 0..cols {
            let idx = row_offset + x;
            if idx < grid.len() {
                let cell = grid[idx];

                if cur_bold != Some(cell.bold) {
                    cur_bold = Some(cell.bold);
                    if cell.bold {
                        buf.push_str("\x1b[1m");
                    } else {
                        buf.push_str("\x1b[22m");
                    }
                }

                if cur_fg != Some(cell.fg) {
                    cur_fg = Some(cell.fg);
                    let (r, g, b) = cell.fg;
                    let _ = write!(buf, "\x1b[38;2;{r};{g};{b}m");
                }

                if cur_bg != Some(cell.bg) {
                    cur_bg = Some(cell.bg);
                    let (r, g, b) = cell.bg;
                    let _ = write!(buf, "\x1b[48;2;{r};{g};{b}m");
                }

                buf.push(cell.ch);
            }
        }
        if y + 1 < rows {
            buf.push('\n');
        }
    }

    buf.push_str("\x1b[0m");
    let _ = write!(out, "{buf}");
    let _ = out.flush();
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_preview_args_empty() {
        let args: Vec<String> = vec![];
        let opts = parse_preview_args(&args).unwrap().unwrap();
        assert_eq!(opts.saver, None);
        assert_eq!(opts.timeout, None);
    }

    #[test]
    fn test_parse_preview_args_saver_only() {
        let args = vec!["storm".to_string()];
        let opts = parse_preview_args(&args).unwrap().unwrap();
        assert_eq!(opts.saver.as_deref(), Some("storm"));
        assert_eq!(opts.timeout, None);
    }

    #[test]
    fn test_parse_preview_args_timeout_flags() {
        let args = vec!["--timeout".to_string(), "3.5".to_string()];
        let opts = parse_preview_args(&args).unwrap().unwrap();
        assert_eq!(opts.timeout, Some(Duration::from_millis(3500)));

        let args_short = vec!["-t".to_string(), "2".to_string()];
        let opts_short = parse_preview_args(&args_short).unwrap().unwrap();
        assert_eq!(opts_short.timeout, Some(Duration::from_secs(2)));

        let args_eq = vec!["--timeout=1.2".to_string()];
        let opts_eq = parse_preview_args(&args_eq).unwrap().unwrap();
        assert_eq!(opts_eq.timeout, Some(Duration::from_millis(1200)));
    }

    #[test]
    fn test_parse_preview_args_saver_and_timeout() {
        let args = vec![
            "matrix".to_string(),
            "--timeout".to_string(),
            "4".to_string(),
        ];
        let opts = parse_preview_args(&args).unwrap().unwrap();
        assert_eq!(opts.saver.as_deref(), Some("matrix"));
        assert_eq!(opts.timeout, Some(Duration::from_secs(4)));

        // Order reversed
        let args_rev = vec![
            "--timeout".to_string(),
            "4".to_string(),
            "aurora".to_string(),
        ];
        let opts_rev = parse_preview_args(&args_rev).unwrap().unwrap();
        assert_eq!(opts_rev.saver.as_deref(), Some("aurora"));
        assert_eq!(opts_rev.timeout, Some(Duration::from_secs(4)));
    }

    #[test]
    fn test_parse_preview_args_invalid_timeout() {
        let args = vec!["--timeout".to_string(), "invalid".to_string()];
        assert!(parse_preview_args(&args).is_err());
    }

    #[test]
    fn test_parse_preview_help_returns_none() {
        let args = vec!["--help".to_string()];
        let res = parse_preview_args(&args).unwrap();
        assert!(res.is_none());
    }

    #[test]
    fn test_headless_preview_completes_cleanly() {
        // Run preview with short timeout; must not panic or block
        let args = vec![
            "ascii".to_string(),
            "--timeout".to_string(),
            "0.05".to_string(),
        ];
        let res = execute(&args);
        assert!(res.is_ok());
    }
}
