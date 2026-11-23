//! Roofs, terrain meshes and stairs.

use crate::{Id, Layer, LevelRef, Point};

/// One edge of a roof outline: sloped (an eave) or vertical (a gable).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoofEdge {
    /// Slope of the roof plane that rises from this edge, in radians; `None`
    /// for a gable, where the roof stops on a vertical cut.
    pub slope: Option<f64>,
    /// How far the roof runs past the edge, measured in plan.
    pub overhang: f64,
}

/// Which plane of the build-up the eaves level gives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoofPlane {
    /// The top of the build-up.
    #[default]
    Top,
    /// The underside of the build-up.
    Bottom,
}

/// A roof over an outline in plan: each edge rises at its own slope from
/// the eaves level, or stops on a gable. One sloped edge makes a shed roof,
/// two opposite ones a gabled roof, all of them a hipped roof.
#[derive(Clone, Debug, PartialEq)]
pub struct Roof {
    /// Permanent identifier.
    pub id: Id,
    /// The story it is placed on.
    pub story: Id,
    /// The outline in plan, closed, at least three points: usually the
    /// line of the walls it rests on.
    pub outline: Vec<Point>,
    /// One per outline edge: edge `i` runs from point `i` to the next.
    pub edges: Vec<RoofEdge>,
    /// The build-up, from the top face down.
    pub layers: Vec<Layer>,
    /// Height of the reference plane above the outline.
    pub eaves: LevelRef,
    /// Which plane `eaves` gives.
    pub plane: RoofPlane,
}

impl Roof {
    /// Total thickness of the build-up, square to the roof plane.
    pub fn thickness(&self) -> f64 {
        self.layers.iter().map(|l| l.thickness).sum()
    }
}

/// A point of a terrain mesh, with its height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshPoint {
    /// Position in plan.
    pub at: Point,
    /// Height above the mesh's level.
    pub z: f64,
}

/// A terrain mesh: an outline and inner points, each at its own height, as
/// in a land survey, over a flat underside.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    /// Permanent identifier.
    pub id: Id,
    /// The story it is placed on.
    pub story: Id,
    /// The outline in plan, closed, at least three points.
    pub outline: Vec<MeshPoint>,
    /// Surveyed points inside the outline.
    pub inner: Vec<MeshPoint>,
    /// The level heights are given from.
    pub level: LevelRef,
    /// Height of the flat underside above `level`.
    pub bottom: f64,
    /// Material.
    pub material: Id,
}

/// A stair along a walking line drawn in plan, from one level to another.
/// Straight, turning and (later) curved stairs are the same element: only
/// the line changes.
#[derive(Clone, Debug, PartialEq)]
pub struct Stair {
    /// Permanent identifier.
    pub id: Id,
    /// The story it is placed on.
    pub story: Id,
    /// The walking line, from the foot to the head, at least two points.
    pub path: Vec<Point>,
    /// Width, centred on the walking line.
    pub width: f64,
    /// Level of the foot.
    pub bottom: LevelRef,
    /// Level of the head, the floor the stair arrives on.
    pub top: LevelRef,
    /// Number of risers; `None` to work it out from the height.
    pub risers: Option<u32>,
    /// Material.
    pub material: Id,
}
