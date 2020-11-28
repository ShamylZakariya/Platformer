use cgmath::{vec2, Point2, Vector2};
use std::time::Duration;
use winit::event::*;

use crate::map::FLAG_MAP_TILE_IS_COLLIDER;
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

    // sprites the character is overlapping and might collide with
    pub overlapping_sprites: Vec<sprite::SpriteDesc>,

    // sprites the character is contacting
    pub contacting_sprites: Vec<sprite::SpriteDesc>,
}

impl CharacterController {
    pub fn new(position: &Point2<f32>) -> Self {
        Self {
            input_state: Default::default(),
            character_state: CharacterState::new(position),
            overlapping_sprites: vec![],
            contacting_sprites: vec![],
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
        self.contacting_sprites.clear();

        let center = self.character_state.position + vec2(0.5, 0.5);
        let (center, gravity_motion) = self.apply_gravity(&center, dt);
        let (center, user_motion) = self.apply_character_movement(&center, dt);
        let motion = gravity_motion + user_motion;

        let center = self.sanitize_character_position(collision_space, &center, &motion);

        self.character_state.position = center - vec2(0.5, 0.5);
        &self.character_state
    }

    fn sanitize_character_position(
        &mut self,
        collision_space: &sprite::SpriteHitTester,
        center: &Point2<f32>,
        _motion: &Vector2<f32>,
    ) -> Point2<f32> {
        let mut center = *center;

        {
            let sprite = collision_space.get_sprite_at(&center, FLAG_MAP_TILE_IS_COLLIDER);
            if let Some(sprite) = sprite {
                self.overlapping_sprites.push(sprite);
                if let Some(intersection) = sprite
                    .line_intersection(&(center + vec2(0.0, 0.5)), &(center + vec2(0.0, -0.5)))
                {
                    self.handle_collision_with(&sprite);
                    center.y = intersection.y + 0.5;
                }
            }

            // check sprite beneath
            let sprite = collision_space
                .get_sprite_at(&(center + vec2(0.0, -1.0)), FLAG_MAP_TILE_IS_COLLIDER);
            if let Some(sprite) = sprite {
                self.overlapping_sprites.push(sprite);
                if let Some(intersection) = sprite
                    .line_intersection(&(center + vec2(0.0, 0.5)), &(center + vec2(0.0, -0.5)))
                {
                    self.handle_collision_with(&sprite);
                    center.y = intersection.y + 0.5;
                }
            }
        }

        self.overlapping_sprites.dedup();
        self.contacting_sprites.dedup();
        center
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

    fn handle_collision_with(&mut self, sprite: &sprite::SpriteDesc) {
        self.contacting_sprites.push(*sprite);
    }
}
