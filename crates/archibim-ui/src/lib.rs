//! Windows, tools and palettes of OPEN-ARCHIBIM. Turns user gestures into
//! commands; never changes the model directly.

mod i18n;
mod look;
mod model_view;
mod plan;
mod properties;
mod select;
mod snap;
mod typed;
mod wall_tool;

use archibim_commands::{Command, History};
use archibim_geometry::{
    LayerPlan, beam_outline, column_outline, contains, contour_at, cuts_through, floor_contour_at,
    opening_outline, roof_faces, roof_overhang, stair_steps,
};
use archibim_model::{
    Beam, Column, Guide, GuideShape, Id, Layer, Mesh, MeshPoint, Opening, Point, Project, Roof,
    Slab, Stair, Story, Wall,
};
use egui::{Key, Modifiers, Sense, Ui};

pub use i18n::t;
pub use look::style;
use look::{icon_button, icon_row, icons};
use model_view::ModelView;
use plan::PlanView;
use properties::fields;
pub use properties::{Element, Properties};
pub use select::SelectMethod;
use select::{Combine, caught_in, removal};
pub use snap::{BASE_ANGLES, PICKED_EDGES, Scene, Snap, SnapKind, SnapOptions};
use typed::Typed;
use wall_tool::rectangle;
pub use wall_tool::{WallMethod, WallTool};

/// How long the pointer rests on a point, before the first one is placed,
/// for that point to become the reference, in seconds.
const REST: f64 = 0.4;

