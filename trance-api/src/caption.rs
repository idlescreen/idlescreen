// SPDX-License-Identifier: MIT

//! Native-resolution caption text published by screensaver plugins.

use std::sync::Mutex;

static CAPTION: Mutex<String> = Mutex::new(String::new());

/// Publish caption text for the presentation layer to draw at native pixel density.
pub fn publish_caption(text: &str) {
    if let Ok(mut caption) = CAPTION.lock() {
        if caption.capacity() < text.len() {
            caption.reserve(text.len());
        }
        caption.clear();
        caption.push_str(text);
    }
}

/// Read the current caption (empty when none is active).
pub fn caption_text() -> String {
    CAPTION
        .lock()
        .map(|caption| caption.clone())
        .unwrap_or_default()
}

/// Clear any published caption.
pub fn clear_caption() {
    if let Ok(mut caption) = CAPTION.lock() {
        caption.clear();
    }
}