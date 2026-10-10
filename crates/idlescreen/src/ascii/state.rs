// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Tri-rotation text mutation state and 8-entry LRU font atlas cache.

use super::layout::{RenderedBanner, render_banner};
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
            "os" => TextMode::Os,
            "de" => TextMode::De,
            "kernel" => TextMode::Kernel,
            "custom" => TextMode::Custom,
            _ => TextMode::Cycle,
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
        match exclude {
            Some(BrandTarget::Os) => match (rng.next_u64() >> 33) & 1 {
                0 => BrandTarget::De,
                _ => BrandTarget::Kernel,
            },
            Some(BrandTarget::De) => match (rng.next_u64() >> 33) & 1 {
                0 => BrandTarget::Os,
                _ => BrandTarget::Kernel,
            },
            Some(BrandTarget::Kernel) => match (rng.next_u64() >> 33) & 1 {
                0 => BrandTarget::Os,
                _ => BrandTarget::De,
            },
            None => match (rng.next_u64() >> 33) % 3 {
                0 => BrandTarget::Os,
                1 => BrandTarget::De,
                _ => BrandTarget::Kernel,
            },
        }
    }

    pub fn next(self) -> Self {
        match self {
            BrandTarget::Os => BrandTarget::De,
            BrandTarget::De => BrandTarget::Kernel,
            BrandTarget::Kernel => BrandTarget::Os,
        }
    }
}

/// 8-entry LRU cache storing pre-rendered banners to avoid re-computations and allocations.
pub struct LruFontAtlasCache {
    entries: Vec<((String, usize, usize), Vec<String>, usize, usize)>,
    capacity: usize,
}

impl Default for LruFontAtlasCache {
    fn default() -> Self {
        Self::new(8)
    }
}

impl LruFontAtlasCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            capacity: capacity.max(1),
        }
    }

    pub fn get(&mut self, key: &(String, usize, usize)) -> Option<RenderedBanner> {
        if let Some(pos) = self.entries.iter().position(|(k, _, _, _)| k == key) {
            let entry = self.entries.remove(pos);
            let res = RenderedBanner {
                lines: entry.1.clone(),
                width: entry.2,
                height: entry.3,
            };
            self.entries.push(entry);
            Some(res)
        } else {
            None
        }
    }

    pub fn put(&mut self, key: (String, usize, usize), banner: RenderedBanner) {
        if let Some(pos) = self.entries.iter().position(|(k, _, _, _)| k == &key) {
            self.entries.remove(pos);
        } else if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }
        self.entries.push((key, banner.lines, banner.width, banner.height));
    }

    pub fn get_or_render(
        &mut self,
        text: &str,
        subtitle: Option<&str>,
        cols: usize,
        rows: usize,
    ) -> RenderedBanner {
        let key = (text.to_string(), cols, rows);
        if let Some(cached) = self.get(&key) {
            cached
        } else {
            let rendered = render_banner(text, subtitle, cols, rows);
            let copy = RenderedBanner {
                lines: rendered.lines.clone(),
                width: rendered.width,
                height: rendered.height,
            };
            self.put(key, rendered);
            copy
        }
    }
}

pub struct TriRotationState {
    pub mode: TextMode,
    pub custom_text: Option<String>,
    pub current_target: BrandTarget,
    pub os_name: String,
    pub de_name: String,
    pub kernel_version: String,
    pub atlas_cache: LruFontAtlasCache,
}

impl Default for TriRotationState {
    fn default() -> Self {
        Self::new(TextMode::Cycle, None)
    }
}

impl TriRotationState {
    pub fn new(mode: TextMode, custom_text: Option<String>) -> Self {
        Self::new_with_seed(mode, custom_text, LcgRng::new_random().next_u64())
    }

    pub fn new_with_seed(mode: TextMode, custom_text: Option<String>, seed: u64) -> Self {
        let mut rng = LcgRng::new(seed);
        let current_target = BrandTarget::random_excluding(&mut rng, None);
        let os_name = detect_os_name();
        let de_name = detect_desktop_env();
        let kernel_version = detect_kernel_version();

        Self {
            mode,
            custom_text,
            current_target,
            os_name,
            de_name,
            kernel_version,
            atlas_cache: LruFontAtlasCache::new(8),
        }
    }

    pub fn active_text(&self) -> String {
        if let Some(ref custom) = self.custom_text
            && !custom.trim().is_empty() {
                return custom.trim().to_string();
            }
        match self.mode {
            TextMode::Custom => self.custom_text.clone().unwrap_or_else(|| self.os_name.clone()),
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
        if self.mode == TextMode::Cycle {
            self.current_target = BrandTarget::random_excluding(rng, Some(self.current_target));
        }
    }
}
