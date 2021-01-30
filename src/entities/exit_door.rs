use std::time::Duration;

use cgmath::*;

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, bounds, collision, rendering},
    state::{
        constants::sprite_layers::{BACKGROUND, FOREGROUND},
        events::Event,
    },
};

const OPEN_SPEED: f32 = 1.0;

pub struct ExitDoor {
    entity_id: u32,
    offset: Point3<f32>,
    stage_sprites: Vec<sprite::Sprite>,
    bounds: (Point2<f32>, Vector2<f32>),
    opening: bool,
}

impl ExitDoor {
    pub fn new(stage_sprites: Vec<sprite::Sprite>) -> Self {
        let bounds = bounds(&stage_sprites);
        println!("ExitDoor bounds: {:?}", bounds);
        Self {
            entity_id: 0,
            offset: point3(0.0, 0.0, BACKGROUND),
            stage_sprites,
            bounds,
            opening: false,
        }
    }
}

impl Entity for ExitDoor {
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
        if self.opening {
            self.offset.x -= OPEN_SPEED * dt.as_secs_f32();
            if self.offset.x < -self.bounds.1.x {
                self.offset.x = -self.bounds.1.x;
                println!("Done opening door...");
                self.opening = false;
            }
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        uniforms.data.set_model_position(self.offset);
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::ExitDoor
    }

    fn is_alive(&self) -> bool {
        true
    }

    fn position(&self) -> Point3<f32> {
        point3(
            self.bounds.0.x + self.offset.x,
            self.bounds.0.y + self.offset.y,
            BACKGROUND,
        )
    }

    fn stage_sprites(&self) -> Option<Vec<sprite::Sprite>> {
        Some(self.stage_sprites.clone())
    }

    fn handle_message(&mut self, message: &Message) {
        if let Event::OpenExitDoor = message.event {
            println!("Opening Exit Door!");
            self.opening = true;
        }
    }
}
