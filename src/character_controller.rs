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
    North,
    East,
    South,
    West,
}

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
        let mut delta_x = dt
            * WALK_SPEED
            * input_accumulator(
                self.input_state.move_left_pressed,
                self.input_state.move_right_pressed,
            );

        let mut delta_y = dt * WALK_SPEED * input_accumulator(false, self.input_state.jump_pressed);

        {
            let mut possibly_collided_with = Vec::with_capacity(2);

            if delta_x > 0.0 {
                if let Some(result) = self.probe(
                    collision_space,
                    position,
                    ProbeDir::East,
                    4,
                    FLAG_MAP_TILE_IS_COLLIDER,
                    &mut possibly_collided_with,
                ) {
                    if result < delta_x {
                        delta_x = result;
                        for s in possibly_collided_with {
                            self.handle_collision_with(&s);
                        }
                    }
                }
            } else if delta_x < 0.0 {
                if let Some(result) = self.probe(
                    collision_space,
                    position,
                    ProbeDir::West,
                    4,
                    FLAG_MAP_TILE_IS_COLLIDER,
                    &mut possibly_collided_with,
                ) {
                    if result < -delta_x {
                        delta_x = -result;
                        for s in possibly_collided_with {
                            self.handle_collision_with(&s);
                        }
                    }
                }
            }
        }

        {
            let mut possibly_collided_with = Vec::with_capacity(2);
            if delta_y > 0.0 {
                if let Some(result) = self.probe(
                    collision_space,
                    position,
                    ProbeDir::North,
                    4,
                    FLAG_MAP_TILE_IS_COLLIDER,
                    &mut possibly_collided_with,
                ) {
                    if result < delta_y {
                        delta_y = result;
                        for s in possibly_collided_with {
                            self.handle_collision_with(&s);
                        }
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
        possibly_collided_with: &mut Vec<sprite::SpriteDesc>,
    ) -> Option<f32> {
        possibly_collided_with.clear();

        // The problem here is that if the character is on an integerial boundary (e.g., x == 22, not 22.2) we're still probing the offset position

        let (offset, should_probe_offset) = match dir {
            ProbeDir::North | ProbeDir::South => (vec2(1.0, 0.0), position.x.fract().abs() > 0.0),
            ProbeDir::East | ProbeDir::West => (vec2(0.0, 1.0), position.y.fract().abs() > 0.0),
        };

        let mut dist = None;
        let r0 = self._probe(collision_space, position, dir, max_steps, mask);
        if let Some(r0) = r0 {
            dist = Some(r0.0);
            possibly_collided_with.push(r0.1);
        }

        let r1 = if should_probe_offset {
            let r1 = self._probe(collision_space, position + offset, dir, max_steps, mask);
            if let Some(r1) = r1 {
                dist = Some(r1.0);
                possibly_collided_with.push(r1.1);
            }
            r1
        } else {
            None
        };

        dist

        // if let Some(result) = self._probe(collision_space, position, dir, max_steps, mask) {
        //     Some(result)
        // } else if should_probe_offset {
        //     if let Some(result) =
        //         self._probe(collision_space, position + offset, dir, max_steps, mask)
        //     {
        //         Some(result)
        //     } else {
        //         None
        //     }
        // } else {
        //     None
        // }
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
            ProbeDir::East => {
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
            ProbeDir::North => {
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
            ProbeDir::South => {
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
            ProbeDir::West => {
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
