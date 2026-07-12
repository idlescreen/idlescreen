// SPDX-License-Identifier: MIT

use std::os::unix::net::UnixStream;
use std::time::Duration;
use trance_ipc::{SharedMemory, FfiTerminalCell, IpcCommand, IpcResponse, compute_shm_size};
use trance_runner::plugin_session::PluginSession;
use trance_runner::launcher::LaunchMode;

pub fn run_ipc_runner(
    saver_name: &str,
    socket_path: &str,
    shm_name: &str,
    cols: usize,
    rows: usize,
    gpu_enabled: bool,
    render_scale: Option<f32>,
) -> Result<(), String> {
    tracing::info!(
        "IPC Runner starting for saver '{}', cols: {}, rows: {}, gpu: {}, scale: {:?}",
        saver_name,
        cols,
        rows,
        gpu_enabled,
        render_scale
    );

    // 1. Connect to control socket
    let mut socket = UnixStream::connect(socket_path)
        .map_err(|e| format!("failed to connect to socket {}: {}", socket_path, e))?;

    // 2. Open shared memory
    let shm_size = compute_shm_size(cols, rows);
    let shm = SharedMemory::open(shm_name, shm_size)
        .map_err(|e| format!("failed to open shm {}: {}", shm_name, e))?;

    // 3. Load screensaver plugin
    let mut session = PluginSession::load_with_options(
        saver_name,
        &LaunchMode::Daemon,
        Some(gpu_enabled),
        render_scale,
    )
    .map_err(|e| format!("failed to load plugin {}: {}", saver_name, e))?;

    if let Err(e) = session.start_watcher() {
        tracing::warn!("Failed to start screensaver file watcher: {:?}", e);
    }

    // 4. Send Ready response
    IpcResponse::Ready.write_to(&mut socket)
        .map_err(|e| format!("failed to send Ready response: {}", e))?;

    // 5. Command loop
    loop {
        if let Ok(true) = session.poll_reload() {
            tracing::info!("Screensaver reloaded successfully.");
        }

        let command = match IpcCommand::read_from(&mut socket) {
            Ok(cmd) => cmd,
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                tracing::info!("IPC control socket closed, runner exiting.");
                break;
            }
            Err(e) => {
                return Err(format!("failed to read IPC command: {}", e));
            }
        };

        match command {
            IpcCommand::Init { cols: c, rows: r } => {
                session.init(c as usize, r as usize);
                IpcResponse::Ack.write_to(&mut socket)
                    .map_err(|e| format!("failed to send Ack: {}", e))?;
            }
            IpcCommand::TickAndDraw { dt_micros } => {
                session.tick(Duration::from_micros(dt_micros));
                let scanlines = session.draw_frame(cols, rows);

                // Copy cells to shared memory
                let cells = unsafe { shm.cells_mut() };
                for (i, cell) in session.grid().iter().enumerate() {
                    if i < cells.len() {
                        cells[i] = FfiTerminalCell::from(*cell);
                    }
                }

                // Increment frame counter
                unsafe {
                    let header = shm.header_mut();
                    header.frame_counter = header.frame_counter.wrapping_add(1);
                }

                IpcResponse::FrameReady { scanlines }.write_to(&mut socket)
                    .map_err(|e| format!("failed to send FrameReady: {}", e))?;
            }
            IpcCommand::SetSimulationRate { hz } => {
                session.set_simulation_rate(hz);
                IpcResponse::Ack.write_to(&mut socket)
                    .map_err(|e| format!("failed to send Ack: {}", e))?;
            }
            IpcCommand::Stop => {
                tracing::info!("received Stop command, exiting.");
                break;
            }
        }
    }

    Ok(())
}
