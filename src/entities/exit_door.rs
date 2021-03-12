use std::time::Duration;

use cgmath::*;

use crate::{
    audio, collision,
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, find_bounds, rendering},
    state::{constants::layers, events::Event},
    util::{self, Bounds},
};

use super::util::HorizontalDir;

const OPEN_SPEED: f32 = 1.25;

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
    open_dir: HorizontalDir,
    should_send_exit_message: bool,
    last_player_position: Option<Point2<f32>>,
}

impl ExitDoor {
    pub fn new(stage_sprites: Vec<sprite::Sprite>, open_dir: HorizontalDir) -> Self {
        let bounds = find_bounds(&stage_sprites);
        Self {
            entity_id: 0,
            offset: point3(0.0, 0.0, layers::stage::BACKGROUND + 1.0),
            stage_sprites,
            bounds,
            mode: Mode::Closed,
            open_dir,
            should_send_exit_message: ExitDoor::should_send_exit_message_for_dir(open_dir),
            last_player_position: None,
        }
    }

    fn should_send_exit_message_for_dir(dir: HorizontalDir) -> bool {
        // we have two doors, so only send the message from the west door
        dir == HorizontalDir::West
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
        _audio: &mut audio::Audio,
        message_dispatcher: &mut Dispatcher,
        game_state_peek: &GameStatePeek,
    ) {
        match self.mode {
            Mode::Closed => {
                self.offset.x = 0.0;
                self.should_send_exit_message =
                    ExitDoor::should_send_exit_message_for_dir(self.open_dir);
            }
            Mode::Opening => match self.open_dir {
                HorizontalDir::East => {
                    self.offset.x += OPEN_SPEED * dt.as_secs_f32();
                    if self.offset.x > self.bounds.width() - 0.5 {
                        self.offset.x = self.bounds.width() - 0.5;
                        self.mode = Mode::Open;
                        message_dispatcher.broadcast(Event::ExitDoorOpened);
                    }
                }
                HorizontalDir::West => {
                    self.offset.x -= OPEN_SPEED * dt.as_secs_f32();
                    if self.offset.x < -self.bounds.width() + 0.5 {
                        self.offset.x = -self.bounds.width() + 0.5;
                        self.mode = Mode::Open;
                        message_dispatcher.broadcast(Event::ExitDoorOpened);
                    }
                }
            },
            Mode::Open if self.should_send_exit_message => {
                if let Some(last_player_position) = self.last_player_position {
                    let threshold = self.bounds.left() + self.bounds.width() * 0.5;

                    // only send exit message if player crosses threshold, going left or right
                    if (game_state_peek.player_position.x > threshold
                        && last_player_position.x < threshold)
                        || (game_state_peek.player_position.x < threshold
                            && last_player_position.x > threshold)
                    {
                        message_dispatcher.broadcast(Event::FirebrandPassedThroughExitDoor);
                        self.should_send_exit_message = false;
                    }
                }
            }
            _ => {}
        }

        self.last_player_position = Some(game_state_peek.player_position);
    }

    fn update_uniforms(&self, uniforms: &mut util::UniformWrapper<rendering::Uniforms>) {
        // round offset to 0.5 unit increments
        let x = (self.offset.x / 0.5).round() * 0.5;
        let offset = point3(x, self.offset.y, self.offset.z);

        uniforms.data.set_model_position(offset);
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
        self.offset
    }

    fn stage_sprites(&self) -> Option<Vec<sprite::Sprite>> {
        Some(self.stage_sprites.clone())
    }

    fn handle_message(&mut self, message: &Message) {
        match message.event {
            Event::OpenExitDoor => self.mode = Mode::Opening,
            Event::ResetState => self.mode = Mode::Closed,
            _ => {}
        }
    }
}
