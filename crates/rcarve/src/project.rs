use crate::geometry::{CurveId, RegionId, ShapeId, ShapeRegistry};
use crate::{Operation, OperationTarget, Toolpath};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use ulid::Ulid;

/// Increment when the on-disk layout changes.
pub const PROJECT_FILE_VERSION: u32 = 1;

/// High-level representation of a CAM project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub meta: ProjectMeta,
    pub stock: StockSpec,
    pub shapes: ShapeRegistry,
    pub imported_svgs: Vec<SvgImport>,
    pub operations: Vec<Operation>,
    #[serde(default)]
    pub operation_states: Vec<OperationState>,
    pub toolpaths: Vec<Toolpath>,
}

impl Project {
    /// Create a new project seeded with basic metadata and stock information.
    pub fn new(name: impl Into<String>, stock: StockSpec) -> Self {
        let now = current_epoch_ms();
        Self {
            meta: ProjectMeta {
                name: name.into(),
                description: None,
                version: 1,
                created_at_epoch_ms: now,
                updated_at_epoch_ms: now,
                file_version: PROJECT_FILE_VERSION,
            },
            stock,
            shapes: ShapeRegistry::new(),
            imported_svgs: Vec::new(),
            operations: Vec::new(),
            operation_states: Vec::new(),
            toolpaths: Vec::new(),
        }
    }

