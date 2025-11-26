use iced::keyboard;
use iced::widget::{button, canvas, center, checkbox, column, container, pick_list, row, text, shader, stack};
use iced::{Alignment, Element, Length, Subscription, Task};
use kurbo::Affine;
use rcarve::ids::CurveId;
use rcarve::{CutSide, StockSpec, ToolLibrary, ToolpathArtifact, ToolpathGenerationReport};
use rfd::AsyncFileDialog;
use std::fmt;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use ulid::Ulid;

mod canvas_view;
mod canvas_view_3d;
mod canvas_view_wgpu;
mod imports_panel;
mod operation_form;
mod operations_panel;
mod project;
mod stock_form;
mod tool_form;
mod tools_panel;
mod util;

use canvas_view::{WorkspaceCanvas, build_scene};
use canvas_view_3d::Workspace3DView;
use canvas_view_wgpu::WorkspaceView3D;
use imports_panel::imports_view;
use operation_form::{OperationForm, OperationKindForm};
use operations_panel::operations_view;
use project::{
    OpenProject, ProjectError, create_new_project, import_svg_into_project, infer_project_name,
    load_project_from_path, projects_directory,
};
use stock_form::StockForm;
use tool_form::{ToolForm, ToolKind};
use util::{format_dimension, format_origin_label, modal_overlay};

pub fn run() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .run_with(App::new)
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePanel,
    OpenProjectDialog,
    ProjectPicked(Result<OpenProject, ProjectError>),
    NewProjectDialog,
    NewProjectCreated(Result<OpenProject, ProjectError>),
    ImportSvg,
    SvgImported(Result<OpenProject, ProjectError>),
    SelectImport(Ulid),
    EditStock,
    CloseStockModal,
    StockWidthChanged(String),
    StockHeightChanged(String),
    StockThicknessChanged(String),
    StockMaterialChanged(String),
    StockOriginChanged(String),
    SaveStock,
    SelectTab(SidebarTab),
    DeleteImport(ulid::Ulid),
    AddTool,
    EditTool(usize),
    DeleteTool(usize),
    CloseToolModal,
    ToolNameChanged(String),
    ToolDiameterChanged(String),
    ToolStepoverChanged(String),
    ToolPassDepthChanged(String),
    ToolVBitAngleChanged(String),
    ToolTypeChanged(ToolKind),
    SaveTool,
    GenerateToolpaths,
    ClearToolpath(usize),
    ToggleToolpathVisibility(usize),
    HoverOperation(Option<usize>),
    ToggleDebugPolygons,
    LogOperationPolygons(usize),
    AddOperation,
    EditOperation(usize),
    DeleteOperation(usize),
    CloseOperationModal,
    OperationKindChanged(OperationKindForm),
    OperationDepthChanged(String),
    OperationCutSideChanged(CutSide),
    OperationToolChanged(usize),
    OperationClearanceToolChanged(Option<usize>),
    OperationVCarveDepthChanged(String),
    RefreshOperationSelection,
    SaveOperation,
    CanvasZoom(f32),
    CanvasPanStart(iced::Point),
    CanvasPanUpdate(iced::Point),
    CanvasPanEnd,
    CanvasDragStart {
        mode: DragMode,
        cursor_position: iced::Point,
        import_center: iced::Point,
        anchor_point: Option<iced::Point>,
    },
    CanvasDragUpdate(iced::Point),
    CanvasDragEnd,
    // 3D View messages
    Canvas3DOrbitStart(iced::Point),
    Canvas3DOrbitUpdate(iced::Point),
    Canvas3DOrbitEnd,
    Canvas3DPanStart(iced::Point),
    Canvas3DPanUpdate(iced::Point),
    Canvas3DPanEnd,
    Canvas3DZoom(f32),
    Toggle3DStockMode,
    Toggle3DCurves,
    // V-carve debug settings modal
    OpenVCarveSettings,
    CloseVCarveSettings,
    ToggleCreasePaths(bool),
    TogglePocketBoundaryPaths(bool),
    ToggleVoronoiPrePrune(bool),
    ToggleVoronoiPostPrune(bool),
    TogglePrunedEdges(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragMode {
    Translate,
    Rotate,
    Scale,
}

#[derive(Debug, Clone)]
pub struct DragState {
    pub start_cursor_pos: iced::Point,
    pub start_transform: Affine,
    pub import_center: iced::Point,
    pub mode: DragMode,
    pub anchor_point: Option<iced::Point>, // For scaling - the opposite corner that stays fixed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Stock,
    Imports,
    Tools,
    Operations,
    View3D,
}

impl Default for SidebarTab {
    fn default() -> Self {
        SidebarTab::Stock
    }
}

#[derive(Default)]
pub struct App {
    panel_collapsed: bool,
    project: Option<OpenProject>,
    opening_dialog: bool,
    creating_dialog: bool,
    importing_svg: bool,
    show_stock_modal: bool,
    stock_form: StockForm,
    selected_import: Option<Ulid>,
    selected_curves: Vec<CurveId>,
    camera: CameraState,
    camera_3d: Camera3DState,
    current_tab: SidebarTab,
    tool_library: ToolLibrary,
    show_tool_modal: bool,
    tool_form: ToolForm,
    editing_tool_index: Option<usize>,
    show_operation_modal: bool,
    operation_form: OperationForm,
    editing_operation_index: Option<usize>,
    generating_toolpaths: bool,
    generation_reports: Vec<ToolpathGenerationReport>,
    visible_toolpaths: HashSet<usize>,
    highlighted_toolpath: Option<usize>,
    toolpath_segments: HashMap<usize, Vec<Vec<(f32, f32)>>>,
    toolpath_segments_3d: HashMap<usize, Vec<Vec<(f32, f32, f32)>>>,
    show_debug_polygons: bool,
    debug_polygons: HashMap<usize, Vec<Vec<(f32, f32)>>>,
    drag_state: Option<DragState>,
    show_3d_stock_wireframe: bool,
    show_3d_curves: bool,
    // V-carve debug settings
    show_vcarve_settings_modal: bool,
    vcarve_debug_settings: VCarveDebugSettings,
    vcarve_debug_edges: HashMap<usize, VCarveDebugEdges>,
}

#[derive(Debug, Clone)]
pub struct CameraState {
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub pan_start: Option<iced::Point>,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            pan_start: None,
        }
    }
}

/// 3D camera state with orbit controls
#[derive(Debug, Clone)]
pub struct Camera3DState {
    /// Horizontal rotation angle in radians
    pub azimuth: f32,
    /// Vertical rotation angle in radians (clamped to avoid gimbal lock)
    pub elevation: f32,
    /// Distance from the camera to the orbit center
    pub distance: f32,
    /// Pan offset in world space
    pub pan_x: f32,
    pub pan_y: f32,
    /// Scene center for orbit (calculated from stock)
    pub center: glam::Vec3,
    /// For tracking drag state
    pub orbit_start: Option<iced::Point>,
    pub pan_start: Option<iced::Point>,
}

impl Default for Camera3DState {
    fn default() -> Self {
        Self {
            azimuth: std::f32::consts::PI * 0.75,  // 135 degrees - isometric-ish view
            elevation: std::f32::consts::PI * 0.25, // 45 degrees
            distance: 300.0,
            pan_x: 0.0,
            pan_y: 0.0,
            center: glam::Vec3::ZERO,
            orbit_start: None,
            pan_start: None,
        }
    }
}

impl Camera3DState {
    /// Calculate camera position from spherical coordinates
    pub fn camera_position(&self) -> glam::Vec3 {
        let x = self.distance * self.elevation.cos() * self.azimuth.cos();
        let y = self.distance * self.elevation.cos() * self.azimuth.sin();
        let z = self.distance * self.elevation.sin();
        self.center + glam::Vec3::new(x, y, z)
    }

    /// Generate view matrix (look-at)
    pub fn view_matrix(&self) -> glam::Mat4 {
        let eye = self.camera_position();
        let target = self.center + glam::Vec3::new(self.pan_x, self.pan_y, 0.0);
        let up = glam::Vec3::Z; // Z-up for CNC coordinate system
        glam::Mat4::look_at_rh(eye, target, up)
    }

