use std::{collections::HashSet, time::Duration};

use cgmath::Point3;
use winit::event::{ElementState, VirtualKeyCode};

use crate::map;
use crate::sprite::{self, collision, rendering};
use crate::tileset;

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
    /// Initializes a new Entity using a sprite/tile template loaded from the level map.
    /// # Arguments
    /// * `sprite` The sprite created from the tile from the level map which instantiated this Entity instance
    /// * `tile` the Tile from the level map which instantiated this Entity instance
    /// * `map` the map from which the Tile was loaded.
    /// * `collision_space` the shared collision space
    ///
    fn init_from_map_sprite(
        &mut self,
        sprite: &sprite::Sprite,
        tile: &tileset::Tile,
        map: &map::Map,
        collision_space: &mut collision::Space,
    );

    /// Initializes an entity whcih is not loaded from the level map. This is generally for dynamic
    /// entity creation not based on map tiles, such as fireballs, etc.
    fn init(&mut self, entity_id: u32, map: &map::Map, collision_space: &mut collision::Space);

    /// Handle keyboard input, returning true iff said input was consumed.
    fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool;

    /// Update internal state of entity.
    /// # Arguments
    /// * `dt` delta time since last update
    /// * `collision_space` the shared collision space for collision lookup. A moving sprite should update its position in the collision space
    /// * `message_dispatcher` the dispatcher for queing messages to be processed by entities at end of update loop.
    fn update(
        &mut self,
        dt: Duration,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
    );

    /// Write updated state into this entity's uniform buffer for rendering.
    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms);

    /// The unique id for this Entity, a value from [0,u32::MAX]
    fn entity_id(&self) -> u32;

    /// An entity should return true here so long as it needs to be updated and drawn.
    fn is_alive(&self) -> bool;

    /// Return true if you want this entity to draw right now. Entity will still be updated.
    fn should_draw(&self) -> bool;

    /// The current position of the entity
    fn position(&self) -> Point3<f32>;

    /// The name identifying the entity's sprites in the Entity spritesheet. E.g., "firebrand" or "falling_bridge"
    fn sprite_name(&self) -> &str;

    /// The current sprite "cycle", e.g., "walk_0", "default", etc.
    fn sprite_cycle(&self) -> &str;

    /// Handle receipt of a dispatched message.
    fn handle_message(&mut self, message: &Message);

    /// Return a set of overlapping sprites used for debug visualization, or None if not applicable.
    fn overlapping_sprites(&self) -> Option<&HashSet<sprite::Sprite>>;

    /// Return a set of contacting sprites used for debug visualization, or None if not applicable.
    fn contacting_sprites(&self) -> Option<&HashSet<sprite::Sprite>>;
}

// ---------------------------------------------------------------------------------------------------------------------

/// An Event payload for Message
#[derive(Debug, Clone, Copy)]
pub enum Event {
    /// Received by an Entity when contacted by the character
    CharacterContact,

    /// Recevied by the global handler to signal ;aunching a fireball.
    ShootFireball {
        origin: cgmath::Point2<f32>,
        velocity: cgmath::Vector2<f32>,
    },
}

/// A Message to be sent to an Entity instance.
#[derive(Debug, Clone, Copy)]
pub struct Message {
    /// The entity to which to route this Message.
    /// If None, the Stage will process the message
    pub entity_id: Option<u32>,
    /// The event payload describing whatever happened
    pub event: Event,
}

impl Message {
    /// Creates a message to be processed by the global handler
    pub fn global(event: Event) -> Self {
        Message {
            entity_id: None,
            event,
        }
    }

    /// Creates a message to be routed to a specific Entity
    pub fn routed_to(entity_id: u32, event: Event) -> Self {
        Message {
            entity_id: Some(entity_id),
            event,
        }
    }
}

pub trait MessageHandler {
    fn handle_message(&mut self, message: &Message);
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

    // TODO: I would prefer dispatch to be a member fn, not static. But State owns
    // the dispatcher, and as such can't be a message handler too since
    pub fn dispatch(messages: &Vec<Message>, handler: &mut dyn MessageHandler) {
        for m in messages {
            handler.handle_message(m);
        }
    }

    /// Returns the current message buffer, and clears it.
    pub fn drain(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.messages)
    }
}
