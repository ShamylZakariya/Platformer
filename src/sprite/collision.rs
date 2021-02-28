use cgmath::*;
use std::hash::Hash;
use std::{collections::HashMap, unimplemented};

use crate::{
    geom::{self, Bounds},
    sprite::core::*,
};

fn rel_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < f32::EPSILON
}

/// Simple cross product for 2D vectors; cgmath doesn't define this because cross product
/// doesn't make sense generally for 2D.
fn cross(a: &Vector2<f32>, b: &Vector2<f32>) -> f32 {
    a.x * b.y - a.y * b.x
}

fn hash_point2<H: std::hash::Hasher>(point: &Point2<f32>, state: &mut H) {
    ((point.x * 1000.0) as i32).hash(state);
    ((point.y * 1000.0) as i32).hash(state);
}

fn hash_vec2<H: std::hash::Hasher>(v: &Vector2<f32>, state: &mut H) {
    ((v.x * 1000.0) as i32).hash(state);
    ((v.y * 1000.0) as i32).hash(state);
}

#[derive(Debug, Clone, Copy)]
pub struct Collider {
    pub bounds: Bounds,
    pub shape: CollisionShape,
    pub mask: u32,
    pub entity_id: Option<u32>,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            bounds: Bounds::default(),
            shape: CollisionShape::None,
            mask: 0,
            entity_id: None,
        }
    }
}

impl From<Sprite> for Collider {
    fn from(sprite: Sprite) -> Self {
        Self {
            bounds: Bounds::new(sprite.origin.xy(), sprite.extent),
            shape: sprite.collision_shape,
            mask: sprite.mask,
            entity_id: sprite.entity_id,
        }
    }
}

impl From<&Sprite> for Collider {
    fn from(sprite: &Sprite) -> Self {
        Self {
            bounds: Bounds::new(sprite.origin.xy(), sprite.extent),
            shape: sprite.collision_shape,
            mask: sprite.mask,
            entity_id: sprite.entity_id,
        }
    }
}

impl PartialEq for Collider {
    fn eq(&self, other: &Self) -> bool {
        self.shape == other.shape
            && self.entity_id == other.entity_id
            && self.mask == other.mask
            && relative_eq!(self.bounds.origin, other.bounds.origin)
            && relative_eq!(self.bounds.extent, other.bounds.extent)
    }
}

impl Eq for Collider {}

impl Hash for Collider {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.shape.hash(state);
        self.entity_id.hash(state);
        hash_point2(&self.bounds.origin, state);
        hash_vec2(&self.bounds.extent, state);
        self.mask.hash(state);
    }
}

impl Collider {
    pub fn new(bounds: Bounds, shape: CollisionShape, mask: u32, entity_id: Option<u32>) -> Self {
        Self {
            bounds,
            shape,
            mask,
            entity_id,
        }
    }

    pub fn contains(&self, point: &Point2<f32>) -> bool {
        if point.x >= self.bounds.origin.x
            && point.x <= self.bounds.origin.x + self.bounds.extent.x
            && point.y >= self.bounds.origin.y
            && point.y <= self.bounds.origin.y + self.bounds.extent.y
        {
            let p = vec2(point.x, point.y);
            return match self.shape {
                CollisionShape::None => false,

                CollisionShape::Square => true,

                CollisionShape::NorthEast => {
                    let a = vec2(
                        self.bounds.origin.x,
                        self.bounds.origin.y + self.bounds.extent.y,
                    );
                    let b = vec2(
                        self.bounds.origin.x + self.bounds.extent.x,
                        self.bounds.origin.y,
                    );
                    let ba = b - a;
                    let pa = p - a;
                    cross(&ba, &pa) <= 0.0
                }

                CollisionShape::SouthEast => {
                    let a = vec2(self.bounds.origin.x, self.bounds.origin.y);
                    let b = vec2(
                        self.bounds.origin.x + self.bounds.extent.x,
                        self.bounds.origin.y + self.bounds.extent.y,
                    );
                    let ba = b - a;
                    let pa = p - a;
                    cross(&ba, &pa) >= 0.0
                }

                CollisionShape::SouthWest => {
                    let a = vec2(
                        self.bounds.origin.x,
                        self.bounds.origin.y + self.bounds.extent.y,
                    );
                    let b = vec2(
                        self.bounds.origin.x + self.bounds.extent.x,
                        self.bounds.origin.y,
                    );
                    let ba = b - a;
                    let pa = p - a;
                    // opposite winding of northeast
                    cross(&ba, &pa) >= 0.0
                }

                CollisionShape::NorthWest => {
                    let a = vec2(self.bounds.origin.x, self.bounds.origin.y);
                    let b = vec2(
                        self.bounds.origin.x + self.bounds.extent.x,
                        self.bounds.origin.y + self.bounds.extent.y,
                    );
                    let ba = b - a;
                    let pa = p - a;
                    // opposite winding of southeast
                    cross(&ba, &pa) <= 0.0
                }
            };
        }

        false
    }

