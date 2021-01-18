pub mod constants;
pub mod events;
pub mod game_state;
pub mod gpu_state;
pub mod ui;

use game_state::GameState;
use gpu_state::GpuState;
use ui::InputHandler;
use winit::{event::WindowEvent, window::Window};

use self::constants::{MAX_CAMERA_SCALE, MIN_CAMERA_SCALE};

// --------------------------------------------------------------------------------------------------------------------

pub struct AppState {
    gpu: GpuState,
    game_state: GameState,

    // Imgui
    winit_platform: imgui_winit_support::WinitPlatform,
    imgui: imgui::Context,
    imgui_renderer: imgui_wgpu::Renderer,
}

impl AppState {
    pub fn new(window: &Window, mut gpu: GpuState) -> Self {
        let game_state = GameState::new(&mut gpu);

        //
        // set up imgui
        //

        let hidpi_factor = window.scale_factor();
        let mut imgui = imgui::Context::create();
        let mut winit_platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        winit_platform.attach_window(
            imgui.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );
        imgui.set_ini_filename(None);

        let font_size = (13.0 * hidpi_factor) as f32;
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        imgui
            .fonts()
            .add_font(&[imgui::FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: font_size,
                    ..Default::default()
                }),
            }]);

        let imgui_renderer = {
            imgui_wgpu::RendererConfig::new()
                .set_texture_format(gpu.sc_desc.format)
                .build(&mut imgui, &gpu.device, &gpu.queue)
        };

        Self {
            gpu,
            game_state,
            winit_platform,
            imgui,
            imgui_renderer,
        }
    }

    pub fn event(&mut self, window: &Window, event: &winit::event::Event<()>) {
        self.winit_platform
            .handle_event(self.imgui.io_mut(), &window, &event);
    }

    pub fn resize(&mut self, _window: &Window, new_size: winit::dpi::PhysicalSize<u32>) {
        self.gpu.resize(new_size);
        self.game_state.resize(new_size);
    }

    pub fn input(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.game_state.input(window, event)
    }

    pub fn update(&mut self, _window: &Window, dt: std::time::Duration) {
        self.imgui.io_mut().update_delta_time(dt);
        self.game_state.update(dt, &mut self.gpu);
    }

    pub fn render(&mut self, window: &Window) {
        let input_state = {
            let frame = self
                .gpu
                .swap_chain
                .get_current_frame()
                .expect("Timeout getting texture");

            let mut encoder =
                self.gpu
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    });

            //
            //  Let GameState render game contents
            //

            self.game_state.render(&mut self.gpu, &frame, &mut encoder);

            //
            //  Render ImGUI overlay
            //

            self.winit_platform
                .prepare_frame(self.imgui.io_mut(), window)
                .expect("Failed to prepare frame");

            let display_state = self.current_display_state();

            let ui = self.imgui.frame();
            let mut ui_input_state = ui::InputState::default();

            //
            // Build the UI, mutating ui_input_state to indicate user interaction.
            //

            imgui::Window::new(imgui::im_str!("Debug"))
                .size([280.0, 128.0], imgui::Condition::FirstUseEver)
                .build(&ui, || {
                    let mut camera_tracks_character = display_state.camera_tracks_character;
                    if ui.checkbox(
                        imgui::im_str!("Camera Tracks Character"),
                        &mut camera_tracks_character,
                    ) {
                        ui_input_state.camera_tracks_character = Some(camera_tracks_character);
                    }
                    ui.text(imgui::im_str!(
                        "camera: ({:.2},{:.2}) zoom: {:.2}",
                        display_state.camera_position.x,
                        display_state.camera_position.y,
                        display_state.zoom,
                    ));

                    ui.text(imgui::im_str!(
                        "character: ({:.2},{:.2}) cycle: {}",
                        display_state.character_position.x,
                        display_state.character_position.y,
                        display_state.character_cycle,
                    ));

                    let mut zoom = display_state.zoom;
                    if imgui::Slider::new(imgui::im_str!("Zoom"))
                        .range(MIN_CAMERA_SCALE..=MAX_CAMERA_SCALE as f32)
                        .build(&ui, &mut zoom)
                    {
                        ui_input_state.zoom = Some(zoom);
                    }

                    let mut draw_stage_collision_info = display_state.draw_stage_collision_info;
                    if ui.checkbox(
                        imgui::im_str!("Stage Collision Visible"),
                        &mut draw_stage_collision_info,
                    ) {
                        ui_input_state.draw_stage_collision_info = Some(draw_stage_collision_info);
                    }
                });

            //
            // Create and submit the render pass
            //

            self.winit_platform.prepare_render(&ui, &window);

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.output.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // Do not clear
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });

                self.imgui_renderer
                    .render(
                        ui.render(),
                        &self.gpu.queue,
                        &self.gpu.device,
                        &mut render_pass,
                    )
                    .expect("Imgui render failed");
            }

            let commands = encoder.finish();
            self.gpu.queue.submit(std::iter::once(commands));

            ui_input_state
        };

        self.process_input(&input_state);
    }
}

// ---------------------------------------------------------------------------------------------------------------------

impl InputHandler for AppState {
    fn current_display_state(&self) -> ui::DisplayState {
        let firebrand = self.game_state.get_firebrand();
        let position = firebrand.entity.position();
        let cc = &self.game_state.camera_controller;

        ui::DisplayState {
            camera_tracks_character: self.game_state.camera_tracks_character,
            camera_position: cc.camera.position(),
            zoom: cc.projection.scale(),
            character_position: position.xy(),
            draw_stage_collision_info: self.game_state.draw_stage_collision_info,
            character_cycle: firebrand.entity.sprite_cycle().to_string(),
        }
    }

    fn process_input(&mut self, ui_input_state: &ui::InputState) {
        if let Some(z) = ui_input_state.zoom {
            self.game_state.camera_controller.projection.set_scale(z);
        }
        if let Some(d) = ui_input_state.draw_stage_collision_info {
            self.game_state.draw_stage_collision_info = d;
        }
        if let Some(ctp) = ui_input_state.camera_tracks_character {
            self.game_state.camera_tracks_character = ctp;
        }
    }
}
