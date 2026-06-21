// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use trance_api::MonitorCellBounds;
use trance_gpu::{simulation_tick_hz, target_fps};
use trance_api::{caption_text, clear_caption};
use trance_runner::{caption_overlay, fps_overlay};
use trance_runner::plugin_meta::{display_mode_for, DisplayMode};
use trance_runner::launcher::LaunchMode;
use trance_runner::plugin_session::PluginSession;
use trance_runner::toolkit::theme_query;
use wayland_present::OutputLayout;
use wayland_present::OverlayPresenter;

static PRIMARY_BOUNDS: OnceLock<RwLock<MonitorCellBounds>> = OnceLock::new();

#[derive(Clone)]
pub struct PresentationOptions {
    pub gpu_enabled: bool,
    pub show_fps_overlay: bool,
    pub display_mode: DisplayMode,
    pub render_scale: Option<f32>,
    pub launch_mode: LaunchMode,
}

pub struct PluginPresentation {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl PluginPresentation {
    pub fn start(
        presenter: Arc<OverlayPresenter>,
        saver_name: String,
        options: PresentationOptions,
    ) -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let presenter_for_thread = presenter.clone();

        let thread = thread::spawn(move || {
            if let Err(error) =
                run_plugin_loop(&presenter_for_thread, &saver_name, &stop_flag, options)
            {
                eprintln!("trance-daemon: plugin presentation ended: {error}");
                presenter_for_thread.hide();
            }
        });

        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }

