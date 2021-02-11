use std::time::Duration;

use cgmath::*;

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, collision, rendering},
    tileset,
};

pub struct UiHealthDot {
    entity_id: u32,
    sprite: Option<sprite::Sprite>,
    position: Point3<f32>,
}

impl Default for UiHealthDot {
    fn default() -> Self {
        Self {
            entity_id: 0,
            sprite: None,
            position: point3(0.0, 0.0, 0.0),
        }
    }
}

impl Entity for UiHealthDot {
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
        println!(
            "UihealthDot[{}]::init_from_map_sprite sprite:{:?} position:{:?}",
            self.entity_id, sprite, sprite.origin
        );

        // TODO: look through the map to find which health dot this one is
    }

    fn update(
        &mut self,
        _dt: Duration,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
        _message_dispatcher: &mut Dispatcher,
        _game_state_peek: &GameStatePeek,
    ) {
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        uniforms.data.set_model_position(self.position);
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::UiHealthDot
    }

    fn is_alive(&self) -> bool {
        true
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "health_dot"
    }

    fn sprite_cycle(&self) -> &str {
        "full"
    }
}
