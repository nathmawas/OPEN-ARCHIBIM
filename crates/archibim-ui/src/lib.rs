//! Windows, tools and palettes of OPEN-ARCHIBIM. Turns user gestures into
//! commands; never changes the model directly.

mod i18n;
mod model_view;
mod plan;
mod properties;
mod snap;
mod typed;
mod wall_tool;

use archibim_commands::{Command, History};
use archibim_geometry::{
    LayerPlan, beam_outline, column_outline, contains, contour_at, opening_outline, roof_faces,
    roof_overhang, stair_steps,
};
use archibim_model::{
    Beam, Column, Guide, GuideShape, Id, Layer, Mesh, MeshPoint, Opening, Point, Project, Roof,
    Slab, Stair, Wall,
};
use egui::{Key, Modifiers, Sense, Ui};

pub use i18n::t;
use model_view::ModelView;
use plan::PlanView;
use properties::fields;
pub use properties::{Element, Properties};
pub use snap::{BASE_ANGLES, PICKED_EDGES, Scene, Snap, SnapKind, SnapOptions};
use typed::Typed;
use wall_tool::rectangle;
pub use wall_tool::{WallMethod, WallTool};

/// How long the pointer rests on a point, before the first one is placed,
/// for that point to become the reference, in seconds.
const REST: f64 = 0.4;

/// Snapping distance to existing points, in screen pixels.
const SNAP_PX: f32 = 10.0;

/// A ready-made build-up (TOOLS.md, rule 17): one layer for a simple wall,
/// several for a composite one.
#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    /// Language pack key of its name.
    pub name: &'static str,
    /// Its layers, from the left face.
    pub layers: Vec<Layer>,
}

/// The elements each tool copies when it draws.
#[derive(Clone, Debug)]
pub struct Templates {
    /// Build-up, story and heights of new walls.
    pub wall: Wall,
    /// The build-ups offered to the wall tool, simple ones first.
    pub wall_presets: Vec<Preset>,
    /// Build-up and level of new slabs; the outline is drawn.
    pub slab: Slab,
    /// Size, material and heights of new columns.
    pub column: Column,
    /// Size, material and level of new beams.
    pub beam: Beam,
    /// A window; its wall and position are set where it is placed.
    pub window: Opening,
    /// A door; its wall and position are set where it is placed.
    pub door: Opening,
    /// Build-up and levels of new roofs; every edge of a new roof starts
    /// with the first edge setting.
    pub roof: Roof,
    /// Material and levels of new terrain meshes; the outline is drawn at
    /// height 0.
    pub mesh: Mesh,
    /// Width, levels and material of new stairs; the walking line is drawn.
    pub stair: Stair,
}

/// The tool the plan's clicks go to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    /// Picks an element to show in the properties palette.
    Select,
    /// Draws walls.
    Wall,
    /// Draws slab outlines.
    Slab,
    /// Places columns.
    Column,
    /// Draws beams.
    Beam,
    /// Places a window or a door in a wall.
    Opening,
    /// Draws roof outlines.
    Roof,
    /// Draws the walking line of a stair.
    Stair,
    /// Draws terrain mesh outlines.
    Mesh,
    /// Places guides to draw against.
    // ponytail: no tool has an icon yet (ADR 0004); draw this one with the
    // others when the tool bar gets its icons.
    Guide,
}

/// How the clicks of the guide tool make a guide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuideMethod {
    /// The line through two points.
    Line,
    /// A line parallel to an edge, on the side and at the distance clicked
    /// or typed.
    Parallel,
    /// A circle: its centre, then a point on it or a typed radius.
    Circle,
}

/// How the clicks of the slab tool make an outline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlabMethod {
    /// Click every corner, close on the first one or with Enter.
    Polygon,
    /// Two opposite corners.
    Rectangle,
}

/// What the middle of the window shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// The plan alone.
    Plan,
    /// The 3D view alone.
    Model,
    /// The plan and the 3D view side by side.
    Both,
}

/// The working window: tools on the left, the plan and the 3D view in the
/// middle, the properties on the right.
#[derive(Debug)]
pub struct Workspace {
    view: PlanView,
    model: ModelView,
    layout: Layout,
    /// The 3D view must be rebuilt.
    dirty: bool,
    tool: Tool,
    templates: Templates,
    wall: WallTool,
    slab_points: Vec<Point>,
    slab_method: SlabMethod,
    beam_start: Option<Point>,
    /// Each beam starts where the last one ended.
    beam_chain: bool,
    door: bool,
    selection: Option<Id>,
    properties: Properties,
    message: Option<String>,
    snap_options: SnapOptions,
    /// Edges the pointer passed over, most recent first, for guide lines.
    picked: Vec<(Point, Point)>,
    /// Length and angle typed for the next point.
    typed: Typed,
    /// Picking two points to place the next one halfway between them:
    /// the points picked so far.
    halfway: Option<Vec<Point>>,
    guide_method: GuideMethod,
    /// The edge a parallel guide follows, once clicked.
    guide_edge: Option<(Point, Point)>,
    /// Guides placed by hand are shown and snapped to.
    show_guides: bool,
    /// Measuring the next point from a reference point: `None` inside
    /// until the reference is clicked.
    reference: Option<Option<Point>>,
    /// The reference was taken by resting on it, not asked for with B.
    reference_auto: bool,
    /// The point the pointer rests on, and since when.
    resting: Option<(Point, f64)>,
}

