use cgmath::*;
use std::{path::Path, rc::Rc};

use winit::{event::WindowEvent, window::Window};

use crate::map;
use crate::texture;
use crate::Options;
use crate::{camera, sprite::rendering, state::gpu_state};

use super::constants::{layers, CAMERA_FAR_PLANE, CAMERA_NEAR_PLANE, MIN_CAMERA_SCALE};

pub struct GameUi {
    camera_view: camera::Camera,
    camera_projection: camera::Projection,
    camera_uniforms: camera::Uniforms,

    // ui tile graphics
    ui_material: Rc<rendering::Material>,
    ui_uniforms: rendering::Uniforms,
    ui_drawable: rendering::Drawable,

    // state
    time: f32,
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
        let ui_bg_sprites =
            ui_map.generate_sprites(ui_bg_layer, |_, _| layers::ui::BACKGROUND);
        let ui_bg_mesh =
            rendering::Mesh::new(&ui_bg_sprites, 0, &gpu.device, "UI background Sprite Mesh");
        let ui_bg_drawable = rendering::Drawable::with(ui_bg_mesh, ui_material.clone());

        Self {
            camera_view,
            camera_projection,
            camera_uniforms,
            ui_material,
            ui_uniforms,
            ui_drawable: ui_bg_drawable,
            time: 0.0,
        }
    }

    pub fn resize(&mut self, _window: &Window, new_size: winit::dpi::PhysicalSize<u32>) {
        self.camera_projection
            .resize(new_size.width, new_size.height);
    }

    pub fn input(&mut self, _window: &Window, _event: &WindowEvent) -> bool {
        false
    }

    pub fn update(
        &mut self,
        _window: &Window,
        dt: std::time::Duration,
        _gpu: &mut gpu_state::GpuState,
    ) {
        self.time += dt.as_secs_f32();
    }

    pub fn render(
        &mut self,
        _window: &Window,
        _gpu: &mut gpu_state::GpuState,
        _frame: &wgpu::SwapChainFrame,
        _encoder: &mut wgpu::CommandEncoder,
    ) {
    }
}
