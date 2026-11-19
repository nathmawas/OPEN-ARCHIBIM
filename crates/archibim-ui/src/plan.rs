//! The plan view: navigation and drawing of the model.

use archibim_geometry::{
    LayerPlan, beam_outline, column_outline, opening_outline, triangulate, visible_edges,
};
use archibim_model::{Id, LayerRole, Point, Project};
use egui::{Color32, Key, Mesh, Painter, Pos2, Rect, Response, Shape, Stroke, Ui, Vec2};

use crate::WallTool;

const INK: Color32 = Color32::from_gray(40);
pub(crate) const SELECTED: Color32 = Color32::from_rgb(0, 110, 200);
const SLAB: Color32 = Color32::from_gray(235);
pub(crate) const CONCRETE: Color32 = Color32::from_gray(150);
/// Fill colour of framing members (studs, joists).
const MEMBER: Color32 = Color32::from_rgb(160, 110, 60);

/// Pan, zoom and the transform between the model and the screen.
#[derive(Clone, Debug)]
pub struct PlanView {
    /// Screen pixels per millimetre; 0 until the view is fitted.
    scale: f32,
    /// Model point (in millimetres) at the centre of the view.
    centre: Vec2,
    /// Where the view is on screen.
    rect: Rect,
}

impl Default for PlanView {
    fn default() -> Self {
        Self {
            scale: 0.0,
            centre: Vec2::ZERO,
            rect: Rect::NOTHING,
        }
    }
}