    /// if the line described by a->b intersects this Sprite, returns the point on it where the line
    /// segment intersects, otherwise, returns None
    pub fn line_intersection(&self, a: &Point2<f32>, b: &Point2<f32>) -> Option<Point2<f32>> {
        match self.shape {
            CollisionShape::None => None,
            CollisionShape::Square => geom::intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(self.bounds.origin.x, self.bounds.origin.y),
                    point2(
                        self.bounds.origin.x + self.bounds.extent.x,
                        self.bounds.origin.y,
                    ),
                    point2(
                        self.bounds.origin.x + self.bounds.extent.x,
                        self.bounds.origin.y + self.bounds.extent.y,
                    ),
                    point2(
                        self.bounds.origin.x,
                        self.bounds.origin.y + self.bounds.extent.y,
                    ),
                ],
            ),
            CollisionShape::NorthEast => geom::intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(self.bounds.origin.x, self.bounds.origin.y),
                    point2(
                        self.bounds.origin.x + self.bounds.extent.x,
                        self.bounds.origin.y,
                    ),
                    point2(
                        self.bounds.origin.x,
                        self.bounds.origin.y + self.bounds.extent.y,
                    ),
                ],
            ),
            CollisionShape::SouthEast => geom::intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(self.bounds.origin.x, self.bounds.origin.y),
                    point2(
                        self.bounds.origin.x + self.bounds.extent.x,
                        self.bounds.origin.y + self.bounds.extent.y,
                    ),
                    point2(
                        self.bounds.origin.x,
                        self.bounds.origin.y + self.bounds.extent.y,
                    ),
                ],
            ),
            CollisionShape::SouthWest => geom::intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(
                        self.bounds.origin.x + self.bounds.extent.x,
                        self.bounds.origin.y,
                    ),
                    point2(
                        self.bounds.origin.x + self.bounds.extent.x,
                        self.bounds.origin.y + self.bounds.extent.y,
                    ),
                    point2(
                        self.bounds.origin.x,
                        self.bounds.origin.y + self.bounds.extent.y,
                    ),
                ],
            ),
            CollisionShape::NorthWest => geom::intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(self.bounds.origin.x, self.bounds.origin.y),
                    point2(
                        self.bounds.origin.x + self.bounds.extent.x,
                        self.bounds.origin.y,
                    ),
                    point2(
                        self.bounds.origin.x + self.bounds.extent.x,
                        self.bounds.origin.y + self.bounds.extent.y,
                    ),
                ],
            ),
        }
    }

    /// Returns true if this Sprite overlaps the described rect with lower/left origin and extent and inset.
    /// Inset: The amount to inset the test rect
    /// contact: If true, contacts will also count as an intersection, not just overlap. In this case rects with touching edges will be treated as intersections.
    pub fn rect_intersection(
        &self,
        origin: &Point2<f32>,
        extent: &Vector2<f32>,
        inset: f32,
        contact: bool,
    ) -> bool {
        let origin = point2(origin.x + inset, origin.y + inset);
        let extent = vec2(extent.x - 2.0 * inset, extent.y - 2.0 * inset);

        let (x_overlap, y_overlap) = if contact {
            (
                self.bounds.origin.x <= origin.x + extent.x
                    && self.bounds.origin.x + self.bounds.extent.x >= origin.x,
                self.bounds.origin.y <= origin.y + extent.y
                    && self.bounds.origin.y + self.bounds.extent.y >= origin.y,
            )
        } else {
            (
                self.bounds.origin.x < origin.x + extent.x
                    && self.bounds.origin.x + self.bounds.extent.x > origin.x,
                self.bounds.origin.y < origin.y + extent.y
                    && self.bounds.origin.y + self.bounds.extent.y > origin.y,
            )
        };

        if x_overlap && y_overlap {
            // TODO: Implement colliders for corner blocks? GGQ doesn't need them,
            // but it seems like I'd want to flesh that out for a real 2d engine.
            // So for now, we treat any non-none shape as rectangular.
            !matches!(self.shape, CollisionShape::None)
        } else {
            false
        }
    }

    /// Returns true if this Sprite overlaps the described unit square with lower/left origin and extent of (1,1).
    pub fn unit_rect_intersection(&self, origin: &Point2<f32>, inset: f32, contact: bool) -> bool {
        self.rect_intersection(origin, &vec2(1.0, 1.0), inset, contact)
    }

    pub fn left(&self) -> f32 {
        self.bounds.origin.x
    }

    pub fn right(&self) -> f32 {
        self.bounds.origin.x + self.bounds.extent.x
    }

    pub fn bottom(&self) -> f32 {
        self.bounds.origin.y
    }

    pub fn top(&self) -> f32 {
        self.bounds.origin.y + self.bounds.extent.y
    }
}

