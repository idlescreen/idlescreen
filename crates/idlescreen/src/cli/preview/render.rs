// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Terminal render loop and frame dispatch for preview sessions.

use std::fmt::Write as FmtWrite;
use std::io::{IsTerminal, Write, stdout};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event};
use crossterm::terminal::size;

use super::options::parse_preview_args;
use super::term_guard::TerminalGuard;
use crate::ascii::Ascii;
use crate::ascii::effects::EffectKind;
use crate::ascii::state::TextMode;
use crate::cell::TerminalCell;
use crate::daemon::config::Config;
use crate::extras_ipc::ExtrasSession;

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

    let eff_mode = opts.mode.unwrap_or_else(|| config.saver.ascii.mode.clone());
    let eff_dwell = opts.dwell.unwrap_or(config.saver.ascii.dwell_secs);
    let eff_text = opts.text.unwrap_or_else(|| config.saver.ascii.text.clone());

    let is_interactive = std::io::stdin().is_terminal();
    let timeout = if !is_interactive && opts.timeout.is_none() {
        Some(Duration::from_millis(500))
    } else {
        opts.timeout
    };

    let try_wayland =
        !opts.terminal && (opts.fullscreen || std::env::var_os("WAYLAND_DISPLAY").is_some());
    if try_wayland
        && crate::wayland::presenter::run_wayland_fullscreen_preview(
            &saver_name,
            &eff_mode,
            eff_dwell,
            &eff_text,
            timeout,
        )
        .is_ok()
    {
        return Ok(());
    }

    let _guard = TerminalGuard::enter();
    let mut out = stdout();
    run_preview_loop(
        &mut out,
        &saver_name,
        &eff_mode,
        eff_dwell,
        &eff_text,
        timeout,
    )
}

fn run_preview_loop(
    out: &mut std::io::Stdout,
    saver_name: &str,
    mode_str: &str,
    dwell_secs: f32,
    custom_text: &str,
    timeout: Option<Duration>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (raw_cols, raw_rows) = size().unwrap_or((80, 24));
    let mut cols = (raw_cols as usize).max(10);
    let mut rows = (raw_rows as usize).max(5);

    let effect_kind = EffectKind::parse(saver_name);
    let is_extras_cand = effect_kind.is_none() && saver_name != "ascii";
    let mut is_extras = is_extras_cand && ExtrasSession::is_available();
    let mut extras_session = if is_extras {
        ExtrasSession::spawn(saver_name, cols, rows).ok()
    } else {
        None
    };
    if extras_session.is_none() {
        is_extras = false;
    }

    let text_mode = TextMode::parse(mode_str);
    let custom_opt = (!custom_text.trim().is_empty()).then(|| custom_text.to_string());
    let mut ascii_saver = Ascii::new()
        .with_mode(text_mode, custom_opt)
        .with_dwell(dwell_secs);
    if let Some(effect) = effect_kind {
        ascii_saver.pin_effect(effect);
    }
    ascii_saver.init(cols, rows);

    let mut grid = vec![TerminalCell::default(); cols * rows];
    let frame_dur = Duration::from_millis(16);
    let start_time = Instant::now();
    let mut last_frame = Instant::now();

    // Drain pending input events (e.g. Enter key from command invocation)
    while event::poll(Duration::from_millis(0)).unwrap_or(false) {
        let _ = event::read();
    }

    draw_frame(
        frame_dur,
        &mut is_extras,
        &mut extras_session,
        &mut ascii_saver,
        &mut grid,
        (cols, rows),
        out,
    )?;

    loop {
        if let Some(limit) = timeout
            && start_time.elapsed() >= limit
        {
            break;
        }

        let elapsed = last_frame.elapsed();
        let poll_dur = frame_dur.saturating_sub(elapsed);
        let wait_dur = if poll_dur == Duration::ZERO {
            Duration::from_millis(1)
        } else {
            poll_dur
        };

        if event::poll(wait_dur).unwrap_or(false) {
            let mut should_exit = false;
            while let Ok(ev) = event::read() {
                match ev {
                    Event::Key(key) => {
                        if key.kind != event::KeyEventKind::Release {
                            should_exit = true;
                            break;
                        }
                    }
                    Event::Resize(w, h) => {
                        let new_c = (w as usize).max(10);
                        let new_r = (h as usize).max(5);
                        if new_c != cols || new_r != rows {
                            cols = new_c;
                            rows = new_r;
                            grid.resize(cols * rows, TerminalCell::default());
                            grid.fill(TerminalCell::default());
                            if is_extras {
                                if let Some(ref mut session) = extras_session {
                                    let _ = session.init_grid(cols, rows);
                                }
                            } else {
                                ascii_saver.init(cols, rows);
                            }
                        }
                    }
                    _ => {}
                }
                if !event::poll(Duration::from_millis(0)).unwrap_or(false) {
                    break;
                }
            }
            if should_exit {
                break;
            }
        }

        let now = Instant::now();
        let dt = now.duration_since(last_frame);
        if dt >= frame_dur {
            last_frame = now;
            draw_frame(
                dt,
                &mut is_extras,
                &mut extras_session,
                &mut ascii_saver,
                &mut grid,
                (cols, rows),
                out,
            )?;
        }
    }

    Ok(())
}

fn draw_frame(
    dt: Duration,
    is_extras: &mut bool,
    session: &mut Option<ExtrasSession>,
    saver: &mut Ascii,
    grid: &mut [TerminalCell],
    (cols, rows): (usize, usize),
    out: &mut std::io::Stdout,
) -> Result<(), Box<dyn std::error::Error>> {
    if *is_extras
        && let Some(s) = session
        && s.tick_and_draw(dt, grid, cols, rows).is_err()
    {
        *is_extras = false;
        saver.init(cols, rows);
    }
    if !*is_extras {
        saver.update(dt, cols, rows);
        saver.draw(grid, cols, rows);
    }
    render_grid_to_terminal(out, grid, cols, rows)?;
    Ok(())
}

fn render_grid_to_terminal(
    out: &mut std::io::Stdout,
    grid: &[TerminalCell],
    cols: usize,
    rows: usize,
) -> std::io::Result<()> {
    let mut buf = String::with_capacity(cols * rows * 4);
    let mut cur_fg: Option<(u8, u8, u8)> = None;
    let mut cur_bg: Option<(u8, u8, u8)> = None;
    let mut cur_bold: Option<bool> = None;

    for y in 0..rows {
        let _ = write!(buf, "\x1b[{};1H", y + 1);
        let row_offset = y * cols;
        for x in 0..cols {
            let idx = row_offset + x;
            if idx < grid.len() {
                let cell = grid[idx];
                if cur_bold != Some(cell.bold) {
                    cur_bold = Some(cell.bold);
                    buf.push_str(if cell.bold { "\x1b[1m" } else { "\x1b[22m" });
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
    }
    buf.push_str("\x1b[0m");
    let _ = write!(out, "{buf}");
    out.flush()
}
