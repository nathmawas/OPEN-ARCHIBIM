//! Shapes derived from the model: wall layers in plan and their junctions,
//! outlines of openings, columns and beams, triangles for fills, and the
//! volumes of the 3D view.
//!
//! Pure functions: nothing here keeps state or draws.

use archibim_model::{
    Beam, Column, Framing, Id, JunctionMode, LayerRole, LevelKind, LevelRef, ModelError, Point,
    Project, RenovationStatus, Wall,
};

/// Which state of the building to draw (renovation filter).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The building before works: existing layers and layers to demolish.
    Existing,
    /// The building after works: existing layers and new layers.
    Proposed,
}

impl Phase {
    fn shows(self, status: RenovationStatus) -> bool {
        match self {
            Self::Existing => status != RenovationStatus::New,
            Self::Proposed => status != RenovationStatus::ToDemolish,
        }
    }
}

/// Two points closer than this are the same point, in millimetres.
const TOL: f64 = 1.0;

/// The plan shape of one layer of one wall: one or more polygons, split
/// where other walls pass through it.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerPlan {
    /// The wall.
    pub wall: Id,
    /// Index of the layer in the wall's build-up.
    pub layer: usize,
    /// The pieces of the layer, each a closed polygon.
    pub polygons: Vec<Vec<Point>>,
    /// The framing members within the layer (studs, joists), each a closed
    /// polygon drawn over the layer.
    pub members: Vec<Vec<Point>>,
}

/// The plan shapes of every wall layer of a story, with junctions resolved.
///
/// - Corners (two walls ending at the same point) meet on a mitre, unless
///   the user chose a wall to pass in front.
/// - A wall ending against another stops layer by layer: each of its layers
///   passes through the layers of lower priority and stops at the first
///   layer of equal or higher priority.
/// - Two crossing walls: the wall with the highest priority layer is
///   continuous; the other stops on both sides as above.
// ponytail: recomputes the whole story; recompute only changed walls and
// their neighbours when the performance budget needs it (ADR 0003).
pub fn plan_layers(
    project: &Project,
    story: Id,
    phase: Phase,
) -> Result<Vec<LayerPlan>, ModelError> {
    let mut states = Vec::new();
    for wall in project.walls.iter().filter(|w| w.story == story) {
        states.push(WallState::new(project, wall, phase)?);
    }
    for i in 0..states.len() {
        for j in i + 1..states.len() {
            let mode = project.junction_mode(states[i].id, states[j].id);
            resolve(&mut states, i, j, mode);
        }
    }
    // Openings cut every layer of their wall across its whole thickness.
    // ponytail: no reveal returns yet (case openings-reveal-returns).
    for opening in &project.openings {
        if let Some(state) = states.iter_mut().find(|s| s.id == opening.wall) {
            let across = |t: f64| Line {
                p: add(state.frame.start, mul(state.frame.d, t)),
                d: state.frame.r,
            };
            let gap = (
                across(opening.position),
                across(opening.position + opening.wall_opening_width()),
            );
            for band in &mut state.bands {
                band.gaps.push(gap);
            }
        }
    }
    Ok(states
        .iter()
        .flat_map(|s| {
            s.bands
                .iter()
                .zip(&s.layer_index)
                .map(|(band, &layer)| LayerPlan {
                    wall: s.id,
                    layer,
                    polygons: band.pieces(&s.frame),
                    members: band.members(&s.frame),
                })
        })
        .collect())
}

/// The hole of an opening in plan: a rectangle across the whole thickness
/// of its wall, as four corners (start face, then far face).
pub fn opening_outline(project: &Project, opening: Id) -> Result<Vec<Point>, ModelError> {
    let o = project.opening(opening)?;
    let wall = project.wall(o.wall)?;
    let (start, end) = wall
        .left_face()
        .ok_or(ModelError::Invalid("a wall has no length"))?;
    let d = unit(sub(end, start)).ok_or(ModelError::Invalid("a wall has no length"))?;
    let r = Point::new(d.y, -d.x);
    let a = add(start, mul(d, o.position));
    let b = add(start, mul(d, o.position + o.wall_opening_width()));
    let t = wall.thickness();
    Ok(vec![a, b, add(b, mul(r, t)), add(a, mul(r, t))])
}

