//! Drawing aids: where the pointer lands once it snaps to points, edges,
//! angles, alignments and guide lines, and the dashed lines that show why.
//!
//! An edge the pointer passes over is picked up: until two others replace
//! it, it offers guide lines (its extension and its perpendiculars at both
//! ends), directions parallel and perpendicular to it, and the foot of the
//! perpendicular from the last point placed.
//!
//! Curved walls give their arcs, the middle of each, and from the last
//! point placed the points where a line touches the arc (tangent) or meets
//! it square (through the centre).
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
    /// Middles of every edge and arc, or their divisions.
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
    /// Guide lines, parallels and perpendiculars from picked-up edges, and
    /// tangents and perpendiculars to arcs.
    pub guides: bool,
    /// The angle step, in degrees: 15, 30, 45 or 90.
    pub step: u16,
    /// Into how many equal parts edges and arcs are divided: 2 gives their
    /// middles.
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
    /// The point where a line from the last point touches an arc.
    Tangent,
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
    /// The straight guide the point lies on, if any: where it starts and
    /// its unit direction, so that a length typed before the first point
    /// measures along it.
    pub along: Option<(Point, Point)>,
}

impl Snap {
    fn new(point: Point, kind: SnapKind, guides: Vec<(Point, Point)>) -> Self {
        Self {
            point,
            kind,
            guides,
            along: None,
        }
    }

    /// The same snap, lying on the guide from `origin` through its point.
    fn from(mut self, origin: Point) -> Self {
        self.along = unit(sub(self.point, origin)).map(|u| (origin, u));
        self
    }
}

/// A circular arc: the reference line or a face of a curved wall.
#[derive(Clone, Copy, Debug)]
struct Arc {
    centre: Point,
    radius: f64,
    /// Direction of the start from the centre, in radians.
    from: f64,
    /// Turn from the start to the end, anticlockwise when positive.
    sweep: f64,
}

impl Arc {
    fn at(&self, a: f64) -> Point {
        Point::new(
            self.centre.x + self.radius * a.cos(),
            self.centre.y + self.radius * a.sin(),
        )
    }

    /// Whether the direction `a` from the centre lies within the arc.
    fn spans(&self, a: f64) -> bool {
        let tau = std::f64::consts::TAU;
        let turn = if self.sweep >= 0.0 {
            (a - self.from).rem_euclid(tau)
        } else {
            (self.from - a).rem_euclid(tau)
        };
        turn <= self.sweep.abs() + 1e-9
    }

