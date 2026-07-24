// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Geometry helpers for layer-shell overlays (native mode size vs configure).

use std::collections::HashMap;

use wayland_client::protocol::wl_surface;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

use super::types::SessionState;

impl SessionState {
    pub(crate) fn render_dimensions(
        output_id: u32,
        configured_w: u32,
        configured_h: u32,
        mode_sizes: &HashMap<u32, (u32, u32)>,
    ) -> (u32, u32) {
        let Some((native_w, native_h)) = mode_sizes.get(&output_id).copied() else {
            return (configured_w, configured_h);
        };
        (native_w.max(configured_w), native_h.max(configured_h))
    }

    #[allow(clippy::cast_possible_wrap)]
    pub(crate) fn apply_tiling_margins(
        layer_surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        surface: &wl_surface::WlSurface,
        output_id: u32,
        configured_w: u32,
        configured_h: u32,
        mode_sizes: &HashMap<u32, (u32, u32)>,
    ) {
        let Some((native_w, native_h)) = mode_sizes.get(&output_id).copied() else {
            layer_surface.set_margin(0, 0, 0, 0);
            surface.commit();
            return;
        };

        let inset_x = native_w.saturating_sub(configured_w) / 2;
        let inset_y = native_h.saturating_sub(configured_h) / 2;
        let m_y = if inset_y > 0 { -(inset_y as i32) } else { 0 };
        let m_x = if inset_x > 0 { -(inset_x as i32) } else { 0 };
        layer_surface.set_margin(m_y, m_x, m_y, m_x);
        surface.commit();
    }
}
