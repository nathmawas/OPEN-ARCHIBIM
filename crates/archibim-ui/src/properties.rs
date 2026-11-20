//! The properties palette: the parameters of the selected element, edited
//! in place. Every change becomes one command when the user lets go.

use archibim_commands::Command;
use archibim_model::{
    Align, Anchor, Beam, Column, Id, Layer, LayerRole, Material, Opening, Project,
    RenovationStatus, Slab, Wall, WallReference,
};
use egui::{DragValue, Response, Ui};

use crate::i18n::{label, t};

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
        let edit = fields(ui, draft, &project.materials);
        self.editing = edit.active;
        ui.separator();
        if let Element::Wall(w) = draft
            && ui.button(t("action.flip")).clicked()
        {
            self.editing = false;
            return Some(flip(project, w));
        }
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

/// The wall mirrored about its reference line, with its openings: one
/// undo step.
// ponytail: door hands and parts are kept as they are; check them against
// a real door once openings are detailed.
pub(crate) fn flip(project: &Project, wall: &Wall) -> Command {
    let length = ((wall.end.x - wall.start.x).powi(2) + (wall.end.y - wall.start.y).powi(2)).sqrt();
    let mut commands = vec![Command::ReplaceWall(Wall {
        start: wall.end,
        end: wall.start,
        // The line stays where it is, now seen from the other end.
        offset: -wall.offset,
        ..wall.clone()
    })];
    for o in project.openings.iter().filter(|o| o.wall == wall.id) {
        let mut parts = o.parts.clone();
        parts.reverse();
        commands.push(Command::ReplaceOpening(Opening {
            position: length - o.position - o.wall_opening_width(),
            parts,
            ..o.clone()
        }));
    }
    Command::Group(commands)
}

/// Draws the parameters of `element`, edited in place, and tells what the
/// widgets did.
pub(crate) fn fields(ui: &mut Ui, element: &mut Element, materials: &[Material]) -> Edit {
    let mut edit = Edit::default();
    let e = &mut edit;
    match element {
        Element::Wall(w) => {
            ui.label(t("el.wall"));
            build_up(ui, e, &mut w.layers, materials, "buildup.layers");
            number(ui, e, "p.bottom_offset", &mut w.bottom.offset, -1.0e6);
            number(ui, e, "p.top_offset", &mut w.top.offset, -1.0e6);
            ui.label(t("settings.reference"));
            number(ui, e, "p.ref_offset", &mut w.offset, -1.0e6);
            for (reference, key) in [
                (WallReference::LeftFace, "ref.left"),
                (WallReference::Axis, "ref.axis"),
                (WallReference::RightFace, "ref.right"),
                (WallReference::LoadBearingLeft, "ref.bearing_left"),
                (WallReference::LoadBearingRight, "ref.bearing_right"),
            ] {
                e.track(&ui.selectable_value(&mut w.reference, reference, t(key)));
            }
        }
        Element::Slab(s) => {
            ui.label(t("el.slab"));
            build_up(ui, e, &mut s.layers, materials, "buildup.layers_slab");
            number(ui, e, "p.top_offset", &mut s.top.offset, -1.0e6);
        }
        Element::Column(c) => {
            ui.label(t("el.column"));
            e.track(&ui.checkbox(&mut c.round, t("p.round")));
            let width = if c.round { "p.diameter" } else { "p.width" };
            number(ui, e, width, &mut c.width, 1.0);
            if !c.round {
                number(ui, e, "p.depth", &mut c.depth, 1.0);
            }
            number(ui, e, "p.bottom_offset", &mut c.bottom.offset, -1.0e6);
            number(ui, e, "p.top_offset", &mut c.top.offset, -1.0e6);
            angle(ui, e, &mut c.angle);
            // Seen in plan: the front edge at the bottom.
            anchor(ui, e, &mut c.anchor, false);
        }
        Element::Beam(b) => {
            ui.label(t("el.beam"));
            number(ui, e, "p.width", &mut b.width, 1.0);
            number(ui, e, "p.height", &mut b.height, 1.0);
            number(ui, e, "p.level_offset", &mut b.level.offset, -1.0e6);
            // Seen in section: the top face at the top.
            anchor(ui, e, &mut b.anchor, true);
        }
        Element::Opening(o) => {
            ui.label(t("el.opening"));
            number(ui, e, "p.position", &mut o.position, 0.0);
            number(ui, e, "p.sill", &mut o.sill, -1.0e6);
            number(ui, e, "p.height", &mut o.height, 1.0);
            number(ui, e, "p.frame", &mut o.frame, 0.0);
            for part in &mut o.parts {
                number(ui, e, "p.part", &mut part.width, 1.0);
            }
        }
    }
    edit
}

