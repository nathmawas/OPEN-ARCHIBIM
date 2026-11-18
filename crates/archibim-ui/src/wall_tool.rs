//! The wall tool: click, click, with lengths typed at the keyboard.

use archibim_commands::{Command, History};
use archibim_model::{Point, Wall};

/// Draws walls one after the other, each starting where the last ended.
#[derive(Clone, Debug)]
pub struct WallTool {
    /// The wall to copy: build-up, story, bottom and top.
    pub template: Wall,
    start: Option<Point>,
    typed: String,
}

impl WallTool {
    /// A wall tool drawing copies of `template`.
    pub fn new(template: Wall) -> Self {
        Self {
            template,
            start: None,
            typed: String::new(),
        }
    }

    /// Where the wall being drawn starts, if one is.
    pub fn start(&self) -> Option<Point> {
        self.start
    }

    /// The length typed so far, in millimetres.
    pub fn typed(&self) -> &str {
        &self.typed
    }

    /// A click: starts a wall, or ends it there and starts the next one.
    /// Returns true when a wall was added.
    pub fn click(&mut self, history: &mut History, p: Point) -> bool {
        let added = match self.start {
            Some(start) => self.add(history, start, self.typed_end(start, p).unwrap_or(p)),
            None => {
                self.start = Some(p);
                false
            }
        };
        self.typed.clear();
        added
    }

    /// Enter: ends the wall at the typed length, towards the pointer.
    pub fn enter(&mut self, history: &mut History, toward: Option<Point>) -> bool {
        let (Some(start), Some(toward)) = (self.start, toward) else {
            return false;
        };
        let Some(end) = self.typed_end(start, toward) else {
            return false;
        };
        self.typed.clear();
        self.add(history, start, end)
    }

    /// The end of a wall of the typed length, from `start` towards `toward`.
    pub fn typed_end(&self, start: Point, toward: Point) -> Option<Point> {
        let length: f64 = self.typed.replace(',', ".").parse().ok()?;
        let (dx, dy) = (toward.x - start.x, toward.y - start.y);
        let norm = (dx * dx + dy * dy).sqrt();
        (length > 0.0 && norm > 0.0)
            .then(|| Point::new(start.x + dx / norm * length, start.y + dy / norm * length))
    }

    /// A typed character: digits and a decimal separator build the length.
    pub fn type_char(&mut self, c: char) {
        if self.start.is_some() && (c.is_ascii_digit() || c == '.' || c == ',') {
            self.typed.push(c);
        }
    }

    /// Backspace: erases the last typed character.
    pub fn erase(&mut self) {
        self.typed.pop();
    }

    /// Escape: stops drawing.
    pub fn stop(&mut self) {
        self.start = None;
        self.typed.clear();
    }

    fn add(&mut self, history: &mut History, start: Point, end: Point) -> bool {
        let length = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
        if length < 1.0 {
            return false;
        }
        let id = history.new_id();
        let wall = Wall {
            id,
            start,
            end,
            ..self.template.clone()
        };
        // ponytail: a refused wall is silently dropped; show the reason
        // once the interface has a message area.
        let added = history.run(Command::AddWall(wall)).is_ok();
        self.start = Some(end);
        added
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archibim_model::{
        Layer, LayerRole, LevelKind, LevelRef, Material, Project, RenovationStatus, Story,
    };

    type Result = std::result::Result<(), Box<dyn std::error::Error>>;

    fn setup() -> std::result::Result<(History, WallTool), Box<dyn std::error::Error>> {
        let mut h = History::new(Project::new());
        let (story, material, wall) = (h.new_id(), h.new_id(), h.new_id());
        h.run(Command::AddStory(Story {
            id: story,
            name: String::new(),
            finished_level: 0.0,
            floor_build_up: 0.0,
        }))?;
        h.run(Command::AddMaterial(Material {
            id: material,
            name: String::new(),
            priority: 5,
        }))?;
        let level = |offset| LevelRef {
            story,
            kind: LevelKind::Finished,
            offset,
        };
        let template = Wall {
            id: wall,
            story,
            start: Point::new(0.0, 0.0),
            end: Point::new(1.0, 0.0),
            layers: vec![Layer {
                role: LayerRole::LoadBearing,
                material,
                thickness: 200.0,
                status: RenovationStatus::New,
                framing: None,
            }],
            bottom: level(0.0),
            top: level(2500.0),
        };
        Ok((h, WallTool::new(template)))
    }

    #[test]
    fn walls_chain_and_take_the_typed_length() -> Result {
        let (mut h, mut tool) = setup()?;
        assert!(
            !tool.click(&mut h, Point::new(0.0, 0.0)),
            "first click starts"
        );
        assert!(
            tool.click(&mut h, Point::new(3000.0, 0.0)),
            "second click adds"
        );
        "4000".chars().for_each(|c| tool.type_char(c));
        assert!(tool.enter(&mut h, Some(Point::new(3000.0, 10.0))));
        let walls = &h.project().walls;
        assert_eq!(walls.len(), 2);
        assert_eq!(walls[1].start, Point::new(3000.0, 0.0), "chained");
        assert_eq!(walls[1].end, Point::new(3000.0, 4000.0), "typed length");
        h.undo()?;
        assert_eq!(h.project().walls.len(), 1, "undo removes the last wall");
        Ok(())
    }
}
