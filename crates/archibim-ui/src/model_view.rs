//! The 3D view: an orbiting camera around the model, drawn on the GPU.

use std::sync::Arc;

use archibim_geometry::{Phase, model_blocks};
use archibim_model::{Id, Project};
use archibim_render::{Camera, Frame, Scene};
use egui::{Color32, Key, PointerButton, Sense, Ui};

use crate::plan::{CONCRETE, SELECTED, fill_of};

const BACKGROUND: Color32 = Color32::from_gray(225);
// ponytail: one colour for every terrain, until materials carry an
// appearance.
const TERRAIN: Color32 = Color32::from_rgb(170, 190, 140);

/// The camera and the triangles of the model, rebuilt when it changes.
#[derive(Debug, Default)]
pub struct ModelView {
    camera: Camera,
    scene: Arc<Scene>,
    /// The selection the scene was coloured for.
    coloured_for: Option<Id>,
}

impl ModelView {
    /// Rebuilds the triangles from the model, the selection highlighted.
    pub fn rebuild(&mut self, project: &Project, selection: Option<Id>) {
        // ponytail: a model that fails to compute shows nothing; show the
        // error once the interface has a message area for it.
        let blocks = model_blocks(project, Phase::Proposed).unwrap_or_default();
        let coloured: Vec<_> = blocks
            .iter()
            .map(|p| {
                let color = if selection == Some(p.element) {
                    SELECTED
                } else if project.meshes.iter().any(|m| m.id == p.element) {
                    TERRAIN
                } else {
                    p.role.map_or(CONCRETE, fill_of)
                };
                (
                    p,
                    [color.r(), color.g(), color.b()].map(|c| f32::from(c) / 255.0),
                )
            })
            .collect();
        self.scene = Arc::new(Scene::new(self.scene.generation + 1, &coloured));
        self.coloured_for = selection;
        if self.camera.distance <= 0.0 {
            self.camera.fit(&self.scene);
        }
    }

    /// Whether the scene was coloured for another selection.
    pub fn stale(&self, selection: Option<Id>) -> bool {
        self.scene.generation == 0 || self.coloured_for != selection
    }

    /// Left drag turns around the model, middle or right drag pans, the
    /// wheel zooms, Home shows everything.
    // ponytail: no picking in 3D yet, no edges, no section box.
    pub fn show(&mut self, ui: &mut Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::drag());
        let rect = response.rect;
        let c = &mut self.camera;
        let delta = response.drag_delta();
        if response.dragged_by(PointerButton::Primary) {
            c.yaw -= delta.x * 0.008;
            c.pitch = (c.pitch + delta.y * 0.008).clamp(-1.5, 1.5);
        }
        if response.dragged_by(PointerButton::Middle)
            || response.dragged_by(PointerButton::Secondary)
        {
            let (right, up) = c.right_up();
            let k = c.mm_per_point(rect.height());
            for i in 0..3 {
                c.target[i] += (-right[i] * delta.x + up[i] * delta.y) * k;
            }
        }
        if response.hovered() {
            let wheel = ui.input(|i| i.smooth_scroll_delta.y);
            c.distance /= (wheel * 0.002).exp() * ui.input(|i| i.zoom_delta());
        }
        if ui.input(|i| i.key_pressed(Key::Home)) {
            c.fit(&self.scene);
        }
        painter.rect_filled(rect, 0.0, BACKGROUND);
        painter.add(
            Frame {
                scene: self.scene.clone(),
                camera: self.camera,
                aspect: rect.width() / rect.height().max(1.0),
            }
            .shape(rect),
        );
    }
}