impl Workspace {
    /// A workspace whose tools draw copies of `templates`.
    pub fn new(templates: Templates) -> Self {
        Self {
            view: PlanView::default(),
            model: ModelView::default(),
            layout: Layout::Both,
            dirty: true,
            tool: Tool::Select,
            wall: WallTool::new(templates.wall.clone()),
            templates,
            slab_points: Vec::new(),
            slab_method: SlabMethod::Polygon,
            beam_start: None,
            beam_chain: false,
            door: false,
            selection: None,
            properties: Properties::default(),
            message: None,
            snap_options: SnapOptions::default(),
            picked: Vec::new(),
            typed: Typed::default(),
            halfway: None,
            guide_method: GuideMethod::Line,
            guide_edge: None,
            show_guides: true,
            reference: None,
            reference_auto: false,
            resting: None,
        }
    }

    /// Selects an element, as a click with the selection tool would, or
    /// nothing.
    pub fn select(&mut self, id: Option<Id>) {
        self.tool = Tool::Select;
        self.selection = id;
    }

    /// Shows the plan, the 3D view, or both.
    pub fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
    }

    /// Makes `tool` the active tool, as clicking it does.
    pub fn set_tool(&mut self, tool: Tool) {
        if self.tool != tool {
            self.stop();
        }
        self.tool = tool;
    }

    /// Where the model point `p` shows on screen in the plan view.
    pub fn screen_of(&self, p: Point) -> egui::Pos2 {
        self.view.screen(p)
    }

    /// Called on the input before each frame: while a tool draws and no
    /// palette field has the keyboard, Tab switches between the typed
    /// length and angle instead of moving the keyboard focus.
    pub fn filter_input(&self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        if self.tool == Tool::Select || ctx.memory(|m| m.focused().is_some()) {
            return;
        }
        let mut events = Vec::with_capacity(raw.events.len());
        for event in raw.events.drain(..) {
            match event {
                egui::Event::Key {
                    key: Key::Tab,
                    pressed,
                    modifiers,
                    ..
                } if modifiers.is_none() => {
                    if pressed {
                        events.push(egui::Event::Text("<".to_owned()));
                    }
                }
                other => events.push(other),
            }
        }
        raw.events = events;
    }

    /// Shows the whole workspace. Returns true when the project changed.
    pub fn show(&mut self, ui: &mut Ui, history: &mut History, plan: &[LayerPlan]) -> bool {
        let mut changed = false;
        egui::Panel::left("tools").show(ui, |ui| self.tools(ui));
        egui::Panel::right("properties").show(ui, |ui| {
            if self.selection.is_none() && self.tool != Tool::Select {
                if let Some(command) = self.settings(ui, history.project()) {
                    changed |= self.run(history, command);
                }
            } else if let Some(command) =
                self.properties.show(ui, history.project(), self.selection)
            {
                changed |= self.run(history, command);
            }
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            let pending = match (&self.halfway, self.reference) {
                (Some(h), _) if h.is_empty() => Some(t("halfway.first")),
                (Some(_), _) => Some(t("halfway.second")),
                (None, Some(None)) => Some(t("reference.first")),
                (None, Some(Some(_))) => Some(t("reference.second")),
                (None, None) => None,
            };
            let hint = match self.tool {
                Tool::Guide if self.guide_method == GuideMethod::Parallel => {
                    t(if self.guide_edge.is_some() {
                        "guide.offset"
                    } else {
                        "guide.pick_edge"
                    })
                }
                Tool::Guide if self.guide_method == GuideMethod::Line => t("guide.line"),
                Tool::Guide => t("guide.circle"),
                Tool::Select => "",
                _ => t("draw.hint"),
            };
            ui.label(match (pending, self.message.as_deref()) {
                (Some(pending), _) => pending,
                (None, Some(message)) => message,
                (None, None) => hint,
            });
        });
        if self.dirty || self.model.stale(self.selection) {
            self.model.rebuild(history.project(), self.selection);
            self.dirty = false;
        }
        egui::CentralPanel::default().show(ui, |ui| match self.layout {
            Layout::Plan => changed |= self.plan(ui, history, plan),
            Layout::Model => {
                let typing = ui.ctx().memory(|m| m.focused().is_some());
                changed |= !typing && self.keys(ui, history, plan, None, None, 0.0);
                self.model.show(ui);
            }
            Layout::Both => ui.columns(2, |columns| {
                changed |= self.plan(&mut columns[0], history, plan);
                self.model.show(&mut columns[1]);
            }),
        });
        if changed {
            self.dirty = true;
            ui.ctx().request_repaint();
        }
        if self
            .selection
            .is_some_and(|id| Element::find(history.project(), id).is_none())
        {
            self.selection = None;
        }
        changed
    }

    fn tools(&mut self, ui: &mut Ui) {
        let before = self.tool;
        for (tool, key) in [
            (Tool::Select, "tool.select"),
            (Tool::Wall, "tool.wall"),
            (Tool::Slab, "tool.slab"),
            (Tool::Column, "tool.column"),
            (Tool::Beam, "tool.beam"),
            (Tool::Opening, "tool.opening"),
            (Tool::Roof, "tool.roof"),
            (Tool::Stair, "tool.stair"),
            (Tool::Mesh, "tool.mesh"),
            (Tool::Guide, "tool.guide"),
        ] {
            ui.selectable_value(&mut self.tool, tool, t(key));
        }
        if self.tool == Tool::Opening {
            ui.separator();
            ui.selectable_value(&mut self.door, false, t("preset.window"));
            ui.selectable_value(&mut self.door, true, t("preset.door"));
        }
        if self.tool != before {
            self.stop();
        }
        ui.separator();
        ui.label(t("snap.title"));
        let o = &mut self.snap_options;
        for (on, key) in [
            (&mut o.points, "snap.points"),
            (&mut o.midpoints, "snap.midpoints"),
            (&mut o.intersections, "snap.intersections"),
            (&mut o.edges, "snap.edges"),
            (&mut o.angles, "snap.angles"),
            (&mut o.alignments, "snap.alignments"),
            (&mut o.guides, "snap.guides"),
            (&mut self.show_guides, "snap.placed"),
        ] {
            ui.checkbox(on, t(key));
        }
        ui.label(t("snap.step"));
        ui.horizontal_wrapped(|ui| {
            for (on, angle) in o.base_angles.iter_mut().zip(BASE_ANGLES) {
                ui.toggle_value(on, format!("{angle}°"));
            }
            ui.label(t("snap.other"));
            ui.add(
                egui::DragValue::new(&mut o.other_angle)
                    .range(0..=180)
                    .suffix("°"),
            );
        });
        ui.horizontal(|ui| {
            ui.label(t("snap.divisions"));
            ui.add(egui::DragValue::new(&mut o.divisions).range(2..=12));
        });
        if self.tool != Tool::Select
            && ui
                .selectable_label(self.halfway.is_some(), t("snap.halfway"))
                .clicked()
        {
            self.halfway = match self.halfway {
                Some(_) => None,
                None => Some(Vec::new()),
            };
        }
        if self.tool != Tool::Select
            && ui
                .selectable_label(self.reference.is_some(), t("snap.reference"))
                .clicked()
        {
            self.ask_reference();
        }
        ui.separator();
        for (layout, key) in [
            (Layout::Plan, "view.plan"),
            (Layout::Model, "view.model"),
            (Layout::Both, "view.both"),
        ] {
            ui.selectable_value(&mut self.layout, layout, t(key));
        }
    }

    /// The settings of the active tool: how it draws, and the parameters
    /// of what it draws next. Returns the command of a button pressed.
    fn settings(&mut self, ui: &mut Ui, project: &Project) -> Option<Command> {
        ui.heading(t("settings.title"));
        ui.label(t("settings.method"));
        match self.tool {
            Tool::Guide => {
                let before = self.guide_method;
                for (method, key) in [
                    (GuideMethod::Line, "method.guide_line"),
                    (GuideMethod::Parallel, "method.guide_parallel"),
                    (GuideMethod::Circle, "method.guide_circle"),
                ] {
                    ui.selectable_value(&mut self.guide_method, method, t(key));
                }
                if self.guide_method != before {
                    self.stop();
                }
                ui.separator();
                // The guides of the story drawn on, in one undo step.
                let story = self.wall.template.story;
                let removes: Vec<Command> = project
                    .guides
                    .iter()
                    .filter(|g| g.story == story)
                    .map(|g| Command::RemoveGuide(g.id))
                    .collect();
                ui.label(t("guide.placed"));
                let label = format!("{} ({})", t("guide.clear"), removes.len());
                let clear = ui.add_enabled(!removes.is_empty(), egui::Button::new(label));
                return clear.clicked().then_some(Command::Group(removes));
            }
            Tool::Wall => {
                let w = &mut self.wall;
                for (method, key) in [
                    (WallMethod::Single, "method.single"),
                    (WallMethod::Chain, "method.chain"),
                    (WallMethod::Rectangle, "method.rectangle"),
                    (WallMethod::RotatedRectangle, "method.rotated_rectangle"),
                    (WallMethod::Circle, "method.circle"),
                    (WallMethod::Arc, "method.arc"),
                ] {
                    ui.selectable_value(&mut w.method, method, t(key));
                }
                ui.separator();
                ui.label(t("buildup.preset"));
                let chosen = self
                    .templates
                    .wall_presets
                    .iter()
                    .find(|p| p.layers == w.template.layers)
                    .map_or(t("buildup.custom"), |p| t(p.name));
                egui::ComboBox::from_id_salt("wall_preset")
                    .selected_text(chosen)
                    .show_ui(ui, |ui| {
                        for p in &self.templates.wall_presets {
                            if ui
                                .selectable_label(p.layers == w.template.layers, t(p.name))
                                .clicked()
                            {
                                w.template.layers = p.layers.clone();
                            }
                        }
                    });
            }
            Tool::Slab | Tool::Roof | Tool::Mesh => {
                for (method, key) in [
                    (SlabMethod::Polygon, "method.polygon"),
                    (SlabMethod::Rectangle, "method.rectangle"),
                ] {
                    ui.selectable_value(&mut self.slab_method, method, t(key));
                }
            }
            Tool::Beam => {
                ui.selectable_value(&mut self.beam_chain, false, t("method.single"));
                ui.selectable_value(&mut self.beam_chain, true, t("method.chain"));
            }
            Tool::Column | Tool::Opening | Tool::Select => {
                ui.label(t("method.point"));
            }
            Tool::Stair => {
                ui.label(t("method.path"));
            }
        }
        ui.separator();
        let mut element = match self.tool {
            Tool::Wall => Element::Wall(self.wall.template.clone()),
            Tool::Slab => Element::Slab(self.templates.slab.clone()),
            Tool::Column => Element::Column(self.templates.column.clone()),
            Tool::Beam => Element::Beam(self.templates.beam.clone()),
            Tool::Roof => Element::Roof(self.templates.roof.clone()),
            Tool::Mesh => Element::Mesh(self.templates.mesh.clone()),
            Tool::Stair => Element::Stair(self.templates.stair.clone()),
            Tool::Opening if self.door => Element::Opening(self.templates.door.clone()),
            Tool::Opening | Tool::Select | Tool::Guide => {
                Element::Opening(self.templates.window.clone())
            }
        };
        fields(ui, &mut element, project);
        // Settings are not part of the model: they change at once, with no
        // command and no undo.
        match element {
            Element::Wall(w) => self.wall.template = w,
            Element::Slab(s) => self.templates.slab = s,
            Element::Column(c) => self.templates.column = c,
            Element::Beam(b) => self.templates.beam = b,
            Element::Roof(r) => self.templates.roof = r,
            Element::Mesh(m) => self.templates.mesh = m,
            Element::Stair(s) => self.templates.stair = s,
            Element::Opening(o) if self.door => self.templates.door = o,
            Element::Opening(o) => self.templates.window = o,
            Element::Guide(_) => {}
        }
        None
    }

    fn stop(&mut self) {
        self.wall.stop();
        self.slab_points.clear();
        self.beam_start = None;
        self.picked.clear();
        self.typed.clear();
        self.halfway = None;
        self.guide_edge = None;
        self.reference = None;
    }

    /// B or its button: the next click picks the reference point, or, if
    /// one was asked for already, no reference.
    fn ask_reference(&mut self) {
        self.reference = asked_reference(self.reference, self.reference_auto);
        self.reference_auto = false;
    }

    /// Before the first point, the point the pointer rests on for a
    /// moment becomes the reference, unless one was asked for with B.
    fn rest(&mut self, ui: &Ui, on: Option<&Snap>, drawing: bool) {
        let now = ui.input(|i| i.time);
        let point = on
            .filter(|s| {
                matches!(
                    s.kind,
                    SnapKind::Point | SnapKind::Midpoint | SnapKind::Intersection
                )
            })
            .map(|s| s.point);
        let free = self.reference.is_none() || self.reference_auto;
        match (point, self.resting) {
            (Some(p), Some((q, since))) if p == q => {
                if !drawing && free && now - since >= REST && self.tool != Tool::Select {
                    self.reference = Some(Some(p));
                    self.reference_auto = true;
                }
            }
            (Some(p), _) => {
                self.resting = Some((p, now));
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f64(REST));
            }
            (None, _) => self.resting = None,
        }
    }

    fn run(&mut self, history: &mut History, command: Command) -> bool {
        match history.run(command) {
            Ok(_) => {
                self.message = None;
                true
            }
            Err(e) => {
                // ponytail: model errors are shown in English; translate
                // them once they carry a language pack key.
                self.message = Some(format!("{} {e}", t("status.refused")));
                false
            }
        }
    }

    fn plan(&mut self, ui: &mut Ui, history: &mut History, plan: &[LayerPlan]) -> bool {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        self.view.navigate(ui, &response, plan);

        let snap = f64::from(SNAP_PX / self.view.scale());
        let shift = ui.input(|i| i.modifiers.shift);
        let from = self
            .wall
            .start()
            .or(self.beam_start)
            .or(self.slab_points.last().copied());
        // The direction of the previous segment, which typed angles turn
        // from. ponytail: the chord of a curved wall, not its end tangent;
        // a chained beam starts afresh from the x axis.
        let placed = match self.wall.points() {
            [] => &self.slab_points[..],
            points => points,
        };
        // An arc drawn by its points has no previous segment yet.
        let arc = matches!(self.wall.method, WallMethod::Arc | WallMethod::Circle);
        let base = match placed {
            [.., a, b] if !arc => (b.y - a.y).atan2(b.x - a.x).to_degrees(),
            _ => 0.0,
        };
        let drawing_started = from.is_some();
        // Measuring from a reference point: typed values start from it.
        let (from, base) = match self.reference {
            Some(Some(r)) => (Some(r), 0.0),
            _ => (from, base),
        };
        let drawing: Vec<Point> = self
            .wall
            .points()
            .iter()
            .chain(&self.slab_points)
            .chain(&self.beam_start)
            .copied()
            .collect();
        let snapped = response.hover_pos().map(|p| {
            let p = self.view.model(p);
            let mut scene = Scene::new(history.project(), &drawing);
            // ponytail: guides of every story, like everything else in the
            // one plan there is; keep those of the view's story once views
            // have their own story (ADR 0009).
            if self.show_guides {
                let shapes: Vec<GuideShape> =
                    history.project().guides.iter().map(|g| g.shape).collect();
                scene.place_guides(&shapes);
            }
            // The edge under the pointer is picked up for guide lines.
            if let Some(edge) = scene.edge_near(p, snap)
                && !self.picked.contains(&edge)
            {
                self.picked.insert(0, edge);
                self.picked.truncate(PICKED_EDGES);
            }
            // Edges undone, deleted or moved give no guides any more.
            self.picked.retain(|e| scene.has_edge(*e));
            scene.pick(&self.picked);
            scene.snap(&self.snap_options, p, from, shift, snap)
        });
        self.rest(ui, snapped.as_ref(), drawing_started);
        // Where the next point goes: what was typed, or the snapped pointer.
        let cursor = snapped
            .as_ref()
            .map(|s| self.next_point(from, base, s).unwrap_or(s.point));
        if let Some(p) = cursor {
            self.wall.hover(p);
        }

        let typing = ui.ctx().memory(|m| m.focused().is_some());
        let mut changed = !typing && self.keys(ui, history, plan, snapped.as_ref(), from, base);
        // Space held: the tool takes the contour under the pointer.
        let space = !typing && ui.input(|i| i.key_down(Key::Space));
        let outline_story = match self.tool {
            Tool::Slab => Some(self.templates.slab.story),
            Tool::Roof => Some(self.templates.roof.story),
            Tool::Mesh => Some(self.templates.mesh.story),
            _ => None,
        };
        let contour = match (space, outline_story, self.tool, response.hover_pos()) {
            (true, Some(story), _, Some(p)) => {
                contour_at(history.project(), story, self.view.model(p))
            }
            (true, None, Tool::Wall, Some(p)) => contour_at(
                history.project(),
                self.wall.template.story,
                self.view.model(p),
            ),
            _ => None,
        };
        if response.clicked() {
            if let Some(outline) = contour.clone() {
                changed |= match self.tool {
                    Tool::Wall => self.wall.outline(history, outline),
                    _ => {
                        self.slab_points = outline;
                        self.close_outline(history)
                    }
                };
            } else if let Some(p) = cursor {
                changed |= self.place(history, plan, p, snap);
            }
        }

        self.view
            .paint(&painter, history.project(), plan, self.selection);
        if self.show_guides {
            for g in &history.project().guides {
                let stroke = if self.selection == Some(g.id) {
                    egui::Stroke::new(2.0, plan::SELECTED)
                } else {
                    egui::Stroke::new(1.0, plan::PLACED)
                };
                self.view.guide(&painter, &g.shape, stroke);
            }
        }
        if self.tool == Tool::Guide {
            // The edge followed, then the guide about to be placed.
            if let Some((a, b)) = self.guide_edge {
                let edge = egui::Stroke::new(3.0, plan::SELECTED);
                painter.line_segment([self.view.screen(a), self.view.screen(b)], edge);
            }
            if let Some(shape) = cursor.and_then(|p| self.guide_at(p)) {
                self.view
                    .guide(&painter, &shape, egui::Stroke::new(2.0, plan::PLACED));
            }
        }
        if let Some(Some(r)) = self.reference {
            self.view.reference(&painter, r, cursor);
        }
        let rectangle_from = match self.tool {
            Tool::Wall if self.wall.method == WallMethod::Rectangle => self.wall.start(),
            Tool::Slab | Tool::Roof | Tool::Mesh if self.slab_method == SlabMethod::Rectangle => {
                self.slab_points.first().copied()
            }
            _ => None,
        };
        self.view.preview(
            &painter,
            &self.wall,
            &self.slab_points,
            self.beam_start,
            rectangle_from,
            cursor,
        );
        if let Some(p) = cursor
            && self.tool != Tool::Select
        {
            let live = from.map(|a| {
                let (dx, dy) = (p.x - a.x, p.y - a.y);
                let turn = (dy.atan2(dx).to_degrees() - base + 180.0).rem_euclid(360.0) - 180.0;
                ((dx * dx + dy * dy).sqrt(), turn)
            });
            // Typed values that give no point show as a warning.
            let gives_point = snapped
                .as_ref()
                .is_some_and(|s| self.next_point(from, base, s).is_some());
            let label = self.typed.label(live);
            let warn = !self.typed.is_empty() && !gives_point;
            self.view
                .measure(&painter, p, &label, warn, self.halfway.as_deref());
        }
        if let Some(outline) = &contour {
            self.view.contour(&painter, outline);
        } else if let Some(s) = &snapped {
            self.view.snap_marks(&painter, s);
        }
        if from.is_some() {
            ui.ctx().request_repaint();
        }
        changed
    }

    /// Places the point `p`, or, while picking two points to go halfway
    /// between, picks it.
    fn place(&mut self, history: &mut History, plan: &[LayerPlan], p: Point, snap: f64) -> bool {
        self.typed.clear();
        // The reference is picked first, then serves for one point.
        match self.reference {
            Some(None) => {
                self.reference = Some(Some(p));
                return false;
            }
            Some(Some(_)) => self.reference = None,
            None => {}
        }
        let Some(picked) = &mut self.halfway else {
            return self.click(history, plan, p, snap);
        };
        let Some(&a) = picked.first() else {
            picked.push(p);
            return false;
        };
        self.halfway = None;
        let middle = Point::new((a.x + p.x) / 2.0, (a.y + p.y) / 2.0);
        self.click(history, plan, middle, snap)
    }

    fn click(&mut self, history: &mut History, plan: &[LayerPlan], p: Point, snap: f64) -> bool {
        match self.tool {
            Tool::Select => {
                self.selection = hit(history.project(), plan, p, snap, self.show_guides);
                false
            }
            Tool::Guide => {
                if let Some(shape) = self.guide_at(p) {
                    self.slab_points.clear();
                    self.guide_edge = None;
                    let guide = Guide {
                        id: history.new_id(),
                        story: self.wall.template.story,
                        shape,
                    };
                    return self.run(history, Command::AddGuide(guide));
                }
                if self.guide_method == GuideMethod::Parallel {
                    self.guide_edge = Scene::new(history.project(), &[]).edge_near(p, snap);
                    if self.guide_edge.is_none() {
                        self.message = Some(t("guide.no_edge").to_owned());
                    }
                } else {
                    self.slab_points.push(p);
                }
                false
            }
            Tool::Stair => {
                // Clicking the last point again ends the walking line.
                if self.slab_points.len() >= 2
                    && self
                        .slab_points
                        .last()
                        .is_some_and(|q| distance(*q, p) <= snap)
                {
                    self.close_stair(history)
                } else {
                    self.slab_points.push(p);
                    false
                }
            }
            Tool::Wall => self.wall.click(history, p),
            Tool::Slab | Tool::Roof | Tool::Mesh => {
                let closes = self.slab_points.len() >= 3
                    && self
                        .slab_points
                        .first()
                        .is_some_and(|q| distance(*q, p) <= snap);
                if self.slab_method == SlabMethod::Rectangle
                    && let Some(&a) = self.slab_points.first()
                {
                    self.slab_points = rectangle(a, p).unwrap_or_default();
                    self.close_outline(history)
                } else if closes {
                    self.close_outline(history)
                } else {
                    self.slab_points.push(p);
                    false
                }
            }
            Tool::Column => {
                let column = Column {
                    id: history.new_id(),
                    at: p,
                    ..self.templates.column.clone()
                };
                self.run(history, Command::AddColumn(column))
            }
            Tool::Beam => match self.beam_start.take() {
                None => {
                    self.beam_start = Some(p);
                    false
                }
                Some(start) => {
                    let beam = Beam {
                        id: history.new_id(),
                        start,
                        end: p,
                        ..self.templates.beam.clone()
                    };
                    let added = self.run(history, Command::AddBeam(beam));
                    if self.beam_chain {
                        self.beam_start = Some(p);
                    }
                    added
                }
            },
            Tool::Opening => {
                let template = if self.door {
                    self.templates.door.clone()
                } else {
                    self.templates.window.clone()
                };
                let Some((wall, along, length)) = wall_at(history.project(), plan, p) else {
                    return false;
                };
                let width = template.wall_opening_width();
                let opening = Opening {
                    id: history.new_id(),
                    wall,
                    position: (along - width / 2.0).clamp(0.0, (length - width).max(0.0)),
                    ..template
                };
                self.run(history, Command::AddOpening(opening))
            }
        }
    }

    /// Where the next point goes with what was typed, if that gives one: a
    /// parallel guide at the typed distance from its edge, on the pointer's
    /// side, or the typed length and angle.
    fn next_point(&self, from: Option<Point>, base: f64, s: &Snap) -> Option<Point> {
        if let Some((a, b)) = self.guide_edge {
            let l = self.typed.length()?;
            let f = foot(a, b, s.point)?;
            let (dx, dy) = (s.point.x - f.x, s.point.y - f.y);
            let d = dx.hypot(dy);
            return (d > 1e-9).then(|| Point::new(f.x + dx / d * l, f.y + dy / d * l));
        }
        self.typed.point(from, base, s)
    }

    /// The guide a click at `p` places, once its first point or its edge
    /// is picked.
    fn guide_at(&self, p: Point) -> Option<GuideShape> {
        if self.tool != Tool::Guide {
            return None;
        }
        match (self.guide_method, self.guide_edge, self.slab_points.first()) {
            (GuideMethod::Parallel, Some((a, b)), _) => {
                let f = foot(a, b, p)?;
                let (dx, dy) = (p.x - f.x, p.y - f.y);
                Some(GuideShape::Line {
                    a: Point::new(a.x + dx, a.y + dy),
                    b: Point::new(b.x + dx, b.y + dy),
                })
            }
            (GuideMethod::Line, _, Some(&a)) => Some(GuideShape::Line { a, b: p }),
            (GuideMethod::Circle, _, Some(&centre)) => Some(GuideShape::Circle {
                centre,
                radius: distance(centre, p),
            }),
            _ => None,
        }
    }

    /// Makes the element of the active tool from the outline drawn.
    fn close_outline(&mut self, history: &mut History) -> bool {
        let outline = std::mem::take(&mut self.slab_points);
        if outline.len() < 3 {
            return false;
        }
        let id = history.new_id();
        let command = match self.tool {
            Tool::Roof => {
                let first = self.templates.roof.edges.first().copied();
                Command::AddRoof(Roof {
                    id,
                    edges: first.map(|e| vec![e; outline.len()]).unwrap_or_default(),
                    outline,
                    ..self.templates.roof.clone()
                })
            }
            Tool::Mesh => Command::AddMesh(Mesh {
                id,
                outline: outline.iter().map(|&at| MeshPoint { at, z: 0.0 }).collect(),
                ..self.templates.mesh.clone()
            }),
            _ => Command::AddSlab(Slab {
                id,
                outline,
                ..self.templates.slab.clone()
            }),
        };
        self.run(history, command)
    }

    fn close_stair(&mut self, history: &mut History) -> bool {
        if self.slab_points.len() < 2 {
            self.slab_points.clear();
            return false;
        }
        let stair = Stair {
            id: history.new_id(),
            path: std::mem::take(&mut self.slab_points),
            ..self.templates.stair.clone()
        };
        self.run(history, Command::AddStair(stair))
    }

    /// Keyboard: undo, redo, delete, escape, the typed lengths and angles,
    /// and M for the point halfway between two others.
    fn keys(
        &mut self,
        ui: &Ui,
        history: &mut History,
        plan: &[LayerPlan],
        snapped: Option<&Snap>,
        from: Option<Point>,
        base: f64,
    ) -> bool {
        let drawing = self.tool != Tool::Select;
        let snap = f64::from(SNAP_PX / self.view.scale());
        let (undo, redo, delete) = ui.input_mut(|i| {
            let redo = i.consume_key(Modifiers::COMMAND, Key::Y)
                || i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
            let undo = i.consume_key(Modifiers::COMMAND, Key::Z);
            (undo, redo, i.key_pressed(Key::Delete))
        });
        let mut changed = false;
        if undo || redo {
            self.stop();
            let done = if undo { history.undo() } else { history.redo() };
            changed |= matches!(done, Ok(Some(_)));
        }
        if delete
            && let Some(element) = self
                .selection
                .and_then(|id| Element::find(history.project(), id))
        {
            changed |= self.run(history, element.remove());
        }
        let events = ui.input(|i| i.events.clone());
        for event in events {
            match event {
                egui::Event::Text(text) if drawing => {
                    for c in text.chars() {
                        if c.eq_ignore_ascii_case(&'m') {
                            self.halfway = match self.halfway {
                                Some(_) => None,
                                None => Some(Vec::new()),
                            };
                        } else if c.eq_ignore_ascii_case(&'b') {
                            self.ask_reference();
                        } else {
                            self.typed.type_char(c, from.is_some());
                        }
                    }
                }
                egui::Event::Key {
                    key, pressed: true, ..
                } => match key {
                    Key::Backspace => self.typed.erase(),
                    // Escape first forgets what was typed, then the halfway
                    // or reference point being picked, then stops drawing.
                    Key::Escape if !self.typed.is_empty() => self.typed.clear(),
                    Key::Escape if self.halfway.is_some() || self.reference.is_some() => {
                        self.halfway = None;
                        self.reference = None;
                    }
                    // A parallel guide: back to picking its edge.
                    Key::Escape if self.guide_edge.is_some() => self.guide_edge = None,
                    Key::Escape => {
                        self.stop();
                        self.selection = None;
                    }
                    // With something typed, Enter places that point.
                    Key::Enter
                        if drawing
                            && snapped
                                .is_some_and(|s| self.next_point(from, base, s).is_some()) =>
                    {
                        if let Some(p) = snapped.and_then(|s| self.next_point(from, base, s)) {
                            changed |= self.place(history, plan, p, snap);
                        }
                    }
                    // Typed values that give no point: Enter waits for a fix.
                    Key::Enter if !self.typed.is_empty() => {}
                    Key::Enter if matches!(self.tool, Tool::Slab | Tool::Roof | Tool::Mesh) => {
                        changed |= self.close_outline(history);
                    }
                    Key::Enter if self.tool == Tool::Stair => {
                        changed |= self.close_stair(history);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        changed
    }
}

/// The reference after B: a reference to pick, or none if one was asked
/// for already; one taken by resting (`auto`) gives way to a new pick.
fn asked_reference(current: Option<Option<Point>>, auto: bool) -> Option<Option<Point>> {
    match current {
        Some(_) if !auto => None,
        _ => Some(None),
    }
}

/// The foot of the perpendicular from `p` on the line through `a` and `b`.
fn foot(a: Point, b: Point, p: Point) -> Option<Point> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    (l2 > 1e-18).then(|| {
        let k = ((p.x - a.x) * dx + (p.y - a.y) * dy) / l2;
        Point::new(a.x + dx * k, a.y + dy * k)
    })
}

fn distance(a: Point, b: Point) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// The element under `p`: openings first, then columns, beams, stairs,
/// walls, the lines of roofs (within `near`), the guides placed by hand
/// if `guides` are shown, slabs, the inside of roofs and terrain meshes.
fn hit(project: &Project, plan: &[LayerPlan], p: Point, near: f64, guides: bool) -> Option<Id> {
    let opening = project
        .openings
        .iter()
        .map(|o| o.id)
        .find(|&id| opening_outline(project, id).is_ok_and(|outline| contains(&outline, p)));
    let column = || {
        project
            .columns
            .iter()
            .find(|c| contains(&column_outline(c), p))
            .map(|c| c.id)
    };
    let beam = || {
        project
            .beams
            .iter()
            .find(|b| beam_outline(b).is_some_and(|o| contains(&o, p)))
            .map(|b| b.id)
    };
    let stair = || {
        project.stairs.iter().map(|s| s.id).find(|&id| {
            project.stair(id).is_ok_and(|s| {
                let (b, t) = (project.level(&s.bottom), project.level(&s.top));
                stair_steps(s, b.unwrap_or(0.0), t.unwrap_or(0.0))
                    .treads
                    .iter()
                    .any(|tread| contains(tread, p))
            })
        })
    };
    let roof_line = || {
        project
            .roofs
            .iter()
            .find(|r| {
                roof_faces(r).iter().any(|f| {
                    let n = f.polygon.len();
                    (0..n)
                        .any(|i| segment_distance(p, f.polygon[i], f.polygon[(i + 1) % n]) <= near)
                })
            })
            .map(|r| r.id)
    };
    let roof = || {
        project
            .roofs
            .iter()
            .find(|r| contains(&roof_overhang(r), p))
            .map(|r| r.id)
    };
    let mesh = || {
        project
            .meshes
            .iter()
            .find(|m| contains(&m.outline.iter().map(|q| q.at).collect::<Vec<_>>(), p))
            .map(|m| m.id)
    };
    let wall = || wall_at(project, plan, p).map(|(id, _, _)| id);
    let guide = || {
        let on = |g: &Guide| match g.shape {
            GuideShape::Line { a, b } => foot(a, b, p).is_some_and(|f| distance(f, p) <= near),
            GuideShape::Circle { centre, radius } => (distance(centre, p) - radius).abs() <= near,
        };
        project
            .guides
            .iter()
            .filter(|_| guides)
            .find(|g| on(g))
            .map(|g| g.id)
    };
    let slab = || {
        project
            .slabs
            .iter()
            .find(|s| contains(&s.outline, p))
            .map(|s| s.id)
    };
    opening
        .or_else(column)
        .or_else(beam)
        .or_else(stair)
        .or_else(wall)
        .or_else(roof_line)
        .or_else(guide)
        .or_else(slab)
        .or_else(roof)
        .or_else(mesh)
}

/// Distance from `p` to the segment from `a` to `b`.
fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let k = if l2 > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    distance(p, Point::new(a.x + dx * k, a.y + dy * k))
}

/// The wall under `p`, with the distance of `p` along it and its length.
fn wall_at(project: &Project, plan: &[LayerPlan], p: Point) -> Option<(Id, f64, f64)> {
    let id = plan
        .iter()
        .find(|l| l.polygons.iter().any(|poly| contains(poly, p)))
        .map(|l| l.wall)?;
    let w = project.wall(id).ok()?;
    let length = distance(w.start, w.end);
    let along = ((p.x - w.start.x) * (w.end.x - w.start.x)
        + (p.y - w.start.y) * (w.end.y - w.start.y))
        / length;
    Some((id, along, length))
}

#[cfg(test)]
mod tests {
    #[test]
    fn b_asks_for_a_reference_or_drops_the_one_asked_for() {
        use super::asked_reference;
        let p = Some(archibim_model::Point::new(1.0, 2.0));
        assert_eq!(asked_reference(None, false), Some(None), "B with none");
        assert_eq!(
            asked_reference(Some(p), true),
            Some(None),
            "B after resting"
        );
        assert_eq!(asked_reference(Some(None), false), None, "B twice");
        assert_eq!(asked_reference(Some(p), false), None, "B after picking");
    }

    use super::*;
    use archibim_model::Story;

    fn pt(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn the_foot_of_a_perpendicular_needs_two_distinct_points() {
        let f = foot(pt(0.0, 0.0), pt(10.0, 0.0), pt(3.0, 5.0));
        assert_eq!(f, Some(pt(3.0, 0.0)));
        // Beyond the ends: still on the infinite line.
        assert_eq!(
            foot(pt(0.0, 0.0), pt(10.0, 0.0), pt(-30.0, 5.0)),
            Some(pt(-30.0, 0.0))
        );
        assert_eq!(foot(pt(1.0, 1.0), pt(1.0, 1.0), pt(3.0, 5.0)), None);
        assert_eq!(foot(pt(1.0, 1.0), pt(1.0 + 1e-10, 1.0), pt(3.0, 5.0)), None);
    }

    fn project_with(shapes: &[GuideShape]) -> Project {
        let mut p = Project::new();
        let story = p.new_id();
        let _ = p.insert_story(Story {
            id: story,
            name: String::new(),
            finished_level: 0.0,
            floor_build_up: 0.0,
        });
        for &shape in shapes {
            let id = p.new_id();
            let _ = p.insert_guide(Guide { id, story, shape });
        }
        p
    }

    #[test]
    fn a_guide_is_selected_on_its_line_or_circle_and_only_when_shown() {
        let p = project_with(&[
            GuideShape::Line {
                a: pt(0.0, 0.0),
                b: pt(1000.0, 0.0),
            },
            GuideShape::Circle {
                centre: pt(0.0, 5000.0),
                radius: 1000.0,
            },
        ]);
        assert_eq!(p.guides.len(), 2);
        let (line, circle) = (p.guides[0].id, p.guides[1].id);
        // The line is infinite: far past its points still hits.
        assert_eq!(hit(&p, &[], pt(9000.0, 3.0), 10.0, true), Some(line));
        assert_eq!(hit(&p, &[], pt(9000.0, 30.0), 10.0, true), None);
        // The circle: on it, not inside, not at the centre.
        assert_eq!(hit(&p, &[], pt(1000.0, 5004.0), 10.0, true), Some(circle));
        assert_eq!(hit(&p, &[], pt(0.0, 5000.0), 10.0, true), None);
        assert_eq!(hit(&p, &[], pt(500.0, 5000.0), 10.0, true), None);
        // Hidden guides are not selectable.
        assert_eq!(hit(&p, &[], pt(9000.0, 3.0), 10.0, false), None);
    }

    #[test]
    fn selecting_a_guide_tolerates_nothing_to_hit() {
        let p = project_with(&[]);
        assert_eq!(hit(&p, &[], pt(0.0, 0.0), 10.0, true), None);
    }
}
