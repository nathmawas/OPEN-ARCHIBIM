//! Every change to the model is a command: it can be undone, redone,
//! scripted and called through MCP. Applying a command returns its inverse
//! and the elements it changed.

use archibim_model::{
    Beam, Column, Id, JunctionMode, Layer, Material, ModelError, Opening, OpeningKind, Operation,
    Part, Project, Slab, Story, Wall,
};

/// A change to the model.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Adds a story.
    AddStory(Story),
    /// Removes a story that nothing is placed on.
    RemoveStory(Id),
    /// Sets the finished level and the floor build-up of a story.
    SetStoryLevels {
        /// The story.
        story: Id,
        /// Top of the finished floor.
        finished_level: f64,
        /// Depth of the floor build-up.
        floor_build_up: f64,
    },
    /// Adds a material.
    AddMaterial(Material),
    /// Removes a material that no wall uses.
    RemoveMaterial(Id),
    /// Sets the junction priority of a material.
    SetMaterialPriority {
        /// The material.
        material: Id,
        /// The new priority.
        priority: u8,
    },
    /// Adds a wall.
    AddWall(Wall),
    /// Removes a wall with no openings.
    RemoveWall(Id),
    /// Replaces the build-up of a wall, in place: the wall keeps its
    /// identity, its openings and its junctions.
    SetWallLayers {
        /// The wall.
        wall: Id,
        /// The new build-up, from the left face to the right face.
        layers: Vec<Layer>,
    },
    /// Sets or clears how the junction between two walls is resolved.
    SetJunctionMode {
        /// One wall.
        a: Id,
        /// The other wall.
        b: Id,
        /// The mode, or `None` for the default.
        mode: Option<JunctionMode>,
    },
    /// Adds an opening.
    AddOpening(Opening),
    /// Removes an opening.
    RemoveOpening(Id),
    /// Adds a part to an opening, on the left or on the right.
    AddOpeningPart {
        /// The opening.
        opening: Id,
        /// The new part.
        part: Part,
        /// On the left, or on the right.
        at_left: bool,
    },
    /// Changes how one part of an opening opens.
    SetPartOperation {
        /// The opening.
        opening: Id,
        /// Index of the part, from the left.
        part: usize,
        /// How it opens.
        operation: Operation,
    },
    /// Makes an opening interior or exterior, keeping its sizes and parts.
    SetOpeningKind {
        /// The opening.
        opening: Id,
        /// Interior or exterior.
        kind: OpeningKind,
    },
    /// Replaces an opening with a new state of itself, same identifier.
    ReplaceOpening(Opening),
    /// Replaces a wall with a new state of itself, same identifier: its
    /// openings and junctions are kept.
    ReplaceWall(Wall),
    /// Adds a slab.
    AddSlab(Slab),
    /// Removes a slab.
    RemoveSlab(Id),
    /// Replaces a slab with a new state of itself, same identifier.
    ReplaceSlab(Slab),
    /// Adds a column.
    AddColumn(Column),
    /// Removes a column.
    RemoveColumn(Id),
    /// Replaces a column with a new state of itself, same identifier.
    ReplaceColumn(Column),
    /// Adds a beam.
    AddBeam(Beam),
    /// Removes a beam.
    RemoveBeam(Id),
    /// Replaces a beam with a new state of itself, same identifier.
    ReplaceBeam(Beam),
    /// Several commands applied in order, undone in one step. If one is
    /// refused, the ones before it are undone and nothing changes.
    Group(Vec<Command>),
}

/// The result of applying a command.
#[derive(Clone, Debug, PartialEq)]
pub struct Applied {
    /// The command that undoes this one.
    pub inverse: Command,
    /// The elements whose shape or data changed.
    pub changed: Vec<Id>,
}

