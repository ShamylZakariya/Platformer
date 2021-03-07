use std::{collections::HashSet, fmt::Debug, time::Duration};

use cgmath::*;
use winit::event::{ElementState, VirtualKeyCode};

use crate::{
    collision,
    event_dispatch::*,
    map,
    sprite::{self, rendering},
    tileset,
    util::{self, Bounds},
};

// ---------------------------------------------------------------------------------------------------------------------

/// IDVendor vends a new unique id, starting from zero, for each entity.
pub struct IdVendor {
    current_id: u32,
}

impl Default for IdVendor {
    fn default() -> Self {
        IdVendor {
            current_id: 1000u32,
        }
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

/// Entities don't have direct access to game state; but a read-only peek is useful so that
/// entities may "chase" the player, etc. GameStatePeek is a holder for this information. An proper engine
/// might store a snapshot of state for each entity in the level, but all we need right now is to know
/// firebrand's position.
#[derive(Clone, Copy, Debug)]
pub struct GameStatePeek {
    pub player_position: Point2<f32>,
    pub player_health: (u32, u32), // current health points, max health points
    pub player_flight: (f32, f32), // current flight time remaining, max flight time
    pub player_vials: u32,         // number of vials the player has
    pub player_lives: u32,         // number of lives remaining
    pub current_map_bounds: Bounds,
}

impl Default for GameStatePeek {
    fn default() -> Self {
        Self {
            player_position: point2(0.0, 0.0),
            player_health: (0, 0),
            player_flight: (0.0, 0.0),
            player_vials: 0,
            player_lives: 0,
            current_map_bounds: Bounds::default(),
        }
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
        _entity_id: u32,
        _sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
    ) {
    }

    /// Initializes an entity whcih is not loaded from the level map. This is generally for dynamic
    /// entity creation not based on map tiles, such as fireballs, etc.
    fn init(&mut self, _entity_id: u32, _map: &map::Map, _collision_space: &mut collision::Space) {}

    /// Handle keyboard input, returning true iff said input was consumed.
    fn process_keyboard(&mut self, _key: VirtualKeyCode, _state: ElementState) -> bool {
        false
    }

    /// Handle gamepad input events. This is invoked on each entity when the
    /// gamepad input state changes.
    fn process_gamepad(&mut self, _event: gilrs::Event) {}

    /// Update internal state of entity.
    /// # Arguments
    /// * `dt` delta time since last update
    /// * `collision_space` the shared collision space for collision lookup. A moving sprite should update its position in the collision space
    /// * `message_dispatcher` the dispatcher for queing messages to be processed by entities at end of update loop.
    fn update(
        &mut self,
        _dt: Duration,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
        _message_dispatcher: &mut Dispatcher,
        _game_state_peek: &GameStatePeek,
    ) {
    }

    /// Write updated state into this entity's uniform buffer for rendering.
    fn update_uniforms(&self, _uniforms: &mut util::UniformWrapper<rendering::Uniforms>) {}

    /// Called on an entity before removing it from GameState
    fn deactivate_collider(&mut self, _collision_space: &mut collision::Space) {}

    /// The unique id for this Entity, a value from [0,u32::MAX]
    fn entity_id(&self) -> u32;

    /// The class represented by this Entity
    fn entity_class(&self) -> crate::entities::EntityClass;

    /// An entity should return true here so long as it needs to be updated and drawn.
    fn is_alive(&self) -> bool {
        true
    }

    /// Return true if you want this entity to draw right now. Entity will still be updated.
    fn should_draw(&self) -> bool {
        true
    }

    /// The current position of the entity
    fn position(&self) -> Point3<f32>;

    /// The bounds of the entity, expressed as (origin, extent)
    /// Note: An entity's bounds origin are not necessarily same as the entity's position.
    /// Default implementation simply returns a unit box with lower-left at position().
    fn bounds(&self) -> Bounds {
        Bounds::new(self.position().xy(), vec2(1.0, 1.0))
    }

    /// The name identifying the entity's sprites in the Entity spritesheet. E.g., "firebrand" or "falling_bridge".
    /// If returning an empty string, no EntityDrawable will be created.
    fn sprite_name(&self) -> &str {
        ""
    }

    /// The current sprite "cycle", e.g., "walk_0", "default", etc.
    fn sprite_cycle(&self) -> &str {
        ""
    }

    /// Handle receipt of a dispatched message.
    fn handle_message(&mut self, _message: &Message) {}

    /// Return a set of overlapping sprites used for debug visualization, or None if not applicable.
    fn overlapping_sprites(&self) -> Option<&HashSet<sprite::Sprite>> {
        None
    }

    /// Return a set of contacting sprites used for debug visualization, or None if not applicable.
    fn contacting_sprites(&self) -> Option<&HashSet<sprite::Sprite>> {
        None
    }

    /// Called when the entity entered the viewport
    fn did_enter_viewport(&mut self) {}

    /// Called when the entity entered the viewport
    fn did_exit_viewport(&mut self) {}

    /// Most entities are rendered via an EntityDrawable, but some (RisingFloor, ExitDoor) draw stage sprites,
    /// and use a sprite::Drawable. To do so, return a vector of Sprite here and an empty string from sprite_name.
    fn stage_sprites(&self) -> Option<Vec<sprite::Sprite>> {
        None
    }
}

// ---------------------------------------------------------------------------------------------------------------------

/// EntityComponents represent a unit that can own an Entity and its sprite and uniforms, suitable
/// for updating state, and drawing.
pub struct EntityComponents {
    pub entity: Box<dyn Entity>,
    pub entity_drawable: Option<crate::sprite::rendering::EntityDrawable>,
    pub sprite_drawable: Option<crate::sprite::rendering::Drawable>,
    pub uniforms: Option<util::UniformWrapper<rendering::Uniforms>>,
}

impl EntityComponents {
    pub fn with_entity_drawable(
        entity: Box<dyn Entity>,
        entity_drawable: crate::sprite::rendering::EntityDrawable,
        uniforms: util::UniformWrapper<rendering::Uniforms>,
    ) -> Self {
        Self {
            entity,
            entity_drawable: Some(entity_drawable),
            sprite_drawable: None,
            uniforms: Some(uniforms),
        }
    }

    pub fn with_sprite_drawable(
        entity: Box<dyn Entity>,
        sprite_drawable: crate::sprite::rendering::Drawable,
        uniforms: util::UniformWrapper<rendering::Uniforms>,
    ) -> Self {
        Self {
            entity,
            entity_drawable: None,
            sprite_drawable: Some(sprite_drawable),
            uniforms: Some(uniforms),
        }
    }

    pub fn just_entity(entity: Box<dyn Entity>) -> Self {
        Self {
            entity,
            entity_drawable: None,
            sprite_drawable: None,
            uniforms: None,
        }
    }

    pub fn id(&self) -> u32 {
        self.entity.entity_id()
    }

    pub fn class(&self) -> crate::entities::EntityClass {
        self.entity.entity_class()
    }
}

impl Debug for EntityComponents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[EntityComponents id: {} class: {:?} position: {:?}]",
            self.id(),
            self.class(),
            self.entity.position()
        )
    }
}
