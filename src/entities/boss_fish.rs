use cgmath::*;
use rand::{prelude::*, Rng};
use std::time::Duration;
use winit::event::{ElementState, VirtualKeyCode};

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, collision, rendering},
    state::{
        constants::{sprite_masks, ORIGINAL_VIEWPORT_TILES_WIDE},
        events::Event,
    },
    tileset,
};

use super::util::{Axis, CompassDir, Direction};

// --------------------------------------------------------------------------------------------------------------------

const ANIMATION_CYCLE_DURATION: f32 = 0.133;
const MOVEMENT_SPEED: f32 = 3.0 / 1.9; // units per second
const FIRESPRITE_MOVEMENT_SPEED: f32 = MOVEMENT_SPEED * 2.0;
const SUBMERGED_DURATION: f32 = 1.0;
const DEATH_ANIMATION_DURATION: f32 = 2.0;
const DEATH_BLINK_PERIOD: f32 = ANIMATION_CYCLE_DURATION;
const HIT_POINTS: i32 = 5;
const SPRITE_SIZE: Vector2<f32> = vec2(3.0, 3.0);
const SHOOT_DISTANCE: f32 = 3.0;
const SHOOT_CYCLE_PERIOD: f32 = 0.5;

#[derive(Debug, Clone, Copy)]
enum AttackPhase {
    Submerged { time_started: f32 },
    Raising,
    Attacking { target_x: f32 },
    Submerging,
}

// --------------------------------------------------------------------------------------------------------------------

pub struct BossFish {
    entity_id: u32,
    spawn_point_id: u32,
    collider: sprite::Sprite,
    position: Point3<f32>,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
    time: f32,
    rng: ThreadRng,
    attack_phase: AttackPhase,
    hit_points: i32,
    sent_encountered_message: bool,
    sent_death_message: bool,
    death_animation_countdown: f32,
    alive: bool,
    facing: Direction,
    arena_origin: Point2<f32>,
    arena_extent: Vector2<f32>,
    water_height: f32,
    should_launch_firesprites: bool,
    shoot_countdown: Option<f32>,
    post_shoot_countdown: Option<f32>,
}

impl Default for BossFish {
    fn default() -> Self {
        Self {
            entity_id: 0,
            spawn_point_id: 0,
            collider: sprite::Sprite::default(),
            position: point3(0.0, 0.0, 0.0),
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
            time: 0.0,
            rng: thread_rng(),
            attack_phase: AttackPhase::Submerged { time_started: 0.0 },
            hit_points: HIT_POINTS,
            sent_encountered_message: false,
            sent_death_message: false,
            alive: true,
            death_animation_countdown: DEATH_ANIMATION_DURATION,
            facing: Direction::West,
            arena_origin: point2(0.0, 0.0),
            arena_extent: vec2(0.0, 0.0),
            water_height: 0.0,
            should_launch_firesprites: false,
            shoot_countdown: None,
            post_shoot_countdown: None,
        }
    }
}

impl Entity for BossFish {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        tile: &tileset::Tile,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.spawn_point_id = sprite
            .entity_id
            .expect("Spawned entities expect to find a spawn point id from the sprite");

        self.position = sprite.origin;
        self.arena_extent = vec2(
            tile.float_property("arena_width"),
            tile.float_property("arena_height"),
        );
        self.arena_origin = sprite.origin.xy() - self.arena_extent / 2.0;
        self.water_height = tile.float_property("water_height");

