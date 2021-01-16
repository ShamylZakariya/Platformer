use std::time::Duration;

use cgmath::*;

use crate::{
    entity::{Dispatcher, Entity, Message},
    map,
    sprite::{self, collision, rendering},
    tileset,
};

const FALLING_BRIDGE_CONTACT_DELAY: f32 = 0.2;

pub struct FireSprite {
    entity_id: u32,
    sprite: Option<sprite::Sprite>,
    position: Point3<f32>,
}

impl Default for FireSprite {
    fn default() -> Self {
        Self {
            entity_id: 0,
            sprite: None,
            position: point3(0.0, 0.0, 0.0),
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
        _dt: Duration,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
        _message_dispatcher: &mut Dispatcher,
    ) {
        // let dt = dt.as_secs_f32();
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        uniforms.data.set_model_position(self.position);
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
        point3(
            self.position.x,
            self.position.y,
            self.sprite.unwrap().origin.z,
        )
    }

    fn sprite_name(&self) -> &str {
        "fire_sprite"
    }

    fn sprite_cycle(&self) -> &str {
        "default"
    }

    fn handle_message(&mut self, _message: &Message) {}
}
