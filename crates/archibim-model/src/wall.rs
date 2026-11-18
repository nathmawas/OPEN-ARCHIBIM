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

/// What a layer does in the construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerRole {
    /// Carries loads.
    LoadBearing,
    /// Thermal or acoustic insulation.
    Insulation,
    /// Ventilated or still air gap.
    AirGap,
    /// Vapour control, rain screen or waterproofing.
    Membrane,
    /// Lining or cladding fixed to the structure (plasterboard, boarding).
    Lining,
    /// Surface finish (plaster, render, paint).
    Finish,
    /// Roof or wall covering (tiles, slates, zinc).
    Covering,
}

/// Members set within the thickness of a layer, at a regular spacing: the
/// studs of a timber frame wall in its insulation, the joists of a floor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Framing {
    /// Material of the members.
    pub material: Id,
    /// Width of a member, measured along the wall.
    pub width: f64,
    /// Distance between member centres.
    pub spacing: f64,
}

/// One layer of a build-up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    /// What the layer does.
    pub role: LayerRole,
    /// The material, which carries the junction priority.
    pub material: Id,
    /// Thickness.
    pub thickness: f64,
    /// Renovation status of this layer only.
    pub status: RenovationStatus,
    /// Members within the layer, if any.
    pub framing: Option<Framing>,
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