/// How much one press of the zoom buttons zooms.
const ZOOM: f32 = 1.25;

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
    /// Space and click with the slab tool takes the floor, on the bearing
    /// walls, rather than the room.
    floor_contour: bool,
    /// The outline being closed came from the floor contour.
    joining: bool,
    beam_start: Option<Point>,
    /// Each beam starts where the last one ended.
    beam_chain: bool,
    door: bool,
    /// The selected elements, in the order they were picked.
    selection: Vec<Id>,
    select_method: SelectMethod,
    /// Polygons and lassos take the elements wholly inside, not those
    /// touched.
    select_whole: bool,
    /// The rectangle or lasso being dragged with the selection tool: its
    /// first corner and the pointer, or the points of its line.
    dragging: Option<Vec<Point>>,
    properties: Properties,
    message: Option<String>,
    snap_options: SnapOptions,
    /// Edges the pointer rested on, most recent first, for guide lines.
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
    /// The edge the pointer stops on, where, and since when, before it is
    /// picked up.
    edge_resting: Option<((Point, Point), Point, f64)>,
    /// The drawing aids bar under the menu is shown.
    show_aids: bool,
    /// The tools panel on the left is shown.
    show_tools: bool,
    /// The settings panel on the right is shown.
    show_settings: bool,
    /// Where the next point goes, under the pointer in the plan.
    pointer: Option<Point>,
    /// Quitting was asked for while the project has changes.
    ask_quit: bool,
    /// Quitting without saving was confirmed.
    quit: bool,
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
            floor_contour: true,
            joining: false,
            beam_start: None,
            beam_chain: false,
            door: false,
            selection: Vec::new(),
            select_method: SelectMethod::Rectangle,
            select_whole: false,
            dragging: None,
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
            edge_resting: None,
            show_aids: true,
            show_tools: true,
            show_settings: true,
            pointer: None,
            ask_quit: false,
            quit: false,
        }
    }

    /// Lets the window close without asking, for the pictures the
    /// application takes of itself.
    pub fn quit_without_asking(&mut self) {
        self.quit = true;
    }

    /// Selects an element, as a click with the selection tool would, or
    /// nothing.
    pub fn select(&mut self, id: Option<Id>) {
        self.tool = Tool::Select;
        self.selection = id.into_iter().collect();
    }

    /// The selected elements, in the order they were picked.
    pub fn selection(&self) -> &[Id] {
        &self.selection
    }

    /// The active tool.
    pub fn tool(&self) -> Tool {
        self.tool
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
        self.quitting(ui, history);
        egui::Panel::top("menu").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| changed |= self.menu(ui, history));
        });
        if self.show_aids {
            egui::Panel::top("aids").show(ui, |ui| {
                ui.horizontal_wrapped(|ui| self.aids(ui));
            });
        }
        if self.show_tools {
            egui::Panel::left("tools").show(ui, |ui| self.tools(ui));
        }
        if self.show_settings {
            egui::Panel::right("properties").show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.tool == Tool::Select {
                        self.select_settings(ui);
                        ui.separator();
                    }
                    if self.selection.is_empty() && self.tool != Tool::Select {
                        if let Some(command) = self.settings(ui, history.project()) {
                            changed |= self.run(history, command);
                        }
                    } else if self.selection.len() > 1 {
                        // ponytail: a count and Delete only; editing a shared
                        // parameter on all of them (TOOLS.md rule 3) comes next.
                        ui.heading(t("props.title"));
                        ui.label(format!("{} {}", self.selection.len(), t("select.count")));
                        if ui.button(t("action.delete")).clicked() {
                            changed |= self.delete(history);
                        }
                    } else if let Some(command) =
                        self.properties
                            .show(ui, history.project(), self.selection.first().copied())
                    {
                        changed |= self.run(history, command);
                    }
                });
            });
        }
        egui::Panel::bottom("display").show(ui, |ui| self.display(ui));
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
                Tool::Select => t(match self.select_method {
                    SelectMethod::Rectangle => "select.rectangle_hint",
                    SelectMethod::Polygon => "select.polygon_hint",
                    SelectMethod::Lasso => "select.lasso_hint",
                }),
                _ => t("draw.hint"),
            };
            ui.horizontal_wrapped(|ui| {
                if let Some(p) = self.pointer {
                    ui.monospace(format!("x {:>8.0}  y {:>8.0}", p.x, p.y));
                    ui.separator();
                }
                ui.label(match (pending, self.message.as_deref()) {
                    (Some(pending), _) => pending,
                    (None, Some(message)) => message,
                    (None, None) => hint,
                });
            });
        });
        if self.dirty || self.model.stale(&self.selection) {
            self.model.rebuild(history.project(), &self.selection);
            self.dirty = false;
        }
        egui::CentralPanel::default().show(ui, |ui| match self.layout {
            Layout::Plan => changed |= self.plan(ui, history, plan),
            Layout::Model => {
                self.pointer = None;
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
        self.selection
            .retain(|&id| Element::find(history.project(), id).is_some());
        changed
    }

    /// The menu bar. Returns true when the project changed.
    fn menu(&mut self, ui: &mut Ui, history: &mut History) -> bool {
        let ctx = ui.ctx().clone();
        let keys =
            |modifiers, key| ctx.format_shortcut(&egui::KeyboardShortcut::new(modifiers, key));
        let (ctrl, shift) = (Modifiers::COMMAND, Modifiers::COMMAND | Modifiers::SHIFT);
        let item = |key, shortcut: String| egui::Button::new(t(key)).shortcut_text(shortcut);
        let mut changed = false;
        ui.menu_button(t("menu.file"), |ui| {
            // ponytail: nothing is saved yet (V1.0); the entries show where
            // it will be.
            for (key, shortcut) in [
                ("menu.new", keys(ctrl, Key::N)),
                ("menu.open", keys(ctrl, Key::O)),
                ("menu.save", keys(ctrl, Key::S)),
                ("menu.save_as", keys(shift, Key::S)),
                ("menu.export_ifc", String::new()),
            ] {
                ui.add_enabled(false, item(key, shortcut))
                    .on_disabled_hover_text(t("menu.not_yet"));
            }
            ui.separator();
            if ui.add(item("menu.quit", keys(ctrl, Key::Q))).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button(t("menu.edit"), |ui| {
            if ui
                .add_enabled(history.can_undo(), item("menu.undo", keys(ctrl, Key::Z)))
                .clicked()
            {
                changed |= self.undo(history, false);
            }
            if ui
                .add_enabled(history.can_redo(), item("menu.redo", keys(ctrl, Key::Y)))
                .clicked()
            {
                changed |= self.undo(history, true);
            }
            ui.separator();
            let delete = item("menu.delete", keys(Modifiers::NONE, Key::Delete));
            if ui.add_enabled(!self.selection.is_empty(), delete).clicked() {
                changed |= self.delete(history);
            }
        });
        ui.menu_button(t("menu.view"), |ui| {
            if ui
                .add(item("view.fit", keys(Modifiers::NONE, Key::Home)))
                .clicked()
            {
                self.fit();
            }
            if ui.button(t("view.zoom_in")).clicked() {
                self.zoom(ZOOM);
            }
            if ui.button(t("view.zoom_out")).clicked() {
                self.zoom(1.0 / ZOOM);
            }
            ui.separator();
            self.layouts(ui);
            ui.separator();
            ui.checkbox(&mut self.show_guides, t("snap.placed"));
            ui.checkbox(&mut self.show_aids, t("view.aids"));
            ui.checkbox(&mut self.show_tools, t("view.tools"));
            ui.checkbox(&mut self.show_settings, t("view.settings"));
        });
        ui.menu_button(t("menu.help"), |ui| {
            if ui.button(t("menu.about")).clicked() {
                self.message = Some(format!("{} {}", t("about.text"), env!("CARGO_PKG_VERSION")));
            }
        });
        changed
    }

    /// Closing the window while the project has changes asks first:
    /// nothing is saved yet.
    fn quitting(&mut self, ui: &Ui, history: &History) {
        if ui.input(|i| i.viewport().close_requested()) && !self.quit && history.can_undo() {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.ask_quit = true;
        }
        if !self.ask_quit {
            return;
        }
        let modal = egui::Modal::new(egui::Id::unique("quit")).show(ui.ctx(), |ui| {
            ui.label(t("quit.unsaved"));
            ui.horizontal(|ui| {
                if ui.button(t("quit.anyway")).clicked() {
                    self.quit = true;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if ui.button(t("quit.cancel")).clicked() {
                    self.ask_quit = false;
                }
            });
        });
        if modal.should_close() {
            self.ask_quit = false;
        }
    }

    /// Undoes the last command, or redoes the last undone one.
    fn undo(&mut self, history: &mut History, redo: bool) -> bool {
        self.stop();
        let done = if redo { history.redo() } else { history.undo() };
        matches!(done, Ok(Some(_)))
    }

    /// Deletes the selected elements, in one undo step.
    fn delete(&mut self, history: &mut History) -> bool {
        match removal(history.project(), &self.selection) {
            Some(command) => self.run(history, command),
            None => false,
        }
    }

    /// Shows the whole drawing, in the plan and in 3D.
    fn fit(&mut self) {
        self.view.refit();
        self.model.fit();
    }

    fn zoom(&mut self, factor: f32) {
        self.view.zoom(factor);
        self.model.zoom(factor);
    }

    fn layouts(&mut self, ui: &mut Ui) {
        for (layout, key) in [
            (Layout::Plan, "view.plan"),
            (Layout::Model, "view.model"),
            (Layout::Both, "view.both"),
        ] {
            ui.selectable_value(&mut self.layout, layout, t(key));
        }
    }

    /// The display bar at the bottom: what the middle shows, and how close.
    fn display(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            self.layouts(ui);
            ui.separator();
            if ui.button(t("view.fit")).clicked() {
                self.fit();
            }
            if ui.button("-").on_hover_text(t("view.zoom_out")).clicked() {
                self.zoom(1.0 / ZOOM);
            }
            if ui.button("+").on_hover_text(t("view.zoom_in")).clicked() {
                self.zoom(ZOOM);
            }
            if self.layout != Layout::Model
                && let Some(n) = screen_scale(self.view.scale())
            {
                ui.label(format!("{} 1:{n}", t("view.screen_scale")))
                    .on_hover_text(t("view.screen_scale_hint"));
            }
            ui.separator();
            ui.checkbox(&mut self.show_guides, t("snap.placed"));
        });
    }

    /// The tools, in groups: selection, the elements of the model, guides;
    /// each one its icon and its name.
    fn tools(&mut self, ui: &mut Ui) {
        let before = self.tool;
        let groups: [&[(Tool, &str, &str)]; 3] = [
            &[(Tool::Select, "tool.select", icons::SELECT)],
            &[
                (Tool::Wall, "tool.wall", icons::WALL),
                (Tool::Column, "tool.column", icons::COLUMN),
                (Tool::Beam, "tool.beam", icons::BEAM),
                (Tool::Slab, "tool.slab", icons::SLAB),
                (Tool::Roof, "tool.roof", icons::ROOF),
                (Tool::Stair, "tool.stair", icons::STAIR),
                (Tool::Opening, "tool.opening", icons::WINDOW),
                (Tool::Mesh, "tool.mesh", icons::TERRAIN),
            ],
            &[(Tool::Guide, "tool.guide", icons::GUIDE)],
        ];
        ui.spacing_mut().item_spacing.y = 2.0;
        for (i, group) in groups.into_iter().enumerate() {
            if i > 0 {
                ui.separator();
            }
            for &(tool, key, icon) in group {
                if icon_row(ui, self.tool == tool, icon, t(key)).clicked() {
                    self.tool = tool;
                }
                if tool == Tool::Opening && self.tool == Tool::Opening {
                    ui.indent("opening", |ui| {
                        for (door, key, icon) in [
                            (false, "preset.window", icons::WINDOW),
                            (true, "preset.door", icons::DOOR),
                        ] {
                            if icon_row(ui, self.door == door, icon, t(key)).clicked() {
                                self.door = door;
                            }
                        }
                    });
                }
            }
        }
        if self.tool != before {
            self.stop();
        }
    }

    /// The drawing aids, in a row: what the pointer snaps to, and the
    /// points placed from others. Each aid is an icon, its name on hover.
    fn aids(&mut self, ui: &mut Ui) {
        let gap = || egui::Separator::default().spacing(4.0);
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.strong(t("snap.title"));
        let o = &mut self.snap_options;
        for (on, icon, key) in [
            (&mut o.points, icons::SNAP_POINTS, "snap.points"),
            (&mut o.midpoints, icons::SNAP_MIDPOINTS, "snap.midpoints"),
            (
                &mut o.intersections,
                icons::SNAP_INTERSECTIONS,
                "snap.intersections",
            ),
            (&mut o.edges, icons::SNAP_EDGES, "snap.edges"),
            (&mut o.angles, icons::SNAP_ANGLES, "snap.angles"),
            (&mut o.alignments, icons::SNAP_ALIGNMENTS, "snap.alignments"),
            (&mut o.guides, icons::SNAP_GUIDES, "snap.guides"),
        ] {
            if icon_button(ui, *on, icon, t(key)).clicked() {
                *on = !*on;
            }
        }
        ui.add(gap());
        for (on, angle) in o.base_angles.iter_mut().zip(BASE_ANGLES) {
            ui.toggle_value(on, format!("{angle}°"))
                .on_hover_text(t("snap.step_hint"));
        }
        ui.add(
            egui::DragValue::new(&mut o.other_angle)
                .range(0..=180)
                .prefix(format!("{} ", t("snap.other_short")))
                .suffix("°"),
        )
        .on_hover_text(t("snap.other"));
        ui.add(gap());
        ui.add(
            egui::DragValue::new(&mut o.divisions)
                .range(2..=12)
                .prefix(format!("{} ", t("snap.divisions_short"))),
        )
        .on_hover_text(t("snap.divisions"));
        if self.tool != Tool::Select {
            ui.add(gap());
        }
        if self.tool != Tool::Select
            && icon_button(
                ui,
                self.halfway.is_some(),
                icons::SNAP_HALFWAY,
                t("snap.halfway"),
            )
            .clicked()
        {
            self.halfway = match self.halfway {
                Some(_) => None,
                None => Some(Vec::new()),
            };
        }
        if self.tool != Tool::Select
            && icon_button(
                ui,
                self.reference.is_some(),
                icons::SNAP_REFERENCE,
                t("snap.reference"),
            )
            .clicked()
        {
            self.ask_reference();
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
                if self.tool == Tool::Slab {
                    ui.separator();
                    ui.label(t("contour.space"));
                    ui.selectable_value(&mut self.floor_contour, true, t("contour.floor"))
                        .on_hover_text(t("contour.floor_hint"));
                    ui.selectable_value(&mut self.floor_contour, false, t("contour.room"))
                        .on_hover_text(t("contour.room_hint"));
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
        // Before drawing, the side swap stays on for the next walls.
        fields(ui, &mut element, project, &mut self.wall.swapped);
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
        self.dragging = None;
    }

    /// Points are placed, or an area is being dragged.
    fn pending(&self) -> bool {
        !self.wall.points().is_empty()
            || !self.slab_points.is_empty()
            || self.beam_start.is_some()
            || self.dragging.is_some()
    }

    /// The settings of the selection tool: how it draws an area, and what
    /// a polygon or a lasso takes.
    fn select_settings(&mut self, ui: &mut Ui) {
        ui.heading(t("settings.title"));
        ui.label(t("select.method"));
        let before = self.select_method;
        for (method, key) in [
            (SelectMethod::Rectangle, "method.rectangle"),
            (SelectMethod::Polygon, "method.polygon"),
            (SelectMethod::Lasso, "method.lasso"),
        ] {
            ui.selectable_value(&mut self.select_method, method, t(key));
        }
        if self.select_method != before {
            self.stop();
        }
        if self.select_method != SelectMethod::Rectangle {
            ui.separator();
            ui.selectable_value(&mut self.select_whole, false, t("select.touched"));
            ui.selectable_value(&mut self.select_whole, true, t("select.whole"));
        }
        ui.separator();
        ui.label(t("select.modifiers"));
    }

    /// The area being drawn with the selection tool, closed, up to the
    /// pointer `p`, and whether it takes only the elements wholly inside.
    fn select_area(&self, p: Option<Point>) -> Option<(Vec<Point>, bool)> {
        if self.tool != Tool::Select {
            return None;
        }
        match (self.select_method, &self.dragging) {
            // Left to right takes the elements wholly inside.
            (SelectMethod::Rectangle, Some(points)) => {
                let (&a, &b) = (points.first()?, points.last()?);
                let corners = vec![a, Point::new(a.x, b.y), b, Point::new(b.x, a.y)];
                Some((corners, b.x >= a.x))
            }
            (SelectMethod::Lasso, Some(points)) => Some((points.clone(), self.select_whole)),
            (SelectMethod::Polygon, _) if !self.slab_points.is_empty() => {
                let mut points = self.slab_points.clone();
                points.extend(p);
                Some((points, self.select_whole))
            }
            _ => None,
        }
    }

    /// Selects with the area, as `how` says.
    fn select_with(
        &mut self,
        project: &Project,
        plan: &[LayerPlan],
        area: Option<(Vec<Point>, bool)>,
        how: Combine,
    ) {
        if let Some((area, whole)) = area {
            let caught = caught_in(project, plan, &area, whole, self.show_guides);
            how.apply(&mut self.selection, &caught);
        }
    }

    /// The selection tool in the plan: a click picks the element under the
    /// pointer, or a corner of the polygon; a drag draws
    /// a rectangle or a lasso, which selects when let go.
    fn select_gesture(
        &mut self,
        ui: &Ui,
        response: &egui::Response,
        project: &Project,
        plan: &[LayerPlan],
        snap: f64,
    ) {
        let how = Combine::from(ui.input(|i| i.modifiers));
        let p = response.hover_pos().map(|p| self.view.model(p));
        if response.clicked()
            && let Some(p) = response.interact_pointer_pos().map(|p| self.view.model(p))
        {
            if self.select_method != SelectMethod::Polygon {
                let caught: Vec<Id> = hit(project, plan, p, snap, self.show_guides)
                    .into_iter()
                    .collect();
                how.apply(&mut self.selection, &caught);
            } else if self.slab_points.len() >= 3
                && self
                    .slab_points
                    .first()
                    .is_some_and(|q| distance(*q, p) <= snap)
            {
                let area = self.select_area(None);
                self.slab_points.clear();
                self.select_with(project, plan, area, how);
            } else {
                self.slab_points.push(p);
            }
        }
        if self.select_method == SelectMethod::Polygon {
            return;
        }
        // ponytail: a drag draws an area wherever it starts, even on an
        // element: the terrain or a slab lies under most of a plan, so
        // "from empty space" would rarely happen. Once selected elements
        // move by dragging, a drag that starts on one moves it instead.
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            self.dragging = Some(vec![self.view.model(origin)]);
        }
        let lasso = self.select_method == SelectMethod::Lasso;
        if let (Some(points), Some(p)) = (&mut self.dragging, p) {
            match points.last() {
                // ponytail: a lasso point every half snap distance keeps
                // the fill and the catch quick; finer if lassos look jagged.
                Some(&q) if lasso && distance(q, p) < snap / 2.0 => {}
                Some(_) if !lasso && points.len() > 1 => {
                    let last = points.len() - 1;
                    points[last] = p;
                }
                _ => points.push(p),
            }
        }
        if response.drag_stopped() {
            let area = self.select_area(None);
            self.dragging = None;
            self.select_with(project, plan, area, how);
        }
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
            // The edge the pointer stops on for a moment is picked up for
            // guide lines; one passed over, or slid along, is not.
            let now = ui.input(|i| i.time);
            match scene
                .edge_near(p, snap)
                .filter(|e| !self.picked.contains(e))
            {
                Some(edge) => match self.edge_resting {
                    Some((e, at, since))
                        if e == edge && (at.x - p.x).hypot(at.y - p.y) < snap / 4.0 =>
                    {
                        if now - since >= REST {
                            self.picked.insert(0, edge);
                            self.picked.truncate(PICKED_EDGES);
                            self.edge_resting = None;
                        }
                    }
                    _ => {
                        self.edge_resting = Some((edge, p, now));
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_secs_f64(REST));
                    }
                },
                None => self.edge_resting = None,
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
        self.pointer = cursor;

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
        let floor = self.tool == Tool::Slab && self.floor_contour;
        // The floor contour was found, not the room or slab taken instead.
        let mut on_bearing = false;
        let contour = match (space, outline_story, self.tool, response.hover_pos()) {
            (true, Some(story), _, Some(p)) if floor => {
                // On the walls of its story, or else of the story below,
                // which an upper floor bears on.
                let project = history.project();
                let level = |s: &Story| s.finished_level;
                let below = project
                    .story(story)
                    .ok()
                    .and_then(|s| {
                        project
                            .stories
                            .iter()
                            .filter(|o| level(o) < level(s))
                            .max_by(|a, b| level(a).total_cmp(&level(b)))
                    })
                    .map(|s| s.id);
                let at = self.view.model(p);
                let found = floor_contour_at(project, story, at)
                    .or_else(|| below.and_then(|b| floor_contour_at(project, b, at)));
                on_bearing = found.is_some();
                found.or_else(|| contour_at(project, story, at))
            }
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
        if self.tool == Tool::Select {
            self.select_gesture(ui, &response, history.project(), plan, snap);
        } else if response.clicked() {
            if let Some(outline) = contour.clone() {
                changed |= match self.tool {
                    Tool::Wall => self.wall.outline(history, outline),
                    _ => {
                        self.slab_points = outline;
                        self.joining = on_bearing;
                        self.close_outline(history)
                    }
                };
            } else if let Some(p) = cursor {
                changed |= self.place(history, plan, p, snap);
            }
        }

        // While an area is drawn, the plan shows what letting go selects.
        let area = self.select_area(response.hover_pos().map(|p| self.view.model(p)));
        let mut shown = self.selection.clone();
        if let Some((points, whole)) = &area {
            let caught = caught_in(history.project(), plan, points, *whole, self.show_guides);
            Combine::from(ui.input(|i| i.modifiers)).apply(&mut shown, &caught);
        }
        self.view.paint(&painter, history.project(), plan, &shown);
        if let Some((points, whole)) = &area {
            self.view.area(&painter, points, *whole);
        }
        if self.show_guides {
            for g in &history.project().guides {
                let stroke = if shown.contains(&g.id) {
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
        // The selection polygon is drawn as an area, above.
        let outline: &[Point] = match self.tool {
            Tool::Select => &[],
            _ => &self.slab_points,
        };
        self.view.preview(
            &painter,
            &self.wall,
            outline,
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
            let offset = Some(from.map_or((p.x, p.y), |a| (p.x - a.x, p.y - a.y)));
            let label = self.typed.label(live, offset);
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
            // Its clicks go to `select_gesture`.
            Tool::Select => false,
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
            _ => {
                let slab = Slab {
                    id,
                    outline,
                    ..self.templates.slab.clone()
                };
                if std::mem::take(&mut self.joining) {
                    match joined_slab(history.project(), slab) {
                        Ok(command) => command,
                        Err(e) => {
                            self.message = Some(format!("{} {e}", t("status.refused")));
                            return false;
                        }
                    }
                } else {
                    Command::AddSlab(slab)
                }
            }
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
        let (undo, redo, delete, quit) = ui.input_mut(|i| {
            let redo = i.consume_key(Modifiers::COMMAND, Key::Y)
                || i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
            let undo = i.consume_key(Modifiers::COMMAND, Key::Z);
            let quit = i.consume_key(Modifiers::COMMAND, Key::Q);
            (undo, redo, i.key_pressed(Key::Delete), quit)
        });
        let mut changed = false;
        if undo || redo {
            changed |= self.undo(history, redo);
        }
        if delete {
            changed |= self.delete(history);
        }
        if quit {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
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
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => match key {
                    Key::Backspace => self.typed.erase(),
                    // Escape first forgets what was typed, then the halfway
                    // or reference point being picked, then stops drawing,
                    // then goes back to the selection tool, then empties
                    // the selection.
                    Key::Escape if !self.typed.is_empty() => self.typed.clear(),
                    Key::Escape if self.halfway.is_some() || self.reference.is_some() => {
                        self.halfway = None;
                        self.reference = None;
                    }
                    // A parallel guide: back to picking its edge.
                    Key::Escape if self.guide_edge.is_some() => self.guide_edge = None,
                    Key::Escape if self.pending() => self.stop(),
                    Key::Escape if self.tool != Tool::Select => self.set_tool(Tool::Select),
                    Key::Escape => self.selection.clear(),
                    // Enter closes the selection polygon.
                    Key::Enter if self.tool == Tool::Select && self.slab_points.len() >= 3 => {
                        let area = self.select_area(None);
                        self.slab_points.clear();
                        self.select_with(history.project(), plan, area, Combine::from(modifiers));
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

/// The scale the plan shows at on screen, 1:N, from the view's zoom in
/// screen points per millimetre, rounded to two figures; none before the
/// view is fitted. Not the drawing's scale.
// ponytail: takes a screen point as 1/96 inch, as the system's display
// scaling means it; measure the screen if architects find it off.
fn screen_scale(points_per_mm: f32) -> Option<u32> {
    if points_per_mm <= 0.0 {
        return None;
    }
    let n = 96.0 / 25.4 / points_per_mm;
    let unit = 10f32.powf(n.log10().floor() - 1.0).max(1.0);
    Some(((n / unit).round() * unit) as u32)
}

/// A new slab with the walls, columns and beams it passes through joined
/// to it, in one undo step.
fn joined_slab(project: &Project, slab: Slab) -> Result<Command, archibim_model::ModelError> {
    let id = slab.id;
    let mut after = project.clone();
    archibim_commands::apply(&mut after, Command::AddSlab(slab.clone()))?;
    let through = cuts_through(&after, id)?;
    if through.is_empty() {
        return Ok(Command::AddSlab(slab));
    }
    let joins = through.into_iter().map(|element| Command::SetCut {
        element,
        by: id,
        on: true,
    });
    Ok(Command::Group(
        std::iter::once(Command::AddSlab(slab))
            .chain(joins)
            .collect(),
    ))
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
    fn the_screen_scale_reads_as_a_rounded_one_to_n() {
        use super::screen_scale;
        // 1:100: 1 mm of the model is 1/100 mm on screen, 96 points to 25.4 mm.
        let at = |n: f32| screen_scale(96.0 / 25.4 / n);
        assert_eq!(at(100.0), Some(100));
        assert_eq!(at(52.0), Some(52));
        assert_eq!(at(1234.0), Some(1200));
        assert_eq!(at(3.0), Some(3));
        assert_eq!(screen_scale(0.0), None, "not fitted yet");
    }

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
