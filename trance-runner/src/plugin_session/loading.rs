// SPDX-License-Identifier: MIT

use libloading::Library;
use std::path::Path;
use std::time::Duration;
use trance_api::ScreensaverInstance;
use trance_upscaler::{FilterMode, FrameUpscaler, resolve_render_scale};

use crate::cell_renderer::CellRenderer;
use crate::launcher::{LaunchMode, PluginError, resolve_saver_binary};

use super::{PluginGuard, PluginSession};

impl PluginSession {
    #[tracing::instrument(skip_all, fields(saver_name = %saver_name))]
    pub fn load(saver_name: &str) -> Result<Self, PluginError> {
        Self::load_with_options(saver_name, &LaunchMode::Daemon, None, None)
    }

    #[tracing::instrument(skip_all, fields(saver_name = %saver_name))]
    pub fn load_with_options(
        saver_name: &str,
        launch_mode: &LaunchMode,
        gpu_enabled: Option<bool>,
        render_scale: Option<f32>,
    ) -> Result<Self, PluginError> {
        let path = resolve_saver_binary(saver_name, launch_mode)?;
        tracing::info!(
            "trance-runner: loading plugin '{}' from {}",
            saver_name,
            path.display()
        );
        Self::load_path_with_options(&path, gpu_enabled, render_scale)
    }

    #[tracing::instrument(skip_all, fields(path = %path.display()))]
    pub fn load_path_with_options(
        path: &Path,
        gpu_enabled: Option<bool>,
        render_scale: Option<f32>,
    ) -> Result<Self, PluginError> {
        let renderer = CellRenderer::new().map_err(|error| {
            PluginError::Io(std::io::Error::new(std::io::ErrorKind::Other, error))
        })?;
        let use_gpu = gpu_enabled.unwrap_or_else(trance_upscaler::gpu_enabled);
        let render_scale = resolve_render_scale(use_gpu, render_scale);
        let upscaler = FrameUpscaler::new(use_gpu, FilterMode::from_env());
        // Note: TRANCE_GPU_ACTIVE was previously set/removed here. This ran from
        // a non-main worker thread, which is undefined behavior on edition 2024
        // (and a soundness footgun even before that). The env var is unread, so
        // the setter is removed entirely.
        if upscaler.using_gpu() {
            tracing::info!(
                "GPU upscale enabled (render scale {:.0}%, adapter: {})",
                render_scale * 100.0,
                upscaler.adapter_name().unwrap_or("unknown")
            );
        } else {
            tracing::info!("CPU upscale (render scale {:.0}%)", render_scale * 100.0);
        }

        unsafe {
            let lib = Library::new(path)?;

            // Eagerly set OS and logo text environment variables so plugins can read them
            // even inside the Landlock sandbox.
            // Propagate export determinism if seed already set by render.
            if std::env::var_os("IDLESCREEN_RENDER_SEED").is_some() {
                std::env::set_var("TRANCE_EXPORT_MODE", "1");
            }
            let sys_info = if trance_api::SystemInfo::export_mode_enabled() {
                trance_api::SystemInfo::export_fixture()
            } else {
                crate::toolkit::sys_info::get_system_info()
            };
            std::env::set_var("TRANCE_OS_NAME", &sys_info.os);
            std::env::set_var("TRANCE_LOGO_TEXT", &sys_info.logo_text);

            // Eagerly load caption font before filesystem is locked
            crate::caption_overlay::init_font();
            if let Err(e) = crate::sandbox::enforce_sandbox() {
                tracing::warn!("Could not enforce Landlock sandbox: {e}");
            }

            // Optional ABI negotiation: missing symbol => assume host-compatible legacy.
            match lib.get::<unsafe extern "C" fn() -> u32>(b"trance_api_version") {
                Ok(ver_fn) => {
                    let found = ver_fn();
                    let expected = trance_api::API_VERSION;
                    if found != expected {
                        return Err(PluginError::ApiVersionMismatch { found, expected });
                    }
                    tracing::info!(found, expected, "plugin API version ok");
                }
                Err(_) => {
                    tracing::debug!(
                        "plugin has no trance_api_version symbol; assuming host-compatible"
                    );
                }
            }

            let create_fn: libloading::Symbol<unsafe extern "C" fn() -> *mut ScreensaverInstance> =
                lib.get(b"create_screensaver")
                    .map_err(|_| PluginError::SymbolMissing("create_screensaver"))?;
            let destroy_fn: libloading::Symbol<unsafe extern "C" fn(*mut ScreensaverInstance)> =
                lib.get(b"destroy_screensaver")
                    .map_err(|_| PluginError::SymbolMissing("destroy_screensaver"))?;

            let raw_ptr = create_fn();
            if raw_ptr.is_null() {
                return Err(PluginError::SymbolMissing("create_screensaver (null)"));
            }

            let guard = PluginGuard {
                ptr: raw_ptr,
                destroy: *destroy_fn,
                _lib: lib,
            };

            Ok(Self {
                plugin: Some(guard),
                plugin_path: path.to_path_buf(),
                renderer,
                upscaler,
                render_scale,
                grid: Vec::new(),
                content_buf: Vec::new(),
                pixel_buf: Vec::new(),
                physics_accumulator: Duration::ZERO,
                physics_duration: Duration::from_secs_f32(1.0 / 120.0),
                time_elapsed: Duration::ZERO,
                simulation_cols: 0,
                simulation_rows: 0,
                hardware_scaling: false,
                watcher: None,
                needs_reload: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            })
        }
    }
}
