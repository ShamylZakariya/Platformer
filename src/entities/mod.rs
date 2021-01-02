use anyhow::Result;
use cgmath::Vector2;

use crate::{collision, entity::Entity, map, sprite, tileset};

pub mod falling_bridge;
pub mod firebrand;

pub fn instantiate(
    classname: &str,
    sprite: &sprite::SpriteDesc,
    tile: &tileset::Tile,
    map: &map::Map,
    collision_space: &mut collision::Space,
    sprite_size_px: Vector2<f32>,
) -> Result<Box<dyn Entity>> {
    if let Some(mut e) = match classname {
        "FallingBridge" => {
            Some(Box::new(falling_bridge::FallingBridge::default()) as Box<dyn Entity>)
        }
        "Firebrand" => Some(Box::new(firebrand::Firebrand::default()) as Box<dyn Entity>),
        _ => None,
    } {
        e.init(sprite, tile, map, collision_space, sprite_size_px);
        Ok(e)
    } else {
        anyhow::bail!("Unrecognized entity class \"{}\"", classname)
    }
}
