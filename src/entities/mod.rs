use anyhow::Result;
use cgmath::Vector2;

use crate::{collision, entity::Entity, sprite, tileset};

pub mod falling_bridge;

pub fn instantiate(
    classname: &str,
    sprite: &sprite::SpriteDesc,
    tile: &tileset::Tile,
    collision_space: &mut collision::Space,
    sprite_size_px: Vector2<f32>,
) -> Result<Box<dyn Entity>> {
    if let Some(mut e) = match classname {
        "FallingBridge" => Some(Box::new(falling_bridge::FallingBridge::default())),
        _ => None,
    } {
        e.init(sprite, tile, collision_space, sprite_size_px);
        Ok(e)
    } else {
        anyhow::bail!("Unrecognized entity class \"{}\"", classname)
    }
}
