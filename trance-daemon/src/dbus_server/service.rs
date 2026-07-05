// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::sync::Arc;

use trance_dbus::DaemonStatus;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;

use super::auth::require_control_peer;
use crate::controller::{DaemonCommand, DaemonController};

pub struct TranceService {
    pub controller: Arc<DaemonController>,
}

#[zbus::interface(name = "com.ubermetroid.Trance")]
#[allow(deprecated)]
impl TranceService {
    async fn get_status(&self) -> zbus::fdo::Result<HashMap<String, OwnedValue>> {
        Ok(self.live_status().to_map())
    }

    async fn enable(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        self.authorize_control(&header).await?;
        if let Err(error) = self.controller.apply_command(DaemonCommand::Enable) {
            tracing::error!(target: "trance_daemon::dbus", "Enable failed: {error:?}");
            return Err(zbus::fdo::Error::Failed(error.to_string()));
        }
        self.sync_config_status();
        Ok(())
    }

    async fn disable(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        self.authorize_control(&header).await?;
        if let Err(error) = self.controller.apply_command(DaemonCommand::Disable) {
            tracing::error!(target: "trance_daemon::dbus", "Disable failed: {error:?}");
            return Err(zbus::fdo::Error::Failed(error.to_string()));
        }
        let _ = self
            .controller
            .command_tx
            .send(DaemonCommand::StopPresentation);
        self.sync_config_status();
        Ok(())
    }

    async fn set_timeout(
        &self,
        minutes: u32,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        self.authorize_control(&header).await?;
        if let Err(error) = self
            .controller
            .apply_command(DaemonCommand::SetTimeout(minutes))
        {
            tracing::error!(target: "trance_daemon::dbus", "SetTimeout failed: {error:?}");
            return Err(zbus::fdo::Error::Failed(error.to_string()));
        }
        let _ = self
            .controller
            .command_tx
            .send(DaemonCommand::SetTimeout(minutes));
        self.sync_config_status();
        Ok(())
    }

    async fn set_saver(
        &self,
        name: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        self.authorize_control(&header).await?;
        let saver = (!name.is_empty()).then(|| name.to_string());
        if let Err(error) = self
            .controller
            .apply_command(DaemonCommand::SetSaver(saver))
        {
            tracing::error!(target: "trance_daemon::dbus", "SetSaver failed: {error:?}");
            return Err(zbus::fdo::Error::Failed(error.to_string()));
        }
        self.sync_config_status();
        Ok(())
    }

    async fn list_savers(&self) -> zbus::fdo::Result<Vec<String>> {
        Ok(crate::controller::installed_savers())
    }

    async fn preview(
        &self,
        name: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        self.authorize_control(&header).await?;
        trance_runner::launcher::sanitize_saver_name(name).ok_or_else(|| {
            zbus::fdo::Error::Failed(format!("unknown or invalid screensaver name: {name}"))
        })?;
        trance_runner::launcher::resolve_saver_binary(
            name,
            &trance_runner::launcher::LaunchMode::Preview,
        )
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;

        let _ = self
            .controller
            .command_tx
            .send(DaemonCommand::Preview(name.to_string()));
        self.controller.mark_dirty();
        Ok(())
    }

    async fn stop_preview(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        self.authorize_control(&header).await?;
        let _ = self
            .controller
            .command_tx
            .send(DaemonCommand::StopPresentation);
        self.controller.mark_dirty();
        Ok(())
    }

