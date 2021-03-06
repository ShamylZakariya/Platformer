use cgmath::*;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::{sprite::core::*, util::*};

pub mod intersection {
    use super::*;

    pub fn range_range_intersects(
        origin_a: f32,
        extent_a: f32,
        origin_b: f32,
        extent_b: f32,
    ) -> bool {
        (origin_a <= origin_b + extent_b) && (origin_a + extent_a >= origin_b)
    }

    /// Returns true if the two rectangle
    pub fn rect_rect_intersects(rect_a: Bounds, rect_b: Bounds) -> bool {
        let (x_overlap, y_overlap) = {
            (
                rect_a.origin.x <= rect_b.origin.x + rect_b.extent.x
                    && rect_a.origin.x + rect_a.extent.x >= rect_b.origin.x,
                rect_a.origin.y <= rect_b.origin.y + rect_b.extent.y
                    && rect_a.origin.y + rect_a.extent.y >= rect_b.origin.y,
            )
        };

        x_overlap && y_overlap
    }

    // https://www.swtestacademy.com/intersection-convex-polygons-algorithm/

    /// Return the intersection of two line segments, or None if they don't intersect
    pub fn line_line(
        l1p1: &Point2<f32>,
        l1p2: &Point2<f32>,
        l2p1: &Point2<f32>,
        l2p2: &Point2<f32>,
    ) -> Option<Point2<f32>> {
        let e = 1e-4_f32;
        let a1 = l1p2.y - l1p1.y;
        let b1 = l1p1.x - l1p2.x;
        let c1 = a1 * l1p1.x + b1 * l1p1.y;

        let a2 = l2p2.y - l2p1.y;
        let b2 = l2p1.x - l2p2.x;
        let c2 = a2 * l2p1.x + b2 * l2p1.y;

        let det = a1 * b2 - a2 * b1;
        if det.abs() < e {
            //parallel lines
            return None;
        } else {
            let x = (b2 * c1 - b1 * c2) / det;
            let y = (a1 * c2 - a2 * c1) / det;

            let min_x = l1p1.x.min(l1p2.x);
            let max_x = l1p1.x.max(l1p2.x);
            let min_y = l1p1.y.min(l1p2.y);
            let max_y = l1p1.y.max(l1p2.y);
            let online1 = (min_x < x || (min_x - x).abs() < e)
                && (max_x > x || (max_x - x).abs() < e)
                && (min_y < y || (min_y - y).abs() < e)
                && (max_y > y || (max_y - y).abs() < e);

            let min_x = l2p1.x.min(l2p2.x);
            let max_x = l2p1.x.max(l2p2.x);
            let min_y = l2p1.y.min(l2p2.y);
            let max_y = l2p1.y.max(l2p2.y);
            let online2 = (min_x < x || (min_x - x).abs() < e)
                && (max_x > x || (max_x - x).abs() < e)
                && (min_y < y || (min_y - y).abs() < e)
                && (max_y > y || (max_y - y).abs() < e);

            if online1 && online2 {
                return Some(point2(x, y));
            }
        }

        None
    }

    /// Return the intersection(s) of a line segment with the perimeter of a convex polygon.
    /// Winding direction is unimportant.
    pub fn line_convex_poly(
        a: &Point2<f32>,
        b: &Point2<f32>,
        convex_poly: &[Point2<f32>],
    ) -> Vec<Point2<f32>> {
        let mut intersections = vec![];
        for i in 0..convex_poly.len() {
            let next = (i + 1) % convex_poly.len();
            if let Some(p) = line_line(a, b, &convex_poly[i], &convex_poly[next]) {
                intersections.push(p);
            }
        }
        intersections
    }

    /// If the line a->b intersects the convex polygon, returns the intersection closest to a
    pub fn line_convex_poly_closest(
        a: &Point2<f32>,
        b: &Point2<f32>,
        convex_poly: &[Point2<f32>],
    ) -> Option<Point2<f32>> {
        let mut intersections = vec![];
        for i in 0..convex_poly.len() {
            let next = (i + 1) % convex_poly.len();
            if let Some(p) = line_line(a, b, &convex_poly[i], &convex_poly[next]) {
                intersections.push(p);
            }
        }
        intersections.sort_by(|m, n| {
            let m_a = m.distance2(*a);
            let n_a = n.distance2(*a);
            m_a.partial_cmp(&n_a).unwrap()
        });
        if let Some(p) = intersections.first() {
            Some(*p)
        } else {
            None
        }
    }

    #[cfg(test)]
    mod intersection_tests {
        use super::*;

