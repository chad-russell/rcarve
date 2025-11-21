use anyhow::{anyhow, Context, Result};
use kurbo::{BezPath, Circle, Line, Point};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path as StdPath;

pub mod curve;
pub mod ids;
pub mod offset;
pub mod region;
pub mod shape;

// Re-export public types
pub use curve::Curve;
pub use ids::{CurveId, RegionId, ShapeId};
pub use region::Region;
pub use shape::{Shape, ShapeKind, ShapeSource};

// Internal imports for use in this module
use curve::Curve as CurveType;
use ids::{CurveId as CurveIdType, RegionId as RegionIdType, ShapeId as ShapeIdType};
use region::Region as RegionType;
use shape::Shape as ShapeType;

/// Registry of all shapes, curves, and regions in a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeRegistry {
    /// All shapes in the project, indexed by their ID.
    pub shapes: HashMap<ShapeIdType, ShapeType>,
    /// All curves in the project, indexed by their ID.
    pub curves: HashMap<CurveIdType, CurveType>,
    /// All regions in the project, indexed by their ID.
    pub regions: HashMap<RegionIdType, RegionType>,
}

/// Result of importing shapes/curves into the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedBatch {
    pub shape_ids: Vec<ShapeId>,
    pub curve_ids: Vec<CurveId>,
    pub region_ids: Vec<RegionId>,
}

impl Default for ShapeRegistry {
    fn default() -> Self {
        Self {
            shapes: HashMap::new(),
            curves: HashMap::new(),
            regions: HashMap::new(),
        }
    }
}