    async fn inhibit(
        &self,
        application: &str,
        reason: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<u32> {
        let sender = header.sender().ok_or_else(|| {
            zbus::fdo::Error::Failed("inhibit request missing D-Bus sender".into())
        })?;
        let cookie = self
            .controller
            .inhibitors
            .add(
                application.to_string(),
                reason.to_string(),
                sender.to_owned(),
            )
            .map_err(|error| zbus::fdo::Error::LimitsExceeded(error.to_string()))?;
        let _ = self
            .controller
            .command_tx
            .send(DaemonCommand::StopPresentation);
        self.controller.mark_dirty();
        Ok(cookie)
    }

    async fn un_inhibit(
        &self,
        cookie: u32,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        let sender = header.sender().ok_or_else(|| {
            zbus::fdo::Error::Failed("un_inhibit request missing D-Bus sender".into())
        })?;
        if !self.controller.inhibitors.remove_for_client(cookie, sender) {
            return Err(zbus::fdo::Error::Failed(format!(
                "unknown inhibit cookie for caller: {cookie}"
            )));
        }
        self.controller.mark_dirty();
        Ok(())
    }

    /// DEPRECATED (2026) — no-op.
    ///
    /// The previous `trance-gpu` crate was renamed to `trance-upscaler`
    /// and is now pure CPU code. We keep the D-Bus method to avoid
    /// breaking existing clients (`trance config set gpu ...`,
    /// `trance-applet` UI), but the parameter is ignored.
    #[deprecated(note = "GPU upscaler removed; this method is a no-op")]
    #[allow(deprecated)]
    async fn set_gpu_enabled(
        &self,
        _enabled: bool,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        self.authorize_control(&header).await?;
        tracing::warn!(target: "trance_daemon::deprecation", "set_gpu_enabled called; GPU upscaler removed in 2026");
        Ok(())
    }

    async fn set_show_fps_overlay(
        &self,
        enabled: bool,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        self.authorize_control(&header).await?;
        if let Err(error) = self
            .controller
            .apply_command(DaemonCommand::SetShowFpsOverlay(enabled))
        {
            tracing::error!(target: "trance_daemon::dbus", "SetShowFpsOverlay failed: {error:?}");
            return Err(zbus::fdo::Error::Failed(error.to_string()));
        }
        self.sync_config_status();
        Ok(())
    }

    async fn set_render_scale(
        &self,
        scale: f64,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        self.authorize_control(&header).await?;
        if let Err(error) = self
            .controller
            .apply_command(DaemonCommand::SetRenderScale(Some(scale as f32)))
        {
            tracing::error!(target: "trance_daemon::dbus", "SetRenderScale failed: {error:?}");
            return Err(zbus::fdo::Error::Failed(error.to_string()));
        }
        self.sync_config_status();
        Ok(())
    }

    #[zbus(signal)]
    pub(super) async fn status_changed(
        signal_emitter: &SignalEmitter<'_>,
        status: HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;
}

impl TranceService {
    async fn authorize_control(&self, header: &zbus::message::Header<'_>) -> zbus::fdo::Result<()> {
        require_control_peer(
            &self
                .controller
                .dbus_connection()
                .map_err(zbus::fdo::Error::Failed)?,
            header,
        )
        .await
    }

    #[tracing::instrument(skip(self), level = "trace")]
    fn live_status(&self) -> DaemonStatus {
        let mut status = self.controller.status.lock().unwrap().clone();
        status.session_locked = self
            .controller
            .session_locked
            .load(std::sync::atomic::Ordering::Relaxed);
        status.inhibited = self.controller.inhibitors.is_inhibited();
        status
    }

    #[tracing::instrument(skip(self))]
    fn sync_config_status(&self) {
        let config = self.controller.config.lock().unwrap().clone();
        {
            let mut status = self.controller.status.lock().unwrap();
            status.idle_enabled = config.idle_enabled;
            status.idle_timeout_mins = config.idle_timeout_mins;
            status.active_saver = config.active_saver.clone().unwrap_or_default();
            #[allow(deprecated)]
            {
                status.gpu_enabled = false;
            }
            status.show_fps_overlay = config.show_fps_overlay;
            status.render_scale = config
                .render_scale
                .map(|s| s.to_string())
                .unwrap_or_default();
        }
        self.controller.mark_dirty();
        self.controller.publish_status_if_dirty();
    }
}
