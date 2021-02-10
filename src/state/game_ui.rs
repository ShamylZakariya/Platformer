use cgmath::*;
use std::{path::Path, rc::Rc, time::Duration};

use winit::{
    event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent},
    window::Window,
};

use crate::texture;
use crate::Options;
use crate::{camera, sprite::rendering, state::gpu_state};
use crate::{geom::lerp, map};

use super::{
    constants::{layers, CAMERA_FAR_PLANE, CAMERA_NEAR_PLANE, DEFAULT_CAMERA_SCALE},
    game_state,
};

// ---------------------------------------------------------------------------------------------------------------------

const DRAWER_OPEN_VEL: f32 = 2.0;

// ---------------------------------------------------------------------------------------------------------------------

pub struct GameUi {
    pipeline: wgpu::RenderPipeline,

    camera_view: camera::Camera,
    camera_projection: camera::Projection,
    camera_uniforms: camera::Uniforms,

    // ui tile graphics
    drawer_map: map::Map,
    material: Rc<rendering::Material>,
    uniforms: rendering::Uniforms,
    background_drawable: rendering::Drawable,

    // state
    time: f32,
    drawer_open: bool,
    drawer_open_progress: f32,
}

impl GameUi {
    pub fn new(gpu: &mut gpu_state::GpuState, _options: &Options) -> Self {
        // build camera
        let camera_view = camera::Camera::new((0.0, 0.0, 0.0), (0.0, 0.0, 1.0), None);
        let camera_uniforms = camera::Uniforms::new(&gpu.device);
        let camera_projection = camera::Projection::new(
            gpu.sc_desc.width,
            gpu.sc_desc.height,
            DEFAULT_CAMERA_SCALE * 2.0, // ui units are half size of game units
            CAMERA_NEAR_PLANE,
            CAMERA_FAR_PLANE,
        );

        // load game ui map and construct material/drawable etcs
        let ui_map = map::Map::new_tmx(Path::new("res/game_ui.tmx"));
        let map = ui_map.expect("Expected 'res/game_ui.tmx' to load");

        let bind_group_layout = rendering::Material::bind_group_layout(&gpu.device);

        let material = {
            let spritesheet_path = Path::new("res").join(&map.tileset.image_path);
            let spritesheet =
                texture::Texture::load(&gpu.device, &gpu.queue, spritesheet_path, false).unwrap();
            Rc::new(rendering::Material::new(
                &gpu.device,
                "UI Sprite Material",
                spritesheet,
                &bind_group_layout,
            ))
        };

        let uniforms =
            rendering::Uniforms::new(&gpu.device, map.tileset.get_sprite_size().cast().unwrap());

        let get_layer = |name: &str| {
            map.layer_named(name)
                .unwrap_or_else(|| panic!("Expect layer named \"{}\"", name))
        };

        let ui_bg_layer = get_layer("Background");
        let ui_bg_sprites = map.generate_sprites(ui_bg_layer, |_, _| layers::ui::BACKGROUND);
        let ui_bg_mesh =
            rendering::Mesh::new(&ui_bg_sprites, 0, &gpu.device, "UI background Sprite Mesh");
        let background_drawable = rendering::Drawable::with(ui_bg_mesh, material.clone());

        let ui_health_layer = get_layer("Health");
        let ui_health_sprites =
            map.generate_sprites(ui_health_layer, |_, _| layers::ui::FOREGROUND);

        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &bind_group_layout,
                    &camera_uniforms.bind_group_layout,
                    &uniforms.bind_group_layout,
                ],
                label: Some("GameUi Pipeline Layout"),
                push_constant_ranges: &[],
            });

        let pipeline = rendering::create_render_pipeline(
            &gpu.device,
            &pipeline_layout,
            gpu.sc_desc.format,
            Some(texture::Texture::DEPTH_FORMAT),
        );

        let mut game_ui = Self {
            pipeline,

            camera_view,
            camera_projection,
            camera_uniforms,

            drawer_map: map,
            material,
            uniforms,
            background_drawable,

            time: 0.0,
            drawer_open: false,
            drawer_open_progress: 0.0,
        };

        game_ui.update_drawer_position(Duration::from_secs(0));

        game_ui
    }

    pub fn resize(&mut self, _window: &Window, new_size: winit::dpi::PhysicalSize<u32>) {
        self.camera_projection
            .resize(new_size.width, new_size.height);
    }

    pub fn input(&mut self, _window: &Window, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    },
                ..
            } => match (key, state) {
                (VirtualKeyCode::F1, ElementState::Pressed) => {
                    self.drawer_open = !self.drawer_open;
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    pub fn update(
        &mut self,
        _window: &Window,
        dt: std::time::Duration,
        gpu: &mut gpu_state::GpuState,
        game: &game_state::GameState,
    ) {
        self.time += dt.as_secs_f32();

        // Canter camera on window, and set projection scale
        self.camera_view.set_position(point3(0.0, 0.0, 0.0));
        self.camera_projection
            .set_scale(game.camera_controller.projection.scale() * 2.0);

        self.camera_uniforms
            .data
            .update_view_proj(&self.camera_view, &self.camera_projection);
        self.camera_uniforms.write(&mut gpu.queue);

        // update ui uniforms
        let bounds = self.drawer_map.bounds();
        let drawer_y = self.update_drawer_position(dt);

        // let vp_units_high = self.camera_projection.viewport_size().y;
        // let drawer_y = (-vp_units_high/2.0) - 1.0;

        self.uniforms
            .data
            .set_color(vec4(1.0, 1.0, 1.0, 1.0))
            .set_model_position(point3(-bounds.width() / 2.0, drawer_y, 0.0));
        self.uniforms.write(&mut gpu.queue);
    }

    pub fn render(
        &mut self,
        _window: &Window,
        gpu: &mut gpu_state::GpuState,
        frame: &wgpu::SwapChainFrame,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.output.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                attachment: &gpu.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_pipeline(&self.pipeline);

        self.background_drawable
            .draw(&mut render_pass, &self.camera_uniforms, &self.uniforms);
    }
    // MARK: Public

    pub fn is_paused(&self) -> bool {
        self.drawer_open
    }

    // MARK: Private

    fn update_drawer_position(&mut self, dt: Duration) -> f32 {
        let bounds = self.drawer_map.bounds();
        let vp_units_high = self.camera_projection.viewport_size().y;
        let drawer_closed_y = (-vp_units_high / 2.0) - bounds.height() - 1.0 + 3.0;
        let drawer_open_y = (-vp_units_high / 2.0) - 1.0;

        if dt > Duration::from_secs(0) {
            if self.drawer_open {
                self.drawer_open_progress =
                    (self.drawer_open_progress + DRAWER_OPEN_VEL * dt.as_secs_f32()).min(1.0);
            } else {
                self.drawer_open_progress =
                    (self.drawer_open_progress - DRAWER_OPEN_VEL * dt.as_secs_f32()).max(0.0);
            }
        } else {
            self.drawer_open_progress = if self.drawer_open { 1.0 } else { 0.0 };
        }

        lerp(self.drawer_open_progress, drawer_closed_y, drawer_open_y)
    }
}
