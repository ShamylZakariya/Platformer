use std::time::Duration;

use cgmath::*;

use crate::{
    constants::sprite_masks::COLLIDER,
    entity::{Dispatcher, Entity, Message},
    map,
    sprite::{self, collision, rendering},
    tileset,
};

// --------------------------------------------------------------------------------------------------------------------

const ANIMATION_CYCLE_DURATION: f32 = 0.133;
const MOVEMENT_SPEED: f32 = 0.5; // units per second

#[derive(Debug)]
enum MovementDir {
    East,
    West,
}

impl MovementDir {
    fn invert(&self) -> MovementDir {
        match self {
            MovementDir::East => MovementDir::West,
            MovementDir::West => MovementDir::East,
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub struct FireSprite {
    entity_id: u32,
    sprite: Option<sprite::Sprite>,
    position: Point3<f32>,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
    current_movement: MovementDir,
}

impl Default for FireSprite {
    fn default() -> Self {
        Self {
            entity_id: 0,
            sprite: None,
            position: point3(0.0, 0.0, 0.0),
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
            current_movement: MovementDir::East,
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
        _collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.sprite = Some(*sprite);
        self.position = sprite.origin;
    }

    fn update(
        &mut self,
        dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        _message_dispatcher: &mut Dispatcher,
    ) {
        let dt = dt.as_secs_f32();

        //
        //  Update position - firesprite simply marches left/right stopping at "cliffs" or obstacles
        //

        let next_position = self.position.xy()
            + match self.current_movement {
                MovementDir::East => vec2(1.0, 0.0),
                MovementDir::West => vec2(-1.0, 0.0),
            } * dt;
        let snapped_next_position = point2(
            next_position.x.floor() as i32,
            next_position.y.floor() as i32,
        );
        let mut should_reverse_direction = false;

        match self.current_movement {
            MovementDir::East => {
                // check for obstacle to right
                if let Some(sprite_to_right) =
                    collision_space.get_sprite_at(snapped_next_position + vec2(1, 0), COLLIDER)
                {
                    if sprite_to_right.rect_intersection(&next_position, &vec2(1.0, 1.0), 0.0, true)
                    {
                        should_reverse_direction = true
                    }
                }
                // check if the platform falls away to right
                if collision_space
                    .get_sprite_at(snapped_next_position + vec2(1, -1), COLLIDER)
                    .is_none()
                {
                    should_reverse_direction = true
                }
            }
            MovementDir::West => {
                // check for obstacle to left
                if let Some(sprite_to_left) =
                    collision_space.get_sprite_at(snapped_next_position, COLLIDER)
                {
                    if sprite_to_left.rect_intersection(&next_position, &vec2(1.0, 1.0), 0.0, true)
                    {
                        should_reverse_direction = true
                    }
                }
                // check if the platform falls away to left
                if collision_space
                    .get_sprite_at(snapped_next_position + vec2(0, -1), COLLIDER)
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
        true
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

    fn handle_message(&mut self, _message: &Message) {}
}
