//! Drawing aids: where the pointer lands once it snaps to points, edges,
//! angles, alignments and guide lines, and the dashed lines that show why.
//!
//! An edge the pointer passes over is picked up: until two others replace
//! it, it offers guide lines (its extension and its perpendiculars at both
//! ends), directions parallel and perpendicular to it, and the foot of the
//! perpendicular from the last point placed.
//!
//! Pure logic: the plan view draws the result.

use archibim_geometry::{beam_outline, column_outline};
use archibim_model::{Point, Project};

/// How many edges stay picked up.
pub const PICKED_EDGES: usize = 2;

/// Which drawing aids are on. All are on by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapOptions {
    /// Ends of walls and beams, corners of faces, slabs and columns.
    pub points: bool,
    /// Middles of walls and slab edges, or their divisions.
    pub midpoints: bool,
    /// Where wall lines and slab edges cross.
    pub intersections: bool,
    /// Anywhere on a face or an edge.
    pub edges: bool,
    /// Directions every `step` degrees from the last point.
    pub angles: bool,
    /// Horizontal and vertical alignments with nearby points, and along
    /// the current direction.
    pub alignments: bool,
    /// Guide lines, parallels and perpendiculars from picked-up edges.
    pub guides: bool,
    /// The angle step, in degrees: 15, 30, 45 or 90.
    pub step: u16,
    /// Into how many equal parts walls and slab edges are divided: 2 gives
    /// their middles.
    pub divisions: u8,
}

impl Default for SnapOptions {
    fn default() -> Self {
        Self {
            points: true,
            midpoints: true,
            intersections: true,
            edges: true,
            angles: true,
            alignments: true,
            guides: true,
            step: 45,
            divisions: 2,
        }
    }
}

/// What the pointer landed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapKind {
    /// Nothing: the pointer itself.
    Free,
    /// An end or a corner.
    Point,
    /// A middle, or a division point.
    Midpoint,
    /// A crossing of two lines.
    Intersection,
    /// A point on an edge.
    Edge,
    /// A point on the current direction.
    Direction,
    /// A point aligned with another point.
    Alignment,
    /// The foot of the perpendicular from the last point on an edge.
    Perpendicular,
    /// A point on a guide line of a picked-up edge.
    Guide,
    /// A point on a direction parallel or perpendicular to a picked-up
    /// edge.
    Parallel,
}

/// Where the pointer lands, and the guides to draw.
#[derive(Clone, Debug, PartialEq)]
pub struct Snap {
    /// The point used by the tool.
    pub point: Point,
    /// Why it is there.
    pub kind: SnapKind,
    /// Dashed lines from where the snap comes from to the point.
    pub guides: Vec<(Point, Point)>,
}

/// An infinite guide line, drawn from the nearer of its anchors.
#[derive(Clone, Copy, Debug)]
struct Line {
    anchors: [Point; 2],
    u: Point,
}

impl Line {
    fn foot(&self, p: Point) -> Point {
        let a = self.anchors[0];
        add(a, mul(self.u, dot(sub(p, a), self.u)))
    }

    fn anchor(&self, p: Point) -> Point {
        let [a, b] = self.anchors;
        if distance(a, p) <= distance(b, p) {
            a
        } else {
            b
        }
    }
}

/// What can be snapped to, gathered from the project and the drawing in
/// progress.
#[derive(Debug, Default)]
pub struct Scene {
    points: Vec<Point>,
    /// Walls and slab edges, divided for the middle points.
    divided: Vec<(Point, Point)>,
    intersections: Vec<Point>,
    segments: Vec<(Point, Point)>,
    picked: Vec<(Point, Point)>,
}

