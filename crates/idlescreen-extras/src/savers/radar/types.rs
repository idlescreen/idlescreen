// SPDX-License-Identifier: MIT

#[derive(Clone, Debug)]
pub struct EnemyShip {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub visibility: f32,
    pub is_heavy: bool,
    pub target_type: u8, // 0: Scout, 1: Battery, 2: Nuke, 3: Boss
    pub health: u8,
    pub altitude: f32,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub struct GoodShip {
    pub x: f32,
    pub y: f32,
    pub target_id: u32,
    pub last_known_x: f32,
    pub last_known_y: f32,
    pub altitude: f32,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub struct RadarJet {
    pub x: f32,
    pub y: f32,
    pub heading_ch: char,
    pub sweep_angle: f32,
    pub t: f32,
    pub lock_target_id: Option<u32>,
    pub lock_timer: f32,
    pub altitude: f32,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub struct LaserBeam {
    pub from_x: f32,
    pub from_y: f32,
    pub to_x: f32,
    pub to_y: f32,
    pub life: f32,
    pub max_life: f32,
}

#[derive(Clone, Debug)]
pub struct ExplosionParticle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
}
