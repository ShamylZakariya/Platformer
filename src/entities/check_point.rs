use std::time::Duration;

use cgmath::*;

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, collision},
    state::events::Event,
    tileset,
};

pub struct CheckPoint {
    entity_id: u32,
    position: Point3<f32>,
    did_send_pass_message: bool,
}

impl Default for CheckPoint {
    fn default() -> Self {
        Self {
            entity_id: 0,
            position: point3(0.0, 0.0, 0.0),
            did_send_pass_message: false,
        }
    }
}

impl Entity for CheckPoint {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.position = sprite.origin;
    }

    fn update(
        &mut self,
        _dt: Duration,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
        game_state_peek: &GameStatePeek,
    ) {
        if !self.did_send_pass_message && game_state_peek.player_position.x > self.position.x {
            message_dispatcher.entity_to_global(self.entity_id(), Event::FirebrandPassedCheckpoint);
            self.did_send_pass_message = true;
        }
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::CheckPoint
    }

    fn position(&self) -> Point3<f32> {
        point3(self.position.x, self.position.y, -1.0)
    }

    fn should_draw(&self) -> bool {
        false
    }
}
