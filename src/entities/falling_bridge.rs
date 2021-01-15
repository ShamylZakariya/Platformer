use std::{collections::HashSet, time::Duration};

use cgmath::{vec2, Point3, Vector2};
use winit::event::{ElementState, VirtualKeyCode};

use crate::{
    constants,
    entity::{Dispatcher, Entity, Event, Message},
    map,
    sprite::{self, collision, rendering},
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
            position: Point3::new(0.0, 0.0, 0.0),
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
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        map: &map::Map,
        collision_space: &mut collision::Space,
    ) {
        self.entity_id = sprite
            .entity_id
            .expect("Entity sprites should have an entity_id");
        self.sprite = Some(*sprite);
        self.position = sprite.origin;
        self.sprite_size_px = vec2(
            map.tileset.tile_width as f32,
            map.tileset.tile_height as f32,
        );
        collision_space.add_sprite(sprite);
    }

    fn update(
        &mut self,
        dt: Duration,
        collision_space: &mut collision::Space,
        _message_dispatcher: &mut Dispatcher,
    ) {
        let dt = dt.as_secs_f32();

        if self.is_falling {
            self.vertical_velocity = constants::apply_gravity(self.vertical_velocity, dt);
            self.position.y += self.vertical_velocity * dt;
        } else {
            if let Some(mut time_remaining) = self.time_remaining {
                time_remaining -= dt;
                if time_remaining <= 0.0 {
                    // we're done!
                    self.is_falling = true;
                    self.time_remaining = None;

                    collision_space.remove_sprite(
                        &self
                            .sprite
                            .expect("Should have a sprite associated with FallingBridge instance"),
                    );
                } else {
                    self.time_remaining = Some(time_remaining);
                }
            }
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        uniforms.data.set_model_position(&self.position);
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn is_alive(&self) -> bool {
        // once we fall off bottom of the screen, we're done
        self.position.y > -1.0
    }

    fn position(&self) -> Point3<f32> {
        Point3::new(
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
        match message.event {
            Event::CharacterContact => {
                if self.time_remaining.is_none() {
                    self.position.y -= 2.0 / self.sprite_size_px.y;
                    self.time_remaining = Some(FALLING_BRIDGE_CONTACT_DELAY);
                }
            }
            _ => {}
        }
    }
}
