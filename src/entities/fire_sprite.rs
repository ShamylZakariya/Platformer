use cgmath::*;
use std::time::Duration;
use winit::event::{ElementState, VirtualKeyCode};

use crate::{
    entity::Entity,
    event_dispatch::*,
    map,
    sprite::{self, collision, rendering},
    state::constants::sprite_masks::{self, COLLIDER},
    tileset,
};

use super::util::{Direction, HitPointState};

// --------------------------------------------------------------------------------------------------------------------

const ANIMATION_CYCLE_DURATION: f32 = 0.133;
const MOVEMENT_SPEED: f32 = 0.5; // units per second
const HIT_POINTS: i32 = 2;

// --------------------------------------------------------------------------------------------------------------------

pub struct FireSprite {
    entity_id: u32,
    spawn_point_id: u32,
    sprite: sprite::Sprite,
    position: Point3<f32>,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
    current_movement: Direction,
    life: HitPointState,
}

impl Default for FireSprite {
    fn default() -> Self {
        Self {
            entity_id: 0,
            spawn_point_id: 0,
            sprite: sprite::Sprite::default(),
            position: point3(0.0, 0.0, 0.0),
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
            current_movement: Direction::East,
            life: HitPointState::new(HIT_POINTS),
        }
    }
}

impl Entity for FireSprite {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        _map: &map::Map,
        collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.spawn_point_id = sprite
            .entity_id
            .expect("Spawned entities expect to find a spawn point id from the sprite");

        self.position = sprite.origin;

        // Make copy of sprite for ourselves, we'll use it for collision testing
        // Note: The map sprite is our spawn point, so we need to overwrite the entity_id and mask
        self.sprite = *sprite;
        self.sprite.entity_id = Some(entity_id);
        self.sprite.mask =
            sprite_masks::SHOOTABLE | sprite_masks::COLLIDER | sprite_masks::CONTACT_DAMAGE;
        self.sprite.collision_shape = sprite::CollisionShape::Square;
        collision_space.add_dynamic_sprite(&self.sprite);
    }

    fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        if self.life.is_alive() {
            if key == VirtualKeyCode::Delete && state == ElementState::Pressed {
                println!("BOOM");
                self.life.injure(self.life.hit_points(), Direction::East);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        message_dispatcher: &mut Dispatcher,
    ) {
        let dt = dt.as_secs_f32();

        if !self.life.update(
            self.entity_id(),
            self.spawn_point_id,
            self.position(),
            collision_space,
            message_dispatcher,
        ) {
            return;
        }

        //
        //  Update position - firesprite simply marches left/right stopping at "cliffs" or obstacles
        //

        let next_position = self.position.xy()
            + match self.current_movement {
                Direction::East => vec2(1.0, 0.0),
                Direction::West => vec2(-1.0, 0.0),
            } * dt;
        let snapped_next_position = point2(
            next_position.x.floor() as i32,
            next_position.y.floor() as i32,
        );
        let snapped_next_position_center = point2(
            (next_position.x + 0.5).floor() as i32,
            next_position.y.floor() as i32,
        );
        let mut should_reverse_direction = false;

        match self.current_movement {
            Direction::East => {
                // check for obstacle to right
                if let Some(sprite_to_right) = collision_space
                    .get_static_sprite_at(snapped_next_position + vec2(1, 0), COLLIDER)
                {
                    if sprite_to_right.rect_intersection(&next_position, &vec2(1.0, 1.0), 0.0, true)
                    {
                        should_reverse_direction = true
                    }
                }
                // check if the platform falls away to right
                if collision_space
                    .get_static_sprite_at(snapped_next_position_center + vec2(0, -1), COLLIDER)
                    .is_none()
                {
                    should_reverse_direction = true
                }
            }
            Direction::West => {
                // check for obstacle to left
                if let Some(sprite_to_left) =
                    collision_space.get_static_sprite_at(snapped_next_position, COLLIDER)
                {
                    if sprite_to_left.rect_intersection(&next_position, &vec2(1.0, 1.0), 0.0, true)
                    {
                        should_reverse_direction = true
                    }
                }
                // check if the platform falls away to left
                if collision_space
                    .get_static_sprite_at(snapped_next_position_center + vec2(0, -1), COLLIDER)
                    .is_none()
                {
                    should_reverse_direction = true
                }
            }
        }

        if should_reverse_direction {
            self.current_movement = self.current_movement.invert();
        } else {
            self.position.x = next_position.x;
            self.position.y = next_position.y;
        }

        //
        //  Update the sprite for collision detection
        //

        self.sprite.origin.x = self.position.x;
        self.sprite.origin.y = self.position.y;
        collision_space.update_dynamic_sprite(&self.sprite);

        //
        //  Update sprite animation cycle
        //

        self.animation_cycle_tick_countdown -= dt;
        if self.animation_cycle_tick_countdown <= 0.0 {
            self.animation_cycle_tick_countdown += ANIMATION_CYCLE_DURATION;
            self.animation_cycle_tick += 1;
        }
    }

    fn update_uniforms(&self, uniforms: &mut rendering::Uniforms) {
        let (xscale, xoffset) = if self.animation_cycle_tick / 2 % 2 == 0 {
            (1.0, 0.0)
        } else {
            (-1.0, 1.0)
        };
        uniforms
            .data
            .set_model_position(self.position + vec3(xoffset, 0.0, 0.0))
            .set_sprite_scale(vec2(xscale, 1.0));
    }

    fn entity_id(&self) -> u32 {
        self.entity_id
    }

    fn entity_class(&self) -> crate::entities::EntityClass {
        crate::entities::EntityClass::FireSprite
    }

    fn is_alive(&self) -> bool {
        self.life.is_alive()
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "fire_sprite"
    }

    fn sprite_cycle(&self) -> &str {
        if self.animation_cycle_tick % 2 == 0 {
            "default"
        } else {
            "alt"
        }
    }

    fn handle_message(&mut self, message: &Message) {
        self.life.handle_message(message);
    }
}
