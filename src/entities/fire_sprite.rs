use cgmath::*;
use std::time::Duration;

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, collision, rendering},
    state::constants::sprite_masks,
    tileset,
};

use super::util::{Direction, HitPointState, MarchState};

// --------------------------------------------------------------------------------------------------------------------

const ANIMATION_CYCLE_DURATION: f32 = 0.133;
const MOVEMENT_SPEED: f32 = 0.5; // units per second
const HIT_POINTS: i32 = 2;

// --------------------------------------------------------------------------------------------------------------------

pub struct FireSprite {
    entity_id: u32,
    spawn_point_id: u32,
    sprite: sprite::Sprite,
    position: Point3<f32>,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
    life: Option<HitPointState>,
    march: Option<MarchState>,
    launch_velocity: Option<Vector2<f32>>,
    launch_active: bool,
}

impl Default for FireSprite {
    fn default() -> Self {
        Self {
            entity_id: 0,
            spawn_point_id: 0,
            sprite: sprite::Sprite::default(),
            position: point3(0.0, 0.0, 0.0),
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
            life: Some(HitPointState::new(HIT_POINTS)),
            march: Some(MarchState::new(Direction::East, MOVEMENT_SPEED)),
            launch_velocity: None,
            launch_active: false,
        }
    }
}

impl FireSprite {
    pub fn launch(position: Point3<f32>, direction: Vector2<f32>, velocity: f32) -> Self {
        Self {
            entity_id: 0,
            spawn_point_id: 0,
            sprite: sprite::Sprite::default(),
            position,
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
            life: None,
            march: None,
            launch_velocity: Some(direction * velocity),
            launch_active: true,
        }
    }
}

impl Entity for FireSprite {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        _map: &map::Map,
        collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.spawn_point_id = sprite
            .entity_id
            .expect("Spawned entities expect to find a spawn point id from the sprite");

        self.position = sprite.origin;

        // Make copy of sprite for ourselves, we'll use it for collision testing
        // Note: The map sprite is our spawn point, so we need to overwrite the entity_id and mask
        self.sprite = *sprite;
        self.sprite.entity_id = Some(entity_id);
        self.sprite.mask =
            sprite_masks::SHOOTABLE | sprite_masks::COLLIDER | sprite_masks::CONTACT_DAMAGE;
        self.sprite.collision_shape = sprite::CollisionShape::Square;
        collision_space.add_dynamic_sprite(&self.sprite);
    }

    fn init(&mut self, entity_id: u32, _map: &map::Map, _collision_space: &mut collision::Space) {
        self.entity_id = entity_id;
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
        _game_state_peek: &GameStatePeek,
    ) {
        if let Some(launch_velocity) = self.launch_velocity {
            let dt = dt.as_secs_f32();
            self.position.x += launch_velocity.x * dt;
            self.position.y += launch_velocity.y * dt;
        } else {
            let entity_id = self.entity_id;
            let spawn_point_id = self.spawn_point_id;
            let position = self.position;

            let alive = if let Some(ref mut life) = self.life {
                life.update(
                    entity_id,
                    spawn_point_id,
                    position,
                    collision_space,
                    message_dispatcher,
                )
            } else {
                false
            };

            if alive {
                //
                // Perform basic march behavior
                // TODO: Why can't I map on the march optional and get a mutable ref? There must be a
                // simpler way to manage am optional, mutable field.
                //

                let next_position = if let Some(ref mut march) = self.march {
                    Some(march.update(dt, position.xy(), collision_space))
                } else {
                    None
                };

                if let Some(next_position) = next_position {
                    self.position.x = next_position.x;
                    self.position.y = next_position.y;
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

                self.animation_cycle_tick_countdown -= dt.as_secs_f32();
                if self.animation_cycle_tick_countdown <= 0.0 {
                    self.animation_cycle_tick_countdown += ANIMATION_CYCLE_DURATION;
                    self.animation_cycle_tick += 1;
                }
            }
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        let (xscale, xoffset) = if self.animation_cycle_tick / 2 % 2 == 0 {
            (1.0, 0.0)
        } else {
            (-1.0, 1.0)
        };
        uniforms
            .data
            .set_model_position(self.position + vec3(xoffset, 0.0, 0.0))
            .set_sprite_scale(vec2(xscale, 1.0));
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::FireSprite
    }

    fn is_alive(&self) -> bool {
        if let Some(ref life) = self.life {
            life.is_alive()
        } else {
            self.launch_active
        }
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "fire_sprite"
    }

    fn sprite_cycle(&self) -> &str {
        if self.animation_cycle_tick % 2 == 0 {
            "default"
        } else {
            "alt"
        }
    }

    fn handle_message(&mut self, message: &Message) {
        if let Some(ref mut life) = self.life {
            life.handle_message(message);
        }
    }

    fn did_exit_viewport(&mut self) {
        if let Some(ref mut life) = self.life {
            life.terminate();
        } else {
            self.launch_active = false;
        }
    }
}
