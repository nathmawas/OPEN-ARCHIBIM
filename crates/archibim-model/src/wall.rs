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

/// Which line of a wall its `start` and `end` points lie on. Left and
/// right are seen looking from `start` to `end`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WallReference {
    /// The left face, where the first layer is.
    #[default]
    LeftFace,
    /// The middle of the whole build-up.
    Axis,
    /// The right face, where the last layer is.
    RightFace,
    /// The left face of the load-bearing layers.
    LoadBearingLeft,
    /// The right face of the load-bearing layers.
    LoadBearingRight,
}

/// A wall drawn along its reference line, from `start` to `end`.
///
/// Layers are listed from the left face to the right face, looking from
/// `start` to `end`. When the build-up changes, the reference line stays
/// where it is and the faces move.
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
    /// Which line of the wall `start` and `end` lie on.
    pub reference: WallReference,
    /// How far that line lies to the right of `start` and `end`; zero
    /// puts it on them.
    pub offset: f64,
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

    /// How far the left face lies to the left of `start` and `end`.
    /// Without load-bearing layers, a load-bearing reference falls back on
    /// the face on its side.
    pub fn left_face_shift(&self) -> f64 {
        self.reference_shift() - self.offset
    }

    /// How far the left face lies to the left of the reference line.
    fn reference_shift(&self) -> f64 {
        let bearing = |l: &&Layer| l.role == LayerRole::LoadBearing;
        let before = |i: usize| self.layers[..i].iter().map(|l| l.thickness).sum::<f64>();
        match self.reference {
            WallReference::LeftFace => 0.0,
            WallReference::Axis => self.thickness() / 2.0,
            WallReference::RightFace => self.thickness(),
            WallReference::LoadBearingLeft => self
                .layers
                .iter()
                .position(|l| bearing(&l))
                .map_or(0.0, before),
            WallReference::LoadBearingRight => self
                .layers
                .iter()
                .rposition(|l| bearing(&l))
                .map_or(self.thickness(), |i| before(i + 1)),
        }
    }

    /// The left face, from the start end to the end end, or nothing if the
    /// wall has no length.
    pub fn left_face(&self) -> Option<(Point, Point)> {
        let (dx, dy) = (self.end.x - self.start.x, self.end.y - self.start.y);
        let length = (dx * dx + dy * dy).sqrt();
        if length < 1e-9 {
            return None;
        }
        let k = self.left_face_shift() / length;
        // The left of (dx, dy) is (-dy, dx).
        let shift = |p: Point| Point::new(p.x - dy * k, p.y + dx * k);
        Some((shift(self.start), shift(self.end)))
    }
}
