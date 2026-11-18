use crate::Id;

/// A story. Its level is the top of the finished floor.
#[derive(Clone, Debug, PartialEq)]
pub struct Story {
    /// Permanent identifier.
    pub id: Id,
    /// Name shown to the user.
    pub name: String,
    /// Top of the finished floor.
    pub finished_level: f64,
    /// Depth of the floor build-up above the structure (screed, insulation,
    /// finish).
    pub floor_build_up: f64,
}

impl Story {
    /// Top of the structural slab, below the floor build-up.
    pub fn structural_level(&self) -> f64 {
        self.finished_level - self.floor_build_up
    }
}

/// Which level of a story a height refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelKind {
    /// Top of the finished floor.
    Finished,
    /// Top of the structural slab.
    Structural,
}

/// A height linked to a level of a story, plus an offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LevelRef {
    /// The story the height refers to.
    pub story: Id,
    /// Which of its levels.
    pub kind: LevelKind,
    /// Offset above that level.
    pub offset: f64,
}
