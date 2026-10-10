pub struct Particle {
    pub home_x: f32,
    pub home_y: f32,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub ch: char,
    pub orig_ch: char,
    pub glow: f32,
    pub snapped: bool,
}

pub struct Star {
    pub x: f32,
    pub y: f32,
    pub phase: f32,
    pub ch: char,
    pub excitation: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Assembled,
    Exploding,
    Chaos,
    SnapBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplosionType {
    Supernova,
    BlackHole,
    Vortex,
    GlitchWave,
    Shockwave,
    Entropy,
    Resonance,
}

impl ExplosionType {
    pub fn next(self) -> Self {
        match self {
            ExplosionType::Supernova => ExplosionType::BlackHole,
            ExplosionType::BlackHole => ExplosionType::Vortex,
            ExplosionType::Vortex => ExplosionType::GlitchWave,
            ExplosionType::GlitchWave => ExplosionType::Shockwave,
            ExplosionType::Shockwave => ExplosionType::Entropy,
            ExplosionType::Entropy => ExplosionType::Resonance,
            ExplosionType::Resonance => ExplosionType::Supernova,
        }
    }
}
