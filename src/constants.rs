// These constants were determined by examination of recorded gamplay (and fiddling)
// Units are seconds & tiles-per-second unless otherwise specified.

pub const GRAVITY_VEL: f32 = -1.0 / 0.12903225806451613;
pub fn apply_gravity(vertical_velocity: f32, dt: f32) -> f32 {
    vertical_velocity + (2.5 * dt * (GRAVITY_VEL - vertical_velocity))
}
