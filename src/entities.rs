use std::time::Duration;

use anyhow::Result;
use cgmath::Point3;

use crate::collision;
use crate::constants;
use crate::sprite;
use crate::tileset;

pub struct EntityIdVendor {
    current_id: u32,
}

impl Default for EntityIdVendor {
    fn default() -> Self {
        EntityIdVendor { current_id: 0u32 }
    }
}

impl EntityIdVendor {
    pub fn next_id(&mut self) -> u32 {
        let r = self.current_id;
        self.current_id += 1;
        r
    }
}

// ---------------------------------------------------------------------------------------------------------------------

pub trait Entity {
    fn init(
        &mut self,
        sprite: &sprite::SpriteDesc,
        tile: &tileset::Tile,
        collision_space: &mut collision::Space,
    );
    fn update(
        &mut self,
        dt: Duration,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
        uniforms: &mut sprite::Uniforms,
    );
    fn entity_id(&self) -> u32;
    fn is_alive(&self) -> bool;
    fn sprite_name(&self) -> &str;
    fn sprite_cycle(&self) -> &str;
    fn handle_collision(&mut self, message: &Message);
}

pub fn instantiate(
    classname: &str,
    sprite: &sprite::SpriteDesc,
    tile: &tileset::Tile,
    collision_space: &mut collision::Space,
) -> Result<Box<dyn Entity>> {
    if let Some(mut e) = match classname {
        "FallingBridge" => Some(Box::new(FallingBridge::default())),
        _ => None,
    } {
        e.init(sprite, tile, collision_space);
        Ok(e)
    } else {
        anyhow::bail!("Unrecognized entity class \"{}\"", classname)
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub enum Event {
    CharacterContact,
}

#[derive(Debug, Clone, Copy)]
pub struct Message {
    pub entity_id: u32,
    pub event: Event,
}

impl Message {
    pub fn new(entity_id: u32, event: Event) -> Self {
        Message { entity_id, event }
    }
}

pub struct Dispatcher {
    pub messages: Vec<Message>,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Dispatcher { messages: vec![] }
    }
}

impl Dispatcher {
    pub fn enqueue(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

// ---------------------------------------------------------------------------------------------------------------------

const FALLING_BRIDGE_CONTACT_DELAY: f32 = 0.2;

struct FallingBridge {
    entity_id: u32,
    sprite: Option<sprite::SpriteDesc>,
    position: Point3<f32>,
    time_remaining: Option<f32>,
    is_falling: bool,
    vertical_velocity: f32,
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
        }
    }
}

impl Entity for FallingBridge {
    fn init(
        &mut self,
        sprite: &sprite::SpriteDesc,
        _tile: &tileset::Tile,
        collision_space: &mut collision::Space,
    ) {
        self.entity_id = sprite
            .entity_id
            .expect("Entity sprites should have an entity_id");
        self.sprite = Some(*sprite);
        self.position = sprite.origin;
        collision_space.add_sprite(sprite);
    }

    fn update(
        &mut self,
        dt: Duration,
        collision_space: &mut collision::Space,
        _message_dispatcher: &mut Dispatcher,
        uniforms: &mut sprite::Uniforms,
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

        uniforms.data.set_model_position(&self.position);
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn is_alive(&self) -> bool {
        // once we fall off bottom of the screen, we're done
        self.position.y > -1.0
    }

    fn sprite_name(&self) -> &str {
        "falling_bridge"
    }

    fn sprite_cycle(&self) -> &str {
        "default"
    }

    fn handle_collision(&mut self, message: &Message) {
        match message.event {
            Event::CharacterContact => {
                if self.time_remaining.is_none() {
                    self.position.y -= 2.0 / 16.0; // TODO: Plumb in pixel density
                    self.time_remaining = Some(FALLING_BRIDGE_CONTACT_DELAY);
                }
            }
        }
    }
}