/// A column in plan, placed by its anchor and turned by its angle.
pub fn column_outline(column: &Column) -> Vec<Point> {
    let x0 = column.anchor.width.first_edge(column.width);
    let y0 = column.anchor.depth.first_edge(column.depth);
    let (x1, y1) = (x0 + column.width, y0 + column.depth);
    let (sin, cos) = column.angle.sin_cos();
    let c = column.at;
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        .iter()
        .map(|&(x, y)| Point::new(c.x + x * cos - y * sin, c.y + x * sin + y * cos))
        .collect()
}

/// A beam in plan, placed across by its anchor, or nothing if it has no
/// length.
pub fn beam_outline(beam: &Beam) -> Option<Vec<Point>> {
    let d = unit(sub(beam.end, beam.start))?;
    // Measured towards the right face, from the reference line.
    let r = Point::new(d.y, -d.x);
    let near = beam.anchor.width.first_edge(beam.width);
    let (a, b) = (mul(r, near), mul(r, near + beam.width));
    Some(vec![
        add(beam.start, a),
        add(beam.end, a),
        add(beam.end, b),
        add(beam.start, b),
    ])
}

/// The bottom and top of a beam whose reference line is at `level`.
fn beam_heights(beam: &Beam, level: f64) -> (f64, f64) {
    // Measured down from the reference line: first is the top face.
    let top = level - beam.anchor.depth.first_edge(beam.height);
    (top - beam.height, top)
}

/// A straight extrusion for the 3D view: an outline in plan between two
/// heights.
#[derive(Clone, Debug, PartialEq)]
pub struct Prism {
    /// The element it belongs to.
    pub element: Id,
    /// The role of its layer, for walls and slabs; none for columns and
    /// beams.
    pub role: Option<LayerRole>,
    /// The outline in plan, closed.
    pub outline: Vec<Point>,
    /// Height of the underside.
    pub bottom: f64,
    /// Height of the top.
    pub top: f64,
}

/// The volumes of every element of every story: each wall layer from the
/// bottom to the top of its wall, with the wall kept under the sill and over
/// the head of its openings; each slab layer under the slab's top level;
/// columns from foot to head; beams under their top level.
// ponytail: openings cut straight through, no reveal returns, no frame or
// glass yet; walls do not stop under slabs.
pub fn model_prisms(project: &Project, phase: Phase) -> Result<Vec<Prism>, ModelError> {
    let mut prisms = Vec::new();
    let mut push = |element, role, outline, bottom: f64, top: f64| {
        if top > bottom {
            prisms.push(Prism {
                element,
                role,
                outline,
                bottom,
                top,
            });
        }
    };
    for story in &project.stories {
        for layer in plan_layers(project, story.id, phase)? {
            let wall = project.wall(layer.wall)?;
            let (bottom, top) = (project.level(&wall.bottom)?, project.level(&wall.top)?);
            let role = wall.layers.get(layer.layer).map(|l| l.role);
            for outline in layer.polygons {
                push(wall.id, role, outline, bottom, top);
            }
        }
    }
    for opening in &project.openings {
        let wall = project.wall(opening.wall)?;
        let (bottom, top) = (project.level(&wall.bottom)?, project.level(&wall.top)?);
        let floor = project.level(&LevelRef {
            story: wall.story,
            kind: LevelKind::Finished,
            offset: 0.0,
        })?;
        let (sill, head) = (floor + opening.sill, floor + opening.sill + opening.height);
        let outline = opening_outline(project, opening.id)?;
        let (a, b) = (outline[0], outline[1]);
        let r = unit(sub(outline[3], a)).unwrap_or(Point::new(0.0, 0.0));
        let mut from = 0.0;
        for layer in wall.layers.iter().filter(|l| phase.shows(l.status)) {
            let to = from + layer.thickness;
            let strip = vec![
                add(a, mul(r, from)),
                add(b, mul(r, from)),
                add(b, mul(r, to)),
                add(a, mul(r, to)),
            ];
            push(
                wall.id,
                Some(layer.role),
                strip.clone(),
                bottom,
                sill.min(top),
            );
            push(wall.id, Some(layer.role), strip, head.max(bottom), top);
            from = to;
        }
    }
    for slab in &project.slabs {
        let mut top = project.level(&slab.top)?;
        for layer in slab.layers.iter().filter(|l| phase.shows(l.status)) {
            let bottom = top - layer.thickness;
            push(slab.id, Some(layer.role), slab.outline.clone(), bottom, top);
            top = bottom;
        }
    }
    for column in &project.columns {
        let (bottom, top) = (project.level(&column.bottom)?, project.level(&column.top)?);
        push(column.id, None, column_outline(column), bottom, top);
    }
    for beam in &project.beams {
        if let Some(outline) = beam_outline(beam) {
            let (bottom, top) = beam_heights(beam, project.level(&beam.level)?);
            push(beam.id, None, outline, bottom, top);
        }
    }
    Ok(prisms)
}

