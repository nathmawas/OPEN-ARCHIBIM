//! The building as data: stories, materials, walls, openings, slabs,
//! columns, beams, roofs, terrain meshes and stairs, and the guides drawn
//! against.
//!
//! This crate depends on no other crate of the workspace (ADR 0002). It does
//! not read files and does not draw. Lengths are in millimetres.

mod envelope;
mod guide;
mod opening;
mod project;
mod story;
mod structure;
mod wall;

pub use envelope::{Mesh, MeshPoint, Roof, RoofEdge, RoofPlane, Stair};
pub use guide::{Guide, GuideShape};
pub use opening::{Hand, NominalSize, Opening, OpeningKind, Operation, Part, Performance};
pub use project::{
    Id, JunctionMode, Material, MaterialFamily, ModelError, Project, STARTING_LIBRARY,
};
pub use story::{LevelKind, LevelRef, Story};
pub use structure::{Align, Anchor, Beam, Column, Slab};
pub use wall::{Framing, Layer, LayerRole, Point, RenovationStatus, Wall, WallReference};
