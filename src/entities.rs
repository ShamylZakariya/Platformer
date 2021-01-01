use std::time::Duration;

use anyhow::Result;
use cgmath::Point3;

use crate::sprite;
use crate::sprite_collision;
use crate::tileset;

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
    position: Point3<f32>,
}

impl Default for FallingBridge {
    fn default() -> Self {
        Self {
            position: Point3::new(0.0, 0.0, 0.0),
        }
    }
}

impl Entity for FallingBridge {
    fn init(
        &mut self,
        sprite: &sprite::SpriteDesc,
        tile: &tileset::Tile,
        collision_space: &mut sprite_collision::CollisionSpace,
    ) {
        self.position = sprite.origin;
        collision_space.add_sprite(sprite);
    }

    fn update(
        &mut self,
        dt: Duration,
        collision_space: &mut sprite_collision::CollisionSpace,
        uniforms: &mut sprite::Uniforms,
    ) {
        uniforms.data.set_model_position(&self.position);
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
