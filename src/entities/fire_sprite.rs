use cgmath::*;
use std::time::Duration;

use crate::{
    collision,
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, rendering},
    state::constants::{layers, sprite_masks},
    tileset, util,
};

use super::util::{HitPointState, HorizontalDir, MarchState};

// --------------------------------------------------------------------------------------------------------------------

const ANIMATION_CYCLE_DURATION: f32 = 0.133;
const MOVEMENT_SPEED: f32 = 0.5; // units per second
const HIT_POINTS: i32 = 2;

// --------------------------------------------------------------------------------------------------------------------

pub struct FireSprite {
    entity_id: u32,
    collider_id: Option<u32>,
    spawn_point_id: u32,
    position: Point3<f32>,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
    life: HitPointState,
    march: Option<MarchState>,
}

impl Default for FireSprite {
    fn default() -> Self {
        Self {
            entity_id: 0,
            collider_id: None,
            spawn_point_id: 0,
            position: point3(0.0, 0.0, 0.0),
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
            life: HitPointState::new(HIT_POINTS),
            march: None,
        }
    }
}

impl Entity for FireSprite {
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

        // Make collider
        self.collider_id = Some(
            collision_space.add_collider(collision::Collider::new_dynamic(
                sprite.bounds(),
                entity_id,
                collision::Shape::Square,
                sprite_masks::ENTITY | sprite_masks::SHOOTABLE | sprite_masks::CONTACT_DAMAGE,
            )),
        );

        let fixed_position = {
            if let Some(properties) = map.object_group_properties_for_sprite(sprite, "Metadata") {
                properties.property("fixed_position") == Some("true")
            } else {
                false
            }
        };

        if !fixed_position {
            self.march = Some(MarchState::new(HorizontalDir::East, MOVEMENT_SPEED));
        }
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
        _game_state_peek: &GameStatePeek,
    ) {
        let entity_id = self.entity_id;
        let spawn_point_id = self.spawn_point_id;
        let position = self.position;

        let alive = self.life.update(
            entity_id,
            spawn_point_id,
            position,
            collision_space,
            message_dispatcher,
        );

        if alive {
            //
            // Perform basic march behavior
            //

            if let Some(march) = &mut self.march {
                let next_position = march.update(dt, position.xy(), collision_space);
                self.position.x = next_position.x;
                self.position.y = next_position.y;
            }

            //
            //  Update the sprite for collision detection
            //

            if let Some(id) = self.collider_id {
                collision_space.update_collider_position(id, self.position.xy())
            }

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

    fn update_uniforms(&self, uniforms: &mut util::Uniforms<rendering::UniformData>) {
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

    fn deactivate_collider(&mut self, collision_space: &mut collision::Space) {
        if let Some(id) = self.collider_id {
            collision_space.deactivate_collider(id);
        }
        self.collider_id = None;
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::FireSprite
    }

    fn is_alive(&self) -> bool {
        self.life.is_alive()
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
        self.life.handle_message(message);
    }
}
