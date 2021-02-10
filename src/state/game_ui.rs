use cgmath::*;
use std::{path::Path, rc::Rc, time::Duration};

use winit::{
    event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent},
    window::Window,
};

use crate::map;
use crate::texture;
use crate::Options;
use crate::{camera, sprite::rendering, state::gpu_state};

use super::constants::{layers, CAMERA_FAR_PLANE, CAMERA_NEAR_PLANE, MIN_CAMERA_SCALE};

// ---------------------------------------------------------------------------------------------------------------------

const DRAWER_OPEN_VEL: f32 = 8.0;

// ---------------------------------------------------------------------------------------------------------------------

pub struct GameUi {
    pipeline: wgpu::RenderPipeline,

    camera_view: camera::Camera,
    camera_projection: camera::Projection,
    camera_uniforms: camera::Uniforms,

    // ui tile graphics
    ui_map: map::Map,
    ui_material: Rc<rendering::Material>,
    ui_uniforms: rendering::Uniforms,
    ui_drawable: rendering::Drawable,

    // state
    time: f32,
    drawer_open: bool,
    drawer_y: f32,
}

impl GameUi {
    pub fn new(gpu: &mut gpu_state::GpuState, _options: &Options) -> Self {
        // build camera
        let camera_view = camera::Camera::new((0.0, 0.0, 0.0), (0.0, 0.0, 1.0), None);
        let camera_uniforms = camera::Uniforms::new(&gpu.device);
        let camera_projection = camera::Projection::new(
            gpu.sc_desc.width,
            gpu.sc_desc.height,
            MIN_CAMERA_SCALE,
            CAMERA_NEAR_PLANE,
            CAMERA_FAR_PLANE,
        );

        // load game ui map and construct material/drawable etcs
        let ui_map = map::Map::new_tmx(Path::new("res/game_ui.tmx"));
        let ui_map = ui_map.expect("Expected 'res/game_ui.tmx' to load");

        let bind_group_layout = rendering::Material::bind_group_layout(&gpu.device);

        let ui_material = {
            let spritesheet_path = Path::new("res").join(&ui_map.tileset.image_path);
            let spritesheet =
                texture::Texture::load(&gpu.device, &gpu.queue, spritesheet_path, false).unwrap();
            Rc::new(rendering::Material::new(
                &gpu.device,
                "UI Sprite Material",
                spritesheet,
                &bind_group_layout,
            ))
        };

        let ui_uniforms = rendering::Uniforms::new(
            &gpu.device,
            ui_map.tileset.get_sprite_size().cast().unwrap(),
        );

        let get_layer = |name: &str| {
            ui_map
                .layer_named(name)
                .unwrap_or_else(|| panic!("Expect layer named \"{}\"", name))
        };

        let ui_bg_layer = get_layer("Background");
        let ui_bg_sprites = ui_map.generate_sprites(ui_bg_layer, |_, _| layers::ui::BACKGROUND);
        let ui_bg_mesh =
            rendering::Mesh::new(&ui_bg_sprites, 0, &gpu.device, "UI background Sprite Mesh");
        let ui_bg_drawable = rendering::Drawable::with(ui_bg_mesh, ui_material.clone());

        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &bind_group_layout,
                    &camera_uniforms.bind_group_layout,
                    &ui_uniforms.bind_group_layout,
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

            ui_map,
            ui_material,
            ui_uniforms,
            ui_drawable: ui_bg_drawable,

            time: 0.0,
            drawer_open: false,
            drawer_y: 0.0,
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
    ) {
        self.time += dt.as_secs_f32();

        // Canter camera on window
        self.camera_view.set_position(point3(0.0, 0.0, 0.0));
        self.camera_projection.set_scale(MIN_CAMERA_SCALE * 2.0);
        self.camera_uniforms
            .data
            .update_view_proj(&self.camera_view, &self.camera_projection);
        self.camera_uniforms.write(&mut gpu.queue);

        // update ui uniforms
        let bounds = self.ui_map.bounds();
        let drawer_y = self.update_drawer_position(dt);
        self.ui_uniforms
            .data
            .set_color(vec4(1.0, 1.0, 1.0, 1.0))
            .set_model_position(point3(-bounds.width() / 2.0, drawer_y, 0.0));
        self.ui_uniforms.write(&mut gpu.queue);
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

        self.ui_drawable
            .draw(&mut render_pass, &self.camera_uniforms, &self.ui_uniforms);
    }
    // MARK: Public

    pub fn is_paused(&self) -> bool {
        self.drawer_open
    }

    // MARK: Private

    fn update_drawer_position(&mut self, dt: Duration) -> f32 {
        let bounds = self.ui_map.bounds();
        let target_y = if self.drawer_open {
            -bounds.height() - 1.0
        } else {
            -bounds.height() - 6.0
        };

        if dt > Duration::from_secs(0) {
            let dir = if target_y > self.drawer_y {
                1.0
            } else if target_y < self.drawer_y {
                -1.0
            } else {
                0.0
            };
            self.drawer_y += dir * DRAWER_OPEN_VEL * dt.as_secs_f32();
        } else {
            self.drawer_y = target_y;
        }

        self.drawer_y
    }
}
