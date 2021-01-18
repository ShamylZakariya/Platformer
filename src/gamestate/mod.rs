use cgmath::*;
use core::panic;
use entities::EntityClass;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};
use std::{path::Path, time::Duration};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyboardInput, MouseButton, WindowEvent},
    window::Window,
};

use crate::event_dispatch::*;
use crate::map;
use crate::sprite::rendering;
use crate::texture;
use crate::tileset;
use crate::{camera, entities};
use crate::{
    entity::{self, EntityComponents},
    sprite::collision,
};

use crate::camera::Uniforms as CameraUniforms;
use crate::sprite::rendering::Drawable as SpriteDrawable;
use crate::sprite::rendering::Uniforms as SpriteUniforms;

pub mod constants;
pub mod events;
use constants::sprite_layers;

use self::{
    constants::{MAX_CAMERA_SCALE, MIN_CAMERA_SCALE, ORIGINAL_VIEWPORT_TILES_WIDE},
    events::Event,
};

// --------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct UiDisplayState {
    camera_tracks_character: bool,
    camera_position: Point3<f32>,
    zoom: f32,
    character_position: Point2<f32>,
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
    last_mouse_pos: PhysicalPosition<f64>,
    mouse_pressed: bool,

    // Camera
    camera_controller: camera::CameraController,

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
    message_dispatcher: Dispatcher,

    // Entity rendering
    entity_id_vendor: entity::IdVendor,
    entity_tileset: tileset::TileSet,
    entity_material: Rc<crate::sprite::rendering::Material>,
    entities: HashMap<u32, EntityComponents>,
    firebrand_entity_id: u32,
    visible_entities: HashSet<u32>,

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
        let mut entity_id_vendor = entity::IdVendor::default();
        let map = map::Map::new_tmx(Path::new("res/level_1.tmx"));
        let map = map.expect("Expected map to load");
        let sprite_size_px = vec2(
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
            let bg_sprites = map.generate_sprites(bg_layer, |_, _| sprite_layers::BACKGROUND);
            let level_sprites = map.generate_sprites(level_layer, |_sprite, tile| {
                if tile.get_property("foreground") == Some("true") {
                    sprite_layers::FOREGROUND
                } else {
                    sprite_layers::LEVEL
                }
            });

            // generate level entities
            let mut collision_space = collision::Space::new(&level_sprites);
            let entities = map.generate_entities(
                entity_layer,
                &mut collision_space,
                &mut entity_id_vendor,
                |_, _| sprite_layers::ENTITIES,
            );

            // generate animations
            let stage_animation_flipbooks =
                map.generate_animations(bg_layer, |_, _| sprite_layers::BACKGROUND);

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
        let map_origin = point2(0.0, 0.0);
        let map_extent = vec2(map.width as f32, map.height as f32);
        let camera = camera::Camera::new((8.0, 8.0, -1.0), (0.0, 0.0, 1.0), None);
        let projection = camera::Projection::new(sc_desc.width, sc_desc.height, 16.0, 0.1, 100.0);
        let camera_uniforms = CameraUniforms::new(&device);
        let camera_controller = camera::CameraController::new(
            camera,
            projection,
            camera_uniforms,
            map_origin,
            map_extent,
        );

        // Build the sprite render pipeline

        let stage_uniforms = SpriteUniforms::new(&device, sprite_size_px);
        let stage_debug_draw_overlap_uniforms = SpriteUniforms::new(&device, sprite_size_px);
        let stage_debug_draw_contact_uniforms = SpriteUniforms::new(&device, sprite_size_px);

        let sprite_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &material_bind_group_layout,
                    &camera_controller.uniforms.bind_group_layout,
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

        let mut entity_components = HashMap::new();
        let mut firebrand_entity_id: u32 = 0;

        for e in entities.into_iter() {
            if e.sprite_name() == "firebrand" {
                firebrand_entity_id = e.entity_id();
            }

            let name = e.sprite_name().to_string();
            entity_components.insert(
                e.entity_id(),
                EntityComponents::new(
                    e,
                    crate::sprite::rendering::EntityDrawable::load(
                        &entity_tileset,
                        entity_material.clone(),
                        &device,
                        &name,
                        0,
                    ),
                    SpriteUniforms::new(&device, sprite_size_px),
                ),
            );
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

            last_mouse_pos: (0, 0).into(),
            mouse_pressed: false,

            camera_controller,

            sprite_render_pipeline,

            stage_uniforms,
            stage_debug_draw_overlap_uniforms,
            stage_debug_draw_contact_uniforms,
            stage_sprite_drawable,
            map,

            collision_space: stage_hit_tester,
            message_dispatcher: Dispatcher::default(),

            entity_id_vendor,
            entity_tileset,
            entity_material,
            entities: entity_components,
            firebrand_entity_id,
            visible_entities: HashSet::new(),

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
        self.camera_controller
            .projection
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
            } => {
                let mut consumed = false;
                for e in self.entities.values_mut() {
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
                self.camera_controller.mouse_movement(
                    self.mouse_pressed,
                    point2(position.x, position.y).cast().unwrap(),
                    vec2(mouse_dx, mouse_dy).cast().unwrap(),
                );
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
            .set_model_position(point3(0.0, 0.0, 0.0));
        self.stage_uniforms.data.set_color(vec4(1.0, 1.0, 1.0, 1.0));
        self.stage_uniforms.write(&mut self.queue);

        self.stage_debug_draw_overlap_uniforms
            .data
            .set_model_position(point3(0.0, 0.0, -0.1)); // bring closer
        self.stage_debug_draw_overlap_uniforms
            .data
            .set_color(vec4(0.0, 1.0, 0.0, 0.75));
        self.stage_debug_draw_overlap_uniforms
            .write(&mut self.queue);

        self.stage_debug_draw_contact_uniforms
            .data
            .set_model_position(point3(0.0, 0.0, -0.2)); // bring closer
        self.stage_debug_draw_contact_uniforms
            .data
            .set_color(vec4(1.0, 0.0, 0.0, 0.75));
        self.stage_debug_draw_contact_uniforms
            .write(&mut self.queue);

        //
        //  Update entities - if any are expired, remove them.
        //

        {
            let mut expired_count = 0;
            for e in self.entities.values_mut() {
                e.entity.update(
                    dt,
                    &self.map,
                    &mut self.collision_space,
                    &mut self.message_dispatcher,
                );
                e.entity.update_uniforms(&mut e.uniforms);
                e.uniforms.write(&mut self.queue);

                if !e.entity.is_alive() {
                    expired_count += 1;
                }
            }

            if expired_count > 0 {
                self.entities.retain(|_, e| e.entity.is_alive())
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

        let tracking = if self.camera_tracks_character {
            Some(
                self.entities
                    .get(&self.firebrand_entity_id)
                    .expect("firebrand_entity_id should be the player's entity_id")
                    .entity
                    .position()
                    .xy(),
            )
        } else {
            None
        };

        self.camera_controller.update(dt, tracking);
        self.camera_controller.uniforms.write(&mut self.queue);

        //
        //  Notify entities of their visibility
        //

        self.update_entity_visibility();

        //
        // Dispatch collected messages
        //

        Dispatcher::dispatch(&self.message_dispatcher.drain(), self);
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
                &self.camera_controller.uniforms,
                &self.stage_uniforms,
            );

            // Render flipbook animations
            for a in &self.flipbook_animations {
                a.flipbook_animation.draw(
                    &mut render_pass,
                    &self.camera_controller.uniforms,
                    &a.uniforms,
                );
            }

            if self.draw_stage_collision_info {
                for e in self.entities.values() {
                    if let Some(overlapping) = e.entity.overlapping_sprites() {
                        self.stage_sprite_drawable.draw_sprites(
                            overlapping,
                            &mut render_pass,
                            &self.camera_controller.uniforms,
                            &self.stage_debug_draw_overlap_uniforms,
                        );
                    }

                    if let Some(contacting) = e.entity.contacting_sprites() {
                        self.stage_sprite_drawable.draw_sprites(
                            contacting,
                            &mut render_pass,
                            &self.camera_controller.uniforms,
                            &self.stage_debug_draw_contact_uniforms,
                        );
                    }
                }
            }

            // render entities
            for e in self.entities.values() {
                if e.entity.is_alive() && e.entity.should_draw() {
                    e.sprite.draw(
                        &mut render_pass,
                        &self.camera_controller.uniforms,
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
                    .range(MIN_CAMERA_SCALE..=MAX_CAMERA_SCALE as f32)
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
        let firebrand = self
            .entities
            .get(&self.firebrand_entity_id)
            .expect("Expect player entity");
        let position = firebrand.entity.position();

        UiDisplayState {
            camera_tracks_character: self.camera_tracks_character,
            camera_position: self.camera_controller.camera.position(),
            zoom: self.camera_controller.projection.scale(),
            character_position: position.xy(),
            draw_stage_collision_info: self.draw_stage_collision_info,
            character_cycle: firebrand.entity.sprite_cycle().to_string(),
        }
    }

    fn process_ui_input(&mut self, ui_input_state: UiInputState) {
        if let Some(z) = ui_input_state.zoom {
            self.camera_controller.projection.set_scale(z);
        }
        if let Some(d) = ui_input_state.draw_stage_collision_info {
            self.draw_stage_collision_info = d;
        }
        if let Some(ctp) = ui_input_state.camera_tracks_character {
            self.camera_tracks_character = ctp;
        }
    }

    /// Adds this entity to the simulation state
    fn add_entity(&mut self, mut entity: Box<dyn entity::Entity>) -> u32 {
        if entity.entity_id() == 0 {
            entity.init(
                self.entity_id_vendor.next_id(),
                &self.map,
                &mut self.collision_space,
            );
        }

        let sprite_name = entity.sprite_name().to_string();
        let components = EntityComponents::new(
            entity,
            crate::sprite::rendering::EntityDrawable::load(
                &self.entity_tileset,
                self.entity_material.clone(),
                &self.device,
                &sprite_name,
                0,
            ),
            SpriteUniforms::new(
                &self.device,
                self.map.tileset.get_sprite_size().cast().unwrap(),
            ),
        );

        let id = components.id();
        self.entities.insert(id, components);
        id
    }

    /// Returns true iff the player can shoot.
    fn player_can_shoot_fireball(&self) -> bool {
        // The original game only allows one fireball on screen at a time; we have dynamic viewport sizes
        // so instead we're going to only allow a shot if there are no active fireballs closer than half
        // the stage width in the original game (since character is always in center)

        let mut closest_fireball_distance = f32::MAX;
        let character_position = self
            .entities
            .get(&self.firebrand_entity_id)
            .unwrap()
            .entity
            .position();
        for e in self.entities.values() {
            if e.class() == EntityClass::Fireball {
                let dist = (e.entity.position().x - character_position.x).abs();
                closest_fireball_distance = closest_fireball_distance.min(dist);
            }
        }

        closest_fireball_distance > (ORIGINAL_VIEWPORT_TILES_WIDE as f32 / 2.0)
    }

    fn update_entity_visibility(&mut self) {
        // get the viewport - outset it by 1 unit in each edge to "pad" it.
        // since enemy re-spawning isn't exactly a matter of going offscreen,
        // but more like going "a little offscreen".
        let viewport = self.camera_controller.viewport_bounds(
            &self.camera_controller.camera,
            &self.camera_controller.projection,
            -1.0,
        );

        let previously_visible_entities = std::mem::take(&mut self.visible_entities);
        for e in self.entities.values() {
            let bounds = e.entity.bounds();
            if crate::geom::intersection::rect_rect_intersects(viewport, bounds) {
                self.visible_entities.insert(e.id());
            }
        }

        for entity_id in self.visible_entities.iter() {
            if !previously_visible_entities.contains(entity_id) {
                if let Some(entity) = self.entities.get_mut(entity_id) {
                    entity.entity.did_enter_viewport();
                }
            }
        }

        for entity_id in previously_visible_entities.iter() {
            if !self.visible_entities.contains(entity_id) {
                if let Some(entity) = self.entities.get_mut(entity_id) {
                    entity.entity.did_exit_viewport();
                }
            }
        }
    }
}

impl MessageHandler for State {
    fn handle_message(&mut self, message: &Message) {
        if let Some(recipient_entity_id) = message.recipient_entity_id {
            //
            // if the message has a destination entity, route it - if no destination
            // entity is found that's OK, it might be expired.
            //
            if let Some(e) = self.entities.get_mut(&recipient_entity_id) {
                e.entity.handle_message(&message);
            }
        } else {
            //
            //  The message has no destination, so we handle it
            //

            match &message.event {
                Event::TryShootFireball {
                    origin,
                    direction,
                    velocity,
                } => {
                    if self.player_can_shoot_fireball() {
                        self.add_entity(Box::new(entities::fireball::Fireball::new(
                            point3(origin.x, origin.y, 0.0),
                            *direction,
                            *velocity,
                        )));

                        // Reply to firebrand that a shot was fired
                        self.message_dispatcher.enqueue(Message::global_to_entity(
                            self.firebrand_entity_id,
                            Event::DidShootFireball,
                        ));
                    }
                }

                Event::PlayEntityDeathAnimation {
                    position,
                    direction,
                } => {
                    let direction = match direction {
                        -1 => entities::death_animation::Direction::West,
                        _ => entities::death_animation::Direction::East,
                    };
                    self.add_entity(Box::new(entities::death_animation::DeathAnimation::new(
                        point3(position.x, position.y, sprite_layers::FOREGROUND),
                        direction,
                    )));
                }

                Event::SpawnEntity {
                    class_name,
                    spawn_point_sprite,
                    spawn_point_tile,
                } => {
                    match entities::instantiate_map_sprite(
                        class_name,
                        spawn_point_sprite,
                        spawn_point_tile,
                        &self.map,
                        &mut self.collision_space,
                        Some(&mut self.entity_id_vendor),
                    ) {
                        Ok(entity) => {
                            let id = self.add_entity(entity);
                            self.message_dispatcher.enqueue(Message::global_to_entity(
                                message.sender_entity_id.unwrap(),
                                Event::EntityWasSpawned {
                                    entity_id: Some(id),
                                },
                            ));
                        }
                        Err(e) => {
                            println!("Unable to instantiate \"{}\", error: {:?}", class_name, e);
                            panic!("Failed to instantiate SpawnPoint entity");
                        }
                    }
                }

                _ => {}
            }
        }
    }
}
