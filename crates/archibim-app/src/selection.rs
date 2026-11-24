//! The selection tool driven on the demonstration house with no window:
//! stands for tests/cases/interface/selection-areas-and-modifiers.yaml,
//! which the case runner of archibim-commands cannot reach.

use archibim_commands::History;
use archibim_geometry::{LayerPlan, Phase, plan_layers};
use archibim_model::{Id, Point};
use archibim_ui::{Layout, Tool, Workspace};
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

/// The workspace on the demonstration house, one frame at a time.
struct Bench {
    ctx: egui::Context,
    h: History,
    story: Id,
    plan: Vec<LayerPlan>,
    ws: Workspace,
    time: f64,
}

impl Bench {
    fn new() -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let (h, story, templates) = crate::demo::house()?;
        let plan = plan_layers(h.project(), story, Phase::Proposed)?;
        let mut ws = Workspace::new(templates);
        ws.set_layout(Layout::Plan);
        let mut bench = Self {
            ctx: egui::Context::default(),
            h,
            story,
            plan,
            ws,
            time: 0.0,
        };
        // The panels take their width and the plan fits the house.
        for _ in 0..5 {
            bench.frame(Vec::new(), Modifiers::NONE);
        }
        Ok(bench)
    }

    fn frame(&mut self, events: Vec<Event>, modifiers: Modifiers) {
        self.time += 1.0 / 60.0;
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 900.0))),
            time: Some(self.time),
            events: std::iter::once(Event::ModifiersChanged(modifiers))
                .chain(events)
                .collect(),
            ..RawInput::default()
        };
        let (ws, h, plan) = (&mut self.ws, &mut self.h, &self.plan);
        let mut changed = false;
        let mut out = self.ctx.run_ui(raw, |ui| changed = ws.show(ui, h, plan));
        out.textures_delta.clear();
        if changed && let Ok(plan) = plan_layers(self.h.project(), self.story, Phase::Proposed) {
            self.plan = plan;
        }
    }

    fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        }
    }

    fn click(&mut self, p: Point, modifiers: Modifiers) {
        let pos = self.ws.screen_of(p);
        let press = vec![Event::PointerMoved(pos), Self::button(pos, true, modifiers)];
        self.frame(press, modifiers);
        self.frame(vec![Self::button(pos, false, modifiers)], modifiers);
        self.frame(Vec::new(), modifiers);
    }

    fn drag(&mut self, from: Point, to: Point, modifiers: Modifiers) {
        let (a, b) = (self.ws.screen_of(from), self.ws.screen_of(to));
        let press = vec![Event::PointerMoved(a), Self::button(a, true, modifiers)];
        self.frame(press, modifiers);
        for i in 1..=10 {
            let p = a + (b - a) * (i as f32 / 10.0);
            self.frame(vec![Event::PointerMoved(p)], modifiers);
        }
        self.frame(vec![Self::button(b, false, modifiers)], modifiers);
        self.frame(Vec::new(), modifiers);
    }

    fn key(&mut self, key: Key) {
        let event = |pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![event(true)], Modifiers::NONE);
        self.frame(vec![event(false)], Modifiers::NONE);
    }

    /// The middle of the first piece of wall the plan draws for `wall`.
    fn on(&self, wall: Id) -> Option<Point> {
        let poly = self
            .plan
            .iter()
            .find(|l| l.wall == wall)?
            .polygons
            .first()?;
        let n = poly.len() as f64;
        Some(poly.iter().fold(Point::new(0.0, 0.0), |c, p| {
            Point::new(c.x + p.x / n, c.y + p.y / n)
        }))
    }
}

#[test]
fn selection_areas_and_modifiers() -> Result {
    let mut b = Bench::new()?;
    let walls: Vec<Id> = b.h.project().walls.iter().map(|w| w.id).collect();
    let terrain =
        b.h.project()
            .meshes
            .first()
            .map(|m| m.id)
            .ok_or("terrain")?;
    // The two partitions: north to south at x 4000, west to east at y -4500.
    let (a, c) = (walls[4], walls[5]);
    let (shift, ctrl, alt) = (Modifiers::SHIFT, Modifiers::COMMAND, Modifiers::ALT);

    // Escape stops the wall being drawn, then goes back to selecting,
    // then empties the selection.
    b.ws.set_tool(Tool::Wall);
    b.click(Point::new(7000.0, -9000.0), Modifiers::NONE);
    b.key(Key::Escape);
    assert_eq!(b.ws.tool(), Tool::Wall, "the first Escape stops drawing");
    b.key(Key::Escape);
    assert_eq!(b.ws.tool(), Tool::Select);
    b.ws.select(Some(a));
    b.key(Key::Escape);
    assert!(b.ws.selection().is_empty());

    // A click selects alone; Shift adds, Ctrl takes out.
    let (on_a, on_c) = (b.on(a).ok_or("a")?, b.on(c).ok_or("c")?);
    b.click(on_a, Modifiers::NONE);
    assert_eq!(b.ws.selection(), [a]);
    b.click(on_c, shift);
    assert_eq!(b.ws.selection(), [a, c]);
    b.click(on_a, ctrl);
    assert_eq!(b.ws.selection(), [c]);
    b.click(on_c, Modifiers::NONE);
    assert_eq!(b.ws.selection(), [c]);

    // Right to left, across the north-south partition only: touched, with
    // the terrain the area lies on; clear of the door in it.
    b.drag(
        Point::new(4300.0, -3000.0),
        Point::new(3700.0, -3400.0),
        Modifiers::NONE,
    );
    let s = b.ws.selection();
    assert!(s.contains(&a) && s.contains(&terrain), "{s:?}");
    assert!(!s.contains(&c) && !s.contains(&walls[0]), "{s:?}");

    // Alt across both partitions: the one selected goes, the other comes.
    b.ws.select(Some(a));
    b.drag(
        Point::new(4600.0, -4000.0),
        Point::new(3400.0, -5000.0),
        alt,
    );
    let s = b.ws.selection();
    assert!(!s.contains(&a) && s.contains(&c), "{s:?}");

    // Left to right around the house: every wall, wholly inside, not the
    // terrain that reaches further.
    let (mut lo, mut hi) = (
        Point::new(f64::MAX, f64::MAX),
        Point::new(f64::MIN, f64::MIN),
    );
    for p in b.plan.iter().flat_map(|l| &l.polygons).flatten() {
        (lo.x, lo.y, hi.x, hi.y) = (lo.x.min(p.x), lo.y.min(p.y), hi.x.max(p.x), hi.y.max(p.y));
    }
    let (mx, my) = ((hi.x - lo.x) * 0.03, (hi.y - lo.y) * 0.03);
    b.drag(
        Point::new(lo.x - mx, hi.y + my),
        Point::new(hi.x + mx, lo.y - my),
        Modifiers::NONE,
    );
    let s = b.ws.selection().to_vec();
    assert!(walls.iter().all(|w| s.contains(w)), "{s:?}");
    assert!(!s.contains(&terrain));

    // Delete removes them all, their openings with them, in one undo step.
    let (n_walls, n_openings) = (b.h.project().walls.len(), b.h.project().openings.len());
    assert!(n_openings > 0);
    b.key(Key::Delete);
    assert!(b.h.project().walls.is_empty());
    assert!(b.h.project().openings.is_empty());
    assert!(b.ws.selection().is_empty());
    b.h.undo()?;
    assert_eq!(b.h.project().walls.len(), n_walls);
    assert_eq!(b.h.project().openings.len(), n_openings);
    Ok(())
}
