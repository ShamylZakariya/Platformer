use cgmath::{vec2, Point2, Vector2, Zero};
use std::{collections::HashSet, f32::consts::PI, time::Duration, unimplemented};
use winit::event::*;

use crate::map::{FLAG_MAP_TILE_IS_COLLIDER, FLAG_MAP_TILE_IS_RATCHET, FLAG_MAP_TILE_IS_WATER};
use crate::sprite;
use crate::sprite_collision::{CollisionSpace, ProbeDir, ProbeResult};

// ---------------------------------------------------------------------------------------------------------------------

const CHARACTER_CYCLE_DEFAULT: &str = "default";
pub const CHARACTER_CYCLE_DEBUG: &str = "debug";
const CHARACTER_CYCLE_SHOOT: &str = "shoot";
const CHARACTER_CYCLE_WALK_0: &str = "walk_0";
const CHARACTER_CYCLE_WALK_1: &str = "walk_1";
const CHARACTER_CYCLE_WALK_2: &str = "walk_2";
const CHARACTER_CYCLE_JUMP_0: &str = "jump_0";
const CHARACTER_CYCLE_JUMP_1: &str = "jump_1";
const CHARACTER_CYCLE_JUMP_2: &str = "jump_2";
const CHARACTER_CYCLE_FLY_0: &str = "fly_0";
const CHARACTER_CYCLE_FLY_1: &str = "fly_1";
const CHARACTER_CYCLE_FLY_2: &str = "fly_2";
const CHARACTER_CYCLE_WALL: &str = "wall";

// These constants were determined by examination of recorded gamplay (and fiddling)
// Units are seconds & tiles-per-second unless otherwise specified.
const GRAVITY_VEL: f32 = -1.0 / 0.12903225806451613;
const WALK_SPEED: f32 = 1.0 / 0.4;
const JUMP_DURATION: f32 = 0.45;
const GRAVITY_ACCEL_TIME: f32 = JUMP_DURATION;
const FLIGHT_DURATION: f32 = 1.0;
const FLIGHT_BOB_CYCLE_PERIOD: f32 = 0.5;
const FLIGHT_BOB_CYCLE_PIXELS_OFFSET: i32 = -2;
const COLLISION_PROBE_STEPS: i32 = 3;
const WALLGRAB_JUMP_LATERAL_MOTION_DURATION: f32 = 0.17;
const WALLGRAB_JUMP_LATERAL_VEL: f32 = 20.0;
const WATER_DAMPING: f32 = 0.5;

// Animation timings
const WALK_CYCLE_DURATION: f32 = 0.2;
const FLIGHT_CYCLE_DURATION: f32 = 0.1;
const JUMP_CYCLE_DURATION: f32 = 0.1;

// ---------------------------------------------------------------------------------------------------------------------

fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}

fn clamp(v: f32, min: f32, max: f32) -> f32 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

