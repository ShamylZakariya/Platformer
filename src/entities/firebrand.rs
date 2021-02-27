use std::{collections::HashSet, f32::consts::PI, fmt::Display, time::Duration};

use cgmath::*;
use sprite::CollisionShape;
use winit::event::{ElementState, VirtualKeyCode};

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    geom::{clamp, lerp, Bounds},
    input::*,
    map,
    sprite::{self, collision, rendering},
    state::{
        constants::{self, layers, sprite_masks::*, GRAVITY_VEL},
        events::Event,
    },
};

use super::{power_up, util::HorizontalDir};

// ---------------------------------------------------------------------------------------------------------------------

const CYCLE_DEFAULT: &str = "default";
pub const CYCLE_DEBUG: &str = "debug";
const CYCLE_SHOOT: &str = "shoot";
const CYCLE_WALK_0: &str = "walk_0";
const CYCLE_WALK_1: &str = "walk_1";
const CYCLE_WALK_2: &str = "walk_2";
const CYCLE_JUMP_0: &str = "jump_0";
const CYCLE_JUMP_1: &str = "jump_1";
const CYCLE_JUMP_2: &str = "jump_2";
const CYCLE_FLY_0: &str = "fly_0";
const CYCLE_FLY_1: &str = "fly_1";
const CYCLE_FLY_2: &str = "fly_2";
const CYCLE_FLY_SHOOT_0: &str = "fly_shoot_0";
const CYCLE_FLY_SHOOT_1: &str = "fly_shoot_1";
const CYCLE_FLY_SHOOT_2: &str = "fly_shoot_2";
const CYCLE_INJURY_0: &str = "fly_shoot_1";
const CYCLE_INJURY_1: &str = "injured";
const CYCLE_INJURY_2: &str = "fly_shoot_2";
const CYCLE_INJURY_3: &str = "injured";
const CYCLE_WALL: &str = "wall";
const CYCLE_WALL_SHOOT: &str = "wall_shoot";

const COLLISION_PROBE_STEPS: i32 = 3;

// These constants were determined by examination of recorded gamplay (and fiddling)
// Units are seconds & tiles-per-second unless otherwise specified.

const WALK_SPEED: f32 = 1.0 / 0.4;
const JUMP_DURATION: f32 = 0.45;
const FLIGHT_DURATION: f32 = 1.0;
const FLIGHT_BOB_CYCLE_PERIOD: f32 = 0.5;
const FLIGHT_BOB_CYCLE_PIXELS_OFFSET: i32 = -2;
const WALLGRAB_JUMP_LATERAL_MOTION_DURATION: f32 = 0.17;
const WALLGRAB_JUMP_LATERAL_VEL: f32 = 20.0;
const WATER_DAMPING: f32 = 0.5;
const INJURY_DURATION: f32 = 0.3;
const INJURY_KICKBACK_VEL: f32 = 0.5 / INJURY_DURATION;
const INVULNERABILITY_DURATION: f32 = 2.3;
const FIREBALL_VELOCITY: f32 = 1.0 / 0.166;
const FIREBALL_SHOOT_RATE: f32 = 1.0; // in the game only one fireball was visible
                                      // on screen at a time. It took 1 second to
                                      // go off screen and then could shoot again.
const FIREBALL_SHOOT_MOVEMENT_PAUSE_DURATION: f32 = 0.3;

// Animation timings
const WALK_CYCLE_DURATION: f32 = 0.2;
const FLIGHT_CYCLE_DURATION: f32 = 0.1;
const JUMP_CYCLE_DURATION: f32 = 0.1;
const INJURY_CYCLE_DURATION: f32 = 0.1;
const INVULNERABILITY_BLINK_PERIOD: f32 = 0.1;
const FIREBALL_CYCLE_DURATION: f32 = 0.3;

// Damage
const HIT_POINTS: u32 = 2;
const CONTACT_DAMAGE_HIT_POINTS: u32 = 1;
const FIREBALL_PROJECTILE_DAMAGE: u32 = 1;

// ---------------------------------------------------------------------------------------------------------------------

fn create_collision_probe_test(position: Point2<f32>) -> impl Fn(f32, &sprite::Sprite) -> bool {
    move |_dist: f32, sprite: &sprite::Sprite| -> bool {
        // ignore collision if the sprite is a ratched and position is below sprite
        !(position.y < sprite.top() && sprite.mask & RATCHET != 0)
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stance {
    Standing,
    InAir,
    Flying,
    WallHold(sprite::Sprite),
    Injury,
}

impl Eq for Stance {}

impl Display for Stance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Stance::Standing => write!(f, "Standing"),
            Stance::InAir => write!(f, "InAir"),
            Stance::Flying => write!(f, "Flying"),
            Stance::WallHold(_) => write!(f, "WallHold"),
            Stance::Injury => write!(f, "Injury"),
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

struct FirebrandInputState {
    input_state: InputState,
}

impl Default for FirebrandInputState {
    fn default() -> Self {
        Self {
            input_state: InputState::for_keys(&[
                VirtualKeyCode::W,
                VirtualKeyCode::A,
                VirtualKeyCode::D,
                VirtualKeyCode::Space,
            ]),
        }
    }
}

impl FirebrandInputState {
    fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        self.input_state.process_keyboard(key, state)
    }

    fn update(&mut self) {
        self.input_state.update();
    }

    fn jump(&self) -> &ButtonState {
        self.input_state
            .get_button_state(VirtualKeyCode::W)
            .unwrap()
    }

    fn move_left(&self) -> &ButtonState {
        self.input_state
            .get_button_state(VirtualKeyCode::A)
            .unwrap()
    }

    fn move_right(&self) -> &ButtonState {
        self.input_state
            .get_button_state(VirtualKeyCode::D)
            .unwrap()
    }

    fn fire(&self) -> &ButtonState {
        self.input_state
            .get_button_state(VirtualKeyCode::Space)
            .unwrap()
    }
}

// ---------------------------------------------------------------------------------------------------------------------
#[derive(Clone, Copy, Debug)]
pub enum ProbeDir {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Clone, Copy, Debug)]
pub enum ProbeResult<'a> {
    None,
    OneHit {
        dist: f32,
        sprite: &'a sprite::Sprite,
    },
    TwoHits {
        dist: f32,
        sprite_0: &'a sprite::Sprite,
        sprite_1: &'a sprite::Sprite,
    },
}