impl Scene {
    /// Everything snappable in `project`, and the points being drawn.
    // ponytail: rebuilt each frame and crossings found pair by pair; keep
    // it with the plan cache and a spatial index when projects get large.
    pub fn new(project: &Project, drawing: &[Point]) -> Self {
        let mut s = Self::default();
        let mut lines = Vec::new();
        for w in &project.walls {
            s.points.extend([w.start, w.end]);
            if let Some((centre, _)) = w.arc() {
                s.points.push(centre);
                continue;
            }
            s.divided.push((w.start, w.end));
            lines.push((w.start, w.end));
            if let Some((a, b)) = w.left_face() {
                let k = w.thickness() / distance(a, b);
                let right = |q: Point| Point::new(q.x + (b.y - a.y) * k, q.y - (b.x - a.x) * k);
                let (c, d) = (right(a), right(b));
                s.points.extend([a, b, c, d]);
                s.segments.extend([(a, b), (c, d), (a, c), (b, d)]);
            }
        }
        for slab in &project.slabs {
            let n = slab.outline.len();
            for i in 0..n {
                let (a, b) = (slab.outline[i], slab.outline[(i + 1) % n]);
                s.points.push(a);
                s.divided.push((a, b));
                lines.push((a, b));
            }
        }
        for c in &project.columns {
            s.points.push(c.at);
            s.closed(&column_outline(c));
        }
        for b in &project.beams {
            s.points.extend([b.start, b.end]);
            if let Some(outline) = beam_outline(b) {
                s.closed(&outline);
            }
        }
        s.points.extend(drawing.iter().copied());
        for (i, &(a, b)) in lines.iter().enumerate() {
            for &(c, d) in &lines[i + 1..] {
                if let Some(x) = crossing(a, b, c, d) {
                    s.intersections.push(x);
                }
            }
        }
        s.segments.extend(lines);
        s
    }

    fn closed(&mut self, outline: &[Point]) {
        let n = outline.len();
        self.points.extend(outline.iter().copied());
        self.segments
            .extend((0..n).map(|i| (outline[i], outline[(i + 1) % n])));
    }

    /// Sets the edges picked up so far, most recent first.
    pub fn pick(&mut self, edges: &[(Point, Point)]) {
        self.picked = edges.to_vec();
    }

    /// The edge under the pointer, to pick up.
    pub fn edge_near(&self, p: Point, tolerance: f64) -> Option<(Point, Point)> {
        self.on_edge(p, tolerance).map(|(_, edge)| edge)
    }

    /// The guide lines of the picked-up edges.
    fn lines(&self) -> Vec<Line> {
        self.picked
            .iter()
            .filter_map(|&(a, b)| {
                let u = unit(sub(b, a))?;
                let n = Point::new(-u.y, u.x);
                Some([
                    Line { anchors: [a, b], u },
                    Line {
                        anchors: [a, a],
                        u: n,
                    },
                    Line {
                        anchors: [b, b],
                        u: n,
                    },
                ])
            })
            .flatten()
            .collect()
    }

    /// Where the pointer at `p` lands. `from` is the last point placed, if
    /// any; `shift` locks the direction from it to the nearest angle step
    /// or picked-up direction; `tolerance` is the snapping distance in
    /// model units.
    pub fn snap(
        &self,
        options: &SnapOptions,
        p: Point,
        from: Option<Point>,
        shift: bool,
        tolerance: f64,
    ) -> Snap {
        let free = |kind, point| Snap {
            point,
            kind,
            guides: Vec::new(),
        };
        let lines = if options.guides {
            self.lines()
        } else {
            Vec::new()
        };
        if !shift {
            if let Some(s) = self.on_point(options, p, tolerance) {
                return s;
            }
            if let Some(s) = self.perpendicular(options, p, from, tolerance) {
                return s;
            }
            if let Some(s) = crossing_lines(&lines, p, tolerance) {
                return s;
            }
        }
        if let Some(start) = from
            && (shift || options.angles || options.guides)
            && let Some((u, kind)) = self.direction(options, start, p, shift)
        {
            let t = dot(sub(p, start), u);
            let on_ray = add(start, mul(u, t));
            if shift || distance(on_ray, p) <= tolerance {
                return self.along(options, &lines, start, u, t, kind, tolerance);
            }
        }
        if let Some(l) = lines
            .iter()
            .filter(|l| distance(l.foot(p), p) <= tolerance)
            .min_by(|a, b| distance(a.foot(p), p).total_cmp(&distance(b.foot(p), p)))
        {
            let point = l.foot(p);
            return Snap {
                point,
                kind: SnapKind::Guide,
                guides: vec![(l.anchor(point), point)],
            };
        }
        if options.alignments
            && let Some(s) = self.aligned(p, from, tolerance)
        {
            return s;
        }
        if options.edges
            && let Some((q, _)) = self.on_edge(p, tolerance)
        {
            return free(SnapKind::Edge, q);
        }
        free(SnapKind::Free, p)
    }

