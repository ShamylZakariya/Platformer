use cgmath::{vec2, Point2, Vector2, Zero};
use std::{collections::HashSet, time::Duration};
use winit::event::*;

use crate::map::{FLAG_MAP_TILE_IS_COLLIDER, FLAG_MAP_TILE_IS_RATCHET};
use crate::sprite;
use crate::sprite_collision::{CollisionSpace, ProbeDir, ProbeResult};

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

// These constants were determined by examination of recorded gamplay
const GRAVITY_SPEED_FINAL: f32 = -1.0 / 0.12903225806451613;
const WALK_SPEED: f32 = 1.0 / 0.3145;
const JUMP_DURATION: f32 = 0.45;
const GRAVITY_ACCEL_TIME: f32 = JUMP_DURATION;

// ---------------------------------------------------------------------------------------------------------------------

fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stance {
    Standing,
    InAir,
    WallHold,
}

impl Eq for Stance {}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct CharacterState {
    // The current position of the character
    pub position: Point2<f32>,
    // The current display cycle of the character, will be one of the CHARACTER_CYCLE_* constants.
    pub cycle: &'static str,

    // the character's current stance state
    pub stance: Stance,
}

impl CharacterState {
    fn new(position: &Point2<f32>) -> Self {
        CharacterState {
            position: *position,
            cycle: CHARACTER_CYCLE_DEFAULT,
            stance: Stance::Standing,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ButtonState {
    Pressed,
    Down,
    Released,
    Up,
}

impl ButtonState {
    fn transition(&self, key_down: bool) -> ButtonState {
        if key_down {
            match self {
                ButtonState::Pressed => ButtonState::Down,
                ButtonState::Down => ButtonState::Down,
                ButtonState::Released => ButtonState::Pressed,
                ButtonState::Up => ButtonState::Pressed,
            }
        } else {
            match self {
                ButtonState::Pressed => ButtonState::Released,
                ButtonState::Down => ButtonState::Released,
                ButtonState::Released => ButtonState::Up,
                ButtonState::Up => ButtonState::Up,
            }
        }
    }

    fn is_active(&self) -> bool {
        match self {
            ButtonState::Pressed | ButtonState::Down => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
struct InputState {
    move_left: ButtonState,
    move_right: ButtonState,
    jump: ButtonState,
    fire: ButtonState,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            move_left: ButtonState::Up,
            move_right: ButtonState::Up,
            jump: ButtonState::Up,
            fire: ButtonState::Up,
        }
    }
}

impl InputState {
    fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;
        match key {
            VirtualKeyCode::W => {
                self.jump = self.jump.transition(pressed);
                true
            }
            VirtualKeyCode::A => {
                self.move_left = self.move_left.transition(pressed);
                true
            }
            VirtualKeyCode::D => {
                self.move_right = self.move_right.transition(pressed);
                true
            }
            VirtualKeyCode::Space => {
                self.fire = self.fire.transition(pressed);
                true
            }
            _ => false,
        }
    }

    fn update(&mut self) {
        self.jump = self.jump.transition(self.jump.is_active());
        self.move_left = self.move_left.transition(self.move_left.is_active());
        self.move_right = self.move_right.transition(self.move_right.is_active());
        self.fire = self.fire.transition(self.fire.is_active());
    }
}

fn input_accumulator(negative: ButtonState, positive: ButtonState) -> f32 {
    let mut acc = 0.0;
    match negative {
        ButtonState::Pressed | ButtonState::Down | ButtonState::Released => {
            acc -= 1.0;
        }
        ButtonState::Up => {}
    }
    match positive {
        ButtonState::Pressed | ButtonState::Down | ButtonState::Released => {
            acc += 1.0;
        }
        ButtonState::Up => {}
    }

    acc
}

#[derive(Debug)]
pub struct CharacterController {
    time: f32,
    input_state: InputState,
    pub character_state: CharacterState,

    // sprites the character is overlapping and might collide with
    pub overlapping_sprites: HashSet<sprite::SpriteDesc>,

    // sprites the character is contacting
    pub contacting_sprites: HashSet<sprite::SpriteDesc>,

    vertical_velocity: f32,
    jump_start_time: Option<f32>,
}

impl CharacterController {
    pub fn new(position: &Point2<f32>) -> Self {
        Self {
            time: 0.0,
            input_state: Default::default(),
            character_state: CharacterState::new(position),
            overlapping_sprites: HashSet::new(),
            contacting_sprites: HashSet::new(),
            vertical_velocity: 0.0,
            jump_start_time: None,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        self.input_state.process_keyboard(key, state)
    }

    pub fn update(&mut self, dt: Duration, collision_space: &CollisionSpace) -> &CharacterState {
        self.overlapping_sprites.clear();
        self.contacting_sprites.clear();

        let dt = dt.as_secs_f32();
        self.time += dt;

        let movement = vec2(
            input_accumulator(self.input_state.move_left, self.input_state.move_right) * WALK_SPEED,
            0.0,
        ) * dt;

        if self.input_state.jump.is_active() {
            if self.input_state.jump == ButtonState::Pressed
                && self.character_state.stance == Stance::Standing
            {
                println!("Jump started");
                self.jump_start_time = Some(self.time);
                self.set_stance(Stance::InAir);
            }
        } else {
            self.jump_start_time = None;
        }

        if let Some(jump_start_time) = self.jump_start_time {
            if self.time - jump_start_time > JUMP_DURATION {
                println!("Jump expired");
                self.jump_start_time = None;
            }
        }

        //
        //  Determine if the character is standing on a surface or in the air.
        //  This method probes downwards one step the farthest gravity would carry character.
        //  It returns the position of the character and whether they're in the air.
        //
        let in_air = {
            let gravity_delta_position = vec2(0.0, GRAVITY_SPEED_FINAL) * dt;
            let mut position = self.character_state.position + gravity_delta_position;

            let footing_center = self.find_character_footing(
                collision_space,
                position,
                gravity_delta_position,
                Zero::zero(),
                true,
            );
            position = footing_center.0;

            let footing_right = self.find_character_footing(
                collision_space,
                position,
                gravity_delta_position,
                vec2(1.0, 0.0),
                footing_center.1.is_none(),
            );
            position = footing_right.0;

            let footing_left = self.find_character_footing(
                collision_space,
                position,
                gravity_delta_position,
                vec2(-1.0, 0.0),
                footing_center.1.is_none() && footing_right.1.is_none(),
            );

            let in_air =
                footing_center.1.is_none() && footing_right.1.is_none() && footing_left.1.is_none();

            if in_air {
                self.set_stance(Stance::InAir);
            }

            if self.is_jumping() {
                true
            } else {
                in_air
            }
        };

        let g = self.apply_gravity(self.character_state.position, dt);
        let position = g.0;
        //
        //  Move character left/right/up
        //

        let position = self
            .apply_character_movement(collision_space, position, movement)
            .0;

        for s in &self.contacting_sprites {
            self.overlapping_sprites.remove(s);
        }

        self.character_state.position = position;

        if in_air {
            self.set_stance(Stance::InAir);
        } else {
            self.vertical_velocity = 0.0;
            self.set_stance(Stance::Standing);
        }

        self.input_state.update();

        &self.character_state
    }

    fn apply_gravity(&mut self, position: Point2<f32>, dt: f32) -> (Point2<f32>, Vector2<f32>) {
        match self.character_state.stance {
            Stance::Standing => {
                self.vertical_velocity = 0.0;
            }
            Stance::InAir => {
                let mut is_jumping = false;
                if let Some(jump_start_time) = self.jump_start_time {
                    let elapsed = self.time - jump_start_time;
                    if elapsed < JUMP_DURATION {
                        is_jumping = true;
                        self.vertical_velocity =
                            lerp(elapsed / JUMP_DURATION, -GRAVITY_SPEED_FINAL, 0.0);
                    }
                }
                // if not applying a jump force, we're falling
                if !is_jumping {
                    self.vertical_velocity =
                        lerp(0.01, self.vertical_velocity, GRAVITY_SPEED_FINAL);
                }
            }
            Stance::WallHold => {}
        }
        let motion = vec2(0.0, self.vertical_velocity * dt);
        (position + motion, motion)
    }

    fn is_jumping(&self) -> bool {
        if let Some(jump_start_time) = self.jump_start_time {
            let elapsed = self.time - jump_start_time;
            if elapsed < JUMP_DURATION {
                return true;
            }
        }
        false
    }

    fn set_stance(&mut self, new_stance: Stance) {
        if new_stance != self.character_state.stance {
            println!(
                "Transition from {:?} to {:?}",
                self.character_state.stance, new_stance
            );
            match self.character_state.stance {
                Stance::Standing => match new_stance {
                    Stance::Standing => {}
                    Stance::InAir => {}
                    Stance::WallHold => {}
                },
                Stance::InAir => match new_stance {
                    Stance::Standing => {}
                    Stance::InAir => {}
                    Stance::WallHold => {}
                },
                Stance::WallHold => {}
            }
            self.character_state.stance = new_stance;
        }
    }

    /// looks beneath `position` to find the surface that the character would be standing on. This should be called
    /// after gravity is applied, but before any user initiated movement.
    /// - position: The position of the character
    /// - gravity_delta_position: The change in position caused by gravity from last game state
    /// - test_offset: An offset to apply to position
    /// - may apply_correction: If true, this method will apply correction to position if it is found to be intersecting a footing
    ///
    /// Returns the updated (if necessary) character position, and if found the sprite which the player is standing on
    /// or would be standing on if player were lower
    ///
    /// If player is contacting any surfaces, they will be passed to handle_collision_with()
    fn find_character_footing(
        &mut self,
        collision_space: &CollisionSpace,
        position: Point2<f32>,
        gravity_delta_position: Vector2<f32>,
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

        let can_collide_width = |p: &Point2<f32>, s: &sprite::SpriteDesc| -> bool {
            if s.mask & FLAG_MAP_TILE_IS_RATCHET != 0 && p.y < (s.top() + gravity_delta_position.y)
            {
                false
            } else {
                true
            }
        };

        for test_point in [below_center, center].iter() {
            if let Some(s) = collision_space.get_sprite_at(*test_point, FLAG_MAP_TILE_IS_COLLIDER) {
                if can_collide_width(&position, &s) {
                    match s.collision_shape {
                        sprite::CollisionShape::Square => {
                            if s.unit_rect_intersection(&position, inset, contacts_are_collision) {
                                self.handle_collision_with(&s);
                                tracking = Some(s);
                                if may_apply_correction {
                                    position.y = s.origin.y + s.extent.y;
                                }
                            }
                        }
                        sprite::CollisionShape::NorthEast | sprite::CollisionShape::NorthWest => {
                            if let Some(intersection) = s.line_intersection(
                                &(position + vec2(0.5, 1.0)),
                                &(position + vec2(0.5, 0.0)),
                            ) {
                                self.handle_collision_with(&s);
                                tracking = Some(s);
                                if may_apply_correction {
                                    position.y = intersection.y;
                                }
                            }
                        }
                        _ => (),
                    }
                    self.overlapping_sprites.insert(s);
                }
            }
        }

        (position, tracking)
    }

    /// Applies a specified character movement to player, returning the player's new position and the change in position.
    /// If player is contacting any surfaces, they will be passed to handle_collision_with()
    fn apply_character_movement(
        &mut self,
        collision_space: &CollisionSpace,
        position: Point2<f32>,
        movement: Vector2<f32>,
    ) -> (Point2<f32>, Vector2<f32>) {
        let steps = 3;
        let mask = FLAG_MAP_TILE_IS_COLLIDER;
        let mut delta_x = movement.x;
        let mut delta_y = movement.y;

        // if the probe result is a ratchet tile, determine if it should be skipped.
        let probe_test = |_dist: f32, sprite: &sprite::SpriteDesc| -> bool {
            if position.y < sprite.top() && sprite.mask & FLAG_MAP_TILE_IS_RATCHET != 0 {
                false
            } else {
                true
            }
        };

        if delta_x > 0.0 {
            match collision_space.probe(position, ProbeDir::Right, steps, mask, probe_test) {
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
            match collision_space.probe(position, ProbeDir::Left, steps, mask, probe_test) {
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
            match collision_space.probe(position, ProbeDir::Up, steps, mask, probe_test) {
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

    /// Callback for handling collision with scene geometry.
    fn handle_collision_with(&mut self, sprite: &sprite::SpriteDesc) {
        self.contacting_sprites.insert(*sprite);
    }
}