    /// Generate perspective projection matrix
    pub fn projection_matrix(&self, aspect_ratio: f32) -> glam::Mat4 {
        let fov = std::f32::consts::PI / 4.0; // 45 degrees
        let near = 0.1;
        let far = 10000.0;
        glam::Mat4::perspective_rh(fov, aspect_ratio, near, far)
    }

    /// Combined view-projection matrix
    pub fn view_projection_matrix(&self, aspect_ratio: f32) -> glam::Mat4 {
        self.projection_matrix(aspect_ratio) * self.view_matrix()
    }

    /// Set center based on stock dimensions
    pub fn set_center_from_stock(&mut self, stock: &StockSpec) {
        let origin = stock.origin.unwrap_or((0.0, 0.0, 0.0));
        self.center = glam::Vec3::new(
            (origin.0 + stock.width / 2.0) as f32,
            (origin.1 + stock.height / 2.0) as f32,
            -(stock.thickness / 2.0) as f32, // Center in Z (negative is down)
        );
        // Set initial distance based on stock size
        let max_dim = stock.width.max(stock.height).max(stock.thickness) as f32;
        self.distance = max_dim * 2.5;
    }
}

/// Settings for V-carve debug visualization
#[derive(Debug, Clone)]
pub struct VCarveDebugSettings {
    /// Show crease paths (Voronoi-derived variable depth)
    pub show_crease_paths: bool,
    /// Show pocket boundary paths (offset at max depth)
    pub show_pocket_boundary_paths: bool,
    /// Show all Voronoi edges before pruning
    pub show_voronoi_pre_prune: bool,
    /// Show Voronoi edges after pruning (kept edges)
    pub show_voronoi_post_prune: bool,
    /// Show edges that were pruned/removed
    pub show_pruned_edges: bool,
}

impl Default for VCarveDebugSettings {
    fn default() -> Self {
        Self {
            show_crease_paths: true,
            show_pocket_boundary_paths: true,
            show_voronoi_pre_prune: false,
            show_voronoi_post_prune: false,
            show_pruned_edges: false,
        }
    }
}

/// Cached debug edge data from V-carve generation
#[derive(Debug, Clone, Default)]
pub struct VCarveDebugEdges {
    /// Pre-prune Voronoi edges [[x1,y1], [x2,y2]]
    pub pre_prune: Vec<[[f32; 2]; 2]>,
    /// Post-prune (kept) Voronoi edges
    pub post_prune: Vec<[[f32; 2]; 2]>,
    /// Pruned (removed) edges
    pub pruned: Vec<[[f32; 2]; 2]>,
    /// Crease path segments with depth
    pub crease_paths: Vec<[[f32; 3]; 2]>,
    /// Pocket boundary paths
    pub pocket_boundary_paths: Vec<Vec<[f32; 2]>>,
}

impl App {
    const PANEL_WIDTH: f32 = 320.0;
    const HANDLE_WIDTH: f32 = 64.0;

    fn new() -> (Self, Task<Message>) {
        let mut app = Self::default();
        app.tool_library = match ToolLibrary::default_library_path()
            .and_then(|path| ToolLibrary::load_from_path(path))
        {
            Ok(library) => library,
            Err(error) => {
                eprintln!("Failed to load tool library: {error}");
                ToolLibrary::new()
            }
        };

        if let Some(default_path) = default_project_path() {
            if default_path.exists() {
                match load_project_from_path(default_path) {
                    Ok(project) => {
                        let _ = app.handle_project_loaded(Ok(project));
                    }
                    Err(error) => {
                        eprintln!("Failed to load default project: {error:?}");
                    }
                }
            }
        }

        (app, Task::none())
    }

