use rcarve::{CurveId, Project as RcarveProject, ShapeId, StockSpec, SvgImport};
use std::path::{Path, PathBuf};
use ulid::Ulid;

#[derive(Debug, Clone)]
pub struct OpenProject {
    pub path: PathBuf,
    pub data: RcarveProject,
    pub imports: Vec<ImportedSvgEntry>,
}

impl OpenProject {
    pub fn new(path: PathBuf, data: RcarveProject) -> Self {
        let mut data = data;
        data.sync_operation_states();
        let imports = data
            .imported_svgs
            .iter()
            .map(ImportedSvgEntry::from)
            .collect();
        Self {
            path,
            data,
            imports,
        }
    }

    pub fn name(&self) -> &str {
        &self.data.meta.name
    }

    pub fn stock(&self) -> &StockSpec {
        &self.data.stock
    }

    pub fn save(&mut self) -> Result<(), String> {
        self.data
            .save_to_path(&self.path)
            .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct ImportedSvgEntry {
    pub id: Ulid,
    pub label: String,
    pub source_path: Option<String>,
    pub curve_ids: Vec<CurveId>,
    pub shape_ids: Vec<ShapeId>,
}

impl From<&SvgImport> for ImportedSvgEntry {
    fn from(import: &SvgImport) -> Self {
        Self {
            id: import.id,
            label: import.label.clone(),
            source_path: import.source_path.clone(),
            curve_ids: import.curve_ids.clone(),
            shape_ids: import.shape_ids.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ProjectError {
    DialogClosed,
    Io(String),
    Parse(String),
}

pub fn projects_directory() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .map(|home| home.join(".rcarve/projects"))
}

pub fn infer_project_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|name| name.to_string())
        .unwrap_or_else(|| path.display().to_string())
}

pub fn create_new_project(path: PathBuf, name: String) -> Result<(), ProjectError> {
    let stock = StockSpec::new(100.0, 100.0, 12.5);
    let mut project = RcarveProject::new(name, stock);
    project
        .save_to_path(&path)
        .map_err(|error| ProjectError::Io(error.to_string()))
}

pub fn load_project_from_path(path: PathBuf) -> Result<OpenProject, ProjectError> {
    let data = RcarveProject::load_from_path(&path)
        .map_err(|error| ProjectError::Io(error.to_string()))?;
    Ok(OpenProject::new(path, data))
}

pub fn import_svg_into_project(
    project_path: PathBuf,
    svg_path: PathBuf,
) -> Result<OpenProject, ProjectError> {
    let mut data = RcarveProject::load_from_path(&project_path)
        .map_err(|error| ProjectError::Io(error.to_string()))?;

    data.import_svg(&svg_path)
        .map_err(|error| ProjectError::Parse(error.to_string()))?;

    data.save_to_path(&project_path)
        .map_err(|error| ProjectError::Io(error.to_string()))?;

    Ok(OpenProject::new(project_path, data))
}
