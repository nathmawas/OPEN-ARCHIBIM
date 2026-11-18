//! Windows, tools and palettes of OPEN-ARCHIBIM. Turns user gestures into
//! commands; never changes the model directly.

use archibim_geometry::LayerPlan;
use archibim_model::{LayerRole, Point, Project};
use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};

/// A plan view: draws the wall layers of a story, with pan and zoom.
#[derive(Clone, Debug, Default)]
pub struct PlanView {
    /// Screen pixels per millimetre; 0 until the view is fitted.
    scale: f32,
    /// Model point (in millimetres) at the centre of the view.
    centre: Vec2,
}

impl PlanView {
    /// Draws `plan` in the whole available space. Drag pans, the wheel
    /// zooms around the pointer, a double click fits the drawing.
    pub fn show(&mut self, ui: &mut Ui, project: &Project, plan: &[LayerPlan]) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, Color32::WHITE);

        if self.scale <= 0.0 || response.double_clicked() {
            self.fit(rect, plan);
        }
        if response.dragged() {
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