pub struct Space {
    static_unit_colliders: HashMap<Point2<i32>, Collider>,
    dynamic_colliders: HashMap<u32, Collider>,
}

/// A "space" for hit testing against static and dynamic colliders.
/// Static colliders can be added and removed, but should generally stay in position.
/// Static colliders also are unit sized, and generally represent level tiles and
/// unmoving single unit sized objects.
/// Dynamic colliders are expected to move about during runtime, and are intended for
/// representing moving entities. Dynamic colliders can be arbitrarily sized.
/// Dynamic colliders are identified by their entity_id. It is illegal to attempt
/// to add a Dynamic collider without an entity id.
impl Space {
    /// Constructs a new Space with the provided static colliders.
    /// Static colliders don't move at runtime. Colliders that move at
    /// runtime should be added and manipulated via the dynamic_ methods.
    pub fn new(static_colliders: &[Collider]) -> Self {
        let mut static_unit_colliders = HashMap::new();

        for c in static_colliders {
            // copy sprites into appropriate storage
            if rel_eq(c.bounds.extent.x, 1.0) && rel_eq(c.bounds.extent.y, 1.0) {
                static_unit_colliders.insert(
                    point2(
                        c.bounds.origin.x.floor() as i32,
                        c.bounds.origin.y.floor() as i32,
                    ),
                    *c,
                );
            } else {
                unimplemented!("Static colliders must be unit-sized")
            }
        }

        Self {
            static_unit_colliders,
            dynamic_colliders: HashMap::new(),
        }
    }

    pub fn get_static_collider_at(&self, point: Point2<i32>, mask: u32) -> Option<&Collider> {
        self.static_unit_colliders
            .get(&(point))
            .filter(|s| s.mask & mask != 0)
    }

    pub fn has_static_collider(&self, collider: &Collider) -> bool {
        let coord = point2(
            collider.bounds.origin.x.floor() as i32,
            collider.bounds.origin.y.floor() as i32,
        );

        self.static_unit_colliders.contains_key(&coord)
    }

    pub fn add_static_collider(&mut self, collider: &Collider) {
        let coord = point2(
            collider.bounds.origin.x.floor() as i32,
            collider.bounds.origin.y.floor() as i32,
        );
        self.static_unit_colliders.insert(coord, *collider);
    }

    pub fn remove_static_collider(&mut self, collider: &Collider) {
        let coord = point2(
            collider.bounds.origin.x.floor() as i32,
            collider.bounds.origin.y.floor() as i32,
        );
        self.static_unit_colliders.remove(&coord);
    }

