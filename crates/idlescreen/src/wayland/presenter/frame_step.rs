// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Shared surface buffer initialization, saver construction, and per-frame blitting.

use std::sync::atomic::Ordering;
use std::time::Duration;

use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm;

use super::arcade_grid_dimensions;
use super::shm_buf::{allocate_shm_buffer_with_format, compute_cell_damage_rect};
use super::wl_state::WaylandState;
use crate::ascii::Ascii;
use crate::ascii::effects::EffectKind;
use crate::ascii::state::TextMode;
use crate::cell::TerminalCell;
use crate::wayland::blit::{
    GLYPH_HEIGHT, GLYPH_WIDTH, blit_cells_with_format, has_subcell_overlay,
    subcell_overlay_generation,
};

#[inline]
pub fn viewport_buffer_dimensions(width: u32, height: u32, cols: usize, rows: usize) -> (u32, u32) {
    let scale = if width >= 3840 && height >= 2160 {
        2
    } else {
        1
    };
    let bw = u32::try_from(cols * GLYPH_WIDTH * scale).unwrap_or(width);
    let bh = u32::try_from(rows * GLYPH_HEIGHT * scale).unwrap_or(height);
    (bw, bh)
}

pub(super) fn init_surface_buffers(
    state: &mut WaylandState,
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<WaylandState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (fmt, is_10bit) = state.preferred_shm_format();
    for s in &mut state.surfaces {
        let width = if s.configured_width > 0 {
            s.configured_width
        } else if s.width > 0 {
            s.width
        } else {
            1920
        };
        let height = if s.configured_height > 0 {
            s.configured_height
        } else if s.height > 0 {
            s.height
        } else {
            1080
        };
        (s.configured_width, s.configured_height, s.width, s.height) =
            (width, height, width, height);

        let (cols, rows) = arcade_grid_dimensions(width as usize, height as usize);
        let (buf_w, buf_h) = if s.viewport.has_viewport() {
            let (bw, bh) = viewport_buffer_dimensions(width, height, cols, rows);
            s.viewport.set_scaling(
                i32::try_from(width).unwrap_or(1920),
                i32::try_from(height).unwrap_or(1080),
                f64::from(bw),
                f64::from(bh),
            );
            (bw, bh)
        } else {
            (width, height)
        };

        let buf0 = allocate_shm_buffer_with_format(shm, qh, buf_w, buf_h, fmt, is_10bit)
            .ok_or("Failed to allocate wl_shm buffer 0")?;
        let buf1 = allocate_shm_buffer_with_format(shm, qh, buf_w, buf_h, fmt, is_10bit)
            .ok_or("Failed to allocate wl_shm buffer 1")?;
        s.buffers = Some([buf0, buf1]);
        s.displayed_cells.clear();
        s.needs_commit = true;
    }
    Ok(())
}

pub(super) fn build_ascii_saver(
    saver_name: &str,
    mode_str: &str,
    dwell_secs: f32,
    dissolve_secs: f32,
    custom_opt: Option<String>,
    targets: Vec<String>,
    target_fps: u32,
) -> Ascii {
    let tour_mode =
        saver_name.eq_ignore_ascii_case("new") || saver_name.eq_ignore_ascii_case("tour");
    let effect_kind = (!tour_mode)
        .then(|| EffectKind::parse(saver_name))
        .flatten();
    let text_mode = TextMode::parse(mode_str);
    let is_superfedora =
        saver_name == "superfedora" || mode_str.eq_ignore_ascii_case("superfedora");
    let use_superfedora =
        is_superfedora && crate::ascii::sys_info::is_superfedora_machine() && custom_opt.is_none();

    let mut saver = if use_superfedora {
        Ascii::new_superfedora()
            .with_dwell(dwell_secs)
            .with_dissolve(dissolve_secs)
            .with_fps(target_fps)
    } else {
        Ascii::new()
            .with_mode(text_mode, custom_opt)
            .with_targets(targets)
            .with_dwell(dwell_secs)
            .with_dissolve(dissolve_secs)
            .with_fps(target_fps)
    };
    if tour_mode {
        saver.enable_tour_mode();
    } else if let Some(effect) = effect_kind {
        saver.pin_effect(effect);
    }
    saver
}

