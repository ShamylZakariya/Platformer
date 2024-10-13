use cgmath::*;
use winit::window::Window;

use egui::Context;
use egui_wgpu::wgpu::{CommandEncoder, Device, Queue, StoreOp, TextureFormat, TextureView};
use egui_wgpu::{wgpu, Renderer, ScreenDescriptor};
use egui_winit::State;
use winit::event::WindowEvent;

use super::constants::{MAX_CAMERA_SCALE, MIN_CAMERA_SCALE};
use super::{
    // constants::{MAX_CAMERA_SCALE, MIN_CAMERA_SCALE},
    game_state::GameState,
    gpu_state::GpuState,
    lcd_filter::LcdFilter,
};

///////////////////////////////////////////////////////////////////////////////
// From https://github.com/kaphula/winit-egui-wgpu-template

pub struct EguiRenderer {
    state: State,
    renderer: Renderer,
    frame_started: bool,
}

impl EguiRenderer {
    pub fn context(&self) -> &Context {
        self.state.egui_ctx()
    }

    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: &Window,
    ) -> EguiRenderer {
        let egui_context = Context::default();

        let egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(2 * 1024), // default dimension is 2048
        );
        let egui_renderer = Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
            true,
        );

        EguiRenderer {
            state: egui_state,
            renderer: egui_renderer,
            frame_started: false,
        }
    }

    pub fn handle_input(&mut self, window: &Window, event: &WindowEvent) {
        let _ = self.state.on_window_event(window, event);
    }

    pub fn ppp(&mut self, v: f32) {
        self.context().set_pixels_per_point(v);
    }

    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.state.egui_ctx().begin_pass(raw_input);
        self.frame_started = true;
    }

    pub fn end_frame_and_draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
    ) {
        if !self.frame_started {
            panic!("begin_frame must be called before end_frame_and_draw can be called!");
        }

        self.ppp(screen_descriptor.pixels_per_point);

        let full_output = self.state.egui_ctx().end_pass();

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }
        self.renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);
        let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: window_surface_view,
                resolve_target: None,
                ops: egui_wgpu::wgpu::Operations {
                    load: egui_wgpu::wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            label: Some("egui main render pass"),
            occlusion_query_set: None,
        });

        self.renderer
            .render(&mut rpass.forget_lifetime(), &tris, &screen_descriptor);
        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }

        self.frame_started = false;
    }
}

///////////////////////////////////////////////////////////////////////////////

/// UiStateInput is the input data which will be used to render the UI.
/// The method DebugOverlay::create_ui_state_input is responsible for vending an
/// instance that represents the data needed to populate the UI.
/// User interaction with the UI will be used to populate UiInteractionOutput.
/// UiInteractionOutput will be used to modify game state, accordingly.
#[derive(Clone, Debug)]
struct UiStateInput {
    camera_tracks_character: bool,
    camera_position: Point3<f32>,
    zoom: f32,
    character_position: Point2<f32>,
    character_cycle: String,
    draw_stage_collision_info: bool,
    lcd_hysteresis: Option<std::time::Duration>,
}

/// UiInteractionOutput represents the values from UiStateInput which changed
/// due to user interaction, which then need to be promoted to the game state.
/// Any non-None value is a value that changed, and must be handled in
/// DebugOverlay::handle_ui_interaction_output
#[derive(Default)]
struct UiInteractionOutput {
    camera_tracks_character: Option<bool>,
    zoom: Option<f32>,
    draw_stage_collision_info: Option<bool>,
    draw_entity_debug: Option<bool>,
    lcd_hysteresis: Option<Option<std::time::Duration>>,
}

pub struct DebugOverlay {
    pub egui_renderer: EguiRenderer,
    pub scale_factor: f64,
}

impl DebugOverlay {
    pub fn new(window: &Window, gpu: &GpuState) -> Self {
        let egui_renderer = EguiRenderer::new(&gpu.device, gpu.config.format, None, 1, window);
        let scale_factor = window.scale_factor();

        Self {
            egui_renderer,
            scale_factor,
        }
    }

    pub fn event(&mut self, window: &Window, event: &winit::event::WindowEvent) {
        self.egui_renderer.handle_input(window, event);
    }

    pub fn update(&mut self, _window: &Window, _dt: std::time::Duration) {}

    pub fn render(
        &mut self,
        gpu: &mut GpuState,
        output: &wgpu::SurfaceTexture,
        encoder: &mut wgpu::CommandEncoder,
        game_state: &mut GameState,
        lcd_filter: &mut LcdFilter,
    ) {
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [gpu.config.width, gpu.config.height],
            pixels_per_point: gpu.window.scale_factor() as f32 * 1.0,
        };

        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.egui_renderer.begin_frame(&gpu.window);

        // Render the UI, handling user interaction

