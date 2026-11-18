//! OPEN-ARCHIBIM: assembles the crates into the application.
//!
//! Stage 0 prototype: opens a demonstration house and shows its ground
//! floor plan, wall layers and junctions computed by the geometry crate.

mod demo;

use archibim_commands::History;
use archibim_geometry::{LayerPlan, Phase, plan_layers};
use archibim_ui::PlanView;

struct App {
    history: History,
    plan: Vec<LayerPlan>,
    view: PlanView,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        self.view.show(ui, self.history.project(), &self.plan);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (history, story) = demo::house()?;
    let plan = plan_layers(history.project(), story, Phase::Proposed)?;
    let app = App {
        history,
        plan,
        view: PlanView::default(),
    };
    eframe::run_native(
        "OPEN-ARCHIBIM",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(app))),
    )?;
    Ok(())
}
