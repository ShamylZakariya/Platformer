use cgmath::{prelude::*, *};
use core::panic;
use std::hash::Hash;
use std::rc::Rc;
use std::{collections::HashMap, time::Duration};

use crate::texture;
use crate::tileset;
use crate::{camera, util::Bounds};
use crate::{sprite::core::*, util::*};
use wgpu::util::DeviceExt;

// --------------------------------------------------------------------------------------------------------------------

pub fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
) -> wgpu::RenderPipeline {
    let vertex_descs = &[Vertex::desc()];
    let sprite_shader_module_desc = wgpu::include_wgsl!("../shaders/sprite.wgsl");
    let sprite_shader_module = device.create_shader_module(sprite_shader_module_desc);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(layout),

        vertex: wgpu::VertexState {
            module: &sprite_shader_module,
            entry_point: Some("sprite_vs_main"),
            buffers: vertex_descs,
            compilation_options: Default::default(),
        },

        fragment: Some(wgpu::FragmentState {
            module: &sprite_shader_module,
            entry_point: Some("sprite_fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),

        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Cw,
            cull_mode: None,
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },

        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),

        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },

        multiview: None,
        cache: None,
    })
}

// --------------------------------------------------------------------------------------------------------------------

pub trait VertexBufferDescription {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub tex_coord: Vector2<f32>,
    pub corner: Vector2<f32>, // represents whcih corner of the sprite quad it is, e.g., (-1,-1) is top left.
    pub color: Vector4<f32>,
}
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

impl Vertex {
    pub fn new(
        position: Vector3<f32>,
        tex_coord: Vector2<f32>,
        corner: Vector2<f32>,
        color: Vector4<f32>,
    ) -> Self {
        Self {
            position,
            tex_coord,
            corner,
            color,
        }
    }
}

impl VertexBufferDescription for Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 7]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct UniformData {
    model_position: Vector4<f32>,
    color: Vector4<f32>,
    sprite_scale: Vector2<f32>,
    pixels_per_unit: Vector2<f32>,
    tex_coord_offset: Vector2<f32>,
    palette_shift: f32,
    _unused: f32,
}

unsafe impl bytemuck::Pod for UniformData {}
unsafe impl bytemuck::Zeroable for UniformData {}

impl Default for UniformData {
    fn default() -> Self {
        Self {
            model_position: Vector4::zero(),
            color: vec4(1.0, 1.0, 1.0, 1.0),
            sprite_scale: vec2(1.0, 1.0),
            pixels_per_unit: vec2(1.0, 1.0),
            tex_coord_offset: vec2(0.0, 0.0),
            palette_shift: 0.0,
            _unused: 0.0,
        }
    }
}

impl UniformData {
    pub fn set_color(&mut self, color: Vector4<f32>) -> &mut Self {
        self.color = color;
        self
    }

    pub fn set_model_position(&mut self, position: Point3<f32>) -> &mut Self {
        self.model_position = vec4(position.x, position.y, position.z, 1.0);
        self
    }

    pub fn offset_model_position(&mut self, delta: Vector3<f32>) -> &mut Self {
        self.model_position += vec4(delta.x, delta.y, delta.z, 0.0);
        self
    }

    pub fn set_sprite_scale(&mut self, sprite_scale: Vector2<f32>) -> &mut Self {
        self.sprite_scale = sprite_scale;
        self
    }

    pub fn set_pixels_per_unit(&mut self, pixels_per_unit: Vector2<f32>) -> &mut Self {
        self.pixels_per_unit = pixels_per_unit;
        self
    }

    pub fn set_tex_coord_offset(&mut self, tex_coord_offset: Vector2<f32>) -> &mut Self {
        self.tex_coord_offset = tex_coord_offset;
        self
    }

    pub fn set_palette_shift(&mut self, palette_shift: f32) -> &mut Self {
        self.palette_shift = palette_shift.clamp(-1.0, 1.0);
        self
    }
}