    /// Persist the project to disk as prettified JSON.
    pub fn save_to_path<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.touch_updated_timestamp();
        let data = serde_json::to_vec_pretty(self).context("serialize project")?;
        fs::write(path, data).context("write project file")
    }

    /// Load a project from disk.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let bytes = fs::read(&path).with_context(|| {
            format!("read project file from {}", path.as_ref().to_string_lossy())
        })?;
        let project: Project =
            serde_json::from_slice(&bytes).context("deserialize project file")?;
        Ok(project)
    }

    /// Update the project's updated timestamp.
    pub fn touch_updated_timestamp(&mut self) {
        self.meta.updated_at_epoch_ms = current_epoch_ms();
    }

    /// Provide a compact summary for UIs.
    pub fn summary(&self) -> ProjectSummary {
        ProjectSummary {
            name: self.meta.name.clone(),
            stock: self.stock.clone(),
            shapes: self.shapes.shapes.len(),
            curves: self.shapes.curves.len(),
            regions: self.shapes.regions.len(),
            svg_imports: self.imported_svgs.len(),
            operations: self.operations.len(),
            toolpaths: self.toolpaths.len(),
        }
    }

    /// Append a new operation to the project, returning its index.
    pub fn add_operation(&mut self, operation: Operation) -> usize {
        self.ensure_operation_states_len();
        self.operations.push(operation);
        self.operation_states.push(OperationState::dirty());
        self.operations.len() - 1
    }

    /// Replace the operation at `index`.
    pub fn update_operation(&mut self, index: usize, operation: Operation) -> Result<()> {
        let slot = self
            .operations
            .get_mut(index)
            .ok_or_else(|| anyhow!("invalid operation index {index}"))?;
        *slot = operation;
        self.mark_operation_dirty(index);
        Ok(())
    }

    /// Remove the operation at `index`, returning it.
    pub fn remove_operation(&mut self, index: usize) -> Result<Operation> {
        if index >= self.operations.len() {
            return Err(anyhow!("invalid operation index {index}"));
        }
        self.ensure_operation_states_len();
        self.operation_states.remove(index);
        Ok(self.operations.remove(index))
    }

    /// Produce summaries suitable for UI lists.
    pub fn operation_summaries(&self) -> Vec<OperationSummary> {
        self.operations
            .iter()
            .enumerate()
            .map(|(index, operation)| OperationSummary::from_operation(index, operation))
            .collect()
    }

    /// Summaries with current toolpath status.
    pub fn operations_with_status(&self) -> Vec<(OperationSummary, ToolpathStatus)> {
        self.operation_summaries()
            .into_iter()
            .enumerate()
            .map(|(index, summary)| {
                let status = self
                    .operation_states
                    .get(index)
                    .map(OperationState::status)
                    .unwrap_or(ToolpathStatus::Dirty);
                (summary, status)
            })
            .collect()
    }

    /// Ensure state vector matches operations length (useful after deserialization).
    pub fn sync_operation_states(&mut self) {
        self.ensure_operation_states_len();
    }

    /// Mark an operation dirty and drop cached toolpath.
    pub fn mark_operation_dirty(&mut self, index: usize) {
        self.ensure_operation_state(index);
        if let Some(state) = self.operation_states.get_mut(index) {
            state.dirty = true;
            state.artifact = None;
        }
    }

    /// Attach a toolpath artifact and mark the operation clean.
    pub fn attach_toolpath(&mut self, index: usize, artifact: ToolpathArtifact) -> Result<()> {
        self.ensure_operation_state(index);
        let state = self
            .operation_states
            .get_mut(index)
            .ok_or_else(|| anyhow!("invalid operation index {index}"))?;
        state.dirty = false;
        state.artifact = Some(artifact);
        Ok(())
    }

    /// Remove any toolpath for an operation and mark it dirty.
    pub fn remove_toolpath_for_operation(&mut self, index: usize) -> Result<()> {
        self.ensure_operation_state(index);
        let state = self
            .operation_states
            .get_mut(index)
            .ok_or_else(|| anyhow!("invalid operation index {index}"))?;
        state.artifact = None;
        state.dirty = true;
        Ok(())
    }

    /// Fetch toolpath artifact for an operation.
    pub fn toolpath_for_operation(&self, index: usize) -> Option<&ToolpathArtifact> {
        self.operation_states
            .get(index)
            .and_then(|state| state.artifact.as_ref())
    }

    fn ensure_operation_states_len(&mut self) {
        if self.operation_states.len() < self.operations.len() {
            let missing = self.operations.len() - self.operation_states.len();
            self.operation_states
                .extend((0..missing).map(|_| OperationState::dirty()));
        }
    }

    fn ensure_operation_state(&mut self, index: usize) {
        if self.operation_states.len() <= index {
            self.ensure_operation_states_len();
        }
    }

    /// Record a newly imported SVG's metadata.
    pub fn add_svg_import(&mut self, import: SvgImport) {
        self.imported_svgs.push(import);
    }

    /// Convenience helper to build and insert a metadata record.
    pub fn record_svg_import(
        &mut self,
        label: impl Into<String>,
        source_path: Option<String>,
        shape_ids: Vec<ShapeId>,
        curve_ids: Vec<CurveId>,
        region_ids: Vec<RegionId>,
    ) {
        let import = SvgImport {
            id: Ulid::new(),
            label: label.into(),
            source_path,
            shape_ids,
            curve_ids,
            region_ids,
            imported_at_epoch_ms: current_epoch_ms(),
        };
        self.imported_svgs.push(import);
    }

    /// Import an SVG file, embedding its geometry into the registry and recording metadata.
    pub fn import_svg<P: AsRef<Path>>(&mut self, path: P) -> Result<SvgImport> {
        let path_ref = path.as_ref();
        let batch = self
            .shapes
            .import_svg(path_ref)
            .with_context(|| format!("import svg {}", path_ref.to_string_lossy()))?;

        let label = path_ref
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Imported SVG".to_string());

        let import = SvgImport {
            id: Ulid::new(),
            label,
            source_path: Some(path_ref.to_string_lossy().to_string()),
            shape_ids: batch.shape_ids.clone(),
            curve_ids: batch.curve_ids.clone(),
            region_ids: batch.region_ids.clone(),
            imported_at_epoch_ms: current_epoch_ms(),
        };
        self.imported_svgs.push(import.clone());
        Ok(import)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub description: Option<String>,
    pub version: u32,
    pub created_at_epoch_ms: u64,
    pub updated_at_epoch_ms: u64,
    pub file_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockSpec {
    pub width: f64,
    pub height: f64,
    pub thickness: f64,
    /// Optional material or notes.
    pub material: Option<String>,
    /// Optional origin offset, useful if the stock is not at (0,0,0).
    pub origin: Option<(f64, f64, f64)>,
}

impl StockSpec {
    pub fn new(width: f64, height: f64, thickness: f64) -> Self {
        Self {
            width,
            height,
            thickness,
            material: None,
            origin: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub name: String,
    pub stock: StockSpec,
    pub shapes: usize,
    pub curves: usize,
    pub regions: usize,
    pub svg_imports: usize,
    pub operations: usize,
    pub toolpaths: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolpathArtifact {
    pub operation_index: usize,
    pub toolpath: Toolpath,
    #[serde(default)]
    pub passes: Vec<ToolpathPass>,
    pub generated_at_epoch_ms: u64,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default = "default_true")]
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolpathPass {
    pub tool_index: usize,
    pub kind: ToolpathPassKind,
    pub toolpath: Toolpath,
}

impl ToolpathPass {
    pub fn new(tool_index: usize, kind: ToolpathPassKind, toolpath: Toolpath) -> Self {
        Self {
            tool_index,
            kind,
            toolpath,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolpathPassKind {
    Clearance,
    Finish,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationState {
    pub dirty: bool,
    #[serde(default)]
    pub artifact: Option<ToolpathArtifact>,
}

impl OperationState {
    pub fn dirty() -> Self {
        Self {
            dirty: true,
            artifact: None,
        }
    }

    pub fn status(&self) -> ToolpathStatus {
        if self.dirty {
            ToolpathStatus::Dirty
        } else if let Some(artifact) = &self.artifact {
            if artifact.is_valid {
                ToolpathStatus::Ready {
                    generated_at_epoch_ms: artifact.generated_at_epoch_ms,
                    warning_count: artifact.warnings.len(),
                }
            } else {
                ToolpathStatus::Invalid {
                    warnings: artifact.warnings.clone(),
                }
            }
        } else {
            ToolpathStatus::Dirty
        }
    }
}

impl Default for OperationState {
    fn default() -> Self {
        Self::dirty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolpathStatus {
    Dirty,
    Ready {
        generated_at_epoch_ms: u64,
        warning_count: usize,
    },
    Invalid {
        warnings: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Profile,
    Pocket,
    VCarve,
}

#[derive(Debug, Clone)]
pub struct OperationSummary {
    pub index: usize,
    pub kind: OperationKind,
    pub target_count: usize,
    pub primary_tool_index: usize,
    pub clearance_tool_index: Option<usize>,
}

impl OperationSummary {
    fn from_operation(index: usize, operation: &Operation) -> Self {
        match operation {
            Operation::Profile {
                target_depth: _,
                cut_side: _,
                tool_index,
                targets,
            } => Self {
                index,
                kind: OperationKind::Profile,
                target_count: count_targets(targets),
                primary_tool_index: *tool_index,
                clearance_tool_index: None,
            },
            Operation::Pocket {
                target_depth: _,
                tool_index,
                target,
            } => Self {
                index,
                kind: OperationKind::Pocket,
                target_count: count_targets(target),
                primary_tool_index: *tool_index,
                clearance_tool_index: None,
            },
            Operation::VCarve {
                target_depth: _,
                tool_index,
                targets,
                clearance_tool_index,
            } => Self {
                index,
                kind: OperationKind::VCarve,
                target_count: count_targets(targets),
                primary_tool_index: *tool_index,
                clearance_tool_index: *clearance_tool_index,
            },
        }
    }
}

fn count_targets(targets: &OperationTarget) -> usize {
    match targets {
        OperationTarget::Curves(curves) => curves.len(),
        OperationTarget::Region(_) => 1,
    }
}

/// Metadata about an imported SVG file with the registry IDs it created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvgImport {
    pub id: Ulid,
    pub label: String,
    pub source_path: Option<String>,
    pub shape_ids: Vec<ShapeId>,
    pub curve_ids: Vec<CurveId>,
    pub region_ids: Vec<RegionId>,
    pub imported_at_epoch_ms: u64,
}

impl SvgImport {
    pub fn new(
        label: impl Into<String>,
        source_path: Option<String>,
        shape_ids: Vec<ShapeId>,
        curve_ids: Vec<CurveId>,
        region_ids: Vec<RegionId>,
        imported_at_epoch_ms: u64,
    ) -> Self {
        Self {
            id: Ulid::new(),
            label: label.into(),
            source_path,
            shape_ids,
            curve_ids,
            region_ids,
            imported_at_epoch_ms,
        }
    }
}

fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_round_trip() {
        let stock = StockSpec::new(100.0, 50.0, 12.5);
        let mut project = Project::new("Demo Project", stock);
        let curve_id = project.shapes.create_line((0.0, 0.0), (10.0, 10.0));
        project.record_svg_import(
            "Test Import",
            Some("demo.svg".to_string()),
            Vec::new(),
            vec![curve_id],
            Vec::new(),
        );

        let mut bytes = Vec::new();
        project.touch_updated_timestamp();
        serde_json::to_writer(&mut bytes, &project).expect("serialize");
        let restored: Project = serde_json::from_slice(&bytes).expect("deserialize");

        assert_eq!(restored.meta.name, "Demo Project");
        assert_eq!(restored.shapes.curves.len(), 1);
        assert_eq!(restored.stock.width, 100.0);
        assert_eq!(restored.imported_svgs.len(), 1);
        assert_eq!(restored.imported_svgs[0].curve_ids.len(), 1);
    }

    #[test]
    fn summary_counts_svg_imports() {
        let stock = StockSpec::new(10.0, 10.0, 1.0);
        let mut project = Project::new("Summary", stock);
        let curve_id = project.shapes.create_line((0.0, 0.0), (1.0, 1.0));
        project.record_svg_import("Line", None, Vec::new(), vec![curve_id], Vec::new());

        let summary = project.summary();
        assert_eq!(summary.svg_imports, 1);
        assert_eq!(summary.curves, 1);
    }

    #[test]
    fn project_import_svg_records_metadata() {
        let stock = StockSpec::new(10.0, 10.0, 1.0);
        let mut project = Project::new("Import", stock);
        let svg_path =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/simple.svg");
        let import = project.import_svg(&svg_path).expect("import svg");
        assert_eq!(project.imported_svgs.len(), 1);
        assert_eq!(import.curve_ids.len(), project.shapes.curves.len());
        assert!(project.imported_svgs[0].source_path.is_some());
    }
}
