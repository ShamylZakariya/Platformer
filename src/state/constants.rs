// These constants were determined by examination of recorded gamplay (and fiddling)
// Units are seconds & tiles-per-second unless otherwise specified.

pub mod sprite_masks {
    pub const COLLIDER: u32 = 1 << 0;
    pub const WATER: u32 = 1 << 1;
    pub const RATCHET: u32 = 1 << 2;
    pub const ENTITY: u32 = 1 << 3;
    pub const CONTACT_DAMAGE: u32 = 1 << 4;
    pub const SHOOTABLE: u32 = 1 << 5;
}

pub mod sprite_layers {
    pub const BACKGROUND: f32 = 0.9;
    pub const LEVEL: f32 = 0.8;
    pub const ENTITIES: f32 = 0.7;
    pub const PLAYER: f32 = 0.6;
    pub const FOREGROUND: f32 = 0.1;
}

// In original game, stage was 160px wide, 144px tall, made from 16px tiles, making the viewport 10 units wide.
pub const ORIGINAL_WINDOW_WIDTH: i32 = 160;
pub const ORIGINAL_WINDOW_HEIGHT: i32 = 144;
pub const ORIGINAL_VIEWPORT_TILES_WIDE: i32 = 10;
pub const MIN_CAMERA_SCALE: f32 = ORIGINAL_VIEWPORT_TILES_WIDE as f32;
pub const DEFAULT_CAMERA_SCALE: f32 = 16.0;
pub const MAX_CAMERA_SCALE: f32 = 32.0;

pub const GRAVITY_VEL: f32 = -1.0 / 0.129_032_25;
pub fn apply_gravity(vertical_velocity: f32, dt: f32) -> f32 {
    vertical_velocity + (2.5 * dt * (GRAVITY_VEL - vertical_velocity))
}
