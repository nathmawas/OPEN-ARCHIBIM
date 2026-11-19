//! Drawing aids: where the pointer lands once it snaps to points, edges,
//! angles and alignments, and the guide lines that show why.
//!
//! Pure logic: the plan view draws the result.

use archibim_geometry::{beam_outline, column_outline};
use archibim_model::{Point, Project};

/// Which drawing aids are on. All are on by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapOptions {
    /// Ends of walls and beams, corners of faces, slabs and columns.
    pub points: bool,
    /// Middles of walls and slab edges.
    pub midpoints: bool,
    /// Where wall lines and slab edges cross.
    pub intersections: bool,
    /// Anywhere on a face or an edge.
    pub edges: bool,
    /// Directions every 45 degrees from the last point.
    pub angles: bool,
    /// Horizontal and vertical alignments with nearby points, and along
    /// the current direction.
    pub alignments: bool,
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
    /// A middle.
    Midpoint,
    /// A crossing of two lines.
    Intersection,
    /// A point on an edge.
    Edge,
    /// A point on the current direction.
    Direction,
    /// A point aligned with another point.
    Alignment,
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

/// What can be snapped to, gathered from the project and the drawing in
/// progress.
#[derive(Debug, Default)]
pub struct Scene {
    points: Vec<Point>,
    midpoints: Vec<Point>,
    intersections: Vec<Point>,
    segments: Vec<(Point, Point)>,
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
            s.midpoints.push(mid(w.start, w.end));
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
                s.midpoints.push(mid(a, b));
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

    /// Where the pointer at `p` lands. `from` is the last point placed, if
    /// any; `shift` locks the direction from it to the nearest 45 degrees;
    /// `tolerance` is the snapping distance in model units.
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
        if !shift && let Some(s) = self.on_point(options, p, tolerance) {
            return s;
        }
        if let Some(start) = from
            && (shift || options.angles)
            && let Some(u) = direction(start, p)
        {
            let t = dot(sub(p, start), u);
            let on_ray = add(start, mul(u, t));
            if shift || distance(on_ray, p) <= tolerance {
                return self.along(options, start, u, t, tolerance);
            }
        }
        if options.alignments
            && let Some(s) = self.aligned(p, from, tolerance)
        {
            return s;
        }
        if options.edges
            && let Some(q) = self.on_edge(p, tolerance)
        {
            return free(SnapKind::Edge, q);
        }
        free(SnapKind::Free, p)
    }

    fn on_point(&self, options: &SnapOptions, p: Point, tolerance: f64) -> Option<Snap> {
        let sets = [
            (options.points, &self.points, SnapKind::Point),
            (
                options.intersections,
                &self.intersections,
                SnapKind::Intersection,
            ),
            (options.midpoints, &self.midpoints, SnapKind::Midpoint),
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

    /// On the ray from `start` along `u`, near the distance `t`: a point on
    /// the ray, an edge it crosses, or a point it aligns with.
    fn along(&self, options: &SnapOptions, start: Point, u: Point, t: f64, tolerance: f64) -> Snap {
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
            .filter(|(s, _)| near(*s))
            .collect();
        if let Some((s, _)) = best(&mut crossings) {
            return snap(s, SnapKind::Intersection, None);
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
        snap(t, SnapKind::Direction, None)
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

    fn on_edge(&self, p: Point, tolerance: f64) -> Option<Point> {
        self.segments
            .iter()
            .filter_map(|&(a, b)| {
                let ab = sub(b, a);
                let l2 = dot(ab, ab);
                (l2 > 0.0).then(|| add(a, mul(ab, (dot(sub(p, a), ab) / l2).clamp(0.0, 1.0))))
            })
            .filter(|q| distance(*q, p) <= tolerance)
            .min_by(|a, b| distance(*a, p).total_cmp(&distance(*b, p)))
    }
}

/// The unit direction, a multiple of 45 degrees, nearest to the way from
/// `start` to `p`.
fn direction(start: Point, p: Point) -> Option<Point> {
    let v = sub(p, start);
    if dot(v, v) < 1e-12 {
        return None;
    }
    let step = std::f64::consts::FRAC_PI_4;
    let angle = (v.y.atan2(v.x) / step).round() * step;
    Some(Point::new(angle.cos(), angle.sin()))
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

fn mid(a: Point, b: Point) -> Point {
    Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
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
        };
        let r = s.snap(&none, Point::new(4.0, 3.0), None, false, 10.0);
        assert_eq!(r.kind, SnapKind::Free, "every aid can be turned off");
    }
}
