use cgmath::*;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    rc::Rc,
};
use wgpu::{CommandEncoder, SwapChainFrame};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyboardInput, MouseButton, WindowEvent},
    window::Window,
};

use crate::{
    camera,
    entities::{self, EntityClass},
    entity,
    entity::EntityComponents,
    event_dispatch,
    event_dispatch::{Dispatcher, Message, MessageHandler},
    map,
    sprite::rendering::Uniforms as SpriteUniforms,
    sprite::{collision, rendering},
    texture, tileset,
};

use super::{
    constants::{sprite_layers, ORIGINAL_VIEWPORT_TILES_WIDE},
    events::Event,
    gpu_state,
};

struct EntityAdditionRequest {
    entity_id: u32,
    entity: Box<dyn entity::Entity>,
    needs_init: bool,
}

pub struct GameState {
    // Camera
    pub camera_controller: camera::CameraController,

    // Pipelines
    sprite_render_pipeline: wgpu::RenderPipeline,

    // Stage rendering
    stage_uniforms: SpriteUniforms,
    stage_debug_draw_overlap_uniforms: SpriteUniforms,
    stage_debug_draw_contact_uniforms: SpriteUniforms,
    stage_sprite_drawable: rendering::Drawable,

    // Collision detection and dispatch
    map: map::Map,
    collision_space: collision::Space,
    message_dispatcher: event_dispatch::Dispatcher,

    // Entity rendering
    entity_id_vendor: entity::IdVendor,
    entity_tileset: tileset::TileSet,
    entity_material: Rc<crate::sprite::rendering::Material>,
    entities: HashMap<u32, entity::EntityComponents>,
    firebrand_entity_id: u32,
    visible_entities: HashSet<u32>,
    entities_to_add: Vec<EntityAdditionRequest>,

    // Flipbook animations
    flipbook_animations: Vec<rendering::FlipbookAnimationComponents>,

    // Input state
    last_mouse_pos: PhysicalPosition<f64>,
    mouse_pressed: bool,

    // Toggles
    pub draw_stage_collision_info: bool,
    pub camera_tracks_character: bool,
}

