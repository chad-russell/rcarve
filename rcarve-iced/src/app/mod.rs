use iced::keyboard;
use iced::widget::{button, canvas, center, column, container, pick_list, row, text};
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
mod imports_panel;
mod operation_form;
mod operations_panel;
mod project;
mod stock_form;
mod tool_form;
mod tools_panel;
mod util;

use canvas_view::{WorkspaceCanvas, build_scene};
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
    },
    CanvasDragUpdate(iced::Point),
    CanvasDragEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragMode {
    Translate,
    Rotate,
}

#[derive(Debug, Clone)]
pub struct DragState {
    pub start_cursor_pos: iced::Point,
    pub start_transform: Affine,
    pub import_center: iced::Point,
    pub mode: DragMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Stock,
    Imports,
    Tools,
    Operations,
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
    show_debug_polygons: bool,
    debug_polygons: HashMap<usize, Vec<Vec<(f32, f32)>>>,
    drag_state: Option<DragState>,
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
                self.project = Some(project);
                self.show_stock_modal = false;
                // Reset camera to default view when loading new project
                self.camera = CameraState::default();
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
            column![button(">>").on_press(Message::TogglePanel),]
                .align_x(Alignment::Center)
                .spacing(12)
                .padding(16)
        } else {
            let tab_bar = row![
                tab_button("Stock", SidebarTab::Stock, self.current_tab),
                tab_button("Imports", SidebarTab::Imports, self.current_tab),
                tab_button("Tools", SidebarTab::Tools, self.current_tab),
                tab_button("Operations", SidebarTab::Operations, self.current_tab),
            ]
            .spacing(8);

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
            };

            column![
                row![
                    text(project.name()).size(26),
                    button("<<").on_press(Message::TogglePanel),
                ],
                text(project.path.display().to_string()).size(14),
                tab_bar,
                tab_content,
            ]
            .spacing(16)
            .padding(24)
            .width(Length::Fill)
        };

        let panel = container(panel_content)
            .width(Length::Fixed(panel_width))
            .height(Length::Fill);

        let canvas_program = WorkspaceCanvas {
            scene: self.canvas_scene(),
            camera: self.camera.clone(),
        };

        let canvas_area = container(
            canvas(canvas_program)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill);

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
        )
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
            let mut new_cache = HashMap::new();

            for (index, state) in project.data.operation_states.iter().enumerate() {
                if let Some(artifact) = &state.artifact {
                    valid.insert(index);
                    new_cache.insert(index, Self::flatten_toolpath_segments(artifact));
                }
            }

            self.toolpath_segments = new_cache;

            self.visible_toolpaths.retain(|index| valid.contains(index));

            for index in valid {
                if !self.visible_toolpaths.contains(&index) {
                    self.visible_toolpaths.insert(index);
                }
            }
        } else {
            self.visible_toolpaths.clear();
            self.toolpath_segments.clear();
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

fn tab_button<'a>(
    label: &'static str,
    tab: SidebarTab,
    current: SidebarTab,
) -> Element<'a, Message> {
    let active = tab == current;
    let color = if active {
        iced::Color::from_rgb8(0xdd, 0xdd, 0xdd)
    } else {
        iced::Color::from_rgb8(0xaa, 0xaa, 0xaa)
    };

    let label_text = text(label).style(move |_theme| iced::widget::text::Style {
        color: Some(color),
        ..Default::default()
    });

    button(label_text)
        .padding([6, 12])
        .on_press(Message::SelectTab(tab))
        .into()
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
