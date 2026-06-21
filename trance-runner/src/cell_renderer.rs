// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use fontdue::{Font, Metrics};
use trance_api::TerminalCell;

const FONT_SIZE: f32 = 16.0;

pub const FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
];

/// Returns the first installed monospace font used for cell rasterization.
pub fn resolve_font_path() -> Option<&'static str> {
    FONT_CANDIDATES
        .iter()
        .find(|path| Path::new(path).is_file())
        .copied()
}

/// Whether a supported monospace font is installed on the system.
pub fn font_available() -> bool {
    resolve_font_path().is_some()
}

struct CachedGlyph {
    metrics: Metrics,
    bitmap: Arc<[u8]>,
}

/// Rasterizes [`TerminalCell`] grids into ARGB8888 pixel buffers.
pub struct CellRenderer {
    font: Font,
    cell_width: usize,
    cell_height: usize,
    glyph_cache: HashMap<char, CachedGlyph>,
    scratch: Vec<u8>,
}

impl CellRenderer {
    pub fn new() -> Result<Self, String> {
        let font_bytes = load_monospace_font()?;
        let font = Font::from_bytes(font_bytes, fontdue::FontSettings::default())
            .map_err(|error| format!("failed to parse font: {error}"))?;

        let (metrics, _) = font.rasterize('M', FONT_SIZE);
        let cell_width = metrics.width.max(8);
        let cell_height = metrics.height.max(14);

        Ok(Self {
            font,
            cell_width,
            cell_height,
            glyph_cache: HashMap::new(),
            scratch: Vec::new(),
        })
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
        cols.saturating_mul(self.cell_width) as u32
    }

    pub fn content_height(&self, rows: usize) -> u32 {
        rows.saturating_mul(self.cell_height) as u32
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
        let content = self.render_content(grid, cols, rows, scanlines);
        let offset_x = width.saturating_sub(content_w) as usize / 2;
        let offset_y = height.saturating_sub(content_h) as usize / 2;
        letterbox_into(
            &content,
            content_w,
            content_h,
            width,
            height,
            offset_x,
            offset_y,
        )
    }

    pub fn render_content_viewport(
        &mut self,
        grid: &[TerminalCell],
        grid_cols: usize,
        col_start: usize,
        row_start: usize,
        cols: usize,
        rows: usize,
        scanlines: bool,
    ) -> Vec<u8> {
        let content_w = self.content_width(cols);
        let content_h = self.content_height(rows);
        let byte_len = (content_w * content_h * 4) as usize;
        self.scratch.resize(byte_len, 0);
        self.scratch.fill(0);

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
                    &mut self.scratch,
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
                        &mut self.scratch,
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
                            &mut self.scratch,
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
                        &mut self.scratch,
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

        self.scratch.clone()
    }

    pub fn render_content(
        &mut self,
        grid: &[TerminalCell],
        cols: usize,
        rows: usize,
        scanlines: bool,
    ) -> Vec<u8> {
        self.render_content_viewport(grid, cols, 0, 0, cols, rows, scanlines)
    }
}

fn letterbox_into(
    content: &[u8],
    content_w: u32,
    content_h: u32,
    width: u32,
    height: u32,
    offset_x: usize,
    offset_y: usize,
) -> Vec<u8> {
    let mut framed = vec![0u8; (width * height * 4) as usize];
    for row in 0..content_h as usize {
        let src_start = row * content_w as usize * 4;
        let src_end = src_start + content_w as usize * 4;
        let dst_row = offset_y + row;
        if dst_row >= height as usize {
            break;
        }
        let dst_start = (dst_row * width as usize + offset_x) * 4;
        let dst_end = dst_start + content_w as usize * 4;
        if src_end <= content.len() && dst_end <= framed.len() {
            framed[dst_start..dst_end].copy_from_slice(&content[src_start..src_end]);
        }
    }
    framed
}

fn load_monospace_font() -> Result<Vec<u8>, String> {
    let path = resolve_font_path().ok_or_else(|| {
        "no monospace font found; install the fonts-dejavu-core package".to_string()
    })?;
    fs::read(path).map_err(|error| format!("failed to read {path}: {error}"))
}

fn fill_rect(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: (u8, u8, u8),
) {
    for row in y..y.saturating_add(h).min(height as usize) {
        for col in x..x.saturating_add(w).min(width as usize) {
            write_pixel(pixels, width, col, row, color, 0xFF);
        }
    }
}

fn dim_rect(pixels: &mut [u8], width: u32, height: u32, x: usize, y: usize, w: usize, h: usize) {
    for row in y..y.saturating_add(h).min(height as usize) {
        for col in x..x.saturating_add(w).min(width as usize) {
            let offset = pixel_offset(width, col, row);
            if offset + 2 < pixels.len() {
                pixels[offset] = pixels[offset] / 2;
                pixels[offset + 1] = pixels[offset + 1] / 2;
                pixels[offset + 2] = pixels[offset + 2] / 2;
            }
        }
    }
}

fn blit_bitmap(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x: usize,
    y: usize,
    bitmap: &[u8],
    bitmap_w: usize,
    bitmap_h: usize,
    color: (u8, u8, u8),
) {
    for row in 0..bitmap_h {
        for col in 0..bitmap_w {
            let alpha = *bitmap.get(row * bitmap_w + col).unwrap_or(&0);
            if alpha == 0 {
                continue;
            }
            let px = x + col;
            let py = y + row;
            if px >= width as usize || py >= height as usize {
                continue;
            }
            write_pixel(pixels, width, px, py, color, alpha);
        }
    }
}

fn write_pixel(pixels: &mut [u8], width: u32, x: usize, y: usize, color: (u8, u8, u8), alpha: u8) {
    let offset = pixel_offset(width, x, y);
    if offset + 3 >= pixels.len() {
        return;
    }

    if alpha == 0xFF {
        pixels[offset] = color.2;
        pixels[offset + 1] = color.1;
        pixels[offset + 2] = color.0;
        pixels[offset + 3] = 0xFF;
        return;
    }

    let src_a = alpha as f32 / 255.0;
    let dst_a = pixels[offset + 3] as f32 / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a <= 0.0 {
        return;
    }

    let blend = |src: u8, dst: u8| {
        let src_f = src as f32 / 255.0;
        let dst_f = dst as f32 / 255.0;
        ((src_f * src_a + dst_f * dst_a * (1.0 - src_a)) / out_a * 255.0) as u8
    };

    pixels[offset] = blend(color.2, pixels[offset]);
    pixels[offset + 1] = blend(color.1, pixels[offset + 1]);
    pixels[offset + 2] = blend(color.0, pixels[offset + 2]);
    pixels[offset + 3] = (out_a * 255.0) as u8;
}

fn pixel_offset(width: u32, x: usize, y: usize) -> usize {
    ((y as u32 * width + x as u32) * 4) as usize
}