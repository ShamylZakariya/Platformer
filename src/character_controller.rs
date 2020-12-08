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

#[derive(Clone, Copy, Debug)]
enum ProbeDir {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Clone, Copy, Debug)]
enum ProbeResult {
    None,
    OneHit {
        dist: f32,
        sprite: sprite::SpriteDesc,
    },
    TwoHits {
        dist: f32,
        sprite_0: sprite::SpriteDesc,
        sprite_1: sprite::SpriteDesc,
    },
}

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

        let r = self.find_character_footing(collision_space, position, Zero::zero());
        position = r.0;
        let mut footing = r.1;

        if let Some(t) = footing {
            self.overlapping_sprites.insert(t);
        } else {
            let r = self.find_character_footing(collision_space, position, vec2(1.0, 0.0));
            position = r.0;
            footing = r.1;
            if let Some(t) = footing {
                self.overlapping_sprites.insert(t);
            } else {
                let r = self.find_character_footing(collision_space, position, vec2(-1.0, 0.0));
                position = r.0;
                footing = r.1;
                if let Some(t) = footing {
                    self.overlapping_sprites.insert(t);
                }
            }
        }

        if let Some(t) = footing {
            self.overlapping_sprites.insert(t);
        }

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
        collision_space: &sprite::SpriteHitTester,
        position: Point2<f32>,
        test_offset: Vector2<f32>,
    ) -> (Point2<f32>, Option<sprite::SpriteDesc>) {
        let mut position = position;
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
        &mut self,
        collision_space: &sprite::SpriteHitTester,
        position: Point2<f32>,
        dt: f32,
    ) -> (Point2<f32>, Vector2<f32>) {
        let steps = 4;
        let mask = FLAG_MAP_TILE_IS_COLLIDER;
        let mut delta_x = dt
            * WALK_SPEED
            * input_accumulator(
                self.input_state.move_left_pressed,
                self.input_state.move_right_pressed,
            );

        let mut delta_y = dt * WALK_SPEED * input_accumulator(false, self.input_state.jump_pressed);
        if delta_x > 0.0 {
            match self.probe(collision_space, position, ProbeDir::Right, steps, mask) {
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
            match self.probe(collision_space, position, ProbeDir::Left, steps, mask) {
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
            match self.probe(collision_space, position, ProbeDir::Up, steps, mask) {
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

    fn probe(
        &mut self,
        collision_space: &sprite::SpriteHitTester,
        position: Point2<f32>,
        dir: ProbeDir,
        max_steps: i32,
        mask: u32,
    ) -> ProbeResult {
        let (offset, should_probe_offset) = match dir {
            ProbeDir::Up | ProbeDir::Down => (vec2(1.0, 0.0), position.x.fract().abs() > 0.0),
            ProbeDir::Right | ProbeDir::Left => (vec2(0.0, 1.0), position.y.fract().abs() > 0.0),
        };

        let mut dist = None;
        let mut sprite_0 = None;
        let mut sprite_1 = None;
        if let Some(r) = self._probe(collision_space, position, dir, max_steps, mask) {
            dist = Some(r.0);
            sprite_0 = Some(r.1);
        }

        if should_probe_offset {
            if let Some(r) = self._probe(collision_space, position + offset, dir, max_steps, mask) {
                dist = Some(r.0);
                sprite_1 = Some(r.1);
            }
        }

        match (sprite_0, sprite_1) {
            (None, None) => ProbeResult::None,
            (None, Some(s)) => ProbeResult::OneHit {
                dist: dist.unwrap(),
                sprite: s,
            },
            (Some(s), None) => ProbeResult::OneHit {
                dist: dist.unwrap(),
                sprite: s,
            },
            (Some(s0), Some(s1)) => ProbeResult::TwoHits {
                dist: dist.unwrap(),
                sprite_0: s0,
                sprite_1: s1,
            },
        }
    }

    fn _probe(
        &self,
        collision_space: &sprite::SpriteHitTester,
        position: Point2<f32>,
        dir: ProbeDir,
        max_steps: i32,
        mask: u32,
    ) -> Option<(f32, sprite::SpriteDesc)> {
        let position_snapped = Point2::new(position.x.floor() as i32, position.y.floor() as i32);
        let mut result = None;
        match dir {
            ProbeDir::Right => {
                for i in 0..max_steps {
                    let x = position_snapped.x + i;
                    if let Some(s) =
                        collision_space.get_sprite_at(&Point2::new(x, position_snapped.y), mask)
                    {
                        result = Some((s.origin.x - (position.x + 1.0), s));
                        break;
                    }
                }
            }
            ProbeDir::Up => {
                for i in 0..max_steps {
                    let y = position_snapped.y + i;
                    if let Some(s) =
                        collision_space.get_sprite_at(&Point2::new(position_snapped.x, y), mask)
                    {
                        result = Some((s.origin.y - (position.y + 1.0), s));
                        break;
                    }
                }
            }
            ProbeDir::Down => {
                for i in 0..max_steps {
                    let y = position_snapped.y - i;
                    if let Some(s) =
                        collision_space.get_sprite_at(&Point2::new(position_snapped.x, y), mask)
                    {
                        result = Some((position.y - s.top(), s));
                        break;
                    }
                }
            }
            ProbeDir::Left => {
                for i in 0..max_steps {
                    let x = position_snapped.x - i;
                    if let Some(s) =
                        collision_space.get_sprite_at(&Point2::new(x, position_snapped.y), mask)
                    {
                        result = Some((position.x - s.right(), s));
                        break;
                    }
                }
            }
        };

        // we only accept collisions with square shapes - because slopes are special cases handled by
        // find_character_footing only (note, the ganme only has northeast, and northwest slopes)
        if let Some(result) = result {
            if result.0 >= 0.0 && result.1.collision_shape == sprite::CollisionShape::Square {
                return Some(result);
            }
        }

        None
    }
}
