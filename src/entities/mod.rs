use anyhow::Result;

use crate::{collision, entity::Entity, map::Map, sprite::core::Sprite, tileset::Tile};

pub mod falling_bridge;
pub mod firebrand;

pub fn instantiate(
    classname: &str,
    sprite: &Sprite,
    tile: &Tile,
    map: &Map,
    collision_space: &mut collision::Space,
) -> Result<Box<dyn Entity>> {
    if let Some(mut e) = match classname {
        "FallingBridge" => {
            Some(Box::new(falling_bridge::FallingBridge::default()) as Box<dyn Entity>)
        }
        "Firebrand" => Some(Box::new(firebrand::Firebrand::default()) as Box<dyn Entity>),
        _ => None,
    } {
        e.init(sprite, tile, map, collision_space);
        Ok(e)
    } else {
        anyhow::bail!("Unrecognized entity class \"{}\"", classname)
    }
}
