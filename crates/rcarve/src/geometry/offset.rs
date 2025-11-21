use crate::vcarve::CarvePolygon;
use anyhow::Result;
use cavalier_contours::polyline::{PlineSource, PlineSourceMut, PlineVertex, Polyline};

pub fn offset_polygon(
    polygons: &[CarvePolygon],
    delta: f64,
) -> Result<Vec<CarvePolygon>> {
    let mut result_polygons = Vec::new();

    for poly in polygons {
        // 1. Convert CarvePolygon to Cavalier Polyline(s)
        let mut plines = Vec::new();

        // Outer
        if !poly.outer.is_empty() {
            let mut pline = create_polyline(&poly.outer, true);
            // Ensure outer is CCW (positive area)
            if pline.area() < 0.0 {
                pline.invert_direction_mut();
            }
            plines.push(pline);
        }

        // Holes
        for hole in &poly.holes {
            if !hole.is_empty() {
                let mut pline = create_polyline(hole, true);
                // Ensure holes are CW (negative area)
                if pline.area() > 0.0 {
                    pline.invert_direction_mut();
                }
                plines.push(pline);
            }
        }

        // 2. Perform Offset
        let mut offset_result = Vec::new();
        for pline in plines {
             let offsets = pline.parallel_offset(delta);
             offset_result.extend(offsets);
        }
        
        // 3. Reconstruct
        let mut outers = Vec::new();
        let mut holes = Vec::new();

        for pline in offset_result {
            let points = extract_points(&pline);
            if points.len() < 3 {
                continue;
            }
            
            if pline.area() > 0.0 {
                outers.push(points);
            } else {
                holes.push(points);
            }
        }

        // Match holes to outers
        for outer in outers {
            let mut my_holes = Vec::new();
            let mut i = 0;
            while i < holes.len() {
                let hole = &holes[i];
                if is_point_inside(&hole[0], &outer) {
                    my_holes.push(holes.remove(i));
                } else {
                    i += 1;
                }
            }
            result_polygons.push(CarvePolygon {
                outer,
                holes: my_holes,
            });
        }
    }

    Ok(result_polygons)
}

fn create_polyline(points: &[(f64, f64)], closed: bool) -> Polyline {
    let mut pline = Polyline::new();
    
    if points.is_empty() {
        return pline;
    }

    let mut effective_points = points.to_vec();
    
    // If closed, and last point == first point, remove the last one
    if closed && effective_points.len() > 1 {
        let first = effective_points[0];
        let last = effective_points[effective_points.len() - 1];
        if (first.0 - last.0).abs() < 1e-9 && (first.1 - last.1).abs() < 1e-9 {
            effective_points.pop();
        }
    }
    
    // Filter out duplicate consecutive points (zero length segments)
    if !effective_points.is_empty() {
        let mut clean_points = Vec::new();
        clean_points.push(effective_points[0]);
        for i in 1..effective_points.len() {
            let prev = clean_points.last().unwrap();
            let curr = effective_points[i];
            if (prev.0 - curr.0).abs() > 1e-9 || (prev.1 - curr.1).abs() > 1e-9 {
                clean_points.push(curr);
            }
        }
        effective_points = clean_points;
    }

    for (x, y) in effective_points {
        pline.add_vertex(PlineVertex::new(x, y, 0.0));
    }
    
    if closed {
        pline.set_is_closed(true);
    }
    pline
}

fn extract_points(pline: &Polyline) -> Vec<(f64, f64)> {
    let has_arcs = pline.iter_vertexes().any(|v| v.bulge.abs() > 1e-6);
    
    if has_arcs {
        let mut points = Vec::new();
        let vertex_count = pline.vertex_count();
        for i in 0..vertex_count {
            let v = pline.at(i);
            let next_index = (i + 1) % vertex_count;
            
            points.push((v.x, v.y));
            
            if !pline.is_closed() && i == vertex_count - 1 {
                 break;
            }

            let v_next = pline.at(next_index);
            if v.bulge.abs() > 1e-6 {
                let arc_points = tessellate_arc(v.x, v.y, v_next.x, v_next.y, v.bulge);
                points.extend(arc_points);
            }
        }
        points
    } else {
        pline.iter_vertexes().map(|v| (v.x, v.y)).collect()
    }
}

fn tessellate_arc(x1: f64, y1: f64, x2: f64, y2: f64, bulge: f64) -> Vec<(f64, f64)> {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let chord_len = (dx * dx + dy * dy).sqrt();
    if chord_len < 1e-6 { return vec![]; }
    
    let mx = (x1 + x2) / 2.0;
    let my = (y1 + y2) / 2.0;
    
    let sagitta = bulge * (chord_len / 2.0);
    
    let nx = -dy;
    let ny = dx;
    let len = (nx*nx + ny*ny).sqrt();
    let u_nx = nx / len;
    let u_ny = ny / len;
    
    let px = mx + u_nx * sagitta;
    let py = my + u_ny * sagitta;
    
    vec![(px, py)]
}

fn is_point_inside(point: &(f64, f64), polygon: &[(f64, f64)]) -> bool {
    let x = point.0;
    let y = point.1;
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let (xi, yi) = polygon[i];
        let (xj, yj) = polygon[j];
        
        let intersect = ((yi > y) != (yj > y))
            && (x < (xj - xi) * (y - yi) / (yj - yi) + xi);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}
