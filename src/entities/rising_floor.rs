use std::time::Duration;

use cgmath::*;

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, collision, rendering},
    state::{constants::sprite_layers::FOREGROUND, events::Event},
};

pub struct RisingFloor {
    entity_id: u32,
    position: Point3<f32>,
    stage_sprites: Vec<sprite::Sprite>,
}

impl RisingFloor {
    pub fn new(stage_sprites: Vec<sprite::Sprite>) -> Self {
        Self {
            entity_id: 0,
            position: point3(0.0, 0.0, 0.0),
            stage_sprites,
        }
    }
}

impl Entity for RisingFloor {
    fn init(&mut self, entity_id: u32, _map: &map::Map, _collision_space: &mut collision::Space) {
        self.entity_id = entity_id;
        self.position.y -= 6.0; // TODO: COmpute self-extent
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
        crate::entities::EntityClass::RisingFloor
    }

    fn is_alive(&self) -> bool {
        true
    }

    fn position(&self) -> Point3<f32> {
        point3(self.position.x, self.position.y, FOREGROUND)
    }

    fn stage_sprites(&self) -> Option<Vec<sprite::Sprite>> {
        Some(self.stage_sprites.clone())
    }

    fn handle_message(&mut self, message: &Message) {
        if let Event::RaiseExitFloor = message.event {
            println!("RisingFloor - time to rise!");
        }
    }
}
