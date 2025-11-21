use crate::geometry::ids::{CurveId, RegionId};
use serde::{Deserialize, Serialize};

/// Represents a single tool in the tool library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub diameter: f64,
    /// The percentage of the tool's diameter to step over, e.g., 0.4 for 40%.
    pub stepover: f64,
    /// The maximum Z-depth to cut in a single pass.
    pub pass_depth: f64,
    /// The specific geometry of the tool.
    pub tool_type: ToolType,
}

/// Defines the geometric type of the tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolType {
    /// A flat-bottomed cylindrical cutter.
    Endmill { diameter: f64 },
    /// A V-shaped cutter defined by its included angle.
    VBit { angle_degrees: f64 },
    /// A cylindrical cutter with a hemispherical tip.
    Ballnose { diameter: f64 },
}

/// Defines which side of the vector to cut.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CutSide {
    Inside,
    Outside,
    OnLine,
}

/// Target geometry for an operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationTarget {
    /// Target one or more curves.
    Curves(Vec<CurveId>),
    /// Target a region (outer boundary with optional holes).
    Region(RegionId),
}

/// A single CAM operation to be performed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    /// A 2D profile cut along one or more curves.
    Profile {
        /// The final Z-depth for this operation.
        target_depth: f64,
        /// The side(s) of the geometry to cut.
        cut_side: CutSide,
        /// Index of the tool to use from the tool library.
        tool_index: usize,
        /// The curves to apply this operation to.
        targets: OperationTarget,
        // Future additions: tabs, ramps, leads.
    },
    /// A 2D pocketing operation to clear an area.
    Pocket {
        target_depth: f64,
        tool_index: usize,
        /// The region to pocket (outer boundary with optional holes).
        target: OperationTarget,
        // Future additions: island handling, pocketing strategy (offset/raster).
    },
    /// A V-carving operation.
    VCarve {
        /// Optional: A maximum depth for flat-bottom v-carving.
        target_depth: Option<f64>,
        tool_index: usize,
        /// The curves to apply this operation to.
        targets: OperationTarget,
        /// Optional: A second tool for clearing large areas.
        clearance_tool_index: Option<usize>,
    },
}

/// Represents a complete, continuous 3D tool movement path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toolpath {
    /// A list of (X, Y, Z) coordinates.
    pub paths: Vec<Vec<(f64, f64, f64)>>,
    // Metadata could include feed rates, spindle speeds, etc.
}

/// Represents the final, machine-specific G-code output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GCode {
    /// A list of G-code command strings.
    pub lines: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_construction() {
        let tool = Tool {
            name: "6mm Endmill".to_string(),
            diameter: 6.0,
            stepover: 0.4,
            pass_depth: 5.0,
            tool_type: ToolType::Endmill { diameter: 6.0 },
        };
        assert_eq!(tool.diameter, 6.0);
    }

    #[test]
    fn test_operation_construction() {
        let curve_id = CurveId::new();
        let op = Operation::Profile {
            target_depth: 5.0,
            cut_side: CutSide::Outside,
            tool_index: 0,
            targets: OperationTarget::Curves(vec![curve_id]),
        };
        match op {
            Operation::Profile { target_depth, .. } => {
                assert_eq!(target_depth, 5.0);
            }
            _ => panic!("Expected Profile operation"),
        }
    }

    #[test]
    fn test_toolpath_construction() {
        let toolpath = Toolpath {
            paths: vec![vec![(0.0, 0.0, -5.0), (100.0, 0.0, -5.0)]],
        };
        assert_eq!(toolpath.paths.len(), 1);
        assert_eq!(toolpath.paths[0].len(), 2);
    }

    #[test]
    fn test_gcode_construction() {
        let gcode = GCode {
            lines: vec!["G90".to_string(), "G21".to_string()],
        };
        assert_eq!(gcode.lines.len(), 2);
        assert_eq!(gcode.lines[0], "G90");
    }
}
