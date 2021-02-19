use cgmath::*;
use rand::{prelude::*, Rng};
use std::{f32::consts::PI, time::Duration};

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, collision, rendering},
    state::constants::sprite_masks,
    tileset,
};

use super::util::HitPointState;

// --------------------------------------------------------------------------------------------------------------------

const PARABOLA_HALF_HEIGHT_SHORT: f32 = 2.0;
const PARABOLA_HALF_HEIGHT_TALL: f32 = 3.0;
const PARABOLA_HALF_WIDTH: f32 = 1.0;
const PARABOLA_MOTION_DURATION: f32 = 1.1;
const HIT_POINTS: i32 = 1;

// --------------------------------------------------------------------------------------------------------------------

pub struct FlyingFish {
    entity_id: u32,
    spawn_point_id: u32,
    collider: sprite::Sprite,
    centroid: Point2<f32>,
    position: Point3<f32>,
    time_in_phase: f32,
    phase: i32,
    sprite_size_px: Vector2<f32>,
    jump_phase: i32,
    jump_height: f32,
    rng: ThreadRng,
    life: HitPointState,
}

impl Default for FlyingFish {
    fn default() -> Self {
        Self {
            entity_id: 0,
            spawn_point_id: 0,
            collider: sprite::Sprite::default(),
            centroid: point2(0.0, 0.0),
            position: point3(0.0, 0.0, 0.0),
            time_in_phase: 0.0,
            phase: 0,
            sprite_size_px: vec2(0.0, 0.0),
            jump_phase: 0,
            jump_height: PARABOLA_HALF_HEIGHT_SHORT,
            rng: thread_rng(),
            life: HitPointState::new(HIT_POINTS),
        }
    }
}

impl Entity for FlyingFish {
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
        self.centroid = sprite.origin.xy();
        self.sprite_size_px = map.tileset.get_sprite_size().cast().unwrap();

        // offset phase such that neighbor fish don't jump in same dir
        self.phase = self.position.x as i32 % 2;

        // Make copy of sprite for ourselves, we'll use it for collision testing
        // Note: The map sprite is our spawn point, so we need to overwrite the entity_id and mask
        self.collider = *sprite;
        self.collider.entity_id = Some(entity_id);
        self.collider.mask = sprite_masks::SHOOTABLE | sprite_masks::CONTACT_DAMAGE;
        self.collider.collision_shape = sprite::CollisionShape::Square;
        collision_space.add_dynamic_sprite(&self.collider);
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
        _game_state_peek: &GameStatePeek,
    ) {
        let dt = dt.as_secs_f32();

        if !self.life.update(
            self.entity_id(),
            self.spawn_point_id,
            self.position(),
            collision_space,
            message_dispatcher,
        ) {
            return;
        }

        //
        //  Bounce left then right
        //

        let phase_elapsed_normalized = self.time_in_phase / PARABOLA_MOTION_DURATION;
        let phase_circular = if self.phase % 2 == 0 {
            // left to right
            (1.0 - phase_elapsed_normalized) * PI
        } else {
            // right to left
            phase_elapsed_normalized * PI
        };

        let x = phase_circular.cos() * PARABOLA_HALF_WIDTH;
        let y = phase_circular.sin() * self.jump_height;
        self.position.x = self.centroid.x + x;
        self.position.y = self.centroid.y + y;

        self.time_in_phase += dt;
        if self.time_in_phase > PARABOLA_MOTION_DURATION {
            self.time_in_phase -= PARABOLA_MOTION_DURATION;
            self.phase += 1;

            if self.rng.gen::<f32>() < 0.25 {
                self.jump_phase += 1;
                self.jump_height = match self.jump_phase % 2 {
                    0 => PARABOLA_HALF_HEIGHT_SHORT,
                    _ => PARABOLA_HALF_HEIGHT_TALL,
                };
            }
        }

        //
        //  Update the sprite for collision detection
        //

        self.collider.origin.x = self.position.x;
        self.collider.origin.y = self.position.y;
        collision_space.update_dynamic_sprite(&self.collider);
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        let (xscale, xoffset) = match self.phase % 2 {
            0 => (-1.0, 1.0 - 1.0 / self.sprite_size_px.x),
            _ => (1.0, 0.0),
        };
        uniforms
            .data
            .set_model_position(self.position + vec3(xoffset, 0.0, 0.0))
            .set_sprite_scale(vec2(xscale, 1.0));
    }

    fn remove_collider(&self, collision_space: &mut collision::Space) {
        collision_space.remove_dynamic_sprite(&self.collider);
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::FlyingFish
    }

    fn is_alive(&self) -> bool {
        self.life.is_alive()
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "flying_fish"
    }

    fn sprite_cycle(&self) -> &str {
        "default"
    }

    fn handle_message(&mut self, message: &Message) {
        self.life.handle_message(message);
    }
}
