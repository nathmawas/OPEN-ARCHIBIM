//! OPEN-ARCHIBIM: assembles the crates into the application.
//!
//! Stage 0 prototype: opens a demonstration house on its ground floor plan,
//! with the base tools (select, wall, slab, column, beam, opening, roof,
//! stair, terrain), the properties palette and the 3D view.
//!
//! `archibim-app --capture <folder>` saves pictures of the house instead,
//! then closes.

mod capture;
mod demo;
#[cfg(test)]
mod selection;

use capture::Capture;

use archibim_commands::History;
use archibim_geometry::{LayerPlan, Phase, plan_layers};
use archibim_model::Id;
use archibim_ui::Workspace;

struct App {
    history: History,
    story: Id,
    plan: Vec<LayerPlan>,
    workspace: Workspace,
    capture: Option<Capture>,
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
        if let Some(capture) = &mut self.capture {
            capture.frame(ui.ctx(), &mut self.workspace);
        }
    }

    fn raw_input_hook(&mut self, ctx: &eframe::egui::Context, raw: &mut eframe::egui::RawInput) {
        self.workspace.filter_input(ctx, raw);
        if let Some(capture) = &mut self.capture {
            capture.input(&self.workspace, raw);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (history, story, templates) = demo::house()?;
    let plan = plan_layers(history.project(), story, Phase::Proposed)?;
    let args: Vec<String> = std::env::args().collect();
    let capture = match args.iter().position(|a| a == "--capture") {
        Some(i) => {
            let dir = std::path::PathBuf::from(args.get(i + 1).ok_or("--capture <folder>")?);
            std::fs::create_dir_all(&dir)?;
            Some(Capture::new(dir, history.project()))
        }
        None => None,
    };
    let mut viewport = eframe::egui::ViewportBuilder::default();
    if capture.is_some() {
        // The same size every time, so pictures compare from one round to
        // the next.
        viewport = viewport.with_inner_size([1600.0, 900.0]);
    }
    let app = App {
        history,
        story,
        plan,
        workspace: Workspace::new(templates),
        capture,
    };
    eframe::run_native(
        "OPEN-ARCHIBIM",
        eframe::NativeOptions {
            depth_buffer: archibim_render::DEPTH_BITS,
            viewport,
            ..Default::default()
        },
        Box::new(|cc| {
            archibim_ui::style(&cc.egui_ctx);
            if let Some(state) = &cc.wgpu_render_state {
                archibim_render::install(state);
            }
            Ok(Box::new(app))
        }),
    )?;
    Ok(())
}
