use std::time::Duration;

use cgmath::*;

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, collision},
    state::{constants::layers, events::Event},
    tileset,
};

const FALLING_BRIDGE_CONTACT_DELAY: f32 = 0.2;

pub struct BossFightTrigger {
    entity_id: u32,
    position: Point3<f32>,
    did_send_fight_started_message: bool,
}

impl Default for BossFightTrigger {
    fn default() -> Self {
        Self {
            entity_id: 0,
            position: point3(0.0, 0.0, 0.0),
            did_send_fight_started_message: false,
        }
    }
}

impl Entity for BossFightTrigger {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.position = point3(sprite.origin.x, sprite.origin.y, layers::stage::ENTITIES);
    }

    fn update(
        &mut self,
        _dt: Duration,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
        game_state_peek: &GameStatePeek,
    ) {
        if game_state_peek.player_position.x > (self.position.x + 1.0)
            && !self.did_send_fight_started_message
        {
            message_dispatcher.entity_to_global(
                self.entity_id,
                Event::BossArenaEncountered {
                    arena_left: self.position.x + 1.0,
                },
            );

            self.did_send_fight_started_message = true;
        }
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::BossFightTrigger
    }

    fn is_alive(&self) -> bool {
        true
    }

    fn should_draw(&self) -> bool {
        false
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn handle_message(&mut self, message: &Message) {
        if matches!(message.event, Event::ResetState) {
            self.did_send_fight_started_message = false;
        }
    }
}
