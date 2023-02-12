use cgmath::*;
use winit::window::Window;

use super::{
    constants::{MAX_CAMERA_SCALE, MIN_CAMERA_SCALE},
    game_state::GameState,
    gpu_state::GpuState,
    lcd_filter::LcdFilter,
};

/// UiStateInput is the input data which will be used to render an ImGUI UI
/// The method OverlayUi::create_ui_state_input is responsible for vending an
/// instance that represents the data needed to populate the ImGUI UI.
/// User interaction with the UI will be used to populate UiInteractionOutput.
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

/// UiInteractionOutput represents the values from UiSTateInput which changed
/// due to user interaction, which then need to be promoted to the game state.
/// Any non-None value is a value that changed, and must be handled in
/// OverlayUi::handle_ui_interaction_output
#[derive(Default)]
struct UiInteractionOutput {
    camera_tracks_character: Option<bool>,
    zoom: Option<f32>,
    draw_stage_collision_info: Option<bool>,
    draw_entity_debug: Option<bool>,
    lcd_hysteresis: Option<Option<std::time::Duration>>,
}

pub struct DebugOverlay {
    winit_platform: imgui_winit_support::WinitPlatform,
    imgui: imgui::Context,
    imgui_renderer: imgui_wgpu::Renderer,
}

impl DebugOverlay {
    pub fn new(window: &Window, gpu: &GpuState) -> Self {
        let mut imgui = imgui::Context::create();
        let mut winit_platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        winit_platform.attach_window(
            imgui.io_mut(),
            window,
            imgui_winit_support::HiDpiMode::Default,
        );
        imgui.set_ini_filename(None);

        let hidpi_factor = window.scale_factor();
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

        let renderer_config = imgui_wgpu::RendererConfig {
            texture_format: gpu.config.format,
            ..Default::default()
        };

        let imgui_renderer =
            imgui_wgpu::Renderer::new(&mut imgui, &gpu.device, &gpu.queue, renderer_config);

        Self {
            winit_platform,
            imgui,
            imgui_renderer,
        }
    }

    pub fn event(&mut self, window: &Window, event: &winit::event::Event<()>) {
        self.winit_platform
            .handle_event(self.imgui.io_mut(), window, event);
    }

    pub fn update(&mut self, _window: &Window, dt: std::time::Duration) {
        self.imgui.io_mut().update_delta_time(dt);
    }

    pub fn render(
        &mut self,
        window: &Window,
        gpu: &mut GpuState,
        output: &wgpu::SurfaceTexture,
        encoder: &mut wgpu::CommandEncoder,
        game_state: &mut GameState,
        lcd_filter: &mut LcdFilter,
    ) {
        self.winit_platform
            .prepare_frame(self.imgui.io_mut(), window)
            .expect("Failed to prepare frame");

        let display_state = self.create_ui_state_input(game_state, lcd_filter);
        let ui = self.imgui.frame();
        let mut ui_input_state = UiInteractionOutput::default();

        //
        // Build the UI, mutating ui_input_state to indicate user interaction.
        //

        ui.window("Debug")
            .size([280.0, 156.0], imgui::Condition::FirstUseEver)
            .build(|| {
                let mut camera_tracks_character = display_state.camera_tracks_character;
                if ui.checkbox("Camera Tracks Character", &mut camera_tracks_character) {
                    ui_input_state.camera_tracks_character = Some(camera_tracks_character);
                }
                ui.text(format!(
                    "camera: ({:.2},{:.2}) zoom: {:.2}",
                    display_state.camera_position.x,
                    display_state.camera_position.y,
                    display_state.zoom,
                ));

                ui.text(format!(
                    "character: ({:.2},{:.2}) cycle: {}",
                    display_state.character_position.x,
                    display_state.character_position.y,
                    display_state.character_cycle,
                ));

                let mut zoom = display_state.zoom;
                if ui.slider("Zoom", MIN_CAMERA_SCALE, MAX_CAMERA_SCALE, &mut zoom) {
                    ui_input_state.zoom = Some(zoom);
                }

                let mut draw_stage_collision_info = display_state.draw_stage_collision_info;
                if ui.checkbox("Stage Collision Visible", &mut draw_stage_collision_info) {
                    ui_input_state.draw_stage_collision_info = Some(draw_stage_collision_info);
                }

                let min_hysteresis_seconds: f32 = 0.0;
                let max_hysteresis_seconds: f32 = 0.5;
                let mut current_hysteresis_seconds = lcd_filter
                    .lcd_hysteresis()
                    .map_or_else(|| 0.0, |h| h.as_secs_f32());

                if ui.slider(
                    "LCD Hysteresis",
                    min_hysteresis_seconds,
                    max_hysteresis_seconds,
                    &mut current_hysteresis_seconds,
                ) {
                    ui_input_state.lcd_hysteresis = if current_hysteresis_seconds > 0.0 {
                        Some(Some(std::time::Duration::from_secs_f32(
                            current_hysteresis_seconds,
                        )))
                    } else {
                        Some(None)
                    };
                }
            });

        //
        // Create and submit the render pass
        //

        self.winit_platform.prepare_render(ui, window);

        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Debug Overlay Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Do not clear
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.imgui_renderer
                .render(
                    self.imgui.render(),
                    &gpu.queue,
                    &gpu.device,
                    &mut render_pass,
                )
                .expect("Imgui render failed");
        }

        self.handle_ui_interaction_output(&ui_input_state, game_state, lcd_filter);
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
