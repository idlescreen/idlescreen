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

pub struct ActiveSession {
    pub output_id: u32,
    pub session: IpcPluginSession,
    pub cols: usize,
    pub rows: usize,
}

/// Per-frame loop locals: inputs + state mutated across iterations.
struct FrameLoopState<'a> {
    presenter: &'a OverlayPresenter,
    stop: &'a AtomicBool,
    sessions: &'a mut [ActiveSession],
    layouts: &'a [OutputLayout],
    primary: OutputLayout,
    independent_rendering: bool,
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
    session_start: Instant,
}

pub fn run_frame_loop(
    presenter: &OverlayPresenter,
    stop: &AtomicBool,
    sessions: &mut [ActiveSession],
    layouts: &[OutputLayout],
    primary: OutputLayout,
    independent_rendering: bool,
    options: PresentationOptions,
    present_fps: f32,
    tick_hz: f32,
    frame_duration: Duration,
    last_frame: &mut Instant,
    frame_counter: &mut u64,
    fps_report: &mut Instant,
    achieved_fps: &mut f32,
) -> Result<(), String> {
    let use_hw_scaling = presenter.supports_scaling() && !sessions[0].session.using_gpu_upscale();
    for s in sessions.iter_mut() {
        s.session.set_hardware_scaling(use_hw_scaling);
    }
    if use_hw_scaling {
        tracing::info!("wayland-present: hardware scaling enabled via wp_viewporter");
    }

    let mut state = FrameLoopState {
        presenter,
        stop,
        sessions,
        layouts,
        primary,
        independent_rendering,
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
        session_start: Instant::now(),
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
    for s in state.sessions.iter_mut() {
        if !s.session.is_plugin_alive() {
            s.session.recover(s.cols, s.rows)?;
            s.session.set_simulation_rate(state.tick_hz);
        }
        s.session.tick(frame_dt);
    }
    Ok(())
}

fn present_frame(state: &mut FrameLoopState) {
    let (min_x, min_y, total_w, total_h) = virtual_desktop(state.layouts);

    if state.independent_rendering {
        for s in state.sessions.iter_mut() {
            let scanlines = s.session.draw_frame(s.cols, s.rows);
            if let Some(layout) = state.layouts.iter().find(|l| l.id == s.output_id) {
                let target_w = if state.use_hw_scaling {
                    s.session.content_width(s.cols)
                } else {
                    layout.width
                };
                let target_h = if state.use_hw_scaling {
                    s.session.content_height(s.rows)
                } else {
                    layout.height
                };

                let mut pixels = s.session.raster_viewport(
                    0, 0, s.cols, s.rows, s.cols, s.rows, target_w, target_h, scanlines,
                );
                apply_fade_in(
                    std::sync::Arc::make_mut(&mut pixels).as_mut_slice(),
                    state.frame_start.duration_since(state.session_start),
                );
                maybe_draw_overlays(
                    std::sync::Arc::make_mut(&mut pixels).as_mut_slice(),
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
    } else {
        let s = &mut state.sessions[0];
        let scanlines = s.session.draw_frame(s.cols, s.rows);
        for layout in state.layouts {
            let bounds = monitor_cell_bounds(
                *layout,
                min_x,
                min_y,
                total_w,
                total_h,
                s.cols,
                s.rows,
                layout.id == state.primary.id,
            );
            let col_w = bounds.end_col.saturating_sub(bounds.start_col).max(1);
            let row_h = bounds.end_row.saturating_sub(bounds.start_row).max(1);

            let (target_w, target_h) = if state.use_hw_scaling {
                (
                    s.session.content_width(col_w),
                    s.session.content_height(row_h),
                )
            } else {
                (layout.width, layout.height)
            };

            let mut pixels = s.session.raster_viewport(
                bounds.start_col,
                bounds.start_row,
                col_w,
                row_h,
                s.cols,
                s.rows,
                target_w,
                target_h,
                scanlines,
            );
            apply_fade_in(
                std::sync::Arc::make_mut(&mut pixels).as_mut_slice(),
                state.frame_start.duration_since(state.session_start),
            );
            maybe_draw_overlays(
                std::sync::Arc::make_mut(&mut pixels).as_mut_slice(),
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

fn apply_fade_in(pixels: &mut [u8], elapsed: Duration) {
    let fade_duration = Duration::from_millis(500);
    if elapsed >= fade_duration {
        return;
    }

    let alpha_multiplier = elapsed.as_secs_f32() / fade_duration.as_secs_f32();
    let mult = (alpha_multiplier * 255.0) as u32;

    for chunk in pixels.chunks_exact_mut(4) {
        chunk[0] = ((chunk[0] as u32 * mult) / 255) as u8;
        chunk[1] = ((chunk[1] as u32 * mult) / 255) as u8;
        chunk[2] = ((chunk[2] as u32 * mult) / 255) as u8;
        chunk[3] = ((chunk[3] as u32 * mult) / 255) as u8;
    }
}
