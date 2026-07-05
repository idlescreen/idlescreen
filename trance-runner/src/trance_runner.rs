//! Cross-platform screensaver runtime host.
//! Vendored from `runner::trance_runner`.

use crate::core::screensaver::Screensaver;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::EnvFilter;

#[path = "args.rs"]
mod args;
#[path = "platform_helpers.rs"]
mod platform_helpers;
#[path = "renderer.rs"]
mod renderer;
#[path = "terminal_guard.rs"]
mod terminal_guard;
#[path = "trance_runner_fullscreen.rs"]
mod trance_runner_fullscreen;

pub use args::{Mode, parse_args, print_usage};

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_signal(_sig: libc::c_int) {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .try_init();
}

/// Run the screensaver with the given effect.
#[tracing::instrument(skip_all, fields(name = %name))]
pub fn run_main<S: Screensaver + 'static>(mut saver: S, name: &str) {
    init_tracing();
    let mode = parse_args();
    match mode {
        Mode::Run => {
            let code = match run_fullscreen(&mut saver) {
                Ok(()) => 0,
                Err(_) => 1,
            };
            std::process::exit(code as i32);
        }
        Mode::Configure => {
            tracing::warn!("({name}) configuration dialog: not yet implemented.");
            std::process::exit(0);
        }
        Mode::Preview => {
            #[cfg(target_os = "windows")]
            {
                let code = run_preview_stub(&mut saver);
                std::process::exit(code as i32);
            }
            #[cfg(not(target_os = "windows"))]
            {
                let code = match run_fullscreen(&mut saver) {
                    Ok(()) => 0,
                    Err(_) => 1,
                };
                std::process::exit(code as i32);
            }
        }
        Mode::ShowUsage => {
            print_usage(name);
            std::process::exit(0);
        }
    }
}

#[cfg(target_os = "windows")]
fn run_preview_stub(_saver: &mut dyn Screensaver) -> isize {
    tracing::warn!("Windows preview mode is not supported in console mode.");
    0
}

/// Loads a screensaver plugin dynamic library and runs it fullscreen.
#[tracing::instrument(skip_all, fields(plugin_path = %plugin_path))]
pub fn run_plugin_fullscreen(plugin_path: &str) -> Result<isize, Box<dyn std::error::Error>> {
    use trance_api::ScreensaverInstance;

    unsafe {
        let lib = libloading::Library::new(plugin_path)?;
        let create_fn: libloading::Symbol<unsafe extern "C" fn() -> *mut ScreensaverInstance> =
            lib.get(b"create_screensaver")?;
        let destroy_fn: libloading::Symbol<unsafe extern "C" fn(*mut ScreensaverInstance)> =
            lib.get(b"destroy_screensaver")?;

        let raw_ptr = create_fn();
        if raw_ptr.is_null() {
            return Err("failed to create screensaver instance (null pointer)".into());
        }

        struct PluginGuard {
            ptr: *mut ScreensaverInstance,
            destroy: unsafe extern "C" fn(*mut ScreensaverInstance),
            _lib: libloading::Library,
        }

        impl Drop for PluginGuard {
            fn drop(&mut self) {
                unsafe {
                    (self.destroy)(self.ptr);
                }
            }
        }

        let guard = PluginGuard {
            ptr: raw_ptr,
            destroy: *destroy_fn,
            _lib: lib,
        };

        let exit_code = match run_fullscreen(&mut *(*guard.ptr).inner) {
            Ok(()) => 0,
            Err(_) => 1,
        };
        Ok(exit_code)
    }
}

// ---------------------------------------------------------------------------
// Common Fullscreen Animation Loop
// ---------------------------------------------------------------------------

#[tracing::instrument(skip_all)]
fn run_fullscreen(saver: &mut dyn Screensaver) -> Result<(), Box<dyn std::error::Error>> {
    let terminal = trance_runner_fullscreen::setup_terminal()?;
    let result = trance_runner_fullscreen::drive_plugin_loop(saver);
    trance_runner_fullscreen::teardown_terminal(terminal);
    result
}
