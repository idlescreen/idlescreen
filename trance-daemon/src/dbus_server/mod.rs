mod auth;
mod legacy;
mod screensaver;
mod service;
mod watchers;

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use trance_dbus::{LEGACY_OBJECT_PATH, LEGACY_SERVICE_NAME, OBJECT_PATH, SERVICE_NAME};

use crate::controller::DaemonController;
use crate::lock_monitor;

use service::TranceService;

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
        .serve_at(
            LEGACY_OBJECT_PATH,
            legacy::TranceLegacyService {
                controller: controller.clone(),
            },
        )
        .map_err(|error| error.to_string())?
        .serve_at(
            "/org/freedesktop/ScreenSaver",
            screensaver::ScreenSaverService {
                controller: controller.clone(),
            },
        )
        .map_err(|error| error.to_string())?
        .build()
        .await
        .map_err(|error| error.to_string())?;

    let _ = connection.request_name(LEGACY_SERVICE_NAME).await;
    let _ = connection.request_name("org.freedesktop.ScreenSaver").await;

    controller.set_dbus_connection(connection.clone());

    tracing::info!("exporting D-Bus service {SERVICE_NAME} and {LEGACY_SERVICE_NAME}");

    tokio::spawn(lock_monitor::watch_session_lock(
        controller.session_locked.clone(),
        controller.shutdown.clone(),
    ));

    tokio::spawn(watchers::watch_inhibitor_clients(
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

pub async fn emit_status_changes(
    connection: zbus::Connection,
    receiver: std::sync::mpsc::Receiver<trance_dbus::DaemonStatus>,
    shutdown: Arc<std::sync::atomic::AtomicBool>,
) {
    while !shutdown.load(Ordering::Relaxed) {
        match receiver.recv_timeout(Duration::from_millis(200)) {
            Ok(status) => {
                if let Ok(emitter) =
                    zbus::object_server::SignalEmitter::new(&connection, OBJECT_PATH)
                {
                    let _ = TranceService::status_changed(&emitter, status.to_map()).await;
                }
                if let Ok(emitter) =
                    zbus::object_server::SignalEmitter::new(&connection, LEGACY_OBJECT_PATH)
                {
                    let _ = legacy::TranceLegacyService::status_changed(&emitter, status.to_map()).await;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}
