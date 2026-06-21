// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use futures_lite::StreamExt;
use trance_dbus::{DaemonStatus, OBJECT_PATH, SERVICE_NAME};
use zbus::fdo::DBusProxy;
use zbus::names::BusName;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;

use crate::controller::{DaemonCommand, DaemonController};
use crate::inhibit::InhibitorState;
use crate::lock_monitor;

pub fn run(controller: Arc<DaemonController>) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .thread_name("trance-dbus")
        .build()
        .map_err(|error| error.to_string())?;

    runtime.block_on(serve(controller))
}

async fn serve(controller: Arc<DaemonController>) -> Result<(), String> {
    let (status_emit_tx, status_emit_rx) = std::sync::mpsc::channel();
    {
        let mut slot = controller.status_emit_tx.lock().unwrap();
        *slot = Some(status_emit_tx);
    }

    let connection = zbus::connection::Builder::session()
        .map_err(|error| error.to_string())?
        .name(SERVICE_NAME)
        .map_err(|error| error.to_string())?
        .serve_at(
            OBJECT_PATH,
            TranceService {
                controller: controller.clone(),
            },
        )
        .map_err(|error| error.to_string())?
        .build()
        .await
        .map_err(|error| error.to_string())?;

    println!("trance-daemon exporting D-Bus service {SERVICE_NAME}");

    tokio::spawn(lock_monitor::watch_session_lock(
        controller.session_locked.clone(),
        controller.shutdown.clone(),
    ));

    tokio::spawn(watch_inhibitor_clients(
        connection.clone(),
        controller.inhibitors.clone(),
        controller.clone(),
    ));

    tokio::spawn(emit_status_changes(
        connection,
        status_emit_rx,
        controller.shutdown.clone(),
    ));

    while !controller.shutdown.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    Ok(())
}

struct TranceService {
    controller: Arc<DaemonController>,
}

#[zbus::interface(name = "com.local76.Trance")]
impl TranceService {
    async fn get_status(&self) -> zbus::fdo::Result<HashMap<String, OwnedValue>> {
        Ok(self.live_status().to_map())
    }

    async fn enable(&self) -> zbus::fdo::Result<()> {
        self.controller
            .apply_command(DaemonCommand::Enable)
            .map_err(|error| zbus::fdo::Error::Failed(error))?;
        self.sync_config_status();
        Ok(())
    }

    async fn disable(&self) -> zbus::fdo::Result<()> {
        self.controller
            .apply_command(DaemonCommand::Disable)
            .map_err(|error| zbus::fdo::Error::Failed(error))?;
        let _ = self
            .controller
            .command_tx
            .send(DaemonCommand::StopPresentation);
        self.sync_config_status();
        Ok(())
    }

    async fn set_timeout(&self, minutes: u32) -> zbus::fdo::Result<()> {
        self.controller
            .apply_command(DaemonCommand::SetTimeout(minutes))
            .map_err(|error| zbus::fdo::Error::Failed(error))?;
        self.sync_config_status();
        Ok(())
    }

