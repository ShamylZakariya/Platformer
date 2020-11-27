use cgmath::{prelude::*, relative_eq, vec2, vec3, Point2, Point3, Vector2, Vector3, Vector4};
use std::time::Duration;
use winit::event::*;

use crate::map::{FLAG_MAP_TILE_IS_COLLIDER, FLAG_MAP_TILE_IS_RATCHET};
use crate::sprite;

// ---------------------------------------------------------------------------------------------------------------------

const CHARACTER_CYCLE_DEFAULT: &str = "default";
const CHARACTER_CYCLE_DEBUG: &str = "debug";
const CHARACTER_CYCLE_SHOOT: &str = "shoot";
const CHARACTER_CYCLE_WALK_0: &str = "walk_0";
const CHARACTER_CYCLE_WALK_1: &str = "walk_1";
const CHARACTER_CYCLE_WALK_2: &str = "walk_2";
const CHARACTER_CYCLE_FLY_0: &str = "fly_0";
const CHARACTER_CYCLE_FLY_1: &str = "fly_1";
const CHARACTER_CYCLE_WALL: &str = "wall";

// Gravity is applied as a constant downward speed
const GRAVITY_SPEED: f32 = -1.0;
const WALK_SPEED: f32 = 2.0;

#[derive(Debug)]
pub struct CharacterState {
    pub position: Point2<f32>,
    pub cycle: &'static str,
}