    fn on_point(&self, options: &SnapOptions, p: Point, tolerance: f64) -> Option<Snap> {
        let n = options.divisions.max(2);
        let divisions: Vec<Point> = self
            .divided
            .iter()
            .flat_map(|&(a, b)| {
                (1..n).map(move |k| add(a, mul(sub(b, a), f64::from(k) / f64::from(n))))
            })
            .collect();
        let sets = [
            (options.points, &self.points, SnapKind::Point),
            (
                options.intersections,
                &self.intersections,
                SnapKind::Intersection,
            ),
            (options.midpoints, &divisions, SnapKind::Midpoint),
        ];
        sets.iter()
            .filter(|(on, ..)| *on)
            .flat_map(|(_, points, kind)| points.iter().map(move |q| (*q, *kind)))
            .filter(|(q, _)| distance(*q, p) <= tolerance)
            .min_by(|a, b| distance(a.0, p).total_cmp(&distance(b.0, p)))
            .map(|(point, kind)| Snap {
                point,
                kind,
                guides: Vec::new(),
            })
    }

    /// The foot of the perpendicular from `from` on a picked-up edge, or
    /// on its extension.
    fn perpendicular(
        &self,
        options: &SnapOptions,
        p: Point,
        from: Option<Point>,
        tolerance: f64,
    ) -> Option<Snap> {
        let start = from.filter(|_| options.guides)?;
        self.picked
            .iter()
            .filter_map(|&(a, b)| {
                let u = unit(sub(b, a))?;
                let line = Line { anchors: [a, b], u };
                let foot = line.foot(start);
                (distance(foot, p) <= tolerance && distance(foot, start) > tolerance)
                    .then(|| (foot, line.anchor(foot)))
            })
            .min_by(|x, y| distance(x.0, p).total_cmp(&distance(y.0, p)))
            .map(|(point, anchor)| Snap {
                point,
                kind: SnapKind::Perpendicular,
                guides: vec![(start, point), (anchor, point)],
            })
    }

    /// The unit direction from `start` nearest to the way to `p`: a
    /// multiple of the angle step, or parallel or perpendicular to a
    /// picked-up edge.
    fn direction(
        &self,
        options: &SnapOptions,
        start: Point,
        p: Point,
        shift: bool,
    ) -> Option<(Point, SnapKind)> {
        let v = unit(sub(p, start))?;
        let mut best: Option<(Point, SnapKind)> = None;
        let mut consider = |u: Point, kind| {
            for u in [u, mul(u, -1.0)] {
                if best.is_none_or(|(b, _)| dot(u, v) > dot(b, v) + 1e-9) {
                    best = Some((u, kind));
                }
            }
        };
        if shift || options.angles {
            let step = f64::from(options.step.clamp(1, 90)).to_radians();
            let angle = (v.y.atan2(v.x) / step).round() * step;
            consider(Point::new(angle.cos(), angle.sin()), SnapKind::Direction);
        }
        if options.guides {
            for l in self.lines() {
                consider(l.u, SnapKind::Parallel);
            }
        }
        best
    }

    /// On the ray from `start` along `u`, near the distance `t`: a point on
    /// the ray, an edge or a guide line it crosses, or a point it aligns
    /// with.
    #[allow(clippy::too_many_arguments)]
    fn along(
        &self,
        options: &SnapOptions,
        lines: &[Line],
        start: Point,
        u: Point,
        t: f64,
        kind: SnapKind,
        tolerance: f64,
    ) -> Snap {
        let at = |s: f64| add(start, mul(u, s));
        let off = |q: Point| cross(sub(q, start), u).abs();
        let foot = |q: Point| dot(sub(q, start), u);
        let near = |s: f64| (s - t).abs() <= tolerance;
        let best = |list: &mut Vec<(f64, Option<Point>)>| {
            list.sort_by(|a, b| (a.0 - t).abs().total_cmp(&(b.0 - t).abs()));
            list.first().copied()
        };
        let snap = |s: f64, kind, guide: Option<Point>| {
            let point = at(s);
            let mut guides = vec![(start, point)];
            guides.extend(guide.map(|g| (g, point)));
            Snap {
                point,
                kind,
                guides,
            }
        };
        let mut on_ray: Vec<(f64, Option<Point>)> = self
            .points
            .iter()
            .filter(|q| options.points && off(**q) <= tolerance && near(foot(**q)))
            .map(|q| (foot(*q), None))
            .collect();
        if let Some((s, _)) = best(&mut on_ray) {
            return snap(s, SnapKind::Point, None);
        }
        let mut crossings: Vec<(f64, Option<Point>)> = self
            .segments
            .iter()
            .filter(|_| options.edges || options.intersections)
            .filter_map(|&(a, b)| line_cross(start, u, a, b))
            .map(|s| (s, None))
            .chain(lines.iter().filter_map(|l| {
                let s = line_line(start, u, l.anchors[0], l.u)?;
                Some((s, Some(l.anchor(at(s)))))
            }))
            .filter(|(s, _)| near(*s))
            .collect();
        if let Some((s, guide)) = best(&mut crossings) {
            return snap(s, SnapKind::Intersection, guide);
        }
        if options.alignments {
            let mut aligned: Vec<(f64, Option<Point>)> = self
                .points
                .iter()
                .filter(|q| off(**q) > tolerance && near(foot(**q)))
                .map(|q| (foot(*q), Some(*q)))
                .collect();
            if let Some((s, q)) = best(&mut aligned) {
                return snap(s, SnapKind::Alignment, q);
            }
        }
        snap(t, kind, None)
    }

