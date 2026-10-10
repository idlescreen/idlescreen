//! Auxiliary structs and constants for the bursts screensaver.

pub struct Rocket {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub color: (u8, u8, u8),
}

pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub color: (u8, u8, u8),
    pub ch: char,
    pub life: f32,
    pub max_life: f32,
}

pub struct Explosion {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub color: (u8, u8, u8),
    pub life: f32,
    pub max_life: f32,
}

pub struct ActiveExplosion {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub color: (u8, u8, u8),
    pub intensity: f32,
}

pub struct Star {
    pub x: f32,
    pub y: f32,
    pub phase: f32,
    pub ch: char,
    pub excitation: f32,
    pub excited_color: (u8, u8, u8),
}

pub const FIREWORK_COLORS: &[(u8, u8, u8)] = &[
    (255, 50, 50),   // Red
    (50, 255, 50),   // Green
    (50, 100, 255),  // Blue
    (255, 200, 0),   // Gold/Yellow
    (255, 50, 255),  // Magenta/Pink
    (0, 255, 255),   // Cyan
    (255, 128, 0),   // Orange
    (200, 200, 255), // Silver
];
