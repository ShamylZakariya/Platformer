use std::time::Duration;

use cgmath::Vector2;

use crate::collision;
use crate::sprite;
use crate::tileset;

pub struct IdVendor {
    current_id: u32,
}

impl Default for IdVendor {
    fn default() -> Self {
        IdVendor { current_id: 0u32 }
    }
}

impl IdVendor {
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
        sprite_size_px: Vector2<f32>,
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
