use cgmath::*;
use rand::{prelude::*, Rng};
use std::time::Duration;

use crate::{
    audio, collision,
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, rendering},
    state::{
        constants::{layers, sprite_masks, ORIGINAL_VIEWPORT_TILES_WIDE},
        events::Event,
    },
    tileset,
    util::Bounds,
};

use super::util::{Axis, CompassDir, HorizontalDir};

// --------------------------------------------------------------------------------------------------------------------

const ANIMATION_CYCLE_DURATION: f32 = 0.133;
const MOVEMENT_SPEED: f32 = 3.0 / 1.9; // units per second
const FIRESPRITE_MOVEMENT_SPEED: f32 = MOVEMENT_SPEED * 2.0;
const SUBMERGED_DURATION: f32 = 1.0;
const DEATH_ANIMATION_DURATION: f32 = 2.0;
const INJURY_BLINK_PERIOD: f32 = 0.1;
const HIT_POINTS: i32 = 12;
const SPRITE_SIZE: Vector2<f32> = vec2(2.0, 2.5);
const SHOOT_DISTANCE: f32 = 5.0;
const SHOOT_CYCLE_PERIOD: f32 = 0.5;
const INJURY_FLASH_DURATION: f32 = 4.0 * INJURY_BLINK_PERIOD;

const FIRESPRITE_PROJECTILE_DAMAGE: u32 = 1;

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
    collider_id: Option<u32>,
    position: Point3<f32>,
    active: bool,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
    time: f32,
    rng: ThreadRng,
    attack_phase: AttackPhase,
    hit_points: i32,
    sent_defeated_message: bool,
    death_animation_countdown: f32,
    alive: bool,
    facing: HorizontalDir,
    arena_origin: Point2<f32>,
    arena_extent: Vector2<f32>,
    water_height: f32,
    should_launch_firesprites: bool,
    shoot_countdown: Option<f32>,
    post_shoot_countdown: Option<f32>,
    injury_flash_countdown: Option<f32>,
    sound_to_play: Option<audio::Sounds>,
}