        #[test]
        fn rect_rect_intersects_works() {
            let a = Bounds::new(point2(1.0, 1.0), vec2(1.0, 1.0));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(0.5, 0.0), vec2(1.0, 1.0))
            ));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(1.0, 0.0), vec2(1.0, 1.0))
            ));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(1.0, 0.5), vec2(1.0, 1.0))
            ));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(1.0, 1.0), vec2(1.0, 1.0))
            ));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(0.5, 1.0), vec2(1.0, 1.0))
            ));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(0.0, 1.0), vec2(1.0, 1.0))
            ));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(0.5, 1.0), vec2(1.0, 1.0))
            ));

            assert!(!rect_rect_intersects(
                a,
                Bounds::new(point2(3.0, 0.0), vec2(1.0, 1.0))
            ));
            assert!(!rect_rect_intersects(
                a,
                Bounds::new(point2(0.0, 3.0), vec2(1.0, 1.0))
            ));
            assert!(!rect_rect_intersects(
                a,
                Bounds::new(point2(-2.0, 0.0), vec2(1.0, 1.0))
            ));
            assert!(!rect_rect_intersects(
                a,
                Bounds::new(point2(-2.0, -2.0), vec2(1.0, 1.0))
            ));
        }

        #[test]
        fn line_line_works() {
            assert_eq!(
                line_line(
                    &point2(0.0, 0.0),
                    &point2(10.0, 0.0),
                    &point2(2.0, 1.0),
                    &point2(2.0, -1.0),
                ),
                Some(point2(2.0, 0.0))
            );

            assert_eq!(
                line_line(
                    &point2(0.0, 0.0),
                    &point2(10.0, 10.0),
                    &point2(5.0, 10.0),
                    &point2(5.0, 0.0),
                ),
                Some(point2(5.0, 5.0))
            );

            assert_eq!(
                line_line(
                    &point2(0.0, 0.0),
                    &point2(10.0, 10.0),
                    &point2(0.0, 1.0),
                    &point2(10.0, 11.0),
                ),
                None
            );
        }

        #[test]
        fn line_convex_poly_works() {
            let square = vec![
                point2(0.0, 0.0),
                point2(1.0, 0.0),
                point2(1.0, 1.0),
                point2(0.0, 1.0),
            ];

            assert_eq!(
                line_convex_poly(&point2(-1.0, 0.5), &point2(0.5, 0.5), &square),
                vec![point2(0.0, 0.5)]
            );
            assert_eq!(
                line_convex_poly(&point2(2.0, 0.5), &point2(0.5, 0.5), &square),
                vec![point2(1.0, 0.5)]
            );
            assert_eq!(
                line_convex_poly(&point2(0.5, 2.0), &point2(0.5, 0.5), &square),
                vec![point2(0.5, 1.0)]
            );
            assert_eq!(
                line_convex_poly(&point2(0.5, -1.0), &point2(0.5, 0.5), &square),
                vec![point2(0.5, 0.0)]
            );

            let triangle = vec![point2(0.0, 0.0), point2(1.0, 0.0), point2(0.0, 1.0)];

            assert_eq!(
                line_convex_poly(&point2(0.5, 1.0), &point2(0.5, 0.01), &triangle),
                vec![point2(0.5, 0.5)]
            );
        }
    }
}

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
pub enum Mode {
    Static { position: Point2<i32> },
    Dynamic { bounds: Bounds, entity_id: u32 },
}

impl Mode {
    pub fn bounds(&self) -> Bounds {
        match self {
            Mode::Static { position } => {
                Bounds::new(point2(position.x as f32, position.y as f32), vec2(1.0, 1.0))
            }
            Mode::Dynamic { bounds, .. } => *bounds,
        }
    }
}

impl PartialEq for Mode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Mode::Static { position: p1 }, Mode::Static { position: p2 }) => p1 == p2,

            (
                Mode::Dynamic {
                    bounds: b1,
                    entity_id: eid1,
                },
                Mode::Dynamic {
                    bounds: b2,
                    entity_id: eid2,
                },
            ) => b1.eq(b2) && eid1 == eid2,

            _ => false,
        }
    }
}

impl Hash for Mode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Mode::Static { position } => position.hash(state),
            Mode::Dynamic { bounds, entity_id } => {
                hash_point2(&bounds.origin, state);
                hash_vec2(&bounds.extent, state);
                entity_id.hash(state);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Collider {
    pub mode: Mode,
    pub shape: Shape,
    pub mask: u32,
}

impl PartialEq for Collider {
    fn eq(&self, other: &Self) -> bool {
        self.mode == other.mode && self.shape == other.shape && self.mask == other.mask
    }
}

impl Eq for Collider {}

impl Hash for Collider {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.mode.hash(state);
        self.shape.hash(state);
        self.mask.hash(state);
    }
}

impl Collider {
    pub fn new_static(position: Point2<i32>, shape: Shape, mask: u32) -> Self {
        Self {
            mode: Mode::Static { position },
            shape,
            mask,
        }
    }

    pub fn new_dynamic(bounds: Bounds, entity_id: u32, shape: Shape, mask: u32) -> Self {
        Self {
            mode: Mode::Dynamic { bounds, entity_id },
            shape,
            mask,
        }
    }

    pub fn from_static_sprite(sprite: &Sprite) -> Self {
        Self {
            mode: Mode::Static {
                position: point2(
                    sprite.origin.x.floor() as i32,
                    sprite.origin.y.floor() as i32,
                ),
            },
            shape: sprite.collision_shape,
            mask: sprite.mask,
        }
    }

    pub fn from_dynamic_sprite(sprite: &Sprite) -> Self {
        Self {
            mode: Mode::Dynamic {
                bounds: Bounds::new(sprite.origin.xy(), sprite.extent),
                entity_id: sprite.entity_id.expect(
                    "Collider::from_dynamic_sprite requires the sprite to have an entity_id",
                ),
            },
            shape: sprite.collision_shape,
            mask: sprite.mask,
        }
    }

