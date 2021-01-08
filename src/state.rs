use std::rc::Rc;
use std::{path::Path, time::Duration};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyboardInput, MouseButton, WindowEvent},
    window::Window,
};

use crate::camera;
use crate::entity;
use crate::map;
use crate::sprite::collision;
use crate::sprite::rendering;
use crate::texture;
use crate::tileset;

use crate::camera::Uniforms as CameraUniforms;
use crate::sprite::rendering::Drawable as SpriteDrawable;
use crate::sprite::rendering::Uniforms as SpriteUniforms;

// --------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct UiDisplayState {
    camera_tracks_character: bool,
    camera_position: cgmath::Point3<f32>,
    zoom: f32,
    character_position: cgmath::Point2<f32>,
    character_cycle: String,
    draw_stage_collision_info: bool,
}

impl Default for UiDisplayState {
    fn default() -> Self {
        UiDisplayState {
            camera_tracks_character: true,
            camera_position: [0.0, 0.0, 0.0].into(),
            zoom: 1.0,
            character_position: [0.0, 0.0].into(),
            character_cycle: "".to_string(),
            draw_stage_collision_info: true,
        }
    }
}

#[derive(Default)]
struct UiInputState {
    camera_tracks_character: Option<bool>,
    zoom: Option<f32>,
    draw_stage_collision_info: Option<bool>,
    draw_entity_debug: Option<bool>,
}

struct EntityComponents {
    entity: Box<dyn entity::Entity>,
    sprite: crate::sprite::rendering::EntityDrawable,
    uniforms: SpriteUniforms,
}

