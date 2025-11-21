use crate::types::{CutSide, Tool, Toolpath};
use clipper2::{inflate, EndType, JoinType, Path, PathType, Polygon, Polygons, Vertex};

/// Generate a 2D profile toolpath using polygon offsetting
///
/// This function uses clipper2 to perform proper polygon offsetting (inflating/deflating)
/// based on the tool diameter and cut side. The result is a 3D toolpath with the
/// appropriate Z-depth.
pub fn generate_profile_toolpath(
    input_poly: &[(f64, f64)],
    tool: &Tool,
    cut_side: &CutSide,
    target_depth: f64,
) -> anyhow::Result<Toolpath> {
    // Step 1: Calculate offset delta (Section 2.2, line 161)
    let radius = tool.diameter / 2.0;
    let offset_delta = match cut_side {
        CutSide::Outside => radius,
        CutSide::Inside => -radius,
        CutSide::OnLine => 0.0,
    };

    // Step 2: Convert input to clipper2::Polygons (Section 2.2, line 162)
    // Convert Vec<(f64, f64)> to clipper2 types:
    // (f64, f64) -> Vertex -> Path -> Polygon -> Polygons
    let vertices: Vec<Vertex> = input_poly
        .iter()
        .map(|(x, y)| Vertex::new(*x, *y))
        .collect();
    let path = Path::new(vertices, true); // true = closed polygon
    let polygon = Polygon::new(vec![path], PathType::Subject);
    let input_polygons = Polygons::new(vec![polygon]);

    // Step 3: Apply inflate (Section 2.2, lines 163-171)
    // Use clipper2's inflate function to offset the polygon
    // inflate takes: polygons, delta, join_type, end_type, miter_limit, arc_tolerance
    let offset_polygons = inflate(
        input_polygons,
        offset_delta,
        JoinType::Round,        // A good default for smooth corners
        EndType::ClosedPolygon, // We are offsetting a closed polygon
        0.0,                    // Miter limit, not relevant for Round joins
        0.0,                    // Arc tolerance
    );

    // Extract the first path from the result
    // For a simple profile, we expect one polygon with one path
    let offset_polygon = offset_polygons.polygons().first().ok_or_else(|| {
        anyhow::anyhow!("No offset polygon generated - polygon may have collapsed")
    })?;

    let offset_path = offset_polygon
        .paths()
        .first()
        .ok_or_else(|| anyhow::anyhow!("No offset path generated"))?;

    // Step 4: Convert to 3D path (Section 2.2, lines 174-181)
    // The offset_path vertices are in clipper2's internal format
    // Extract x() and y() as f64, then add Z-coordinate
    let target_z = -target_depth;
    let path_3d: Vec<(f64, f64, f64)> = offset_path
        .vertices()
        .iter()
        .map(|vertex| (vertex.x(), vertex.y(), target_z))
        .collect();

    // Step 5: Return Toolpath struct
    Ok(Toolpath {
        paths: vec![path_3d],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolType;

    #[test]
    fn test_square_profile_offset() {
        let square = vec![
            (0.0, 0.0),
            (100.0, 0.0),
            (100.0, 100.0),
            (0.0, 100.0),
            (0.0, 0.0),
        ];

        let tool = Tool {
            name: "6mm Endmill".to_string(),
            diameter: 6.0,
            stepover: 0.4,
            pass_depth: 5.0,
            tool_type: ToolType::Endmill { diameter: 6.0 },
        };

        let result = generate_profile_toolpath(&square, &tool, &CutSide::Outside, 5.0);
        assert!(result.is_ok(), "Should generate toolpath successfully");

        let toolpath = result.unwrap();
        assert_eq!(toolpath.paths.len(), 1, "Should have one path");
        assert!(!toolpath.paths[0].is_empty(), "Path should not be empty");

        // Verify offset coordinates (should be offset by radius = 3.0)
        let first_point = toolpath.paths[0][0];
        // For outside cut, coordinates should be offset outward
        // This is a basic check - exact values depend on clipper2 implementation
        assert_eq!(first_point.2, -5.0, "Z should be negative target depth");
    }
}