impl GameState {
    pub fn new(gpu: &mut gpu_state::GpuState) -> Self {
        // Load the stage map
        let mut entity_id_vendor = entity::IdVendor::default();
        let map = map::Map::new_tmx(Path::new("res/level_1.tmx"));
        let map = map.expect("Expected map to load");
        let sprite_size_px = vec2(
            map.tileset.tile_width as f32,
            map.tileset.tile_height as f32,
        );

        let material_bind_group_layout =
            crate::sprite::rendering::Material::bind_group_layout(&gpu.device);
        let (
            stage_sprite_material,
            stage_sprite_drawable,
            collision_space,
            entities,
            stage_animation_flipbooks,
        ) = {
            let stage_sprite_material = {
                let spritesheet_path = Path::new("res").join(&map.tileset.image_path);
                let spritesheet =
                    texture::Texture::load(&gpu.device, &gpu.queue, spritesheet_path, false)
                        .unwrap();
                Rc::new(crate::sprite::rendering::Material::new(
                    &gpu.device,
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
            all_sprites.extend(level_sprites);

            let sm =
                crate::sprite::rendering::Mesh::new(&all_sprites, 0, &gpu.device, "Sprite Mesh");
            (
                stage_sprite_material.clone(),
                rendering::Drawable::with(sm, stage_sprite_material),
                collision_space,
                entities,
                stage_animation_flipbooks,
            )
        };

        // Build camera, and camera uniform storage
        let map_origin = point2(0.0, 0.0);
        let map_extent = vec2(map.width as f32, map.height as f32);
        let camera = camera::Camera::new((8.0, 8.0, -1.0), (0.0, 0.0, 1.0), None);
        let projection =
            camera::Projection::new(gpu.sc_desc.width, gpu.sc_desc.height, 16.0, 0.1, 100.0);
        let camera_uniforms = camera::Uniforms::new(&gpu.device);
        let camera_controller = camera::CameraController::new(
            camera,
            projection,
            camera_uniforms,
            map_origin,
            map_extent,
        );

        // Build the sprite render pipeline

        let mut stage_uniforms = SpriteUniforms::new(&gpu.device, sprite_size_px);
        let mut stage_debug_draw_overlap_uniforms =
            SpriteUniforms::new(&gpu.device, sprite_size_px);
        let mut stage_debug_draw_contact_uniforms =
            SpriteUniforms::new(&gpu.device, sprite_size_px);

        let sprite_render_pipeline_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[
                        &material_bind_group_layout,
                        &camera_controller.uniforms.bind_group_layout,
                        &stage_uniforms.bind_group_layout,
                    ],
                    label: Some("Stage Sprite Pipeline Layout"),
                    push_constant_ranges: &[],
                });

        let sprite_render_pipeline = crate::sprite::rendering::create_render_pipeline(
            &gpu.device,
            &sprite_render_pipeline_layout,
            gpu.sc_desc.format,
            Some(texture::Texture::DEPTH_FORMAT),
        );

        // Entities

        let entity_tileset = tileset::TileSet::new_tsx("./res/entities.tsx")
            .expect("Expected to load entities tileset");

        let entity_material = Rc::new({
            let spritesheet_path = Path::new("res").join(&entity_tileset.image_path);
            let spritesheet =
                texture::Texture::load(&gpu.device, &gpu.queue, spritesheet_path, false).unwrap();

            crate::sprite::rendering::Material::new(
                &gpu.device,
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
                entity::EntityComponents::new(
                    e,
                    crate::sprite::rendering::EntityDrawable::load(
                        &entity_tileset,
                        entity_material.clone(),
                        &gpu.device,
                        &name,
                        0,
                    ),
                    SpriteUniforms::new(&gpu.device, sprite_size_px),
                ),
            );
        }

        let flipbook_animations = stage_animation_flipbooks
            .into_iter()
            .map(|a| {
                rendering::FlipbookAnimationDrawable::new(
                    a,
                    stage_sprite_material.clone(),
                    &gpu.device,
                )
            })
            .map(|a| {
                rendering::FlipbookAnimationComponents::new(
                    a,
                    SpriteUniforms::new(&gpu.device, sprite_size_px),
                )
            })
            .collect::<Vec<_>>();

        //
        // Write unchanging values into their uniform buffers
        //

        stage_uniforms
            .data
            .set_model_position(point3(0.0, 0.0, 0.0));
        stage_uniforms.data.set_color(vec4(1.0, 1.0, 1.0, 1.0));
        stage_uniforms.write(&mut gpu.queue);

        stage_debug_draw_overlap_uniforms
            .data
            .set_model_position(point3(0.0, 0.0, -0.1)); // bring closer
        stage_debug_draw_overlap_uniforms
            .data
            .set_color(vec4(0.0, 1.0, 0.0, 0.75));
        stage_debug_draw_overlap_uniforms.write(&mut gpu.queue);

        stage_debug_draw_contact_uniforms
            .data
            .set_model_position(point3(0.0, 0.0, -0.2)); // bring closer
        stage_debug_draw_contact_uniforms
            .data
            .set_color(vec4(1.0, 0.0, 0.0, 0.75));
        stage_debug_draw_contact_uniforms.write(&mut gpu.queue);

        Self {
            camera_controller,
            sprite_render_pipeline,
            stage_uniforms,
            stage_debug_draw_overlap_uniforms,
            stage_debug_draw_contact_uniforms,
            stage_sprite_drawable,
            map,
            collision_space,
            message_dispatcher: event_dispatch::Dispatcher::default(),
            entity_id_vendor,
            entity_tileset,
            entity_material,
            entities: entity_components,
            firebrand_entity_id,
            visible_entities: HashSet::new(),
            entities_to_add: Vec::new(),
            flipbook_animations,

            last_mouse_pos: (0, 0).into(),
            mouse_pressed: false,

            draw_stage_collision_info: false,
            camera_tracks_character: true,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
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

    pub fn update(&mut self, dt: std::time::Duration, gpu: &mut gpu_state::GpuState) {
        //
        //  Process entity additions
        //

        {
            let additions = std::mem::take(&mut self.entities_to_add);
            for addition in additions {
                self.add_entity(gpu, addition);
            }
        }

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
                e.uniforms.write(&mut gpu.queue);

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
            a.uniforms.write(&mut gpu.queue);
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
        self.camera_controller.uniforms.write(&mut gpu.queue);

        //
        //  Notify entities of their visibility
        //

        self.update_entity_visibility();

        //
        // Dispatch collected messages
        //

        Dispatcher::dispatch(&self.message_dispatcher.drain(), self);
    }

    pub fn render(
        &mut self,
        gpu: &mut gpu_state::GpuState,
        frame: &SwapChainFrame,
        encoder: &mut CommandEncoder,
    ) {
        //
        // Render Sprites and entities; this is first pass so we clear color/depth
        //

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
                attachment: &gpu.depth_texture.view,
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
            a.drawable.draw(
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

    /// Returns true iff the player can shoot.
    pub fn player_can_shoot_fireball(&self) -> bool {
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

    pub fn update_entity_visibility(&mut self) {
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

    pub fn get_firebrand(&self) -> &EntityComponents {
        self.entities
            .get(&self.firebrand_entity_id)
            .expect("Expect firebrand_entity_id to be valid")
    }

    /// Request that the provided entity be added to the GameState at the next update.
    /// Returns the entity_id of the entity if it's already been initialized, or the
    /// id that will be assigned when it's late initialized at insertion time.
    fn request_add_entity(&mut self, entity: Box<dyn entity::Entity>) -> u32 {
        if entity.entity_id() == 0 {
            let id = self.entity_id_vendor.next_id();
            self.entities_to_add.push(EntityAdditionRequest {
                entity_id: id,
                entity,
                needs_init: true,
            });
            id
        } else {
            let id = entity.entity_id();
            self.entities_to_add.push(EntityAdditionRequest {
                entity_id: id,
                entity,
                needs_init: false,
            });
            id
        }
    }

    /// Adds the entity specified in the request
    fn add_entity(&mut self, gpu: &mut gpu_state::GpuState, mut req: EntityAdditionRequest) {
        if req.needs_init {
            req.entity
                .init(req.entity_id, &self.map, &mut self.collision_space);
        }

        let sprite_name = req.entity.sprite_name().to_string();
        let components = EntityComponents::new(
            req.entity,
            crate::sprite::rendering::EntityDrawable::load(
                &self.entity_tileset,
                self.entity_material.clone(),
                &gpu.device,
                &sprite_name,
                0,
            ),
            SpriteUniforms::new(
                &gpu.device,
                self.map.tileset.get_sprite_size().cast().unwrap(),
            ),
        );

        self.entities.insert(components.id(), components);
    }
}

impl MessageHandler for GameState {
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
                        self.request_add_entity(Box::new(entities::fireball::Fireball::new(
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
                    self.request_add_entity(Box::new(
                        entities::death_animation::DeathAnimation::new(
                            point3(position.x, position.y, sprite_layers::FOREGROUND),
                            *direction,
                        ),
                    ));
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
                            let id = self.request_add_entity(entity);
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
