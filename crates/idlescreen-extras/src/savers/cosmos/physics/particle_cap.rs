use crate::cosmos::Cosmos;

/// Soft particle budget scaled by adaptive quality and battery. Prevents unbounded
/// growth during long accretion phases (merge bursts can add 100+ particles per event).
pub fn particle_budget(eff: &Cosmos) -> usize {
    let bat = if eff.on_battery { 0.55 } else { 1.0 };
    let floor = if eff.on_battery { 120.0 } else { 200.0 };
    (580.0 * eff.quality_scale * bat).max(floor) as usize
}

/// Drop lowest-energy particles when over budget.
pub fn trim_particles(eff: &mut Cosmos) {
    let budget = particle_budget(eff);
    let excess = eff.particles.len().saturating_sub(budget);
    if excess == 0 {
        return;
    }
    // Highest energy to the front — truncate keeps them, drops the rest.
    // (Previously sorted ascending, which kept the *lowest*-energy
    // particles and culled the hot/fast ones it meant to preserve.)
    eff.particles.sort_unstable_by(|a, b| {
        let ea = a.vx * a.vx + a.vy * a.vy;
        let eb = b.vx * b.vx + b.vy * b.vy;
        eb.partial_cmp(&ea).unwrap_or(std::cmp::Ordering::Equal)
    });
    eff.particles.truncate(budget);
}
