use std::time::Duration;

use cgmath::*;
use sprite::find_bounds;

use crate::{
    audio, collision,
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, rendering},
    state::{
        constants::{layers, sprite_masks},
        events::Event,
    },
    util::{self, Bounds},
};

const RISE_SPEED: f32 = 1.0 / 0.467;

pub struct RisingFloor {
    entity_id: u32,
    collider_id: Option<u32>,
    offset: Point3<f32>,
    stage_sprites: Vec<sprite::Sprite>,
    bounds: Bounds,
    rising: bool,
    sent_started_rising_message: bool,
    pixels_per_unit: f32,
}

impl RisingFloor {
    pub fn new(stage_sprites: Vec<sprite::Sprite>) -> Self {
        let bounds = find_bounds(&stage_sprites);
        Self {
            entity_id: 0,
            collider_id: None,
            offset: point3(0.0, 0.0, layers::stage::FOREGROUND - 1.0),
            stage_sprites,
            bounds,
            rising: false,
            sent_started_rising_message: false,
            pixels_per_unit: 0.0,
        }
    }
}

impl Entity for RisingFloor {
    fn init(&mut self, entity_id: u32, map: &map::Map, collision_space: &mut collision::Space) {
        self.entity_id = entity_id;
        self.offset.y = -self.bounds.extent.y;

        self.collider_id = Some(
            collision_space.add_collider(collision::Collider::new_dynamic(
                Bounds::new(self.collider_position(), self.bounds.extent),
                entity_id,
                collision::Shape::Square,
                sprite_masks::GROUND,
            )),
        );

        self.pixels_per_unit = map.tileset.get_sprite_size().x as f32;
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        _audio: &mut audio::Audio,
        message_dispatcher: &mut Dispatcher,
        _game_state_peek: &GameStatePeek,
    ) {
        if self.rising {
            if !self.sent_started_rising_message {
                self.sent_started_rising_message = true;
                message_dispatcher.broadcast(Event::StartCameraShake {
                    pattern: self.camera_shake_pattern(),
                });
            }

            self.offset.y += RISE_SPEED * dt.as_secs_f32();
            if self.offset.y >= 0.0 {
                self.offset.y = 0.0;
                self.rising = false;

                message_dispatcher.broadcast(Event::EndCameraShake);
                message_dispatcher.broadcast(Event::OpenExitDoor);
            }
        }

        if let Some(id) = self.collider_id {
            collision_space.update_collider_position(id, self.collider_position());
        }
    }

    fn update_uniforms(&self, uniforms: &mut util::UniformWrapper<rendering::Uniforms>) {
        uniforms.data.set_model_position(self.offset);
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
        crate::entities::EntityClass::RisingFloor
    }

    fn is_alive(&self) -> bool {
        true
    }

    fn position(&self) -> Point3<f32> {
        self.offset
    }

    fn stage_sprites(&self) -> Option<Vec<sprite::Sprite>> {
        Some(self.stage_sprites.clone())
    }

    fn handle_message(&mut self, message: &Message) {
        match message.event {
            Event::RaiseExitFloor => self.rising = true,
            Event::ResetState => {
                self.rising = false;
                self.offset.y = -self.bounds.extent.y;
                self.sent_started_rising_message = false;
            }
            _ => {}
        }
    }
}

impl RisingFloor {
    fn collider_position(&self) -> Point2<f32> {
        point2(
            self.bounds.origin.x + self.offset.x,
            self.bounds.origin.y + self.offset.y,
        )
    }

    fn camera_shake_pattern(&self) -> Vec<(Vector2<f32>, f32)> {
        let d = 1.5 / 30.0_f32;
        let p = 1.0 / self.pixels_per_unit;
        vec![
            (vec2(-4.0 * p, 0.0), d),
            (vec2(-2.0 * p, 0.0), d),
            (vec2(0.0, 0.0), d),
            (vec2(1.0 * p, 0.0), d),
            (vec2(0.0, 0.0), d),
            (vec2(0.0, 0.0), d),
            (vec2(4.0 * p, 0.0), d),
            (vec2(2.0 * p, 0.0), d),
            (vec2(0.0, 0.0), d),
            (vec2(-1.0 * p, 0.0), d),
            (vec2(0.0, 0.0), d),
            (vec2(0.0, 0.0), d),
        ]
    }
}
