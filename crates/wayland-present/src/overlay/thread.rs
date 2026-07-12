// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::os::fd::AsFd;
use std::os::unix::io::AsRawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use wayland_client::Connection;

use crate::appearance::OverlayAppearance;
use crate::output::OutputRegistry;

use super::state::SessionState;

pub enum PresenterCommand {
    ShowSolid(OverlayAppearance),
    ShowScreensaver,
    UpdateFrame {
        output_id: u32,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
    Hide,
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_event_thread(
    ready_tx: Sender<Result<(), &'static str>>,
    command_rx: Receiver<PresenterCommand>,
    visible: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    outputs: OutputRegistry,
    is_alive: Arc<AtomicBool>,
    supports_scaling: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        if let Err(message) = run_event_loop(
            ready_tx,
            command_rx,
            visible,
            shutdown,
            outputs,
            supports_scaling,
        ) {
            tracing::warn!("wayland-present: {message}");
        }
        is_alive.store(false, Ordering::SeqCst);
    });
}

#[tracing::instrument(skip_all)]
#[allow(clippy::needless_pass_by_value)]
fn run_event_loop(
    ready_tx: Sender<Result<(), &'static str>>,
    command_rx: Receiver<PresenterCommand>,
    visible: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    outputs: OutputRegistry,
    supports_scaling: Arc<AtomicBool>,
) -> Result<(), &'static str> {
    let connection = Connection::connect_to_env().map_err(|_| "failed to connect to Wayland")?;

    let mut event_queue = connection.new_event_queue();
    let queue = event_queue.handle();
    let _registry = connection.display().get_registry(&queue, ());

    let mut state = SessionState {
        compositor: None,
        shm: None,
        layer_shell: None,
        viewporter: None,
        seat: None,
        pointer: None,
        pointer_serial: 0,
        outputs: Vec::new(),
        overlays: HashMap::new(),
        appearance: None,
        screensaver_mode: false,
        visible,
        output_registry: outputs,
        output_refresh_hz: HashMap::new(),
        output_origin: HashMap::new(),
        output_mode_size: HashMap::new(),
        output_scale: HashMap::new(),
        dismiss_grace_until: None,
        queue: queue.clone(),
    };

    event_queue
        .roundtrip(&mut state)
        .map_err(|_| "initial registry roundtrip failed")?;

    if state.viewporter.is_some() {
        supports_scaling.store(true, Ordering::SeqCst);
    }

    if state.layer_shell.is_none() {
        let _ = ready_tx.send(Err("compositor does not expose zwlr_layer_shell_v1"));
        return Err("compositor does not expose zwlr_layer_shell_v1");
    }

    if state.compositor.is_none() || state.shm.is_none() {
        let _ = ready_tx.send(Err("compositor missing wl_compositor or wl_shm"));
        return Err("compositor missing wl_compositor or wl_shm");
    }

    let _ = ready_tx.send(Ok(()));

    let fd = connection.as_fd().as_raw_fd();
    let mut poll_fd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };

    while !shutdown.load(Ordering::Relaxed) {
        let _ = connection.flush();
        dispatch_pending_events(&connection, &mut event_queue, &mut state, &mut poll_fd)?;
        apply_commands(&mut state, &command_rx);
    }

    state.hide();
    Ok(())
}

#[tracing::instrument(skip_all)]
fn dispatch_pending_events(
    connection: &Connection,
    event_queue: &mut wayland_client::EventQueue<SessionState>,
    state: &mut SessionState,
    poll_fd: &mut libc::pollfd,
) -> Result<(), &'static str> {
    if let Some(guard) = event_queue.prepare_read() {
        let _ = connection.flush();

        let poll_result = unsafe { libc::poll(poll_fd, 1, 100) };
        if poll_result > 0 {
            if poll_fd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
                return Err("Wayland connection closed");
            }

            if poll_fd.revents & libc::POLLIN != 0 {
                guard.read().map_err(|_| "failed to read Wayland events")?;
                event_queue
                    .dispatch_pending(state)
                    .map_err(|_| "failed to dispatch Wayland events")?;
            }
        } else if poll_result < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() != std::io::ErrorKind::Interrupted {
                return Err("poll failed");
            }
        }
    } else {
        event_queue
            .dispatch_pending(state)
            .map_err(|_| "failed to dispatch Wayland events")?;
    }

    Ok(())
}

fn apply_commands(state: &mut SessionState, command_rx: &Receiver<PresenterCommand>) {
    while let Ok(command) = command_rx.try_recv() {
        match command {
            PresenterCommand::ShowSolid(appearance) => state.show_solid(appearance),
            PresenterCommand::ShowScreensaver => state.show_screensaver(),
            PresenterCommand::UpdateFrame {
                output_id,
                width,
                height,
                pixels,
            } => state.update_frame(output_id, width, height, pixels),
            PresenterCommand::Hide => state.hide(),
        }
    }
}
