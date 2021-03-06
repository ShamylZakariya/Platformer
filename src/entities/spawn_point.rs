use std::time::Duration;

use cgmath::*;

use crate::{
    audio, collision,
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map, sprite,
    state::{constants::layers, events::Event},
    tileset,
};

pub struct SpawnPoint {
    entity_id: u32,
    position: Point3<f32>,
    sprite: Option<sprite::Sprite>,
    tile: Option<tileset::Tile>,
    spawned_entity_id: Option<u32>,
    did_become_visible: bool,
}

impl Default for SpawnPoint {
    fn default() -> Self {
        Self {
            entity_id: 0,
            position: point3(0.0, 0.0, 0.0),
            sprite: None,
            tile: None,
            spawned_entity_id: None,
            did_become_visible: false,
        }
    }
}

impl Entity for SpawnPoint {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        tile: &tileset::Tile,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.position = point3(sprite.origin.x, sprite.origin.y, layers::stage::ENTITIES);
        self.sprite = Some(*sprite);
        self.tile = Some(tile.clone());
    }

    fn update(
        &mut self,
        _dt: Duration,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
        _audio: &mut audio::Audio,
        message_dispatcher: &mut Dispatcher,
        _game_state_peek: &GameStatePeek,
    ) {
        if self.did_become_visible && self.spawned_entity_id.is_none() {
            let sprite = self
                .sprite
                .expect("SpawnPoint must be initialized from a map sprite");
            let tile = self
                .tile
                .as_ref()
                .expect("SpawnPoint must be initialized from a map sprite")
                .clone();
            let class_name = tile
                .get_property("spawned_entity_class")
                .expect("Spawn point must have \"spawned_entity_class\" property on tile")
                .to_string();

            message_dispatcher.entity_to_global(
                self.entity_id(),
                Event::SpawnEntity {
                    class_name,
                    spawn_point_sprite: sprite,
                    spawn_point_tile: tile,
                },
            );
        }

        self.did_become_visible = false;
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::SpawnPoint
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn should_draw(&self) -> bool {
        false
    }

    fn handle_message(&mut self, message: &Message) {
        match message.event {
            Event::SpawnedEntityDidDie => {
                assert!(message.sender_entity_id == self.spawned_entity_id);
                self.spawned_entity_id = None;
            }
            Event::EntityWasSpawned { entity_id } => {
                self.spawned_entity_id = entity_id;
            }
            Event::ResetState => {
                self.spawned_entity_id = None;
                self.did_become_visible = false;
            }
            _ => {}
        }
    }

    fn did_enter_viewport(&mut self) {
        self.did_become_visible = true;
    }
}
