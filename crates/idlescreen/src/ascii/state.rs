// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Tri-rotation text mutation state and LRU font atlas cache re-export.

pub use super::layout::LruFontAtlasCache;
use super::sys_info::{detect_desktop_env, detect_kernel_version, detect_os_name};
use crate::cell::LcgRng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextMode {
    Cycle,
    Os,
    De,
    Kernel,
    Custom,
}

impl TextMode {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "os" | "host os" | "host" => Self::Os,
            "de" | "desktop" | "desktop env" | "desktop environment" => Self::De,
            "kernel" | "linux kernel" | "linux" => Self::Kernel,
            "custom" | "custom text" | "text" => Self::Custom,
            _ => Self::Cycle,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cycle => "cycle",
            Self::Os => "os",
            Self::De => "de",
            Self::Kernel => "kernel",
            Self::Custom => "custom",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cycle => "All 3 (Cycle)",
            Self::Os => "Host OS",
            Self::De => "Desktop Environment",
            Self::Kernel => "Linux Kernel",
            Self::Custom => "Custom Text",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrandTarget {
    Os,
    De,
    Kernel,
}

impl BrandTarget {
    pub fn random_excluding(rng: &mut LcgRng, exclude: Option<BrandTarget>) -> Self {
        let idx = usize::from(((rng.next_u64() >> 33) & 1) != 0);
        match exclude {
            Some(Self::Os) => [Self::De, Self::Kernel][idx],
            Some(Self::De) => [Self::Os, Self::Kernel][idx],
            Some(Self::Kernel) => [Self::Os, Self::De][idx],
            None => [Self::Os, Self::De, Self::Kernel][((rng.next_u64() >> 33) % 3) as usize],
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Os => Self::De,
            Self::De => Self::Kernel,
            Self::Kernel => Self::Os,
        }
    }
}

pub struct TriRotationState {
    pub mode: TextMode,
    pub targets: Vec<String>,
    pub current_index: usize,
    pub next_index: usize,
    pub custom_text: Option<String>,
    pub current_target: BrandTarget,
    pub os_name: String,
    pub de_name: String,
    pub kernel_version: String,
    pub atlas_cache: LruFontAtlasCache,
}

impl Default for TriRotationState {
    fn default() -> Self {
        Self::new(TextMode::Os, None)
    }
}

impl TriRotationState {
    pub fn new(mode: TextMode, custom_text: Option<String>) -> Self {
        Self::new_with_seed(mode, custom_text, LcgRng::new_random().next_u64())
    }

    pub fn new_with_seed(mode: TextMode, custom_text: Option<String>, seed: u64) -> Self {
        let mut rng = LcgRng::new(seed);
        let mut state = Self {
            mode,
            targets: Vec::new(),
            current_index: 0,
            next_index: 0,
            custom_text,
            current_target: BrandTarget::random_excluding(&mut rng, None),
            os_name: detect_os_name(),
            de_name: detect_desktop_env(),
            kernel_version: detect_kernel_version(),
            atlas_cache: LruFontAtlasCache::new(8),
        };
        state.sync_targets_from_mode();
        state
    }

    pub fn sync_targets_from_mode(&mut self) {
        self.targets = match self.mode {
            TextMode::Os | TextMode::Cycle => vec!["os".to_string()],
            TextMode::De => vec!["de".to_string()],
            TextMode::Kernel => vec!["kernel".to_string()],
            TextMode::Custom => vec!["custom".to_string()],
        };
        self.current_index = 0;
        self.next_index = usize::from(self.targets.len() > 1);
    }

    pub fn set_targets(&mut self, targets: Vec<String>) {
        if !targets.is_empty() {
            self.targets = targets;
            self.current_index = 0;
            self.next_index = usize::from(self.targets.len() > 1);
        }
    }

    pub fn targets(&self) -> &[String] {
        &self.targets
    }

    pub fn resolve_target_text(&self, target: &str) -> String {
        let t = target.trim();
        match t.to_ascii_lowercase().as_str() {
            "os" | "host" | "host os" => self.os_name.clone(),
            "de" | "desktop" | "desktop env" | "desktop environment" => self.de_name.clone(),
            "kernel" | "linux kernel" | "linux" => "LINUX".to_string(),
            "idlescreen" | "wordmark" => "IDLESCREEN".to_string(),
            "superfedora" | "sf" => {
                if super::sys_info::is_superfedora_machine() {
                    "SUPERFEDORA".to_string()
                } else {
                    self.os_name.clone()
                }
            }
            "custom" | "text" => self
                .custom_text
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map_or_else(|| self.os_name.clone(), ToString::to_string),
            other => {
                if other.eq_ignore_ascii_case(&self.os_name) {
                    self.os_name.clone()
                } else if other.eq_ignore_ascii_case(&self.de_name) {
                    self.de_name.clone()
                } else {
                    let cleaned_os = super::sys_info::clean_os_name(t);
                    if cleaned_os == "IDLESCREEN" {
                        let cleaned_de = super::sys_info::clean_desktop_env(t);
                        if cleaned_de == "DESKTOP" {
                            t.to_ascii_uppercase()
                        } else {
                            cleaned_de
                        }
                    } else {
                        cleaned_os
                    }
                }
            }
        }
    }

    pub fn current_text(&self) -> String {
        if self.targets.is_empty() {
            return self.fallback_text();
        }
        self.resolve_target_text(&self.targets[self.current_index % self.targets.len()])
    }

    pub fn peek_next_text(&self) -> String {
        if self.targets.is_empty() {
            return self.fallback_text();
        }
        self.resolve_target_text(&self.targets[self.next_index % self.targets.len()])
    }

    fn pick_next_index(&self, rng: &mut LcgRng) -> usize {
        if self.targets.len() <= 1 {
            0
        } else if self.targets.len() == 2 {
            1 - (self.current_index % 2)
        } else {
            let offset = 1 + rng.next_usize(self.targets.len() - 1);
            (self.current_index + offset) % self.targets.len()
        }
    }

    pub fn advance_scene(&mut self) {
        if !self.targets.is_empty() {
            self.current_index = self.next_index % self.targets.len();
            let mut rng = LcgRng::new_random();
            self.next_index = self.pick_next_index(&mut rng);
        }
    }

    pub fn active_text(&self) -> String {
        self.current_text()
    }

    fn fallback_text(&self) -> String {
        match self.mode {
            TextMode::Custom => self
                .custom_text
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map_or_else(|| self.os_name.clone(), ToString::to_string),
            TextMode::Os => self.os_name.clone(),
            TextMode::De => self.de_name.clone(),
            TextMode::Kernel => self.kernel_version.clone(),
            TextMode::Cycle => match self.current_target {
                BrandTarget::Os => self.os_name.clone(),
                BrandTarget::De => self.de_name.clone(),
                BrandTarget::Kernel => self.kernel_version.clone(),
            },
        }
    }

    pub fn advance_mutation(&mut self, rng: &mut LcgRng) {
        if !self.targets.is_empty() {
            self.current_index = self.next_index % self.targets.len();
            self.next_index = self.pick_next_index(rng);
        }
        if self.mode == TextMode::Cycle && self.targets.is_empty() {
            self.current_target = BrandTarget::random_excluding(rng, Some(self.current_target));
        }
    }
}
