use std::{collections::HashSet, time::Duration};

use cgmath::{Point2, Point3, Vector2};
use winit::event::{ElementState, VirtualKeyCode};

use crate::{
    entity::{Dispatcher, Entity, Message},
    map,
    sprite::{self, collision, rendering},
    tileset,
};

pub struct Fireball {
    entity_id: u32,
    position: Point3<f32>,
    velocity: Vector2<f32>,
    alive: bool,
    map_origin: Point2<f32>,
    map_extent: Vector2<f32>,
}

impl Fireball {
    pub fn new(position: cgmath::Point3<f32>, velocity: cgmath::Vector2<f32>) -> Self {
        Self {
            entity_id: 0,
            position,
            velocity,
            alive: true,
            map_origin: Point2::new(0.0, 0.0),
            map_extent: Vector2::new(0.0, 0.0),
        }
    }
}

impl Entity for Fireball {
    fn init_from_map_sprite(
        &mut self,
        _sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        _map: &map::Map,
        _collision_space: &mut collision::Space,
    ) {
        panic!("Fireball must be initialized via init() and not init_from_map_sprite()")
    }

    fn init(&mut self, entity_id: u32, map: &map::Map, _collision_space: &mut collision::Space) {
        self.entity_id = entity_id;
        let bounds = map.bounds();
        self.map_origin = bounds.0.cast().unwrap();
        self.map_extent = bounds.1.cast().unwrap();
    }

    fn process_keyboard(&mut self, _key: VirtualKeyCode, _state: ElementState) -> bool {
        // Fireball doesn't consume input
        false
    }

    fn update(
        &mut self,
        dt: Duration,
        _collision_space: &mut collision::Space,
        _message_dispatcher: &mut Dispatcher,
    ) {
        let dt = dt.as_secs_f32();
        self.position.x = self.position.x + self.velocity.x * dt;
        self.position.y = self.position.y + self.velocity.y * dt;
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        uniforms.data.set_model_position(&self.position);
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn is_alive(&self) -> bool {
        self.alive
    }

    fn should_draw(&self) -> bool {
        true
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "fireball"
    }

    fn sprite_cycle(&self) -> &str {
        "default"
    }

    fn handle_message(&mut self, _message: &Message) {}

    fn overlapping_sprites(&self) -> Option<&HashSet<sprite::Sprite>> {
        None
    }
    fn contacting_sprites(&self) -> Option<&HashSet<sprite::Sprite>> {
        None
    }
}
