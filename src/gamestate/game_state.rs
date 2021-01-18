use cgmath::*;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    rc::Rc,
};

use crate::{camera, sprite::rendering::Uniforms as SpriteUniforms, texture};
use crate::{
    entity, event_dispatch, map,
    sprite::{collision, rendering},
    tileset,
};

use super::constants::sprite_layers;

pub struct GameState {
    // Camera
    camera_controller: camera::CameraController,

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

    // Flipbook animations
    flipbook_animations: Vec<rendering::FlipbookAnimationComponents>,
}

impl GameState {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sc_desc: &wgpu::SwapChainDescriptor,
    ) -> Self {
        // Load the stage map
        let mut entity_id_vendor = entity::IdVendor::default();
        let map = map::Map::new_tmx(Path::new("res/level_1.tmx"));
        let map = map.expect("Expected map to load");
        let sprite_size_px = vec2(
            map.tileset.tile_width as f32,
            map.tileset.tile_height as f32,
        );

        let material_bind_group_layout =
            crate::sprite::rendering::Material::bind_group_layout(device);
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
                rendering::Drawable::with(sm, stage_sprite_material.clone()),
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
        let camera_uniforms = camera::Uniforms::new(&device);
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
                entity::EntityComponents::new(
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
                rendering::FlipbookAnimationComponents::new(
                    a,
                    SpriteUniforms::new(&device, sprite_size_px),
                )
            })
            .collect::<Vec<_>>();

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
            flipbook_animations: flipbook_animations,
        }
    }
}
