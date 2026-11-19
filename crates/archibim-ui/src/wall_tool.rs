//! The wall tool: one wall, a chain of walls or a rectangle, with lengths
//! typed at the keyboard. The clicks are the walls' reference line, chosen
//! in the template.

use archibim_commands::{Command, History};
use archibim_model::{Point, Wall};

/// How the clicks make walls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallMethod {
    /// Two clicks, one wall.
    Single,
    /// Each click ends a wall and starts the next; clicking the first point
    /// again closes the chain.
    Chain,
    /// Two opposite corners, four walls drawn clockwise, so that the left
    /// face (the first layer) is outside.
    Rectangle,
}

/// Draws walls from clicks.
#[derive(Clone, Debug)]
pub struct WallTool {
    /// The wall to copy: build-up, reference line, story, bottom and top.
    pub template: Wall,
    /// How clicks make walls.
    pub method: WallMethod,
    /// The points clicked in the current chain or rectangle.
    points: Vec<Point>,
    typed: String,
}

impl WallTool {
    /// A wall tool drawing chains of copies of `template`.
    pub fn new(template: Wall) -> Self {
        Self {
            template,
            method: WallMethod::Chain,
            points: Vec::new(),
            typed: String::new(),
        }
    }

    /// Where the wall being drawn starts, if one is.
    pub fn start(&self) -> Option<Point> {
        self.points.last().copied()
    }

    /// The points clicked so far in the current chain or rectangle.
    pub fn points(&self) -> &[Point] {
        &self.points
    }

    /// The length typed so far, in millimetres.
    pub fn typed(&self) -> &str {
        &self.typed
    }

    /// A click: starts drawing, or places the next point. Returns true when
    /// walls were added.
    pub fn click(&mut self, history: &mut History, p: Point) -> bool {
        let p = match self.start() {
            Some(start) => self.typed_end(start, p).unwrap_or(p),
            None => p,
        };
        self.typed.clear();
        self.place(history, p)
    }

    /// Enter: ends the wall at the typed length, towards the pointer.
    pub fn enter(&mut self, history: &mut History, toward: Option<Point>) -> bool {
        let (Some(start), Some(toward)) = (self.start(), toward) else {
            return false;
        };
        let Some(end) = self.typed_end(start, toward) else {
            return false;
        };
        self.typed.clear();
        self.place(history, end)
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
        if self.start().is_some() && (c.is_ascii_digit() || c == '.' || c == ',') {
            self.typed.push(c);
        }
    }

    /// Backspace: erases the last typed character.
    pub fn erase(&mut self) {
        self.typed.pop();
    }

    /// Escape: stops drawing.
    pub fn stop(&mut self) {
        self.points.clear();
        self.typed.clear();
    }

    fn place(&mut self, history: &mut History, p: Point) -> bool {
        let Some(&last) = self.points.last() else {
            self.points.push(p);
            return false;
        };
        if distance(last, p) < 1.0 {
            return false;
        }
        match self.method {
            WallMethod::Rectangle => {
                self.stop();
                let Some(corners) = rectangle(last, p) else {
                    return false;
                };
                let commands = (0..4)
                    .map(|i| Command::AddWall(self.wall(history, corners[i], corners[(i + 1) % 4])))
                    .collect();
                history.run(Command::Group(commands)).is_ok()
            }
            WallMethod::Single => {
                self.stop();
                let wall = self.wall(history, last, p);
                history.run(Command::AddWall(wall)).is_ok()
            }
            WallMethod::Chain => {
                let closes = self.points.len() >= 3 && distance(self.points[0], p) < 1.0;
                let end = if closes { self.points[0] } else { p };
                let wall = self.wall(history, last, end);
                // ponytail: a refused wall is dropped; show the reason once
                // the tool reports through the status bar.
                let added = history.run(Command::AddWall(wall)).is_ok();
                if closes {
                    self.stop();
                } else if added {
                    self.points.push(p);
                }
                added
            }
        }
    }

    fn wall(&self, history: &mut History, start: Point, end: Point) -> Wall {
        Wall {
            id: history.new_id(),
            start,
            end,
            ..self.template.clone()
        }
    }
}

/// The rectangle with opposite corners `a` and `c`, clockwise, or nothing
/// if it is flat.
pub fn rectangle(a: Point, c: Point) -> Option<Vec<Point>> {
    if (a.x - c.x).abs() < 1.0 || (a.y - c.y).abs() < 1.0 {
        return None;
    }
    let mut corners = vec![a, Point::new(a.x, c.y), c, Point::new(c.x, a.y)];
    if signed_area(&corners) > 0.0 {
        corners.reverse();
    }
    Some(corners)
}

