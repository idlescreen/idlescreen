// SPDX-License-Identifier: MIT

//! GPU upscaling for trance screensaver frames.
//!
//! Uses [wgpu](https://wgpu.rs/) (Vulkan or OpenGL) for cross-vendor support on Linux:
//! AMD (RADV), Intel (ANV), and NVIDIA (proprietary or Nouveau drivers).

mod cpu;
mod gpu;

pub use gpu::FrameUpscaler;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMode {
    Nearest,
    Linear,
}

impl FilterMode {
    pub fn from_env() -> Self {
        match std::env::var("TRANCE_GPU_FILTER").as_deref() {
            Ok("nearest") => Self::Nearest,
            _ => Self::Linear,
        }
    }
}

/// Whether GPU upscaling should be attempted (default: yes).
pub fn gpu_enabled() -> bool {
    !matches!(
        std::env::var("TRANCE_GPU").as_deref(),
        Ok("0") | Ok("false") | Ok("off")
    )
}

/// Simulation grid scale factor in `(0, 1]`. Lower values render chunkier effects
/// that are upscaled to the monitor resolution.
pub fn render_scale() -> f32 {
    render_scale_for_gpu(gpu_enabled())
}

pub fn render_scale_for_gpu(use_gpu: bool) -> f32 {
    resolve_render_scale(use_gpu, None)
}

/// Effective simulation grid scale: env `TRANCE_RENDER_SCALE`, then config, then defaults.
pub fn resolve_render_scale(use_gpu: bool, configured: Option<f32>) -> f32 {
    if let Some(scale) = std::env::var("TRANCE_RENDER_SCALE").ok().and_then(|v| v.parse::<f32>().ok()) {
        return scale.clamp(0.25, 1.0);
    }
    if let Some(scale) = configured {
        return scale.clamp(0.25, 1.0);
    }
    if use_gpu { 1.0 } else { 0.5 }
}

/// Presentation frame-rate cap. `0` means match the detected monitor refresh rate.
pub fn max_fps() -> u32 {
    std::env::var("TRANCE_MAX_FPS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0)
}

/// Physics / simulation tick rate (Hz). Independent of monitor refresh.
pub fn simulation_tick_hz() -> f32 {
    std::env::var("TRANCE_TICK_HZ")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .map(|hz| hz.clamp(15.0, 240.0))
        .unwrap_or(60.0)
}

pub fn target_fps(detected_refresh_hz: u32) -> f32 {
    let detected = detected_refresh_hz.max(60);
    let cap = max_fps();
    if cap == 0 {
        detected as f32
    } else {
        detected.min(cap) as f32
    }
}