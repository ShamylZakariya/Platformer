use crate::event_dispatch::*;
use crate::{sprite::collision, state::events::Event};
use cgmath::*;

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    East,
    West,
}

impl Direction {
    pub fn invert(&self) -> Direction {
        match self {
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }
}

pub struct HitPointState {
    hit_points: i32,
    alive: bool,
    death_animation_dir: Direction,
}

impl HitPointState {
    pub fn new(hit_points: i32) -> Self {
        Self {
            hit_points,
            alive: true,
            death_animation_dir: Direction::East,
        }
    }

    pub fn hit_points(&self) -> i32 {
        self.hit_points
    }

    pub fn injure(&mut self, reduction: i32, direction: Direction) {
        self.hit_points -= reduction;
        self.death_animation_dir = direction;
    }

    pub fn is_alive(&self) -> bool {
        self.alive
    }

    pub fn update(
        &mut self,
        entity_id: u32,
        spawn_point_id: u32,
        position: Point3<f32>,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
    ) -> bool {
        if self.hit_points <= 0 {
            self.alive = false;

            // remove self from collision space
            collision_space.remove_dynamic_sprite_with_entity_id(entity_id);

            // send death message to spawn point
            message_dispatcher.enqueue(Message::entity_to_entity(
                entity_id,
                spawn_point_id,
                Event::SpawnedEntityDidDie,
            ));

            // send death animation message
            message_dispatcher.enqueue(Message::entity_to_global(
                entity_id,
                Event::PlayEntityDeathAnimation {
                    position: position.xy(),
                    direction: self.death_animation_dir,
                },
            ));
        }
        self.alive
    }

    pub fn handle_message(&mut self, message: &Message) -> bool {
        if let Event::HitByFireball { direction } = message.event {
            self.hit_points = (self.hit_points - 1).max(0);
            self.death_animation_dir = direction;
            true
        } else {
            false
        }
    }
}