impl EntityComponents {
    fn new(
        entity: Box<dyn entity::Entity>,
        sprite: crate::sprite::rendering::EntityDrawable,
        uniforms: SpriteUniforms,
    ) -> Self {
        Self {
            entity,
            sprite,
            uniforms,
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

struct FlipbookAnimationComponents {
    flipbook_animation: rendering::FlipbookAnimationDrawable,
    uniforms: SpriteUniforms,
    seconds_until_next_frame: f32,
    current_frame: usize,
}

impl FlipbookAnimationComponents {
    fn new(flipbook: rendering::FlipbookAnimationDrawable, uniforms: SpriteUniforms) -> Self {
        let seconds_until_next_frame = flipbook.duration_for_frame(0).as_secs_f32();
        Self {
            flipbook_animation: flipbook,
            uniforms,
            seconds_until_next_frame,
            current_frame: 0,
        }
    }

    fn update(&mut self, dt: Duration) {
        let dt = dt.as_secs_f32();
        self.seconds_until_next_frame -= dt;
        if self.seconds_until_next_frame <= 0.0 {
            self.current_frame += 1;
            self.seconds_until_next_frame = self
                .flipbook_animation
                .duration_for_frame(self.current_frame)
                .as_secs_f32();

            self.flipbook_animation
                .set_frame(&mut self.uniforms, self.current_frame);
        }
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
    last_mouse_pos: PhysicalPosition<f64>,
    mouse_pressed: bool,

    // Camera
    camera: camera::Camera,
    projection: camera::Projection,
    camera_uniforms: CameraUniforms,

    // Pipelines
    sprite_render_pipeline: wgpu::RenderPipeline,

    // Stage rendering
    stage_uniforms: SpriteUniforms,
    stage_debug_draw_overlap_uniforms: SpriteUniforms,
    stage_debug_draw_contact_uniforms: SpriteUniforms,
    stage_sprite_drawable: SpriteDrawable,
    map: map::Map,

    // Collision detection and dispatch
    collision_space: collision::Space,
    message_dispatcher: entity::Dispatcher,

    // Entity rendering
    entity_material: Rc<crate::sprite::rendering::Material>,
    entities: Vec<EntityComponents>,
    firebrand_entity_id: usize,

    // Flipbook animations
    flipbook_animations: Vec<FlipbookAnimationComponents>,

    // Imgui
    winit_platform: imgui_winit_support::WinitPlatform,
    imgui: imgui::Context,
    imgui_renderer: imgui_wgpu::Renderer,

    // Toggles
    draw_stage_collision_info: bool,
    camera_tracks_character: bool,
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
        let sprite_size_px = cgmath::vec2(
            map.tileset.tile_width as f32,
            map.tileset.tile_height as f32,
        );

        let material_bind_group_layout =
            crate::sprite::rendering::Material::bind_group_layout(&device);
        let (
            stage_sprite_material,
            stage_sprite_drawable,
            stage_hit_tester,
            entities,
            stage_animation_flipbooks,
        ) = {
            let stage_sprite_material = {
                let spritesheet_path = Path::new("res").join(&map.tileset.image_path);
                let spritesheet =
                    texture::Texture::load(&device, &queue, spritesheet_path, false).unwrap();
                Rc::new(crate::sprite::rendering::Material::new(
                    &device,
                    "Sprite Material",
                    spritesheet,
                    &material_bind_group_layout,
                ))
            };

            let bg_layer = map
                .layer_named("Background")
                .expect("Expected layer named 'Background'");
            let level_layer = map
                .layer_named("Level")
                .expect("Expected layer named 'Level'");
            let entity_layer = map
                .layer_named("Entities")
                .expect("Expected layer named 'Entities'");

            // generate level sprites
            let bg_sprites = map.generate_sprites(bg_layer, |_, _| 1.0);
            let level_sprites = map.generate_sprites(level_layer, |_sprite, tile| {
                if tile.get_property("foreground") == Some("true") {
                    0.1
                } else {
                    0.9
                }
            });

            // generate level entities
            let mut entity_id_vendor = entity::IdVendor::default();
            let mut collision_space = collision::Space::new(&level_sprites);
            let entities = map.generate_entities(
                entity_layer,
                &mut collision_space,
                &mut entity_id_vendor,
                |_, _| 0.9,
            );

            // generate animations
            let stage_animation_flipbooks = map.generate_animations(bg_layer, |_, _| 0.9);

            let mut all_sprites = vec![];
            all_sprites.extend(bg_sprites);
            all_sprites.extend(level_sprites.clone());

            let sm = crate::sprite::rendering::Mesh::new(&all_sprites, 0, &device, "Sprite Mesh");
            (
                stage_sprite_material.clone(),
                SpriteDrawable::with(sm, stage_sprite_material.clone()),
                collision_space,
                entities,
                stage_animation_flipbooks,
            )
        };

        // Build camera, and camera uniform storage
        let map_origin = cgmath::Point2::new(0.0, 0.0);
        let map_extent = cgmath::Vector2::new(map.width as f32, map.height as f32);
        let camera = camera::Camera::new((8.0, 8.0, -1.0), (0.0, 0.0, 1.0), None);
        let projection = camera::Projection::new(sc_desc.width, sc_desc.height, 16.0, 0.1, 100.0);
        let camera_controller = camera::CameraController::new(4.0, map_origin, map_extent);

        let mut camera_uniforms = CameraUniforms::new(&device);
        camera_uniforms.data.update_view_proj(&camera, &projection);

        // Build the sprite render pipeline

        let stage_uniforms = SpriteUniforms::new(&device, sprite_size_px);
        let stage_debug_draw_overlap_uniforms = SpriteUniforms::new(&device, sprite_size_px);
        let stage_debug_draw_contact_uniforms = SpriteUniforms::new(&device, sprite_size_px);

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

        let sprite_render_pipeline = crate::sprite::rendering::create_render_pipeline(
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

            crate::sprite::rendering::Material::new(
                &device,
                "Sprite Material",
                spritesheet,
                &material_bind_group_layout,
            )
        });

        let mut entity_components = vec![];
        let mut firebrand_entity_id: usize = 0;

        for (i, e) in entities.into_iter().enumerate() {
            if e.sprite_name() == "firebrand" {
                firebrand_entity_id = i;
            }

            let name = e.sprite_name().to_string();
            entity_components.push(EntityComponents::new(
                e,
                crate::sprite::rendering::EntityDrawable::load(
                    &entity_tileset,
                    entity_material.clone(),
                    &device,
                    &name,
                    0,
                ),
                SpriteUniforms::new(&device, sprite_size_px),
            ));
        }

        let flipbook_animations = stage_animation_flipbooks
            .into_iter()
            .map(|a| {
                rendering::FlipbookAnimationDrawable::new(a, stage_sprite_material.clone(), &device)
            })
            .map(|a| {
                FlipbookAnimationComponents::new(a, SpriteUniforms::new(&device, sprite_size_px))
            })
            .collect::<Vec<_>>();

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
            last_mouse_pos: (0, 0).into(),
            mouse_pressed: false,

            camera,
            projection,
            camera_uniforms,

            sprite_render_pipeline,

            stage_uniforms,
            stage_debug_draw_overlap_uniforms,
            stage_debug_draw_contact_uniforms,
            stage_sprite_drawable,
            map,

            collision_space: stage_hit_tester,
            message_dispatcher: entity::Dispatcher::default(),

            entity_material,
            entities: entity_components,
            firebrand_entity_id,

            flipbook_animations,

            winit_platform,
            imgui,
            imgui_renderer,

            draw_stage_collision_info: false,
            camera_tracks_character: true,
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
                let mut consumed = false;
                for e in &mut self.entities {
                    if e.entity.process_keyboard(*key, *state) {
                        consumed = true;
                    }
                }
                consumed || self.camera_controller.process_keyboard(*key, *state)
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

    pub fn update(&mut self, _window: &Window, dt: std::time::Duration) {
        self.imgui.io_mut().update_delta_time(dt);

        // Update stage uniform state
        self.stage_uniforms
            .data
            .set_model_position(&cgmath::Point3::new(0.0, 0.0, 0.0));
        self.stage_uniforms
            .data
            .set_color(&cgmath::vec4(1.0, 1.0, 1.0, 1.0));
        self.stage_uniforms.write(&mut self.queue);

        self.stage_debug_draw_overlap_uniforms
            .data
            .set_model_position(&cgmath::Point3::new(0.0, 0.0, -0.1)); // bring closer
        self.stage_debug_draw_overlap_uniforms
            .data
            .set_color(&cgmath::vec4(0.0, 1.0, 0.0, 0.75));
        self.stage_debug_draw_overlap_uniforms
            .write(&mut self.queue);

        self.stage_debug_draw_contact_uniforms
            .data
            .set_model_position(&cgmath::Point3::new(0.0, 0.0, -0.2)); // bring closer
        self.stage_debug_draw_contact_uniforms
            .data
            .set_color(&cgmath::vec4(1.0, 0.0, 0.0, 0.75));
        self.stage_debug_draw_contact_uniforms
            .write(&mut self.queue);

        //
        //  Update entities
        //

        for e in &mut self.entities {
            if e.entity.is_alive() {
                e.entity
                    .update(dt, &mut self.collision_space, &mut self.message_dispatcher);
                e.entity.update_uniforms(&mut e.uniforms);
                e.uniforms.write(&mut self.queue);
            }
        }

        //
        //  Update flipbook animations
        //

        for a in &mut self.flipbook_animations {
            a.update(dt);
            a.uniforms.write(&mut self.queue);
        }

        //
        // Update camera state
        //

        self.camera_controller
            .update_camera(&mut self.camera, &mut self.projection, dt);
        self.camera_uniforms
            .data
            .update_view_proj(&self.camera, &self.projection);

        if self.camera_tracks_character {
            let cp = self.camera.position();
            let p = self.entities[self.firebrand_entity_id as usize]
                .entity
                .position();
            self.camera
                .set_position(&cgmath::Point3::new(p.x, p.y, cp.z));
        }

        self.camera_uniforms.write(&mut self.queue);

        //
        // Dispatch collected messages
        //

        for m in &self.message_dispatcher.messages {
            self.entities[m.entity_id as usize].entity.handle_message(m);
        }
        self.message_dispatcher.clear();
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
            self.stage_sprite_drawable.draw(
                &mut render_pass,
                &self.camera_uniforms,
                &self.stage_uniforms,
            );

            // Render flipbook animations
            for a in &self.flipbook_animations {
                a.flipbook_animation
                    .draw(&mut render_pass, &self.camera_uniforms, &a.uniforms);
            }

            if self.draw_stage_collision_info {
                for e in &self.entities {
                    if let Some(overlapping) = e.entity.overlapping_sprites() {
                        self.stage_sprite_drawable.draw_sprites(
                            overlapping,
                            &mut render_pass,
                            &self.camera_uniforms,
                            &self.stage_debug_draw_overlap_uniforms,
                        );
                    }

                    if let Some(contacting) = e.entity.contacting_sprites() {
                        self.stage_sprite_drawable.draw_sprites(
                            contacting,
                            &mut render_pass,
                            &self.camera_uniforms,
                            &self.stage_debug_draw_contact_uniforms,
                        );
                    }
                }
            }

            // render entities
            for e in &self.entities {
                if e.entity.is_alive() && e.entity.should_draw() {
                    e.sprite.draw(
                        &mut render_pass,
                        &self.camera_uniforms,
                        &e.uniforms,
                        e.entity.sprite_cycle(),
                    );
                }
            }
        }

        //
        //  ImGUI
        //

        {
            self.winit_platform
                .prepare_frame(self.imgui.io_mut(), window)
                .expect("Failed to prepare frame");

            let ui_input = self.render_ui(
                self.current_ui_display_state(),
                &frame,
                &mut encoder,
                &window,
            );
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

        imgui::Window::new(imgui::im_str!("Debug"))
            .size([280.0, 128.0], imgui::Condition::FirstUseEver)
            .build(&ui, || {
                let mut camera_tracks_character = ui_display_state.camera_tracks_character;
                if ui.checkbox(
                    imgui::im_str!("Camera Tracks Character"),
                    &mut camera_tracks_character,
                ) {
                    ui_input_state.camera_tracks_character = Some(camera_tracks_character);
                }
                ui.text(imgui::im_str!(
                    "camera: ({:.2},{:.2}) zoom: {:.2}",
                    ui_display_state.camera_position.x,
                    ui_display_state.camera_position.y,
                    ui_display_state.zoom,
                ));

                ui.text(imgui::im_str!(
                    "character: ({:.2},{:.2}) cycle: {}",
                    ui_display_state.character_position.x,
                    ui_display_state.character_position.y,
                    ui_display_state.character_cycle,
                ));

                let mut zoom = ui_display_state.zoom;
                if imgui::Slider::new(imgui::im_str!("Zoom"))
                    .range(0 as f32..=999.0 as f32)
                    .build(&ui, &mut zoom)
                {
                    ui_input_state.zoom = Some(zoom);
                }

                let mut draw_stage_collision_info = ui_display_state.draw_stage_collision_info;
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

    fn current_ui_display_state(&self) -> UiDisplayState {
        UiDisplayState {
            camera_tracks_character: self.camera_tracks_character,
            camera_position: self.camera.position(),
            zoom: self.projection.scale(),
            character_position: self.entities[self.firebrand_entity_id as usize]
                .entity
                .position(),
            draw_stage_collision_info: self.draw_stage_collision_info,
            character_cycle: self.entities[self.firebrand_entity_id as usize]
                .entity
                .sprite_cycle()
                .to_string(),
        }
    }

    fn process_ui_input(&mut self, ui_input_state: UiInputState) {
        if let Some(z) = ui_input_state.zoom {
            self.projection.set_scale(z);
        }
        if let Some(d) = ui_input_state.draw_stage_collision_info {
            self.draw_stage_collision_info = d;
        }
        if let Some(ctp) = ui_input_state.camera_tracks_character {
            self.camera_tracks_character = ctp;
        }
    }
}