    pub fn remove_static_collider_at(&mut self, point: Point2<i32>) {
        self.static_unit_colliders.remove(&(point));
    }

    pub fn add_dynamic_collider(&mut self, collider: &Collider) {
        let id = collider
            .entity_id
            .expect("Dynamic sprites must have an entity_id");
        self.dynamic_colliders.insert(id, *collider);
    }

    pub fn remove_dynamic_collider(&mut self, sprite: &Collider) {
        let id = sprite
            .entity_id
            .expect("Dynamic sprites must have an entity_id");
        self.dynamic_colliders.remove(&id);
    }

    pub fn remove_dynamic_collider_with_entity_id(&mut self, entity_id: u32) {
        self.dynamic_colliders.remove(&entity_id);
    }

    pub fn update_dynamic_collider(&mut self, collider: &Collider) {
        self.add_dynamic_collider(collider);
    }

    /// Tests the specified rect against just the dynamic colliders in this Space
    pub fn test_rect_against_dynamic_colliders<C>(
        &self,
        origin: &Point2<f32>,
        extent: &Vector2<f32>,
        mask: u32,
        mut callback: C,
    ) where
        C: FnMut(&Collider) -> bool,
    {
        for (_, c) in self.dynamic_colliders.iter() {
            if c.mask & mask != 0 && c.rect_intersection(origin, extent, 0.0, true) && callback(c) {
                return;
            }
        }
    }

    /// Tests the specified rect against just the static colliders in this Space calling the callback for each
    /// match, while the callback returns false. On returning true, the search will finish, signalling that the
    /// callback is "done"
    pub fn test_rect_against_static_colliders<C>(
        &self,
        origin: &Point2<f32>,
        extent: &Vector2<f32>,
        mask: u32,
        mut callback: C,
    ) where
        C: FnMut(&Collider) -> bool,
    {
        let snapped_extent = vec2(extent.x.round() as i32, extent.y.round() as i32);
        let a = point2(origin.x.floor() as i32, origin.y.floor() as i32);
        let b = point2(a.x + snapped_extent.x, a.y);
        let c = point2(a.x, a.y + snapped_extent.y);
        let d = point2(a.x + snapped_extent.x, a.y + snapped_extent.y);

        for p in [a, b, c, d].iter() {
            if let Some(c) = self.get_static_collider_at(*p, mask) {
                if c.rect_intersection(origin, extent, 0.0, true) && callback(c) {
                    return;
                }
            }
        }
    }

    /// Tests if a rect intersects with a dynamic or static collider.
    /// Filters by mask, such that only sprites with matching mask bits will be matched.
    /// In the case of overlapping sprites, dynamic sprites will be returned before static,
    /// but otherwise there is no guarantee of which will be returned.
    pub fn test_rect(
        &self,
        origin: &Point2<f32>,
        extent: &Vector2<f32>,
        mask: u32,
    ) -> Option<&Collider> {
        for (_, c) in self.dynamic_colliders.iter() {
            if c.mask & mask != 0 && c.rect_intersection(origin, extent, 0.0, true) {
                return Some(c);
            }
        }
        let snapped_extent = vec2(extent.x.round() as i32, extent.y.round() as i32);
        let a = point2(origin.x.floor() as i32, origin.y.floor() as i32);
        let b = point2(a.x + snapped_extent.x, a.y);
        let c = point2(a.x, a.y + snapped_extent.y);
        let d = point2(a.x + snapped_extent.x, a.y + snapped_extent.y);

        for p in [a, b, c, d].iter() {
            if let Some(c) = self.get_static_collider_at(*p, mask) {
                if c.rect_intersection(origin, extent, 0.0, true) {
                    return Some(c);
                }
            }
        }

        None
    }

    /// Tests if a point in the sprites' coordinate system intersects with a sprite.
    /// Filters by mask, such that only sprites with matching mask bits will be matched.
    /// In the case of overlapping sprites, dynamic sprites will be returned before static,
    /// but otherwise there is no guarantee of which will be returned.
    pub fn test_point(&self, point: Point2<f32>, mask: u32) -> Option<&Collider> {
        for s in self.dynamic_colliders.values() {
            if s.mask & mask != 0 && s.contains(&point) {
                return Some(s);
            }
        }

        self.static_unit_colliders
            .get(&point2(point.x.floor() as i32, point.y.floor() as i32))
            .filter(|s| s.mask & mask != 0 && s.contains(&point))
    }

