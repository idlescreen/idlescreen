// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Wayland surface management, layer-shell presentation, output tracking,
//! and session lock handling.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

pub struct OutputInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub scale_120ths: u32,
    pub is_preferred: bool,
}

pub struct OutputRegistry {
    pub epoch: AtomicU64,
    pub outputs: Vec<OutputInfo>,
    pub configured_primary: Option<String>,
}

impl Default for OutputRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputRegistry {
    pub fn new() -> Self {
        Self {
            epoch: AtomicU64::new(0),
            outputs: Vec::new(),
            configured_primary: None,
        }
    }

    pub fn bump_epoch(&self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
    }

    pub fn current_epoch(&self) -> u64 {
        self.epoch.load(Ordering::SeqCst)
    }

    pub fn add_or_update_output(&mut self, info: OutputInfo) {
        if let Some(pos) = self.outputs.iter().position(|o| o.name == info.name) {
            self.outputs[pos] = info;
        } else {
            self.outputs.push(info);
        }
        self.bump_epoch();
    }

    pub fn remove_output(&mut self, name: &str) {
        self.outputs.retain(|o| o.name != name);
        self.bump_epoch();
    }

    /// Selects the primary output according to priority:
    /// (1) configured primary name
    /// (2) compositor preferred hint
    /// (3) largest pixel area
    pub fn primary_output(&self) -> Option<&OutputInfo> {
        if let Some(ref name) = self.configured_primary
            && !name.trim().is_empty()
            && let Some(o) = self.outputs.iter().find(|o| &o.name == name) {
                return Some(o);
            }

        if let Some(o) = self.outputs.iter().find(|o| o.is_preferred) {
            return Some(o);
        }

        self.outputs.iter().max_by_key(|o| o.width * o.height)
    }

    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState {
    Active,
    Parked,
    Locked,
    Sleeping,
}

pub struct PresentationSession {
    pub state: SessionState,
    pub session_locked: AtomicBool,
    pub lock_cleared_at: Option<Instant>,
    pub registry: OutputRegistry,
}

impl Default for PresentationSession {
    fn default() -> Self {
        Self::new()
    }
}

impl PresentationSession {
    pub fn new() -> Self {
        Self {
            state: SessionState::Parked,
            session_locked: AtomicBool::new(false),
            lock_cleared_at: None,
            registry: OutputRegistry::new(),
        }
    }

    pub fn on_session_locked(&mut self) {
        self.session_locked.store(true, Ordering::SeqCst);
        self.state = SessionState::Locked;
    }

    pub fn on_session_unlocked(&mut self) {
        self.session_locked.store(false, Ordering::SeqCst);
        self.lock_cleared_at = Some(Instant::now());
        self.reconcile_state();
    }

    pub fn reconcile_state(&mut self) {
        if self.session_locked.load(Ordering::SeqCst) {
            self.state = SessionState::Locked;
        } else if self.registry.is_empty() {
            self.state = SessionState::Parked;
        } else {
            self.state = SessionState::Active;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_registry_primary_election() {
        let mut reg = OutputRegistry::new();
        assert!(reg.primary_output().is_none());

        reg.add_or_update_output(OutputInfo {
            name: "eDP-1".to_string(),
            width: 1920,
            height: 1080,
            scale_120ths: 120,
            is_preferred: false,
        });

        reg.add_or_update_output(OutputInfo {
            name: "DP-1".to_string(),
            width: 3840,
            height: 2160,
            scale_120ths: 150,
            is_preferred: false,
        });

        // Largest pixel area wins
        assert_eq!(reg.primary_output().unwrap().name, "DP-1");

        // Preferred wins over larger area if no configured
        reg.add_or_update_output(OutputInfo {
            name: "eDP-1".to_string(),
            width: 1920,
            height: 1080,
            scale_120ths: 120,
            is_preferred: true,
        });
        assert_eq!(reg.primary_output().unwrap().name, "eDP-1");

        // Explicit configuration wins over preferred
        reg.configured_primary = Some("DP-1".to_string());
        assert_eq!(reg.primary_output().unwrap().name, "DP-1");

        // Empty configuration falls back to preferred
        reg.configured_primary = Some("   ".to_string());
        assert_eq!(reg.primary_output().unwrap().name, "eDP-1");
    }

    #[test]
    fn test_session_lifecycle() {
        let mut sess = PresentationSession::new();
        assert_eq!(sess.state, SessionState::Parked);

        sess.registry.add_or_update_output(OutputInfo {
            name: "eDP-1".to_string(),
            width: 1920,
            height: 1080,
            scale_120ths: 120,
            is_preferred: true,
        });
        sess.reconcile_state();
        assert_eq!(sess.state, SessionState::Active);

        sess.on_session_locked();
        assert_eq!(sess.state, SessionState::Locked);

        sess.on_session_unlocked();
        assert_eq!(sess.state, SessionState::Active);
    }
}
