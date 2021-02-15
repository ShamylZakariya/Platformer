use winit::{event::WindowEvent, window::Window};

use crate::Options;

use super::{
    debug_overlay::DebugOverlay, game_state::GameState, game_ui::GameUi, gpu_state::GpuState,
};

// --------------------------------------------------------------------------------------------------------------------

pub struct AppState {
    gpu: GpuState,
    game_state: GameState,
    game_ui: GameUi,
    overlay: Option<DebugOverlay>,
}

impl AppState {
    pub fn new(window: &Window, mut gpu: GpuState, options: Options) -> Self {
        let game_state = GameState::new(&mut gpu, &options);
        let mut game_ui = GameUi::new(&mut gpu, &options);
        let overlay_ui = if options.debug_overlay {
            Some(DebugOverlay::new(window, &gpu))
        } else {
            None
        };

        game_ui.show_start_message();

        Self {
            gpu,
            game_state,
            game_ui,
            overlay: overlay_ui,
        }
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
        if self.game_state.input(window, event) {
            true
        } else {
            self.game_ui.input(window, event)
        }
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

        self.game_state.update(window, game_dt, &mut self.gpu);
        self.game_ui
            .update(window, dt, &mut self.gpu, &self.game_state);
    }

    pub fn render(&mut self, window: &Window) {
        let frame = self
            .gpu
            .swap_chain
            .get_current_frame()
            .expect("Timeout getting texture");

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

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
