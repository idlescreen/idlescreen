// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Shared precomputed canvas geometry and block-shading helpers for mathematical and arcade scenes.

use crate::ascii::scenes::canvas_ops::hex_color;
use ttfx::engine::animation::ExistingColorHandling;
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::EngineCtx;
use ttfx::engine::error::EngineError;
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

const BLOCK_SHADES: [&str; 4] = ["░", "▒", "▓", "█"];

#[derive(Clone)]
pub struct CanvasCell {
    pub id: CharId,
    pub col: i64,
    pub row: i64,
    pub nx: f64,
    pub ny: f64,
    pub dx: f64,
    pub dy: f64,
    pub dist_center: f64,
    pub angle_center: f64,
    pub is_fill: bool,
    pub input_symbol: String,
    pub uses_pre: bool,
    pub final_colors: ColorPair,
    pub settled_colors: ColorPair,
}

pub fn build_palette(stops: &[&str], steps_per_segment: i64) -> Result<Vec<Color>, EngineError> {
    let colors: Vec<Color> = stops.iter().map(|s| hex_color(s)).collect();
    let steps = vec![steps_per_segment.max(1)];
    let grad = Gradient::new(&colors, &steps, false, false).map_err(EngineError::Other)?;
    Ok(grad.spectrum)
}

pub fn build_canvas_cells(
    ctx: &mut EngineCtx,
    final_stops: &[&str],
) -> Result<Vec<CanvasCell>, EngineError> {
    let stop_colors: Vec<Color> = final_stops.iter().map(|s| hex_color(s)).collect();
    let final_gradient =
        Gradient::new(&stop_colors, &[12], false, false).map_err(EngineError::Other)?;
    let final_mapping = final_gradient
        .build_coordinate_color_mapping(
            ctx.terminal.canvas.text_bottom,
            ctx.terminal.canvas.text_top,
            ctx.terminal.canvas.text_left,
            ctx.terminal.canvas.text_right,
            GradientDirection::Vertical,
        )
        .map_err(EngineError::Other)?;

    let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
    let all_filter = CharacterFilter {
        input_chars: true,
        inner_fill_chars: true,
        outer_fill_chars: true,
        added_chars: false,
    };
    let ids = ctx.terminal.get_characters(
        &mut ctx.rng,
        all_filter,
        CharacterSort::TopToBottomLeftToRight,
    );

    let width = (ctx.terminal.canvas.right - ctx.terminal.canvas.left).max(1) as f64;
    let height = (ctx.terminal.canvas.top - ctx.terminal.canvas.bottom).max(1) as f64;
    let aspect = (width / (height * 2.0)).clamp(0.5, 2.5);
    let fallback_color = stop_colors
        .last()
        .cloned()
        .unwrap_or_else(|| hex_color("#FFFFFF"));
    let top_row = ctx.terminal.canvas.top;
    let min_y = (top_row - ctx.terminal.canvas.text_top).max(0) as usize;
    let max_y = (top_row - ctx.terminal.canvas.text_bottom).max(0) as usize;
    let span = max_y.saturating_sub(min_y) + 1;
    let is_stacked_sf = span == 22 || span == 7;

    let mut cells = Vec::with_capacity(ids.len());
    for id in ids {
        let ch = &ctx.terminal.arena[id.0 as usize];
        let coord = ch.input_coord;
        let nx = ((coord.column - ctx.terminal.canvas.left) as f64 / width).clamp(0.0, 1.0);
        let ny = (1.0 - (coord.row - ctx.terminal.canvas.bottom) as f64 / height).clamp(0.0, 1.0);
        let dx = (nx - 0.5) * 2.0 * aspect;
        let dy = (ny - 0.5) * 2.0;
        let dist_center = dx.hypot(dy);
        let angle_center = dy.atan2(dx);
        let is_fill = ch.is_fill_character || ch.input_symbol == " ";
        let final_colors = if is_fill {
            ColorPair::new(None, None)
        } else if dynamic {
            ColorPair::new(
                ch.animation.input_fg_color.clone(),
                ch.animation.input_bg_color.clone(),
            )
        } else {
            let fg = final_mapping
                .get(&coord)
                .cloned()
                .unwrap_or_else(|| fallback_color.clone());
            ColorPair::new(Some(fg), None)
        };
        let settled_colors = if is_fill {
            ColorPair::new(None, None)
        } else {
            let y = (top_row - coord.row).max(0) as usize;
            let (r, g, b) = crate::ascii::draw::banner_bevel_color(y, min_y, max_y, is_stacked_sf);
            ColorPair::new(Some(hex_color(&format!("{r:02X}{g:02X}{b:02X}"))), None)
        };
        cells.push(CanvasCell {
            id,
            col: coord.column,
            row: coord.row,
            nx,
            ny,
            dx,
            dy,
            dist_center,
            angle_center,
            is_fill,
            input_symbol: ch.input_symbol.clone(),
            uses_pre: ch.uses_input_preexisting_colors,
            final_colors,
            settled_colors,
        });
    }
    Ok(cells)
}