impl ShapeRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a curve to the registry and return its ID.
    pub fn add_curve(&mut self, curve: CurveType) -> CurveId {
        let id = CurveId::new();
        self.curves.insert(id, curve);
        id
    }

    /// Get a curve by ID.
    pub fn get_curve(&self, id: &CurveId) -> Option<&CurveType> {
        self.curves.get(id)
    }

    /// Get a mutable reference to a curve by ID.
    pub fn get_curve_mut(&mut self, id: &CurveId) -> Option<&mut CurveType> {
        self.curves.get_mut(id)
    }

    /// Remove a curve from the registry.
    pub fn remove_curve(&mut self, id: &CurveId) -> Option<CurveType> {
        self.curves.remove(id)
    }

    /// Add a shape to the registry and return its ID.
    pub fn add_shape(&mut self, shape: ShapeType) -> ShapeId {
        let id = ShapeId::new();
        let mut shape = shape;
        shape.id = id;
        self.shapes.insert(id, shape.clone());
        id
    }

    /// Get a shape by ID.
    pub fn get_shape(&self, id: &ShapeId) -> Option<&ShapeType> {
        self.shapes.get(id)
    }

    /// Get a mutable reference to a shape by ID.
    pub fn get_shape_mut(&mut self, id: &ShapeId) -> Option<&mut ShapeType> {
        self.shapes.get_mut(id)
    }

    /// Remove a shape from the registry.
    pub fn remove_shape(&mut self, id: &ShapeId) -> Option<ShapeType> {
        self.shapes.remove(id)
    }

    /// Add a region to the registry and return its ID.
    pub fn add_region(&mut self, region: RegionType) -> RegionId {
        let id = RegionId::new();
        let mut region = region;
        region.id = id;
        self.regions.insert(id, region.clone());
        id
    }

    /// Get a region by ID.
    pub fn get_region(&self, id: &RegionId) -> Option<&RegionType> {
        self.regions.get(id)
    }

    /// Get a mutable reference to a region by ID.
    pub fn get_region_mut(&mut self, id: &RegionId) -> Option<&mut RegionType> {
        self.regions.get_mut(id)
    }

    /// Remove a region from the registry.
    pub fn remove_region(&mut self, id: &RegionId) -> Option<RegionType> {
        self.regions.remove(id)
    }

    /// Create a line curve from two points.
    pub fn create_line(&mut self, p0: (f64, f64), p1: (f64, f64)) -> CurveId {
        let line = Line::new(Point::new(p0.0, p0.1), Point::new(p1.0, p1.1));
        self.add_curve(Curve::Line(line))
    }

    /// Create a circle curve.
    pub fn create_circle(&mut self, center: (f64, f64), radius: f64) -> CurveId {
        let circle = Circle::new(Point::new(center.0, center.1), radius);
        self.add_curve(Curve::Circle(circle))
    }

    /// Create a Bézier path curve from a kurbo BezPath.
    pub fn create_bezpath(&mut self, path: BezPath) -> CurveId {
        self.add_curve(Curve::BezPath(path))
    }

    /// Import an SVG file and create shapes/curves from it.
    /// All SVG content (circles, paths, rects, etc.) is converted to high-fidelity
    /// Bezier curves that preserve smooth curves at any zoom level.
    pub fn from_svg<P: AsRef<StdPath>>(path: P) -> Result<Self> {
        let svg_path = path.as_ref();
        let data = fs::read(svg_path)
            .with_context(|| format!("Failed to read SVG {}", svg_path.display()))?;

        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_data(&data, &opt)
            .map_err(|err| anyhow!("Failed to parse SVG {}: {}", svg_path.display(), err))?;

        let mut registry = Self::new();
        let source = ShapeSource::SvgImport {
            path: svg_path.to_string_lossy().to_string(),
            layer_name: None,
        };

        let mut path_count = 0;
        registry.import_usvg_group(tree.root(), &source, &mut path_count);

        Ok(registry)
    }

    /// Import an SVG file into the current registry, returning the IDs created.
    pub fn import_svg<P: AsRef<StdPath>>(&mut self, path: P) -> Result<ImportedBatch> {
        let svg_path = path.as_ref();
        let data = fs::read(svg_path)
            .with_context(|| format!("Failed to read SVG {}", svg_path.display()))?;

        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_data(&data, &opt)
            .map_err(|err| anyhow!("Failed to parse SVG {}: {}", svg_path.display(), err))?;

        let source = ShapeSource::SvgImport {
            path: svg_path.to_string_lossy().to_string(),
            layer_name: None,
        };

        let initial_shape_count = self.shapes.len();
        let initial_curve_count = self.curves.len();

        let mut path_count = 0;
        self.import_usvg_group(tree.root(), &source, &mut path_count);

        let shape_ids = self
            .shapes
            .keys()
            .skip(initial_shape_count)
            .copied()
            .collect();
        let curve_ids = self
            .curves
            .keys()
            .skip(initial_curve_count)
            .copied()
            .collect();

        Ok(ImportedBatch {
            shape_ids,
            curve_ids,
            region_ids: Vec::new(),
        })
    }

    /// Recursively import nodes from a usvg Group, converting all paths to high-fidelity BezPath curves.
    fn import_usvg_group(
        &mut self,
        group: &usvg::Group,
        source: &ShapeSource,
        path_count: &mut usize,
    ) {
        for node in group.children() {
            match node {
                usvg::Node::Group(g) => {
                    self.import_usvg_group(&g, source, path_count);
                }
                usvg::Node::Path(path) => {
                    if !path.is_visible() {
                        continue;
                    }

                    // Convert tiny_skia_path to kurbo BezPath
                    let bezpath = convert_tiny_skia_to_kurbo(path.data());

                    if bezpath.elements().is_empty() {
                        continue;
                    }

                    *path_count += 1;
                    let label = if path.id().is_empty() {
                        format!("Path {}", path_count)
                    } else {
                        path.id().to_string()
                    };

                    let curve_id = self.create_bezpath(bezpath);
                    self.add_shape(Shape {
                        id: ShapeId::new(),
                        label,
                        kind: ShapeKind::Curve(curve_id),
                        origin: None,
                        source: source.clone(),
                    });
                }
                usvg::Node::Image(_) => {
                    // Images are not supported for toolpath generation
                }
                usvg::Node::Text(_) => {
                    // Text nodes are already converted to paths by usvg
                }
            }
        }
    }

    /// Get all curve IDs in the registry.
    pub fn all_curve_ids(&self) -> Vec<CurveId> {
        self.curves.keys().copied().collect()
    }

    /// Get all shape IDs in the registry.
    pub fn all_shape_ids(&self) -> Vec<ShapeId> {
        self.shapes.keys().copied().collect()
    }

    /// Get all region IDs in the registry.
    pub fn all_region_ids(&self) -> Vec<RegionId> {
        self.regions.keys().copied().collect()
    }
}

