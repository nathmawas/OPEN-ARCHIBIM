use std::fmt;

use crate::{LevelKind, LevelRef, Opening, Story, Wall};

/// Permanent identifier of anything in the project.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(pub u64);

/// An error in a model operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelError {
    /// Nothing with this identifier.
    NotFound(Id),
    /// The operation would break a rule of the model.
    Invalid(&'static str),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "nothing with identifier {}", id.0),
            Self::Invalid(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for ModelError {}

/// A building material.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    /// Permanent identifier.
    pub id: Id,
    /// Name shown to the user.
    pub name: String,
    /// Junction priority: at a junction, a layer passes through the layers of
    /// lower priority.
    pub priority: u8,
}

/// How a corner junction between two walls is resolved, when the user
/// overrides the default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JunctionMode {
    /// The layers meet on the bisector of the angle.
    Mitre,
    /// This wall passes in front; the other stops against it.
    InFront(Id),
}

/// The whole building.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Project {
    next_id: u64,
    /// Stories.
    pub stories: Vec<Story>,
    /// Building materials.
    pub materials: Vec<Material>,
    /// Walls.
    pub walls: Vec<Wall>,
    /// Openings.
    pub openings: Vec<Opening>,
    junctions: Vec<(Id, Id, JunctionMode)>,
}

// ponytail: linear scans over Vecs; switch to maps keyed by Id when projects
// get large enough for the benchmarks to show it.
macro_rules! find {
    ($get:ident, $get_mut:ident, $field:ident, $ty:ty) => {
        /// The element with this identifier.
        pub fn $get(&self, id: Id) -> Result<&$ty, ModelError> {
            self.$field
                .iter()
                .find(|e| e.id == id)
                .ok_or(ModelError::NotFound(id))
        }

        /// The element with this identifier, for editing.
        pub fn $get_mut(&mut self, id: Id) -> Result<&mut $ty, ModelError> {
            self.$field
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or(ModelError::NotFound(id))
        }
    };
}

impl Project {
    /// An empty project.
    pub fn new() -> Self {
        Self::default()
    }

    /// A new identifier, never used before in this project.
    pub fn new_id(&mut self) -> Id {
        self.next_id += 1;
        Id(self.next_id)
    }

    find!(story, story_mut, stories, Story);
    find!(material, material_mut, materials, Material);
    find!(wall, wall_mut, walls, Wall);
    find!(opening, opening_mut, openings, Opening);

    fn check_free(&mut self, id: Id) -> Result<(), ModelError> {
        let used = self.stories.iter().any(|e| e.id == id)
            || self.materials.iter().any(|e| e.id == id)
            || self.walls.iter().any(|e| e.id == id)
            || self.openings.iter().any(|e| e.id == id);
        if used {
            return Err(ModelError::Invalid("identifier already used"));
        }
        self.next_id = self.next_id.max(id.0);
        Ok(())
    }

    /// Adds a story, keeping stories sorted by level.
    pub fn insert_story(&mut self, story: Story) -> Result<(), ModelError> {
        self.check_free(story.id)?;
        let at = self
            .stories
            .partition_point(|s| s.finished_level <= story.finished_level);
        self.stories.insert(at, story);
        Ok(())
    }

    /// Adds a material.
    pub fn insert_material(&mut self, material: Material) -> Result<(), ModelError> {
        self.check_free(material.id)?;
        self.materials.push(material);
        Ok(())
    }

    /// Adds a wall. Its story and materials must exist.
    pub fn insert_wall(&mut self, wall: Wall) -> Result<(), ModelError> {
        self.check_free(wall.id)?;
        self.story(wall.story)?;
        for layer in &wall.layers {
            self.material(layer.material)?;
        }
        if wall.layers.is_empty() || wall.layers.iter().any(|l| l.thickness <= 0.0) {
            return Err(ModelError::Invalid("a wall needs layers with a thickness"));
        }
        self.walls.push(wall);
        Ok(())
    }

