use std::time::Duration;

use cgmath::{Point2, Vector2};
use winit::event::{ElementState, VirtualKeyCode};

use crate::sprite;
use crate::tileset;
use crate::{collision, map};

// ---------------------------------------------------------------------------------------------------------------------

/// IDVendor vends a new unique id, starting from zero, for each entity.
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
        map: &map::Map,
        collision_space: &mut collision::Space,
        sprite_size_px: Vector2<f32>,
    );
    fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool;
    fn update(
        &mut self,
        dt: Duration,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
    );
    fn update_uniforms(&self, uniforms: &mut sprite::Uniforms);

    fn entity_id(&self) -> u32;
    fn is_alive(&self) -> bool;
    fn position(&self) -> Point2<f32>;
    fn sprite_name(&self) -> &str;
    fn sprite_cycle(&self) -> &str;
    fn handle_message(&mut self, message: &Message);
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
