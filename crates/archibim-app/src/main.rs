//! OPEN-ARCHIBIM: assembles the crates into the application.
//!
//! Stage 0 prototype: opens a demonstration house, shows its ground floor
//! plan, and lets the user draw more walls with the same build-up.

mod demo;

use archibim_commands::History;
use archibim_geometry::{LayerPlan, Phase, plan_layers};
use archibim_model::Id;
use archibim_ui::PlanView;

struct App {
    history: History,
    story: Id,
    plan: Vec<LayerPlan>,
    view: PlanView,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        if self.view.show(ui, &mut self.history, &self.plan) {
            // ponytail: a plan that fails to compute keeps the last one;
            // show the error once the interface has a message area.
            if let Ok(plan) = plan_layers(self.history.project(), self.story, Phase::Proposed) {
                self.plan = plan;
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (history, story) = demo::house()?;
    let plan = plan_layers(history.project(), story, Phase::Proposed)?;
    let template = history
        .project()
        .walls
        .first()
        .cloned()
        .ok_or("the demonstration house has no wall")?;
    let app = App {
        history,
        story,
        plan,
        view: PlanView::new(template),
    };
    eframe::run_native(
        "OPEN-ARCHIBIM",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(app))),
    )?;
    Ok(())
}
