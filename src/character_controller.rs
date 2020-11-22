use cgmath::{prelude::*, relative_eq, vec2, vec3, Point2, Point3, Vector2, Vector3, Vector4};
use std::time::Duration;
use winit::event::*;

use crate::map::FLAG_MAP_TILE_IS_COLLIDER;
use crate::sprite;

// ---------------------------------------------------------------------------------------------------------------------

const CHARACTER_CYCLE_DEFAULT: &str = "default";
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
            cycle: CHARACTER_CYCLE_DEFAULT,
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
}

impl CharacterController {
    pub fn new(position: &Point2<f32>) -> Self {
        Self {
            input_state: Default::default(),
            character_state: CharacterState::new(position),
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

        let mut center_bottom = self.character_state.position + vec2(0.5, 0.0);

        // apply gravity
        {
            let (pos, floor) = self.apply_gravity(collision_space, &center_bottom, dt);
            center_bottom = pos;

            // check if character landed on something - his is where we'll handle landing on lava, etc
            if let Some(floor) = floor {
                println!("[CharacterController::update] - floor: {:#?}", floor);
            }
        }

        // apply user input
        {
            let delta_position = dt
                * WALK_SPEED
                * Vector2::new(
                    input_accumulator(
                        self.input_state.move_left_pressed,
                        self.input_state.move_right_pressed,
                    ),
                    input_accumulator(false, self.input_state.jump_pressed),
                );

            let (pos, obstruction) =
                self.apply_character_movement(collision_space, &center_bottom, &delta_position);

            center_bottom = pos;

            // determine if character walked into something, e.g. wall spikes etc
            if let Some(obstruction) = obstruction {
                println!(
                    "[CharacterController::update] - movement obstruction: {:#?}",
                    obstruction
                );
            }
        }

        self.character_state.position = center_bottom - vec2(0.5, 0.0);
        &self.character_state
    }

    /// Given unit-sized character and center/bottom position, applies gravity, and returns tuple of new center/bottom
    /// position and any floor sprite which prevented further application of gravity
    fn apply_gravity(
        &self,
        collision_space: &sprite::SpriteHitTester,
        center_bottom: &Point2<f32>,
        dt: f32,
    ) -> (Point2<f32>, Option<sprite::SpriteDesc>) {
        let new_position = center_bottom + vec2(0.0, dt * GRAVITY_SPEED);
        println!(
            "[CharacterController::apply_gravity] new_position: {:?}",
            center_bottom
        );
        if let Some(sprite) = collision_space.test_point(&new_position, FLAG_MAP_TILE_IS_COLLIDER) {
            println!("\tCollided with: {:#?}", sprite);

            // adjust the new position to the collision edge of the sprite
            match sprite.collision_shape {
                sprite::SpriteCollisionShape::Square => {
                    let new_position =
                        Point2::new(new_position.x, sprite.origin.y + sprite.extent.y);
                    (new_position, Some(sprite))
                }
                sprite::SpriteCollisionShape::NorthEast => {
                    // slope is -1, so y is how far across intersection is
                    let across = 1.0 - (new_position.x - sprite.origin.x);
                    let new_position = Point2::new(new_position.x, sprite.origin.y + across);
                    (new_position, Some(sprite))
                }
                sprite::SpriteCollisionShape::NorthWest => {
                    // slope is 1, so y is how far across intersection is
                    let across = new_position.x - sprite.origin.x;
                    let new_position = Point2::new(new_position.x, sprite.origin.y + across);
                    (new_position, Some(sprite))
                }
                _ => (new_position, None),
            }
        } else {
            println!("\tNo collision");
            (new_position, None)
        }
    }

    /// Given a unit-sized character, with position at center/bottom, and given a proposed movement, returns a
    /// tuple of the new character center/bottom position and any obstruction that prevented movement (if any)
    fn apply_character_movement(
        &self,
        collision_space: &sprite::SpriteHitTester,
        center_bottom: &Point2<f32>,
        proposed_movement: &Vector2<f32>,
    ) -> (Point2<f32>, Option<sprite::SpriteDesc>) {
        let proposed_position = center_bottom + proposed_movement;
        (proposed_position, None)
    }
}