    fn title(&self) -> String {
        self.project
            .as_ref()
            .map(|project| project.name().to_string())
            .unwrap_or_else(|| "Rcarve Workspace".to_string())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TogglePanel => {
                self.panel_collapsed = !self.panel_collapsed;
                Task::none()
            }
            Message::OpenProjectDialog => {
                if self.opening_dialog {
                    Task::none()
                } else {
                    self.opening_dialog = true;
                    Task::perform(open_project_dialog(), Message::ProjectPicked)
                }
            }
            Message::ProjectPicked(result) => {
                self.opening_dialog = false;
                self.handle_project_loaded(result)
            }
            Message::NewProjectDialog => {
                if self.creating_dialog {
                    Task::none()
                } else {
                    self.creating_dialog = true;
                    Task::perform(new_project_dialog(), Message::NewProjectCreated)
                }
            }
            Message::NewProjectCreated(result) => {
                self.creating_dialog = false;
                self.handle_project_loaded(result)
            }
            Message::ImportSvg => {
                if self.importing_svg || self.project.is_none() {
                    Task::none()
                } else {
                    self.importing_svg = true;
                    let project_path = self
                        .project
                        .as_ref()
                        .map(|project| project.path.clone())
                        .expect("project path available when project is loaded");
                    Task::perform(import_svg_dialog(project_path), Message::SvgImported)
                }
            }
            Message::SvgImported(result) => {
                self.importing_svg = false;
                self.handle_project_loaded(result)
            }
            Message::SelectImport(id) => {
                // Toggle selection: if already selected, deselect; otherwise select
                if self.selected_import == Some(id) {
                    self.selected_import = None;
                } else {
                    self.selected_import = Some(id);
                }
                self.sync_selected_curves();
                Task::none()
            }
            Message::EditStock => {
                if let Some(project) = &self.project {
                    self.stock_form = StockForm::from_stock(project.stock());
                    self.show_stock_modal = true;
                }
                Task::none()
            }
            Message::CloseStockModal => {
                self.show_stock_modal = false;
                self.stock_form.error = None;
                Task::none()
            }
            Message::StockWidthChanged(value) => {
                self.stock_form.width = value;
                Task::none()
            }
            Message::StockHeightChanged(value) => {
                self.stock_form.height = value;
                Task::none()
            }
            Message::StockThicknessChanged(value) => {
                self.stock_form.thickness = value;
                Task::none()
            }
            Message::StockMaterialChanged(value) => {
                self.stock_form.material = value;
                Task::none()
            }
            Message::StockOriginChanged(value) => {
                self.stock_form.origin = value;
                Task::none()
            }
            Message::SaveStock => {
                if let Some(project) = self.project.as_mut() {
                    match self.stock_form.parse() {
                        Ok(stock) => {
                            project.data.stock = stock;
                            match project.save() {
                                Ok(()) => {
                                    self.stock_form.error = None;
                                    self.show_stock_modal = false;
                                }
                                Err(error) => {
                                    self.stock_form.error = Some(error);
                                }
                            }
                        }
                        Err(error) => {
                            self.stock_form.error = Some(error);
                        }
                    }
                }

                Task::none()
            }
            Message::SelectTab(tab) => {
                self.current_tab = tab;
                Task::none()
            }
            Message::DeleteImport(id) => {
                if let Some(project) = self.project.as_mut() {
                    project.imports.retain(|import| import.id != id);
                    project.data.imported_svgs.retain(|svg| svg.id != id);

                    // Clear selection if we deleted the selected import
                    if self.selected_import == Some(id) {
                        self.selected_import = None;
                    }

                    if let Err(error) = project.save() {
                        eprintln!("Failed to save project after deleting import: {error}");
                    }
                }
                self.sync_selected_curves();
                Task::none()
            }
            Message::AddTool => {
                self.tool_form = ToolForm::new();
                self.editing_tool_index = None;
                self.show_tool_modal = true;
                Task::none()
            }
            Message::EditTool(index) => {
                if let Some(tool) = self.tool_library.tools.get(index) {
                    self.tool_form = ToolForm::from_tool(tool);
                    self.editing_tool_index = Some(index);
                    self.show_tool_modal = true;
                }
                Task::none()
            }
            Message::DeleteTool(index) => {
                if let Err(error) = self.tool_library.remove_tool(index) {
                    eprintln!("Failed to delete tool: {error}");
                } else {
                    self.save_tool_library();
                }
                Task::none()
            }
            Message::CloseToolModal => {
                self.show_tool_modal = false;
                self.tool_form = ToolForm::new();
                self.editing_tool_index = None;
                Task::none()
            }
            Message::ToolNameChanged(value) => {
                self.tool_form.name = value;
                self.tool_form.name_error = None;
                Task::none()
            }
            Message::ToolDiameterChanged(value) => {
                self.tool_form.diameter = value;
                self.tool_form.diameter_error = None;
                Task::none()
            }
            Message::ToolStepoverChanged(value) => {
                self.tool_form.stepover = value;
                self.tool_form.stepover_error = None;
                Task::none()
            }
            Message::ToolPassDepthChanged(value) => {
                self.tool_form.pass_depth = value;
                self.tool_form.pass_depth_error = None;
                Task::none()
            }
            Message::ToolVBitAngleChanged(value) => {
                self.tool_form.vbit_angle = value;
                self.tool_form.vbit_angle_error = None;
                Task::none()
            }
            Message::ToolTypeChanged(kind) => {
                self.tool_form.set_kind(kind);
                self.tool_form.vbit_angle_error = None;
                Task::none()
            }
            Message::SaveTool => {
                match self.tool_form.validate() {
                    Ok(tool) => {
                        if let Some(index) = self.editing_tool_index {
                            if let Err(error) = self.tool_library.update_tool(index, tool) {
                                eprintln!("Failed to update tool: {error}");
                            }
                        } else {
                            self.tool_library.add_tool(tool);
                        }

                        self.save_tool_library();
                        self.show_tool_modal = false;
                        self.editing_tool_index = None;
                        self.tool_form = ToolForm::new();
                    }
                    Err(error) => {
                        eprintln!("Invalid tool definition: {error}");
                    }
                }
                Task::none()
            }
            Message::GenerateToolpaths => {
                if self.generating_toolpaths {
                    return Task::none();
                }
                if let Some(project) = self.project.as_mut() {
                    self.generating_toolpaths = true;
                    let reports = rcarve::generate_toolpaths_for_operations(
                        &mut project.data,
                        &self.tool_library,
                    );
                    
                    // Extract V-carve debug data from reports
                    self.vcarve_debug_edges.clear();
                    for report in &reports {
                        if let Some(debug) = &report.vcarve_debug {
                            let edges = VCarveDebugEdges {
                                pre_prune: debug
                                    .voronoi_edges_pre_prune
                                    .iter()
                                    .map(|e| [[e[0][0] as f32, e[0][1] as f32], [e[1][0] as f32, e[1][1] as f32]])
                                    .collect(),
                                post_prune: debug
                                    .voronoi_edges_post_prune
                                    .iter()
                                    .map(|e| [[e[0][0] as f32, e[0][1] as f32], [e[1][0] as f32, e[1][1] as f32]])
                                    .collect(),
                                pruned: debug
                                    .pruned_edges
                                    .iter()
                                    .map(|e| [[e[0][0] as f32, e[0][1] as f32], [e[1][0] as f32, e[1][1] as f32]])
                                    .collect(),
                                crease_paths: debug
                                    .crease_paths
                                    .iter()
                                    .map(|e| [
                                        [e[0][0] as f32, e[0][1] as f32, e[0][2] as f32],
                                        [e[1][0] as f32, e[1][1] as f32, e[1][2] as f32],
                                    ])
                                    .collect(),
                                pocket_boundary_paths: debug
                                    .pocket_boundary_paths
                                    .iter()
                                    .map(|path| path.iter().map(|p| [p[0] as f32, p[1] as f32]).collect())
                                    .collect(),
                            };
                            self.vcarve_debug_edges.insert(report.operation_index, edges);
                        }
                    }
                    
                    self.generation_reports = reports;
                    if let Err(error) = project.save() {
                        eprintln!("Failed to save project after toolpath generation: {error}");
                    }
                    self.sync_visible_toolpaths();
                    self.sync_debug_polygons();
                    self.generating_toolpaths = false;
                }
                Task::none()
            }
            Message::ClearToolpath(index) => {
                if let Some(project) = self.project.as_mut() {
                    if let Err(error) = project.data.remove_toolpath_for_operation(index) {
                        eprintln!("Failed to clear toolpath: {error}");
                    } else if let Err(error) = project.save() {
                        eprintln!("Failed to save project after clearing toolpath: {error}");
                    }
                    self.sync_visible_toolpaths();
                    self.sync_debug_polygons();
                }
                Task::none()
            }
            Message::ToggleToolpathVisibility(index) => {
                if self.visible_toolpaths.contains(&index) {
                    self.visible_toolpaths.remove(&index);
                } else {
                    // Only allow if toolpath exists
                    if let Some(project) = &self.project {
                        if let Some(state) = project.data.operation_states.get(index) {
                            if state.artifact.is_some() {
                                self.visible_toolpaths.insert(index);
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::HoverOperation(operation) => {
                self.highlighted_toolpath =
                    operation.filter(|index| self.visible_toolpaths.contains(index));
                Task::none()
            }
            Message::ToggleDebugPolygons => {
                self.show_debug_polygons = !self.show_debug_polygons;
                self.sync_debug_polygons();
                Task::none()
            }
            Message::LogOperationPolygons(index) => {
                if let Some(project) = &self.project {
                    match rcarve::polygons_for_operation(&project.data, index) {
                        Ok(polys) => {
                            if polys.is_empty() {
                                eprintln!("Operation {index}: no polygons");
                            } else {
                                eprintln!("Operation {index}: {} polygon(s)", polys.len());
                                for (i, poly) in polys.iter().enumerate() {
                                    if let (Some(first), Some(last)) = (poly.first(), poly.last()) {
                                        eprintln!(
                                            "  #{i}: points={} first=({:.3}, {:.3}) last=({:.3}, {:.3})",
                                            poly.len(),
                                            first.0,
                                            first.1,
                                            last.0,
                                            last.1
                                        );
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Failed to log polygons for operation {index}: {err}");
                        }
                    }
                }
                Task::none()
            }
            Message::AddOperation => {
                let selection = self.current_curve_selection();
                self.operation_form = OperationForm::new();
                self.operation_form.update_selection(&selection);
                self.operation_form.error = None;
                self.show_operation_modal = true;
                self.editing_operation_index = None;
                Task::none()
            }
            Message::EditOperation(index) => {
                if let Some(project) = &self.project {
                    if let Some(operation) = project.data.operations.get(index) {
                        self.operation_form = OperationForm::from_operation(operation);
                        self.editing_operation_index = Some(index);
                        self.show_operation_modal = true;
                    }
                }
                Task::none()
            }
            Message::DeleteOperation(index) => {
                if let Some(project) = self.project.as_mut() {
                    if let Err(error) = project.data.remove_operation(index) {
                        eprintln!("Failed to delete operation: {error}");
                    } else if let Err(error) = project.save() {
                        eprintln!("Failed to save project: {error}");
                    }
                    self.sync_visible_toolpaths();
                    self.sync_debug_polygons();
                }
                Task::none()
            }
            Message::CloseOperationModal => {
                self.show_operation_modal = false;
                self.operation_form = OperationForm::new();
                self.editing_operation_index = None;
                Task::none()
            }
            Message::OperationKindChanged(kind) => {
                self.operation_form.kind = kind;
                self.operation_form.cut_side = match kind {
                    OperationKindForm::Profile => CutSide::Outside,
                    OperationKindForm::Pocket => CutSide::Inside,
                    OperationKindForm::VCarve => CutSide::OnLine,
                };
                self.operation_form.error = None;
                Task::none()
            }
            Message::OperationDepthChanged(value) => {
                self.operation_form.depth = value;
                self.operation_form.error = None;
                Task::none()
            }
            Message::OperationCutSideChanged(cut_side) => {
                self.operation_form.cut_side = cut_side;
                self.operation_form.error = None;
                Task::none()
            }
            Message::OperationToolChanged(index) => {
                self.operation_form.tool_index = Some(index);
                self.operation_form.error = None;
                Task::none()
            }
            Message::OperationClearanceToolChanged(index) => {
                self.operation_form.clearance_tool_index = index;
                self.operation_form.error = None;
                Task::none()
            }
            Message::OperationVCarveDepthChanged(value) => {
                self.operation_form.vcarve_max_depth = value;
                self.operation_form.error = None;
                Task::none()
            }
            Message::RefreshOperationSelection => {
                let selection = self.current_curve_selection();
                self.operation_form.update_selection(&selection);
                self.operation_form.error = None;
                Task::none()
            }
            Message::SaveOperation => {
                let selection = if self.operation_form.selection_snapshot.is_empty() {
                    self.current_curve_selection()
                } else {
                    self.operation_form.selection_snapshot.clone()
                };
                if !selection.is_empty() {
                    self.operation_form.update_selection(&selection);
                }
                match self
                    .operation_form
                    .validate(&selection, self.tool_library.tools.len())
                {
                    Ok(operation) => {
                        if let Some(project) = self.project.as_mut() {
                            let result = if let Some(index) = self.editing_operation_index {
                                project.data.update_operation(index, operation)
                            } else {
                                project.data.add_operation(operation);
                                Ok(())
                            };

                            if let Err(error) = result {
                                eprintln!("Failed to store operation: {error}");
                            } else if let Err(error) = project.save() {
                                eprintln!("Failed to save project: {error}");
                            } else {
                                self.sync_visible_toolpaths();
                                self.sync_debug_polygons();
                            }
                        }

                        self.show_operation_modal = false;
                        self.editing_operation_index = None;
                        self.operation_form = OperationForm::new();
                    }
                    Err(error) => {
                        self.operation_form.error = Some(error);
                    }
                }
                Task::none()
            }
            Message::CanvasZoom(delta) => {
                // Zoom factor: positive delta = zoom in, negative = zoom out
                // Scale zoom by a factor (e.g., 1.1x per scroll unit)
                const ZOOM_FACTOR: f32 = 1.1;
                let zoom_delta = if delta > 0.0 {
                    ZOOM_FACTOR
                } else {
                    1.0 / ZOOM_FACTOR
                };
                self.camera.zoom = (self.camera.zoom * zoom_delta).max(0.1).min(100.0);
                Task::none()
            }
            Message::CanvasPanStart(point) => {
                self.camera.pan_start = Some(point);
                Task::none()
            }
            Message::CanvasPanUpdate(point) => {
                if let Some(start) = self.camera.pan_start {
                    let dx = point.x - start.x;
                    let dy = point.y - start.y;
                    self.camera.pan_x += dx;
                    self.camera.pan_y += dy;
                    self.camera.pan_start = Some(point);
                }
                Task::none()
            }
            Message::CanvasPanEnd => {
                self.camera.pan_start = None;
                Task::none()
            }
            Message::CanvasDragStart {
                mode,
                cursor_position,
                import_center,
                anchor_point,
            } => {
                if let Some(project) = &self.project {
                    if let Some(import_id) = self.selected_import {
                        if let Some(import) = project
                            .data
                            .imported_svgs
                            .iter()
                            .find(|i| i.id == import_id)
                        {
                            self.drag_state = Some(DragState {
                                start_cursor_pos: cursor_position,
                                start_transform: import.transform,
                                import_center,
                                mode,
                                anchor_point,
                            });
                        }
                    }
                }
                Task::none()
            }
            Message::CanvasDragUpdate(cursor_position) => {
                if let Some(drag_state) = &self.drag_state {
                    if let Some(project) = self.project.as_mut() {
                        if let Some(import_id) = self.selected_import {
                            let new_transform = match drag_state.mode {
                                DragMode::Translate => {
                                    let dx = cursor_position.x - drag_state.start_cursor_pos.x;
                                    let dy = cursor_position.y - drag_state.start_cursor_pos.y;
                                    let translation = Affine::translate((dx as f64, dy as f64));
                                    translation * drag_state.start_transform
                                }
                                DragMode::Rotate => {
                                    let center = drag_state.import_center;
                                    let start_angle = (drag_state.start_cursor_pos.y - center.y)
                                        .atan2(drag_state.start_cursor_pos.x - center.x);
                                    let current_angle = (cursor_position.y - center.y)
                                        .atan2(cursor_position.x - center.x);
                                    let delta_angle = current_angle - start_angle;
                                    
                                    let rotation = Affine::rotate_about(
                                        delta_angle as f64,
                                        kurbo::Point::new(center.x as f64, center.y as f64),
                                    );
                                    rotation * drag_state.start_transform
                                }
                                DragMode::Scale => {
                                    if let Some(anchor) = drag_state.anchor_point {
                                        // Calculate distances from cursor to anchor point
                                        let dx_current = cursor_position.x - anchor.x;
                                        let dy_current = cursor_position.y - anchor.y;
                                        let current_distance = (dx_current * dx_current + dy_current * dy_current).sqrt();
                                        
                                        let dx_start = drag_state.start_cursor_pos.x - anchor.x;
                                        let dy_start = drag_state.start_cursor_pos.y - anchor.y;
                                        let start_distance = (dx_start * dx_start + dy_start * dy_start).sqrt();
                                        
                                        // Calculate scale factor (uniform scaling)
                                        let scale_factor = if start_distance > 0.0 {
                                            (current_distance / start_distance) as f64
                                        } else {
                                            1.0
                                        };
                                        
                                        // Apply scale around the anchor point
                                        let anchor_kurbo = kurbo::Point::new(anchor.x as f64, anchor.y as f64);
                                        let scale_transform = Affine::translate(anchor_kurbo.to_vec2())
                                            * Affine::scale(scale_factor)
                                            * Affine::translate(-anchor_kurbo.to_vec2());
                                        scale_transform * drag_state.start_transform
                                    } else {
                                        drag_state.start_transform
                                    }
                                }
                            };

                            if let Err(e) = project.data.update_import_transform(import_id, new_transform) {
                                eprintln!("Failed to update transform: {}", e);
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::CanvasDragEnd => {
                self.drag_state = None;
                if let Some(project) = self.project.as_mut() {
                    if let Err(e) = project.save() {
                        eprintln!("Failed to save project after drag: {}", e);
                    }
                    // Re-sync things if needed, though transform update should be enough
                    self.sync_selected_curves();
                }
                Task::none()
            }
            // 3D View camera controls
            Message::Canvas3DOrbitStart(point) => {
                self.camera_3d.orbit_start = Some(point);
                Task::none()
            }
            Message::Canvas3DOrbitUpdate(point) => {
                if let Some(start) = self.camera_3d.orbit_start {
                    let dx = point.x - start.x;
                    let dy = point.y - start.y;
                    // Sensitivity factor
                    let sensitivity = 0.005;
                    self.camera_3d.azimuth -= dx * sensitivity;
                    self.camera_3d.elevation += dy * sensitivity;
                    // Clamp elevation to avoid gimbal lock
                    self.camera_3d.elevation = self.camera_3d.elevation
                        .clamp(0.1, std::f32::consts::PI - 0.1);
                    self.camera_3d.orbit_start = Some(point);
                }
                Task::none()
            }
            Message::Canvas3DOrbitEnd => {
                self.camera_3d.orbit_start = None;
                Task::none()
            }
            Message::Canvas3DPanStart(point) => {
                self.camera_3d.pan_start = Some(point);
                Task::none()
            }
            Message::Canvas3DPanUpdate(point) => {
                if let Some(start) = self.camera_3d.pan_start {
                    let dx = point.x - start.x;
                    let dy = point.y - start.y;
                    // Pan sensitivity based on distance
                    let sensitivity = self.camera_3d.distance * 0.002;
                    self.camera_3d.pan_x -= dx * sensitivity;
                    self.camera_3d.pan_y += dy * sensitivity;
                    self.camera_3d.pan_start = Some(point);
                }
                Task::none()
            }
            Message::Canvas3DPanEnd => {
                self.camera_3d.pan_start = None;
                Task::none()
            }
            Message::Canvas3DZoom(delta) => {
                // Zoom by adjusting distance
                let zoom_factor = if delta > 0.0 { 0.9 } else { 1.1 };
                self.camera_3d.distance = (self.camera_3d.distance * zoom_factor)
                    .clamp(10.0, 5000.0);
                Task::none()
            }
            Message::Toggle3DStockMode => {
                self.show_3d_stock_wireframe = !self.show_3d_stock_wireframe;
                Task::none()
            }
            Message::Toggle3DCurves => {
                self.show_3d_curves = !self.show_3d_curves;
                Task::none()
            }
            // V-carve debug settings
            Message::OpenVCarveSettings => {
                self.show_vcarve_settings_modal = true;
                Task::none()
            }
            Message::CloseVCarveSettings => {
                self.show_vcarve_settings_modal = false;
                Task::none()
            }
            Message::ToggleCreasePaths(enabled) => {
                self.vcarve_debug_settings.show_crease_paths = enabled;
                Task::none()
            }
            Message::TogglePocketBoundaryPaths(enabled) => {
                self.vcarve_debug_settings.show_pocket_boundary_paths = enabled;
                Task::none()
            }
            Message::ToggleVoronoiPrePrune(enabled) => {
                self.vcarve_debug_settings.show_voronoi_pre_prune = enabled;
                Task::none()
            }
            Message::ToggleVoronoiPostPrune(enabled) => {
                self.vcarve_debug_settings.show_voronoi_post_prune = enabled;
                Task::none()
            }
            Message::TogglePrunedEdges(enabled) => {
                self.vcarve_debug_settings.show_pruned_edges = enabled;
                Task::none()
            }
        }
    }

    fn handle_project_loaded(
        &mut self,
        result: Result<OpenProject, ProjectError>,
    ) -> Task<Message> {
        match result {
            Ok(project) => {
                self.stock_form = StockForm::from_stock(project.stock());
                self.selected_import = project.imports.first().map(|import| import.id);
                // Reset cameras to default view when loading new project
                self.camera = CameraState::default();
                self.camera_3d = Camera3DState::default();
                // Initialize 3D camera center from stock
                self.camera_3d.set_center_from_stock(project.stock());
                self.project = Some(project);
                self.show_stock_modal = false;
                self.sync_selected_curves();
                self.sync_visible_toolpaths();
                self.sync_debug_polygons();
                self.highlighted_toolpath = None;
            }
            Err(ProjectError::DialogClosed) => {}
            Err(ProjectError::Io(message)) => {
                eprintln!("Failed to load project: {message}");
            }
            Err(ProjectError::Parse(message)) => {
                eprintln!("Failed to parse project: {message}");
            }
        }

        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::on_key_press(|key, modifiers| match key.as_ref() {
            keyboard::Key::Character("o") if modifiers.command() => {
                Some(Message::OpenProjectDialog)
            }
            keyboard::Key::Character("n") if modifiers.command() => Some(Message::NewProjectDialog),
            keyboard::Key::Character("i") if modifiers.command() => Some(Message::ImportSvg),
            _ => None,
        })
    }

    fn view(&self) -> Element<'_, Message> {
        if self.project.is_none() {
            return self.blank_state_view();
        }

        let project = self.project.as_ref().expect("project set");
        let panel_width = if self.panel_collapsed {
            Self::HANDLE_WIDTH
        } else {
            Self::PANEL_WIDTH
        };

        let panel_content = if self.panel_collapsed {
            column![button("▶").on_press(Message::TogglePanel),]
                .align_x(Alignment::Center)
                .spacing(12)
                .padding(16)
        } else {
            let tab_bar = row![
                tab_pill("Stock", SidebarTab::Stock, self.current_tab),
                tab_pill("Import", SidebarTab::Imports, self.current_tab),
                tab_pill("Tools", SidebarTab::Tools, self.current_tab),
                tab_pill("Ops", SidebarTab::Operations, self.current_tab),
                tab_pill("3D", SidebarTab::View3D, self.current_tab),
            ]
            .spacing(4)
            .wrap();

            let operation_entries = project.data.operations_with_status();

            let tab_content: Element<'_, Message> = match self.current_tab {
                SidebarTab::Stock => stock_tab_view(project.stock()),
                SidebarTab::Imports => {
                    imports_view(&project.imports, self.selected_import, self.importing_svg)
                }
                SidebarTab::Tools => tools_panel::tools_view(&self.tool_library),
                SidebarTab::Operations => operations_view(
                    operation_entries,
                    &self.tool_library,
                    &self.visible_toolpaths,
                    self.generating_toolpaths,
                    self.show_debug_polygons,
                ),
                SidebarTab::View3D => view_3d_tab_view(self.show_3d_stock_wireframe, self.show_3d_curves),
            };

            // Header with project name and collapse button
            let header = row![
                column![
                    text(project.name()).size(20),
                ]
                .width(Length::Fill),
                button("◀").on_press(Message::TogglePanel),
            ]
            .align_y(Alignment::Center);

            column![
                header,
                tab_bar,
                iced::widget::horizontal_rule(1),
                tab_content,
            ]
            .spacing(12)
            .padding(16)
            .width(Length::Fill)
        };

        let panel = container(panel_content)
            .width(Length::Fixed(panel_width))
            .height(Length::Fill);

        // Choose between 2D and 3D view based on current tab
        let canvas_area: Element<'_, Message> = if self.current_tab == SidebarTab::View3D {
            // 3D View
            let scene_3d = self.build_scene_3d();
            let shader_3d = shader(Workspace3DView {
                scene: scene_3d,
                camera: self.camera_3d.clone(),
                wireframe_mode: self.show_3d_stock_wireframe,
            })
            .width(Length::Fill)
            .height(Length::Fill);

            container(shader_3d)
                .padding(24)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            // 2D View (hybrid approach: WGPU shader for geometry + Canvas overlay for UI)
            let canvas_scene = self.canvas_scene();
            
            let shader_view = shader(WorkspaceView3D {
                scene: canvas_scene.clone(),
                camera: self.camera.clone(),
            })
            .width(Length::Fill)
            .height(Length::Fill);

            let overlay = WorkspaceCanvas {
                scene: canvas_scene,
                camera: self.camera.clone(),
                overlay_only: true,
            };

            container(
                stack![
                    shader_view,
                    canvas(overlay).width(Length::Fill).height(Length::Fill)
                ]
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .padding(24)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        };

        let mut element: Element<'_, Message> = container(
            row![panel, canvas_area]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        if self.show_operation_modal {
            element = modal_overlay(
                element,
                self.operation_modal(),
                Message::CloseOperationModal,
            );
        }

        if self.show_tool_modal {
            element = modal_overlay(element, self.tool_modal(), Message::CloseToolModal);
        }

        if self.show_stock_modal {
            element = modal_overlay(
                element,
                stock_form::modal(&self.stock_form),
                Message::CloseStockModal,
            );
        }

        if self.show_vcarve_settings_modal {
            element = modal_overlay(
                element,
                self.vcarve_settings_modal(),
                Message::CloseVCarveSettings,
            );
        }

        element
    }

    fn blank_state_view(&self) -> Element<'_, Message> {
        let mut open_button = button(if self.opening_dialog {
            "Opening..."
        } else {
            "Open project..."
        })
        .padding([10, 20]);

        if !self.opening_dialog {
            open_button = open_button.on_press(Message::OpenProjectDialog);
        }

        let mut new_button = button(if self.creating_dialog {
            "Creating..."
        } else {
            "New project..."
        })
        .padding([10, 20]);

        if !self.creating_dialog {
            new_button = new_button.on_press(Message::NewProjectDialog);
        }

        let instructions = column![
            text("No project is open").size(28),
            text("Press Command-N to create a new project, or Command-O to open an existing one.")
                .size(16),
            row![new_button, open_button].spacing(12),
        ]
        .align_x(Alignment::Center)
        .spacing(16);

        center(instructions)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn canvas_scene(&self) -> Option<canvas_view::CanvasScene> {
        let project = self.project.as_ref()?;
        
        // Build V-carve debug visualization if any debug edges are stored and settings enabled
        let vcarve_debug = self.build_vcarve_debug();
        
        build_scene(
            project,
            self.selected_import,
            &self.visible_toolpaths,
            self.highlighted_toolpath,
            &self.toolpath_segments,
            if self.show_debug_polygons {
                Some(&self.debug_polygons)
            } else {
                None
            },
            vcarve_debug,
        )
    }
    
    fn build_vcarve_debug(&self) -> Option<canvas_view::CanvasVCarveDebug> {
        let settings = &self.vcarve_debug_settings;
        
        // Check if any visualization is enabled
        if !settings.show_voronoi_pre_prune
            && !settings.show_voronoi_post_prune
            && !settings.show_pruned_edges
            && !settings.show_crease_paths
            && !settings.show_pocket_boundary_paths
        {
            return None;
        }
        
        let mut debug = canvas_view::CanvasVCarveDebug::default();
        
        // Aggregate debug edges from all operations
        for edges in self.vcarve_debug_edges.values() {
            if settings.show_voronoi_pre_prune {
                for edge in &edges.pre_prune {
                    debug.pre_prune_edges.push([
                        iced::Point::new(edge[0][0], edge[0][1]),
                        iced::Point::new(edge[1][0], edge[1][1]),
                    ]);
                }
            }
            
            if settings.show_voronoi_post_prune {
                for edge in &edges.post_prune {
                    debug.post_prune_edges.push([
                        iced::Point::new(edge[0][0], edge[0][1]),
                        iced::Point::new(edge[1][0], edge[1][1]),
                    ]);
                }
            }
            
            if settings.show_pruned_edges {
                for edge in &edges.pruned {
                    debug.pruned_edges.push([
                        iced::Point::new(edge[0][0], edge[0][1]),
                        iced::Point::new(edge[1][0], edge[1][1]),
                    ]);
                }
            }
            
            if settings.show_crease_paths {
                for edge in &edges.crease_paths {
                    debug.crease_paths.push([
                        iced::Point::new(edge[0][0], edge[0][1]),
                        iced::Point::new(edge[1][0], edge[1][1]),
                    ]);
                }
            }
            
            if settings.show_pocket_boundary_paths {
                for path in &edges.pocket_boundary_paths {
                    let points: Vec<iced::Point> = path
                        .iter()
                        .map(|p| iced::Point::new(p[0], p[1]))
                        .collect();
                    debug.pocket_boundary_paths.push(points);
                }
            }
        }
        
        // Return None if no edges were added
        if debug.pre_prune_edges.is_empty()
            && debug.post_prune_edges.is_empty()
            && debug.pruned_edges.is_empty()
            && debug.crease_paths.is_empty()
            && debug.pocket_boundary_paths.is_empty()
        {
            return None;
        }
        
        Some(debug)
    }

    fn build_scene_3d(&self) -> Option<canvas_view_3d::Scene3D> {
        let project = self.project.as_ref()?;
        
        // Build stock
        let stock = canvas_view_3d::Stock3D::from_stock_spec(&project.data.stock);
        
        // Build toolpaths
        let mut toolpaths = Vec::new();
        for (index, segments) in &self.toolpath_segments_3d {
            if !self.visible_toolpaths.contains(index) {
                continue;
            }
            toolpaths.push(canvas_view_3d::Toolpath3D {
                segments: segments.clone(),
                color: canvas_view::toolpath_color(*index),
                highlighted: self.highlighted_toolpath == Some(*index),
            });
        }

        // Build curves
        let mut curves = Vec::new();
        if self.show_3d_curves {
            let z_level = stock.origin.2 + 0.05; // Slight offset to sit on top of stock
            let tolerance = 0.5; // Flatten tolerance

            for import in &project.data.imported_svgs {
                let mut segments = Vec::new();
                for curve_id in &import.curve_ids {
                    if let Some(curve) = project.data.shapes.curves.get(curve_id) {
                        let mut curve = curve.clone();
                        curve.apply_affine(import.transform);

                        let flattened = curve.flatten(tolerance);
                        if flattened.len() < 2 {
                            continue;
                        }
                        
                        let mut segment = Vec::with_capacity(flattened.len());
                        for (x, y) in flattened {
                            segment.push((x as f32, y as f32, z_level));
                        }
                        if segment.len() >= 2 {
                            segments.push(segment);
                        }
                    }
                }

                if !segments.is_empty() {
                    let is_selected = self.selected_import == Some(import.id);
                    curves.push(canvas_view_3d::Curve3D {
                        segments,
                        color: iced::Color::from_rgb8(0x55, 0x55, 0x55),
                        selected: is_selected,
                    });
                }
            }
        }
        
        Some(canvas_view_3d::Scene3D {
            stock: Some(stock),
            toolpaths,
            curves,
        })
    }

    fn sync_selected_curves(&mut self) {
        if let Some(project) = &self.project {
            if let Some(import_id) = self.selected_import {
                if let Some(import) = project.imports.iter().find(|import| import.id == import_id) {
                    self.selected_curves = import.curve_ids.clone();
                    return;
                }
            }
        }
        self.selected_curves.clear();
    }

    fn current_curve_selection(&self) -> Vec<CurveId> {
        self.selected_curves.clone()
    }

    fn sync_visible_toolpaths(&mut self) {
        if let Some(project) = &self.project {
            let mut valid = HashSet::new();
            let mut new_cache_2d = HashMap::new();
            let mut new_cache_3d = HashMap::new();

            for (index, state) in project.data.operation_states.iter().enumerate() {
                if let Some(artifact) = &state.artifact {
                    valid.insert(index);
                    new_cache_2d.insert(index, Self::flatten_toolpath_segments(artifact));
                    new_cache_3d.insert(index, Self::flatten_toolpath_segments_3d(artifact));
                }
            }

            self.toolpath_segments = new_cache_2d;
            self.toolpath_segments_3d = new_cache_3d;

            self.visible_toolpaths.retain(|index| valid.contains(index));

            for index in valid {
                if !self.visible_toolpaths.contains(&index) {
                    self.visible_toolpaths.insert(index);
                }
            }
        } else {
            self.visible_toolpaths.clear();
            self.toolpath_segments.clear();
            self.toolpath_segments_3d.clear();
        }
    }

    fn sync_debug_polygons(&mut self) {
        if !self.show_debug_polygons {
            self.debug_polygons.clear();
            return;
        }

        if let Some(project) = &self.project {
            let mut map = HashMap::new();
            for index in 0..project.data.operations.len() {
                if let Ok(polys) = rcarve::polygons_for_operation(&project.data, index) {
                    let converted = polys
                        .into_iter()
                        .map(|poly| {
                            poly.into_iter()
                                .map(|(x, y)| (x as f32, y as f32))
                                .collect()
                        })
                        .collect();
                    map.insert(index, converted);
                }
            }
            self.debug_polygons = map;
        } else {
            self.debug_polygons.clear();
        }
    }

    fn flatten_toolpath_segments(artifact: &ToolpathArtifact) -> Vec<Vec<(f32, f32)>> {
        let mut segments = Vec::new();

        let mut collect_toolpath = |toolpath: &rcarve::Toolpath| {
            for path in &toolpath.paths {
                if path.len() < 2 {
                    continue;
                }
                let segment: Vec<(f32, f32)> = path
                    .iter()
                    .map(|(x, y, _)| (*x as f32, *y as f32))
                    .collect();
                if segment.len() >= 2 {
                    segments.push(segment);
                }
            }
        };

        if artifact.passes.is_empty() {
            collect_toolpath(&artifact.toolpath);
        } else {
            for pass in &artifact.passes {
                collect_toolpath(&pass.toolpath);
            }
        }

        segments
    }

    fn flatten_toolpath_segments_3d(artifact: &ToolpathArtifact) -> Vec<Vec<(f32, f32, f32)>> {
        let mut segments = Vec::new();

        let mut collect_toolpath = |toolpath: &rcarve::Toolpath| {
            for path in &toolpath.paths {
                if path.len() < 2 {
                    continue;
                }
                let segment: Vec<(f32, f32, f32)> = path
                    .iter()
                    .map(|(x, y, z)| (*x as f32, *y as f32, *z as f32))
                    .collect();
                if segment.len() >= 2 {
                    segments.push(segment);
                }
            }
        };

        if artifact.passes.is_empty() {
            collect_toolpath(&artifact.toolpath);
        } else {
            for pass in &artifact.passes {
                collect_toolpath(&pass.toolpath);
            }
        }

        segments
    }

    fn tool_modal(&self) -> Element<'_, Message> {
        let title = if self.editing_tool_index.is_some() {
            "Edit Tool"
        } else {
            "Add Tool"
        };

        let mut content = column![
            text(title).size(24),
            text_input_row(
                "Name",
                &self.tool_form.name,
                Message::ToolNameChanged,
                self.tool_form.name_error.as_deref()
            ),
            text_input_row(
                "Diameter (mm)",
                &self.tool_form.diameter,
                Message::ToolDiameterChanged,
                self.tool_form.diameter_error.as_deref()
            ),
            text_input_row(
                "Stepover (0.0 - 1.0)",
                &self.tool_form.stepover,
                Message::ToolStepoverChanged,
                self.tool_form.stepover_error.as_deref()
            ),
            text_input_row(
                "Pass depth (mm)",
                &self.tool_form.pass_depth,
                Message::ToolPassDepthChanged,
                self.tool_form.pass_depth_error.as_deref()
            ),
            tool_type_picker(&self.tool_form),
        ]
        .spacing(16);

        if matches!(self.tool_form.kind, ToolKind::VBit) {
            content = content.push(text_input_row(
                "V-bit angle (degrees)",
                &self.tool_form.vbit_angle,
                Message::ToolVBitAngleChanged,
                self.tool_form.vbit_angle_error.as_deref(),
            ));
        }

        content = content.push(
            row![
                button("Cancel").on_press(Message::CloseToolModal),
                button("Save Tool").on_press(Message::SaveTool),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );

        container(content)
            .padding(24)
            .width(Length::Fixed(360.0))
            .style(container::rounded_box)
            .into()
    }

    fn operation_modal(&self) -> Element<'_, Message> {
        let title = if self.editing_operation_index.is_some() {
            "Edit Operation"
        } else {
            "Add Operation"
        };

        let type_selector = row![
            operation_type_button(
                "Profile",
                OperationKindForm::Profile,
                self.operation_form.kind
            ),
            operation_type_button(
                "Pocket",
                OperationKindForm::Pocket,
                self.operation_form.kind
            ),
            operation_type_button(
                "V-Carve",
                OperationKindForm::VCarve,
                self.operation_form.kind
            ),
        ]
        .spacing(8);

        let selection_row = row![
            text(format!(
                "Selected curves: {}",
                self.operation_form.selection_snapshot.len()
            ))
            .size(12),
            button("Use current selection")
                .padding([4, 8])
                .on_press(Message::RefreshOperationSelection),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let depth_field: Element<'_, Message> = match self.operation_form.kind {
            OperationKindForm::VCarve => text_input_row(
                "Max depth (optional, mm)",
                &self.operation_form.vcarve_max_depth,
                Message::OperationVCarveDepthChanged,
                None,
            ),
            _ => text_input_row(
                "Depth (mm)",
                &self.operation_form.depth,
                Message::OperationDepthChanged,
                None,
            ),
        };

        let tool_options: Vec<ToolOption> = self
            .tool_library
            .tools
            .iter()
            .enumerate()
            .map(|(index, tool)| ToolOption {
                index,
                label: tool.name.clone(),
            })
            .collect();

        let tool_section: Element<'_, Message> = if tool_options.is_empty() {
            column![
                text("Tool").size(12),
                text("Add a tool in the Tools tab before creating an operation.").size(12),
                button("Go to Tools")
                    .on_press(Message::SelectTab(SidebarTab::Tools))
                    .padding([4, 8]),
            ]
            .spacing(8)
            .into()
        } else {
            let selected_tool = self
                .operation_form
                .tool_index
                .and_then(|index| tool_options.iter().find(|opt| opt.index == index).cloned());

            column![
                text("Tool").size(12),
                pick_list(tool_options.clone(), selected_tool, |option: ToolOption| {
                    Message::OperationToolChanged(option.index)
                },),
            ]
            .spacing(4)
            .into()
        };

        let mut content = column![
            text(title).size(24),
            column![text("Operation type").size(12), type_selector].spacing(4),
            selection_row,
            depth_field,
            tool_section,
        ]
        .spacing(16);

        if self.operation_form.kind == OperationKindForm::Profile {
            content = content.push(
                column![
                    text("Cut side").size(12),
                    row![
                        cut_side_button("Outside", CutSide::Outside, &self.operation_form.cut_side),
                        cut_side_button("Inside", CutSide::Inside, &self.operation_form.cut_side),
                        cut_side_button("On line", CutSide::OnLine, &self.operation_form.cut_side),
                    ]
                    .spacing(8),
                ]
                .spacing(4),
            );
        }

        if self.operation_form.kind == OperationKindForm::VCarve && !tool_options.is_empty() {
            let mut clearance_options = Vec::with_capacity(tool_options.len() + 1);
            clearance_options.push(ClearanceChoice::None);
            clearance_options.extend(tool_options.iter().cloned().map(ClearanceChoice::Tool));

            let selected_clearance = self
                .operation_form
                .clearance_tool_index
                .and_then(|index| {
                    tool_options
                        .iter()
                        .find(|opt| opt.index == index)
                        .map(|opt| ClearanceChoice::Tool(opt.clone()))
                })
                .unwrap_or(ClearanceChoice::None);

            let picker = pick_list(
                clearance_options,
                Some(selected_clearance.clone()),
                |choice: ClearanceChoice| match choice {
                    ClearanceChoice::None => Message::OperationClearanceToolChanged(None),
                    ClearanceChoice::Tool(option) => {
                        Message::OperationClearanceToolChanged(Some(option.index))
                    }
                },
            );

            content = content
                .push(column![text("Clearance tool (optional)").size(12), picker].spacing(4));
        }

        if let Some(error) = &self.operation_form.error {
            let color = iced::Color::from_rgb8(0xE5, 0x54, 0x54);
            content = content.push(text(error).style(move |_theme| iced::widget::text::Style {
                color: Some(color),
                ..Default::default()
            }));
        }

        content = content.push(
            row![
                button("Cancel").on_press(Message::CloseOperationModal),
                button("Save Operation").on_press(Message::SaveOperation),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );

        container(content)
            .padding(24)
            .width(Length::Fixed(420.0))
            .style(container::rounded_box)
            .into()
    }

    fn vcarve_settings_modal(&self) -> Element<'_, Message> {
        let settings = &self.vcarve_debug_settings;

        let content = column![
            text("V-Carve Debug Settings").size(24),
            text("Path Visualization").size(16),
            checkbox("Show Crease Paths (blue)", settings.show_crease_paths)
                .on_toggle(Message::ToggleCreasePaths),
            checkbox(
                "Show Pocket Boundary Paths (cyan)",
                settings.show_pocket_boundary_paths
            )
            .on_toggle(Message::TogglePocketBoundaryPaths),
            text("Voronoi Edge Visualization").size(16),
            checkbox(
                "Show Pre-Prune Edges (gray)",
                settings.show_voronoi_pre_prune
            )
            .on_toggle(Message::ToggleVoronoiPrePrune),
            checkbox(
                "Show Post-Prune Edges (green)",
                settings.show_voronoi_post_prune
            )
            .on_toggle(Message::ToggleVoronoiPostPrune),
            checkbox("Show Pruned Edges (red)", settings.show_pruned_edges)
                .on_toggle(Message::TogglePrunedEdges),
            row![button("Close").on_press(Message::CloseVCarveSettings),]
                .spacing(10)
                .align_y(Alignment::Center),
        ]
        .spacing(12);

        container(content)
            .padding(24)
            .width(Length::Fixed(380.0))
            .style(container::rounded_box)
            .into()
    }

    fn save_tool_library(&self) {
        match ToolLibrary::default_library_path() {
            Ok(path) => {
                if let Err(error) = self.tool_library.save_to_path(path) {
                    eprintln!("Failed to save tool library: {error}");
                }
            }
            Err(error) => eprintln!("Failed to determine tool library path: {error}"),
        }
    }
}

fn stock_tab_view(stock: &StockSpec) -> Element<'static, Message> {
    let card = container(
        column![
            text("Stock").size(20),
            row![
                text(format!("Width: {} mm", format_dimension(stock.width))),
                text(format!("Height: {} mm", format_dimension(stock.height))),
                text(format!(
                    "Thickness: {} mm",
                    format_dimension(stock.thickness)
                )),
            ]
            .spacing(12)
            .wrap(),
            text(format!(
                "Material: {}",
                stock
                    .material
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("—")
            )),
            text(format!("Origin: {}", format_origin_label(stock.origin))),
            button("Edit Stock").on_press(Message::EditStock),
        ]
        .spacing(12),
    )
    .padding(16)
    .width(Length::Fill)
    .style(container::rounded_box);

    card.into()
}

fn view_3d_tab_view(wireframe_mode: bool, show_curves: bool) -> Element<'static, Message> {
    let mode_label = if wireframe_mode {
        "Wireframe"
    } else {
        "Solid"
    };

    let curves_label = if show_curves {
        "Hide"
    } else {
        "Show"
    };
    
    let card = container(
        column![
            text("3D View").size(20),
            text("Visualize stock and toolpaths in 3D").size(14),
            row![
                text("Stock rendering:").size(14),
                button(mode_label)
                    .padding([4, 12])
                    .on_press(Message::Toggle3DStockMode),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Imported curves:").size(14),
                button(curves_label)
                    .padding([4, 12])
                    .on_press(Message::Toggle3DCurves),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            column![
                text("Controls:").size(14),
                text("• Left drag: Orbit camera").size(12),
                text("• Right drag: Pan").size(12),
                text("• Scroll: Zoom").size(12),
            ]
            .spacing(4),
        ]
        .spacing(12),
    )
    .padding(16)
    .width(Length::Fill)
    .style(container::rounded_box);

    card.into()
}

fn tab_pill<'a>(
    label: &'static str,
    tab: SidebarTab,
    current: SidebarTab,
) -> Element<'a, Message> {
    let active = tab == current;
    
    let label_text = text(label)
        .size(13)
        .style(move |_theme| iced::widget::text::Style {
            color: Some(if active {
                iced::Color::WHITE
            } else {
                iced::Color::from_rgb8(0x99, 0x99, 0x99)
            }),
            ..Default::default()
        });

    let btn = button(label_text)
        .padding([4, 10])
        .on_press(Message::SelectTab(tab))
        .style(move |theme, status| {
            let base = button::primary(theme, status);
            if active {
                button::Style {
                    background: Some(iced::Background::Color(
                        iced::Color::from_rgb8(0x4a, 0x6f, 0xc9)
                    )),
                    text_color: iced::Color::WHITE,
                    border: iced::Border::default().rounded(4),
                    ..base
                }
            } else {
                button::Style {
                    background: Some(iced::Background::Color(
                        iced::Color::from_rgb8(0x3a, 0x3a, 0x3a)
                    )),
                    text_color: iced::Color::from_rgb8(0x99, 0x99, 0x99),
                    border: iced::Border::default().rounded(4),
                    ..base
                }
            }
        });

    btn.into()
}

fn text_input_row<'a>(
    label: &'static str,
    value: &'a str,
    on_input: fn(String) -> Message,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    let mut column = column![
        text(label).size(12),
        iced::widget::text_input(label, value)
            .padding(8)
            .on_input(on_input),
    ]
    .spacing(4);

    if let Some(message) = error {
        let color = iced::Color::from_rgb8(0xE5, 0x54, 0x54);
        column = column.push(
            text(message).style(move |_theme| iced::widget::text::Style {
                color: Some(color),
                ..Default::default()
            }),
        );
    }

    column.into()
}

fn tool_type_picker(form: &ToolForm) -> Element<'_, Message> {
    let options_row = row![
        type_toggle("Endmill", ToolKind::Endmill, form.kind),
        type_toggle("V-bit", ToolKind::VBit, form.kind),
        type_toggle("Ballnose", ToolKind::Ballnose, form.kind),
    ]
    .spacing(8);

    column![text("Tool type").size(12), options_row]
        .spacing(4)
        .into()
}

fn type_toggle<'a>(label: &'static str, kind: ToolKind, current: ToolKind) -> Element<'a, Message> {
    let active = kind == current;
    let color = if active {
        iced::Color::from_rgb8(0x2a, 0x64, 0xc5)
    } else {
        iced::Color::from_rgb8(0x55, 0x55, 0x55)
    };

    let label_text = text(label).style(move |_theme| iced::widget::text::Style {
        color: Some(color),
        ..Default::default()
    });

    button(label_text)
        .padding([4, 8])
        .on_press(Message::ToolTypeChanged(kind))
        .into()
}

fn operation_type_button<'a>(
    label: &'static str,
    kind: OperationKindForm,
    current: OperationKindForm,
) -> Element<'a, Message> {
    let active = kind == current;
    let color = if active {
        iced::Color::from_rgb8(0x2a, 0x64, 0xc5)
    } else {
        iced::Color::from_rgb8(0x55, 0x55, 0x55)
    };

    let label_text = text(label).style(move |_theme| iced::widget::text::Style {
        color: Some(color),
        ..Default::default()
    });

    button(label_text)
        .padding([4, 8])
        .on_press(Message::OperationKindChanged(kind))
        .into()
}

