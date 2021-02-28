use cgmath::*;
use core::f32;
use std::hash::Hash;

use crate::util::*;

use crate::collision::Shape;

/// Sprite represents a sprite in CPU terms, e.g., sprite is for collision detection,
/// positioning, representing a level or entity in memory. For rendering, See sprite::rendering::Drawable
#[derive(Copy, Clone, Debug)]
pub struct Sprite {
    pub collision_shape: Shape,
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
            collision_shape: Shape::None,
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
        collision_shape: Shape,
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
        collision_shape: Shape,
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

    #[test]
    fn double_flip_is_identity() {
        let sprite = Sprite::unit(
            Shape::Square,
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
