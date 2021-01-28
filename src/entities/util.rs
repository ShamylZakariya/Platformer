use std::{f32::consts::PI, time::Duration};

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

const SIN_PI_4: f32 = 0.707_106_77;
const TAU: f32 = 2.0 * PI;

#[derive(Debug, Copy, Clone)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompassDir {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl CompassDir {
    /// Creates the CompassDir which most closely matches the (not-necessarily-normalized) input dir.
    pub fn new(dir: Vector2<f32>) -> Self {
        use CompassDir::*;
        let ndir = dir.normalize();
        let mut angle = ndir.y.atan2(ndir.x);
        if angle < 0.0 {
            angle += TAU;
        }
        let sector = ((angle / (TAU / 16.0)).round() as i32) % 16;
        match sector {
            0 | 15 => East,
            1 | 2 => NorthEast,
            3 | 4 => North,
            5 | 6 => NorthWest,
            7 | 8 => West,
            9 | 10 => SouthWest,
            11 | 12 => South,
            13 | 14 => SouthEast,
            _ => panic!("sector expected to be in range [0,15]"),
        }
    }

    pub fn to_dir(&self) -> Vector2<f32> {
        use CompassDir::*;
        let t = SIN_PI_4;
        match self {
            North => vec2(0.0, 1.0),
            NorthEast => vec2(t, t),
            East => vec2(1.0, 0.0),
            SouthEast => vec2(t, -t),
            South => vec2(0.0, -1.0),
            SouthWest => vec2(-t, -t),
            West => vec2(-1.0, 0.0),
            NorthWest => vec2(-t, t),
        }
    }

    pub fn mirrored(&self, axis: Axis) -> Self {
        use CompassDir::*;
        match axis {
            Axis::Horizontal => match self {
                North => South,
                NorthEast => SouthEast,
                East => East,
                SouthEast => NorthEast,
                South => North,
                SouthWest => NorthWest,
                West => West,
                NorthWest => SouthWest,
            },
            Axis::Vertical => match self {
                North => North,
                NorthEast => NorthWest,
                East => West,
                SouthEast => SouthWest,
                South => South,
                SouthWest => SouthEast,
                West => East,
                NorthWest => NorthEast,
            },
        }
    }
}

#[cfg(test)]
mod chase_dir_tests {
    use super::*;

    #[test]
    fn new_produces_expected_values() {
        assert_eq!(CompassDir::new(vec2(0.0, 1.0)), CompassDir::North);
        assert_eq!(CompassDir::new(vec2(0.0, -1.0)), CompassDir::South);
        assert_eq!(CompassDir::new(vec2(1.0, 0.0)), CompassDir::East);
        assert_eq!(CompassDir::new(vec2(-1.0, 0.0)), CompassDir::West);

        assert_eq!(CompassDir::new(vec2(1.0, 1.0)), CompassDir::NorthEast);
        assert_eq!(CompassDir::new(vec2(1.0, -1.0)), CompassDir::SouthEast);
        assert_eq!(CompassDir::new(vec2(-1.0, -1.0)), CompassDir::SouthWest);
        assert_eq!(CompassDir::new(vec2(-1.0, 1.0)), CompassDir::NorthWest);
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
