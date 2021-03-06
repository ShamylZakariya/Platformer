use std::hash::Hash;

use cgmath::*;

pub fn rel_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < f32::EPSILON
}

/// Simple cross product for 2D vectors; cgmath doesn't define this because cross product
/// doesn't make sense generally for 2D.
pub fn cross(a: &Vector2<f32>, b: &Vector2<f32>) -> f32 {
    a.x * b.y - a.y * b.x
}

pub fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}

pub fn hermite(t: f32) -> f32 {
    let t = t.min(1.0).max(0.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn clamp(v: f32, min: f32, max: f32) -> f32 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

pub fn hash_point2<H: std::hash::Hasher>(point: &Point2<f32>, state: &mut H) {
    ((point.x * 1000.0) as i32).hash(state);
    ((point.y * 1000.0) as i32).hash(state);
}

pub fn hash_point3<H: std::hash::Hasher>(point: &Point3<f32>, state: &mut H) {
    ((point.x * 1000.0) as i32).hash(state);
    ((point.y * 1000.0) as i32).hash(state);
    ((point.z * 1000.0) as i32).hash(state);
}

pub fn hash_vec2<H: std::hash::Hasher>(v: &Vector2<f32>, state: &mut H) {
    ((v.x * 1000.0) as i32).hash(state);
    ((v.y * 1000.0) as i32).hash(state);
}

pub fn hash_vec3<H: std::hash::Hasher>(v: &Vector3<f32>, state: &mut H) {
    ((v.x * 1000.0) as i32).hash(state);
    ((v.y * 1000.0) as i32).hash(state);
    ((v.z * 1000.0) as i32).hash(state);
}

pub fn hash_vec4<H: std::hash::Hasher>(v: &Vector4<f32>, state: &mut H) {
    ((v.x * 1000.0) as i32).hash(state);
    ((v.y * 1000.0) as i32).hash(state);
    ((v.z * 1000.0) as i32).hash(state);
    ((v.w * 1000.0) as i32).hash(state);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub origin: Point2<f32>,
    pub extent: Vector2<f32>,
}

impl Eq for Bounds {}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            origin: point2(0.0, 0.0),
            extent: vec2(0.0, 0.0),
        }
    }
}

impl Bounds {
    pub fn new(origin: Point2<f32>, extent: Vector2<f32>) -> Self {
        Self { origin, extent }
    }

    pub fn right(&self) -> f32 {
        self.origin.x + self.extent.x
    }
    pub fn top(&self) -> f32 {
        self.origin.y + self.extent.y
    }
    pub fn left(&self) -> f32 {
        self.origin.x
    }
    pub fn bottom(&self) -> f32 {
        self.origin.y
    }
    pub fn width(&self) -> f32 {
        self.extent.x
    }
    pub fn height(&self) -> f32 {
        self.extent.y
    }
    pub fn inset(&self, by: Vector2<f32>) -> Bounds {
        Bounds::new(self.origin + by * 0.5, self.extent - by)
    }
}
