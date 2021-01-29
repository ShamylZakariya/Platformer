use std::time::Duration;

use cgmath::*;

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{collision, rendering},
};

use super::util::Direction;

// ---------------------------------------------------------------------------------------------------------------------

const CYCLE_DURATION: f32 = 0.1;
const BLOWBACK_VEL: f32 = 1.5 / 0.4;

// ---------------------------------------------------------------------------------------------------------------------

pub struct DeathAnimation {
    entity_id: u32,
    position: Point3<f32>,
    direction: Direction,
    alive: bool,
    time: f32,
    animation_frame: i32,
}

impl DeathAnimation {
    pub fn new(position: Point3<f32>, direction: Direction) -> Self {
        Self {
            entity_id: 0,
            position,
            direction,
            alive: true,
            time: 0.0,
            animation_frame: 0,
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
        _message_dispatcher: &mut Dispatcher,
        _game_state_peek: &GameStatePeek,
    ) {
        let dt = dt.as_secs_f32();
        self.time += dt;
        self.animation_frame = (self.time / CYCLE_DURATION).floor() as i32;
        if self.animation_frame > 4 {
            self.alive = false;
        }
        match self.direction {
            Direction::East => {
                self.position.x += BLOWBACK_VEL * dt;
            }
            Direction::West => {
                self.position.x -= BLOWBACK_VEL * dt;
            }
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        let (xscale, xoffset) = match self.direction {
            Direction::East => (-1.0, 1.0),
            Direction::West => (1.0, 0.0),
        };

        uniforms
            .data
            .set_model_position(self.position + vec3(xoffset, 0.0, 0.0))
            .set_sprite_scale(vec2(xscale, 1.0));
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

    fn sprite_name(&self) -> &str {
        "death"
    }

    fn sprite_cycle(&self) -> &str {
        match self.animation_frame {
            0 => "death_0",
            1 => "death_1",
            2 => "death_2",
            3 => "death_3",
            _ => "death_4",
        }
    }
}
