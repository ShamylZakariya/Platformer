use std::time::Duration;

use cgmath::*;

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    geom::Bounds,
    map,
    sprite::{self, collision, find_bounds, rendering},
    state::{constants::sprite_layers::BACKGROUND, events::Event},
};

const OPEN_SPEED: f32 = 1.0;

enum Mode {
    Closed,
    Opening,
    Open,
}

pub struct ExitDoor {
    entity_id: u32,
    offset: Point3<f32>,
    stage_sprites: Vec<sprite::Sprite>,
    bounds: Bounds,
    mode: Mode,
    alive: bool,
}

impl ExitDoor {
    pub fn new(stage_sprites: Vec<sprite::Sprite>) -> Self {
        let bounds = find_bounds(&stage_sprites);
        Self {
            entity_id: 0,
            offset: point3(0.0, 0.0, BACKGROUND),
            stage_sprites,
            bounds,
            mode: Mode::Closed,
            alive: true,
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
        message_dispatcher: &mut Dispatcher,
        game_state_peek: &GameStatePeek,
    ) {
        match self.mode {
            Mode::Closed => {}
            Mode::Opening => {
                self.offset.x -= OPEN_SPEED * dt.as_secs_f32();
                if self.offset.x < -self.bounds.extent.x {
                    self.offset.x = -self.bounds.extent.x;
                    self.mode = Mode::Open;
                }
            }
            Mode::Open => {
                if game_state_peek.player_position.x
                    > self.bounds.origin.x + self.bounds.extent.x * 0.5
                {
                    println!("Sending exit message");
                    message_dispatcher.broadcast(Event::PlayerPassedThroughExitDoor);
                    self.alive = false;
                }
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
        self.alive
    }

    fn position(&self) -> Point3<f32> {
        point3(
            self.bounds.origin.x + self.offset.x,
            self.bounds.origin.y + self.offset.y,
            BACKGROUND,
        )
    }

    fn stage_sprites(&self) -> Option<Vec<sprite::Sprite>> {
        Some(self.stage_sprites.clone())
    }

    fn handle_message(&mut self, message: &Message) {
        if let Event::OpenExitDoor = message.event {
            self.mode = Mode::Opening;
        }
    }
}
