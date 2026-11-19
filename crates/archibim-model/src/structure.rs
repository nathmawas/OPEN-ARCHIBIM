use crate::{Id, Layer, LevelRef, Point};

/// A floor or a roof slab: an outline in plan, a build-up, a top level.
#[derive(Clone, Debug, PartialEq)]
pub struct Slab {
    /// Permanent identifier.
    pub id: Id,
    /// The story it is placed on.
    pub story: Id,
    /// The outline in plan, closed, at least three points.
    pub outline: Vec<Point>,
    /// The build-up, from the top face down.
    pub layers: Vec<Layer>,
    /// Level of the top face.
    pub top: LevelRef,
}

impl Slab {
    /// Total thickness of the build-up.
    pub fn thickness(&self) -> f64 {
        self.layers.iter().map(|l| l.thickness).sum()
    }
}

/// A column, rectangular in plan.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    /// Permanent identifier.
    pub id: Id,
    /// The story it is placed on.
    pub story: Id,
    /// Centre in plan.
    pub at: Point,
    /// Size along x.
    pub width: f64,
    /// Size along y.
    pub depth: f64,
    /// Material.
    pub material: Id,
    /// Level of the foot.
    pub bottom: LevelRef,
    /// Level of the head.
    pub top: LevelRef,
}

/// A beam between two points, rectangular in section.
#[derive(Clone, Debug, PartialEq)]
pub struct Beam {
    /// Permanent identifier.
    pub id: Id,
    /// The story it is placed on.
    pub story: Id,
    /// Start of the axis in plan.
    pub start: Point,
    /// End of the axis in plan.
    pub end: Point,
    /// Width, centred on the axis.
    pub width: f64,
    /// Height, below the top level.
    pub height: f64,
    /// Material.
    pub material: Id,
    /// Level of the top face.
    pub top: LevelRef,
}
