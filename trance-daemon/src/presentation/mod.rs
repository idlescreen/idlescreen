// SPDX-License-Identifier: MIT

//! Plugin screensaver presentation on Wayland layer-shell overlays.
//!
//! A dedicated thread loads the selected plugin, renders frames at the target
//! refresh rate, and submits BGRA buffers per output. Display modes (expand,
//! mirror, primary-only, span) are handled in the frame loop submodule.

mod frame_loop;
mod layout;
mod overlays;
mod plugin_loop;
mod refresh;
mod ipc_session;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use trance_runner::launcher::LaunchMode;
use wayland_present::OverlayPresenter;

pub use plugin_loop::run_plugin_loop;

#[derive(Clone)]
pub struct PresentationOptions {
    pub gpu_enabled: bool,
    pub show_fps_overlay: bool,
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
                tracing::error!("plugin presentation ended: {error}");
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
