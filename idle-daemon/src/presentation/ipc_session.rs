// SPDX-License-Identifier: MIT

use idle_api::TerminalCell;
use idle_ipc::{IpcCommand, IpcResponse, SharedMemory};
use idle_runner::cell_renderer::CellRenderer;
use idle_runner::launcher::LaunchMode;
use idle_upscaler::{FilterMode, FrameUpscaler, resolve_render_scale};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Child;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use super::ipc_init::initialize_ipc_session;
use super::ipc_raster::raster_viewport_into;

pub struct IpcPluginSession {
    pub(crate) saver_name: String,
    gpu_enabled: bool,
    render_scale: f32,
    renderer: CellRenderer,
    upscaler: FrameUpscaler,
    pub(crate) grid: Vec<TerminalCell>,
    content_buf: Vec<u8>,
    pixel_buf: Vec<u8>,
    hardware_scaling: bool,

    pub(crate) child: Option<Child>,
    pub(crate) socket: Option<UnixStream>,
    pub(crate) shm: Option<SharedMemory>,
    pub(crate) socket_path: Option<PathBuf>,
    pub(crate) expected_stop: Arc<AtomicBool>,
    /// When true, an unexpected child exit may arm the failsafe locker once.
    pub(crate) failsafe_armed: Arc<AtomicBool>,
}

impl IpcPluginSession {
    pub fn load_with_options(
        saver_name: &str,
        _launch_mode: &LaunchMode,
        gpu_enabled: Option<bool>,
        render_scale: Option<f32>,
    ) -> Result<Self, String> {
        let renderer = CellRenderer::new().map_err(|e| e.to_string())?;
        let use_gpu = gpu_enabled.unwrap_or_else(idle_upscaler::gpu_enabled);
        let render_scale = resolve_render_scale(use_gpu, render_scale);
        let upscaler = FrameUpscaler::new(use_gpu, FilterMode::from_env());

        Ok(Self {
            saver_name: saver_name.to_string(),
            gpu_enabled: use_gpu,
            render_scale,
            renderer,
            upscaler,
            grid: Vec::new(),
            content_buf: Vec::new(),
            pixel_buf: Vec::new(),
            hardware_scaling: false,
            child: None,
            socket: None,
            shm: None,
            socket_path: None,
            expected_stop: Arc::new(AtomicBool::new(false)),
            failsafe_armed: Arc::new(AtomicBool::new(true)),
        })
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

    pub fn init(&mut self, cols: usize, rows: usize) -> Result<(), String> {
        let cells = cols
            .checked_mul(rows)
            .ok_or_else(|| format!("grid size overflow: {cols}x{rows}"))?;
        self.grid = vec![TerminalCell::default(); cells];
        let init_res = initialize_ipc_session(
            &self.saver_name,
            cols,
            rows,
            self.gpu_enabled,
            self.render_scale,
        )?;

        self.child = Some(init_res.child);
        self.socket = Some(init_res.socket);
        self.shm = Some(init_res.shm);
        self.socket_path = Some(init_res.socket_path);

        Ok(())
    }

    pub fn set_simulation_rate(&mut self, fps: f32) {
        if let Some(ref mut socket) = self.socket {
            let cmd = IpcCommand::SetSimulationRate { hz: fps };
            if let Err(e) = cmd.write_to(&mut *socket) {
                tracing::error!("failed to send SetSimulationRate: {}", e);
                return;
            }
            match IpcResponse::read_from(&mut *socket) {
                Ok(IpcResponse::Ack) => {}
                Ok(resp) => tracing::error!("unexpected response to SetSimulationRate: {:?}", resp),
                Err(e) => tracing::error!("failed to read SetSimulationRate Ack: {}", e),
            }
        }
    }

    pub fn tick(&mut self, frame_dt: Duration) {
        if let Some(ref mut socket) = self.socket {
            let cmd = IpcCommand::TickAndDraw {
                dt_micros: frame_dt.as_micros() as u64,
            };
            if let Err(e) = cmd.write_to(&mut *socket) {
                tracing::error!("failed to send TickAndDraw: {}", e);
            }
        }
    }

    pub fn draw_frame(&mut self, grid_cols: usize, grid_rows: usize) -> bool {
        if let Some(ref mut socket) = self.socket {
            match IpcResponse::read_from(&mut *socket) {
                Ok(IpcResponse::FrameReady { scanlines }) => {
                    if let Some(ref shm) = self.shm {
                        // SAFETY: SHM mapped for session lifetime; dims set at init.
                        match unsafe { shm.cells_mut() } {
                            Ok(cells) => {
                                if let Some(need) = grid_cols.checked_mul(grid_rows) {
                                    if self.grid.len() != need {
                                        self.grid = vec![TerminalCell::default(); need];
                                    }
                                } else {
                                    tracing::error!(
                                        "grid resize overflow: {grid_cols}x{grid_rows}"
                                    );
                                    return false;
                                }
                                // Zip avoids bounds checks on the destination grid.
                                for (dst, src) in self.grid.iter_mut().zip(cells.iter()) {
                                    *dst = TerminalCell::from(*src);
                                }
                            }
                            Err(e) => {
                                tracing::error!("shm cells view rejected: {e}");
                            }
                        }
                    }
                    return scanlines;
                }
                Ok(resp) => tracing::error!("unexpected response to TickAndDraw: {:?}", resp),
                Err(e) => tracing::error!("failed to read response to TickAndDraw: {}", e),
            }
        }
        false
    }

    pub fn raster_viewport(
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
    ) -> Vec<u8> {
        let using_gpu = self.using_gpu_upscale();
        let hardware_scaling = self.hardware_scaling;
        raster_viewport_into(
            &mut self.renderer,
            &mut self.upscaler,
            &self.grid,
            hardware_scaling,
            using_gpu,
            &mut self.content_buf,
            &mut self.pixel_buf,
            col_start,
            row_start,
            cols,
            rows,
            grid_cols,
            width,
            height,
            scanlines,
        );
        let cap = self.pixel_buf.capacity();
        std::mem::replace(&mut self.pixel_buf, Vec::with_capacity(cap))
    }
}
