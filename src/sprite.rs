use crate::texture;
use wgpu::util::DeviceExt;

// --------------------------------------------------------------------------------------------------------------------

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SpriteVertex {
    pub position: cgmath::Vector3<f32>,
    pub tex_coord: cgmath::Vector2<f32>,
}
unsafe impl bytemuck::Zeroable for SpriteVertex {}
unsafe impl bytemuck::Pod for SpriteVertex {}

impl SpriteVertex {
    pub fn new(position: cgmath::Vector3<f32>, tex_coord: cgmath::Vector2<f32>) -> Self {
        Self {
            position,
            tex_coord,
        }
    }
}

impl Vertex for SpriteVertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float2,
                },
            ],
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub struct SpriteBounds {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub z: f32,
}

impl SpriteBounds {
    pub fn new(left: f32, top: f32, right: f32, bottom: f32, z: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
            z,
        }
    }
}

pub struct SpriteCollection {
    pub meshes: Vec<SpriteMesh>,
    pub materials: Vec<SpriteMaterial>,
}

pub struct SpriteMaterial {
    pub name: String,
    pub texture: texture::Texture,
    pub color: cgmath::Vector3<f32>,
    pub bind_group: wgpu::BindGroup,
}

#[allow(dead_code)]
pub struct SpriteMesh {
    vertices: Vec<SpriteVertex>,
    indices: Vec<u32>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

#[allow(dead_code)]
impl SpriteMaterial {
    pub fn new(
        device: &wgpu::Device,
        name: &str,
        texture: texture::Texture,
        color: cgmath::Vector3<f32>,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        // TODO: Add color to bind_group and its layout
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
            color,
            bind_group,
        }
    }
}

#[allow(dead_code)]
impl SpriteCollection {
    pub fn default() -> Self {
        Self {
            meshes: vec![],
            materials: vec![],
        }
    }

    pub fn new(meshes: Vec<SpriteMesh>, materials: Vec<SpriteMaterial>) -> Self {
        Self { meshes, materials }
    }
}

impl SpriteMesh {
    pub fn new(
        rects: &Vec<SpriteBounds>,
        material: usize,
        device: &wgpu::Device,
        name: &str,
    ) -> Self {
        let mut vertices = vec![];
        let mut indices = vec![];
        let tc_a = cgmath::vec2::<f32>(0.0, 0.0);
        let tc_b = cgmath::vec2::<f32>(1.0, 0.0);
        let tc_c = cgmath::vec2::<f32>(1.0, 1.0);
        let tc_d = cgmath::vec2::<f32>(0.0, 1.0);
        for rect in rects {
            let p_a = cgmath::vec3(rect.left, rect.top, rect.z);
            let p_b = cgmath::vec3(rect.right, rect.top, rect.z);
            let p_c = cgmath::vec3(rect.right, rect.bottom, rect.z);
            let p_d = cgmath::vec3(rect.left, rect.bottom, rect.z);
            let idx = vertices.len();
            vertices.push(SpriteVertex::new(p_a, tc_a));
            vertices.push(SpriteVertex::new(p_b, tc_b));
            vertices.push(SpriteVertex::new(p_c, tc_c));
            vertices.push(SpriteVertex::new(p_d, tc_d));
            indices.push((idx + 0) as u32);
            indices.push((idx + 1) as u32);
            indices.push((idx + 2) as u32);
            indices.push((idx + 0) as u32);
            indices.push((idx + 2) as u32);
            indices.push((idx + 3) as u32);
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", name)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Index Buffer", name)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsage::INDEX,
        });

        let num_elements = indices.len() as u32;

        Self {
            vertices,
            indices,
            vertex_buffer,
            index_buffer,
            num_elements,
            material,
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub trait DrawSprite<'a, 'b>
where
    'b: 'a,
{
    fn draw_sprite(
        &mut self,
        sprite_mesh: &'b SpriteMesh,
        material: &'b SpriteMaterial,
        uniforms: &'b wgpu::BindGroup,
    );

    fn draw_sprite_collection(
        &mut self,
        sprites: &'b SpriteCollection,
        uniforms: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawSprite<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_sprite(
        &mut self,
        sprite_mesh: &'b SpriteMesh,
        material: &'b SpriteMaterial,
        uniforms: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, sprite_mesh.vertex_buffer.slice(..));
        self.set_index_buffer(sprite_mesh.index_buffer.slice(..));
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, &uniforms, &[]);
        self.draw_indexed(0..sprite_mesh.num_elements, 0, 0..1);
    }

    fn draw_sprite_collection(
        &mut self,
        sprites: &'b SpriteCollection,
        uniforms: &'b wgpu::BindGroup,
    ) {
        for sprite_mesh in &sprites.meshes {
            let material = &sprites.materials[sprite_mesh.material];
            self.draw_sprite(sprite_mesh, material, uniforms);
        }
    }
}
