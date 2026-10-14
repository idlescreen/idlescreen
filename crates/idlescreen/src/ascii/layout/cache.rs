// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 8-entry LRU cache storing pre-rendered banners to avoid re-computations and allocations.

use super::{RenderedBanner, render_banner};

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
        self.entries
            .push((key, banner.lines, banner.width, banner.height));
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
