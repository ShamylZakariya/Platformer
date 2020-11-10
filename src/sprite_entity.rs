use crate::sprite;
use std::collections::HashMap;

struct SpriteEntity {
    // maps a string, e.g., "face_right" to a renderable mesh
    sprite_states: HashMap<String, sprite::SpriteMesh>,

    // TODO: this should be &sprite::SPriteMaterial so multiple entities can share a single spritesheet?
    material: sprite::SpriteMaterial,
}

impl SpriteEntity {
    pub fn new(
        sprite_descs: &HashMap<String, Vec<sprite::SpriteDesc>>,
        material: sprite::SpriteMaterial,
        device: &wgpu::Device,
    ) -> Self {
        let mut sprite_states = HashMap::new();

        for key in sprite_descs.keys() {
            let descs = sprite_descs.get(key).unwrap();
            let mesh = sprite::SpriteMesh::new(descs, 0, device, key.as_str());
            sprite_states.insert(key.clone(), mesh);
        }

        SpriteEntity {
            sprite_states,
            material,
        }
    }

    // draws the mesh named "what"
    pub fn draw<'a, 'b>(&'a self, what:&str, render_pass: &'b mut wgpu::RenderPass<'a>, uniforms: &'a wgpu::BindGroup)
        where 'a : 'b
    {
        if let Some(mesh) = self.sprite_states.get(what) {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..));
            render_pass.set_bind_group(0, &self.material.bind_group, &[]);
            render_pass.set_bind_group(1, &uniforms, &[]);
            render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
        }
    }
}
