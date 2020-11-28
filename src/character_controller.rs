use cgmath::{vec2, Point2, Vector2};
use std::time::Duration;
use winit::event::*;

use crate::map::FLAG_MAP_TILE_IS_COLLIDER;
use crate::sprite;

// ---------------------------------------------------------------------------------------------------------------------

const CHARACTER_CYCLE_DEFAULT: &str = "default";
pub const CHARACTER_CYCLE_DEBUG: &str = "debug";
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

    // if set, this is the current floor supporting the character
    pub floor: Option<sprite::SpriteDesc>,

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
            floor: None,
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

        // let position = self.character_state.position;
        // let (position, gravity_motion) = self.apply_gravity(&position, dt);
        // let (position, user_motion) = self.apply_character_movement(&position, dt);
        // let motion = gravity_motion + user_motion;

        // let position = self.sanitize_character_position(collision_space, &position, &motion);

        let position = self.character_state.position;

        let (position, user_motion) = self.apply_character_movement(&position, dt);
        let position = self.sanitize_character_position(collision_space, &position, &user_motion);

        let (position, gravity_motion) = self.apply_gravity(&position, dt);
        let position =
            self.sanitize_character_position(collision_space, &position, &gravity_motion);

        self.character_state.position = position;
        &self.character_state
    }

    fn sanitize_character_position(
        &mut self,
        collision_space: &sprite::SpriteHitTester,
        position: &Point2<f32>,
        motion: &Vector2<f32>,
    ) -> Point2<f32> {
        let mut position = *position;
        let mut did_contact = false;

        {
            // scan tiles left, center and right of character
            let left = Point2::new(position.x.round() as i32 - 1, position.y.round() as i32);
            let center = Point2::new(position.x.round() as i32, position.y.round() as i32);
            let right = Point2::new(position.x.round() as i32 + 1, position.y.round() as i32);

            if let Some(center) = collision_space.get_sprite_at(&center, FLAG_MAP_TILE_IS_COLLIDER)
            {
                let r = self.sanitize_character_footing(&center, &position, &motion);
                position = r.0;
                did_contact = r.1;
            }
            if !did_contact && left != center {
                if let Some(left) = collision_space.get_sprite_at(&left, FLAG_MAP_TILE_IS_COLLIDER)
                {
                    let r = self.sanitize_character_footing(&left, &position, &motion);
                    position = r.0;
                    did_contact = r.1;
                }
            }
            if !did_contact && right != center {
                if let Some(right) =
                    collision_space.get_sprite_at(&right, FLAG_MAP_TILE_IS_COLLIDER)
                {
                    let r = self.sanitize_character_footing(&right, &position, &motion);
                    position = r.0;
                    did_contact = r.1;
                }
            }
        }

        if !did_contact {
            // scan tiles left, center and right, but one step below character

            let left = Point2::new(position.x.round() as i32 - 1, position.y.round() as i32 - 1);
            let center = Point2::new(position.x.round() as i32, position.y.round() as i32 - 1);
            let right = Point2::new(position.x.round() as i32 + 1, position.y.round() as i32 - 1);

            if let Some(center) = collision_space.get_sprite_at(&center, FLAG_MAP_TILE_IS_COLLIDER)
            {
                let r = self.sanitize_character_footing(&center, &position, &motion);
                position = r.0;
                did_contact = r.1;
            }
            if !did_contact && left != center {
                if let Some(left) = collision_space.get_sprite_at(&left, FLAG_MAP_TILE_IS_COLLIDER)
                {
                    let r = self.sanitize_character_footing(&left, &position, &motion);
                    position = r.0;
                    did_contact = r.1;
                }
            }
            if !did_contact && right != center {
                if let Some(right) =
                    collision_space.get_sprite_at(&right, FLAG_MAP_TILE_IS_COLLIDER)
                {
                    let r = self.sanitize_character_footing(&right, &position, &motion);
                    position = r.0;
                }
            }
        }

        self.overlapping_sprites.dedup();
        self.contacting_sprites.dedup();
        position
    }

    fn sanitize_character_footing(
        &mut self,
        sprite: &sprite::SpriteDesc,
        position: &Point2<f32>,
        motion: &Vector2<f32>,
    ) -> (Point2<f32>, bool) {
        let mut position = *position;
        let mut did_contact = false;

        self.overlapping_sprites.push(*sprite);
        match sprite.collision_shape {
            sprite::SpriteCollisionShape::Square => {
                if sprite.unit_rect_intersection(&position, 0.0) {
                    self.handle_collision_with(&sprite);

                    if motion.y < 0.0 {
                        position.y = sprite.top();
                    } else if motion.x > 0.0 {
                        position.x = sprite.left() - 1.0;
                    } else if motion.x < 0.0 {
                        position.x = sprite.right();
                    }

                    did_contact = true;
                }
            }
            sprite::SpriteCollisionShape::NorthEast | sprite::SpriteCollisionShape::NorthWest => {
                if let Some(intersection) = sprite
                    .line_intersection(&(position + vec2(0.5, 1.0)), &(position + vec2(0.5, 0.0)))
                {
                    self.handle_collision_with(&sprite);
                    position.y = intersection.y;
                    did_contact = true;
                }
            }
            _ => (),
        }

        (position, did_contact)
    }

    fn apply_gravity(&self, position: &Point2<f32>, dt: f32) -> (Point2<f32>, Vector2<f32>) {
        let motion = vec2(0.0, dt * GRAVITY_SPEED);
        (position + motion, motion)
    }

    fn apply_character_movement(
        &self,
        position: &Point2<f32>,
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

        (position + delta_position, delta_position)
    }

    fn handle_collision_with(&mut self, sprite: &sprite::SpriteDesc) {
        self.contacting_sprites.push(*sprite);
    }
}
