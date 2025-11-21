use rcarve::*;

#[test]
fn test_profile_inside_cut() {
    // Test inside profile cut - should offset inward (negative offset)
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

    let toolpath = generate_profile_toolpath(&square, &tool, &CutSide::Inside, 5.0)
        .expect("Failed to generate inside profile toolpath");

    // Post-process to G-code
    let gcode = post_process_grbl(&toolpath);

    // Verify G-code structure
    assert_eq!(gcode.lines[0], "G90", "First command should be G90");
    assert!(
        gcode.lines.iter().any(|l| l.starts_with("G1 Z")),
        "Should have plunge move"
    );

    // For inside cut, coordinates should be offset inward
    // For 100x100 square with 6mm tool (radius=3.0), inside profile
    // corners should be at approximately (3.0, 3.0) and (97.0, 97.0)
    let gcode_str = gcode.lines.join("\n");

    // Verify we have coordinates inside the original boundary
    // The path should be offset inward by the tool radius
    assert!(
        gcode_str.contains("X3.") || gcode_str.contains("X97."),
        "Inside cut should have coordinates offset inward from boundary"
    );

    // Verify Z-depth
    for path in &toolpath.paths {
        for point in path {
            assert_eq!(point.2, -5.0, "Z should be negative target depth");
        }
    }

    println!("Generated G-code for inside cut:\n{}", gcode_str);
}

#[test]
#[ignore = "generate_toolpaths deprecated"]
fn test_profile_via_high_level_api() {
    // Test profile operation through the high-level generate_toolpaths API
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

    let polygons = vec![square];
    let tools = vec![tool];
    let curve_id = CurveId::new();
    let operations = vec![Operation::Profile {
        target_depth: 5.0,
        cut_side: CutSide::Outside,
        tool_index: 0,
        targets: OperationTarget::Curves(vec![curve_id]),
    }];

    let gcode = generate_toolpaths(polygons, tools, operations)
        .expect("Failed to generate toolpaths via high-level API");

    // Verify G-code was generated
    assert!(!gcode.lines.is_empty(), "Should generate G-code");
    assert_eq!(gcode.lines[0], "G90", "First command should be G90");
    assert!(
        gcode.lines.iter().any(|l| l.starts_with("G1 Z")),
        "Should have plunge move"
    );

    println!("High-level API Profile G-code:\n{}", gcode.lines.join("\n"));
}

#[test]
fn test_profile_on_line_cut() {
    // Test on-line profile cut - should follow the exact path (zero offset)
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

    let toolpath = generate_profile_toolpath(&square, &tool, &CutSide::OnLine, 5.0)
        .expect("Failed to generate on-line profile toolpath");

    // Post-process to G-code
    let gcode = post_process_grbl(&toolpath);

    // Verify G-code structure
    assert_eq!(gcode.lines[0], "G90", "First command should be G90");

    // For on-line cut, coordinates should match the original path (offset = 0)
    // Note: clipper2 may still add some points due to rounding, but should be close
    let gcode_str = gcode.lines.join("\n");

    // Verify we have coordinates (exact values depend on clipper2 implementation)
    assert!(
        gcode_str.contains("X") && gcode_str.contains("Y"),
        "Should have X and Y coordinates"
    );

    // Verify Z-depth
    for path in &toolpath.paths {
        for point in path {
            assert_eq!(point.2, -5.0, "Z should be negative target depth");
        }
    }

    println!("Generated G-code for on-line cut:\n{}", gcode_str);
}
