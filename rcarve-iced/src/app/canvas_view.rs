use iced::widget::canvas::{self, Program};
use iced::{Color, Point, Rectangle, Renderer, Theme, event, mouse};
use ulid::Ulid;

use super::CameraState;
use super::Message;
use super::project::OpenProject;
use std::collections::HashSet;

const CURVE_FLATTEN_TOLERANCE: f64 = 0.5;

pub struct WorkspaceCanvas {
    pub scene: Option<CanvasScene>,
    pub camera: CameraState,
}

impl Program<Message> for WorkspaceCanvas {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (event::Status, Option<Message>) {
        match event {
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if let Some(_cursor_position) = cursor.position_in(bounds) {
                    let scroll_delta = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => y,
                        mouse::ScrollDelta::Pixels { y, .. } => y / 10.0, // Normalize pixel scroll
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
                    (
                        event::Status::Captured,
                        Some(Message::CanvasPanStart(cursor_position)),
                    )
                } else {
                    (event::Status::Ignored, None)
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                (event::Status::Captured, Some(Message::CanvasPanEnd))
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if self.camera.pan_start.is_some() {
                    if let Some(cursor_position) = cursor.position_in(bounds) {
                        (
                            event::Status::Captured,
                            Some(Message::CanvasPanUpdate(cursor_position)),
                        )
                    } else {
                        (event::Status::Ignored, None)
                    }
                } else {
                    (event::Status::Ignored, None)
                }
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

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), background);
        draw_crosshair(&mut frame, bounds.size(), accent);

        if let Some(scene) = &self.scene {
            draw_scene(&mut frame, scene, bounds.size(), &self.camera);
        } else {
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
    pub stock: Option<CanvasStock>,
    pub bounds: Bounds,
}

#[derive(Debug, Clone)]
pub struct CanvasStock {
    pub rect: Rectangle,
}

#[derive(Debug, Clone)]
pub struct CanvasImport {
    pub polylines: Vec<Vec<Point>>, // Already flattened curves
    pub selected: bool,
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

#[derive(Debug, Clone)]
pub struct Bounds {
    min: Point,
    max: Point,
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

    fn width(&self) -> f32 {
        (self.max.x - self.min.x).max(1.0)
    }

