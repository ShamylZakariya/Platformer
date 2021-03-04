use cgmath::*;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::{sprite::core::*, util::*};

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

    pub fn contains(&self, point: &Point2<f32>) -> bool {
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
    pub fn line_intersection(&self, a: &Point2<f32>, b: &Point2<f32>) -> Option<Point2<f32>> {
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
    pub fn rect_intersection(
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
    pub fn unit_rect_intersection(&self, origin: &Point2<f32>, inset: f32, contact: bool) -> bool {
        self.rect_intersection(origin, &vec2(1.0, 1.0), inset, contact)
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
    static_colliders: HashMap<Point2<i32>, u32>,
    dynamic_colliders: HashSet<u32>,
    active_colliders: HashSet<u32>,
}

impl Space {
    pub fn new(colliders: &[Collider]) -> Self {
        let mut space = Self {
            colliders: Vec::new(),
            static_colliders: HashMap::new(),
            dynamic_colliders: HashSet::new(),
            active_colliders: HashSet::new(),
        };

        for c in colliders {
            space.add_collider(*c);
        }

        space
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    pub fn add_collider(&mut self, collider: Collider) -> u32 {
        let id = self.colliders.len() as u32;
        self.colliders.push(collider);
        self.active_colliders.insert(id);

        match collider.mode {
            Mode::Static { position } => {
                self.static_colliders.insert(position, id);
            }
            Mode::Dynamic { .. } => {
                self.dynamic_colliders.insert(id);
            }
        }

        id
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
                    self.dynamic_colliders.remove(&collider_id);
                }
            };
        }
    }

    pub fn activate_collider(&mut self, collider_id: u32) {
        if let Some(c) = self.colliders.get_mut(collider_id as usize) {
            self.active_colliders.insert(collider_id);
            match c.mode {
                Mode::Static { position } => {
                    self.static_colliders.insert(position, collider_id);
                }
                Mode::Dynamic { .. } => {
                    self.dynamic_colliders.insert(collider_id);
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
                    *position =
                        point2(new_position.x.floor() as i32, new_position.y.floor() as i32);
                }
                Mode::Dynamic { bounds, .. } => {
                    bounds.origin = new_position;
                }
            }
        }
    }

    pub fn update_collider_extent(&mut self, collider_id: u32, new_extent: Vector2<f32>) {
        if let Some(c) = self.colliders.get_mut(collider_id as usize) {
            match &mut c.mode {
                Mode::Static { .. } => panic!("Cannot change size of a static collider"),
                Mode::Dynamic { bounds, .. } => {
                    bounds.extent = new_extent;
                }
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    pub fn get_collider_at(&self, point: Point2<i32>, mask: u32) -> Option<&Collider> {
        let point_f = point2(point.x as f32 + 0.5, point.y as f32 + 0.5);
        for id in self.dynamic_colliders.iter() {
            let c = &self.colliders[*id as usize];
            if c.mask & mask != 0 && c.contains(&point_f) {
                return Some(c);
            }
        }
        if let Some(id) = self.static_colliders.get(&point) {
            let c = &self.colliders[*id as usize];
            if c.mask & mask != 0 && c.contains(&point_f) {
                return Some(c);
            }
        }
        None
    }

    fn get_static_collider_at(&self, point: Point2<i32>, mask: u32) -> Option<&Collider> {
        let point_f = point2(point.x as f32 + 0.5, point.y as f32 + 0.5);
        if let Some(id) = self.static_colliders.get(&point) {
            let c = &self.colliders[*id as usize];
            if c.mask & mask != 0 && c.contains(&point_f) {
                return Some(c);
            }
        }
        None
    }

    /// Tests the specified rect against colliders, invoking the specified callback for each contact/overlap.
    /// Filters by mask, such that only sprites with matching mask bits will be matched.
    /// In the case of overlapping sprites, dynamic sprites will be passed to the callback before static.
    /// The callback should return true to end the search, false to continue.
    pub fn test_rect<C>(
        &self,
        origin: &Point2<f32>,
        extent: &Vector2<f32>,
        mask: u32,
        mut callback: C,
    ) where
        C: FnMut(&Collider) -> Sentinel,
    {
        for id in self.dynamic_colliders.iter() {
            let c = &self.colliders[*id as usize];
            if c.mask & mask != 0
                && c.rect_intersection(origin, extent, 0.0, true)
                && matches!(callback(c), Sentinel::Stop)
            {
                return;
            }
        }

        let snapped_extent = vec2(extent.x.round() as i32, extent.y.round() as i32);
        let a = point2(origin.x.floor() as i32, origin.y.floor() as i32);
        let b = point2(a.x + snapped_extent.x, a.y);
        let c = point2(a.x, a.y + snapped_extent.y);
        let d = point2(a.x + snapped_extent.x, a.y + snapped_extent.y);

        for p in [a, b, c, d].iter() {
            if let Some(c) = self.get_static_collider_at(*p, mask) {
                if c.rect_intersection(origin, extent, 0.0, true)
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
        for id in self.dynamic_colliders.iter() {
            let c = &self.colliders[*id as usize];
            if c.mask & mask != 0 && c.rect_intersection(origin, extent, 0.0, true) {
                return Some(c);
            }
        }

        let snapped_extent = vec2(extent.x.round() as i32, extent.y.round() as i32);
        let a = point2(origin.x.floor() as i32, origin.y.floor() as i32);
        let b = point2(a.x + snapped_extent.x, a.y);
        let c = point2(a.x, a.y + snapped_extent.y);
        let d = point2(a.x + snapped_extent.x, a.y + snapped_extent.y);

        for p in [a, b, c, d].iter() {
            if let Some(c) = self.get_static_collider_at(*p, mask) {
                if c.rect_intersection(origin, extent, 0.0, true) {
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
    pub fn test_point_first(&self, point: Point2<f32>, mask: u32) -> Option<&Collider> {
        for id in self.dynamic_colliders.iter() {
            let c = &self.colliders[*id as usize];
            if c.mask & mask != 0 && c.contains(&point) {
                return Some(c);
            }
        }

        if let Some(id) = self
            .static_colliders
            .get(&point2(point.x.floor() as i32, point.y.floor() as i32))
        {
            let c = &self.colliders[*id as usize];
            if c.mask & mask != 0 && c.contains(&point) {
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
        assert!(hit_tester.test_point_first(point2(0.1, 4.1), triangle_mask) == Some(&tr0));
        assert!(hit_tester.test_point_first(point2(-0.1, 4.1), triangle_mask) == Some(&tr1));
        assert!(hit_tester.test_point_first(point2(-0.1, 3.9), triangle_mask) == Some(&tr2));
        assert!(hit_tester.test_point_first(point2(0.1, 3.9), triangle_mask) == Some(&tr3));
        assert!(hit_tester
            .test_point_first(point2(0.1, 4.1), square_mask)
            .is_none());
        assert!(hit_tester
            .test_point_first(point2(0.1, 3.9), all_mask)
            .is_some());

        // test square is only hit when mask is square or all_mask
        assert!(hit_tester.test_point_first(point2(0.5, 0.5), square_mask) == Some(&sb1));
        assert!(hit_tester
            .test_point_first(point2(0.5, 0.5), triangle_mask)
            .is_none());
        assert!(hit_tester
            .test_point_first(point2(0.5, 0.5), all_mask)
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
        assert!(!collider.contains(&p0));
        assert!(!collider.contains(&p1));
        assert!(!collider.contains(&p2));
        assert!(!collider.contains(&p3));
        assert!(!collider.contains(&p4));
        assert!(!collider.contains(&p5));
        assert!(!collider.contains(&p6));
        assert!(!collider.contains(&p7));

        collider.shape = Shape::Square;
        assert!(collider.contains(&p0));
        assert!(collider.contains(&p1));
        assert!(collider.contains(&p2));
        assert!(collider.contains(&p3));
        assert!(!collider.contains(&p4));
        assert!(!collider.contains(&p5));
        assert!(!collider.contains(&p6));
        assert!(!collider.contains(&p7));

        collider.shape = Shape::NorthEast;
        assert!(collider.contains(&p0));
        assert!(collider.contains(&p1));
        assert!(!collider.contains(&p2));
        assert!(!collider.contains(&p3));
        assert!(!collider.contains(&p4));
        assert!(!collider.contains(&p5));
        assert!(!collider.contains(&p6));
        assert!(!collider.contains(&p7));

        collider.shape = Shape::SouthEast;
        assert!(collider.contains(&p0));
        assert!(!collider.contains(&p1));
        assert!(!collider.contains(&p2));
        assert!(collider.contains(&p3));
        assert!(!collider.contains(&p4));
        assert!(!collider.contains(&p5));
        assert!(!collider.contains(&p6));
        assert!(!collider.contains(&p7));

        collider.shape = Shape::SouthWest;
        assert!(!collider.contains(&p0));
        assert!(!collider.contains(&p1));
        assert!(collider.contains(&p2));
        assert!(collider.contains(&p3));
        assert!(!collider.contains(&p4));
        assert!(!collider.contains(&p5));
        assert!(!collider.contains(&p6));
        assert!(!collider.contains(&p7));

        collider.shape = Shape::NorthWest;
        assert!(!collider.contains(&p0));
        assert!(collider.contains(&p1));
        assert!(collider.contains(&p2));
        assert!(!collider.contains(&p3));
        assert!(!collider.contains(&p4));
        assert!(!collider.contains(&p5));
        assert!(!collider.contains(&p6));
        assert!(!collider.contains(&p7));
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
            collider.line_intersection(&point2(-0.5, 0.5), &point2(0.5, 0.5)),
            Some(point2(0.0, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, 1.5), &point2(0.5, 0.5)),
            Some(point2(0.5, 1.0))
        );
        assert_eq!(
            collider.line_intersection(&point2(1.5, 0.5), &point2(0.5, 0.5)),
            Some(point2(1.0, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, -0.5), &point2(0.5, 0.5)),
            Some(point2(0.5, 0.0))
        );
    }

    #[test]
    fn line_intersection_with_slopes_works() {
        let mut collider = Collider::new_static((0, 0).into(), Shape::NorthEast, 0);

        assert_eq!(
            collider.line_intersection(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.0, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.0))
        );

        collider.shape = Shape::SouthEast;
        assert_eq!(
            collider.line_intersection(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.0, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 1.0))
        );
        assert_eq!(
            collider.line_intersection(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.5))
        );

        collider.shape = Shape::SouthWest;
        assert_eq!(
            collider.line_intersection(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 1.0))
        );
        assert_eq!(
            collider.line_intersection(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(1.0, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.5))
        );

        collider.shape = Shape::NorthWest;
        assert_eq!(
            collider.line_intersection(&point2(-0.5, 0.5), &point2(1.5, 0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, 1.5), &point2(0.5, -0.5)),
            Some(point2(0.5, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(1.5, 0.5), &point2(-0.5, 0.5)),
            Some(point2(1.0, 0.5))
        );
        assert_eq!(
            collider.line_intersection(&point2(0.5, -0.5), &point2(0.5, 1.5)),
            Some(point2(0.5, 0.0))
        );
    }

    #[test]
    fn rect_intersection_works() {}
}