fn distance(a: Point, b: Point) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn signed_area(points: &[Point]) -> f64 {
    let n = points.len();
    (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archibim_model::{
        Layer, LayerRole, LevelKind, LevelRef, Material, Project, RenovationStatus, Story,
        WallReference,
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
            reference: WallReference::LeftFace,
            offset: 0.0,
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

    fn close(a: Point, b: Point) -> bool {
        distance(a, b) < 1e-6
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

    #[test]
    fn a_single_wall_stops_after_two_clicks() -> Result {
        let (mut h, mut tool) = setup()?;
        tool.method = WallMethod::Single;
        tool.click(&mut h, Point::new(0.0, 0.0));
        assert!(tool.click(&mut h, Point::new(3000.0, 0.0)));
        assert!(tool.start().is_none());
        Ok(())
    }

    #[test]
    fn a_rectangle_on_its_axis_is_four_walls_undone_at_once() -> Result {
        let (mut h, mut tool) = setup()?;
        tool.method = WallMethod::Rectangle;
        tool.template.reference = WallReference::Axis;
        tool.click(&mut h, Point::new(0.0, 0.0));
        assert!(tool.click(&mut h, Point::new(6000.0, 4000.0)));
        let walls = &h.project().walls;
        assert_eq!(walls.len(), 4);
        for i in 0..4 {
            assert!(
                close(walls[i].end, walls[(i + 1) % 4].start),
                "corners meet on the clicks"
            );
        }
        // Clockwise, left face outside: 100 out of the clicked axis.
        let xs: Vec<f64> = walls
            .iter()
            .filter_map(Wall::left_face)
            .flat_map(|(a, b)| [a.x, b.x])
            .collect();
        assert!(xs.iter().any(|x| (x + 100.0).abs() < 1e-6), "{xs:?}");
        assert!(xs.iter().any(|x| (x - 6100.0).abs() < 1e-6), "{xs:?}");
        h.undo()?;
        assert!(h.project().walls.is_empty(), "one undo removes all four");
        Ok(())
    }

    #[test]
    fn a_chain_on_its_right_face_keeps_the_clicks_and_closes() -> Result {
        let (mut h, mut tool) = setup()?;
        tool.template.reference = WallReference::RightFace;
        // Clockwise, so the right face is inside, on the clicks.
        for (x, y) in [
            (0.0, 0.0),
            (4000.0, 0.0),
            (4000.0, -3000.0),
            (0.0, -3000.0),
            (0.0, 0.0),
        ] {
            tool.click(&mut h, Point::new(x, y));
        }
        let walls = &h.project().walls;
        assert_eq!(walls.len(), 4);
        assert!(close(walls[0].start, Point::new(0.0, 0.0)), "on the click");
        assert!(close(walls[3].end, walls[0].start), "closed");
        let (face, _) = walls[0].left_face().ok_or("no length")?;
        assert!(close(face, Point::new(0.0, 200.0)), "left face outside");
        assert!(tool.start().is_none(), "closing stops the chain");
        h.undo()?;
        assert_eq!(h.project().walls.len(), 3, "undo removes the last wall");
        Ok(())
    }

    #[test]
    fn a_flipped_wall_mirrors_about_its_line_and_keeps_its_openings() -> Result {
        let (mut h, mut tool) = setup()?;
        tool.method = WallMethod::Single;
        tool.template.offset = 50.0;
        tool.click(&mut h, Point::new(0.0, 0.0));
        tool.click(&mut h, Point::new(5000.0, 0.0));
        let wall = h.project().walls[0].clone();
        let (face, _) = wall.left_face().ok_or("no length")?;
        assert!(close(face, Point::new(0.0, -50.0)), "offset to the right");
        let opening = archibim_model::Opening {
            id: h.new_id(),
            wall: wall.id,
            kind: archibim_model::OpeningKind::Interior,
            position: 1000.0,
            sill: 0.0,
            height: 2000.0,
            frame: 50.0,
            parts: vec![archibim_model::Part {
                width: 800.0,
                operation: archibim_model::Operation::Fixed,
            }],
            nominal: archibim_model::NominalSize::ClearOpening,
        };
        h.run(Command::AddOpening(opening.clone()))?;
        h.run(crate::properties::flip(h.project(), &wall))?;
        let flipped = h.project().wall(wall.id)?;
        let (face, end) = flipped.left_face().ok_or("no length")?;
        // Seen from the other end, the left face is now at y = -50 on the
        // line's other side: the wall lies between -50 and 150.
        assert!(close(face, Point::new(5000.0, -50.0)), "{face:?}");
        assert!(close(end, Point::new(0.0, -50.0)));
        let moved = h.project().opening(opening.id)?;
        assert!((moved.position - 3100.0).abs() < 1e-6, "{}", moved.position);
        h.undo()?;
        assert_eq!(h.project().wall(wall.id)?.start, wall.start, "one undo");
        Ok(())
    }
}
