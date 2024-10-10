use std::time;

use anyhow::*;
use winit::{event::WindowEvent, window::Window};

use crate::{audio::Audio, entity, event_dispatch, texture, Options};

use super::{
    debug_overlay::DebugOverlay,
    game_controller::GameController,
    game_state::GameState,
    game_ui::GameUi,
    gpu_state::{self, GpuState},
    lcd_filter::LcdFilter,
};

// --------------------------------------------------------------------------------------------------------------------

/// Holder for various AppState fields to pass in to GameController, GameUi, GameState update() methods
pub struct AppContext<'a> {
    pub gpu: &'a mut GpuState,
    pub audio: &'a mut Audio,
    pub message_dispatcher: &'a mut event_dispatch::Dispatcher,
    pub entity_id_vendor: &'a mut entity::IdVendor,
    pub frame_idx: u32,
    pub time: std::time::Instant,
    pub game_delta_time: std::time::Duration,
    pub real_delta_time: std::time::Duration,
}

// --------------------------------------------------------------------------------------------------------------------

pub struct AppState {
    options: Options,
    audio: Audio,
    game_controller: GameController,
    game_state: GameState,
    game_ui: GameUi,
    debug_overlay: Option<DebugOverlay>,
    lcd_filter: LcdFilter,

    entity_id_vendor: entity::IdVendor,
    message_dispatcher: event_dispatch::Dispatcher,

    last_render_time: std::time::Instant,
    frame_index: u32,

    // gpu is last; which means it's last to be destructed. This prevents a crash during shutdown (sigh)
    pub gpu: GpuState,
}

impl AppState {
    pub fn new(window: winit::window::Window, options: Options) -> Result<Self> {
        let mut entity_id_vendor = entity::IdVendor::default();

        let audio = Audio::new(&options);

        let game_controller =
            GameController::new(options.lives, options.checkpoint.unwrap_or(0_u32));

        let mut gpu = pollster::block_on(gpu_state::GpuState::new(window));

        let mut game_state = GameState::new(
            &mut gpu,
            &options,
            &mut entity_id_vendor,
            game_controller.current_checkpoint(),
            game_controller.lives_remaining(),
        );
        let mut game_ui = GameUi::new(&mut gpu, &options, &mut entity_id_vendor);
        let debug_overlay = if options.debug_overlay {
            Some(DebugOverlay::new(gpu.window(), &gpu))
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
            audio,
            game_controller,
            game_state,
            game_ui,
            debug_overlay,
            lcd_filter,
            entity_id_vendor,
            message_dispatcher: event_dispatch::Dispatcher::default(),
            last_render_time: time::Instant::now(),
            frame_index: 0,
            gpu,
        })
    }

    pub fn window(&self) -> &Window {
        self.gpu.window()
    }

    pub fn event(&mut self, event: &WindowEvent, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(ref mut debug_overlay) = self.debug_overlay {
            debug_overlay.event(self.gpu.window(), event);
        }

        if !self.input(event) {
            match event {
                WindowEvent::RedrawRequested => {
                    let now = std::time::Instant::now();
                    let dt = now - self.last_render_time;
                    self.last_render_time = now;

                    self.update(now, dt, self.frame_index);

                    match self.gpu.surface.get_current_texture() {
                        Result::Ok(output) => {
                            let mut encoder = self.gpu.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor {
                                    label: Some("Render Encoder"),
                                },
                            );
                            self.render(&mut encoder, &output, self.frame_index as usize);
                            self.gpu.queue.submit(std::iter::once(encoder.finish()));
                            output.present();

                            self.frame_index = self.frame_index.wrapping_add(1);
                        }
                        Err(wgpu::SurfaceError::Lost) => {
                            let size = self.gpu.size();
                            self.resize(size);
                        }
                        // The system is out of memory, we should probably quit
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            panic!("wgpu::SurfaceError::OutOfMemory - bailing")
                        }
                        // All other errors (Outdated, Timeout) should be resolved by the next frame
                        Err(e) => log::error!("{:?}", e),
                    }
                }

                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::KeyboardInput {
                    event:
                        winit::event::KeyEvent {
                            state: winit::event::ElementState::Pressed,
                            physical_key:
                                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape),
                            ..
                        },
                    ..
                } => event_loop.exit(),

                WindowEvent::Resized(physical_size) => {
                    self.resize(*physical_size);
                }
                _ => {}
            }
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.gpu.resize(new_size);
        self.game_state
            .resize(self.gpu.window(), new_size, &self.gpu);
        self.game_ui.resize(self.gpu.window(), new_size, &self.gpu);
        self.lcd_filter
            .resize(self.gpu.window(), new_size, &self.gpu, &self.game_state);
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        if self
            .game_state
            .input(self.gpu.window(), event, self.game_ui.is_paused())
        {
            true
        } else {
            self.game_ui.input(self.gpu.window(), event)
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
            debug_overlay.update(self.gpu.window(), delta_time);
        }

        let game_dt = if self.game_ui.is_paused() {
            std::time::Duration::from_secs(0)
        } else {
            delta_time
        };

        self.audio.update(delta_time);

        {
            let mut ctx = AppContext {
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

        self.game_state.render(&mut self.gpu, encoder, frame_index);

        self.game_ui.render(&mut self.gpu, encoder, frame_index);

        self.lcd_filter
            .render(&mut self.gpu, output, encoder, frame_index);

        if let Some(ref mut debug_overlay) = self.debug_overlay {
            debug_overlay.render(
                &mut self.gpu,
                output,
                encoder,
                &mut self.game_state,
                &mut self.lcd_filter,
            );
        }
    }
}

impl<'a> event_dispatch::MessageHandler for AppState {
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