/// Specialization of util::UniformWrapper for sprite rendering uniform storage
pub type Uniforms = crate::util::UniformWrapper<UniformData>;

// --------------------------------------------------------------------------------------------------------------------

pub struct Material {
    pub name: String,
    pub texture: Rc<texture::Texture>,
    pub bind_group: wgpu::BindGroup,
}

#[allow(dead_code)]
impl Material {
    pub fn new(
        device: &wgpu::Device,
        name: &str,
        texture: Rc<texture::Texture>,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some(name),
        });
        Self {
            name: String::from(name),
            texture,
            bind_group,
        }
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // diffuse texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                // diffuse texture & tonemap sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        })
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct SpriteSpatialIndexer {
    pub origin: Point2<f32>,
    pub extent: Vector2<f32>,
}

impl SpriteSpatialIndexer {
    fn from(sprite: &Sprite) -> Self {
        Self {
            origin: sprite.origin.xy(),
            extent: sprite.extent,
        }
    }
}

impl Hash for SpriteSpatialIndexer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        hash_point2(&self.origin, state);
        hash_vec2(&self.extent, state);
    }
}

impl PartialEq for SpriteSpatialIndexer {
    fn eq(&self, other: &Self) -> bool {
        relative_eq!(self.origin, other.origin) && relative_eq!(self.extent, other.extent)
    }
}

impl Eq for SpriteSpatialIndexer {}

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
    pub bounds: Bounds, // 2d bounds of the vertices in this mesh

    sprite_element_indices: HashMap<SpriteSpatialIndexer, u32>,
}

