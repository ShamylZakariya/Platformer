use std::time::Duration;

use cgmath::*;

use crate::{
    collision,
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, rendering},
    state::{
        constants::{layers, sprite_masks},
        events::Event,
    },
    tileset, util,
};

// --------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub enum Type {
    Vial,
    Heart,
}

impl Type {
    fn from_str(t: &str) -> Option<Type> {
        match t {
            "vial" => Some(Type::Vial),
            "heart" => Some(Type::Heart),
            _ => None,
        }
    }

    fn sprite_name(&self) -> &'static str {
        match self {
            Type::Vial => "vial",
            Type::Heart => "heart",
        }
    }
}

const FLICKER_PERIOD: f32 = 0.133 * 2.0;

// --------------------------------------------------------------------------------------------------------------------

pub struct PowerUp {
    entity_id: u32,
    collider_id: Option<u32>,
    position: Point3<f32>,
    powerup_type: Option<Type>,
    time: f32,
    needs_collider: bool,
    is_collider_active: bool,
}

impl Default for PowerUp {
    fn default() -> Self {
        Self {
            entity_id: 0,
            collider_id: None,
            position: point3(0.0, 0.0, 0.0),
            powerup_type: None,
            time: 0.0,
            needs_collider: true,
            is_collider_active: false,
        }
    }
}

impl Entity for PowerUp {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        tile: &tileset::Tile,
        _map: &map::Map,
        collision_space: &mut collision::Space,
    ) {
        self.is_collider_active = true;
        self.entity_id = entity_id;
        self.position = point3(sprite.origin.x, sprite.origin.y, layers::stage::ENTITIES);

        // Make collider

        self.collider_id = Some(
            collision_space.add_collider(collision::Collider::new_dynamic(
                sprite.bounds(),
                entity_id,
                collision::Shape::Square,
                sprite_masks::ENTITY,
            )),
        );

        let type_name = tile
            .get_property("powerup_type")
            .expect("PowerUp tile must specify 'powerup_type'");

        self.powerup_type = Some(Type::from_str(type_name).expect("Expect supported powerup type"));
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
        _game_state_peek: &GameStatePeek,
    ) {
        let dt = dt.as_secs_f32();
        self.time += dt;

        if !self.needs_collider && self.is_collider_active {
            if let Some(id) = self.collider_id {
                collision_space.deactivate_collider(id);
            }
            self.is_collider_active = false;

            // broadcast that this powerup has been consumed
            message_dispatcher.broadcast(Event::FirebrandContactedPowerUp {
                powerup_type: self.powerup_type.unwrap(),
            });
        } else if self.needs_collider && !self.is_collider_active {
            if let Some(id) = self.collider_id {
                collision_space.activate_collider(id);
            }
            self.is_collider_active = true;
        }
    }

    fn update_uniforms(&self, uniforms: &mut util::Uniforms<rendering::UniformData>) {
        let cycle = ((self.time / FLICKER_PERIOD).round() as i32) % 2;
        let alpha = if cycle == 0 { 1.0 } else { 0.5 };
        uniforms
            .data
            .set_color(vec4(1.0, 1.0, 1.0, alpha))
            .set_model_position(self.position);
    }

    fn deactivate_collider(&mut self, collision_space: &mut collision::Space) {
        if let Some(id) = self.collider_id {
            collision_space.deactivate_collider(id);
        }
        self.collider_id = None;
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::PowerUp
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        self.powerup_type.unwrap().sprite_name()
    }

    fn sprite_cycle(&self) -> &str {
        "default"
    }

    fn should_draw(&self) -> bool {
        self.needs_collider
    }

    fn handle_message(&mut self, message: &Message) {
        match message.event {
            Event::FirebrandContact => {
                self.needs_collider = false;
            }
            Event::ResetState => {
                self.needs_collider = true;
            }
            _ => {}
        }
    }
}
