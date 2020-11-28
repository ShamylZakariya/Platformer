use std::path::Path;
use std::rc::Rc;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyboardInput, MouseButton, WindowEvent},
    window::Window,
};

use crate::camera;
use crate::character_controller;
use crate::map;
use crate::sprite;
use crate::texture;
use crate::tileset;

// --------------------------------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
struct UiDisplayState {
    camera_position: cgmath::Point3<f32>,
    zoom: f32,
}

impl Default for UiDisplayState {
    fn default() -> Self {
        UiDisplayState {
            camera_position: [0.0, 0.0, 0.0].into(),
            zoom: 1.0,
        }
    }
}

struct UiInputState {
    zoom: Option<f32>,
}

impl Default for UiInputState {
    fn default() -> Self {
        UiInputState { zoom: None }
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub struct State {
    // Basic mechanism
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    depth_texture: texture::Texture,

    // Input state
    camera_controller: camera::CameraController,
    character_controller: character_controller::CharacterController,
    last_mouse_pos: PhysicalPosition<f64>,
    mouse_pressed: bool,

    // Camera
    camera: camera::Camera,
    projection: camera::Projection,
    camera_uniforms: camera::Uniforms,

    // Pipelines
    sprite_render_pipeline: wgpu::RenderPipeline,

    // Stage rendering
    stage_uniforms: sprite::Uniforms,
    stage_debug_draw_overlap_uniforms: sprite::Uniforms,
    stage_debug_draw_contact_uniforms: sprite::Uniforms,
    stage_sprite_collection: sprite::SpriteCollection,
    stage_hit_tester: sprite::SpriteHitTester,
    map: map::Map,

    // Entity rendering
    entity_material: Rc<sprite::SpriteMaterial>,
    firebrand_uniforms: sprite::Uniforms,
    firebrand: sprite::SpriteEntity,

    // Imgui
    winit_platform: imgui_winit_support::WinitPlatform,
    imgui: imgui::Context,
    imgui_renderer: imgui_wgpu::Renderer,
    ui_display_state: UiDisplayState,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    shader_validation: true,
                },
                None,
            )
            .await
            .unwrap();

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &sc_desc, "depth_texture");

        // Load the stage map
        let map = map::Map::new_tmx(Path::new("res/level_1.tmx"));
        let map = map.expect("Expected map to load");

        let material_bind_group_layout = sprite::SpriteMaterial::bind_group_layout(&device);
        let (stage_sprite_collection, stage_hit_tester) = {
            let mat = {
                let spritesheet_path = Path::new("res").join(&map.tileset.image_path);
                let spritesheet =
                    texture::Texture::load(&device, &queue, spritesheet_path, false).unwrap();
                sprite::SpriteMaterial::new(
                    &device,
                    "Sprite Material",
                    spritesheet,
                    &material_bind_group_layout,
                )
            };

            let bg_layer = map
                .layer_named("Background")
                .expect("Expected layer named 'Background'");
            let level_layer = map
                .layer_named("Level")
                .expect("Expected layer named 'Level'");

            let bg_sprites = map.generate_sprites(bg_layer, 1.0);
            let level_sprites = map.generate_sprites(level_layer, 0.9);
            let mut all_sprites = vec![];

            for s in &bg_sprites {
                all_sprites.push(s.clone());
            }

            for s in &level_sprites {
                all_sprites.push(s.clone());
            }

            let sm = sprite::SpriteMesh::new(&all_sprites, 0, &device, "Sprite Mesh");
            (
                sprite::SpriteCollection::new(vec![sm], vec![mat]),
                sprite::SpriteHitTester::new(&level_sprites),
            )
        };

        // Build camera, and camera uniform storage
        let camera = camera::Camera::new((8.0, 8.0, -1.0), (0.0, 0.0, 1.0), map.tileset.tile_width);
        let projection = camera::Projection::new(sc_desc.width, sc_desc.height, 16.0, 0.1, 100.0);
        let camera_controller = camera::CameraController::new(4.0);
        let character_controller =
            character_controller::CharacterController::new(&cgmath::Point2::new(1.0, 4.0));

        // place charatcer near first tree to help debug RATCHET collisions
        // character_controller.character_state.position.x = 23.0;
        // character_controller.character_state.position.y = 12.0;

        let mut camera_uniforms = camera::Uniforms::new(&device);
        camera_uniforms.data.update_view_proj(&camera, &projection);

        // Build the sprite render pipeline
        let sprite_size_px = cgmath::Vector2::new(
            map.tileset.tile_width as f32,
            map.tileset.tile_height as f32,
        );

        let stage_uniforms = sprite::Uniforms::new(&device, sprite_size_px);
        let stage_debug_draw_overlap_uniforms = sprite::Uniforms::new(&device, sprite_size_px);
        let stage_debug_draw_contact_uniforms = sprite::Uniforms::new(&device, sprite_size_px);

        let sprite_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &material_bind_group_layout,
                    &camera_uniforms.bind_group_layout,
                    &stage_uniforms.bind_group_layout,
                ],
                label: Some("Stage Sprite Pipeline Layout"),
                push_constant_ranges: &[],
            });

        let sprite_render_pipeline = sprite::create_render_pipeline(
            &device,
            &sprite_render_pipeline_layout,
            sc_desc.format,
            Some(texture::Texture::DEPTH_FORMAT),
        );

        // Entities

        let entity_tileset = tileset::TileSet::new_tsx("./res/entities.tsx")
            .expect("Expected to load entities tileset");

        let entity_material = Rc::new({
            let spritesheet_path = Path::new("res").join(&entity_tileset.image_path);
            let spritesheet =
                texture::Texture::load(&device, &queue, spritesheet_path, false).unwrap();

            sprite::SpriteMaterial::new(
                &device,
                "Sprite Material",
                spritesheet,
                &material_bind_group_layout,
            )
        });

        let firebrand_uniforms = sprite::Uniforms::new(&device, sprite_size_px);
        let firebrand = sprite::SpriteEntity::load(
            &entity_tileset,
            entity_material.clone(),
            &device,
            "firebrand",
            0,
        );

        // set up imgui

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

        let imgui_renderer = imgui_wgpu::RendererConfig::new()
            .set_texture_format(sc_desc.format)
            .build(&mut imgui, &device, &queue);

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            depth_texture,
            size,

            camera_controller,
            character_controller,
            last_mouse_pos: (0, 0).into(),
            mouse_pressed: false,

            camera,
            projection,
            camera_uniforms,

            sprite_render_pipeline,

            stage_uniforms,
            stage_debug_draw_overlap_uniforms,
            stage_debug_draw_contact_uniforms,
            stage_sprite_collection,
            stage_hit_tester,
            map,

            firebrand_uniforms: firebrand_uniforms,
            entity_material,
            firebrand,

            winit_platform,
            imgui,
            imgui_renderer,
            ui_display_state: UiDisplayState::default(),
        }
    }

    pub fn event(&mut self, window: &Window, event: &winit::event::Event<()>) {
        self.winit_platform
            .handle_event(self.imgui.io_mut(), &window, &event);
    }

    pub fn resize(&mut self, _window: &Window, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.depth_texture =
            texture::Texture::create_depth_texture(&self.device, &self.sc_desc, "depth_texture");
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.projection.resize(new_size.width, new_size.height);
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
            } => {
                self.character_controller.process_keyboard(*key, *state)
                    || self.camera_controller.process_keyboard(*key, *state)
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            WindowEvent::CursorMoved { position, .. } => {
                let mouse_dx = position.x - self.last_mouse_pos.x;
                let mouse_dy = position.y - self.last_mouse_pos.y;
                self.last_mouse_pos = *position;
                if self.mouse_pressed {
                    self.camera_controller.process_mouse(mouse_dx, mouse_dy);
                }
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, window: &Window, dt: std::time::Duration) {
        self.imgui.io_mut().update_delta_time(dt);

        // Update camera uniform state
        self.camera_controller
            .update_camera(&mut self.camera, &mut self.projection, dt);
        self.camera_uniforms
            .data
            .update_view_proj(&self.camera, &self.projection);

        self.camera_uniforms.write(&mut self.queue);

        // Update stage uniform state
        self.stage_uniforms
            .data
            .set_model_position(&cgmath::Point3::new(0.0, 0.0, 0.0));
        self.stage_uniforms
            .data
            .set_color(&cgmath::Vector4::new(1.0, 1.0, 1.0, 1.0));
        self.stage_uniforms.write(&mut self.queue);

        self.stage_debug_draw_overlap_uniforms
            .data
            .set_model_position(&cgmath::Point3::new(0.0, 0.0, -0.1)); // bring closer
        self.stage_debug_draw_overlap_uniforms
            .data
            .set_color(&cgmath::Vector4::new(0.0, 1.0, 0.0, 0.75));
        self.stage_debug_draw_overlap_uniforms
            .write(&mut self.queue);

        self.stage_debug_draw_contact_uniforms
            .data
            .set_model_position(&cgmath::Point3::new(0.0, 0.0, -0.2)); // bring closer
        self.stage_debug_draw_contact_uniforms
            .data
            .set_color(&cgmath::Vector4::new(1.0, 0.0, 0.0, 0.75));
        self.stage_debug_draw_contact_uniforms
            .write(&mut self.queue);

        // Update player character state
        let character_state = self.character_controller.update(dt, &self.stage_hit_tester);

        self.firebrand_uniforms
            .data
            .set_color(&cgmath::Vector4::new(1.0, 1.0, 1.0, 1.0));

        self.firebrand_uniforms
            .data
            .set_model_position(&cgmath::Point3::new(
                character_state.position.x,
                character_state.position.y,
                0.5,
            ));

        self.firebrand_uniforms.write(&mut self.queue);

        // Update UI
        self.update_ui_display_state(window, dt)
    }

    fn update_ui_display_state(&mut self, _window: &Window, _dt: std::time::Duration) {
        self.ui_display_state.camera_position = self.camera.position();
        self.ui_display_state.zoom = self.projection.scale();
    }

    pub fn render(&mut self, window: &Window) {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Timeout getting texture");

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        //
        // Render Sprites and entities; this is first pass so we clear color/depth
        //

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&self.sprite_render_pipeline);

            // Render stage
            self.stage_sprite_collection.draw(
                &mut render_pass,
                &self.camera_uniforms.bind_group,
                &self.stage_uniforms.bind_group,
            );

            if !self.character_controller.overlapping_sprites.is_empty() {
                self.stage_sprite_collection.draw_sprites(
                    &self.character_controller.overlapping_sprites,
                    &mut render_pass,
                    &self.camera_uniforms.bind_group,
                    &self.stage_debug_draw_overlap_uniforms.bind_group,
                );
            }

            if !self.character_controller.contacting_sprites.is_empty() {
                self.stage_sprite_collection.draw_sprites(
                    &self.character_controller.contacting_sprites,
                    &mut render_pass,
                    &self.camera_uniforms.bind_group,
                    &self.stage_debug_draw_contact_uniforms.bind_group,
                );
            }

            // Render player character
            self.firebrand.draw(
                &mut render_pass,
                &self.camera_uniforms.bind_group,
                &self.firebrand_uniforms.bind_group,
                self.character_controller.character_state.cycle,
            );
        }

        //
        //  ImGUI
        //

        {
            self.winit_platform
                .prepare_frame(self.imgui.io_mut(), window)
                .expect("Failed to prepare frame");

            let ui_input = self.render_ui(self.ui_display_state, &frame, &mut encoder, &window);
            self.process_ui_input(ui_input);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    // Renders imgui ui, and returns a UiInputState encapsulating user input.
    // The user input is consumed in process_ui_input.
    fn render_ui(
        &mut self,
        ui_display_state: UiDisplayState,
        frame: &wgpu::SwapChainFrame,
        encoder: &mut wgpu::CommandEncoder,
        window: &Window,
    ) -> UiInputState {
        let ui = self.imgui.frame();
        let mut ui_input_state = UiInputState::default();

        //
        // Build the UI, mutating ui_input_state to indicate user interaction.
        //

        imgui::Window::new(imgui::im_str!("Hello"))
            .size([280.0, 128.0], imgui::Condition::FirstUseEver)
            .build(&ui, || {
                ui.text(imgui::im_str!(
                    "camera: ({:.2},{:.2}) zoom: {:.2}",
                    ui_display_state.camera_position.x,
                    ui_display_state.camera_position.y,
                    ui_display_state.zoom,
                ));

                let mut zoom = ui_display_state.zoom;
                if imgui::Slider::new(imgui::im_str!("Zoom"))
                    .range(0 as f32..=999.0 as f32)
                    .build(&ui, &mut zoom)
                {
                    ui_input_state.zoom = Some(zoom);
                }
            });

        //
        // Create and submit the render pass
        //

        self.winit_platform.prepare_render(&ui, &window);

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
            .render(ui.render(), &self.queue, &self.device, &mut render_pass)
            .expect("Imgui render failed");

        ui_input_state
    }

    fn process_ui_input(&mut self, ui_input_state: UiInputState) {
        if let Some(z) = ui_input_state.zoom {
            self.projection.set_scale(z);
        }
    }
}
