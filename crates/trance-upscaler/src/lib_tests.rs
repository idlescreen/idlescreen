use super::*;

#[test]
fn resolve_render_scale_clamps_high() {
    let s = resolve_render_scale(false, Some(2.0));
    assert!(s <= 1.0);
}

#[test]
fn resolve_render_scale_clamps_low() {
    let s = resolve_render_scale(false, Some(0.1));
    assert!(s >= 0.25);
}

#[test]
fn resolve_render_scale_default_no_gpu() {
    let s = resolve_render_scale(false, None);
    assert!(s > 0.0 && s <= 1.0);
}

#[test]
fn gpu_enabled_returns_false() {
    assert!(!gpu_enabled());
}

#[test]
fn render_scale_in_range() {
    let s = render_scale();
    assert!(s > 0.0 && s <= 1.0);
}

#[test]
fn filter_mode_from_env_recognizes_nearest() {
    let prior = std::env::var("TRANCE_GPU_FILTER").ok();
    unsafe {
        std::env::set_var("TRANCE_GPU_FILTER", "nearest");
    }
    assert!(matches!(FilterMode::from_env(), FilterMode::Nearest));
    match prior {
        Some(v) => unsafe {
            std::env::set_var("TRANCE_GPU_FILTER", v);
        },
        None => unsafe {
            std::env::remove_var("TRANCE_GPU_FILTER");
        },
    }
}

#[test]
fn filter_mode_from_env_defaults_to_linear() {
    let prior = std::env::var("TRANCE_GPU_FILTER").ok();
    unsafe {
        std::env::remove_var("TRANCE_GPU_FILTER");
    }
    assert!(matches!(FilterMode::from_env(), FilterMode::Linear));
    if let Some(v) = prior {
        unsafe {
            std::env::set_var("TRANCE_GPU_FILTER", v);
        }
    }
}

#[test]
fn filter_mode_from_env_unknown_falls_back_to_linear() {
    let prior = std::env::var("TRANCE_GPU_FILTER").ok();
    unsafe {
        std::env::set_var("TRANCE_GPU_FILTER", "bogus");
    }
    assert!(matches!(FilterMode::from_env(), FilterMode::Linear));
    match prior {
        Some(v) => unsafe {
            std::env::set_var("TRANCE_GPU_FILTER", v);
        },
        None => unsafe {
            std::env::remove_var("TRANCE_GPU_FILTER");
        },
    }
}

#[test]
fn max_fps_zero_when_unset() {
    let prior = std::env::var("TRANCE_MAX_FPS").ok();
    unsafe {
        std::env::remove_var("TRANCE_MAX_FPS");
    }
    assert_eq!(max_fps(), 0);
    if let Some(v) = prior {
        unsafe {
            std::env::set_var("TRANCE_MAX_FPS", v);
        }
    }
}

#[test]
fn simulation_tick_hz_default_in_range() {
    let prior = std::env::var("TRANCE_TICK_HZ").ok();
    unsafe {
        std::env::remove_var("TRANCE_TICK_HZ");
    }
    let hz = simulation_tick_hz();
    assert!(hz >= 15.0 && hz <= 240.0);
    if let Some(v) = prior {
        unsafe {
            std::env::set_var("TRANCE_TICK_HZ", v);
        }
    }
}

#[test]
fn target_fps_matches_detected_when_unset() {
    let detected = 144;
    let prior = std::env::var("TRANCE_MAX_FPS").ok();
    unsafe {
        std::env::remove_var("TRANCE_MAX_FPS");
    }
    let fps = target_fps(detected);
    assert!((fps - detected as f32).abs() < f32::EPSILON);
    if let Some(v) = prior {
        unsafe {
            std::env::set_var("TRANCE_MAX_FPS", v);
        }
    }
}

#[test]
fn frame_upscaler_never_uses_gpu() {
    let upscaler = FrameUpscaler::new(true, FilterMode::Linear);
    assert!(!upscaler.using_gpu());
    assert_eq!(upscaler.adapter_name(), None);
}
