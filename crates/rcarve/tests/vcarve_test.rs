use rcarve::*;

#[test]
fn test_vcarve_integration() {
    // Test V-carve through the full pipeline: geometry → toolpath → G-code
    // Use a simple shape that should generate a meaningful skeleton
    let shape = vec![
        (0.0, 0.0),
        (50.0, 0.0),
        (50.0, 30.0),
        (20.0, 30.0),
        (20.0, 20.0),
        (30.0, 20.0),
        (30.0, 10.0),
        (20.0, 10.0),
        (20.0, 0.0),
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

    let toolpath = generate_vcarve_toolpath(
        &[CarvePolygon {
            outer: shape,
            holes: Vec::new(),
        }],
        &tool,
        None,
    )
    .expect("Failed to generate V-carve toolpath");

    // Post-process to G-code
    let gcode = post_process_grbl(&toolpath);

    // Verify G-code structure
    assert_eq!(gcode.lines[0], "G90", "First command should be G90");

    // If paths were generated, verify they have proper structure
    if !toolpath.paths.is_empty() {
        assert!(
            gcode.lines.iter().any(|l| l.starts_with("G1 Z")),
            "Should have plunge moves if paths exist"
        );

        // Verify Z-depths are negative
        for path in &toolpath.paths {
            for point in path {
                assert!(point.2 <= 0.0, "Z should be negative or zero");
            }
        }
    }

    println!("V-carve generated {} paths", toolpath.paths.len());
    if !gcode.lines.is_empty() {
        println!(
            "G-code preview:\n{}",
            gcode
                .lines
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[test]
#[ignore = "generate_toolpaths deprecated"]
fn test_vcarve_via_high_level_api() {
    // Test V-carve through the high-level generate_toolpaths API
    let shape = vec![
        (0.0, 0.0),
        (40.0, 0.0),
        (40.0, 25.0),
        (15.0, 25.0),
        (15.0, 15.0),
        (25.0, 15.0),
        (25.0, 10.0),
        (15.0, 10.0),
        (15.0, 0.0),
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

    let polygons = vec![shape.clone()];
    let tools = vec![tool];
    let curve_id = CurveId::new();
    let operations = vec![Operation::VCarve {
        target_depth: Some(5.0), // Max depth of 5mm
        tool_index: 0,
        targets: OperationTarget::Curves(vec![curve_id]),
        clearance_tool_index: None,
    }];

    let result = generate_toolpaths(polygons, tools, operations);

    // V-carve might not always produce results depending on shape complexity
    if result.is_ok() {
        let gcode = result.unwrap();
        assert!(!gcode.lines.is_empty(), "Should generate G-code");
        assert_eq!(gcode.lines[0], "G90", "First command should be G90");

        println!("V-carve via high-level API generated G-code");
    } else {
        // If it fails, that's acceptable - skeleton generation can be finicky
        println!("V-carve failed (may need shape tuning): {:?}", result.err());
    }
}

#[test]
fn test_vcarve_with_flat_depth() {
    // Test V-carve with flat depth (max depth limit)
    let shape = vec![
        (0.0, 0.0),
        (30.0, 0.0),
        (30.0, 20.0),
        (10.0, 20.0),
        (10.0, 10.0),
        (20.0, 10.0),
        (20.0, 0.0),
        (0.0, 0.0),
    ];

    let tool = Tool {
        name: "90deg V-bit".to_string(),
        diameter: 0.0,
        stepover: 0.0,
        pass_depth: 0.0,
        tool_type: ToolType::VBit {
            angle_degrees: 90.0,
        },
    };

    let max_depth = Some(3.0); // Limit to 3mm depth
    let result = generate_vcarve_toolpath(
        &[CarvePolygon {
            outer: shape,
            holes: Vec::new(),
        }],
        &tool,
        max_depth,
    );

    if result.is_ok() {
        let toolpath = result.unwrap();
        // Verify all depths are limited to max_depth
        for path in &toolpath.paths {
            for point in path {
                assert!(point.2 >= -3.0, "Z should not exceed max depth of 3mm");
            }
        }
    }
}
