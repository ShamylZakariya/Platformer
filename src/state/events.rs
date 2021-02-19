use cgmath::*;

use crate::{entities::util::HorizontalDir, sprite, tileset};

/// An Event payload for Message
#[derive(Debug, Clone)]
pub enum Event {
    /// Sent when Firebrand is created and game has started
    FirebrandCreated,

    /// Received by an Entity when contacted by Firebrand
    FirebrandContact,

    /// broadcast by Firebrand when they die
    FirebrandDied,

    /// Sent by Firebrand to GameState to notify change of health, flight time, etc
    FirebrandStatusChanged {
        health: (u32, u32), // tuple of (current health, max health)
        flight: (f32, f32), // tuple of (flight time remaining, max flight time) in seconds
        vials: u32,
        lives: u32,
    },

    /// Sent by a checkpoint - once - when firebrand passes it
    FirebrandPassedCheckpoint,

    /// Sent by Firebrand to State to signal request to shoot fireball.
    /// If State determines a fireball may be shot (there is some rate limiting)
    /// State will reply with DidShootFireball
    TryShootFireball {
        origin: Point2<f32>,
        direction: HorizontalDir,
        velocity: f32,
        damage: u32,
    },

    /// Sent to Firebrand when a fireball was successfully shot
    DidShootFireball,

    /// Received by an entity when hit by Firebrand's fireball
    HitByFireball {
        // direction of fireball travel, -1 for left, +1 for right
        direction: HorizontalDir,
        damage: u32,
    },

    /// Sent by an entity to GameState to signal request to spawn an entity.
    /// Generally sent by SpawnPoint to request spawning their enemy type.
    /// GameState responds with EntityWasSpawned to signal spawn result.
    SpawnEntity {
        class_name: String,
        spawn_point_sprite: sprite::Sprite,
        spawn_point_tile: tileset::Tile,
    },

    /// Response from GameState to signal if requested entity was spawned.
    /// Bears the spawned entity id on success, None otherwise.
    EntityWasSpawned {
        entity_id: Option<u32>,
    },

    /// Sent by a spawned entity to its spawn point when it dies
    SpawnedEntityDidDie,

    /// Sent by a dying entity to GameState to request display of a death animation
    PlayEntityDeathAnimation {
        // position of death animation
        position: Point2<f32>,
        // direction it should travel, -1 being left, +1 for right
        direction: HorizontalDir,
    },

    /// Sent by BossFish to launch a FireSprite
    ShootFiresprite {
        position: Point2<f32>,
        dir: Vector2<f32>,
        velocity: f32,
        damage: u32,
    },

    /// Sent by boss to GameState when the boss fight starts
    BossEncountered {
        arena_left: f32,
    },

    /// Sent by boss to GameState when defeated
    BossDefeated,

    /// Sent by boss to GameState when death aniamtion completes
    BossDied,

    /// Sent after boiss fight finishes to raise the floor to make exit door accessible
    RaiseExitFloor,

    /// Sent after the floor finishes raising, to signal opening of the exit door.
    OpenExitDoor,

    /// Sent after the exit door finishes opening
    ExitDoorOpened,

    // Sent when the player passes through the exit door
    PlayerPassedThroughExitDoor,

    StartCameraShake {
        // vector of camera offsets in world units, and timing delay for that offset
        pattern: Vec<(Vector2<f32>, f32)>,
    },

    EndCameraShake,

    // Broadcast when firebrand has died with no remaining lives
    GameOver,

    // Broadcast when GameState reset the level after player death
    ResetState,
}