        // Make copy of sprite for ourselves, we'll use it for collision testing
        // Note: The map sprite is our spawn point, so we need to overwrite the entity_id and mask
        self.collider = *sprite;
        self.collider.entity_id = Some(entity_id);
        self.collider.mask = sprite_masks::SHOOTABLE | sprite_masks::CONTACT_DAMAGE;
        self.collider.collision_shape = sprite::CollisionShape::Square;
    }

    fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        match (key, state) {
            (VirtualKeyCode::Delete, ElementState::Pressed) => {
                println!("BosFish[{}]::process_keyboard - SUICIDED", self.entity_id);
                self.hit_points = 0;
                true
            }
            _ => false,
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
        if !self.sent_encountered_message {
            message_dispatcher.entity_to_global(self.entity_id, Event::BossEncountered);
            self.sent_encountered_message = true;
        }

        let dt = dt.as_secs_f32();
        self.time += dt;

        if self.hit_points > 0 {
            //
            //  Update position and sprite
            //

            self.update_phase(dt, game_state_peek, message_dispatcher);
            self.update_sprite(collision_space);

            //
            //  Update sprite animation cycle
            //

            self.animation_cycle_tick_countdown -= dt;
            if self.animation_cycle_tick_countdown <= 0.0 {
                self.animation_cycle_tick_countdown += ANIMATION_CYCLE_DURATION;
                self.animation_cycle_tick += 1;
            }
        } else {
            collision_space.remove_dynamic_sprite(&self.collider);

            // countdown our death animation, before actually terminating
            if self.death_animation_countdown > 0.0 {
                self.death_animation_countdown -= dt;
                if self.death_animation_countdown < 0.0 {
                    // Send the defeat message to clear stage and kick off the ending changes to the level
                    message_dispatcher.entity_to_global(self.entity_id, Event::BossDefeated);
                    self.alive = false;
                }
            }
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        let (xscale, xoffset) = match self.facing {
            Direction::East => (1.0, 0.0),
            Direction::West => (-1.0, 1.0),
        };

        let alpha = if self.hit_points > 0 {
            1.0
        } else {
            let blink_phase = ((DEATH_ANIMATION_DURATION - self.death_animation_countdown)
                / DEATH_BLINK_PERIOD) as i32;
            if blink_phase % 2 == 0 {
                1.0
            } else {
                0.0
            }
        };

        uniforms
            .data
            .set_color(vec4(1.0, 1.0, 1.0, alpha))
            .set_model_position(self.position + vec3(xoffset, 0.0, 0.0))
            .set_sprite_scale(vec2(xscale, 1.0));
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::BossFish
    }

    fn is_alive(&self) -> bool {
        self.alive
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "boss_fish"
    }

    fn sprite_cycle(&self) -> &str {
        // Determine the shoot cycle from our countdowns
        let shoot_cycle = if let Some(countdown) = self.shoot_countdown {
            let t = SHOOT_CYCLE_PERIOD - countdown;
            1 + (t / (SHOOT_CYCLE_PERIOD * 0.5)) as i32
        } else if let Some(countdown) = self.post_shoot_countdown {
            let t = countdown;
            1 + (t / (SHOOT_CYCLE_PERIOD * 0.5)) as i32
        } else {
            0
        };

        if self.animation_cycle_tick % 2 == 0 {
            match shoot_cycle {
                1 => "a_1",
                2 => "a_2",
                _ => "a_0",
            }
        } else {
            match shoot_cycle {
                1 => "b_1",
                2 => "b_2",
                _ => "b_0",
            }
        }
    }

    fn handle_message(&mut self, message: &Message) {
        if let Event::HitByFireball { direction: _ } = message.event {
            self.hit_points = (self.hit_points - 1).max(0);
        }
    }
}

