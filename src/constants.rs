// These constants were determined by examination of recorded gamplay (and fiddling)
// Units are seconds & tiles-per-second unless otherwise specified.
pub const GRAVITY_VEL: f32 = -1.0 / 0.12903225806451613;
pub const WALK_SPEED: f32 = 1.0 / 0.4;
pub const JUMP_DURATION: f32 = 0.45;
pub const FLIGHT_DURATION: f32 = 1.0;
pub const FLIGHT_BOB_CYCLE_PERIOD: f32 = 0.5;
pub const FLIGHT_BOB_CYCLE_PIXELS_OFFSET: i32 = -2;
pub const WALLGRAB_JUMP_LATERAL_MOTION_DURATION: f32 = 0.17;
pub const WALLGRAB_JUMP_LATERAL_VEL: f32 = 20.0;
pub const WATER_DAMPING: f32 = 0.5;

// Animation timings
pub const WALK_CYCLE_DURATION: f32 = 0.2;
pub const FLIGHT_CYCLE_DURATION: f32 = 0.1;
pub const JUMP_CYCLE_DURATION: f32 = 0.1;

pub fn apply_gravity(vertical_velocity: f32, dt: f32) -> f32 {
    vertical_velocity + (2.5 * dt * (GRAVITY_VEL - vertical_velocity))
}