    fn height(&self) -> f32 {
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
    
    // Initialize bounds with stock
    let stock_min = Point::new(stock_rect.x, stock_rect.y);
    let stock_max = Point::new(stock_rect.x + stock_rect.width, stock_rect.y + stock_rect.height);
    
    let mut b = Bounds::new(stock_min);
    b.include(stock_max);
    bounds = Some(b);

    for import in &project.imports {
        let mut polylines = Vec::new();

        for curve_id in &import.curve_ids {
            if let Some(curve) = project.data.shapes.curves.get(curve_id) {
                let flattened = curve.flatten(CURVE_FLATTEN_TOLERANCE);
                if flattened.len() < 2 {
                    continue;
                }
                let mut points = Vec::with_capacity(flattened.len());
                for (x, y) in flattened {
                    let point = Point::new(x as f32, y as f32);
                    if let Some(existing) = &mut bounds {
                        existing.include(point);
                    } else {
                        bounds = Some(Bounds::new(point));
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
                polylines,
                selected: selected == Some(import.id),
            });
        }
    }

    // Incorporate toolpaths into bounds and scene
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
                if let Some(existing) = &mut bounds {
                    existing.include(point);
                } else {
                    bounds = Some(Bounds::new(point));
                }
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
                    bounds.include(point);
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
) {
    let width = scene.bounds.width();
    let height = scene.bounds.height();

    // Base scale to fit scene in view (only used if zoom is at default 1.0)
    let base_scale = (size.width / width).min(size.height / height) * 0.9;

    // Apply camera zoom
    let scale = base_scale * camera.zoom;

    // Base offset to center scene (only used if pan is at default 0,0)
    let base_offset = Point::new(
        (size.width - width * base_scale) / 2.0,
        (size.height - height * base_scale) / 2.0,
    );

    // Apply camera pan
    let offset = Point::new(base_offset.x + camera.pan_x, base_offset.y + camera.pan_y);

    // Draw Stock
    if let Some(stock) = &scene.stock {
        // Stock is stored in CNC coordinates (bottom-left origin usually).
        // We need to calculate the top-left corner in screen coordinates.
        // CNC Top-Left is (x, y + h)
        let cnc_top_left = Point::new(stock.rect.x, stock.rect.y + stock.rect.height);
        
        let normalized_x = cnc_top_left.x - scene.bounds.min.x;
        let normalized_y = scene.bounds.max.y - cnc_top_left.y;
        
        let screen_pos = Point::new(
            normalized_x * scale + offset.x,
            normalized_y * scale + offset.y,
        );
        
        let screen_size = iced::Size::new(
            stock.rect.width * scale,
            stock.rect.height * scale,
        );

        let stock_color = Color::from_rgb8(240, 240, 240); // Light gray
        frame.fill_rectangle(screen_pos, screen_size, stock_color);
        
        // Optional: Draw border
        let border_path = canvas::Path::rectangle(screen_pos, screen_size);
        frame.stroke(
            &border_path, 
            canvas::Stroke::default()
                .with_color(Color::from_rgb8(200, 200, 200))
                .with_width(1.0)
        );
    }

    for import in &scene.imports {
        let stroke = canvas::Stroke::default()
            .with_color(if import.selected {
                Color::from_rgb8(0xFD, 0x7E, 0x14)
            } else {
                Color::from_rgb8(0x55, 0x55, 0x55)
            })
            .with_width(if import.selected { 2.5 } else { 1.0 });

        for polyline in &import.polylines {
            if polyline.len() < 2 {
                continue;
            }
            let path = canvas::Path::new(|builder| {
                for (index, point) in polyline.iter().enumerate() {
                    let normalized_x = point.x - scene.bounds.min.x;
                    let normalized_y = scene.bounds.max.y - point.y;
                    let transformed = Point::new(
                        normalized_x * scale + offset.x,
                        normalized_y * scale + offset.y,
                    );
                    if index == 0 {
                        builder.move_to(transformed);
                    } else {
                        builder.line_to(transformed);
                    }
                }
            });
            frame.stroke(&path, stroke);
        }
    }

    for toolpath in &scene.toolpaths {
        let stroke = canvas::Stroke::default()
            .with_color(if toolpath.meta.highlighted {
                toolpath.meta.color
            } else {
                Color {
                    a: 0.7,
                    ..toolpath.meta.color
                }
            })
            .with_width(if toolpath.meta.highlighted { 2.5 } else { 1.6 });

        for segment in &toolpath.segments {
            if segment.len() < 2 {
                continue;
            }
            let path = canvas::Path::new(|builder| {
                for (idx, point) in segment.iter().enumerate() {
                    let normalized_x = point.x - scene.bounds.min.x;
                    let normalized_y = scene.bounds.max.y - point.y;
                    let transformed = Point::new(
                        normalized_x * scale + offset.x,
                        normalized_y * scale + offset.y,
                    );
                    if idx == 0 {
                        builder.move_to(transformed);
                    } else {
                        builder.line_to(transformed);
                    }
                }
            });
            frame.stroke(&path, stroke);
        }
    }

    for polygon in &scene.debug_polygons {
        let stroke = canvas::Stroke::default()
            .with_color(Color {
                a: 0.5,
                ..polygon.color
            })
            .with_width(1.2);

        for segment in &polygon.segments {
            if segment.len() < 2 {
                continue;
            }
            let path = canvas::Path::new(|builder| {
                for (idx, point) in segment.iter().enumerate() {
                    let normalized_x = point.x - scene.bounds.min.x;
                    let normalized_y = scene.bounds.max.y - point.y;
                    let transformed = Point::new(
                        normalized_x * scale + offset.x,
                        normalized_y * scale + offset.y,
                    );
                    if idx == 0 {
                        builder.move_to(transformed);
                    } else {
                        builder.line_to(transformed);
                    }
                }
            });
            frame.stroke(&path, stroke);
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