fn cut_side_button<'a>(
    label: &'static str,
    side: CutSide,
    current: &CutSide,
) -> Element<'a, Message> {
    let active = cut_side_eq(&side, current);
    let color = if active {
        iced::Color::from_rgb8(0x2a, 0x64, 0xc5)
    } else {
        iced::Color::from_rgb8(0x55, 0x55, 0x55)
    };

    let label_text = text(label).style(move |_theme| iced::widget::text::Style {
        color: Some(color),
        ..Default::default()
    });

    button(label_text)
        .padding([4, 8])
        .on_press(Message::OperationCutSideChanged(side))
        .into()
}

fn cut_side_eq(left: &CutSide, right: &CutSide) -> bool {
    matches!(
        (left, right),
        (CutSide::Inside, CutSide::Inside)
            | (CutSide::Outside, CutSide::Outside)
            | (CutSide::OnLine, CutSide::OnLine)
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ToolOption {
    index: usize,
    label: String,
}

impl fmt::Display for ToolOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ClearanceChoice {
    None,
    Tool(ToolOption),
}

impl fmt::Display for ClearanceChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClearanceChoice::None => write!(f, "None"),
            ClearanceChoice::Tool(option) => write!(f, "{}", option.label),
        }
    }
}

fn open_project_dialog() -> impl std::future::Future<Output = Result<OpenProject, ProjectError>> {
    async {
        let mut dialog = AsyncFileDialog::new()
            .set_title("Open a .rcproj file")
            .add_filter("Rcarve Project", &["rcproj"]);

        if let Some(dir) = projects_directory() {
            dialog = dialog.set_directory(dir);
        }

        let picked = dialog.pick_file().await.ok_or(ProjectError::DialogClosed)?;
        let path = picked.path().to_path_buf();

        load_project_from_path(path)
    }
}

