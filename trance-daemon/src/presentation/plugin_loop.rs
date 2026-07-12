// SPDX-License-Identifier: MIT

use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use trance_api::{clear_caption, clear_primary_bounds};
use super::ipc_session::IpcPluginSession;
use trance_upscaler::{simulation_tick_hz, target_fps};
use wayland_present::{OutputLayout, OverlayPresenter};

use super::frame_loop::{ActiveSession, run_frame_loop};
use super::layout::{
    install_primary_bounds_callback, primary_bounds_in_grid,
    span_simulation_grid, virtual_desktop,
};
use super::refresh::{presentation_refresh_hz, wait_for_output_layouts};
use crate::presentation::PresentationOptions;

#[tracing::instrument(skip_all, fields(saver_name = %saver_name))]
pub fn run_plugin_loop(
    presenter: &OverlayPresenter,
    saver_name: &str,
    stop: &AtomicBool,
    options: PresentationOptions,
) -> Result<(), String> {
    presenter.show_screensaver();

    let mut layouts = wait_for_output_layouts(presenter, Duration::from_secs(3))?;
    if layouts.is_empty() {
        return Err("no configured outputs for screensaver presentation".into());
    }

    let topology = super::topology::DisplayTopologyMap::build(&layouts);
    for layout in layouts.iter_mut() {
        if let Some(topo) = topology.monitors.iter().find(|m| m.id == layout.id) {
            layout.x = topo.x;
            layout.y = topo.y;
            layout.width = topo.width;
            layout.height = topo.height;
            layout.scale = topo.scale;
        }
    }
    log_output_layouts(&layouts);

    let mut sessions = Vec::new();
    if topology.independent_rendering {
        for layout in &layouts {
            let mut session = IpcPluginSession::load_with_options(
                saver_name,
                &options.launch_mode,
                Some(options.gpu_enabled),
                options.render_scale,
            )?;
            let (cols, rows) = session.grid_for_pixels(layout.width, layout.height);
            session.init(cols, rows)?;
            sessions.push(ActiveSession {
                output_id: layout.id,
                session,
                cols,
                rows,
            });
        }
    } else {
        let mut session = IpcPluginSession::load_with_options(
            saver_name,
            &options.launch_mode,
            Some(options.gpu_enabled),
            options.render_scale,
        )?;
        let (_min_x, _min_y, total_w, total_h) = virtual_desktop(&layouts);
        let (virtual_cols, virtual_rows) = span_simulation_grid(&session, total_w, total_h);
        session.init(virtual_cols, virtual_rows)?;
        sessions.push(ActiveSession {
            output_id: 0,
            session,
            cols: virtual_cols,
            rows: virtual_rows,
        });
    }

    let primary = layouts
        .iter()
        .max_by_key(|layout| layout.width.saturating_mul(layout.height))
        .copied()
        .ok_or_else(|| "no primary output found".to_string())?;

    let primary_bounds = if topology.independent_rendering {
        let primary_session = sessions.iter().find(|s| s.output_id == primary.id).unwrap();
        trance_api::MonitorCellBounds {
            start_col: 0,
            end_col: primary_session.cols,
            start_row: 0,
            end_row: primary_session.rows,
            is_primary: true,
        }
    } else {
        let s = &sessions[0];
        let (min_x, min_y, total_w, total_h) = virtual_desktop(&layouts);
        primary_bounds_in_grid(
            primary,
            min_x,
            min_y,
            total_w,
            total_h,
            s.cols,
            s.rows,
        )
    };

    let (v_cols, v_rows) = if topology.independent_rendering {
        let primary_session = sessions.iter().find(|s| s.output_id == primary.id).unwrap();
        (primary_session.cols, primary_session.rows)
    } else {
        (sessions[0].cols, sessions[0].rows)
    };

    install_layout_callbacks(primary_bounds, v_cols, v_rows);

    let pacing = FramePacing::compute(&layouts, primary, &mut sessions);
    log_run_startup(saver_name, &layouts, &pacing, &sessions[0].session);

    let result = pacing.run_loop(
        presenter,
        stop,
        &mut sessions,
        &layouts,
        primary,
        topology.independent_rendering,
        options,
    );

    clear_primary_bounds();
    clear_caption();
    result
}

fn log_output_layouts(layouts: &[OutputLayout]) {
    for layout in layouts {
        tracing::info!(
            "output {} @ ({}, {}) — {}x{} @ {} Hz (scale: {})",
            layout.id,
            layout.x,
            layout.y,
            layout.width,
            layout.height,
            layout.refresh_rate_hz,
            layout.scale
        );
    }
}

fn install_layout_callbacks(
    primary_bounds: trance_api::MonitorCellBounds,
    virtual_cols: usize,
    virtual_rows: usize,
) {
    trance_api::publish_primary_bounds(primary_bounds);
    install_primary_bounds_callback(primary_bounds, virtual_cols, virtual_rows);
    let _ = trance_api::IS_SECONDARY_MONITOR_CALLBACK.set(|| false);
}

struct FramePacing {
    present_fps: f32,
    tick_hz: f32,
    frame_duration: Duration,
    last_frame: Instant,
    frame_counter: u64,
    fps_report: Instant,
    achieved_fps: f32,
}

impl FramePacing {
    fn compute(
        layouts: &[OutputLayout],
        primary: OutputLayout,
        sessions: &mut [ActiveSession],
    ) -> Self {
        let present_refresh = presentation_refresh_hz(layouts, primary);
        let mut present_fps = target_fps(present_refresh);
        let mut tick_hz = simulation_tick_hz();

        let sys = trance_runner::toolkit::sys_info::get_system_info();
        if sys.power_status.contains("Battery") {
            present_fps = present_fps.min(30.0);
            tick_hz = tick_hz.min(30.0);
            tracing::info!(
                "Battery power detected: capping physics simulation and rendering frame rate targets to 30 FPS/Hz"
            );
        }

        let frame_duration = Duration::from_secs_f32(1.0 / present_fps);
        for s in sessions {
            s.session.set_simulation_rate(tick_hz);
        }
        Self {
            present_fps,
            tick_hz,
            frame_duration,
            last_frame: Instant::now(),
            frame_counter: 0,
            fps_report: Instant::now(),
            achieved_fps: 0.0,
        }
    }

    fn run_loop(
        mut self,
        presenter: &OverlayPresenter,
        stop: &AtomicBool,
        sessions: &mut [ActiveSession],
        layouts: &[OutputLayout],
        primary: OutputLayout,
        independent_rendering: bool,
        options: PresentationOptions,
    ) -> Result<(), String> {
        run_frame_loop(
            presenter,
            stop,
            sessions,
            layouts,
            primary,
            independent_rendering,
            options,
            self.present_fps,
            self.tick_hz,
            self.frame_duration,
            &mut self.last_frame,
            &mut self.frame_counter,
            &mut self.fps_report,
            &mut self.achieved_fps,
        )
    }
}

fn log_run_startup(
    saver_name: &str,
    layouts: &[OutputLayout],
    pacing: &FramePacing,
    session: &IpcPluginSession,
) {
    tracing::info!(
        "running plugin '{}' on {} monitor(s) at {:.0} FPS / {:.0} tick (render scale {:.0}%, GPU: {})",
        saver_name,
        layouts.len(),
        pacing.present_fps,
        pacing.tick_hz,
        session.render_scale() * 100.0,
        if session.using_gpu_upscale() { "yes" } else { "no" }
    );
}