impl BossFish {
    fn update_phase(
        &mut self,
        dt: f32,
        game_state_peek: &GameStatePeek,
        message_dispatcher: &mut Dispatcher,
    ) {
        let distance_to_player = (game_state_peek.player_position.x - self.position.x).abs();

        match self.attack_phase {
            AttackPhase::Submerged { time_started } => {
                self.position.y = self.submersion_depth();

                if self.time - time_started > SUBMERGED_DURATION {
                    // move from Submerged to Raising. Pick an emergence point that is half the original
                    // viewport width away, with a lowish probability of being right under player.
                    let max_dist = ORIGINAL_VIEWPORT_TILES_WIDE as f32 / 2.0;
                    let mut dist = if self.rng.gen_bool(0.5) {
                        max_dist
                    } else {
                        -max_dist
                    };
                    if self.rng.gen_bool(0.2) {
                        dist = 0.0
                    };
                    let x = (game_state_peek.player_position.x + dist)
                        .max(self.arena_origin.x + 2.0)
                        .min(self.arena_origin.x + self.arena_extent.x - 3.0);
                    self.position.x = x;
                    self.position.y = self.arena_origin.y - self.water_height - SPRITE_SIZE.y;
                    self.set_attack_phase(AttackPhase::Raising);
                }
            }
            AttackPhase::Raising => {
                self.facing = if game_state_peek.player_position.x - self.position.x > 0.0 {
                    Direction::East
                } else {
                    Direction::West
                };

                // Raise self; when reaching attack height, transition to attack
                self.position.y += MOVEMENT_SPEED * dt;
                if self.position.y >= game_state_peek.player_position.y {
                    self.position.y = game_state_peek.player_position.y;
                    self.should_launch_firesprites = self.rng.gen_bool(0.5);
                    self.set_attack_phase(AttackPhase::Attacking {
                        target_x: game_state_peek.player_position.x,
                    });
                }
            }
            AttackPhase::Attacking { target_x } => {
                // Don't shoot until we're within a threshold of player
                if self.should_launch_firesprites && distance_to_player < SHOOT_DISTANCE {
                    self.shoot_countdown = Some(SHOOT_CYCLE_PERIOD);
                    self.should_launch_firesprites = false;
                }

                // Advance towards player until we reach the target_x, then start submersion
                let done_advancing = if target_x < self.position.x {
                    self.position.x -= MOVEMENT_SPEED * dt;
                    self.position.x <= target_x
                } else {
                    self.position.x += MOVEMENT_SPEED * dt;
                    self.position.x >= target_x
                };
                if done_advancing {
                    self.set_attack_phase(AttackPhase::Submerging);
                }
            }
            AttackPhase::Submerging => {
                // Submerge until we reach target depth, then switch to waiting submersion phase
                self.position.y -= MOVEMENT_SPEED * dt;
                if self.position.y < self.submersion_depth() {
                    self.set_attack_phase(AttackPhase::Submerged {
                        time_started: self.time,
                    });
                }
            }
        }

        if let Some(mut countdown) = self.shoot_countdown {
            if countdown > 0.0 {
                countdown -= dt;
                self.shoot_countdown = Some(countdown);
                if countdown <= 0.0 {
                    self.shoot_countdown = None;
                    self.post_shoot_countdown = Some(SHOOT_CYCLE_PERIOD);

                    let offset = vec2(0.25, 0.25);
                    let dir = CompassDir::new(game_state_peek.player_position - self.position.xy());
                    match dir {
                        CompassDir::North | CompassDir::South => { // no-op
                        }
                        _ => {
                            for (dir, offset) in
                                [(dir, offset), (dir.mirrored(Axis::Horizontal), -offset)].iter()
                            {
                                message_dispatcher.entity_to_global(
                                    self.entity_id,
                                    Event::ShootFiresprite {
                                        position: self.position.xy() + offset,
                                        dir: dir.to_dir(),
                                        velocity: FIRESPRITE_MOVEMENT_SPEED,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        } else if let Some(mut countdown) = self.post_shoot_countdown {
            if countdown > 0.0 {
                countdown -= dt;
                self.post_shoot_countdown = Some(countdown);
                if countdown < 0.0 {
                    self.post_shoot_countdown = None;
                }
            }
        }
    }

    fn set_attack_phase(&mut self, new_phase: AttackPhase) {
        println!(
            "BossFish::set_attack_phase time: {} old_phase: {:?} -> new_phase {:?}",
            self.time, self.attack_phase, new_phase
        );
        self.attack_phase = new_phase;
    }

    fn update_sprite(&mut self, collision_space: &mut collision::Space) {
        // sprite is 3x3 with root centered at bottom
        self.collider.origin.x = self.position.x - 1.0;
        self.collider.origin.y = self.position.y;
        self.collider.extent = SPRITE_SIZE;
        collision_space.update_dynamic_sprite(&self.collider);
    }

    fn submersion_depth(&self) -> f32 {
        self.arena_origin.y - self.water_height - SPRITE_SIZE.y
    }
}
