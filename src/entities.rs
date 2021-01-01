use std::time::Duration;

use anyhow::Result;
use cgmath::Point3;

use crate::sprite;
use crate::sprite_collision;
use crate::tileset;

pub struct EntityIdVendor {
    current_id: u32,
}

impl Default for EntityIdVendor {
    fn default() -> Self {
        EntityIdVendor { current_id: 1u32 }
    }
}

impl EntityIdVendor {
    pub fn next_id(&mut self) -> u32 {
        let r = self.current_id;
        self.current_id += 1;
        r
    }
}

// ---------------------------------------------------------------------------------------------------------------------

pub trait Entity {
    fn init(
        &mut self,
        sprite: &sprite::SpriteDesc,
        tile: &tileset::Tile,
        collision_space: &mut sprite_collision::CollisionSpace,
    );
    fn update(
        &mut self,
        dt: Duration,
        collision_space: &mut sprite_collision::CollisionSpace,
        uniforms: &mut sprite::Uniforms,
    );
    fn entity_id(&self) -> u32;
    fn is_alive(&self) -> bool;
    fn sprite_name(&self) -> &str;
    fn sprite_cycle(&self) -> &str;
}

pub fn instantiate(
    classname: &str,
    sprite: &sprite::SpriteDesc,
    tile: &tileset::Tile,
    collision_space: &mut sprite_collision::CollisionSpace,
) -> Result<Box<dyn Entity>> {
    if let Some(mut e) = match classname {
        "FallingBridge" => Some(Box::new(FallingBridge::default())),
        _ => None,
    } {
        e.init(sprite, tile, collision_space);
        Ok(e)
    } else {
        anyhow::bail!("Unrecognized entity class \"{}\"", classname)
    }
}

// ---------------------------------------------------------------------------------------------------------------------

struct FallingBridge {
    entity_id: u32,
    position: Point3<f32>,
}

impl Default for FallingBridge {
    fn default() -> Self {
        Self {
            entity_id: 0,
            position: Point3::new(0.0, 0.0, 0.0),
        }
    }
}

impl Entity for FallingBridge {
    fn init(
        &mut self,
        sprite: &sprite::SpriteDesc,
        _tile: &tileset::Tile,
        collision_space: &mut sprite_collision::CollisionSpace,
    ) {
        self.entity_id = sprite.entity_id.expect("Entity sprites should have an entity_id");
        self.position = sprite.origin;
        collision_space.add_sprite(sprite);
    }

    fn update(
        &mut self,
        _dt: Duration,
        _collision_space: &mut sprite_collision::CollisionSpace,
        uniforms: &mut sprite::Uniforms,
    ) {
        uniforms.data.set_model_position(&self.position);
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn is_alive(&self) -> bool {
        true
    }

    fn sprite_name(&self) -> &str {
        "falling_bridge"
    }

    fn sprite_cycle(&self) -> &str {
        "default"
    }
}
