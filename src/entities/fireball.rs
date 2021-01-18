use std::time::Duration;

use cgmath::*;

use crate::{
    entity::{Dispatcher, Entity, Event, Message},
    map,
    sprite::{collision, rendering},
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
    pub fn new(position: Point3<f32>, direction: Direction, velocity: f32) -> Self {
        Self {
            entity_id: 0,
            position,
            direction,
            velocity,
            alive: true,
            map_origin: point2(0.0, 0.0),
            map_extent: vec2(0.0, 0.0),
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
        _map: &map::Map,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
    ) {
        let dt = dt.as_secs_f32();
        let mask = crate::constants::sprite_masks::SHOOTABLE;

        let next_position = match self.direction {
            Direction::East => point2(self.position.x + self.velocity * dt, self.position.y),
            Direction::West => point2(self.position.x - self.velocity * dt, self.position.y),
        };

        if let Some(sprite) = collision_space.test_point(next_position, mask) {
            if let Some(target_entity_id) = sprite.entity_id {
                message_dispatcher.enqueue(Message::entity_to_entity(
                    self.entity_id(),
                    target_entity_id,
                    Event::HitByFireball {
                        direction: match self.direction {
                            Direction::East => 1,
                            Direction::West => -1,
                        },
                    },
                ));
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
            .set_model_position(self.position - vec3(0.5, 0.5, 0.0));
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::Fireball
    }

    fn is_alive(&self) -> bool {
        self.alive
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn bounds(&self) -> (Point2<f32>, Vector2<f32>) {
        (self.position().xy() - vec2(0.5, 0.5), vec2(1.0, 1.0))
    }

    fn sprite_name(&self) -> &str {
        "fireball"
    }

    fn sprite_cycle(&self) -> &str {
        CYCLE_DEFAULT
    }

    fn did_exit_viewport(&mut self) {
        self.alive = false;
    }
}
