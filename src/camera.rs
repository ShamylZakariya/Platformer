use cgmath::*;
use std::time::Duration;
use wgpu::util::DeviceExt;
use winit::dpi::LogicalPosition;
use winit::event::*;

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
    pub position: Point3<f32>,
    pub look_dir: Vector3<f32>,
}

impl Camera {
    pub fn new<P: Into<Point3<f32>>, V: Into<Vector3<f32>>>(position: P, look_dir: V) -> Self {
        Self {
            position: position.into(),
            look_dir: look_dir.into(),
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_dir(self.position, self.look_dir.normalize(), Vector3::unit_y())
    }
}

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

#[derive(Debug)]
struct CameraControllerInputState {
    move_left_pressed: bool,
    move_right_pressed: bool,
    move_up_pressed: bool,
    move_down_pressed: bool,
    zoom_in_pressed: bool,
    zoom_out_pressed: bool,
}

impl Default for CameraControllerInputState {
    fn default() -> Self {
        Self {
            move_left_pressed: false,
            move_right_pressed: false,
            move_up_pressed: false,
            move_down_pressed: false,
            zoom_in_pressed: false,
            zoom_out_pressed: false,
        }
    }
}

fn input_accumulator(negative: bool, positive: bool) -> f32 {
    return if negative { -1.0 } else { 0.0 } + if positive { 1.0 } else { 0.0 };
}

#[derive(Debug)]
pub struct CameraController {
    delta_scale: f32,
    speed: f32,
    input_state: CameraControllerInputState,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            delta_scale: 1.0,
            speed,
            input_state: Default::default(),
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.input_state.move_up_pressed = pressed;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.input_state.move_down_pressed = pressed;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.input_state.move_left_pressed = pressed;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.input_state.move_right_pressed = pressed;
                true
            }
            VirtualKeyCode::E => {
                self.input_state.zoom_in_pressed = pressed;
                true
            }
            VirtualKeyCode::Q => {
                self.input_state.zoom_out_pressed = pressed;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, _mouse_dx: f64, _mouse_dy: f64) {}

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.delta_scale = match delta {
            MouseScrollDelta::LineDelta(_, scroll) => *scroll * 50.0,
            MouseScrollDelta::PixelDelta(LogicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }

    pub fn update_camera(
        &mut self,
        camera: &mut Camera,
        projection: &mut Projection,
        dt: Duration,
    ) {
        let dt = dt.as_secs_f32();
        let delta_position = cgmath::Vector3::new(
            input_accumulator(
                self.input_state.move_left_pressed,
                self.input_state.move_right_pressed,
            ),
            input_accumulator(
                self.input_state.move_down_pressed,
                self.input_state.move_up_pressed,
            ),
            0.0,
        );
        let delta_zoom = input_accumulator(
            self.input_state.zoom_out_pressed,
            self.input_state.zoom_in_pressed,
        );

        camera.position += delta_position * self.speed * dt;
        projection.set_scale(projection.scale + delta_zoom * self.speed * dt);
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Uniforms {
    // use vec4 for 16-byte spacing requirement
    view_position: cgmath::Vector4<f32>,
    view_proj: cgmath::Matrix4<f32>,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            view_position: Zero::zero(),
            view_proj: cgmath::Matrix4::identity(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_position = camera.position.to_homogeneous(); // converts to vec4
        self.view_proj = projection.calc_matrix() * camera.calc_matrix();
    }

    pub fn create_resources(
        &self,
        device: &wgpu::Device,
    ) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[*self]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Camera Uniform Bind Group Layout"),
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniform_buffer.slice(..)),
            }],
            label: Some("Camera Uniform Bind Group"),
        });

        (
            uniform_buffer,
            uniform_bind_group_layout,
            uniform_bind_group,
        )
    }
}