    /// `p` moved onto the vertical or the horizontal of a nearby point, or
    /// both.
    fn aligned(&self, p: Point, from: Option<Point>, tolerance: f64) -> Option<Snap> {
        let candidates = || self.points.iter().copied().chain(from);
        let nearest = |gap: &dyn Fn(Point) -> f64| {
            candidates()
                .filter(|q| gap(*q) <= tolerance && distance(*q, p) > tolerance)
                .min_by(|a, b| distance(*a, p).total_cmp(&distance(*b, p)))
        };
        let vertical = nearest(&|q| (q.x - p.x).abs());
        let horizontal = nearest(&|q| (q.y - p.y).abs());
        if vertical.is_none() && horizontal.is_none() {
            return None;
        }
        let point = Point::new(
            vertical.map_or(p.x, |q| q.x),
            horizontal.map_or(p.y, |q| q.y),
        );
        Some(Snap {
            point,
            kind: SnapKind::Alignment,
            guides: vertical
                .into_iter()
                .chain(horizontal)
                .map(|q| (q, point))
                .collect(),
        })
    }

    /// The nearest point on an edge, and that edge.
    fn on_edge(&self, p: Point, tolerance: f64) -> Option<(Point, (Point, Point))> {
        self.segments
            .iter()
            .filter_map(|&(a, b)| {
                let ab = sub(b, a);
                let l2 = dot(ab, ab);
                (l2 > 0.0).then(|| {
                    let q = add(a, mul(ab, (dot(sub(p, a), ab) / l2).clamp(0.0, 1.0)));
                    (q, (a, b))
                })
            })
            .filter(|(q, _)| distance(*q, p) <= tolerance)
            .min_by(|a, b| distance(a.0, p).total_cmp(&distance(b.0, p)))
    }
}

/// Where two guide lines cross, near `p`.
fn crossing_lines(lines: &[Line], p: Point, tolerance: f64) -> Option<Snap> {
    let mut best: Option<Snap> = None;
    for (i, l) in lines.iter().enumerate() {
        for m in &lines[i + 1..] {
            let Some(s) = line_line(l.anchors[0], l.u, m.anchors[0], m.u) else {
                continue;
            };
            let point = add(l.anchors[0], mul(l.u, s));
            let better = best
                .as_ref()
                .is_none_or(|b| distance(point, p) < distance(b.point, p));
            if distance(point, p) <= tolerance && better {
                best = Some(Snap {
                    point,
                    kind: SnapKind::Intersection,
                    guides: vec![(l.anchor(point), point), (m.anchor(point), point)],
                });
            }
        }
    }
    best
}

/// How far along the line from `a` in direction `u` the infinite line
/// through `c` in direction `v` crosses it, if they are not parallel.
fn line_line(a: Point, u: Point, c: Point, v: Point) -> Option<f64> {
    let den = cross(u, v);
    (den.abs() > 1e-9).then(|| cross(sub(c, a), v) / den)
}

fn unit(v: Point) -> Option<Point> {
    let l = dot(v, v).sqrt();
    (l > 1e-9).then(|| mul(v, 1.0 / l))
}

/// How far along the line from `start` in direction `u` the segment `ab`
/// crosses it, if it does.
fn line_cross(start: Point, u: Point, a: Point, b: Point) -> Option<f64> {
    let ab = sub(b, a);
    let den = cross(u, ab);
    if den.abs() < 1e-12 {
        return None;
    }
    let w = sub(a, start);
    let along = cross(w, ab) / den;
    let on = cross(w, u) / den;
    (-1e-9..=1.0 + 1e-9).contains(&on).then_some(along)
}

