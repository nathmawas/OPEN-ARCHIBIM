//! OPEN-ARCHIBIM: assembles the crates into the application.
//!
//! Stage 0 prototype: opens a demonstration house on its ground floor plan,
//! with the base tools (select, wall, slab, column, beam, opening), the
//! properties palette and the 3D view.

mod demo;

use archibim_commands::History;
use archibim_geometry::{LayerPlan, Phase, plan_layers};
use archibim_model::Id;
use archibim_ui::Workspace;

struct App {
    history: History,
    story: Id,
    plan: Vec<LayerPlan>,
    workspace: Workspace,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        if self.workspace.show(ui, &mut self.history, &self.plan) {
            // ponytail: a plan that fails to compute keeps the last one;
            // show the error once the interface has a message area for it.
            if let Ok(plan) = plan_layers(self.history.project(), self.story, Phase::Proposed) {
                self.plan = plan;
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (history, story, templates) = demo::house()?;
    let plan = plan_layers(history.project(), story, Phase::Proposed)?;
    let app = App {
        history,
        story,
        plan,
        workspace: Workspace::new(templates),
    };
    eframe::run_native(
        "OPEN-ARCHIBIM",
        eframe::NativeOptions {
            depth_buffer: archibim_render::DEPTH_BITS,
            ..Default::default()
        },
        Box::new(|cc| {
            if let Some(state) = &cc.wgpu_render_state {
                archibim_render::install(state);
            }
            Ok(Box::new(app))
        }),
    )?;
    Ok(())
}
