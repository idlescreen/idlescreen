// SPDX-License-Identifier: MIT

use super::gpu_init::{GpuCell, GpuCellRenderer, Uniforms};
use trance_api::TerminalCell;

impl GpuCellRenderer {
    pub fn render(
        &mut self,
        grid: &[TerminalCell],
        grid_cols: usize,
        col_start: usize,
        row_start: usize,
        cols: usize,
        rows: usize,
        scanlines: bool,
        cell_width: usize,
        cell_height: usize,
        atlas_cols: usize,
        atlas_rows: usize,
        atlas_image: &[u8],
        atlas_dirty: bool,
        atlas_chars: &[char],
        out: &mut Vec<u8>,
    ) {
        let (content_w, content_h) = ((cols * cell_width) as u32, (rows * cell_height) as u32);
        if content_w == 0 || content_h == 0 { return; }

        let unpadded = content_w * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded = unpadded + (align - unpadded % align) % align;

        let mut recreate_bg = false;
        if self.target_width != content_w || self.target_height != content_h || self.texture.is_none() {
            self.target_width = content_w;
            self.target_height = content_h;
            Self::ensure_texture(
                &self.device,
                &mut self.texture,
                "cell render target",
                content_w,
                content_h,
                wgpu::TextureFormat::Bgra8Unorm,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            );
            self.staging_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("staging"),
                size: (padded * content_h) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }));
            recreate_bg = true;
        }

        let cells_size = (cols * rows * std::mem::size_of::<GpuCell>()) as u64;
        let (cells_buf, c_re) = Self::ensure_buffer(
            &self.device,
            &mut self.cells_buffer,
            "cells",
            cells_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );

        let (uni_buf, u_re) = Self::ensure_buffer(
            &self.device,
            &mut self.uniform_buffer,
            "uniforms",
            std::mem::size_of::<Uniforms>() as u64,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        let (atlas_w, atlas_h) = (atlas_cols * cell_width, atlas_rows * cell_height);
        let mut a_re = false;
        if atlas_dirty || self.atlas_texture.is_none() || self.atlas_width != atlas_w || self.atlas_height != atlas_h {
            self.atlas_width = atlas_w;
            self.atlas_height = atlas_h;
            Self::ensure_texture(
                &self.device,
                &mut self.atlas_texture,
                "atlas",
                atlas_w as u32,
                atlas_h as u32,
                wgpu::TextureFormat::R8Unorm,
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            );
            a_re = true;
        }

        let recreate_bind = recreate_bg || c_re || u_re || a_re;

        if atlas_dirty || recreate_bind {
            if let Some(ref atlas_tex) = self.atlas_texture {
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: atlas_tex,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    atlas_image,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(atlas_w as u32),
                        rows_per_image: None,
                    },
                    wgpu::Extent3d {
                        width: atlas_w as u32,
                        height: atlas_h as u32,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }

        if recreate_bind || self.bind_group.is_none() {
            let atlas_view = self.atlas_texture.as_ref().unwrap().create_view(&wgpu::TextureViewDescriptor::default());
            self.bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bind group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: uni_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: cells_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.atlas_sampler) },
                ],
            }));
        }

        let uniforms = Uniforms {
            cols: cols as u32,
            rows: rows as u32,
            cell_width: cell_width as u32,
            cell_height: cell_height as u32,
            atlas_cols: atlas_cols as u32,
            atlas_rows: atlas_rows as u32,
            scanlines: if scanlines { 1 } else { 0 },
            padding: 0,
        };
        self.queue.write_buffer(&uni_buf, 0, bytemuck::bytes_of(&uniforms));

        let mut gpu_cells = Vec::with_capacity(cols * rows);
        for row in 0..rows {
            for col in 0..cols {
                let index = (row_start + row) * grid_cols + (col_start + col);
                if let Some(cell) = grid.get(index) {
                    let bg_color = ((cell.bg.0 as u32) << 16) | ((cell.bg.1 as u32) << 8) | (cell.bg.2 as u32);
                    let fg_color = ((cell.fg.0 as u32) << 16) | ((cell.fg.1 as u32) << 8) | (cell.fg.2 as u32);
                    let char_idx = if cell.ch == ' ' {
                        0xFFFFFFFF
                    } else {
                        atlas_chars.iter().position(|&c| c == cell.ch).map(|idx| idx as u32).unwrap_or(0xFFFFFFFF)
                    };
                    gpu_cells.push(GpuCell { bg_color, fg_color, char_idx, bold: if cell.bold { 1 } else { 0 } });
                } else {
                    gpu_cells.push(GpuCell { bg_color: 0, fg_color: 0xFFFFFF, char_idx: 0xFFFFFFFF, bold: 0 });
                }
            }
        }
        self.queue.write_buffer(&cells_buf, 0, bytemuck::cast_slice(&gpu_cells));

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("render") });
        {
            let target_view = self.texture.as_ref().unwrap().create_view(&wgpu::TextureViewDescriptor::default());
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
            render_pass.draw(0..6, 0..(cols * rows) as u32);
        }

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: self.texture.as_ref().unwrap(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: self.staging_buffer.as_ref().unwrap(),
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d { width: content_w, height: content_h, depth_or_array_layers: 1 },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        let buffer_slice = self.staging_buffer.as_ref().unwrap().slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| { let _ = sender.send(v); });
        let _ = self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });

        if let Ok(Ok(())) = receiver.recv() {
            let data = buffer_slice.get_mapped_range();
            let byte_len = (content_w * content_h * 4) as usize;
            out.resize(byte_len, 0);
            for row in 0..content_h {
                let src_start = (row * padded) as usize;
                let src_end = src_start + unpadded as usize;
                let dst_start = (row * unpadded) as usize;
                let dst_end = dst_start + unpadded as usize;
                if src_end <= data.len() && dst_end <= out.len() {
                    out[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
                }
            }
            drop(data);
            self.staging_buffer.as_ref().unwrap().unmap();
        } else {
            tracing::error!("Failed to map staging buffer for wgpu cell renderer");
        }
    }
}