// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Fullscreen Wayland daemon presenter loop with live D-Bus/socket config updates and DPMS parking.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use wayland_client::Connection;

use super::frame_step::{build_ascii_saver, init_surface_buffers, step_and_present_surfaces};
use super::shm_buf::poll_and_read_events;
use super::wl_state::WaylandState;
use super::{PowerThrottle, arcade_grid_dimensions};
use crate::ascii::effects::EffectKind;
use crate::ascii::state::TextMode;
use crate::cell::TerminalCell;
use crate::daemon::config::Config;
use crate::extras_ipc::ExtrasSession;

/// Runs fullscreen Wayland screensaver presentation for the background daemon while active.
#[allow(clippy::needless_pass_by_value)]
pub fn run_wayland_daemon_presenter(
    state: Arc<crate::daemon::dbus::ScreenSaverState>,
    dwell_secs: f32,
    mode_str: &str,
    custom_text: &str,
    config_fps: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let connection = Connection::connect_to_env()?;
    let mut event_queue = connection.new_event_queue();
    let qh = event_queue.handle();
    let _registry = connection.display().get_registry(&qh, ());

    let mut state_wl = WaylandState::new();
    event_queue.roundtrip(&mut state_wl)?;
    event_queue.roundtrip(&mut state_wl)?;

    state_wl.create_presentation_surfaces(&qh)?;

    let configure_start = Instant::now();
    while !state_wl.all_configured()
        && !state_wl.exit_requested
        && state.active.load(Ordering::SeqCst)
    {
        if configure_start.elapsed() >= Duration::from_millis(500) {
            if !state_wl.any_configured() {
                return Err("Wayland surface configure negotiation timed out".into());
            }
            break;
        }
        poll_and_read_events(
            &connection,
            &mut event_queue,
            &mut state_wl,
            Duration::from_millis(50),
        )?;
    }

    if state_wl.exit_requested || !state.active.load(Ordering::SeqCst) {
        if state_wl.exit_requested {
            state.stop();
        }
        state_wl.cleanup_presentation(&mut event_queue);
        return Ok(());
    }

    let shm = state_wl.shm.clone().ok_or("wl_shm not available")?;
    init_surface_buffers(&mut state_wl, &shm, &qh)?;

    let cfg = Config::load_or_migrate();
    let (primary_w, primary_h, primary_refresh) =
        state_wl.primary_metrics(Some(cfg.general.primary_output.as_str()));
    let (cols, rows) = arcade_grid_dimensions(primary_w as usize, primary_h as usize);

    let saver_name = state
        .current_saver
        .lock()
        .map_or_else(|_| "ascii".to_string(), |s| s.clone());
    let is_superfedora =
        saver_name == "superfedora" || mode_str.eq_ignore_ascii_case("superfedora");
    let is_extras_cand =
        EffectKind::parse(&saver_name).is_none() && saver_name != "ascii" && !is_superfedora;
    let mut is_extras = is_extras_cand && ExtrasSession::is_available();
    let mut extras_session = is_extras
        .then(|| ExtrasSession::spawn(&saver_name, cols, rows).ok())
        .flatten();
    if extras_session.is_none() {
        is_extras = false;
    }

    let target_fps = if config_fps > 0 {
        config_fps
    } else if primary_refresh > 0 {
        u32::try_from(((primary_refresh + 500) / 1000).max(1)).unwrap_or(60)
    } else {
        60
    };
    let base_frame_dur = Duration::from_nanos(1_000_000_000 / u64::from(target_fps));

    let custom_opt = if let Ok(guard) = state.custom_text.lock() {
        (!guard.trim().is_empty()).then(|| guard.clone())
    } else {
        (!custom_text.trim().is_empty()).then(|| custom_text.to_string())
    };
    let targets = if let Ok(guard) = state.text_targets.lock()
        && !guard.is_empty()
    {
        guard.clone()
    } else {
        crate::daemon::config::parse_targets_from_mode(mode_str)
    };
    let mut ascii_saver = build_ascii_saver(
        &saver_name,
        mode_str,
        dwell_secs,
        cfg.saver.ascii.dissolve_duration_secs,
        custom_opt,
        targets,
        target_fps,
    );
    ascii_saver.init(cols, rows);

    let mut cells = vec![TerminalCell::default(); cols * rows];
    let mut power = PowerThrottle::new();
    let mut last_frame = Instant::now();
    let mut last_custom_epoch = state.custom_text_epoch.load(Ordering::Acquire);
    state_wl.start_time = Instant::now();

    while state.active.load(Ordering::SeqCst) && !state_wl.exit_requested {
        if let Ok(guard) = state.text_targets.try_lock()
            && *guard != ascii_saver.rotation.targets
            && !guard.is_empty()
        {
            ascii_saver.rotation.set_targets(guard.clone());
        }
        let cur_epoch = state.custom_text_epoch.load(Ordering::Acquire);
        if cur_epoch != last_custom_epoch
            && let Ok(guard) = state.custom_text.lock()
        {
            let opt = (!guard.trim().is_empty()).then(|| guard.clone());
            if opt != ascii_saver.rotation.custom_text {
                ascii_saver.rotation.mode = if opt.is_some() {
                    TextMode::Custom
                } else {
                    TextMode::Os
                };
                ascii_saver.rotation.custom_text = opt;
                if !is_extras {
                    ascii_saver.init(cols, rows);
                }
            }
            last_custom_epoch = cur_epoch;
        }

        if state_wl.dpms.is_sleeping() {
            poll_and_read_events(
                &connection,
                &mut event_queue,
                &mut state_wl,
                Duration::from_millis(50),
            )?;
            continue;
        }

        let frame_dur =
            power.effective_frame_dur(base_frame_dur, !is_extras && ascii_saver.is_settled());
        let now = Instant::now();
        let dt = now.duration_since(last_frame);
        if dt >= frame_dur {
            last_frame = now;
            step_and_present_surfaces(
                &mut state_wl,
                &shm,
                &qh,
                dt,
                (cols, rows),
                &mut is_extras,
                &mut extras_session,
                &mut ascii_saver,
                &mut cells,
            );
            connection.flush()?;
        }

        let sleep_dur = frame_dur.saturating_sub(last_frame.elapsed());
        poll_and_read_events(&connection, &mut event_queue, &mut state_wl, sleep_dur)?;
    }

    if state_wl.exit_requested {
        state.stop();
    }

    state_wl.cleanup_presentation(&mut event_queue);
    Ok(())
}
