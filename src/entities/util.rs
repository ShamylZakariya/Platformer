use std::time::Duration;

use crate::{event_dispatch::*, state::constants::sprite_masks::COLLIDER};
use crate::{sprite::collision, state::events::Event};
use cgmath::*;
use collision::Space;

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

// ---------------------------------------------------------------------------------------------------------------------

/// Implements a lifecycle system with hitpoints and dispatching a death message to the spawn point and a
/// death message to the game state on death. This is for enemies in the game, not Firebrand.
pub struct HitPointState {
    hit_points: i32,
    alive: bool,
    death_animation_dir: Direction,
    terminated: bool,
}

impl HitPointState {
    pub fn new(hit_points: i32) -> Self {
        Self {
            hit_points,
            alive: true,
            death_animation_dir: Direction::East,
            terminated: false,
        }
    }

    pub fn hit_points(&self) -> i32 {
        self.hit_points
    }

    pub fn injure(&mut self, reduction: i32, direction: Direction) {
        self.hit_points -= reduction;
        self.death_animation_dir = direction;
    }

    /// kills entity without playing a death animation - useful for "resetting" an entity after it goes offscreen
    pub fn terminate(&mut self) {
        self.terminated = true;
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
        if self.terminated || self.hit_points <= 0 {
            self.alive = false;

            // remove self from collision space
            collision_space.remove_dynamic_sprite_with_entity_id(entity_id);

            // send death message to spawn point
            message_dispatcher.entity_to_entity(
                entity_id,
                spawn_point_id,
                Event::SpawnedEntityDidDie,
            );

            if self.hit_points <= 0 && !self.terminated {
                // send death animation message
                message_dispatcher.entity_to_global(
                    entity_id,
                    Event::PlayEntityDeathAnimation {
                        position: position.xy(),
                        direction: self.death_animation_dir,
                    },
                );
            }
        }

        self.alive
    }

    pub fn handle_message(&mut self, message: &Message) -> bool {
        if let Event::HitByFireball { direction } = message.event {
            println!("HitPointState[{:?}]::handle_message - HitByFireball", message.recipient_entity_id);
            self.hit_points = (self.hit_points - 1).max(0);
            self.death_animation_dir = direction;
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

/// Implements a very basic marching behavior for entities which need to march left/right reversing at cliffs
/// or obstacles.
pub struct MarchState {
    current_movement_dir: Direction,
    velocity: f32,
}

impl MarchState {
    pub fn new(initial_movement_dir: Direction, velocity: f32) -> Self {
        Self {
            current_movement_dir: initial_movement_dir,
            velocity,
        }
    }

    pub fn current_movement_dir(&self) -> Direction {
        self.current_movement_dir
    }

    /// Computes next position for the marching behavior, taking the current position and returning the
    /// updated position.
    pub fn update(
        &mut self,
        dt: Duration,
        position: Point2<f32>,
        collision_space: &Space,
    ) -> Point2<f32> {
        let dt = dt.as_secs_f32();

        let next_position = position
            + match self.current_movement_dir {
                Direction::East => vec2(1.0, 0.0),
                Direction::West => vec2(-1.0, 0.0),
            } * self.velocity
                * dt;

        let snapped_next_position = point2(
            next_position.x.floor() as i32,
            next_position.y.floor() as i32,
        );

        let snapped_next_position_center = point2(
            (next_position.x + 0.5).floor() as i32,
            next_position.y.floor() as i32,
        );

        let mut should_reverse_direction = false;

        match self.current_movement_dir {
            Direction::East => {
                // check for obstacle to right
                if let Some(sprite_to_right) = collision_space
                    .get_static_sprite_at(snapped_next_position + vec2(1, 0), COLLIDER)
                {
                    if sprite_to_right.rect_intersection(&next_position, &vec2(1.0, 1.0), 0.0, true)
                    {
                        should_reverse_direction = true
                    }
                }
                // check if the platform falls away to right
                if collision_space
                    .get_static_sprite_at(snapped_next_position_center + vec2(0, -1), COLLIDER)
                    .is_none()
                {
                    should_reverse_direction = true
                }
            }
            Direction::West => {
                // check for obstacle to left
                if let Some(sprite_to_left) =
                    collision_space.get_static_sprite_at(snapped_next_position, COLLIDER)
                {
                    if sprite_to_left.rect_intersection(&next_position, &vec2(1.0, 1.0), 0.0, true)
                    {
                        should_reverse_direction = true
                    }
                }
                // check if the platform falls away to left
                if collision_space
                    .get_static_sprite_at(snapped_next_position_center + vec2(0, -1), COLLIDER)
                    .is_none()
                {
                    should_reverse_direction = true
                }
            }
        }

        if should_reverse_direction {
            self.current_movement_dir = self.current_movement_dir.invert();
            position
        } else {
            next_position
        }
    }
}
