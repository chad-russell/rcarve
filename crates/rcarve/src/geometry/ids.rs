use serde::{Deserialize, Serialize};
use std::fmt;
use ulid::Ulid;

/// Unique identifier for a shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShapeId(Ulid);

/// Unique identifier for a curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CurveId(Ulid);

/// Unique identifier for a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RegionId(Ulid);

impl ShapeId {
    /// Create a new ShapeId with a random ULID.
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Create a ShapeId from a ULID.
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Get the underlying ULID.
    pub fn ulid(&self) -> Ulid {
        self.0
    }
}

impl Default for ShapeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ShapeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl CurveId {
    /// Create a new CurveId with a random ULID.
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Create a CurveId from a ULID.
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Get the underlying ULID.
    pub fn ulid(&self) -> Ulid {
        self.0
    }
}

impl Default for CurveId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CurveId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl RegionId {
    /// Create a new RegionId with a random ULID.
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Create a RegionId from a ULID.
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Get the underlying ULID.
    pub fn ulid(&self) -> Ulid {
        self.0
    }
}

impl Default for RegionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RegionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_id_creation() {
        let id1 = ShapeId::new();
        let id2 = ShapeId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_curve_id_creation() {
        let id1 = CurveId::new();
        let id2 = CurveId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_region_id_creation() {
        let id1 = RegionId::new();
        let id2 = RegionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_id_serialization() {
        let id = ShapeId::new();
        let serialized = serde_json::to_string(&id).expect("serialize");
        let deserialized: ShapeId = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(id, deserialized);
    }
}
