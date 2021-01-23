use cgmath::*;
use std::{f32::consts::PI, time::Duration};

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, collision, rendering},
    state::constants::{sprite_masks, ORIGINAL_VIEWPORT_TILES_WIDE},
    tileset,
};

use super::util::HitPointState;

// --------------------------------------------------------------------------------------------------------------------

const ANIMATION_CYCLE_DURATION: f32 = 0.133;
const MOVEMENT_SPEED: f32 = 1.0; // units per second
const HIT_POINTS: i32 = 1;
const PLAYER_CLOSENESS_THRESHOLD: f32 = (ORIGINAL_VIEWPORT_TILES_WIDE as f32 / 2.0) - 1.0;
const SIN_PI_4: f32 = 0.707_106_77;
const TAU: f32 = 2.0 * PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChaseDir {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl ChaseDir {
    fn new(dir: Vector2<f32>) -> Self {
        let ndir = dir.normalize();
        let mut angle = ndir.y.atan2(ndir.x);
        if angle < 0.0 {
            angle += TAU;
        }
        let sector = (angle / (TAU / 16.0)).round() as i32;
        match sector {
            0 | 15 => ChaseDir::East,
            1 | 2 => ChaseDir::NorthEast,
            3 | 4 => ChaseDir::North,
            5 | 6 => ChaseDir::NorthWest,
            7 | 8 => ChaseDir::West,
            9 | 10 => ChaseDir::SouthWest,
            11 | 12 => ChaseDir::South,
            13 | 14 => ChaseDir::SouthEast,
            _ => panic!("sector expected to be in range [0,15]"),
        }
    }

    fn to_dir(&self) -> Vector2<f32> {
        let t = SIN_PI_4;
        match self {
            ChaseDir::North => vec2(0.0, 1.0),
            ChaseDir::NorthEast => vec2(t, t),
            ChaseDir::East => vec2(1.0, 1.0),
            ChaseDir::SouthEast => vec2(t, -t),
            ChaseDir::South => vec2(0.0, -1.0),
            ChaseDir::SouthWest => vec2(-t, -t),
            ChaseDir::West => vec2(-1.0, 0.0),
            ChaseDir::NorthWest => vec2(-t, t),
        }
    }
}

#[cfg(test)]
mod chase_dir_tests {
    use super::*;

    #[test]
    fn new_produces_expected_values() {
        assert_eq!(ChaseDir::new(vec2(0.0, 1.0)), ChaseDir::North);
        assert_eq!(ChaseDir::new(vec2(0.0, -1.0)), ChaseDir::South);
        assert_eq!(ChaseDir::new(vec2(1.0, 0.0)), ChaseDir::East);
        assert_eq!(ChaseDir::new(vec2(-1.0, 0.0)), ChaseDir::West);

        assert_eq!(ChaseDir::new(vec2(1.0, 1.0)), ChaseDir::NorthEast);
        assert_eq!(ChaseDir::new(vec2(1.0, -1.0)), ChaseDir::SouthEast);
        assert_eq!(ChaseDir::new(vec2(-1.0, -1.0)), ChaseDir::SouthWest);
        assert_eq!(ChaseDir::new(vec2(-1.0, 1.0)), ChaseDir::NorthWest);
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub struct Bat {
    entity_id: u32,
    spawn_point_id: u32,
    sprite: sprite::Sprite,
    sprite_size_px: Vector2<f32>,
    position: Point3<f32>,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
    chase_dir: Option<ChaseDir>,
    life: HitPointState,
}

impl Default for Bat {
    fn default() -> Self {
        Self {
            entity_id: 0,
            spawn_point_id: 0,
            sprite: sprite::Sprite::default(),
            sprite_size_px: vec2(0.0, 0.0),
            position: point3(0.0, 0.0, 0.0),
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
            chase_dir: None,
            life: HitPointState::new(HIT_POINTS),
        }
    }
}

impl Entity for Bat {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        map: &map::Map,
        collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.spawn_point_id = sprite
            .entity_id
            .expect("Spawned entities expect to find a spawn point id from the sprite");

        self.position = sprite.origin;
        self.sprite_size_px = map.tileset.get_sprite_size().cast().unwrap();

        // Make copy of sprite for ourselves, we'll use it for collision testing
        // Note: The map sprite is our spawn point, so we need to overwrite the entity_id and mask
        self.sprite = *sprite;
        self.sprite.entity_id = Some(entity_id);
        self.sprite.mask =
            sprite_masks::SHOOTABLE | sprite_masks::COLLIDER | sprite_masks::CONTACT_DAMAGE;
        self.sprite.collision_shape = sprite::CollisionShape::Square;
        collision_space.add_dynamic_sprite(&self.sprite);
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
        game_state_peek: &GameStatePeek,
    ) {
        //
        // Update life state
        //

        if self.life.update(
            self.entity_id(),
            self.spawn_point_id,
            self.position(),
            collision_space,
            message_dispatcher,
        ) {
            // Determine if the player is close enough for bat to wakeup
            if self.chase_dir.is_none()
                && (game_state_peek.player_position.x - self.position.x).abs()
                    < PLAYER_CLOSENESS_THRESHOLD
            {
                self.chase_dir = Some(ChaseDir::new(
                    game_state_peek.player_position - self.position.xy(),
                ));
            }

            let dt = dt.as_secs_f32();
            if let Some(chase_dir) = self.chase_dir {
                let dp = chase_dir.to_dir() * MOVEMENT_SPEED * dt;
                self.position.x += dp.x;
                self.position.y += dp.y;
            }

            //
            //  Update the sprite for collision detection
            //

            self.sprite.origin.x = self.position.x;
            self.sprite.origin.y = self.position.y;
            collision_space.update_dynamic_sprite(&self.sprite);

            //
            //  Update sprite animation cycle
            //

            self.animation_cycle_tick_countdown -= dt;
            if self.animation_cycle_tick_countdown <= 0.0 {
                self.animation_cycle_tick_countdown += ANIMATION_CYCLE_DURATION;
                self.animation_cycle_tick += 1;
            }
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        uniforms.data.set_model_position(self.position);
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::Bat
    }

    fn is_alive(&self) -> bool {
        self.life.is_alive()
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "bat"
    }

    fn sprite_cycle(&self) -> &str {
        if self.chase_dir.is_some() {
            if self.animation_cycle_tick % 2 == 0 {
                "fly_0"
            } else {
                "fly_1"
            }
        } else {
            "default"
        }
    }

    fn handle_message(&mut self, message: &Message) {
        self.life.handle_message(message);
    }

    fn did_exit_viewport(&mut self) {
        if self.chase_dir.is_some() {
            self.life.terminate();
        }
    }
}
