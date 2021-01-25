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

use super::util::{Direction, HitPointState};

// --------------------------------------------------------------------------------------------------------------------

const ANIMATION_CYCLE_DURATION: f32 = 0.133;
const MOVEMENT_SPEED: f32 = 0.5; // units per second
const HIT_POINTS: i32 = 5;

// --------------------------------------------------------------------------------------------------------------------

pub struct BossFish {
    entity_id: u32,
    spawn_point_id: u32,
    sprite: sprite::Sprite,
    position: Point3<f32>,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
    life: HitPointState,
    facing: Direction,
}

impl Default for BossFish {
    fn default() -> Self {
        Self {
            entity_id: 0,
            spawn_point_id: 0,
            sprite: sprite::Sprite::default(),
            position: point3(0.0, 0.0, 0.0),
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
            life: HitPointState::new(HIT_POINTS),
            facing: Direction::West,
        }
    }
}

impl Entity for BossFish {
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

        self.position = sprite.origin + vec3(0.0, -3.0, 0.0);

        // Make copy of sprite for ourselves, we'll use it for collision testing
        // Note: The map sprite is our spawn point, so we need to overwrite the entity_id and mask
        self.sprite = *sprite;
        self.sprite.entity_id = Some(entity_id);
        self.sprite.mask =
            sprite_masks::SHOOTABLE | sprite_masks::COLLIDER | sprite_masks::CONTACT_DAMAGE;
        self.sprite.collision_shape = sprite::CollisionShape::Square;
        self.update_sprite_extents();
        collision_space.add_dynamic_sprite(&self.sprite);

        println!("BossFish::init_from_map_sprite position: {:?} sprite: {:?}", self.position(), &self.sprite);
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
        game_state_peek: &GameStatePeek,
        _drawable: &rendering::EntityDrawable,
    ) {
        self.facing = if game_state_peek.player_position.x - self.position.x > 0.0 {
            Direction::East
        } else {
            Direction::West
        };

        if self.life.update(
            self.entity_id(),
            self.spawn_point_id,
            self.position(),
            collision_space,
            message_dispatcher,
        ) {
            //
            //  Update the sprite for collision detection
            //

            self.update_sprite_extents();
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

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        let (xscale, xoffset) = match self.facing {
            Direction::East => (1.0, 0.0),
            Direction::West => (-1.0, 1.0),
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
        crate::entities::EntityClass::BossFish
    }

    fn is_alive(&self) -> bool {
        self.life.is_alive()
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "boss_fish"
    }

    fn sprite_cycle(&self) -> &str {
        // TODO: Handle shooting animation cycle
        if self.animation_cycle_tick % 2 == 0 {
            "a_0"
        } else {
            "b_0"
        }
    }

    fn handle_message(&mut self, message: &Message) {
        self.life.handle_message(message);
    }
}

impl BossFish {
    fn update_sprite_extents(&mut self) {
        // sprite is 3x2 with root centered at bottom
        self.sprite.origin.x = self.position.x - 1.0;
        self.sprite.origin.y = self.position.y;
        self.sprite.extent.x = 3.0;
        self.sprite.extent.y = 2.0;
    }
}
