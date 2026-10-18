// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Borderless fullscreen Wayland preview runner with interactive scene stepping.

use std::time::{Duration, Instant};

use wayland_client::Connection;

use super::frame_step::{build_ascii_saver, init_surface_buffers, step_and_present_surfaces};
use super::shm_buf::poll_and_read_events;
use super::wl_state::WaylandState;
use super::{PowerThrottle, arcade_grid_dimensions};
use crate::ascii::effects::EffectKind;
use crate::cell::TerminalCell;
use crate::daemon::config::Config;

/// Runs a borderless fullscreen screensaver preview on Wayland.
pub fn run_wayland_fullscreen_preview(
    saver_name: &str,
    mode_str: &str,
    dwell_secs: f32,
    custom_text: &str,
    timeout: Option<Duration>,
    config_fps: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let connection = Connection::connect_to_env()?;
    let mut event_queue = connection.new_event_queue();
    let qh = event_queue.handle();
    let _registry = connection.display().get_registry(&qh, ());

    let mut state = WaylandState::new();
    state.preview_mode = true;
    event_queue.roundtrip(&mut state)?;
    let _ = event_queue.roundtrip(&mut state);

    state.create_presentation_surfaces(&qh)?;

    let configure_start = Instant::now();
    while !state.all_configured() && !state.exit_requested {
        if configure_start.elapsed() >= Duration::from_millis(500) {
            if !state.any_configured() {
                return Err("Wayland surface configure negotiation timed out".into());
            }
            break;
        }
        poll_and_read_events(
            &connection,
            &mut event_queue,
            &mut state,
            Duration::from_millis(50),
        )?;
    }
    if state.exit_requested {
        state.cleanup_presentation(&mut event_queue);
        return Ok(());
    }

    let shm = state.shm.clone().ok_or("wl_shm not available")?;
    init_surface_buffers(&mut state, &shm, &qh)?;

    let cfg = Config::load_or_migrate();
    let (primary_w, primary_h, primary_refresh) =
        state.primary_metrics(Some(cfg.general.primary_output.as_str()));
    let (cols, rows) = arcade_grid_dimensions(primary_w as usize, primary_h as usize);

    let target_fps = if config_fps > 0 {
        config_fps
    } else if primary_refresh > 0 {
        u32::try_from(((primary_refresh + 500) / 1000).max(1)).unwrap_or(60)
    } else {
        60
    };
    let base_frame_dur = Duration::from_nanos(1_000_000_000 / u64::from(target_fps));
    let custom_opt = (!custom_text.trim().is_empty()).then(|| custom_text.to_string());
    let targets = crate::daemon::config::parse_targets_from_mode(mode_str);
    let mut ascii_saver = build_ascii_saver(
        saver_name,
        mode_str,
        dwell_secs,
        cfg.saver.ascii.dissolve_duration_secs,
        custom_opt,
        targets,
        target_fps,
    );
    ascii_saver.init(cols, rows);
    state.tour_mode =
        saver_name.eq_ignore_ascii_case("new") || saver_name.eq_ignore_ascii_case("tour");

    let mut cells = vec![TerminalCell::default(); cols * rows];
    let mut power = PowerThrottle::new();
    let mut last_frame = Instant::now();
    state.start_time = Instant::now();

    while !state.exit_requested {
        if let Some(limit) = timeout
            && state.start_time.elapsed() >= limit
        {
            break;
        }
        let stepped = if state.scene_step_delta != 0 {
            let delta = std::mem::replace(&mut state.scene_step_delta, 0);
            ascii_saver.step_effect(&EffectKind::ALL, delta);
            true
        } else {
            false
        };
        let frame_dur = power.effective_frame_dur(base_frame_dur, ascii_saver.is_settled());
        let now = Instant::now();
        let dt = now.duration_since(last_frame);
        if stepped || dt >= frame_dur {
            last_frame = now;
            step_and_present_surfaces(
                &mut state,
                &shm,
                &qh,
                if stepped { Duration::ZERO } else { dt },
                (cols, rows),
                &mut ascii_saver,
                &mut cells,
            );
            connection.flush()?;
        }
        let sleep_dur = frame_dur.saturating_sub(last_frame.elapsed());
        poll_and_read_events(&connection, &mut event_queue, &mut state, sleep_dur)?;
    }

    drop(cells);
    drop(ascii_saver);
    state.cleanup_presentation(&mut event_queue);
    drop(shm);
    drop(state);
    drop(event_queue);
    drop(connection);
    super::shm_buf::reclaim_presenter_memory();
    Ok(())
}