    /// The point of the arc nearest `p`, if `p` is within its span.
    fn nearest(&self, p: Point) -> Option<Point> {
        let a = direction(self.centre, p)?;
        self.spans(a).then(|| self.at(a))
    }
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
    /// Straight edges, divided for the middle points.
    divided: Vec<(Point, Point)>,
    intersections: Vec<Point>,
    segments: Vec<(Point, Point)>,
    picked: Vec<(Point, Point)>,
    arcs: Vec<Arc>,
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
            if let Some((centre, r)) = w.arc() {
                s.points.push(centre);
                let Some(from) = direction(centre, w.start) else {
                    continue;
                };
                // The left of an anticlockwise arc is towards the centre.
                let inward = if w.sweep > 0.0 { -1.0 } else { 1.0 };
                let left = r + inward * w.left_face_shift();
                let right = left - inward * w.thickness();
                for radius in [r, left, right].into_iter().filter(|r| *r > 1e-9) {
                    let arc = Arc {
                        centre,
                        radius,
                        from,
                        sweep: w.sweep,
                    };
                    s.points.extend([arc.at(from), arc.at(from + w.sweep)]);
                    s.arcs.push(arc);
                }
                continue;
            }
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
                lines.push((a, b));
            }
        }
        let outlines = project.roofs.iter().map(|r| r.outline.clone()).chain(
            project
                .meshes
                .iter()
                .map(|m| m.outline.iter().map(|q| q.at).collect()),
        );
        for outline in outlines {
            let n = outline.len();
            s.points.extend(outline.iter().copied());
            lines.extend((0..n).map(|i| (outline[i], outline[(i + 1) % n])));
        }
        for stair in &project.stairs {
            s.points.extend(stair.path.iter().copied());
            lines.extend(stair.path.windows(2).map(|w| (w[0], w[1])));
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
        s.divided.clone_from(&s.segments);
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

    /// Whether the edge from `a` to `b` is still in the scene.
    pub fn has_edge(&self, edge: (Point, Point)) -> bool {
        self.segments.contains(&edge)
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
        let free = |kind, point| Snap::new(point, kind, Vec::new());
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
            if let Some(s) = self.curved(options, p, from, tolerance) {
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
            let anchor = l.anchor(point);
            return Snap::new(point, SnapKind::Guide, vec![(anchor, point)]).from(anchor);
        }
        if options.alignments
            && let Some(s) = self.aligned(p, from, tolerance)
        {
            return s;
        }
        if options.edges {
            let arc = self
                .arcs
                .iter()
                .filter_map(|a| a.nearest(p))
                .filter(|q| distance(*q, p) <= tolerance)
                .min_by(|a, b| distance(*a, p).total_cmp(&distance(*b, p)));
            match (self.on_edge(p, tolerance), arc) {
                (Some((q, _)), Some(c)) if distance(c, p) < distance(q, p) => {
                    return free(SnapKind::Edge, c);
                }
                // On a straight edge, a typed length runs from its nearer end.
                (Some((q, (a, b))), _) => {
                    let end = if distance(a, q) <= distance(b, q) {
                        a
                    } else {
                        b
                    };
                    return free(SnapKind::Edge, q).from(end);
                }
                (None, Some(c)) => return free(SnapKind::Edge, c),
                (None, None) => {}
            }
        }
        free(SnapKind::Free, p)
    }

    fn on_point(&self, options: &SnapOptions, p: Point, tolerance: f64) -> Option<Snap> {
        let n = options.divisions.max(2);
        let part = move |k: u8| f64::from(k) / f64::from(n);
        let divisions: Vec<Point> = self
            .divided
            .iter()
            .flat_map(|&(a, b)| (1..n).map(move |k| add(a, mul(sub(b, a), part(k)))))
            .chain(
                self.arcs
                    .iter()
                    .flat_map(|c| (1..n).map(move |k| c.at(c.from + c.sweep * part(k)))),
            )
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
            .map(|(point, kind)| Snap::new(point, kind, Vec::new()))
    }

    /// From the last point, the points of an arc where a line touches it,
    /// and where the line through its centre meets it square.
    fn curved(
        &self,
        options: &SnapOptions,
        p: Point,
        from: Option<Point>,
        tolerance: f64,
    ) -> Option<Snap> {
        let start = from.filter(|_| options.guides)?;
        let pi = std::f64::consts::PI;
        let mut found = Vec::new();
        for arc in &self.arcs {
            let Some(towards) = direction(arc.centre, start) else {
                continue;
            };
            for a in [towards, towards + pi] {
                found.push((a, SnapKind::Perpendicular, arc));
            }
            let d = distance(start, arc.centre);
            if d > arc.radius + 1e-9 {
                let half = (arc.radius / d).acos();
                for a in [towards + half, towards - half] {
                    found.push((a, SnapKind::Tangent, arc));
                }
            }
        }
        found
            .into_iter()
            .filter(|(a, _, arc)| arc.spans(*a))
            .map(|(a, kind, arc)| (arc.at(a), kind, arc.centre))
            .filter(|(q, ..)| distance(*q, p) <= tolerance && distance(*q, start) > tolerance)
            .min_by(|x, y| distance(x.0, p).total_cmp(&distance(y.0, p)))
            .map(|(point, kind, centre)| {
                let mut guides = vec![(start, point)];
                if kind == SnapKind::Perpendicular {
                    guides.push((centre, point));
                }
                Snap::new(point, kind, guides)
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
            .map(|(point, anchor)| {
                Snap::new(
                    point,
                    SnapKind::Perpendicular,
                    vec![(start, point), (anchor, point)],
                )
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
    // ponytail: eight plain arguments; group them in a ray struct if the
    // ray gains more.
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
            Snap::new(point, kind, guides)
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
            .chain(
                self.arcs
                    .iter()
                    .filter(|_| options.edges || options.intersections)
                    .flat_map(|c| circle_cross(start, u, c))
                    .map(|s| (s, None)),
            )
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
        let snap = Snap::new(
            point,
            SnapKind::Alignment,
            vertical
                .into_iter()
                .chain(horizontal)
                .map(|q| (q, point))
                .collect(),
        );
        // Lined up with one point only: a typed length runs from it.
        Some(match (vertical, horizontal) {
            (Some(q), None) | (None, Some(q)) => snap.from(q),
            _ => snap,
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
                best = Some(Snap::new(
                    point,
                    SnapKind::Intersection,
                    vec![(l.anchor(point), point), (m.anchor(point), point)],
                ));
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

/// How far along the line from `start` in direction `u` it crosses the
/// arc `c`, at most twice.
fn circle_cross(start: Point, u: Point, c: &Arc) -> Vec<f64> {
    let w = sub(start, c.centre);
    let b = dot(u, w);
    let disc = b * b - (dot(w, w) - c.radius * c.radius);
    if disc < 0.0 {
        return Vec::new();
    }
    let root = disc.sqrt();
    [-b - root, -b + root]
        .into_iter()
        .filter(|s| direction(c.centre, add(start, mul(u, *s))).is_some_and(|a| c.spans(a)))
        .collect()
}

/// The direction of `p` from `centre`, in radians.
fn direction(centre: Point, p: Point) -> Option<f64> {
    let v = sub(p, centre);
    (dot(v, v) > 1e-18).then(|| v.y.atan2(v.x))
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

    fn pt(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn a_picked_edge_of_zero_length_offers_nothing() {
        let mut s = scene(&[], &[]);
        s.pick(&[(pt(100.0, 100.0), pt(100.0, 100.0))]);
        let o = SnapOptions::default();
        assert!(s.lines().is_empty(), "no direction, no guide line");
        for from in [None, Some(pt(0.0, 0.0))] {
            for shift in [false, true] {
                let r = s.snap(&o, pt(500.0, 300.0), from, shift, 10.0);
                assert!(r.point.x.is_finite() && r.point.y.is_finite(), "{r:?}");
                assert!(!matches!(r.kind, SnapKind::Guide | SnapKind::Parallel));
            }
        }
    }

    #[test]
    fn a_zero_length_edge_is_never_picked_under_the_pointer() {
        let s = scene(
            &[],
            &[((50.0, 50.0), (50.0, 50.0)), ((0.0, 0.0), (1000.0, 0.0))],
        );
        assert_eq!(s.edge_near(pt(50.0, 52.0), 10.0), None);
        let edge = s.edge_near(pt(500.0, 4.0), 10.0);
        assert_eq!(edge, Some((pt(0.0, 0.0), pt(1000.0, 0.0))));
        assert_eq!(s.edge_near(pt(500.0, 40.0), 10.0), None);
    }

    #[test]
    fn two_parallel_picked_edges_have_no_crossing_of_their_own() {
        let mut s = scene(&[], &[]);
        s.pick(&[
            (pt(0.0, 3000.0), pt(4000.0, 3000.0)),
            (pt(0.0, 0.0), pt(4000.0, 0.0)),
        ]);
        let o = SnapOptions::default();
        assert!(line_line(pt(0.0, 0.0), pt(1.0, 0.0), pt(0.0, 3000.0), pt(1.0, 0.0)).is_none());
        // Between them, nothing: the two extensions never meet.
        let r = s.snap(&o, pt(6000.0, 1500.0), None, false, 10.0);
        assert_eq!(r.kind, SnapKind::Free, "{r:?}");
        // Where an extension meets the other edge's perpendicular, they do.
        let r = s.snap(&o, pt(4003.0, 2998.0), None, false, 10.0);
        assert_eq!(r.kind, SnapKind::Intersection, "{r:?}");
        assert!(close(r.point, 4000.0, 3000.0));
        // The same edge twice does not crash.
        let e = (pt(0.0, 0.0), pt(4000.0, 0.0));
        s.pick(&[e, e]);
        let _ = s.snap(&o, pt(1.0, 1.0), Some(pt(5.0, 5.0)), false, 10.0);
    }

    #[test]
    fn a_horizontal_picked_edge_gives_the_angle_direction_not_a_parallel() {
        let mut s = scene(&[], &[]);
        s.pick(&[(pt(0.0, 0.0), pt(4000.0, 0.0))]);
        let o = SnapOptions::default();
        let r = s.snap(&o, pt(2500.0, 2004.0), Some(pt(0.0, 2000.0)), false, 10.0);
        assert_eq!(r.kind, SnapKind::Direction, "{r:?}");
        assert!(close(r.point, 2500.0, 2000.0));
        // With the angles off, the same direction is a parallel.
        let no_angles = SnapOptions { angles: false, ..o };
        let r = s.snap(
            &no_angles,
            pt(2500.0, 2004.0),
            Some(pt(0.0, 2000.0)),
            false,
            10.0,
        );
        assert_eq!(r.kind, SnapKind::Parallel, "{r:?}");
        assert!(close(r.point, 2500.0, 2000.0));
    }

    #[test]
    fn the_foot_of_the_perpendicular_is_not_offered_from_a_point_on_the_edge() {
        let mut s = scene(&[], &[]);
        s.pick(&[(pt(0.0, 0.0), pt(4000.0, 0.0))]);
        let o = SnapOptions::default();
        // The start lies on the picked edge's line: its foot is itself.
        let r = s.snap(&o, pt(6003.0, 2.0), Some(pt(6000.0, 0.0)), false, 10.0);
        assert_ne!(r.kind, SnapKind::Perpendicular, "{r:?}");
        let r = s.snap(&o, pt(6002.0, 1.0), Some(pt(6000.0, 0.0)), false, 10.0);
        assert_ne!(r.kind, SnapKind::Perpendicular, "{r:?}");
        // From above the edge, on its extension, the foot is offered.
        let r = s.snap(&o, pt(6003.0, 2.0), Some(pt(6000.0, 2000.0)), false, 10.0);
        assert_eq!(r.kind, SnapKind::Perpendicular, "{r:?}");
        assert!(close(r.point, 6000.0, 0.0));
        // Guides off: no foot.
        let off = SnapOptions { guides: false, ..o };
        let r = s.snap(&off, pt(6003.0, 2.0), Some(pt(6000.0, 2000.0)), false, 10.0);
        assert_ne!(r.kind, SnapKind::Perpendicular, "{r:?}");
    }

    #[test]
    fn walls_divide_in_twelve_and_never_in_fewer_than_two() {
        let mut s = scene(&[], &[]);
        s.divided.push((pt(0.0, 0.0), pt(1200.0, 0.0)));
        let twelve = SnapOptions {
            divisions: 12,
            ..SnapOptions::default()
        };
        for k in 1..12 {
            let x = f64::from(k) * 100.0;
            let r = s.snap(&twelve, pt(x + 3.0, 4.0), None, false, 10.0);
            assert!(
                close(r.point, x, 0.0) && r.kind == SnapKind::Midpoint,
                "{k} {r:?}"
            );
        }
        // Zero and one count as two: the middle.
        for n in [0, 1, 2] {
            let o = SnapOptions {
                divisions: n,
                ..SnapOptions::default()
            };
            let r = s.snap(&o, pt(603.0, 4.0), None, false, 10.0);
            assert!(close(r.point, 600.0, 0.0), "{n} {r:?}");
        }
        // The ends are not division points.
        let r = s.snap(&twelve, pt(2.0, 2.0), None, false, 1.0);
        assert_eq!(r.kind, SnapKind::Free);
        // Midpoints off: no division either.
        let off = SnapOptions {
            midpoints: false,
            ..twelve
        };
        let r = s.snap(&off, pt(103.0, 4.0), None, false, 10.0);
        assert_eq!(r.kind, SnapKind::Free);
    }

    #[test]
    fn the_angle_step_of_90_gives_only_the_axes_and_odd_steps_are_clamped() {
        let s = scene(&[], &[]);
        let start = Some(pt(0.0, 0.0));
        let at = |step| SnapOptions {
            step,
            ..SnapOptions::default()
        };
        let r = s.snap(&at(90), pt(1000.0, 700.0), start, true, 10.0);
        assert!(close(r.point, 1000.0, 0.0), "{r:?}");
        let r = s.snap(&at(90), pt(700.0, 1000.0), start, true, 10.0);
        assert!(close(r.point, 0.0, 1000.0), "{r:?}");
        // 45 degrees is no longer a direction at step 90.
        let r = s.snap(&at(90), pt(1000.0, 1004.0), start, false, 10.0);
        assert_eq!(r.kind, SnapKind::Free, "{r:?}");
        // A step of 0 does not divide by zero; above 90 acts as 90.
        for step in [0, 200, u16::MAX] {
            let r = s.snap(&at(step), pt(1000.0, 300.0), start, true, 10.0);
            assert!(
                r.point.x.is_finite() && r.point.y.is_finite(),
                "{step} {r:?}"
            );
        }
        let r = s.snap(&at(200), pt(1000.0, 700.0), start, true, 10.0);
        assert!(close(r.point, 1000.0, 0.0), "{r:?}");
        // Backwards too.
        let r = s.snap(&at(90), pt(-1000.0, 20.0), start, true, 10.0);
        assert!(close(r.point, -1000.0, 0.0), "{r:?}");
    }

    #[test]
    fn guides_and_angles_are_turned_off_independently() {
        let s = oblique();
        let on = SnapOptions::default();
        let (sin, cos) = 30f64.to_radians().sin_cos();
        let start = pt(0.0, 2000.0);
        let along = pt(start.x + 2500.0 * cos, start.y + 2500.0 * sin + 4.0);
        let level = pt(2500.0, 2004.0);
        // Guides off, angles on: the horizontal still locks, the parallel not.
        let no_guides = SnapOptions {
            guides: false,
            ..on
        };
        let r = s.snap(&no_guides, level, Some(start), false, 10.0);
        assert_eq!(r.kind, SnapKind::Direction, "{r:?}");
        let r = s.snap(&no_guides, along, Some(start), false, 10.0);
        assert_eq!(r.kind, SnapKind::Free, "{r:?}");
        // Shift then locks the nearest angle, never a picked direction.
        let r = s.snap(&no_guides, along, Some(start), true, 10.0);
        assert_eq!(r.kind, SnapKind::Direction, "{r:?}");
        // Angles off, guides on: the parallel locks, the horizontal not.
        let no_angles = SnapOptions {
            angles: false,
            ..on
        };
        let r = s.snap(&no_angles, along, Some(start), false, 10.0);
        assert_eq!(r.kind, SnapKind::Parallel, "{r:?}");
        // (Alignments off too: the level would otherwise align with the start.)
        let bare = SnapOptions {
            alignments: false,
            ..no_angles
        };
        let r = s.snap(&bare, level, Some(start), false, 10.0);
        assert_eq!(r.kind, SnapKind::Free, "{r:?}");
        // Both off, Shift still locks the nearest angle.
        let neither = SnapOptions {
            angles: false,
            guides: false,
            ..on
        };
        let r = s.snap(&neither, pt(1000.0, 300.0), Some(pt(0.0, 0.0)), true, 10.0);
        assert!(close(r.point, 1000.0, 0.0), "{r:?}");
    }

    #[test]
    fn forgetting_the_picked_edges_removes_their_guides() {
        let mut s = oblique();
        let o = SnapOptions::default();
        let (sin, cos) = 30f64.to_radians().sin_cos();
        let far = pt(6000.0 * cos - 5.0 * sin, 6000.0 * sin + 5.0 * cos);
        assert_eq!(s.snap(&o, far, None, false, 10.0).kind, SnapKind::Guide);
        s.pick(&[]);
        assert_eq!(s.snap(&o, far, None, false, 10.0).kind, SnapKind::Free);
    }

    /// A quarter of a circle of radius 3000 round the origin, anticlockwise
    /// from the x axis, or clockwise when `sweep` is negative.
    fn quarter(sweep: f64) -> Scene {
        Scene {
            arcs: vec![Arc {
                centre: pt(0.0, 0.0),
                radius: 3000.0,
                from: 0.0,
                sweep,
            }],
            ..Scene::default()
        }
    }

    // ponytail: drawing cases have no runner under their own name in
    // archibim-commands (it cannot reach the ui crate); this test stands
    // for tests/cases/drawing/drawing-instruments.yaml.
    #[test]
    fn drawing_instruments() {
        let half = std::f64::consts::FRAC_PI_2;
        let s = quarter(half);
        let o = SnapOptions::default();
        let k = 3000.0 * std::f64::consts::FRAC_1_SQRT_2;
        // On the arc, and its middle.
        let r = s.snap(&o, pt(2700.0, 1310.0), None, false, 10.0);
        assert!(
            (distance(r.point, pt(0.0, 0.0)) - 3000.0).abs() < 1e-6,
            "{r:?}"
        );
        assert_eq!(r.kind, SnapKind::Edge);
        let r = s.snap(&o, pt(k + 4.0, k - 3.0), None, false, 10.0);
        assert!(
            close(r.point, k, k) && r.kind == SnapKind::Midpoint,
            "{r:?}"
        );
        // From (6000, 0) a line touches the arc at 60 degrees.
        let start = Some(pt(6000.0, 0.0));
        let touch = pt(1500.0, 3000.0 * 60f64.to_radians().sin());
        let r = s.snap(&o, pt(touch.x + 5.0, touch.y + 4.0), start, false, 10.0);
        assert!(close(r.point, touch.x, touch.y), "{r:?}");
        assert_eq!(r.kind, SnapKind::Tangent);
        assert!(r.guides.contains(&(pt(6000.0, 0.0), r.point)));
        // The other tangent, at -60 degrees, is off the arc.
        let r = s.snap(&o, pt(1505.0, -touch.y), start, false, 10.0);
        assert_eq!(r.kind, SnapKind::Free, "{r:?}");
        // Square to the arc: on the line from the last point to the centre.
        let from = pt(6000.0, 1000.0);
        let a = from.y.atan2(from.x);
        let square = pt(3000.0 * a.cos(), 3000.0 * a.sin());
        let r = s.snap(&o, pt(square.x - 6.0, square.y), Some(from), false, 10.0);
        assert!(close(r.point, square.x, square.y), "{r:?}");
        assert_eq!(r.kind, SnapKind::Perpendicular);
        // From inside the circle there is no tangent.
        let inside = Some(pt(1000.0, 0.0));
        let r = s.snap(&o, pt(touch.x + 5.0, touch.y + 4.0), inside, false, 10.0);
        assert_ne!(r.kind, SnapKind::Tangent, "{r:?}");
        // Guides off: no tangent; edges off: no point on the arc.
        let off = SnapOptions { guides: false, ..o };
        let r = s.snap(&off, pt(touch.x + 5.0, touch.y + 4.0), start, false, 10.0);
        assert_ne!(r.kind, SnapKind::Tangent, "{r:?}");
        let off = SnapOptions { edges: false, ..o };
        let r = s.snap(&off, pt(2700.0, 1310.0), None, false, 10.0);
        assert_eq!(r.kind, SnapKind::Free, "{r:?}");
        // Going straight up from below, the line stops on the arc.
        let r = s.snap(
            &o,
            pt(1500.0, 2590.0),
            Some(pt(1500.0, -5000.0)),
            true,
            10.0,
        );
        assert!(close(r.point, touch.x, touch.y), "{r:?}");
        assert_eq!(r.kind, SnapKind::Intersection);
        // A clockwise quarter lies below the x axis.
        let s = quarter(-half);
        let r = s.snap(&o, pt(1505.0, -touch.y), start, false, 10.0);
        assert_eq!(r.kind, SnapKind::Tangent, "{r:?}");
    }

    #[test]
    fn a_point_on_a_guide_or_an_edge_knows_where_it_starts() {
        let mut s = scene(&[], &[((0.0, 0.0), (4000.0, 0.0))]);
        let o = SnapOptions::default();
        // On the edge, nearer its start.
        let r = s.snap(&o, pt(1000.0, 3.0), None, false, 10.0);
        assert_eq!(r.kind, SnapKind::Edge);
        assert_eq!(r.along, Some((pt(0.0, 0.0), pt(1.0, 0.0))), "{r:?}");
        // On the extension of a picked edge, from its end.
        s.pick(&[(pt(0.0, 0.0), pt(1000.0, 0.0))]);
        let r = s.snap(&o, pt(6000.0, 4.0), None, false, 10.0);
        assert_eq!(r.kind, SnapKind::Guide, "{r:?}");
        let Some((origin, u)) = r.along else {
            panic!("{r:?}")
        };
        assert!(close(origin, 1000.0, 0.0) && close(u, 1.0, 0.0), "{r:?}");
        // Free: no guide.
        let r = s.snap(&o, pt(2000.0, 3000.0), None, false, 10.0);
        assert_eq!(r.along, None, "{r:?}");
    }

    #[test]
    fn every_straight_edge_has_a_middle() {
        let mut s = scene(&[], &[]);
        // A beam side, a roof edge, a stair line: all are divided alike.
        s.divided.extend([
            (pt(0.0, 0.0), pt(0.0, 2000.0)),
            (pt(5000.0, 0.0), pt(7000.0, 0.0)),
        ]);
        let o = SnapOptions::default();
        for (x, y) in [(0.0, 1000.0), (6000.0, 0.0)] {
            let r = s.snap(&o, pt(x + 3.0, y + 3.0), None, false, 10.0);
            assert!(
                close(r.point, x, y) && r.kind == SnapKind::Midpoint,
                "{r:?}"
            );
        }
    }

    fn curved_wall(sweep: f64, thickness: f64) -> archibim_model::Wall {
        use archibim_model::*;
        let level = LevelRef {
            story: Id(2),
            kind: LevelKind::Structural,
            offset: 0.0,
        };
        Wall {
            id: Id(1),
            story: Id(2),
            start: pt(3000.0, 0.0),
            end: pt(0.0, 3000.0),
            reference: WallReference::Axis,
            offset: 0.0,
            sweep,
            layers: vec![Layer {
                role: LayerRole::LoadBearing,
                material: Id(3),
                thickness,
                status: RenovationStatus::New,
                framing: None,
            }],
            bottom: level,
            top: level,
        }
    }

    #[test]
    fn curved_walls_give_arcs_even_thick_or_nearly_closed() {
        let o = SnapOptions::default();
        let quarter = std::f64::consts::FRAC_PI_2;
        let tau = std::f64::consts::TAU;
        for (sweep, thickness) in [
            (quarter, 200.0),
            (-quarter, 200.0),
            // Thicker than the radius: the inner face has no radius.
            (quarter, 9000.0),
            (tau - 0.01, 200.0),
            (-(tau - 0.01), 200.0),
        ] {
            let mut project = Project::new();
            project.walls.push(curved_wall(sweep, thickness));
            let s = Scene::new(&project, &[pt(0.0, 0.0)]);
            for a in &s.arcs {
                assert!(a.radius > 0.0 && a.radius.is_finite(), "{a:?}");
            }
            let r = s.snap(&o, pt(100.0, 100.0), Some(pt(9000.0, 50.0)), false, 10.0);
            assert!(r.point.x.is_finite() && r.point.y.is_finite(), "{r:?}");
        }
    }

    #[test]
    fn an_arc_close_to_a_full_turn_spans_almost_every_direction() {
        let tau = std::f64::consts::TAU;
        for sweep in [tau - 0.01, -(tau - 0.01)] {
            let arc = Arc {
                centre: pt(0.0, 0.0),
                radius: 1.0,
                from: 0.0,
                sweep,
            };
            let gap = if sweep > 0.0 { -0.005 } else { 0.005 };
            assert!(arc.spans(0.0) && arc.spans(3.0) && arc.spans(-3.0));
            assert!(!arc.spans(gap), "the missing sliver");
        }
    }

    #[test]
    fn the_last_point_on_the_centre_or_the_circle_gives_no_tangent() {
        let s = quarter(std::f64::consts::FRAC_PI_2);
        let o = SnapOptions::default();
        let r = s.snap(&o, pt(5.0, 5.0), Some(pt(0.0, 0.0)), false, 10.0);
        assert!(r.point.x.is_finite(), "{r:?}");
        assert_ne!(r.kind, SnapKind::Tangent);
        // On the circle: the tangent point is the last point itself.
        let r = s.snap(&o, pt(3002.0, 3.0), Some(pt(3000.0, 0.0)), false, 10.0);
        assert_ne!(r.kind, SnapKind::Tangent, "{r:?}");
        assert!(r.point.x.is_finite() && r.point.y.is_finite(), "{r:?}");
    }

    #[test]
    fn the_tangent_point_at_the_very_end_of_an_arc_is_found() {
        // From (6000, 0) the line touches at 60 degrees: an arc of 60 degrees
        // ends exactly there.
        let s = quarter(60f64.to_radians());
        let touch = pt(1500.0, 3000.0 * 60f64.to_radians().sin());
        let r = s.snap(
            &SnapOptions::default(),
            pt(touch.x + 3.0, touch.y + 2.0),
            Some(pt(6000.0, 0.0)),
            false,
            10.0,
        );
        assert_eq!(r.kind, SnapKind::Tangent, "{r:?}");
    }

    #[test]
    fn degenerate_edges_and_divisions_do_not_break_the_snap() {
        let mut s = scene(&[], &[((0.0, 0.0), (0.0, 0.0)), ((0.0, 0.0), (0.0, 0.0))]);
        // A stair path with a repeated point, and a very short edge.
        s.divided.push((pt(5.0, 5.0), pt(5.0, 5.0)));
        s.divided.push((pt(100.0, 0.0), pt(100.5, 0.0)));
        for divisions in [0, 1, 2, 255] {
            let o = SnapOptions {
                divisions,
                ..SnapOptions::default()
            };
            for from in [None, Some(pt(0.0, 0.0)), Some(pt(5.0, 5.0))] {
                for shift in [false, true] {
                    let r = s.snap(&o, pt(100.2, 0.1), from, shift, 10.0);
                    assert!(r.point.x.is_finite() && r.point.y.is_finite(), "{r:?}");
                }
            }
        }
        assert!(s.edge_near(pt(0.0, 0.0), 10.0).is_none());
    }
}
