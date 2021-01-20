use winit::{event::WindowEvent, window::Window};

use super::debug_overlay::DebugOverlay;

use super::{game_state::GameState, gpu_state::GpuState};

// --------------------------------------------------------------------------------------------------------------------

pub struct AppState {
    gpu: GpuState,
    game_state: GameState,
    overlay: DebugOverlay,
}

impl AppState {
    pub fn new(window: &Window, mut gpu: GpuState) -> Self {
        let game_state = GameState::new(&mut gpu);
        let overlay_ui = DebugOverlay::new(window, &gpu);

        Self {
            gpu,
            game_state,
            overlay: overlay_ui,
        }
    }

    pub fn event(&mut self, window: &Window, event: &winit::event::Event<()>) {
        self.overlay.event(window, event);
    }

    pub fn resize(&mut self, _window: &Window, new_size: winit::dpi::PhysicalSize<u32>) {
        self.gpu.resize(new_size);
        self.game_state.resize(new_size);
    }

    pub fn input(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.game_state.input(window, event)
    }

    pub fn update(&mut self, window: &Window, dt: std::time::Duration) {
        self.overlay.update(window, dt);
        self.game_state.update(dt, &mut self.gpu);
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

        self.game_state.render(&mut self.gpu, &frame, &mut encoder);
        self.overlay.render(
            window,
            &mut self.game_state,
            &mut self.gpu,
            &frame,
            &mut encoder,
        );

        let commands = encoder.finish();
        self.gpu.queue.submit(std::iter::once(commands));
    }
}
