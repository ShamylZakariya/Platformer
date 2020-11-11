use crate::sprite;
use crate::tileset;
use std::collections::{HashMap, HashSet};

struct SpriteEntity {
    // maps a string, e.g., "face_right" to a renderable mesh
    meshes_by_cycle: HashMap<String, sprite::SpriteMesh>,

    // TODO: this should be &sprite::SPriteMaterial so multiple entities can share a single spritesheet?
    material: sprite::SpriteMaterial,
}

impl SpriteEntity {
    // Loads all tiles with the specified name from the tileset, gathering them by "cycle", populating
    // meshes_by_cycle accordingly. It ius required that all cycle groups are of same size, e.g., in
    // the case of the firebrand tiles, each cycle is made up of 3x2 tiles.
    // The tile with property root=true is the center tile.
    pub fn load(tileset: &tileset::TileSet, named: &str) -> Self {
        let named = named.to_string();
        let tiles = tileset
            .tiles
            .iter()
            .filter(|tile| tile.get_property("name") == Some(&named))
            .collect::<Vec<_>>();

        let mut tiles_by_cycle: HashMap<&String, Vec<&tileset::Tile>> = HashMap::new();
        for tile in tiles {
            let cycle = tile.get_property("cycle").unwrap();
            if let Some(tiles) = tiles_by_cycle.get_mut(cycle) {
                tiles.push(tile);
            } else {
                let v = vec![tile];
                tiles_by_cycle.insert(cycle, v);
            }
        }

        // now we need to find the width/height of a cycle
        // no need to enfore, just find the min/max dims of the tiles of the first cycle,
        // and assume that the requirement that all cycles are same dim is satisfied in editor.

        let sprite_descs_by_cycle: HashMap<String, Vec<sprite::SpriteDesc>> = HashMap::new();

        todo!();
    }

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
            meshes_by_cycle: sprite_states,
            material,
        }
    }

    // draws the mesh named "what"
    pub fn draw<'a, 'b>(
        &'a self,
        what: &str,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        camera_uniforms: &'a wgpu::BindGroup,
        sprite_uniforms: &'a wgpu::BindGroup,
    ) where
        'a: 'b,
    {
        if let Some(mesh) = self.meshes_by_cycle.get(what) {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..));
            render_pass.set_bind_group(0, &self.material.bind_group, &[]);
            render_pass.set_bind_group(1, &camera_uniforms, &[]);
            render_pass.set_bind_group(2, &sprite_uniforms, &[]);
            render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
        }
    }
}