/// Is `p` inside the closed polygon?
pub fn contains(polygon: &[Point], p: Point) -> bool {
    let n = polygon.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (polygon[i], polygon[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

/// The edges to draw around a set of fills: an edge, or the part of it,
/// that runs along a fill of the same material is left out, so that equal
/// materials read as one continuous mass across junctions. Each piece comes
/// with the index of the polygon it belongs to.
// ponytail: O(e²) over every edge of the story; group by material or use
// a spatial index when stories reach thousands of layers.
pub fn visible_edges(polygons: &[(Id, &[Point])]) -> Vec<(usize, Point, Point)> {
    const TOLERANCE: f64 = 0.5;
    let edges = |polygon: &'_ [Point]| {
        let n = polygon.len();
        (0..n)
            .map(move |i| (polygon[i], polygon[(i + 1) % n]))
            .collect::<Vec<_>>()
    };
    let mut visible = Vec::new();
    for (index, (material, polygon)) in polygons.iter().enumerate() {
        for (a, b) in edges(polygon) {
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let length = (dx * dx + dy * dy).sqrt();
            if length < TOLERANCE {
                continue;
            }
            // Covered stretches of [0, 1] along a→b.
            let mut covered: Vec<(f64, f64)> = Vec::new();
            for (other, (m, q)) in polygons.iter().enumerate() {
                if other == index || m != material {
                    continue;
                }
                for (c, d) in edges(q) {
                    let off = |p: Point| ((p.x - a.x) * dy - (p.y - a.y) * dx).abs() / length;
                    if off(c) > TOLERANCE || off(d) > TOLERANCE {
                        continue;
                    }
                    let t = |p: Point| ((p.x - a.x) * dx + (p.y - a.y) * dy) / (length * length);
                    let (t0, t1) = (t(c).min(t(d)).max(0.0), t(c).max(t(d)).min(1.0));
                    if t1 > t0 {
                        covered.push((t0, t1));
                    }
                }
            }
            covered.sort_by(|x, y| x.0.total_cmp(&y.0));
            let at = |t: f64| Point::new(a.x + dx * t, a.y + dy * t);
            let mut from = 0.0;
            for (t0, t1) in covered {
                if (t0 - from) * length > TOLERANCE {
                    visible.push((index, at(from), at(t0)));
                }
                from = f64::max(from, t1);
            }
            if (1.0 - from) * length > TOLERANCE {
                visible.push((index, at(from), b));
            }
        }
    }
    visible
}

/// Splits a simple polygon (convex or not) into triangles, as indices into
/// it. Ear clipping.
// ponytail: O(n³) ear clipping; fine for slab outlines of a few dozen
// points.
pub fn triangulate(polygon: &[Point]) -> Vec<[usize; 3]> {
    let n = polygon.len();
    let mut left: Vec<usize> = (0..n).collect();
    let signed: f64 = (0..n)
        .map(|i| cross(polygon[i], polygon[(i + 1) % n]))
        .sum();
    if signed < 0.0 {
        left.reverse();
    }
    let mut triangles = Vec::new();
    while left.len() > 3 {
        let m = left.len();
        let ear = (0..m).find(|&k| {
            let (a, b, c) = (
                polygon[left[(k + m - 1) % m]],
                polygon[left[k]],
                polygon[left[(k + 1) % m]],
            );
            cross(sub(b, a), sub(c, b)) > 1e-9
                && left.iter().all(|&q| {
                    let p = polygon[q];
                    p == a || p == b || p == c || !contains(&[a, b, c], p)
                })
        });
        let Some(k) = ear else {
            break;
        };
        triangles.push([left[(k + m - 1) % m], left[k], left[(k + 1) % m]]);
        left.remove(k);
    }
    if left.len() == 3 {
        triangles.push([left[0], left[1], left[2]]);
    }
    triangles
}

/// Area of a closed polygon.
pub fn polygon_area(polygon: &[Point]) -> f64 {
    let n = polygon.len();
    let twice: f64 = (0..n)
        .map(|i| {
            let (a, b) = (polygon[i], polygon[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    twice.abs() / 2.0
}

// ---------------------------------------------------------------------------
// Vectors and lines

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
fn length(a: Point) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: Point) -> Option<Point> {
    let l = length(a);
    (l > 1e-9).then(|| mul(a, 1.0 / l))
}

/// An infinite line through `p` with direction `d`.
#[derive(Clone, Copy, Debug)]
struct Line {
    p: Point,
    d: Point,
}

/// Distance along `v` from `p` to where the ray meets `line`.
fn ray_t(p: Point, v: Point, line: Line) -> Option<f64> {
    let den = cross(v, line.d);
    (den.abs() > 1e-12).then(|| cross(sub(line.p, p), line.d) / den)
}

fn intersect(a: Line, b: Line) -> Option<Point> {
    ray_t(a.p, a.d, b).map(|t| add(a.p, mul(a.d, t)))
}

// ---------------------------------------------------------------------------
// Walls

/// The axes of a wall: `d` along it, `r` towards its right face.
struct Frame {
    start: Point,
    d: Point,
    r: Point,
    len: f64,
    /// Offsets of the layer faces from the reference line, from 0 to the
    /// total thickness.
    offsets: Vec<f64>,
}

impl Frame {
    fn line(&self, offset: f64) -> Line {
        Line {
            p: add(self.start, mul(self.r, offset)),
            d: self.d,
        }
    }
    fn t(&self, p: Point) -> f64 {
        dot(sub(p, self.start), self.d)
    }
    fn s(&self, p: Point) -> f64 {
        dot(sub(p, self.start), self.r)
    }
    fn thickness(&self) -> f64 {
        self.offsets.last().copied().unwrap_or(0.0)
    }
}

/// One layer as a band between two lines, cut at both ends and interrupted
/// by gaps where other walls pass through.
#[derive(Clone)]
struct Band {
    lo: Line,
    hi: Line,
    mid: Line,
    start: Line,
    end: Line,
    gaps: Vec<(Line, Line)>,
    framing: Option<Framing>,
}

impl Band {
    /// The cut lines bounding each piece of the band, in order along the
    /// wall.
    fn cut_pairs(&self, frame: &Frame) -> Vec<(Line, Line)> {
        let at = |cut: Line| intersect(self.mid, cut).map(|p| frame.t(p));
        let mut gaps: Vec<(f64, Line, Line)> = self
            .gaps
            .iter()
            .filter_map(|&(a, b)| {
                let (ta, tb) = (at(a)?, at(b)?);
                Some(if ta <= tb { (ta, a, b) } else { (tb, b, a) })
            })
            .collect();
        gaps.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut cuts = vec![self.start];
        for (_, enter, exit) in gaps {
            cuts.extend([enter, exit]);
        }
        cuts.push(self.end);
        cuts.chunks(2).map(|c| (c[0], c[1])).collect()
    }

    fn quad(&self, a: Line, b: Line) -> Option<Vec<Point>> {
        let quad = vec![
            intersect(self.lo, a)?,
            intersect(self.lo, b)?,
            intersect(self.hi, b)?,
            intersect(self.hi, a)?,
        ];
        (polygon_area(&quad) > TOL).then_some(quad)
    }

    fn pieces(&self, frame: &Frame) -> Vec<Vec<Point>> {
        self.cut_pairs(frame)
            .into_iter()
            .filter_map(|(a, b)| self.quad(a, b))
            .collect()
    }

    /// Framing members: one at each end of the wall and one every `spacing`
    /// from the start, kept only where they fit inside a piece of the layer.
    fn members(&self, frame: &Frame) -> Vec<Vec<Point>> {
        let Some(f) = self.framing else {
            return Vec::new();
        };
        let half = f.width / 2.0;
        let at = |cut: Line| intersect(self.mid, cut).map(|p| frame.t(p));
        let ranges: Vec<(f64, f64)> = self
            .cut_pairs(frame)
            .into_iter()
            .filter_map(|(a, b)| {
                let (ta, tb) = (at(a)?, at(b)?);
                Some((ta.min(tb), ta.max(tb)))
            })
            .collect();
        let mut centres = Vec::new();
        let mut c = half;
        while c <= frame.len - half + 1e-9 {
            centres.push(c);
            c += f.spacing;
        }
        let last = frame.len - half;
        if centres.last().is_some_and(|&c| last - c >= f.width) {
            centres.push(last);
        }
        let across = |t: f64| Line {
            p: add(frame.start, mul(frame.d, t)),
            d: frame.r,
        };
        centres
            .into_iter()
            .filter(|&c| {
                ranges
                    .iter()
                    .any(|&(a, b)| c - half >= a - TOL && c + half <= b + TOL)
            })
            .filter_map(|c| self.quad(across(c - half), across(c + half)))
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum End {
    Start,
    End,
}

struct WallState {
    id: Id,
    /// The ends of the reference line, where walls join.
    reference: [Point; 2],
    frame: Frame,
    priorities: Vec<u8>,
    bands: Vec<Band>,
    /// Index in the wall's build-up of each band.
    layer_index: Vec<usize>,
}

impl WallState {
    fn new(project: &Project, wall: &Wall, phase: Phase) -> Result<Self, ModelError> {
        let (start, end) = wall
            .left_face()
            .ok_or(ModelError::Invalid("a wall has no length"))?;
        let d = unit(sub(end, start)).ok_or(ModelError::Invalid("a wall has no length"))?;
        let r = Point::new(d.y, -d.x);
        let mut offsets = vec![0.0];
        let mut priorities = Vec::new();
        let (layer_index, shown): (Vec<usize>, Vec<&archibim_model::Layer>) = wall
            .layers
            .iter()
            .enumerate()
            .filter(|(_, l)| phase.shows(l.status))
            .unzip();
        let framings: Vec<Option<Framing>> = shown.iter().map(|l| l.framing).collect();
        for layer in &shown {
            offsets.push(offsets[offsets.len() - 1] + layer.thickness);
            priorities.push(project.material(layer.material)?.priority);
        }
        let frame = Frame {
            start,
            d,
            r,
            len: length(sub(end, start)),
            offsets,
        };
        let bands = (0..shown.len())
            .map(|i| {
                let (a, b) = (frame.offsets[i], frame.offsets[i + 1]);
                Band {
                    lo: frame.line(a),
                    hi: frame.line(b),
                    mid: frame.line((a + b) / 2.0),
                    start: Line { p: start, d: r },
                    end: Line { p: end, d: r },
                    gaps: Vec::new(),
                    framing: framings[i],
                }
            })
            .collect();
        Ok(Self {
            id: wall.id,
            reference: [wall.start, wall.end],
            frame,
            priorities,
            layer_index,
            bands,
        })
    }

    fn ends(&self) -> [(Point, End); 2] {
        [
            (self.reference[0], End::Start),
            (self.reference[1], End::End),
        ]
    }

    /// Direction from an end into the body of the wall.
    fn body(&self, end: End) -> Point {
        match end {
            End::Start => self.frame.d,
            End::End => mul(self.frame.d, -1.0),
        }
    }

    fn cut(&mut self, end: End, layer: Option<usize>, line: Line) {
        for (i, band) in self.bands.iter_mut().enumerate() {
            if layer.is_none_or(|l| l == i) {
                match end {
                    End::Start => band.start = line,
                    End::End => band.end = line,
                }
            }
        }
    }

    /// Does `p` touch this wall away from its ends?
    fn touches(&self, p: Point) -> bool {
        let (t, s) = (self.frame.t(p), self.frame.s(p));
        t > TOL && t < self.frame.len - TOL && s > -TOL && s < self.frame.thickness() + TOL
    }

    /// The face line among this wall's faces that lies furthest from `p`
    /// along `v`.
    fn furthest_face(&self, p: Point, v: Point) -> Line {
        let faces = [
            self.frame.line(0.0),
            self.frame.line(self.frame.thickness()),
        ];
        let t = |l: Line| ray_t(p, v, l).unwrap_or(f64::MIN);
        if t(faces[0]) >= t(faces[1]) {
            faces[0]
        } else {
            faces[1]
        }
    }

    /// Where a layer of priority `priority`, coming from `body` (direction from
    /// this wall into the other wall's body), stops in this wall, and which
    /// layers of this wall it passes through.
    fn stop(&self, body: Point, priority: u8) -> (Line, Vec<usize>) {
        let from_right = dot(body, self.frame.r) > 0.0;
        let n = self.bands.len();
        let order: Vec<usize> = if from_right {
            (0..n).rev().collect()
        } else {
            (0..n).collect()
        };
        let mut passed = Vec::new();
        for i in order {
            if self.priorities[i] >= priority {
                let face = if from_right {
                    self.frame.offsets[i + 1]
                } else {
                    self.frame.offsets[i]
                };
                return (self.frame.line(face), passed);
            }
            passed.push(i);
        }
        let far = if from_right {
            0.0
        } else {
            self.frame.thickness()
        };
        (self.frame.line(far), passed)
    }

    fn max_priority(&self) -> u8 {
        self.priorities.iter().copied().max().unwrap_or(0)
    }
}

/// Two walls meeting at a shared end point.
fn corner(
    states: &mut [WallState],
    (i, ei): (usize, End),
    (j, ej): (usize, End),
    p: Point,
    mode: Option<JunctionMode>,
) {
    let (a, b) = (&states[i], &states[j]);
    let (cut_a, cut_b) = match mode {
        Some(JunctionMode::InFront(front)) => {
            let front_is_a = front == a.id;
            let (f, fe, o, oe) = if front_is_a {
                (a, ei, b, ej)
            } else {
                (b, ej, a, ei)
            };
            let front_cut = o.furthest_face(p, mul(f.body(fe), -1.0));
            let other_cut = f.furthest_face(p, o.body(oe));
            if front_is_a {
                (front_cut, other_cut)
            } else {
                (other_cut, front_cut)
            }
        }
        _ => {
            // The mitre joins where the faces on each side meet. Walls
            // running the same way round have their left faces on the same
            // side; walls meeting end to end or start to start, opposite.
            let (ta, tb) = (a.frame.thickness(), b.frame.thickness());
            let (b_left, b_right) = if ei == ej { (tb, 0.0) } else { (0.0, tb) };
            let left = intersect(a.frame.line(0.0), b.frame.line(b_left));
            let right = intersect(a.frame.line(ta), b.frame.line(b_right));
            let mitre = match (left, right) {
                (Some(l), Some(r)) if length(sub(r, l)) > 1e-9 => Line {
                    p: l,
                    d: mul(sub(r, l), 1.0 / length(sub(r, l))),
                },
                _ => match unit(add(a.body(ei), b.body(ej))) {
                    Some(d) => Line { p, d },
                    None => Line { p, d: a.frame.r },
                },
            };
            (mitre, mitre)
        }
    };
    states[i].cut(ei, None, cut_a);
    states[j].cut(ej, None, cut_b);
}

/// Wall `j` ends against the side of wall `i`.
fn tee(states: &mut [WallState], i: usize, (j, ej): (usize, End)) {
    let body = states[j].body(ej);
    for layer in 0..states[j].bands.len() {
        let (line, passed) = states[i].stop(body, states[j].priorities[layer]);
        let band = states[j].bands[layer].clone();
        states[j].cut(ej, Some(layer), line);
        for k in passed {
            states[i].bands[k].gaps.push((band.lo, band.hi));
        }
    }
}

/// Walls `i` and `j` cross each other away from their ends.
fn crossing(states: &mut [WallState], i: usize, j: usize) {
    let (c, d) = if states[i].max_priority() >= states[j].max_priority() {
        (i, j)
    } else {
        (j, i)
    };
    let dir = states[d].frame.d;
    for layer in 0..states[d].bands.len() {
        let priority = states[d].priorities[layer];
        let (before, passed_before) = states[c].stop(mul(dir, -1.0), priority);
        let (after, passed_after) = states[c].stop(dir, priority);
        let band = states[d].bands[layer].clone();
        states[d].bands[layer].gaps.push((before, after));
        for k in passed_before.into_iter().chain(passed_after) {
            states[c].bands[k].gaps.push((band.lo, band.hi));
        }
    }
}

fn resolve(states: &mut [WallState], i: usize, j: usize, mode: Option<JunctionMode>) {
    for (pa, ea) in states[i].ends() {
        for (pb, eb) in states[j].ends() {
            if length(sub(pa, pb)) < TOL {
                corner(states, (i, ea), (j, eb), pa, mode);
                return;
            }
        }
    }
    for (pb, eb) in states[j].ends() {
        if states[i].touches(pb) {
            tee(states, i, (j, eb));
            return;
        }
    }
    for (pa, ea) in states[i].ends() {
        if states[j].touches(pa) {
            tee(states, j, (i, ea));
            return;
        }
    }
    let (a, b) = (&states[i].frame, &states[j].frame);
    if let Some(x) = intersect(Line { p: a.start, d: a.d }, Line { p: b.start, d: b.d }) {
        let inside = |f: &Frame| f.t(x) > TOL && f.t(x) < f.len - TOL;
        if inside(a) && inside(b) {
            crossing(states, i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shared_edge_is_hidden_between_equal_materials_only() {
        let square = |x: f64| {
            vec![
                Point::new(x, 0.0),
                Point::new(x + 1000.0, 0.0),
                Point::new(x + 1000.0, 1000.0),
                Point::new(x, 1000.0),
            ]
        };
        // The right square is taller: only the shared stretch disappears.
        let (left, mut right) = (square(0.0), square(1000.0));
        right[2].y = 2000.0;
        right[3].y = 2000.0;
        let (brick, wood) = (Id(1), Id(2));
        let same = visible_edges(&[(brick, &left), (brick, &right)]);
        assert_eq!(same.len(), 7, "{same:?}");
        assert!(
            same.iter()
                .all(|(_, a, b)| !(a.x == 1000.0 && b.x == 1000.0 && a.y.max(b.y) <= 1000.0))
        );
        assert_eq!(visible_edges(&[(brick, &left), (wood, &right)]).len(), 8);
    }

    #[test]
    fn an_l_shaped_outline_splits_into_triangles_covering_it() {
        let l = [
            Point::new(0.0, 0.0),
            Point::new(4.0, 0.0),
            Point::new(4.0, 2.0),
            Point::new(2.0, 2.0),
            Point::new(2.0, 4.0),
            Point::new(0.0, 4.0),
        ];
        for outline in [l.to_vec(), l.iter().rev().copied().collect()] {
            let triangles = triangulate(&outline);
            assert_eq!(triangles.len(), 4);
            let area: f64 = triangles
                .iter()
                .map(|t| polygon_area(&[outline[t[0]], outline[t[1]], outline[t[2]]]))
                .sum();
            assert!((area - 12.0).abs() < 1e-9, "the triangles cover the L");
        }
        assert!(contains(&l, Point::new(1.0, 3.0)));
        assert!(!contains(&l, Point::new(3.0, 3.0)), "the notch is outside");
    }
}
