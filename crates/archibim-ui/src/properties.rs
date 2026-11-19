//! The properties palette: the parameters of the selected element, edited
//! in place. Every change becomes one command when the user lets go.

use archibim_commands::Command;
use archibim_model::{Beam, Column, Id, LayerRole, Opening, Project, Slab, Wall};
use egui::{DragValue, Response, Ui};

use crate::i18n::t;

/// Any element the palette can show.
#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    /// A wall.
    Wall(Wall),
    /// A slab.
    Slab(Slab),
    /// A column.
    Column(Column),
    /// A beam.
    Beam(Beam),
    /// An opening.
    Opening(Opening),
}

impl Element {
    /// The element with this identifier, if any.
    pub fn find(project: &Project, id: Id) -> Option<Self> {
        None.or_else(|| project.wall(id).ok().cloned().map(Self::Wall))
            .or_else(|| project.slab(id).ok().cloned().map(Self::Slab))
            .or_else(|| project.column(id).ok().cloned().map(Self::Column))
            .or_else(|| project.beam(id).ok().cloned().map(Self::Beam))
            .or_else(|| project.opening(id).ok().cloned().map(Self::Opening))
    }

    fn id(&self) -> Id {
        match self {
            Self::Wall(e) => e.id,
            Self::Slab(e) => e.id,
            Self::Column(e) => e.id,
            Self::Beam(e) => e.id,
            Self::Opening(e) => e.id,
        }
    }

    fn replace(self) -> Command {
        match self {
            Self::Wall(e) => Command::ReplaceWall(e),
            Self::Slab(e) => Command::ReplaceSlab(e),
            Self::Column(e) => Command::ReplaceColumn(e),
            Self::Beam(e) => Command::ReplaceBeam(e),
            Self::Opening(e) => Command::ReplaceOpening(e),
        }
    }

    /// The command that deletes this element.
    pub fn remove(&self) -> Command {
        let id = self.id();
        match self {
            Self::Wall(_) => Command::RemoveWall(id),
            Self::Slab(_) => Command::RemoveSlab(id),
            Self::Column(_) => Command::RemoveColumn(id),
            Self::Beam(_) => Command::RemoveBeam(id),
            Self::Opening(_) => Command::RemoveOpening(id),
        }
    }
}

/// What the widgets of one frame did.
#[derive(Default)]
pub(crate) struct Edit {
    /// A widget is being dragged or typed into.
    pub(crate) active: bool,
    /// A widget was let go with a new value.
    pub(crate) commit: bool,
}

impl Edit {
    fn track(&mut self, r: &Response) {
        self.active |= r.dragged() || r.has_focus();
        self.commit |=
            r.drag_stopped() || r.lost_focus() || (r.changed() && !r.dragged() && !r.has_focus());
    }
}

/// The palette, with the values being edited.
#[derive(Debug, Default)]
pub struct Properties {
    draft: Option<Element>,
    editing: bool,
}

impl Properties {
    /// Shows the parameters of `selection`. Returns the command to run when
    /// the user changed a value or asked to delete the element.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        project: &Project,
        selection: Option<Id>,
    ) -> Option<Command> {
        ui.heading(t("props.title"));
        let current = selection.and_then(|id| Element::find(project, id));
        let stale = self.draft.as_ref().map(Element::id) != current.as_ref().map(Element::id);
        if !self.editing || stale {
            self.draft = current.clone();
        }
        let Some(draft) = &mut self.draft else {
            ui.label(t("props.none"));
            return None;
        };
        let edit = fields(ui, draft);
        self.editing = edit.active;
        ui.separator();
        if ui.button(t("action.delete")).clicked() {
            self.editing = false;
            return Some(draft.remove());
        }
        (edit.commit && current.as_ref() != Some(draft)).then(|| {
            self.editing = false;
            draft.clone().replace()
        })
    }
}

/// Draws the parameters of `element`, edited in place, and tells what the
/// widgets did.
pub(crate) fn fields(ui: &mut Ui, element: &mut Element) -> Edit {
    let mut edit = Edit::default();
    let mut number = |ui: &mut Ui, key: &'static str, value: &mut f64, min: f64| {
        ui.horizontal(|ui| {
            ui.label(t(key));
            let r = ui.add(
                DragValue::new(value)
                    .speed(5.0)
                    .range(min..=1.0e6)
                    .suffix(" mm"),
            );
            edit.track(&r);
        });
    };
    {
        match element {
            Element::Wall(w) => {
                ui.label(t("el.wall"));
                for layer in &mut w.layers {
                    number(ui, role_key(layer.role), &mut layer.thickness, 1.0);
                }
                number(ui, "p.bottom_offset", &mut w.bottom.offset, -1.0e6);
                number(ui, "p.top_offset", &mut w.top.offset, -1.0e6);
            }
            Element::Slab(s) => {
                ui.label(t("el.slab"));
                for layer in &mut s.layers {
                    number(ui, role_key(layer.role), &mut layer.thickness, 1.0);
                }
                number(ui, "p.top_offset", &mut s.top.offset, -1.0e6);
            }
            Element::Column(c) => {
                ui.label(t("el.column"));
                number(ui, "p.width", &mut c.width, 1.0);
                number(ui, "p.depth", &mut c.depth, 1.0);
                number(ui, "p.bottom_offset", &mut c.bottom.offset, -1.0e6);
                number(ui, "p.top_offset", &mut c.top.offset, -1.0e6);
            }
            Element::Beam(b) => {
                ui.label(t("el.beam"));
                number(ui, "p.width", &mut b.width, 1.0);
                number(ui, "p.height", &mut b.height, 1.0);
                number(ui, "p.top_offset", &mut b.top.offset, -1.0e6);
            }
            Element::Opening(o) => {
                ui.label(t("el.opening"));
                number(ui, "p.position", &mut o.position, 0.0);
                number(ui, "p.sill", &mut o.sill, -1.0e6);
                number(ui, "p.height", &mut o.height, 1.0);
                number(ui, "p.frame", &mut o.frame, 0.0);
                for part in &mut o.parts {
                    number(ui, "p.part", &mut part.width, 1.0);
                }
            }
        }
    }
    edit
}

fn role_key(role: LayerRole) -> &'static str {
    match role {
        LayerRole::LoadBearing => "role.load_bearing",
        LayerRole::Insulation => "role.insulation",
        LayerRole::AirGap => "role.air_gap",
        LayerRole::Membrane => "role.membrane",
        LayerRole::Lining => "role.lining",
        LayerRole::Finish => "role.finish",
        LayerRole::Covering => "role.covering",
    }
}
