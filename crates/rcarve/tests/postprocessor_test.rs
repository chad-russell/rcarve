use rcarve::*;

#[test]
fn test_postprocessor_empty_toolpath() {
    // Test post-processor with empty toolpath (no paths)
    let toolpath = Toolpath { paths: vec![] };

    let gcode = post_process_grbl(&toolpath);

    // Should still have header commands
    assert!(gcode.lines.len() >= 4, "Should have header commands");
    assert_eq!(gcode.lines[0], "G90", "Should have G90");
    assert_eq!(gcode.lines[1], "G21", "Should have G21");
    assert_eq!(gcode.lines[2], "G17", "Should have G17");
    assert_eq!(gcode.lines[3], "G0 Z10.0", "Should have safe height");

    // Should not have any cutting moves
    assert!(
        !gcode.lines.iter().any(|l| l.starts_with("G1 X")),
        "Should not have cutting moves for empty toolpath"
    );
}

#[test]
fn test_postprocessor_multiple_paths() {
    // Test post-processor with multiple paths
    let toolpath = Toolpath {
        paths: vec![
            vec![(0.0, 0.0, -5.0), (10.0, 0.0, -5.0), (10.0, 10.0, -5.0)],
            vec![(20.0, 20.0, -5.0), (30.0, 20.0, -5.0), (30.0, 30.0, -5.0)],
        ],
    };

    let gcode = post_process_grbl(&toolpath);

    // Should have header
    assert_eq!(gcode.lines[0], "G90", "First command should be G90");

    // Should have multiple exit moves (one per path, plus one in header)
    // Header has "G0 Z10.0", then each path adds another "G0 Z10.0"
    let exit_moves = gcode.lines.iter().filter(|l| l == &"G0 Z10.0").count();
    assert_eq!(
        exit_moves, 3,
        "Should have three exit moves (1 header + 2 paths)"
    );

    // Should have multiple entry moves (one per path)
    let entry_moves = gcode.lines.iter().filter(|l| l.starts_with("G0 X")).count();
    assert_eq!(entry_moves, 2, "Should have two entry moves for two paths");

    // Should have multiple plunge moves (one per path)
    let plunge_moves = gcode.lines.iter().filter(|l| l.starts_with("G1 Z")).count();
    assert_eq!(
        plunge_moves, 2,
        "Should have two plunge moves for two paths"
    );
}

#[test]
fn test_postprocessor_single_point_path() {
    // Test post-processor with a path containing only one point
    let toolpath = Toolpath {
        paths: vec![vec![(10.0, 20.0, -5.0)]],
    };

    let gcode = post_process_grbl(&toolpath);

    // Should have header
    assert_eq!(gcode.lines[0], "G90", "First command should be G90");

    // Should have entry move
    assert!(
        gcode.lines.iter().any(|l| l.starts_with("G0 X")),
        "Should have entry move"
    );

    // Should have plunge move
    assert!(
        gcode.lines.iter().any(|l| l.starts_with("G1 Z")),
        "Should have plunge move"
    );

    // Should have exit move
    assert!(
        gcode.lines.iter().any(|l| l == "G0 Z10.0"),
        "Should have exit move"
    );

    // Should not have any cutting moves (only one point, so no G1 X Y moves after plunge)
    let cutting_moves = gcode
        .lines
        .iter()
        .filter(|l| l.starts_with("G1 X") && !l.starts_with("G1 Z"))
        .count();
    assert_eq!(
        cutting_moves, 0,
        "Should not have cutting moves for single point path"
    );
}

#[test]
fn test_postprocessor_empty_paths_skipped() {
    // Test post-processor with empty paths (should be skipped)
    let toolpath = Toolpath {
        paths: vec![
            vec![], // Empty path - should be skipped
            vec![(0.0, 0.0, -5.0), (10.0, 0.0, -5.0)],
            vec![], // Another empty path - should be skipped
        ],
    };

    let gcode = post_process_grbl(&toolpath);

    // Should have header
    assert_eq!(gcode.lines[0], "G90", "First command should be G90");

    // Should have exactly two exit moves (1 header + 1 for the non-empty path)
    // Empty paths are skipped, so only one path gets processed
    let exit_moves = gcode.lines.iter().filter(|l| l == &"G0 Z10.0").count();
    assert_eq!(
        exit_moves, 2,
        "Should have two exit moves (1 header + 1 for non-empty path, empty paths skipped)"
    );

    // Should have exactly one entry move
    let entry_moves = gcode.lines.iter().filter(|l| l.starts_with("G0 X")).count();
    assert_eq!(
        entry_moves, 1,
        "Should have one entry move (empty paths should be skipped)"
    );
}
