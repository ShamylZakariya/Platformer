use cgmath::*;

use crate::{sprite, tileset};

/// An Event payload for Message
#[derive(Debug, Clone)]
pub enum Event {
    /// Received by an Entity when contacted by the character
    CharacterContact,

    /// Sent by Firebrand to State to signal request to shoot fireball.
    /// If State determines a fireball may be shot (there is some rate limiting)
    /// State will reply with DidShootFireball
    TryShootFireball {
        origin: Point2<f32>,
        direction: crate::entities::fireball::Direction,
        velocity: f32,
    },

    /// Sent to Firebrand when a fireball was successfully shot
    DidShootFireball,

    /// Received by an entity when hit by Firebrand's fireball
    HitByFireball {
        // direction of fireball travel, -1 for left, +1 for right
        direction: i32,
    },

    /// Sent by an entity to Global to signal request to spawn an entity.
    /// Generally sent by SpawnPoint to request spawning their enemy type.
    /// Global responds with EntityWasSpawned to signal spawn result.
    SpawnEntity {
        class_name: String,
        spawn_point_sprite: sprite::Sprite,
        spawn_point_tile: tileset::Tile,
    },

    /// Response from Global to signal if requested entity was spawned.
    /// Bears the spawned entity id on success, None otherwise.
    EntityWasSpawned { entity_id: Option<u32> },

    /// Sent by a spawned entity to its spawn point when it dies
    SpawnedEntityDidDie,

    /// Sent by a dying entity to Global to request display of a death animation
    PlayEntityDeathAnimation {
        // position of death animation
        position: Point2<f32>,
        // direction it should travel, -1 being left, +1 for right
        direction: i32,
    },
}