#[allow(clippy::too_many_arguments)]
pub(super) fn step_and_present_surfaces(
    state: &mut WaylandState,
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<WaylandState>,
    dt: Duration,
    default_grid: (usize, usize),
    ascii_saver: &mut Ascii,
    cells: &mut Vec<TerminalCell>,
) {
    let (fmt, is_10bit) = state.preferred_shm_format();
    let (mut max_cols, mut max_rows) = default_grid;
    for s in &state.surfaces {
        let (c, r) =
            arcade_grid_dimensions(s.configured_width as usize, s.configured_height as usize);
        (max_cols, max_rows) = (max_cols.max(c), max_rows.max(r));
    }
    ascii_saver.update(dt, max_cols, max_rows);

    for s in &mut state.surfaces {
        let cur_w = if s.configured_width > 0 {
            s.configured_width
        } else {
            1920
        };
        let cur_h = if s.configured_height > 0 {
            s.configured_height
        } else {
            1080
        };
        let (out_cols, out_rows) = arcade_grid_dimensions(cur_w as usize, cur_h as usize);
        let (target_buf_w, target_buf_h) = if s.viewport.has_viewport() {
            let (bw, bh) = viewport_buffer_dimensions(cur_w, cur_h, out_cols, out_rows);
            let (sw, sh) = (
                i32::try_from(cur_w).unwrap_or(1920),
                i32::try_from(cur_h).unwrap_or(1080),
            );
            s.viewport.set_scaling(sw, sh, f64::from(bw), f64::from(bh));
            (bw, bh)
        } else {
            (cur_w, cur_h)
        };

        let Some(ref mut bufs) = s.buffers else {
            continue;
        };
        let total = out_cols * out_rows;
        if cells.len() < total {
            cells.resize(total, TerminalCell::default());
        }
        let active_cells = &mut cells[..total];
        ascii_saver.draw(active_cells, out_cols, out_rows);
        if state.preview_mode {
            crate::wayland::glyph_font::draw_preview_hud(
                active_cells,
                out_cols,
                out_rows,
                ascii_saver.effect,
                state.tour_mode,
            );
        }

        let subcell_now = active_cells
            .iter()
            .any(|c| !c.bold && matches!(c.ch, '▀' | '▄'))
            && has_subcell_overlay(out_cols, out_rows);
        let overlay_gen = subcell_overlay_generation();
        let force_full = s.needs_commit
            || subcell_now
            || (s.displayed_subcell_gen != overlay_gen)
            || target_buf_w != bufs[s.current_buf].width
            || target_buf_h != bufs[s.current_buf].height;

        let (buf_w, buf_h) = (target_buf_w as usize, target_buf_h as usize);
        let Some((dx, dy, dw, dh)) = compute_cell_damage_rect(
            &s.displayed_cells,
            active_cells,
            (out_cols, out_rows),
            (buf_w, buf_h),
            force_full,
        ) else {
            continue;
        };

        if bufs[s.current_buf].busy.load(Ordering::Acquire) {
            if !bufs[s.current_buf ^ 1].busy.load(Ordering::Acquire) {
                s.current_buf ^= 1;
            } else if let Some(b) =
                allocate_shm_buffer_with_format(shm, qh, target_buf_w, target_buf_h, fmt, is_10bit)
            {
                bufs[s.current_buf] = b;
            }
        }
        if (target_buf_w != bufs[s.current_buf].width || target_buf_h != bufs[s.current_buf].height)
            && let Some(b) =
                allocate_shm_buffer_with_format(shm, qh, target_buf_w, target_buf_h, fmt, is_10bit)
        {
            bufs[s.current_buf] = b;
        }

        let buf = &mut bufs[s.current_buf];
        let (actual_w, actual_h, is_hdr) =
            (buf.width as usize, buf.height as usize, buf.is_10bit_hdr);
        let (slice, prev_opt) = buf.as_mut_slice_and_prev(out_cols, out_rows);
        let subcell_drawn = blit_cells_with_format(
            active_cells,
            (out_cols, out_rows),
            slice,
            (actual_w, actual_h),
            is_hdr,
            prev_opt,
        );
        buf.prev_cells.clear();
        buf.prev_cells.extend_from_slice(active_cells);
        (buf.prev_cols, buf.prev_rows, buf.prev_subcell) = (out_cols, out_rows, subcell_drawn);

        s.displayed_cells.clear();
        s.displayed_cells.extend_from_slice(active_cells);
        (s.displayed_subcell_gen, s.needs_commit) = (overlay_gen, false);

        buf.busy.store(true, Ordering::Release);
        s.surface.attach(Some(&buf.wl_buffer), 0, 0);
        let (max_w, max_h) = (actual_w as i32, actual_h as i32);
        s.surface
            .damage_buffer(dx, dy, dw.min(max_w), dh.min(max_h));
        s.surface.commit();
        s.current_buf ^= 1;
    }
}
