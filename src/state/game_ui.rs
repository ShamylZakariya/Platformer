use cgmath::*;
use std::{collections::HashMap, path::Path, rc::Rc, time::Duration};

use winit::{
    event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent},
    window::Window,
};

use crate::{audio, camera, sprite::rendering, state::gpu_state, texture::Texture};
use crate::{
    collision,
    entity::{self, EntityComponents},
    texture,
};
use crate::{event_dispatch, Options};
use crate::{
    map,
    util::{self, lerp},
};

use super::{
    app_state::AppContext,
    constants::{layers, CAMERA_FAR_PLANE, CAMERA_NEAR_PLANE, DEFAULT_CAMERA_SCALE},
    events::Event,
    game_state,
};

// ---------------------------------------------------------------------------------------------------------------------

const DRAWER_OPEN_VEL: f32 = 2.0;
const START_MESSAGE_DURATION: f32 = 2.0;
const START_MESSAGE_BLINK_PERIOD: f32 = 0.25;

// ---------------------------------------------------------------------------------------------------------------------

pub struct GameUi {
    pipeline: wgpu::RenderPipeline,

    camera_view: camera::Camera,
    camera_projection: camera::Projection,
    camera_uniforms: camera::Uniforms,

    // drawer tile map, entities, and associated gfx state for drawing sprites
    drawer_collision_space: collision::Space,
    game_ui_map: map::Map,
    sprite_material: Rc<rendering::Material>,
    drawer_drawable: rendering::Drawable,
    drawer_uniforms: rendering::Uniforms,
    game_start_drawable: rendering::Drawable,
    game_start_uniforms: rendering::Uniforms,
    game_over_drawable: rendering::Drawable,
    game_over_uniforms: rendering::Uniforms,
    entities: HashMap<u32, entity::EntityComponents>,

    // state
    time: f32,
    drawer_open: bool,
    drawer_open_progress: f32,
    start_message_blink_countdown: f32,
    game_over_message_visible: bool,
    sprite_size_px: Vector2<f32>,
    palette_shift: f32,
    toggle_drawer_needed: bool,
}

