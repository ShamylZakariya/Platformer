use cgmath::{vec2, Point2, Vector2, Zero};
use std::{collections::HashSet, time::Duration};
use winit::event::*;

use crate::map::FLAG_MAP_TILE_IS_COLLIDER;
use crate::sprite;
use crate::sprite_collision::{ProbeDir, ProbeResult, CollisionSpace};

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

// ---------------------------------------------------------------------------------------------------------------------

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

    pub fn update(&mut self, dt: Duration, collision_space: &CollisionSpace) -> &CharacterState {
        let dt = dt.as_secs_f32();

        self.overlapping_sprites.clear();
        self.contacting_sprites.clear();

        let mut position = self.character_state.position;

        position = self.apply_gravity(&position, dt).0;

        let footing_center =
            self.find_character_footing(collision_space, position, Zero::zero(), true);
        position = footing_center.0;

        let footing_right = self.find_character_footing(
            collision_space,
            position,
            vec2(1.0, 0.0),
            footing_center.1.is_none(),
        );
        position = footing_right.0;

        let footing_left = self.find_character_footing(
            collision_space,
            position,
            vec2(-1.0, 0.0),
            footing_center.1.is_none() && footing_right.1.is_none(),
        );
        position = footing_left.0;

        position = self
            .apply_character_movement(collision_space, position, dt)
            .0;

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
        collision_space: &CollisionSpace,
        position: Point2<f32>,
        test_offset: Vector2<f32>,
        may_apply_correction: bool,
    ) -> (Point2<f32>, Option<sprite::SpriteDesc>) {
        let mut position = position;
        let mut tracking = None;

        // scan sprites beneath character
        let center = Point2::new(
            (position.x + test_offset.x).round() as i32,
            (position.y + test_offset.y).round() as i32,
        );

        let below_center = Point2::new(center.x, center.y - 1);
        let inset = 0.0 as f32;
        let contacts_are_collision = !may_apply_correction;

        if let Some(s) = collision_space.get_sprite_at(&below_center, FLAG_MAP_TILE_IS_COLLIDER) {
            match s.collision_shape {
                sprite::CollisionShape::Square => {
                    if s.unit_rect_intersection(&position, inset, contacts_are_collision) {
                        self.handle_collision_with(&s);
                        if may_apply_correction {
                            position.y = s.origin.y + s.extent.y
                        }
                    }
                }
                sprite::CollisionShape::NorthEast | sprite::CollisionShape::NorthWest => {
                    if let Some(intersection) = s.line_intersection(
                        &(position + vec2(0.5, 1.0)),
                        &(position + vec2(0.5, 0.0)),
                    ) {
                        self.handle_collision_with(&s);
                        if may_apply_correction {
                            position.y = intersection.y;
                        }
                    }
                }
                _ => (),
            }
            self.overlapping_sprites.insert(s);
            tracking = Some(s);
        }

        if let Some(s) = collision_space.get_sprite_at(&center, FLAG_MAP_TILE_IS_COLLIDER) {
            match s.collision_shape {
                sprite::CollisionShape::Square => {
                    if s.unit_rect_intersection(&position, inset, contacts_are_collision) {
                        self.handle_collision_with(&s);
                        if may_apply_correction {
                            position.y = s.origin.y + s.extent.y
                        }
                    }
                }
                sprite::CollisionShape::NorthEast | sprite::CollisionShape::NorthWest => {
                    if let Some(intersection) = s.line_intersection(
                        &(position + vec2(0.5, 1.0)),
                        &(position + vec2(0.5, 0.0)),
                    ) {
                        self.handle_collision_with(&s);
                        if may_apply_correction {
                            position.y = intersection.y;
                        }
                    }
                }
                _ => (),
            }
            self.overlapping_sprites.insert(s);
            tracking = Some(s);
        }

        (position, tracking)
    }

    fn apply_character_movement(
        &mut self,
        collision_space: &CollisionSpace,
        position: Point2<f32>,
        dt: f32,
    ) -> (Point2<f32>, Vector2<f32>) {
        let steps = 3;
        let mask = FLAG_MAP_TILE_IS_COLLIDER;
        let mut delta_x = dt
            * WALK_SPEED
            * input_accumulator(
                self.input_state.move_left_pressed,
                self.input_state.move_right_pressed,
            );

        let mut delta_y = dt * WALK_SPEED * input_accumulator(false, self.input_state.jump_pressed);
        if delta_x > 0.0 {
            match collision_space.probe(position, ProbeDir::Right, steps, mask) {
                ProbeResult::None => {}
                ProbeResult::OneHit { dist, sprite } => {
                    if dist < delta_x {
                        delta_x = dist;
                        self.handle_collision_with(&sprite);
                    }
                }
                ProbeResult::TwoHits {
                    dist,
                    sprite_0,
                    sprite_1,
                } => {
                    if dist < delta_x {
                        delta_x = dist;
                        self.handle_collision_with(&sprite_0);
                        self.handle_collision_with(&sprite_1);
                    }
                }
            }
        } else if delta_x < 0.0 {
            match collision_space.probe(position, ProbeDir::Left, steps, mask) {
                ProbeResult::None => {}
                ProbeResult::OneHit { dist, sprite } => {
                    if dist < -delta_x {
                        delta_x = -dist;
                        self.handle_collision_with(&sprite);
                    }
                }
                ProbeResult::TwoHits {
                    dist,
                    sprite_0,
                    sprite_1,
                } => {
                    if dist < -delta_x {
                        delta_x = -dist;
                        self.handle_collision_with(&sprite_0);
                        self.handle_collision_with(&sprite_1);
                    }
                }
            }
        }

        if delta_y > 0.0 {
            match collision_space.probe(position, ProbeDir::Up, steps, mask) {
                ProbeResult::None => {}
                ProbeResult::OneHit { dist, sprite } => {
                    if dist < delta_y {
                        delta_y = dist;
                        self.handle_collision_with(&sprite);
                    }
                }
                ProbeResult::TwoHits {
                    dist,
                    sprite_0,
                    sprite_1,
                } => {
                    if dist < delta_y {
                        delta_y = dist;
                        self.handle_collision_with(&sprite_0);
                        self.handle_collision_with(&sprite_1);
                    }
                }
            }
        }

        let delta_position = vec2(delta_x, delta_y);
        (position + delta_position, delta_position)
    }

    fn handle_collision_with(&mut self, sprite: &sprite::SpriteDesc) {
        self.contacting_sprites.insert(*sprite);
    }
}
