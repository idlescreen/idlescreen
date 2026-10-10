// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Fractional scale coordinate calculation conforming to wp_fractional_scale_v1.

/// Calculates the physical buffer dimension from logical dimension and scale in 120ths.
///
/// Uses standardized wlroots/Smithay integer rounding:
/// `buf_dim = (logical_dim * scale_120ths + 60) / 120`
#[inline]
pub fn calculate_buffer_dimension(logical_dim: u32, scale_120ths: u32) -> u32 {
    if scale_120ths == 0 || scale_120ths == 120 {
        return logical_dim;
    }
    (logical_dim.saturating_mul(scale_120ths).saturating_add(60)) / 120
}

/// Calculates physical buffer dimensions (width, height) from logical dimensions and scale_120ths.
pub fn calculate_buffer_size(logical_w: u32, logical_h: u32, scale_120ths: u32) -> (u32, u32) {
    (
        calculate_buffer_dimension(logical_w, scale_120ths),
        calculate_buffer_dimension(logical_h, scale_120ths),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fractional_scale_calculations() {
        // 100% scale (120/120)
        assert_eq!(calculate_buffer_size(1920, 1080, 120), (1920, 1080));

        // 125% scale (150/120)
        // 1920 * 150 = 288000 + 60 = 288060 / 120 = 2400
        // 1080 * 150 = 162000 + 60 = 162060 / 120 = 1350
        assert_eq!(calculate_buffer_size(1920, 1080, 150), (2400, 1350));

        // 150% scale (180/120)
        assert_eq!(calculate_buffer_size(1920, 1080, 180), (2880, 1620));

        // 175% scale (210/120)
        assert_eq!(calculate_buffer_size(1920, 1080, 210), (3360, 1890));

        // 200% scale (240/120)
        assert_eq!(calculate_buffer_size(1920, 1080, 240), (3840, 2160));
    }
}
