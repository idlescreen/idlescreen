// SPDX-License-Identifier: MIT

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures_lite::StreamExt;

#[zbus::proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto"
)]
trait LogindSession {
    #[zbus(property)]
    fn locked_hint(&self) -> zbus::Result<bool>;
}

pub async fn watch_session_lock(session_locked: Arc<AtomicBool>, shutdown: Arc<AtomicBool>) {
    let connection = match zbus::Connection::system().await {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!("trance-daemon: logind lock monitor unavailable: {error}");
            return;
        }
    };

    let proxy = match LogindSessionProxy::new(&connection).await {
        Ok(proxy) => proxy,
        Err(error) => {
            eprintln!("trance-daemon: logind session proxy unavailable: {error}");
            return;
        }
    };

    match proxy.locked_hint().await {
        Ok(locked) => session_locked.store(locked, Ordering::Relaxed),
        Err(error) => eprintln!("trance-daemon: failed to read LockedHint: {error}"),
    }

    let mut stream = proxy.receive_locked_hint_changed().await;

    while !shutdown.load(Ordering::Relaxed) {
        match stream.next().await {
            Some(change) => match change.get().await {
                Ok(locked) => session_locked.store(locked, Ordering::Relaxed),
                Err(error) => eprintln!("trance-daemon: LockedHint update failed: {error}"),
            },
            None => break,
        }
    }
}