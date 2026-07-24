// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Frame and solid-buffer attachment for monitor overlays.

use crate::output::OutputLayout;

use super::types::{MonitorOverlay, SessionState};

impl SessionState {
    /// Publish configured size into the output registry used by presenters.
    pub(super) fn register_configured_output(
        &mut self,
        output_id: u32,
        render_w: u32,
        render_h: u32,
    ) {
        let refresh_rate_hz = self
            .output_refresh_hz
            .get(&output_id)
            .copied()
            .unwrap_or(60);
        let (x, y) = self
            .output_origin
            .get(&output_id)
            .copied()
            .unwrap_or((0, 0));
        let scale = self.output_scale.get(&output_id).copied().unwrap_or(1);
        self.output_registry.upsert(OutputLayout {
            id: output_id,
            width: render_w,
            height: render_h,
            refresh_rate_hz,
            x,
            y,
            scale,
        });
    }

    /// Attach a solid-color buffer for non-screensaver overlay mode.
    #[allow(clippy::cast_possible_wrap)]
    pub(super) fn attach_solid_buffer(&mut self, output_id: u32, render_w: u32, render_h: u32) {
        let Some(appearance) = self.appearance else {
            return;
        };
        let Some(shm) = &self.shm else {
            return;
        };

        let buffer = super::super::buffer::create_solid_buffer(
            shm,
            &self.queue,
            render_w,
            render_h,
            appearance.color,
        );

        let Some(overlay) = self.overlays.get_mut(&output_id) else {
            return;
        };
        overlay.buffer = buffer;

        if let Some(buffer) = &overlay.buffer {
            overlay.surface.attach(Some(&buffer.wl_buffer), 0, 0);
            overlay
                .surface
                .damage_buffer(0, 0, render_w as i32, render_h as i32);
            overlay.surface.commit();
        }
    }

    /// Attach a screensaver frame buffer after `ensure_frame_buffer` succeeds.
    #[allow(clippy::cast_possible_wrap)]
    pub(super) fn commit_frame_buffer(
        overlay: &mut MonitorOverlay,
        width: u32,
        height: u32,
    ) -> bool {
        let Some(buffer) = overlay.buffer.as_ref() else {
            return false;
        };

        let dst_w = if overlay.width > 0 {
            overlay.width
        } else {
            width
        };
        let dst_h = if overlay.height > 0 {
            overlay.height
        } else {
            height
        };
        if let Some(viewport) = &overlay.viewport {
            viewport.set_destination(dst_w as i32, dst_h as i32);
        }

        overlay.surface.attach(Some(&buffer.wl_buffer), 0, 0);
        overlay
            .surface
            .damage_buffer(0, 0, width as i32, height as i32);
        overlay.surface.commit();
        true
    }
}
