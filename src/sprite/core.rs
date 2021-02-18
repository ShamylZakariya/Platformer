use cgmath::*;
use core::f32;
use std::hash::Hash;

use crate::geom::{self, Bounds};

fn hash_point2<H: std::hash::Hasher>(point: &Point2<f32>, state: &mut H) {
    ((point.x * 1000.0) as i32).hash(state);
    ((point.y * 1000.0) as i32).hash(state);
}

fn hash_point3<H: std::hash::Hasher>(point: &Point3<f32>, state: &mut H) {
    ((point.x * 1000.0) as i32).hash(state);
    ((point.y * 1000.0) as i32).hash(state);
    ((point.z * 1000.0) as i32).hash(state);
}

fn hash_vec2<H: std::hash::Hasher>(v: &Vector2<f32>, state: &mut H) {
    ((v.x * 1000.0) as i32).hash(state);
    ((v.y * 1000.0) as i32).hash(state);
}

fn hash_vec3<H: std::hash::Hasher>(v: &Vector3<f32>, state: &mut H) {
    ((v.x * 1000.0) as i32).hash(state);
    ((v.y * 1000.0) as i32).hash(state);
    ((v.z * 1000.0) as i32).hash(state);
}

fn hash_vec4<H: std::hash::Hasher>(v: &Vector4<f32>, state: &mut H) {
    ((v.x * 1000.0) as i32).hash(state);
    ((v.y * 1000.0) as i32).hash(state);
    ((v.z * 1000.0) as i32).hash(state);
    ((v.w * 1000.0) as i32).hash(state);
}

// --------------------------------------------------------------------------------------------------------------------

/// Represents the shape of a sprite, where Square represents a standard, square, sprite and the remainder
/// are triangles, with the surface normal facing in the specqified direction. E.g., NorthEast would be a triangle
/// with the edge normal facing up and to the right.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CollisionShape {
    None,
    Square,
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

impl CollisionShape {
    pub fn flipped_horizontally(&self) -> Self {
        match self {
            CollisionShape::None => CollisionShape::None,
            CollisionShape::Square => CollisionShape::Square,
            CollisionShape::NorthEast => CollisionShape::NorthWest,
            CollisionShape::SouthEast => CollisionShape::SouthWest,
            CollisionShape::SouthWest => CollisionShape::SouthEast,
            CollisionShape::NorthWest => CollisionShape::NorthEast,
        }
    }
    pub fn flipped_vertically(&self) -> Self {
        match self {
            CollisionShape::None => CollisionShape::None,
            CollisionShape::Square => CollisionShape::Square,
            CollisionShape::NorthEast => CollisionShape::SouthEast,
            CollisionShape::SouthEast => CollisionShape::NorthEast,
            CollisionShape::SouthWest => CollisionShape::NorthWest,
            CollisionShape::NorthWest => CollisionShape::SouthWest,
        }
    }
    pub fn flipped_diagonally(&self) -> Self {
        // https://doc.mapeditor.org/en/stable/reference/tmx-map-format/#tile-flipping
        // Under section "Tile Flipping" diagonal flip is defined as x/y axis swap.
        // On paper, this transform was worked out for triangles. Since this is a
        // mirroring along the +x/+y diagonal axis, it only affects NorthWest and SouthEast
        // triangles, which are not symmetrical across the flip axis.
        match self {
            CollisionShape::None => CollisionShape::None,
            CollisionShape::Square => CollisionShape::Square,
            CollisionShape::NorthEast => CollisionShape::NorthEast,
            CollisionShape::SouthEast => CollisionShape::NorthWest,
            CollisionShape::SouthWest => CollisionShape::SouthWest,
            CollisionShape::NorthWest => CollisionShape::SouthEast,
        }
    }
}

/// Sprite represents a sprite in CPU terms, e.g., sprite is for collision detection,
/// positioning, representing a level or entity in memory. For rendering, See sprite::rendering::Drawable
#[derive(Copy, Clone, Debug)]
pub struct Sprite {
    pub collision_shape: CollisionShape,
    pub origin: Point3<f32>,
    pub extent: Vector2<f32>,
    pub tex_coord_origin: Point2<f32>,
    pub tex_coord_extent: Vector2<f32>,
    pub color: Vector4<f32>,
    pub mask: u32,
    pub entity_id: Option<u32>,
    pub flipped_diagonally: bool,
    pub flipped_horizontally: bool,
    pub flipped_vertically: bool,
}

