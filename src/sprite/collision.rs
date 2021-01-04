use cgmath::{vec2, Point2};
use std::{collections::HashMap, unimplemented};

use crate::sprite::core::*;

#[derive(Clone, Copy, Debug)]
pub enum ProbeDir {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Clone, Copy, Debug)]
pub enum ProbeResult {
    None,
    OneHit {
        dist: f32,
        sprite: Sprite,
    },
    TwoHits {
        dist: f32,
        sprite_0: Sprite,
        sprite_1: Sprite,
    },
}

pub struct Space {
    unit_sprites: HashMap<Point2<i32>, Sprite>,
}

const HASH_MAP_SCALE: i32 = 1000;

impl Space {
    pub fn new(sprites: &[Sprite]) -> Self {
        let mut unit_sprite_map = HashMap::new();

        for sprite in sprites {
            // copy sprites into appropriate storage
            if sprite.extent.x == 1.0 && sprite.extent.y == 1.0 {
                unit_sprite_map.insert(
                    Point2::new(
                        sprite.origin.x.floor() as i32 * HASH_MAP_SCALE,
                        sprite.origin.y.floor() as i32 * HASH_MAP_SCALE,
                    ),
                    *sprite,
                );
            } else {
                unimplemented!("SpriteHitTester does not support non-unit static sprites.")
            }
        }

        Self {
            unit_sprites: unit_sprite_map,
        }
    }

    pub fn get_sprite_at(&self, point: Point2<i32>, mask: u32) -> Option<Sprite> {
        self.unit_sprites
            .get(&(point * HASH_MAP_SCALE))
            .filter(|s| s.mask & mask != 0)
            .map(|s| *s)
    }

    pub fn add_sprite(&mut self, sprite: &Sprite) {
        let coord = Point2::new(
            sprite.origin.x.floor() as i32 * HASH_MAP_SCALE,
            sprite.origin.y.floor() as i32 * HASH_MAP_SCALE,
        );
        self.unit_sprites.insert(coord, *sprite);
    }

    pub fn remove_sprite(&mut self, sprite: &Sprite) {
        let coord = Point2::new(
            sprite.origin.x.floor() as i32 * HASH_MAP_SCALE,
            sprite.origin.y.floor() as i32 * HASH_MAP_SCALE,
        );
        self.unit_sprites.remove(&coord);
    }

    pub fn remove_sprite_at(&mut self, point: Point2<i32>) {
        self.unit_sprites.remove(&(point * HASH_MAP_SCALE));
    }

    /// tests if a point in the sprites' coordinate system intersects with a sprite.
    /// Filters by mask, such that only sprites with matching mask bits will be matched.
    /// In the case of overlapping sprites, there is no guarantee which will be returned,
    /// except that unit sprites will be tested before non-unit sprites.
    pub fn test_point(&self, point: Point2<f32>, mask: u32) -> Option<Sprite> {
        self.unit_sprites
            .get(&Point2::new(
                point.x.floor() as i32 * HASH_MAP_SCALE,
                point.y.floor() as i32 * HASH_MAP_SCALE,
            ))
            .filter(|s| s.mask & mask != 0 && s.contains(&point))
            .map(|s| *s)
    }

    /// Probes `max_steps` sprites in the collision space from `position` in `dir`, returning a ProbeResult
    /// Ignores any sprites which don't match the provided `mask`
    /// NOTE: Probe only tests for sprites with Square collision shape, because, well, that's what's needed here
    /// and I'm not writing a library.
    pub fn probe<F>(
        &self,
        position: Point2<f32>,
        dir: ProbeDir,
        max_steps: i32,
        mask: u32,
        test: F,
    ) -> ProbeResult
    where
        F: Fn(f32, &Sprite) -> bool,
    {
        let (offset, should_probe_offset) = match dir {
            ProbeDir::Up | ProbeDir::Down => (vec2(1.0, 0.0), position.x.fract().abs() > 0.0),
            ProbeDir::Right | ProbeDir::Left => (vec2(0.0, 1.0), position.y.fract().abs() > 0.0),
        };

        let mut dist = None;
        let mut sprite_0 = None;
        let mut sprite_1 = None;
        if let Some(r) = self._probe_line(position, dir, max_steps, mask) {
            if test(r.0, &r.1) {
                dist = Some(r.0);
                sprite_0 = Some(r.1);
            }
        }

        if should_probe_offset {
            if let Some(r) = self._probe_line(position + offset, dir, max_steps, mask) {
                if test(r.0, &r.1) {
                    dist = match dist {
                        Some(d) => Some(d.min(r.0)),
                        None => Some(r.0),
                    };
                    sprite_1 = Some(r.1);
                }
            }
        }

        match (sprite_0, sprite_1) {
            (None, None) => ProbeResult::None,
            (None, Some(s)) => ProbeResult::OneHit {
                dist: dist.unwrap(),
                sprite: s,
            },
            (Some(s), None) => ProbeResult::OneHit {
                dist: dist.unwrap(),
                sprite: s,
            },
            (Some(s0), Some(s1)) => ProbeResult::TwoHits {
                dist: dist.unwrap(),
                sprite_0: s0,
                sprite_1: s1,
            },
        }
    }

