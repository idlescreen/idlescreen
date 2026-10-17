// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Deferred frame emission and panic-free colour helpers for ASCII scenes.

use std::cell::Cell;
use ttfx::engine::ctx::EngineCtx;
use ttfx::utils::graphics::Color;

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
    Color::from_hex(hex).unwrap_or_else(|_| Color::from_xterm(15))
}
