// SPDX-License-Identifier: MIT

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use zbus::names::UniqueName;

#[derive(Debug, Clone)]
pub struct Inhibitor {
    pub cookie: u32,
    pub application_name: String,
    pub reason: String,
    pub client: UniqueName<'static>,
}

#[derive(Debug)]
pub struct InhibitorState {
    inhibitors: Mutex<Vec<Inhibitor>>,
    last_cookie: AtomicU32,
}

impl InhibitorState {
    pub fn new() -> Self {
        Self {
            inhibitors: Mutex::new(Vec::new()),
            last_cookie: AtomicU32::new(0),
        }
    }

    pub fn is_inhibited(&self) -> bool {
        !self.inhibitors.lock().unwrap().is_empty()
    }

    pub fn add(
        &self,
        application_name: String,
        reason: String,
        client: UniqueName<'static>,
    ) -> u32 {
        let cookie = self.last_cookie.fetch_add(1, Ordering::Relaxed) + 1;
        let mut inhibitors = self.inhibitors.lock().unwrap();
        inhibitors.push(Inhibitor {
            cookie,
            application_name,
            reason,
            client,
        });
        cookie
    }

    pub fn remove(&self, cookie: u32) -> bool {
        let mut inhibitors = self.inhibitors.lock().unwrap();
        if let Some(index) = inhibitors.iter().position(|entry| entry.cookie == cookie) {
            inhibitors.remove(index);
            true
        } else {
            false
        }
    }

    pub fn remove_client(&self, client: &UniqueName<'_>) {
        let mut inhibitors = self.inhibitors.lock().unwrap();
        inhibitors.retain(|entry| entry.client != *client);
    }
}