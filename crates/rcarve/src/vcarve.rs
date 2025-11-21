use crate::geometry::offset::offset_polygon;
use crate::types::{Tool, Toolpath};
use anyhow::{anyhow, Context, Result};
use geo::algorithm::coords_iter::CoordsIter;
use geo::algorithm::euclidean_distance::EuclideanDistance;
use geo::{LineString, Point, Polygon};
use geo_buffer::skeleton_of_polygon_to_linestring;

/// Represents a polygon (with optional holes) for V-carving.
#[derive(Debug, Clone)]
pub struct CarvePolygon {
    pub outer: Vec<(f64, f64)>,
    pub holes: Vec<Vec<(f64, f64)>>,
}

/// Generate a V-carve toolpath using a straight-skeleton derived from the input polygons.
pub fn generate_vcarve_toolpath(
    polygons: &[CarvePolygon],
    tool: &Tool,
    max_depth: Option<f64>,
) -> Result<Toolpath> {
    let vbit_angle = match tool.tool_type {
        crate::types::ToolType::VBit { angle_degrees } => angle_degrees,
        _ => {
            return Err(anyhow!(
                "V-carve requires a V-bit tool, but tool type is {:?}",
                tool.tool_type
            ));
        }
    };

    if polygons.is_empty() {
        return Err(anyhow!("V-carve requires at least one polygon target"));
    }

    let tool_angle_rad = vbit_angle.to_radians() / 2.0;
    let tan_a = tool_angle_rad.tan();
    let max_depth_value = max_depth.unwrap_or(f64::MAX);
    // If max_depth is infinite, limit_dist is infinite
    let limit_dist = if let Some(d) = max_depth {
        d * tan_a
    } else {
        f64::MAX
    };

    let mut paths_3d: Vec<Vec<(f64, f64, f64)>> = Vec::new();

    // 1. Generate "Wall Paths" (Flat Bottom Contour) if max_depth is set
    if let Some(depth) = max_depth {
        // Offset by limit_dist to shrink inwards (CavalierContours: positive = left/inwards for CCW)
        if let Ok(inner_polys) = offset_polygon(polygons, limit_dist) {
            for poly in inner_polys {
                // Tracing outer loop
                let mut p: Vec<_> = poly
                    .outer
                    .iter()
                    .map(|(x, y)| (*x, *y, -depth))
                    .collect();
                if !p.is_empty() && p[0] != *p.last().unwrap() {
                    p.push(p[0]);
                }
                if p.len() >= 2 {
                    paths_3d.push(p);
                }

                // Tracing holes
                for hole in poly.holes {
                    let mut p: Vec<_> = hole
                        .iter()
                        .map(|(x, y)| (*x, *y, -depth))
                        .collect();
                    if !p.is_empty() && p[0] != *p.last().unwrap() {
                        p.push(p[0]);
                    }
                    if p.len() >= 2 {
                        paths_3d.push(p);
                    }
                }
            }
        }
    }

    // 2. Generate Straight Skeleton (Corner/Slope Paths)
    for poly in polygons {
        // Simplify the polygon to remove collinear vertices which cause "comb" artifacts
        let simple_poly = simplify_carve_polygon(poly);

        let geo_poly = build_geo_polygon(&simple_poly)
            .with_context(|| "Failed to convert polygon for straight-skeleton computation")?;
        // true => assume polygons use standard (counter-clockwise) orientation
        let skeleton_segments = skeleton_of_polygon_to_linestring(&geo_poly, true);

        for segment in skeleton_segments {
            if segment.coords_count() < 2 {
                continue;
            }

            let mut current_path = Vec::new();
            let mut prev_point: Option<(Point<f64>, f64)> = None; // (point, distance)

            for coord in segment.coords_iter() {
                let point = Point::new(coord.x, coord.y);
                let distance = distance_to_polygon_boundary(point, &geo_poly);
                
                // Skip points extremely close to boundary if they cause numerical issues?
                // But we need Z=0 points. 
                
                let is_shallow = distance <= limit_dist;

                if let Some((prev_p, prev_d)) = prev_point {
                    let was_shallow = prev_d <= limit_dist;

                    if was_shallow && is_shallow {
                        // Fully shallow segment
                        if current_path.is_empty() {
                            // Start path
                            let z = -(prev_d / tan_a);
                            current_path.push((prev_p.x(), prev_p.y(), z));
                        }
                        let z = -(distance / tan_a);
                        current_path.push((point.x(), point.y(), z));
                    } else if was_shallow && !is_shallow {
                        // Crossing Shallow -> Deep
                        let (ix, iy) = interpolate(prev_p, prev_d, point, distance, limit_dist);
                        let z = -max_depth_value;

                        if current_path.is_empty() {
                            let prev_z = -(prev_d / tan_a);
                            current_path.push((prev_p.x(), prev_p.y(), prev_z));
                        }
                        current_path.push((ix, iy, z));

                        // Terminate path
                        paths_3d.push(current_path);
                        current_path = Vec::new();
                    } else if !was_shallow && is_shallow {
                        // Crossing Deep -> Shallow
                        let (ix, iy) = interpolate(prev_p, prev_d, point, distance, limit_dist);
                        let z = -max_depth_value;
                        
                        // Start new path
                        current_path.push((ix, iy, z));
                        let curr_z = -(distance / tan_a);
                        current_path.push((point.x(), point.y(), curr_z));
                    } else {
                        // Deep -> Deep
                        // Ignore
                    }
                } else {
                    // First point of segment
                    // Just record it for next iteration
                }
                prev_point = Some((point, distance));
            }

            if !current_path.is_empty() {
                paths_3d.push(current_path);
            }
        }
    }

    if paths_3d.is_empty() {
        return Err(anyhow!(
            "Straight skeleton produced no toolpaths. Ensure shapes are valid closed polygons."
        ));
    }

    Ok(Toolpath { paths: paths_3d })
}

