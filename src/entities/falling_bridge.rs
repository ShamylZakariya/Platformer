use std::time::Duration;

use cgmath::*;

use crate::{
    entity::Entity,
    event_dispatch::*,
    map,
    sprite::{self, collision, rendering},
    state::{constants, events::Event},
    tileset,
};

const FALLING_BRIDGE_CONTACT_DELAY: f32 = 0.2;

pub struct FallingBridge {
    entity_id: u32,
    sprite: Option<sprite::Sprite>,
    position: Point3<f32>,
    time_remaining: Option<f32>,
    is_falling: bool,
    vertical_velocity: f32,
    sprite_size_px: Vector2<f32>,
}

impl Default for FallingBridge {
    fn default() -> Self {
        Self {
            entity_id: 0,
            sprite: None,
            position: point3(0.0, 0.0, 0.0),
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
        self.sprite = Some(*sprite);
        self.position = sprite.origin;
        self.sprite_size_px = map.tileset.get_sprite_size().cast().unwrap();
        collision_space.add_static_sprite(sprite);
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        _message_dispatcher: &mut Dispatcher,
    ) {
        let dt = dt.as_secs_f32();

        if self.is_falling {
            self.vertical_velocity = constants::apply_gravity(self.vertical_velocity, dt);
            self.position.y += self.vertical_velocity * dt;
        } else if let Some(mut time_remaining) = self.time_remaining {
            time_remaining -= dt;
            if time_remaining <= 0.0 {
                // we're done!
                self.is_falling = true;
                self.time_remaining = None;

                collision_space.remove_static_sprite(
                    &self
                        .sprite
                        .expect("Should have a sprite associated with FallingBridge instance"),
                );
            } else {
                self.time_remaining = Some(time_remaining);
            }
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        uniforms.data.set_model_position(self.position);
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::FallingBridge
    }

    fn is_alive(&self) -> bool {
        // once we fall off bottom of the screen, we're done
        self.position.y > -1.0
    }

    fn position(&self) -> Point3<f32> {
        point3(
            self.position.x,
            self.position.y,
            self.sprite.unwrap().origin.z,
        )
    }

    fn sprite_name(&self) -> &str {
        "falling_bridge"
    }

    fn sprite_cycle(&self) -> &str {
        "default"
    }

    fn handle_message(&mut self, message: &Message) {
        if let Event::CharacterContact = message.event {
            if self.time_remaining.is_none() {
                self.position.y -= 2.0 / self.sprite_size_px.y;
                self.time_remaining = Some(FALLING_BRIDGE_CONTACT_DELAY);
            }
        }
    }
}