/// The layers of a wall or a slab. One layer is a simple element, shown
/// as its material and thickness; several make a build-up, each layer with
/// its role, listed from the left face (the top of a slab).
fn build_up(
    ui: &mut Ui,
    e: &mut Edit,
    layers: &mut Vec<Layer>,
    materials: &[Material],
    title: &'static str,
) {
    let simple = layers.len() == 1;
    ui.label(t(if simple { "buildup.simple" } else { title }));
    let (mut remove, mut raise) = (None, None);
    for (i, layer) in layers.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            if !simple {
                egui::ComboBox::from_id_salt(("role", i))
                    .selected_text(t(role_key(layer.role)))
                    .show_ui(ui, |ui| {
                        for role in ROLES {
                            e.track(&ui.selectable_value(&mut layer.role, role, t(role_key(role))));
                        }
                    });
            }
            let name = |id| {
                materials
                    .iter()
                    .find(|m| m.id == id)
                    .map_or("?", |m| label(&m.name))
            };
            egui::ComboBox::from_id_salt(("material", i))
                .selected_text(name(layer.material))
                .show_ui(ui, |ui| {
                    for m in materials {
                        e.track(&ui.selectable_value(&mut layer.material, m.id, label(&m.name)));
                    }
                });
            let r = ui.add(
                DragValue::new(&mut layer.thickness)
                    .speed(1.0)
                    .range(1.0..=1.0e5)
                    .suffix(" mm"),
            );
            e.track(&r);
            if !simple {
                if i > 0
                    && ui
                        .small_button("↑")
                        .on_hover_text(t("buildup.up"))
                        .clicked()
                {
                    raise = Some(i);
                }
                if ui
                    .small_button("×")
                    .on_hover_text(t("buildup.remove"))
                    .clicked()
                {
                    remove = Some(i);
                }
            }
        });
    }
    if let Some(i) = raise {
        layers.swap(i - 1, i);
        e.commit = true;
    }
    if let Some(i) = remove {
        layers.remove(i);
        e.commit = true;
    }
    if !simple {
        ui.label(format!(
            "{} {:.0} mm",
            t("buildup.total"),
            layers.iter().map(|l| l.thickness).sum::<f64>()
        ));
    }
    ui.horizontal(|ui| {
        if ui.button(t("buildup.add")).clicked()
            && let Some(last) = layers.last()
        {
            // A thin finish on the right face, of the last material; the
            // architect then sets it.
            layers.push(Layer {
                role: LayerRole::Finish,
                thickness: 15.0,
                status: RenovationStatus::New,
                framing: None,
                ..*last
            });
            e.commit = true;
        }
        if !simple && ui.button(t("buildup.make_simple")).clicked() {
            *layers = vec![simplified(layers)];
            e.commit = true;
        }
    });
    // Alert, never refuse: removing or merging layers that are already
    // built loses their renovation status.
    if !simple && layers.iter().any(|l| l.status != RenovationStatus::New) {
        ui.colored_label(ui.visuals().warn_fg_color, t("buildup.renovation_warning"));
    }
}

/// One layer standing for a whole build-up: its total thickness, in the
/// material of its first load-bearing layer, or of its thickest.
pub(crate) fn simplified(layers: &[Layer]) -> Layer {
    let thickness = layers.iter().map(|l| l.thickness).sum();
    let main = layers
        .iter()
        .find(|l| l.role == LayerRole::LoadBearing)
        .or_else(|| {
            layers
                .iter()
                .max_by(|a, b| a.thickness.total_cmp(&b.thickness))
        });
    Layer {
        role: LayerRole::LoadBearing,
        thickness,
        framing: None,
        ..main.copied().unwrap_or(Layer {
            role: LayerRole::LoadBearing,
            material: Id(0),
            thickness,
            status: RenovationStatus::New,
            framing: None,
        })
    }
}

const ROLES: [LayerRole; 7] = [
    LayerRole::LoadBearing,
    LayerRole::Insulation,
    LayerRole::AirGap,
    LayerRole::Membrane,
    LayerRole::Lining,
    LayerRole::Finish,
    LayerRole::Covering,
];

fn number(ui: &mut Ui, edit: &mut Edit, key: &'static str, value: &mut f64, min: f64) {
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
}

