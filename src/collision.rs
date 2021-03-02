use cgmath::*;
use std::hash::Hash;
use std::{collections::HashMap, unimplemented};

use crate::{sprite::core::*, util::*};

/// Represents the shape of a Collider, where Square represents simple square; the remainder
/// are triangles, with the surface normal facing in the specified direction. E.g., NorthEast would be a triangle
/// with the edge normal facing up and to the right.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Shape {
    None,
    Square,
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

impl Shape {
    pub fn flipped_horizontally(&self) -> Self {
        match self {
            Shape::None => Shape::None,
            Shape::Square => Shape::Square,
            Shape::NorthEast => Shape::NorthWest,
            Shape::SouthEast => Shape::SouthWest,
            Shape::SouthWest => Shape::SouthEast,
            Shape::NorthWest => Shape::NorthEast,
        }
    }
    pub fn flipped_vertically(&self) -> Self {
        match self {
            Shape::None => Shape::None,
            Shape::Square => Shape::Square,
            Shape::NorthEast => Shape::SouthEast,
            Shape::SouthEast => Shape::NorthEast,
            Shape::SouthWest => Shape::NorthWest,
            Shape::NorthWest => Shape::SouthWest,
        }
    }
    pub fn flipped_diagonally(&self) -> Self {
        // https://doc.mapeditor.org/en/stable/reference/tmx-map-format/#tile-flipping
        // Under section "Tile Flipping" diagonal flip is defined as x/y axis swap.
        // On paper, this transform was worked out for triangles. Since this is a
        // mirroring along the +x/+y diagonal axis, it only affects NorthWest and SouthEast
        // triangles, which are not symmetrical across the flip axis.
        match self {
            Shape::None => Shape::None,
            Shape::Square => Shape::Square,
            Shape::NorthEast => Shape::NorthEast,
            Shape::SouthEast => Shape::NorthWest,
            Shape::SouthWest => Shape::SouthWest,
            Shape::NorthWest => Shape::SouthEast,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Collider {
    pub bounds: Bounds,
    pub shape: Shape,
    pub mask: u32,
    pub entity_id: Option<u32>,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            bounds: Bounds::default(),
            shape: Shape::None,
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
    pub fn new(bounds: Bounds, shape: Shape, mask: u32, entity_id: Option<u32>) -> Self {
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
                Shape::None => false,

                Shape::Square => true,

                Shape::NorthEast => {
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

                Shape::SouthEast => {
                    let a = vec2(self.bounds.origin.x, self.bounds.origin.y);
                    let b = vec2(
                        self.bounds.origin.x + self.bounds.extent.x,
                        self.bounds.origin.y + self.bounds.extent.y,
                    );
                    let ba = b - a;
                    let pa = p - a;
                    cross(&ba, &pa) >= 0.0
                }

                Shape::SouthWest => {
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

                Shape::NorthWest => {
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
            Shape::None => None,
            Shape::Square => intersection::line_convex_poly_closest(
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
            Shape::NorthEast => intersection::line_convex_poly_closest(
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
            Shape::SouthEast => intersection::line_convex_poly_closest(
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
            Shape::SouthWest => intersection::line_convex_poly_closest(
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
            Shape::NorthWest => intersection::line_convex_poly_closest(
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
            !matches!(self.shape, Shape::None)
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

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

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

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    pub fn get_collider_at(&self, point: Point2<i32>, mask: u32) -> Option<&Collider> {
        let point_f = point2(point.x as f32 + 0.5, point.y as f32 + 0.5);
        for s in self.dynamic_colliders.values() {
            if s.mask & mask != 0 && s.contains(&point_f) {
                return Some(s);
            }
        }

        self.static_unit_colliders
            .get(&(point))
            .filter(|s| s.mask & mask != 0)
    }

    pub fn has_collider(&self, collider: &Collider) -> bool {
        let coord = point2(
            collider.bounds.origin.x.floor() as i32,
            collider.bounds.origin.y.floor() as i32,
        );

        if self.static_unit_colliders.contains_key(&coord) {
            return true;
        }

        if let Some(id) = collider.entity_id {
            return self.dynamic_colliders.contains_key(&id);
        }

        false
    }

    /// Tests the specified rect against colliders, invoking the specified callback for each contact/overlap.
    /// Filters by mask, such that only sprites with matching mask bits will be matched.
    /// In the case of overlapping sprites, dynamic sprites will be passed to the callback before static.
    /// The callback should return true to end the search, false to continue.
    pub fn test_rect<C>(
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

        let snapped_extent = vec2(extent.x.round() as i32, extent.y.round() as i32);
        let a = point2(origin.x.floor() as i32, origin.y.floor() as i32);
        let b = point2(a.x + snapped_extent.x, a.y);
        let c = point2(a.x, a.y + snapped_extent.y);
        let d = point2(a.x + snapped_extent.x, a.y + snapped_extent.y);

        for p in [a, b, c, d].iter() {
            if let Some(c) = self.get_collider_at(*p, mask) {
                if c.rect_intersection(origin, extent, 0.0, true) && callback(c) {
                    return;
                }
            }
        }
    }

    /// Tests if a rect intersects with a dynamic or static collider, returning first hit.
    /// Filters by mask, such that only sprites with matching mask bits will be matched.
    /// In the case of overlapping sprites, dynamic sprites will be returned before static,
    /// but otherwise there is no guarantee of which will be returned.
    pub fn test_rect_first(
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
            if let Some(c) = self.get_collider_at(*p, mask) {
                if c.rect_intersection(origin, extent, 0.0, true) {
                    return Some(c);
                }
            }
        }

        None
    }

    /// Tests if a point in the sprites' coordinate system intersects with a collider, returning first hit.
    /// Filters by mask, such that only sprites with matching mask bits will be matched.
    /// In the case of overlapping sprites, dynamic sprites will be returned before static,
    /// but otherwise there is no guarantee of which will be returned.
    pub fn test_point_first(&self, point: Point2<f32>, mask: u32) -> Option<&Collider> {
        for s in self.dynamic_colliders.values() {
            if s.mask & mask != 0 && s.contains(&point) {
                return Some(s);
            }
        }

        self.static_unit_colliders
            .get(&point2(point.x.floor() as i32, point.y.floor() as i32))
            .filter(|s| s.mask & mask != 0 && s.contains(&point))
    }
}

// ---------------------------------------------------------------------------------------------------------------------
#[derive(Clone, Copy, Debug)]
pub enum ProbeDir {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Clone, Copy, Debug)]
pub enum ProbeResult<'a> {
    None,
    OneHit {
        dist: f32,
        collider: &'a Collider,
    },
    TwoHits {
        dist: f32,
        collider_0: &'a Collider,
        collider_1: &'a Collider,
    },
}

impl Space {
    /// Probes `max_steps` in the collision space from `position` in `dir`, returning a ProbeResult
    /// Ignores any colliders which don't match the provided `mask`
    /// NOTE: Probe only tests for static sprites with Square collision shape, because, well,
    /// that's what's needed here and I'm not writing a damned game engine, I'm writing a damned Gargoyle's Quest engine.
    pub fn probe<F>(
        &self,
        position: Point2<f32>,
        dir: ProbeDir,
        max_steps: i32,
        mask: u32,
        test: F,
    ) -> ProbeResult
    where
        F: Fn(f32, &Collider) -> bool,
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
                collider: s,
            },
            (Some(s), None) => ProbeResult::OneHit {
                dist: dist.unwrap(),
                collider: s,
            },
            (Some(s0), Some(s1)) => ProbeResult::TwoHits {
                dist: dist.unwrap(),
                collider_0: s0,
                collider_1: s1,
            },
        }
    }

    fn _get_static_collider_at(&self, point: Point2<i32>, mask: u32) -> Option<&Collider> {
        self.static_unit_colliders
            .get(&(point))
            .filter(|s| s.mask & mask != 0)
    }

    fn _probe_line(
        &self,
        position: Point2<f32>,
        dir: ProbeDir,
        max_steps: i32,
        mask: u32,
    ) -> Option<(f32, &Collider)> {
        let position_snapped = point2(position.x.floor() as i32, position.y.floor() as i32);
        let mut result = None;
        match dir {
            ProbeDir::Right => {
                for i in 0..max_steps {
                    let x = position_snapped.x + i;
                    if let Some(c) =
                        self._get_static_collider_at(point2(x, position_snapped.y), mask)
                    {
                        result = Some((c.bounds.origin.x - (position.x + 1.0), c));
                        break;
                    }
                }
            }
            ProbeDir::Up => {
                for i in 0..max_steps {
                    let y = position_snapped.y + i;
                    if let Some(c) =
                        self._get_static_collider_at(point2(position_snapped.x, y), mask)
                    {
                        result = Some((c.bounds.origin.y - (position.y + 1.0), c));
                        break;
                    }
                }
            }
            ProbeDir::Down => {
                for i in 0..max_steps {
                    let y = position_snapped.y - i;
                    if let Some(c) =
                        self._get_static_collider_at(point2(position_snapped.x, y), mask)
                    {
                        result = Some((position.y - c.top(), c));
                        break;
                    }
                }
            }
            ProbeDir::Left => {
                for i in 0..max_steps {
                    let x = position_snapped.x - i;
                    if let Some(c) =
                        self._get_static_collider_at(point2(x, position_snapped.y), mask)
                    {
                        result = Some((position.x - c.right(), c));
                        break;
                    }
                }
            }
        };

        // we only accept collisions with square shapes - because slopes are special cases handled by
        // find_character_footing only (note, the game only has northeast, and northwest slopes)
        if let Some(result) = result {
            if result.0 >= 0.0 && result.1.shape == Shape::Square {
                return Some(result);
            }
        }

        None
    }
}

#[cfg(test)]
mod space_tests {
    use super::*;

    #[test]
    fn new_produces_expected_storage() {
        let unit_0 = Collider::new(
            Bounds::new(point2(0.0, 0.0), vec2(1.0, 1.0)),
            Shape::Square,
            0,
            None,
        );

        let unit_1 = Collider::new(
            Bounds::new(point2(11.0, -33.0), vec2(1.0, 1.0)),
            Shape::Square,
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
            Shape::Square,
            square_mask,
            None,
        );

        let sb2 = Collider::new(
            Bounds::new(point2(-1.0, -1.0), vec2(1.0, 1.0)),
            Shape::Square,
            square_mask,
            None,
        );

        let tr0 = Collider::new(
            Bounds::new(point2(0.0, 4.0), vec2(1.0, 1.0)),
            Shape::NorthEast,
            triangle_mask,
            None,
        );

        let tr1 = Collider::new(
            Bounds::new(point2(-1.0, 4.0), vec2(1.0, 1.0)),
            Shape::NorthWest,
            triangle_mask,
            None,
        );

        let tr2 = Collider::new(
            Bounds::new(point2(-1.0, 3.0), vec2(1.0, 1.0)),
            Shape::SouthWest,
            triangle_mask,
            None,
        );

        let tr3 = Collider::new(
            Bounds::new(point2(0.0, 3.0), vec2(1.0, 1.0)),
            Shape::SouthEast,
            triangle_mask,
            None,
        );

        let hit_tester = Space::new(&[sb1, sb2, tr0, tr1, tr2, tr3]);

        // test triangle is hit only when using triangle_flags or all_mask
        assert!(hit_tester.test_point_first(point2(0.1, 4.1), triangle_mask) == Some(&tr0));
        assert!(hit_tester.test_point_first(point2(-0.1, 4.1), triangle_mask) == Some(&tr1));
        assert!(hit_tester.test_point_first(point2(-0.1, 3.9), triangle_mask) == Some(&tr2));
        assert!(hit_tester.test_point_first(point2(0.1, 3.9), triangle_mask) == Some(&tr3));
        assert!(hit_tester
            .test_point_first(point2(0.1, 4.1), square_mask)
            .is_none());
        assert!(hit_tester
            .test_point_first(point2(0.1, 3.9), all_mask)
            .is_some());

        // test square is only hit when mask is square or all_mask
        assert!(hit_tester.test_point_first(point2(0.5, 0.5), square_mask) == Some(&sb1));
        assert!(hit_tester
            .test_point_first(point2(0.5, 0.5), triangle_mask)
            .is_none());
        assert!(hit_tester
            .test_point_first(point2(0.5, 0.5), all_mask)
            .is_some());
    }

    fn test_points(
        collider: &Collider,
    ) -> (
        Point2<f32>,
        Point2<f32>,
        Point2<f32>,
        Point2<f32>,
        Point2<f32>,
        Point2<f32>,
        Point2<f32>,
        Point2<f32>,
    ) {
        (
            // inside
            point2(
                collider.bounds.origin.x + collider.bounds.extent.x * 0.25,
                collider.bounds.origin.y + collider.bounds.extent.y * 0.5,
            ),
            point2(
                collider.bounds.origin.x + collider.bounds.extent.x * 0.5,
                collider.bounds.origin.y + collider.bounds.extent.y * 0.25,
            ),
            point2(
                collider.bounds.origin.x + collider.bounds.extent.x * 0.75,
                collider.bounds.origin.y + collider.bounds.extent.y * 0.5,
            ),
            point2(
                collider.bounds.origin.x + collider.bounds.extent.x * 0.5,
                collider.bounds.origin.y + collider.bounds.extent.y * 0.75,
            ),
            // outside
            point2(
                collider.bounds.origin.x - collider.bounds.extent.x * 0.25,
                collider.bounds.origin.y + collider.bounds.extent.y * 0.5,
            ),
            point2(
                collider.bounds.origin.x + collider.bounds.extent.x * 0.5,
                collider.bounds.origin.y - collider.bounds.extent.y * 0.25,
            ),
            point2(
                collider.bounds.origin.x + collider.bounds.extent.x * 1.25,
                collider.bounds.origin.y + collider.bounds.extent.y * 0.5,
            ),
            point2(
                collider.bounds.origin.x + collider.bounds.extent.x * 0.5,
                collider.bounds.origin.y + collider.bounds.extent.y * 1.25,
            ),
        )
    }

    fn test_containment(mut collider: Collider) {
        let (p0, p1, p2, p3, p4, p5, p6, p7) = test_points(&collider);

        collider.shape = Shape::None;
        assert!(!collider.contains(&p0));
        assert!(!collider.contains(&p1));
        assert!(!collider.contains(&p2));
        assert!(!collider.contains(&p3));
        assert!(!collider.contains(&p4));
        assert!(!collider.contains(&p5));
        assert!(!collider.contains(&p6));
        assert!(!collider.contains(&p7));

        collider.shape = Shape::Square;
        assert!(collider.contains(&p0));
        assert!(collider.contains(&p1));
        assert!(collider.contains(&p2));
        assert!(collider.contains(&p3));
        assert!(!collider.contains(&p4));
        assert!(!collider.contains(&p5));
        assert!(!collider.contains(&p6));
        assert!(!collider.contains(&p7));

        collider.shape = Shape::NorthEast;
        assert!(collider.contains(&p0));
        assert!(collider.contains(&p1));
        assert!(!collider.contains(&p2));
        assert!(!collider.contains(&p3));
        assert!(!collider.contains(&p4));
        assert!(!collider.contains(&p5));
        assert!(!collider.contains(&p6));
        assert!(!collider.contains(&p7));

        collider.shape = Shape::SouthEast;
        assert!(collider.contains(&p0));
        assert!(!collider.contains(&p1));
        assert!(!collider.contains(&p2));
        assert!(collider.contains(&p3));
        assert!(!collider.contains(&p4));
        assert!(!collider.contains(&p5));
        assert!(!collider.contains(&p6));
        assert!(!collider.contains(&p7));

        collider.shape = Shape::SouthWest;
        assert!(!collider.contains(&p0));
        assert!(!collider.contains(&p1));
        assert!(collider.contains(&p2));
        assert!(collider.contains(&p3));
        assert!(!collider.contains(&p4));
        assert!(!collider.contains(&p5));
        assert!(!collider.contains(&p6));
        assert!(!collider.contains(&p7));

        collider.shape = Shape::NorthWest;
        assert!(!collider.contains(&p0));
        assert!(collider.contains(&p1));
        assert!(collider.contains(&p2));
        assert!(!collider.contains(&p3));
        assert!(!collider.contains(&p4));
        assert!(!collider.contains(&p5));
        assert!(!collider.contains(&p6));
        assert!(!collider.contains(&p7));
    }

    #[test]
    fn contains_works() {
        let mut collider = Collider::new(
            Bounds::new(point2(0.0, 0.0), vec2(0.0, 0.0)),
            Shape::Square,
            0,
            None,
        );

        test_containment(collider);

        // tall, NE quadrant
        collider.bounds.origin.x = 10.0;
        collider.bounds.origin.y = 5.0;
        collider.bounds.extent.y = 50.0;
        collider.bounds.extent.x = 1.0;
        test_containment(collider);

        // wide, NE quad
        collider.bounds.origin.x = 10.0;
        collider.bounds.origin.y = 5.0;
        collider.bounds.extent.y = 1.0;
        collider.bounds.extent.x = 50.0;
        test_containment(collider);

        // tall, SE quadrant
        collider.bounds.origin.x = 10.0;
        collider.bounds.origin.y = -70.0;
        collider.bounds.extent.y = 50.0;
        collider.bounds.extent.x = 1.0;
        test_containment(collider);

        // wide, SE quad
        collider.bounds.origin.x = 10.0;
        collider.bounds.origin.y = -10.0;
        collider.bounds.extent.y = 1.0;
        collider.bounds.extent.x = 50.0;
        test_containment(collider);

        // tall, SW quadrant
        collider.bounds.origin.x = -100.0;
        collider.bounds.origin.y = -500.0;
        collider.bounds.extent.y = 50.0;
        collider.bounds.extent.x = 1.0;
        test_containment(collider);

        // wide, SW quad
        collider.bounds.origin.x = -100.0;
        collider.bounds.origin.y = -500.0;
        collider.bounds.extent.y = 1.0;
        collider.bounds.extent.x = 50.0;
        test_containment(collider);

        // tall, NW quadrant
        collider.bounds.origin.x = -100.0;
        collider.bounds.origin.y = 500.0;
        collider.bounds.extent.y = 50.0;
        collider.bounds.extent.x = 1.0;
        test_containment(collider);

        // wide, NW quad
        collider.bounds.origin.x = -100.0;
        collider.bounds.origin.y = 500.0;
        collider.bounds.extent.y = 1.0;
        collider.bounds.extent.x = 50.0;
        test_containment(collider);
    }

    #[test]
    fn line_intersection_with_square_works() {
        let collider = Collider::new(
            Bounds::new(point2(0.0, 0.0), vec2(0.0, 0.0)),
            Shape::Square,
            0,
            None,
        );

        assert_eq!(
            collider.line_intersection(&point2(-0.5, 0.5), &point2(0.5, 0.5)),
            Some(point2(0.0, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, 1.5), &point2(0.5, 0.5)),
            Some(point2(0.5, 1.0))
        );
        assert_eq!(
            collider.line_intersection(&point2(1.5, 0.5), &point2(0.5, 0.5)),
            Some(point2(1.0, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, -0.5), &point2(0.5, 0.5)),
            Some(point2(0.5, 0.0))
        );
    }

    #[test]
    fn line_intersection_with_slopes_works() {
        let mut collider = Collider::new(
            Bounds::new(point2(0.0, 0.0), vec2(1.0, 1.0)),
            Shape::NorthEast,
            0,
            None,
        );

        assert_eq!(
            collider.line_intersection(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.0, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.0))
        );

        collider.shape = Shape::SouthEast;
        assert_eq!(
            collider.line_intersection(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.0, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 1.0))
        );
        assert_eq!(
            collider.line_intersection(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.5))
        );

        collider.shape = Shape::SouthWest;
        assert_eq!(
            collider.line_intersection(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 1.0))
        );
        assert_eq!(
            collider.line_intersection(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(1.0, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.5))
        );

        collider.shape = Shape::NorthWest;
        assert_eq!(
            collider.line_intersection(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(1.0, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.0))
        );
    }

    #[test]
    fn rect_intersection_works() {}
}
