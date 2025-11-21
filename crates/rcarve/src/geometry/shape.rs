use crate::geometry::ids::{CurveId, ShapeId};
use serde::{Deserialize, Serialize};

/// A shape in the project, which can be a single curve, multiple curves, or a region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shape {
    /// Unique identifier for this shape.
    pub id: ShapeId,
    /// Human-readable label for the shape.
    pub label: String,
    /// The kind of shape (what it contains).
    pub kind: ShapeKind,
    /// Optional origin offset for the shape.
    pub origin: Option<(f64, f64, f64)>,
    /// Source of the shape (where it came from).
    pub source: ShapeSource,
}

/// The kind of shape, indicating what geometry it contains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeKind {
    /// A single curve.
    Curve(CurveId),
    /// Multiple curves (e.g., from an SVG group).
    Curves(Vec<CurveId>),
    /// A region (outer boundary with optional holes).
    Region(crate::geometry::ids::RegionId),
}

/// Source information for a shape, indicating where it came from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeSource {
    /// Manually created by the user.
    Manual,
    /// Imported from an SVG file.
    SvgImport {
        /// Path to the SVG file.
        path: String,
        /// Optional layer name from the SVG.
        layer_name: Option<String>,
    },
    /// Created from a font (future feature).
    Font {
        /// Font name.
        font_name: String,
        /// Text content.
        text: String,
    },
    /// Created from a primitive (future feature).
    Primitive {
        /// Type of primitive.
        primitive_type: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_creation() {
        let curve_id = CurveId::new();
        let shape = Shape {
            id: ShapeId::new(),
            label: "Test Shape".to_string(),
            kind: ShapeKind::Curve(curve_id),
            origin: None,
            source: ShapeSource::Manual,
        };
        assert_eq!(shape.label, "Test Shape");
    }

    #[test]
    fn test_shape_serialization() {
        let curve_id = CurveId::new();
        let shape = Shape {
            id: ShapeId::new(),
            label: "Test".to_string(),
            kind: ShapeKind::Curve(curve_id),
            origin: Some((1.0, 2.0, 3.0)),
            source: ShapeSource::Manual,
        };
        let serialized = serde_json::to_string(&shape).expect("serialize");
        let deserialized: Shape = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(shape.label, deserialized.label);
    }

    #[test]
    fn test_svg_import_source() {
        let source = ShapeSource::SvgImport {
            path: "/path/to/file.svg".to_string(),
            layer_name: Some("Layer1".to_string()),
        };
        match source {
            ShapeSource::SvgImport { path, layer_name } => {
                assert_eq!(path, "/path/to/file.svg");
                assert_eq!(layer_name, Some("Layer1".to_string()));
            }
            _ => panic!("Expected SvgImport"),
        }
    }
}
