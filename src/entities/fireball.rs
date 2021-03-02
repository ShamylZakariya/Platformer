use std::time::Duration;

use cgmath::*;

use crate::{
    collision,
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::rendering,
    state::{constants::layers, events::Event},
    util::Bounds,
};

use super::util::HorizontalDir;

// ---------------------------------------------------------------------------------------------------------------------

const FIREBALL_DIAMETER: f32 = 0.25;
const ANIMATION_CYCLE_DURATION: f32 = 0.133;
const CYCLE_DEFAULT: &str = "default";

enum Mode {
    Fireball,
    Firesprite,
}

// ---------------------------------------------------------------------------------------------------------------------

pub struct Fireball {
    sender_id: u32,
    entity_id: u32,
    position: Point3<f32>,
    velocity: Vector2<f32>,
    alive: bool,
    map_origin: Point2<f32>,
    map_extent: Vector2<f32>,
    mode: Mode,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
    damage: u32,
}

impl Fireball {
    pub fn new_fireball(
        sender_id: u32,
        position: Point2<f32>,
        direction: HorizontalDir,
        velocity: f32,
        damage: u32,
    ) -> Self {
        let dv: Vector2<f32> = direction.into();
        Self {
            sender_id,
            entity_id: 0,
            position: point3(position.x, position.y, layers::stage::FIREBRAND + 1.0),
            velocity: dv * velocity,
            alive: true,
            map_origin: point2(0.0, 0.0),
            map_extent: vec2(0.0, 0.0),
            mode: Mode::Fireball,
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
            damage,
        }
    }
    pub fn new_firesprite(
        sender_id: u32,
        position: Point2<f32>,
        direction: Vector2<f32>,
        velocity: f32,
        damage: u32,
    ) -> Self {
        Self {
            sender_id,
            entity_id: 0,
            position: point3(position.x, position.y, layers::stage::FIREBRAND + 1.0),
            velocity: direction * velocity,
            alive: true,
            map_origin: point2(0.0, 0.0),
            map_extent: vec2(0.0, 0.0),
            mode: Mode::Firesprite,
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
            damage,
        }
    }
}

impl Entity for Fireball {
    fn init(&mut self, entity_id: u32, map: &map::Map, _collision_space: &mut collision::Space) {
        self.entity_id = entity_id;
        let bounds = map.bounds();
        self.map_origin = bounds.origin.cast().unwrap();
        self.map_extent = bounds.extent.cast().unwrap();
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
        let mask = crate::state::constants::sprite_masks::SHOOTABLE;

        let next_position = self.position.xy() + self.velocity * dt;
        let collider_origin = point2(
            next_position.x - FIREBALL_DIAMETER / 2.0,
            next_position.y - FIREBALL_DIAMETER / 2.0,
        );
        let collider_extent = vec2(FIREBALL_DIAMETER, FIREBALL_DIAMETER);
        if let Some(sprite) =
            collision_space.test_rect_first(&collider_origin, &collider_extent, mask)
        {
            if let Some(target_entity_id) = sprite.entity_id {
                if target_entity_id != self.sender_id {
                    //
                    // hit an entity that's not the sender
                    //

                    message_dispatcher.entity_to_entity(
                        self.entity_id(),
                        target_entity_id,
                        Event::HitByFireball {
                            direction: self.velocity.into(),
                            damage: self.damage,
                        },
                    );
                    self.alive = false;
                }
            } else {
                //
                // hit static level geometry
                //
                self.alive = false;
            }
        }

        self.position.x = next_position.x;
        self.position.y = next_position.y;

        //  Update sprite animation cycle for Firesprite
        if matches!(self.mode, Mode::Firesprite) {
            self.animation_cycle_tick_countdown -= dt;
            if self.animation_cycle_tick_countdown <= 0.0 {
                self.animation_cycle_tick_countdown += ANIMATION_CYCLE_DURATION;
                self.animation_cycle_tick += 1;
            }
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        uniforms
            .data
            .set_model_position(self.position - vec3(0.5, 0.5, 0.0));
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::Fireball
    }

    fn is_alive(&self) -> bool {
        self.alive
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn bounds(&self) -> Bounds {
        Bounds::new(self.position().xy() - vec2(0.5, 0.5), vec2(1.0, 1.0))
    }

    fn sprite_name(&self) -> &str {
        match self.mode {
            Mode::Fireball => "fireball",
            Mode::Firesprite => "fire_sprite",
        }
    }

    fn sprite_cycle(&self) -> &str {
        match self.mode {
            Mode::Fireball => "default",
            Mode::Firesprite => {
                if self.animation_cycle_tick % 2 == 0 {
                    "default"
                } else {
                    "alt"
                }
            }
        }
    }

    fn did_exit_viewport(&mut self) {
        self.alive = false;
    }
}
