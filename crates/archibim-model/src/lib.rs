//! The building as data: stories, materials, walls, openings, slabs,
//! columns and beams.
//!
//! This crate depends on no other crate of the workspace (ADR 0002). It does
//! not read files and does not draw. Lengths are in millimetres.

mod opening;
mod project;
mod story;
mod structure;
mod wall;

pub use opening::{Hand, NominalSize, Opening, OpeningKind, Operation, Part, Performance};
pub use project::{Id, JunctionMode, Material, ModelError, Project};
pub use story::{LevelKind, LevelRef, Story};
pub use structure::{Beam, Column, Slab};
pub use wall::{Framing, Layer, LayerRole, Point, RenovationStatus, Wall};
