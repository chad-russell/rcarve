use iced::widget::canvas::{self, Program};
use iced::{Color, Point, Rectangle, Renderer, Theme, event, keyboard, mouse};
use kurbo::Affine;
use ulid::Ulid;

use super::CameraState;
use super::project::OpenProject;
use super::{DragMode, Message};
use std::collections::HashSet;

const CURVE_FLATTEN_TOLERANCE: f64 = 0.5;

pub struct WorkspaceCanvas {
    pub scene: Option<CanvasScene>,
    pub camera: CameraState,
    pub overlay_only: bool,
}

impl Program<Message> for WorkspaceCanvas {
    type State = keyboard::Modifiers;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (event::Status, Option<Message>) {
        match event {
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if let Some(_cursor_position) = cursor.position_in(bounds) {
                    let scroll_delta = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => y / 100.0,
                        mouse::ScrollDelta::Pixels { y, .. } => y / 800.0, // Normalize pixel scroll
                    };
                    (
                        event::Status::Captured,
                        Some(Message::CanvasZoom(scroll_delta)),
                    )
                } else {
                    (event::Status::Ignored, None)
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(cursor_position) = cursor.position_in(bounds) {
                    // Check for hit on handles or bodies
                    if let Some(scene) = &self.scene {
                        let (scale, offset) =
                            calculate_transform(bounds.size(), &scene.bounds, &self.camera);

                        // Transform cursor to world space
                        let world_cursor = Point::new(
                            (cursor_position.x - offset.x) / scale + scene.bounds.min.x,
                            scene.bounds.max.y - (cursor_position.y - offset.y) / scale,
                        );

                        // Check handles first (if any selected)
                        for import in &scene.imports {
                            if import.selected {
                                // Check corner handles for scaling (before rotation handle)
                                let hit_radius = 12.0 / scale;
                                
                                // Define the 4 corners in world coordinates
                                let corners = [
                                    (Point::new(import.bounds.x, import.bounds.y + import.bounds.height), 
                                     Point::new(import.bounds.x + import.bounds.width, import.bounds.y)), // Top-left, anchor: bottom-right
                                    (Point::new(import.bounds.x + import.bounds.width, import.bounds.y + import.bounds.height), 
                                     Point::new(import.bounds.x, import.bounds.y)), // Top-right, anchor: bottom-left
                                    (Point::new(import.bounds.x, import.bounds.y), 
                                     Point::new(import.bounds.x + import.bounds.width, import.bounds.y + import.bounds.height)), // Bottom-left, anchor: top-right
                                    (Point::new(import.bounds.x + import.bounds.width, import.bounds.y), 
                                     Point::new(import.bounds.x, import.bounds.y + import.bounds.height)), // Bottom-right, anchor: top-left
                                ];
                                
                                for (corner_pos, anchor_pos) in &corners {
                                    let dist = world_cursor.distance(*corner_pos);
                                    if dist < hit_radius {
                                        let center = Point::new(
                                            import.bounds.x + import.bounds.width / 2.0,
                                            import.bounds.y + import.bounds.height / 2.0,
                                        );
                                        
                                        return (
                                            event::Status::Captured,
                                            Some(Message::CanvasDragStart {
                                                mode: DragMode::Scale,
                                                cursor_position: world_cursor,
                                                import_center: center,
                                                anchor_point: Some(*anchor_pos),
                                            }),
                                        );
                                    }
                                }
                                
                                // Check rotation handle
                                let handle_world_pos = calculate_handle_position(&import.bounds, scale);

                                // Hit radius: visual radius (6px) + padding (6px) = 12px total
                                let hit_radius = 12.0 / scale;
                                let dist = world_cursor.distance(handle_world_pos);

                                if dist < hit_radius {
                                    let center = Point::new(
                                        import.bounds.x + import.bounds.width / 2.0,
                                        import.bounds.y + import.bounds.height / 2.0,
                                    );

                                    return (
                                        event::Status::Captured,
                                        Some(Message::CanvasDragStart {
                                            mode: DragMode::Rotate,
                                            cursor_position: world_cursor,
                                            import_center: center,
                                            anchor_point: None,
                                        }),
                                    );
                                }
                            }
                        }

                        // Check bodies
                        for import in &scene.imports {
                            if world_cursor.x >= import.bounds.x
                                && world_cursor.x <= import.bounds.x + import.bounds.width
                                && world_cursor.y >= import.bounds.y
                                && world_cursor.y <= import.bounds.y + import.bounds.height
                            {
                                let center = Point::new(
                                    import.bounds.x + import.bounds.width / 2.0,
                                    import.bounds.y + import.bounds.height / 2.0,
                                );

                                // If not selected, we should select it.
                                // But selection logic is in App::update via SelectImport.
                                // We can emit SelectImport if we want, but here we want to start drag.
                                // If we start drag on unselected item, we should probably select it first.
                                // For now, let's assume we can drag even if not previously selected,
                                // or the App will handle selection on click.
                                // Actually, if we return DragStart, the App should probably also ensure it's selected.

                                return (
                                    event::Status::Captured,
                                    Some(if import.selected {
                                        Message::CanvasDragStart {
                                            mode: DragMode::Translate,
                                            cursor_position: world_cursor,
                                            import_center: center,
                                            anchor_point: None,
                                        }
                                    } else {
                                        Message::SelectImport(import.id)
                                    }),
                                );
                            }
                        }
                    }

                    (
                        event::Status::Captured,
                        Some(Message::CanvasPanStart(cursor_position)),
                    )
                } else {
                    (event::Status::Ignored, None)
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if self.camera.pan_start.is_some() {
                    (event::Status::Captured, Some(Message::CanvasPanEnd))
                } else {
                    (event::Status::Captured, Some(Message::CanvasDragEnd))
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(cursor_position) = cursor.position_in(bounds) {
                    if self.camera.pan_start.is_some() {
                        (
                            event::Status::Captured,
                            Some(Message::CanvasPanUpdate(cursor_position)),
                        )
                    } else {
                        // If we are dragging, we need to emit DragUpdate with WORLD coordinates
                        if let Some(scene) = &self.scene {
                            let (scale, offset) =
                                calculate_transform(bounds.size(), &scene.bounds, &self.camera);
                            let world_cursor = Point::new(
                                (cursor_position.x - offset.x) / scale + scene.bounds.min.x,
                                scene.bounds.max.y - (cursor_position.y - offset.y) / scale,
                            );
                            (
                                event::Status::Captured,
                                Some(Message::CanvasDragUpdate(world_cursor)),
                            )
                        } else {
                            (event::Status::Ignored, None)
                        }
                    }
                } else {
                    (event::Status::Ignored, None)
                }
            }
            canvas::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                *state = modifiers;
                (event::Status::Ignored, None)
            }
            _ => (event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let palette = theme.extended_palette();
        let background = palette.background.weak.color;
        let accent = palette.primary.strong.color;

        if !self.overlay_only {
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), background);
            draw_crosshair(&mut frame, bounds.size(), accent);
        }

        if let Some(scene) = &self.scene {
            draw_scene(&mut frame, scene, bounds.size(), &self.camera, self.overlay_only);
        } else if !self.overlay_only {
            draw_placeholder_circle(&mut frame, bounds.size(), accent);
        }

        vec![frame.into_geometry()]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CanvasToolpathMeta {
    #[allow(dead_code)]
    pub operation_index: usize,
    pub color: iced::Color,
    pub highlighted: bool,
}

#[derive(Debug, Clone)]
pub struct CanvasScene {
    pub imports: Vec<CanvasImport>,
    pub toolpaths: Vec<CanvasToolpath>,
    pub debug_polygons: Vec<CanvasDebugPolygon>,
    pub vcarve_debug: Option<CanvasVCarveDebug>,
    pub stock: Option<CanvasStock>,
    pub bounds: Bounds,
}

#[derive(Debug, Clone)]
pub struct CanvasStock {
    pub rect: Rectangle,
}

#[derive(Debug, Clone)]
pub struct CanvasImport {
    pub id: Ulid,
    pub polylines: Vec<Vec<Point>>, // Already flattened curves
    pub selected: bool,
    pub bounds: Rectangle,
    pub transform: Affine,
}

#[derive(Debug, Clone)]
pub struct CanvasToolpath {
    pub meta: CanvasToolpathMeta,
    pub segments: Vec<Vec<Point>>,
}

#[derive(Debug, Clone)]
pub struct CanvasDebugPolygon {
    pub color: iced::Color,
    pub segments: Vec<Vec<Point>>,
}

/// Debug visualization for V-carve Voronoi edges
#[derive(Debug, Clone, Default)]
pub struct CanvasVCarveDebug {
    /// Pre-prune Voronoi edges (light gray)
    pub pre_prune_edges: Vec<[Point; 2]>,
    /// Post-prune (kept) Voronoi edges (green)
    pub post_prune_edges: Vec<[Point; 2]>,
    /// Pruned (removed) edges (red)
    pub pruned_edges: Vec<[Point; 2]>,
    /// Crease path segments (blue)
    pub crease_paths: Vec<[Point; 2]>,
    /// Pocket boundary path segments (cyan)
    pub pocket_boundary_paths: Vec<Vec<Point>>,
}

#[derive(Debug, Clone)]
pub struct Bounds {
    pub min: Point,
    pub max: Point,
}

impl Bounds {
    fn new(point: Point) -> Self {
        Self {
            min: point,
            max: point,
        }
    }

    fn include(&mut self, point: Point) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
    }

    pub fn width(&self) -> f32 {
        (self.max.x - self.min.x).max(1.0)
    }

    pub fn height(&self) -> f32 {
        (self.max.y - self.min.y).max(1.0)
    }
}

pub fn build_scene(
    project: &OpenProject,
    selected: Option<Ulid>,
    visible_toolpaths: &HashSet<usize>,
    highlighted_toolpath: Option<usize>,
    cached_segments: &std::collections::HashMap<usize, Vec<Vec<(f32, f32)>>>,
    debug_polygons: Option<&std::collections::HashMap<usize, Vec<Vec<(f32, f32)>>>>,
    vcarve_debug: Option<CanvasVCarveDebug>,
) -> Option<CanvasScene> {
    let mut bounds: Option<Bounds> = None;
    let mut imports = Vec::new();
    let mut toolpaths = Vec::new();

    // Process stock
    let stock_spec = &project.data.stock;
    let origin = stock_spec.origin.unwrap_or((0.0, 0.0, 0.0));
    let stock_rect = Rectangle {
        x: origin.0 as f32,
        y: origin.1 as f32,
        width: stock_spec.width as f32,
        height: stock_spec.height as f32,
    };

    // Use stock bounds as fixed camera reference
    let stock_min = Point::new(stock_rect.x, stock_rect.y);
    let stock_max = Point::new(
        stock_rect.x + stock_rect.width,
        stock_rect.y + stock_rect.height,
    );

    let mut b = Bounds::new(stock_min);
    b.include(stock_max);
    bounds = Some(b);

    for import in &project.data.imported_svgs {
        let mut polylines = Vec::new();
        let mut import_bounds = Bounds::new(Point::ORIGIN);
        let mut first_point = true;

        for curve_id in &import.curve_ids {
            if let Some(curve) = project.data.shapes.curves.get(curve_id) {
                let mut curve = curve.clone();
                curve.apply_affine(import.transform);

                let flattened = curve.flatten(CURVE_FLATTEN_TOLERANCE);
                if flattened.len() < 2 {
                    continue;
                }
                let mut points = Vec::with_capacity(flattened.len());
                for (x, y) in flattened {
                    let point = Point::new(x as f32, y as f32);
                    
                    // Track import bounds but don't expand scene bounds
                    // (scene bounds are fixed to stock for stable camera)
                    if first_point {
                        import_bounds = Bounds::new(point);
                        first_point = false;
                    } else {
                        import_bounds.include(point);
                    }

                    points.push(point);
                }
                if points.len() >= 2 {
                    polylines.push(points);
                }
            }
        }

        if !polylines.is_empty() {
            imports.push(CanvasImport {
                id: import.id,
                polylines,
                selected: selected == Some(import.id),
                bounds: Rectangle {
                    x: import_bounds.min.x,
                    y: import_bounds.min.y,
                    width: import_bounds.width(),
                    height: import_bounds.height(),
                },
                transform: import.transform,
            });
        }
    }

    // Add toolpaths to scene (don't expand bounds - camera is fixed to stock)
    for (index, state) in project.data.operation_states.iter().enumerate() {
        if !visible_toolpaths.contains(&index) {
            continue;
        }
        if state.artifact.is_none() {
            continue;
        }

        let mut segments = Vec::new();
        let raw_segments = match cached_segments.get(&index) {
            Some(segments) => segments,
            None => continue,
        };

        for raw in raw_segments {
            if raw.len() < 2 {
                continue;
            }
            let mut segment = Vec::new();
            for &(x, y) in raw {
                let point = Point::new(x, y);
                segment.push(point);
            }
            if segment.len() >= 2 {
                segments.push(segment);
            }
        }

        if segments.is_empty() {
            continue;
        }

        let color = toolpath_color(index);
        toolpaths.push(CanvasToolpath {
            meta: CanvasToolpathMeta {
                operation_index: index,
                color,
                highlighted: highlighted_toolpath == Some(index),
            },
            segments,
        });
    }

    if imports.is_empty() && toolpaths.is_empty() {
        return None;
    }

    let mut bounds = match bounds {
        Some(b) => b,
        None => return None,
    };

    let mut debug_paths = Vec::new();
    if let Some(polygons) = debug_polygons {
        for (index, polys) in polygons {
            let mut segments = Vec::new();
            for poly in polys {
                if poly.len() < 2 {
                    continue;
                }
                let mut segment = Vec::new();
                for &(x, y) in poly {
                    let point = Point::new(x, y);
                    segment.push(point);
                }
                if segment.len() >= 2 {
                    segments.push(segment);
                }
            }
            if !segments.is_empty() {
                debug_paths.push(CanvasDebugPolygon {
                    color: toolpath_color(*index),
                    segments,
                });
            }
        }
    }

    Some(CanvasScene {
        imports,
        toolpaths,
        debug_polygons: debug_paths,
        vcarve_debug,
        stock: Some(CanvasStock { rect: stock_rect }),
        bounds,
    })
}

fn draw_crosshair(frame: &mut canvas::Frame, size: iced::Size, color: Color) {
    let center = Point::new(size.width / 2.0, size.height / 2.0);
    let mut accent = color;
    accent.a = 0.3;

    let horizontal =
        canvas::Path::line(Point::new(0.0, center.y), Point::new(size.width, center.y));
    let vertical = canvas::Path::line(Point::new(center.x, 0.0), Point::new(center.x, size.height));
    let stroke = canvas::Stroke::default().with_color(accent).with_width(1.0);
    frame.stroke(&horizontal, stroke);
    frame.stroke(&vertical, stroke);
}

fn draw_placeholder_circle(frame: &mut canvas::Frame, size: iced::Size, color: Color) {
    let center = Point::new(size.width / 2.0, size.height / 2.0);
    let circle = canvas::Path::circle(center, (size.width.min(size.height)) * 0.2);
    let mut fill_color = color;
    fill_color.a = 0.08;
    frame.fill(&circle, fill_color);
    frame.stroke(
        &circle,
        canvas::Stroke::default().with_color(color).with_width(1.0),
    );
}

fn draw_scene(
    frame: &mut canvas::Frame,
    scene: &CanvasScene,
    size: iced::Size,
    camera: &CameraState,
    overlay_only: bool,
) {
    // This function now only draws UI overlays (selection handles, gizmos)
    // The actual scene geometry (imports, toolpaths, stock) is rendered by the WGPU shader
    
    if overlay_only {
        let (scale, offset) = calculate_transform(size, &scene.bounds, camera);
        
        // Draw selection handles and manipulation gizmos
        for import in &scene.imports {
            if import.selected {
                // Draw bounding box
                let top_left_world = Point::new(
                    import.bounds.x,
                    import.bounds.y + import.bounds.height,
                );

                let screen_min = world_to_screen(top_left_world, &scene.bounds, scale, offset);
                let screen_size =
                    iced::Size::new(import.bounds.width * scale, import.bounds.height * scale);

                let bounds_path = canvas::Path::rectangle(screen_min, screen_size);
                frame.stroke(
                    &bounds_path,
                    canvas::Stroke::default()
                        .with_color(Color::from_rgb8(0xFD, 0x7E, 0x14))
                        .with_width(1.0),
                );

                // Draw corner handles for scaling
                let corner_positions = [
                    Point::new(import.bounds.x, import.bounds.y + import.bounds.height), // Top-left
                    Point::new(import.bounds.x + import.bounds.width, import.bounds.y + import.bounds.height), // Top-right
                    Point::new(import.bounds.x, import.bounds.y), // Bottom-left
                    Point::new(import.bounds.x + import.bounds.width, import.bounds.y), // Bottom-right
                ];
                
                for corner_world in &corner_positions {
                    let corner_screen = world_to_screen(*corner_world, &scene.bounds, scale, offset);
                    
                    // Draw square handle (8x8 pixels)
                    let handle_size = iced::Size::new(8.0, 8.0);
                    let handle_pos = Point::new(corner_screen.x - 4.0, corner_screen.y - 4.0);
                    
                    let handle_rect = canvas::Path::rectangle(handle_pos, handle_size);
                    frame.fill(&handle_rect, Color::WHITE);
                    frame.stroke(
                        &handle_rect,
                        canvas::Stroke::default()
                            .with_color(Color::from_rgb8(0x00, 0x7A, 0xFF))
                            .with_width(1.5),
                    );
                }
                
                // Draw rotation handle
                let handle_world_pos = calculate_handle_position(&import.bounds, scale);
                let handle_screen_pos = world_to_screen(handle_world_pos, &scene.bounds, scale, offset);
                
                // Top center of bounding box
                let top_center_screen =
                    Point::new(screen_min.x + screen_size.width / 2.0, screen_min.y);

                // Line connecting handle to bounding box
                let connector = canvas::Path::line(top_center_screen, handle_screen_pos);
                frame.stroke(
                    &connector,
                    canvas::Stroke::default()
                        .with_color(Color::from_rgb8(0x00, 0x7A, 0xFF))
                        .with_width(1.5),
                );

                // Handle circle
                let handle_circle = canvas::Path::circle(handle_screen_pos, 6.0);
                frame.fill(&handle_circle, Color::from_rgb8(0x00, 0x7A, 0xFF));
                frame.stroke(
                    &handle_circle,
                    canvas::Stroke::default()
                        .with_color(Color::WHITE)
                        .with_width(1.5),
                );
            }
        }
    }
}

pub fn toolpath_color(index: usize) -> Color {
    match index % 6 {
        0 => Color::from_rgb8(0x5B, 0x8D, 0xFF),
        1 => Color::from_rgb8(0xFF, 0x98, 0x6C),
        2 => Color::from_rgb8(0x8B, 0xE9, 0x66),
        3 => Color::from_rgb8(0xFF, 0xD7, 0x5E),
        4 => Color::from_rgb8(0xC1, 0x7D, 0xFF),
        _ => Color::from_rgb8(0x76, 0xE4, 0xFF),
    }
}

fn calculate_transform(size: iced::Size, bounds: &Bounds, camera: &CameraState) -> (f32, Point) {
    let width = bounds.width();
    let height = bounds.height();

    let base_scale = (size.width / width).min(size.height / height) * 0.9;
    let scale = base_scale * camera.zoom;

    let base_offset = Point::new(
        (size.width - width * base_scale) / 2.0,
        (size.height - height * base_scale) / 2.0,
    );

    let offset = Point::new(base_offset.x + camera.pan_x, base_offset.y + camera.pan_y);

    (scale, offset)
}

/// Calculate handle position in world coordinates (20px above bounding box)
fn calculate_handle_position(bounds: &Rectangle, scale: f32) -> Point {
    let handle_offset_world = 20.0 / scale;
    Point::new(
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height + handle_offset_world,
    )
}

/// Convert world coordinates to screen coordinates
fn world_to_screen(world_pos: Point, scene_bounds: &Bounds, scale: f32, offset: Point) -> Point {
    let normalized_x = world_pos.x - scene_bounds.min.x;
    let normalized_y = scene_bounds.max.y - world_pos.y;
    Point::new(
        normalized_x * scale + offset.x,
        normalized_y * scale + offset.y,
    )
}
