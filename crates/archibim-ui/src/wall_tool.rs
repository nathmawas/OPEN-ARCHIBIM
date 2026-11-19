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
    /// One side from two clicks, then the depth: a rectangle at any angle.
    RotatedRectangle,
    /// The centre, then a point of the circle: two half circles.
    Circle,
    /// The centre, the start, then the end: the shorter way round.
    Arc,
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
    /// The length typed so far.
    typed: String,
    /// The angle typed so far, in degrees anticlockwise from the x axis.
    angle: String,
    /// Typing goes to the angle rather than the length.
    on_angle: bool,
    /// How far the pointer has turned around the centre of an arc, and
    /// where it was last.
    turn: f64,
    last_angle: Option<f64>,
}

impl WallTool {
    /// A wall tool drawing chains of copies of `template`.
    pub fn new(template: Wall) -> Self {
        Self {
            template,
            method: WallMethod::Chain,
            points: Vec::new(),
            typed: String::new(),
            angle: String::new(),
            on_angle: false,
            turn: 0.0,
            last_angle: None,
        }
    }

    /// The pointer moved to `p`: an arc follows it round its centre, so it
    /// can turn past a half circle.
    pub fn hover(&mut self, p: Point) {
        let (WallMethod::Arc, &[c, _]) = (self.method, self.points.as_slice()) else {
            self.last_angle = None;
            return;
        };
        if distance(c, p) < 1.0 {
            return;
        }
        let a = (p.y - c.y).atan2(p.x - c.x);
        if let Some(previous) = self.last_angle {
            let tau = std::f64::consts::TAU;
            let mut step = (a - previous).rem_euclid(tau);
            if step > std::f64::consts::PI {
                step -= tau;
            }
            self.turn = (self.turn + step).clamp(-tau + 1e-3, tau - 1e-3);
        }
        self.last_angle = Some(a);
    }

    /// The start angle and the turn of the arc being drawn towards `p`: as
    /// far as the pointer went round, or else the shorter way.
    fn arc_turn(&self, c: Point, a: Point, p: Point) -> Option<(f64, f64)> {
        let (from, shorter) = arc_sweep(c, a, p)?;
        let followed = self.last_angle.is_some() && self.turn.abs() > 1e-6;
        Some((from, if followed { self.turn } else { shorter }))
    }

    /// Where the wall being drawn starts, if one is.
    pub fn start(&self) -> Option<Point> {
        self.points.last().copied()
    }

    /// The points clicked so far in the current chain or rectangle.
    pub fn points(&self) -> &[Point] {
        &self.points
    }

    /// What was typed, to show next to the pointer: the length, then the
    /// angle after `<`.
    pub fn typed(&self) -> String {
        if self.angle.is_empty() && !self.on_angle {
            self.typed.clone()
        } else {
            format!("{} < {}°", self.typed, self.angle)
        }
    }

    /// Tab or `<`: typing goes to the angle, or back to the length.
    pub fn switch_field(&mut self) {
        if self.start().is_some() {
            self.on_angle = !self.on_angle;
        }
    }

    fn clear_typed(&mut self) {
        self.typed.clear();
        self.angle.clear();
        self.on_angle = false;
    }

    /// A click: starts drawing, or places the next point. Returns true when
    /// walls were added.
    pub fn click(&mut self, history: &mut History, p: Point) -> bool {
        let p = match self.start() {
            Some(start) => self.typed_end(start, p).unwrap_or(p),
            None => p,
        };
        self.clear_typed();
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
        self.clear_typed();
        self.place(history, end)
    }

    /// The end of the wall from `start` with what was typed: the typed
    /// length, along the typed angle or towards `toward`. With an angle
    /// and no length, the pointer's distance along that angle.
    pub fn typed_end(&self, start: Point, toward: Point) -> Option<Point> {
        let number = |s: &str| s.replace(',', ".").parse::<f64>().ok();
        let (length, angle) = (number(&self.typed), number(&self.angle));
        let (dx, dy) = match angle {
            Some(degrees) => {
                let a = degrees.to_radians();
                (a.cos(), a.sin())
            }
            None => {
                let (dx, dy) = (toward.x - start.x, toward.y - start.y);
                let norm = (dx * dx + dy * dy).sqrt();
                if norm == 0.0 {
                    return None;
                }
                (dx / norm, dy / norm)
            }
        };
        let length = match (length, angle) {
            (Some(l), _) => l,
            (None, Some(_)) => (toward.x - start.x) * dx + (toward.y - start.y) * dy,
            (None, None) => return None,
        };
        (length > 0.0).then(|| Point::new(start.x + dx * length, start.y + dy * length))
    }

