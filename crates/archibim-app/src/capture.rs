//! Pictures of the application taken by itself, with no access to the
//! screen: `archibim-app --capture <folder>` opens the demonstration house,
//! saves one picture per shot as PNG, and closes.

use std::{
    collections::VecDeque,
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender, channel},
};

use archibim_model::{Id, Point, Project};
use archibim_ui::{Layout, Tool, Workspace};
use eframe::egui::{self, ColorImage, Event, PointerButton, RawInput};

/// Frames drawn before each picture, so the views are fitted, the pointer
/// has moved and the 3D scene is built.
const SETTLE: u32 = 16;

/// One picture: what is selected and what the window shows, and for a
/// drawing tool the points clicked and where the pointer then rests.
struct Shot {
    name: &'static str,
    selection: Option<Id>,
    layout: Layout,
    tool: Tool,
    clicks: Vec<Point>,
    hover: Option<Point>,
    /// A point the pointer rests on for a moment before the clicks.
    rest: Option<Point>,
    /// Keys typed after the pointer is in place.
    typed: &'static str,
}

impl Shot {
    fn new(name: &'static str, selection: Option<Id>, layout: Layout) -> Self {
        Self {
            name,
            selection,
            layout,
            tool: Tool::Select,
            clicks: Vec::new(),
            hover: None,
            rest: None,
            typed: "",
        }
    }
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
    /// Pointer events still to send, one frame each.
    script: VecDeque<Vec<Event>>,
    scripted: bool,
}

impl Capture {
    /// The pictures of the demonstration house: plan and 3D, then the
    /// roof, the stair and the terrain selected with their palette, then
    /// the 3D view alone.
    pub fn new(dir: PathBuf, project: &Project) -> Self {
        let first = |ids: Vec<Id>| ids.first().copied();
        let shots = vec![
            Shot::new("01-plan-3d", None, Layout::Both),
            Shot::new(
                "02-roof",
                first(project.roofs.iter().map(|e| e.id).collect()),
                Layout::Both,
            ),
            Shot::new(
                "03-stair",
                first(project.stairs.iter().map(|e| e.id).collect()),
                Layout::Both,
            ),
            Shot::new(
                "04-terrain",
                first(project.meshes.iter().map(|e| e.id).collect()),
                Layout::Both,
            ),
            // A wall started south-east of the garden wall, the pointer
            // near the point where a line from it touches the curve.
            Shot {
                tool: Tool::Wall,
                clicks: vec![Point::new(12000.0, -13000.0)],
                hover: Some(Point::new(4130.0, -12290.0)),
                ..Shot::new("05-tangent", None, Layout::Plan)
            },
            Shot::new("06-3d", None, Layout::Model),
            // A wall started on the guide line south of the house, the
            // pointer near where the line crosses the tree circle.
            Shot {
                tool: Tool::Wall,
                clicks: vec![Point::new(2000.0, -9500.0)],
                hover: Some(Point::new(10870.0, -9470.0)),
                ..Shot::new("07-guides", None, Layout::Plan)
            },
            // Resting on the south-east corner of the house makes it the
            // reference; 2000 typed with the pointer to the south-west
            // gives the point 2000 from that corner.
            Shot {
                tool: Tool::Wall,
                rest: Some(Point::new(10000.0, -8000.0)),
                hover: Some(Point::new(8000.0, -9800.0)),
                typed: "2000",
                ..Shot::new("08-reference", None, Layout::Plan)
            },
        ];
        Self {
            dir,
            shots,
            next: 0,
            wait: SETTLE,
            saved: channel(),
            pending: false,
            script: VecDeque::new(),
            scripted: false,
        }
    }

    /// Whether this frame should fit both views to the model, as Home does:
    /// a few frames into each shot, once the palette has taken its width.
    fn fits(&self) -> bool {
        !self.pending && self.wait == SETTLE / 2
    }

    /// Called before every frame: fits the views as Home does, then moves
    /// and clicks the pointer as the shot says, one step per frame.
    pub fn input(&mut self, workspace: &Workspace, raw: &mut RawInput) {
        let key = |key| Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        if self.fits() {
            raw.events.push(key(egui::Key::Home));
            self.scripted = false;
            return;
        }
        let Some(shot) = self.shots.get(self.next).filter(|_| !self.pending) else {
            return;
        };
        if !self.scripted && self.wait < SETTLE / 2 {
            self.scripted = true;
            let button = |pos, pressed| Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            if let Some(p) = shot.rest {
                self.script
                    .push_back(vec![Event::PointerMoved(workspace.screen_of(p))]);
                // Sixty empty frames of 10 ms each, so the rest counts.
                self.script.extend((0..60).map(|_| Vec::new()));
            }
            for &p in &shot.clicks {
                let pos = workspace.screen_of(p);
                self.script
                    .push_back(vec![Event::PointerMoved(pos), button(pos, true)]);
                self.script.push_back(vec![button(pos, false)]);
            }
            if let Some(p) = shot.hover {
                self.script
                    .push_back(vec![Event::PointerMoved(workspace.screen_of(p))]);
            }
            if !shot.typed.is_empty() {
                self.script
                    .push_back(vec![Event::Text(shot.typed.to_owned())]);
            }
        }
        if let Some(events) = self.script.pop_front() {
            if events.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            raw.events.extend(events);
        }
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
            workspace.set_tool(shot.tool);
            workspace.set_layout(shot.layout);
        }
        if self.wait > 0 || !self.script.is_empty() {
            self.wait = self.wait.saturating_sub(1);
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
