use cgmath::{prelude::*, relative_eq, vec2, vec3, Point2, Point3, Vector2, Vector3, Vector4};
use std::collections::HashMap;
use std::rc::Rc;

use crate::texture;
use crate::tileset;
use wgpu::util::DeviceExt;

// --------------------------------------------------------------------------------------------------------------------

pub fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
) -> wgpu::RenderPipeline {
    let vertex_descs = &[SpriteVertex::desc()];
    let vs_src = wgpu::include_spirv!("shaders/sprite.vs.spv");
    let fs_src = wgpu::include_spirv!("shaders/sprite.fs.spv");

    let vs_module = device.create_shader_module(vs_src);
    let fs_module = device.create_shader_module(fs_src);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            // Since we're rendering sprites, we don't care about backface culling
            front_face: wgpu::FrontFace::Cw,
            cull_mode: wgpu::CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            clamp_depth: false,
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: color_format,
            color_blend: wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha_blend: wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: depth_format.map(|format| wgpu::DepthStencilStateDescriptor {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilStateDescriptor::default(),
        }),
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: vertex_descs,
        },
    })
}

// --------------------------------------------------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct UniformData {
    model_position: cgmath::Vector4<f32>,
    color: cgmath::Vector4<f32>,
}

unsafe impl bytemuck::Pod for UniformData {}
unsafe impl bytemuck::Zeroable for UniformData {}

impl UniformData {
    pub fn new() -> Self {
        Self {
            model_position: cgmath::Vector4::zero(),
            color: cgmath::Vector4::new(1.0, 1.0, 1.0, 1.0),
        }
    }

    pub fn set_color(&mut self, color: &cgmath::Vector4<f32>) {
        self.color = *color;
    }