    pub fn contains_point(&self, point: &Point2<f32>) -> bool {
        let bounds = self.mode.bounds();
        if point.x >= bounds.origin.x
            && point.x <= bounds.origin.x + bounds.extent.x
            && point.y >= bounds.origin.y
            && point.y <= bounds.origin.y + bounds.extent.y
        {
            let p = vec2(point.x, point.y);
            return match self.shape {
                Shape::None => false,

                Shape::Square => true,

                Shape::NorthEast => {
                    let a = vec2(bounds.origin.x, bounds.origin.y + bounds.extent.y);
                    let b = vec2(bounds.origin.x + bounds.extent.x, bounds.origin.y);
                    let ba = b - a;
                    let pa = p - a;
                    cross(&ba, &pa) <= 0.0
                }

                Shape::SouthEast => {
                    let a = vec2(bounds.origin.x, bounds.origin.y);
                    let b = vec2(
                        bounds.origin.x + bounds.extent.x,
                        bounds.origin.y + bounds.extent.y,
                    );
                    let ba = b - a;
                    let pa = p - a;
                    cross(&ba, &pa) >= 0.0
                }

                Shape::SouthWest => {
                    let a = vec2(bounds.origin.x, bounds.origin.y + bounds.extent.y);
                    let b = vec2(bounds.origin.x + bounds.extent.x, bounds.origin.y);
                    let ba = b - a;
                    let pa = p - a;
                    // opposite winding of northeast
                    cross(&ba, &pa) >= 0.0
                }

                Shape::NorthWest => {
                    let a = vec2(bounds.origin.x, bounds.origin.y);
                    let b = vec2(
                        bounds.origin.x + bounds.extent.x,
                        bounds.origin.y + bounds.extent.y,
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
    pub fn intersects_line(&self, a: &Point2<f32>, b: &Point2<f32>) -> Option<Point2<f32>> {
        let bounds = self.mode.bounds();
        match self.shape {
            Shape::None => None,
            Shape::Square => intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(bounds.origin.x, bounds.origin.y),
                    point2(bounds.origin.x + bounds.extent.x, bounds.origin.y),
                    point2(
                        bounds.origin.x + bounds.extent.x,
                        bounds.origin.y + bounds.extent.y,
                    ),
                    point2(bounds.origin.x, bounds.origin.y + bounds.extent.y),
                ],
            ),
            Shape::NorthEast => intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(bounds.origin.x, bounds.origin.y),
                    point2(bounds.origin.x + bounds.extent.x, bounds.origin.y),
                    point2(bounds.origin.x, bounds.origin.y + bounds.extent.y),
                ],
            ),
            Shape::SouthEast => intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(bounds.origin.x, bounds.origin.y),
                    point2(
                        bounds.origin.x + bounds.extent.x,
                        bounds.origin.y + bounds.extent.y,
                    ),
                    point2(bounds.origin.x, bounds.origin.y + bounds.extent.y),
                ],
            ),
            Shape::SouthWest => intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(bounds.origin.x + bounds.extent.x, bounds.origin.y),
                    point2(
                        bounds.origin.x + bounds.extent.x,
                        bounds.origin.y + bounds.extent.y,
                    ),
                    point2(bounds.origin.x, bounds.origin.y + bounds.extent.y),
                ],
            ),
            Shape::NorthWest => intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(bounds.origin.x, bounds.origin.y),
                    point2(bounds.origin.x + bounds.extent.x, bounds.origin.y),
                    point2(
                        bounds.origin.x + bounds.extent.x,
                        bounds.origin.y + bounds.extent.y,
                    ),
                ],
            ),
        }
    }

    /// Returns true if this Sprite overlaps the described rect with lower/left origin and extent and inset.
    /// Inset: The amount to inset the test rect
    /// contact: If true, contacts will also count as an intersection, not just overlap. In this case rects with touching edges will be treated as intersections.
    pub fn intersects_rect(
        &self,
        origin: &Point2<f32>,
        extent: &Vector2<f32>,
        inset: f32,
        contact: bool,
    ) -> bool {
        let bounds = self.mode.bounds();
        let origin = point2(origin.x + inset, origin.y + inset);
        let extent = vec2(extent.x - 2.0 * inset, extent.y - 2.0 * inset);

        let (x_overlap, y_overlap) = if contact {
            (
                bounds.origin.x <= origin.x + extent.x
                    && bounds.origin.x + bounds.extent.x >= origin.x,
                bounds.origin.y <= origin.y + extent.y
                    && bounds.origin.y + bounds.extent.y >= origin.y,
            )
        } else {
            (
                bounds.origin.x < origin.x + extent.x
                    && bounds.origin.x + bounds.extent.x > origin.x,
                bounds.origin.y < origin.y + extent.y
                    && bounds.origin.y + bounds.extent.y > origin.y,
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
    pub fn intersects_unit_rect(&self, origin: &Point2<f32>, inset: f32, contact: bool) -> bool {
        self.intersects_rect(origin, &vec2(1.0, 1.0), inset, contact)
    }

    pub fn bounds(&self) -> Bounds {
        self.mode.bounds()
    }

    pub fn origin(&self) -> Point2<f32> {
        match self.mode {
            Mode::Static { position } => point2(position.x as f32, position.y as f32),
            Mode::Dynamic { bounds, .. } => bounds.origin,
        }
    }

    fn set_origin(&mut self, new_origin: Point2<f32>) {
        match &mut self.mode {
            Mode::Static { position } => {
                *position = point2(new_origin.x.floor() as i32, new_origin.y.floor() as i32)
            }
            Mode::Dynamic { bounds, .. } => bounds.origin = new_origin,
        }
    }

    pub fn extent(&self) -> Vector2<f32> {
        match self.mode {
            Mode::Static { .. } => vec2(1.0, 1.0),
            Mode::Dynamic { bounds, .. } => bounds.extent,
        }
    }

    fn set_extent(&mut self, new_extent: Vector2<f32>) {
        match &mut self.mode {
            Mode::Static { .. } => panic!("Can't change extent of a static collider"),
            Mode::Dynamic { bounds, .. } => bounds.extent = new_extent,
        }
    }

    pub fn left(&self) -> f32 {
        match self.mode {
            Mode::Static { position } => position.x as f32,
            Mode::Dynamic { bounds, .. } => bounds.origin.x,
        }
    }

    pub fn right(&self) -> f32 {
        match self.mode {
            Mode::Static { position } => (position.x + 1) as f32,
            Mode::Dynamic { bounds, .. } => bounds.right(),
        }
    }

    pub fn bottom(&self) -> f32 {
        match self.mode {
            Mode::Static { position } => position.y as f32,
            Mode::Dynamic { bounds, .. } => bounds.origin.y,
        }
    }

    pub fn top(&self) -> f32 {
        match self.mode {
            Mode::Static { position } => (position.y + 1) as f32,
            Mode::Dynamic { bounds, .. } => bounds.top(),
        }
    }

    pub fn width(&self) -> f32 {
        match self.mode {
            Mode::Static { .. } => 1.0,
            Mode::Dynamic { bounds, .. } => bounds.width(),
        }
    }

    pub fn height(&self) -> f32 {
        match self.mode {
            Mode::Static { .. } => 1.0,
            Mode::Dynamic { bounds, .. } => bounds.height(),
        }
    }

    pub fn entity_id(&self) -> Option<u32> {
        match self.mode {
            Mode::Static { .. } => None,
            Mode::Dynamic { entity_id, .. } => Some(entity_id),
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Sentinel {
    Continue,
    Stop,
}

pub struct Space {
    colliders: Vec<Collider>,
    active_colliders: HashSet<u32>,
    static_colliders: HashMap<Point2<i32>, usize>,
    dynamic_colliders: Vec<usize>,
    dynamic_colliders_need_sort: bool,
}

impl Space {
    pub fn new(colliders: &[Collider]) -> Self {
        let mut space = Self {
            colliders: Vec::new(),
            active_colliders: HashSet::new(),
            static_colliders: HashMap::new(),
            dynamic_colliders: Vec::new(),
            dynamic_colliders_need_sort: false,
        };

        for c in colliders {
            space.add_collider(*c);
        }

        space
    }

    pub fn update(&mut self) {
        if self.dynamic_colliders_need_sort {
            let colliders = std::mem::replace(&mut self.colliders, vec![]);

            self.dynamic_colliders.sort_by(|a, b| {
                let c_a = &colliders[*a as usize];
                let c_b = &colliders[*b as usize];
                c_a.left().partial_cmp(&c_b.left()).unwrap()
            });

            let _ = std::mem::replace(&mut self.colliders, colliders);

            self.dynamic_colliders_need_sort = false
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    pub fn add_collider(&mut self, collider: Collider) -> u32 {
        let index = self.colliders.len();
        self.colliders.push(collider);
        self.active_colliders.insert(index as u32);

        match collider.mode {
            Mode::Static { position } => {
                self.static_colliders.insert(position, index);
            }
            Mode::Dynamic { .. } => {
                self.dynamic_colliders.push(index);
                self.dynamic_colliders_need_sort = true;
            }
        }

        index as u32
    }

    pub fn get_collider(&self, collider_id: u32) -> Option<&Collider> {
        self.colliders.get(collider_id as usize)
    }

    pub fn deactivate_collider(&mut self, collider_id: u32) {
        self.active_colliders.remove(&collider_id);
        if let Some(c) = self.colliders.get_mut(collider_id as usize) {
            match c.mode {
                Mode::Static { position } => {
                    self.static_colliders.remove(&position);
                }
                Mode::Dynamic { .. } => {
                    // remove; note we don't need to re-sort when
                    // removing an item from an already sorted list.
                    self.dynamic_colliders
                        .retain(|id| *id != collider_id as usize);
                }
            };
        }
    }

    pub fn activate_collider(&mut self, collider_id: u32) {
        if let Some(c) = self.colliders.get_mut(collider_id as usize) {
            self.active_colliders.insert(collider_id);
            match c.mode {
                Mode::Static { position } => {
                    self.static_colliders.insert(position, collider_id as usize);
                }
                Mode::Dynamic { .. } => {
                    self.dynamic_colliders.push(collider_id as usize);
                    self.dynamic_colliders_need_sort = true;
                }
            }
        }
    }

    pub fn is_collider_activated(&self, collider_id: u32) -> bool {
        self.active_colliders.contains(&collider_id)
    }

    pub fn update_collider_position(&mut self, collider_id: u32, new_position: Point2<f32>) {
        if let Some(c) = self.colliders.get_mut(collider_id as usize) {
            match &mut c.mode {
                Mode::Static { position } => {
                    // remove the entry from the old position, update the position, and re-insert
                    self.static_colliders.remove(position);
                    *position =
                        point2(new_position.x.floor() as i32, new_position.y.floor() as i32);
                    self.static_colliders
                        .insert(*position, collider_id as usize);
                }
                Mode::Dynamic { bounds, .. } => {
                    bounds.origin = new_position;
                    self.dynamic_colliders_need_sort = true;
                }
            }
        }
    }

    pub fn update_collider_extent(&mut self, collider_id: u32, new_extent: Vector2<f32>) {
        if let Some(c) = self.colliders.get_mut(collider_id as usize) {
            match &mut c.mode {
                Mode::Static { .. } => panic!("Cannot change size of a static collider"),
                Mode::Dynamic { bounds, .. } => {
                    // we don't need to re-sort list when changing a colider's extent
                    bounds.extent = new_extent;
                }
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    pub fn get_collider_at(&self, point: Point2<i32>, mask: u32) -> Option<&Collider> {
        let point_f = point2(point.x as f32 + 0.5, point.y as f32 + 0.5);
        let found = self.get_first_dynamic_collider_containing_point(&point_f, mask);
        if found.is_some() {
            return found;
        }

        if let Some(id) = self.static_colliders.get(&point) {
            let c = &self.colliders[*id as usize];
            if c.mask & mask != 0 && c.contains_point(&point_f) {
                return Some(c);
            }
        }
        None
    }

    fn get_first_dynamic_collider_intersecting_rect(
        &self,
        origin: &Point2<f32>,
        extent: &Vector2<f32>,
        mask: u32,
    ) -> Option<&Collider> {
        let mut perform_test = false;
        for idx in self.dynamic_colliders.iter() {
            let collider = &self.colliders[*idx];
            let left = collider.left();
            let right = collider.right();

            if intersection::range_range_intersects(left, right - left, origin.x, extent.x) {
                perform_test = true;
            } else if left > origin.x + extent.x {
                // we're done
                break;
            }

            if perform_test
                && collider.mask & mask != 0
                && collider.intersects_rect(origin, extent, 0.0, true)
            {
                return Some(collider);
            }
        }
        None
    }

    fn get_first_dynamic_collider_containing_point(
        &self,
        point: &Point2<f32>,
        mask: u32,
    ) -> Option<&Collider> {
        let mut perform_test = false;
        for idx in self.dynamic_colliders.iter() {
            let collider = &self.colliders[*idx];
            let left = collider.left();
            let right = collider.right();

            if left <= point.x && right >= point.x {
                perform_test = true;
            } else if left > point.x {
                // we're done
                break;
            }

            if perform_test && collider.mask & mask != 0 && collider.contains_point(point) {
                return Some(collider);
            }
        }
        None
    }

    fn get_dynamic_colliders_intersecting_rect<C>(
        &self,
        origin: &Point2<f32>,
        extent: &Vector2<f32>,
        mask: u32,
        mut cb: C,
    ) where
        C: FnMut(&Collider) -> Sentinel,
    {
        if self.dynamic_colliders.is_empty() {
            return;
        }

        // find first collider whos x-range overlaps point, and scan the following
        // until we leave possibility of collision. TODO: Use a binary search to
        // speed up the initial search.

        let mut perform_test = false;
        for idx in self.dynamic_colliders.iter() {
            let collider = &self.colliders[*idx];
            let left = collider.left();
            let right = collider.right();

            if intersection::range_range_intersects(left, right - left, origin.x, extent.x) {
                perform_test = true;
            } else if left > origin.x + extent.x {
                // we're done
                break;
            }

            if perform_test
                && collider.mask & mask != 0
                && collider.intersects_rect(origin, extent, 0.0, true)
                && matches!(cb(collider), Sentinel::Stop)
            {
                return;
            }
        }
    }

    fn get_dynamic_colliders_intersecting_point<C>(&self, point: &Point2<f32>, mask: u32, mut cb: C)
    where
        C: FnMut(&Collider) -> Sentinel,
    {
        if self.dynamic_colliders.is_empty() {
            return;
        }

        // find first collider whos x-range overlaps point, and scan the following
        // until we leave possibility of collision. TODO: Use a binary search to
        // speed up the initial search.
        let mut perform_test = false;
        for idx in self.dynamic_colliders.iter() {
            let collider = &self.colliders[*idx];
            let left = collider.left();
            let right = collider.right();
            if left <= point.x && right >= point.x {
                perform_test = true;
            } else if left > point.x {
                // we're done
                break;
            }

            if perform_test
                && collider.mask & mask != 0
                && collider.contains_point(point)
                && matches!(cb(collider), Sentinel::Stop)
            {
                return;
            }
        }
    }

    fn get_static_collider_at(&self, point: Point2<i32>, mask: u32) -> Option<&Collider> {
        let point_f = point2(point.x as f32 + 0.5, point.y as f32 + 0.5);
        if let Some(id) = self.static_colliders.get(&point) {
            let c = &self.colliders[*id as usize];
            if c.mask & mask != 0 && c.contains_point(&point_f) {
                return Some(c);
            }
        }
        None
    }

    /// Tests the specified rect against colliders, invoking the specified callback for each contact/overlap.
    /// Filters by mask, such that only sprites with matching mask bits will be matched.
    /// In the case of overlapping sprites, dynamic sprites will be passed to the callback before static.
    /// The callback returns Sentinel::Continue to continue search, or Sentinel::Stop to terminate search.
    pub fn test_rect<C>(
        &self,
        origin: &Point2<f32>,
        extent: &Vector2<f32>,
        mask: u32,
        mut callback: C,
    ) where
        C: FnMut(&Collider) -> Sentinel,
    {
        let mut early_exit = false;
        self.get_dynamic_colliders_intersecting_rect(origin, extent, mask, |c| {
            let s = callback(c);
            if s == Sentinel::Stop {
                early_exit = true;
            }
            s
        });

        if early_exit {
            return;
        }

        let snapped_extent = vec2(extent.x.round() as i32, extent.y.round() as i32);
        let a = point2(origin.x.floor() as i32, origin.y.floor() as i32);
        let b = point2(a.x + snapped_extent.x, a.y);
        let c = point2(a.x, a.y + snapped_extent.y);
        let d = point2(a.x + snapped_extent.x, a.y + snapped_extent.y);

        for p in [a, b, c, d].iter() {
            if let Some(c) = self.get_static_collider_at(*p, mask) {
                if c.intersects_rect(origin, extent, 0.0, true)
                    && matches!(callback(c), Sentinel::Stop)
                {
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
        let found = self.get_first_dynamic_collider_intersecting_rect(origin, extent, mask);
        if found.is_some() {
            return found;
        }

        let snapped_extent = vec2(extent.x.round() as i32, extent.y.round() as i32);
        let a = point2(origin.x.floor() as i32, origin.y.floor() as i32);
        let b = point2(a.x + snapped_extent.x, a.y);
        let c = point2(a.x, a.y + snapped_extent.y);
        let d = point2(a.x + snapped_extent.x, a.y + snapped_extent.y);

        for p in [a, b, c, d].iter() {
            if let Some(c) = self.get_static_collider_at(*p, mask) {
                if c.intersects_rect(origin, extent, 0.0, true) {
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
    pub fn test_point_first(&self, point: &Point2<f32>, mask: u32) -> Option<&Collider> {
        let found = self.get_first_dynamic_collider_containing_point(point, mask);
        if found.is_some() {
            return found;
        }

        if let Some(id) = self
            .static_colliders
            .get(&point2(point.x.floor() as i32, point.y.floor() as i32))
        {
            let c = &self.colliders[*id as usize];
            if c.mask & mask != 0 && c.contains_point(&point) {
                return Some(c);
            }
        }

        None
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
                        self.get_static_collider_at(point2(x, position_snapped.y), mask)
                    {
                        result = Some((c.left() - (position.x + 1.0), c));
                        break;
                    }
                }
            }
            ProbeDir::Up => {
                for i in 0..max_steps {
                    let y = position_snapped.y + i;
                    if let Some(c) =
                        self.get_static_collider_at(point2(position_snapped.x, y), mask)
                    {
                        result = Some((c.bottom() - (position.y + 1.0), c));
                        break;
                    }
                }
            }
            ProbeDir::Down => {
                for i in 0..max_steps {
                    let y = position_snapped.y - i;
                    if let Some(c) =
                        self.get_static_collider_at(point2(position_snapped.x, y), mask)
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
                        self.get_static_collider_at(point2(x, position_snapped.y), mask)
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
    fn unit_sprite_hit_test_works() {
        let square_mask = 1 << 0;
        let triangle_mask = 1 << 1;
        let all_mask = square_mask | triangle_mask;

        let sb1 = Collider::new_static((0, 0).into(), Shape::Square, square_mask);
        let sb2 = Collider::new_static((-1, -1).into(), Shape::Square, square_mask);
        let tr0 = Collider::new_static((0, 4).into(), Shape::NorthEast, triangle_mask);
        let tr1 = Collider::new_static((-1, 4).into(), Shape::NorthWest, triangle_mask);
        let tr2 = Collider::new_static((-1, 3).into(), Shape::SouthWest, triangle_mask);
        let tr3 = Collider::new_static((0, 3).into(), Shape::SouthEast, triangle_mask);

        let hit_tester = Space::new(&[sb1, sb2, tr0, tr1, tr2, tr3]);

        // test triangle is hit only when using triangle_flags or all_mask
        assert!(hit_tester.test_point_first(&point2(0.1, 4.1), triangle_mask) == Some(&tr0));
        assert!(hit_tester.test_point_first(&point2(-0.1, 4.1), triangle_mask) == Some(&tr1));
        assert!(hit_tester.test_point_first(&point2(-0.1, 3.9), triangle_mask) == Some(&tr2));
        assert!(hit_tester.test_point_first(&point2(0.1, 3.9), triangle_mask) == Some(&tr3));
        assert!(hit_tester
            .test_point_first(&point2(0.1, 4.1), square_mask)
            .is_none());
        assert!(hit_tester
            .test_point_first(&point2(0.1, 3.9), all_mask)
            .is_some());

        // test square is only hit when mask is square or all_mask
        assert!(hit_tester.test_point_first(&point2(0.5, 0.5), square_mask) == Some(&sb1));
        assert!(hit_tester
            .test_point_first(&point2(0.5, 0.5), triangle_mask)
            .is_none());
        assert!(hit_tester
            .test_point_first(&point2(0.5, 0.5), all_mask)
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
        let bounds = collider.bounds();
        (
            // inside
            point2(
                bounds.origin.x + bounds.extent.x * 0.25,
                bounds.origin.y + bounds.extent.y * 0.5,
            ),
            point2(
                bounds.origin.x + bounds.extent.x * 0.5,
                bounds.origin.y + bounds.extent.y * 0.25,
            ),
            point2(
                bounds.origin.x + bounds.extent.x * 0.75,
                bounds.origin.y + bounds.extent.y * 0.5,
            ),
            point2(
                bounds.origin.x + bounds.extent.x * 0.5,
                bounds.origin.y + bounds.extent.y * 0.75,
            ),
            // outside
            point2(
                bounds.origin.x - bounds.extent.x * 0.25,
                bounds.origin.y + bounds.extent.y * 0.5,
            ),
            point2(
                bounds.origin.x + bounds.extent.x * 0.5,
                bounds.origin.y - bounds.extent.y * 0.25,
            ),
            point2(
                bounds.origin.x + bounds.extent.x * 1.25,
                bounds.origin.y + bounds.extent.y * 0.5,
            ),
            point2(
                bounds.origin.x + bounds.extent.x * 0.5,
                bounds.origin.y + bounds.extent.y * 1.25,
            ),
        )
    }

    fn test_containment(mut collider: Collider) {
        let (p0, p1, p2, p3, p4, p5, p6, p7) = test_points(&collider);

        collider.shape = Shape::None;
        assert!(!collider.contains_point(&p0));
        assert!(!collider.contains_point(&p1));
        assert!(!collider.contains_point(&p2));
        assert!(!collider.contains_point(&p3));
        assert!(!collider.contains_point(&p4));
        assert!(!collider.contains_point(&p5));
        assert!(!collider.contains_point(&p6));
        assert!(!collider.contains_point(&p7));

        collider.shape = Shape::Square;
        assert!(collider.contains_point(&p0));
        assert!(collider.contains_point(&p1));
        assert!(collider.contains_point(&p2));
        assert!(collider.contains_point(&p3));
        assert!(!collider.contains_point(&p4));
        assert!(!collider.contains_point(&p5));
        assert!(!collider.contains_point(&p6));
        assert!(!collider.contains_point(&p7));

        collider.shape = Shape::NorthEast;
        assert!(collider.contains_point(&p0));
        assert!(collider.contains_point(&p1));
        assert!(!collider.contains_point(&p2));
        assert!(!collider.contains_point(&p3));
        assert!(!collider.contains_point(&p4));
        assert!(!collider.contains_point(&p5));
        assert!(!collider.contains_point(&p6));
        assert!(!collider.contains_point(&p7));

        collider.shape = Shape::SouthEast;
        assert!(collider.contains_point(&p0));
        assert!(!collider.contains_point(&p1));
        assert!(!collider.contains_point(&p2));
        assert!(collider.contains_point(&p3));
        assert!(!collider.contains_point(&p4));
        assert!(!collider.contains_point(&p5));
        assert!(!collider.contains_point(&p6));
        assert!(!collider.contains_point(&p7));

        collider.shape = Shape::SouthWest;
        assert!(!collider.contains_point(&p0));
        assert!(!collider.contains_point(&p1));
        assert!(collider.contains_point(&p2));
        assert!(collider.contains_point(&p3));
        assert!(!collider.contains_point(&p4));
        assert!(!collider.contains_point(&p5));
        assert!(!collider.contains_point(&p6));
        assert!(!collider.contains_point(&p7));

        collider.shape = Shape::NorthWest;
        assert!(!collider.contains_point(&p0));
        assert!(collider.contains_point(&p1));
        assert!(collider.contains_point(&p2));
        assert!(!collider.contains_point(&p3));
        assert!(!collider.contains_point(&p4));
        assert!(!collider.contains_point(&p5));
        assert!(!collider.contains_point(&p6));
        assert!(!collider.contains_point(&p7));
    }

    #[test]
    fn contains_works() {
        let collider =
            |bounds: Bounds| -> Collider { Collider::new_dynamic(bounds, 0, Shape::Square, 0) };

        let mut bounds = Bounds::default();

        // tall, NE quadrant
        bounds.origin.x = 10.0;
        bounds.origin.y = 5.0;
        bounds.extent.y = 50.0;
        bounds.extent.x = 1.0;
        test_containment(collider(bounds));

        // wide, NE quad
        bounds.origin.x = 10.0;
        bounds.origin.y = 5.0;
        bounds.extent.y = 1.0;
        bounds.extent.x = 50.0;
        test_containment(collider(bounds));

        // tall, SE quadrant
        bounds.origin.x = 10.0;
        bounds.origin.y = -70.0;
        bounds.extent.y = 50.0;
        bounds.extent.x = 1.0;
        test_containment(collider(bounds));

        // wide, SE quad
        bounds.origin.x = 10.0;
        bounds.origin.y = -10.0;
        bounds.extent.y = 1.0;
        bounds.extent.x = 50.0;
        test_containment(collider(bounds));

        // tall, SW quadrant
        bounds.origin.x = -100.0;
        bounds.origin.y = -500.0;
        bounds.extent.y = 50.0;
        bounds.extent.x = 1.0;
        test_containment(collider(bounds));

        // wide, SW quad
        bounds.origin.x = -100.0;
        bounds.origin.y = -500.0;
        bounds.extent.y = 1.0;
        bounds.extent.x = 50.0;
        test_containment(collider(bounds));

        // tall, NW quadrant
        bounds.origin.x = -100.0;
        bounds.origin.y = 500.0;
        bounds.extent.y = 50.0;
        bounds.extent.x = 1.0;
        test_containment(collider(bounds));

        // wide, NW quad
        bounds.origin.x = -100.0;
        bounds.origin.y = 500.0;
        bounds.extent.y = 1.0;
        bounds.extent.x = 50.0;
        test_containment(collider(bounds));
    }

    #[test]
    fn line_intersection_with_square_works() {
        let collider = Collider::new_static((0, 0).into(), Shape::Square, 0);

        assert_eq!(
            collider.intersects_line(&point2(-0.5, 0.5), &point2(0.5, 0.5)),
            Some(point2(0.0, 0.5))
        );
        assert_eq!(
            collider.intersects_line(&point2(0.5, 1.5), &point2(0.5, 0.5)),
            Some(point2(0.5, 1.0))
        );
        assert_eq!(
            collider.intersects_line(&point2(1.5, 0.5), &point2(0.5, 0.5)),
            Some(point2(1.0, 0.5))
        );
        assert_eq!(
            collider.intersects_line(&point2(0.5, -0.5), &point2(0.5, 0.5)),
            Some(point2(0.5, 0.0))
        );
    }

    #[test]
    fn line_intersection_with_slopes_works() {
        let mut collider = Collider::new_static((0, 0).into(), Shape::NorthEast, 0);

        assert_eq!(
            collider.intersects_line(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.0, 0.5))
        );
        assert_eq!(
            collider.intersects_line(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.intersects_line(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.intersects_line(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.0))
        );

        collider.shape = Shape::SouthEast;
        assert_eq!(
            collider.intersects_line(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.0, 0.5))
        );
        assert_eq!(
            collider.intersects_line(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 1.0))
        );
        assert_eq!(
            collider.intersects_line(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.intersects_line(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.5))
        );

        collider.shape = Shape::SouthWest;
        assert_eq!(
            collider.intersects_line(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.intersects_line(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 1.0))
        );
        assert_eq!(
            collider.intersects_line(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(1.0, 0.5))
        );
        assert_eq!(
            collider.intersects_line(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.5))
        );

        collider.shape = Shape::NorthWest;
        assert_eq!(
            collider.intersects_line(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.intersects_line(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.intersects_line(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(1.0, 0.5))
        );
        assert_eq!(
            collider.intersects_line(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.0))
        );
    }

    #[test]
    fn rect_intersection_works() {}

    #[test]
    fn get_dynamic_colliders_lookup_works() {
        let mask = 1;
        let colliders = [
            Collider::new_dynamic(
                Bounds::new(point2(2.0, 1.0), vec2(2.0, 2.0)),
                0,
                Shape::Square,
                mask,
            ),
            Collider::new_dynamic(
                Bounds::new(point2(3.0, 2.0), vec2(2.0, 3.0)),
                1,
                Shape::Square,
                mask,
            ),
            Collider::new_dynamic(
                Bounds::new(point2(8.0, 3.0), vec2(1.0, 1.0)),
                2,
                Shape::Square,
                mask,
            ),
            Collider::new_dynamic(
                Bounds::new(point2(16.0, 3.0), vec2(2.0, 2.0)),
                3,
                Shape::Square,
                mask,
            ),
            Collider::new_dynamic(
                Bounds::new(point2(17.0, 7.0), vec2(1.0, 1.0)),
                4,
                Shape::Square,
                mask,
            ),
            Collider::new_dynamic(
                Bounds::new(point2(22.0, 2.0), vec2(3.0, 3.0)),
                5,
                Shape::Square,
                mask,
            ),
            Collider::new_dynamic(
                Bounds::new(point2(24.0, 4.0), vec2(2.0, 2.0)),
                6,
                Shape::Square,
                mask,
            ),
            Collider::new_dynamic(
                Bounds::new(point2(32.0, 3.0), vec2(1.0, 1.0)),
                7,
                Shape::Square,
                mask,
            ),
        ];
        let space = Space::new(&colliders);

        //
        //  Test points
        //

        let test_point = |point: Point2<f32>, expected_ids: &[u32]| {
            let mut found: HashSet<u32> = HashSet::new();
            space.get_dynamic_colliders_intersecting_point(&point, mask, |c| {
                found.insert(c.entity_id().unwrap());
                Sentinel::Continue
            });
            assert_eq!(found.len(), expected_ids.len());
            for id in expected_ids.iter() {
                assert!(found.contains(id));
            }
        };

        // confirm that we get nothing for a non-hit
        test_point(point2(0.0, 0.0), &[]);
        test_point(point2(12.0, 1.0), &[]);

        test_point(point2(2.5, 1.5), &[0]);
        test_point(point2(4.5, 4.0), &[1]);
        test_point(point2(3.5, 2.5), &[0, 1]);
        test_point(point2(8.5, 3.5), &[2]);
        test_point(point2(16.5, 3.5), &[3]);
        test_point(point2(17.5, 7.5), &[4]);
        test_point(point2(17.5, 6.5), &[]); // miss
        test_point(point2(22.5, 2.5), &[5]);
        test_point(point2(24.5, 4.5), &[5, 6]);
        test_point(point2(25.5, 5.5), &[6]);
        test_point(point2(32.5, 3.5), &[7]);
        test_point(point2(32.5, 1.5), &[]); // miss

        //
        //  test rects
        //

        let test_rect = |origin: Point2<f32>, extent: Vector2<f32>, expected_ids: &[u32]| {
            let mut found: HashSet<u32> = HashSet::new();
            space.get_dynamic_colliders_intersecting_rect(&origin, &extent, mask, |c| {
                found.insert(c.entity_id().unwrap());
                Sentinel::Continue
            });
            assert_eq!(found.len(), expected_ids.len());
            for id in expected_ids.iter() {
                assert!(found.contains(id));
            }
        };
        let unit = vec2(1.0, 1.0);

        test_rect(point2(0.0, 0.0), unit, &[]);
        test_rect(point2(12.0, 1.0), unit, &[]);

        test_rect(point2(1.5, 1.0), unit, &[0]);
        test_rect(point2(4.0, 4.0), unit, &[1]);
        test_rect(point2(3.0, 2.0), unit, &[0, 1]);
        test_rect(point2(8.0, 3.0), unit, &[2]);
        test_rect(point2(16.0, 3.0), unit, &[3]);
        test_rect(point2(17.0, 7.0), unit, &[4]);
        test_rect(point2(21.5, 2.5), unit, &[5]);
        test_rect(point2(24.0, 4.0), unit, &[5, 6]);
        test_rect(point2(25.5, 5.5), unit, &[6]);
        test_rect(point2(31.5, 2.5), unit, &[7]);
        test_rect(point2(31.5, 7.5), unit, &[]); // miss
        test_rect(point2(35.0, 2.5), unit, &[]); // miss
    }
}