impl PlanView {
    /// Screen pixels per millimetre.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Middle or right drag pans, the wheel zooms around the pointer, Home
    /// fits the drawing.
    pub fn navigate(&mut self, ui: &Ui, response: &Response, plan: &[LayerPlan]) {
        self.rect = response.rect;
        if self.scale <= 0.0 || ui.input(|i| i.key_pressed(Key::Home)) {
            self.fit(plan);
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
                let before = self.model(pointer);
                self.scale = (self.scale * factor).clamp(1e-4, 50.0);
                let after = self.model(pointer);
                self.centre += Vec2::new((before.x - after.x) as f32, (before.y - after.y) as f32);
            }
        }
    }

    fn fit(&mut self, plan: &[LayerPlan]) {
        let mut bounds = Rect::NOTHING;
        for p in plan.iter().flat_map(|l| &l.polygons).flatten() {
            bounds.extend_with(Pos2::new(p.x as f32, p.y as f32));
        }
        if !bounds.is_positive() {
            self.scale = 0.05;
            return;
        }
        self.centre = bounds.center().to_vec2();
        self.scale =
            0.9 * (self.rect.width() / bounds.width()).min(self.rect.height() / bounds.height());
    }

    fn screen(&self, p: Point) -> Pos2 {
        let d = Vec2::new(p.x as f32, p.y as f32) - self.centre;
        self.rect.center() + Vec2::new(d.x, -d.y) * self.scale
    }

    /// The model point under a screen point.
    pub fn model(&self, p: Pos2) -> Point {
        let d = (p - self.rect.center()) / self.scale;
        let m = self.centre + Vec2::new(d.x, -d.y);
        Point::new(f64::from(m.x), f64::from(m.y))
    }

    fn fill(&self, painter: &Painter, outline: &[Point], fill: Color32) {
        let mut mesh = Mesh::default();
        for p in outline {
            mesh.colored_vertex(self.screen(*p), fill);
        }
        for [a, b, c] in triangulate(outline) {
            mesh.add_triangle(a as u32, b as u32, c as u32);
        }
        painter.add(Shape::mesh(mesh));
    }

    fn polygon(&self, painter: &Painter, outline: &[Point], fill: Color32, stroke: Stroke) {
        self.fill(painter, outline, fill);
        let points: Vec<Pos2> = outline.iter().map(|p| self.screen(*p)).collect();
        painter.add(Shape::closed_line(points, stroke));
    }

    /// Draws slabs, wall layers, openings, columns and beams, the selected
    /// element highlighted.
    pub fn paint(
        &self,
        painter: &Painter,
        project: &Project,
        plan: &[LayerPlan],
        selection: Option<Id>,
    ) {
        painter.rect_filled(self.rect, 0.0, Color32::WHITE);
        let stroke = |id: Id| {
            if selection == Some(id) {
                Stroke::new(2.5, SELECTED)
            } else {
                Stroke::new(1.0, INK)
            }
        };
        for slab in &project.slabs {
            self.polygon(painter, &slab.outline, SLAB, stroke(slab.id));
        }
        // Fills first, then only the edges between different materials, so
        // equal materials run on through junctions without a seam.
        let mut pieces: Vec<(Id, &[Point])> = Vec::new();
        let mut owners = Vec::new();
        for layer in plan {
            let found = project
                .wall(layer.wall)
                .ok()
                .and_then(|w| w.layers.get(layer.layer));
            let fill = found.map_or(Color32::LIGHT_GRAY, |l| fill_of(l.role));
            let material = found.map_or(Id(u64::MAX), |l| l.material);
            for polygon in &layer.polygons {
                self.fill(painter, polygon, fill);
                pieces.push((material, polygon));
                owners.push(layer.wall);
            }
        }
        for (index, a, b) in visible_edges(&pieces) {
            painter.line_segment([self.screen(a), self.screen(b)], stroke(owners[index]));
        }
        for member in plan.iter().flat_map(|l| &l.members) {
            self.polygon(painter, member, MEMBER, Stroke::new(1.0, INK));
        }
        for opening in &project.openings {
            self.opening(painter, project, opening.id, stroke(opening.id));
        }
        for column in &project.columns {
            self.polygon(
                painter,
                &column_outline(column),
                CONCRETE,
                stroke(column.id),
            );
        }
        for beam in &project.beams {
            if let Some(outline) = beam_outline(beam) {
                let mut points: Vec<Pos2> = outline.iter().map(|p| self.screen(*p)).collect();
                if let Some(first) = points.first().copied() {
                    points.push(first);
                }
                painter.extend(Shape::dashed_line(&points, stroke(beam.id), 8.0, 5.0));
            }
        }
    }

    /// The symbol of an opening: a frame with its glass for a window, a leaf
    /// and its swing for a door (an opening down to the floor).
    // ponytail: one symbol per kind; drawn from the parts and their
    // operation once the opening tool is detailed.
    fn opening(&self, painter: &Painter, project: &Project, id: Id, stroke: Stroke) {
        let (Ok(outline), Ok(o)) = (opening_outline(project, id), project.opening(id)) else {
            return;
        };
        let [a, b, c, d] = [outline[0], outline[1], outline[2], outline[3]];
        let mid = |p: Point, q: Point| Point::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0);
        let s = |p: Point| self.screen(p);
        let thin = Stroke::new(1.0, stroke.color);
        painter.line_segment([s(a), s(d)], stroke);
        painter.line_segment([s(b), s(c)], stroke);
        if o.sill > 0.0 {
            painter.line_segment([s(mid(a, d)), s(mid(b, c))], thin);
            painter.line_segment([s(a), s(b)], thin);
            painter.line_segment([s(d), s(c)], thin);
        } else {
            // Hinge on the far face at the start side, leaf opened square
            // to the wall, swing back to the other jamb.
            let width = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
            let along = Point::new((b.x - a.x) / width, (b.y - a.y) / width);
            let out = Point::new(along.y, -along.x);
            let leaf = Point::new(d.x + out.x * width, d.y + out.y * width);
            painter.line_segment([s(d), s(leaf)], stroke);
            let arc: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let angle = f64::from(i) / 16.0 * std::f64::consts::FRAC_PI_2;
                    let (sin, cos) = angle.sin_cos();
                    s(Point::new(
                        d.x + width * (out.x * cos + along.x * sin),
                        d.y + width * (out.y * cos + along.y * sin),
                    ))
                })
                .collect();
            painter.add(Shape::line(arc, thin));
        }
    }

    /// The element being drawn, from its last point to the pointer.
    pub fn preview(
        &self,
        painter: &Painter,
        wall: &WallTool,
        slab: &[Point],
        beam: Option<Point>,
        cursor: Option<Point>,
    ) {
        let line = Stroke::new(1.5, SELECTED);
        let Some(p) = cursor else {
            return;
        };
        painter.circle_stroke(self.screen(p), 3.0, line);
        if let Some(start) = wall.start() {
            let end = wall.typed_end(start, p).unwrap_or(p);
            painter.line_segment([self.screen(start), self.screen(end)], line);
            let length = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
            let label = match wall.typed() {
                "" => format!("{length:.0}"),
                typed => typed.to_owned(),
            };
            painter.text(
                self.screen(end) + Vec2::new(12.0, -12.0),
                egui::Align2::LEFT_BOTTOM,
                label,
                egui::FontId::proportional(14.0),
                SELECTED,
            );
        }
        if !slab.is_empty() {
            let mut points: Vec<Pos2> = slab.iter().map(|q| self.screen(*q)).collect();
            points.push(self.screen(p));
            painter.add(Shape::line(points, line));
        }
        if let Some(start) = beam {
            painter.line_segment([self.screen(start), self.screen(p)], line);
        }
    }
}

/// Fill colour of a layer by its constructive role.
// ponytail: fixed colours per role; replaced by the material's own
// appearance once materials carry parameters.
pub(crate) fn fill_of(role: LayerRole) -> Color32 {
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
