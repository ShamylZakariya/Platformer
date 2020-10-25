use cgmath::*;
use std::time::Duration;
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
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug)]
pub struct CameraController {
    delta_position: Vector3<f32>,
    delta_scale: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            delta_position: Zero::zero(),
            delta_scale: 1.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.delta_position.y = amount;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.delta_position.y = -amount;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.delta_position.x = -amount;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.delta_position.x = amount;
                true
            }
            VirtualKeyCode::E => {
                self.delta_position.z = amount;
                true
            }
            VirtualKeyCode::Q => {
                self.delta_position.z = -amount;
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

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();
        camera.position += self.delta_position * self.speed * dt;
        println!(
            "camera.position: {:?} look_dir: {:?}",
            camera.position, camera.look_dir
        );
    }
}
