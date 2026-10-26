// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Shared surface buffer initialization, saver construction, and per-frame blitting.

use std::sync::atomic::Ordering;
use std::time::Duration;

use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm;

use super::arcade_grid_dimensions;
use super::bufpool;
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
use crate::wayland::fractional;

#[inline]
pub fn viewport_buffer_dimensions(
    width: u32,
    height: u32,
    cols: usize,
    rows: usize,
    scale_120ths: u32,
) -> (u32, u32) {
    if scale_120ths > 120 {
        // The compositor reported a scale above 100%: size the buffer with
        // the standard fractional-scale rounding so the viewport scales the
        // glyph grid without blurring it.
        let cell_w = u32::try_from(cols * GLYPH_WIDTH).unwrap_or(width);
        let cell_h = u32::try_from(rows * GLYPH_HEIGHT).unwrap_or(height);
        return (
            fractional::calculate_buffer_dimension(cell_w, scale_120ths),
            fractional::calculate_buffer_dimension(cell_h, scale_120ths),
        );
    }
    // No scale reported yet: fall back to the 4K heuristic.
    let scale = if width >= 3840 && height >= 2160 {
        2
    } else {
        1
    };
    let bw = u32::try_from(cols * GLYPH_WIDTH * scale).unwrap_or(width);
    let bh = u32::try_from(rows * GLYPH_HEIGHT * scale).unwrap_or(height);
    (bw, bh)
}

#[inline]
fn resolve_surface_dims(cw: u32, ch: u32, w: u32, h: u32) -> (u32, u32) {
    let rw = [cw, w, 1920].into_iter().find(|&v| v > 0).unwrap_or(1920);
    let rh = [ch, h, 1080].into_iter().find(|&v| v > 0).unwrap_or(1080);
    (rw, rh)
}

pub(super) fn init_surface_buffers(
    state: &mut WaylandState,
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<WaylandState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (fmt, is_10bit) = state.preferred_shm_format();
    for s in &mut state.surfaces {
        let (width, height) =
            resolve_surface_dims(s.configured_width, s.configured_height, s.width, s.height);
        (s.configured_width, s.configured_height, s.width, s.height) =
            (width, height, width, height);

        let (cols, rows) = arcade_grid_dimensions(width as usize, height as usize);
        let (buf_w, buf_h) = if s.viewport.has_viewport() {
            let (bw, bh) = viewport_buffer_dimensions(width, height, cols, rows, s.scale_120ths);
            let (sw, sh) = (
                i32::try_from(width).unwrap_or(1920),
                i32::try_from(height).unwrap_or(1080),
            );
            s.viewport.set_scaling(sw, sh, f64::from(bw), f64::from(bh));
            (bw, bh)
        } else {
            (width, height)
        };

        let buf0 = allocate_shm_buffer_with_format(shm, qh, buf_w, buf_h, fmt, is_10bit)
            .ok_or("Failed to allocate wl_shm buffer 0")?;
        let buf1 = allocate_shm_buffer_with_format(shm, qh, buf_w, buf_h, fmt, is_10bit)
            .ok_or("Failed to allocate wl_shm buffer 1")?;
        s.buffers = Some([buf0, buf1]);
        s.retired.clear();
        s.displayed_cells.clear();
        (s.displayed_subcell, s.needs_commit) = (false, true);
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
        let (sw, sh) =
            resolve_surface_dims(s.configured_width, s.configured_height, s.width, s.height);
        let (c, r) = arcade_grid_dimensions(sw as usize, sh as usize);
        (max_cols, max_rows) = (max_cols.max(c), max_rows.max(r));
    }
    ascii_saver.update(dt, max_cols, max_rows);

    for s in &mut state.surfaces {
        let Some(ref mut bufs) = s.buffers else {
            continue;
        };
        let (cur_w, cur_h) =
            resolve_surface_dims(s.configured_width, s.configured_height, s.width, s.height);
        let (out_cols, out_rows) = arcade_grid_dimensions(cur_w as usize, cur_h as usize);
        let (target_buf_w, target_buf_h) = if s.viewport.has_viewport() {
            viewport_buffer_dimensions(cur_w, cur_h, out_cols, out_rows, s.scale_120ths)
        } else {
            (cur_w, cur_h)
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
            || s.displayed_subcell
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

        // Reclaim retired buffers whose wl_buffer.release has arrived: the
        // compositor is done reading them, so destroy() + munmap are safe.
        bufpool::reclaim_released(&mut s.retired);

        if !bufpool::select_buffer(
            bufs,
            &mut s.retired,
            &mut s.current_buf,
            shm,
            qh,
            &bufpool::BufferTarget {
                width: target_buf_w,
                height: target_buf_h,
                format: fmt,
                is_10bit,
            },
        ) {
            continue;
        }

        let buf = &mut bufs[s.current_buf];
        let (actual_w, actual_h, is_hdr) =
            (buf.width as usize, buf.height as usize, buf.is_10bit_hdr);
        if s.viewport.has_viewport() {
            let (sw, sh) = (
                i32::try_from(cur_w).unwrap_or(1920),
                i32::try_from(cur_h).unwrap_or(1080),
            );
            s.viewport
                .set_scaling(sw, sh, f64::from(buf.width), f64::from(buf.height));
        }
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
        (s.displayed_subcell_gen, s.displayed_subcell, s.needs_commit) =
            (overlay_gen, subcell_drawn, false);

        buf.busy.store(true, Ordering::Release);
        s.surface.attach(Some(&buf.wl_buffer), 0, 0);
        let (max_w, max_h) = (actual_w as i32, actual_h as i32);
        let (cx, cy) = (
            dx.clamp(0, max_w.saturating_sub(1)),
            dy.clamp(0, max_h.saturating_sub(1)),
        );
        let (cw, ch) = (
            dw.min(max_w.saturating_sub(cx)).max(1),
            dh.min(max_h.saturating_sub(cy)).max(1),
        );
        s.surface.damage_buffer(cx, cy, cw, ch);
        s.surface.commit();
        s.current_buf ^= 1;
    }
}
