//! The properties palette: the parameters of the selected element, edited
//! in place. Every change becomes one command when the user lets go.

use archibim_commands::Command;
use archibim_geometry::{contains, cuts_through, stair_steps};
use archibim_model::{
    Align, Anchor, Beam, Column, Guide, GuideShape, Id, Layer, LayerRole, Material, Mesh,
    MeshPoint, Opening, Point, Project, RenovationStatus, Roof, RoofPlane, Slab, Stair, Wall,
    WallReference,
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
    /// A roof.
    Roof(Roof),
    /// A terrain mesh.
    Mesh(Mesh),
    /// A stair.
    Stair(Stair),
    /// A guide placed by hand.
    Guide(Guide),
}

impl Element {
    /// The element with this identifier, if any.
    pub fn find(project: &Project, id: Id) -> Option<Self> {
        None.or_else(|| project.wall(id).ok().cloned().map(Self::Wall))
            .or_else(|| project.slab(id).ok().cloned().map(Self::Slab))
            .or_else(|| project.column(id).ok().cloned().map(Self::Column))
            .or_else(|| project.beam(id).ok().cloned().map(Self::Beam))
            .or_else(|| project.opening(id).ok().cloned().map(Self::Opening))
            .or_else(|| project.roof(id).ok().cloned().map(Self::Roof))
            .or_else(|| project.mesh(id).ok().cloned().map(Self::Mesh))
            .or_else(|| project.stair(id).ok().cloned().map(Self::Stair))
            .or_else(|| project.guide(id).ok().cloned().map(Self::Guide))
    }

    fn id(&self) -> Id {
        match self {
            Self::Wall(e) => e.id,
            Self::Slab(e) => e.id,
            Self::Column(e) => e.id,
            Self::Beam(e) => e.id,
            Self::Opening(e) => e.id,
            Self::Roof(e) => e.id,
            Self::Mesh(e) => e.id,
            Self::Stair(e) => e.id,
            Self::Guide(e) => e.id,
        }
    }

