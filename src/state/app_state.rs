use anyhow::*;
use winit::{event::WindowEvent, window::Window};

use crate::{audio::Audio, entity, event_dispatch, texture, Options};

use super::{
    debug_overlay::DebugOverlay, game_controller::GameController, game_state::GameState,
    game_ui::GameUi, gpu_state::GpuState, lcd_filter::LcdFilter,
};

// --------------------------------------------------------------------------------------------------------------------

/// Holder for various AppState fields to pass in to GameController, GameUi, GameState update() methods
pub struct AppContext<'a, 'window> {
    pub window: &'a Window,
    pub gpu: &'a mut GpuState<'window>,
    pub audio: &'a mut Audio,
    pub message_dispatcher: &'a mut event_dispatch::Dispatcher,
    pub entity_id_vendor: &'a mut entity::IdVendor,
    pub frame_idx: u32,
    pub time: std::time::Instant,
    pub game_delta_time: std::time::Duration,
    pub real_delta_time: std::time::Duration,
}

// --------------------------------------------------------------------------------------------------------------------

pub struct AppState<'a> {
    options: Options,
    pub gpu: GpuState<'a>,
    audio: Audio,
    game_controller: GameController,
    game_state: GameState,
    game_ui: GameUi,
    debug_overlay: Option<DebugOverlay>,
    lcd_filter: LcdFilter,

    entity_id_vendor: entity::IdVendor,
    message_dispatcher: event_dispatch::Dispatcher,

    window: &'a winit::window::Window,
}

impl<'a> AppState<'a> {
    pub fn new(window: &'a Window, mut gpu: GpuState<'a>, options: Options) -> Result<Self> {
        let mut entity_id_vendor = entity::IdVendor::default();

        let audio = Audio::new(&options);

        let game_controller =
            GameController::new(options.lives, options.checkpoint.unwrap_or(0_u32));
        let mut game_state = GameState::new(
            &mut gpu,
            &options,
            &mut entity_id_vendor,
            game_controller.current_checkpoint(),
            game_controller.lives_remaining(),
        );
        let mut game_ui = GameUi::new(&mut gpu, &options, &mut entity_id_vendor);
        let debug_overlay = if options.debug_overlay {
            Some(DebugOverlay::new(window, &gpu))
        } else {
            None
        };

        let tonemap_file = format!("res/tonemaps/{}.png", options.palette);
        let tonemap = texture::Texture::load(&gpu.device, &gpu.queue, &tonemap_file)
            .with_context(|| format!("Failed to load palette \"{}\"", tonemap_file))?;
        let lcd_filter = LcdFilter::new(&mut gpu, &options, tonemap);

        if options.checkpoint == Some(0) {
            // when game starts, palette is shifted to white, an Event::FirebrandCreated
            // broadcast will be received by GameController which will animate palette
            // shift from 1.0 to 0.0
            game_state.set_palette_shift(1.0);
            game_ui.set_palette_shift(1.0);
        }

        Ok(Self {
            options,
            gpu,
            audio,
            game_controller,
            game_state,
            game_ui,
            debug_overlay,
            lcd_filter,
            entity_id_vendor,
            message_dispatcher: event_dispatch::Dispatcher::default(),
            window,
        })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn event(&mut self, event: &winit::event::Event<()>) {
        if let Some(ref mut debug_overlay) = self.debug_overlay {
            debug_overlay.event(self.window, event);
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.gpu.resize(new_size);
        self.game_state.resize(self.window, new_size, &self.gpu);
        self.game_ui.resize(self.window, new_size, &self.gpu);
        self.lcd_filter
            .resize(self.window, new_size, &self.gpu, &self.game_state);
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        if self
            .game_state
            .input(self.window, event, self.game_ui.is_paused())
        {
            true
        } else {
            self.game_ui.input(self.window, event)
        }
    }

    pub fn gamepad_input(&mut self, event: gilrs::Event) {
        self.game_state
            .gamepad_input(event, self.game_ui.is_paused());
        self.game_ui.gamepad_input(event);
        self.game_controller.gamepad_input(event);
    }

    pub fn update(
        &mut self,
        time: std::time::Instant,
        delta_time: std::time::Duration,
        frame_idx: u32,
    ) {
        // Skip update if delta time is huge - this can happen after
        // resuming from a pause.
        if delta_time > std::time::Duration::from_millis(32) {
            return;
        }

        if let Some(ref mut debug_overlay) = self.debug_overlay {
            debug_overlay.update(self.window, delta_time);
        }

        let game_dt = if self.game_ui.is_paused() {
            std::time::Duration::from_secs(0)
        } else {
            delta_time
        };

        self.audio.update(delta_time);

        {
            let mut ctx = AppContext {
                window: self.window,
                gpu: &mut self.gpu,
                audio: &mut self.audio,
                message_dispatcher: &mut self.message_dispatcher,
                entity_id_vendor: &mut self.entity_id_vendor,
                frame_idx,
                time,
                game_delta_time: game_dt,
                real_delta_time: delta_time,
            };

            self.game_state.update(&mut ctx);

            self.game_ui.update(&mut ctx, &self.game_state);

            self.lcd_filter.update(&mut ctx, &self.game_state);

            self.game_controller
                .update(&mut ctx, &mut self.game_state, &mut self.game_ui);
        }

        event_dispatch::Dispatcher::dispatch(&self.message_dispatcher.drain(), self);
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::SurfaceTexture,
        frame_index: usize,
    ) {
        //
        //  Render game and UI overlay
        //

        self.game_state
            .render(self.window, &mut self.gpu, encoder, frame_index);

        self.game_ui
            .render(self.window, &mut self.gpu, encoder, frame_index);

        self.lcd_filter
            .render(self.window, &mut self.gpu, output, encoder, frame_index);

        if let Some(ref mut debug_overlay) = self.debug_overlay {
            debug_overlay.render(
                self.window,
                &mut self.gpu,
                output,
                encoder,
                &mut self.game_state,
                &mut self.lcd_filter,
            );
        }
    }
}

impl<'a> event_dispatch::MessageHandler for AppState<'a> {
    fn handle_message(&mut self, message: &event_dispatch::Message) {
        self.game_controller.handle_message(
            message,
            &mut self.message_dispatcher,
            &mut self.entity_id_vendor,
            &mut self.audio,
            &mut self.game_state,
        );
        self.game_state.handle_message(
            message,
            &mut self.message_dispatcher,
            &mut self.entity_id_vendor,
            &mut self.audio,
        );
        self.game_ui.handle_message(message);
    }
}
