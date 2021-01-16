use anyhow::Result;

use crate::entity;
use crate::map;
use crate::sprite::{self, collision};
use crate::tileset;

pub mod falling_bridge;
pub mod fire_sprite;
pub mod fireball;
pub mod firebrand;
pub mod spawn_point;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityClass {
    Firebrand,
    Fireball,
    FallingBridge,
    SpawnPoint,
    FireSprite,
}

pub fn instantiate_entity_by_class_name(classname: &str) -> Option<Box<dyn entity::Entity>> {
    match classname {
        "FallingBridge" => {
            Some(Box::new(falling_bridge::FallingBridge::default()) as Box<dyn entity::Entity>)
        }
        "Firebrand" => Some(Box::new(firebrand::Firebrand::default()) as Box<dyn entity::Entity>),
        "FireSprite" => {
            Some(Box::new(fire_sprite::FireSprite::default()) as Box<dyn entity::Entity>)
        }
        "SpawnPoint" => {
            Some(Box::new(spawn_point::SpawnPoint::default()) as Box<dyn entity::Entity>)
        }
        _ => None,
    }
}

pub fn instantiate_map_sprite(
    classname: &str,
    sprite: &sprite::Sprite,
    tile: &tileset::Tile,
    map: &map::Map,
    collision_space: &mut collision::Space,
    entity_id_vendor: Option<&mut entity::IdVendor>,
) -> Result<Box<dyn entity::Entity>> {
    if let Some(mut e) = instantiate_entity_by_class_name(classname) {
        if let Some(id_vendor) = entity_id_vendor {
            e.init_from_map_sprite(id_vendor.next_id(), sprite, tile, map, collision_space);
        } else {
            let id = sprite
                .entity_id
                .expect("Expect entity_id on Sprite when loading from map");
            e.init_from_map_sprite(id, sprite, tile, map, collision_space);
        }
        Ok(e)
    } else {
        anyhow::bail!("Unable to instantiate entity class \"{}\"", classname)
    }
}
