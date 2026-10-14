// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Terminal render loop and Wayland preview dispatcher.

use std::io::{IsTerminal, stdout};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event};
use crossterm::terminal::size;

use super::ansi_out::draw_frame;
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

    if saver_name == "superfedora" && !crate::ascii::sys_info::is_superfedora_machine() {
        eprintln!("error: screensaver 'superfedora' is locked.");
        eprintln!(
            "It only unlocks when IdleScreen is installed and run on a SuperFedora workstation."
        );
        eprintln!("Visit https://superfedora.github.io for installation instructions.");
        return Err("screensaver 'superfedora' is locked on this machine".into());
    }

    let eff_mode = opts.mode.unwrap_or_else(|| {
        if config.saver.ascii.targets.is_empty() {
            config.saver.ascii.mode.clone()
        } else {
            config.saver.ascii.targets.join(",")
        }
    });
    let eff_dwell = opts.dwell.unwrap_or(config.saver.ascii.dwell_secs);
    let eff_dissolve = config.saver.ascii.dissolve_duration_secs;
    let eff_text = opts.text.unwrap_or_else(|| config.saver.ascii.text.clone());
    let eff_fps = config.saver.ascii.fps;

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
            eff_fps,
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
        (eff_dwell, eff_dissolve),
        &eff_text,
        timeout,
        eff_fps,
    )
}

fn run_preview_loop(
    out: &mut std::io::Stdout,
    saver_name: &str,
    mode_str: &str,
    (dwell_secs, dissolve_secs): (f32, f32),
    custom_text: &str,
    timeout: Option<Duration>,
    fps: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let (raw_cols, raw_rows) = size().unwrap_or((80, 24));
    let mut cols = (raw_cols as usize).max(10);
    let mut rows = (raw_rows as usize).max(5);

    let effect_kind = EffectKind::parse(saver_name);
    let is_superfedora =
        saver_name == "superfedora" || mode_str.eq_ignore_ascii_case("superfedora");
    if saver_name == "superfedora" && !crate::ascii::sys_info::is_superfedora_machine() {
        return Err("screensaver 'superfedora' is locked on this machine".into());
    }

    let is_extras_cand = effect_kind.is_none() && saver_name != "ascii" && !is_superfedora;
    let mut is_extras = is_extras_cand && ExtrasSession::is_available();
    let mut extras_session = is_extras
        .then(|| ExtrasSession::spawn(saver_name, cols, rows).ok())
        .flatten();
    if extras_session.is_none() {
        is_extras = false;
    }

    let target_fps = if fps > 0 { fps } else { 60 };
    let text_mode = TextMode::parse(mode_str);
    let custom_opt = (!custom_text.trim().is_empty()).then(|| custom_text.to_string());
    let targets = crate::daemon::config::parse_targets_from_mode(mode_str);
    let use_superfedora =
        is_superfedora && crate::ascii::sys_info::is_superfedora_machine() && custom_opt.is_none();
    let mut ascii_saver = if use_superfedora {
        let mut saver = Ascii::new_superfedora()
            .with_dwell(dwell_secs)
            .with_dissolve(dissolve_secs)
            .with_fps(target_fps);
        if let Some(effect) = effect_kind {
            saver.pin_effect(effect);
        }
        saver
    } else {
        let mut saver = Ascii::new()
            .with_mode(text_mode, custom_opt)
            .with_targets(targets)
            .with_dwell(dwell_secs)
            .with_dissolve(dissolve_secs)
            .with_fps(target_fps);
        if let Some(effect) = effect_kind {
            saver.pin_effect(effect);
        }
        saver
    };
    ascii_saver.init(cols, rows);

    let mut grid = vec![TerminalCell::default(); cols * rows];
    let frame_dur = Duration::from_nanos(1_000_000_000 / u64::from(target_fps));
    let start_time = Instant::now();
    let mut last_frame = Instant::now();

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

        let poll_dur = frame_dur.saturating_sub(last_frame.elapsed());
        let wait_dur = poll_dur.max(Duration::from_millis(1));

        if event::poll(wait_dur).unwrap_or(false) {
            let mut should_exit = false;
            while let Ok(ev) = event::read() {
                match ev {
                    Event::Key(key) if key.kind != event::KeyEventKind::Release => {
                        should_exit = true;
                        break;
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
