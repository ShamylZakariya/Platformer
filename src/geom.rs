// https://www.swtestacademy.com/intersection-convex-polygons-algorithm/

pub mod intersection {
    use cgmath::{MetricSpace, Point2};

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
                return Some(Point2::new(x, y));
            }
        }

        None
    }

    /// Return the intersection(s) of a line segment with the perimeter of a convex polygon.
    /// Winding direction is unimportant.
    pub fn line_convex_poly(
        a: &Point2<f32>,
        b: &Point2<f32>,
        convex_poly: &Vec<Point2<f32>>,
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
        convex_poly: &Vec<Point2<f32>>,
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
        fn line_line_works() {
            assert_eq!(
                line_line(
                    &Point2::new(0.0, 0.0),
                    &Point2::new(10.0, 0.0),
                    &Point2::new(2.0, 1.0),
                    &Point2::new(2.0, -1.0),
                ),
                Some(Point2::new(2.0, 0.0))
            );

            assert_eq!(
                line_line(
                    &Point2::new(0.0, 0.0),
                    &Point2::new(10.0, 10.0),
                    &Point2::new(5.0, 10.0),
                    &Point2::new(5.0, 0.0),
                ),
                Some(Point2::new(5.0, 5.0))
            );

            assert_eq!(
                line_line(
                    &Point2::new(0.0, 0.0),
                    &Point2::new(10.0, 10.0),
                    &Point2::new(0.0, 1.0),
                    &Point2::new(10.0, 11.0),
                ),
                None
            );
        }

        #[test]
        fn line_convex_poly_works() {
            let square = vec![
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(1.0, 1.0),
                Point2::new(0.0, 1.0),
            ];

            assert_eq!(
                line_convex_poly(&Point2::new(-1.0, 0.5), &Point2::new(0.5, 0.5), &square),
                vec![Point2::new(0.0, 0.5)]
            );
            assert_eq!(
                line_convex_poly(&Point2::new(2.0, 0.5), &Point2::new(0.5, 0.5), &square),
                vec![Point2::new(1.0, 0.5)]
            );
            assert_eq!(
                line_convex_poly(&Point2::new(0.5, 2.0), &Point2::new(0.5, 0.5), &square),
                vec![Point2::new(0.5, 1.0)]
            );
            assert_eq!(
                line_convex_poly(&Point2::new(0.5, -1.0), &Point2::new(0.5, 0.5), &square),
                vec![Point2::new(0.5, 0.0)]
            );

            let triangle = vec![
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(0.0, 1.0),
            ];

            assert_eq!(
                line_convex_poly(&Point2::new(0.5, 1.0), &Point2::new(0.5, 0.01), &triangle),
                vec![Point2::new(0.5, 0.5)]
            );
        }
    }
}
