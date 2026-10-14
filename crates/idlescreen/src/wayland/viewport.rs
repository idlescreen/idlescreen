// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Viewport hardware scaling via wp_viewporter.

use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;

/// Viewport abstraction ensuring strict hardware destination scaling
/// and compliant destruction ordering.
pub struct SurfaceViewport {
    viewport: Option<WpViewport>,
}

impl SurfaceViewport {
    pub fn new(viewport: Option<WpViewport>) -> Self {
        Self { viewport }
    }

    pub fn has_viewport(&self) -> bool {
        self.viewport.is_some()
    }

    pub fn set_scaling(&self, logical_w: i32, logical_h: i32, buf_w: f64, buf_h: f64) {
        if let Some(ref vp) = self.viewport
            && logical_w > 0
            && logical_h > 0
            && buf_w > 0.0
            && buf_h > 0.0
        {
            vp.set_source(0.0, 0.0, buf_w, buf_h);
            vp.set_destination(logical_w, logical_h);
        }
    }

    /// Explicitly destroys the viewport before role and surface destruction.
    ///
    /// Essential to avoid `wp_viewport::error::out_of_sequences`.
    pub fn destroy(&mut self) {
        if let Some(vp) = self.viewport.take() {
            vp.destroy();
        }
    }
}

impl Drop for SurfaceViewport {
    fn drop(&mut self) {
        self.destroy();
    }
}
