//! Windows, tools and palettes of OPEN-ARCHIBIM. Turns user gestures into
//! commands; never changes the model directly.

mod i18n;
mod model_view;
mod plan;
mod properties;
mod snap;
mod wall_tool;

use archibim_commands::{Command, History};
use archibim_geometry::{
    LayerPlan, beam_outline, column_outline, contains, contour_at, opening_outline,
};
use archibim_model::{Beam, Column, Id, Opening, Point, Project, Slab, Wall};
use egui::{Key, Modifiers, Sense, Ui};

pub use i18n::t;
use model_view::ModelView;
use plan::PlanView;
use properties::fields;
pub use properties::{Element, Properties};
pub use snap::{Scene, Snap, SnapKind, SnapOptions};
use wall_tool::rectangle;
pub use wall_tool::{WallMethod, WallTool};

/// Snapping distance to existing points, in screen pixels.
const SNAP_PX: f32 = 10.0;

/// The elements each tool copies when it draws.
#[derive(Clone, Debug)]
pub struct Templates {
    /// Build-up, story and heights of new walls.
    pub wall: Wall,
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
        }
    }

    /// Shows the whole workspace. Returns true when the project changed.
    pub fn show(&mut self, ui: &mut Ui, history: &mut History, plan: &[LayerPlan]) -> bool {
        let mut changed = false;
        egui::Panel::left("tools").show(ui, |ui| self.tools(ui));
        egui::Panel::right("properties").show(ui, |ui| {
            if self.selection.is_none() && self.tool != Tool::Select {
                self.settings(ui);
            } else if let Some(command) =
                self.properties.show(ui, history.project(), self.selection)
            {
                changed |= self.run(history, command);
            }
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.label(self.message.as_deref().unwrap_or(""));
        });
        if self.dirty || self.model.stale(self.selection) {
            self.model.rebuild(history.project(), self.selection);
            self.dirty = false;
        }
        egui::CentralPanel::default().show(ui, |ui| match self.layout {
            Layout::Plan => changed |= self.plan(ui, history, plan),
            Layout::Model => {
                let typing = ui.ctx().memory(|m| m.focused().is_some());
                changed |= !typing && self.keys(ui, history, None);
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
        ] {
            ui.checkbox(on, t(key));
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
    /// of what it draws next.
    fn settings(&mut self, ui: &mut Ui) {
        ui.heading(t("settings.title"));
        ui.label(t("settings.method"));
        match self.tool {
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
            }
            Tool::Slab => {
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
        }
        ui.separator();
        let mut element = match self.tool {
            Tool::Wall => Element::Wall(self.wall.template.clone()),
            Tool::Slab => Element::Slab(self.templates.slab.clone()),
            Tool::Column => Element::Column(self.templates.column.clone()),
            Tool::Beam => Element::Beam(self.templates.beam.clone()),
            Tool::Opening if self.door => Element::Opening(self.templates.door.clone()),
            Tool::Opening | Tool::Select => Element::Opening(self.templates.window.clone()),
        };
        fields(ui, &mut element);
        // Settings are not part of the model: they change at once, with no
        // command and no undo.
        match element {
            Element::Wall(w) => self.wall.template = w,
            Element::Slab(s) => self.templates.slab = s,
            Element::Column(c) => self.templates.column = c,
            Element::Beam(b) => self.templates.beam = b,
            Element::Opening(o) if self.door => self.templates.door = o,
            Element::Opening(o) => self.templates.window = o,
        }
    }

    fn stop(&mut self) {
        self.wall.stop();
        self.slab_points.clear();
        self.beam_start = None;
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
        let drawing: Vec<Point> = self
            .wall
            .points()
            .iter()
            .chain(&self.slab_points)
            .chain(&self.beam_start)
            .copied()
            .collect();
        let snapped = response.hover_pos().map(|p| {
            Scene::new(history.project(), &drawing).snap(
                &self.snap_options,
                self.view.model(p),
                from,
                shift,
                snap,
            )
        });
        let cursor = snapped.as_ref().map(|s| s.point);

        let typing = ui.ctx().memory(|m| m.focused().is_some());
        let mut changed = !typing && self.keys(ui, history, cursor);
        // Space held: the tool takes the contour under the pointer.
        let space = !typing && ui.input(|i| i.key_down(Key::Space));
        let contour = match (space, self.tool, response.hover_pos()) {
            (true, Tool::Slab, Some(p)) => contour_at(
                history.project(),
                self.templates.slab.story,
                self.view.model(p),
            ),
            (true, Tool::Wall, Some(p)) => contour_at(
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
                        self.close_slab(history)
                    }
                };
            } else if let Some(p) = cursor {
                changed |= self.click(history, plan, p, snap);
            }
        }

        self.view
            .paint(&painter, history.project(), plan, self.selection);
        let rectangle_from = match self.tool {
            Tool::Wall if self.wall.method == WallMethod::Rectangle => self.wall.start(),
            Tool::Slab if self.slab_method == SlabMethod::Rectangle => {
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

    fn click(&mut self, history: &mut History, plan: &[LayerPlan], p: Point, snap: f64) -> bool {
        match self.tool {
            Tool::Select => {
                self.selection = hit(history.project(), plan, p);
                false
            }
            Tool::Wall => self.wall.click(history, p),
            Tool::Slab => {
                let closes = self.slab_points.len() >= 3
                    && self
                        .slab_points
                        .first()
                        .is_some_and(|q| distance(*q, p) <= snap);
                if self.slab_method == SlabMethod::Rectangle
                    && let Some(&a) = self.slab_points.first()
                {
                    self.slab_points = rectangle(a, p).unwrap_or_default();
                    self.close_slab(history)
                } else if closes {
                    self.close_slab(history)
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

    fn close_slab(&mut self, history: &mut History) -> bool {
        let outline = std::mem::take(&mut self.slab_points);
        let slab = Slab {
            id: history.new_id(),
            outline,
            ..self.templates.slab.clone()
        };
        self.run(history, Command::AddSlab(slab))
    }

    /// Keyboard: undo, redo, delete, escape, and the typed lengths.
    fn keys(&mut self, ui: &Ui, history: &mut History, cursor: Option<Point>) -> bool {
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
                egui::Event::Text(text) if self.tool == Tool::Wall => {
                    text.chars().for_each(|c| self.wall.type_char(c));
                }
                egui::Event::Key {
                    key, pressed: true, ..
                } => match key {
                    Key::Backspace => self.wall.erase(),
                    Key::Escape => {
                        self.stop();
                        self.selection = None;
                    }
                    Key::Enter if self.tool == Tool::Wall => {
                        changed |= self.wall.enter(history, cursor);
                    }
                    Key::Enter if self.tool == Tool::Slab => {
                        changed |= self.close_slab(history);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        changed
    }
}

fn distance(a: Point, b: Point) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// The element under `p`: openings first, then columns, beams, walls and
/// slabs.
fn hit(project: &Project, plan: &[LayerPlan], p: Point) -> Option<Id> {
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
    let wall = || wall_at(project, plan, p).map(|(id, _, _)| id);
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
        .or_else(wall)
        .or_else(slab)
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
