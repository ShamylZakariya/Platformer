use anyhow::Result;

use crate::entity;
use crate::map;
use crate::sprite::{self, collision};
use crate::tileset;

pub mod falling_bridge;
pub mod fireball;
pub mod firebrand;
pub mod spawn_point;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityClass {
    Firebrand,
    Fireball,
    FallingBridge,
    SpawnPoint,
}

pub fn instantiate_from_map(
    classname: &str,
    sprite: &sprite::Sprite,
    tile: &tileset::Tile,
    map: &map::Map,
    collision_space: &mut collision::Space,
) -> Result<Box<dyn entity::Entity>> {
    if let Some(mut e) = match classname {
        "FallingBridge" => {
            Some(Box::new(falling_bridge::FallingBridge::default()) as Box<dyn entity::Entity>)
        }
        "Firebrand" => Some(Box::new(firebrand::Firebrand::default()) as Box<dyn entity::Entity>),
        "SpawnPoint" => {
            Some(Box::new(spawn_point::SpawnPoint::default()) as Box<dyn entity::Entity>)
        }
        _ => None,
    } {
        e.init_from_map_sprite(sprite, tile, map, collision_space);
        Ok(e)
    } else {
        anyhow::bail!("Unrecognized entity class \"{}\"", classname)
    }
}
