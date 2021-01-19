use cgmath::*;
use core::time;
use std::{f32::consts::PI, time::Duration};
use winit::event::{ElementState, VirtualKeyCode};

use crate::{
    entity::Entity,
    event_dispatch::*,
    map,
    sprite::{self, collision, rendering},
    state::{
        constants::sprite_masks::{self, COLLIDER},
        events::Event,
    },
    tileset,
};

// --------------------------------------------------------------------------------------------------------------------

const PARABOLA_HALF_HEIGHT: f32 = 2.0;
const PARABOLA_HALF_WIDTH: f32 = 1.0;
const PARABOLA_MOTION_DURATION: f32 = 1.1;
const HIT_POINTS: i32 = 1;

// --------------------------------------------------------------------------------------------------------------------

pub struct FlyingFish {
    entity_id: u32,
    spawn_point_id: u32,
    sprite: sprite::Sprite,
    centroid: Point2<f32>,
    position: Point3<f32>,
    alive: bool,
    hit_points: i32,
    death_animation_dir: i32,
    time_in_phase: f32,
    phase: i32,
}

impl Default for FlyingFish {
    fn default() -> Self {
        Self {
            entity_id: 0,
            spawn_point_id: 0,
            sprite: sprite::Sprite::default(),
            centroid: point2(0.0, 0.0),
            position: point3(0.0, 0.0, 0.0),
            alive: true,
            hit_points: HIT_POINTS,
            death_animation_dir: 0,
            time_in_phase: 0.0,
            phase: 0,
        }
    }
}

impl Entity for FlyingFish {
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
        self.centroid = sprite.origin.xy();

        // Make copy of sprite for ourselves, we'll use it for collision testing
        // Note: The map sprite is our spawn point, so we need to overwrite the entity_id and mask
        self.sprite = *sprite;
        self.sprite.entity_id = Some(entity_id);
        self.sprite.mask = sprite_masks::SHOOTABLE | sprite_masks::COLLIDER;
        self.sprite.collision_shape = sprite::CollisionShape::Square;
        collision_space.add_dynamic_sprite(&self.sprite);
    }

    fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        if self.alive {
            if key == VirtualKeyCode::Delete && state == ElementState::Pressed {
                println!("BOOM");
                self.hit_points = 0;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
    ) {
        let dt = dt.as_secs_f32();

        if self.hit_points == 0 {
            self.alive = false;

            // remove self from collision space
            collision_space.remove_dynamic_sprite_with_entity_id(self.entity_id());

            // send death message to spawn point
            message_dispatcher.enqueue(Message::entity_to_entity(
                self.entity_id(),
                self.spawn_point_id,
                Event::SpawnedEntityDidDie,
            ));

            // send death animation message
            message_dispatcher.enqueue(Message::entity_to_global(
                self.entity_id(),
                Event::PlayEntityDeathAnimation {
                    position: self.position.xy(),
                    direction: self.death_animation_dir,
                },
            ));

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
        let y = phase_circular.sin() * PARABOLA_HALF_HEIGHT;
        self.position.x = self.centroid.x + x;
        self.position.y = self.centroid.y + y;

        self.time_in_phase += dt;
        if self.time_in_phase > PARABOLA_MOTION_DURATION {
            self.time_in_phase -= PARABOLA_MOTION_DURATION;
            self.phase += 1;
        }

        //
        //  Update the sprite for collision detection
        //

        self.sprite.origin.x = self.position.x;
        self.sprite.origin.y = self.position.y;
        collision_space.update_dynamic_sprite(&self.sprite);
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        let (xscale, xoffset) = match self.phase % 2 {
            0 => (-1.0, 1.0),
            _ => (1.0, 0.0),
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
        crate::entities::EntityClass::FlyingFish
    }

    fn is_alive(&self) -> bool {
        self.alive
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
        match message.event {
            Event::HitByFireball { direction } => {
                self.hit_points = (self.hit_points - 1).max(0);
                self.death_animation_dir = direction;
            }
            _ => {}
        }
    }
}