/// Applies a command to the project. On error, the project is unchanged.
pub fn apply(project: &mut Project, command: Command) -> Result<Applied, ModelError> {
    use Command::*;
    let applied = |inverse, changed| Ok(Applied { inverse, changed });
    match command {
        AddStory(story) => {
            let id = story.id;
            project.insert_story(story)?;
            applied(RemoveStory(id), vec![id])
        }
        RemoveStory(id) => {
            let story = project.remove_story(id)?;
            applied(AddStory(story), vec![id])
        }
        SetStoryLevels {
            story,
            finished_level,
            floor_build_up,
        } => {
            let s = project.story_mut(story)?;
            let inverse = SetStoryLevels {
                story,
                finished_level: s.finished_level,
                floor_build_up: s.floor_build_up,
            };
            s.finished_level = finished_level;
            s.floor_build_up = floor_build_up;
            project
                .stories
                .sort_by(|a, b| a.finished_level.total_cmp(&b.finished_level));
            let mut changed = vec![story];
            changed.extend(
                project
                    .walls
                    .iter()
                    .filter(|w| [w.story, w.bottom.story, w.top.story].contains(&story))
                    .map(|w| w.id),
            );
            applied(inverse, changed)
        }
        AddMaterial(material) => {
            let id = material.id;
            project.insert_material(material)?;
            applied(RemoveMaterial(id), vec![id])
        }
        RemoveMaterial(id) => {
            let material = project.remove_material(id)?;
            applied(AddMaterial(material), vec![id])
        }
        SetMaterialPriority { material, priority } => {
            let m = project.material_mut(material)?;
            let inverse = SetMaterialPriority {
                material,
                priority: m.priority,
            };
            m.priority = priority;
            let mut changed = vec![material];
            changed.extend(
                project
                    .walls
                    .iter()
                    .filter(|w| w.layers.iter().any(|l| l.material == material))
                    .map(|w| w.id),
            );
            applied(inverse, changed)
        }
        AddWall(wall) => {
            let id = wall.id;
            project.insert_wall(wall)?;
            applied(RemoveWall(id), vec![id])
        }
        RemoveWall(id) => {
            let mode_holders: Vec<(Id, JunctionMode)> = project
                .walls
                .iter()
                .filter_map(|w| project.junction_mode(id, w.id).map(|m| (w.id, m)))
                .collect();
            if !mode_holders.is_empty() {
                // ponytail: undo would lose the junction modes; refuse until a
                // compound command exists.
                return Err(ModelError::Invalid(
                    "clear the junction modes of this wall first",
                ));
            }
            let wall = project.remove_wall(id)?;
            applied(AddWall(wall), vec![id])
        }
        SetWallLayers { wall, layers } => {
            project.check_layers(&layers)?;
            let w = project.wall_mut(wall)?;
            let old = std::mem::replace(&mut w.layers, layers);
            applied(SetWallLayers { wall, layers: old }, vec![wall])
        }
        SetJunctionMode { a, b, mode } => {
            let previous = project.set_junction_mode(a, b, mode)?;
            applied(
                SetJunctionMode {
                    a,
                    b,
                    mode: previous,
                },
                vec![a, b],
            )
        }
        AddOpening(opening) => {
            let (id, wall) = (opening.id, opening.wall);
            project.insert_opening(opening)?;
            applied(RemoveOpening(id), vec![id, wall])
        }
        RemoveOpening(id) => {
            let opening = project.remove_opening(id)?;
            let wall = opening.wall;
            applied(AddOpening(opening), vec![id, wall])
        }
        AddOpeningPart {
            opening,
            part,
            at_left,
        } => edit_opening(project, opening, |o| o.add_part(part, at_left).map(|_| ())),
        SetPartOperation {
            opening,
            part,
            operation,
        } => edit_opening(project, opening, |o| {
            let p = o
                .parts
                .get_mut(part)
                .ok_or(ModelError::Invalid("no such part"))?;
            p.operation = operation;
            Ok(())
        }),
        SetOpeningKind { opening, kind } => edit_opening(project, opening, |o| {
            o.kind = kind;
            Ok(())
        }),
        ReplaceOpening(new) => {
            let id = new.id;
            let o = project.opening_mut(id)?;
            let old = std::mem::replace(o, new);
            let wall = old.wall;
            applied(ReplaceOpening(old), vec![id, wall])
        }
        ReplaceWall(new) => {
            let id = new.id;
            let old = project.replace_wall(new)?;
            applied(ReplaceWall(old), vec![id])
        }
        AddSlab(slab) => {
            let id = slab.id;
            project.insert_slab(slab)?;
            applied(RemoveSlab(id), vec![id])
        }
        RemoveSlab(id) => applied(AddSlab(project.remove_slab(id)?), vec![id]),
        ReplaceSlab(new) => {
            let id = new.id;
            applied(ReplaceSlab(project.replace_slab(new)?), vec![id])
        }
        AddColumn(column) => {
            let id = column.id;
            project.insert_column(column)?;
            applied(RemoveColumn(id), vec![id])
        }
        RemoveColumn(id) => applied(AddColumn(project.remove_column(id)?), vec![id]),
        ReplaceColumn(new) => {
            let id = new.id;
            applied(ReplaceColumn(project.replace_column(new)?), vec![id])
        }
        AddBeam(beam) => {
            let id = beam.id;
            project.insert_beam(beam)?;
            applied(RemoveBeam(id), vec![id])
        }
        RemoveBeam(id) => applied(AddBeam(project.remove_beam(id)?), vec![id]),
        ReplaceBeam(new) => {
            let id = new.id;
            applied(ReplaceBeam(project.replace_beam(new)?), vec![id])
        }
        Group(commands) => {
            let mut inverses = Vec::new();
            let mut changed = Vec::new();
            for command in commands {
                match apply(project, command) {
                    Ok(done) => {
                        inverses.push(done.inverse);
                        changed.extend(done.changed);
                    }
                    Err(e) => {
                        for inverse in inverses.into_iter().rev() {
                            apply(project, inverse)?;
                        }
                        return Err(e);
                    }
                }
            }
            inverses.reverse();
            applied(Group(inverses), changed)
        }
    }
}

