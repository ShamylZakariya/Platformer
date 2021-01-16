use std::time::Duration;

use cgmath::*;

use crate::{
    entity::{Dispatcher, Entity, Message},
    map,
    sprite::{self, collision},
    tileset,
};

pub struct SpawnPoint {
    entity_id: u32,
    position: Point3<f32>,
}

impl Default for SpawnPoint {
    fn default() -> Self {
        Self {
            entity_id: 0,
            position: point3(0.0, 0.0, 0.0),
        }
    }
}

impl Entity for SpawnPoint {
    fn init_from_map_sprite(
        &mut self,
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
    ) {
        self.entity_id = sprite
            .entity_id
            .expect("Entity sprites should have an entity_id");
        self.position = sprite.origin;
    }

    fn update(
        &mut self,
        _dt: Duration,
        _collision_space: &mut collision::Space,
        _message_dispatcher: &mut Dispatcher,
    ) {
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::SpawnPoint
    }

    fn position(&self) -> Point3<f32> {
        point3(self.position.x, self.position.y, -1.0)
    }

    fn sprite_name(&self) -> &str {
        ""
    }

    fn sprite_cycle(&self) -> &str {
        ""
    }

    fn handle_message(&mut self, _message: &Message) {}

    fn did_enter_viewport(&mut self) {
        println!("SpawnPoint[{}]::did_enter_viewport", self.entity_id());
    }

    fn did_exit_viewport(&mut self) {
        println!("SpawnPoint[{}]::did_exit_viewport", self.entity_id());
    }
}