impl Mesh {
    pub fn new(sprites: &[Sprite], material: usize, device: &wgpu::Device, name: &str) -> Self {
        let mut left = f32::MAX;
        let mut bottom = f32::MAX;
        let mut right = f32::MIN;
        let mut top = f32::MIN;

        let mut vertices = vec![];
        let mut indices = vec![];
        let mut sprite_element_indices: HashMap<SpriteSpatialIndexer, u32> = HashMap::new();
        for sprite in sprites {
            // update bounds
            left = left.min(sprite.left());
            bottom = bottom.min(sprite.bottom());
            right = right.max(sprite.right());
            top = top.max(sprite.top());

            // compute quad vertices
            let p_a = vec3(sprite.origin.x, sprite.origin.y, sprite.origin.z);
            let p_b = vec3(
                sprite.origin.x + sprite.extent.x,
                sprite.origin.y,
                sprite.origin.z,
            );
            let p_c = vec3(
                sprite.origin.x + sprite.extent.x,
                sprite.origin.y + sprite.extent.y,
                sprite.origin.z,
            );
            let p_d = vec3(
                sprite.origin.x,
                sprite.origin.y + sprite.extent.y,
                sprite.origin.z,
            );

            let mut tc_a = vec2::<f32>(sprite.tex_coord_origin.x, 1.0 - sprite.tex_coord_origin.y);
            let mut tc_b = vec2::<f32>(
                sprite.tex_coord_origin.x + sprite.tex_coord_extent.x,
                1.0 - (sprite.tex_coord_origin.y),
            );
            let mut tc_c = vec2::<f32>(
                sprite.tex_coord_origin.x + sprite.tex_coord_extent.x,
                1.0 - (sprite.tex_coord_origin.y + sprite.tex_coord_extent.y),
            );
            let mut tc_d = vec2::<f32>(
                sprite.tex_coord_origin.x,
                1.0 - (sprite.tex_coord_origin.y + sprite.tex_coord_extent.y),
            );

            if sprite.flipped_diagonally {
                std::mem::swap(&mut tc_a, &mut tc_c);
            }

            if sprite.flipped_horizontally {
                std::mem::swap(&mut tc_a, &mut tc_b);
                std::mem::swap(&mut tc_d, &mut tc_c);
            }

            if sprite.flipped_vertically {
                std::mem::swap(&mut tc_a, &mut tc_d);
                std::mem::swap(&mut tc_b, &mut tc_c);
            }

            let sv_a = Vertex::new(p_a, tc_a, vec2(-1.0, -1.0), sprite.color);
            let sv_b = Vertex::new(p_b, tc_b, vec2(1.0, -1.0), sprite.color);
            let sv_c = Vertex::new(p_c, tc_c, vec2(1.0, 1.0), sprite.color);
            let sv_d = Vertex::new(p_d, tc_d, vec2(-1.0, 1.0), sprite.color);
            let idx = vertices.len();

            vertices.push(sv_a);
            vertices.push(sv_b);
            vertices.push(sv_c);
            vertices.push(sv_d);

            sprite_element_indices.insert(SpriteSpatialIndexer::from(sprite), indices.len() as u32);
            indices.push(idx as u32);
            indices.push((idx + 1) as u32);
            indices.push((idx + 2) as u32);

            indices.push(idx as u32);
            indices.push((idx + 2) as u32);
            indices.push((idx + 3) as u32);
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", name)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Index Buffer", name)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let num_elements = indices.len() as u32;

        let bounds = Bounds::new(point2(left, bottom), vec2(right - left, top - bottom));

        Self {
            vertex_buffer,
            index_buffer,
            num_elements,
            material,
            bounds,
            sprite_element_indices,
        }
    }

    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        material: &'a Material,
        camera_uniforms: &'a camera::Uniforms,
        sprite_uniforms: &'a Uniforms,
    ) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, &material.bind_group, &[]);
        render_pass.set_bind_group(1, &camera_uniforms.bind_group, &[]);
        render_pass.set_bind_group(2, &sprite_uniforms.bind_group, &[]);
        render_pass.draw_indexed(0..self.num_elements, 0, 0..1);
    }

    /// Draws the sprites that best match the sprites in I. Since meshes are expensive to create, we
    /// attempt to find the sprite mesh that best corresponds to the ones in the provided array, and
    /// draw them instead. This means the sprite passes in only has to have the same 2D origin and extent to
    /// match up to a sprite in this mesh to draw that mesh sprite. The other fields - z, texcoords, color, etc are ignored.
    /// This is a hack, but it's for displaying collision response, where the collision system has no knowledge of
    /// the graphics system, and as such, there's no good way to extract exact sprite data from a collision response,
    /// just position and extent of the collider.
    pub fn draw_sprites<'a, 'b, I>(
        &'a self,
        sprites: I,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        material: &'a Material,
        camera_uniforms: &'a camera::Uniforms,
        sprite_uniforms: &'a Uniforms,
    ) where
        I: IntoIterator<Item = &'a Sprite>,
    {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, &material.bind_group, &[]);
        render_pass.set_bind_group(1, &camera_uniforms.bind_group, &[]);
        render_pass.set_bind_group(2, &sprite_uniforms.bind_group, &[]);

        for sprite in sprites.into_iter() {
            if let Some(index) = self
                .sprite_element_indices
                .get(&SpriteSpatialIndexer::from(sprite))
            {
                render_pass.draw_indexed(*index..*index + 6, 0, 0..1);
            }
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// Drawable manages a vec of Mesh and Material, such that each Mesh's material index can point to a
/// specific Material. The common case is for a Drawable to be made with a single mesh and material pair.
#[derive(Default)]
pub struct Drawable {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Rc<Material>>,
}

impl Drawable {
    pub fn with(mesh: Mesh, material: Rc<Material>) -> Self {
        Self::new(vec![mesh], vec![material])
    }

    pub fn new(meshes: Vec<Mesh>, materials: Vec<Rc<Material>>) -> Self {
        if materials.is_empty() {
            panic!("Attempted to create Drawable without materials")
        }
        for m in &meshes {
            if m.material > materials.len() {
                panic!("Material index {} is out of range", m.material);
            }
        }

        Self { meshes, materials }
    }

    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_uniforms: &'a camera::Uniforms,
        sprite_uniforms: &'a Uniforms,
    ) {
        for mesh in &self.meshes {
            let material = &self.materials[mesh.material];
            mesh.draw(render_pass, material, camera_uniforms, sprite_uniforms);
        }
    }

    pub fn draw_sprites<'a, 'b, I>(
        &'a self,
        sprites: I,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        camera_uniforms: &'a camera::Uniforms,
        sprite_uniforms: &'a Uniforms,
    ) where
        // TODO: Not happy about this +Copy here, the sprites array is being copied for each pass of the loop?
        I: IntoIterator<Item = &'a Sprite> + Copy,
    {
        for mesh in &self.meshes {
            let material = &self.materials[mesh.material];
            mesh.draw_sprites(
                sprites,
                render_pass,
                material,
                camera_uniforms,
                sprite_uniforms,
            );
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// EntityDrawable is a Drawable for entities which will draw from res/entities.tsx tileset.
/// EntityDrawable allows for an entity to specify a subset name, e.g., "firebrand" and then a
/// specific cycle, e.g., "walk_0" to display when draw() is called.
pub struct EntityDrawable {
    // maps a string, e.g., "face_right" to a renderable mesh
    meshes_by_cycle: HashMap<String, Mesh>,

    material: Rc<Material>,

    // maps a string, e.g., "face_right" to a the sprites it is made up of
    sprites: HashMap<String, Vec<Sprite>>,
}

impl EntityDrawable {
    // Loads all tiles with the specified name from the tileset, gathering them by "cycle", populating
    // meshes_by_cycle accordingly.
    // REQUISITES:
    // All tiles part of an entity have a property "cycle"="some_noun" (e.g., "walk_1")
    // The root tile has property "role" = "root". All tiles will be placed relative to root, with root at (0,0)
    pub fn load(
        tileset: &tileset::TileSet,
        material: Rc<Material>,
        device: &wgpu::Device,
        named: &str,
        mask: u32,
    ) -> Self {
        let tiles = tileset.get_tiles_with_property("name", named);

        // collect all tiles for each cycle, and root tiles too
        let mut tiles_by_cycle: HashMap<&str, Vec<&tileset::Tile>> = HashMap::new();
        let mut root_tiles_by_cycle: HashMap<&str, &tileset::Tile> = HashMap::new();
        for tile in tiles {
            let cycle = tile.get_property("cycle").unwrap();
            tiles_by_cycle.entry(cycle).or_default().push(tile);

            if tile.get_property("role") == Some("root") {
                root_tiles_by_cycle.insert(cycle, tile);
            }
        }

        // now for each root tile, assemble Sprites
        let mut sprites_by_cycle: HashMap<String, Vec<Sprite>> = HashMap::new();
        for cycle in root_tiles_by_cycle.keys() {
            let root_tile = *root_tiles_by_cycle.get(cycle).unwrap();
            let tiles = tiles_by_cycle.get(cycle).unwrap();

            let root_position = tileset.get_tile_position(root_tile).cast::<i32>().unwrap();

            for tile in tiles {
                let tile_position = tileset.get_tile_position(tile).cast::<i32>().unwrap();

                let sprite_position = tile_position - root_position;

                let tex_coords = tileset.get_tex_coords_for_tile(tile);
                // now create a Sprite at this position
                let sprite = Sprite::unit(
                    tile.shape(),
                    point2(sprite_position.x, -sprite_position.y),
                    0.0,
                    tex_coords.origin,
                    tex_coords.extent,
                    vec4(1.0, 1.0, 1.0, 1.0),
                    mask,
                );

                sprites_by_cycle
                    .entry(cycle.to_string())
                    .or_default()
                    .push(sprite);
            }
        }

        // Convert sprites to sprite meshes
        Self::new(sprites_by_cycle, material, device)
    }

    pub fn new(
        sprites: HashMap<String, Vec<Sprite>>,
        material: Rc<Material>,
        device: &wgpu::Device,
    ) -> Self {
        let mut sprite_states = HashMap::new();

        for key in sprites.keys() {
            let sprites = sprites.get(key).unwrap();
            let mesh = Mesh::new(sprites, 0, device, key);
            sprite_states.insert(key.to_string(), mesh);
        }

        EntityDrawable {
            sprites,
            meshes_by_cycle: sprite_states,
            material,
        }
    }

    /// draws the mesh corresponding to "cycle". If no cycle by that name is found nothing will be drawn.
    pub fn draw<'a, 'b>(
        &'a self,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        camera_uniforms: &'a camera::Uniforms,
        sprite_uniforms: &'a Uniforms,
        cycle: &str,
    ) where
        'a: 'b,
    {
        if let Some(mesh) = self.meshes_by_cycle.get(cycle) {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, &self.material.bind_group, &[]);
            render_pass.set_bind_group(1, &camera_uniforms.bind_group, &[]);
            render_pass.set_bind_group(2, &sprite_uniforms.bind_group, &[]);
            render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

pub struct FlipbookAnimationDrawable {
    sequence: crate::map::SpriteFlipbookAnimation,
    mesh: Mesh,
    material: Rc<Material>,
}

impl FlipbookAnimationDrawable {
    pub fn new(
        sequence: crate::map::SpriteFlipbookAnimation,
        material: Rc<Material>,
        device: &wgpu::Device,
    ) -> Self {
        let mesh = Mesh::new(&sequence.sprites, 0, device, "Flipbook");

        Self {
            sequence,
            mesh,
            material,
        }
    }

    pub fn num_frames(&self) -> usize {
        self.sequence.offsets.len()
    }

    pub fn duration_for_frame(&self, frame: usize) -> Duration {
        self.sequence.durations[frame % self.sequence.durations.len()]
    }

    pub fn set_frame(&self, sprite_uniforms: &mut Uniforms, frame: usize) {
        sprite_uniforms
            .data
            .set_tex_coord_offset(self.sequence.offsets[frame % self.sequence.offsets.len()]);
    }

    pub fn draw<'a, 'b>(
        &'a self,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        camera_uniforms: &'a camera::Uniforms,
        sprite_uniforms: &'a Uniforms,
    ) where
        'a: 'b,
    {
        render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, &self.material.bind_group, &[]);
        render_pass.set_bind_group(1, &camera_uniforms.bind_group, &[]);
        render_pass.set_bind_group(2, &sprite_uniforms.bind_group, &[]);
        render_pass.draw_indexed(0..self.mesh.num_elements, 0, 0..1);
    }
}

// ---------------------------------------------------------------------------------------------------------------------

/// FlipbookAnimationComponents represents a unit owning a flipbook animation and the uniforms
/// it needs to render.
pub struct FlipbookAnimationComponents {
    pub drawable: FlipbookAnimationDrawable,
    pub uniforms: Uniforms,
    seconds_until_next_frame: f32,
    current_frame: usize,
}

impl FlipbookAnimationComponents {
    pub fn new(flipbook: FlipbookAnimationDrawable, uniforms: Uniforms) -> Self {
        let seconds_until_next_frame = flipbook.duration_for_frame(0).as_secs_f32();
        Self {
            drawable: flipbook,
            uniforms,
            seconds_until_next_frame,
            current_frame: 0,
        }
    }

    pub fn update(&mut self, dt: Duration) {
        let dt = dt.as_secs_f32();
        self.seconds_until_next_frame -= dt;
        if self.seconds_until_next_frame <= 0.0 {
            self.current_frame += 1;
            self.seconds_until_next_frame = self
                .drawable
                .duration_for_frame(self.current_frame)
                .as_secs_f32();

            self.drawable
                .set_frame(&mut self.uniforms, self.current_frame);
        }
    }
}
