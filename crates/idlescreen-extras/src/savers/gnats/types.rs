pub struct Firefly {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub color: (u8, u8, u8),
    pub size: u8,
    pub speed_mult: f32,
    pub history: Vec<(i32, i32)>,
    /// Phase offset so blinks stay desynced across the swarm.
    pub blink_phase: f32,
    /// Individual blink rate (rad/s).
    pub blink_rate: f32,
}

pub struct Attractor {
    pub x: f32,
    pub y: f32,
    pub color: (u8, u8, u8),
    pub phase: f32,
    pub speed: f32,
}

pub struct Star {
    pub x: f32,
    pub y: f32,
    pub phase: f32,
    pub ch: char,
    pub excitation: f32,
}

pub struct KillSpark {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub color: (u8, u8, u8),
    pub life: f32,
}
