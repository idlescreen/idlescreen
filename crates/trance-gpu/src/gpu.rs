// SPDX-License-Identifier: MIT

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::cpu;
use crate::FilterMode;

const SHADER: &str = r#"
struct Params {
    dst_size: vec2<f32>,
    rect_origin: vec2<f32>,
    rect_size: vec2<f32>,
    _pad: vec2<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var src_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let pos = positions[vertex_index];
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = (pos + vec2<f32>(1.0, 1.0)) * 0.5;
    return out;
}

@fragment
fn fs(input: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = input.uv * params.dst_size;
    let rect_min = params.rect_origin;
    let rect_max = params.rect_origin + params.rect_size;
    if (pixel.x < rect_min.x || pixel.y < rect_min.y || pixel.x >= rect_max.x || pixel.y >= rect_max.y) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let local = (pixel - rect_min) / params.rect_size;
    let uv = vec2<f32>(local.x, 1.0 - local.y);
    return textureSample(src_tex, src_sampler, uv);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    dst_size: [f32; 2],
    rect_origin: [f32; 2],
    rect_size: [f32; 2],
    _pad: [f32; 2],
}

pub struct GpuUpscaler {
    backend_label: String,
    adapter_name: String,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    sampler_linear: wgpu::Sampler,
    sampler_nearest: wgpu::Sampler,
    src_texture: Option<TextureSlot>,
    dst_texture: Option<TextureSlot>,
    readback: [Option<ReadbackSlot>; 2],
    readback_index: usize,
    src_rgba: Vec<u8>,
    src_rgba_dims: (u32, u32),
    output_bgra: Vec<u8>,
    output_dims: (u32, u32),
}

struct TextureSlot {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

struct ReadbackSlot {
    width: u32,
    height: u32,
    buffer: wgpu::Buffer,
    bytes_per_row: u32,
}

impl GpuUpscaler {
    pub fn new() -> Result<Self, String> {
        pollster::block_on(Self::new_async())
    }

    async fn new_async() -> Result<Self, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| "no compatible GPU adapter found".to_string())?;

        let backend_label = format!("{:?}", adapter.get_info().backend);
        let adapter_name = adapter.get_info().name.clone();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("trance-gpu"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|error| format!("failed to open GPU device: {error}"))?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("trance-upscale"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("trance-upscale-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("trance-upscale-pipeline-layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("trance-upscale-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler_linear = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("trance-linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let sampler_nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("trance-nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            backend_label,
            adapter_name,
            device,
            queue,
            pipeline,
            bind_layout,
            sampler_linear,
            sampler_nearest,
            src_texture: None,
            dst_texture: None,
            readback: [None, None],
            readback_index: 0,
            src_rgba: Vec::new(),
            src_rgba_dims: (0, 0),
            output_bgra: Vec::new(),
            output_dims: (0, 0),
        })
    }

    pub fn backend_label(&self) -> &str {
        &self.backend_label
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    pub fn upscale_letterbox(
        &mut self,
        src: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
        filter: FilterMode,
    ) -> Result<Vec<u8>, String> {
        if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
            return Ok(vec![0u8; (dst_w * dst_h * 4) as usize]);
        }

        let expected = (src_w * src_h * 4) as usize;
        if src.len() < expected {
            return Err("source pixel buffer is too small".into());
        }

        let scale = (dst_w as f32 / src_w as f32).min(dst_h as f32 / src_h as f32);
        let display_w = (src_w as f32 * scale).floor().max(1.0);
        let display_h = (src_h as f32 * scale).floor().max(1.0);
        let offset_x = ((dst_w as f32 - display_w) * 0.5).floor();
        let offset_y = ((dst_h as f32 - display_h) * 0.5).floor();

        self.upscale_rect(
            src,
            src_w,
            src_h,
            dst_w,
            dst_h,
            [offset_x, offset_y],
            [display_w, display_h],
            filter,
        )
    }

    pub fn upscale_stretch(
        &mut self,
        src: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
    ) -> Result<Vec<u8>, String> {
        if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
            return Ok(vec![0u8; (dst_w * dst_h * 4) as usize]);
        }

        let expected = (src_w * src_h * 4) as usize;
        if src.len() < expected {
            return Err("source pixel buffer is too small".into());
        }

        self.upscale_rect(
            src,
            src_w,
            src_h,
            dst_w,
            dst_h,
            [0.0, 0.0],
            [dst_w as f32, dst_h as f32],
            FilterMode::Nearest,
        )
    }

