use cgmath::*;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    rc::Rc,
    time::Duration,
};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::PhysicalKey,
    window::Window,
};

use crate::{
    audio, camera, collision,
    entities::{
        self,
        util::{CompassDir, HorizontalDir},
        EntityClass,
    },
    entity::{self, EntityComponents, GameStatePeek},
    event_dispatch, map,
    sprite::rendering,
    texture, tileset,
    util::{self, hermite, lerp, Bounds},
    Options,
};

use super::{
    app_state::AppContext,
    constants::{
        layers, CAMERA_FAR_PLANE, CAMERA_NEAR_PLANE, DEFAULT_CAMERA_SCALE, MIN_CAMERA_SCALE,
        ORIGINAL_VIEWPORT_TILES_WIDE,
    },
    events::Event,
    gpu_state,
};

// ---------------------------------------------------------------------------------------------------------------------

struct EntityAdditionRequest {
    entity_id: u32,
    entity: Box<dyn entity::Entity>,
    needs_init: bool,
}

fn build_stage_entities() -> Vec<Box<dyn entity::Entity>> {
    todo!();
}

struct CameraShaker {
    pattern: Vec<(Vector2<f32>, f32)>,
    time: f32,
    index: usize,
}

impl CameraShaker {
    fn new(pattern: Vec<(Vector2<f32>, f32)>) -> Self {
        Self {
            pattern,
            time: 0.0,
            index: 0,
        }
    }

    fn update(&mut self, dt: Duration) -> Vector2<f32> {
        self.time += dt.as_secs_f32();
        if self.time > self.pattern[self.index].1 {
            self.time -= self.pattern[self.index].1;
            self.index += 1;
            self.index %= self.pattern.len();
        }
        self.pattern[self.index].0
    }
}

// ---------------------------------------------------------------------------------------------------------------------

const BOSS_FIGHT_START_TIME_ARENA_CONTRACTION_DURATION: f32 = 2.0;

// ---------------------------------------------------------------------------------------------------------------------

pub struct GameState {
    // Camera
    pub camera_controller: camera::CameraController,

    // Pipelines
    sprite_render_pipeline: wgpu::RenderPipeline,

    // Stage rendering
    stage_material: Rc<rendering::Material>,
    stage_uniforms: rendering::Uniforms,
    stage_debug_draw_overlap_uniforms: rendering::Uniforms,
    stage_debug_draw_contact_uniforms: rendering::Uniforms,
    stage_sprite_drawable: rendering::Drawable,

    // Collision detection and dispatch
    map: map::Map,
    collision_space: collision::Space,

    // Entity rendering
    entity_tileset: tileset::TileSet,
    entity_material: Rc<rendering::Material>,
    entities: HashMap<u32, entity::EntityComponents>,
    firebrand_entity_id: Option<u32>,
    firebrand_start_checkpoint: u32,
    firebrand_start_lives_remaining: u32,
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

    // General game state
    time: f32,
    boss_arena_entered_time: Option<f32>,
    boss_arena_left_bounds: Option<f32>,
    viewport_left_when_boss_arena_entered: Option<f32>,
    camera_shaker: Option<CameraShaker>,
    game_state_peek: GameStatePeek,
    pub pixels_per_unit: Vector2<f32>,
    palette_shift: f32,
    num_restarts: u32,
}