/// Removes collinear vertices from polygon boundary and holes.
/// This is critical for the Straight Skeleton algorithm, which generates spurious
/// branches for every vertex, even collinear ones.
fn simplify_carve_polygon(poly: &CarvePolygon) -> CarvePolygon {
    const TOLERANCE: f64 = 0.001; // 1 micron tolerance for collinearity

    fn simplify_ring(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
        if points.len() < 3 {
            return points.to_vec();
        }
        let mut simplified = Vec::new();
        simplified.push(points[0]);

        for i in 1..points.len() {
            let _p_prev = simplified.last().unwrap();
            let p_curr = points[i];
            
            // Look ahead to next point
            let _next_idx = (i + 1) % points.len();
            let _p_next = points[_next_idx];

            // Check if p_curr is collinear with p_prev and p_next
            // If it is, skip p_curr (don't add it to simplified)
            // Wait, this logic needs to be careful. 
            // Easier: Add point. Then check if last 3 are collinear.
            
            simplified.push(p_curr);
            
            while simplified.len() >= 3 {
                let n = simplified.len();
                let p1 = simplified[n - 3];
                let p2 = simplified[n - 2];
                let p3 = simplified[n - 1];
                
                if are_collinear(p1, p2, p3, TOLERANCE) {
                    // Remove p2
                    simplified.remove(n - 2);
                } else {
                    break;
                }
            }
        }
        
        // Also check wrap-around collinearity
        if simplified.len() >= 3 {
            let p_last = simplified[simplified.len() - 1];
            let p_first = simplified[0];
            // If start and end are same, remove one
            if (p_last.0 - p_first.0).abs() < f64::EPSILON && (p_last.1 - p_first.1).abs() < f64::EPSILON {
                simplified.pop();
            }
        }
        
        // Check closure collinearity
        if simplified.len() >= 3 {
            let p1 = simplified[simplified.len() - 1];
            let p2 = simplified[0];
            let p3 = simplified[1];
            if are_collinear(p1, p2, p3, TOLERANCE) {
                simplified.remove(0);
            }
        }

        // Close it back up if input was closed
        if let (Some(first), Some(last)) = (points.first(), points.last()) {
             if (first.0 - last.0).abs() < f64::EPSILON && (first.1 - last.1).abs() < f64::EPSILON {
                 if let Some(s_first) = simplified.first().cloned() {
                     if let Some(s_last) = simplified.last() {
                         if (s_first.0 - s_last.0).abs() > f64::EPSILON || (s_first.1 - s_last.1).abs() > f64::EPSILON {
                             simplified.push(s_first);
                         }
                     }
                 }
             }
        }

        simplified
    }

    fn are_collinear(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), tol: f64) -> bool {
        // Area of triangle formed by p1, p2, p3 should be near zero
        let area = (p1.0 * (p2.1 - p3.1) + p2.0 * (p3.1 - p1.1) + p3.0 * (p1.1 - p2.1)).abs() * 0.5;
        
        if area > tol {
            return false;
        }
        
        // Check directions
        let v1 = (p2.0 - p1.0, p2.1 - p1.1);
        let v2 = (p3.0 - p2.0, p3.1 - p2.1);
        let dot = v1.0 * v2.0 + v1.1 * v2.1;
        
        dot > 0.0
    }

    CarvePolygon {
        outer: simplify_ring(&poly.outer),
        holes: poly.holes.iter().map(|h| simplify_ring(h)).collect(),
    }
}

