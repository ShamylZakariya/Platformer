// These constants were determined by examination of recorded gamplay (and fiddling)
// Units are seconds & tiles-per-second unless otherwise specified.

pub mod sprite_masks {
    pub const GROUND: u32 = 1 << 0;
    pub const WATER: u32 = 1 << 1;
    pub const RATCHET: u32 = 1 << 2;
    pub const ENTITY: u32 = 1 << 3;
    pub const CONTACT_DAMAGE: u32 = 1 << 4;
    pub const SHOOTABLE: u32 = 1 << 5;
    pub const PLAYER: u32 = 1 << 6;

    pub mod ui {
        pub const HEALTH_DOT: u32 = 1 << 0;
    }
}

pub mod layers {
    pub mod stage {
        pub const EXIT: f32 = 95.0;
        pub const BACKGROUND: f32 = 90.0;
        pub const LEVEL: f32 = 80.0;
        pub const ENTITIES: f32 = 70.0;
        pub const FIREBRAND: f32 = 60.0;
        pub const FOREGROUND: f32 = 50.0;
    }

    pub mod ui {
        pub const BACKGROUND: f32 = 40.0;
        pub const FOREGROUND: f32 = 10.0;
    }
}

// In original game, stage was 160px wide, 144px tall, made from 16px tiles, making the viewport 10 units wide.
pub const ORIGINAL_WINDOW_WIDTH: i32 = 160;
pub const ORIGINAL_WINDOW_HEIGHT: i32 = 144;
pub const ORIGINAL_VIEWPORT_TILES_WIDE: i32 = 10;
pub const MIN_CAMERA_SCALE: f32 = ORIGINAL_VIEWPORT_TILES_WIDE as f32;
pub const DEFAULT_CAMERA_SCALE: f32 = 16.0;
pub const MAX_CAMERA_SCALE: f32 = 32.0;
pub const CAMERA_NEAR_PLANE: f32 = 0.0;
pub const CAMERA_FAR_PLANE: f32 = 100.0;

pub const GRAVITY_VEL: f32 = -1.0 / 0.129_032_25;
pub fn apply_gravity(vertical_velocity: f32, dt: f32) -> f32 {
    vertical_velocity + (2.5 * dt * (GRAVITY_VEL - vertical_velocity))
}
