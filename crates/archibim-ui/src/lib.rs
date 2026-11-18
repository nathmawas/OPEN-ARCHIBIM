//! Windows, tools and palettes of OPEN-ARCHIBIM. Turns user gestures into
//! commands; never changes the model directly.

mod wall_tool;

use archibim_commands::History;
use archibim_geometry::LayerPlan;
use archibim_model::{LayerRole, Point, Project, Wall};
use egui::{Color32, Key, Modifiers, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};

pub use wall_tool::WallTool;

/// Snapping distance to existing points, in screen pixels.
const SNAP_PX: f32 = 10.0;

/// A plan view: draws the wall layers of a story, with pan and zoom, and
/// lets the user draw walls.
#[derive(Clone, Debug)]
pub struct PlanView {
    /// Screen pixels per millimetre; 0 until the view is fitted.
    scale: f32,
    /// Model point (in millimetres) at the centre of the view.
    centre: Vec2,
    /// The wall tool, with the wall it draws.
    pub wall_tool: WallTool,
}

impl PlanView {
    /// A plan view whose wall tool draws copies of `template`.
    pub fn new(template: Wall) -> Self {
        Self {
            scale: 0.0,
            centre: Vec2::ZERO,
            wall_tool: WallTool::new(template),
        }
    }

    /// Draws `plan` in the whole available space and handles the user's
    /// gestures. Returns true when the project changed.
    ///
    /// - Middle or right drag pans, the wheel zooms around the pointer,
    ///   Home fits the drawing.
    /// - Click, click draws a wall; the next one starts where the last
    ///   ended. Typing a length then Enter sets it exactly. Shift keeps the
    ///   wall horizontal or vertical. Escape stops.
    /// - Ctrl+Z undoes, Ctrl+Y or Ctrl+Shift+Z redoes.
    pub fn show(&mut self, ui: &mut Ui, history: &mut History, plan: &[LayerPlan]) -> bool {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, Color32::WHITE);

        if self.scale <= 0.0 || ui.input(|i| i.key_pressed(Key::Home)) {
            self.fit(rect, plan);
        }
        if response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary)
        {
            self.centre -= response.drag_delta() / self.scale * Vec2::new(1.0, -1.0);
        }
        if let Some(pointer) = response.hover_pos() {
            let wheel = ui.input(|i| i.smooth_scroll_delta.y);
            let factor = (wheel * 0.002).exp() * ui.input(|i| i.zoom_delta());
            if factor != 1.0 {
                let before = self.to_model(rect, pointer);
                self.scale = (self.scale * factor).clamp(1e-4, 50.0);
                self.centre += before - self.to_model(rect, pointer);
            }
        }

        // The pointer in the model, snapped to wall ends and, with Shift,
        // to the horizontal or vertical of the wall start.
        let shift = ui.input(|i| i.modifiers.shift);
        let cursor = response.hover_pos().map(|p| {
            let p = self.to_model(rect, p);
            let mut p = Point::new(f64::from(p.x), f64::from(p.y));
            let snap = f64::from(SNAP_PX / self.scale);
            if let Some(end) = nearest_end(history.project(), p, snap) {
                p = end;
            } else if shift && let Some(start) = self.wall_tool.start() {
                p = orthogonal(start, p);
            }
            p
        });
        let mut changed = self.keys(ui, history, cursor);

        if response.clicked()
            && let Some(p) = cursor
        {
            changed |= self.wall_tool.click(history, p);
        }

        self.paint(&painter, rect, history.project(), plan);
        if let (Some(start), Some(p)) = (self.wall_tool.start(), cursor) {
            let end = self.wall_tool.typed_end(start, p).unwrap_or(p);
            let line = [self.to_screen(rect, start), self.to_screen(rect, end)];
            painter.line_segment(line, Stroke::new(1.5, Color32::from_rgb(0, 110, 200)));
            let length = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
            let label = match self.wall_tool.typed() {
                "" => format!("{length:.0}"),
                typed => typed.to_owned(),
            };
            painter.text(
                line[1] + Vec2::new(12.0, -12.0),
                egui::Align2::LEFT_BOTTOM,
                label,
                egui::FontId::proportional(14.0),
                Color32::from_rgb(0, 110, 200),
            );
        }
        if self.wall_tool.start().is_some() {
            ui.ctx().request_repaint();
        }
        changed
    }

    /// Keyboard: undo, redo, and the wall tool's typed length.
    fn keys(&mut self, ui: &Ui, history: &mut History, cursor: Option<Point>) -> bool {
        let (undo, redo) = ui.input_mut(|i| {
            let redo = i.consume_key(Modifiers::COMMAND, Key::Y)
                || i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
            (i.consume_key(Modifiers::COMMAND, Key::Z), redo)
        });
        let mut changed = false;
        if undo {
            self.wall_tool.stop();
            changed |= matches!(history.undo(), Ok(Some(_)));
        }
        if redo {
            self.wall_tool.stop();
            changed |= matches!(history.redo(), Ok(Some(_)));
        }
        let events = ui.input(|i| i.events.clone());
        for event in events {
            match event {
                egui::Event::Text(text) => text.chars().for_each(|c| self.wall_tool.type_char(c)),
                egui::Event::Key {
                    key, pressed: true, ..
                } => match key {
                    Key::Backspace => self.wall_tool.erase(),
                    Key::Escape => self.wall_tool.stop(),
                    Key::Enter => changed |= self.wall_tool.enter(history, cursor),
                    _ => {}
                },
                _ => {}
            }
        }
        changed
    }

    fn paint(&self, painter: &egui::Painter, rect: Rect, project: &Project, plan: &[LayerPlan]) {
        let outline = Stroke::new(1.0, Color32::from_gray(40));
        for layer in plan {
            let role = project
                .wall(layer.wall)
                .ok()
                .and_then(|w| w.layers.get(layer.layer))
                .map(|l| l.role);
            let fill = role.map_or(Color32::LIGHT_GRAY, fill_of);
            // ponytail: convex fills only; wall pieces are quadrilaterals
            // today. Triangulate when a piece can be concave.
            let pieces = layer.polygons.iter().map(|p| (p, fill));
            let members = layer.members.iter().map(|p| (p, MEMBER));
            for (polygon, fill) in pieces.chain(members) {
                let points = polygon.iter().map(|p| self.to_screen(rect, *p)).collect();
                painter.add(Shape::convex_polygon(points, fill, outline));
            }
        }
    }

    fn fit(&mut self, rect: Rect, plan: &[LayerPlan]) {
        let points = plan.iter().flat_map(|l| &l.polygons).flatten();
        let mut bounds = Rect::NOTHING;
        for p in points {
            bounds.extend_with(Pos2::new(p.x as f32, p.y as f32));
        }
        if !bounds.is_positive() {
            self.scale = 0.05;
            return;
        }
        self.centre = bounds.center().to_vec2();
        self.scale = 0.9 * (rect.width() / bounds.width()).min(rect.height() / bounds.height());
    }

    fn to_screen(&self, rect: Rect, p: Point) -> Pos2 {
        let d = Vec2::new(p.x as f32, p.y as f32) - self.centre;
        rect.center() + Vec2::new(d.x, -d.y) * self.scale
    }

    fn to_model(&self, rect: Rect, p: Pos2) -> Vec2 {
        let d = (p - rect.center()) / self.scale;
        self.centre + Vec2::new(d.x, -d.y)
    }
}

