use cgmath::*;

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
}

pub mod intersection {
    use super::*;

    // https://www.swtestacademy.com/intersection-convex-polygons-algorithm/

    /// Returns true if the two rectangle
    pub fn rect_rect_intersects(rect_a: Bounds, rect_b: Bounds) -> bool {
        let (x_overlap, y_overlap) = {
            (
                rect_a.origin.x <= rect_b.origin.x + rect_b.extent.x
                    && rect_a.origin.x + rect_a.extent.x >= rect_b.origin.x,
                rect_a.origin.y <= rect_b.origin.y + rect_b.extent.y
                    && rect_a.origin.y + rect_a.extent.y >= rect_b.origin.y,
            )
        };

        x_overlap && y_overlap
    }

    /// Return the intersection of two line segments, or None if they don't intersect
    pub fn line_line(
        l1p1: &Point2<f32>,
        l1p2: &Point2<f32>,
        l2p1: &Point2<f32>,
        l2p2: &Point2<f32>,
    ) -> Option<Point2<f32>> {
        let e = 1e-4 as f32;
        let a1 = l1p2.y - l1p1.y;
        let b1 = l1p1.x - l1p2.x;
        let c1 = a1 * l1p1.x + b1 * l1p1.y;

        let a2 = l2p2.y - l2p1.y;
        let b2 = l2p1.x - l2p2.x;
        let c2 = a2 * l2p1.x + b2 * l2p1.y;

        let det = a1 * b2 - a2 * b1;
        if det.abs() < e {
            //parallel lines
            return None;
        } else {
            let x = (b2 * c1 - b1 * c2) / det;
            let y = (a1 * c2 - a2 * c1) / det;

            let min_x = l1p1.x.min(l1p2.x);
            let max_x = l1p1.x.max(l1p2.x);
            let min_y = l1p1.y.min(l1p2.y);
            let max_y = l1p1.y.max(l1p2.y);
            let online1 = (min_x < x || (min_x - x).abs() < e)
                && (max_x > x || (max_x - x).abs() < e)
                && (min_y < y || (min_y - y).abs() < e)
                && (max_y > y || (max_y - y).abs() < e);

            let min_x = l2p1.x.min(l2p2.x);
            let max_x = l2p1.x.max(l2p2.x);
            let min_y = l2p1.y.min(l2p2.y);
            let max_y = l2p1.y.max(l2p2.y);
            let online2 = (min_x < x || (min_x - x).abs() < e)
                && (max_x > x || (max_x - x).abs() < e)
                && (min_y < y || (min_y - y).abs() < e)
                && (max_y > y || (max_y - y).abs() < e);

            if online1 && online2 {
                return Some(point2(x, y));
            }
        }

        None
    }

    /// Return the intersection(s) of a line segment with the perimeter of a convex polygon.
    /// Winding direction is unimportant.
    pub fn line_convex_poly(
        a: &Point2<f32>,
        b: &Point2<f32>,
        convex_poly: &[Point2<f32>],
    ) -> Vec<Point2<f32>> {
        let mut intersections = vec![];
        for i in 0..convex_poly.len() {
            let next = (i + 1) % convex_poly.len();
            if let Some(p) = line_line(a, b, &convex_poly[i], &convex_poly[next]) {
                intersections.push(p);
            }
        }
        intersections
    }

    /// If the line a->b intersects the convex polygon, returns the intersection closest to a
    pub fn line_convex_poly_closest(
        a: &Point2<f32>,
        b: &Point2<f32>,
        convex_poly: &[Point2<f32>],
    ) -> Option<Point2<f32>> {
        let mut intersections = vec![];
        for i in 0..convex_poly.len() {
            let next = (i + 1) % convex_poly.len();
            if let Some(p) = line_line(a, b, &convex_poly[i], &convex_poly[next]) {
                intersections.push(p);
            }
        }
        intersections.sort_by(|m, n| {
            let m_a = m.distance2(*a);
            let n_a = n.distance2(*a);
            m_a.partial_cmp(&n_a).unwrap()
        });
        if let Some(p) = intersections.first() {
            Some(*p)
        } else {
            None
        }
    }

    #[cfg(test)]
    mod intersection_tests {
        use super::*;

        #[test]
        fn rect_rect_intersects_works() {
            let a = Bounds::new(point2(1.0, 1.0), vec2(1.0, 1.0));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(0.5, 0.0), vec2(1.0, 1.0))
            ));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(1.0, 0.0), vec2(1.0, 1.0))
            ));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(1.0, 0.5), vec2(1.0, 1.0))
            ));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(1.0, 1.0), vec2(1.0, 1.0))
            ));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(0.5, 1.0), vec2(1.0, 1.0))
            ));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(0.0, 1.0), vec2(1.0, 1.0))
            ));
            assert!(rect_rect_intersects(
                a,
                Bounds::new(point2(0.5, 1.0), vec2(1.0, 1.0))
            ));

            assert!(!rect_rect_intersects(
                a,
                Bounds::new(point2(3.0, 0.0), vec2(1.0, 1.0))
            ));
            assert!(!rect_rect_intersects(
                a,
                Bounds::new(point2(0.0, 3.0), vec2(1.0, 1.0))
            ));
            assert!(!rect_rect_intersects(
                a,
                Bounds::new(point2(-2.0, 0.0), vec2(1.0, 1.0))
            ));
            assert!(!rect_rect_intersects(
                a,
                Bounds::new(point2(-2.0, -2.0), vec2(1.0, 1.0))
            ));
        }

        #[test]
        fn line_line_works() {
            assert_eq!(
                line_line(
                    &point2(0.0, 0.0),
                    &point2(10.0, 0.0),
                    &point2(2.0, 1.0),
                    &point2(2.0, -1.0),
                ),
                Some(point2(2.0, 0.0))
            );

            assert_eq!(
                line_line(
                    &point2(0.0, 0.0),
                    &point2(10.0, 10.0),
                    &point2(5.0, 10.0),
                    &point2(5.0, 0.0),
                ),
                Some(point2(5.0, 5.0))
            );

            assert_eq!(
                line_line(
                    &point2(0.0, 0.0),
                    &point2(10.0, 10.0),
                    &point2(0.0, 1.0),
                    &point2(10.0, 11.0),
                ),
                None
            );
        }

        #[test]
        fn line_convex_poly_works() {
            let square = vec![
                point2(0.0, 0.0),
                point2(1.0, 0.0),
                point2(1.0, 1.0),
                point2(0.0, 1.0),
            ];

            assert_eq!(
                line_convex_poly(&point2(-1.0, 0.5), &point2(0.5, 0.5), &square),
                vec![point2(0.0, 0.5)]
            );
            assert_eq!(
                line_convex_poly(&point2(2.0, 0.5), &point2(0.5, 0.5), &square),
                vec![point2(1.0, 0.5)]
            );
            assert_eq!(
                line_convex_poly(&point2(0.5, 2.0), &point2(0.5, 0.5), &square),
                vec![point2(0.5, 1.0)]
            );
            assert_eq!(
                line_convex_poly(&point2(0.5, -1.0), &point2(0.5, 0.5), &square),
                vec![point2(0.5, 0.0)]
            );

            let triangle = vec![point2(0.0, 0.0), point2(1.0, 0.0), point2(0.0, 1.0)];

            assert_eq!(
                line_convex_poly(&point2(0.5, 1.0), &point2(0.5, 0.01), &triangle),
                vec![point2(0.5, 0.5)]
            );
        }
    }
}