impl sprite::collision::Space {
    /// Probes `max_steps` in the collision space from `position` in `dir`, returning a ProbeResult
    /// Ignores any sprites which don't match the provided `mask`
    /// NOTE: Probe only tests for static sprites with Square collision shape, because, well,
    /// that's what's needed here and I'm not writing a game engine.
    pub fn probe_static_sprites<F>(
        &self,
        position: Point2<f32>,
        dir: ProbeDir,
        max_steps: i32,
        mask: u32,
        test: F,
    ) -> ProbeResult
    where
        F: Fn(f32, &sprite::Sprite) -> bool,
    {
        let (offset, should_probe_offset) = match dir {
            ProbeDir::Up | ProbeDir::Down => (vec2(1.0, 0.0), position.x.fract().abs() > 0.0),
            ProbeDir::Right | ProbeDir::Left => (vec2(0.0, 1.0), position.y.fract().abs() > 0.0),
        };

        let mut dist = None;
        let mut sprite_0 = None;
        let mut sprite_1 = None;
        if let Some(r) = self._probe_line(position, dir, max_steps, mask) {
            if test(r.0, &r.1) {
                dist = Some(r.0);
                sprite_0 = Some(r.1);
            }
        }

        if should_probe_offset {
            if let Some(r) = self._probe_line(position + offset, dir, max_steps, mask) {
                if test(r.0, &r.1) {
                    dist = match dist {
                        Some(d) => Some(d.min(r.0)),
                        None => Some(r.0),
                    };
                    sprite_1 = Some(r.1);
                }
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

    fn _probe_line(
        &self,
        position: Point2<f32>,
        dir: ProbeDir,
        max_steps: i32,
        mask: u32,
    ) -> Option<(f32, &sprite::Sprite)> {
        let position_snapped = point2(position.x.floor() as i32, position.y.floor() as i32);
        let mut result = None;
        match dir {
            ProbeDir::Right => {
                for i in 0..max_steps {
                    let x = position_snapped.x + i;
                    if let Some(s) = self.get_static_sprite_at(point2(x, position_snapped.y), mask)
                    {
                        result = Some((s.origin.x - (position.x + 1.0), s));
                        break;
                    }
                }
            }
            ProbeDir::Up => {
                for i in 0..max_steps {
                    let y = position_snapped.y + i;
                    if let Some(s) = self.get_static_sprite_at(point2(position_snapped.x, y), mask)
                    {
                        result = Some((s.origin.y - (position.y + 1.0), s));
                        break;
                    }
                }
            }
            ProbeDir::Down => {
                for i in 0..max_steps {
                    let y = position_snapped.y - i;
                    if let Some(s) = self.get_static_sprite_at(point2(position_snapped.x, y), mask)
                    {
                        result = Some((position.y - s.top(), s));
                        break;
                    }
                }
            }
            ProbeDir::Left => {
                for i in 0..max_steps {
                    let x = position_snapped.x - i;
                    if let Some(s) = self.get_static_sprite_at(point2(x, position_snapped.y), mask)
                    {
                        result = Some((position.x - s.right(), s));
                        break;
                    }
                }
            }
        };

        // we only accept collisions with square shapes - because slopes are special cases handled by
        // find_character_footing only (note, the game only has northeast, and northwest slopes)
        if let Some(result) = result {
            if result.0 >= 0.0 && result.1.collision_shape == sprite::CollisionShape::Square {
                return Some(result);
            }
        }

        None
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct CharacterState {
    // The current position of the character
    pub position: Point2<f32>,

    // The current position-offset of the character - this is purely visual, used for bobbing effects,
    // and is not part of collision detection.
    pub position_offset: Vector2<f32>,

    // The current display cycle of the character, will be one of the CYCLE_* constants.
    pub cycle: &'static str,

    // the character's current stance state
    pub stance: Stance,

    // the direction the character is currently facing
    pub facing: HorizontalDir,

    // player's current remaining hitpoints
    pub hit_points: u32,

    // player's max hit points
    pub hit_points_max: u32,

    // flight time remaining, in seconds
    pub flight_time_remaining: f32,

    // max flight time, in seconds
    pub flight_time_max: f32,

    // number of vials player has caught
    pub num_vials: u32,

    // number of lives remaining to player
    pub num_lives: u32,

    // is player currently alive
    pub alive: bool,
}

impl CharacterState {
    fn new(position: Point2<f32>, num_lives_remaining: u32) -> Self {
        CharacterState {
            position,
            position_offset: Zero::zero(),
            cycle: CYCLE_DEFAULT,
            stance: Stance::Standing,
            facing: HorizontalDir::East,
            hit_points: HIT_POINTS,
            hit_points_max: HIT_POINTS,
            flight_time_remaining: FLIGHT_DURATION,
            flight_time_max: FLIGHT_DURATION,
            num_vials: 0,
            num_lives: num_lives_remaining,
            alive: true,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

pub struct Firebrand {
    entity_id: u32,
    collider: Option<sprite::Sprite>,
    pixels_per_unit: Vector2<f32>,

    time: f32,
    step: usize,
    input_state: FirebrandInputState,
    character_state: CharacterState,

    // sprites the character is overlapping and might collide with
    overlapping_sprites: HashSet<sprite::Sprite>,

    // sprites the character is contacting
    contacting_sprites: HashSet<sprite::Sprite>,

    vertical_velocity: f32,
    jump_time_remaining: f32,
    flight_countdown: f32,
    wallgrab_jump_lateral_motion_countdown: f32,
    wallgrab_jump_dir: f32, // -1 for left, +1 for right
    cycle_animation_time_elapsed: Option<f32>,
    in_water: bool,
    injury_kickback_vel: f32,
    injury_countdown: f32,
    invulnerability_countdown: f32,
    last_shoot_time: f32,
    frozen: bool,
    did_send_death_message: bool,
    did_pass_through_exit_door: bool,
}

impl Firebrand {
    pub fn new(position: Point2<f32>, num_lives_remaining: u32) -> Firebrand {
        let mask = constants::sprite_masks::ENTITY | constants::sprite_masks::SHOOTABLE;
        let collider = sprite::Sprite::unit(
            sprite::CollisionShape::Square,
            position.xy().cast().unwrap(),
            layers::stage::FIREBRAND,
            point2(0.0, 0.0),
            vec2(0.0, 0.0),
            vec4(1.0, 1.0, 1.0, 1.0),
            mask,
        );
        Self {
            entity_id: 0,
            collider: Some(collider),
            pixels_per_unit: vec2(0.0, 0.0),
            time: 0.0,
            step: 0,
            input_state: FirebrandInputState::default(),
            character_state: CharacterState::new(position.xy(), num_lives_remaining),
            overlapping_sprites: HashSet::new(),
            contacting_sprites: HashSet::new(),
            vertical_velocity: 0.0,
            jump_time_remaining: 0.0,
            flight_countdown: FLIGHT_DURATION,
            wallgrab_jump_lateral_motion_countdown: 0.0,
            wallgrab_jump_dir: 0.0,
            cycle_animation_time_elapsed: None,
            in_water: false,
            injury_kickback_vel: 1.0,
            injury_countdown: 0.0,
            invulnerability_countdown: 0.0,
            last_shoot_time: 0.0,
            frozen: false,
            did_send_death_message: false,
            did_pass_through_exit_door: false,
        }
    }
}

impl Entity for Firebrand {
    fn init(&mut self, entity_id: u32, map: &map::Map, collision_space: &mut collision::Space) {
        self.entity_id = entity_id;
        self.pixels_per_unit = map.tileset.get_sprite_size().cast().unwrap();
        if let Some(ref mut collider) = self.collider {
            collider.entity_id = Some(entity_id);
            collision_space.add_dynamic_sprite(collider);
        }
    }

    fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        if self.input_state.process_keyboard(key, state) {
            true
        } else {
            match (key, state) {
                (VirtualKeyCode::Delete, ElementState::Pressed) => {
                    self.receive_injury(self.character_state.hit_points);
                    true
                }
                _ => false,
            }
        }
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
        game_state_peek: &GameStatePeek,
    ) {
        //
        // If we died, remove collision sprite and broadcast
        //

        if !self.character_state.alive {
            if !self.did_send_death_message {
                collision_space.remove_dynamic_sprite_with_entity_id(self.entity_id);
                message_dispatcher.broadcast(Event::FirebrandDied);
                self.did_send_death_message = true;
            }
            return;
        }

        let dt = dt.as_secs_f32();
        self.time += dt;
        self.step += 1;

        self.overlapping_sprites.clear();
        self.contacting_sprites.clear();

        if let ButtonState::Pressed = self.input_state.fire() {
            if !self.frozen {
                self.shoot_fireball(message_dispatcher);
            }
        }

        //
        //  No user input processing while in injury state
        //

        let can_process_jump_inputs = !self.frozen && !self.is_in_injury();

        //
        //  Handle jump button
        //

        if can_process_jump_inputs {
            match self.input_state.jump() {
                ButtonState::Pressed => match self.character_state.stance {
                    Stance::Standing => {
                        self.jump_time_remaining = JUMP_DURATION;
                        self.set_stance(Stance::InAir);
                    }
                    Stance::InAir => {
                        if self.in_water {
                            // firebrand can jump while in water, it actslike a ground-contacting reset.
                            self.jump_time_remaining = JUMP_DURATION;
                            self.flight_countdown = FLIGHT_DURATION;
                            self.set_stance(Stance::InAir);
                        } else if self.flight_countdown > 0.0 {
                            self.jump_time_remaining = 0.0;
                            self.set_stance(Stance::Flying);
                        }
                    }
                    Stance::Flying => {
                        self.set_stance(Stance::InAir);
                    }
                    Stance::WallHold(surface) => {
                        self.wallgrab_jump_lateral_motion_countdown =
                            WALLGRAB_JUMP_LATERAL_MOTION_DURATION;
                        self.jump_time_remaining = JUMP_DURATION;
                        self.wallgrab_jump_dir =
                            if surface.origin.x > self.character_state.position.x {
                                -1.0
                            } else {
                                1.0
                            };
                        self.set_stance(Stance::InAir);
                    }

                    Stance::Injury => {} // no-op during injury
                },
                ButtonState::Released => {
                    self.jump_time_remaining = 0.0;
                }
                _ => {}
            }
        }

        //
        //  Determine if the character is standing on a surface or in the air.
        //  This method probes downwards one step the farthest gravity would carry character.
        //  It returns the position of the character and whether they're in the air.
        //

        let (position, contacting_ground) = {
            if self.character_state.stance == Stance::Injury {
                (self.character_state.position, false)
            } else {
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

                let contacting_ground = footing_center.1.is_some()
                    || footing_right.1.is_some()
                    || footing_left.1.is_some();

                //
                //  If character just walked off a ledge start falling
                //

                if !contacting_ground
                    && self.character_state.stance != Stance::Flying
                    && !self.is_wallholding()
                    && !self.is_in_injury()
                {
                    self.set_stance(Stance::InAir);
                }

                if self.character_state.stance == Stance::Flying
                    || self.character_state.stance == Stance::InAir
                    || self.is_wallholding()
                {
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
        //  Apply character movement. Note, we inset the bounds by 1 px so firebrand doesn't
        //  contact offscreen elements.
        //

        let (position, wall_contact) = self.apply_lateral_movement(
            collision_space,
            position,
            dt,
            &game_state_peek
                .current_map_bounds
                .inset(vec2(2.0 / self.pixels_per_unit.x, 0.0)),
        );

        if self.character_state.stance != Stance::Injury {
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
                    self.set_stance(Stance::WallHold(*wall_contact));
                }
            }
        }

        //
        //  Final steps - update character position and if flying, apply the bob offset
        //

        self.character_state.position = position;

        //
        //  Track jump, flight, injury timed expirations countdowns
        //

        match self.character_state.stance {
            Stance::InAir => {
                if self.jump_time_remaining > 0.0 {
                    self.jump_time_remaining -= dt;
                }
                self.jump_time_remaining = self.jump_time_remaining.max(0.0);

                if self.wallgrab_jump_lateral_motion_countdown > 0.0 {
                    self.wallgrab_jump_lateral_motion_countdown -= dt;
                }

                if self.wallgrab_jump_lateral_motion_countdown < 0.0 {
                    self.wallgrab_jump_lateral_motion_countdown = 0.0;
                }
            }
            Stance::Flying => {
                // Apply flight bob cycle
                if self.flight_countdown > 0.0 {
                    let elapsed = FLIGHT_DURATION - self.flight_countdown;
                    let bob_cycle =
                        ((elapsed / FLIGHT_BOB_CYCLE_PERIOD) * 2.0 * PI - PI / 2.0).sin() * 0.5
                            + 0.5; // remap to [0,1]
                    let bob_offset = bob_cycle * FLIGHT_BOB_CYCLE_PIXELS_OFFSET as f32;
                    self.character_state.position_offset =
                        vec2(0.0, bob_offset / self.pixels_per_unit.y);
                }

                // Decrement remaining flight time
                self.flight_countdown -= dt;
                if self.flight_countdown <= 0.0 {
                    self.flight_countdown = 0.0;
                    self.set_stance(Stance::InAir);
                }
            }
            Stance::Injury => {
                if self.injury_countdown > 0.0 {
                    self.injury_countdown -= dt;
                }
                if self.injury_countdown <= 0.0 {
                    self.injury_countdown = 0.0;
                    self.set_stance(Stance::InAir);
                }
            }
            _ => {}
        }

        //
        //  Track countdowns which don't affect stance
        //

        self.invulnerability_countdown = (self.invulnerability_countdown - dt).max(0.0);

        //
        //  Determine if character is in water
        //

        self.in_water = self.is_in_water(collision_space, self.character_state.position);

        //
        //  Update character cycle and animation, and facing dir
        //

        if !self.frozen {
            self.character_state.cycle = self.update_character_cycle(dt);
            self.character_state.facing = self.character_facing();
        }

        //
        //  Test against dynamic sprites (e.g., enemies) in scene
        //

        collision_space.test_rect_against_dynamic_sprites(
            &self.character_state.position.xy(),
            &vec2(1.0, 1.0),
            COLLIDER | ENTITY,
            |sprite| {
                if sprite.entity_id != Some(self.entity_id) {
                    self.process_potential_collision_with(sprite);
                }
                false
            },
        );

        //
        //  Update our own collider in case other entities are probing for contacts
        //

        if let Some(ref mut collider) = self.collider {
            collider.origin.x = self.character_state.position.x;
            collider.origin.y = self.character_state.position.y;
            collision_space.update_dynamic_sprite(collider);
        }

        //
        //  Remove any sprites in the contacting set from the overlapping set.
        //

        for s in &self.contacting_sprites {
            self.overlapping_sprites.remove(s);
        }

        //
        //  Process contacts
        //

        self.process_contacts(message_dispatcher);

        //
        //  Update input state *after* all input has been processed.
        //

        self.input_state.update();

        //
        //  Send status update to game state
        //

        self.character_state.flight_time_remaining = self.flight_countdown;

        message_dispatcher.entity_to_global(
            self.entity_id,
            Event::FirebrandStatusChanged {
                status: self.character_state,
            },
        );
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        //
        //  Write state into uniform storage
        //

        {
            let (xscale, xoffset) = match self.character_state.facing {
                HorizontalDir::West => (-1.0, 1.0),
                HorizontalDir::East => (1.0, 0.0),
            };

            let z_offset = if self.did_pass_through_exit_door {
                layers::stage::EXIT - 1.0
            } else {
                layers::stage::FIREBRAND
            };

            uniforms
                .data
                .set_color(vec4(1.0, 1.0, 1.0, 1.0))
                .set_sprite_scale(vec2(xscale, 1.0))
                .set_model_position(point3(
                    self.character_state.position.x
                        + self.character_state.position_offset.x
                        + xoffset,
                    self.character_state.position.y + self.character_state.position_offset.y,
                    z_offset,
                ));
        }
    }

    fn remove_collider(&self, collision_space: &mut collision::Space) {
        if let Some(collider) = self.collider {
            collision_space.remove_dynamic_sprite(&collider);
        }
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::Firebrand
    }

    fn is_alive(&self) -> bool {
        // firebrand is always "alive" as far as the engine is concerned, but
        // once character_state.alive == false, we stop drawing and updating.
        true
    }

    fn should_draw(&self) -> bool {
        if !self.character_state.alive {
            false
        } else if self.invulnerability_countdown > 0.0 {
            if self.injury_countdown > 0.0 {
                // if playing injury stance cycle, we're visible
                true
            } else {
                // after injury cycle animation finishes, we blink until invuln period ends
                let elapsed = INVULNERABILITY_DURATION - self.invulnerability_countdown;
                let cycle = (elapsed / INVULNERABILITY_BLINK_PERIOD) as i32 % 2;
                cycle == 0
            }
        } else {
            // by default we're visible
            true
        }
    }

    fn position(&self) -> Point3<f32> {
        point3(
            self.character_state.position.x,
            self.character_state.position.y,
            layers::stage::FIREBRAND,
        )
    }

    fn sprite_name(&self) -> &str {
        "firebrand"
    }

    fn sprite_cycle(&self) -> &str {
        self.character_state.cycle
    }

    fn handle_message(&mut self, message: &Message) {
        match message.event {
            Event::DidShootFireball => {
                self.last_shoot_time = self.time;
            }
            Event::RaiseExitFloor => {
                // firebrand is frozen while the floor raises, until the exit door finishes opening.
                self.frozen = true;
            }
            Event::ExitDoorOpened => self.frozen = false,
            Event::HitByFireball {
                direction: _,
                damage,
            } => {
                self.receive_injury(damage);
            }
            Event::FirebrandContactedPowerUp { powerup_type } => {
                self.receive_powerup(powerup_type);
            }
            Event::FirebrandPassedThroughExitDoor => {
                self.did_pass_through_exit_door = true;
            }
            Event::FirebrandCreated {
                checkpoint: _,
                is_first_time,
            } => {
                println!(
                    "Firebrand[{}]::handle_message FirebrandCreated is_first_time: {}",
                    self.entity_id(),
                    is_first_time
                );
            }
            _ => {}
        }
    }

    fn overlapping_sprites(&self) -> Option<&HashSet<sprite::Sprite>> {
        Some(&self.overlapping_sprites)
    }

    fn contacting_sprites(&self) -> Option<&HashSet<sprite::Sprite>> {
        Some(&self.contacting_sprites)
    }
}

impl Firebrand {
    pub fn is_jumping(&self) -> bool {
        self.character_state.stance == Stance::InAir && self.jump_time_remaining > 0.0
    }

    pub fn is_flying(&self) -> bool {
        self.character_state.stance == Stance::Flying && self.flight_countdown > 0.0
    }

    pub fn is_wallholding(&self) -> bool {
        matches!(self.character_state.stance, Stance::WallHold(_))
    }

    pub fn is_in_injury(&self) -> bool {
        self.character_state.stance == Stance::Injury
    }

    pub fn is_invulnerable(&self) -> bool {
        self.invulnerability_countdown > 0.0
    }

    fn shoot_fireball(&mut self, message_dispatcher: &mut Dispatcher) {
        let origin = self.character_state.position + vec2(0.5, 0.7);
        message_dispatcher.entity_to_global(
            self.entity_id(),
            Event::TryShootFireball {
                origin,
                direction: self.character_facing(),
                velocity: FIREBALL_VELOCITY,
                damage: FIREBALL_PROJECTILE_DAMAGE,
            },
        );
    }

    fn process_contacts(&mut self, message_dispatcher: &mut Dispatcher) {
        let mut contact_damage = false;
        for s in &self.contacting_sprites {
            if s.mask & CONTACT_DAMAGE != 0 {
                contact_damage = true;
            }
            if let Some(entity_id) = s.entity_id {
                message_dispatcher.entity_to_entity(
                    self.entity_id(),
                    entity_id,
                    Event::FirebrandContact,
                );
            }
        }

        if contact_damage {
            self.receive_injury(CONTACT_DAMAGE_HIT_POINTS);
        }
    }

    fn set_stance(&mut self, new_stance: Stance) {
        // Discard any injuries while invulnerabile
        if new_stance == Stance::Injury && self.invulnerability_countdown > 0.0 {
            return;
        }

        if new_stance != self.character_state.stance {
            // println!(
            //     "Transition at {} (@{}) from {} -> {}",
            //     self.time, self.step, self.character_state.stance, new_stance
            // );

            // NOTE This is a useless match block, but is useful to set breakpoints for specific transitions
            // match self.character_state.stance {
            //     Stance::Standing => match new_stance {
            //         Stance::Standing => {}
            //         Stance::InAir => {}
            //         Stance::Flying => {}
            //         Stance::WallHold(_) => {}
            //         Stance::Injury => {}
            //     },
            //     Stance::InAir => match new_stance {
            //         Stance::Standing => {}
            //         Stance::InAir => {}
            //         Stance::Flying => {}
            //         Stance::WallHold(_) => {}
            //         Stance::Injury => {}
            //     },
            //     Stance::Flying => match new_stance {
            //         Stance::Standing => {}
            //         Stance::InAir => {}
            //         Stance::Flying => {}
            //         Stance::WallHold(_) => {}
            //         Stance::Injury => {}
            //     },
            //     Stance::WallHold(_) => match new_stance {
            //         Stance::Standing => {}
            //         Stance::InAir => {}
            //         Stance::Flying => {}
            //         Stance::WallHold(_) => {}
            //         Stance::Injury => {}
            //     },
            //     Stance::Injury => match new_stance {
            //         Stance::Standing => {}
            //         Stance::InAir => {}
            //         Stance::Flying => {}
            //         Stance::WallHold(_) => {}
            //         Stance::Injury => {}
            //     },
            // }

            self.injury_countdown = 0.0;

            match new_stance {
                // Flight time is reset whenever character touches ground or wallholds
                Stance::Standing | Stance::WallHold(_) => {
                    self.flight_countdown = FLIGHT_DURATION;
                }
                Stance::Injury => {
                    self.injury_countdown = INJURY_DURATION;
                    self.invulnerability_countdown = INVULNERABILITY_DURATION;

                    let sign = if self.is_wallholding() { -1.0 } else { 1.0 };
                    self.injury_kickback_vel = sign
                        * match self.character_facing() {
                            HorizontalDir::West => INJURY_KICKBACK_VEL,
                            HorizontalDir::East => -INJURY_KICKBACK_VEL,
                        };
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
    fn find_character_footing<'a>(
        &mut self,
        collision_space: &'a collision::Space,
        position: Point2<f32>,
        test_offset: Vector2<f32>,
        may_apply_correction: bool,
    ) -> (Point2<f32>, Option<&'a sprite::Sprite>) {
        let mut position = position;
        let mut tracking = None;

        // scan sprites beneath character
        let center = point2(
            (position.x + test_offset.x).round() as i32,
            (position.y + test_offset.y).round() as i32,
        );

        let below_center = point2(center.x, center.y - 1);
        let contacts_are_collision = !may_apply_correction;

        let can_collide_width = |p: &Point2<f32>, s: &sprite::Sprite| -> bool {
            // if character is more than 75% up a ratchet block consider it a collision
            !(s.mask & RATCHET != 0 && p.y < (s.top() - 0.25))
        };

        let sprite_size_px = self.pixels_per_unit.x;
        let inset_for_sprite = |s: &sprite::Sprite| -> f32 {
            if s.mask & CONTACT_DAMAGE != 0 {
                2.0 / sprite_size_px
            } else {
                0.0
            }
        };

        for test_point in [below_center, center].iter() {
            if let Some(s) = collision_space.get_sprite_at(*test_point, COLLIDER) {
                if can_collide_width(&position, s) {
                    match s.collision_shape {
                        CollisionShape::Square => {
                            if s.unit_rect_intersection(
                                &position,
                                inset_for_sprite(s),
                                contacts_are_collision,
                            ) {
                                self.process_potential_collision_with(s);
                                tracking = Some(s);
                                if may_apply_correction {
                                    position.y = s.origin.y + s.extent.y;
                                }
                            }
                        }
                        CollisionShape::NorthEast | CollisionShape::NorthWest => {
                            if let Some(intersection) = s.line_intersection(
                                &(position + vec2(0.5, 1.0)),
                                &(position + vec2(0.5, 0.0)),
                            ) {
                                self.process_potential_collision_with(s);
                                tracking = Some(s);
                                if may_apply_correction {
                                    position.y = intersection.y;
                                }
                            }
                        }
                        _ => (),
                    }
                    self.overlapping_sprites.insert(*s);
                }
            }
        }

        (position, tracking)
    }

    /// Moves character horizontally, based on the current left/right input state.
    /// Returns tuple of updated position, and an optional sprite representing the wall surface the
    /// character may have contacted. The character can collide with up to two sprites if on fractional
    /// y coord, so this returns the one closer to the character's y position)
    fn apply_lateral_movement<'a>(
        &mut self,
        collision_space: &'a collision::Space,
        position: Point2<f32>,
        dt: f32,
        map_bounds: &Bounds,
    ) -> (Point2<f32>, Option<&'a sprite::Sprite>) {
        // this is a no-op while wallholding or frozen
        if self.is_wallholding() || self.frozen {
            return (position, None);
        }

        let mask = COLLIDER;
        let probe_test = create_collision_probe_test(position);

        let mut delta_x =
            input_accumulator(self.input_state.move_left(), self.input_state.move_right()) as f32
                * WALK_SPEED
                * dt;

        // if character is on foot and shot fireball recently, we don't apply left/right motion
        if self.character_state.stance == Stance::Standing
            && self.time - self.last_shoot_time < FIREBALL_SHOOT_MOVEMENT_PAUSE_DURATION
        {
            delta_x = 0.0;
        }

        // walljump overrides user input vel birefly.
        if self.wallgrab_jump_lateral_motion_countdown > 0.0 {
            delta_x = WALLGRAB_JUMP_LATERAL_VEL
                * self.wallgrab_jump_lateral_motion_countdown
                * dt
                * self.wallgrab_jump_dir;
        }

        // injury overrides user input - during the kickback cycle the character moves in opposite direction
        // of their facing state, and for the remainder the character simply falls.
        if self.injury_countdown > 0.0 {
            delta_x = self.injury_kickback_vel * dt;
        }

        let mut contacted: Option<&sprite::Sprite> = None;

        //
        // Check if moving left or right would cause a collision, and adjust distance accordingly
        //

        if delta_x > 0.0 {
            match collision_space.probe_static_sprites(
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
                        self.process_potential_collision_with(&sprite);
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
                        self.process_potential_collision_with(&sprite_0);
                        self.process_potential_collision_with(&sprite_1);
                    }
                }
            }
        } else if delta_x < 0.0 {
            match collision_space.probe_static_sprites(
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
                        self.process_potential_collision_with(&sprite);
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
                        self.process_potential_collision_with(&sprite_0);
                        self.process_potential_collision_with(&sprite_1);
                    }
                }
            }
        }

        //
        //  Wallgrabs are dissallowed on the top-helf of a ledge (no tile above the contacted tile) and on sprites
        //  which deal contact damage.
        //

        if let Some(c) = contacted {
            if c.mask & CONTACT_DAMAGE != 0
                || (collision_space
                    .get_static_sprite_at(point2(c.origin.x as i32, c.origin.y as i32 + 1), mask)
                    .is_none()
                    && position.y > c.origin.y + (c.extent.y * 0.5))
            {
                contacted = None;
            }
        }

        //
        //  Clamp position to fit on stage
        //

        (
            point2(
                clamp(
                    position.x + delta_x,
                    map_bounds.origin.x,
                    map_bounds.origin.x + map_bounds.extent.x - 1.0,
                ),
                clamp(
                    position.y,
                    map_bounds.origin.y,
                    map_bounds.origin.y + map_bounds.extent.y - 1.0,
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
        collision_space: &collision::Space,
        position: Point2<f32>,
        dt: f32,
    ) -> Point2<f32> {
        // if we're frozen, this is a no-op
        if self.frozen {
            return position;
        }

        match self.character_state.stance {
            Stance::Standing | Stance::Flying | Stance::WallHold(_) => {
                if self.vertical_velocity.abs() != 0.0 {
                    self.vertical_velocity = 0.0;
                }
            }
            Stance::InAir | Stance::Injury => {
                //
                // During injury character is kicked upwards, and that overrides normal jump/gravity rules.
                //

                let mut should_apply_gravity = true;
                if self.injury_countdown > 0.0 {
                    self.vertical_velocity = INJURY_KICKBACK_VEL;
                    should_apply_gravity = false;
                } else if self.jump_time_remaining > 0.0 {
                    let elapsed = JUMP_DURATION - self.jump_time_remaining;
                    let jump_completion = elapsed / JUMP_DURATION;
                    self.vertical_velocity = lerp(jump_completion, -GRAVITY_VEL, 0.0);
                    should_apply_gravity = false;
                }

                if should_apply_gravity {
                    self.vertical_velocity = constants::apply_gravity(self.vertical_velocity, dt);
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
            let mask = COLLIDER;
            let probe_test = create_collision_probe_test(position);
            match collision_space.probe_static_sprites(
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
                        self.process_potential_collision_with(&sprite);
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
                        self.process_potential_collision_with(&sprite_0);
                        self.process_potential_collision_with(&sprite_1);
                    }
                }
            }
        }

        position + delta
    }

    /// If the sprite contacts our player's bounds, inserts into contacting_sprites, otherwise
    /// inserts into overlapping_sprites (which are useful for debugging potential contacts)
    fn process_potential_collision_with(&mut self, sprite: &sprite::Sprite) {
        if sprite.unit_rect_intersection(&self.position().xy(), 0.0, true) {
            self.contacting_sprites.insert(*sprite);
        } else {
            self.overlapping_sprites.insert(*sprite);
        }
    }

    fn update_character_cycle(&mut self, dt: f32) -> &'static str {
        // The character "walks" when in water, otherwise use the actual stance.
        let stance = if self.in_water {
            match self.character_state.stance {
                Stance::Standing | Stance::InAir | Stance::Flying => Stance::Standing,
                _ => self.character_state.stance,
            }
        } else {
            self.character_state.stance
        };

        let is_shooting = self.time - self.last_shoot_time < FIREBALL_CYCLE_DURATION;

        if self.cycle_animation_time_elapsed.is_none() {
            self.cycle_animation_time_elapsed = Some(0.0);
        }
        let elapsed = self.cycle_animation_time_elapsed.unwrap();
        self.cycle_animation_time_elapsed = Some(elapsed + dt);

        match stance {
            Stance::Standing => {
                if is_shooting {
                    CYCLE_SHOOT
                } else if self.input_state.move_left().is_active()
                    || self.input_state.move_right().is_active()
                {
                    let frame = ((elapsed / WALK_CYCLE_DURATION).floor() as i32) % 4;
                    match frame {
                        0 => CYCLE_WALK_0,
                        1 => CYCLE_WALK_1,
                        2 => CYCLE_WALK_0,
                        3 => CYCLE_WALK_2,
                        _ => unimplemented!("This shouldn't be reached"),
                    }
                } else {
                    self.cycle_animation_time_elapsed = None;
                    CYCLE_WALK_0
                }
            }
            Stance::InAir => {
                let frame = ((elapsed / JUMP_CYCLE_DURATION).floor() as i32) % 4;
                if is_shooting {
                    match frame {
                        0 => CYCLE_FLY_SHOOT_0,
                        1 => CYCLE_FLY_SHOOT_1,
                        2 => CYCLE_FLY_SHOOT_2,
                        3 => CYCLE_FLY_SHOOT_1,
                        _ => unimplemented!("This shouldn't be reached"),
                    }
                } else {
                    match frame {
                        0 => CYCLE_JUMP_0,
                        1 => CYCLE_JUMP_1,
                        2 => CYCLE_JUMP_2,
                        3 => CYCLE_JUMP_1,
                        _ => unimplemented!("This shouldn't be reached"),
                    }
                }
            }
            Stance::Flying => {
                let frame = ((elapsed / FLIGHT_CYCLE_DURATION).floor() as i32) % 4;
                if is_shooting {
                    match frame {
                        0 => CYCLE_FLY_SHOOT_0,
                        1 => CYCLE_FLY_SHOOT_1,
                        2 => CYCLE_FLY_SHOOT_2,
                        3 => CYCLE_FLY_SHOOT_1,
                        _ => unimplemented!("This shouldn't be reached"),
                    }
                } else {
                    match frame {
                        0 => CYCLE_FLY_0,
                        1 => CYCLE_FLY_1,
                        2 => CYCLE_FLY_2,
                        3 => CYCLE_FLY_1,
                        _ => unimplemented!("This shouldn't be reached"),
                    }
                }
            }
            Stance::WallHold(_) => {
                if is_shooting {
                    CYCLE_WALL_SHOOT
                } else {
                    CYCLE_WALL
                }
            }
            Stance::Injury => {
                let frame = ((elapsed / INJURY_CYCLE_DURATION).floor() as i32) % 4;
                match frame {
                    0 => CYCLE_INJURY_0,
                    1 => CYCLE_INJURY_1,
                    2 => CYCLE_INJURY_2,
                    3 => CYCLE_INJURY_3,
                    _ => unimplemented!("This shouldn't be reached"),
                }
            }
        }
    }

    fn character_facing(&self) -> HorizontalDir {
        match self.character_state.stance {
            Stance::Standing | Stance::InAir | Stance::Flying | Stance::Injury => {
                if self.input_state.move_left().is_active() {
                    HorizontalDir::West
                } else if self.input_state.move_right().is_active() {
                    HorizontalDir::East
                } else {
                    self.character_state.facing
                }
            }
            Stance::WallHold(attached_to) => {
                if attached_to.left() > self.character_state.position.x {
                    HorizontalDir::West
                } else {
                    HorizontalDir::East
                }
            }
        }
    }

    fn is_in_water(&self, collision_space: &collision::Space, position: Point2<f32>) -> bool {
        let mut in_water = false;
        collision_space.test_rect_against_static_sprites(
            &position,
            &vec2(1.0, 1.0),
            WATER,
            |_sprite| {
                in_water = true;
                true
            },
        );

        in_water
    }

    fn receive_powerup(&mut self, powerup_type: power_up::Type) {
        match powerup_type {
            super::power_up::Type::Vial => {
                self.character_state.num_vials += 1;
            }
            super::power_up::Type::Heart => {
                self.character_state.hit_points =
                    (self.character_state.hit_points + 1).min(HIT_POINTS);
            }
        }
    }

    fn receive_injury(&mut self, damage: u32) {
        if self.character_state.alive && !self.is_invulnerable() {
            self.character_state.hit_points -= self.character_state.hit_points.min(damage);

            if self.character_state.hit_points == 0 {
                self.character_state.alive = false;
            } else {
                self.set_stance(Stance::Injury);
            }
        }
    }
}
