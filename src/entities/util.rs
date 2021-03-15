use std::{f32::consts::PI, time::Duration};

use crate::{audio, collision, entity::GameStatePeek, state::events::Event};
use crate::{
    event_dispatch::*,
    state::constants::sprite_masks::{CONTACT_DAMAGE, GROUND},
};
use cgmath::*;
use collision::Space;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalDir {
    East,
    West,
}

impl HorizontalDir {
    pub fn invert(&self) -> HorizontalDir {
        match self {
            HorizontalDir::East => HorizontalDir::West,
            HorizontalDir::West => HorizontalDir::East,
        }
    }
}

impl From<Vector2<f32>> for HorizontalDir {
    fn from(v: Vector2<f32>) -> Self {
        if v.x > 0.0 {
            HorizontalDir::East
        } else {
            HorizontalDir::West
        }
    }
}

impl Into<Vector2<f32>> for HorizontalDir {
    fn into(self) -> Vector2<f32> {
        match self {
            HorizontalDir::East => vec2(1.0, 0.0),
            HorizontalDir::West => vec2(-1.0, 0.0),
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

    pub fn is_diagonal(&self) -> bool {
        use CompassDir::*;
        !matches!(self, North | South | East | West)
    }

    pub fn iter() -> impl Iterator<Item = CompassDir> {
        use CompassDir::*;
        [
            North, NorthEast, East, SouthEast, South, SouthWest, West, NorthWest,
        ]
        .iter()
        .copied()
    }
}

impl From<Vector2<f32>> for CompassDir {
    fn from(v: Vector2<f32>) -> Self {
        CompassDir::new(v)
    }
}

impl From<HorizontalDir> for CompassDir {
    fn from(d: HorizontalDir) -> Self {
        match d {
            HorizontalDir::East => CompassDir::East,
            HorizontalDir::West => CompassDir::West,
        }
    }
}

impl Into<Vector2<f32>> for CompassDir {
    fn into(self) -> Vector2<f32> {
        self.to_dir()
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
    death_animation_dir: HorizontalDir,
    terminated: bool,
}

impl HitPointState {
    pub fn new(hit_points: i32) -> Self {
        Self {
            hit_points,
            alive: true,
            death_animation_dir: HorizontalDir::East,
            terminated: false,
        }
    }

    pub fn hit_points(&self) -> i32 {
        self.hit_points
    }

    pub fn injure(&mut self, reduction: i32, direction: HorizontalDir) {
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
        audio: &mut audio::Audio,
        message_dispatcher: &mut Dispatcher,
        game_state_peek: &GameStatePeek,
    ) -> bool {
        if self.terminated || self.hit_points <= 0 {
            self.alive = false;

            // send death message to spawn point
            message_dispatcher.entity_to_entity(
                entity_id,
                spawn_point_id,
                Event::SpawnedEntityDidDie,
            );

            if self.hit_points <= 0 && !self.terminated {
                // play death sound and kick off death animation

                let channel = if position.x < game_state_peek.camera_position.x {
                    audio::Channel::Left
                } else {
                    audio::Channel::Right
                };
                audio.play_stereo_sound(audio::Sounds::EnemyDeath, channel);
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
        if let Event::HitByFireball { direction, damage } = message.event {
            self.hit_points = (self.hit_points - (damage as i32)).max(0);
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
    current_movement_dir: HorizontalDir,
    velocity: f32,
}

impl MarchState {
    pub fn new(initial_movement_dir: HorizontalDir, velocity: f32) -> Self {
        Self {
            current_movement_dir: initial_movement_dir,
            velocity,
        }
    }

    pub fn current_movement_dir(&self) -> HorizontalDir {
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
                HorizontalDir::East => vec2(1.0, 0.0),
                HorizontalDir::West => vec2(-1.0, 0.0),
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
            HorizontalDir::East => {
                // check for obstacle to right
                if let Some(sprite_to_right) =
                    collision_space.get_collider_at(snapped_next_position + vec2(1, 0), GROUND)
                {
                    if sprite_to_right.intersects_rect(&next_position, &vec2(1.0, 1.0), 0.0, true) {
                        should_reverse_direction = true
                    }
                }
                // check if the platform falls away to right
                let to_right = collision_space
                    .get_collider_at(snapped_next_position_center + vec2(0, -1), GROUND);
                if let Some(to_right) = to_right {
                    if to_right.shape != collision::Shape::Square
                        || to_right.mask & CONTACT_DAMAGE != 0
                    {
                        should_reverse_direction = true;
                    }
                } else {
                    should_reverse_direction = true;
                }
            }
            HorizontalDir::West => {
                // check for obstacle to left
                if let Some(sprite_to_left) =
                    collision_space.get_collider_at(snapped_next_position, GROUND)
                {
                    if sprite_to_left.intersects_rect(&next_position, &vec2(1.0, 1.0), 0.0, true) {
                        should_reverse_direction = true
                    }
                }
                // check if the platform falls away to left
                let to_left = collision_space
                    .get_collider_at(snapped_next_position_center + vec2(0, -1), GROUND);
                if let Some(to_left) = to_left {
                    if to_left.shape != collision::Shape::Square
                        || to_left.mask & CONTACT_DAMAGE != 0
                    {
                        should_reverse_direction = true;
                    }
                } else {
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
