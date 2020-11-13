use std::collections::HashMap;
use std::rc::Rc;

use crate::sprite;
use crate::tileset;

pub struct SpriteEntity {
    // maps a string, e.g., "face_right" to a renderable mesh
    meshes_by_cycle: HashMap<String, sprite::SpriteMesh>,

    // TODO: this should be &sprite::SPriteMaterial so multiple entities can share a single spritesheet?
    material: Rc<sprite::SpriteMaterial>,
}

impl SpriteEntity {
    // Loads all tiles with the specified name from the tileset, gathering them by "cycle", populating
    // meshes_by_cycle accordingly.
    // REQUISITES:
    // All tiles part of an entity have a property "cycle"="some_noun" (e.g., "walk_1")
    // The root tile has property "role" = "root". All tiles will be placed relative to root, with root at (0,0)
    pub fn load(
        tileset: &tileset::TileSet,
        material: Rc<sprite::SpriteMaterial>,
        device: &wgpu::Device,
        named: &str,
        z_depth: f32,
        mask: u32,
    ) -> Self {
        let named = named.to_string();
        let tiles = tileset
            .tiles
            .iter()
            .filter(|tile| tile.get_property("name") == Some(&named))
            .collect::<Vec<_>>();

        // collect all tiles for each cycle, and root tiles too
        let mut tiles_by_cycle: HashMap<&str, Vec<&tileset::Tile>> = HashMap::new();
        let mut root_tiles_by_cycle: HashMap<&str, &tileset::Tile> = HashMap::new();
        for tile in tiles {
            let cycle = tile.get_property("cycle").unwrap();
            tiles_by_cycle.entry(cycle).or_insert(Vec::new()).push(tile);

            if tile.get_property("role") == Some("root") {
                root_tiles_by_cycle.insert(cycle, tile);
            }
        }

        // now for each root tile, assemble SpriteDescs
        let mut sprite_descs_by_cycle: HashMap<&str, Vec<sprite::SpriteDesc>> = HashMap::new();
        for cycle in root_tiles_by_cycle.keys() {
            let root_tile = *root_tiles_by_cycle.get(cycle).unwrap();
            let tiles = tiles_by_cycle.get(cycle).unwrap();

            let root_position = tileset.get_tile_position(root_tile).cast::<i32>().unwrap();

            for tile in tiles {
                let tile_position = tileset.get_tile_position(tile).cast::<i32>().unwrap();

                let sprite_position = tile_position - root_position;
                let sprite_position =
                    cgmath::Point2::new(sprite_position.x as i32, sprite_position.y as i32);
                let (tex_coords, tex_extents) = tileset.get_tex_coords_for_tile(tile);
                // now create a SpriteDesc at this position
                let sd = sprite::SpriteDesc::unit(
                    tile.shape(),
                    sprite_position,
                    z_depth,
                    tex_coords,
                    tex_extents,
                    cgmath::vec4(1.0, 1.0, 1.0, 1.0),
                    mask,
                );

                sprite_descs_by_cycle
                    .entry(cycle)
                    .or_insert(Vec::new())
                    .push(sd);
            }
        }

        // now convert spritedescs into sprite meshes
        Self::new(&sprite_descs_by_cycle, material, device)
    }

    pub fn new(
        sprite_descs: &HashMap<&str, Vec<sprite::SpriteDesc>>,
        material: Rc<sprite::SpriteMaterial>,
        device: &wgpu::Device,
    ) -> Self {
        let mut sprite_states = HashMap::new();

        for key in sprite_descs.keys() {
            let descs = sprite_descs.get(key).unwrap();
            let mesh = sprite::SpriteMesh::new(descs, 0, device, key);
            sprite_states.insert(key.to_string(), mesh);
        }

        SpriteEntity {
            meshes_by_cycle: sprite_states,
            material,
        }
    }

    /// draws the mesh corresponding to "cycle"
    pub fn draw<'a, 'b>(
        &'a self,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        camera_uniforms: &'a wgpu::BindGroup,
        sprite_uniforms: &'a wgpu::BindGroup,
        cycle: &str,
    ) where
        'a: 'b,
    {
        if let Some(mesh) = self.meshes_by_cycle.get(cycle) {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..));
            render_pass.set_bind_group(0, &self.material.bind_group, &[]);
            render_pass.set_bind_group(1, &camera_uniforms, &[]);
            render_pass.set_bind_group(2, &sprite_uniforms, &[]);
            render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
        }
    }
}
