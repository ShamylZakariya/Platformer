use cgmath::*;
use std::time::Duration;

use crate::{
    collision,
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, rendering},
    state::constants::{layers, sprite_masks},
    tileset,
};

use super::util::{HitPointState, HorizontalDir, MarchState};

// --------------------------------------------------------------------------------------------------------------------

const ANIMATION_CYCLE_DURATION: f32 = 0.133;
const MOVEMENT_SPEED: f32 = 1.0; // units per second
const HIT_POINTS: i32 = 1;

// --------------------------------------------------------------------------------------------------------------------

pub struct Hoodie {
    entity_id: u32,
    spawn_point_id: u32,
    collider: collision::Collider,
    sprite_size_px: Vector2<f32>,
    position: Point3<f32>,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
    life: HitPointState,
    march: MarchState,
}

impl Default for Hoodie {
    fn default() -> Self {
        Self {
            entity_id: 0,
            spawn_point_id: 0,
            collider: collision::Collider::default(),
            sprite_size_px: vec2(0.0, 0.0),
            position: point3(0.0, 0.0, 0.0),
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
            life: HitPointState::new(HIT_POINTS),
            march: MarchState::new(HorizontalDir::East, MOVEMENT_SPEED),
        }
    }
}

impl Entity for Hoodie {
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

        // Create a collider
        self.collider = sprite.into();
        self.collider.entity_id = Some(entity_id);
        self.collider.mask |= sprite_masks::SHOOTABLE | sprite_masks::CONTACT_DAMAGE;
        self.collider.shape = collision::Shape::Square;
        self.collider.bounds.extent.y = 1.5; // hoodie can be shot in the hat, too
        collision_space.add_dynamic_collider(&self.collider);
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
        _game_state_peek: &GameStatePeek,
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
            //
            // Perform basic march behavior
            //
            {
                let next_position = self.march.update(dt, self.position.xy(), collision_space);
                self.position.x = next_position.x;
                self.position.y = next_position.y;
            }

            //
            //  Update the sprite for collision detection
            //

            self.collider.bounds.origin.x = self.position.x;
            self.collider.bounds.origin.y = self.position.y;
            collision_space.update_dynamic_collider(&self.collider);

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
        let one_px = 1.0 / self.sprite_size_px.x;

        let (xscale, xoffset) = match self.march.current_movement_dir() {
            HorizontalDir::East => (1.0, 4.0 * one_px),
            HorizontalDir::West => (-1.0, 1.0 - 4.0 * one_px),
        };

        uniforms
            .data
            .set_model_position(self.position + vec3(xoffset, 0.0, 0.0))
            .set_sprite_scale(vec2(xscale, 1.0));
    }

    fn remove_collider(&self, collision_space: &mut collision::Space) {
        collision_space.remove_dynamic_collider(&self.collider);
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::Hoodie
    }

    fn is_alive(&self) -> bool {
        self.life.is_alive()
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "hoodie"
    }

    fn sprite_cycle(&self) -> &str {
        match self.animation_cycle_tick % 4 {
            0 => "walk_0",
            1 => "walk_1",
            2 => "walk_0",
            _ => "walk_2",
        }
    }

    fn handle_message(&mut self, message: &Message) {
        self.life.handle_message(message);
    }
}