        let current_state = self.create_ui_state_input(game_state, lcd_filter);
        let result = self.render_gui(current_state);
        self.handle_ui_interaction_output(&result, game_state, lcd_filter);

        // Present the frame

        self.egui_renderer.end_frame_and_draw(
            &gpu.device,
            &gpu.queue,
            encoder,
            &gpu.window,
            &output_view,
            screen_descriptor,
        );
    }

    fn render_gui(&mut self, input: UiStateInput) -> UiInteractionOutput {
        let mut output = UiInteractionOutput::default();

        egui::Window::new("Knobs!")
            .resizable(true)
            .vscroll(true)
            .default_open(false)
            .auto_sized()
            .show(self.egui_renderer.context(), |ui| {
                ui.label(format!(
                    "camera: ({:.2},{:.2}) zoom: {:.2}",
                    input.camera_position.x, input.camera_position.y, input.camera_position.z
                ));
                ui.label(format!(
                    "character: ({:.2},{:.2})",
                    input.character_position.x, input.character_position.y
                ));
                ui.end_row();

                let mut camera_tracks_character = input.camera_tracks_character;
                if ui
                    .checkbox(&mut camera_tracks_character, "Camera Follows")
                    .clicked()
                {
                    output.camera_tracks_character = Some(camera_tracks_character);
                }
                ui.end_row();

                let mut zoom = input.zoom;
                ui.add(egui::Slider::new(
                    &mut zoom,
                    MIN_CAMERA_SCALE..=MAX_CAMERA_SCALE,
                ));
                if (zoom - input.zoom).abs() > 1e-5 {
                    output.zoom = Some(zoom);
                }
                ui.end_row();

                let mut draw_collision_info = input.draw_stage_collision_info;
                if ui
                    .checkbox(&mut draw_collision_info, "Draw Colliders")
                    .clicked()
                {
                    output.draw_stage_collision_info = Some(draw_collision_info);
                }
                ui.end_row();

                let min_hysteresis_seconds: f32 = 0.0;
                let max_hysteresis_seconds: f32 = 0.5;
                let current_hysteresis_seconds = input
                    .lcd_hysteresis
                    .map_or_else(|| 0.0, |h| h.as_secs_f32());
                let mut new_hysteresis_seconds = current_hysteresis_seconds;

                ui.label("LCD Sludge");
                ui.add(egui::Slider::new(
                    &mut new_hysteresis_seconds,
                    min_hysteresis_seconds..=max_hysteresis_seconds,
                ));
                if (current_hysteresis_seconds - new_hysteresis_seconds).abs() > 1e-5 {
                    output.lcd_hysteresis = if new_hysteresis_seconds > 0.0 {
                        Some(Some(std::time::Duration::from_secs_f32(
                            new_hysteresis_seconds,
                        )))
                    } else {
                        Some(None)
                    };
                }
                ui.end_row();
            });
        output
    }

    fn create_ui_state_input(
        &self,
        game_state: &GameState,
        lcd_filter: &LcdFilter,
    ) -> UiStateInput {
        let cc = &game_state.camera_controller;
        if let Some(firebrand) = game_state.try_get_firebrand() {
            let position = firebrand.entity.position();

            UiStateInput {
                camera_tracks_character: game_state.camera_tracks_character,
                camera_position: cc.camera.position(),
                zoom: cc.projection.scale(),
                character_position: position.xy(),
                draw_stage_collision_info: game_state.draw_stage_collision_info,
                character_cycle: firebrand.entity.sprite_cycle().to_string(),
                lcd_hysteresis: lcd_filter.lcd_hysteresis(),
            }
        } else {
            UiStateInput {
                camera_tracks_character: game_state.camera_tracks_character,
                camera_position: cc.camera.position(),
                zoom: cc.projection.scale(),
                character_position: point2(0.0, 0.0),
                draw_stage_collision_info: game_state.draw_stage_collision_info,
                character_cycle: "<none>".to_owned(),
                lcd_hysteresis: lcd_filter.lcd_hysteresis(),
            }
        }
    }

    fn handle_ui_interaction_output(
        &mut self,
        ui_input_state: &UiInteractionOutput,
        game_state: &mut GameState,
        lcd_filter: &mut LcdFilter,
    ) {
        if let Some(z) = ui_input_state.zoom {
            game_state.camera_controller.projection.set_scale(z);
        }
        if let Some(d) = ui_input_state.draw_stage_collision_info {
            game_state.draw_stage_collision_info = d;
        }
        if let Some(ctp) = ui_input_state.camera_tracks_character {
            game_state.camera_tracks_character = ctp;
        }
        if let Some(lcd_hysteresis) = ui_input_state.lcd_hysteresis {
            lcd_filter.set_lcd_hysteresis(lcd_hysteresis);
        }
    }
}