/// An angle kept in radians, shown in degrees.
fn angle(ui: &mut Ui, edit: &mut Edit, radians: &mut f64) {
    ui.horizontal(|ui| {
        ui.label(t("p.angle"));
        let mut degrees = radians.to_degrees();
        let r = ui.add(
            DragValue::new(&mut degrees)
                .speed(1.0)
                .range(-360.0..=360.0)
                .suffix(" °"),
        );
        if r.changed() {
            *radians = degrees.to_radians();
        }
        edit.track(&r);
    });
}

/// The anchor as a grid of nine points, the chosen one filled.
fn anchor(ui: &mut Ui, edit: &mut Edit, anchor: &mut Anchor, first_on_top: bool) {
    ui.label(t("p.anchor"));
    let aligns = [Align::First, Align::Middle, Align::Last];
    let rows: Vec<Align> = if first_on_top {
        aligns.to_vec()
    } else {
        aligns.iter().rev().copied().collect()
    };
    egui::Grid::new(ui.next_auto_id()).show(ui, |ui| {
        for depth in rows {
            for width in aligns {
                let point = Anchor { width, depth };
                let mark = if *anchor == point { "●" } else { "○" };
                let r = ui.selectable_label(*anchor == point, mark);
                if r.clicked() {
                    *anchor = point;
                }
                edit.track(&r);
            }
            ui.end_row();
        }
    });
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

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(role: LayerRole, material: u64, thickness: f64) -> Layer {
        Layer {
            role,
            material: Id(material),
            thickness,
            status: RenovationStatus::New,
            framing: None,
        }
    }

    #[test]
    fn a_build_up_made_simple_keeps_its_thickness_and_its_structure() {
        let external = [
            layer(LayerRole::Finish, 1, 20.0),
            layer(LayerRole::Insulation, 2, 140.0),
            layer(LayerRole::LoadBearing, 3, 200.0),
            layer(LayerRole::Finish, 4, 15.0),
        ];
        let one = simplified(&external);
        assert_eq!(one.thickness, 375.0);
        assert_eq!(one.material, Id(3), "the load-bearing material");
        let lining = [
            layer(LayerRole::Finish, 1, 13.0),
            layer(LayerRole::Insulation, 2, 100.0),
        ];
        assert_eq!(simplified(&lining).material, Id(2), "else the thickest");
    }

    #[test]
    fn a_one_layer_build_up_made_simple_is_unchanged() {
        let one = [layer(LayerRole::Finish, 7, 90.0)];
        let simple = simplified(&one);
        assert_eq!(simple.thickness, 90.0);
        assert_eq!(simple.material, Id(7));
        assert_eq!(simple.role, LayerRole::LoadBearing, "a simple wall bears");
    }

    #[test]
    fn a_build_up_with_no_load_bearing_layer_takes_its_thickest() {
        let l = [
            layer(LayerRole::Finish, 1, 13.0),
            layer(LayerRole::Insulation, 2, 100.0),
            layer(LayerRole::Lining, 3, 13.0),
        ];
        let one = simplified(&l);
        assert_eq!((one.material, one.thickness), (Id(2), 126.0));
        // A tie keeps one of the thickest, whichever; the total holds.
        let tie = [
            layer(LayerRole::Finish, 1, 50.0),
            layer(LayerRole::Insulation, 2, 50.0),
        ];
        let one = simplified(&tie);
        assert!([Id(1), Id(2)].contains(&one.material));
        assert_eq!(one.thickness, 100.0);
    }

    #[test]
    fn the_first_load_bearing_layer_wins_even_when_thinner() {
        let l = [
            layer(LayerRole::LoadBearing, 1, 20.0),
            layer(LayerRole::Insulation, 2, 300.0),
            layer(LayerRole::LoadBearing, 3, 200.0),
        ];
        assert_eq!(simplified(&l).material, Id(1));
    }

    #[test]
    fn making_simple_drops_the_framing_and_keeps_the_main_status() {
        let framing = archibim_model::Framing {
            material: Id(9),
            width: 45.0,
            spacing: 600.0,
        };
        let mut bearing = layer(LayerRole::LoadBearing, 3, 140.0);
        bearing.framing = Some(framing);
        bearing.status = RenovationStatus::Existing;
        let l = [layer(LayerRole::Finish, 1, 12.0), bearing];
        let one = simplified(&l);
        assert_eq!(one.framing, None, "the members go with the layers");
        assert_eq!(one.thickness, 152.0);
        assert_eq!(
            one.status,
            RenovationStatus::Existing,
            "from the main layer"
        );
    }

    #[test]
    fn an_empty_build_up_gives_a_zero_layer_without_panicking() {
        let one = simplified(&[]);
        assert_eq!(one.thickness, 0.0);
        assert_eq!(one.role, LayerRole::LoadBearing);
    }
}