    fn replace(self) -> Command {
        match self {
            Self::Wall(e) => Command::ReplaceWall(e),
            Self::Slab(e) => Command::ReplaceSlab(e),
            Self::Column(e) => Command::ReplaceColumn(e),
            Self::Beam(e) => Command::ReplaceBeam(e),
            Self::Opening(e) => Command::ReplaceOpening(e),
            Self::Roof(e) => Command::ReplaceRoof(e),
            Self::Mesh(e) => Command::ReplaceMesh(e),
            Self::Stair(e) => Command::ReplaceStair(e),
            Self::Guide(e) => Command::ReplaceGuide(e),
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
            Self::Roof(_) => Command::RemoveRoof(id),
            Self::Mesh(_) => Command::RemoveMesh(id),
            Self::Stair(_) => Command::RemoveStair(id),
            Self::Guide(_) => Command::RemoveGuide(id),
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
        // On a drawn wall, the side-swap button flips it at once.
        let mut swap = false;
        let edit = fields(ui, draft, project, &mut swap);
        self.editing = edit.active;
        ui.separator();
        if let Element::Wall(w) = draft
            && swap
        {
            self.editing = false;
            return Some(flip(project, w));
        }
        if let Some(command) = cuts(ui, project, draft.id()) {
            self.editing = false;
            return Some(command);
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

/// The joins of `id`: for a roof or a slab, joining the walls, columns and
/// beams that pass through it, or releasing them; for a joined element,
/// releasing it. Each is one undo step.
fn cuts(ui: &mut Ui, project: &Project, id: Id) -> Option<Command> {
    let set = |pairs: Vec<(Id, Id)>, on: bool| {
        let commands: Vec<Command> = pairs
            .into_iter()
            .map(|(element, by)| Command::SetCut { element, by, on })
            .collect();
        (!commands.is_empty()).then_some(Command::Group(commands))
    };
    let mut command = None;
    let roof = project.roof(id).is_ok();
    if roof || project.slab(id).is_ok() {
        let cut: Vec<(Id, Id)> = project
            .cuts()
            .iter()
            .copied()
            .filter(|&(_, by)| by == id)
            .collect();
        // What passes through is worked out on the click only: it goes over
        // the whole model.
        let hint = if roof {
            "action.join_roof_hint"
        } else {
            "action.join_slab_hint"
        };
        if ui.button(t("action.join")).on_hover_text(t(hint)).clicked()
            && let Ok(through) = cuts_through(project, id)
        {
            command = set(through.into_iter().map(|e| (e, id)).collect(), true);
        }
        let label = format!("{} ({})", t("action.release_joins"), cut.len());
        if ui
            .add_enabled(!cut.is_empty(), egui::Button::new(label))
            .clicked()
        {
            command = set(cut, false);
        }
    } else {
        let cut: Vec<(Id, Id)> = project.cuts_of(id).map(|r| (id, r)).collect();
        if !cut.is_empty() && ui.button(t("action.unjoin")).clicked() {
            command = set(cut, false);
        }
    }
    command
}

/// The wall mirrored about its reference line, with its openings: one
/// undo step.
// ponytail: door hands and parts are kept as they are; check them against
// a real door once openings are detailed.
pub(crate) fn flip(project: &Project, wall: &Wall) -> Command {
    let length = ((wall.end.x - wall.start.x).powi(2) + (wall.end.y - wall.start.y).powi(2)).sqrt();
    let mut commands = vec![Command::ReplaceWall(wall.reversed())];
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

/// The reference lines of a wall, in the order of the list.
const REFERENCES: [(WallReference, &str); 6] = [
    (WallReference::LeftFace, "ref.left"),
    (WallReference::Axis, "ref.axis"),
    (WallReference::RightFace, "ref.right"),
    (WallReference::LoadBearingLeft, "ref.bearing_left"),
    (WallReference::LoadBearingAxis, "ref.bearing_axis"),
    (WallReference::LoadBearingRight, "ref.bearing_right"),
];

/// Draws the parameters of `element`, edited in place, and tells what the
/// widgets did. The side-swap button of a wall toggles `swap`.
pub(crate) fn fields(
    ui: &mut Ui,
    element: &mut Element,
    project: &Project,
    swap: &mut bool,
) -> Edit {
    let materials = &project.materials;
    let mut edit = Edit::default();
    let e = &mut edit;
    match element {
        Element::Wall(w) => {
            ui.label(t("el.wall"));
            build_up(ui, e, w.id, &mut w.layers, materials, "buildup.layers");
            number(ui, e, "p.bottom_offset", &mut w.bottom.offset, -1.0e6);
            number(ui, e, "p.top_offset", &mut w.top.offset, -1.0e6);
            ui.label(t("settings.reference"))
                .on_hover_text(t("ref.side_hint"));
            ui.horizontal(|ui| {
                let chosen = REFERENCES
                    .iter()
                    .find(|(r, _)| *r == w.reference)
                    .map_or("", |(_, key)| t(key));
                egui::ComboBox::from_id_salt("wall_reference")
                    .selected_text(chosen)
                    .show_ui(ui, |ui| {
                        for (reference, key) in REFERENCES {
                            e.track(&ui.selectable_value(&mut w.reference, reference, t(key)));
                        }
                    });
                ui.toggle_value(swap, t("ref.swap"))
                    .on_hover_text(t("ref.side_hint"));
            });
            number(ui, e, "p.ref_offset", &mut w.offset, -1.0e6);
        }
        Element::Slab(s) => {
            ui.label(t("el.slab"));
            build_up(ui, e, s.id, &mut s.layers, materials, "buildup.layers_slab");
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
        Element::Roof(r) => roof(ui, e, r, materials),
        Element::Mesh(m) => mesh(ui, e, m, materials),
        Element::Stair(s) => stair(ui, e, s, project),
        Element::Guide(g) => match &mut g.shape {
            GuideShape::Line { .. } => {
                ui.label(t("el.guide_line"));
            }
            GuideShape::Circle { radius, .. } => {
                ui.label(t("el.guide_circle"));
                number(ui, e, "p.radius", radius, 1.0);
            }
        },
    }
    edit
}

/// A roof: edge by edge a slope or a gable and the overhang, then the
/// eaves level, the plane it gives, and the build-up.
fn roof(ui: &mut Ui, e: &mut Edit, r: &mut Roof, materials: &[Material]) {
    ui.label(t("el.roof"));
    let numbered = r.edges.len() > 1;
    ui.label(t(if numbered {
        "roof.edges_hint"
    } else {
        "roof.all_edges"
    }));
    // ponytail: a gable given a slope again takes the first sloped edge's;
    // keep each edge's own once edges remember it.
    let fallback = r
        .edges
        .iter()
        .find_map(|edge| edge.slope)
        .unwrap_or(45f64.to_radians());
    for (i, edge) in r.edges.iter_mut().enumerate() {
        if numbered {
            ui.separator();
            ui.label(format!("{} {}", t("roof.edge"), i + 1));
        }
        if let Some(slope) = &mut edge.slope {
            ui.horizontal(|ui| {
                ui.label(t("roof.slope"));
                let mut degrees = slope.to_degrees();
                let s = ui.add(
                    DragValue::new(&mut degrees)
                        .speed(0.5)
                        .range(0.0..=89.0)
                        .suffix(" °"),
                );
                if s.changed() {
                    *slope = degrees.to_radians();
                }
                e.track(&s);
            });
        }
        number(ui, e, "roof.overhang", &mut edge.overhang, -1.0e5);
        let mut gable = edge.slope.is_none();
        let g = ui
            .checkbox(&mut gable, t("roof.gable"))
            .on_hover_text(t("roof.gable_hint"));
        if g.changed() {
            edge.slope = (!gable).then_some(fallback);
        }
        e.track(&g);
    }
    if numbered && ui.button(t("roof.apply_all")).clicked() {
        let first = r.edges[0];
        r.edges.iter_mut().for_each(|edge| *edge = first);
        e.commit = true;
    }
    ui.separator();
    number(ui, e, "p.eaves_offset", &mut r.eaves.offset, -1.0e6);
    for (plane, key) in [
        (RoofPlane::Bottom, "roof.plane_bottom"),
        (RoofPlane::Top, "roof.plane_top"),
    ] {
        e.track(&ui.selectable_value(&mut r.plane, plane, t(key)));
    }
    // Alert, never refuse.
    if !convex(&r.outline) {
        ui.colored_label(ui.visuals().warn_fg_color, t("roof.concave_warning"));
    }
    ui.separator();
    build_up(ui, e, r.id, &mut r.layers, materials, "buildup.layers_slab");
}

/// Whether a closed outline turns the same way at every corner.
fn convex(outline: &[Point]) -> bool {
    let n = outline.len();
    let turns: Vec<f64> = (0..n)
        .map(|i| {
            let (a, b, c) = (outline[i], outline[(i + 1) % n], outline[(i + 2) % n]);
            (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x)
        })
        .filter(|t| t.abs() > 1e-6)
        .collect();
    turns.iter().all(|t| *t > 0.0) || turns.iter().all(|t| *t < 0.0)
}

/// A terrain mesh: the height of each point of its outline, the points
/// inside it, its underside and material.
fn mesh(ui: &mut Ui, e: &mut Edit, m: &mut Mesh, materials: &[Material]) {
    ui.label(t("el.mesh"));
    material(ui, e, &mut m.material, materials);
    number(ui, e, "p.level_offset", &mut m.level.offset, -1.0e6);
    number(ui, e, "mesh.bottom", &mut m.bottom, -1.0e6);
    ui.label(t("mesh.points_hint"));
    for (i, p) in m.outline.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("{} {}", t("mesh.point"), i + 1));
            let r = ui.add(DragValue::new(&mut p.z).speed(5.0).suffix(" mm"));
            e.track(&r);
        });
    }
    ui.separator();
    ui.label(t("mesh.inner"));
    let mut remove = None;
    for (i, p) in m.inner.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            for value in [&mut p.at.x, &mut p.at.y, &mut p.z] {
                e.track(&ui.add(DragValue::new(value).speed(5.0)));
            }
            if ui
                .small_button("×")
                .on_hover_text(t("buildup.remove"))
                .clicked()
            {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        m.inner.remove(i);
        e.commit = true;
    }
    if !m.outline.is_empty() && ui.button(t("mesh.add_point")).clicked() {
        // In the middle of the outline, at its average height: then moved
        // and raised by hand.
        let n = m.outline.len() as f64;
        let sum = |f: fn(&MeshPoint) -> f64| m.outline.iter().map(f).sum::<f64>() / n;
        m.inner.push(MeshPoint {
            at: Point::new(sum(|p| p.at.x), sum(|p| p.at.y)),
            z: sum(|p| p.z),
        });
        e.commit = true;
    }
    // Alerts, never refusals.
    let outline: Vec<Point> = m.outline.iter().map(|p| p.at).collect();
    let warn = ui.visuals().warn_fg_color;
    if m.inner.iter().any(|p| !contains(&outline, p.at)) {
        ui.colored_label(warn, t("mesh.outside_warning"));
    }
    if m.outline.iter().chain(&m.inner).any(|p| p.z < m.bottom) {
        ui.colored_label(warn, t("mesh.below_warning"));
    }
}

/// Usual comfort of a stair (French practice): 2 risers + going, and the
/// riser height.
// ponytail: French ranges written here; preset data by country (TOOLS.md,
// rule 17).
const BLONDEL: (f64, f64) = (600.0, 640.0);
const RISER: (f64, f64) = (150.0, 200.0);

/// A stair: width, levels, risers worked out or set, and what they give;
/// a value outside the usual comfort is shown in the warning colour.
fn stair(ui: &mut Ui, e: &mut Edit, s: &mut Stair, project: &Project) {
    ui.label(t("el.stair"));
    number(ui, e, "p.width", &mut s.width, 1.0);
    number(ui, e, "p.bottom_offset", &mut s.bottom.offset, -1.0e6);
    number(ui, e, "p.top_offset", &mut s.top.offset, -1.0e6);
    material(ui, e, &mut s.material, &project.materials);
    let (Ok(bottom), Ok(top)) = (project.level(&s.bottom), project.level(&s.top)) else {
        return;
    };
    let steps = stair_steps(s, bottom, top);
    ui.separator();
    let mut auto = s.risers.is_none();
    let a = ui.checkbox(&mut auto, t("stair.auto_risers"));
    if a.changed() {
        s.risers = (!auto).then_some(steps.risers);
    }
    e.track(&a);
    ui.horizontal(|ui| {
        ui.label(t("stair.risers"));
        match &mut s.risers {
            Some(n) => e.track(&ui.add(DragValue::new(n).range(1..=200))),
            None => {
                ui.label(steps.risers.to_string());
            }
        }
    });
    let warn = ui.visuals().warn_fg_color;
    let value = |ui: &mut Ui, key, v: f64, range: Option<(f64, f64)>| {
        let text = match range {
            Some((lo, hi)) => format!("{} {v:.0} mm ({lo:.0} – {hi:.0})", t(key)),
            None => format!("{} {v:.0} mm", t(key)),
        };
        if range.is_some_and(|(lo, hi)| !(lo..=hi).contains(&v)) {
            ui.colored_label(warn, text);
        } else {
            ui.label(text);
        }
    };
    value(ui, "stair.riser", steps.riser.abs(), Some(RISER));
    value(ui, "stair.going", steps.going, None);
    value(ui, "stair.blondel", steps.blondel(), Some(BLONDEL));
}

fn material(ui: &mut Ui, e: &mut Edit, id: &mut Id, materials: &[Material]) {
    let name = materials
        .iter()
        .find(|m| m.id == *id)
        .map_or("?", |m| label(&m.name));
    ui.horizontal(|ui| {
        ui.label(t("p.material"));
        egui::ComboBox::from_id_salt("element_material")
            .selected_text(name)
            .show_ui(ui, |ui| {
                for m in materials {
                    e.track(&ui.selectable_value(id, m.id, label(&m.name)));
                }
            });
    });
}

/// The layers of a wall or a slab. One layer is a simple element, shown
/// as its material and thickness; several make a build-up, each layer with
/// its role, listed from the left face (the top of a slab).
fn build_up(
    ui: &mut Ui,
    e: &mut Edit,
    owner: Id,
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
                        .small_button(egui::RichText::new("↑").monospace())
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
    let built = |l: &Layer| l.status != RenovationStatus::New;
    // Removing or merging layers that are already built loses their
    // renovation status: asked first, never refused.
    let confirm_id = egui::Id::unique("buildup_confirm").with(owner.0);
    let mut confirm: Option<Change> = ui.data(|d| d.get_temp(confirm_id));
    if let Some(i) = remove {
        if layers.get(i).is_some_and(built) {
            confirm = Some(Change::Remove(i));
        } else {
            layers.remove(i);
            e.commit = true;
        }
    }
    if !simple {
        ui.label(format!(
            "{} {:.0} mm",
            t("buildup.total"),
            layers.iter().map(|l| l.thickness).sum::<f64>()
        ));
    }
    // A new layer has no thickness of its own: the architect types it
    // before the layer is added.
    // ponytail: the tool templates all have id 0, so a half-typed layer
    // follows from one tool's settings to another; key it by tool too if
    // that gets in the way.
    let adding_id = egui::Id::unique("buildup_adding").with(owner.0);
    let mut adding: Option<String> = ui.data(|d| d.get_temp(adding_id));
    ui.horizontal(|ui| {
        if adding.is_none() && ui.button(t("buildup.add")).clicked() {
            adding = Some(String::new());
        }
        if !simple && ui.button(t("buildup.make_simple")).clicked() {
            if layers.iter().any(built) {
                confirm = Some(Change::Simplify);
            } else {
                *layers = vec![simplified(layers)];
                e.commit = true;
            }
        }
    });
    if let Some(text) = &mut adding {
        let mut close = false;
        ui.horizontal(|ui| {
            ui.label(t("buildup.new_thickness"));
            let r = ui.add(
                egui::TextEdit::singleline(text)
                    .desired_width(60.0)
                    .hint_text("mm"),
            );
            let thickness = crate::typed::number(text).filter(|v| *v > 0.0);
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let ok = ui
                .add_enabled(thickness.is_some(), egui::Button::new(t("buildup.add_ok")))
                .clicked();
            if let (Some(thickness), true, Some(last)) = (thickness, ok || enter, layers.last()) {
                // On the interior face, of the last material.
                layers.push(Layer {
                    role: LayerRole::Finish,
                    thickness,
                    status: RenovationStatus::New,
                    framing: None,
                    ..*last
                });
                e.commit = true;
                close = true;
            }
            if ui
                .small_button("×")
                .on_hover_text(t("buildup.add_cancel"))
                .clicked()
            {
                close = true;
            }
        });
        if close {
            adding = None;
        }
    }
    ui.data_mut(|d| match adding {
        Some(s) => {
            d.insert_temp(adding_id, s);
        }
        None => d.remove::<String>(adding_id),
    });
    if !simple && layers.iter().any(built) {
        ui.colored_label(ui.visuals().warn_fg_color, t("buildup.renovation_warning"));
    }
    if let Some(change) = confirm {
        let mut answered = false;
        egui::Modal::new(confirm_id.with("modal")).show(ui.ctx(), |ui| {
            ui.label(t(match change {
                Change::Remove(_) => "buildup.confirm_remove",
                Change::Simplify => "buildup.confirm_merge",
            }));
            ui.horizontal(|ui| {
                if ui.button(t("buildup.confirm_yes")).clicked() {
                    match change {
                        Change::Remove(i) if i < layers.len() => {
                            layers.remove(i);
                        }
                        Change::Remove(_) => {}
                        Change::Simplify => *layers = vec![simplified(layers)],
                    }
                    e.commit = true;
                    answered = true;
                }
                if ui.button(t("buildup.confirm_no")).clicked() {
                    answered = true;
                }
            });
        });
        if answered {
            confirm = None;
        }
    }
    ui.data_mut(|d| match confirm {
        Some(c) => {
            d.insert_temp(confirm_id, c);
        }
        None => d.remove::<Change>(confirm_id),
    });
}

/// A change to a build-up that waits for the architect's yes.
#[derive(Clone, Copy)]
enum Change {
    Remove(usize),
    Simplify,
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
