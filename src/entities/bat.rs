use cgmath::*;
use std::time::Duration;

use crate::{
    collision,
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, rendering},
    state::constants::{layers, sprite_masks, ORIGINAL_VIEWPORT_TILES_WIDE},
    tileset,
};

use super::util::{CompassDir, HitPointState};

// --------------------------------------------------------------------------------------------------------------------

const ANIMATION_CYCLE_DURATION: f32 = 0.133;
const MOVEMENT_SPEED: f32 = 1.0 / 0.3; // units per second
const HIT_POINTS: i32 = 1;
const PLAYER_CLOSENESS_THRESHOLD: f32 = (ORIGINAL_VIEWPORT_TILES_WIDE as f32 / 2.0) - 1.0;

// --------------------------------------------------------------------------------------------------------------------

pub struct Bat {
    entity_id: u32,
    collider_id: Option<u32>,
    spawn_point_id: u32,
    sprite_size_px: Vector2<f32>,
    position: Point3<f32>,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
    chase_dir: Option<CompassDir>,
    life: HitPointState,
}

impl Default for Bat {
    fn default() -> Self {
        Self {
            entity_id: 0,
            collider_id: None,
            spawn_point_id: 0,
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

        self.position = point3(sprite.origin.x, sprite.origin.y, layers::stage::ENTITIES);
        self.sprite_size_px = map.tileset.get_sprite_size().cast().unwrap();

        // Create collider
        let collider = collision::Collider::new_dynamic(
            sprite.bounds(),
            entity_id,
            collision::Shape::Square,
            sprite_masks::ENTITY | sprite_masks::SHOOTABLE | sprite_masks::CONTACT_DAMAGE,
        );
        self.collider_id = Some(collision_space.add_collider(collider));
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
                self.chase_dir = Some(CompassDir::new(
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

            if let Some(id) = self.collider_id {
                collision_space.update_collider_position(id, self.position.xy());
            }

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

    fn deactivate_collider(&mut self, collision_space: &mut collision::Space) {
        if let Some(id) = self.collider_id {
            collision_space.deactivate_collider(id);
        }
        self.collider_id = None
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
