// SPDX-License-Identifier: MIT

#![allow(clippy::too_many_arguments)]

//! Rasterizes [`trance_api::TerminalCell`] grids into BGRA pixel buffers.

mod atlas;
mod font;
mod pixels;

use std::collections::HashMap;
use std::sync::Arc;

use fontdue::{Font, Metrics};
use trance_api::TerminalCell;

use pixels::{blit_bitmap, dim_rect, fill_rect, letterbox_into};

pub use font::{FONT_CANDIDATES, font_available, resolve_font_path};

const FONT_SIZE: f32 = 16.0;

struct CachedGlyph {
    metrics: Metrics,
    bitmap: Arc<[u8]>,
}

/// Rasterizes [`TerminalCell`] grids into ARGB8888 pixel buffers.
pub struct CellRenderer {
    font: Font,
    pub(crate) cell_width: usize,
    pub(crate) cell_height: usize,
    glyph_cache: HashMap<char, CachedGlyph>,
    pub(crate) atlas_chars: Vec<char>,
    pub(crate) atlas_image: Vec<u8>,
    pub(crate) atlas_cols: usize,
    pub(crate) atlas_rows: usize,
    pub(crate) atlas_dirty: bool,
}

impl CellRenderer {
    pub fn new() -> Result<Self, String> {
        let font_bytes = font::load_monospace_font()?;
        let font = Font::from_bytes(font_bytes, fontdue::FontSettings::default())
            .map_err(|error| format!("failed to parse font: {error}"))?;

        let (metrics, _) = font.rasterize('M', FONT_SIZE);
        let cell_width = metrics.width.max(8);
        let cell_height = metrics.height.max(14);

        let mut renderer = Self {
            font,
            cell_width,
            cell_height,
            glyph_cache: HashMap::new(),
            atlas_chars: Vec::new(),
            atlas_image: Vec::new(),
            atlas_cols: 32,
            atlas_rows: 32,
            atlas_dirty: true,
        };
        renderer.prepopulate_atlas();
        Ok(renderer)
    }

    fn glyph_for(&mut self, ch: char) -> (Metrics, Arc<[u8]>) {
        if let Some(glyph) = self.glyph_cache.get(&ch) {
            return (glyph.metrics, Arc::clone(&glyph.bitmap));
        }

        let (metrics, bitmap) = self.font.rasterize(ch, FONT_SIZE);
        let bitmap = Arc::from(bitmap);
        self.glyph_cache.insert(
            ch,
            CachedGlyph {
                metrics,
                bitmap: Arc::clone(&bitmap),
            },
        );
        (metrics, bitmap)
    }

    pub fn cell_width(&self) -> usize {
        self.cell_width
    }

    pub fn cell_height(&self) -> usize {
        self.cell_height
    }

    pub fn grid_for_pixels(&self, width: u32, height: u32) -> (usize, usize) {
        self.grid_for_pixels_scaled(width, height, 1.0)
    }

    pub fn grid_for_pixels_scaled(&self, width: u32, height: u32, scale: f32) -> (usize, usize) {
        let cols = (width as usize / self.cell_width).max(1);
        let rows = (height as usize / self.cell_height).max(1);
        let scale = scale.clamp(0.25, 1.0);
        (
            ((cols as f32 * scale).floor() as usize).max(1),
            ((rows as f32 * scale).floor() as usize).max(1),
        )
    }

    pub fn content_width(&self, cols: usize) -> u32 {
        cols.saturating_mul(self.cell_width).min(u32::MAX as usize) as u32
    }

    pub fn content_height(&self, rows: usize) -> u32 {
        rows.saturating_mul(self.cell_height).min(u32::MAX as usize) as u32
    }

    pub fn render(
        &mut self,
        grid: &[TerminalCell],
        cols: usize,
        rows: usize,
        width: u32,
        height: u32,
        scanlines: bool,
    ) -> Vec<u8> {
        let content_w = self.content_width(cols);
        let content_h = self.content_height(rows);
        let mut content = Vec::new();
        self.render_content_viewport_into(grid, cols, 0, 0, cols, rows, scanlines, &mut content);
        let offset_x = width.saturating_sub(content_w) as usize / 2;
        let offset_y = height.saturating_sub(content_h) as usize / 2;
        letterbox_into(
            &content, content_w, content_h, width, height, offset_x, offset_y,
        )
    }

    pub fn render_content_viewport_into(
        &mut self,
        grid: &[TerminalCell],
        grid_cols: usize,
        col_start: usize,
        row_start: usize,
        cols: usize,
        rows: usize,
        scanlines: bool,
        out: &mut Vec<u8>,
    ) {
        let content_w = self.content_width(cols);
        let content_h = self.content_height(rows);
        let byte_len = (content_w * content_h * 4) as usize;
        out.resize(byte_len, 0);
        out.fill(0);

        for row in 0..rows {
            for col in 0..cols {
                let grid_row = row_start + row;
                let grid_col = col_start + col;
                let index = grid_row * grid_cols + grid_col;
                let Some(cell) = grid.get(index) else {
                    continue;
                };

                let x0 = col * self.cell_width;
                let y0 = row * self.cell_height;
                fill_rect(
                    out,
                    content_w,
                    content_h,
                    x0,
                    y0,
                    self.cell_width,
                    self.cell_height,
                    cell.bg,
                );

                if cell.ch != ' ' {
                    let (metrics, bitmap) = self.glyph_for(cell.ch);
                    blit_bitmap(
                        out,
                        content_w,
                        content_h,
                        x0,
                        y0.saturating_add(metrics.ymin.max(0) as usize),
                        &bitmap,
                        metrics.width,
                        metrics.height,
                        cell.fg,
                    );
                    if cell.bold {
                        blit_bitmap(
                            out,
                            content_w,
                            content_h,
                            x0 + 1,
                            y0.saturating_add(metrics.ymin.max(0) as usize),
                            &bitmap,
                            metrics.width,
                            metrics.height,
                            cell.fg,
                        );
                    }
                }

                if scanlines && row % 2 == 1 {
                    dim_rect(
                        out,
                        content_w,
                        content_h,
                        x0,
                        y0,
                        self.cell_width,
                        self.cell_height,
                    );
                }
            }
        }
    }
}