    pub fn set_model_position(&mut self, position: &cgmath::Point3<f32>) {
        self.model_position.x = position.x;
        self.model_position.y = position.y;
        self.model_position.z = position.z;
        self.model_position.w = 1.0;
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub struct Uniforms {
    pub data: UniformData,
    pub buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl Uniforms {
    pub fn new(device: &wgpu::Device) -> Self {
        let data = UniformData::new();

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Uniform Buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::UniformBuffer {
                    dynamic: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Sprite Uniform Bind Group Layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
            }],
            label: Some("Sprite Uniform Bind Group"),
        });

        Self {
            data,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn write(&self, queue: &mut wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.data]));
    }
}

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
pub enum SpriteCollisionShape {
    None,
    Square,
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

impl SpriteCollisionShape {
    pub fn flipped_horizontally(&self) -> Self {
        match self {
            SpriteCollisionShape::None => SpriteCollisionShape::None,
            SpriteCollisionShape::Square => SpriteCollisionShape::Square,
            SpriteCollisionShape::NorthEast => SpriteCollisionShape::NorthWest,
            SpriteCollisionShape::SouthEast => SpriteCollisionShape::SouthWest,
            SpriteCollisionShape::SouthWest => SpriteCollisionShape::SouthEast,
            SpriteCollisionShape::NorthWest => SpriteCollisionShape::NorthEast,
        }
    }
    pub fn flipped_vertically(&self) -> Self {
        match self {
            SpriteCollisionShape::None => SpriteCollisionShape::None,
            SpriteCollisionShape::Square => SpriteCollisionShape::Square,
            SpriteCollisionShape::NorthEast => SpriteCollisionShape::SouthEast,
            SpriteCollisionShape::SouthEast => SpriteCollisionShape::NorthEast,
            SpriteCollisionShape::SouthWest => SpriteCollisionShape::NorthWest,
            SpriteCollisionShape::NorthWest => SpriteCollisionShape::SouthWest,
        }
    }
    pub fn flipped_diagonally(&self) -> Self {
        // https://doc.mapeditor.org/en/stable/reference/tmx-map-format/#tile-flipping
        // Under section "Tile Flipping" diagonal flip is defined as x/y axis swap.
        // On paper, this transform was worked out for triangles. Since this is a
        // mirroring along the +x/+y diagonal axis, it only affects NorthWest and SouthEast
        // triangles, which are not symmetrical across the flip axis.
        match self {
            SpriteCollisionShape::None => SpriteCollisionShape::None,
            SpriteCollisionShape::Square => SpriteCollisionShape::Square,
            SpriteCollisionShape::NorthEast => SpriteCollisionShape::NorthEast,
            SpriteCollisionShape::SouthEast => SpriteCollisionShape::NorthWest,
            SpriteCollisionShape::SouthWest => SpriteCollisionShape::SouthWest,
            SpriteCollisionShape::NorthWest => SpriteCollisionShape::SouthEast,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SpriteDesc {
    pub collision_shape: SpriteCollisionShape,
    pub origin: Point3<f32>,
    pub extent: Vector2<f32>,
    pub tex_coord_origin: Point2<f32>,
    pub tex_coord_extent: Vector2<f32>,
    pub color: Vector4<f32>,
    pub mask: u32,
    flipped_diagonally: bool,
    flipped_horizontally: bool,
    flipped_vertically: bool,
}

impl PartialEq for SpriteDesc {
    fn eq(&self, other: &Self) -> bool {
        self.collision_shape == other.collision_shape
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

impl Eq for SpriteDesc {}

/// Simple corss product for 2D vectors; cgmath doesn't define this because cross product
/// doesn't make sense generally for 2D.
fn cross(a: &Vector2<f32>, b: &Vector2<f32>) -> f32 {
    a.x * b.y - a.y * b.x
}

impl SpriteDesc {
    /// Creates a new SpriteDesc at an arbitrary origin with a specified extent
    pub fn new(
        collision_shape: SpriteCollisionShape,
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
            flipped_diagonally: false,
            flipped_horizontally: false,
            flipped_vertically: false,
        }
    }

    /// Creates a 1x1 sprite at a given integral origin point.
    pub fn unit(
        collision_shape: SpriteCollisionShape,
        origin: Point2<i32>,
        z: f32,
        tex_coord_origin: Point2<f32>,
        tex_coord_extent: Vector2<f32>,
        color: Vector4<f32>,
        mask: u32,
    ) -> Self {
        Self {
            collision_shape,
            origin: Point3::new(origin.x as f32, origin.y as f32, z),
            extent: Vector2::new(1.0, 1.0),
            tex_coord_origin,
            tex_coord_extent,
            color,
            mask,
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
            let p = Vector2::new(point.x, point.y);
            return match self.collision_shape {
                SpriteCollisionShape::None => false,

                SpriteCollisionShape::Square => true,

                SpriteCollisionShape::NorthEast => {
                    let a = Vector2::new(self.origin.x, self.origin.y + self.extent.y);
                    let b = Vector2::new(self.origin.x + self.extent.x, self.origin.y);
                    let ba = b - a;
                    let pa = p - a;
                    cross(&ba, &pa) <= 0.0
                }

                SpriteCollisionShape::SouthEast => {
                    let a = Vector2::new(self.origin.x, self.origin.y);
                    let b =
                        Vector2::new(self.origin.x + self.extent.x, self.origin.y + self.extent.y);
                    let ba = b - a;
                    let pa = p - a;
                    cross(&ba, &pa) >= 0.0
                }

                SpriteCollisionShape::SouthWest => {
                    let a = Vector2::new(self.origin.x, self.origin.y + self.extent.y);
                    let b = Vector2::new(self.origin.x + self.extent.x, self.origin.y);
                    let ba = b - a;
                    let pa = p - a;
                    // opposite winding of northeast
                    cross(&ba, &pa) >= 0.0
                }

                SpriteCollisionShape::NorthWest => {
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
            collision_shape: self.collision_shape.flipped_horizontally(),
            origin: self.origin,
            extent: self.extent,
            tex_coord_origin: self.tex_coord_origin,
            tex_coord_extent: self.tex_coord_extent,
            color: self.color,
            mask: self.mask,
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
            flipped_diagonally: !self.flipped_diagonally,
            flipped_horizontally: self.flipped_horizontally,
            flipped_vertically: self.flipped_vertically,
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

        sprite.collision_shape = SpriteCollisionShape::None;
        assert!(!sprite.contains(&p0));
        assert!(!sprite.contains(&p1));
        assert!(!sprite.contains(&p2));
        assert!(!sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.collision_shape = SpriteCollisionShape::Square;
        assert!(sprite.contains(&p0));
        assert!(sprite.contains(&p1));
        assert!(sprite.contains(&p2));
        assert!(sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.collision_shape = SpriteCollisionShape::NorthEast;
        assert!(sprite.contains(&p0));
        assert!(sprite.contains(&p1));
        assert!(!sprite.contains(&p2));
        assert!(!sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.collision_shape = SpriteCollisionShape::SouthEast;
        assert!(sprite.contains(&p0));
        assert!(!sprite.contains(&p1));
        assert!(!sprite.contains(&p2));
        assert!(sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.collision_shape = SpriteCollisionShape::SouthWest;
        assert!(!sprite.contains(&p0));
        assert!(!sprite.contains(&p1));
        assert!(sprite.contains(&p2));
        assert!(sprite.contains(&p3));
        assert!(!sprite.contains(&p4));
        assert!(!sprite.contains(&p5));
        assert!(!sprite.contains(&p6));
        assert!(!sprite.contains(&p7));

        sprite.collision_shape = SpriteCollisionShape::NorthWest;
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
            SpriteCollisionShape::Square,
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
            SpriteCollisionShape::Square,
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
            SpriteCollisionShape::Square,
            Point2::new(0, 0),
            0.0,
            tco,
            tce,
            color,
            0,
        );
        let unit_1 = SpriteDesc::unit(
            SpriteCollisionShape::Square,
            Point2::new(11, -33),
            0.0,
            tco,
            tce,
            color,
            0,
        );
        let non_unit_0 = SpriteDesc::new(
            SpriteCollisionShape::Square,
            Point3::new(10.0, 5.0, 0.0),
            Vector2::new(5.0, 1.0),
            tco,
            tce,
            color,
            0,
        );

        let non_unit_1 = SpriteDesc::new(
            SpriteCollisionShape::Square,
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
            SpriteCollisionShape::Square,
            Point2::new(0, 0),
            10.0,
            tco,
            tce,
            color,
            square_mask,
        );

        let sb2 = SpriteDesc::unit(
            SpriteCollisionShape::Square,
            Point2::new(-1, -1),
            10.0,
            tco,
            tce,
            color,
            square_mask,
        );

        let tr0 = SpriteDesc::unit(
            SpriteCollisionShape::NorthEast,
            Point2::new(0, 4),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let tr1 = SpriteDesc::unit(
            SpriteCollisionShape::NorthWest,
            Point2::new(-1, 4),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let tr2 = SpriteDesc::unit(
            SpriteCollisionShape::SouthWest,
            Point2::new(-1, 3),
            10.0,
            tco,
            tce,
            color,
            triangle_mask,
        );

        let tr3 = SpriteDesc::unit(
            SpriteCollisionShape::SouthEast,
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
            SpriteCollisionShape::Square,
            Point3::new(-4.0, -4.0, 0.0),
            Vector2::new(8.0, 4.0),
            tco,
            tce,
            color,
            mask0,
        );

        let b1 = SpriteDesc::new(
            SpriteCollisionShape::Square,
            Point3::new(3.0, -1.0, 0.0),
            Vector2::new(3.0, 1.0),
            tco,
            tce,
            color,
            mask1,
        );

        let b2 = SpriteDesc::new(
            SpriteCollisionShape::Square,
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

            let mut tc_a = vec2::<f32>(sprite.tex_coord_origin.x, 1.0 - sprite.tex_coord_origin.y);
            let mut tc_b = vec2::<f32>(
                sprite.tex_coord_origin.x + sprite.tex_coord_extent.x,
                1.0 - (sprite.tex_coord_origin.y),
            );
            let mut tc_c = vec2::<f32>(
                sprite.tex_coord_origin.x + sprite.tex_coord_extent.x,
                1.0 - (sprite.tex_coord_origin.y + sprite.tex_coord_extent.y),
            );
            let mut tc_d = vec2::<f32>(
                sprite.tex_coord_origin.x,
                1.0 - (sprite.tex_coord_origin.y + sprite.tex_coord_extent.y),
            );

            if sprite.flipped_diagonally {
                std::mem::swap(&mut tc_a, &mut tc_c);
            }

            if sprite.flipped_horizontally {
                std::mem::swap(&mut tc_a, &mut tc_b);
                std::mem::swap(&mut tc_d, &mut tc_c);
            }

            if sprite.flipped_vertically {
                std::mem::swap(&mut tc_a, &mut tc_d);
                std::mem::swap(&mut tc_b, &mut tc_c);
            }

            let sv_a = SpriteVertex::new(p_a, tc_a, sprite.color);
            let sv_b = SpriteVertex::new(p_b, tc_b, sprite.color);
            let sv_c = SpriteVertex::new(p_c, tc_c, sprite.color);
            let sv_d = SpriteVertex::new(p_d, tc_d, sprite.color);
            let idx = vertices.len();

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

    pub fn draw<'a, 'b>(
        &'a self,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        material: &'a wgpu::BindGroup,
        camera_uniforms: &'a wgpu::BindGroup,
        sprite_uniforms: &'a wgpu::BindGroup,
    ) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..));
        render_pass.set_bind_group(0, &material, &[]);
        render_pass.set_bind_group(1, &camera_uniforms, &[]);
        render_pass.set_bind_group(2, &sprite_uniforms, &[]);
        render_pass.draw_indexed(0..self.num_elements, 0, 0..1);
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

    pub fn draw<'a, 'b>(
        &'a self,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        camera_uniforms: &'a wgpu::BindGroup,
        sprite_uniforms: &'a wgpu::BindGroup,
    ) {
        for mesh in &self.meshes {
            let material = &self.materials[mesh.material];
            mesh.draw(
                render_pass,
                &material.bind_group,
                camera_uniforms,
                sprite_uniforms,
            );
        }
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

pub struct SpriteEntity {
    // maps a string, e.g., "face_right" to a renderable mesh
    meshes_by_cycle: HashMap<String, SpriteMesh>,

    // TODO: this should be &sprite::SPriteMaterial so multiple entities can share a single spritesheet?
    material: Rc<SpriteMaterial>,
}

impl SpriteEntity {
    // Loads all tiles with the specified name from the tileset, gathering them by "cycle", populating
    // meshes_by_cycle accordingly.
    // REQUISITES:
    // All tiles part of an entity have a property "cycle"="some_noun" (e.g., "walk_1")
    // The root tile has property "role" = "root". All tiles will be placed relative to root, with root at (0,0)
    pub fn load(
        tileset: &tileset::TileSet,
        material: Rc<SpriteMaterial>,
        device: &wgpu::Device,
        named: &str,
        mask: u32,
    ) -> Self {
        let tiles = tileset.get_tiles_with_property("name", named);

        // collect all tiles for each cycle, and root tiles too
        let mut tiles_by_cycle: HashMap<&str, Vec<&tileset::Tile>> = HashMap::new();
        let mut root_tiles_by_cycle: HashMap<&str, &tileset::Tile> = HashMap::new();
        for tile in tiles {
            let cycle = tile.get_property("cycle").unwrap();
            tiles_by_cycle.entry(cycle).or_insert(Vec::new()).push(tile);

            if tile.get_property("role") == Some("root") {
                root_tiles_by_cycle.insert(cycle, tile);
            }
        }

        // now for each root tile, assemble SpriteDescs
        let mut sprite_descs_by_cycle: HashMap<&str, Vec<SpriteDesc>> = HashMap::new();
        for cycle in root_tiles_by_cycle.keys() {
            let root_tile = *root_tiles_by_cycle.get(cycle).unwrap();
            let tiles = tiles_by_cycle.get(cycle).unwrap();

            let root_position = tileset.get_tile_position(root_tile).cast::<i32>().unwrap();

            for tile in tiles {
                let tile_position = tileset.get_tile_position(tile).cast::<i32>().unwrap();

                let sprite_position = tile_position - root_position;

                let (tex_coords, tex_extents) = tileset.get_tex_coords_for_tile(tile);
                // now create a SpriteDesc at this position
                let sd = SpriteDesc::unit(
                    tile.shape(),
                    cgmath::Point2::new(sprite_position.x, -sprite_position.y),
                    0.0,
                    tex_coords,
                    tex_extents,
                    cgmath::vec4(1.0, 1.0, 1.0, 1.0),
                    mask,
                );

                sprite_descs_by_cycle
                    .entry(cycle)
                    .or_insert(Vec::new())
                    .push(sd);
            }
        }

        // now convert spritedescs into sprite meshes
        Self::new(&sprite_descs_by_cycle, material, device)
    }

    pub fn new(
        sprite_descs: &HashMap<&str, Vec<SpriteDesc>>,
        material: Rc<SpriteMaterial>,
        device: &wgpu::Device,
    ) -> Self {
        let mut sprite_states = HashMap::new();

        for key in sprite_descs.keys() {
            let descs = sprite_descs.get(key).unwrap();
            let mesh = SpriteMesh::new(descs, 0, device, key);
            sprite_states.insert(key.to_string(), mesh);
        }

        SpriteEntity {
            meshes_by_cycle: sprite_states,
            material,
        }
    }

    /// draws the mesh corresponding to "cycle"
    pub fn draw<'a, 'b>(
        &'a self,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        camera_uniforms: &'a wgpu::BindGroup,
        sprite_uniforms: &'a wgpu::BindGroup,
        cycle: &str,
    ) where
        'a: 'b,
    {
        if let Some(mesh) = self.meshes_by_cycle.get(cycle) {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..));
            render_pass.set_bind_group(0, &self.material.bind_group, &[]);
            render_pass.set_bind_group(1, &camera_uniforms, &[]);
            render_pass.set_bind_group(2, &sprite_uniforms, &[]);
            render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
        }
    }
}
