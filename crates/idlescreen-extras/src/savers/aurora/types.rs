/// One aurora curtain: a waving vertical light sheet. The bright edge is a
/// sine-warped line; intensity falls off exponentially below it.
pub struct Curtain {
    /// Resting edge height as a fraction of rows (0 = top).
    pub base_y: f32,
    /// Primary wave amplitude as a fraction of rows.
    pub amp: f32,
    /// Primary spatial frequency (radians per cell-ish).
    pub freq: f32,
    /// Primary temporal speed.
    pub speed: f32,
    pub phase: f32,
    /// Secondary harmonic (skipped when quality_scale is low).
    pub amp2: f32,
    pub freq2: f32,
    /// Falloff thickness below the edge, in cells.
    pub sigma: f32,
    /// 0 = aurora green, 1 = theme accent.
    pub hue_mix: f32,
    /// Global brightness multiplier (the surge curtain breathes with it).
    pub strength: f32,
}

/// A static twinkling star in the sky band behind the curtains.
pub struct Star {
    /// Normalized position (0..1 of grid).
    pub x: f32,
    pub y: f32,
    pub phase: f32,
    pub rate: f32,
    pub ch: char,
}
