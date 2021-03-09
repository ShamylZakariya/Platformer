use std::rc::Rc;

use anyhow::*;
use winit::{event::WindowEvent, window::Window};

use crate::{entity, event_dispatch, texture, Options};

use super::{
    debug_overlay::DebugOverlay, game_controller::GameController, game_state::GameState,
    game_ui::GameUi, gpu_state::GpuState,
};

// --------------------------------------------------------------------------------------------------------------------

pub struct AppState {
    options: Options,
    gpu: GpuState,
    game_controller: GameController,
    game_state: GameState,
    game_ui: GameUi,
    overlay: Option<DebugOverlay>,

    entity_id_vendor: entity::IdVendor,
    message_dispatcher: event_dispatch::Dispatcher,
}

impl AppState {
    pub fn new(window: &Window, mut gpu: GpuState, options: Options) -> Result<Self> {
        let mut entity_id_vendor = entity::IdVendor::default();

        let tonemap_file = format!("res/tonemaps/{}.png", options.palette);
        let tonemap = Rc::new(
            texture::Texture::load(&gpu.device, &gpu.queue, &tonemap_file, false)
                .with_context(|| format!("Failed to load palette \"{}\"", tonemap_file))?,
        );

        let game_controller =
            GameController::new(options.lives, options.checkpoint.unwrap_or(0_u32));
        let game_state = GameState::new(
            &mut gpu,
            &options,
            &mut entity_id_vendor,
            tonemap.clone(),
            game_controller.current_checkpoint(),
            game_controller.lives_remaining(),
        );
        let game_ui = GameUi::new(&mut gpu, &options, &mut entity_id_vendor, tonemap);
        let overlay_ui = if options.debug_overlay {
            Some(DebugOverlay::new(window, &gpu))
        } else {
            None
        };

        Ok(Self {
            options,
            gpu,
            game_controller,
            game_state,
            game_ui,
            overlay: overlay_ui,

            entity_id_vendor,
            message_dispatcher: event_dispatch::Dispatcher::default(),
        })
    }

    pub fn event(&mut self, window: &Window, event: &winit::event::Event<()>) {
        if let Some(ref mut overlay) = self.overlay {
            overlay.event(window, event);
        }
    }

    pub fn resize(&mut self, window: &Window, new_size: winit::dpi::PhysicalSize<u32>) {
        self.gpu.resize(window, new_size);
        self.game_state.resize(window, new_size);
        self.game_ui.resize(window, new_size)
    }

    pub fn input(&mut self, window: &Window, event: &WindowEvent) -> bool {
        if self
            .game_state
            .input(window, event, self.game_ui.is_paused())
        {
            true
        } else {
            self.game_ui.input(window, event)
        }
    }

    pub fn gamepad_input(&mut self, event: gilrs::Event) {
        self.game_state
            .gamepad_input(event, self.game_ui.is_paused());
        self.game_ui.gamepad_input(event);
        self.game_controller.gamepad_input(event);
    }

    pub fn update(&mut self, window: &Window, dt: std::time::Duration) {
        // Set a max timestep - this is crude, but prevents explosions when stopping
        // execution in the debugger, and we get a HUGE timestep after resuming.
        let dt = dt.min(std::time::Duration::from_millis(32));

        if let Some(ref mut overlay) = self.overlay {
            overlay.update(window, dt);
        }

        let game_dt = if self.game_ui.is_paused() {
            std::time::Duration::from_secs(0)
        } else {
            dt
        };

        self.game_state.update(
            window,
            game_dt,
            &mut self.gpu,
            &mut self.message_dispatcher,
            &mut self.entity_id_vendor,
        );

        self.game_ui.update(
            window,
            dt,
            &mut self.gpu,
            &self.game_state,
            &mut self.message_dispatcher,
            &mut self.entity_id_vendor,
        );

        self.game_controller.update(
            window,
            dt,
            &mut self.game_state,
            &mut self.game_ui,
            &mut self.message_dispatcher,
        );

        event_dispatch::Dispatcher::dispatch(&self.message_dispatcher.drain(), self);
    }

    pub fn render(&mut self, window: &Window) {
        let frame = self.gpu.next_frame();
        let mut encoder = self.gpu.encoder();

        //
        //  Render game and UI overlay
        //

        self.game_state
            .render(window, &mut self.gpu, &frame, &mut encoder);

        self.game_ui
            .render(window, &mut self.gpu, &frame, &mut encoder);

        if let Some(ref mut overlay) = self.overlay {
            overlay.render(
                window,
                &mut self.game_state,
                &mut self.gpu,
                &frame,
                &mut encoder,
            );
        }

        let commands = encoder.finish();
        self.gpu.queue.submit(std::iter::once(commands));
    }
}

impl event_dispatch::MessageHandler for AppState {
    fn handle_message(&mut self, message: &event_dispatch::Message) {
        self.game_controller.handle_message(
            message,
            &mut self.message_dispatcher,
            &mut self.entity_id_vendor,
            &mut self.game_state,
        );
        self.game_state.handle_message(
            message,
            &mut self.message_dispatcher,
            &mut self.entity_id_vendor,
        );
        self.game_ui.handle_message(
            message,
            &mut self.message_dispatcher,
            &mut self.entity_id_vendor,
        );
    }
}