impl GameState {
    /// Creates new GameState
    /// start_checkpoint: Index of the checkpoint to place character at
    pub fn new(
        gpu: &mut gpu_state::GpuState,
        options: &Options,
        entity_id_vendor: &mut entity::IdVendor,
        start_checkpoint: u32,
        lives_remaining: u32,
    ) -> Self {
        // Load the stage map
        let map = map::Map::new_tmx(Path::new("res/level_1.tmx"));
        let map = map.expect("Expected map to load");
        let pixels_per_unit = map.tileset.get_sprite_size().cast().unwrap();

        let material_bind_group_layout = rendering::Material::bind_group_layout(&gpu.device);
        let (
            stage_sprite_material,
            stage_sprite_drawable,
            collision_space,
            entities,
            stage_entities,
            stage_animation_flipbooks,
        ) = {
            let stage_sprite_material = {
                let spritesheet_path = Path::new("res").join(&map.tileset.image_path);
                let spritesheet = Rc::new(
                    texture::Texture::load(&gpu.device, &gpu.queue, spritesheet_path).unwrap(),
                );
                Rc::new(rendering::Material::new(
                    &gpu.device,
                    "Sprite Material",
                    spritesheet,
                    &material_bind_group_layout,
                ))
            };

            let get_layer = |name: &str| {
                map.layer_named(name)
                    .unwrap_or_else(|| panic!("Expect layer named \"{}\"", name))
            };

            let bg_layer = get_layer("Background");
            let level_layer = get_layer("Level");
            let exit_layer = get_layer("Exit");
            let entity_layer = get_layer("Entities");
            let rising_floor_layer = get_layer("RisingFloor");
            let exit_door_left_layer = get_layer("ExitDoorLeft");
            let exit_door_right_layer = get_layer("ExitDoorRight");

            // generate level sprites
            let bg_sprites = map.generate_sprites(bg_layer, |_, _| layers::stage::BACKGROUND);
            let level_sprites = map.generate_sprites(level_layer, |_sprite, tile| {
                if tile.get_property("foreground") == Some("true") {
                    layers::stage::FOREGROUND
                } else {
                    layers::stage::LEVEL
                }
            });
            let exit_sprites = map.generate_sprites(exit_layer, |_, _| layers::stage::EXIT);

            // Collect sprites for RisingFloor and ExitDoor entities.
            // The entities which draw these sprites will assign correct z depth at render time
            let rising_floor_sprites = map.generate_sprites(rising_floor_layer, |_, _| 0.0);
            let exit_door_left_sprites = map.generate_sprites(exit_door_left_layer, |_, _| 0.0);
            let exit_door_right_sprites = map.generate_sprites(exit_door_right_layer, |_, _| 0.0);

            let rising_floor_entity = Box::new(entities::rising_floor::RisingFloor::new(
                rising_floor_sprites,
            ));

            let exit_door_left_entity = Box::new(entities::exit_door::ExitDoor::new(
                exit_door_left_sprites,
                HorizontalDir::West,
            ));

            let exit_door_right_entity = Box::new(entities::exit_door::ExitDoor::new(
                exit_door_right_sprites,
                HorizontalDir::East,
            ));

            // generate level entities
            let level_colliders: Vec<collision::Collider> = level_sprites
                .iter()
                .map(collision::Collider::from_static_sprite)
                .collect();
            let mut collision_space = collision::Space::new(&level_colliders);
            let entities = map.generate_entities(
                entity_layer,
                &mut collision_space,
                entity_id_vendor,
                |_, _| 0.0, // entities assign depth at render time
            );

            // generate animations
            let stage_animation_flipbooks =
                map.generate_animations(bg_layer, |_, _| layers::stage::BACKGROUND);

            let mut stage_sprites = vec![];
            stage_sprites.extend(bg_sprites);
            stage_sprites.extend(level_sprites);
            stage_sprites.extend(exit_sprites);

            let stage_sprites_mesh =
                rendering::Mesh::new(&stage_sprites, 0, &gpu.device, "Stage Sprite Mesh");

            (
                stage_sprite_material.clone(),
                rendering::Drawable::with(stage_sprites_mesh, stage_sprite_material),
                collision_space,
                entities,
                vec![
                    rising_floor_entity as Box<dyn entity::Entity>,
                    exit_door_left_entity as Box<dyn entity::Entity>,
                    exit_door_right_entity as Box<dyn entity::Entity>,
                ],
                stage_animation_flipbooks,
            )
        };

        // Build camera, and camera uniform storage
        let camera = camera::Camera::new((8.0, 8.0, -1.0), (0.0, 0.0, 1.0), Some(pixels_per_unit));
        let viewport_scale = if options.gameboy {
            MIN_CAMERA_SCALE
        } else {
            DEFAULT_CAMERA_SCALE
        };
        let projection = camera::Projection::new(
            gpu.config.width,
            gpu.config.height,
            viewport_scale,
            CAMERA_NEAR_PLANE,
            CAMERA_FAR_PLANE,
        );
        let camera_uniforms: camera::Uniforms = util::UniformWrapper::new(&gpu.device);
        let camera_controller = camera::CameraController::new(camera, projection, camera_uniforms);

        // Build the sprite render pipeline

        let mut stage_uniforms = util::UniformWrapper::<rendering::UniformData>::new(&gpu.device);
        let mut stage_debug_draw_overlap_uniforms =
            util::UniformWrapper::<rendering::UniformData>::new(&gpu.device);
        let mut stage_debug_draw_contact_uniforms =
            util::UniformWrapper::<rendering::UniformData>::new(&gpu.device);

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

        let sprite_render_pipeline = rendering::create_render_pipeline(
            &gpu.device,
            &sprite_render_pipeline_layout,
            gpu.config.format,
            Some(texture::Texture::DEPTH_FORMAT),
        );

        // Entities

        let entity_tileset = tileset::TileSet::new_tsx("./res/entities.tsx")
            .expect("Expected to load entities tileset");

        let entity_material = Rc::new({
            let spritesheet_path = Path::new("res").join(&entity_tileset.image_path);
            let spritesheet =
                Rc::new(texture::Texture::load(&gpu.device, &gpu.queue, spritesheet_path).unwrap());

            rendering::Material::new(
                &gpu.device,
                "Sprite Material",
                spritesheet,
                &material_bind_group_layout,
            )
        });

        let mut entity_add_requests = vec![];
        for e in entities.into_iter() {
            entity_add_requests.push(EntityAdditionRequest {
                entity_id: e.entity_id(),
                entity: e,
                needs_init: false,
            });
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
                let uniforms = util::UniformWrapper::<rendering::UniformData>::new(&gpu.device);
                rendering::FlipbookAnimationComponents::new(a, uniforms)
            })
            .collect::<Vec<_>>();

