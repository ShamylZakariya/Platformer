use cgmath::{vec2, vec3, MetricSpace, Point2, Point3, Vector2, Vector3, Vector4};
use std::collections::HashMap;

use crate::texture;
use wgpu::util::DeviceExt;

// --------------------------------------------------------------------------------------------------------------------

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SpriteVertex {
    pub position: Vector3<f32>,
    pub tex_coord: Vector2<f32>,
    pub color: Vector4<f32>,
}
unsafe impl bytemuck::Zeroable for SpriteVertex {}
unsafe impl bytemuck::Pod for SpriteVertex {}

impl SpriteVertex {
    pub fn new(position: Vector3<f32>, tex_coord: Vector2<f32>, color: Vector4<f32>) -> Self {
        Self {
            position,
            tex_coord,
            color,
        }
    }
}

impl Vertex for SpriteVertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float2,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float4,
                },
            ],
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// Represents the shape of a sprite, where Square represents a standard, square, sprite and the remainder
/// are triangles, with the surface normal facing in the specqified direction. E.g., NorthEast would be a triangle
/// with the edge normal facing up and to the right.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SpriteShape {
    Square,
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

impl SpriteShape {
    pub fn flipped_horizontally(&self) -> Self {
        match self {
            SpriteShape::Square => SpriteShape::Square,
            SpriteShape::NorthEast => SpriteShape::NorthWest,
            SpriteShape::SouthEast => SpriteShape::SouthWest,
            SpriteShape::SouthWest => SpriteShape::SouthEast,
            SpriteShape::NorthWest => SpriteShape::NorthEast,
        }
    }
    pub fn flipped_vertically(&self) -> Self {
        match self {
            SpriteShape::Square => SpriteShape::Square,
            SpriteShape::NorthEast => SpriteShape::SouthEast,
            SpriteShape::SouthEast => SpriteShape::NorthEast,
            SpriteShape::SouthWest => SpriteShape::NorthWest,
            SpriteShape::NorthWest => SpriteShape::SouthWest,
        }
    }
    pub fn flipped_diagonally(&self) -> Self {
        // https://doc.mapeditor.org/en/stable/reference/tmx-map-format/
        // Under section "Tile Flipping" diagonal flip is defined as x/y axis swap.
        // On paper, this transform was worked out for triangles. Since this is a
        // mirroring along the +x/+y diagonal axis, it only affects NorthWest and SouthEast
        // triangles, which are not symmetrical across the flip axis.
        match self {
            SpriteShape::Square => SpriteShape::Square,
            SpriteShape::NorthEast => SpriteShape::NorthEast,
            SpriteShape::SouthEast => SpriteShape::NorthWest,
            SpriteShape::SouthWest => SpriteShape::SouthWest,
            SpriteShape::NorthWest => SpriteShape::SouthEast,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SpriteDesc {
    pub shape: SpriteShape,
    pub origin: Point3<f32>,
    pub extent: Vector2<f32>,
    pub tex_coord_origin: Point2<f32>,
    pub tex_coord_extent: Vector2<f32>,
    pub color: Vector4<f32>,
    pub mask: u32,
}

impl PartialEq for SpriteDesc {
    fn eq(&self, other: &Self) -> bool {
        let eps = 1e-4 as f32;
        // TODO: cgmath uses the approx rate, which should allow for convenience macros
        // to do safe float approximate comparison, but I can't get the
        // traits to be visible here.
        self.shape == other.shape
            && self.mask == other.mask
            && self.origin.distance2(other.origin) < eps
            && self.extent.distance2(other.extent) < eps
            && self.tex_coord_origin.distance2(other.tex_coord_origin) < eps
            && self.tex_coord_extent.distance2(other.tex_coord_extent) < eps
            && self.color.distance2(other.color) < eps
    }
}

impl Eq for SpriteDesc {}

/// Simple corss product for 2D vectors; cgmath doesn't define this because cross product
/// doesn't make sense generally for 2D.
fn cross(a: &Vector2<f32>, b: &Vector2<f32>) -> f32 {
    a.x * b.y - a.y * b.x
}

impl SpriteDesc {
    /// Creates a new SpriteDesc at an arbitrary origin with a specified extent
    pub fn new(
        shape: SpriteShape,
        origin: Point3<f32>,
        extent: Vector2<f32>,
        tex_coord_origin: Point2<f32>,
        tex_coord_extent: Vector2<f32>,
        color: Vector4<f32>,
        mask: u32,
    ) -> Self {
        Self {
            shape,
            origin,
            extent,
            tex_coord_origin,
            tex_coord_extent,
            color,
            mask,
        }
    }

    /// Creates a 1x1 sprite at a given integral origin point.
    pub fn unit(
        shape: SpriteShape,
        origin: Point2<i32>,
        z: f32,
        tex_coord_origin: Point2<f32>,
        tex_coord_extent: Vector2<f32>,
        color: Vector4<f32>,
        mask: u32,
    ) -> Self {
        Self {
            shape,
            origin: Point3::new(origin.x as f32, origin.y as f32, z),
            extent: Vector2::new(1.0, 1.0),
            tex_coord_origin,
            tex_coord_extent,
            color,
            mask,
        }
    }

    pub fn left(&self) -> f32 {
        self.origin.x
    }

    pub fn bottom(&self) -> f32 {
        self.origin.y
    }

    pub fn right(&self) -> f32 {
        self.origin.x + self.extent.x
    }

    pub fn top(&self) -> f32 {
        self.origin.y + self.extent.y
    }

    pub fn contains(&self, point: &Point2<f32>) -> bool {
        if point.x >= self.origin.x
            && point.x <= self.origin.x + self.extent.x
            && point.y >= self.origin.y
            && point.y <= self.origin.y + self.extent.y
        {
            let p = Vector2::new(point.x, point.y);
            return match self.shape {
                SpriteShape::Square => true,

                SpriteShape::NorthEast => {
                    let a = Vector2::new(self.origin.x, self.origin.y + self.extent.y);
                    let b = Vector2::new(self.origin.x + self.extent.x, self.origin.y);
                    let ba = b - a;
                    let pa = p - a;
                    cross(&ba, &pa) <= 0.0
                }

                SpriteShape::SouthEast => {
                    let a = Vector2::new(self.origin.x, self.origin.y);
                    let b =
                        Vector2::new(self.origin.x + self.extent.x, self.origin.y + self.extent.y);
                    let ba = b - a;
                    let pa = p - a;
                    cross(&ba, &pa) >= 0.0
                }

                SpriteShape::SouthWest => {
                    let a = Vector2::new(self.origin.x, self.origin.y + self.extent.y);
                    let b = Vector2::new(self.origin.x + self.extent.x, self.origin.y);
                    let ba = b - a;
                    let pa = p - a;
                    // opposite winding of northeast
                    cross(&ba, &pa) >= 0.0
                }

                SpriteShape::NorthWest => {
                    let a = Vector2::new(self.origin.x, self.origin.y);
                    let b =
                        Vector2::new(self.origin.x + self.extent.x, self.origin.y + self.extent.y);
                    let ba = b - a;
                    let pa = p - a;
                    // opposite winding of southeast
                    cross(&ba, &pa) <= 0.0
                }
            };
        }

        false
    }

    // returns a copy of self, flipped horizontally. This only affects shape and texture coordinates
    pub fn flipped_horizontally(&self) -> Self {
        Self {
            shape: self.shape,
            origin: self.origin,
            extent: self.extent,
            tex_coord_origin: Point2::new(
                self.tex_coord_origin.x + self.tex_coord_extent.x,
                self.tex_coord_origin.y,
            ),
            tex_coord_extent: Vector2::new(-self.tex_coord_extent.x, self.tex_coord_extent.y),
            color: self.color,
            mask: self.mask,
        }
    }

    // returns a copy of self, flipped vertically. This only affects shape and texture coordinates
    pub fn flipped_vertically(&self) -> Self {
        Self {
            shape: self.shape,
            origin: self.origin,
            extent: self.extent,
            tex_coord_origin: Point2::new(
                self.tex_coord_origin.x,
                self.tex_coord_origin.y + self.tex_coord_extent.y,
            ),
            tex_coord_extent: Vector2::new(self.tex_coord_extent.x, -self.tex_coord_extent.y),
            color: self.color,
            mask: self.mask,
        }
    }

    // returns a copy of self, flipped diagonally. This only affects shape and texture coordinates
    pub fn flipped_diagonally(&self) -> Self {
        // https://doc.mapeditor.org/en/stable/reference/tmx-map-format/
        // Under section "Tile Flipping" diagonal flip is defined as x/y axis swap.
        Self {
            shape: self.shape,
            origin: self.origin,
            extent: self.extent,
            tex_coord_origin: Point2::new(self.tex_coord_origin.y, self.tex_coord_origin.x),
            tex_coord_extent: Vector2::new(self.tex_coord_extent.y, self.tex_coord_extent.x),
            color: self.color,
            mask: self.mask,
        }
    }
}

#[cfg(test)]
mod sprite_desc_tests {
    use super::*;

    fn test_points(
        sprite: &SpriteDesc,
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
            Point2::new(
                sprite.origin.x + sprite.extent.x * 0.25,
                sprite.origin.y + sprite.extent.y * 0.5,
            ),
            Point2::new(
                sprite.origin.x + sprite.extent.x * 0.5,
                sprite.origin.y + sprite.extent.y * 0.25,
            ),
            Point2::new(
                sprite.origin.x + sprite.extent.x * 0.75,
                sprite.origin.y + sprite.extent.y * 0.5,
            ),
            Point2::new(
                sprite.origin.x + sprite.extent.x * 0.5,
                sprite.origin.y + sprite.extent.y * 0.75,
            ),
            // outside
            Point2::new(
                sprite.origin.x - sprite.extent.x * 0.25,
                sprite.origin.y + sprite.extent.y * 0.5,
            ),
            Point2::new(
                sprite.origin.x + sprite.extent.x * 0.5,
                sprite.origin.y - sprite.extent.y * 0.25,
            ),
            Point2::new(
                sprite.origin.x + sprite.extent.x * 1.25,
                sprite.origin.y + sprite.extent.y * 0.5,
            ),
            Point2::new(
                sprite.origin.x + sprite.extent.x * 0.5,
                sprite.origin.y + sprite.extent.y * 1.25,
            ),
        )
    }

    fn test_containment(mut sprite: SpriteDesc) {
        let (p0, p1, p2, p3, p4, p5, p6, p7) = test_points(&sprite);

        sprite.shape = SpriteShape::Square;
        assert!(sprite.contains(&p0));
        assert!(sprite.contains(&p1));
        assert!(sprite.contains(&p2));
        assert!(sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.shape = SpriteShape::NorthEast;
        assert!(sprite.contains(&p0));
        assert!(sprite.contains(&p1));
        assert!(!sprite.contains(&p2));
        assert!(!sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.shape = SpriteShape::SouthEast;
        assert!(sprite.contains(&p0));
        assert!(!sprite.contains(&p1));
        assert!(!sprite.contains(&p2));
        assert!(sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.shape = SpriteShape::SouthWest;
        assert!(!sprite.contains(&p0));
        assert!(!sprite.contains(&p1));
        assert!(sprite.contains(&p2));
        assert!(sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.shape = SpriteShape::NorthWest;
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
        let mut sprite = SpriteDesc::new(
            SpriteShape::Square,
            Point3::new(0.0, 0.0, 0.0),
            Vector2::new(1.0, 1.0),
            Point2::new(0.0, 0.0),
            Vector2::new(1.0, 1.0),
            Vector4::new(0.0, 0.0, 0.0, 0.0),
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
    fn double_flip_is_identity() {
        let mut sprite = SpriteDesc::unit(
            SpriteShape::Square,
            Point2::new(0, 0),
            0.0,
            Point2::new(0.1, 0.1),
            Vector2::new(0.2, 0.2),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
            0,
        );

        assert_eq!(sprite, sprite.flipped_horizontally().flipped_horizontally());
        assert_eq!(sprite, sprite.flipped_vertically().flipped_vertically());
        assert_eq!(sprite, sprite.flipped_diagonally().flipped_diagonally());
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub struct SpriteHitTester {
    unit_sprites: HashMap<Point2<i32>, SpriteDesc>,
    non_unit_sprites: Vec<SpriteDesc>,
}

impl SpriteHitTester {
    pub fn new(sprite_descs: &[SpriteDesc]) -> Self {
        let mut unit_sprites = HashMap::new();
        let mut non_unit_sprites = vec![];

        for sprite in sprite_descs {
            // copy sprites into appropriate storage
            if sprite.extent.x == 1.0 && sprite.extent.y == 1.0 {
                unit_sprites.insert(
                    Point2::new(sprite.origin.x as i32, sprite.origin.y as i32),
                    *sprite,
                );
            } else {
                non_unit_sprites.push(*sprite);
            }
        }

        // sort non-unit sprites along x and (secondarily) y
        non_unit_sprites.sort_by(|a, b| {
            let ord_0 = a.origin.x.partial_cmp(&b.origin.x).unwrap();
            if ord_0 == std::cmp::Ordering::Equal {
                a.origin.y.partial_cmp(&b.origin.y).unwrap()
            } else {
                ord_0
            }
        });

        Self {
            unit_sprites,
            non_unit_sprites,
        }
    }

    /// tests if a point in the sprites' coordinate system intersects with a sprite.
    /// Filters by mask, such that only sprites with matching mask bits will be matched.
    /// In the case of overlapping sprites, there is no guarantee which will be returned,
    /// except that unit sprites will be tested before non-unit sprites.
    pub fn test(&self, point: &Point2<f32>, mask: u32) -> Option<SpriteDesc> {
        // first test the unit sprites
        if let Some(sprite) = self
            .unit_sprites
            .get(&Point2::new(point.x.floor() as i32, point.y.floor() as i32))
        {
            if sprite.mask & mask != 0 {
                return Some(*sprite);
            }
        }

        // non_unit sprites are stored sorted along x, so we can early exit
        // TODO: Some kind of partitioning/binary search?

        for sprite in &self.non_unit_sprites {
            if sprite.origin.x > point.x {
                break;
            }
            if sprite.contains(point) && sprite.mask & mask != 0 {
                return Some(*sprite);
            }
        }

        None
    }
}

#[cfg(test)]
mod sprite_hit_tester {
    use super::*;

    #[test]
    fn new_produces_expected_unit_and_non_unit_sprite_storage() {
        let tco = Point2::new(0.0, 0.0);
        let tce = Vector2::new(1.0, 1.0);
        let color = Vector4::new(1.0, 1.0, 1.0, 1.0);

        let unit_0 = SpriteDesc::unit(
            SpriteShape::Square,
            Point2::new(0, 0),
            0.0,
            tco,
            tce,
            color,
            0,
        );
        let unit_1 = SpriteDesc::unit(
            SpriteShape::Square,
            Point2::new(11, -33),
            0.0,
            tco,
            tce,
            color,
            0,
        );
        let non_unit_0 = SpriteDesc::new(
            SpriteShape::Square,
            Point3::new(10.0, 5.0, 0.0),
            Vector2::new(5.0, 1.0),
            tco,
            tce,
            color,
            0,
        );

        let non_unit_1 = SpriteDesc::new(
            SpriteShape::Square,
            Point3::new(-1.0, -10.0, 0.0),
            Vector2::new(50.0, 5.0),
            tco,
            tce,
            color,
            0,
        );

        let hit_tester = SpriteHitTester::new(&[unit_0, unit_1, non_unit_0, non_unit_1]);
        assert_eq!(
            hit_tester
                .unit_sprites
                .get(&Point2::new(unit_0.origin.x as i32, unit_0.origin.y as i32))
                .unwrap(),
            &unit_0
        );
        assert_eq!(
            hit_tester
                .unit_sprites
                .get(&Point2::new(unit_1.origin.x as i32, unit_1.origin.y as i32))
                .unwrap(),
            &unit_1
        );

        // non-unit sprites are sorted along X
        assert_eq!(hit_tester.non_unit_sprites[0], non_unit_1);
        assert_eq!(hit_tester.non_unit_sprites[1], non_unit_0);
    }

    #[test]
    fn unit_sprite_hit_test_works() {
        let square_mask = 1 << 0;
        let triangle_mask = 1 << 1;
        let all_mask = square_mask | triangle_mask;

        let tco = Point2::new(0.0, 0.0);
        let tce = Vector2::new(1.0, 1.0);
        let color = Vector4::new(1.0, 1.0, 1.0, 1.0);

        let sb1 = SpriteDesc::unit(
            SpriteShape::Square,
            Point2::new(0, 0),
            10.0,
            tco,
            tce,
            color,
            square_mask,
        );

        let sb2 = SpriteDesc::unit(
            SpriteShape::Square,
            Point2::new(-1, -1),
            10.0,
            tco,
            tce,
            color,
            square_mask,
        );

        let tr0 = SpriteDesc::unit(
            SpriteShape::NorthEast,
            Point2::new(0, 4),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let tr1 = SpriteDesc::unit(
            SpriteShape::NorthWest,
            Point2::new(-1, 4),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let tr2 = SpriteDesc::unit(
            SpriteShape::SouthWest,
            Point2::new(-1, 3),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let tr3 = SpriteDesc::unit(
            SpriteShape::SouthEast,
            Point2::new(0, 3),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let hit_tester = SpriteHitTester::new(&[sb1, sb2, tr0, tr1, tr2, tr3]);

        // test triangle is hit only when using triangle_flags or all_mask
        assert!(hit_tester.test(&Point2::new(0.1, 4.1), triangle_mask) == Some(tr0));
        assert!(hit_tester.test(&Point2::new(-0.1, 4.1), triangle_mask) == Some(tr1));
        assert!(hit_tester.test(&Point2::new(-0.1, 3.9), triangle_mask) == Some(tr2));
        assert!(hit_tester.test(&Point2::new(0.1, 3.9), triangle_mask) == Some(tr3));
        assert!(hit_tester
            .test(&Point2::new(0.1, 4.1), square_mask)
            .is_none());
        assert!(hit_tester.test(&Point2::new(0.1, 3.9), all_mask).is_some());

        // test square is only hit when mask is square or all_mask
        assert!(hit_tester.test(&Point2::new(0.5, 0.5), square_mask) == Some(sb1));
        assert!(hit_tester
            .test(&Point2::new(0.5, 0.5), triangle_mask)
            .is_none());
        assert!(hit_tester.test(&Point2::new(0.5, 0.5), all_mask).is_some());
    }

    #[test]
    fn non_unit_hit_test_works() {
        let tco = Point2::new(0.0, 0.0);
        let tce = Vector2::new(1.0, 1.0);
        let color = Vector4::new(1.0, 1.0, 1.0, 1.0);

        let mask0 = 1 << 0;
        let mask1 = 1 << 1;
        let mask2 = 1 << 2;
        let unused_mask = 1 << 16;
        let all_mask = mask0 | mask1 | mask2 | unused_mask;

        let b0 = SpriteDesc::new(
            SpriteShape::Square,
            Point3::new(-4.0, -4.0, 0.0),
            Vector2::new(8.0, 4.0),
            tco,
            tce,
            color,
            mask0,
        );

        let b1 = SpriteDesc::new(
            SpriteShape::Square,
            Point3::new(3.0, -1.0, 0.0),
            Vector2::new(3.0, 1.0),
            tco,
            tce,
            color,
            mask1,
        );

        let b2 = SpriteDesc::new(
            SpriteShape::Square,
            Point3::new(3.0, -2.0, 0.0),
            Vector2::new(2.0, 5.0),
            tco,
            tce,
            color,
            mask2,
        );

        let hit_tester = SpriteHitTester::new(&[b0, b1, b2]);

        // this point is in all three boxes
        let p = Point2::new(3.5, -0.5);

        assert_eq!(hit_tester.test(&p, mask0), Some(b0));
        assert_eq!(hit_tester.test(&p, mask1), Some(b1));
        assert_eq!(hit_tester.test(&p, mask2), Some(b2));
        assert_eq!(hit_tester.test(&p, unused_mask), None);
        assert!(hit_tester.test(&p, all_mask).is_some());
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub struct SpriteMaterial {
    pub name: String,
    pub texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

#[allow(dead_code)]
impl SpriteMaterial {
    pub fn new(
        device: &wgpu::Device,
        name: &str,
        texture: texture::Texture,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some(name),
        });
        Self {
            name: String::from(name),
            texture,
            bind_group,
        }
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // diffuse texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Uint,
                    },
                    count: None,
                },
                // diffuse texture sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        })
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub struct SpriteMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

impl SpriteMesh {
    pub fn new(
        sprites: &Vec<SpriteDesc>,
        material: usize,
        device: &wgpu::Device,
        name: &str,
    ) -> Self {
        let mut vertices = vec![];
        let mut indices = vec![];
        for sprite in sprites {
            let p_a = vec3(sprite.origin.x, sprite.origin.y, sprite.origin.z);
            let p_b = vec3(
                sprite.origin.x + sprite.extent.x,
                sprite.origin.y,
                sprite.origin.z,
            );
            let p_c = vec3(
                sprite.origin.x + sprite.extent.x,
                sprite.origin.y + sprite.extent.y,
                sprite.origin.z,
            );
            let p_d = vec3(
                sprite.origin.x,
                sprite.origin.y + sprite.extent.y,
                sprite.origin.z,
            );

            let tc_a = vec2::<f32>(sprite.tex_coord_origin.x, 1.0 - sprite.tex_coord_origin.y);
            let tc_b = vec2::<f32>(
                sprite.tex_coord_origin.x + sprite.tex_coord_extent.x,
                1.0 - sprite.tex_coord_origin.y,
            );
            let tc_c = vec2::<f32>(
                sprite.tex_coord_origin.x + sprite.tex_coord_extent.x,
                1.0 - (sprite.tex_coord_origin.y + sprite.tex_coord_extent.y),
            );
            let tc_d = vec2::<f32>(
                sprite.tex_coord_origin.x,
                1.0 - (sprite.tex_coord_origin.y + sprite.tex_coord_extent.y),
            );

            let sv_a = SpriteVertex::new(p_a, tc_a, sprite.color);
            let sv_b = SpriteVertex::new(p_b, tc_b, sprite.color);
            let sv_c = SpriteVertex::new(p_c, tc_c, sprite.color);
            let sv_d = SpriteVertex::new(p_d, tc_d, sprite.color);
            let idx = vertices.len();

            match sprite.shape {
                SpriteShape::Square => {
                    vertices.push(sv_a);
                    vertices.push(sv_b);
                    vertices.push(sv_c);
                    vertices.push(sv_d);

                    indices.push((idx + 0) as u32);
                    indices.push((idx + 1) as u32);
                    indices.push((idx + 2) as u32);

                    indices.push((idx + 0) as u32);
                    indices.push((idx + 2) as u32);
                    indices.push((idx + 3) as u32);
                }
                SpriteShape::NorthEast => {
                    vertices.push(sv_a);
                    vertices.push(sv_b);
                    vertices.push(sv_d);
                    indices.push((idx + 0) as u32);
                    indices.push((idx + 1) as u32);
                    indices.push((idx + 2) as u32);
                }
                SpriteShape::SouthEast => {
                    vertices.push(sv_a);
                    vertices.push(sv_c);
                    vertices.push(sv_d);
                    indices.push((idx + 0) as u32);
                    indices.push((idx + 1) as u32);
                    indices.push((idx + 2) as u32);
                }
                SpriteShape::SouthWest => {
                    vertices.push(sv_b);
                    vertices.push(sv_c);
                    vertices.push(sv_d);
                    indices.push((idx + 0) as u32);
                    indices.push((idx + 1) as u32);
                    indices.push((idx + 2) as u32);
                }
                SpriteShape::NorthWest => {
                    vertices.push(sv_a);
                    vertices.push(sv_b);
                    vertices.push(sv_c);
                    indices.push((idx + 0) as u32);
                    indices.push((idx + 1) as u32);
                    indices.push((idx + 2) as u32);
                }
            }
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", name)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Index Buffer", name)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsage::INDEX,
        });

        let num_elements = indices.len() as u32;

        Self {
            vertex_buffer,
            index_buffer,
            num_elements,
            material,
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub struct SpriteCollection {
    pub meshes: Vec<SpriteMesh>,
    pub materials: Vec<SpriteMaterial>,
}

impl SpriteCollection {
    pub fn new(meshes: Vec<SpriteMesh>, materials: Vec<SpriteMaterial>) -> Self {
        Self { meshes, materials }
    }
}

impl Default for SpriteCollection {
    fn default() -> Self {
        Self {
            meshes: vec![],
            materials: vec![],
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub trait DrawSprite<'a, 'b>
where
    'b: 'a,
{
    fn draw_sprite(
        &mut self,
        sprite_mesh: &'b SpriteMesh,
        material: &'b SpriteMaterial,
        uniforms: &'b wgpu::BindGroup,
    );

    fn draw_sprite_collection(
        &mut self,
        sprites: &'b SpriteCollection,
        uniforms: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawSprite<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_sprite(
        &mut self,
        sprite_mesh: &'b SpriteMesh,
        material: &'b SpriteMaterial,
        uniforms: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, sprite_mesh.vertex_buffer.slice(..));
        self.set_index_buffer(sprite_mesh.index_buffer.slice(..));
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, &uniforms, &[]);
        self.draw_indexed(0..sprite_mesh.num_elements, 0, 0..1);
    }

    fn draw_sprite_collection(
        &mut self,
        sprites: &'b SpriteCollection,
        uniforms: &'b wgpu::BindGroup,
    ) {
        for sprite_mesh in &sprites.meshes {
            let material = &sprites.materials[sprite_mesh.material];
            self.draw_sprite(sprite_mesh, material, uniforms);
        }
    }
}