impl CharacterState {
    fn new(position: &Point2<f32>) -> Self {
        CharacterState {
            position: *position,
            cycle: CHARACTER_CYCLE_DEBUG,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

/// Response value for requests to move the character.
pub struct CharacterMovementRequestResult {
    // the new position of the character after applying the requested movement
    pub position: Point2<f32>,
    // if the character is standing on a tile, this is it
    pub floor: Option<sprite::SpriteDesc>,
    // if the character's requested movement was blocked (wall, etc) this is the blocker.
    // in the case of applying downward gravity force, floor and obstruction will be same thing.
    pub obstruction: Option<sprite::SpriteDesc>,
}

impl Default for CharacterMovementRequestResult {
    fn default() -> Self {
        CharacterMovementRequestResult {
            position: Point2::new(0.0, 0.0),
            floor: None,
            obstruction: None,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct InputState {
    move_left_pressed: bool,
    move_right_pressed: bool,
    jump_pressed: bool,
    fire_pressed: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            move_left_pressed: false,
            move_right_pressed: false,
            jump_pressed: false,
            fire_pressed: false,
        }
    }
}

fn input_accumulator(negative: bool, positive: bool) -> f32 {
    return if negative { -1.0 } else { 0.0 } + if positive { 1.0 } else { 0.0 };
}

#[derive(Debug)]
pub struct CharacterController {
    input_state: InputState,
    pub character_state: CharacterState,
    pub overlapping_sprites: Vec<sprite::SpriteDesc>,
    pub contact_sprites: Vec<sprite::SpriteDesc>,
}

impl CharacterController {
    pub fn new(position: &Point2<f32>) -> Self {
        Self {
            input_state: Default::default(),
            character_state: CharacterState::new(position),
            overlapping_sprites: vec![],
            contact_sprites: vec![],
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;
        match key {
            VirtualKeyCode::W => {
                self.input_state.jump_pressed = pressed;
                true
            }
            VirtualKeyCode::A => {
                self.input_state.move_left_pressed = pressed;
                true
            }
            VirtualKeyCode::D => {
                self.input_state.move_right_pressed = pressed;
                true
            }
            VirtualKeyCode::Space => {
                self.input_state.fire_pressed = pressed;
                true
            }
            _ => false,
        }
    }

    pub fn update(
        &mut self,
        dt: Duration,
        collision_space: &sprite::SpriteHitTester,
    ) -> &CharacterState {
        let dt = dt.as_secs_f32();

        self.overlapping_sprites.clear();
        self.contact_sprites.clear();

        let center_bottom = self.character_state.position + vec2(0.5, 0.0);
        let (center_bottom, gravity_motion) = self.apply_gravity(&center_bottom, dt);
        let (center_bottom, user_motion) = self.apply_character_movement(&center_bottom, dt);
        let motion = gravity_motion + user_motion;

        let center_bottom =
            self.sanitize_character_position(collision_space, &center_bottom, &motion);

        self.character_state.position = center_bottom - vec2(0.5, 0.0);
        &self.character_state
    }

    fn sanitize_character_position(
        &mut self,
        collision_space: &sprite::SpriteHitTester,
        center_bottom: &Point2<f32>,
        motion: &Vector2<f32>,
    ) -> Point2<f32> {
        let mut center_bottom = *center_bottom;

        // Get the four sprites
        let (top_left, top_right, bottom_right, bottom_left) = collision_space
            .get_overlapping_sprites(
                &Point2::new(center_bottom.x - 0.5, center_bottom.y),
                FLAG_MAP_TILE_IS_COLLIDER,
            );

        for s in vec![top_left, top_right, bottom_right, bottom_left] {
            if let Some(s) = s {
                self.overlapping_sprites.push(s);
            }
        }

        // Perform downward ray cast
        if let Some(bottom_left) = bottom_left {
            if let Some(intersection) =
                bottom_left.line_intersection(&(center_bottom + vec2(0.0, 1.0)), &center_bottom)
            {
                center_bottom = intersection;
            }
        }
        if let Some(bottom_right) = bottom_right {
            if let Some(intersection) =
                bottom_right.line_intersection(&(center_bottom + vec2(0.0, 1.0)), &center_bottom)
            {
                center_bottom = intersection;
            }
        }

        if motion.y < 0.0 {
        } else {
            // Perform upwards motion testing using a rectangle test
            if let Some(top_left) = top_left {
                if top_left.rect_intersection(&(center_bottom + vec2(-0.5, 0.0)), &vec2(1.0, 1.0)) {
                    center_bottom.y = top_left.origin.y - 1.0;
                }
            }
        }

        // // this almost but doesn't quite work
        // // do I need to do the four sprite check?
        // // I think that the center_bottom foot check is only valid for when character is on a slope,
        // // otherwise I should be using square/square collision?

        // {
        //     let foot_collisision = collision_space.test_point(&center_bottom, FLAG_MAP_TILE_IS_COLLIDER);
        //     let head_collision = collision_space.test_point(&center_top, FLAG_MAP_TILE_IS_COLLIDER);

        //     if let Some(foot_collision) = foot_collisision
        //     {
        //         if foot_collision.mask & FLAG_MAP_TILE_IS_RATCHET != 0 {
        //             // in the case of a ratchet, collision can only happen when foot is traveling downwards and
        //             // transitioned from above to below
        //             let previous_foot_collision = collision_space.test_point(&(center_bottom - motion), FLAG_MAP_TILE_IS_COLLIDER);
        //             if moving_downwards && head_collision != Some(foot_collision) {
        //                 return center_bottom;
        //             }
        //         }

        //         //println!("[{:?}] center_bottom collision", motion);
        //         center_bottom = foot_collision.line_intersection(&center_top, &center_bottom).unwrap();
        //         center_top.y = center_bottom.y + 1.0;
        //     }

        //     // test if character's head position in collider
        //     if let Some(head_collision) = head_collision
        //     {
        //         if head_collision.mask & FLAG_MAP_TILE_IS_RATCHET != 0 {
        //             //println!("[{:?}] center_top RATCHET collision", motion);
        //             return center_bottom;
        //         } else {
        //             //println!("[{:?}] center_top collision", motion);
        //             center_top = head_collision.line_intersection(&center_bottom, &center_top).unwrap();
        //             center_bottom.y = center_top.y - 1.0;
        //         }
        //     }
        // }

        center_bottom
    }

    fn apply_gravity(&self, center_bottom: &Point2<f32>, dt: f32) -> (Point2<f32>, Vector2<f32>) {
        let motion = vec2(0.0, dt * GRAVITY_SPEED);
        (center_bottom + motion, motion)
    }

    fn apply_character_movement(
        &self,
        center_bottom: &Point2<f32>,
        dt: f32,
    ) -> (Point2<f32>, Vector2<f32>) {
        let delta_position = dt
            * WALK_SPEED
            * Vector2::new(
                input_accumulator(
                    self.input_state.move_left_pressed,
                    self.input_state.move_right_pressed,
                ),
                input_accumulator(false, self.input_state.jump_pressed),
            );

        (center_bottom + delta_position, delta_position)
    }

    fn handle_collision_with(&mut self, sprite: &sprite::SpriteDesc) {}
}
