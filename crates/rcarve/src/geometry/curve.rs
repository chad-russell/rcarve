use kurbo::{BezPath, Circle, Line, PathEl, Rect, Shape as KurboShape};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use kurbo::Point;

/// A curve that can be used in shapes and operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Curve {
    /// A straight line segment.
    Line(Line),
    /// A circle.
    Circle(Circle),
    /// A Bézier path (can contain multiple segments).
    BezPath(BezPath),
}

impl Curve {
    /// Get the bounding box of the curve.
    pub fn bounding_box(&self) -> Rect {
        match self {
            Curve::Line(line) => line.bounding_box(),
            Curve::Circle(circle) => circle.bounding_box(),
            Curve::BezPath(path) => path.bounding_box(),
        }
    }

    /// Flatten the curve into line segments at the given tolerance.
    /// Returns a vector of (x, y) points.
    pub fn flatten(&self, tolerance: f64) -> Vec<(f64, f64)> {
        match self {
            Curve::Line(line) => {
                vec![(line.p0.x, line.p0.y), (line.p1.x, line.p1.y)]
            }
            Curve::Circle(circle) => {
                // Approximate circle with line segments
                let mut points = Vec::new();
                let num_segments = (2.0 * std::f64::consts::PI * circle.radius / tolerance)
                    .ceil()
                    .max(4.0) as usize;

                for i in 0..=num_segments {
                    let angle = 2.0 * std::f64::consts::PI * i as f64 / num_segments as f64;
                    let x = circle.center.x + circle.radius * angle.cos();
                    let y = circle.center.y + circle.radius * angle.sin();
                    points.push((x, y));
                }
                points
            }
            Curve::BezPath(path) => {
                use kurbo::{ParamCurve, ParamCurveArclen};
                let mut points = Vec::new();

                // Flatten the path using kurbo's path_segments
                for seg in path.path_segments(tolerance) {
                    // Sample the segment at regular intervals
                    let arclen = seg.arclen(tolerance);
                    let num_samples = (arclen / tolerance).ceil().max(2.0) as usize;
                    for i in 0..=num_samples {
                        let t = if num_samples > 0 {
                            i as f64 / num_samples as f64
                        } else {
                            0.0
                        };
                        let p = seg.eval(t);
                        points.push((p.x, p.y));
                    }
                }

                points
            }
        }
    }

    /// Check if the curve is closed (forms a loop).
    pub fn is_closed(&self) -> bool {
        match self {
            Curve::Line(_) => false,
            Curve::Circle(_) => true,
            Curve::BezPath(path) => path
                .elements()
                .last()
                .map(|el| matches!(el, PathEl::ClosePath))
                .unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_bounding_box() {
        let line = Line::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let curve = Curve::Line(line);
        let bbox = curve.bounding_box();
        assert_eq!(bbox.min_x(), 0.0);
        assert_eq!(bbox.min_y(), 0.0);
        assert_eq!(bbox.max_x(), 10.0);
        assert_eq!(bbox.max_y(), 10.0);
    }

    #[test]
    fn test_circle_bounding_box() {
        let circle = Circle::new(Point::new(5.0, 5.0), 10.0);
        let curve = Curve::Circle(circle);
        let bbox = curve.bounding_box();
        assert_eq!(bbox.min_x(), -5.0);
        assert_eq!(bbox.min_y(), -5.0);
        assert_eq!(bbox.max_x(), 15.0);
        assert_eq!(bbox.max_y(), 15.0);
    }

    #[test]
    fn test_line_flatten() {
        let line = Line::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let curve = Curve::Line(line);
        let points = curve.flatten(0.1);
        assert_eq!(points.len(), 2);
        assert_eq!(points[0], (0.0, 0.0));
        assert_eq!(points[1], (10.0, 10.0));
    }

    #[test]
    fn test_circle_flatten() {
        let circle = Circle::new(Point::new(0.0, 0.0), 10.0);
        let curve = Curve::Circle(circle);
        let points = curve.flatten(0.1);
        assert!(points.len() >= 4); // At least 4 segments for a circle
                                    // Check that first and last points are close (closed loop)
        let first = points[0];
        let last = points[points.len() - 1];
        let dist = ((first.0 - last.0).powi(2) + (first.1 - last.1).powi(2)).sqrt();
        assert!(dist < 1.0); // Should be close
    }

    #[test]
    fn test_circle_is_closed() {
        let circle = Circle::new(Point::new(0.0, 0.0), 10.0);
        let curve = Curve::Circle(circle);
        assert!(curve.is_closed());
    }

    #[test]
    fn test_line_is_not_closed() {
        let line = Line::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let curve = Curve::Line(line);
        assert!(!curve.is_closed());
    }

    #[test]
    fn test_bezpath_flatten() {
        let mut path = BezPath::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(10.0, 0.0));
        path.line_to(Point::new(10.0, 10.0));
        path.close_path();
        let curve = Curve::BezPath(path);
        let points = curve.flatten(0.1);
        assert!(points.len() >= 3);
    }
}
