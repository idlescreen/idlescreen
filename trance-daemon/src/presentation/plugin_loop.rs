// SPDX-License-Identifier: MIT

use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use trance_api::{clear_caption, clear_primary_bounds};
use trance_runner::plugin_session::PluginSession;
use trance_upscaler::{simulation_tick_hz, target_fps};
use wayland_present::{OutputLayout, OverlayPresenter};

use super::frame_loop::run_frame_loop;
use super::layout::{
    install_primary_bounds_callback, normalize_layout_positions, primary_bounds_in_grid,
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
    log_output_layouts(&layouts);

    let mut session = PluginSession::load_with_options(
        saver_name,
        &options.launch_mode,
        Some(options.gpu_enabled),
        options.render_scale,
    )
    .map_err(|e| e.to_string())?;

    let context = PresentationContext::build(&mut session, &mut layouts)?;
    install_layout_callbacks(
        context.primary_bounds,
        context.virtual_cols,
        context.virtual_rows,
    );
    session.init(context.virtual_cols, context.virtual_rows);

    let pacing = FramePacing::compute(&layouts, context.primary, &mut session);
    log_run_startup(saver_name, &layouts, &pacing, &session);

    let result = pacing.run_loop(
        presenter,
        stop,
        &mut session,
        &layouts,
        context.primary,
        context.virtual_cols,
        context.virtual_rows,
        options,
    );

    clear_primary_bounds();
    clear_caption();
    result
}

fn log_output_layouts(layouts: &[OutputLayout]) {
    for layout in layouts {
        tracing::info!(
            "output {} @ ({}, {}) — {}x{} @ {} Hz",
            layout.id,
            layout.x,
            layout.y,
            layout.width,
            layout.height,
            layout.refresh_rate_hz
        );
    }
}

struct PresentationContext {
    primary: OutputLayout,
    virtual_cols: usize,
    virtual_rows: usize,
    primary_bounds: trance_api::MonitorCellBounds,
}

impl PresentationContext {
    fn build(session: &mut PluginSession, layouts: &mut [OutputLayout]) -> Result<Self, String> {
        let primary = layouts
            .iter()
            .max_by_key(|layout| layout.width.saturating_mul(layout.height))
            .copied()
            .ok_or_else(|| "no primary output found".to_string())?;

        normalize_layout_positions(layouts);
        let (min_x, min_y, total_w, total_h) = virtual_desktop(layouts);
        let (virtual_cols, virtual_rows) = span_simulation_grid(session, total_w, total_h);
        let primary_bounds = primary_bounds_in_grid(
            primary,
            min_x,
            min_y,
            total_w,
            total_h,
            virtual_cols,
            virtual_rows,
        );

        Ok(Self {
            primary,
            virtual_cols,
            virtual_rows,
            primary_bounds,
        })
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
        session: &mut PluginSession,
    ) -> Self {
        let present_refresh = presentation_refresh_hz(layouts, primary);
        let present_fps = target_fps(present_refresh);
        let tick_hz = simulation_tick_hz();
        let frame_duration = Duration::from_secs_f32(1.0 / present_fps);
        session.set_simulation_rate(tick_hz);
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

    #[allow(clippy::too_many_arguments)]
    fn run_loop(
        mut self,
        presenter: &OverlayPresenter,
        stop: &AtomicBool,
        session: &mut PluginSession,
        layouts: &[OutputLayout],
        primary: OutputLayout,
        virtual_cols: usize,
        virtual_rows: usize,
        options: PresentationOptions,
    ) -> Result<(), String> {
        run_frame_loop(
            presenter,
            stop,
            session,
            layouts,
            primary,
            virtual_cols,
            virtual_rows,
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
    session: &PluginSession,
) {
    tracing::info!(
        "running plugin '{}' on {} monitor(s) at {:.0} FPS / {:.0} tick (render scale {:.0}%, GPU: {})",
        saver_name,
        layouts.len(),
        pacing.present_fps,
        pacing.tick_hz,
        session.render_scale() * 100.0,
        if session.using_gpu_upscale() {
            "yes"
        } else {
            "no"
        }
    );
}
