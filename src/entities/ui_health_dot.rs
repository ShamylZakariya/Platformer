use std::time::Duration;

use cgmath::*;

use crate::{
    collision,
    entity::{Entity, GameStatePeek},
    event_dispatch::*,
    map,
    sprite::{self, rendering},
    state::constants::{layers, sprite_masks},
    tileset, util,
};

pub struct UiHealthDot {
    entity_id: u32,
    collider_id: Option<u32>,
    position: Point3<f32>,
    index: Option<i32>,
    visible: bool, // is the dot visible
    filled: bool,  // is the dot filled? else empty.
}

impl Default for UiHealthDot {
    fn default() -> Self {
        Self {
            entity_id: 0,
            collider_id: None,
            position: point3(0.0, 0.0, 0.0),
            index: None,
            visible: false,
            filled: false,
        }
    }
}

impl Entity for UiHealthDot {
    fn init_from_map_sprite(
        &mut self,
        entity_id: u32,
        sprite: &sprite::Sprite,
        _tile: &tileset::Tile,
        _map: &map::Map,
        collision_space: &mut collision::Space,
    ) {
        self.entity_id = entity_id;
        self.position = point3(sprite.origin.x, sprite.origin.y, layers::ui::FOREGROUND);

        let mut collider = collision::Collider::from_static_sprite(sprite);
        collider.mask = sprite_masks::ui::HEALTH_DOT;

        self.collider_id = Some(collision_space.add_collider(collider));
    }

    fn update(
        &mut self,
        _dt: Duration,
        _map: &map::Map,
        collision_space: &mut collision::Space,
        _message_dispatcher: &mut Dispatcher,
        game_state_peek: &GameStatePeek,
    ) {
        if self.index.is_none() {
            self.index = Some(self.determine_index(collision_space));
        }

        if let Some(index) = self.index {
            self.visible = index < game_state_peek.player_health.1 as i32;
            self.filled = index < game_state_peek.player_health.0 as i32;
        }
    }

    fn update_uniforms(&self, uniforms: &mut util::Uniforms<rendering::UniformData>) {
        uniforms.data.set_model_position(self.position);
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
        crate::entities::EntityClass::UiHealthDot
    }

    fn is_alive(&self) -> bool {
        true
    }

    fn should_draw(&self) -> bool {
        self.visible
    }

    fn position(&self) -> Point3<f32> {
        self.position
    }

    fn sprite_name(&self) -> &str {
        "health_dot"
    }

    fn sprite_cycle(&self) -> &str {
        if self.filled {
            "full"
        } else {
            "empty"
        }
    }
}

impl UiHealthDot {
    /// Determine the health point index of this dot. Since we don't have any way to pass arguments
    /// to an entity at construction, there's no way in game_ui.tmx to specify that one dot is for health
    /// point 0, the next for health point 1, and so on. So, instead, we need to figure it out on our own.
    /// Here we walk left, looking for other heath dots in the collision space, until we come up empty. The
    /// length of that walk determines our index. Yeesh.
    fn determine_index(&self, collision_space: &collision::Space) -> i32 {
        let position: Point2<i32> = collision_space
            .get_collider(self.collider_id.unwrap())
            .unwrap()
            .bounds()
            .origin
            .cast()
            .unwrap();
        let mut offset: i32 = 1;
        loop {
            let test_position = point2(position.x - offset, position.y);
            if collision_space
                .get_collider_at(test_position, sprite_masks::ui::HEALTH_DOT)
                .is_some()
            {
                offset += 1;
            } else {
                break;
            }
        }
        offset - 1
    }
}
