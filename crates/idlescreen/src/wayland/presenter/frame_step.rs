// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Shared surface buffer initialization, saver construction, and per-frame blitting.

use std::sync::atomic::Ordering;
use std::time::Duration;

use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm;

use super::arcade_grid_dimensions;
use super::shm_buf::allocate_shm_buffer;
use super::wl_state::WaylandState;
use crate::ascii::Ascii;
use crate::ascii::effects::EffectKind;
use crate::ascii::state::TextMode;
use crate::cell::TerminalCell;
use crate::wayland::blit::{GLYPH_HEIGHT, GLYPH_WIDTH, blit_cells_to_argb};

pub(super) fn init_surface_buffers(
    state: &mut WaylandState,
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<WaylandState>,
) -> Result<(), Box<dyn std::error::Error>> {
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
        s.configured_width = width;
        s.configured_height = height;
        s.width = width;
        s.height = height;

        let (cols, rows) = arcade_grid_dimensions(width as usize, height as usize);
        let (buf_w, buf_h) = if s.viewport.has_viewport() {
            let bw = u32::try_from(cols * GLYPH_WIDTH).unwrap_or(width);
            let bh = u32::try_from(rows * GLYPH_HEIGHT).unwrap_or(height);
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

        let buf0 = allocate_shm_buffer(shm, qh, buf_w, buf_h)
            .ok_or("Failed to allocate wl_shm buffer 0")?;
        let buf1 = allocate_shm_buffer(shm, qh, buf_w, buf_h)
            .ok_or("Failed to allocate wl_shm buffer 1")?;
        s.buffers = Some([buf0, buf1]);
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
    let effect_kind = if tour_mode {
        None
    } else {
        EffectKind::parse(saver_name)
    };
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
    let (cols, rows) = default_grid;
    let max_cols = state
        .surfaces
        .iter()
        .map(|s| {
            arcade_grid_dimensions(s.configured_width as usize, s.configured_height as usize).0
        })
        .max()
        .unwrap_or(cols);
    let max_rows = state
        .surfaces
        .iter()
        .map(|s| {
            arcade_grid_dimensions(s.configured_width as usize, s.configured_height as usize).1
        })
        .max()
        .unwrap_or(rows);

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
            let bw = u32::try_from(out_cols * GLYPH_WIDTH).unwrap_or(cur_w);
            let bh = u32::try_from(out_rows * GLYPH_HEIGHT).unwrap_or(cur_h);
            s.viewport.set_scaling(
                i32::try_from(cur_w).unwrap_or(1920),
                i32::try_from(cur_h).unwrap_or(1080),
                f64::from(bw),
                f64::from(bh),
            );
            (bw, bh)
        } else {
            (cur_w, cur_h)
        };

        if let Some(ref mut bufs) = s.buffers {
            if bufs[s.current_buf].busy.load(Ordering::Acquire) {
                if !bufs[s.current_buf ^ 1].busy.load(Ordering::Acquire) {
                    s.current_buf ^= 1;
                } else if let Some(new_b) = allocate_shm_buffer(shm, qh, target_buf_w, target_buf_h)
                {
                    bufs[s.current_buf] = new_b;
                }
            }
            if (target_buf_w != bufs[s.current_buf].width
                || target_buf_h != bufs[s.current_buf].height)
                && let Some(new_b) = allocate_shm_buffer(shm, qh, target_buf_w, target_buf_h)
            {
                bufs[s.current_buf] = new_b;
            }

            let buf = &mut bufs[s.current_buf];
            let buf_w = buf.width as usize;
            let buf_h = buf.height as usize;

            if cells.len() < out_cols * out_rows {
                cells.resize(out_cols * out_rows, TerminalCell::default());
            }
            ascii_saver.draw(&mut cells[..out_cols * out_rows], out_cols, out_rows);
            if state.preview_mode {
                crate::wayland::glyph_font::draw_preview_hud(
                    &mut cells[..out_cols * out_rows],
                    out_cols,
                    out_rows,
                    ascii_saver.effect,
                    state.tour_mode,
                );
            }

            blit_cells_to_argb(
                &cells[..out_cols * out_rows],
                out_cols,
                out_rows,
                buf.as_mut_slice(),
                buf_w,
                buf_h,
            );

            buf.busy.store(true, Ordering::Release);
            s.surface.attach(Some(&buf.wl_buffer), 0, 0);
            s.surface.damage_buffer(
                0,
                0,
                i32::try_from(buf_w).unwrap_or(i32::MAX),
                i32::try_from(buf_h).unwrap_or(i32::MAX),
            );
            s.surface.commit();
            s.current_buf ^= 1;
        }
    }
}