fn create_collision_probe_test(position: Point2<f32>) -> impl Fn(f32, &sprite::SpriteDesc) -> bool {
    move |_dist: f32, sprite: &sprite::SpriteDesc| -> bool {
        if position.y < sprite.top() && sprite.mask & FLAG_MAP_TILE_IS_RATCHET != 0 {
            false
        } else {
            true
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stance {
    Standing,
    InAir,
    Flying,
    WallHold(sprite::SpriteDesc),
}

impl Eq for Stance {}

#[derive(Debug, Clone, Copy)]
pub enum Facing {
    Left,
    Right,
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct CharacterState {
    // The current position of the character
    pub position: Point2<f32>,

    // The current position-offset of the character - this is purely visual, used for bobbing effects,
    // and is not part of collision detection.
    pub position_offset: Vector2<f32>,

    // The current display cycle of the character, will be one of the CHARACTER_CYCLE_* constants.
    pub cycle: &'static str,

    // the character's current stance state
    pub stance: Stance,

    // the direction the character is currently facing
    pub facing: Facing,
}

impl CharacterState {
    fn new(position: &Point2<f32>) -> Self {
        CharacterState {
            position: *position,
            position_offset: Zero::zero(),
            cycle: CHARACTER_CYCLE_DEFAULT,
            stance: Stance::Standing,
            facing: Facing::Left,
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
    jump_time_remaining: f32,
    flight_time_remaining: f32,
    wallgrab_jump_lateral_motion_time_remaining: f32,
    wallgrab_jump_dir: f32, // -1 for left, +1 for right
    map_origin: Point2<f32>,
    map_extent: Vector2<f32>,
    pixels_per_unit: f32,
    cycle_animation_time_elapsed: Option<f32>,
    in_water: bool,
}

impl CharacterController {
    pub fn new(
        position: &Point2<f32>,
        map_origin: Point2<f32>,
        map_extent: Vector2<f32>,
        pixels_per_unit: u32,
    ) -> Self {
        Self {
            time: 0.0,
            input_state: Default::default(),
            character_state: CharacterState::new(position),
            overlapping_sprites: HashSet::new(),
            contacting_sprites: HashSet::new(),
            vertical_velocity: 0.0,
            jump_time_remaining: 0.0,
            flight_time_remaining: FLIGHT_DURATION,
            wallgrab_jump_lateral_motion_time_remaining: 0.0,
            wallgrab_jump_dir: 0.0,
            map_origin,
            map_extent,
            pixels_per_unit: pixels_per_unit as f32,
            cycle_animation_time_elapsed: None,
            in_water: false,
        }
    }

    pub fn is_jumping(&self) -> bool {
        self.character_state.stance == Stance::InAir && self.jump_time_remaining > 0.0
    }

    pub fn is_flying(&self) -> bool {
        self.character_state.stance == Stance::Flying && self.flight_time_remaining > 0.0
    }

    pub fn is_wallholding(&self) -> bool {
        match self.character_state.stance {
            Stance::WallHold(_) => true,
            _ => false,
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

        //
        //  Handle jump button
        //

        match self.input_state.jump {
            ButtonState::Pressed => match self.character_state.stance {
                Stance::Standing => {
                    self.jump_time_remaining = JUMP_DURATION;
                    self.set_stance(Stance::InAir);
                }
                Stance::InAir => {
                    if self.flight_time_remaining > 0.0 {
                        self.jump_time_remaining = 0.0;
                        self.set_stance(Stance::Flying);
                    }
                }
                Stance::Flying => {
                    self.set_stance(Stance::InAir);
                }
                Stance::WallHold(surface) => {
                    self.wallgrab_jump_lateral_motion_time_remaining =
                        WALLGRAB_JUMP_LATERAL_MOTION_DURATION;
                    self.jump_time_remaining = JUMP_DURATION;
                    self.wallgrab_jump_dir = if surface.origin.x > self.character_state.position.x {
                        -1.0
                    } else {
                        1.0
                    };
                    self.set_stance(Stance::InAir);
                }
            },
            ButtonState::Released => {
                self.jump_time_remaining = 0.0;
            }
            _ => {}
        }

        //
        //  Determine if the character is standing on a surface or in the air.
        //  This method probes downwards one step the farthest gravity would carry character.
        //  It returns the position of the character and whether they're in the air.
        //

        let (position, contacting_ground) = {
            let gravity_delta_position = vec2(0.0, GRAVITY_VEL) * dt;
            let mut position = self.character_state.position + gravity_delta_position;

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

            let contacting_ground =
                footing_center.1.is_some() || footing_right.1.is_some() || footing_left.1.is_some();

            //
            //  If character just walked off a ledge start falling
            //

            if !contacting_ground
                && self.character_state.stance != Stance::Flying
                && !self.is_wallholding()
            {
                self.set_stance(Stance::InAir);
            }

            if self.character_state.stance == Stance::Flying
                || (self.character_state.stance == Stance::InAir && self.vertical_velocity > 0.0)
                || self.is_wallholding()
            {
                (self.character_state.position, contacting_ground)
            } else {
                if self.character_state.stance == Stance::InAir {
                    (self.character_state.position, contacting_ground)
                } else {
                    (position, contacting_ground)
                }
            }
        };

        //
        //  Apply gravity to character position - will update vertical velocity
        //  if character stance is InAir. Also performs collision detection of head against
        //  ceilings, and will terminate the upward phase of a jump if bump head.
        //

        let position = self.apply_vertical_movement(collision_space, position, dt);

        //
        //  Apply character movement
        //

        let (position, wall_contact) = self.apply_lateral_movement(collision_space, position, dt);

        //
        //  Note, vertical_velocity may have been changed by apply_gravity, so only
        //  change stance to Standing iff contacting ground and vertical_vel is not upwards.
        //

        if contacting_ground && self.vertical_velocity <= 0.0 {
            self.set_stance(Stance::Standing);
        }

        if let Some(wall_contact) = wall_contact {
            if self.character_state.stance == Stance::InAir
                || self.character_state.stance == Stance::Flying
            {
                self.set_stance(Stance::WallHold(wall_contact));
            }
        }

        //
        //  Final steps - update character position and if flying, apply the bob offset
        //

        self.character_state.position = position;

        //
        //  Track jump and flight timed expirations
        //

        if self.character_state.stance == Stance::InAir {
            if self.jump_time_remaining > 0.0 {
                self.jump_time_remaining -= dt;
            }

            if self.jump_time_remaining < 0.0 {
                self.jump_time_remaining = 0.0;
            }

            if self.wallgrab_jump_lateral_motion_time_remaining > 0.0 {
                self.wallgrab_jump_lateral_motion_time_remaining -= dt;
            }

            if self.wallgrab_jump_lateral_motion_time_remaining < 0.0 {
                self.wallgrab_jump_lateral_motion_time_remaining = 0.0;
            }
        }

        if self.character_state.stance == Stance::Flying {
            // Apply flight bob cycle
            if self.flight_time_remaining > 0.0 {
                let elapsed = FLIGHT_DURATION - self.flight_time_remaining;
                let bob_cycle =
                    ((elapsed / FLIGHT_BOB_CYCLE_PERIOD) * 2.0 * PI - PI / 2.0).sin() * 0.5 + 0.5; // remap to [0,1]
                let bob_offset = bob_cycle * FLIGHT_BOB_CYCLE_PIXELS_OFFSET as f32;
                self.character_state.position_offset = vec2(0.0, bob_offset / self.pixels_per_unit);
            }

            // Decrement remaining flight time
            self.flight_time_remaining = self.flight_time_remaining - dt;
            if self.flight_time_remaining <= 0.0 {
                self.flight_time_remaining = 0.0;
                self.set_stance(Stance::InAir);
            }
        } else {
            self.character_state.position_offset = Zero::zero();
        }

        //
        //  Determine if character is in water
        //

        self.in_water = self.is_in_water(collision_space, self.character_state.position);

        //
        //  Update character cycle and animation, and facing dir
        //

        self.character_state.cycle = self.update_character_cycle(dt);
        self.character_state.facing = self.character_facing();

        //
        //  Remove any sprites in the contacting set from the overlapping set.
        //

        for s in &self.contacting_sprites {
            self.overlapping_sprites.remove(s);
        }

        self.input_state.update();

        &self.character_state
    }

    fn set_stance(&mut self, new_stance: Stance) {
        if new_stance != self.character_state.stance {
            // println!(
            //     "Transition from {:?} to {:?}",
            //     self.character_state.stance, new_stance
            // );

            match self.character_state.stance {
                Stance::Standing => match new_stance {
                    Stance::Standing => {}
                    Stance::InAir => {}
                    Stance::Flying => {}
                    Stance::WallHold(_) => {}
                },
                Stance::InAir => match new_stance {
                    Stance::Standing => {}
                    Stance::InAir => {}
                    Stance::Flying => {}
                    Stance::WallHold(_) => {}
                },
                Stance::Flying => match new_stance {
                    Stance::Standing => {}
                    Stance::InAir => {}
                    Stance::Flying => {}
                    Stance::WallHold(_) => {}
                },
                Stance::WallHold(_) => match new_stance {
                    Stance::Standing => {}
                    Stance::InAir => {}
                    Stance::Flying => {}
                    Stance::WallHold(_) => {}
                },
            }

            match new_stance {
                // Flight time is reset whenever character touches ground or wallholds
                Stance::Standing | Stance::WallHold(_) => {
                    self.flight_time_remaining = FLIGHT_DURATION;
                }
                _ => {}
            }

            self.character_state.stance = new_stance;
        }
    }

    /// looks beneath `position` to find the surface that the character would be standing on. This should be called
    /// after gravity is applied, but before any user initiated movement.
    /// - position: The position of the character
    /// - gravity_delta_position: The change in position caused by gravity from last game state
    /// - test_offset: An offset to apply to position
    /// - may apply_correction: Icharacterf player were lower
    ///
    /// If player is contacting any surfaces, they will be passed to handle_collision_with()
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

        let can_collide_width = |p: &Point2<f32>, s: &sprite::SpriteDesc| -> bool {
            // if character is more than 75% up a ratchet block consider it a collision
            if s.mask & FLAG_MAP_TILE_IS_RATCHET != 0 && p.y < (s.top() - 0.25) {
                false
            } else {
                true
            }
        };

        for test_point in [below_center, center].iter() {
            if let Some(s) =
                collision_space.get_sprite_at(*test_point, FLAG_MAP_TILE_IS_COLLIDER)
            {
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

    /// Moves character horizontally, based on the current left/right input state.
    /// Returns tuple of updated position, and an optional sprite representing the wall surface the
    /// character may have contacted. The character can collide with up to two sprites if on fractional
    /// y coord, so this returns the one closer to the character's y position)
    fn apply_lateral_movement(
        &mut self,
        collision_space: &CollisionSpace,
        position: Point2<f32>,
        dt: f32,
    ) -> (Point2<f32>, Option<sprite::SpriteDesc>) {
        // this is a no-op while wallholding
        if self.is_wallholding() {
            return (position, None);
        }

        let mask = FLAG_MAP_TILE_IS_COLLIDER;
        let probe_test = create_collision_probe_test(position);

        let mut delta_x =
            input_accumulator(self.input_state.move_left, self.input_state.move_right)
                * WALK_SPEED
                * dt;

        // walljump overrides user input vel birefly.
        if self.wallgrab_jump_lateral_motion_time_remaining > 0.0 {
            delta_x = WALLGRAB_JUMP_LATERAL_VEL
                * self.wallgrab_jump_lateral_motion_time_remaining
                * dt
                * self.wallgrab_jump_dir;
        }

        let mut contacted: Option<sprite::SpriteDesc> = None;

        //
        // Check if moving left or right would cause a collision, and adjust distance accordingly
        //

        if delta_x > 0.0 {
            match collision_space.probe(
                position,
                ProbeDir::Right,
                COLLISION_PROBE_STEPS,
                mask,
                probe_test,
            ) {
                ProbeResult::None => {}
                ProbeResult::OneHit { dist, sprite } => {
                    if dist < delta_x {
                        delta_x = dist;
                        contacted = Some(sprite);
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
                        let dist_0 = (sprite_0.origin.y - position.y).abs();
                        let dist_1 = (sprite_1.origin.y - position.y).abs();
                        contacted = if dist_0 < dist_1 {
                            Some(sprite_0)
                        } else {
                            Some(sprite_1)
                        };
                        self.handle_collision_with(&sprite_0);
                        self.handle_collision_with(&sprite_1);
                    }
                }
            }
        } else if delta_x < 0.0 {
            match collision_space.probe(
                position,
                ProbeDir::Left,
                COLLISION_PROBE_STEPS,
                mask,
                probe_test,
            ) {
                ProbeResult::None => {}
                ProbeResult::OneHit { dist, sprite } => {
                    if dist < -delta_x {
                        delta_x = -dist;
                        contacted = Some(sprite);
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
                        let dist_0 = (sprite_0.origin.y - position.y).abs();
                        let dist_1 = (sprite_1.origin.y - position.y).abs();
                        contacted = if dist_0 < dist_1 {
                            Some(sprite_0)
                        } else {
                            Some(sprite_1)
                        };
                        self.handle_collision_with(&sprite_0);
                        self.handle_collision_with(&sprite_1);
                    }
                }
            }
        }

        //
        //  Wallgrabs are dissallowed on the top-helf of a ledge (no tile above the contacted tile)
        //

        if let Some(c) = contacted {
            if collision_space
                .get_sprite_at(Point2::new(c.origin.x as i32, c.origin.y as i32 + 1), mask)
                .is_none()
            {
                if position.y > c.origin.y + (c.extent.y * 0.5) {
                    contacted = None;
                }
            }
        }

        //
        //  Clamp position to fit on stage
        //

        (
            Point2::new(
                clamp(
                    position.x + delta_x,
                    self.map_origin.x,
                    self.map_origin.x + self.map_extent.x - 1.0,
                ),
                clamp(
                    position.y,
                    self.map_origin.y,
                    self.map_origin.y + self.map_extent.y - 1.0,
                ),
            ),
            contacted,
        )
    }

    //
    //  Applies gravity to `position`, if the current stance is InAir.
    //  Updates self.vertical_velocity to make an accel curve.
    //

    fn apply_vertical_movement(
        &mut self,
        collision_space: &CollisionSpace,
        position: Point2<f32>,
        dt: f32,
    ) -> Point2<f32> {
        match self.character_state.stance {
            Stance::Standing | Stance::Flying | Stance::WallHold(_) => {
                if self.vertical_velocity.abs() != 0.0 {
                    self.vertical_velocity = 0.0;
                }
            }
            Stance::InAir => {
                if self.jump_time_remaining > 0.0 {
                    let elapsed = JUMP_DURATION - self.jump_time_remaining;
                    let jump_completion = elapsed / JUMP_DURATION;
                    self.vertical_velocity = lerp(jump_completion, -GRAVITY_VEL, 0.0);
                } else {
                    self.vertical_velocity = lerp(2.5 * dt, self.vertical_velocity, GRAVITY_VEL);
                }
            }
        }

        let mut delta = vec2(0.0, self.vertical_velocity * dt);
        if self.in_water && self.vertical_velocity < 0.0 {
            delta.y *= WATER_DAMPING;
        }

        //
        //  Now, if the movement is vertical, do a collision check with ceiling
        //

        if delta.y > 0.0 {
            let mask = FLAG_MAP_TILE_IS_COLLIDER;
            let probe_test = create_collision_probe_test(position);
            match collision_space.probe(
                position,
                ProbeDir::Up,
                COLLISION_PROBE_STEPS,
                mask,
                probe_test,
            ) {
                ProbeResult::None => {}
                ProbeResult::OneHit { dist, sprite } => {
                    if dist < delta.y {
                        delta.y = dist;
                        self.jump_time_remaining = 0.0;
                        self.handle_collision_with(&sprite);
                    }
                }
                ProbeResult::TwoHits {
                    dist,
                    sprite_0,
                    sprite_1,
                } => {
                    if dist < delta.y {
                        delta.y = dist;
                        self.jump_time_remaining = 0.0;
                        self.handle_collision_with(&sprite_0);
                        self.handle_collision_with(&sprite_1);
                    }
                }
            }
        }

        position + delta
    }

    /// Callback for handling collision with scene geometry.
    fn handle_collision_with(&mut self, sprite: &sprite::SpriteDesc) {
        self.contacting_sprites.insert(*sprite);
    }

    fn update_character_cycle(&mut self, dt: f32) -> &'static str {
        // The character "walks" when in water, otherwise use the actual stance.
        let stance = if self.in_water {
            match self.character_state.stance {
                Stance::Standing | Stance::InAir | Stance::Flying => Stance::Standing,
                Stance::WallHold(_) => self.character_state.stance,
            }
        } else {
            self.character_state.stance
        };

        match stance {
            Stance::Standing => {
                if self.input_state.move_left.is_active() || self.input_state.move_right.is_active()
                {
                    if self.cycle_animation_time_elapsed.is_none() {
                        self.cycle_animation_time_elapsed = Some(0.0);
                    }
                    let elapsed = self.cycle_animation_time_elapsed.unwrap();

                    let frame = ((elapsed / WALK_CYCLE_DURATION).floor() as i32) % 4;
                    self.cycle_animation_time_elapsed = Some(elapsed + dt);

                    match frame {
                        0 => CHARACTER_CYCLE_WALK_0,
                        1 => CHARACTER_CYCLE_WALK_1,
                        2 => CHARACTER_CYCLE_WALK_0,
                        3 => CHARACTER_CYCLE_WALK_2,
                        _ => unimplemented!("This shouldn't be reached"),
                    }
                } else {
                    self.cycle_animation_time_elapsed = None;
                    CHARACTER_CYCLE_WALK_0
                }
            }
            Stance::InAir => {
                if self.cycle_animation_time_elapsed.is_none() {
                    self.cycle_animation_time_elapsed = Some(0.0);
                }
                let elapsed = self.cycle_animation_time_elapsed.unwrap();

                let frame = ((elapsed / JUMP_CYCLE_DURATION).floor() as i32) % 4;
                self.cycle_animation_time_elapsed = Some(elapsed + dt);

                match frame {
                    0 => CHARACTER_CYCLE_JUMP_0,
                    1 => CHARACTER_CYCLE_JUMP_1,
                    2 => CHARACTER_CYCLE_JUMP_2,
                    3 => CHARACTER_CYCLE_JUMP_1,
                    _ => unimplemented!("This shouldn't be reached"),
                }
            }
            Stance::Flying => {
                if self.cycle_animation_time_elapsed.is_none() {
                    self.cycle_animation_time_elapsed = Some(0.0);
                }
                let elapsed = self.cycle_animation_time_elapsed.unwrap();

                let frame = ((elapsed / FLIGHT_CYCLE_DURATION).floor() as i32) % 4;
                self.cycle_animation_time_elapsed = Some(elapsed + dt);

                match frame {
                    0 => CHARACTER_CYCLE_FLY_0,
                    1 => CHARACTER_CYCLE_FLY_1,
                    2 => CHARACTER_CYCLE_FLY_2,
                    3 => CHARACTER_CYCLE_FLY_1,
                    _ => unimplemented!("This shouldn't be reached"),
                }
            }
            Stance::WallHold(_) => CHARACTER_CYCLE_WALL,
        }
    }

    fn character_facing(&self) -> Facing {
        match self.character_state.stance {
            Stance::Standing | Stance::InAir | Stance::Flying => {
                if self.input_state.move_left.is_active() {
                    Facing::Left
                } else if self.input_state.move_right.is_active() {
                    Facing::Right
                } else {
                    self.character_state.facing
                }
            }
            Stance::WallHold(attached_to) => {
                if attached_to.left() > self.character_state.position.x {
                    Facing::Left
                } else {
                    Facing::Right
                }
            }
        }
    }

    fn is_in_water(&self, collision_space: &CollisionSpace, position: Point2<f32>) -> bool {
        let a = Point2::new(position.x.floor() as i32, position.y.floor() as i32);
        let b = Point2::new(a.x + 1, a.y);
        let c = Point2::new(a.x, a.y + 1);
        let d = Point2::new(a.x + 1, a.y + 1);

        for p in [a, b, c, d].iter() {
            if collision_space
                .get_sprite_at(*p, FLAG_MAP_TILE_IS_WATER)
                .is_some()
            {
                return true;
            }
        }
        return false;
    }
}
