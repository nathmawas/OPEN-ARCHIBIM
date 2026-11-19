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

/// Where the placed point sits across one size of an element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// On the first edge.
    First,
    /// In the middle.
    #[default]
    Middle,
    /// On the last edge.
    Last,
}

impl Align {
    /// Where the first edge lies from the placed point, for this size.
    pub fn first_edge(self, size: f64) -> f64 {
        match self {
            Self::First => 0.0,
            Self::Middle => -size / 2.0,
            Self::Last => -size,
        }
    }
}

/// The point of a rectangle that the user places, one choice per size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Anchor {
    /// Across the width.
    pub width: Align,
    /// Across the depth, or the height for a beam.
    pub depth: Align,
}

/// A column, rectangular in plan.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    /// Permanent identifier.
    pub id: Id,
    /// The story it is placed on.
    pub story: Id,
    /// The placed point in plan.
    pub at: Point,
    /// Which point of the section `at` is: across the width, first is the
    /// left edge; across the depth, first is the front edge.
    pub anchor: Anchor,
    /// Turn around `at`, anticlockwise, in radians. At zero the width runs
    /// along x.
    pub angle: f64,
    /// Size along the width.
    pub width: f64,
    /// Size along the depth.
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
    /// Start of the reference line in plan.
    pub start: Point,
    /// End of the reference line in plan.
    pub end: Point,
    /// Which point of the section the reference line runs through: across
    /// the width, first is the left face looking from `start` to `end`;
    /// across the height, first is the top face.
    pub anchor: Anchor,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
    /// Material.
    pub material: Id,
    /// Level of the reference line.
    pub level: LevelRef,
}
