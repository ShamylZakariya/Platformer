use std::{collections::HashSet, time::Duration};

use cgmath::{vec3, Point2, Point3, Vector2};
use winit::event::{ElementState, VirtualKeyCode};

use crate::{
    entity::{Dispatcher, Entity, Event, Message},
    map,
    sprite::{self, collision, rendering},
    tileset,
};

// ---------------------------------------------------------------------------------------------------------------------

const CYCLE_DEFAULT: &str = "default";

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    East,
    West,
}

pub struct Fireball {
    entity_id: u32,
    position: Point3<f32>,
    direction: Direction,
    velocity: f32,
    alive: bool,
    map_origin: Point2<f32>,
    map_extent: Vector2<f32>,
}

impl Fireball {
    pub fn new(position: cgmath::Point3<f32>, direction: Direction, velocity: f32) -> Self {
        Self {
            entity_id: 0,
            position,
            direction,
            velocity,
            alive: true,
            map_origin: Point2::new(0.0, 0.0),
            map_extent: Vector2::new(0.0, 0.0),
        }
    }
}

impl Entity for Fireball {
    fn init(&mut self, entity_id: u32, map: &map::Map, _collision_space: &mut collision::Space) {
        self.entity_id = entity_id;
        let bounds = map.bounds();
        self.map_origin = bounds.0.cast().unwrap();
        self.map_extent = bounds.1.cast().unwrap();
    }

    fn update(
        &mut self,
        dt: Duration,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
    ) {
        let dt = dt.as_secs_f32();
        let mask = crate::constants::sprite_masks::SHOOTABLE;

        let next_position = match self.direction {
            Direction::East => Point2::new(self.position.x + self.velocity * dt, self.position.y),
            Direction::West => Point2::new(self.position.x - self.velocity * dt, self.position.y),
        };

        if let Some(sprite) = collision_space.test_point(next_position, mask) {
            if let Some(entity_id) = sprite.entity_id {
                message_dispatcher.enqueue(Message::routed_to(entity_id, Event::HitByFireball));
            }
            self.alive = false;
        } else {
            self.position.x = next_position.x;
            self.position.y = next_position.y;
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        uniforms
            .data
            .set_model_position(&(self.position - vec3(0.5, 0.5, 0.0)));
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn is_alive(&self) -> bool {
        self.alive
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "fireball"
    }

    fn sprite_cycle(&self) -> &str {
        CYCLE_DEFAULT
    }
}
