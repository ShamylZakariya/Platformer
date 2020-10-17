use crate::texture;

// --------------------------------------------------------------------------------------------------------------------

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SpriteVertex {
    pub position: cgmath::Vector3<f32>,
    pub tex_coords: cgmath::Vector2<f32>,
}
unsafe impl bytemuck::Zeroable for SpriteVertex {}
unsafe impl bytemuck::Pod for SpriteVertex {}

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
                }
            ],
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------


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

pub struct SpriteMesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

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

impl SpriteCollection {
    pub fn default() -> Self {
        Self {
            meshes: vec![],
            materials: vec![],
        }
    }
}

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