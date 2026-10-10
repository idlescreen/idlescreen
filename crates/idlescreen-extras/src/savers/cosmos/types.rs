//! Types and definitions for the cosmos screensaver universe lifecycle.

/// State of the universe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UniverseState {
    Darkness,
    BigBang,
    Expansion,
    Accretion,
    Singularity,
    Collapse,
}

/// A cosmic particle.
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub mass: f32,
    pub color: (u8, u8, u8),
    pub ch: char,
    pub history: Vec<(i32, i32)>,
}

/// A gravity center (star or black hole).
pub struct GravityCenter {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub mass: f32,
    pub color: (u8, u8, u8),
    pub active: bool,
    pub is_black_hole: bool,
    pub birth_timer: f32,
}

/// A character in the centered OS caption (physics + display).
pub struct LogoPixel {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub ch: char,
    pub exc: f32,
    pub active: bool,
    /// Letter dissolved into shard particles (hidden until reform completes).
    pub dissolved: bool,
    /// Shard particles still flying home.
    pub shards_pending: u8,
}
