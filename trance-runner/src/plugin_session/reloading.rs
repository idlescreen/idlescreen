// SPDX-License-Identifier: MIT

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use libloading::Library;
use notify::Watcher;
use trance_api::ScreensaverInstance;
use crate::launcher::PluginError;
use super::{PluginGuard, PluginSession};

impl PluginSession {
    #[tracing::instrument(skip(self))]
    pub fn reload(&mut self) -> Result<(), PluginError> {
        tracing::info!("Reloading plugin from {:?}", self.plugin_path);

        if !self.plugin_path.exists() {
            return Err(PluginError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "plugin file not found",
            )));
        }

        let (was_active, was_focused) = if let Some(ref mut old_plugin) = self.plugin {
            let old_saver = old_plugin.saver_mut();
            (old_saver.active(), old_saver.focused())
        } else {
            (true, true)
        };

        self.plugin = None;

        let mut new_guard = unsafe {
            let lib = Library::new(&self.plugin_path)?;

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

            PluginGuard {
                ptr: raw_ptr,
                destroy: *destroy_fn,
                _lib: lib,
            }
        };

        {
            let new_saver = new_guard.saver_mut();
            new_saver.set_active(was_active);
            new_saver.set_focused(was_focused);
            if self.simulation_cols > 0 && self.simulation_rows > 0 {
                new_saver.init(self.simulation_cols, self.simulation_rows);
            }
        }

        self.plugin = Some(new_guard);
        tracing::info!("Plugin successfully reloaded and state restored.");
        Ok(())
    }

    pub fn start_watcher(&mut self) -> Result<(), PluginError> {
        let needs_reload = self.needs_reload.clone();
        let target_filename = self.plugin_path.file_name()
            .ok_or_else(|| PluginError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid plugin path",
            )))?
            .to_os_string();

        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let matches = event.paths.iter().any(|p| p.file_name() == Some(&target_filename));
                if matches && (event.kind.is_modify() || event.kind.is_create()) {
                    tracing::info!("Watcher detected modification for {:?}", target_filename);
                    needs_reload.store(true, Ordering::Relaxed);
                }
            }
        }).map_err(|e| PluginError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        if let Some(parent) = self.plugin_path.parent() {
            watcher.watch(parent, notify::RecursiveMode::NonRecursive)
                .map_err(|e| PluginError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        }

        self.watcher = Some(watcher);
        tracing::info!("Started file watcher on {:?}", self.plugin_path.parent().unwrap());
        Ok(())
    }

    pub fn poll_reload(&mut self) -> Result<bool, PluginError> {
        if self.needs_reload.load(Ordering::Relaxed) {
            self.needs_reload.store(false, Ordering::Relaxed);
            std::thread::sleep(Duration::from_millis(100));
            self.reload()?;
            return Ok(true);
        }
        Ok(false)
    }
}