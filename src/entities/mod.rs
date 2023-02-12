pub mod bat;
pub mod boss_fight_trigger;
pub mod boss_fish;
pub mod check_point;
pub mod death_animation;
pub mod exit_door;
pub mod falling_bridge;
pub mod fire_sprite;
pub mod fireball;
pub mod firebrand;
pub mod flying_fish;
pub mod hoodie;
pub mod power_up;
pub mod rising_floor;
pub mod spawn_point;
pub mod ui_digit;
pub mod ui_flight_bar;
pub mod ui_health_dot;
pub mod util;

use anyhow::Result;

use crate::collision;
use crate::entity;
use crate::map;
use crate::sprite;
use crate::tileset;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityClass {
    Bat,
    BossFightTrigger,
    BossFish,
    CheckPoint,
    DeathAnimation,
    ExitDoor,
    Firebrand,
    FirebrandDeath,
    Fireball,
    FireSprite,
    FallingBridge,
    FlyingFish,
    Hoodie,
    PowerUp,
    RisingFloor,
    SpawnPoint,

    // Ui classes
    UiDigit,
    UiFlightBar,
    UiHealthDot,
}

impl EntityClass {
    pub fn is_enemy(&self) -> bool {
        matches!(
            self,
            EntityClass::Bat
                | EntityClass::FlyingFish
                | EntityClass::FireSprite
                | EntityClass::Hoodie
        )
    }
    pub fn is_projectile(&self) -> bool {
        matches!(self, EntityClass::Fireball)
    }
    pub fn is_boss(&self) -> bool {
        matches!(self, EntityClass::BossFish)
    }
    pub fn is_player(&self) -> bool {
        matches!(self, EntityClass::Firebrand)
    }
    pub fn is_ui(&self) -> bool {
        matches!(
            self,
            EntityClass::UiDigit | EntityClass::UiFlightBar | EntityClass::UiHealthDot
        )
    }
    pub fn survives_level_restart(&self) -> bool {
        use EntityClass::*;
        matches!(
            self,
            BossFightTrigger
                | CheckPoint
                | ExitDoor
                | FallingBridge
                | RisingFloor
                | SpawnPoint
                | PowerUp
        )
    }
}

pub fn instantiate_entity_by_class_name(classname: &str) -> Option<Box<dyn entity::Entity>> {
    match classname {
        "BossFightTrigger" => {
            Some(Box::<boss_fight_trigger::BossFightTrigger>::default() as Box<dyn entity::Entity>)
        }
        "BossFish" => Some(Box::<boss_fish::BossFish>::default() as Box<dyn entity::Entity>),
        "Bat" => Some(Box::<bat::Bat>::default() as Box<dyn entity::Entity>),
        "CheckPoint" => Some(Box::<check_point::CheckPoint>::default() as Box<dyn entity::Entity>),
        "FallingBridge" => {
            Some(Box::<falling_bridge::FallingBridge>::default() as Box<dyn entity::Entity>)
        }
        "FireSprite" => Some(Box::<fire_sprite::FireSprite>::default() as Box<dyn entity::Entity>),
        "FlyingFish" => Some(Box::<flying_fish::FlyingFish>::default() as Box<dyn entity::Entity>),
        "Hoodie" => Some(Box::<hoodie::Hoodie>::default() as Box<dyn entity::Entity>),
        "PowerUp" => Some(Box::<power_up::PowerUp>::default() as Box<dyn entity::Entity>),
        "SpawnPoint" => Some(Box::<spawn_point::SpawnPoint>::default() as Box<dyn entity::Entity>),

        // Ui entities
        "UiDigit" => Some(Box::<ui_digit::UiDigit>::default() as Box<dyn entity::Entity>),

        "UiFlightBar" => {
            Some(Box::<ui_flight_bar::UiFlightBar>::default() as Box<dyn entity::Entity>)
        }

        "UiHealthDot" => {
            Some(Box::<ui_health_dot::UiHealthDot>::default() as Box<dyn entity::Entity>)
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
