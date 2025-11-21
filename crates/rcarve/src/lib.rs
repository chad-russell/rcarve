mod geometry;
mod pocket;
mod postprocessor;
mod profile;
mod project;
mod tool_library;
mod toolpath_generation;
mod types;
mod vcarve;

pub use geometry::*;
pub use pocket::generate_pocket_toolpath;
pub use postprocessor::post_process_grbl;
pub use profile::generate_profile_toolpath;
pub use project::*;
pub use tool_library::*;
pub use toolpath_generation::*;
pub use types::*;
pub use vcarve::{generate_vcarve_toolpath, CarvePolygon};

/// High-level function: geometry → toolpath → G-code
///
/// Routes operations to appropriate toolpath generators based on operation type.
/// NOTE: This function is deprecated in favor of the new ShapeRegistry-based API.
/// TODO: Update to work with ShapeRegistry and extract polygons from curves.
#[allow(deprecated)]
pub fn generate_toolpaths(
    _polygons: Vec<Vec<(f64, f64)>>,
    _tools: Vec<Tool>,
    _operations: Vec<Operation>,
) -> anyhow::Result<GCode> {
    // TODO: Implement full support for new Operation structure with CurveId/RegionId
    // For now, this function needs to be refactored to work with ShapeRegistry
    Err(anyhow::anyhow!(
        "generate_toolpaths needs to be updated to work with the new geometry model"
    ))
}
