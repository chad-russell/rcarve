use rcarve::*;

#[test]
fn test_square_profile() {
    // Input: 100x100mm square (Section 2.1, line 150)
    let input_poly = vec![
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0, 100.0),
        (0.0, 100.0),
        (0.0, 0.0),
    ];
    
    // Tool: 6mm endmill (Section 2.1, line 151)
    let tool = Tool {
        name: "6mm Endmill".to_string(),
        diameter: 6.0,
        stepover: 0.4,
        pass_depth: 5.0,
        tool_type: ToolType::Endmill { diameter: 6.0 },
    };
    
    // Operation: Outside profile, 5mm depth (Section 2.1, line 152)
    let toolpath = generate_profile_toolpath(
        &input_poly,
        &tool,
        &CutSide::Outside,
        5.0,
    ).expect("Failed to generate toolpath");
    
    // Post-process to G-code
    let gcode = post_process_grbl(&toolpath);
    
    // Critical assertion (Section 2.4, line 215)
    // For 100x100 square + 6mm bit (radius=3.0), outside profile
    // corners should be at (-3.0, -3.0) and (103.0, 103.0)
    let gcode_str = gcode.lines.join("\n");
    assert!(
        gcode_str.contains("X103.0") || gcode_str.contains("X103."),
        "Expected offset corner coordinate X103.0"
    );
    assert!(
        gcode_str.contains("X-3.0") || gcode_str.contains("X-3."),
        "Expected offset corner coordinate X-3.0"
    );
    
    // Verify G-code structure
    assert!(gcode.lines[0] == "G90", "First command should be G90");
    assert!(gcode.lines.iter().any(|l| l.starts_with("G1 Z")), "Should have plunge move");
    
    println!("Generated G-code:\n{}", gcode_str);
}

