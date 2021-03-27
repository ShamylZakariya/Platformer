use cgmath::*;
use std::time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::*;

use crate::{
    state::constants::{MAX_CAMERA_SCALE, MIN_CAMERA_SCALE},
    util::{Bounds, UniformWrapper},
};

// ---------------------------------------------------------------------------------------------------------------------

// CGMath uses an OpenGL clipspace of [-1,+1] on z, where wgpu uses [0,+1] for z
// We need to scale and translate the cgmath clipspace to wgpu's. Note we're also
// flipping X, giving us a coordinate system with +x to right, +z into screen, and +y up.
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    -1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[derive(Debug)]
pub struct Camera {
    position: Point3<f32>,
    pub look_dir: Vector3<f32>,
    pixels_per_unit: Option<Vector2<f32>>,
}

impl Camera {
    pub fn new<P: Into<Point3<f32>>, V: Into<Vector3<f32>>>(
        position: P,
        look_dir: V,
        pixels_per_unit: Option<Vector2<f32>>,
    ) -> Self {
        Self {
            position: position.into(),
            look_dir: look_dir.into(),
            pixels_per_unit: pixels_per_unit,
        }
    }

    pub fn position(&self) -> Point3<f32> {
        if let Some(ppu) = self.pixels_per_unit {
            let cx = (self.position.x * ppu.x).ceil() / ppu.x;
            let cy = (self.position.y * ppu.y).ceil() / ppu.y;
            point3(cx, cy, self.position.z)
        } else {
            self.position
        }
    }

    pub fn set_position(&mut self, position: Point3<f32>) {
        self.position = position;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_to_rh(
            self.position(),
            self.look_dir.normalize(),
            Vector3::unit_y(),
        )
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct Projection {
    width: f32,
    height: f32,
    aspect: f32,
    scale: f32,
    near: f32,
    far: f32,
}

impl Projection {
    pub fn new(width: u32, height: u32, scale: f32, near: f32, far: f32) -> Self {
        Self {
            width: width as f32,
            height: height as f32,
            aspect: width as f32 / height as f32,
            scale,
            near,
            far,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width as f32;
        self.height = height as f32;
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        let width = self.scale;
        let height = self.scale / self.aspect;
        OPENGL_TO_WGPU_MATRIX
            * ortho(
                -width / 2.0,
                width / 2.0,
                -height / 2.0,
                height / 2.0,
                self.near,
                self.far,
            )
    }

    pub fn aspect(&self) -> f32 {
        self.aspect
    }

    pub fn size(&self) -> Vector2<f32> {
        vec2(self.width, self.height)
    }

    pub fn viewport_size(&self) -> Vector2<f32> {
        vec2(self.scale, self.scale / self.aspect)
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
        if self.scale < 0.00001 {
            self.scale = 0.00001;
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct UniformData {
    // use vec4 for 16-byte spacing requirement
    position: Vector4<f32>,
    view_proj: Matrix4<f32>,
    framebuffer_size: Vector2<f32>,
}

unsafe impl bytemuck::Pod for UniformData {}
unsafe impl bytemuck::Zeroable for UniformData {}

impl Default for UniformData {
    fn default() -> Self {
        Self {
            position: Zero::zero(),
            view_proj: Matrix4::identity(),
            framebuffer_size: Zero::zero(),
        }
    }
}

impl UniformData {
    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) -> &mut Self {
        self.position = camera.position().to_homogeneous(); // converts to vec4
        self.view_proj = projection.calc_matrix() * camera.calc_matrix();
        self.framebuffer_size = vec2(projection.width, projection.height);
        self
    }
}

/// Specialization of util::UniformWrapper for camera uniform storage
pub type Uniforms = crate::util::UniformWrapper<UniformData>;

// ---------------------------------------------------------------------------------------------------------------------

pub struct CameraController {
    pub camera: Camera,
    pub projection: Projection,
    pub uniforms: UniformWrapper<UniformData>,
}

impl CameraController {
    pub fn new(
        camera: Camera,
        projection: Projection,
        uniforms: UniformWrapper<UniformData>,
    ) -> Self {
        Self {
            camera,
            projection,
            uniforms,
        }
    }

    pub fn process_keyboard(&mut self, _key: VirtualKeyCode, _state: ElementState) -> bool {
        false
    }

    pub fn mouse_movement(&mut self, pressed: bool, _position: Point2<f32>, delta: Vector2<f32>) {
        // FIXME: there's some weirdness about position/delta - they don't really correlate to pixels, I think
        // there's some peculiar scaling thing going on.
        if pressed {
            let delta = (delta * 0.125) / self.projection.scale;
            self.camera.position.x -= delta.x;
            self.camera.position.y += delta.y;
        }
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        let delta_scale = match delta {
            MouseScrollDelta::LineDelta(_, scroll) => *scroll * 0.05,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => {
                *scroll as f32 * -0.05
            }
        };
        let new_scale = self.projection.scale + delta_scale * self.projection.scale;
        let new_scale = new_scale.min(MAX_CAMERA_SCALE).max(MIN_CAMERA_SCALE);
        self.projection.set_scale(new_scale);
    }

    pub fn update(
        &mut self,
        _dt: Duration,
        tracking: Option<Point2<f32>>,
        offset: Option<Vector2<f32>>,
        bounds: Option<Bounds>,
    ) {
        if let Some(tracking) = tracking {
            self.camera.position.x = tracking.x;
            self.camera.position.y = tracking.y;
        }

        if let Some(bounds) = bounds {
            self.clamp_camera_position_to_bounds(bounds);
        }

        if let Some(offset) = offset {
            self.camera.position.x += offset.x;
            self.camera.position.y += offset.y;
        }

        self.uniforms
            .data
            .update_view_proj(&self.camera, &self.projection);
    }

    /// Return the bounds of the camera viewport expressed as (bottom_left,extent)
    pub fn viewport_bounds(&self, inset_by: f32) -> Bounds {
        let viewport_size = vec2(
            self.projection.scale - 2.0 * inset_by,
            (self.projection.scale / self.projection.aspect) - 2.0 * inset_by,
        );
        let bottom_left = point2(
            self.camera.position.x - viewport_size.x / 2.0,
            self.camera.position.y - viewport_size.y / 2.0,
        );
        Bounds::new(bottom_left, viewport_size)
    }

    fn clamp_camera_position_to_bounds(&mut self, bounds: Bounds) {
        let viewport_size = vec2(
            self.projection.scale,
            self.projection.scale / self.projection.aspect,
        );
        self.camera.position.x = self
            .camera
            .position
            .x
            .max(bounds.origin.x + viewport_size.x * 0.5)
            .min(bounds.origin.x + bounds.extent.x - viewport_size.x * 0.5);

        self.camera.position.y = self
            .camera
            .position
            .y
            .max(bounds.origin.y + 1.0 + viewport_size.y * 0.5)
            .min(bounds.origin.y + 1.0 + bounds.extent.y - viewport_size.y * 0.5);
    }
}