pub fn sample_palette(palette: &[Color], t: f64) -> Color {
    let clamped = t.clamp(0.0, 1.0);
    let max_idx = palette.len().saturating_sub(1);
    let idx = (clamped * max_idx as f64).round() as usize;
    palette[idx.min(max_idx)].clone()
}

pub fn paint_math_cell(
    ctx: &mut EngineCtx,
    cell: &CanvasCell,
    palette: &[Color],
    intensity: f64,
    locked: bool,
    fill_fade: f64,
) {
    if !cell.is_fill && locked {
        ctx.terminal.set_character_visibility(cell.id, true);
        let ch = &mut ctx.terminal.arena[cell.id.0 as usize];
        ch.animation.set_appearance(
            &cell.input_symbol,
            cell.uses_pre,
            Some(&cell.input_symbol),
            Some(cell.final_colors.clone()),
        );
        return;
    }

    let eff = if cell.is_fill {
        (intensity * fill_fade).clamp(0.0, 1.0)
    } else {
        intensity.clamp(0.0, 1.0)
    };

    if eff <= 0.14 {
        ctx.terminal.set_character_visibility(cell.id, false);
        return;
    }

    let shade_norm = ((eff - 0.14) / 0.86).clamp(0.0, 0.999);
    let sym = BLOCK_SHADES[(shade_norm * BLOCK_SHADES.len() as f64) as usize];
    let color = sample_palette(palette, eff);
    ctx.terminal.set_character_visibility(cell.id, true);
    let ch = &mut ctx.terminal.arena[cell.id.0 as usize];
    ch.animation.set_appearance(
        &cell.input_symbol,
        cell.uses_pre,
        Some(sym),
        Some(ColorPair::new(Some(color), None)),
    );
}

pub fn paint_symbol_cell(
    ctx: &mut EngineCtx,
    cell: &CanvasCell,
    palette: &[Color],
    sym: Option<&str>,
    color_t: f64,
    locked: bool,
) {
    if !cell.is_fill && locked {
        ctx.terminal.set_character_visibility(cell.id, true);
        let ch = &mut ctx.terminal.arena[cell.id.0 as usize];
        ch.animation.set_appearance(
            &cell.input_symbol,
            cell.uses_pre,
            Some(&cell.input_symbol),
            Some(cell.final_colors.clone()),
        );
        return;
    }
    if let Some(symbol) = sym {
        let color = sample_palette(palette, color_t);
        ctx.terminal.set_character_visibility(cell.id, true);
        let ch = &mut ctx.terminal.arena[cell.id.0 as usize];
        ch.animation.set_appearance(
            &cell.input_symbol,
            cell.uses_pre,
            Some(symbol),
            Some(ColorPair::new(Some(color), None)),
        );
    } else {
        ctx.terminal.set_character_visibility(cell.id, false);
    }
}

pub fn settle_canvas_cells(ctx: &mut EngineCtx, cells: &[CanvasCell]) {
    for cell in cells {
        if cell.is_fill {
            ctx.terminal.set_character_visibility(cell.id, false);
        } else {
            ctx.terminal.set_character_visibility(cell.id, true);
            let ch = &mut ctx.terminal.arena[cell.id.0 as usize];
            ch.animation.set_appearance(
                &cell.input_symbol,
                cell.uses_pre,
                Some(&cell.input_symbol),
                Some(cell.settled_colors.clone()),
            );
        }
    }
}
