// SPDX-License-Identifier: MIT

#![allow(clippy::too_many_arguments)]

use crate::cell_renderer::CellRenderer;
use idle_api::{Screensaver, ScreensaverInstance, TerminalCell};
use idle_upscaler::FrameUpscaler;
use std::time::Duration;

mod loading;
mod reloading;

pub(crate) struct PluginGuard {
    pub(crate) ptr: *mut ScreensaverInstance,
    pub(crate) destroy: unsafe extern "C" fn(*mut ScreensaverInstance),
    pub(crate) _lib: libloading::Library,
}

impl Drop for PluginGuard {
    fn drop(&mut self) {
        unsafe {
            (self.destroy)(self.ptr);
        }
    }
}

impl PluginGuard {
    pub(crate) fn saver_mut(&mut self) -> &mut dyn Screensaver {
        unsafe { &mut *(*self.ptr).inner }
    }
}

/// Headless screensaver plugin host for Wayland frame presentation.
pub struct PluginSession {
    pub(crate) plugin: Option<PluginGuard>,
    pub(crate) plugin_path: std::path::PathBuf,
    pub(crate) renderer: CellRenderer,
    pub(crate) upscaler: FrameUpscaler,
    pub(crate) render_scale: f32,
    pub(crate) grid: Vec<TerminalCell>,
    pub(crate) content_buf: Vec<u8>,
    pub(crate) pixel_buf: Vec<u8>,
    pub(crate) physics_accumulator: Duration,
    pub(crate) physics_duration: Duration,
    pub(crate) time_elapsed: Duration,
    pub(crate) simulation_cols: usize,
    pub(crate) simulation_rows: usize,
    pub(crate) hardware_scaling: bool,
    pub(crate) watcher: Option<notify::RecommendedWatcher>,
    pub(crate) needs_reload: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl PluginSession {
    pub fn grid(&self) -> &[TerminalCell] {
        &self.grid
    }

    pub fn render_scale(&self) -> f32 {
        self.render_scale
    }

    pub fn using_gpu_upscale(&self) -> bool {
        self.upscaler.using_gpu()
    }

    pub fn set_hardware_scaling(&mut self, enabled: bool) {
        self.hardware_scaling = enabled;
    }

    pub fn content_width(&self, cols: usize) -> u32 {
        self.renderer.content_width(cols)
    }

    pub fn content_height(&self, rows: usize) -> u32 {
        self.renderer.content_height(rows)
    }

    pub fn grid_for_pixels(&self, width: u32, height: u32) -> (usize, usize) {
        self.renderer
            .grid_for_pixels_scaled(width, height, self.render_scale)
    }

    pub fn init(&mut self, cols: usize, rows: usize) {
        self.simulation_cols = cols;
        self.simulation_rows = rows;
        let cells = cols.checked_mul(rows).unwrap_or(0);
        self.grid = vec![TerminalCell::default(); cells];
        if let Some(plugin) = self.plugin.as_mut() {
            plugin.saver_mut().init(cols, rows);
        }
    }

    pub fn set_simulation_rate(&mut self, fps: f32) {
        // Finite, bounded Hz only — NaN/inf must not yield zero-duration busy loops.
        let hz = if fps.is_finite() {
            fps.clamp(30.0, 240.0)
        } else {
            30.0
        };
        self.physics_duration = Duration::from_secs_f32(1.0 / hz);
    }

    #[tracing::instrument(skip_all)]
    pub fn tick(&mut self, frame_dt: Duration) {
        if let Some(plugin) = self.plugin.as_mut() {
            plugin.saver_mut().update_frame_time(frame_dt);
        }
        self.time_elapsed += frame_dt;

        self.physics_accumulator += frame_dt;
        if self.physics_accumulator > Duration::from_millis(100) {
            self.physics_accumulator = Duration::from_millis(100);
        }

        while self.physics_accumulator >= self.physics_duration {
            let dt = self.physics_duration;
            let cols = self.simulation_cols;
            let rows = self.simulation_rows;
            if let Some(plugin) = self.plugin.as_mut() {
                plugin.saver_mut().update(dt, cols, rows);
            }
            self.physics_accumulator -= dt;
        }
    }

    pub fn blit_to_monitor_into(
        &mut self,
        src: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
        out: &mut Vec<u8>,
    ) {
        self.upscaler
            .upscale_letterbox_into(src, src_w, src_h, dst_w, dst_h, out);
    }

    pub fn draw_frame(&mut self, grid_cols: usize, grid_rows: usize) -> bool {
        if self.grid.len() != grid_cols * grid_rows {
            self.grid = vec![TerminalCell::default(); grid_cols * grid_rows];
        }
        if let Some(plugin) = self.plugin.as_mut() {
            let saver = plugin.saver_mut();
            saver.draw(&mut self.grid, grid_cols, grid_rows);
            saver.has_scanlines()
        } else {
            false
        }
    }

    #[tracing::instrument(skip_all, fields(cols, rows, width, height))]
    pub fn render(&mut self, cols: usize, rows: usize, width: u32, height: u32) -> Vec<u8> {
        let scanlines = self.draw_frame(cols, rows);
        self.raster_viewport_internal(0, 0, cols, rows, cols, rows, width, height, scanlines);
        let cap = self.pixel_buf.capacity();
        std::mem::replace(&mut self.pixel_buf, Vec::with_capacity(cap))
    }

    pub fn raster_viewport(
        &mut self,
        col_start: usize,
        row_start: usize,
        cols: usize,
        rows: usize,
        grid_cols: usize,
        grid_rows: usize,
        width: u32,
        height: u32,
        scanlines: bool,
    ) -> Vec<u8> {
        self.raster_viewport_internal(
            col_start, row_start, cols, rows, grid_cols, grid_rows, width, height, scanlines,
        );
        let cap = self.pixel_buf.capacity();
        std::mem::replace(&mut self.pixel_buf, Vec::with_capacity(cap))
    }

    fn raster_viewport_internal(
        &mut self,
        col_start: usize,
        row_start: usize,
        cols: usize,
        rows: usize,
        grid_cols: usize,
        _grid_rows: usize,
        width: u32,
        height: u32,
        scanlines: bool,
    ) {
        if self.hardware_scaling && !self.using_gpu_upscale() {
            self.renderer.render_content_viewport_into(
                &self.grid,
                grid_cols,
                col_start,
                row_start,
                cols,
                rows,
                scanlines,
                &mut self.pixel_buf,
            );
            return;
        }

        let content_w = self.renderer.content_width(cols);
        let content_h = self.renderer.content_height(rows);
        self.renderer.render_content_viewport_into(
            &self.grid,
            grid_cols,
            col_start,
            row_start,
            cols,
            rows,
            scanlines,
            &mut self.content_buf,
        );

        self.upscaler.upscale_stretch_into(
            &self.content_buf,
            content_w,
            content_h,
            width,
            height,
            &mut self.pixel_buf,
        );
    }
}
