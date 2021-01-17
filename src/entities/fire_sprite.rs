use std::time::Duration;

use cgmath::*;

use crate::{
    entity::{Dispatcher, Entity, Message},
    map,
    sprite::{self, collision, rendering},
    tileset,
};

const ANIMATION_CYCLE_DURATION: f32 = 0.133;

pub struct FireSprite {
    entity_id: u32,
    sprite: Option<sprite::Sprite>,
    position: Point3<f32>,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
}

impl Default for FireSprite {
    fn default() -> Self {
        Self {
            entity_id: 0,
            sprite: None,
            position: point3(0.0, 0.0, 0.0),
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
        }
    }
}

impl Entity for FireSprite {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.sprite = Some(*sprite);
        self.position = sprite.origin;
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
        _message_dispatcher: &mut Dispatcher,
    ) {
        let dt = dt.as_secs_f32();
        self.animation_cycle_tick_countdown -= dt;
        if self.animation_cycle_tick_countdown <= 0.0 {
            self.animation_cycle_tick_countdown += ANIMATION_CYCLE_DURATION;
            self.animation_cycle_tick += 1;
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        let (xscale, xoffset) = if self.animation_cycle_tick / 2 % 2 == 0 {
            (1.0, 0.0)
        } else {
            (-1.0, 1.0)
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
        crate::entities::EntityClass::FireSprite
    }

    fn is_alive(&self) -> bool {
        true
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "fire_sprite"
    }

    fn sprite_cycle(&self) -> &str {
        if self.animation_cycle_tick % 2 == 0 {
            "default"
        } else {
            "alt"
        }
    }

    fn handle_message(&mut self, _message: &Message) {}
}