fn edit_opening(
    project: &mut Project,
    id: Id,
    edit: impl FnOnce(&mut Opening) -> Result<(), ModelError>,
) -> Result<Applied, ModelError> {
    let o = project.opening_mut(id)?;
    let old = o.clone();
    let mut new = old.clone();
    edit(&mut new)?;
    *o = new;
    let wall = old.wall;
    Ok(Applied {
        inverse: Command::ReplaceOpening(old),
        changed: vec![id, wall],
    })
}

/// A project with its undo and redo history.
#[derive(Debug, Default)]
pub struct History {
    project: Project,
    undo: Vec<Command>,
    redo: Vec<Command>,
}

impl History {
    /// Starts a history on a project.
    pub fn new(project: Project) -> Self {
        Self {
            project,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// The project in its current state.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// A new identifier for an element to be added.
    pub fn new_id(&mut self) -> Id {
        self.project.new_id()
    }

    /// Runs a command. Returns the elements it changed.
    pub fn run(&mut self, command: Command) -> Result<Vec<Id>, ModelError> {
        let done = apply(&mut self.project, command)?;
        self.undo.push(done.inverse);
        self.redo.clear();
        Ok(done.changed)
    }

    /// Undoes the last command. Returns the elements it changed, or `None`
    /// when there is nothing to undo.
    pub fn undo(&mut self) -> Result<Option<Vec<Id>>, ModelError> {
        let Some(command) = self.undo.pop() else {
            return Ok(None);
        };
        let done = apply(&mut self.project, command)?;
        self.redo.push(done.inverse);
        Ok(Some(done.changed))
    }

    /// Redoes the last undone command.
    pub fn redo(&mut self) -> Result<Option<Vec<Id>>, ModelError> {
        let Some(command) = self.redo.pop() else {
            return Ok(None);
        };
        let done = apply(&mut self.project, command)?;
        self.undo.push(done.inverse);
        Ok(Some(done.changed))
    }
}
