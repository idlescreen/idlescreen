// SPDX-License-Identifier: MIT

use anyhow::{Context, anyhow};
use trance_runner::launcher::{LaunchMode, resolve_saver_binary, sanitize_saver_name};

use super::{DaemonCommand, DaemonController};
use crate::config::DaemonConfig;

impl DaemonController {
    pub fn mutate_config<F>(&self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut DaemonConfig),
    {
        let mut config = self
            .config
            .lock()
            .expect("controller config mutex poisoned");
        f(&mut config);
        config.save().context("saving config")?;
        self.mark_dirty();
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(command = ?command))]
    pub fn apply_command(&self, command: DaemonCommand) -> anyhow::Result<()> {
        match command {
            DaemonCommand::Enable => self
                .mutate_config(|c| c.idle_enabled = true)
                .context("persisting config after Enable command"),
            DaemonCommand::Disable => self
                .mutate_config(|c| c.idle_enabled = false)
                .context("persisting config after Disable command"),
            DaemonCommand::SetTimeout(minutes) => {
                validate_idle_timeout(minutes)?;
                self.mutate_config(|c| c.idle_timeout_mins = minutes)
                    .context("persisting config after SetTimeout command")
            }
            DaemonCommand::SetSaver(name) => {
                validate_saver_choice(name.as_deref())?;
                self.mutate_config(|c| c.active_saver = name)
                    .context("persisting config after SetSaver command")
            }
            DaemonCommand::SetShowFpsOverlay(enabled) => self
                .mutate_config(|c| c.show_fps_overlay = enabled)
                .context("persisting config after SetShowFpsOverlay command"),
            DaemonCommand::SetRenderScale(scale) => {
                let stored = normalize_render_scale(scale)?;
                self.mutate_config(|c| c.render_scale = stored)
                    .context("persisting config after SetRenderScale command")
            }
            DaemonCommand::Preview(_) | DaemonCommand::StopPresentation => Ok(()),
        }
    }
}

fn validate_idle_timeout(minutes: u32) -> anyhow::Result<()> {
    if minutes == 0 || minutes > 240 {
        anyhow::bail!("timeout must be between 1 and 240 minutes");
    }
    Ok(())
}

fn validate_saver_choice(saver: Option<&str>) -> anyhow::Result<()> {
    if let Some(name) = saver {
        sanitize_saver_name(name)
            .ok_or_else(|| anyhow!("unknown or invalid screensaver name: {name}"))?;
        resolve_saver_binary(name, &LaunchMode::Daemon)
            .with_context(|| format!("resolving saver binary for {name}"))?;
    }
    Ok(())
}

fn validate_render_scale(scale: f32) -> anyhow::Result<()> {
    if !scale.is_finite() || !(0.25..=1.0).contains(&scale) {
        anyhow::bail!("render_scale must be between 0.25 and 1.0");
    }
    Ok(())
}

