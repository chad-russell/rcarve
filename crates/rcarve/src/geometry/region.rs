use crate::geometry::ids::{CurveId, RegionId};
use serde::{Deserialize, Serialize};

/// A region represents a filled area with an outer boundary and optional holes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    /// Unique identifier for this region.
    pub id: RegionId,
    /// The outer boundary curve (must be closed).
    pub outer: CurveId,
    /// Optional inner curves (holes) that should be excluded from the region.
    pub holes: Vec<CurveId>,
}

impl Region {
    /// Create a new region with just an outer boundary.
    pub fn new(outer: CurveId) -> Self {
        Self {
            id: RegionId::new(),
            outer,
            holes: Vec::new(),
        }
    }

    /// Create a new region with an outer boundary and holes.
    pub fn with_holes(outer: CurveId, holes: Vec<CurveId>) -> Self {
        Self {
            id: RegionId::new(),
            outer,
            holes,
        }
    }

    /// Add a hole to the region.
    pub fn add_hole(&mut self, hole: CurveId) {
        self.holes.push(hole);
    }

    /// Remove a hole from the region.
    pub fn remove_hole(&mut self, hole: &CurveId) -> bool {
        if let Some(pos) = self.holes.iter().position(|h| h == hole) {
            self.holes.remove(pos);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_creation() {
        let outer_id = CurveId::new();
        let region = Region::new(outer_id);
        assert_eq!(region.outer, outer_id);
        assert_eq!(region.holes.len(), 0);
    }

    #[test]
    fn test_region_with_holes() {
        let outer_id = CurveId::new();
        let hole1 = CurveId::new();
        let hole2 = CurveId::new();
        let region = Region::with_holes(outer_id, vec![hole1, hole2]);
        assert_eq!(region.outer, outer_id);
        assert_eq!(region.holes.len(), 2);
    }

    #[test]
    fn test_region_add_hole() {
        let outer_id = CurveId::new();
        let mut region = Region::new(outer_id);
        let hole = CurveId::new();
        region.add_hole(hole);
        assert_eq!(region.holes.len(), 1);
    }

    #[test]
    fn test_region_remove_hole() {
        let outer_id = CurveId::new();
        let hole = CurveId::new();
        let mut region = Region::with_holes(outer_id, vec![hole]);
        assert_eq!(region.holes.len(), 1);
        let removed = region.remove_hole(&hole);
        assert!(removed);
        assert_eq!(region.holes.len(), 0);
    }

    #[test]
    fn test_region_serialization() {
        let outer_id = CurveId::new();
        let hole = CurveId::new();
        let region = Region::with_holes(outer_id, vec![hole]);
        let serialized = serde_json::to_string(&region).expect("serialize");
        let deserialized: Region = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(region.holes.len(), deserialized.holes.len());
    }
}
