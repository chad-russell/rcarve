use crate::types::{GCode, Toolpath};

/// Convert generic toolpath to Grbl-compatible G-code
pub fn post_process_grbl(toolpath: &Toolpath) -> GCode {
    let mut lines = Vec::new();

    // Header (Section 2.3, lines 191-195)
    lines.push("G90".to_string()); // Absolute positioning
    lines.push("G21".to_string()); // Millimeters
    lines.push("G17".to_string()); // XY plane
    lines.push("G0 Z10.0".to_string()); // Safe height

    // Iterate through toolpath.paths
    for path in &toolpath.paths {
        if path.is_empty() {
            continue;
        }

        // Entry move (Section 2.3, lines 196-199)
        let start = path[0];
        lines.push(format!("G0 X{:.4} Y{:.4}", start.0, start.1));
        lines.push(format!("G1 Z{:.4} F100", start.2));

        // Cutting moves (Section 2.3, lines 200-202)
        // Generate G1 commands for remaining points
        for point in path.iter().skip(1) {
            lines.push(format!("G1 X{:.4} Y{:.4}", point.0, point.1));
        }

        // Exit move (Section 2.3, lines 203-204)
        lines.push("G0 Z10.0".to_string());
    }

    GCode { lines }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postprocessor_structure() {
        let toolpath = Toolpath {
            paths: vec![vec![
                (0.0, 0.0, -5.0),
                (100.0, 0.0, -5.0),
                (100.0, 100.0, -5.0),
            ]],
        };

        let gcode = post_process_grbl(&toolpath);

        // Verify header
        assert_eq!(gcode.lines[0], "G90", "First command should be G90");
        assert_eq!(gcode.lines[1], "G21", "Second command should be G21");
        assert_eq!(gcode.lines[2], "G17", "Third command should be G17");
        assert_eq!(
            gcode.lines[3], "G0 Z10.0",
            "Fourth command should be safe height"
        );

        // Verify entry move
        assert!(
            gcode.lines[4].starts_with("G0 X"),
            "Should have rapid move to start"
        );
        assert!(
            gcode.lines[5].starts_with("G1 Z"),
            "Should have plunge move"
        );

        // Verify cutting moves
        assert!(
            gcode.lines[6].starts_with("G1 X"),
            "Should have cutting move"
        );

        // Verify exit move
        assert!(
            gcode.lines.iter().any(|l| l == "G0 Z10.0"),
            "Should have exit move to safe height"
        );
    }

    #[test]
    fn test_postprocessor_plunge_move() {
        let toolpath = Toolpath {
            paths: vec![vec![(10.0, 20.0, -5.0)]],
        };

        let gcode = post_process_grbl(&toolpath);
        let gcode_str = gcode.lines.join("\n");

        assert!(gcode_str.contains("G1 Z"), "Should have plunge move");
        assert!(gcode_str.contains("F100"), "Should have feed rate");
    }
}
