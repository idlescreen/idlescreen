// SPDX-License-Identifier: MIT

use std::path::Path;
use std::time::Duration;

use libloading::Library;
use trance_api::{Screensaver, ScreensaverInstance, TerminalCell};
use trance_gpu::{resolve_render_scale, FilterMode, FrameUpscaler};

use crate::cell_renderer::CellRenderer;
use crate::launcher::{resolve_saver_binary, LaunchMode};

struct PluginGuard {
    ptr: *mut ScreensaverInstance,
    destroy: unsafe extern "C" fn(*mut ScreensaverInstance),
    _lib: Library,
}

impl Drop for PluginGuard {
    fn drop(&mut self) {
        unsafe {
            (self.destroy)(self.ptr);
        }
    }
}

/// Headless screensaver plugin host for Wayland frame presentation.
pub struct PluginSession {
    plugin: PluginGuard,
    renderer: CellRenderer,
    upscaler: FrameUpscaler,
    render_scale: f32,
    grid: Vec<TerminalCell>,
    physics_accumulator: Duration,
    physics_duration: Duration,
    simulation_cols: usize,
    simulation_rows: usize,
}

impl PluginSession {
    pub fn load(saver_name: &str) -> Result<Self, String> {
        Self::load_with_options(saver_name, &LaunchMode::Daemon, None, None)
    }

    pub fn load_with_options(
        saver_name: &str,
        launch_mode: &LaunchMode,
        gpu_enabled: Option<bool>,
        render_scale: Option<f32>,
    ) -> Result<Self, String> {
        let path = resolve_saver_binary(saver_name, launch_mode)
            .map_err(|error| error.to_string())?;
        println!("trance-runner: loading plugin '{}' from {}", saver_name, path.display());
        Self::load_path_with_options(&path, gpu_enabled, render_scale)
    }

    pub fn load_path(path: &Path) -> Result<Self, String> {
        Self::load_path_with_options(path, None, None)
    }

    pub fn load_path_with_options(
        path: &Path,
        gpu_enabled: Option<bool>,
        render_scale: Option<f32>,
    ) -> Result<Self, String> {
        let renderer = CellRenderer::new()?;
        let use_gpu = gpu_enabled.unwrap_or_else(trance_gpu::gpu_enabled);
        let render_scale = resolve_render_scale(use_gpu, render_scale);
        let upscaler = FrameUpscaler::new(use_gpu, FilterMode::from_env());
        if upscaler.using_gpu() {
            println!(
                "trance-runner: GPU upscale enabled (render scale {:.0}%, adapter: {})",
                render_scale * 100.0,
                upscaler.adapter_name().unwrap_or("unknown")
            );
        } else {
            println!(
                "trance-runner: CPU upscale (render scale {:.0}%)",
                render_scale * 100.0
            );
        }

        unsafe {
            let lib = Library::new(path).map_err(|error| error.to_string())?;
            let create_fn: libloading::Symbol<unsafe extern "C" fn() -> *mut ScreensaverInstance> =
                lib.get(b"create_screensaver")
                    .map_err(|error| error.to_string())?;
            let destroy_fn: libloading::Symbol<unsafe extern "C" fn(*mut ScreensaverInstance)> =
                lib.get(b"destroy_screensaver")
                    .map_err(|error| error.to_string())?;

            let raw_ptr = create_fn();
            if raw_ptr.is_null() {
                return Err("plugin returned null screensaver instance".into());
            }

            let guard = PluginGuard {
                ptr: raw_ptr,
                destroy: *destroy_fn,
                _lib: lib,
            };

            Ok(Self {
                plugin: guard,
                renderer,
                upscaler,
                render_scale,
                grid: Vec::new(),
                physics_accumulator: Duration::ZERO,
                physics_duration: Duration::from_secs_f32(1.0 / 120.0),
                simulation_cols: 0,
                simulation_rows: 0,
            })
        }
    }

    pub fn render_scale(&self) -> f32 {
        self.render_scale
    }

    pub fn using_gpu_upscale(&self) -> bool {
        self.upscaler.using_gpu()
    }

    pub fn grid_for_pixels(&self, width: u32, height: u32) -> (usize, usize) {
        self.renderer
            .grid_for_pixels_scaled(width, height, self.render_scale)
    }

    pub fn init(&mut self, cols: usize, rows: usize) {
        self.simulation_cols = cols;
        self.simulation_rows = rows;
        self.grid = vec![TerminalCell::default(); cols * rows];
        self.plugin.saver_mut().init(cols, rows);
    }

    pub fn set_simulation_rate(&mut self, fps: f32) {
        let hz = fps.max(30.0);
        self.physics_duration = Duration::from_secs_f32(1.0 / hz);
    }

    pub fn tick(&mut self, frame_dt: Duration) {
        self.plugin.saver_mut().update_frame_time(frame_dt);

        self.physics_accumulator += frame_dt;
        if self.physics_accumulator > Duration::from_millis(100) {
            self.physics_accumulator = Duration::from_millis(100);
        }

        while self.physics_accumulator >= self.physics_duration {
            let dt = self.physics_duration;
            let cols = self.simulation_cols;
            let rows = self.simulation_rows;
            self.plugin.saver_mut().update(dt, cols, rows);
            self.physics_accumulator -= dt;
        }
    }

    pub fn blit_to_monitor(
        &mut self,
        src: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
    ) -> Vec<u8> {
        self.upscaler
            .upscale_letterbox(src, src_w, src_h, dst_w, dst_h)
    }

    pub fn draw_frame(&mut self, grid_cols: usize, grid_rows: usize) -> bool {
        if self.grid.len() != grid_cols * grid_rows {
            self.grid = vec![TerminalCell::default(); grid_cols * grid_rows];
        }
        let saver = self.plugin.saver_mut();
        saver.draw(&mut self.grid, grid_cols, grid_rows);
        saver.has_scanlines()
    }

    pub fn render(&mut self, cols: usize, rows: usize, width: u32, height: u32) -> Vec<u8> {
        let scanlines = self.draw_frame(cols, rows);
        self.raster_viewport(0, 0, cols, rows, cols, rows, width, height, scanlines)
    }

    pub fn render_viewport(
        &mut self,
        col_start: usize,
        row_start: usize,
        cols: usize,
        rows: usize,
        grid_cols: usize,
        grid_rows: usize,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let scanlines = self.draw_frame(grid_cols, grid_rows);
        self.raster_viewport(
            col_start,
            row_start,
            cols,
            rows,
            grid_cols,
            grid_rows,
            width,
            height,
            scanlines,
        )
    }

    pub fn raster_viewport(
        &mut self,
        col_start: usize,
        row_start: usize,
        cols: usize,
        rows: usize,
        _grid_cols: usize,
        _grid_rows: usize,
        width: u32,
        height: u32,
        scanlines: bool,
    ) -> Vec<u8> {
        let content_w = self.renderer.content_width(cols);
        let content_h = self.renderer.content_height(rows);
        let content = self.renderer.render_content_viewport(
            &self.grid,
            _grid_cols,
            col_start,
            row_start,
            cols,
            rows,
            scanlines,
        );

        self.upscaler
            .upscale_stretch(&content, content_w, content_h, width, height)
    }
}

impl PluginGuard {
    fn saver_mut(&mut self) -> &mut dyn Screensaver {
        unsafe { &mut *(*self.ptr).inner }
    }
}