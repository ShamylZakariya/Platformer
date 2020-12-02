use cgmath::{vec2, Point2, Vector2, Zero};
use std::{collections::HashSet, time::Duration};
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
    pub overlapping_sprites: HashSet<sprite::SpriteDesc>,

    // sprites the character is contacting
    pub contacting_sprites: HashSet<sprite::SpriteDesc>,
}

impl CharacterController {
    pub fn new(position: &Point2<f32>) -> Self {
        Self {
            input_state: Default::default(),
            character_state: CharacterState::new(position),
            floor: None,
            overlapping_sprites: HashSet::new(),
            contacting_sprites: HashSet::new(),
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

        let mut position = self.character_state.position;

        position = self.apply_gravity(&position, dt).0;

        let r = self.find_character_footing(collision_space, &position, &Zero::zero());
        position = r.0;
        let mut footing = r.1;

        if let Some(t) = footing {
            self.overlapping_sprites.insert(t);
        } else {
            let r = self.find_character_footing(collision_space, &position, &vec2(1.0, 0.0));
            position = r.0;
            footing = r.1;
            if let Some(t) = footing {
                self.overlapping_sprites.insert(t);
            } else {
                let r = self.find_character_footing(collision_space, &position, &vec2(-1.0, 0.0));
                position = r.0;
                footing = r.1;
                if let Some(t) = footing {
                    self.overlapping_sprites.insert(t);
                }
            }
        }

        let r = self.apply_character_movement(&position, dt);
        position = r.0;
        let motion = r.1;

        let r = self.find_character_collisions(collision_space, &position, &motion, &footing);
        position = r.0;
        footing = r.1;

        if let Some(t) = footing {
            self.overlapping_sprites.insert(t);
        }

        for s in &self.contacting_sprites {
            self.overlapping_sprites.remove(s);
        }

        self.character_state.position = position;
        &self.character_state
    }

    fn apply_gravity(&self, position: &Point2<f32>, dt: f32) -> (Point2<f32>, Vector2<f32>) {
        let motion = vec2(0.0, dt * GRAVITY_SPEED);
        (position + motion, motion)
    }

    // assumes character motion vector is (0.0, -N) -- e.g. falling. Finds the sprite which is best suitable for use in
    // collision detection to act as footing.
    fn find_character_footing(
        &mut self,
        collision_space: &sprite::SpriteHitTester,
        position: &Point2<f32>,
        test_offset: &Vector2<f32>,
    ) -> (Point2<f32>, Option<sprite::SpriteDesc>) {
        let mut position = *position;
        let mut tracking = None;

        // scan sprites beneath character
        let center = Point2::new(
            (position.x + test_offset.x).round() as i32,
            (position.y + test_offset.y).round() as i32,
        );

        let below_center = Point2::new(center.x, center.y - 1);

        if let Some(s) = collision_space.get_sprite_at(&below_center, FLAG_MAP_TILE_IS_COLLIDER) {
            match s.collision_shape {
                sprite::CollisionShape::Square => {
                    if s.unit_rect_intersection(&position, 0.0) {
                        self.handle_collision_with(&s);
                        position.y = s.origin.y + s.extent.y
                    }
                }
                sprite::CollisionShape::NorthEast | sprite::CollisionShape::NorthWest => {
                    if let Some(intersection) = s.line_intersection(
                        &(position + vec2(0.5, 1.0)),
                        &(position + vec2(0.5, 0.0)),
                    ) {
                        self.handle_collision_with(&s);
                        position.y = intersection.y;
                    }
                }
                _ => (),
            }
            tracking = Some(s);
        }

        if let Some(s) = collision_space.get_sprite_at(&center, FLAG_MAP_TILE_IS_COLLIDER) {
            match s.collision_shape {
                sprite::CollisionShape::Square => {
                    if s.unit_rect_intersection(&position, 0.0) {
                        self.handle_collision_with(&s);
                        position.y = s.origin.y + s.extent.y
                    }
                }
                sprite::CollisionShape::NorthEast | sprite::CollisionShape::NorthWest => {
                    if let Some(intersection) = s.line_intersection(
                        &(position + vec2(0.5, 1.0)),
                        &(position + vec2(0.5, 0.0)),
                    ) {
                        self.handle_collision_with(&s);
                        position.y = intersection.y;
                    }
                }
                _ => (),
            }
            tracking = Some(s);
        }

        (position, tracking)
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

    fn find_character_collisions(
        &mut self,
        collision_space: &sprite::SpriteHitTester,
        position: &Point2<f32>,
        motion: &Vector2<f32>,
        footing: &Option<sprite::SpriteDesc>,
    ) -> (Point2<f32>, Option<sprite::SpriteDesc>) {
        let mut position = *position;
        let mut obstruction = None;

        let is_on_slope = if let Some(s) = footing {
            match s.collision_shape {
                sprite::CollisionShape::None => false,
                sprite::CollisionShape::Square => false,
                // if the character is actually standing atop a slope, we need to disredard
                // sideward collisions. This allows character to not collide with the square shapes
                // beneath the slope tiles.
                _ => s.origin.y <= position.y,
            }
        } else {
            false
        };

        if motion.y > 0.0 {
            let above = Point2::new(position.x.round() as i32, position.y.round() as i32 + 1);
            let above_left = above + vec2(-1, 0);
            let above_right = above + vec2(1, 0);

            for test in [above_left, above, above_right].iter() {
                if let Some(s) = collision_space.get_sprite_at(test, FLAG_MAP_TILE_IS_COLLIDER) {
                    if s.collision_shape == sprite::CollisionShape::Square
                    && s.unit_rect_intersection(&position, 0.0)
                    {
                        self.overlapping_sprites.insert(s);
                        if s.origin.y > position.y {
                            self.handle_collision_with(&s);
                            position.y = s.origin.y - 1.0;
                            obstruction = Some(s);
                            break;
                        }
                    }
                }
            }
        }

        {
            // Perform left/right collision testing
            // scan to right or left, depending on motion vector
            let tests = if motion.x > 0.0 {
                let right =
                    Point2::new((position.x + 1.0).round() as i32, position.y.round() as i32);
                [right, right + vec2(0, -1), right + vec2(0, 1)]
            } else {
                let left =
                    Point2::new((position.x - 1.0).round() as i32, position.y.round() as i32);
                [left, left + vec2(0, -1), left + vec2(0, 1)]
            };

            if motion.x.abs() > 0.0 {
                for t in tests.iter() {
                    if let Some(s) = collision_space.get_sprite_at(t, FLAG_MAP_TILE_IS_COLLIDER) {
                        if s.collision_shape == sprite::CollisionShape::Square
                        && s.unit_rect_intersection(&position, 0.0)
                        {
                            self.overlapping_sprites.insert(s);
                            if motion.x > 0.0 {
                                if s.origin.x > position.x {
                                    self.handle_collision_with(&s);
                                    position = Point2::new(s.origin.x - 1.0, position.y);
                                    obstruction = Some(s);
                                }
                            } else {
                                if s.origin.x < position.x {
                                    self.handle_collision_with(&s);
                                    position = Point2::new(s.origin.x + s.extent.x, position.y);
                                    obstruction = Some(s);
                                }
                            }
                        }
                    }
                    // we only test the sprites directly in front or behind when on a slope
                    // which are the first in the tests array.
                    if is_on_slope {
                        break;
                    }
                }
            }
        }

        (position, obstruction)
    }

    fn handle_collision_with(&mut self, sprite: &sprite::SpriteDesc) {
        self.contacting_sprites.insert(*sprite);
    }
}
