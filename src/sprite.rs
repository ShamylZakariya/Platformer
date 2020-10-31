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
    pub position: cgmath::Vector3<f32>,
    pub tex_coord: cgmath::Vector2<f32>,
    pub color: cgmath::Vector4<f32>,
}
unsafe impl bytemuck::Zeroable for SpriteVertex {}
unsafe impl bytemuck::Pod for SpriteVertex {}

impl SpriteVertex {
    pub fn new(
        position: cgmath::Vector3<f32>,
        tex_coord: cgmath::Vector2<f32>,
        color: cgmath::Vector4<f32>,
    ) -> Self {
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
/// are triangles, with the surface normal facing in the specified direction. E.g., NorthEast would be a triangle
/// with the edge normal facing up and to the right.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SpriteShape {
    Square,
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SpriteDesc {
    pub shape: SpriteShape,
    pub left: f32,
    pub bottom: f32,
    pub width: f32,
    pub height: f32,
    pub z: f32,
    pub color: cgmath::Vector4<f32>,
    pub mask: u32,
}

impl Eq for SpriteDesc {}

/// Simple corss product for 2D vectors; cgmath doesn't define this because cross product
/// doesn't make sense generally for 2D.
fn cross(a: &cgmath::Vector2<f32>, b: &cgmath::Vector2<f32>) -> f32 {
    a.x * b.y - a.y * b.x
}

impl SpriteDesc {
    /// Creates a new SpriteDesc of arbitrary position and size
    pub fn new(
        shape: SpriteShape,
        left: f32,
        bottom: f32,
        width: f32,
        height: f32,
        z: f32,
        color: cgmath::Vector4<f32>,
        mask: u32,
    ) -> Self {
        Self {
            shape,
            left,
            bottom,
            width,
            height,
            z,
            color,
            mask,
        }
    }

    /// Creates a 1x1 sprite with lower-left origin at left/bottom
    pub fn unit(
        shape: SpriteShape,
        left: i32,
        bottom: i32,
        z: f32,
        color: cgmath::Vector4<f32>,
        mask: u32,
    ) -> Self {
        Self {
            shape,
            left: left as f32,
            bottom: bottom as f32,
            width: 1.0,
            height: 1.0,
            z,
            color,
            mask: mask,
        }
    }

    pub fn right(&self) -> f32 {
        self.left + self.width
    }

    pub fn top(&self) -> f32 {
        self.bottom + self.height
    }

    pub fn contains(&self, point: &cgmath::Point2<f32>) -> bool {
        if point.x >= self.left
            && point.x <= self.left + self.width
            && point.y >= self.bottom
            && point.y <= self.bottom + self.height
        {
            let p = cgmath::Vector2::new(point.x, point.y);
            return match self.shape {
                SpriteShape::Square => true,

                SpriteShape::NorthEast => {
                    let a = cgmath::Vector2::new(self.left, self.bottom + self.height);
                    let b = cgmath::Vector2::new(self.left + self.width, self.bottom);
                    let ba = b - a;
                    let pa = p - a;
                    cross(&ba, &pa) <= 0.0
                }

                SpriteShape::SouthEast => {
                    let a = cgmath::Vector2::new(self.left, self.bottom);
                    let b = cgmath::Vector2::new(self.left + self.width, self.bottom + self.height);
                    let ba = b - a;
                    let pa = p - a;
                    cross(&ba, &pa) >= 0.0
                }

                SpriteShape::SouthWest => {
                    let a = cgmath::Vector2::new(self.left, self.bottom + self.height);
                    let b = cgmath::Vector2::new(self.left + self.width, self.bottom);
                    let ba = b - a;
                    let pa = p - a;
                    // opposite winding of northeast
                    cross(&ba, &pa) >= 0.0
                }

                SpriteShape::NorthWest => {
                    let a = cgmath::Vector2::new(self.left, self.bottom);
                    let b = cgmath::Vector2::new(self.left + self.width, self.bottom + self.height);
                    let ba = b - a;
                    let pa = p - a;
                    // opposite winding of southeast
                    cross(&ba, &pa) <= 0.0
                }
            };
        }

        false
    }
}

#[cfg(test)]
mod sprite_desc_tests {
    use super::*;

    fn test_points(
        sprite: &SpriteDesc,
    ) -> (
        cgmath::Point2<f32>,
        cgmath::Point2<f32>,
        cgmath::Point2<f32>,
        cgmath::Point2<f32>,
        cgmath::Point2<f32>,
        cgmath::Point2<f32>,
        cgmath::Point2<f32>,
        cgmath::Point2<f32>,
    ) {
        (
            // inside
            cgmath::Point2::new(
                sprite.left + sprite.width * 0.25,
                sprite.bottom + sprite.height * 0.5,
            ),
            cgmath::Point2::new(
                sprite.left + sprite.width * 0.5,
                sprite.bottom + sprite.height * 0.25,
            ),
            cgmath::Point2::new(
                sprite.left + sprite.width * 0.75,
                sprite.bottom + sprite.height * 0.5,
            ),
            cgmath::Point2::new(
                sprite.left + sprite.width * 0.5,
                sprite.bottom + sprite.height * 0.75,
            ),
            // outside
            cgmath::Point2::new(
                sprite.left - sprite.width * 0.25,
                sprite.bottom + sprite.height * 0.5,
            ),
            cgmath::Point2::new(
                sprite.left + sprite.width * 0.5,
                sprite.bottom - sprite.height * 0.25,
            ),
            cgmath::Point2::new(
                sprite.left + sprite.width * 1.25,
                sprite.bottom + sprite.height * 0.5,
            ),
            cgmath::Point2::new(
                sprite.left + sprite.width * 0.5,
                sprite.bottom + sprite.height * 1.25,
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
            0.0,
            0.0,
            1.0,
            1.0,
            0.0,
            [0.0, 0.0, 0.0, 1.0].into(),
            0,
        );
        test_containment(sprite);

        // tall, NE quadrant
        sprite.left = 10.0;
        sprite.bottom = 5.0;
        sprite.height = 50.0;
        sprite.width = 1.0;
        test_containment(sprite);

        // wide, NE quad
        sprite.left = 10.0;
        sprite.bottom = 5.0;
        sprite.height = 1.0;
        sprite.width = 50.0;
        test_containment(sprite);

        // tall, SE quadrant
        sprite.left = 10.0;
        sprite.bottom = -70.0;
        sprite.height = 50.0;
        sprite.width = 1.0;
        test_containment(sprite);

        // wide, SE quad
        sprite.left = 10.0;
        sprite.bottom = -10.0;
        sprite.height = 1.0;
        sprite.width = 50.0;
        test_containment(sprite);

        // tall, SW quadrant
        sprite.left = -100.0;
        sprite.bottom = -500.0;
        sprite.height = 50.0;
        sprite.width = 1.0;
        test_containment(sprite);

        // wide, SW quad
        sprite.left = -100.0;
        sprite.bottom = -500.0;
        sprite.height = 1.0;
        sprite.width = 50.0;
        test_containment(sprite);

        // tall, NW quadrant
        sprite.left = -100.0;
        sprite.bottom = 500.0;
        sprite.height = 50.0;
        sprite.width = 1.0;
        test_containment(sprite);

        // wide, NW quad
        sprite.left = -100.0;
        sprite.bottom = 500.0;
        sprite.height = 1.0;
        sprite.width = 50.0;
        test_containment(sprite);
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub struct SpriteHitTester {
    unit_sprites: HashMap<cgmath::Point2<i32>, SpriteDesc>,
    non_unit_sprites: Vec<SpriteDesc>,
}

impl SpriteHitTester {
    fn new(sprite_descs: &[SpriteDesc]) -> Self {
        let mut unit_sprites = HashMap::new();
        let mut non_unit_sprites = vec![];

        for sprite in sprite_descs {
            // copy sprites into appropriate storage
            if sprite.width == 1.0 && sprite.height == 1.0 {
                unit_sprites.insert(
                    cgmath::Point2::new(sprite.left as i32, sprite.bottom as i32),
                    *sprite,
                );
            } else {
                non_unit_sprites.push(*sprite);
            }
        }

        // sort non-unit sprites along x and (secondarily) y
        non_unit_sprites.sort_by(|a, b| {
            let ord_0 = a.left.partial_cmp(&b.left).unwrap();
            if ord_0 == std::cmp::Ordering::Equal {
                a.bottom.partial_cmp(&b.bottom).unwrap()
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
    pub fn test(&self, point: &cgmath::Point2<f32>, mask: u32) -> Option<SpriteDesc> {
        // first test the unit sprites
        if let Some(sprite) = self.unit_sprites.get(&cgmath::Point2::new(
            point.x.floor() as i32,
            point.y.floor() as i32,
        )) {
            if sprite.mask & mask != 0 {
                return Some(*sprite);
            }
        }

        // non_unit sprites are stored sorted along x, so we can early exit
        // TODO: Some kind of partitioning/binary search?

        for sprite in &self.non_unit_sprites {
            if sprite.left > point.x {
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
        let unit_0 = SpriteDesc::unit(
            SpriteShape::Square,
            0,
            0,
            1.0,
            [1.0, 1.0, 1.0, 1.0].into(),
            0,
        );
        let unit_1 = SpriteDesc::unit(
            SpriteShape::Square,
            11,
            -33,
            0.0,
            [0.0, 0.0, 0.0, 1.0].into(),
            0,
        );
        let non_unit_0 = SpriteDesc::new(
            SpriteShape::NorthEast,
            1.0,
            10.0,
            5.0,
            5.0,
            1.0,
            [1.0, 0.0, 0.0, 1.0].into(),
            0,
        );
        let non_unit_1 = SpriteDesc::new(
            SpriteShape::NorthEast,
            -1.0,
            -10.0,
            50.0,
            5.0,
            1.0,
            [1.0, 0.0, 0.0, 1.0].into(),
            0,
        );

        let hit_tester = SpriteHitTester::new(&[unit_0, unit_1, non_unit_0, non_unit_1]);
        assert_eq!(
            hit_tester
                .unit_sprites
                .get(&cgmath::Point2::new(
                    unit_0.left as i32,
                    unit_0.bottom as i32
                ))
                .unwrap(),
            &unit_0
        );
        assert_eq!(
            hit_tester
                .unit_sprites
                .get(&cgmath::Point2::new(
                    unit_1.left as i32,
                    unit_1.bottom as i32
                ))
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

        let sb1 = SpriteDesc::unit(
            SpriteShape::Square,
            0,
            0,
            10.0,
            [1.0, 1.0, 1.0, 1.0].into(),
            square_mask,
        );
        let sb2 = SpriteDesc::unit(
            SpriteShape::Square,
            -1,
            -1,
            10.0,
            [0.0, 0.0, 0.5, 1.0].into(),
            square_mask,
        );

        let tr0 = SpriteDesc::unit(
            SpriteShape::NorthEast,
            0,
            4,
            10.0,
            [0.0, 1.0, 1.0, 1.0].into(),
            triangle_mask,
        );
        let tr1 = SpriteDesc::unit(
            SpriteShape::NorthWest,
            -1,
            4,
            10.0,
            [1.0, 0.0, 1.0, 1.0].into(),
            triangle_mask,
        );
        let tr2 = SpriteDesc::unit(
            SpriteShape::SouthWest,
            -1,
            3,
            10.0,
            [0.0, 1.0, 0.0, 1.0].into(),
            triangle_mask,
        );
        let tr3 = SpriteDesc::unit(
            SpriteShape::SouthEast,
            0,
            3,
            10.0,
            [1.0, 1.0, 0.0, 1.0].into(),
            triangle_mask,
        );

        let hit_tester = SpriteHitTester::new(&[sb1, sb2, tr0, tr1, tr2, tr3]);

        // test triangle is hit only when using triangle_flags or all_mask
        assert!(hit_tester.test(&cgmath::Point2::new(0.1, 4.1), triangle_mask) == Some(tr0));
        assert!(hit_tester.test(&cgmath::Point2::new(-0.1, 4.1), triangle_mask) == Some(tr1));
        assert!(hit_tester.test(&cgmath::Point2::new(-0.1, 3.9), triangle_mask) == Some(tr2));
        assert!(hit_tester.test(&cgmath::Point2::new(0.1, 3.9), triangle_mask) == Some(tr3));
        assert!(hit_tester
            .test(&cgmath::Point2::new(0.1, 4.1), square_mask)
            .is_none());
        assert!(hit_tester
            .test(&cgmath::Point2::new(0.1, 3.9), all_mask)
            .is_some());

        // test square is only hit when mask is square or all_mask
        assert!(hit_tester.test(&cgmath::Point2::new(0.5, 0.5), square_mask) == Some(sb1));
        assert!(hit_tester
            .test(&cgmath::Point2::new(0.5, 0.5), triangle_mask)
            .is_none());
        assert!(hit_tester
            .test(&cgmath::Point2::new(0.5, 0.5), all_mask)
            .is_some());
    }

    #[test]
    fn non_unit_hit_test_works() {
        let mask0 = 1 << 0;
        let mask1 = 1 << 1;
        let mask2 = 1 << 2;
        let unused_mask = 1 << 16;
        let all_mask = mask0 | mask1 | mask2 | unused_mask;

        let b0 = SpriteDesc::new(
            SpriteShape::Square,
            -4.0,
            -4.0,
            8.0,
            4.0,
            0.0,
            [0.0, 0.0, 0.0, 1.0].into(),
            mask0,
        );
        let b1 = SpriteDesc::new(
            SpriteShape::Square,
            3.0,
            -1.0,
            3.0,
            1.0,
            0.0,
            [0.0, 0.0, 0.0, 1.0].into(),
            mask1,
        );
        let b2 = SpriteDesc::new(
            SpriteShape::Square,
            3.0,
            -2.0,
            2.0,
            5.0,
            0.0,
            [0.0, 0.0, 0.0, 1.0].into(),
            mask2,
        );
        let hit_tester = SpriteHitTester::new(&[b0, b1, b2]);

        // this point is in all three boxes
        let p = cgmath::Point2::new(3.5, -0.5);

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
        let tc_a = cgmath::vec2::<f32>(0.0, 0.0);
        let tc_b = cgmath::vec2::<f32>(1.0, 0.0);
        let tc_c = cgmath::vec2::<f32>(1.0, 1.0);
        let tc_d = cgmath::vec2::<f32>(0.0, 1.0);
        for sprite in sprites {
            let p_a = cgmath::vec3(sprite.left, sprite.bottom, sprite.z);
            let p_b = cgmath::vec3(sprite.left + sprite.width, sprite.bottom, sprite.z);
            let p_c = cgmath::vec3(
                sprite.left + sprite.width,
                sprite.bottom + sprite.height,
                sprite.z,
            );
            let p_d = cgmath::vec3(sprite.left, sprite.bottom + sprite.height, sprite.z);
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