    /// A typed character: digits and a decimal separator build the length,
    /// or the angle after `<`, which may be negative.
    pub fn type_char(&mut self, c: char) {
        if self.start().is_none() {
            return;
        }
        match c {
            '<' => self.switch_field(),
            '-' if self.on_angle && self.angle.is_empty() => self.angle.push(c),
            c if c.is_ascii_digit() || c == '.' || c == ',' => {
                if self.on_angle {
                    self.angle.push(c);
                } else {
                    self.typed.push(c);
                }
            }
            _ => {}
        }
    }

    /// Backspace: erases the last typed character.
    pub fn erase(&mut self) {
        if !self.on_angle {
            self.typed.pop();
        } else if self.angle.pop().is_none() {
            self.on_angle = false;
        }
    }

    /// Escape: stops drawing.
    pub fn stop(&mut self) {
        self.points.clear();
        self.clear_typed();
        self.turn = 0.0;
        self.last_angle = None;
    }

    /// What the next click would draw, as a line through its points, for
    /// the preview of the methods that need more than one step.
    pub fn preview(&self, p: Point) -> Option<Vec<Point>> {
        match (self.method, self.points.as_slice()) {
            (WallMethod::RotatedRectangle, &[a, b]) => {
                let mut corners = rotated(a, b, p)?;
                corners.push(corners[0]);
                Some(corners)
            }
            (WallMethod::Circle, &[c]) => {
                let r = distance(c, p);
                Some(arc_points(c, r, 0.0, std::f64::consts::TAU))
            }
            (WallMethod::Arc, &[c, a]) => {
                let (from, sweep) = self.arc_turn(c, a, p)?;
                Some(arc_points(c, distance(c, a), from, sweep))
            }
            _ => None,
        }
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
            WallMethod::RotatedRectangle => match *self.points.as_slice() {
                [_] => {
                    self.points.push(p);
                    false
                }
                [a, b] => match rotated(a, b, p) {
                    Some(corners) => self.outline(history, corners),
                    None => false,
                },
                _ => false,
            },
            WallMethod::Circle => {
                self.stop();
                let r = distance(last, p);
                let east = Point::new(last.x + r, last.y);
                let west = Point::new(last.x - r, last.y);
                let half = std::f64::consts::PI;
                let commands = vec![
                    Command::AddWall(self.curved(history, east, west, half)),
                    Command::AddWall(self.curved(history, west, east, half)),
                ];
                history.run(Command::Group(commands)).is_ok()
            }
            WallMethod::Arc => match *self.points.as_slice() {
                [c] => {
                    self.points.push(p);
                    // The pointer starts turning from the start of the arc.
                    self.turn = 0.0;
                    self.last_angle = Some((p.y - c.y).atan2(p.x - c.x));
                    false
                }
                [c, a] => {
                    let turn = self.arc_turn(c, a, p);
                    self.stop();
                    let Some((_, sweep)) = turn else {
                        return false;
                    };
                    let r = distance(c, a);
                    let end = (a.y - c.y).atan2(a.x - c.x) + sweep;
                    let e = Point::new(c.x + r * end.cos(), c.y + r * end.sin());
                    let wall = self.curved(history, a, e, sweep);
                    history.run(Command::AddWall(wall)).is_ok()
                }
                _ => false,
            },
            WallMethod::Rectangle => {
                self.stop();
                rectangle(last, p).is_some_and(|corners| self.outline(history, corners))
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

    /// Walls all around a closed outline, in one undo step. They run
    /// clockwise, so that the first layer is outside.
    pub fn outline(&mut self, history: &mut History, mut corners: Vec<Point>) -> bool {
        self.stop();
        if signed_area(&corners) > 0.0 {
            corners.reverse();
        }
        let n = corners.len();
        let commands = (0..n)
            .map(|i| Command::AddWall(self.wall(history, corners[i], corners[(i + 1) % n])))
            .collect();
        n >= 3 && history.run(Command::Group(commands)).is_ok()
    }

    fn curved(&self, history: &mut History, start: Point, end: Point, sweep: f64) -> Wall {
        Wall {
            sweep,
            ..self.wall(history, start, end)
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

/// The rectangle with side `a` to `b` and its depth out to `c`, or
/// nothing if it is flat.
fn rotated(a: Point, b: Point, c: Point) -> Option<Vec<Point>> {
    let l = distance(a, b);
    let (dx, dy) = ((b.x - a.x) / l, (b.y - a.y) / l);
    // Signed depth towards the left of a to b.
    let h = -(c.x - a.x) * dy + (c.y - a.y) * dx;
    if l < 1.0 || h.abs() < 1.0 {
        return None;
    }
    let up = |q: Point| Point::new(q.x - dy * h, q.y + dx * h);
    Some(vec![a, b, up(b), up(a)])
}

/// The start angle and the turn, the shorter way round, of the arc of
/// centre `c` from `a` towards `p`.
fn arc_sweep(c: Point, a: Point, p: Point) -> Option<(f64, f64)> {
    if distance(c, a) < 1.0 || distance(c, p) < 1.0 {
        return None;
    }
    let from = (a.y - c.y).atan2(a.x - c.x);
    let to = (p.y - c.y).atan2(p.x - c.x);
    let tau = std::f64::consts::TAU;
    let mut sweep = (to - from).rem_euclid(tau);
    if sweep > std::f64::consts::PI {
        sweep -= tau;
    }
    (sweep.abs() > 1e-6).then_some((from, sweep))
}

fn arc_points(c: Point, r: f64, from: f64, sweep: f64) -> Vec<Point> {
    let steps = 64;
    (0..=steps)
        .map(|k| {
            let a = from + sweep * f64::from(k) / f64::from(steps);
            Point::new(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect()
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
            sweep: 0.0,
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

    #[test]
    fn a_rotated_rectangle_takes_a_side_then_a_depth() -> Result {
        let (mut h, mut tool) = setup()?;
        tool.method = WallMethod::RotatedRectangle;
        tool.click(&mut h, Point::new(0.0, 0.0));
        assert!(!tool.click(&mut h, Point::new(3000.0, 3000.0)), "side only");
        assert!(tool.preview(Point::new(-1000.0, 1000.0)).is_some());
        assert!(tool.click(&mut h, Point::new(-1000.0, 1000.0)));
        let walls = &h.project().walls;
        assert_eq!(walls.len(), 4);
        let side = distance(walls[0].start, walls[0].end);
        let other = distance(walls[1].start, walls[1].end);
        let (a, b) = (side.min(other), side.max(other));
        assert!((a - 1000.0 * 2f64.sqrt()).abs() < 1e-6, "{a}");
        assert!((b - 3000.0 * 2f64.sqrt()).abs() < 1e-6, "{b}");
        Ok(())
    }

    #[test]
    fn a_circle_is_two_half_circles_and_an_arc_the_shorter_way() -> Result {
        let (mut h, mut tool) = setup()?;
        tool.method = WallMethod::Circle;
        tool.click(&mut h, Point::new(0.0, 0.0));
        assert!(tool.click(&mut h, Point::new(0.0, 2000.0)));
        let walls = h.project().walls.clone();
        assert_eq!(walls.len(), 2);
        for w in &walls {
            let (c, r) = w.arc().ok_or("not curved")?;
            assert!(close(c, Point::new(0.0, 0.0)) && (r - 2000.0).abs() < 1e-6);
        }
        h.undo()?;
        assert!(h.project().walls.is_empty(), "one undo");

        tool.method = WallMethod::Arc;
        tool.click(&mut h, Point::new(0.0, 0.0));
        tool.click(&mut h, Point::new(1000.0, 0.0));
        assert!(tool.click(&mut h, Point::new(0.0, 500.0)));
        let w = &h.project().walls[0];
        assert!(
            (w.sweep - std::f64::consts::FRAC_PI_2).abs() < 1e-9,
            "quarter, anticlockwise"
        );
        assert!(close(w.end, Point::new(0.0, 1000.0)), "on the circle");
        Ok(())
    }

    #[test]
    fn a_typed_angle_sets_the_direction_with_or_without_a_length() -> Result {
        let (mut h, mut tool) = setup()?;
        tool.method = WallMethod::Single;
        tool.click(&mut h, Point::new(0.0, 0.0));
        "2000<90".chars().for_each(|c| tool.type_char(c));
        assert_eq!(tool.typed(), "2000 < 90°");
        assert!(tool.enter(&mut h, Some(Point::new(500.0, 0.0))));
        assert!(close(h.project().walls[0].end, Point::new(0.0, 2000.0)));
        // An angle alone: the pointer gives the length along it.
        tool.click(&mut h, Point::new(0.0, 0.0));
        tool.switch_field();
        "-45".chars().for_each(|c| tool.type_char(c));
        assert!(tool.click(&mut h, Point::new(1000.0, 0.0)));
        let end = h.project().walls[1].end;
        // The pointer at 1000 east, projected on the 45 degrees south-east.
        assert!(close(end, Point::new(500.0, -500.0)), "{end:?}");
        Ok(())
    }

    #[test]
    fn an_arc_follows_the_pointer_past_a_half_circle() -> Result {
        let (mut h, mut tool) = setup()?;
        tool.method = WallMethod::Arc;
        tool.click(&mut h, Point::new(0.0, 0.0));
        tool.click(&mut h, Point::new(1000.0, 0.0));
        // Round anticlockwise through north and west to south.
        for (x, y) in [
            (700.0, 700.0),
            (0.0, 1000.0),
            (-700.0, 700.0),
            (-1000.0, 0.0),
            (-700.0, -700.0),
            (0.0, -1000.0),
        ] {
            tool.hover(Point::new(x, y));
        }
        assert!(tool.click(&mut h, Point::new(0.0, -1000.0)));
        let w = &h.project().walls[0];
        assert!(
            (w.sweep - 1.5 * std::f64::consts::PI).abs() < 1e-9,
            "{}",
            w.sweep
        );
        Ok(())
    }
}