        //
        // Write unchanging values into their uniform buffers
        //

        stage_uniforms
            .data
            .set_model_position(point3(0.0, 0.0, 0.0))
            .set_pixels_per_unit(pixels_per_unit)
            .set_color(vec4(1.0, 1.0, 1.0, 1.0));
        stage_uniforms.write(&mut gpu.queue);

        stage_debug_draw_overlap_uniforms
            .data
            .set_model_position(point3(0.0, 0.0, -0.1)) // bring closer
            .set_pixels_per_unit(pixels_per_unit)
            .set_color(vec4(0.0, 1.0, 0.0, 0.75));
        stage_debug_draw_overlap_uniforms.write(&mut gpu.queue);

        stage_debug_draw_contact_uniforms
            .data
            .set_model_position(point3(0.0, 0.0, -0.2)) // bring closer
            .set_pixels_per_unit(pixels_per_unit)
            .set_color(vec4(1.0, 0.0, 0.0, 0.75));
        stage_debug_draw_contact_uniforms.write(&mut gpu.queue);

        let mut game_state = Self {
            camera_controller,
            sprite_render_pipeline,
            stage_material: stage_sprite_material,
            stage_uniforms,
            stage_debug_draw_overlap_uniforms,
            stage_debug_draw_contact_uniforms,
            stage_sprite_drawable,

            map,
            collision_space,
            entity_tileset,
            entity_material,
            entities: HashMap::new(),
            firebrand_entity_id: None,
            firebrand_start_checkpoint: start_checkpoint,
            firebrand_start_lives_remaining: lives_remaining,
            visible_entities: HashSet::new(),
            entities_to_add: Vec::new(),
            flipbook_animations,

            last_mouse_pos: (0, 0).into(),
            mouse_pressed: false,

            draw_stage_collision_info: false,
            camera_tracks_character: true,

            time: 0.0,
            boss_arena_entered_time: None,
            boss_arena_left_bounds: None,
            viewport_left_when_boss_arena_entered: None,
            camera_shaker: None,
            game_state_peek: GameStatePeek::default(),
            pixels_per_unit,
            palette_shift: 0.0,
            num_restarts: 0,
        };