    fn _probe_line(
        &self,
        position: Point2<f32>,
        dir: ProbeDir,
        max_steps: i32,
        mask: u32,
    ) -> Option<(f32, Sprite)> {
        let position_snapped = Point2::new(position.x.floor() as i32, position.y.floor() as i32);
        let mut result = None;
        match dir {
            ProbeDir::Right => {
                for i in 0..max_steps {
                    let x = position_snapped.x + i;
                    if let Some(s) = self.get_sprite_at(Point2::new(x, position_snapped.y), mask) {
                        result = Some((s.origin.x - (position.x + 1.0), s));
                        break;
                    }
                }
            }
            ProbeDir::Up => {
                for i in 0..max_steps {
                    let y = position_snapped.y + i;
                    if let Some(s) = self.get_sprite_at(Point2::new(position_snapped.x, y), mask) {
                        result = Some((s.origin.y - (position.y + 1.0), s));
                        break;
                    }
                }
            }
            ProbeDir::Down => {
                for i in 0..max_steps {
                    let y = position_snapped.y - i;
                    if let Some(s) = self.get_sprite_at(Point2::new(position_snapped.x, y), mask) {
                        result = Some((position.y - s.top(), s));
                        break;
                    }
                }
            }
            ProbeDir::Left => {
                for i in 0..max_steps {
                    let x = position_snapped.x - i;
                    if let Some(s) = self.get_sprite_at(Point2::new(x, position_snapped.y), mask) {
                        result = Some((position.x - s.right(), s));
                        break;
                    }
                }
            }
        };

        // we only accept collisions with square shapes - because slopes are special cases handled by
        // find_character_footing only (note, the game only has northeast, and northwest slopes)
        if let Some(result) = result {
            if result.0 >= 0.0 && result.1.collision_shape == CollisionShape::Square {
                return Some(result);
            }
        }

        None
    }
}

#[cfg(test)]
mod sprite_hit_tester {
    use super::*;
    use cgmath::vec4;

    #[test]
    fn new_produces_expected_storage() {
        let tco = Point2::new(0.0, 0.0);
        let tce = vec2(1.0, 1.0);
        let color = vec4(1.0, 1.0, 1.0, 1.0);

        let unit_0 = Sprite::unit(
            CollisionShape::Square,
            Point2::new(0, 0),
            0.0,
            tco,
            tce,
            color,
            0,
        );
        let unit_1 = Sprite::unit(
            CollisionShape::Square,
            Point2::new(11, -33),
            0.0,
            tco,
            tce,
            color,
            0,
        );

        let hit_tester = Space::new(&[unit_0, unit_1]);
        assert_eq!(
            hit_tester
                .unit_sprites
                .get(&Point2::new(
                    unit_0.origin.x as i32 * HASH_MAP_SCALE,
                    unit_0.origin.y as i32 * HASH_MAP_SCALE
                ))
                .unwrap(),
            &unit_0
        );
        assert_eq!(
            hit_tester
                .unit_sprites
                .get(&Point2::new(
                    unit_1.origin.x as i32 * HASH_MAP_SCALE,
                    unit_1.origin.y as i32 * HASH_MAP_SCALE
                ))
                .unwrap(),
            &unit_1
        );
    }

    #[test]
    fn unit_sprite_hit_test_works() {
        let square_mask = 1 << 0;
        let triangle_mask = 1 << 1;
        let all_mask = square_mask | triangle_mask;

        let tco = Point2::new(0.0, 0.0);
        let tce = vec2(1.0, 1.0);
        let color = vec4(1.0, 1.0, 1.0, 1.0);

        let sb1 = Sprite::unit(
            CollisionShape::Square,
            Point2::new(0, 0),
            10.0,
            tco,
            tce,
            color,
            square_mask,
        );

        let sb2 = Sprite::unit(
            CollisionShape::Square,
            Point2::new(-1, -1),
            10.0,
            tco,
            tce,
            color,
            square_mask,
        );

        let tr0 = Sprite::unit(
            CollisionShape::NorthEast,
            Point2::new(0, 4),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let tr1 = Sprite::unit(
            CollisionShape::NorthWest,
            Point2::new(-1, 4),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let tr2 = Sprite::unit(
            CollisionShape::SouthWest,
            Point2::new(-1, 3),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let tr3 = Sprite::unit(
            CollisionShape::SouthEast,
            Point2::new(0, 3),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let hit_tester = Space::new(&[sb1, sb2, tr0, tr1, tr2, tr3]);

        // test triangle is hit only when using triangle_flags or all_mask
        assert!(hit_tester.test_point(Point2::new(0.1, 4.1), triangle_mask) == Some(tr0));
        assert!(hit_tester.test_point(Point2::new(-0.1, 4.1), triangle_mask) == Some(tr1));
        assert!(hit_tester.test_point(Point2::new(-0.1, 3.9), triangle_mask) == Some(tr2));
        assert!(hit_tester.test_point(Point2::new(0.1, 3.9), triangle_mask) == Some(tr3));
        assert!(hit_tester
            .test_point(Point2::new(0.1, 4.1), square_mask)
            .is_none());
        assert!(hit_tester
            .test_point(Point2::new(0.1, 3.9), all_mask)
            .is_some());

        // test square is only hit when mask is square or all_mask
        assert!(hit_tester.test_point(Point2::new(0.5, 0.5), square_mask) == Some(sb1));
        assert!(hit_tester
            .test_point(Point2::new(0.5, 0.5), triangle_mask)
            .is_none());
        assert!(hit_tester
            .test_point(Point2::new(0.5, 0.5), all_mask)
            .is_some());
    }
}
