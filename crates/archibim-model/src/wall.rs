use crate::{Id, LevelRef};

/// A point in plan.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
}

impl Point {
    /// A point from its coordinates.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Renovation status of a layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenovationStatus {
    /// Already built, kept.
    Existing,
    /// Already built, to be removed.
    ToDemolish,
    /// Built by the project.
    New,
}

/// One layer of a wall build-up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    /// The material, which carries the junction priority.
    pub material: Id,
    /// Thickness.
    pub thickness: f64,
    /// Renovation status of this layer only.
    pub status: RenovationStatus,
}

/// A wall drawn along its reference line, from `start` to `end`.
///
/// The reference line is the left face, looking from `start` to `end`.
/// Layers are listed from the left face to the right face.
#[derive(Clone, Debug, PartialEq)]
pub struct Wall {
    /// Permanent identifier.
    pub id: Id,
    /// The story the wall is placed on.
    pub story: Id,
    /// Start of the reference line.
    pub start: Point,
    /// End of the reference line.
    pub end: Point,
    /// The build-up, from the left face to the right face.
    pub layers: Vec<Layer>,
    /// Bottom of the wall.
    pub bottom: LevelRef,
    /// Top of the wall.
    pub top: LevelRef,
}

impl Wall {
    /// Total thickness of the build-up.
    pub fn thickness(&self) -> f64 {
        self.layers.iter().map(|l| l.thickness).sum()
    }
}