    async fn set_saver(&self, name: &str) -> zbus::fdo::Result<()> {
        let saver = if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };
        self.controller
            .apply_command(DaemonCommand::SetSaver(saver))
            .map_err(|error| zbus::fdo::Error::Failed(error))?;
        self.sync_config_status();
        Ok(())
    }

    async fn list_savers(&self) -> zbus::fdo::Result<Vec<String>> {
        Ok(crate::controller::installed_savers())
    }

    async fn preview(&self, name: &str) -> zbus::fdo::Result<()> {
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

    async fn stop_preview(&self) -> zbus::fdo::Result<()> {
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
        let cookie = self.controller.inhibitors.add(
            application.to_string(),
            reason.to_string(),
            sender.to_owned(),
        );
        let _ = self
            .controller
            .command_tx
            .send(DaemonCommand::StopPresentation);
        self.controller.mark_dirty();
        Ok(cookie)
    }

    async fn un_inhibit(&self, cookie: u32) -> zbus::fdo::Result<()> {
        if !self.controller.inhibitors.remove(cookie) {
            return Err(zbus::fdo::Error::Failed(format!(
                "unknown inhibit cookie: {cookie}"
            )));
        }
        self.controller.mark_dirty();
        Ok(())
    }

    async fn set_gpu_enabled(&self, enabled: bool) -> zbus::fdo::Result<()> {
        self.controller
            .apply_command(DaemonCommand::SetGpuEnabled(enabled))
            .map_err(|error| zbus::fdo::Error::Failed(error))?;
        self.sync_config_status();
        Ok(())
    }

    async fn set_show_fps_overlay(&self, enabled: bool) -> zbus::fdo::Result<()> {
        self.controller
            .apply_command(DaemonCommand::SetShowFpsOverlay(enabled))
            .map_err(|error| zbus::fdo::Error::Failed(error))?;
        self.sync_config_status();
        Ok(())
    }

    async fn set_display_mode(&self, mode: &str) -> zbus::fdo::Result<()> {
        self.controller
            .apply_command(DaemonCommand::SetDisplayMode(mode.to_string()))
            .map_err(|error| zbus::fdo::Error::Failed(error))?;
        self.sync_config_status();
        Ok(())
    }

    async fn set_render_scale(&self, scale: f64) -> zbus::fdo::Result<()> {
        self.controller
            .apply_command(DaemonCommand::SetRenderScale(Some(scale as f32)))
            .map_err(|error| zbus::fdo::Error::Failed(error))?;
        self.sync_config_status();
        Ok(())
    }

    #[zbus(signal)]
    async fn status_changed(
        signal_emitter: &SignalEmitter<'_>,
        status: HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;
}

impl TranceService {
    fn live_status(&self) -> DaemonStatus {
        let mut status = self.controller.status.lock().unwrap().clone();
        status.session_locked = self
            .controller
            .session_locked
            .load(std::sync::atomic::Ordering::Relaxed);
        status.inhibited = self.controller.inhibitors.is_inhibited();
        status
    }

    fn sync_config_status(&self) {
        let config = self.controller.config.lock().unwrap().clone();
        {
            let mut status = self.controller.status.lock().unwrap();
            status.idle_enabled = config.idle_enabled;
            status.idle_timeout_mins = config.idle_timeout_mins;
            status.active_saver = config.active_saver.clone().unwrap_or_default();
            status.gpu_enabled = config.gpu_enabled;
            status.show_fps_overlay = config.show_fps_overlay;
            status.display_mode = config.display_mode.clone();
            status.render_scale = config
                .render_scale
                .map(|s| s.to_string())
                .unwrap_or_default();
        }
        self.controller.mark_dirty();
        self.controller.publish_status_if_dirty();
    }
}

async fn emit_status_changes(
    connection: zbus::Connection,
    receiver: std::sync::mpsc::Receiver<DaemonStatus>,
    shutdown: Arc<std::sync::atomic::AtomicBool>,
) {
    while !shutdown.load(Ordering::Relaxed) {
        match receiver.recv_timeout(Duration::from_millis(200)) {
            Ok(status) => {
                if let Ok(emitter) = SignalEmitter::new(&connection, OBJECT_PATH) {
                    let _ = TranceService::status_changed(&emitter, status.to_map()).await;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

async fn watch_inhibitor_clients(
    connection: zbus::Connection,
    inhibitors: Arc<InhibitorState>,
    controller: Arc<DaemonController>,
) {
    let dbus = match DBusProxy::new(&connection).await {
        Ok(proxy) => proxy,
        Err(error) => {
            eprintln!("trance-daemon: failed to watch inhibitor clients: {error}");
            return;
        }
    };

    let mut stream = match dbus.receive_name_owner_changed().await {
        Ok(stream) => stream,
        Err(error) => {
            eprintln!("trance-daemon: failed to subscribe to NameOwnerChanged: {error}");
            return;
        }
    };

    while let Some(event) = stream.next().await {
        let args = match event.args() {
            Ok(args) => args,
            Err(_) => continue,
        };
        if args.new_owner.is_some() {
            continue;
        }
        let BusName::Unique(name) = &args.name else {
            continue;
        };
        inhibitors.remove_client(name);
        controller.mark_dirty();
    }
}