    /// Adds an opening. Its wall must exist.
    pub fn insert_opening(&mut self, opening: Opening) -> Result<(), ModelError> {
        self.check_free(opening.id)?;
        self.wall(opening.wall)?;
        self.openings.push(opening);
        Ok(())
    }

    /// Removes a story that nothing is placed on.
    pub fn remove_story(&mut self, id: Id) -> Result<Story, ModelError> {
        let used = self
            .walls
            .iter()
            .any(|w| w.story == id || w.bottom.story == id || w.top.story == id);
        if used {
            return Err(ModelError::Invalid("walls still refer to this story"));
        }
        let at = self
            .stories
            .iter()
            .position(|s| s.id == id)
            .ok_or(ModelError::NotFound(id))?;
        Ok(self.stories.remove(at))
    }

    /// Removes a material that no wall uses.
    pub fn remove_material(&mut self, id: Id) -> Result<Material, ModelError> {
        if self
            .walls
            .iter()
            .flat_map(|w| &w.layers)
            .any(|l| l.material == id)
        {
            return Err(ModelError::Invalid("walls still use this material"));
        }
        let at = self
            .materials
            .iter()
            .position(|m| m.id == id)
            .ok_or(ModelError::NotFound(id))?;
        Ok(self.materials.remove(at))
    }

    /// Removes a wall with no openings.
    pub fn remove_wall(&mut self, id: Id) -> Result<Wall, ModelError> {
        if self.openings.iter().any(|o| o.wall == id) {
            return Err(ModelError::Invalid("the wall still hosts openings"));
        }
        let at = self
            .walls
            .iter()
            .position(|w| w.id == id)
            .ok_or(ModelError::NotFound(id))?;
        self.junctions.retain(|(a, b, _)| *a != id && *b != id);
        Ok(self.walls.remove(at))
    }

    /// Removes an opening.
    pub fn remove_opening(&mut self, id: Id) -> Result<Opening, ModelError> {
        let at = self
            .openings
            .iter()
            .position(|o| o.id == id)
            .ok_or(ModelError::NotFound(id))?;
        Ok(self.openings.remove(at))
    }

    /// The absolute height of a level reference.
    pub fn level(&self, at: &LevelRef) -> Result<f64, ModelError> {
        let story = self.story(at.story)?;
        let level = match at.kind {
            LevelKind::Finished => story.finished_level,
            LevelKind::Structural => story.structural_level(),
        };
        Ok(level + at.offset)
    }

    /// Absolute bottom and top of a wall.
    pub fn wall_extent(&self, id: Id) -> Result<(f64, f64), ModelError> {
        let wall = self.wall(id)?;
        Ok((self.level(&wall.bottom)?, self.level(&wall.top)?))
    }

    /// Absolute sill level of an opening.
    pub fn sill_level(&self, id: Id) -> Result<f64, ModelError> {
        let opening = self.opening(id)?;
        let wall = self.wall(opening.wall)?;
        Ok(self.story(wall.story)?.finished_level + opening.sill)
    }

    /// The junction mode the user chose between two walls, if any.
    pub fn junction_mode(&self, a: Id, b: Id) -> Option<JunctionMode> {
        self.junctions
            .iter()
            .find(|(x, y, _)| (*x, *y) == (a, b) || (*x, *y) == (b, a))
            .map(|(_, _, mode)| *mode)
    }

    /// Sets or clears the junction mode between two walls. Returns the
    /// previous one.
    pub fn set_junction_mode(
        &mut self,
        a: Id,
        b: Id,
        mode: Option<JunctionMode>,
    ) -> Result<Option<JunctionMode>, ModelError> {
        self.wall(a)?;
        self.wall(b)?;
        if let Some(JunctionMode::InFront(front)) = mode
            && front != a
            && front != b
        {
            return Err(ModelError::Invalid(
                "the wall in front must be one of the two walls",
            ));
        }
        let previous = self.junction_mode(a, b);
        self.junctions
            .retain(|(x, y, _)| !((*x, *y) == (a, b) || (*x, *y) == (b, a)));
        if let Some(mode) = mode {
            self.junctions.push((a, b, mode));
        }
        Ok(previous)
    }
}
