// SPDX-License-Identifier: MIT

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::time::Duration;

use crate::appearance::OverlayAppearance;
use crate::output::{OutputLayout, OutputRegistry};
use crate::overlay::{spawn_event_thread, PresenterCommand};

/// Presents fullscreen Wayland overlays on top of the desktop.
pub struct OverlayPresenter {
    command_tx: Sender<PresenterCommand>,
    visible: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    outputs: OutputRegistry,
}

impl OverlayPresenter {
    /// Connect to the compositor and prepare the overlay session.
    pub fn new() -> Option<Self> {
        if !Self::is_available() {
            return None;
        }

        let (ready_tx, ready_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();
        let visible = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));
        let outputs = OutputRegistry::new();

        spawn_event_thread(
            ready_tx,
            command_rx,
            visible.clone(),
            shutdown.clone(),
            outputs.clone(),
        );

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => Some(Self {
                command_tx,
                visible,
                shutdown,
                outputs,
            }),
            _ => None,
        }
    }

    pub fn is_available() -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
    }

    pub fn is_visible(&self) -> bool {
        self.visible.load(Ordering::SeqCst)
    }

    pub fn output_layouts(&self) -> Vec<OutputLayout> {
        self.outputs.layouts()
    }

    pub fn show(&self, appearance: OverlayAppearance) {
        let _ = self.command_tx.send(PresenterCommand::ShowSolid(appearance));
    }

    pub fn show_screensaver(&self) {
        let _ = self
            .command_tx
            .send(PresenterCommand::ShowScreensaver);
    }

    pub fn submit_frame(&self, output_id: u32, width: u32, height: u32, pixels: Vec<u8>) {
        let _ = self.command_tx.send(PresenterCommand::UpdateFrame {
            output_id,
            width,
            height,
            pixels,
        });
    }

    pub fn hide(&self) {
        let _ = self.command_tx.send(PresenterCommand::Hide);
    }
}

impl Drop for OverlayPresenter {
    fn drop(&mut self) {
        let _ = self.command_tx.send(PresenterCommand::Hide);
        self.shutdown.store(true, Ordering::Relaxed);
    }
}