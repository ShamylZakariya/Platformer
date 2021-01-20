use cgmath::*;
use std::time::Duration;
use winit::event::{ElementState, VirtualKeyCode};

use crate::{
    entity::Entity,
    event_dispatch::*,
    map,
    sprite::{self, collision, rendering},
    state::{
        constants::sprite_masks::{self, COLLIDER},
        events::Event,
    },
    tileset,
};

// --------------------------------------------------------------------------------------------------------------------

const ANIMATION_CYCLE_DURATION: f32 = 0.133;
const MOVEMENT_SPEED: f32 = 0.5; // units per second
const HIT_POINTS: i32 = 2;

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
    spawn_point_id: u32,
    sprite: sprite::Sprite,
    position: Point3<f32>,
    animation_cycle_tick_countdown: f32,
    animation_cycle_tick: u32,
    current_movement: MovementDir,
    alive: bool,
    hit_points: i32,
    death_animation_dir: i32,
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
            current_movement: MovementDir::East,
            alive: true,
            hit_points: HIT_POINTS,
            death_animation_dir: 0,
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
        if self.alive {
            if key == VirtualKeyCode::Delete && state == ElementState::Pressed {
                println!("BOOM");
                self.hit_points = 0;
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

        if self.hit_points == 0 {
            self.alive = false;

            // remove self from collision space
            collision_space.remove_dynamic_sprite_with_entity_id(self.entity_id());

            // send death message to spawn point
            message_dispatcher.enqueue(Message::entity_to_entity(
                self.entity_id(),
                self.spawn_point_id,
                Event::SpawnedEntityDidDie,
            ));

            // send death animation message
            message_dispatcher.enqueue(Message::entity_to_global(
                self.entity_id(),
                Event::PlayEntityDeathAnimation {
                    position: self.position.xy(),
                    direction: self.death_animation_dir,
                },
            ));

            return;
        }

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
        let snapped_next_position_center = point2(
            (next_position.x + 0.5).floor() as i32,
            next_position.y.floor() as i32,
        );
        let mut should_reverse_direction = false;

        match self.current_movement {
            MovementDir::East => {
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
            MovementDir::West => {
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
        self.alive
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
        if let Event::HitByFireball { direction } = message.event {
            self.hit_points = (self.hit_points - 1).max(0);
            self.death_animation_dir = direction;
        }
    }
}