impl Default for BossFish {
    fn default() -> Self {
        Self {
            entity_id: 0,
            spawn_point_id: 0,
            collider_id: None,
            position: point3(0.0, 0.0, 0.0),
            active: false, // waits for Event::QueryBossFightMayStart
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
            time: 0.0,
            rng: thread_rng(),
            attack_phase: AttackPhase::Submerged { time_started: 0.0 },
            hit_points: HIT_POINTS,
            sent_defeated_message: false,
            alive: true,
            death_animation_countdown: DEATH_ANIMATION_DURATION,
            facing: HorizontalDir::West,
            arena_origin: point2(0.0, 0.0),
            arena_extent: vec2(0.0, 0.0),
            water_height: 0.0,
            should_launch_firesprites: false,
            shoot_countdown: None,
            post_shoot_countdown: None,
            injury_flash_countdown: None,
            sound_to_play: None,
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
        collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.spawn_point_id = sprite
            .entity_id
            .expect("Spawned entities expect to find a spawn point id from the sprite");

        self.position = point3(sprite.origin.x, sprite.origin.y, layers::stage::ENTITIES);
        self.arena_extent = vec2(
            tile.float_property("arena_width"),
            tile.float_property("arena_height"),
        );
        self.arena_origin = sprite.origin.xy() - self.arena_extent / 2.0;
        self.water_height = tile.float_property("water_height");

        // Create collider
        let collider = collision::Collider::new_dynamic(
            self.collider_bounds(),
            entity_id,
            collision::Shape::Square,
            sprite_masks::ENTITY | sprite_masks::SHOOTABLE | sprite_masks::CONTACT_DAMAGE,
        );
        self.collider_id = Some(collision_space.add_collider(collider));
    }

    fn process_keyboard(
        &mut self,
        key: winit::keyboard::KeyCode,
        state: winit::event::ElementState,
    ) -> bool {
        match (key, state) {
            (winit::keyboard::KeyCode::F12, winit::event::ElementState::Pressed) => {
                println!("\n\nBOSSFISH SUICIDE\n\n");
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
        audio: &mut audio::Audio,
        message_dispatcher: &mut Dispatcher,
        game_state_peek: &GameStatePeek,
    ) {
        if !self.active {
            message_dispatcher.entity_to_global(self.entity_id, Event::QueryBossFightMayStart);
            return;
        }

        let dt = dt.as_secs_f32();
        self.time += dt;

        if self.hit_points > 0 {
            //
            //  Update position and sprite
            //

            self.update_phase(dt, game_state_peek, message_dispatcher);

            if let Some(id) = self.collider_id {
                collision_space.update_collider_position(id, self.collider_bounds().origin);
            }

            //
            //  Update sprite animation cycle
            //

            self.animation_cycle_tick_countdown -= dt;
            if self.animation_cycle_tick_countdown <= 0.0 {
                self.animation_cycle_tick_countdown += ANIMATION_CYCLE_DURATION;
                self.animation_cycle_tick += 1;
            }
        } else {
            if let Some(id) = self.collider_id {
                collision_space.deactivate_collider(id);
            }

            if !self.sent_defeated_message {
                message_dispatcher.broadcast(Event::BossDefeated);
                self.sent_defeated_message = true;
            }

            // countdown our death animation, before actually terminating
            if self.death_animation_countdown > 0.0 {
                self.death_animation_countdown -= dt;
                if self.death_animation_countdown < 0.0 {
                    // Send the death message to clear stage and kick off the ending changes to the level
                    message_dispatcher.broadcast(Event::BossDied);
                    self.alive = false;
                }
            }
        }

        if let Some(injury_flash_countdown) = self.injury_flash_countdown {
            let injury_flash_countdown = injury_flash_countdown - dt;
            if injury_flash_countdown >= 0.0 {
                self.injury_flash_countdown = Some(injury_flash_countdown);
            } else {
                self.injury_flash_countdown = None;
            }
        }

        if let Some(sound) = self.sound_to_play {
            audio.play_sound(sound);
            self.sound_to_play = None;
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        let (xscale, xoffset) = match self.facing {
            HorizontalDir::East => (1.0, 0.0),
            HorizontalDir::West => (-1.0, 1.0),
        };

        let alpha = if self.active {
            if self.hit_points > 0 {
                if let Some(injury_flash_countdown) = self.injury_flash_countdown {
                    let blink_phase = ((INJURY_FLASH_DURATION - injury_flash_countdown)
                        / INJURY_BLINK_PERIOD) as i32;
                    if blink_phase % 2 == 0 {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    1.0
                }
            } else {
                let blink_phase = ((DEATH_ANIMATION_DURATION - self.death_animation_countdown)
                    / INJURY_BLINK_PERIOD) as i32;
                if blink_phase % 2 == 0 {
                    1.0
                } else {
                    0.0
                }
            }
        } else {
            0.0
        };

        uniforms
            .data
            .set_color(vec4(1.0, 1.0, 1.0, alpha))
            .set_model_position(self.position + vec3(xoffset, 0.0, 0.0))
            .set_sprite_scale(vec2(xscale, 1.0));
    }

    fn deactivate_collider(&mut self, collision_space: &mut collision::Space) {
        if let Some(id) = self.collider_id {
            collision_space.deactivate_collider(id);
        }
        self.collider_id = None
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

    fn should_draw(&self) -> bool {
        self.active
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
        match message.event {
            Event::HitByFireball {
                direction: _,
                damage,
            } => {
                self.hit_points = (self.hit_points - (damage as i32)).max(0);
                self.injury_flash_countdown = Some(INJURY_FLASH_DURATION);
                self.sound_to_play = Some(audio::Sounds::BossInjured);
            }
            Event::BossFightMayStart => {
                println!(
                    "BossFish[{}]::handle_message - BossFightMayStart",
                    self.entity_id()
                );
                self.active = true;
            }
            _ => {}
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
                    let dist = match self.rng.gen_range(0..10) {
                        0..=3 => max_dist,
                        4..=7 => -max_dist,
                        _ => 0.0,
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
                    HorizontalDir::East
                } else {
                    HorizontalDir::West
                };

                // Raise self; when reaching attack height, transition to attack
                self.position.y += MOVEMENT_SPEED * dt;
                if self.position.y >= game_state_peek.player_position.y {
                    self.position.y = game_state_peek.player_position.y;
                    self.should_launch_firesprites = self.rng.gen_bool(0.75);
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
                                        damage: FIRESPRITE_PROJECTILE_DAMAGE,
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
        // println!(
        //     "BossFish::set_attack_phase time: {} old_phase: {:?} -> new_phase {:?}",
        //     self.time, self.attack_phase, new_phase
        // );
        self.attack_phase = new_phase;
    }

    fn collider_bounds(&mut self) -> Bounds {
        Bounds::new(self.position.xy() - vec2(0.5, 0.0), SPRITE_SIZE)
    }

    fn submersion_depth(&self) -> f32 {
        self.arena_origin.y - self.water_height - SPRITE_SIZE.y
    }
}