fn interpolate(p1: Point<f64>, d1: f64, p2: Point<f64>, d2: f64, limit: f64) -> (f64, f64) {
    if (d2 - d1).abs() < f64::EPSILON {
        return (p1.x(), p1.y());
    }
    let t = (limit - d1) / (d2 - d1);
    let x = p1.x() + t * (p2.x() - p1.x());
    let y = p1.y() + t * (p2.y() - p1.y());
    (x, y)
}

fn build_geo_polygon(poly: &CarvePolygon) -> Result<Polygon<f64>> {
    fn to_line_string(points: &[(f64, f64)]) -> Result<LineString<f64>> {
        if points.len() < 3 {
            return Err(anyhow!("Polygon loop must contain at least three points"));
        }
        let mut coords = points.to_vec();
        if let (Some(first), Some(last)) = (coords.first(), coords.last()) {
            if first != last {
                coords.push(*first);
            }
        }
        Ok(LineString::from(coords))
    }

    let exterior = to_line_string(&poly.outer)?;
    let mut interiors = Vec::new();
    for hole in &poly.holes {
        interiors.push(to_line_string(hole)?);
    }

    Ok(Polygon::new(exterior, interiors))
}

fn distance_to_polygon_boundary(point: Point<f64>, polygon: &Polygon<f64>) -> f64 {
    let mut distance = point.euclidean_distance(polygon.exterior());
    for interior in polygon.interiors() {
        distance = distance.min(point.euclidean_distance(interior));
    }
    distance
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolType;

    #[test]
    fn test_vcarve_simple_letter() {
        // Test: A letter-like shape (U shape)
        let u_shape = vec![
            (0.0, 0.0),    // Bottom left
            (100.0, 0.0),  // Bottom right
            (100.0, 50.0), // Top right (outer)
            (50.0, 50.0),  // Top right (inner)
            (50.0, 30.0),  // Bottom of inner leg
            (0.0, 30.0),   // Bottom left (inner)
            (0.0, 50.0),   // Top left (inner)
            (50.0, 50.0),  // Top left (back to inner)
            (50.0, 50.0),  // Close the inner part
            (100.0, 50.0), // Back to top right
            (100.0, 0.0),  // Back to start
            (0.0, 0.0),    // Close
        ];

        let tool = Tool {
            name: "60deg V-bit".to_string(),
            diameter: 0.0,
            stepover: 0.0,
            pass_depth: 0.0,
            tool_type: ToolType::VBit {
                angle_degrees: 60.0,
            },
        };

        let result = generate_vcarve_toolpath(
            &[CarvePolygon {
                outer: u_shape,
                holes: Vec::new(),
            }],
            &tool,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_vcarve_with_max_depth() {
        // Test flat-bottom V-carving with max depth limit
        let square = vec![
            (0.0, 0.0),
            (50.0, 0.0),
            (50.0, 50.0),
            (0.0, 50.0),
            (0.0, 0.0),
        ];

        let tool = Tool {
            name: "60deg V-bit".to_string(),
            diameter: 0.0,
            stepover: 0.0,
            pass_depth: 0.0,
            tool_type: ToolType::VBit {
                angle_degrees: 60.0,
            },
        };

        let max_depth = Some(5.0); // Limit to 5mm depth
        let result = generate_vcarve_toolpath(
            &[CarvePolygon {
                outer: square,
                holes: Vec::new(),
            }],
            &tool,
            max_depth,
        );
        assert!(
            result.is_ok(),
            "Should generate V-carve toolpath with max depth"
        );

        let toolpath = result.unwrap();
        // Verify all depths are limited to max_depth
        for path in &toolpath.paths {
            for point in path {
                // Allow small epsilon for float comparison
                assert!(point.2 >= -5.001, "Z should not exceed max depth (got {})", point.2);
            }
        }
    }

    #[test]
    fn test_vcarve_requires_vbit() {
        let square = vec![
            (0.0, 0.0),
            (50.0, 0.0),
            (50.0, 50.0),
            (0.0, 50.0),
            (0.0, 0.0),
        ];

        let tool = Tool {
            name: "6mm Endmill".to_string(),
            diameter: 6.0,
            stepover: 0.4,
            pass_depth: 5.0,
            tool_type: ToolType::Endmill { diameter: 6.0 },
        };

        let result = generate_vcarve_toolpath(
            &[CarvePolygon {
                outer: square,
                holes: Vec::new(),
            }],
            &tool,
            None,
        );
        assert!(result.is_err(), "Should reject non-V-bit tool");
    }
}
