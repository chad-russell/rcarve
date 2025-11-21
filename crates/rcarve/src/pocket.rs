use crate::types::{Tool, Toolpath};
use clipper2::{difference, inflate, EndType, JoinType, Path, PathType, Polygon, Polygons, Vertex};

/// Generate a 2D pocket toolpath using iterative offset (contour-parallel) strategy
///
/// This function uses clipper2 to perform offset pocketing by iteratively shrinking
/// the pocket boundary inward by the stepover distance until the area is cleared.
/// Supports islands (holes) that should not be milled.
pub fn generate_pocket_toolpath(
    outer_boundary: &[(f64, f64)],
    islands: &[Vec<(f64, f64)>],
    tool: &Tool,
    target_depth: f64,
) -> anyhow::Result<Toolpath> {
    // Step 1: Calculate stepover distance (Section 4.2, line 309)
    let stepover_dist = tool.diameter * tool.stepover;

    // Step 2: Convert outer boundary to clipper2::Polygons
    let outer_vertices: Vec<Vertex> = outer_boundary
        .iter()
        .map(|(x, y)| Vertex::new(*x, *y))
        .collect();
    let outer_path = Path::new(outer_vertices, true); // closed polygon
    let outer_polygon = Polygon::new(vec![outer_path], PathType::Subject);
    let mut outer_polygons = Polygons::new(vec![outer_polygon]);

    // Step 3: Handle islands by subtracting them from the outer boundary (Section 4.4, lines 351-356)
    if !islands.is_empty() {
        // Convert islands to clipper2::Polygons
        let island_polygons: Vec<Polygon> = islands
            .iter()
            .map(|island| {
                let island_vertices: Vec<Vertex> =
                    island.iter().map(|(x, y)| Vertex::new(*x, *y)).collect();
                let island_path = Path::new(island_vertices, true);
                Polygon::new(vec![island_path], PathType::Clip)
            })
            .collect();
        let islands_polygons = Polygons::new(island_polygons);

        // Use clipper2's difference operation to subtract islands from outer boundary
        outer_polygons = difference(outer_polygons, islands_polygons);
    }

    // Step 4: Iterative offset pocketing (Section 4.2, lines 307-329)
    // Start with the pocket shape (outer boundary minus islands)
    let mut current_pocket = outer_polygons;
    let mut pocket_paths: Vec<Vec<(f64, f64)>> = Vec::new();

    // Loop: shrink the polygon inward by stepover distance until it collapses
    loop {
        // Use negative delta to shrink (deflate) the polygon inward
        let offset_result = inflate(
            current_pocket.clone(),
            -stepover_dist,  // Negative = shrink inward
            JoinType::Round, // Round joins for smooth curves (circles, arcs)
            EndType::ClosedPolygon,
            2.0,  // Miter limit (unused for Round joins)
            0.25, // Arc tolerance - controls smoothness of rounded corners
        );

        // If inflating (shrinking) produces no paths, we are done
        if offset_result.polygons().is_empty() {
            break;
        }

        // Extract all paths from the offset result and add to pocket_paths
        for polygon in offset_result.polygons() {
            for path in polygon.paths() {
                // Convert path vertices to (f64, f64) tuples
                let path_2d: Vec<(f64, f64)> =
                    path.vertices().iter().map(|v| (v.x(), v.y())).collect();
                if !path_2d.is_empty() {
                    pocket_paths.push(path_2d);
                }
            }
        }

        // Update current_pocket for next iteration
        current_pocket = offset_result;
    }

    // Step 5: Convert 2D paths to 3D toolpaths (Section 4.2, line 332)
    // TODO: Multi-pass Z handling (for now, single pass at target_depth)
    let target_z = -target_depth;
    let mut paths_3d: Vec<Vec<(f64, f64, f64)>> = Vec::new();

    for path_2d in pocket_paths {
        // Ensure path is closed by repeating first point at end if needed
        let mut path_3d: Vec<(f64, f64, f64)> =
            path_2d.iter().map(|(x, y)| (*x, *y, target_z)).collect();

        // Close the path by adding first point at end if not already closed
        if !path_3d.is_empty() && path_3d[0] != *path_3d.last().unwrap() {
            path_3d.push(path_3d[0]);
        }

        paths_3d.push(path_3d);
    }

    // Step 6: Return Toolpath struct
    Ok(Toolpath { paths: paths_3d })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolType;

    #[test]
    fn test_simple_pocket() {
        // Simple square pocket: 100x100mm outer boundary
        let outer = vec![
            (0.0, 0.0),
            (100.0, 0.0),
            (100.0, 100.0),
            (0.0, 100.0),
            (0.0, 0.0),
        ];

        let tool = Tool {
            name: "6mm Endmill".to_string(),
            diameter: 6.0,
            stepover: 0.4, // 40% = 2.4mm stepover
            pass_depth: 5.0,
            tool_type: ToolType::Endmill { diameter: 6.0 },
        };

        let result = generate_pocket_toolpath(&outer, &[], &tool, 5.0);
        assert!(
            result.is_ok(),
            "Should generate pocket toolpath successfully"
        );

        let toolpath = result.unwrap();
        assert!(!toolpath.paths.is_empty(), "Should have at least one path");
        assert!(
            toolpath.paths.len() > 1,
            "Should have multiple concentric paths for pocketing"
        );

        // Verify Z-depth
        for path in &toolpath.paths {
            for point in path {
                assert_eq!(point.2, -5.0, "Z should be negative target depth");
            }
        }
    }

    #[test]
    fn test_pocket_with_island() {
        // Square pocket with a circular island in the center
        let outer = vec![
            (0.0, 0.0),
            (100.0, 0.0),
            (100.0, 100.0),
            (0.0, 100.0),
            (0.0, 0.0),
        ];

        // Square island in center (40x40mm)
        let island = vec![
            (30.0, 30.0),
            (70.0, 30.0),
            (70.0, 70.0),
            (30.0, 70.0),
            (30.0, 30.0),
        ];

        let tool = Tool {
            name: "6mm Endmill".to_string(),
            diameter: 6.0,
            stepover: 0.4,
            pass_depth: 5.0,
            tool_type: ToolType::Endmill { diameter: 6.0 },
        };

        let result = generate_pocket_toolpath(&outer, &[island], &tool, 5.0);
        assert!(
            result.is_ok(),
            "Should generate pocket toolpath with island"
        );

        let toolpath = result.unwrap();
        assert!(!toolpath.paths.is_empty(), "Should have at least one path");

        // Verify paths don't intersect the island area
        // (This is a basic check - more sophisticated validation could be added)
        for path in &toolpath.paths {
            for point in path {
                // Points should be outside the island (30-70 range)
                let x = point.0;
                let y = point.1;
                assert!(
                    x < 30.0 || x > 70.0 || y < 30.0 || y > 70.0,
                    "Path should not intersect island area"
                );
            }
        }
    }
}
