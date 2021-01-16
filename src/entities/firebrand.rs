use std::{collections::HashSet, f32::consts::PI, fmt::Display, time::Duration};

use cgmath::*;
use winit::event::{ElementState, VirtualKeyCode};

use crate::{
    constants::{sprite_masks::*, GRAVITY_VEL},
    entities::fireball::Direction,
    entity::{Dispatcher, Entity, Event, Message},
    map,
    sprite::{self, collision, rendering},
    tileset,
};

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

// ---------------------------------------------------------------------------------------------------------------------

pub fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}

pub fn clamp(v: f32, min: f32, max: f32) -> f32 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

fn create_collision_probe_test(position: Point2<f32>) -> impl Fn(f32, &sprite::Sprite) -> bool {
    move |_dist: f32, sprite: &sprite::Sprite| -> bool {
        if position.y < sprite.top() && sprite.mask & RATCHET != 0 {
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

    // The current display cycle of the character, will be one of the CYCLE_* constants.
    pub cycle: &'static str,

    // the character's current stance state
    pub stance: Stance,

    // the direction the character is currently facing
    pub facing: Facing,
}

impl CharacterState {
    fn new(position: Point2<f32>) -> Self {
        CharacterState {
            position: position,
            position_offset: Zero::zero(),
            cycle: CYCLE_DEFAULT,
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

// ---------------------------------------------------------------------------------------------------------------------

pub struct Firebrand {
    entity_id: u32,
    sprite: Option<sprite::Sprite>,
    sprite_size_px: Vector2<f32>,

    time: f32,
    step: usize,
    input_state: InputState,
    character_state: CharacterState,

    // sprites the character is overlapping and might collide with
    pub overlapping_sprites: HashSet<sprite::Sprite>,

    // sprites the character is contacting
    pub contacting_sprites: HashSet<sprite::Sprite>,

    vertical_velocity: f32,
    jump_time_remaining: f32,
    flight_countdown: f32,
    wallgrab_jump_lateral_motion_countdown: f32,
    wallgrab_jump_dir: f32, // -1 for left, +1 for right
    map_origin: Point2<f32>,
    map_extent: Vector2<f32>,
    cycle_animation_time_elapsed: Option<f32>,
    in_water: bool,
    injury_kickback_vel: f32,
    injury_countdown: f32,
    invulnerability_countdown: f32,
    last_shoot_time: f32,
}

impl Default for Firebrand {
    fn default() -> Self {
        Self {
            entity_id: 0,
            sprite: None,
            sprite_size_px: vec2(0.0, 0.0),
            time: 0.0,
            step: 0,
            input_state: Default::default(),
            character_state: CharacterState::new(point2(0.0, 0.0)),
            overlapping_sprites: HashSet::new(),
            contacting_sprites: HashSet::new(),
            vertical_velocity: 0.0,
            jump_time_remaining: 0.0,
            flight_countdown: FLIGHT_DURATION,
            wallgrab_jump_lateral_motion_countdown: 0.0,
            wallgrab_jump_dir: 0.0,
            map_origin: point2(0.0, 0.0),
            map_extent: vec2(0.0, 0.0),
            cycle_animation_time_elapsed: None,
            in_water: false,
            injury_kickback_vel: 1.0,
            injury_countdown: 0.0,
            invulnerability_countdown: 0.0,
            last_shoot_time: 0.0,
        }
    }
}

impl Entity for Firebrand {
    fn init_from_map_sprite(
        &mut self,
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        map: &map::Map,
        _collision_space: &mut collision::Space,
    ) {
        self.entity_id = sprite
            .entity_id
            .expect("Entity sprites should have an entity_id");
        self.sprite = Some(*sprite);
        self.sprite_size_px = vec2(
            map.tileset.tile_width as f32,
            map.tileset.tile_height as f32,
        );
        self.map_origin = map.bounds().0.cast().unwrap();
        self.map_extent = map.bounds().1.cast().unwrap();
        self.character_state.position = sprite.origin.xy();
    }

    fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        self.input_state.process_keyboard(key, state)
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
    ) {
        self.overlapping_sprites.clear();
        self.contacting_sprites.clear();

        let dt = dt.as_secs_f32();
        self.time += dt;
        self.step += 1;

        match self.input_state.fire {
            ButtonState::Pressed => self.shoot_fireball(message_dispatcher),
            _ => {}
        }

        //
        //  No user input processing while in injury state
        //

        let can_process_jump_inputs = !self.is_in_injury();

        //
        //  Handle jump button
        //

        if can_process_jump_inputs {
            match self.input_state.jump {
                ButtonState::Pressed => match self.character_state.stance {
                    Stance::Standing => {
                        self.jump_time_remaining = JUMP_DURATION;
                        self.set_stance(Stance::InAir);
                    }
                    Stance::InAir => {
                        if self.flight_countdown > 0.0 {
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
                    || (self.character_state.stance == Stance::InAir
                        && self.vertical_velocity > 0.0)
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
                    self.set_stance(Stance::WallHold(wall_contact));
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
                        vec2(0.0, bob_offset / self.sprite_size_px.y);
                }

                // Decrement remaining flight time
                self.flight_countdown = self.flight_countdown - dt;
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

        self.character_state.cycle = self.update_character_cycle(dt);
        self.character_state.facing = self.character_facing();

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
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        //
        //  Write state into uniform storage
        //

        {
            let (xscale, xoffset) = match self.character_state.facing {
                Facing::Left => (-1.0, 1.0),
                Facing::Right => (1.0, 0.0),
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
                    0.5,
                ));
        }
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::Firebrand
    }

    fn is_alive(&self) -> bool {
        true
    }

    fn should_draw(&self) -> bool {
        if self.invulnerability_countdown > 0.0 {
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
            self.sprite.unwrap().origin.z,
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
        match self.character_state.stance {
            Stance::WallHold(_) => true,
            _ => false,
        }
    }

    pub fn is_in_injury(&self) -> bool {
        self.character_state.stance == Stance::Injury
    }

    pub fn is_invulnerable(&self) -> bool {
        self.invulnerability_countdown > 0.0
    }

    fn shoot_fireball(&mut self, message_dispatcher: &mut Dispatcher) {
        let origin = self.character_state.position + vec2(0.5, 0.7);
        let direction = match self.character_facing() {
            Facing::Left => Direction::West,
            Facing::Right => Direction::East,
        };
        message_dispatcher.enqueue(Message::entity_to_global(
            self.entity_id(),
            Event::TryShootFireball {
                origin,
                direction,
                velocity: FIREBALL_VELOCITY,
            },
        ));
    }

    fn process_contacts(&mut self, message_dispatcher: &mut Dispatcher) {
        let mut contact_damage = false;
        for s in &self.contacting_sprites {
            if s.mask & CONTACT_DAMAGE != 0 {
                contact_damage = true;
            }
            if let Some(entity_id) = s.entity_id {
                message_dispatcher.enqueue(Message::entity_to_entity(
                    self.entity_id(),
                    entity_id,
                    Event::CharacterContact,
                ));
            }
        }

        if contact_damage {
            self.set_stance(Stance::Injury);
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
            match self.character_state.stance {
                Stance::Standing => match new_stance {
                    Stance::Standing => {}
                    Stance::InAir => {}
                    Stance::Flying => {}
                    Stance::WallHold(_) => {}
                    Stance::Injury => {}
                },
                Stance::InAir => match new_stance {
                    Stance::Standing => {}
                    Stance::InAir => {}
                    Stance::Flying => {}
                    Stance::WallHold(_) => {}
                    Stance::Injury => {}
                },
                Stance::Flying => match new_stance {
                    Stance::Standing => {}
                    Stance::InAir => {}
                    Stance::Flying => {}
                    Stance::WallHold(_) => {}
                    Stance::Injury => {}
                },
                Stance::WallHold(_) => match new_stance {
                    Stance::Standing => {}
                    Stance::InAir => {}
                    Stance::Flying => {}
                    Stance::WallHold(_) => {}
                    Stance::Injury => {}
                },
                Stance::Injury => match new_stance {
                    Stance::Standing => {}
                    Stance::InAir => {}
                    Stance::Flying => {}
                    Stance::WallHold(_) => {}
                    Stance::Injury => {}
                },
            }

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
                            Facing::Left => INJURY_KICKBACK_VEL,
                            Facing::Right => -INJURY_KICKBACK_VEL,
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
    fn find_character_footing(
        &mut self,
        collision_space: &collision::Space,
        position: Point2<f32>,
        test_offset: Vector2<f32>,
        may_apply_correction: bool,
    ) -> (Point2<f32>, Option<sprite::Sprite>) {
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
            if s.mask & RATCHET != 0 && p.y < (s.top() - 0.25) {
                false
            } else {
                true
            }
        };

        let sprite_size_px = self.sprite_size_px.x;
        let inset_for_sprite = |s: &sprite::Sprite| -> f32 {
            if s.mask & CONTACT_DAMAGE != 0 {
                2.0 / sprite_size_px
            } else {
                0.0
            }
        };

        for test_point in [below_center, center].iter() {
            use crate::sprite::core::CollisionShape;

            if let Some(s) = collision_space.get_sprite_at(*test_point, COLLIDER) {
                if can_collide_width(&position, &s) {
                    match s.collision_shape {
                        CollisionShape::Square => {
                            if s.unit_rect_intersection(
                                &position,
                                inset_for_sprite(&s),
                                contacts_are_collision,
                            ) {
                                self.handle_collision_with(&s);
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
        collision_space: &collision::Space,
        position: Point2<f32>,
        dt: f32,
    ) -> (Point2<f32>, Option<sprite::Sprite>) {
        use collision::{ProbeDir, ProbeResult};

        // this is a no-op while wallholding
        if self.is_wallholding() {
            return (position, None);
        }

        let mask = COLLIDER;
        let probe_test = create_collision_probe_test(position);

        let mut delta_x =
            input_accumulator(self.input_state.move_left, self.input_state.move_right)
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

        let mut contacted: Option<sprite::Sprite> = None;

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
        //  Wallgrabs are dissallowed on the top-helf of a ledge (no tile above the contacted tile) and on sprites
        //  which deal contact damage.
        //

        if let Some(c) = contacted {
            if c.mask & CONTACT_DAMAGE != 0 {
                contacted = None;
            } else if collision_space
                .get_sprite_at(point2(c.origin.x as i32, c.origin.y as i32 + 1), mask)
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
            point2(
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
        collision_space: &collision::Space,
        position: Point2<f32>,
        dt: f32,
    ) -> Point2<f32> {
        use collision::{ProbeDir, ProbeResult};

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
                    self.vertical_velocity =
                        crate::constants::apply_gravity(self.vertical_velocity, dt);
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
    fn handle_collision_with(&mut self, sprite: &sprite::Sprite) {
        self.contacting_sprites.insert(*sprite);
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
                } else if self.input_state.move_left.is_active()
                    || self.input_state.move_right.is_active()
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

    fn character_facing(&self) -> Facing {
        match self.character_state.stance {
            Stance::Standing | Stance::InAir | Stance::Flying | Stance::Injury => {
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

    fn is_in_water(&self, collision_space: &collision::Space, position: Point2<f32>) -> bool {
        let a = point2(position.x.floor() as i32, position.y.floor() as i32);
        let b = point2(a.x + 1, a.y);
        let c = point2(a.x, a.y + 1);
        let d = point2(a.x + 1, a.y + 1);

        for p in [a, b, c, d].iter() {
            if collision_space.get_sprite_at(*p, WATER).is_some() {
                return true;
            }
        }
        return false;
    }
}
