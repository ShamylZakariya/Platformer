use std::time::Duration;

use cgmath::*;

use crate::{
    audio, collision,
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::rendering,
    state::constants::layers,
    util::Bounds,
};

use super::util::{CompassDir, HorizontalDir};

// ---------------------------------------------------------------------------------------------------------------------

enum Mode {
    EnemyDeath,
    FirebrandDeath,
}

const ENEMY_CYCLE_DURATION: f32 = 0.133;
const ENEMY_VEL: f32 = 1.5 / 0.4;
const FIREBRAND_CYCLE_DURATION: f32 = 0.133;
const FIREBRAND_VEL: f32 = 1.5 / 0.4;

// ---------------------------------------------------------------------------------------------------------------------

pub struct DeathAnimation {
    entity_id: u32,
    position: Point3<f32>,
    direction: CompassDir,
    alive: bool,
    time: f32,
    animation_frame: i32,
    mode: Mode,
}

impl DeathAnimation {
    pub fn new_firebrand_death(position: Point2<f32>, direction: CompassDir) -> Self {
        Self {
            entity_id: 0,
            position: point3(position.x, position.y, layers::stage::FOREGROUND),
            direction,
            alive: true,
            time: 0.0,
            animation_frame: 0,
            mode: Mode::FirebrandDeath,
        }
    }

    pub fn new_enemy_death(position: Point2<f32>, direction: HorizontalDir) -> Self {
        Self {
            entity_id: 0,
            position: point3(position.x, position.y, layers::stage::FOREGROUND),
            direction: CompassDir::from(direction),
            alive: true,
            time: 0.0,
            animation_frame: 0,
            mode: Mode::EnemyDeath,
        }
    }

    fn velocity(&self) -> f32 {
        match self.mode {
            Mode::EnemyDeath => ENEMY_VEL,
            Mode::FirebrandDeath => FIREBRAND_VEL,
        }
    }

    fn num_animation_frames(&self) -> i32 {
        match self.mode {
            Mode::EnemyDeath => 4,
            Mode::FirebrandDeath => 3,
        }
    }

    fn cycle_duration(&self) -> f32 {
        match self.mode {
            Mode::EnemyDeath => ENEMY_CYCLE_DURATION,
            Mode::FirebrandDeath => FIREBRAND_CYCLE_DURATION,
        }
    }
}

impl Entity for DeathAnimation {
    fn init(&mut self, entity_id: u32, _map: &map::Map, _collision_space: &mut collision::Space) {
        self.entity_id = entity_id;
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
        _audio: &mut audio::Audio,
        _message_dispatcher: &mut Dispatcher,
        _game_state_peek: &GameStatePeek,
    ) {
        let dt = dt.as_secs_f32();
        self.time += dt;
        self.animation_frame = (self.time / self.cycle_duration()).floor() as i32;
        if self.animation_frame > self.num_animation_frames() {
            // Entity death animations run through a single cycle only
            if let Mode::EnemyDeath = self.mode {
                self.alive = false;
            }
        }

        let next_position = self.position.xy() + (self.direction.to_dir() * self.velocity() * dt);
        self.position.x = next_position.x;
        self.position.y = next_position.y;
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        if let Mode::EnemyDeath = self.mode {
            let (xscale, xoffset) = match self.direction {
                CompassDir::East => (-1.0, 1.0),
                _ => (1.0, 0.0),
            };

            uniforms
                .data
                .set_model_position(self.position + vec3(xoffset, 0.0, 0.0))
                .set_sprite_scale(vec2(xscale, 1.0));
        } else {
            uniforms.data.set_model_position(self.position);
        }
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::DeathAnimation
    }

    fn is_alive(&self) -> bool {
        self.alive
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn bounds(&self) -> Bounds {
        Bounds::new(self.position().xy() - vec2(0.5, 0.5), vec2(1.0, 1.0))
    }

    fn sprite_name(&self) -> &str {
        match self.mode {
            Mode::EnemyDeath => "death",
            Mode::FirebrandDeath => "firebrand_death",
        }
    }

    fn sprite_cycle(&self) -> &str {
        match self.mode {
            Mode::EnemyDeath => match self.animation_frame {
                0 => "death_0",
                1 => "death_1",
                2 => "death_2",
                3 => "death_3",
                _ => "death_4",
            },

            // Firebrand's death animation loops, where entity deaths run through a single cycle
            Mode::FirebrandDeath => {
                let cycle_offset = if self.direction.is_diagonal() { 1 } else { 0 };
                match (self.animation_frame + cycle_offset) % self.num_animation_frames() {
                    0 => "death_0",
                    1 => "death_1",
                    2 => "death_2",
                    _ => "death_3",
                }
            }
        }
    }

    fn did_exit_viewport(&mut self) {
        self.alive = false;
    }
}
