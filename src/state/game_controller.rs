use winit::window::Window;

use crate::{entity, event_dispatch};

use super::{events::Event, game_state::GameState};

pub struct GameController {
    current_checkpoint: u32,
    num_lives: u32,
}

impl Default for GameController {
    fn default() -> Self {
        Self {
            current_checkpoint: 0,
            num_lives: 3,
        }
    }
}

impl GameController {
    pub fn update(
        &mut self,
        _window: &Window,
        _dt: std::time::Duration,
        _game_state: &mut GameState,
    ) {
    }

    pub fn handle_message(
        &mut self,
        message: &event_dispatch::Message,
        _message_dispatcher: &mut event_dispatch::Dispatcher,
        _entity_id_vendor: &mut entity::IdVendor,
        game_state: &mut GameState,
    ) {
        match &message.event {
            Event::FirebrandPassedCheckpoint => {
                if let Some(sender_id) = message.sender_entity_id {
                    if let Some(idx) = game_state.index_of_checkpoint(sender_id) {
                        println!(
                            "Firebrand passed checkpoint: {:?} index: {}",
                            message.sender_entity_id, idx
                        );
                    }
                }
            }

            Event::FirebrandDied => {
                println!("Firebrand died!");
            }

            _ => {}
        }
    }

    pub fn current_checkpoint(&self) -> u32 {
        0
    }
}
