//! Struct definitions for Hearth screensaver.

pub struct Ember {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
    pub size: u8,
    pub heat: f32,
}

pub struct Smoke {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
}

pub struct Log {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub phase: f32,
}

pub struct Tongue {
    pub offset: f32,
    pub phase: f32,
    pub speed: f32,
    pub height_scale: f32,
    pub width_scale: f32,
}
