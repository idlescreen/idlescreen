// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Shared value parsers, deferred frame emission, and colour helpers for ASCII scenes.

use std::cell::Cell;
use ttfx::engine::ctx::EngineCtx;
use ttfx::utils::easing::Easing;
use ttfx::utils::graphics::{Color, GradientDirection};

thread_local! {
    static DEFER_FRAME_RENDER: Cell<bool> = const { Cell::new(false) };
}

pub fn set_defer_frame_render(defer: bool) {
    DEFER_FRAME_RENDER.with(|c| c.set(defer));
}

pub fn emit_frame(ctx: &mut EngineCtx) -> String {
    if DEFER_FRAME_RENDER.with(Cell::get) {
        ctx.clock.advance_frame();
        String::new()
    } else {
        ctx.frame()
    }
}

pub fn hex_color(hex: &str) -> Color {
    Color::from_hex(hex).must_val()
}

pub trait SceneValueExt<T> {
    fn must_val(self) -> T;
}

impl<T> SceneValueExt<T> for Option<T> {
    #[inline]
    fn must_val(self) -> T {
        match self {
            Some(v) => v,
            None => unreachable!("scene state invariant violated"),
        }
    }
}

impl<T, E: std::fmt::Debug> SceneValueExt<T> for Result<T, E> {
    #[inline]
    fn must_val(self) -> T {
        match self {
            Ok(v) => v,
            Err(e) => unreachable!("scene result invariant violated: {e:?}"),
        }
    }
}

pub fn parse_positive_float(s: &str) -> Result<f64, String> {
    let v: f64 = s
        .parse()
        .map_err(|_| format!("invalid float value: '{s}'"))?;
    if v > 0.0 {
        Ok(v)
    } else {
        Err(format!(
            "{v} is not a valid value. Argument must be a float > 0."
        ))
    }
}

pub fn parse_non_negative_float(s: &str) -> Result<f64, String> {
    let v: f64 = s
        .parse()
        .map_err(|_| format!("invalid float value: '{s}'"))?;
    if v >= 0.0 {
        Ok(v)
    } else {
        Err(format!(
            "{v} is not a valid value. Argument must be a float >= 0."
        ))
    }
}

pub fn parse_gradient_steps(s: &str) -> Result<i64, String> {
    let v: i64 = s.parse().map_err(|_| format!("invalid int value: '{s}'"))?;
    if v > 0 {
        Ok(v)
    } else {
        Err(format!(
            "{v} is not a valid value. Argument must be an int > 0."
        ))
    }
}

pub fn parse_positive_int(s: &str) -> Result<i64, String> {
    parse_gradient_steps(s)
}

pub fn parse_non_negative_int(s: &str) -> Result<i64, String> {
    let v: i64 = s.parse().map_err(|_| format!("invalid int value: '{s}'"))?;
    if v >= 0 {
        Ok(v)
    } else {
        Err(format!(
            "{v} is not a valid value. Argument must be an int >= 0."
        ))
    }
}

pub fn parse_non_negative_ratio(s: &str) -> Result<f64, String> {
    let v: f64 = s
        .parse()
        .map_err(|_| format!("invalid float value: '{s}'"))?;
    if (0.0..=1.0).contains(&v) {
        Ok(v)
    } else {
        Err(format!(
            "{v} is not a valid value. Argument must be a float 0 <= n <= 1."
        ))
    }
}

pub fn parse_positive_ratio(s: &str) -> Result<f64, String> {
    let v: f64 = s
        .parse()
        .map_err(|_| format!("invalid float value: '{s}'"))?;
    if v > 0.0 && v <= 1.0 {
        Ok(v)
    } else {
        Err(format!(
            "{v} is not a valid value. Argument must be a float 0 < n <= 1."
        ))
    }
}

pub fn parse_gradient_direction(s: &str) -> Result<GradientDirection, String> {
    match s {
        "horizontal" => Ok(GradientDirection::Horizontal),
        "vertical" => Ok(GradientDirection::Vertical),
        "diagonal" => Ok(GradientDirection::Diagonal),
        "radial" => Ok(GradientDirection::Radial),
        _ => Err(format!("invalid gradient direction: '{s}'")),
    }
}

pub fn parse_easing(s: &str) -> Result<Easing, String> {
    Easing::parse(s).ok_or_else(|| format!("invalid easing function: '{s}'"))
}

pub fn parse_character_group(s: &str) -> Result<ttfx::engine::terminal::CharacterGroup, String> {
    use ttfx::engine::terminal::CharacterGroup::*;
    Ok(match s {
        "column_left_to_right" => ColumnLeftToRight,
        "column_right_to_left" => ColumnRightToLeft,
        "row_top_to_bottom" => RowTopToBottom,
        "row_bottom_to_top" => RowBottomToTop,
        "diagonal_top_left_to_bottom_right" => DiagonalTopLeftToBottomRight,
        "diagonal_bottom_left_to_top_right" => DiagonalBottomLeftToTopRight,
        "diagonal_top_right_to_bottom_left" => DiagonalTopRightToBottomLeft,
        "diagonal_bottom_right_to_top_left" => DiagonalBottomRightToTopLeft,
        "center_to_outside" => CenterToOutside,
        "outside_to_center" => OutsideToCenter,
        _ => return Err(format!("invalid character group: '{s}'")),
    })
}

pub fn parse_character_sort(s: &str) -> Result<ttfx::engine::terminal::CharacterSort, String> {
    use ttfx::engine::terminal::CharacterSort::*;
    Ok(match s {
        "random" => Random,
        "top_to_bottom_left_to_right" => TopToBottomLeftToRight,
        "top_to_bottom_right_to_left" => TopToBottomRightToLeft,
        "bottom_to_top_left_to_right" => BottomToTopLeftToRight,
        "bottom_to_top_right_to_left" => BottomToTopRightToLeft,
        "outside_row_to_middle" => OutsideRowToMiddle,
        "middle_row_to_outside" => MiddleRowToOutside,
        _ => return Err(format!("invalid character sort: '{s}'")),
    })
}

pub fn parse_symbol(s: &str) -> Result<String, String> {
    if s.chars().count() == 1 {
        Ok(s.to_string())
    } else {
        Err(format!(
            "invalid symbol: '{s}' argument must be a single character"
        ))
    }
}

pub fn parse_positive_int_range(s: &str) -> Result<(i64, i64), String> {
    let (a, b) = s
        .split_once('-')
        .ok_or_else(|| format!("invalid range: '{s}'"))?;
    let a: i64 = a.parse().map_err(|_| format!("invalid range: '{s}'"))?;
    let b: i64 = b.parse().map_err(|_| format!("invalid range: '{s}'"))?;
    if a > 0 && a <= b {
        Ok((a, b))
    } else {
        Err(format!("invalid range: '{s}'"))
    }
}

pub fn parse_positive_float_range(s: &str) -> Result<(f64, f64), String> {
    let (a, b) = s
        .split_once('-')
        .ok_or_else(|| format!("invalid range: '{s}'"))?;
    let a: f64 = a.parse().map_err(|_| format!("invalid range: '{s}'"))?;
    let b: f64 = b.parse().map_err(|_| format!("invalid range: '{s}'"))?;
    if a > 0.0 && a <= b {
        Ok((a, b))
    } else {
        Err(format!("invalid range: '{s}'"))
    }
}