/// The wall end closest to `p`, if closer than `tolerance`.
// ponytail: scans every wall; use a spatial index when projects get large.
fn nearest_end(project: &Project, p: Point, tolerance: f64) -> Option<Point> {
    let distance = |q: &Point| ((q.x - p.x).powi(2) + (q.y - p.y).powi(2)).sqrt();
    project
        .walls
        .iter()
        .flat_map(|w| [w.start, w.end])
        .filter(|q| distance(q) <= tolerance)
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
}

/// `p` moved onto the horizontal or the vertical through `start`, whichever
/// is closer.
fn orthogonal(start: Point, p: Point) -> Point {
    if (p.x - start.x).abs() >= (p.y - start.y).abs() {
        Point::new(p.x, start.y)
    } else {
        Point::new(start.x, p.y)
    }
}

/// Fill colour of framing members (studs, joists).
const MEMBER: Color32 = Color32::from_rgb(160, 110, 60);

/// Fill colour of a layer by its constructive role.
// ponytail: fixed colours per role; replaced by the material's own
// appearance once materials carry parameters.
fn fill_of(role: LayerRole) -> Color32 {
    match role {
        LayerRole::LoadBearing => Color32::from_rgb(190, 120, 100),
        LayerRole::Insulation => Color32::from_rgb(250, 225, 120),
        LayerRole::AirGap => Color32::WHITE,
        LayerRole::Membrane => Color32::from_rgb(90, 90, 160),
        LayerRole::Lining => Color32::from_rgb(210, 180, 140),
        LayerRole::Finish => Color32::from_rgb(225, 225, 225),
        LayerRole::Covering => Color32::from_rgb(150, 150, 150),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_keeps_the_wall_square() {
        let s = Point::new(0.0, 0.0);
        assert_eq!(
            orthogonal(s, Point::new(1000.0, 30.0)),
            Point::new(1000.0, 0.0)
        );
        assert_eq!(
            orthogonal(s, Point::new(-30.0, 900.0)),
            Point::new(0.0, 900.0)
        );
    }
}