    fn upscale_rect(
        &mut self,
        src: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
        rect_origin: [f32; 2],
        rect_size: [f32; 2],
        filter: FilterMode,
    ) -> Result<Vec<u8>, String> {
        bgra_to_rgba_into(&mut self.src_rgba, &mut self.src_rgba_dims, src, src_w, src_h);
        self.ensure_src_texture(src_w, src_h);
        self.ensure_dst_texture(dst_w, dst_h);
        self.ensure_readback(dst_w, dst_h);

        let src_slot = self.src_texture.as_ref().expect("source texture");
        let dst_slot = self.dst_texture.as_ref().expect("destination texture");
        let readback_idx = self.readback_index;
        let readback = self.readback[readback_idx]
            .as_ref()
            .expect("readback buffer");

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &src_slot.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.src_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(src_w * 4),
                rows_per_image: Some(src_h),
            },
            wgpu::Extent3d {
                width: src_w,
                height: src_h,
                depth_or_array_layers: 1,
            },
        );

        let params = Params {
            dst_size: [dst_w as f32, dst_h as f32],
            rect_origin,
            rect_size,
            _pad: [0.0, 0.0],
        };

        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("trance-upscale-params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let sampler = match filter {
            FilterMode::Linear => &self.sampler_linear,
            FilterMode::Nearest => &self.sampler_nearest,
        };

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("trance-upscale-bind-group"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&src_slot.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("trance-upscale-encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("trance-upscale-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &dst_slot.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &dst_slot.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(readback.bytes_per_row),
                    rows_per_image: Some(dst_h),
                },
            },
            wgpu::Extent3d {
                width: dst_w,
                height: dst_h,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        let slice = readback.buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|_| "GPU readback channel closed".to_string())?
            .map_err(|error| format!("GPU readback failed: {error}"))?;

        let mapped = slice.get_mapped_range();
        let row_bytes = (dst_w * 4) as usize;
        let needed = (dst_w * dst_h * 4) as usize;
        if self.output_dims != (dst_w, dst_h) || self.output_bgra.len() != needed {
            self.output_bgra.resize(needed, 0);
            self.output_dims = (dst_w, dst_h);
        }
        for row in 0..dst_h as usize {
            let src_start = row * readback.bytes_per_row as usize;
            let src_end = src_start + row_bytes;
            let dst_start = row * row_bytes;
            let dst_end = dst_start + row_bytes;
            if src_end <= mapped.len() && dst_end <= self.output_bgra.len() {
                self.output_bgra[dst_start..dst_end]
                    .copy_from_slice(&mapped[src_start..src_end]);
            }
        }
        drop(mapped);
        readback.buffer.unmap();
        self.readback_index = 1 - readback_idx;

        rgba_to_bgra_inplace(&mut self.output_bgra);
        Ok(self.output_bgra.clone())
    }

    fn ensure_src_texture(&mut self, width: u32, height: u32) {
        if self
            .src_texture
            .as_ref()
            .is_some_and(|slot| slot.width == width && slot.height == height)
        {
            return;
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("trance-src"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        self.src_texture = Some(TextureSlot {
            width,
            height,
            texture,
            view,
        });
    }

    fn ensure_dst_texture(&mut self, width: u32, height: u32) {
        if self
            .dst_texture
            .as_ref()
            .is_some_and(|slot| slot.width == width && slot.height == height)
        {
            return;
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("trance-dst"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        self.dst_texture = Some(TextureSlot {
            width,
            height,
            texture,
            view,
        });
    }

    fn ensure_readback(&mut self, width: u32, height: u32) {
        let bytes_per_row = align_to(width * 4, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        for slot in &mut self.readback {
            if slot
                .as_ref()
                .is_some_and(|s| s.width == width && s.height == height)
            {
                continue;
            }

            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("trance-readback"),
                size: (bytes_per_row * height) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            *slot = Some(ReadbackSlot {
                width,
                height,
                buffer,
                bytes_per_row,
            });
        }
    }
}

pub struct FrameUpscaler {
    gpu: Option<GpuUpscaler>,
    filter: FilterMode,
    stretch_buf: Vec<u8>,
    stretch_dims: (u32, u32, u32, u32),
    stretch_cache: cpu::StretchCache,
    letterbox_buf: Vec<u8>,
    letterbox_dims: (u32, u32, u32, u32),
}

impl FrameUpscaler {
    pub fn new(prefer_gpu: bool, filter: FilterMode) -> Self {
        let gpu = if prefer_gpu {
            match GpuUpscaler::new() {
                Ok(gpu) => {
                    println!(
                        "trance-gpu: using {} ({})",
                        gpu.adapter_name(),
                        gpu.backend_label()
                    );
                    Some(gpu)
                }
                Err(error) => {
                    eprintln!("trance-gpu: GPU unavailable ({error}); using CPU upscale");
                    None
                }
            }
        } else {
            println!("trance-gpu: GPU disabled; using CPU upscale");
            None
        };

        Self {
            gpu,
            filter,
            stretch_buf: Vec::new(),
            stretch_dims: (0, 0, 0, 0),
            stretch_cache: cpu::StretchCache::new(),
            letterbox_buf: Vec::new(),
            letterbox_dims: (0, 0, 0, 0),
        }
    }

    fn ensure_stretch_buf(&mut self, src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) {
        let dims = (src_w, src_h, dst_w, dst_h);
        let needed = (dst_w * dst_h * 4) as usize;
        if self.stretch_dims != dims || self.stretch_buf.len() != needed {
            self.stretch_buf.resize(needed, 0);
            self.stretch_dims = dims;
        }
    }

    fn ensure_letterbox_buf(&mut self, src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) {
        let dims = (src_w, src_h, dst_w, dst_h);
        let needed = (dst_w * dst_h * 4) as usize;
        if self.letterbox_dims != dims || self.letterbox_buf.len() != needed {
            self.letterbox_buf.resize(needed, 0);
            self.letterbox_dims = dims;
        }
    }

    pub fn using_gpu(&self) -> bool {
        self.gpu.is_some()
    }

    pub fn adapter_name(&self) -> Option<&str> {
        self.gpu.as_ref().map(|gpu| gpu.adapter_name())
    }

    pub fn upscale_letterbox(
        &mut self,
        src: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
    ) -> Vec<u8> {
        if let Some(gpu) = self.gpu.as_mut() {
            match gpu.upscale_letterbox(src, src_w, src_h, dst_w, dst_h, self.filter) {
                Ok(pixels) => return pixels,
                Err(error) => eprintln!("trance-gpu: frame upscale failed ({error}); CPU fallback"),
            }
        }

        self.ensure_letterbox_buf(src_w, src_h, dst_w, dst_h);
        cpu::upscale_letterbox_into(
            &mut self.letterbox_buf,
            src,
            src_w,
            src_h,
            dst_w,
            dst_h,
            self.filter,
        );
        self.letterbox_buf.clone()
    }

    /// Stretch source to fill the destination (fullscreen presentation path).
    pub fn upscale_stretch(
        &mut self,
        src: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
    ) -> Vec<u8> {
        if let Some(gpu) = self.gpu.as_mut() {
            match gpu.upscale_stretch(src, src_w, src_h, dst_w, dst_h) {
                Ok(pixels) => return pixels,
                Err(error) => eprintln!("trance-gpu: stretch upscale failed ({error}); CPU fallback"),
            }
        }

        self.ensure_stretch_buf(src_w, src_h, dst_w, dst_h);
        cpu::upscale_stretch_into(
            &mut self.stretch_buf,
            src,
            src_w,
            src_h,
            dst_w,
            dst_h,
            &mut self.stretch_cache,
        );
        self.stretch_buf.clone()
    }
}

fn align_to(value: u32, alignment: u32) -> u32 {
    ((value + alignment - 1) / alignment) * alignment
}

fn bgra_to_rgba_into(
    dst: &mut Vec<u8>,
    dims: &mut (u32, u32),
    src: &[u8],
    width: u32,
    height: u32,
) {
    let needed = (width * height * 4) as usize;
    if *dims != (width, height) || dst.len() != needed {
        dst.resize(needed, 0);
        *dims = (width, height);
    }
    for (src_px, dst_px) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        dst_px[0] = src_px[2];
        dst_px[1] = src_px[1];
        dst_px[2] = src_px[0];
        dst_px[3] = src_px[3];
    }
}

fn rgba_to_bgra_inplace(pixels: &mut [u8]) {
    for px in pixels.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
}