        for req in entity_add_requests {
            game_state.add_entity(gpu, req);
        }

        for se in stage_entities {
            game_state.request_add_entity(entity_id_vendor, se);
        }

        game_state
    }

    pub fn resize(
        &mut self,
        _window: &Window,
        new_size: winit::dpi::PhysicalSize<u32>,
        _gpu: &gpu_state::GpuState,
    ) {
        self.camera_controller
            .projection
            .resize(new_size.width, new_size.height);
    }

    pub fn input(&mut self, _window: &Window, event: &WindowEvent, is_paused: bool) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state,
                        ..
                    },
                ..
            } => {
                let mut consumed = false;
                if !is_paused {
                    for e in self.entities.values_mut() {
                        if e.entity.process_keyboard(*key_code, *state) {
                            consumed = true;
                            break;
                        }
                    }
                }
                consumed || self.camera_controller.process_keyboard(*key_code, *state)
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

    pub fn gamepad_input(&mut self, event: gilrs::Event, is_paused: bool) {
        if !is_paused {
            for e in self.entities.values_mut() {
                e.entity.process_gamepad(event);
            }
        }
    }

    pub fn update(&mut self, ctx: &mut AppContext) {
        self.collision_space.update();

        //
        //  Process pending entity additions
        //

        self.process_entity_additions(ctx.gpu);

        //
        // If firebrand hasn't been constructed yet, we need to instantiate him at the assigned checkpoint
        //

        if self.firebrand_entity_id.is_none() {
            let positions = self
                .ordered_checkpoints()
                .iter()
                .map(|ec| ec.entity.position())
                .collect::<Vec<_>>();
            let checkpoint_idx =
                (self.firebrand_start_checkpoint as usize).min(positions.len() - 1);
            let position = positions[checkpoint_idx];

            // create firebrand and immediately process addition request since update()
            // depends on firebrand's location.
            self.firebrand_entity_id = Some(self.request_add_entity(
                ctx.entity_id_vendor,
                Box::new(entities::firebrand::Firebrand::new(
                    position.xy(),
                    self.firebrand_start_lives_remaining,
                )),
            ));
            self.process_entity_additions(ctx.gpu);

            ctx.message_dispatcher.broadcast(Event::FirebrandCreated {
                checkpoint: self.firebrand_start_checkpoint,
                num_restarts: self.num_restarts,
            });
        }

        self.time += ctx.game_delta_time.as_secs_f32();
        let current_map_bounds = self.current_map_bounds();
        let firebrand = &self.get_firebrand().entity;
        self.game_state_peek.player_position = firebrand.position().xy();
        self.game_state_peek.current_map_bounds = current_map_bounds;
        self.game_state_peek.camera_position = self.camera_controller.camera.position().xy();
        let palette_shift = self.palette_shift();

        //
        //  Update entities - if any are expired, remove them.
        //

        {
            let game_state_peek = self.game_state_peek();

            let mut expired_count = 0;
            for e in self.entities.values_mut() {
                e.entity.update(
                    ctx.game_delta_time,
                    &self.map,
                    &mut self.collision_space,
                    ctx.audio,
                    ctx.message_dispatcher,
                    &game_state_peek,
                );
                if let Some(ref mut uniforms) = e.uniforms {
                    e.entity.update_uniforms(uniforms);
                    uniforms
                        .data
                        .set_pixels_per_unit(self.pixels_per_unit)
                        .set_palette_shift(palette_shift);
                    uniforms.write(&mut ctx.gpu.queue);
                }

                if !e.entity.is_alive() {
                    e.entity.deactivate_collider(&mut self.collision_space);
                    expired_count += 1;
                }
            }

            if expired_count > 0 {
                self.entities.retain(|_, e| e.entity.is_alive())
            }
        }

        self.stage_uniforms.data.set_palette_shift(palette_shift);
        self.stage_uniforms.write(&mut ctx.gpu.queue);

        //
        //  Update flipbook animations
        //

        for a in &mut self.flipbook_animations {
            a.update(ctx.game_delta_time);
            a.uniforms
                .data
                .set_pixels_per_unit(self.pixels_per_unit)
                .set_palette_shift(palette_shift);
            a.uniforms.write(&mut ctx.gpu.queue);
        }

        //
        // Update camera state
        //

        let tracking = if self.camera_tracks_character {
            Some(self.get_firebrand().entity.position().xy())
        } else {
            None
        };

        let offset = self
            .camera_shaker
            .as_mut()
            .map(|shaker| shaker.update(ctx.game_delta_time));

        self.camera_controller.update(
            ctx.game_delta_time,
            tracking,
            offset,
            Some(current_map_bounds),
        );
        self.camera_controller.uniforms.write(&mut ctx.gpu.queue);

        //
        //  Notify entities of their visibility
        //

        self.update_entity_visibility();
    }

    pub fn render(
        &mut self,
        _window: &Window,
        gpu: &mut gpu_state::GpuState,
        encoder: &mut wgpu::CommandEncoder,
        frame_index: usize,
    ) {
        //
        // Render Sprites and entities; this is first pass so we clear color/depth
        //

        let layer_index = frame_index % gpu.color_attachment.layer_array_views.len();
        let color_attachment = wgpu::RenderPassColorAttachment {
            view: &gpu.color_attachment.layer_array_views[layer_index],
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        };

        let depth_attachment = wgpu::RenderPassDepthStencilAttachment {
            view: &gpu.depth_attachment.view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Game State Render Pass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: Some(depth_attachment),
            timestamp_writes: None,
            occlusion_query_set: None,
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
                if let Some(ref drawable) = e.entity_drawable {
                    if let Some(ref uniforms) = e.uniforms {
                        drawable.draw(
                            &mut render_pass,
                            &self.camera_controller.uniforms,
                            uniforms,
                            e.entity.sprite_cycle(),
                        );
                    }
                }
                if let Some(ref drawable) = e.sprite_drawable {
                    if let Some(ref uniforms) = e.uniforms {
                        drawable.draw(&mut render_pass, &self.camera_controller.uniforms, uniforms);
                    }
                }
            }
        }
    }

    pub fn handle_message(
        &mut self,
        message: &event_dispatch::Message,
        message_dispatcher: &mut event_dispatch::Dispatcher,
        entity_id_vendor: &mut entity::IdVendor,
        audio: &mut audio::Audio,
    ) {
        if let Some(recipient_entity_id) = message.recipient_entity_id {
            //
            // if the message has a destination entity, route it - if no `destination
            // entity is found that's OK, it might be expired.
            //
            if let Some(e) = self.entities.get_mut(&recipient_entity_id) {
                e.entity.handle_message(message);
            }
        } else {
            // if broadcast, send to everybody.
            if message.is_broadcast() {
                for e in self.entities.values_mut() {
                    e.entity.handle_message(message);
                }
            }

            match &message.event {
                Event::TryShootFireball {
                    origin,
                    direction,
                    velocity,
                    damage,
                } => {
                    if self.player_can_shoot_fireball() {
                        self.request_add_entity(
                            entity_id_vendor,
                            Box::new(entities::fireball::Fireball::new_fireball(
                                self.firebrand_entity_id.unwrap(),
                                origin.xy(),
                                *direction,
                                *velocity,
                                *damage,
                            )),
                        );

                        // Reply to firebrand that a shot was fired
                        message_dispatcher.global_to_entity(
                            self.firebrand_entity_id.unwrap(),
                            Event::DidShootFireball,
                        );
                    }
                }

                Event::PlayEntityDeathAnimation {
                    position,
                    direction,
                } => {
                    self.request_add_entity(
                        entity_id_vendor,
                        Box::new(entities::death_animation::DeathAnimation::new_enemy_death(
                            *position, *direction,
                        )),
                    );
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
                        Some(entity_id_vendor),
                    ) {
                        Ok(entity) => {
                            let id = self.request_add_entity(entity_id_vendor, entity);
                            message_dispatcher.global_to_entity(
                                message.sender_entity_id.unwrap(),
                                Event::EntityWasSpawned {
                                    entity_id: Some(id),
                                },
                            );
                        }
                        Err(e) => {
                            println!("Unable to instantiate \"{}\", error: {:?}", class_name, e);
                            panic!("Failed to instantiate SpawnPoint entity");
                        }
                    }
                }

                Event::ShootFiresprite {
                    position,
                    dir,
                    velocity,
                    damage,
                } => {
                    let sender_id = message
                        .sender_entity_id
                        .expect("Expect ShootFiresprite message to have a sender_entity_id");
                    self.request_add_entity(
                        entity_id_vendor,
                        Box::new(entities::fireball::Fireball::new_firesprite(
                            sender_id, *position, *dir, *velocity, *damage,
                        )),
                    );
                }

                Event::BossArenaEncountered { arena_left } => {
                    self.on_boss_arena_entered(message_dispatcher, *arena_left);
                }

                Event::BossDefeated => {
                    self.on_boss_was_defeated(audio, message_dispatcher);
                }

                Event::BossDied => {
                    self.on_boss_died(audio, message_dispatcher);
                }

                Event::StartCameraShake { pattern } => {
                    self.camera_shaker = Some(CameraShaker::new(pattern.clone()));
                }

                Event::EndCameraShake => {
                    self.camera_shaker = None;
                }

                Event::FirebrandDied => {
                    self.on_player_dead(entity_id_vendor, message_dispatcher);
                }

                Event::FirebrandStatusChanged { status } => {
                    self.game_state_peek.player_health = (status.hit_points, status.hit_points_max);
                    self.game_state_peek.player_flight =
                        (status.flight_time_remaining, status.flight_time_max);
                    self.game_state_peek.player_vials = status.num_vials;
                    self.game_state_peek.player_lives = status.num_lives;
                }

                Event::FirebrandPassedThroughExitDoor => {
                    self.on_level_complete();
                }

                Event::QueryBossFightMayStart => {
                    if self.boss_arena_entered_time.is_some() {
                        let sender = message
                            .sender_entity_id
                            .expect("TryBossRaise must be sent by Boss entity");
                        message_dispatcher.global_to_entity(sender, Event::BossFightMayStart);
                    }
                }

                _ => {}
            }
        }
    }

    pub fn set_palette_shift(&mut self, palette_shift: f32) {
        self.palette_shift = palette_shift.clamp(-1.0, 1.0);
    }

    pub fn palette_shift(&self) -> f32 {
        (self.palette_shift * 4.0).round() / 4.0
    }

    pub fn restart_game_at_checkpoint(
        &mut self,
        start_checkpoint: u32,
        lives_remaining: u32,
        message_dispatcher: &mut event_dispatch::Dispatcher,
    ) {
        self.num_restarts += 1;
        self.firebrand_start_checkpoint = start_checkpoint;
        self.firebrand_start_lives_remaining = lives_remaining;

        self.firebrand_entity_id = None;
        self.visible_entities.clear();
        self.boss_arena_entered_time = None;
        self.boss_arena_left_bounds = None;
        self.viewport_left_when_boss_arena_entered = None;
        self.camera_shaker = None;

        // For every entity which will be removed in reset, we need to remove collider.
        for ec in self.entities.values_mut() {
            if !ec.entity.entity_class().survives_level_restart() {
                ec.entity.deactivate_collider(&mut self.collision_space);
            }
        }

        // prune out everything which doesn't survive a level restart, and broadcast a reset event
        self.entities
            .retain(|_, e| e.entity.entity_class().survives_level_restart());

        message_dispatcher.broadcast(Event::ResetState);
    }

    pub fn game_over(&mut self, message_dispatcher: &mut event_dispatch::Dispatcher) {
        message_dispatcher.broadcast(Event::GameOver);
    }

    pub fn game_state_peek(&self) -> GameStatePeek {
        self.game_state_peek
    }

    pub fn entities_of_type(&self, entity_class: entities::EntityClass) -> Vec<&EntityComponents> {
        self.entities
            .values()
            .filter(|ec| ec.entity.entity_class() == entity_class)
            .collect()
    }

    /// Returns vector of the level's checkpoints, sorted along x from left to right,
    /// such that checkpoint 0 is the "first" in the level, and so on.
    pub fn ordered_checkpoints(&self) -> Vec<&EntityComponents> {
        // find the assigned checkpoint by sorting checkpoints on X and picking by index
        let mut checkpoints = self.entities_of_type(entities::EntityClass::CheckPoint);
        checkpoints.sort_by(|a, b| {
            a.entity
                .position()
                .x
                .partial_cmp(&b.entity.position().x)
                .unwrap()
        });
        checkpoints
    }

    /// Returns the index of the checkpoint with a given entity_id, if one exists, otherwise None.
    pub fn index_of_checkpoint(&self, entity_id: u32) -> Option<u32> {
        self.ordered_checkpoints()
            .iter()
            .enumerate()
            .filter(|(_, ec)| ec.entity.entity_id() == entity_id)
            .map(|(idx, _)| idx as u32)
            .next() // next() get's 0th element
    }

    /// Returns true iff the player can shoot.
    pub fn player_can_shoot_fireball(&self) -> bool {
        // The original game only allows one fireball on screen at a time; we have dynamic viewport sizes
        // so instead we're going to only allow a shot if there are no active fireballs closer than half
        // the stage width in the original game (since character is always in center)

        let mut closest_fireball_distance = f32::MAX;
        let character_position = self.get_firebrand().entity.position();
        for e in self.entities.values() {
            if e.class() == EntityClass::Fireball {
                let dist = (e.entity.position().x - character_position.x).abs();
                closest_fireball_distance = closest_fireball_distance.min(dist);
            }
        }

        closest_fireball_distance > (ORIGINAL_VIEWPORT_TILES_WIDE as f32 / 2.0)
    }

    /// For each entity, dispatches did_enter_viewport or did_leave_viewport as needed.
    pub fn update_entity_visibility(&mut self) {
        // get the viewport - outset it by a few units in each edge to "pad" it.
        // since enemy re-spawning isn't exactly a matter of going offscreen,
        // but more like going "a little offscreen".
        let outset = ORIGINAL_VIEWPORT_TILES_WIDE as f32 / 2.0;
        let viewport = self.camera_controller.viewport_bounds(-outset);

        let previously_visible_entities = std::mem::take(&mut self.visible_entities);
        for e in self.entities.values() {
            let bounds = e.entity.bounds();
            if collision::intersection::rect_rect_intersects(viewport, bounds) {
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

    /// returns the bounds of the map, which may be contracted owing to having entered the boss-fight-arena
    fn current_map_bounds(&self) -> Bounds {
        let map_bounds = self.map.bounds();

        if let Some(arena_left_bounds) = self.boss_arena_left_bounds {
            let elapsed = self.time
                - self
                    .boss_arena_entered_time
                    .expect("Expect boss_fight_start_time to be set");

            let t = (elapsed / BOSS_FIGHT_START_TIME_ARENA_CONTRACTION_DURATION).min(1.0);

            let vpl = self
                .viewport_left_when_boss_arena_entered
                .expect("Expect viewport_left_when_boss_encountered to be set");
            let x = lerp(hermite(t), vpl, arena_left_bounds);
            let origin = point2(x, map_bounds.bottom());

            Bounds::new(
                origin,
                vec2(map_bounds.right() - origin.x, map_bounds.top() - origin.y),
            )
        } else {
            map_bounds
        }
    }

    pub fn try_get_firebrand(&self) -> Option<&EntityComponents> {
        if let Some(e_id) = self.firebrand_entity_id {
            self.entities.get(&e_id)
        } else {
            None
        }
    }

    pub fn get_firebrand(&self) -> &EntityComponents {
        self.entities
            .get(
                &self
                    .firebrand_entity_id
                    .expect("Called get_firebrand before instantiating player"),
            )
            .expect("Expect firebrand_entity_id to be valid")
    }

    /// Request that the provided entity be added to the GameState at the next update.
    /// Returns the entity_id of the entity if it's already been initialized, or the
    /// id that will be assigned when it's late initialized at insertion time.
    fn request_add_entity(
        &mut self,
        entity_id_vendor: &mut entity::IdVendor,
        entity: Box<dyn entity::Entity>,
    ) -> u32 {
        if entity.entity_id() == 0 {
            let id = entity_id_vendor.next_id();
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

        let components = if !req.entity.sprite_name().is_empty() {
            let sprite_name = req.entity.sprite_name().to_string();
            // The Entity has specified a sprite name, which means it's using
            // an EntityDrawable to render.
            let uniforms = util::UniformWrapper::<rendering::UniformData>::new(&gpu.device);
            EntityComponents::with_entity_drawable(
                req.entity,
                rendering::EntityDrawable::load(
                    &self.entity_tileset,
                    self.entity_material.clone(),
                    &gpu.device,
                    &sprite_name,
                    0,
                ),
                uniforms,
            )
        } else if let Some(sprites) = req.entity.stage_sprites() {
            // The Entity has specified sprites to render, which means its using a
            // sprite::Drawable using the stage material to render.
            let uniforms = util::UniformWrapper::<rendering::UniformData>::new(&gpu.device);
            EntityComponents::with_sprite_drawable(
                req.entity,
                rendering::Drawable::with(
                    rendering::Mesh::new(&sprites, 0, &gpu.device, "Entity Stage Sprite Mesh"),
                    self.stage_material.clone(),
                ),
                uniforms,
            )
        } else {
            EntityComponents::just_entity(req.entity)
        };

        self.entities.insert(components.id(), components);
    }

    /// Adds all entities in the entities_to_add queue
    fn process_entity_additions(&mut self, gpu: &mut gpu_state::GpuState) {
        for addition in std::mem::take(&mut self.entities_to_add) {
            self.add_entity(gpu, addition);
        }
    }

    fn on_boss_arena_entered(
        &mut self,
        _message_dispatcher: &mut event_dispatch::Dispatcher,
        arena_left_bounds: f32,
    ) {
        println!("\n\nBOSS FIGHT!!\n\n");
        self.boss_arena_entered_time = Some(self.time);
        self.boss_arena_left_bounds = Some(arena_left_bounds);
        self.viewport_left_when_boss_arena_entered =
            Some(self.camera_controller.viewport_bounds(0.0).left());
    }

    fn on_boss_was_defeated(
        &mut self,
        audio: &mut audio::Audio,
        _message_dispatcher: &mut event_dispatch::Dispatcher,
    ) {
        println!("\n\nBOSS DEFEATED!!\n\n");

        // Clear enemies and projectiles from stage
        let should_retain = |ec: &EntityComponents| -> bool {
            !ec.class().is_enemy() && !ec.class().is_projectile()
        };

        for ec in self.entities.values_mut() {
            if !should_retain(ec) {
                ec.entity.deactivate_collider(&mut self.collision_space);
            }
        }

        self.entities.retain(|_, ec| should_retain(ec));
        audio.play_sound(audio::Sounds::BossDied);
        audio.stop_current_track();
    }

    fn on_boss_died(
        &mut self,
        audio: &mut audio::Audio,
        message_dispatcher: &mut event_dispatch::Dispatcher,
    ) {
        //  Kick off the floor raise.
        message_dispatcher.broadcast(Event::RaiseExitFloor);

        audio.play_sound(audio::Sounds::FloorRaise);
    }

    fn on_player_dead(
        &mut self,
        entity_id_vendor: &mut entity::IdVendor,
        _message_dispatcher: &mut event_dispatch::Dispatcher,
    ) {
        // spawn eight DeathAnimations, one in each compass dir
        let position = self.get_firebrand().entity.position();
        for dir in CompassDir::iter() {
            let e =
                entities::death_animation::DeathAnimation::new_firebrand_death(position.xy(), dir);
            self.request_add_entity(entity_id_vendor, Box::new(e));
        }
    }

    fn on_level_complete(&mut self) {
        println!("GameState::on_level_complete");
    }
}