fn normalize_render_scale(scale: Option<f32>) -> anyhow::Result<Option<f32>> {
    let stored = match scale {
        None => None,
        Some(value) if value <= 0.0 => None,
        Some(value) => {
            validate_render_scale(value)?;
            Some(value)
        }
    };
    Ok(stored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DaemonConfig;

    static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn test_controller() -> (
        DaemonController,
        std::path::PathBuf,
        std::sync::MutexGuard<'static, ()>,
    ) {
        let guard = TEST_MUTEX.lock().unwrap();
        let temp = std::env::temp_dir().join(format!(
            "trance-daemon-cmd-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&temp).unwrap();
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &temp);
        }
        let controller = DaemonController::new(DaemonConfig::default());
        (controller, temp, guard)
    }

    #[test]
    fn enable_sets_idle_true() {
        let (c, _tmp, _guard) = test_controller();
        c.apply_command(DaemonCommand::Enable).unwrap();
        assert!(c.config.lock().unwrap().idle_enabled);
    }

    #[test]
    fn disable_sets_idle_false() {
        let (c, _tmp, _guard) = test_controller();
        c.apply_command(DaemonCommand::Disable).unwrap();
        assert!(!c.config.lock().unwrap().idle_enabled);
    }

    #[test]
    fn set_timeout_validates_range() {
        let (c, _tmp, _guard) = test_controller();
        assert!(c.apply_command(DaemonCommand::SetTimeout(0)).is_err());
        assert!(c.apply_command(DaemonCommand::SetTimeout(241)).is_err());
        assert!(c.apply_command(DaemonCommand::SetTimeout(10)).is_ok());
        assert_eq!(c.config.lock().unwrap().idle_timeout_mins, 10);
    }

    #[test]
    fn set_timeout_accepts_boundaries() {
        let (c, _tmp, _guard) = test_controller();
        assert!(c.apply_command(DaemonCommand::SetTimeout(1)).is_ok());
        assert!(c.apply_command(DaemonCommand::SetTimeout(240)).is_ok());
        assert_eq!(c.config.lock().unwrap().idle_timeout_mins, 240);
    }

    #[test]
    fn set_render_scale_zero_normalizes_to_none() {
        let (c, _tmp, _guard) = test_controller();
        c.apply_command(DaemonCommand::SetRenderScale(Some(0.0)))
            .unwrap();
        assert!(c.config.lock().unwrap().render_scale.is_none());
    }

    #[test]
    fn set_render_scale_rejects_out_of_range() {
        let (c, _tmp, _guard) = test_controller();
        assert!(
            c.apply_command(DaemonCommand::SetRenderScale(Some(2.0)))
                .is_err()
        );
        assert!(
            c.apply_command(DaemonCommand::SetRenderScale(Some(0.1)))
                .is_err()
        );
    }

    #[test]
    fn set_render_scale_accepts_in_range() {
        let (c, _tmp, _guard) = test_controller();
        c.apply_command(DaemonCommand::SetRenderScale(Some(0.5)))
            .unwrap();
        assert_eq!(c.config.lock().unwrap().render_scale, Some(0.5));
    }

    #[test]
    fn set_render_scale_accepts_none() {
        let (c, _tmp, _guard) = test_controller();
        c.apply_command(DaemonCommand::SetRenderScale(None))
            .unwrap();
        assert!(c.config.lock().unwrap().render_scale.is_none());
    }

    #[test]
    fn set_show_fps_overlay_toggles() {
        let (c, _tmp, _guard) = test_controller();
        c.apply_command(DaemonCommand::SetShowFpsOverlay(true))
            .unwrap();
        assert!(c.config.lock().unwrap().show_fps_overlay);
        c.apply_command(DaemonCommand::SetShowFpsOverlay(false))
            .unwrap();
        assert!(!c.config.lock().unwrap().show_fps_overlay);
    }

    #[test]
    fn preview_and_stop_are_no_ops() {
        let (c, _tmp, _guard) = test_controller();
        assert!(
            c.apply_command(DaemonCommand::Preview("beams".into()))
                .is_ok()
        );
        assert!(c.apply_command(DaemonCommand::StopPresentation).is_ok());
    }

    #[test]
    fn mark_dirty_sets_status_dirty_flag() {
        let (c, _tmp, _guard) = test_controller();
        // Reset the flag (may be set by other tests/operations)
        let _ = c.take_dirty();
        c.mark_dirty();
        assert!(c.take_dirty());
        assert!(!c.take_dirty());
    }

    #[test]
    fn validate_idle_timeout_bounds() {
        assert!(validate_idle_timeout(0).is_err());
        assert!(validate_idle_timeout(241).is_err());
        assert!(validate_idle_timeout(1).is_ok());
        assert!(validate_idle_timeout(240).is_ok());
        assert!(validate_idle_timeout(120).is_ok());
    }

    #[test]
    fn validate_render_scale_in_range() {
        assert!(validate_render_scale(0.25).is_ok());
        assert!(validate_render_scale(1.0).is_ok());
        assert!(validate_render_scale(0.5).is_ok());
        assert!(validate_render_scale(0.24).is_err());
        assert!(validate_render_scale(1.01).is_err());
        assert!(validate_render_scale(f32::NAN).is_err());
    }

    #[test]
    fn normalize_render_scale_handles_edges() {
        assert!(normalize_render_scale(None).unwrap().is_none());
        assert!(normalize_render_scale(Some(0.0)).unwrap().is_none());
        assert!(normalize_render_scale(Some(-1.0)).unwrap().is_none());
        assert_eq!(normalize_render_scale(Some(0.5)).unwrap(), Some(0.5));
        assert!(normalize_render_scale(Some(2.0)).is_err());
    }
}
