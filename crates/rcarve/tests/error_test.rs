use rcarve::*;

#[test]
#[ignore = "generate_toolpaths deprecated"]
fn test_invalid_tool_index() {
    // Test error handling for invalid tool index
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
    let tools = vec![tool]; // Only one tool (index 0)
    let curve_id = CurveId::new();
    let operations = vec![Operation::Profile {
        target_depth: 5.0,
        cut_side: CutSide::Outside,
        tool_index: 99, // Invalid index
        targets: OperationTarget::Curves(vec![curve_id]),
    }];

    let result = generate_toolpaths(polygons, tools, operations);
    assert!(
        result.is_err(),
        "Should return error for invalid tool index"
    );

    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("Tool index") || error_msg.contains("out of range"),
            "Error message should mention tool index: {}",
            error_msg
        );
    }
}

#[test]
#[ignore = "generate_toolpaths deprecated"]
fn test_invalid_polygon_index() {
    // Test error handling for invalid polygon index
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

    let polygons = vec![square]; // Only one polygon (index 0)
    let tools = vec![tool];
    let curve_id = CurveId::new();
    let operations = vec![Operation::Profile {
        target_depth: 5.0,
        cut_side: CutSide::Outside,
        tool_index: 0,
        targets: OperationTarget::Curves(vec![curve_id]),
    }];

    let result = generate_toolpaths(polygons, tools, operations);
    assert!(
        result.is_err(),
        "Should return error for invalid polygon index"
    );

    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("Polygon index") || error_msg.contains("out of range"),
            "Error message should mention polygon index: {}",
            error_msg
        );
    }
}

#[test]
#[ignore = "generate_toolpaths deprecated"]
fn test_empty_operations_list() {
    // Test behavior with empty operations list
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
    let operations = vec![]; // Empty operations

    let result = generate_toolpaths(polygons, tools, operations);
    // Should succeed but generate minimal G-code (just header)
    assert!(result.is_ok(), "Should handle empty operations list");

    let gcode = result.unwrap();
    // Should have at least the header commands
    assert!(gcode.lines.len() >= 4, "Should have header commands");
    assert_eq!(gcode.lines[0], "G90", "Should have G90");
    assert_eq!(gcode.lines[1], "G21", "Should have G21");
}

#[test]
#[ignore = "generate_toolpaths deprecated"]
fn test_pocket_invalid_polygon_index() {
    // Test error handling for invalid polygon index in pocket operation
    let outer = vec![
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

    let polygons = vec![outer]; // Only one polygon (index 0)
    let tools = vec![tool];
    let curve_id = CurveId::new();
    let operations = vec![Operation::Pocket {
        target_depth: 5.0,
        tool_index: 0,
        target: OperationTarget::Curves(vec![curve_id]),
    }];

    let result = generate_toolpaths(polygons, tools, operations);
    assert!(
        result.is_err(),
        "Should return error for invalid polygon index in pocket"
    );
}

#[test]
#[ignore = "generate_toolpaths deprecated"]
fn test_pocket_invalid_island_index() {
    // Test error handling for invalid island index in pocket operation
    let outer = vec![
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

    let polygons = vec![outer]; // Only one polygon (index 0)
    let tools = vec![tool];
    let curve_id_0 = CurveId::new();
    let curve_id_1 = CurveId::new();
    let operations = vec![Operation::Pocket {
        target_depth: 5.0,
        tool_index: 0,
        target: OperationTarget::Curves(vec![curve_id_0, curve_id_1]),
    }];

    let result = generate_toolpaths(polygons, tools, operations);
    // This should succeed but skip the invalid island (filter_map will skip it)
    // However, let's verify it doesn't crash
    assert!(
        result.is_ok(),
        "Should handle invalid island index gracefully"
    );
}
