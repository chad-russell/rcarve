use rcarve::*;

#[test]
#[ignore = "generate_toolpaths deprecated"]
fn test_multiple_operations_profile_and_pocket() {
    // Test multiple operations: Profile then Pocket
    let profile_shape = vec![
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0, 20.0),
        (0.0, 20.0),
        (0.0, 0.0),
    ];

    let pocket_outer = vec![
        (10.0, 30.0),
        (90.0, 30.0),
        (90.0, 90.0),
        (10.0, 90.0),
        (10.0, 30.0),
    ];

    let tool = Tool {
        name: "6mm Endmill".to_string(),
        diameter: 6.0,
        stepover: 0.4,
        pass_depth: 5.0,
        tool_type: ToolType::Endmill { diameter: 6.0 },
    };

    let polygons = vec![profile_shape, pocket_outer];
    let tools = vec![tool];
    // Note: These tests use the deprecated generate_toolpaths function
    // TODO: Update to use ShapeRegistry and new toolpath generation
    let curve_id_0 = CurveId::new();
    let curve_id_1 = CurveId::new();
    let operations = vec![
        Operation::Profile {
            target_depth: 5.0,
            cut_side: CutSide::Outside,
            tool_index: 0,
            targets: OperationTarget::Curves(vec![curve_id_0]),
        },
        Operation::Pocket {
            target_depth: 5.0,
            tool_index: 0,
            target: OperationTarget::Curves(vec![curve_id_1]),
        },
    ];

    let gcode = generate_toolpaths(polygons, tools, operations)
        .expect("Failed to generate toolpaths for multiple operations");

    // Verify G-code was generated
    assert!(!gcode.lines.is_empty(), "Should generate G-code");
    assert_eq!(gcode.lines[0], "G90", "First command should be G90");

    // Should have multiple paths (profile + pocket paths)
    // Count the number of "G0 Z10.0" commands (exit moves) to verify multiple paths
    let exit_moves = gcode.lines.iter().filter(|l| l == &"G0 Z10.0").count();
    assert!(
        exit_moves >= 2,
        "Should have multiple exit moves (at least one for profile, one for pocket)"
    );

    // Verify we have both profile and pocket coordinates
    let gcode_str = gcode.lines.join("\n");
    assert!(
        gcode_str.contains("X") && gcode_str.contains("Y"),
        "Should have X and Y coordinates"
    );

    println!("Multiple operations G-code:\n{}", gcode_str);
}

#[test]
#[ignore = "generate_toolpaths deprecated"]
fn test_multiple_operations_two_profiles() {
    // Test multiple profile operations
    let shape1 = vec![
        (0.0, 0.0),
        (50.0, 0.0),
        (50.0, 50.0),
        (0.0, 50.0),
        (0.0, 0.0),
    ];

    let shape2 = vec![
        (60.0, 0.0),
        (110.0, 0.0),
        (110.0, 50.0),
        (60.0, 50.0),
        (60.0, 0.0),
    ];

    let tool = Tool {
        name: "6mm Endmill".to_string(),
        diameter: 6.0,
        stepover: 0.4,
        pass_depth: 5.0,
        tool_type: ToolType::Endmill { diameter: 6.0 },
    };

    let polygons = vec![shape1, shape2];
    let tools = vec![tool];
    let curve_id_0 = CurveId::new();
    let curve_id_1 = CurveId::new();
    let operations = vec![
        Operation::Profile {
            target_depth: 5.0,
            cut_side: CutSide::Outside,
            tool_index: 0,
            targets: OperationTarget::Curves(vec![curve_id_0]),
        },
        Operation::Profile {
            target_depth: 5.0,
            cut_side: CutSide::Outside,
            tool_index: 0,
            targets: OperationTarget::Curves(vec![curve_id_1]),
        },
    ];

    let gcode = generate_toolpaths(polygons, tools, operations)
        .expect("Failed to generate toolpaths for multiple profiles");

    // Verify G-code was generated
    assert!(!gcode.lines.is_empty(), "Should generate G-code");

    // Should have multiple exit moves (one per profile)
    let exit_moves = gcode.lines.iter().filter(|l| l == &"G0 Z10.0").count();
    assert!(
        exit_moves >= 2,
        "Should have multiple exit moves for multiple profiles"
    );

    println!("Multiple profiles G-code:\n{}", gcode.lines.join("\n"));
}

#[test]
#[ignore = "generate_toolpaths deprecated"]
fn test_multiple_operations_different_tools() {
    // Test multiple operations with different tools
    let profile_shape = vec![
        (0.0, 0.0),
        (50.0, 0.0),
        (50.0, 50.0),
        (0.0, 50.0),
        (0.0, 0.0),
    ];

    let pocket_outer = vec![
        (10.0, 60.0),
        (40.0, 60.0),
        (40.0, 90.0),
        (10.0, 90.0),
        (10.0, 60.0),
    ];

    let tool1 = Tool {
        name: "6mm Endmill".to_string(),
        diameter: 6.0,
        stepover: 0.4,
        pass_depth: 5.0,
        tool_type: ToolType::Endmill { diameter: 6.0 },
    };

    let tool2 = Tool {
        name: "3mm Endmill".to_string(),
        diameter: 3.0,
        stepover: 0.4,
        pass_depth: 5.0,
        tool_type: ToolType::Endmill { diameter: 3.0 },
    };

    let polygons = vec![profile_shape, pocket_outer];
    let tools = vec![tool1, tool2];
    let curve_id_0 = CurveId::new();
    let curve_id_1 = CurveId::new();
    let operations = vec![
        Operation::Profile {
            target_depth: 5.0,
            cut_side: CutSide::Outside,
            tool_index: 0, // Use first tool (6mm)
            targets: OperationTarget::Curves(vec![curve_id_0]),
        },
        Operation::Pocket {
            target_depth: 5.0,
            tool_index: 1, // Use second tool (3mm)
            target: OperationTarget::Curves(vec![curve_id_1]),
        },
    ];

    let gcode = generate_toolpaths(polygons, tools, operations)
        .expect("Failed to generate toolpaths with different tools");

    // Verify G-code was generated
    assert!(!gcode.lines.is_empty(), "Should generate G-code");

    // Should have multiple paths
    let exit_moves = gcode.lines.iter().filter(|l| l == &"G0 Z10.0").count();
    assert!(
        exit_moves >= 2,
        "Should have multiple exit moves for multiple operations"
    );

    println!(
        "Multiple operations with different tools G-code:\n{}",
        gcode.lines.join("\n")
    );
}
