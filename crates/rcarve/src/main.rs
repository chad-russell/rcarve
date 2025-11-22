use rcarve::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let operation = args.get(1).map(|s| s.as_str()).unwrap_or("profile");

    match operation {
        "profile" => demo_profile(),
        "pocket" => demo_pocket(),
        "pocket-island" => demo_pocket_with_island(),
        "vcarve" => demo_vcarve(),
        _ => {
            println!("Usage: rcarve [profile|pocket|pocket-island|vcarve]");
            println!("  profile        - Generate profile toolpath (default)");
            println!("  pocket         - Generate simple pocket toolpath");
            println!("  pocket-island  - Generate pocket toolpath with island");
            println!("  vcarve         - Generate V-carve toolpath (Phase 3)");
        }
    }
}

fn demo_profile() {
    println!("rcarve Phase 1 - Profile Toolpath Generator");
    println!("============================================\n");

    // Hardcoded test case matching integration test
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

    match generate_profile_toolpath(&square, &tool, &CutSide::Outside, 5.0) {
        Ok(toolpath) => {
            let gcode = post_process_grbl(&toolpath);
            println!("Generated {} path(s)", toolpath.paths.len());
            println!("\nG-code:\n");
            for line in gcode.lines {
                println!("{}", line);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn demo_pocket() {
    println!("rcarve Phase 2 - Pocket Toolpath Generator");
    println!("==========================================\n");

    // Simple square pocket: 100x100mm
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

    match generate_pocket_toolpath(&outer, &[], &tool, 5.0) {
        Ok(toolpath) => {
            let gcode = post_process_grbl(&toolpath);
            println!(
                "Generated {} concentric path(s) for pocket",
                toolpath.paths.len()
            );
            println!("\nG-code:\n");
            for line in gcode.lines {
                println!("{}", line);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn demo_pocket_with_island() {
    println!("rcarve Phase 2 - Pocket Toolpath Generator (with Island)");
    println!("=======================================================\n");

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

    match generate_pocket_toolpath(&outer, &[island], &tool, 5.0) {
        Ok(toolpath) => {
            let gcode = post_process_grbl(&toolpath);
            println!(
                "Generated {} path(s) for pocket with island",
                toolpath.paths.len()
            );
            println!("\nG-code:\n");
            for line in gcode.lines {
                println!("{}", line);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn demo_vcarve() {
    println!("rcarve Phase 3 - V-Carve Toolpath Generator");
    println!("==========================================\n");

    // Letter-like shape for V-carving (simplified "A" shape)
    let shape = vec![
        (0.0, 0.0),   // Bottom left
        (30.0, 0.0),  // Bottom right
        (25.0, 20.0), // Right side of A
        (20.0, 15.0), // Right inner
        (10.0, 15.0), // Left inner
        (5.0, 20.0),  // Left side of A
        (0.0, 0.0),   // Close
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

    let polygons = vec![CarvePolygon {
        outer: shape,
        holes: Vec::new(),
    }];

    match generate_vcarve_toolpath(&polygons, &tool, Some(5.0)) {
        Ok(path_types) => {
            // Convert PathType to Toolpath
            let mut paths_3d = Vec::new();
            for pt in path_types {
                match pt {
                    PathType::Crease { start, end } => {
                        paths_3d.push(vec![
                            (start[0], start[1], start[2]),
                            (end[0], end[1], end[2]),
                        ]);
                    }
                    PathType::PocketBoundary { path, depth } => {
                        let z = -depth.abs();
                        let path_3d = path.into_iter().map(|p| (p[0], p[1], z)).collect();
                        paths_3d.push(path_3d);
                    }
                }
            }
            let toolpath = Toolpath { paths: paths_3d };
            
            let gcode = post_process_grbl(&toolpath);
            println!("Generated {} path(s) for V-carve", toolpath.paths.len());
            println!("\nG-code:\n");
            for line in gcode.lines {
                println!("{}", line);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
