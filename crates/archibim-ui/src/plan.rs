//! The plan view: navigation and drawing of the model.

use archibim_geometry::{
    LayerPlan, beam_outline, column_outline, mesh_contours, opening_outline, roof_faces,
    stair_steps, triangulate, visible_edges,
};
use archibim_model::{GuideShape, Id, LayerRole, Point, Project, Stair};
use egui::{Color32, Key, Mesh, Painter, Pos2, Rect, Response, Shape, Stroke, Ui, Vec2};

use crate::{Snap, SnapKind, WallTool};

const INK: Color32 = Color32::from_gray(40);
/// Colour of drawing guides.
const GUIDE: Color32 = Color32::from_rgb(220, 90, 40);
/// Colour of the guides placed by hand.
pub(crate) const PLACED: Color32 = Color32::from_rgb(0, 150, 140);
/// Colour of the reference point that typed values measure from.
const REFERENCE: Color32 = Color32::from_rgb(140, 60, 170);
pub(crate) const SELECTED: Color32 = Color32::from_rgb(0, 110, 200);
/// Colour of a selection area that takes the elements it touches.
const TOUCHED: Color32 = Color32::from_rgb(30, 140, 60);
const SLAB: Color32 = Color32::from_gray(235);
/// Fill of terrain meshes in plan.
const TERRAIN: Color32 = Color32::from_rgb(232, 240, 222);
/// Contour lines: one every interval, a stronger one every fifth.
// ponytail: fixed for every terrain; a setting of the terrain or the view,
// and the fill from the terrain's material, once materials carry an
// appearance.
const CONTOUR: f64 = 500.0;
/// Height of the plan cut above the foot of a stair: treads above it are
/// dashed.
// ponytail: one cut height for every stair; a setting of the story or the
// view once views have their own settings.
const STAIR_CUT: f64 = 1100.0;
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

    /// Fits the drawing in the view the next time it is shown.
    pub fn refit(&mut self) {
        self.scale = 0.0;
    }

    /// Zooms in by `factor` around the middle of the view, or out below 1.
    pub fn zoom(&mut self, factor: f32) {
        if self.scale > 0.0 {
            self.scale = (self.scale * factor).clamp(1e-4, 50.0);
        }
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

    pub(crate) fn screen(&self, p: Point) -> Pos2 {
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
        selection: &[Id],
    ) {
        painter.rect_filled(self.rect, 0.0, Color32::WHITE);
        let stroke = |id: Id| {
            if selection.contains(&id) {
                Stroke::new(2.5, SELECTED)
            } else {
                Stroke::new(1.0, INK)
            }
        };
        for mesh in &project.meshes {
            let outline: Vec<Point> = mesh.outline.iter().map(|p| p.at).collect();
            self.polygon(painter, &outline, TERRAIN, stroke(mesh.id));
            let level = project.level(&mesh.level).unwrap_or(0.0);
            for (h, a, b) in mesh_contours(mesh, level, CONTOUR) {
                let major = (h / CONTOUR).round() as i64 % 5 == 0;
                let width = if major { 1.2 } else { 0.5 };
                painter.line_segment([self.screen(a), self.screen(b)], Stroke::new(width, INK));
            }
        }
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
        for stair in &project.stairs {
            self.stair(painter, project, stair, stroke(stair.id));
        }
        // Roofs over everything, as lines: the overhang and the ridges,
        // hips and valleys where the planes meet; the wall line dashed.
        for roof in &project.roofs {
            let s = stroke(roof.id);
            for face in roof_faces(roof) {
                let points = face.polygon.iter().map(|p| self.screen(*p)).collect();
                painter.add(Shape::closed_line(points, s));
            }
            let mut line: Vec<Pos2> = roof.outline.iter().map(|p| self.screen(*p)).collect();
            if let Some(first) = line.first().copied() {
                line.push(first);
            }
            painter.extend(Shape::dashed_line(
                &line,
                Stroke::new(0.8, s.color),
                6.0,
                4.0,
            ));
            // The slope of each plane, in its middle.
            for face in roof_faces(roof).iter().filter(|f| f.slope > 0.0) {
                let n = face.polygon.len() as f64;
                let c = face.polygon.iter().fold(Point::new(0.0, 0.0), |c, p| {
                    Point::new(c.x + p.x / n, c.y + p.y / n)
                });
                self.label(
                    painter,
                    c,
                    format!("{:.0}°", face.slope.to_degrees()),
                    s.color,
                );
            }
            if selection.contains(&roof.id) {
                self.numbers(painter, &roof.outline, true);
            }
        }
        for mesh in project.meshes.iter().filter(|m| selection.contains(&m.id)) {
            let outline: Vec<Point> = mesh.outline.iter().map(|p| p.at).collect();
            self.numbers(painter, &outline, false);
        }
    }

    fn label(&self, painter: &Painter, at: Point, text: String, color: Color32) {
        painter.text(
            self.screen(at),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(12.0),
            color,
        );
    }

    /// Numbers from 1 on the edges (their middles) or the corners of an
    /// outline, as the properties palette lists them.
    fn numbers(&self, painter: &Painter, outline: &[Point], edges: bool) {
        let n = outline.len();
        for i in 0..n {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            let at = if edges {
                Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
            } else {
                a
            };
            let p = self.screen(at);
            painter.circle_filled(p, 8.0, SELECTED);
            painter.text(
                p,
                egui::Align2::CENTER_CENTER,
                (i + 1).to_string(),
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );
        }
    }

    /// A stair: its treads, dashed above the plan cut, and the walking line
    /// with an arrow pointing up the stair.
    fn stair(&self, painter: &Painter, project: &Project, stair: &Stair, stroke: Stroke) {
        let (Ok(bottom), Ok(top)) = (project.level(&stair.bottom), project.level(&stair.top))
        else {
            return;
        };
        let steps = stair_steps(stair, bottom, top);
        for (tread, h) in steps.treads.iter().zip(&steps.heights) {
            if *h <= bottom + STAIR_CUT {
                self.polygon(painter, tread, Color32::WHITE, stroke);
            } else {
                let mut points: Vec<Pos2> = tread.iter().map(|p| self.screen(*p)).collect();
                if let Some(first) = points.first().copied() {
                    points.push(first);
                }
                painter.extend(Shape::dashed_line(
                    &points,
                    Stroke::new(0.8, stroke.color),
                    5.0,
                    4.0,
                ));
            }
        }
        let line: Vec<Pos2> = stair.path.iter().map(|p| self.screen(*p)).collect();
        painter.add(Shape::line(line.clone(), Stroke::new(1.0, stroke.color)));
        if let [.., a, b] = line[..] {
            let d = (b - a).normalized() * 10.0;
            let side = Vec2::new(-d.y, d.x) * 0.5;
            painter.add(Shape::convex_polygon(
                vec![b, b - d + side, b - d - side],
                stroke.color,
                Stroke::NONE,
            ));
        }
        painter.circle_stroke(line[0], 4.0, Stroke::new(1.0, stroke.color));
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
    /// The contour the space bar would take, dashed.
    pub(crate) fn contour(&self, painter: &Painter, outline: &[Point]) {
        let mut points: Vec<Pos2> = outline.iter().map(|q| self.screen(*q)).collect();
        if let Some(&first) = points.first() {
            points.push(first);
        }
        painter.extend(Shape::dashed_line(
            &points,
            Stroke::new(2.0, GUIDE),
            8.0,
            4.0,
        ));
    }

    /// An area drawn with the selection tool: solid and blue when it takes
    /// the elements wholly inside, dashed and green when it takes those it
    /// touches.
    pub(crate) fn area(&self, painter: &Painter, area: &[Point], whole: bool) {
        let color = if whole { SELECTED } else { TOUCHED };
        let [r, g, b, _] = color.to_array();
        self.fill(painter, area, Color32::from_rgba_unmultiplied(r, g, b, 30));
        let mut points: Vec<Pos2> = area.iter().map(|q| self.screen(*q)).collect();
        if let Some(&first) = points.first() {
            points.push(first);
        }
        let stroke = Stroke::new(1.5, color);
        if whole {
            painter.add(Shape::line(points, stroke));
        } else {
            painter.extend(Shape::dashed_line(&points, stroke, 6.0, 4.0));
        }
    }

    /// The guides of a snap, dashed, and a mark telling what it found.
    pub(crate) fn snap_marks(&self, painter: &Painter, snap: &Snap) {
        let guide = Stroke::new(1.0, GUIDE);
        for &(a, b) in &snap.guides {
            painter.extend(Shape::dashed_line(
                &[self.screen(a), self.screen(b)],
                guide,
                6.0,
                4.0,
            ));
        }
        let c = self.screen(snap.point);
        let r = 5.0;
        let mark = Stroke::new(1.5, GUIDE);
        match snap.kind {
            SnapKind::Free | SnapKind::Direction | SnapKind::Parallel => {}
            SnapKind::Perpendicular => {
                // A right-angle mark.
                let corner = c + Vec2::new(-r, r);
                painter.add(Shape::line(
                    vec![c + Vec2::new(-r, -r), corner, c + Vec2::new(r, r)],
                    mark,
                ));
                painter.add(Shape::line(
                    vec![corner + Vec2::new(0.0, -r), c, corner + Vec2::new(r, 0.0)],
                    mark,
                ));
            }
            SnapKind::Point => {
                painter.rect_stroke(
                    Rect::from_center_size(c, Vec2::splat(2.0 * r)),
                    0.0,
                    mark,
                    egui::StrokeKind::Middle,
                );
            }
            SnapKind::Midpoint => {
                let points = vec![
                    c + Vec2::new(0.0, -r),
                    c + Vec2::new(r, r),
                    c + Vec2::new(-r, r),
                ];
                painter.add(Shape::closed_line(points, mark));
            }
            SnapKind::Intersection => {
                painter.line_segment([c + Vec2::splat(-r), c + Vec2::splat(r)], mark);
                painter.line_segment([c + Vec2::new(-r, r), c + Vec2::new(r, -r)], mark);
            }
            SnapKind::Tangent => {
                // A circle touched by a line along the tangent.
                let r = 1.5 * r;
                let d = snap
                    .guides
                    .first()
                    .map(|&(a, b)| (self.screen(b) - self.screen(a)).normalized())
                    .filter(|d| d.is_finite())
                    .unwrap_or(Vec2::X);
                let touch = c + d.rot90() * r;
                painter.circle_stroke(c, r, mark);
                painter.line_segment([touch - d * 2.0 * r, touch + d * 2.0 * r], mark);
            }
            SnapKind::Edge | SnapKind::Alignment | SnapKind::Guide => {
                painter.line_segment([c + Vec2::new(-r, 0.0), c + Vec2::new(r, 0.0)], mark);
                painter.line_segment([c + Vec2::new(0.0, -r), c + Vec2::new(0.0, r)], mark);
            }
        }
    }

    pub fn preview(
        &self,
        painter: &Painter,
        wall: &WallTool,
        slab: &[Point],
        beam: Option<Point>,
        rectangle: Option<Point>,
        cursor: Option<Point>,
    ) {
        let line = Stroke::new(1.5, SELECTED);
        let Some(p) = cursor else {
            return;
        };
        painter.circle_stroke(self.screen(p), 3.0, line);
        if let Some(a) = rectangle {
            let corners = [a, Point::new(a.x, p.y), p, Point::new(p.x, a.y)];
            let points = corners.iter().map(|q| self.screen(*q)).collect();
            painter.add(Shape::closed_line(points, line));
            return;
        }
        if let Some(shape) = wall.preview(p) {
            let points = shape.iter().map(|q| self.screen(*q)).collect();
            painter.add(Shape::line(points, line));
            return;
        }
        if let Some(start) = wall.start() {
            let end = p;
            painter.line_segment([self.screen(start), self.screen(end)], line);
            // The faces, placed from the clicked reference line, on the
            // side the wall will be built.
            let ghost = wall.ghost(start, end, 0.0);
            if let Some((a, b)) = ghost.left_face() {
                let l = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
                let k = ghost.thickness() / l;
                // The right of (dx, dy) is (dy, -dx).
                let r = |q: Point| Point::new(q.x + (b.y - a.y) * k, q.y - (b.x - a.x) * k);
                let outline = [a, b, r(b), r(a)].iter().map(|q| self.screen(*q)).collect();
                painter.add(Shape::closed_line(outline, Stroke::new(1.0, SELECTED)));
            }
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

impl PlanView {
    /// A guide placed by hand, dashed: a line across the whole view, or a
    /// circle.
    pub(crate) fn guide(&self, painter: &Painter, shape: &GuideShape, stroke: Stroke) {
        let points: Vec<Pos2> = match *shape {
            GuideShape::Line { a, b } => {
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let l = dx.hypot(dy);
                if l <= 0.0 || self.scale <= 0.0 {
                    return;
                }
                let (ux, uy) = (dx / l, dy / l);
                // From the point of the line nearest the middle of the view,
                // half the view's diagonal each way.
                let c = self.model(self.rect.center());
                let t = (c.x - a.x) * ux + (c.y - a.y) * uy;
                let half = f64::from(self.rect.size().length() / 2.0 / self.scale);
                let at = |s: f64| self.screen(Point::new(a.x + ux * s, a.y + uy * s));
                vec![at(t - half), at(t + half)]
            }
            GuideShape::Circle { centre, radius } => (0..=96)
                .map(|i| {
                    let a = f64::from(i) * std::f64::consts::TAU / 96.0;
                    self.screen(Point::new(
                        centre.x + radius * a.cos(),
                        centre.y + radius * a.sin(),
                    ))
                })
                .collect(),
        };
        painter.extend(Shape::dashed_line(&points, stroke, 10.0, 5.0));
    }

    /// The reference point that typed values measure from: a cross, and a
    /// dashed line to the point `p` about to be placed.
    pub(crate) fn reference(&self, painter: &Painter, r: Point, p: Option<Point>) {
        let c = self.screen(r);
        let mark = Stroke::new(1.5, REFERENCE);
        let k = 6.0;
        painter.line_segment([c + Vec2::splat(-k), c + Vec2::splat(k)], mark);
        painter.line_segment([c + Vec2::new(-k, k), c + Vec2::new(k, -k)], mark);
        painter.circle_stroke(c, k, mark);
        if let Some(p) = p {
            painter.extend(Shape::dashed_line(
                &[c, self.screen(p)],
                Stroke::new(1.0, REFERENCE),
                6.0,
                4.0,
            ));
        }
    }

    /// Next to the point `p` about to be placed, `label`: what was typed
    /// or the length and angle from the last point, in the warning colour
    /// when `warn` (typed values that give no point). While picking two
    /// points to go halfway, the first one and the middle instead.
    pub(crate) fn measure(
        &self,
        painter: &Painter,
        p: Point,
        label: &str,
        warn: bool,
        halfway: Option<&[Point]>,
    ) {
        if let Some(&a) = halfway.and_then(|h| h.first()) {
            let line = Stroke::new(1.5, GUIDE);
            painter.extend(Shape::dashed_line(
                &[self.screen(a), self.screen(p)],
                Stroke::new(1.0, GUIDE),
                6.0,
                4.0,
            ));
            painter.circle_stroke(self.screen(a), 6.0, line);
            let middle = self.screen(Point::new((a.x + p.x) / 2.0, (a.y + p.y) / 2.0));
            painter.circle_filled(middle, 5.0, GUIDE);
            painter.text(
                middle + Vec2::new(8.0, -8.0),
                egui::Align2::LEFT_BOTTOM,
                "1/2",
                egui::FontId::proportional(14.0),
                GUIDE,
            );
            return;
        }
        if label.is_empty() {
            return;
        }
        painter.text(
            self.screen(p) + Vec2::new(12.0, -12.0),
            egui::Align2::LEFT_BOTTOM,
            label,
            egui::FontId::proportional(14.0),
            if warn { GUIDE } else { SELECTED },
        );
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
