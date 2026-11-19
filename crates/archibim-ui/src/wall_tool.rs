//! The wall tool: one wall, a chain of walls or a rectangle, drawn on a
//! reference line, with lengths typed at the keyboard.

use archibim_commands::{Command, History};
use archibim_model::{Id, Point, Wall};

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

/// Which line of the wall the clicks place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reference {
    /// The left face, where the first layer is.
    Left,
    /// The middle of the build-up.
    Axis,
    /// The right face, where the last layer is.
    Right,
}

impl Reference {
    /// How far the left face is from the clicked line, as a share of the
    /// thickness, towards the left.
    fn share(self) -> f64 {
        match self {
            Self::Left => 0.0,
            Self::Axis => 0.5,
            Self::Right => 1.0,
        }
    }
}

/// Draws walls from clicks.
// ponytail: the reference line lives in the tool, not in the wall; a wall
// changed later keeps its left face in place. Store it on the wall when
// the wall tool is detailed.
#[derive(Clone, Debug)]
pub struct WallTool {
    /// The wall to copy: build-up, story, bottom and top.
    pub template: Wall,
    /// How clicks make walls.
    pub method: WallMethod,
    /// Which line the clicks place.
    pub reference: Reference,
    /// The points clicked in the current chain or rectangle.
    points: Vec<Point>,
    /// The walls drawn so far in the current chain.
    walls: Vec<Id>,
    typed: String,
}

impl WallTool {
    /// A wall tool drawing chains of copies of `template` on their left face.
    pub fn new(template: Wall) -> Self {
        Self {
            template,
            method: WallMethod::Chain,
            reference: Reference::Left,
            points: Vec::new(),
            walls: Vec::new(),
            typed: String::new(),
        }
    }

    /// Where the wall being drawn starts, if one is.
    pub fn start(&self) -> Option<Point> {
        self.points.last().copied()
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
        self.walls.clear();
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
        let shift = self.reference.share() * self.template.thickness();
        match self.method {
            WallMethod::Rectangle => {
                self.stop();
                let Some(corners) = rectangle(last, p) else {
                    return false;
                };
                let faces = offset(&corners, shift, true);
                let commands = (0..4)
                    .map(|i| Command::AddWall(self.wall(history, faces[i], faces[(i + 1) % 4])))
                    .collect();
                history.run(Command::Group(commands)).is_ok()
            }
            WallMethod::Single => {
                self.stop();
                let faces = offset(&[last, p], shift, false);
                let wall = self.wall(history, faces[0], faces[1]);
                history.run(Command::AddWall(wall)).is_ok()
            }
            WallMethod::Chain => self.chain(history, p, shift),
        }
    }

    fn chain(&mut self, history: &mut History, p: Point, shift: f64) -> bool {
        let closes = self.points.len() >= 3 && distance(self.points[0], p) < 1.0;
        if !closes {
            self.points.push(p);
        }
        let faces = offset(&self.points, shift, closes);
        // The new wall runs from corner `from` to corner `to`.
        let n = self.points.len();
        let (from, to) = if closes { (n - 1, 0) } else { (n - 2, n - 1) };
        let mut commands = Vec::new();
        // The previous wall now ends on the new corner.
        if let Some(&previous) = self.walls.last()
            && let Ok(w) = history.project().wall(previous)
        {
            commands.push(Command::ReplaceWall(Wall {
                end: faces[from],
                ..w.clone()
            }));
        }
        let wall = self.wall(history, faces[from], faces[to]);
        let id = wall.id;
        commands.push(Command::AddWall(wall));
        // Closing: the first wall starts on the closing corner.
        if closes
            && let Some(&first) = self.walls.first()
            && let Ok(w) = history.project().wall(first)
        {
            commands.push(Command::ReplaceWall(Wall {
                start: faces[0],
                ..w.clone()
            }));
        }
        // ponytail: a refused wall is dropped; show the reason once the
        // tool reports through the status bar.
        let added = history.run(Command::Group(commands)).is_ok();
        if closes {
            self.stop();
        } else if added {
            self.walls.push(id);
        } else {
            self.points.pop();
        }
        added
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

/// The points moved `shift` to the left of the path through them, each
/// corner where the moved segments meet.
fn offset(points: &[Point], shift: f64, closed: bool) -> Vec<Point> {
    let n = points.len();
    if shift == 0.0 || n < 2 {
        return points.to_vec();
    }
    let segments = if closed { n } else { n - 1 };
    // Each segment moved: a point on it and its direction.
    let moved: Vec<(Point, Point)> = (0..segments)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            let l = distance(a, b).max(1e-9);
            let d = Point::new((b.x - a.x) / l, (b.y - a.y) / l);
            (Point::new(a.x - d.y * shift, a.y + d.x * shift), d)
        })
        .collect();
    let meet = |(p, d): (Point, Point), (q, e): (Point, Point)| {
        let cross = d.x * e.y - d.y * e.x;
        if cross.abs() < 1e-9 {
            return q;
        }
        let t = ((q.x - p.x) * e.y - (q.y - p.y) * e.x) / cross;
        Point::new(p.x + d.x * t, p.y + d.y * t)
    };
    (0..n)
        .map(|i| {
            let after = (i < segments).then(|| moved[i]);
            let before = match i {
                0 if closed => Some(moved[segments - 1]),
                0 => None,
                _ => Some(moved[i - 1]),
            };
            match (before, after) {
                (Some(b), Some(a)) => meet(b, a),
                (None, Some((p, _))) => p,
                (Some((p, d)), None) => {
                    let l = distance(points[i - 1], points[i]);
                    Point::new(p.x + d.x * l, p.y + d.y * l)
                }
                (None, None) => points[i],
            }
        })
        .collect()
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
        tool.reference = Reference::Axis;
        tool.click(&mut h, Point::new(0.0, 0.0));
        assert!(tool.click(&mut h, Point::new(6000.0, 4000.0)));
        let walls = &h.project().walls;
        assert_eq!(walls.len(), 4);
        // Clockwise, left face outside: 100 out of the clicked axis.
        let xs: Vec<f64> = walls.iter().flat_map(|w| [w.start.x, w.end.x]).collect();
        assert!(xs.iter().any(|x| (x + 100.0).abs() < 1e-6), "{xs:?}");
        assert!(xs.iter().any(|x| (x - 6100.0).abs() < 1e-6), "{xs:?}");
        for i in 0..4 {
            assert!(
                close(walls[i].end, walls[(i + 1) % 4].start),
                "corners meet"
            );
        }
        h.undo()?;
        assert!(h.project().walls.is_empty(), "one undo removes all four");
        Ok(())
    }

    #[test]
    fn a_chain_on_its_right_face_moves_the_corner_and_closes() -> Result {
        let (mut h, mut tool) = setup()?;
        tool.reference = Reference::Right;
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
        assert!(
            close(walls[0].start, Point::new(-200.0, 200.0)),
            "closed corner"
        );
        assert!(
            close(walls[0].end, Point::new(4200.0, 200.0)),
            "moved corner"
        );
        assert!(close(walls[3].end, walls[0].start));
        assert!(tool.start().is_none(), "closing stops the chain");
        h.undo()?;
        assert!(
            close(h.project().walls[0].start, Point::new(0.0, 200.0)),
            "undo puts the first corner back"
        );
        Ok(())
    }
}
