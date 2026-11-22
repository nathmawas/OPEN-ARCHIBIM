//! Pictures of the application taken by itself, with no access to the
//! screen: `archibim-app --capture <folder>` opens the demonstration house,
//! saves one picture per shot as PNG, and closes.

use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender, channel},
};

use archibim_model::{Id, Project};
use archibim_ui::{Layout, Workspace};
use eframe::egui::{self, ColorImage};

/// Frames drawn before each picture, so the views are fitted and the 3D
/// scene is built.
const SETTLE: u32 = 10;

/// One picture: what is selected and what the window shows.
struct Shot {
    name: &'static str,
    selection: Option<Id>,
    layout: Layout,
}

/// Whether a picture was written, or why not.
type Saved = Result<(), String>;

/// The pictures still to take, one after the other.
pub struct Capture {
    dir: PathBuf,
    shots: Vec<Shot>,
    next: usize,
    wait: u32,
    saved: (Sender<Saved>, Receiver<Saved>),
    pending: bool,
}

impl Capture {
    /// The pictures of the demonstration house: plan and 3D, then the
    /// roof, the stair and the terrain selected with their palette, then
    /// the 3D view alone.
    pub fn new(dir: PathBuf, project: &Project) -> Self {
        let first = |ids: Vec<Id>| ids.first().copied();
        let shots = vec![
            Shot {
                name: "01-plan-3d",
                selection: None,
                layout: Layout::Both,
            },
            Shot {
                name: "02-toiture",
                selection: first(project.roofs.iter().map(|e| e.id).collect()),
                layout: Layout::Both,
            },
            Shot {
                name: "03-escalier",
                selection: first(project.stairs.iter().map(|e| e.id).collect()),
                layout: Layout::Both,
            },
            Shot {
                name: "04-terrain",
                selection: first(project.meshes.iter().map(|e| e.id).collect()),
                layout: Layout::Both,
            },
            Shot {
                name: "05-3d",
                selection: None,
                layout: Layout::Model,
            },
        ];
        Self {
            dir,
            shots,
            next: 0,
            wait: SETTLE,
            saved: channel(),
            pending: false,
        }
    }

    /// Whether this frame should fit both views to the model, as Home does:
    /// a few frames into each shot, once the palette has taken its width.
    pub fn fits(&self) -> bool {
        !self.pending && self.wait == SETTLE / 2
    }

    /// Called every frame, after the workspace is drawn: sets up the next
    /// shot, asks for its picture, and closes the window after the last.
    pub fn frame(&mut self, ctx: &egui::Context, workspace: &mut Workspace) {
        ctx.request_repaint();
        if self.pending {
            match self.saved.1.try_recv() {
                Ok(Ok(())) => {
                    self.pending = false;
                    self.next += 1;
                    self.wait = SETTLE;
                }
                Ok(Err(e)) => {
                    eprintln!("{e}");
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Err(_) => {}
            }
            return;
        }
        let Some(shot) = self.shots.get(self.next) else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        };
        if self.wait == SETTLE {
            workspace.select(shot.selection);
            workspace.set_layout(shot.layout);
        }
        if self.wait > 0 {
            self.wait -= 1;
            return;
        }
        let path = self.dir.join(format!("{}.png", shot.name));
        let sender = self.saved.0.clone();
        self.pending = true;
        ctx.request_screenshot(move |image| {
            let done = save(&path, &image).map_err(|e| format!("{}: {e}", path.display()));
            if done.is_ok() {
                println!("{}", path.display());
            }
            // The window may already be closing: nothing left to tell.
            let _ = sender.send(done);
        });
    }
}

/// Writes a picture as PNG.
fn save(path: &Path, image: &ColorImage) -> Result<(), Box<dyn std::error::Error>> {
    let [w, h] = image.size;
    let mut encoder = png::Encoder::new(
        BufWriter::new(File::create(path)?),
        u32::try_from(w)?,
        u32::try_from(h)?,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(image.as_raw())?;
    Ok(())
}