    pub fn stop(&mut self, presenter: &OverlayPresenter) {
        self.stop.store(true, Ordering::Relaxed);
        presenter.hide();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_plugin_loop(
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

    normalize_layout_positions(&mut layouts);

    for layout in &layouts {
        println!(
            "trance-daemon: output {} @ ({}, {}) — {}x{} @ {} Hz",
            layout.id, layout.x, layout.y, layout.width, layout.height, layout.refresh_rate_hz
        );
    }

    let display_mode = display_mode_for(saver_name, Some(options.display_mode));
    println!(
        "trance-daemon: display mode for '{saver_name}': {:?}",
        display_mode
    );

    let mut session = PluginSession::load_with_options(
        saver_name,
        &options.launch_mode,
        Some(options.gpu_enabled),
        options.render_scale,
    )?;

    let primary = layouts
        .iter()
        .max_by_key(|layout| layout.width.saturating_mul(layout.height))
        .copied()
        .ok_or_else(|| "no primary output found".to_string())?;

    let (_virtual_w, _virtual_h, virtual_cols, virtual_rows) = match display_mode {
        DisplayMode::Expand => (primary.width, primary.height, 0usize, 0usize),
        DisplayMode::Mirror | DisplayMode::PrimaryOnly => {
            (primary.width, primary.height, 0usize, 0usize)
        }
        DisplayMode::Span => {
            let (min_x, min_y, total_w, total_h) = virtual_desktop(&layouts);
            let (cols, rows) = span_simulation_grid(&session, total_w, total_h);
            let primary_bounds = primary_bounds_in_grid(
                primary, min_x, min_y, total_w, total_h, cols, rows,
            );
            trance_api::publish_primary_bounds(primary_bounds);
            install_primary_bounds_callback(primary_bounds);
            unsafe {
                std::env::set_var("TRANCE_SPAN_MODE", "1");
            }
            let _ = trance_api::IS_SECONDARY_MONITOR_CALLBACK.set(|| false);
            (total_w, total_h, cols, rows)
        }
    };

    match display_mode {
        DisplayMode::Expand
        | DisplayMode::Mirror
        | DisplayMode::PrimaryOnly => {
            let (cols, rows) = session.grid_for_pixels(primary.width, primary.height);
            session.init(cols, rows);
        }
        DisplayMode::Span => {
            session.init(virtual_cols, virtual_rows);
        }
    }

    let present_refresh = presentation_refresh_hz(&layouts, primary);
    let present_fps = target_fps(present_refresh);
    let tick_hz = simulation_tick_hz();
    let frame_duration = Duration::from_secs_f32(1.0 / present_fps);
    session.set_simulation_rate(tick_hz);

    println!(
        "trance-daemon: running plugin '{}' on {} monitor(s) at {:.0} FPS / {:.0} tick (render scale {:.0}%, GPU: {})",
        saver_name,
        layouts.len(),
        present_fps,
        tick_hz,
        session.render_scale() * 100.0,
        if session.using_gpu_upscale() { "yes" } else { "no" }
    );

    let mut last_frame = Instant::now();
    let mut frame_counter = 0u64;
    let mut fps_report = Instant::now();
    let mut achieved_fps = 0.0f32;
    let mut black_frames: HashMap<(u32, u32), Vec<u8>> = HashMap::new();

    let clear_bounds = display_mode == DisplayMode::Span;
    let result = run_frame_loop(
        presenter,
        stop,
        &mut session,
        &layouts,
        primary,
        display_mode,
        virtual_cols,
        virtual_rows,
        options,
        present_fps,
        tick_hz,
        frame_duration,
        &mut last_frame,
        &mut frame_counter,
        &mut fps_report,
        &mut achieved_fps,
        &mut black_frames,
    );
    if clear_bounds {
        trance_api::clear_primary_bounds();
        clear_caption();
        unsafe {
            std::env::remove_var("TRANCE_SPAN_MODE");
        }
    }
    result
}

fn run_frame_loop(
    presenter: &OverlayPresenter,
    stop: &AtomicBool,
    session: &mut PluginSession,
    layouts: &[OutputLayout],
    primary: OutputLayout,
    display_mode: DisplayMode,
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
    black_frames: &mut HashMap<(u32, u32), Vec<u8>>,
) -> Result<(), String> {
    while !stop.load(Ordering::Relaxed) && presenter.is_visible() {
        let frame_start = Instant::now();
        let frame_dt = frame_start.saturating_duration_since(*last_frame);
        *last_frame = frame_start;
        session.tick(frame_dt);

        match display_mode {
            DisplayMode::Expand => {
                for layout in layouts {
                    let (cols, rows) = session.grid_for_pixels(layout.width, layout.height);
                    let mut pixels = session.render(cols, rows, layout.width, layout.height);
                    maybe_draw_overlays(
                        &mut pixels,
                        layout.width,
                        layout.height,
                        layout.id == primary.id,
                        options.show_fps_overlay,
                        *achieved_fps,
                    );
                    presenter.submit_frame(layout.id, layout.width, layout.height, pixels);
                }
            }
            DisplayMode::Mirror => {
                let (cols, rows) = session.grid_for_pixels(primary.width, primary.height);
                let base = session.render(cols, rows, primary.width, primary.height);
                for layout in layouts {
                    let mut pixels = if layout.id == primary.id {
                        base.clone()
                    } else {
                        session.blit_to_monitor(
                            &base,
                            primary.width,
                            primary.height,
                            layout.width,
                            layout.height,
                        )
                    };
                    maybe_draw_overlays(
                        &mut pixels,
                        layout.width,
                        layout.height,
                        layout.id == primary.id,
                        options.show_fps_overlay,
                        *achieved_fps,
                    );
                    presenter.submit_frame(layout.id, layout.width, layout.height, pixels);
                }
            }
            DisplayMode::PrimaryOnly => {
                let (cols, rows) = session.grid_for_pixels(primary.width, primary.height);
                let mut primary_pixels =
                    session.render(cols, rows, primary.width, primary.height);
                maybe_draw_overlays(
                    &mut primary_pixels,
                    primary.width,
                    primary.height,
                    true,
                    options.show_fps_overlay,
                    *achieved_fps,
                );
                for layout in layouts {
                    if layout.id == primary.id {
                        presenter.submit_frame(
                            layout.id,
                            layout.width,
                            layout.height,
                            primary_pixels.clone(),
                        );
                    } else {
                        presenter.submit_frame(
                            layout.id,
                            layout.width,
                            layout.height,
                            cached_black_frame(black_frames, layout.width, layout.height),
                        );
                    }
                }
            }
            DisplayMode::Span => {
                let (min_x, min_y, total_w, total_h) = virtual_desktop(&layouts);
                let scanlines = session.draw_frame(virtual_cols, virtual_rows);
                for layout in layouts {
                    let bounds = monitor_cell_bounds(
                        *layout,
                        min_x,
                        min_y,
                        total_w,
                        total_h,
                        virtual_cols,
                        virtual_rows,
                        layout.id == primary.id,
                    );
                    let col_w = bounds.end_col.saturating_sub(bounds.start_col).max(1);
                    let row_h = bounds.end_row.saturating_sub(bounds.start_row).max(1);
                    let mut pixels = session.raster_viewport(
                        bounds.start_col,
                        bounds.start_row,
                        col_w,
                        row_h,
                        virtual_cols,
                        virtual_rows,
                        layout.width,
                        layout.height,
                        scanlines,
                    );
                    maybe_draw_overlays(
                        &mut pixels,
                        layout.width,
                        layout.height,
                        layout.id == primary.id,
                        options.show_fps_overlay,
                        *achieved_fps,
                    );
                    presenter.submit_frame(layout.id, layout.width, layout.height, pixels);
                }
            }
        }

        *frame_counter += 1;
        let elapsed = frame_start.elapsed();
        if fps_report.elapsed() >= Duration::from_secs(1) {
            *achieved_fps = *frame_counter as f32 / fps_report.elapsed().as_secs_f32();
            if *frame_counter >= present_fps as u64 || fps_report.elapsed() >= Duration::from_secs(5) {
                println!(
                    "trance-daemon: achieved {:.1} FPS (target {:.0}, tick {:.0})",
                    *achieved_fps, present_fps, tick_hz
                );
                *frame_counter = 0;
                *fps_report = Instant::now();
            }
        }

        if elapsed < frame_duration {
            thread::sleep(frame_duration - elapsed);
        }
    }

    Ok(())
}

fn cached_black_frame(cache: &mut HashMap<(u32, u32), Vec<u8>>, width: u32, height: u32) -> Vec<u8> {
    let key = (width, height);
    if !cache.contains_key(&key) {
        let len = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        let mut pixels = vec![0u8; len];
        for alpha in pixels.iter_mut().skip(3).step_by(4) {
            *alpha = 0xFF;
        }
        cache.insert(key, pixels);
    }
    cache.get(&key).cloned().unwrap_or_default()
}

fn maybe_draw_overlays(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    is_primary: bool,
    show_fps: bool,
    achieved_fps: f32,
) {
    if !is_primary {
        return;
    }

    let caption = caption_text();
    if !caption.is_empty() {
        caption_overlay::draw_bottom_center(pixels, width, height, &caption, (245, 240, 200));
    }

    if show_fps {
        let label = format!("FPS {:.1}", achieved_fps);
        let (accent, _) = theme_query::load_global_theme();
        let color = accent.unwrap_or((0, 191, 255));
        fps_overlay::draw_top_right(pixels, width, height, &label, color);
    }
}

/// Presentation FPS target refresh rate.
///
/// Multi-monitor span uses the **primary** display refresh (e.g. 144 Hz on display 1),
/// not the lowest. One frame loop drives all outputs; the secondary (e.g. 60 Hz) may
/// skip or hold frames, which is fine for spillover content. Physics tick rate stays
/// independent (`TRANCE_TICK_HZ`, default 60).
///
/// Override sync policy with `TRANCE_PRESENT_SYNC=min|primary|max` (default: primary).
fn presentation_refresh_hz(layouts: &[OutputLayout], primary: OutputLayout) -> u32 {
    if layouts.len() <= 1 {
        return layouts
            .first()
            .map(|layout| layout.refresh_rate_hz)
            .unwrap_or(60)
            .max(60);
    }

    let min_hz = layouts
        .iter()
        .map(|layout| layout.refresh_rate_hz)
        .min()
        .unwrap_or(60)
        .max(60);
    let max_hz = layouts
        .iter()
        .map(|layout| layout.refresh_rate_hz)
        .max()
        .unwrap_or(60)
        .max(60);
    let primary_hz = primary.refresh_rate_hz.max(60);

    match std::env::var("TRANCE_PRESENT_SYNC").as_deref() {
        Ok("min") => min_hz,
        Ok("max") => max_hz,
        _ => primary_hz,
    }
}

fn normalize_layout_positions(layouts: &mut [OutputLayout]) {
    if layouts.len() <= 1 {
        return;
    }
    if layouts.iter().any(|layout| layout.x != 0 || layout.y != 0) {
        return;
    }

    let mut x = 0i32;
    for layout in layouts.iter_mut() {
        layout.x = x;
        layout.y = 0;
        x += layout.width as i32;
    }
}

/// Caps span simulation cost: full virtual-desktop coverage with a bounded cell count.
fn span_simulation_grid(session: &PluginSession, total_w: u32, total_h: u32) -> (usize, usize) {
    const MAX_SPAN_CELLS: usize = 12_000;
    let (cols, rows) = session.grid_for_pixels(total_w, total_h);
    let cells = cols.saturating_mul(rows);
    if cells <= MAX_SPAN_CELLS {
        return (cols, rows);
    }

    let scale = (MAX_SPAN_CELLS as f32 / cells as f32).sqrt();
    let capped_cols = ((cols as f32 * scale).floor() as usize).max(1);
    let capped_rows = ((rows as f32 * scale).floor() as usize).max(1);
    println!(
        "trance-daemon: span grid capped from {cols}x{rows} ({cells} cells) to {capped_cols}x{capped_rows}",
        capped_cols = capped_cols,
        capped_rows = capped_rows,
    );
    (capped_cols, capped_rows)
}

fn virtual_desktop(layouts: &[OutputLayout]) -> (i32, i32, u32, u32) {
    let min_x = layouts.iter().map(|layout| layout.x).min().unwrap_or(0);
    let min_y = layouts.iter().map(|layout| layout.y).min().unwrap_or(0);
    let max_x = layouts
        .iter()
        .map(|layout| layout.x + layout.width as i32)
        .max()
        .unwrap_or(0);
    let max_y = layouts
        .iter()
        .map(|layout| layout.y + layout.height as i32)
        .max()
        .unwrap_or(0);
    (
        min_x,
        min_y,
        (max_x - min_x).max(1) as u32,
        (max_y - min_y).max(1) as u32,
    )
}

fn monitor_cell_bounds(
    layout: OutputLayout,
    min_x: i32,
    min_y: i32,
    total_w: u32,
    total_h: u32,
    virtual_cols: usize,
    virtual_rows: usize,
    is_primary: bool,
) -> MonitorCellBounds {
    let rel_x1 = layout.x - min_x;
    let rel_y1 = layout.y - min_y;
    let rel_x2 = rel_x1 + layout.width as i32;
    let rel_y2 = rel_y1 + layout.height as i32;

    MonitorCellBounds {
        start_col: ((rel_x1 as usize).saturating_mul(virtual_cols)) / total_w as usize,
        end_col: ((rel_x2 as usize).saturating_mul(virtual_cols)) / total_w as usize,
        start_row: ((rel_y1 as usize).saturating_mul(virtual_rows)) / total_h as usize,
        end_row: ((rel_y2 as usize).saturating_mul(virtual_rows)) / total_h as usize,
        is_primary,
    }
}

fn primary_bounds_in_grid(
    primary: OutputLayout,
    min_x: i32,
    min_y: i32,
    total_w: u32,
    total_h: u32,
    virtual_cols: usize,
    virtual_rows: usize,
) -> MonitorCellBounds {
    monitor_cell_bounds(
        primary,
        min_x,
        min_y,
        total_w,
        total_h,
        virtual_cols,
        virtual_rows,
        true,
    )
}

fn install_primary_bounds_callback(bounds: MonitorCellBounds) {
    let _ = PRIMARY_BOUNDS.set(RwLock::new(bounds));
    let _ = trance_api::MONITOR_BOUNDS_CALLBACK.set(|_cols, _rows| {
        PRIMARY_BOUNDS
            .get()
            .and_then(|lock| lock.read().ok())
            .map(|guard| *guard)
            .unwrap_or(MonitorCellBounds {
                start_col: 0,
                end_col: 0,
                start_row: 0,
                end_row: 0,
                is_primary: true,
            })
    });
}

fn wait_for_output_layouts(
    presenter: &OverlayPresenter,
    timeout: Duration,
) -> Result<Vec<OutputLayout>, String> {
    let deadline = Instant::now() + timeout;
    let mut best = Vec::new();
    let mut layouts_seen_at = None::<Instant>;

    while Instant::now() < deadline {
        let layouts = presenter.output_layouts();
        if !layouts.is_empty() {
            best = layouts;
            layouts_seen_at.get_or_insert_with(Instant::now);
            if layouts_seen_at.is_some_and(|seen| seen.elapsed() >= Duration::from_millis(500)) {
                return Ok(best);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }

    if best.is_empty() {
        best = presenter.output_layouts();
    }
    Ok(best)
}