impl PartialEq for Sprite {
    fn eq(&self, other: &Self) -> bool {
        self.collision_shape == other.collision_shape
            && self.entity_id == other.entity_id
            && self.mask == other.mask
            && relative_eq!(self.origin, other.origin)
            && relative_eq!(self.extent, other.extent)
            && relative_eq!(self.tex_coord_origin, other.tex_coord_origin)
            && relative_eq!(self.tex_coord_extent, other.tex_coord_extent)
            && relative_eq!(self.color, other.color)
            && self.flipped_diagonally == other.flipped_diagonally
            && self.flipped_horizontally == other.flipped_horizontally
            && self.flipped_vertically == other.flipped_vertically
    }
}

impl Hash for Sprite {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.collision_shape.hash(state);
        self.entity_id.hash(state);
        hash_point3(&self.origin, state);
        hash_vec2(&self.extent, state);
        hash_point2(&self.tex_coord_origin, state);
        hash_vec2(&self.tex_coord_extent, state);
        hash_vec4(&self.color, state);
        self.mask.hash(state);
        self.flipped_diagonally.hash(state);
        self.flipped_horizontally.hash(state);
        self.flipped_vertically.hash(state);
    }
}

impl Eq for Sprite {}

/// Simple corss product for 2D vectors; cgmath doesn't define this because cross product
/// doesn't make sense generally for 2D.
fn cross(a: &Vector2<f32>, b: &Vector2<f32>) -> f32 {
    a.x * b.y - a.y * b.x
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            collision_shape: CollisionShape::None,
            origin: point3(0.0, 0.0, 0.0),
            extent: vec2(0.0, 0.0),
            tex_coord_origin: point2(0.0, 0.0),
            tex_coord_extent: vec2(0.0, 0.0),
            color: vec4(1.0, 1.0, 1.0, 1.0),
            mask: 0,
            entity_id: None,
            flipped_diagonally: false,
            flipped_horizontally: false,
            flipped_vertically: false,
        }
    }
}

impl Sprite {
    /// Creates a new Sprite at an arbitrary origin with a specified extent
    pub fn new(
        collision_shape: CollisionShape,
        origin: Point3<f32>,
        extent: Vector2<f32>,
        tex_coord_origin: Point2<f32>,
        tex_coord_extent: Vector2<f32>,
        color: Vector4<f32>,
        mask: u32,
    ) -> Self {
        Self {
            collision_shape,
            origin,
            extent,
            tex_coord_origin,
            tex_coord_extent,
            color,
            mask,
            entity_id: None,
            flipped_diagonally: false,
            flipped_horizontally: false,
            flipped_vertically: false,
        }
    }

    /// Creates a 1x1 sprite at a given integral origin point.
    pub fn unit(
        collision_shape: CollisionShape,
        origin: Point2<i32>,
        z: f32,
        tex_coord_origin: Point2<f32>,
        tex_coord_extent: Vector2<f32>,
        color: Vector4<f32>,
        mask: u32,
    ) -> Self {
        Self {
            collision_shape,
            origin: point3(origin.x as f32, origin.y as f32, z),
            extent: vec2(1.0, 1.0),
            tex_coord_origin,
            tex_coord_extent,
            color,
            mask,
            entity_id: None,
            flipped_diagonally: false,
            flipped_horizontally: false,
            flipped_vertically: false,
        }
    }

