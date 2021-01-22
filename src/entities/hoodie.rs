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
const HIT_POINTS: i32 = 1;

// --------------------------------------------------------------------------------------------------------------------

pub struct Hoodie {
    entity_id: u32,
    spawn_point_id: u32,
    sprite: sprite::Sprite,
    sprite_size_px: Vector2<f32>,
    position: Point3<f32>,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
    current_movement: Direction,
    life: HitPointState,
}

impl Default for Hoodie {
    fn default() -> Self {
        Self {
            entity_id: 0,
            spawn_point_id: 0,
            sprite: sprite::Sprite::default(),
            sprite_size_px: vec2(0.0, 0.0),
            position: point3(0.0, 0.0, 0.0),
            animation_cycle_tick_countdown: ANIMATION_CYCLE_DURATION,
            animation_cycle_tick: 0,
            current_movement: Direction::East,
            life: HitPointState::new(HIT_POINTS),
        }
    }
}

impl Entity for Hoodie {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        map: &map::Map,
        collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.spawn_point_id = sprite
            .entity_id
            .expect("Spawned entities expect to find a spawn point id from the sprite");

        self.position = sprite.origin;
        self.sprite_size_px = map.tileset.get_sprite_size().cast().unwrap();

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

        let dt = dt.as_secs_f32();
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
        let one_px = 1.0 / self.sprite_size_px.x;

        let (xscale, xoffset) = match self.current_movement {
            Direction::East => (1.0, 4.0 * one_px),
            Direction::West => (-1.0, 1.0 - 4.0 * one_px),
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
        crate::entities::EntityClass::Hoodie
    }

    fn is_alive(&self) -> bool {
        self.life.is_alive()
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "hoodie"
    }

    fn sprite_cycle(&self) -> &str {
        match self.animation_cycle_tick % 4 {
            0 => "walk_0",
            1 => "walk_1",
            2 => "walk_0",
            _ => "walk_2",
        }
    }

    fn handle_message(&mut self, message: &Message) {
        self.life.handle_message(message);
    }
}
