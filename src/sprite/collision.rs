use cgmath::*;
use std::{collections::HashMap, unimplemented};

use crate::sprite::core::*;

pub struct Space {
    static_unit_sprites: HashMap<Point2<i32>, Sprite>,
    dynamic_sprites: HashMap<u32, Sprite>,
}

/// A "space" for hit testing against static and dynamic sprites.
/// Static sprites can be added and removed, but should generally stay in position.
/// Static sprites also are unit sized, and generally represent level tiles and
/// unmoving single unit sized objects.
/// Dynamic sprites are expected to move about during runtime, and are intended for
/// representing moving entities. Dynamic sprites can be arbitrarily sized.
/// Dynamic sprites are identified by their entity_id. It is illegal to attempt
/// to add a Dynamic sprite without an entity id.
impl Space {
    /// Constructs a new Space with the provided static sprites.
    /// Static sprites don't move at runtime. Sprites that move at
    /// runtime should be added and manipulated via the dynamic_sprite methods.
    pub fn new(static_sprites: &[Sprite]) -> Self {
        let mut unit_sprite_map = HashMap::new();

        for sprite in static_sprites {
            // copy sprites into appropriate storage
            if sprite.extent.x == 1.0 && sprite.extent.y == 1.0 {
                unit_sprite_map.insert(
                    point2(
                        sprite.origin.x.floor() as i32,
                        sprite.origin.y.floor() as i32,
                    ),
                    *sprite,
                );
            } else {
                unimplemented!("Static sprites must be unit-sized")
            }
        }

        Self {
            static_unit_sprites: unit_sprite_map,
            dynamic_sprites: HashMap::new(),
        }
    }

    pub fn get_static_sprite_at(&self, point: Point2<i32>, mask: u32) -> Option<Sprite> {
        self.static_unit_sprites
            .get(&(point))
            .filter(|s| s.mask & mask != 0)
            .map(|s| *s)
    }

    pub fn add_static_sprite(&mut self, sprite: &Sprite) {
        let coord = point2(
            sprite.origin.x.floor() as i32,
            sprite.origin.y.floor() as i32,
        );
        self.static_unit_sprites.insert(coord, *sprite);
    }

    pub fn remove_static_sprite(&mut self, sprite: &Sprite) {
        let coord = point2(
            sprite.origin.x.floor() as i32,
            sprite.origin.y.floor() as i32,
        );
        self.static_unit_sprites.remove(&coord);
    }

    pub fn remove_static_sprite_at(&mut self, point: Point2<i32>) {
        self.static_unit_sprites.remove(&(point));
    }

    pub fn add_dynamic_sprite(&mut self, sprite: &Sprite) {
        let id = sprite
            .entity_id
            .expect("Dynamic sprites must have an entity_id");
        self.dynamic_sprites.insert(id, *sprite);
    }

    pub fn remove_dynamic_sprite(&mut self, sprite: &Sprite) {
        let id = sprite
            .entity_id
            .expect("Dynamic sprites must have an entity_id");
        self.dynamic_sprites.remove(&id);
    }

    pub fn remove_dynamic_sprite_with_entity_id(&mut self, entity_id: u32) {
        self.dynamic_sprites.remove(&entity_id);
    }

    pub fn update_dynamic_sprite(&mut self, sprite: &Sprite) {
        self.add_dynamic_sprite(sprite);
    }

    /// Tests if a point in the sprites' coordinate system intersects with a sprite.
    /// Filters by mask, such that only sprites with matching mask bits will be matched.
    /// In the case of overlapping sprites, dynamic sprites will be returned before static,
    /// but otherwise there is no guarantee of which will be returned.
    pub fn test_point(&self, point: Point2<f32>, mask: u32) -> Option<Sprite> {
        for s in self.dynamic_sprites.values() {
            if s.mask & mask != 0 && s.contains(&point) {
                return Some(*s);
            }
        }

        self.static_unit_sprites
            .get(&point2(point.x.floor() as i32, point.y.floor() as i32))
            .filter(|s| s.mask & mask != 0 && s.contains(&point))
            .map(|s| *s)
    }
}

#[cfg(test)]
mod sprite_hit_tester {
    use super::*;

    #[test]
    fn new_produces_expected_storage() {
        let tco = point2(0.0, 0.0);
        let tce = vec2(1.0, 1.0);
        let color = vec4(1.0, 1.0, 1.0, 1.0);

        let unit_0 = Sprite::unit(
            CollisionShape::Square,
            point2(0, 0),
            0.0,
            tco,
            tce,
            color,
            0,
        );
        let unit_1 = Sprite::unit(
            CollisionShape::Square,
            point2(11, -33),
            0.0,
            tco,
            tce,
            color,
            0,
        );

        let hit_tester = Space::new(&[unit_0, unit_1]);
        assert_eq!(
            hit_tester
                .static_unit_sprites
                .get(&point2(unit_0.origin.x as i32, unit_0.origin.y as i32,))
                .unwrap(),
            &unit_0
        );
        assert_eq!(
            hit_tester
                .static_unit_sprites
                .get(&point2(unit_1.origin.x as i32, unit_1.origin.y as i32,))
                .unwrap(),
            &unit_1
        );
    }

    #[test]
    fn unit_sprite_hit_test_works() {
        let square_mask = 1 << 0;
        let triangle_mask = 1 << 1;
        let all_mask = square_mask | triangle_mask;

        let tco = point2(0.0, 0.0);
        let tce = vec2(1.0, 1.0);
        let color = vec4(1.0, 1.0, 1.0, 1.0);

        let sb1 = Sprite::unit(
            CollisionShape::Square,
            point2(0, 0),
            10.0,
            tco,
            tce,
            color,
            square_mask,
        );

        let sb2 = Sprite::unit(
            CollisionShape::Square,
            point2(-1, -1),
            10.0,
            tco,
            tce,
            color,
            square_mask,
        );

        let tr0 = Sprite::unit(
            CollisionShape::NorthEast,
            point2(0, 4),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let tr1 = Sprite::unit(
            CollisionShape::NorthWest,
            point2(-1, 4),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let tr2 = Sprite::unit(
            CollisionShape::SouthWest,
            point2(-1, 3),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let tr3 = Sprite::unit(
            CollisionShape::SouthEast,
            point2(0, 3),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let hit_tester = Space::new(&[sb1, sb2, tr0, tr1, tr2, tr3]);

        // test triangle is hit only when using triangle_flags or all_mask
        assert!(hit_tester.test_point(point2(0.1, 4.1), triangle_mask) == Some(tr0));
        assert!(hit_tester.test_point(point2(-0.1, 4.1), triangle_mask) == Some(tr1));
        assert!(hit_tester.test_point(point2(-0.1, 3.9), triangle_mask) == Some(tr2));
        assert!(hit_tester.test_point(point2(0.1, 3.9), triangle_mask) == Some(tr3));
        assert!(hit_tester
            .test_point(point2(0.1, 4.1), square_mask)
            .is_none());
        assert!(hit_tester.test_point(point2(0.1, 3.9), all_mask).is_some());

        // test square is only hit when mask is square or all_mask
        assert!(hit_tester.test_point(point2(0.5, 0.5), square_mask) == Some(sb1));
        assert!(hit_tester
            .test_point(point2(0.5, 0.5), triangle_mask)
            .is_none());
        assert!(hit_tester.test_point(point2(0.5, 0.5), all_mask).is_some());
    }
}
