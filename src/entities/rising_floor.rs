use std::time::Duration;

use cgmath::*;
use sprite::bounds;

use crate::{
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, collision, rendering},
    state::events::Event,
};

const RISE_SPEED: f32 = 1.0;

pub struct RisingFloor {
    entity_id: u32,
    offset: Point3<f32>,
    stage_sprites: Vec<sprite::Sprite>,
    bounds: (Point2<f32>, Vector2<f32>),
    rising: bool,
    collider: sprite::Sprite,
}

impl RisingFloor {
    pub fn new(stage_sprites: Vec<sprite::Sprite>) -> Self {
        let bounds = bounds(&stage_sprites);
        let mut collider = sprite::Sprite::default();
        collider.collision_shape = sprite::CollisionShape::Square;
        collider.origin = point3(bounds.0.x, bounds.0.y, 0.0);
        collider.extent = bounds.1;
        collider.mask = crate::state::constants::sprite_masks::COLLIDER;

        Self {
            entity_id: 0,
            offset: point3(0.0, 0.0, -0.05),
            stage_sprites,
            bounds,
            rising: false,
            collider,
        }
    }
}

impl Entity for RisingFloor {
    fn init(&mut self, entity_id: u32, _map: &map::Map, collision_space: &mut collision::Space) {
        self.entity_id = entity_id;
        self.offset.y -= self.bounds.1.y;
        self.collider.entity_id = Some(entity_id);

        self.update_collider();
        collision_space.add_dynamic_sprite(&self.collider);
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
        _game_state_peek: &GameStatePeek,
    ) {
        if self.rising {
            self.offset.y += RISE_SPEED * dt.as_secs_f32();
            if self.offset.y >= 0.0 {
                self.offset.y = 0.0;
                self.rising = false;

                message_dispatcher.broadcast(Event::OpenExitDoor);
            }
            self.update_collider();
            collision_space.update_dynamic_sprite(&self.collider);
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        uniforms.data.set_model_position(self.offset);
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
        if let Event::RaiseExitFloor = message.event {
            self.rising = true;
        }
    }
}

impl RisingFloor {
    fn update_collider(&mut self) {
        self.collider.origin.x = self.bounds.0.x + self.offset.x;
        self.collider.origin.y = self.bounds.0.y + self.offset.y;
    }
}