/// Convert a tiny_skia_path to kurbo BezPath, preserving all curve information.
/// This maintains high fidelity - curves stay as curves, not flattened line segments.
fn convert_tiny_skia_to_kurbo(path: &tiny_skia_path::Path) -> BezPath {
    let mut bezpath = BezPath::new();

    for segment in path.segments() {
        match segment {
            tiny_skia_path::PathSegment::MoveTo(p) => {
                bezpath.move_to(Point::new(p.x as f64, p.y as f64));
            }
            tiny_skia_path::PathSegment::LineTo(p) => {
                bezpath.line_to(Point::new(p.x as f64, p.y as f64));
            }
            tiny_skia_path::PathSegment::QuadTo(p1, p2) => {
                bezpath.quad_to(
                    Point::new(p1.x as f64, p1.y as f64),
                    Point::new(p2.x as f64, p2.y as f64),
                );
            }
            tiny_skia_path::PathSegment::CubicTo(p1, p2, p3) => {
                bezpath.curve_to(
                    Point::new(p1.x as f64, p1.y as f64),
                    Point::new(p2.x as f64, p2.y as f64),
                    Point::new(p3.x as f64, p3.y as f64),
                );
            }
            tiny_skia_path::PathSegment::Close => {
                bezpath.close_path();
            }
        }
    }

    bezpath
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ShapeRegistry::new();
        assert_eq!(registry.shapes.len(), 0);
        assert_eq!(registry.curves.len(), 0);
        assert_eq!(registry.regions.len(), 0);
    }

    #[test]
    fn test_add_get_curve() {
        let mut registry = ShapeRegistry::new();
        let line = Line::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let curve_id = registry.add_curve(Curve::Line(line));

        let retrieved = registry.get_curve(&curve_id);
        assert!(retrieved.is_some());
        match retrieved.unwrap() {
            Curve::Line(l) => {
                assert_eq!(l.p0, Point::new(0.0, 0.0));
                assert_eq!(l.p1, Point::new(10.0, 10.0));
            }
            _ => panic!("Expected Line curve"),
        }
    }

    #[test]
    fn test_add_get_shape() {
        let mut registry = ShapeRegistry::new();
        let line_id = registry.create_line((0.0, 0.0), (10.0, 10.0));
        let shape = Shape {
            id: ShapeId::new(),
            label: "Test Line".to_string(),
            kind: ShapeKind::Curve(line_id),
            origin: None,
            source: ShapeSource::Manual,
        };
        let shape_id = registry.add_shape(shape);

        let retrieved = registry.get_shape(&shape_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().label, "Test Line");
    }

    #[test]
    fn test_add_get_region() {
        let mut registry = ShapeRegistry::new();
        let outer_id = registry.create_circle((0.0, 0.0), 10.0);
        let inner_id = registry.create_circle((0.0, 0.0), 5.0);
        let region = Region {
            id: RegionId::new(),
            outer: outer_id,
            holes: vec![inner_id],
        };
        let region_id = registry.add_region(region);

        let retrieved = registry.get_region(&region_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().holes.len(), 1);
    }

    #[test]
    fn test_create_line() {
        let mut registry = ShapeRegistry::new();
        let curve_id = registry.create_line((0.0, 0.0), (10.0, 10.0));
        let curve = registry.get_curve(&curve_id).unwrap();
        match curve {
            Curve::Line(l) => {
                assert_eq!(l.p0, Point::new(0.0, 0.0));
                assert_eq!(l.p1, Point::new(10.0, 10.0));
            }
            _ => panic!("Expected Line"),
        }
    }

    #[test]
    fn test_create_circle() {
        let mut registry = ShapeRegistry::new();
        let curve_id = registry.create_circle((5.0, 5.0), 10.0);
        let curve = registry.get_curve(&curve_id).unwrap();
        match curve {
            Curve::Circle(c) => {
                assert_eq!(c.center, Point::new(5.0, 5.0));
                assert_eq!(c.radius, 10.0);
            }
            _ => panic!("Expected Circle"),
        }
    }

    #[test]
    fn test_import_svg_simple() {
        let mut registry = ShapeRegistry::new();
        let svg_path =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/simple.svg");
        let batch = registry.import_svg(&svg_path).expect("import svg");
        assert!(!batch.curve_ids.is_empty());
        assert_eq!(registry.curves.len(), batch.curve_ids.len());
        assert_eq!(registry.shapes.len(), batch.shape_ids.len());
    }

    #[test]
    fn test_import_circle_as_bezier() {
        // Circles are converted to high-fidelity Bezier curves (4 cubic segments)
        let svg_path =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/circle.svg");
        let registry = ShapeRegistry::from_svg(&svg_path).expect("import circle");

        assert_eq!(registry.curves.len(), 1, "Should import one curve");

        // Verify it's a BezPath (not flattened to many line segments)
        let curve = registry.curves.values().next().unwrap();
        match curve {
            Curve::BezPath(path) => {
                // A circle converted by usvg has exactly 5 elements:
                // MoveTo + 4 CubicTo (one per quadrant)
                let element_count = path.elements().len();
                assert!(
                    element_count <= 10,
                    "Circle should be compact Bezier curve, got {} elements",
                    element_count
                );
            }
            _ => panic!("Expected BezPath for imported circle"),
        }
    }

    #[test]
    fn test_serialization_round_trip() {
        let mut registry = ShapeRegistry::new();
        let line_id = registry.create_line((0.0, 0.0), (10.0, 10.0));
        let shape = Shape {
            id: ShapeId::new(),
            label: "Test".to_string(),
            kind: ShapeKind::Curve(line_id),
            origin: None,
            source: ShapeSource::Manual,
        };
        registry.add_shape(shape);

        let serialized = serde_json::to_string(&registry).expect("serialize");
        let deserialized: ShapeRegistry = serde_json::from_str(&serialized).expect("deserialize");

        assert_eq!(registry.shapes.len(), deserialized.shapes.len());
        assert_eq!(registry.curves.len(), deserialized.curves.len());
    }
}
