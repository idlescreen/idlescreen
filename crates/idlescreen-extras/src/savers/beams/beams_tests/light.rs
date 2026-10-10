use super::super::Spotlight;
use super::super::light::{LightContext, get_light_at};
use super::with_primary_monitor;

#[test]
fn test_math_light_falloff() {
    with_primary_monitor(|| {
        let spot = Spotlight {
            origin_x_ratio: 0.5,
            color_r: 255.0,
            color_g: 255.0,
            color_b: 255.0,
            angle_center: std::f32::consts::FRAC_PI_2,
            angle_amplitude: 0.0,
            phase: 0.0,
            phase_offset: 0.0,
            speed: 1.0,
            spread: 0.5,
            speed_bias: 1.0,
            motion_blend: 1.0,
            motion_timer: 5.0,
            is_calm: false,
        };
        let spotlights = vec![spot];
        let current_angles = vec![std::f32::consts::FRAC_PI_2];
        let half = 0.5_f32;
        let a_min = std::f32::consts::FRAC_PI_2 - half;
        let a_max = std::f32::consts::FRAC_PI_2 + half;
        let spot_cots = vec![(
            a_min,
            a_max,
            a_min.cos() / a_min.sin(),
            a_max.cos() / a_max.sin(),
            1.0 / half,
        )];

        let ctx = LightContext::new(80, 24, &spotlights);
        let (_, _, _, near) = get_light_at(
            40.0,
            22.0,
            &ctx,
            &spotlights,
            &current_angles,
            &spot_cots,
            (255, 200, 120),
            false,
        );
        let (_, _, _, far) = get_light_at(
            40.0,
            4.0,
            &ctx,
            &spotlights,
            &current_angles,
            &spot_cots,
            (255, 200, 120),
            false,
        );

        assert!(
            near > far,
            "near origin intensity ({near}) should exceed far intensity ({far})"
        );
        assert!(
            near > 0.0,
            "expected non-zero intensity near origin, got {near}"
        );
    });
}

#[test]
fn test_math_light_angle_boundary() {
    with_primary_monitor(|| {
        let spot = Spotlight {
            origin_x_ratio: 0.5,
            color_r: 255.0,
            color_g: 255.0,
            color_b: 255.0,
            angle_center: std::f32::consts::FRAC_PI_2,
            angle_amplitude: 0.0,
            phase: 0.0,
            phase_offset: 0.0,
            speed: 1.0,
            spread: 0.1,
            speed_bias: 1.0,
            motion_blend: 1.0,
            motion_timer: 5.0,
            is_calm: false,
        };
        let spotlights = vec![spot];
        let current_angles = vec![std::f32::consts::FRAC_PI_2];
        let spot_cots = vec![(
            std::f32::consts::FRAC_PI_2 - 0.1,
            std::f32::consts::FRAC_PI_2 + 0.1,
            (std::f32::consts::FRAC_PI_2 - 0.1).cos() / (std::f32::consts::FRAC_PI_2 - 0.1).sin(),
            (std::f32::consts::FRAC_PI_2 + 0.1).cos() / (std::f32::consts::FRAC_PI_2 + 0.1).sin(),
            1.0 / 0.1,
        )];

        let ctx = LightContext::new(80, 24, &spotlights);
        let (_, _, _, i_center) = get_light_at(
            40.0,
            20.0,
            &ctx,
            &spotlights,
            &current_angles,
            &spot_cots,
            (255, 200, 120),
            false,
        );
        let (_, _, _, i_offside) = get_light_at(
            20.0,
            20.0,
            &ctx,
            &spotlights,
            &current_angles,
            &spot_cots,
            (255, 200, 120),
            false,
        );

        assert!(i_center > 0.0);
        assert_eq!(i_offside, 0.0);
    });
}
