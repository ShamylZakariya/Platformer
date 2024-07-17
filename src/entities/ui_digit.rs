use cgmath::*;
use core::panic;
use std::time::Duration;

use crate::{
    audio, collision,
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, rendering},
    state::constants::layers,
    tileset,
};

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Tracking {
    Vials,
    Lives,
}

// ---------------------------------------------------------------------------------------------------------------------

pub struct UiDigit {
    entity_id: u32,
    position: Point3<f32>,
    tracking: Option<Tracking>,
    digit: u32,
    cycle: u32,
}

impl Default for UiDigit {
    fn default() -> Self {
        Self {
            entity_id: 0,
            position: point3(0.0, 0.0, 0.0),
            tracking: None,
            digit: 0,
            cycle: 0,
        }
    }
}

impl Entity for UiDigit {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        map: &map::Map,
        _collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.position = point3(sprite.origin.x, sprite.origin.y, layers::ui::FOREGROUND);

        if let Some(obj) = map.object_group_properties_for_sprite(sprite, "Metadata") {
            for property in obj.properties.iter() {
                match property.name.as_str() {
                    "tracking" => match property.value.as_str() {
                        "vials" => self.tracking = Some(Tracking::Vials),
                        "lives" => self.tracking = Some(Tracking::Lives),
                        _ => panic!("Only 'vials' and 'lives' supported for UiDigit"),
                    },
                    "digit" => {
                        self.digit = property
                            .value
                            .parse()
                            .expect("Expected to parse 'digit' property to i32")
                    }
                    _ => {}
                }
            }

            self.tracking
                .expect("Expect UiDigit to have loaded 'tracking' target from metadata");
        } else {
            log::error!("Could not find metadata for sprite");
        }
    }

    fn update(
        &mut self,
        _dt: Duration,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
        _audio: &mut audio::Audio,
        _message_dispatcher: &mut Dispatcher,
        game_state_peek: &GameStatePeek,
    ) {
        let value = match self.tracking {
            Some(Tracking::Vials) => game_state_peek.player_vials,
            Some(Tracking::Lives) => game_state_peek.player_lives,
            _ => 0,
        } as i32;

        let value = value / (10_i32).pow(self.digit);
        self.cycle = (value % 10) as u32;
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        uniforms.data.set_model_position(self.position);
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::UiDigit
    }

    fn is_alive(&self) -> bool {
        true
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "numeral"
    }

    fn sprite_cycle(&self) -> &str {
        match self.cycle {
            0 => "0",
            1 => "1",
            2 => "2",
            3 => "3",
            4 => "4",
            5 => "5",
            6 => "6",
            7 => "7",
            8 => "8",
            9 => "9",
            _ => panic!("Expected cycle to be in range [0,9]"),
        }
    }
}
