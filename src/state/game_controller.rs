use winit::window::Window;

use crate::{entity, event_dispatch};

use super::{events::Event, game_state::GameState, game_ui::GameUi};

//---------------------------------------------------------------------------------------------------------------------

const RESTART_GAME_DELAY: f32 = 4.0;
const GAME_OVER_DELAY: f32 = 2.0;
const FADE_IN_DURATION: f32 = 2.0;
const FADE_OUT_DURATION: f32 = 2.0;

//---------------------------------------------------------------------------------------------------------------------

pub struct GameController {
    current_checkpoint: u32,
    lives_remaining: u32,
    restart_game_countdown: Option<f32>,
    game_over_countdown: Option<f32>,
    fade_in_countdown: Option<f32>,
    fade_out_countdown: Option<f32>,
}

impl Default for GameController {
    fn default() -> Self {
        Self::new(3, 0)
    }
}

impl GameController {
    pub fn new(lives: u32, checkpoint: u32) -> Self {
        Self {
            current_checkpoint: checkpoint,
            lives_remaining: lives,
            restart_game_countdown: None,
            game_over_countdown: None,
            fade_in_countdown: None,
            fade_out_countdown: None,
        }
    }

    pub fn gamepad_input(&mut self, _event: gilrs::Event) {}

    pub fn update(
        &mut self,
        _window: &Window,
        dt: std::time::Duration,
        game_state: &mut GameState,
        game_ui: &mut GameUi,
        message_dispatcher: &mut event_dispatch::Dispatcher,
    ) {
        let dt = dt.as_secs_f32();

        if let Some(fade_in_countdown) = self.fade_in_countdown {
            let fade_in_countdown = fade_in_countdown - dt;
            let palette_shift = (fade_in_countdown / FADE_IN_DURATION).max(0.0);
            game_state.set_palette_shift(palette_shift);
            game_ui.set_palette_shift(palette_shift);
            if fade_in_countdown < 0.0 {
                self.fade_in_countdown = None;
            } else {
                self.fade_in_countdown = Some(fade_in_countdown);
            }
        }

        if let Some(fade_out_countdown) = self.fade_out_countdown {
            let fade_out_countdown = fade_out_countdown - dt;
            let palette_shift = (1.0 - (fade_out_countdown / FADE_OUT_DURATION)).max(0.0);
            game_state.set_palette_shift(-palette_shift);
            game_ui.set_palette_shift(-palette_shift);
            if fade_out_countdown < 0.0 {
                self.fade_out_countdown = None;
            } else {
                self.fade_out_countdown = Some(fade_out_countdown);
            }
        }

        if let Some(restart_game_countdown) = self.restart_game_countdown {
            let restart_game_countdown = restart_game_countdown - dt;
            if restart_game_countdown < 0.0 {
                self.restart_game_countdown = None;
                game_state.restart_game_at_checkpoint(
                    self.current_checkpoint,
                    self.lives_remaining,
                    message_dispatcher,
                );
            } else {
                self.restart_game_countdown = Some(restart_game_countdown);
            }
        }

        if let Some(game_over_countdown) = self.game_over_countdown {
            let game_over_countdown = game_over_countdown - dt;
            if game_over_countdown < 0.0 {
                self.game_over_countdown = None;
                game_state.game_over(message_dispatcher);
            } else {
                self.game_over_countdown = Some(game_over_countdown);
            }
        }
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
                        self.current_checkpoint = self.current_checkpoint.max(idx);
                    }
                }
            }

            Event::FirebrandDied => {
                println!("Firebrand died!");
                if self.lives_remaining > 0 {
                    self.lives_remaining -= 1;
                    self.restart_game_countdown = Some(RESTART_GAME_DELAY);
                } else {
                    self.game_over_countdown = Some(GAME_OVER_DELAY);
                }
            }

            Event::FirebrandCreated {
                checkpoint,
                num_restarts,
            } if *checkpoint == 0 && *num_restarts == 0 => {
                self.fade_in_countdown = Some(FADE_IN_DURATION);
            }

            Event::FirebrandPassedThroughExitDoor => {
                self.fade_out_countdown = Some(FADE_OUT_DURATION);
            }

            _ => {}
        }
    }

    pub fn current_checkpoint(&self) -> u32 {
        self.current_checkpoint
    }

    pub fn lives_remaining(&self) -> u32 {
        self.lives_remaining
    }
}
