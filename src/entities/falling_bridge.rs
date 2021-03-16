use std::time::Duration;

use cgmath::*;

use crate::{
    audio, collision,
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, rendering},
    state::{
        constants::{self, layers, sprite_masks},
        events::Event,
    },
    tileset,
};

const FALLING_BRIDGE_CONTACT_DELAY: f32 = 0.2;

pub struct FallingBridge {
    entity_id: u32,
    collider_id: Option<u32>,
    position: Point3<f32>,
    offset: Vector3<f32>,
    time_remaining: Option<f32>,
    is_falling: bool,
    vertical_velocity: f32,
    sprite_size_px: Vector2<f32>,
}

impl Default for FallingBridge {
    fn default() -> Self {
        Self {
            entity_id: 0,
            collider_id: None,
            position: point3(0.0, 0.0, 0.0),
            offset: vec3(0.0, 0.0, 0.0),
            time_remaining: None,
            is_falling: false,
            vertical_velocity: 0.0,
            sprite_size_px: vec2(0.0, 0.0),
        }
    }
}

impl Entity for FallingBridge {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        map: &map::Map,
        collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.position = point3(sprite.origin.x, sprite.origin.y, layers::stage::LEVEL);
        self.sprite_size_px = map.tileset.get_sprite_size().cast().unwrap();

        // we need to use a dynamic collider so we can assign an entity id
        self.collider_id = Some(
            collision_space.add_collider(collision::Collider::new_dynamic(
                sprite.bounds(),
                entity_id,
                collision::Shape::Square,
                sprite_masks::ENTITY | sprite_masks::GROUND | sprite_masks::RATCHET,
            )),
        );
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        _audio: &mut audio::Audio,
        _message_dispatcher: &mut Dispatcher,
        _game_state_peek: &GameStatePeek,
    ) {
        let dt = dt.as_secs_f32();
        let collider_id = self.collider_id.unwrap();

        if self.is_falling && self.should_draw() {
            self.vertical_velocity = constants::apply_gravity(self.vertical_velocity, dt);
            self.offset.y += self.vertical_velocity * dt;
        } else if let Some(mut time_remaining) = self.time_remaining {
            time_remaining -= dt;
            if time_remaining <= 0.0 {
                // we're done!
                self.is_falling = true;
                self.time_remaining = None;

                collision_space.deactivate_collider(collider_id);
            } else {
                self.time_remaining = Some(time_remaining);
            }
        } else if !collision_space.is_collider_activated(collider_id) {
            collision_space.activate_collider(collider_id);
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        uniforms
            .data
            .set_model_position(self.position + self.offset);
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::FallingBridge
    }

    fn should_draw(&self) -> bool {
        self.position.y + self.offset.y > -1.0
    }

    fn is_alive(&self) -> bool {
        true
    }

    fn position(&self) -> Point3<f32> {
        self.position + self.offset
    }

    fn sprite_name(&self) -> &str {
        "falling_bridge"
    }

    fn sprite_cycle(&self) -> &str {
        "default"
    }

    fn handle_message(&mut self, message: &Message) {
        match message.event {
            Event::FirebrandContact => {
                if self.time_remaining.is_none() {
                    self.offset.y -= 2.0 / self.sprite_size_px.y;
                    self.time_remaining = Some(FALLING_BRIDGE_CONTACT_DELAY);
                }
            }
            Event::ResetState => {
                self.time_remaining = None;
                self.is_falling = false;
                self.vertical_velocity = 0.0;
                self.offset = vec3(0.0, 0.0, 0.0);
            }
            _ => {}
        }
    }
}