    pub fn get_sprite_at(&self, point: Point2<i32>, mask: u32) -> Option<&Collider> {
        for s in self.dynamic_colliders.values() {
            if s.mask & mask != 0 && s.contains(&point2(point.x as f32 + 0.5, point.y as f32 + 0.5))
            {
                return Some(s);
            }
        }

        self.static_unit_colliders
            .get(&(point))
            .filter(|s| s.mask & mask != 0)
    }
}

#[cfg(test)]
mod space_tests {
    use super::*;

    #[test]
    fn new_produces_expected_storage() {
        let unit_0 = Collider::new(
            Bounds::new(point2(0.0, 0.0), vec2(1.0, 1.0)),
            CollisionShape::Square,
            0,
            None,
        );

        let unit_1 = Collider::new(
            Bounds::new(point2(11.0, -33.0), vec2(1.0, 1.0)),
            CollisionShape::Square,
            0,
            None,
        );

        let hit_tester = Space::new(&[unit_0, unit_1]);
        assert_eq!(
            hit_tester
                .static_unit_colliders
                .get(&point2(
                    unit_0.bounds.origin.x as i32,
                    unit_0.bounds.origin.y as i32,
                ))
                .unwrap(),
            &unit_0
        );
        assert_eq!(
            hit_tester
                .static_unit_colliders
                .get(&point2(
                    unit_1.bounds.origin.x as i32,
                    unit_1.bounds.origin.y as i32,
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

        let sb1 = Collider::new(
            Bounds::new(point2(0.0, 0.0), vec2(1.0, 1.0)),
            CollisionShape::Square,
            square_mask,
            None,
        );

        let sb2 = Collider::new(
            Bounds::new(point2(-1.0, -1.0), vec2(1.0, 1.0)),
            CollisionShape::Square,
            square_mask,
            None,
        );

        let tr0 = Collider::new(
            Bounds::new(point2(0.0, 4.0), vec2(1.0, 1.0)),
            CollisionShape::NorthEast,
            triangle_mask,
            None,
        );

        let tr1 = Collider::new(
            Bounds::new(point2(-1.0, 4.0), vec2(1.0, 1.0)),
            CollisionShape::NorthWest,
            triangle_mask,
            None,
        );

        let tr2 = Collider::new(
            Bounds::new(point2(-1.0, 3.0), vec2(1.0, 1.0)),
            CollisionShape::SouthWest,
            triangle_mask,
            None,
        );

        let tr3 = Collider::new(
            Bounds::new(point2(0.0, 3.0), vec2(1.0, 1.0)),
            CollisionShape::SouthEast,
            triangle_mask,
            None,
        );

        let hit_tester = Space::new(&[sb1, sb2, tr0, tr1, tr2, tr3]);

        // test triangle is hit only when using triangle_flags or all_mask
        assert!(hit_tester.test_point(point2(0.1, 4.1), triangle_mask) == Some(&tr0));
        assert!(hit_tester.test_point(point2(-0.1, 4.1), triangle_mask) == Some(&tr1));
        assert!(hit_tester.test_point(point2(-0.1, 3.9), triangle_mask) == Some(&tr2));
        assert!(hit_tester.test_point(point2(0.1, 3.9), triangle_mask) == Some(&tr3));
        assert!(hit_tester
            .test_point(point2(0.1, 4.1), square_mask)
            .is_none());
        assert!(hit_tester.test_point(point2(0.1, 3.9), all_mask).is_some());

        // test square is only hit when mask is square or all_mask
        assert!(hit_tester.test_point(point2(0.5, 0.5), square_mask) == Some(&sb1));
        assert!(hit_tester
            .test_point(point2(0.5, 0.5), triangle_mask)
            .is_none());
        assert!(hit_tester.test_point(point2(0.5, 0.5), all_mask).is_some());
    }
}
