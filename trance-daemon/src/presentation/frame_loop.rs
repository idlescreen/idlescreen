// SPDX-License-Identifier: MIT

#![allow(clippy::too_many_arguments)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use super::ipc_session::IpcPluginSession;
use wayland_present::{OutputLayout, OverlayPresenter};

use super::layout::{monitor_cell_bounds, virtual_desktop};
use super::overlays::maybe_draw_overlays;
use crate::presentation::PresentationOptions;

/// Per-frame loop locals: inputs + state mutated across iterations.
struct FrameLoopState<'a> {
    presenter: &'a OverlayPresenter,
    stop: &'a AtomicBool,
    session: &'a mut IpcPluginSession,
    layouts: &'a [OutputLayout],
    primary: OutputLayout,
    virtual_cols: usize,
    virtual_rows: usize,
    options: PresentationOptions,
    present_fps: f32,
    tick_hz: f32,
    frame_duration: Duration,
    last_frame: Instant,
    frame_start: Instant,
    frame_counter: u64,
    fps_report: Instant,
    achieved_fps: f32,
    use_hw_scaling: bool,
}

pub fn run_frame_loop(
    presenter: &OverlayPresenter,
    stop: &AtomicBool,
    session: &mut IpcPluginSession,
    layouts: &[OutputLayout],
    primary: OutputLayout,
    virtual_cols: usize,
    virtual_rows: usize,
    options: PresentationOptions,
    present_fps: f32,
    tick_hz: f32,
    frame_duration: Duration,
    last_frame: &mut Instant,
    frame_counter: &mut u64,
    fps_report: &mut Instant,
    achieved_fps: &mut f32,
) -> Result<(), String> {
    let use_hw_scaling = presenter.supports_scaling() && !session.using_gpu_upscale();
    session.set_hardware_scaling(use_hw_scaling);
    if use_hw_scaling {
        tracing::info!("wayland-present: hardware scaling enabled via wp_viewporter");
    }

    let mut state = FrameLoopState {
        presenter,
        stop,
        session,
        layouts,
        primary,
        virtual_cols,
        virtual_rows,
        options,
        present_fps,
        tick_hz,
        frame_duration,
        last_frame: *last_frame,
        frame_start: *last_frame,
        frame_counter: *frame_counter,
        fps_report: *fps_report,
        achieved_fps: *achieved_fps,
        use_hw_scaling,
    };

    while !state.stop.load(Ordering::Relaxed) && state.presenter.is_visible() {
        state.frame_counter += 1;
        let frame_index = state.frame_counter;
        prepare_frame(&mut state)?;
        present_frame(&mut state);
        update_fps_counter(&mut state, frame_index);
    }

    *last_frame = state.last_frame;
    *frame_counter = state.frame_counter;
    *fps_report = state.fps_report;
    *achieved_fps = state.achieved_fps;
    Ok(())
}

fn prepare_frame(state: &mut FrameLoopState) -> Result<(), String> {
    let frame_start = Instant::now();
    let frame_dt = frame_start.saturating_duration_since(state.last_frame);
    state.last_frame = frame_start;
    state.frame_start = frame_start;
    state.session.tick(frame_dt);
    Ok(())
}

fn present_frame(state: &mut FrameLoopState) {
    let (min_x, min_y, total_w, total_h) = virtual_desktop(state.layouts);
    let scanlines = state
        .session
        .draw_frame(state.virtual_cols, state.virtual_rows);
    for layout in state.layouts {
        let bounds = monitor_cell_bounds(
            *layout,
            min_x,
            min_y,
            total_w,
            total_h,
            state.virtual_cols,
            state.virtual_rows,
            layout.id == state.primary.id,
        );
        let col_w = bounds.end_col.saturating_sub(bounds.start_col).max(1);
        let row_h = bounds.end_row.saturating_sub(bounds.start_row).max(1);

        let (target_w, target_h) = if state.use_hw_scaling {
            (
                state.session.content_width(col_w),
                state.session.content_height(row_h),
            )
        } else {
            (layout.width, layout.height)
        };

        let mut pixels = state.session.raster_viewport(
            bounds.start_col,
            bounds.start_row,
            col_w,
            row_h,
            state.virtual_cols,
            state.virtual_rows,
            target_w,
            target_h,
            scanlines,
        );
        maybe_draw_overlays(
            &mut pixels,
            target_w,
            target_h,
            layout.id == state.primary.id,
            state.options.show_fps_overlay,
            state.achieved_fps,
        );
        state
            .presenter
            .submit_frame(layout.id, target_w, target_h, pixels);
    }
}

fn update_fps_counter(state: &mut FrameLoopState, frame_index: u64) {
    let elapsed = state.frame_start.elapsed();
    if state.fps_report.elapsed() >= Duration::from_secs(1) {
        state.achieved_fps = frame_index as f32 / state.fps_report.elapsed().as_secs_f32();
        if frame_index >= state.present_fps as u64
            || state.fps_report.elapsed() >= Duration::from_secs(5)
        {
            tracing::info!(
                "achieved {:.1} FPS (target {:.0}, tick {:.0})",
                state.achieved_fps,
                state.present_fps,
                state.tick_hz
            );
            state.fps_report = Instant::now();
            state.frame_counter = 0;
        }
    }

    if elapsed < state.frame_duration {
        thread::sleep(state.frame_duration - elapsed);
    }
}