impl GameUi {
    pub fn new(
        gpu: &mut gpu_state::GpuState,
        _options: &Options,
        entity_id_vendor: &mut entity::IdVendor,
        tonemap: Rc<Texture>,
    ) -> Self {
        // build camera
        let camera_view = camera::Camera::new((0.0, 0.0, 0.0), (0.0, 0.0, 1.0), None);
        let camera_uniforms: camera::Uniforms = util::UniformWrapper::new(&gpu.device);
        let camera_projection = camera::Projection::new(
            gpu.sc_desc.width,
            gpu.sc_desc.height,
            DEFAULT_CAMERA_SCALE * 2.0, // ui units are half size of game units
            CAMERA_NEAR_PLANE,
            CAMERA_FAR_PLANE,
        );

        // load game ui map and construct material/drawable etcs
        let game_ui_map = map::Map::new_tmx(Path::new("res/game_ui.tmx"));
        let game_ui_map = game_ui_map.expect("Expected 'res/game_ui.tmx' to load");

        //
        //  Create sprite material and pipeline layout
        //

        let bind_group_layout = rendering::Material::bind_group_layout(&gpu.device);
        let sprite_material = {
            let spritesheet_path = Path::new("res").join(&game_ui_map.tileset.image_path);
            let spritesheet = Rc::new(
                texture::Texture::load(&gpu.device, &gpu.queue, spritesheet_path, false).unwrap(),
            );
            Rc::new(rendering::Material::new(
                &gpu.device,
                "UI Sprite Material",
                spritesheet,
                tonemap,
                &bind_group_layout,
            ))
        };

        let sprite_size_px = game_ui_map.tileset.get_sprite_size().cast().unwrap();
        let drawer_uniforms = util::UniformWrapper::<rendering::UniformData>::new(&gpu.device);

        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &bind_group_layout,
                    &camera_uniforms.bind_group_layout,
                    &drawer_uniforms.bind_group_layout,
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

        //
        //  Load drawables
        //

        let get_layer = |name: &str| {
            game_ui_map
                .layer_named(name)
                .unwrap_or_else(|| panic!("Expect layer named \"{}\"", name))
        };

        let create_drawable = |name: &str, z: f32| {
            let layer = get_layer(name);
            let sprites = game_ui_map.generate_sprites(layer, |_, _| z);
            let mesh = rendering::Mesh::new(&sprites, 0, &gpu.device, name);
            rendering::Drawable::with(mesh, sprite_material.clone())
        };

        let drawer_drawable = create_drawable("Drawer", layers::ui::BACKGROUND);
        let game_over_drawable = create_drawable("GameOver", layers::ui::FOREGROUND);
        let game_start_drawable = create_drawable("GameStart", layers::ui::FOREGROUND);

        //
        //  Load entities
        //

        let mut collision_space = collision::Space::new(&[]);
        let entities_layer = get_layer("Entities");

        let entities = game_ui_map.generate_entities(
            entities_layer,
            &mut collision_space,
            entity_id_vendor,
            |_, _| 0.0,
        );

        // convert entities to a mapping of id -> EntityComponents
        let entities = entities
            .into_iter()
            .map(|e| {
                let sprite_name = e.sprite_name().to_string();
                let ec = EntityComponents::with_entity_drawable(
                    e,
                    rendering::EntityDrawable::load(
                        &game_ui_map.tileset,
                        sprite_material.clone(),
                        &gpu.device,
                        &sprite_name,
                        0,
                    ),
                    util::UniformWrapper::<rendering::UniformData>::new(&gpu.device),
                );
                (ec.id(), ec)
            })
            .collect::<HashMap<_, _>>();

        let game_over_uniforms = util::UniformWrapper::<rendering::UniformData>::new(&gpu.device);
        let game_start_uniforms = util::UniformWrapper::<rendering::UniformData>::new(&gpu.device);

        let mut game_ui = Self {
            pipeline,

            camera_view,
            camera_projection,
            camera_uniforms,

            drawer_collision_space: collision_space,
            game_ui_map,
            sprite_material,
            drawer_drawable,
            drawer_uniforms,
            game_over_drawable,
            game_over_uniforms,
            game_start_drawable,
            game_start_uniforms,
            entities,

            time: 0.0,
            drawer_open: false,
            drawer_open_progress: 0.0,
            start_message_blink_countdown: 0.0,
            game_over_message_visible: false,
            sprite_size_px,
            palette_shift: 0.0,
            toggle_drawer_needed: false,
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
                    self.toggle_drawer_needed = true;
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    pub fn gamepad_input(&mut self, event: gilrs::Event) {
        if let gilrs::EventType::ButtonPressed(button, ..) = event.event {
            if matches!(button, gilrs::Button::Start) {
                self.toggle_drawer_needed = true;
            }
        }
    }

    pub fn update(
        &mut self,
        dt: std::time::Duration,
        ctx: &mut AppContext,
        game: &game_state::GameState,
    ) {
        let sprite_size_px = self.sprite_size_px;
        let palette_shift = self.palette_shift();
        self.drawer_collision_space.update();

        self.time += dt.as_secs_f32();

        if self.toggle_drawer_needed {
            self.drawer_open = !self.drawer_open;
            self.toggle_drawer_needed = false;
            if self.drawer_open {
                ctx.audio.play_sound(audio::Sounds::DrawerOpen);
                ctx.audio.pause_current_track();
            } else {
                ctx.audio.resume_current_track();
            }
        }

        // Canter camera on window, and set projection scale
        self.camera_view.set_position(point3(0.0, 0.0, 0.0));
        self.camera_projection
            .set_scale(game.camera_controller.projection.scale() * 2.0);

        self.camera_uniforms
            .data
            .update_view_proj(&self.camera_view, &self.camera_projection);
        self.camera_uniforms.write(&mut ctx.gpu.queue);

        // update drawer uniforms
        let bounds = self.game_ui_map.bounds();
        let drawer_offset = vec3(-bounds.width() / 2.0, self.update_drawer_position(dt), 0.0);

        self.drawer_uniforms
            .data
            .set_sprite_size_px(sprite_size_px)
            .set_color(vec4(1.0, 1.0, 1.0, 1.0))
            .set_palette_shift(palette_shift)
            .set_model_position(point3(drawer_offset.x, drawer_offset.y, drawer_offset.z));
        self.drawer_uniforms.write(&mut ctx.gpu.queue);

        // update entity uniforms - note we have to apply drawer position offset
        let game_state_peek = game.game_state_peek();
        for e in self.entities.values_mut() {
            e.entity.update(
                dt,
                &self.game_ui_map,
                &mut self.drawer_collision_space,
                ctx.audio,
                ctx.message_dispatcher,
                &game_state_peek,
            );
            if let Some(ref mut uniforms) = e.uniforms {
                e.entity.update_uniforms(uniforms);
                uniforms
                    .data
                    .set_sprite_size_px(sprite_size_px)
                    .set_palette_shift(palette_shift)
                    .offset_model_position(drawer_offset);
                uniforms.write(&mut ctx.gpu.queue);
            }
        }

        // update game over and game start uniforms to center their test strings.
        // Note: We don't apply palette shift to text drawables
        let mut center_text_drawable =
            |drawable: &rendering::Drawable, uniforms: &mut rendering::Uniforms| {
                let bounds = drawable
                    .meshes
                    .first()
                    .expect("Expect drawable to have mesh at index 0")
                    .bounds;
                uniforms
                    .data
                    .set_sprite_size_px(sprite_size_px)
                    .set_color(vec4(1.0, 1.0, 1.0, 1.0))
                    .set_model_position(point3(-bounds.width() / 2.0, -bounds.height() / 2.0, 0.0));
                uniforms.write(&mut ctx.gpu.queue);
            };

        center_text_drawable(&self.game_over_drawable, &mut self.game_over_uniforms);
        center_text_drawable(&self.game_start_drawable, &mut self.game_start_uniforms);

        // update countdowns
        self.start_message_blink_countdown =
            (self.start_message_blink_countdown - dt.as_secs_f32()).max(0.0);
    }

    pub fn render(
        &mut self,
        _window: &Window,
        gpu: &mut gpu_state::GpuState,
        frame: &wgpu::SwapChainFrame,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Game UI Render Pass"),
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

        self.drawer_drawable.draw(
            &mut render_pass,
            &self.camera_uniforms,
            &self.drawer_uniforms,
        );

        for e in self.entities.values() {
            if e.entity.is_alive() && e.entity.should_draw() {
                if let Some(ref drawable) = e.entity_drawable {
                    if let Some(ref uniforms) = e.uniforms {
                        drawable.draw(
                            &mut render_pass,
                            &self.camera_uniforms,
                            uniforms,
                            e.entity.sprite_cycle(),
                        );
                    }
                }
                if let Some(ref drawable) = e.sprite_drawable {
                    if let Some(ref uniforms) = e.uniforms {
                        drawable.draw(&mut render_pass, &self.camera_uniforms, uniforms);
                    }
                }
            }
        }

        if self.start_message_blink_countdown > 0.0 {
            let cycle = (self.start_message_blink_countdown / START_MESSAGE_BLINK_PERIOD) as i32;
            if cycle % 2 == 1 {
                self.game_start_drawable.draw(
                    &mut render_pass,
                    &self.camera_uniforms,
                    &self.game_start_uniforms,
                );
            }
        }

        if self.game_over_message_visible {
            self.game_over_drawable.draw(
                &mut render_pass,
                &self.camera_uniforms,
                &self.game_over_uniforms,
            );
        }
    }

    pub fn handle_message(&mut self, message: &event_dispatch::Message) {
        if let Some(recipient_entity_id) = message.recipient_entity_id {
            // if message has a destination entity, attempt to route it there
            if let Some(e) = self.entities.get_mut(&recipient_entity_id) {
                e.entity.handle_message(&message);
            }
        } else {
            // if broadcast, send to everybody.
            if message.is_broadcast() {
                for e in self.entities.values_mut() {
                    e.entity.handle_message(message);
                }
            }

            if matches!(message.event, Event::GameOver) {
                self.show_game_over_message();
            }
        }
    }

    // MARK: Public
    pub fn set_palette_shift(&mut self, palette_shift: f32) {
        self.palette_shift = palette_shift.clamp(-1.0, 1.0);
    }

    pub fn palette_shift(&self) -> f32 {
        (self.palette_shift * 4.0).round() / 4.0
    }

    pub fn is_paused(&self) -> bool {
        self.drawer_open
    }

    pub fn show_start_message(&mut self) {
        self.start_message_blink_countdown = START_MESSAGE_DURATION;
    }

    pub fn show_game_over_message(&mut self) {
        self.game_over_message_visible = true;
    }

    // MARK: Private

    fn update_drawer_position(&mut self, dt: Duration) -> f32 {
        let bounds = self.game_ui_map.bounds();
        let vp_units_high = self.camera_projection.viewport_size().y;
        let drawer_closed_y =
            (-vp_units_high / 2.0) - bounds.height() - 1.0 + 3.0 - (3.0 / self.sprite_size_px.y);
        let drawer_open_y = (-vp_units_high / 2.0) - 1.0 - 1.0/self.sprite_size_px.y;

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