    pub fn contains(&self, point: &Point2<f32>) -> bool {
        if point.x >= self.origin.x
            && point.x <= self.origin.x + self.extent.x
            && point.y >= self.origin.y
            && point.y <= self.origin.y + self.extent.y
        {
            let p = vec2(point.x, point.y);
            return match self.collision_shape {
                CollisionShape::None => false,

                CollisionShape::Square => true,

                CollisionShape::NorthEast => {
                    let a = vec2(self.origin.x, self.origin.y + self.extent.y);
                    let b = vec2(self.origin.x + self.extent.x, self.origin.y);
                    let ba = b - a;
                    let pa = p - a;
                    cross(&ba, &pa) <= 0.0
                }

                CollisionShape::SouthEast => {
                    let a = vec2(self.origin.x, self.origin.y);
                    let b = vec2(self.origin.x + self.extent.x, self.origin.y + self.extent.y);
                    let ba = b - a;
                    let pa = p - a;
                    cross(&ba, &pa) >= 0.0
                }

                CollisionShape::SouthWest => {
                    let a = vec2(self.origin.x, self.origin.y + self.extent.y);
                    let b = vec2(self.origin.x + self.extent.x, self.origin.y);
                    let ba = b - a;
                    let pa = p - a;
                    // opposite winding of northeast
                    cross(&ba, &pa) >= 0.0
                }

                CollisionShape::NorthWest => {
                    let a = vec2(self.origin.x, self.origin.y);
                    let b = vec2(self.origin.x + self.extent.x, self.origin.y + self.extent.y);
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
        match self.collision_shape {
            CollisionShape::None => None,
            CollisionShape::Square => geom::intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(self.origin.x, self.origin.y),
                    point2(self.origin.x + self.extent.x, self.origin.y),
                    point2(self.origin.x + self.extent.x, self.origin.y + self.extent.y),
                    point2(self.origin.x, self.origin.y + self.extent.y),
                ],
            ),
            CollisionShape::NorthEast => geom::intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(self.origin.x, self.origin.y),
                    point2(self.origin.x + self.extent.x, self.origin.y),
                    point2(self.origin.x, self.origin.y + self.extent.y),
                ],
            ),
            CollisionShape::SouthEast => geom::intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(self.origin.x, self.origin.y),
                    point2(self.origin.x + self.extent.x, self.origin.y + self.extent.y),
                    point2(self.origin.x, self.origin.y + self.extent.y),
                ],
            ),
            CollisionShape::SouthWest => geom::intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(self.origin.x + self.extent.x, self.origin.y),
                    point2(self.origin.x + self.extent.x, self.origin.y + self.extent.y),
                    point2(self.origin.x, self.origin.y + self.extent.y),
                ],
            ),
            CollisionShape::NorthWest => geom::intersection::line_convex_poly_closest(
                a,
                b,
                &[
                    point2(self.origin.x, self.origin.y),
                    point2(self.origin.x + self.extent.x, self.origin.y),
                    point2(self.origin.x + self.extent.x, self.origin.y + self.extent.y),
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
                self.origin.x <= origin.x + extent.x && self.origin.x + self.extent.x >= origin.x,
                self.origin.y <= origin.y + extent.y && self.origin.y + self.extent.y >= origin.y,
            )
        } else {
            (
                self.origin.x < origin.x + extent.x && self.origin.x + self.extent.x > origin.x,
                self.origin.y < origin.y + extent.y && self.origin.y + self.extent.y > origin.y,
            )
        };

        if x_overlap && y_overlap {
            // TODO: Implement colliders for corner blocks? GGQ doesn't need them,
            // but it seems like I'd want to flesh that out for a real 2d engine.
            // So for now, we treat any non-none shape as rectangular.
            !matches!(self.collision_shape, CollisionShape::None)
        } else {
            false
        }
    }

    /// Returns true if this Sprite overlaps the described unit square with lower/left origin and extent of (1,1).
    pub fn unit_rect_intersection(&self, origin: &Point2<f32>, inset: f32, contact: bool) -> bool {
        self.rect_intersection(origin, &vec2(1.0, 1.0), inset, contact)
    }

    pub fn left(&self) -> f32 {
        self.origin.x
    }

    pub fn right(&self) -> f32 {
        self.origin.x + self.extent.x
    }

    pub fn bottom(&self) -> f32 {
        self.origin.y
    }

    pub fn top(&self) -> f32 {
        self.origin.y + self.extent.y
    }

    // returns a copy of self, flipped horizontally. This only affects shape and texture coordinates
    pub fn flipped_horizontally(&self) -> Self {
        Self {
            collision_shape: self.collision_shape.flipped_horizontally(),
            origin: self.origin,
            extent: self.extent,
            tex_coord_origin: self.tex_coord_origin,
            tex_coord_extent: self.tex_coord_extent,
            color: self.color,
            mask: self.mask,
            entity_id: self.entity_id,
            flipped_diagonally: self.flipped_diagonally,
            flipped_horizontally: !self.flipped_horizontally,
            flipped_vertically: self.flipped_vertically,
        }
    }

    // returns a copy of self, flipped vertically. This only affects shape and texture coordinates
    pub fn flipped_vertically(&self) -> Self {
        Self {
            collision_shape: self.collision_shape.flipped_vertically(),
            origin: self.origin,
            extent: self.extent,
            tex_coord_origin: self.tex_coord_origin,
            tex_coord_extent: self.tex_coord_extent,
            color: self.color,
            mask: self.mask,
            entity_id: self.entity_id,
            flipped_diagonally: self.flipped_diagonally,
            flipped_horizontally: self.flipped_horizontally,
            flipped_vertically: !self.flipped_vertically,
        }
    }

    // returns a copy of self, flipped diagonally. This only affects shape and texture coordinates
    pub fn flipped_diagonally(&self) -> Self {
        // https://doc.mapeditor.org/en/stable/reference/tmx-map-format/#tile-flipping
        // Under section "Tile Flipping" diagonal flip is defined as x/y axis swap.
        Self {
            collision_shape: self.collision_shape.flipped_diagonally(),
            origin: self.origin,
            extent: self.extent,
            tex_coord_origin: self.tex_coord_origin,
            tex_coord_extent: self.tex_coord_extent,
            color: self.color,
            mask: self.mask,
            entity_id: self.entity_id,
            flipped_diagonally: !self.flipped_diagonally,
            flipped_horizontally: self.flipped_horizontally,
            flipped_vertically: self.flipped_vertically,
        }
    }
}