fn new_project_dialog() -> impl std::future::Future<Output = Result<OpenProject, ProjectError>> {
    async {
        let mut dialog = AsyncFileDialog::new()
            .set_title("Create a new .rcproj file")
            .add_filter("Rcarve Project", &["rcproj"]);

        if let Some(dir) = projects_directory() {
            dialog = dialog.set_directory(dir);
        }

        let picked = dialog.save_file().await.ok_or(ProjectError::DialogClosed)?;
        let mut path = picked.path().to_path_buf();

        if path.extension().and_then(|s| s.to_str()) != Some("rcproj") {
            path.set_extension("rcproj");
        }

        let name = infer_project_name(&path);
        create_new_project(path.clone(), name)?;
        load_project_from_path(path)
    }
}

fn import_svg_dialog(
    project_path: PathBuf,
) -> impl std::future::Future<Output = Result<OpenProject, ProjectError>> {
    async move {
        let picked = AsyncFileDialog::new()
            .set_title("Import an SVG file")
            .add_filter("Scalable Vector Graphics", &["svg"])
            .pick_file()
            .await
            .ok_or(ProjectError::DialogClosed)?;

        let svg_path = picked.path().to_path_buf();
        import_svg_into_project(project_path, svg_path)
    }
}

fn default_project_path() -> Option<PathBuf> {
    projects_directory().map(|dir| dir.join("Bar.rcproj"))
}
