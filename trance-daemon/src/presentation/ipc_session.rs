// SPDX-License-Identifier: MIT

use std::fs;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use trance_api::TerminalCell;
use trance_runner::cell_renderer::CellRenderer;
use trance_runner::launcher::LaunchMode;
use trance_upscaler::{FilterMode, FrameUpscaler, resolve_render_scale};
use trance_ipc::{SharedMemory, IpcCommand, IpcResponse, compute_shm_size, SHM_MAGIC};

pub struct IpcPluginSession {
    saver_name: String,
    gpu_enabled: bool,
    render_scale: f32,
    renderer: CellRenderer,
    upscaler: FrameUpscaler,
    pub(crate) grid: Vec<TerminalCell>,
    content_buf: Vec<u8>,
    pixel_buf: Vec<u8>,
    hardware_scaling: bool,

    child: Option<Child>,
    socket: Option<UnixStream>,
    shm: Option<SharedMemory>,
    socket_path: Option<PathBuf>,
    expected_stop: Arc<AtomicBool>,
}

impl IpcPluginSession {
    pub fn load_with_options(
        saver_name: &str,
        _launch_mode: &LaunchMode,
        gpu_enabled: Option<bool>,
        render_scale: Option<f32>,
    ) -> Result<Self, String> {
        let renderer = CellRenderer::new().map_err(|e| e.to_string())?;
        let use_gpu = gpu_enabled.unwrap_or_else(trance_upscaler::gpu_enabled);
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
        self.grid = vec![TerminalCell::default(); cols * rows];

        let rand_val = std::process::id();
        let socket_path = std::env::temp_dir().join(format!("trance-uds-{}.sock", rand_val));
        if socket_path.exists() {
            let _ = fs::remove_file(&socket_path);
        }
        let listener = UnixListener::bind(&socket_path)
            .map_err(|e| format!("failed to bind UDS listener: {}", e))?;
        listener.set_nonblocking(true)
            .map_err(|e| format!("failed to set UDS listener nonblocking: {}", e))?;

        self.socket_path = Some(socket_path.clone());

        let shm_name = format!("/trance-shm-{}", rand_val);
        let shm_size = compute_shm_size(cols, rows);
        let shm = SharedMemory::create(&shm_name, shm_size)?;

        unsafe {
            let header = shm.header_mut();
            header.magic = SHM_MAGIC;
            header.cols = cols as u32;
            header.rows = rows as u32;
            header.frame_counter = 0;
        }

        self.shm = Some(shm);

        let current_exe = std::env::current_exe()
            .map_err(|e| format!("failed to get current exe path: {}", e))?;

        let gpu_str = self.gpu_enabled.to_string();
        let scale_str = format!("{:.6}", self.render_scale);

        let child = Command::new(current_exe)
            .arg("run-ipc-runner")
            .arg(&self.saver_name)
            .arg(socket_path.to_str().ok_or("invalid socket path")?)
            .arg(&shm_name)
            .arg(cols.to_string())
            .arg(rows.to_string())
            .arg(&gpu_str)
            .arg(&scale_str)
            .spawn()
            .map_err(|e| format!("failed to spawn runner process: {}", e))?;

        let child_pid = child.id();
        self.child = Some(child);

        let expected_stop = self.expected_stop.clone();
        std::thread::spawn(move || {
            let mut status: libc::c_int = 0;
            loop {
                let res = unsafe { libc::waitpid(child_pid as libc::pid_t, &mut status, 0) };
                if res < 0 {
                    let err = std::io::Error::last_os_error();
                    if err.raw_os_error() == Some(libc::EINTR) {
                        continue;
                    }
                    break;
                }
                break;
            }

            if expected_stop.load(Ordering::Relaxed) {
                return;
            }

            let exited_cleanly = libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0;

            if !exited_cleanly {
                tracing::error!(
                    "Watchdog: screensaver runner (pid {}) exited unexpectedly",
                    child_pid
                );
                if let Err(e) = crate::failsafe::spawn_failsafe_locker() {
                    tracing::error!("Watchdog: failed to spawn failsafe locker: {e}");
                }
            }
        });

        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(5);
        let socket = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if start.elapsed() > timeout {
                        return Err("timeout waiting for runner process connection".into());
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(format!("UDS accept error: {}", e)),
            }
        };

        socket.set_nonblocking(false)
            .map_err(|e| format!("failed to set blocking on runner stream: {}", e))?;

        let mut socket = socket;

        match IpcResponse::read_from(&mut socket) {
            Ok(IpcResponse::Ready) => {}
            Ok(resp) => return Err(format!("unexpected connection message: {:?}", resp)),
            Err(e) => return Err(format!("failed to read connection message: {}", e)),
        }

        IpcCommand::Init {
            cols: cols as u32,
            rows: rows as u32,
        }
        .write_to(&mut socket)
        .map_err(|e| format!("failed to send Init: {}", e))?;

        match IpcResponse::read_from(&mut socket) {
            Ok(IpcResponse::Ack) => {}
            Ok(resp) => return Err(format!("unexpected response to Init: {:?}", resp)),
            Err(e) => return Err(format!("failed to read Init Ack: {}", e)),
        }

        self.socket = Some(socket);
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
                        let cells = unsafe { shm.cells_mut() };
                        if self.grid.len() != grid_cols * grid_rows {
                            self.grid = vec![TerminalCell::default(); grid_cols * grid_rows];
                        }
                        for (i, cell) in cells.iter().take(self.grid.len()).enumerate() {
                            self.grid[i] = TerminalCell::from(*cell);
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

impl Drop for IpcPluginSession {
    fn drop(&mut self) {
        self.expected_stop.store(true, Ordering::Relaxed);
        if let Some(ref mut socket) = self.socket {
            let _ = IpcCommand::Stop.write_to(&mut *socket);
        }
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(ref socket_path) = self.socket_path {
            let _ = fs::remove_file(socket_path);
        }
    }
}
