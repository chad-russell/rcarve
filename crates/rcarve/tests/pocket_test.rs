use rcarve::*;

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

    let toolpath = generate_pocket_toolpath(&outer, &[], &tool, 5.0)
        .expect("Failed to generate pocket toolpath");

    // Post-process to G-code
    let gcode = post_process_grbl(&toolpath);

    // Verify we have multiple paths (concentric rings)
    assert!(
        toolpath.paths.len() > 1,
        "Should have multiple concentric paths for pocketing"
    );

    // Verify G-code structure
    assert_eq!(gcode.lines[0], "G90", "First command should be G90");
    assert!(
        gcode.lines.iter().any(|l| l.starts_with("G1 Z")),
        "Should have plunge moves"
    );

    // Verify that inner paths have coordinates inside the outer boundary
    // The innermost path should be well inside 100x100
    let mut found_inner_path = false;
    for path in &toolpath.paths {
        for point in path {
            if point.0 > 10.0 && point.0 < 90.0 && point.1 > 10.0 && point.1 < 90.0 {
                found_inner_path = true;
                break;
            }
        }
        if found_inner_path {
            break;
        }
    }
    assert!(
        found_inner_path,
        "Should have inner paths with coordinates inside pocket"
    );

    // Verify Z-depth
    for path in &toolpath.paths {
        for point in path {
            assert_eq!(point.2, -5.0, "Z should be negative target depth");
        }
    }

    println!("Generated {} paths for pocket", toolpath.paths.len());
    println!("Generated G-code:\n{}", gcode.lines.join("\n"));
}

#[test]
fn test_pocket_with_island() {
    // Square pocket with a square island in the center
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

    let toolpath = generate_pocket_toolpath(&outer, &[island], &tool, 5.0)
        .expect("Failed to generate pocket toolpath with island");

    // Post-process to G-code
    let gcode = post_process_grbl(&toolpath);

    // Verify we have paths
    assert!(!toolpath.paths.is_empty(), "Should have at least one path");

    // Verify paths don't intersect the island area (30-70 range)
    for path in &toolpath.paths {
        for point in path {
            let x = point.0;
            let y = point.1;
            assert!(
                x < 30.0 || x > 70.0 || y < 30.0 || y > 70.0,
                "Path should not intersect island area at ({}, {})",
                x,
                y
            );
        }
    }

    // Verify G-code contains coordinates outside island
    let gcode_str = gcode.lines.join("\n");
    assert!(
        gcode_str.contains("X") && gcode_str.contains("Y"),
        "G-code should contain X and Y coordinates"
    );

    println!(
        "Generated {} paths for pocket with island",
        toolpath.paths.len()
    );
}

#[test]
#[ignore = "generate_toolpaths deprecated"]
fn test_pocket_via_high_level_api() {
    // Test pocketing through the high-level generate_toolpaths API
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

    let polygons = vec![outer];
    let tools = vec![tool];
    let curve_id = CurveId::new();
    let operations = vec![Operation::Pocket {
        target_depth: 5.0,
        tool_index: 0,
        target: OperationTarget::Curves(vec![curve_id]),
    }];

    let gcode = generate_toolpaths(polygons, tools, operations)
        .expect("Failed to generate toolpaths via high-level API");

    // Verify G-code was generated
    assert!(!gcode.lines.is_empty(), "Should generate G-code");
    assert_eq!(gcode.lines[0], "G90", "First command should be G90");

    println!(
        "High-level API generated G-code:\n{}",
        gcode.lines.join("\n")
    );
}
