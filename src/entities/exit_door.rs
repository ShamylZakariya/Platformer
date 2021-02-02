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

use super::util::Direction;

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
    open_dir: Direction,
}

impl ExitDoor {
    pub fn new(stage_sprites: Vec<sprite::Sprite>, open_dir: Direction) -> Self {
        let bounds = find_bounds(&stage_sprites);
        Self {
            entity_id: 0,
            offset: point3(0.0, 0.0, 0.0),
            stage_sprites,
            bounds,
            mode: Mode::Closed,
            open_dir,
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
            Mode::Opening => match self.open_dir {
                Direction::East => {
                    self.offset.x += OPEN_SPEED * dt.as_secs_f32();
                    if self.offset.x > self.bounds.width() - 0.5 {
                        self.offset.x = self.bounds.width() - 0.5;
                        self.mode = Mode::Open;
                        message_dispatcher.broadcast(Event::ExitDoorOpened);
                    }
                }
                Direction::West => {
                    self.offset.x -= OPEN_SPEED * dt.as_secs_f32();
                    if self.offset.x < -self.bounds.width() + 0.5 {
                        self.offset.x = -self.bounds.width() + 0.5;
                        self.mode = Mode::Open;
                        message_dispatcher.broadcast(Event::ExitDoorOpened);
                    }
                }
            },
            Mode::Open => {
                if game_state_peek.player_position.x
                    > self.bounds.left() + self.bounds.width() * 0.5
                {
                    println!("Sending exit message");
                    message_dispatcher.broadcast(Event::PlayerPassedThroughExitDoor);
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
        true
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