/// Where segments `ab` and `cd` cross, ends included.
fn crossing(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    let (r, s) = (sub(b, a), sub(d, c));
    let den = cross(r, s);
    if den.abs() < 1e-12 {
        return None;
    }
    let qp = sub(c, a);
    let (t, u) = (cross(qp, s) / den, cross(qp, r) / den);
    let e = 1e-9;
    ((-e..=1.0 + e).contains(&t) && (-e..=1.0 + e).contains(&u)).then(|| add(a, mul(r, t)))
}

fn add(a: Point, b: Point) -> Point {
    Point::new(a.x + b.x, a.y + b.y)
}
fn sub(a: Point, b: Point) -> Point {
    Point::new(a.x - b.x, a.y - b.y)
}
fn mul(a: Point, k: f64) -> Point {
    Point::new(a.x * k, a.y * k)
}
fn dot(a: Point, b: Point) -> f64 {
    a.x * b.x + a.y * b.y
}
fn cross(a: Point, b: Point) -> f64 {
    a.x * b.y - a.y * b.x
}
fn distance(a: Point, b: Point) -> f64 {
    let d = sub(a, b);
    dot(d, d).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    type Xy = (f64, f64);

    fn scene(points: &[Xy], segments: &[(Xy, Xy)]) -> Scene {
        let p = |(x, y): Xy| Point::new(x, y);
        Scene {
            points: points.iter().map(|q| p(*q)).collect(),
            segments: segments.iter().map(|(a, b)| (p(*a), p(*b))).collect(),
            ..Scene::default()
        }
    }

    fn close(a: Point, x: f64, y: f64) -> bool {
        (a.x - x).abs() < 1e-6 && (a.y - y).abs() < 1e-6
    }

    #[test]
    fn shift_locks_the_nearest_45_degrees() {
        let s = scene(&[], &[]);
        let o = SnapOptions::default();
        let start = Some(Point::new(0.0, 0.0));
        let r = s.snap(&o, Point::new(1000.0, 300.0), start, true, 10.0);
        assert!(close(r.point, 1000.0, 0.0), "{r:?}");
        let r = s.snap(&o, Point::new(1000.0, 800.0), start, true, 10.0);
        assert!((r.point.x - r.point.y).abs() < 1e-6, "45 degrees: {r:?}");
        // Without shift, far from any direction, the pointer is free.
        let r = s.snap(&o, Point::new(1000.0, 300.0), start, false, 10.0);
        assert_eq!(r.kind, SnapKind::Free);
        // Close to a direction, it snaps on its own.
        let r = s.snap(&o, Point::new(1000.0, 5.0), start, false, 10.0);
        assert!(close(r.point, 1000.0, 0.0) && r.kind == SnapKind::Direction);
    }

    #[test]
    fn on_its_axis_the_wall_stops_level_with_a_parallel_wall_end() {
        // A parallel wall ends at x = 3000, two metres away.
        let s = scene(&[(0.0, 2000.0), (3000.0, 2000.0)], &[]);
        let o = SnapOptions::default();
        let r = s.snap(
            &o,
            Point::new(2994.0, 40.0),
            Some(Point::new(0.0, 0.0)),
            true,
            10.0,
        );
        assert!(close(r.point, 3000.0, 0.0), "{r:?}");
        assert_eq!(r.kind, SnapKind::Alignment);
        assert!(
            r.guides.iter().any(|g| close(g.0, 3000.0, 2000.0)),
            "guide from the end"
        );
    }

    #[test]
    fn points_come_first_then_alignments_then_edges() {
        let s = scene(
            &[(0.0, 0.0), (5000.0, 3000.0)],
            &[((0.0, -1000.0), (0.0, 1000.0))],
        );
        let o = SnapOptions::default();
        let r = s.snap(&o, Point::new(4.0, 3.0), None, false, 10.0);
        assert_eq!(r.kind, SnapKind::Point);
        // Level with both points: the crossing of the two alignments.
        let r = s.snap(&o, Point::new(4996.0, 6.0), None, false, 10.0);
        assert!(
            close(r.point, 5000.0, 0.0) && r.kind == SnapKind::Alignment,
            "{r:?}"
        );
        let r = s.snap(&o, Point::new(6.0, 500.0), None, false, 10.0);
        assert!(
            close(r.point, 0.0, 500.0),
            "on the alignment or the edge: {r:?}"
        );
        let off = SnapOptions {
            alignments: false,
            ..o
        };
        let r = s.snap(&off, Point::new(6.0, 500.0), None, false, 10.0);
        assert_eq!(r.kind, SnapKind::Edge);
        let none = SnapOptions {
            points: false,
            midpoints: false,
            intersections: false,
            edges: false,
            angles: false,
            alignments: false,
            guides: false,
            ..o
        };
        let r = s.snap(&none, Point::new(4.0, 3.0), None, false, 10.0);
        assert_eq!(r.kind, SnapKind::Free, "every aid can be turned off");
    }

    /// A wall face at 30 degrees, picked up.
    fn oblique() -> Scene {
        let (sin, cos) = 30f64.to_radians().sin_cos();
        let mut s = scene(&[], &[]);
        s.pick(&[(Point::new(0.0, 0.0), Point::new(4000.0 * cos, 4000.0 * sin))]);
        s
    }

    #[test]
    fn a_picked_edge_gives_its_extension_and_its_perpendiculars() {
        let s = oblique();
        let o = SnapOptions::default();
        let (sin, cos) = 30f64.to_radians().sin_cos();
        // Beyond its end, on its extension.
        let far = Point::new(6000.0 * cos - 5.0 * sin, 6000.0 * sin + 5.0 * cos);
        let r = s.snap(&o, far, None, false, 10.0);
        assert_eq!(r.kind, SnapKind::Guide, "{r:?}");
        assert!(close(r.point, 6000.0 * cos, 6000.0 * sin));
        // Square to it at its start.
        let r = s.snap(
            &o,
            Point::new(-1000.0 * sin + 4.0 * cos, 1000.0 * cos + 4.0 * sin),
            None,
            false,
            10.0,
        );
        assert_eq!(r.kind, SnapKind::Guide);
        assert!(close(r.point, -1000.0 * sin, 1000.0 * cos), "{r:?}");
        let off = SnapOptions { guides: false, ..o };
        assert_eq!(s.snap(&off, far, None, false, 10.0).kind, SnapKind::Free);
    }

    #[test]
    fn a_wall_drawn_from_elsewhere_runs_parallel_or_stops_square() {
        let s = oblique();
        let o = SnapOptions::default();
        let (sin, cos) = 30f64.to_radians().sin_cos();
        let start = Point::new(0.0, 2000.0);
        // Nearly parallel to the picked edge: locked parallel.
        let p = Point::new(
            start.x + 2500.0 * cos - 6.0 * sin,
            start.y + 2500.0 * sin + 6.0 * cos,
        );
        let r = s.snap(&o, p, Some(start), false, 10.0);
        assert_eq!(r.kind, SnapKind::Parallel, "{r:?}");
        assert!(close(
            r.point,
            start.x + 2500.0 * cos,
            start.y + 2500.0 * sin
        ));
        // Shift also locks onto the parallel.
        let p = Point::new(start.x + 2500.0 * cos, start.y + 2500.0 * sin + 200.0);
        assert_eq!(
            s.snap(&o, p, Some(start), true, 10.0).kind,
            SnapKind::Parallel
        );
        // Near the foot of the perpendicular from the start: square to it.
        let foot = Point::new(2000.0 * sin * cos, 2000.0 * sin * sin);
        let r = s.snap(
            &o,
            Point::new(foot.x + 4.0, foot.y),
            Some(start),
            false,
            10.0,
        );
        assert_eq!(r.kind, SnapKind::Perpendicular, "{r:?}");
        assert!(close(r.point, foot.x, foot.y));
    }

    #[test]
    fn edges_divide_and_angles_follow_the_step() {
        let mut s = scene(&[], &[]);
        s.divided
            .push((Point::new(0.0, 0.0), Point::new(3000.0, 0.0)));
        let o = SnapOptions {
            divisions: 3,
            ..SnapOptions::default()
        };
        let r = s.snap(&o, Point::new(1004.0, 3.0), None, false, 10.0);
        assert!(
            close(r.point, 1000.0, 0.0) && r.kind == SnapKind::Midpoint,
            "{r:?}"
        );
        let fifteen = SnapOptions { step: 15, ..o };
        let r = s.snap(
            &fifteen,
            Point::new(1000.0, 280.0),
            Some(Point::new(0.0, 0.0)),
            true,
            10.0,
        );
        let angle = r.point.y.atan2(r.point.x).to_degrees();
        assert!((angle - 15.0).abs() < 1e-6, "{angle}");
    }
}
