// These constants were determined by examination of recorded gamplay (and fiddling)
// Units are seconds & tiles-per-second unless otherwise specified.

pub mod sprite_masks {
    pub const COLLIDER: u32 = 1 << 31;
    pub const WATER: u32 = 1 << 30;
    pub const RATCHET: u32 = 1 << 29;
    pub const ENTITY: u32 = 1 << 28;
    pub const CONTACT_DAMAGE: u32 = 1 << 27;
    pub const SHOOTABLE: u32 = 1 << 26;
}

// In original game, stage was 160 px wide, made from 16px tiles, making the viewport 10 units wide.
pub const ORIGINAL_VIEWPORT_TILES_WIDE:i32 = 10;

pub const GRAVITY_VEL: f32 = -1.0 / 0.12903225806451613;
pub fn apply_gravity(vertical_velocity: f32, dt: f32) -> f32 {
    vertical_velocity + (2.5 * dt * (GRAVITY_VEL - vertical_velocity))
}
