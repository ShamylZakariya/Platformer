use cgmath::*;
use std::time::Duration;

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, collision, rendering},
    state::constants::layers,
    tileset,
};

// ---------------------------------------------------------------------------------------------------------------------

const FLIGHT_BAR_SCALE: f32 = 2.0;

// ---------------------------------------------------------------------------------------------------------------------

pub struct UiFlightBar {
    entity_id: u32,
    position: Point3<f32>,
    width_scale_max: f32,
    width_scale_current: f32,
}

impl Default for UiFlightBar {
    fn default() -> Self {
        Self {
            entity_id: 0,
            position: point3(0.0, 0.0, 0.0),
            width_scale_max: 1.0,
            width_scale_current: 1.0,
        }
    }
}

impl Entity for UiFlightBar {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.position = point3(sprite.origin.x, sprite.origin.y, layers::ui::FOREGROUND);
    }

    fn update(
        &mut self,
        _dt: Duration,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
        _message_dispatcher: &mut Dispatcher,
        game_state_peek: &GameStatePeek,
    ) {
        self.width_scale_max = game_state_peek.player_flight.1 * FLIGHT_BAR_SCALE;
        self.width_scale_current =
            FLIGHT_BAR_SCALE * (game_state_peek.player_flight.0 / game_state_peek.player_flight.1);
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        uniforms
            .data
            .set_model_position(self.position)
            .set_sprite_scale(vec2(self.width_scale_current, 1.0));
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::UiFlightBar
    }

    fn is_alive(&self) -> bool {
        true
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "flight_bar"
    }

    fn sprite_cycle(&self) -> &str {
        "default"
    }
}