/// Returns the bounding rect containing all the provided sprites
pub fn find_bounds(sprites: &[Sprite]) -> Bounds {
    let mut min: Point2<f32> = point2(f32::MAX, f32::MAX);
    let mut max: Point2<f32> = point2(f32::MIN, f32::MIN);
    for s in sprites {
        min.x = min.x.min(s.origin.x);
        min.y = min.y.min(s.origin.y);
        max.x = max.x.max(s.origin.x + s.extent.x);
        max.y = max.y.max(s.origin.y + s.extent.y);
    }
    Bounds::new(min, max - min)
}

#[cfg(test)]
mod sprite_tests {
    use super::*;

    fn test_points(
        sprite: &Sprite,
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
                sprite.origin.x + sprite.extent.x * 0.25,
                sprite.origin.y + sprite.extent.y * 0.5,
            ),
            point2(
                sprite.origin.x + sprite.extent.x * 0.5,
                sprite.origin.y + sprite.extent.y * 0.25,
            ),
            point2(
                sprite.origin.x + sprite.extent.x * 0.75,
                sprite.origin.y + sprite.extent.y * 0.5,
            ),
            point2(
                sprite.origin.x + sprite.extent.x * 0.5,
                sprite.origin.y + sprite.extent.y * 0.75,
            ),
            // outside
            point2(
                sprite.origin.x - sprite.extent.x * 0.25,
                sprite.origin.y + sprite.extent.y * 0.5,
            ),
            point2(
                sprite.origin.x + sprite.extent.x * 0.5,
                sprite.origin.y - sprite.extent.y * 0.25,
            ),
            point2(
                sprite.origin.x + sprite.extent.x * 1.25,
                sprite.origin.y + sprite.extent.y * 0.5,
            ),
            point2(
                sprite.origin.x + sprite.extent.x * 0.5,
                sprite.origin.y + sprite.extent.y * 1.25,
            ),
        )
    }

    fn test_containment(mut sprite: Sprite) {
        let (p0, p1, p2, p3, p4, p5, p6, p7) = test_points(&sprite);

        sprite.collision_shape = CollisionShape::None;
        assert!(!sprite.contains(&p0));
        assert!(!sprite.contains(&p1));
        assert!(!sprite.contains(&p2));
        assert!(!sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.collision_shape = CollisionShape::Square;
        assert!(sprite.contains(&p0));
        assert!(sprite.contains(&p1));
        assert!(sprite.contains(&p2));
        assert!(sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.collision_shape = CollisionShape::NorthEast;
        assert!(sprite.contains(&p0));
        assert!(sprite.contains(&p1));
        assert!(!sprite.contains(&p2));
        assert!(!sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.collision_shape = CollisionShape::SouthEast;
        assert!(sprite.contains(&p0));
        assert!(!sprite.contains(&p1));
        assert!(!sprite.contains(&p2));
        assert!(sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.collision_shape = CollisionShape::SouthWest;
        assert!(!sprite.contains(&p0));
        assert!(!sprite.contains(&p1));
        assert!(sprite.contains(&p2));
        assert!(sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.collision_shape = CollisionShape::NorthWest;
        assert!(!sprite.contains(&p0));
        assert!(sprite.contains(&p1));
        assert!(sprite.contains(&p2));
        assert!(!sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));
    }

    #[test]
    fn contains_works() {
        let mut sprite = Sprite::new(
            CollisionShape::Square,
            point3(0.0, 0.0, 0.0),
            vec2(1.0, 1.0),
            point2(0.0, 0.0),
            vec2(1.0, 1.0),
            vec4(0.0, 0.0, 0.0, 0.0),
            0,
        );

        test_containment(sprite);

        // tall, NE quadrant
        sprite.origin.x = 10.0;
        sprite.origin.y = 5.0;
        sprite.extent.y = 50.0;
        sprite.extent.x = 1.0;
        test_containment(sprite);

        // wide, NE quad
        sprite.origin.x = 10.0;
        sprite.origin.y = 5.0;
        sprite.extent.y = 1.0;
        sprite.extent.x = 50.0;
        test_containment(sprite);

        // tall, SE quadrant
        sprite.origin.x = 10.0;
        sprite.origin.y = -70.0;
        sprite.extent.y = 50.0;
        sprite.extent.x = 1.0;
        test_containment(sprite);

        // wide, SE quad
        sprite.origin.x = 10.0;
        sprite.origin.y = -10.0;
        sprite.extent.y = 1.0;
        sprite.extent.x = 50.0;
        test_containment(sprite);

        // tall, SW quadrant
        sprite.origin.x = -100.0;
        sprite.origin.y = -500.0;
        sprite.extent.y = 50.0;
        sprite.extent.x = 1.0;
        test_containment(sprite);

        // wide, SW quad
        sprite.origin.x = -100.0;
        sprite.origin.y = -500.0;
        sprite.extent.y = 1.0;
        sprite.extent.x = 50.0;
        test_containment(sprite);

        // tall, NW quadrant
        sprite.origin.x = -100.0;
        sprite.origin.y = 500.0;
        sprite.extent.y = 50.0;
        sprite.extent.x = 1.0;
        test_containment(sprite);

        // wide, NW quad
        sprite.origin.x = -100.0;
        sprite.origin.y = 500.0;
        sprite.extent.y = 1.0;
        sprite.extent.x = 50.0;
        test_containment(sprite);
    }

    #[test]
    fn line_intersection_with_square_works() {
        let sprite = Sprite::new(
            CollisionShape::Square,
            point3(0.0, 0.0, 0.0),
            vec2(1.0, 1.0),
            point2(0.0, 0.0),
            vec2(1.0, 1.0),
            vec4(0.0, 0.0, 0.0, 0.0),
            0,
        );

        assert_eq!(
            sprite.line_intersection(&point2(-0.5, 0.5), &point2(0.5, 0.5)),
            Some(point2(0.0, 0.5))
        );
        assert_eq!(
            sprite.line_intersection(&point2(0.5, 1.5), &point2(0.5, 0.5)),
            Some(point2(0.5, 1.0))
        );
        assert_eq!(
            sprite.line_intersection(&point2(1.5, 0.5), &point2(0.5, 0.5)),
            Some(point2(1.0, 0.5))
        );
        assert_eq!(
            sprite.line_intersection(&point2(0.5, -0.5), &point2(0.5, 0.5)),
            Some(point2(0.5, 0.0))
        );
    }

    #[test]
    fn line_intersection_with_slopes_works() {
        let mut sprite = Sprite::new(
            CollisionShape::NorthEast,
            point3(0.0, 0.0, 0.0),
            vec2(1.0, 1.0),
            point2(0.0, 0.0),
            vec2(1.0, 1.0),
            vec4(0.0, 0.0, 0.0, 0.0),
            0,
        );

        assert_eq!(
            sprite.line_intersection(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.0, 0.5))
        );
        assert_eq!(
            sprite.line_intersection(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            sprite.line_intersection(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            sprite.line_intersection(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.0))
        );

        sprite.collision_shape = CollisionShape::SouthEast;
        assert_eq!(
            sprite.line_intersection(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.0, 0.5))
        );
        assert_eq!(
            sprite.line_intersection(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 1.0))
        );
        assert_eq!(
            sprite.line_intersection(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            sprite.line_intersection(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.5))
        );

        sprite.collision_shape = CollisionShape::SouthWest;
        assert_eq!(
            sprite.line_intersection(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            sprite.line_intersection(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 1.0))
        );
        assert_eq!(
            sprite.line_intersection(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(1.0, 0.5))
        );
        assert_eq!(
            sprite.line_intersection(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.5))
        );

        sprite.collision_shape = CollisionShape::NorthWest;
        assert_eq!(
            sprite.line_intersection(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            sprite.line_intersection(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            sprite.line_intersection(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(1.0, 0.5))
        );
        assert_eq!(
            sprite.line_intersection(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.0))
        );
    }

    #[test]
    fn rect_intersection_works() {}

    #[test]
    fn double_flip_is_identity() {
        let sprite = Sprite::unit(
            CollisionShape::Square,
            point2(0, 0),
            0.0,
            point2(0.1, 0.1),
            vec2(0.2, 0.2),
            vec4(1.0, 1.0, 1.0, 1.0),
            0,
        );

        assert_eq!(sprite, sprite.flipped_horizontally().flipped_horizontally());
        assert_eq!(sprite, sprite.flipped_vertically().flipped_vertically());
        assert_eq!(sprite, sprite.flipped_diagonally().flipped_diagonally());
    }
}
