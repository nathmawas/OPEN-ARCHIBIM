//! Shapes of roofs, terrain meshes and stairs.

use archibim_model::{Id, LayerRole, Mesh, MeshPoint, Point, Roof, RoofPlane, Stair};

use crate::{Phase, Prism, add, contains, dot, intersect, length, mul, sub, triangulate, unit};

/// A volume between two flat faces over an outline, each corner with its
/// own bottom and top height: one layer of a roof plane, one triangle of a
/// terrain. A [`Prism`] is a block with level faces.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// The element it belongs to.
    pub element: Id,
    /// The role of its layer, for walls, slabs and roofs.
    pub role: Option<LayerRole>,
    /// The outline in plan, closed.
    pub outline: Vec<Point>,
    /// Height of the underside at each corner.
    pub bottom: Vec<f64>,
    /// Height of the top at each corner.
    pub top: Vec<f64>,
}

impl From<&Prism> for Block {
    fn from(p: &Prism) -> Self {
        let n = p.outline.len();
        Self {
            element: p.element,
            role: p.role,
            outline: p.outline.clone(),
            bottom: vec![p.bottom; n],
            top: vec![p.top; n],
        }
    }
}

// ---------------------------------------------------------------------------
// Roofs

/// One plane of a roof, seen in plan.
#[derive(Clone, Debug, PartialEq)]
pub struct RoofFace {
    /// The outline edge it rises from; `None` for a flat roof.
    pub edge: Option<usize>,
    /// Its outline in plan, overhang included. The edges it shares with
    /// other faces are the ridges, hips and valleys.
    pub polygon: Vec<Point>,
    /// Its slope, in radians.
    pub slope: f64,
}

/// Unit normal of each outline edge, pointing inside the outline.
fn inward_normals(outline: &[Point]) -> Vec<Point> {
    let n = outline.len();
    let sign = crate::polygon_signed_area(outline).signum();
    (0..n)
        .map(|i| {
            let d = unit(sub(outline[(i + 1) % n], outline[i])).unwrap_or(Point::new(1.0, 0.0));
            mul(Point::new(-d.y, d.x), sign)
        })
        .collect()
}

/// The outline pushed out by the overhang of each edge.
pub fn roof_overhang(roof: &Roof) -> Vec<Point> {
    let o = &roof.outline;
    let n = o.len();
    let normals = inward_normals(o);
    let line = |i: usize| {
        let shift = mul(normals[i], -roof.edges.get(i).map_or(0.0, |e| e.overhang));
        crate::Line {
            p: add(o[i], shift),
            d: sub(o[(i + 1) % n], o[i]),
        }
    };
    (0..n)
        .map(|k| {
            let before = (k + n - 1) % n;
            intersect(line(before), line(k)).unwrap_or(line(k).p)
        })
        .collect()
}

/// Height of the roof plane of edge `i` at `p`, above the eaves level.
fn plane_height(roof: &Roof, normals: &[Point], i: usize, p: Point) -> f64 {
    let slope = roof.edges[i].slope.unwrap_or(0.0);
    dot(sub(p, roof.outline[i]), normals[i]) * slope.tan()
}

/// The planes of a roof: each sloped edge rises until it meets the others,
/// so ridges, hips and valleys come out of the slopes alone. A roof with no
/// sloped edge is one flat plane.
// ponytail: each plane is clipped by every other one, which is exact for
// a convex outline; a concave outline (L-shaped house) needs the straight
// skeleton.
pub fn roof_faces(roof: &Roof) -> Vec<RoofFace> {
    let overhang = roof_overhang(roof);
    let normals = inward_normals(&roof.outline);
    let sloped: Vec<usize> = (0..roof.edges.len())
        .filter(|&i| roof.edges[i].slope.is_some())
        .collect();
    if sloped.is_empty() {
        return vec![RoofFace {
            edge: None,
            polygon: overhang,
            slope: 0.0,
        }];
    }
    sloped
        .iter()
        .filter_map(|&i| {
            let mut polygon = overhang.clone();
            for &j in sloped.iter().filter(|&&j| j != i) {
                polygon = clip(&polygon, |p| {
                    plane_height(roof, &normals, i, p) - plane_height(roof, &normals, j, p)
                });
            }
            (crate::polygon_area(&polygon) > 1.0).then(|| RoofFace {
                edge: Some(i),
                polygon,
                slope: roof.edges[i].slope.unwrap_or(0.0),
            })
        })
        .collect()
}

/// The part of `polygon` where the affine function `f` is not positive.
fn clip(polygon: &[Point], f: impl Fn(Point) -> f64) -> Vec<Point> {
    let n = polygon.len();
    let mut out = Vec::new();
    for k in 0..n {
        let (a, b) = (polygon[k], polygon[(k + 1) % n]);
        let (fa, fb) = (f(a), f(b));
        if fa <= 0.0 {
            out.push(a);
        }
        if (fa <= 0.0) != (fb <= 0.0) {
            out.push(add(a, mul(sub(b, a), fa / (fa - fb))));
        }
    }
    out
}

/// The layers of each roof plane, from the top down, the eaves level at
/// `eaves`. Thicknesses are measured square to the plane; the cuts at the
/// eaves and the gables are plumb.
// ponytail: under planes of different slopes the lower layers meet on the
// hip of the top face, not on their own; exact once layers are cut plane by
// plane.
pub fn roof_blocks(roof: &Roof, eaves: f64, phase: Phase) -> Vec<Block> {
    let normals = inward_normals(&roof.outline);
    let mut blocks = Vec::new();
    for face in roof_faces(roof) {
        let cos = face.slope.cos();
        let height = |p: Point| {
            eaves
                + face
                    .edge
                    .map_or(0.0, |i| plane_height(roof, &normals, i, p))
        };
        let lift = match roof.plane {
            RoofPlane::Top => 0.0,
            RoofPlane::Bottom => roof.thickness() / cos,
        };
        let mut from = lift;
        for layer in roof.layers.iter().filter(|l| phase.shows(l.status)) {
            let to = from - layer.thickness / cos;
            blocks.push(Block {
                element: roof.id,
                role: Some(layer.role),
                outline: face.polygon.clone(),
                bottom: face.polygon.iter().map(|&p| height(p) + to).collect(),
                top: face.polygon.iter().map(|&p| height(p) + from).collect(),
            });
            from = to;
        }
    }
    blocks
}

/// The part of a block where `f` is not positive, `f` being given the
/// point, the bottom and the top of each corner and affine along edges;
/// the heights of new corners are taken along the edges. Nothing when
/// less than a square millimetre is left.
fn clip_block(block: &Block, f: impl Fn(Point, f64, f64) -> f64) -> Option<Block> {
    let n = block.outline.len();
    let corner = |k: usize| (block.outline[k], block.bottom[k], block.top[k]);
    let mut out = Block {
        outline: Vec::new(),
        bottom: Vec::new(),
        top: Vec::new(),
        ..block.clone()
    };
    let mut push = |(p, b, t): (Point, f64, f64)| {
        out.outline.push(p);
        out.bottom.push(b);
        out.top.push(t);
    };
    for k in 0..n {
        let (a, b) = (corner(k), corner((k + 1) % n));
        let (fa, fb) = (f(a.0, a.1, a.2), f(b.0, b.1, b.2));
        if fa <= 0.0 {
            push(a);
        }
        if (fa <= 0.0) != (fb <= 0.0) {
            let s = fa / (fa - fb);
            push((
                add(a.0, mul(sub(b.0, a.0), s)),
                a.1 + (b.1 - a.1) * s,
                a.2 + (b.2 - a.2) * s,
            ));
        }
    }
    (out.outline.len() >= 3 && crate::polygon_area(&out.outline) > 1.0).then_some(out)
}

/// For each edge of a convex polygon, a function positive inside it.
fn inside(polygon: &[Point]) -> Vec<impl Fn(Point) -> f64 + use<>> {
    let n = polygon.len();
    let sign = crate::polygon_signed_area(polygon).signum();
    (0..n)
        .map(|k| {
            let (a, b) = (polygon[k], polygon[(k + 1) % n]);
            move |p: Point| {
                let (d, e) = (sub(b, a), sub(p, a));
                (d.x * e.y - d.y * e.x) * sign
            }
        })
        .collect()
}

/// A block interrupted by a slab running from `bottom` to `top`: what lies
/// inside the slab's outline is kept under and over the slab; what lies
/// outside it, such as insulation or render passing in front of the slab's
/// edge, is kept whole.
// ponytail: split exactly along a convex outline; a concave one takes each
// block whole by its middle, which holds when the edge follows the wall's
// layer faces (the floor contour). A sloped top crossing the slab is met
// corner by corner.
pub fn cut_by_slab(block: &Block, outline: &[Point], bottom: f64, top: f64) -> Vec<Block> {
    let interrupt = |b: &Block| {
        let corners = || b.bottom.iter().zip(&b.top);
        let under = Block {
            top: corners().map(|(&lo, &hi)| hi.min(bottom).max(lo)).collect(),
            ..b.clone()
        };
        let over = Block {
            bottom: corners().map(|(&lo, &hi)| lo.max(top).min(hi)).collect(),
            ..b.clone()
        };
        [under, over]
            .into_iter()
            .filter(|p| p.bottom.iter().zip(&p.top).any(|(lo, hi)| hi - lo > 1e-6))
            .collect::<Vec<_>>()
    };
    if !convex(outline) {
        let n = block.outline.len().max(1) as f64;
        let middle = block
            .outline
            .iter()
            .fold(Point::new(0.0, 0.0), |s, p| add(s, mul(*p, 1.0 / n)));
        return if crate::contains(outline, middle) {
            interrupt(block)
        } else {
            vec![block.clone()]
        };
    }
    let edges = inside(outline);
    let mut pieces = Vec::new();
    // Outside edge k and inside the edges before it: each part of the block
    // outside the outline, once.
    for k in 0..edges.len() {
        let mut part = Some(block.clone());
        for e in &edges[..k] {
            part = part.and_then(|b| clip_block(&b, |p, _, _| -e(p)));
        }
        if let Some(out) = part.and_then(|b| clip_block(&b, |p, _, _| edges[k](p))) {
            pieces.push(out);
        }
    }
    let mut within = Some(block.clone());
    for e in &edges {
        within = within.and_then(|b| clip_block(&b, |p, _, _| -e(p)));
    }
    pieces.extend(within.iter().flat_map(interrupt));
    pieces
}

/// Whether a closed polygon turns always the same way.
fn convex(polygon: &[Point]) -> bool {
    let n = polygon.len();
    let turns: Vec<f64> = (0..n)
        .map(|k| {
            let (a, b, c) = (polygon[k], polygon[(k + 1) % n], polygon[(k + 2) % n]);
            let (u, v) = (sub(b, a), sub(c, b));
            u.x * v.y - u.y * v.x
        })
        .filter(|t| t.abs() > 1e-9)
        .collect();
    n >= 3 && (turns.iter().all(|&t| t > 0.0) || turns.iter().all(|&t| t < 0.0))
}

/// A block cut under a roof whose eaves level is at `eaves`: where the
/// roof's underside is lower than the block's top, the top follows it,
/// pane by pane; where it is lower than the bottom, nothing is left; out
/// of the roof, the block is as it was.
// ponytail: the outline pushed out by the overhang and the panes are
// taken as convex, like the roof's own panes; concave roofs need the
// straight skeleton first.
pub fn cut_under_roof(block: &Block, roof: &Roof, eaves: f64) -> Vec<Block> {
    let mut out = Vec::new();
    // Out of the roof: past one edge of its overhang, inside the ones
    // before, so that no part is kept twice.
    let edges = inside(&roof_overhang(roof));
    for k in 0..edges.len() {
        let mut piece = clip_block(block, |p, _, _| edges[k](p));
        for before in &edges[..k] {
            piece = piece.and_then(|b| clip_block(&b, |p, _, _| -before(p)));
        }
        out.extend(piece);
    }
    let normals = inward_normals(&roof.outline);
    for face in roof_faces(roof) {
        let cos = face.slope.cos();
        let lift = match roof.plane {
            RoofPlane::Top => 0.0,
            RoofPlane::Bottom => roof.thickness() / cos,
        };
        let underside = |p: Point| {
            eaves
                + face
                    .edge
                    .map_or(0.0, |i| plane_height(roof, &normals, i, p))
                + lift
                - roof.thickness() / cos
        };
        let mut piece = Some(block.clone());
        for edge in inside(&face.polygon) {
            piece = piece.and_then(|b| clip_block(&b, |p, _, _| -edge(p)));
        }
        let Some(piece) = piece else { continue };
        // Under the roof already: as it is.
        out.extend(clip_block(&piece, |p, _, top| top - underside(p)));
        // Through it: the top lowered onto the underside. A top just on
        // the underside is kept above only, not twice.
        if let Some(mut b) = clip_block(&piece, |p, _, top| underside(p) - top + 1e-6)
            .and_then(|b| clip_block(&b, |p, bottom, _| bottom - underside(p)))
        {
            b.top = b.outline.iter().map(|&p| underside(p)).collect();
            out.push(b);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Terrain meshes

/// The triangles of a terrain: the outline split into triangles, then each
/// inner point splits the triangle it falls in. Returns the points (outline
/// first, then inner points) and the triangles as indices into them. Inner
/// points outside the outline are left out.
// ponytail: no edge flips, so triangles can be long and thin and the
// contours less smooth than a Delaunay triangulation would give; add flips
// (and break lines) when real surveys show it.
pub fn mesh_triangles(mesh: &Mesh) -> (Vec<MeshPoint>, Vec<[usize; 3]>) {
    let mut points = mesh.outline.clone();
    let outline: Vec<Point> = points.iter().map(|p| p.at).collect();
    let mut triangles = triangulate(&outline);
    const ON: f64 = 0.5;
    for inner in &mesh.inner {
        let p = inner.at;
        // On a corner already there: nothing to split.
        if points.iter().any(|q| length(sub(q.at, p)) <= ON) {
            continue;
        }
        let on_edge = |a: usize, b: usize| segment_distance(p, points[a].at, points[b].at) <= ON;
        let Some(t) = triangles.iter().position(|&[a, b, c]| {
            contains(&[points[a].at, points[b].at, points[c].at], p)
                || on_edge(a, b)
                || on_edge(b, c)
                || on_edge(c, a)
        }) else {
            continue;
        };
        let tri = triangles.swap_remove(t);
        // On an edge, both triangles that share it split in two; inside,
        // the triangle splits in three.
        let edge = (0..3).find(|&e| on_edge(tri[e], tri[(e + 1) % 3]));
        let k = points.len();
        points.push(*inner);
        match edge {
            None => {
                let [a, b, c] = tri;
                triangles.extend([[a, b, k], [b, c, k], [c, a, k]]);
            }
            Some(e) => {
                let (a, b, c) = (tri[e], tri[(e + 1) % 3], tri[(e + 2) % 3]);
                triangles.extend([[a, k, c], [k, b, c]]);
                if let Some(n) = triangles
                    .iter()
                    .position(|t| t.contains(&a) && t.contains(&b) && !t.contains(&k))
                {
                    let other = triangles.swap_remove(n);
                    let Some(d) = other.iter().copied().find(|&v| v != a && v != b) else {
                        continue;
                    };
                    triangles.extend([[b, k, d], [k, a, d]]);
                }
            }
        }
    }
    (points, triangles)
}

/// Distance from `p` to the segment from `a` to `b`.
fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let d = sub(b, a);
    let l2 = dot(d, d);
    let k = if l2 > 0.0 {
        (dot(sub(p, a), d) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    length(sub(p, add(a, mul(d, k))))
}

/// The terrain as one block per triangle, from its flat underside to its
/// surface, heights given from `level`.
pub fn mesh_blocks(mesh: &Mesh, level: f64) -> Vec<Block> {
    let (points, triangles) = mesh_triangles(mesh);
    triangles
        .into_iter()
        .map(|t| Block {
            element: mesh.id,
            role: None,
            outline: t.iter().map(|&i| points[i].at).collect(),
            bottom: vec![level + mesh.bottom; 3],
            top: t.iter().map(|&i| level + points[i].z).collect(),
        })
        .collect()
}

/// Contour lines of a terrain every `interval`, heights given from `level`:
/// each piece with the absolute height it follows.
pub fn mesh_contours(mesh: &Mesh, level: f64, interval: f64) -> Vec<(f64, Point, Point)> {
    let (points, triangles) = mesh_triangles(mesh);
    let mut lines = Vec::new();
    if interval <= 0.0 {
        return lines;
    }
    for t in triangles {
        let corner = |i: usize| (points[t[i]].at, level + points[t[i]].z);
        let zs = [corner(0).1, corner(1).1, corner(2).1];
        let (lo, hi) = (
            zs.iter().copied().fold(f64::MAX, f64::min),
            zs.iter().copied().fold(f64::MIN, f64::max),
        );
        let mut h = (lo / interval).ceil() * interval;
        while h < hi {
            let crossings: Vec<Point> = (0..3)
                .filter_map(|e| {
                    let ((a, za), (b, zb)) = (corner(e), corner((e + 1) % 3));
                    ((za >= h) != (zb >= h)).then(|| add(a, mul(sub(b, a), (h - za) / (zb - za))))
                })
                .collect();
            if let [a, b] = crossings[..] {
                lines.push((h, a, b));
            }
            h += interval;
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// Stairs

/// Riser height aimed at when the number of risers is worked out.
// ponytail: one French default; becomes preset data by country (TOOLS.md,
// rule 17).
pub const RISER_TARGET: f64 = 170.0;

/// The steps of a stair.
#[derive(Clone, Debug, PartialEq)]
pub struct StairSteps {
    /// Number of risers; the last one arrives on the upper floor.
    pub risers: u32,
    /// Height of each riser.
    pub riser: f64,
    /// Depth of each tread, along the walking line.
    pub going: f64,
    /// Each tread in plan, from the foot.
    pub treads: Vec<Vec<Point>>,
    /// Height of the top of each tread.
    pub heights: Vec<f64>,
}

impl StairSteps {
    /// Twice the riser plus the going, the comfort measure of a stair.
    pub fn blondel(&self) -> f64 {
        2.0 * self.riser.abs() + self.going
    }
}

/// The steps of a stair from `bottom` to `top`: the risers share the height
/// equally, the treads share the walking line; the upper floor is the last
/// tread. A head below the foot gives a stair going down.
pub fn stair_steps(stair: &Stair, bottom: f64, top: f64) -> StairSteps {
    let rise = top - bottom;
    let risers = stair
        .risers
        .unwrap_or_else(|| ((rise.abs() / RISER_TARGET).round() as u32).max(1));
    let riser = rise / f64::from(risers);
    let length: f64 = stair.path.windows(2).map(|w| length(sub(w[1], w[0]))).sum();
    let treads_count = risers.saturating_sub(1);
    let going = length / f64::from(treads_count.max(1));
    let treads = (0..treads_count)
        .map(|k| {
            let piece = sub_path(&stair.path, f64::from(k) * going, f64::from(k + 1) * going);
            band(&piece, stair.width)
        })
        .collect();
    let heights = (1..=treads_count)
        .map(|k| bottom + f64::from(k) * riser)
        .collect();
    StairSteps {
        risers,
        riser,
        going,
        treads,
        heights,
    }
}

/// The part of a polyline between two distances along it.
fn sub_path(path: &[Point], from: f64, to: f64) -> Vec<Point> {
    let mut out = Vec::new();
    let mut walked = 0.0;
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let l = length(sub(b, a));
        let at = |s: f64| add(a, mul(sub(b, a), ((s - walked) / l).clamp(0.0, 1.0)));
        if l > 0.0 && walked + l >= from && walked <= to {
            if out.is_empty() {
                out.push(at(from));
            }
            out.push(at(to));
        }
        walked += l;
    }
    out.dedup();
    out
}

/// A band of `width` centred on a polyline, as a closed outline; corners
/// are mitred.
fn band(line: &[Point], width: f64) -> Vec<Point> {
    let n = line.len();
    let left = |d: Point| Point::new(-d.y, d.x);
    let normal = |k: usize| {
        let before = (k > 0).then(|| unit(sub(line[k], line[k - 1]))).flatten();
        let after = (k + 1 < n)
            .then(|| unit(sub(line[k + 1], line[k])))
            .flatten();
        match (before, after) {
            (Some(a), Some(b)) => {
                let m = unit(add(left(a), left(b))).unwrap_or(left(a));
                mul(m, 1.0 / dot(m, left(a)).max(0.2))
            }
            (Some(d), None) | (None, Some(d)) => left(d),
            (None, None) => Point::new(0.0, 0.0),
        }
    };
    let half = width / 2.0;
    let mut outline: Vec<Point> = (0..n).map(|k| add(line[k], mul(normal(k), half))).collect();
    outline.extend((0..n).rev().map(|k| sub(line[k], mul(normal(k), half))));
    outline
}

#[cfg(test)]
mod tests {
    use super::*;
    use archibim_model::{
        Layer, LayerRole, LevelKind, LevelRef, RenovationStatus, RoofEdge, RoofPlane,
    };

    fn level() -> LevelRef {
        LevelRef {
            story: Id(1),
            kind: LevelKind::Finished,
            offset: 0.0,
        }
    }

    fn roof(slopes: [Option<f64>; 4], overhang: f64) -> Roof {
        Roof {
            id: Id(9),
            story: Id(1),
            outline: vec![
                Point::new(0.0, 0.0),
                Point::new(10_000.0, 0.0),
                Point::new(10_000.0, 6_000.0),
                Point::new(0.0, 6_000.0),
            ],
            edges: slopes
                .iter()
                .map(|&slope| RoofEdge { slope, overhang })
                .collect(),
            layers: vec![Layer {
                role: LayerRole::LoadBearing,
                material: Id(2),
                thickness: 200.0,
                status: RenovationStatus::New,
                framing: None,
            }],
            eaves: level(),
            plane: RoofPlane::Top,
        }
    }

    #[test]
    fn a_gabled_roof_has_two_planes_meeting_on_the_ridge() {
        let s = Some(45f64.to_radians());
        let r = roof([s, None, s, None], 0.0);
        let faces = roof_faces(&r);
        assert_eq!(faces.len(), 2);
        let blocks = roof_blocks(&r, 0.0, Phase::Proposed);
        let ridge = blocks
            .iter()
            .flat_map(|b| b.top.iter().copied())
            .fold(f64::MIN, f64::max);
        // Half the 6 m span at 45°.
        assert!((ridge - 3_000.0).abs() < 1.0, "{ridge}");
    }

    #[test]
    fn a_hipped_roof_has_four_planes_and_the_overhang_drops_below_the_eaves() {
        let s = Some(30f64.to_radians());
        let r = roof([s; 4], 500.0);
        assert_eq!(roof_faces(&r).len(), 4);
        let low = roof_blocks(&r, 3_000.0, Phase::Proposed)
            .iter()
            .flat_map(|b| b.top.iter().copied())
            .fold(f64::MAX, f64::min);
        assert!((low - (3_000.0 - 500.0 * 30f64.to_radians().tan())).abs() < 1.0);
    }

    #[test]
    fn a_roof_without_slopes_is_flat_and_can_sit_on_its_underside() {
        let mut r = roof([None; 4], 0.0);
        r.plane = RoofPlane::Bottom;
        let blocks = roof_blocks(&r, 1_000.0, Phase::Proposed);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].bottom.iter().all(|&z| (z - 1_000.0).abs() < 1e-6));
        assert!(blocks[0].top.iter().all(|&z| (z - 1_200.0).abs() < 1e-6));
    }

    #[test]
    fn an_inner_point_splits_the_terrain_and_raises_the_contours() {
        let at = |x, y, z| MeshPoint {
            at: Point::new(x, y),
            z,
        };
        let mesh = Mesh {
            id: Id(5),
            story: Id(1),
            outline: vec![
                at(0.0, 0.0, 0.0),
                at(10_000.0, 0.0, 0.0),
                at(10_000.0, 10_000.0, 0.0),
                at(0.0, 10_000.0, 0.0),
            ],
            inner: vec![at(5_000.0, 5_000.0, 2_000.0), at(50_000.0, 0.0, 9.0)],
            level: level(),
            bottom: -1_000.0,
            material: Id(2),
        };
        let (points, triangles) = mesh_triangles(&mesh);
        // The point outside is left out.
        assert_eq!(points.len(), 5);
        assert_eq!(triangles.len(), 4);
        let contours = mesh_contours(&mesh, 0.0, 500.0);
        // 500, 1000 and 1500 around the hill.
        for h in [500.0, 1_000.0, 1_500.0] {
            assert!(contours.iter().any(|c| c.0 == h));
        }
        assert!(contours.iter().all(|c| c.0 < 2_000.0));
        assert_eq!(mesh_blocks(&mesh, 0.0).len(), 4);
    }

    fn stair(path: Vec<Point>, risers: Option<u32>) -> Stair {
        Stair {
            id: Id(7),
            story: Id(1),
            path,
            width: 900.0,
            bottom: level(),
            top: level(),
            risers,
            material: Id(2),
        }
    }

    #[test]
    fn a_straight_stair_shares_the_height_and_the_length() {
        let s = stair(vec![Point::new(0.0, 0.0), Point::new(3_750.0, 0.0)], None);
        let steps = stair_steps(&s, 0.0, 2_800.0);
        // 2.80 m at about 17 cm: 16 risers of 175, 15 treads of 250.
        assert_eq!(steps.risers, 16);
        assert!((steps.riser - 175.0).abs() < 1e-9);
        assert!((steps.going - 250.0).abs() < 1e-9);
        assert_eq!(steps.treads.len(), 15);
        assert!((steps.blondel() - 600.0).abs() < 1e-9);
        assert!((steps.heights[14] - 2_625.0).abs() < 1e-9);
    }

    #[test]
    fn a_tread_at_a_turn_wraps_around_the_corner() {
        let s = stair(
            vec![
                Point::new(0.0, 0.0),
                Point::new(1_000.0, 0.0),
                Point::new(1_000.0, 1_000.0),
            ],
            Some(5),
        );
        let steps = stair_steps(&s, 0.0, 850.0);
        // 2 m in 4 treads of 500: the second ends right on the corner, the
        // treads after it run up the second flight.
        assert_eq!(steps.treads.len(), 4);
        assert_eq!(steps.treads[1].len(), 4);
        let corner = sub_path(&s.path, 750.0, 1_250.0);
        assert_eq!(corner.len(), 3);
        assert_eq!(band(&corner, 900.0).len(), 6);
    }

    fn top_of(blocks: &[Block]) -> f64 {
        blocks
            .iter()
            .flat_map(|b| b.top.iter().copied())
            .fold(f64::MIN, f64::max)
    }

    #[test]
    fn a_roof_outline_clockwise_or_not_gives_the_same_roof() {
        let s = Some(45f64.to_radians());
        let a = roof([s, None, s, None], 300.0);
        // The same rectangle walked the other way: its edges are 2, 1, 0
        // and 3 of the first.
        let mut b = a.clone();
        b.outline.reverse();
        b.edges = vec![a.edges[2], a.edges[1], a.edges[0], a.edges[3]];
        assert_ne!(
            crate::polygon_signed_area(&a.outline).signum(),
            crate::polygon_signed_area(&b.outline).signum()
        );
        for r in [&a, &b] {
            let faces = roof_faces(r);
            assert_eq!(faces.len(), 2);
            let total: f64 = faces.iter().map(|f| crate::polygon_area(&f.polygon)).sum();
            assert!((total - 10_600.0 * 6_600.0).abs() < 1.0, "{total}");
            assert!((top_of(&roof_blocks(r, 0.0, Phase::Proposed)) - 3_000.0).abs() < 1.0);
        }
    }

    #[test]
    fn a_roof_of_gables_only_is_flat_and_a_zero_overhang_keeps_the_outline() {
        let r = roof([None; 4], 400.0);
        let faces = roof_faces(&r);
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0].edge, None);
        assert!((crate::polygon_area(&faces[0].polygon) - 10_800.0 * 6_800.0).abs() < 1.0);
        let r = roof([Some(0.5); 4], 0.0);
        assert_eq!(roof_overhang(&r), r.outline);
    }

    #[test]
    fn a_triangular_roof_peaks_at_the_inradius_and_a_low_slope_does_not_panic() {
        let s = Some(45f64.to_radians());
        let mut r = roof([s; 4], 0.0);
        r.outline = vec![
            Point::new(0.0, 0.0),
            Point::new(6_000.0, 0.0),
            Point::new(0.0, 6_000.0),
        ];
        r.edges.truncate(3);
        assert_eq!(roof_faces(&r).len(), 3);
        let inradius = (12_000.0 - 6_000.0 * 2f64.sqrt()) / 2.0;
        let peak = top_of(&roof_blocks(&r, 0.0, Phase::Proposed));
        assert!((peak - inradius).abs() < 1.0, "{peak} {inradius}");
        let low = 0.001;
        let r = roof([Some(low), None, Some(low), None], 0.0);
        assert_eq!(roof_faces(&r).len(), 2);
        let peak = top_of(&roof_blocks(&r, 0.0, Phase::Proposed));
        assert!((peak - 3_000.0 * low.tan()).abs() < 1e-6, "{peak}");
    }

    #[test]
    fn a_concave_roof_does_not_panic() {
        // L shape: the known limit (see the ponytail note on roof_faces).
        let mut r = roof([Some(0.5); 4], 0.0);
        r.outline = vec![
            Point::new(0.0, 0.0),
            Point::new(8_000.0, 0.0),
            Point::new(8_000.0, 4_000.0),
            Point::new(4_000.0, 4_000.0),
            Point::new(4_000.0, 8_000.0),
            Point::new(0.0, 8_000.0),
        ];
        r.edges = vec![
            RoofEdge {
                slope: Some(0.5),
                overhang: 0.0
            };
            6
        ];
        assert!(!roof_faces(&r).is_empty());
        assert!(!roof_blocks(&r, 0.0, Phase::Proposed).is_empty());
    }

    fn mp(x: f64, y: f64, z: f64) -> MeshPoint {
        MeshPoint {
            at: Point::new(x, y),
            z,
        }
    }

    fn mesh_of(outline: Vec<MeshPoint>, inner: Vec<MeshPoint>) -> Mesh {
        Mesh {
            id: Id(5),
            story: Id(1),
            outline,
            inner,
            level: level(),
            bottom: -1_000.0,
            material: Id(2),
        }
    }

    fn square(z: f64) -> Vec<MeshPoint> {
        vec![
            mp(0.0, 0.0, z),
            mp(10_000.0, 0.0, z),
            mp(10_000.0, 10_000.0, z),
            mp(0.0, 10_000.0, z),
        ]
    }

    #[test]
    fn a_flat_terrain_has_no_contours() {
        for z in [0.0, 500.0, 750.0] {
            let m = mesh_of(square(z), vec![mp(5_000.0, 2_000.0, z)]);
            assert!(mesh_contours(&m, 0.0, 500.0).is_empty(), "z {z}");
        }
        let m = mesh_of(square(0.0), vec![mp(5_000.0, 2_000.0, 900.0)]);
        assert!(mesh_contours(&m, 0.0, 0.0).is_empty());
        assert!(mesh_contours(&m, 0.0, -5.0).is_empty());
    }

    #[test]
    fn an_inner_point_on_the_diagonal_splits_both_triangles() {
        // The centre of a square lies on the diagonal of the first split.
        let m = mesh_of(square(0.0), vec![mp(5_000.0, 5_000.0, 2_000.0)]);
        let (points, triangles) = mesh_triangles(&m);
        assert_eq!(points.len(), 5);
        assert!(
            triangles.iter().all(|t| t.contains(&4)),
            "a triangle is left whole beside the new point: {triangles:?}"
        );
    }

    #[test]
    fn inner_points_on_the_outline_or_on_a_vertex_do_not_panic() {
        let on_edge = mesh_of(square(0.0), vec![mp(5_000.0, 0.0, 1_000.0)]);
        let (points, triangles) = mesh_triangles(&on_edge);
        assert!(!triangles.is_empty());
        let on_vertex = mesh_of(square(0.0), vec![mp(0.0, 0.0, 1_000.0)]);
        assert!(!mesh_triangles(&on_vertex).1.is_empty());
        for m in [&on_edge, &on_vertex] {
            for (h, a, b) in mesh_contours(m, 0.0, 250.0) {
                assert!(h.is_finite() && a.x.is_finite() && b.y.is_finite());
            }
            assert_eq!(mesh_blocks(m, 0.0).len(), mesh_triangles(m).1.len());
        }
        // Reported, not asserted: whether the edge point is kept.
        eprintln!("edge point kept: {}", points.len() == 5);
    }

    #[test]
    fn a_concave_terrain_is_covered_by_its_triangles() {
        let outline = vec![
            mp(0.0, 0.0, 0.0),
            mp(8_000.0, 0.0, 0.0),
            mp(8_000.0, 4_000.0, 0.0),
            mp(4_000.0, 4_000.0, 0.0),
            mp(4_000.0, 8_000.0, 0.0),
            mp(0.0, 8_000.0, 0.0),
        ];
        // One point in each arm, one in the notch (outside).
        let m = mesh_of(
            outline,
            vec![
                mp(6_000.0, 2_000.0, 1_000.0),
                mp(2_000.0, 6_000.0, 1_000.0),
                mp(6_000.0, 6_000.0, 3_000.0),
            ],
        );
        let (points, triangles) = mesh_triangles(&m);
        assert_eq!(points.len(), 8, "the notch point is left out");
        let area: f64 = triangles
            .iter()
            .map(|t| crate::polygon_area(&t.map(|i| points[i].at)))
            .sum();
        assert!((area - 48_000_000.0).abs() < 1.0, "{area}");
        assert!(!mesh_contours(&m, 0.0, 500.0).is_empty());
    }

    fn line(len: f64) -> Vec<Point> {
        vec![Point::new(0.0, 0.0), Point::new(len, 0.0)]
    }

    #[test]
    fn a_stair_of_one_or_two_risers_has_no_or_one_tread() {
        let one = stair_steps(&stair(line(3_000.0), Some(1)), 0.0, 175.0);
        assert!(one.treads.is_empty() && one.heights.is_empty());
        assert!((one.riser - 175.0).abs() < 1e-9);
        let two = stair_steps(&stair(line(3_000.0), Some(2)), 0.0, 350.0);
        assert_eq!(two.treads.len(), 1);
        assert!((two.going - 3_000.0).abs() < 1e-9);
        assert!((crate::polygon_area(&two.treads[0]) - 3_000.0 * 900.0).abs() < 1e-6);
        assert!((two.heights[0] - 175.0).abs() < 1e-9);
    }

    #[test]
    fn a_duplicated_point_in_the_walking_line_changes_nothing() {
        let path = vec![
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            Point::new(3_750.0, 0.0),
            Point::new(3_750.0, 0.0),
        ];
        let steps = stair_steps(&stair(path, None), 0.0, 2_800.0);
        assert_eq!(steps.treads.len(), 15);
        for t in &steps.treads {
            assert!((crate::polygon_area(t) - 250.0 * 900.0).abs() < 1e-6);
        }
    }

    #[test]
    fn a_u_turn_stair_has_finite_treads() {
        let path = vec![
            Point::new(0.0, 0.0),
            Point::new(2_000.0, 0.0),
            Point::new(2_000.0, 1_000.0),
            Point::new(0.0, 1_000.0),
        ];
        let steps = stair_steps(&stair(path, Some(9)), 0.0, 1_600.0);
        assert_eq!(steps.treads.len(), 8);
        assert!((steps.going - 5_000.0 / 8.0).abs() < 1e-9);
        for p in steps.treads.iter().flatten() {
            assert!(p.x > -1_000.0 && p.x < 3_000.0 && p.y > -1_000.0 && p.y < 2_000.0);
        }
    }

    #[test]
    fn a_path_going_back_on_itself_does_not_panic() {
        let path = vec![
            Point::new(0.0, 0.0),
            Point::new(2_000.0, 0.0),
            Point::new(0.0, 0.0),
        ];
        let steps = stair_steps(&stair(path, Some(5)), 0.0, 850.0);
        assert_eq!(steps.treads.len(), 4);
        assert!(steps.treads.iter().flatten().all(|p| p.x.is_finite()));
    }

    #[test]
    fn a_stair_going_down_shares_the_height_too() {
        // From 2800 down to 0: 16 risers of -175, as going up.
        let s = stair(line(3_750.0), None);
        let steps = stair_steps(&s, 2_800.0, 0.0);
        assert_eq!(steps.risers, 16, "got {} risers", steps.risers);
        assert!((steps.riser + 175.0).abs() < 1e-9, "{}", steps.riser);
        assert_eq!(steps.treads.len(), 15);
    }

    #[test]
    fn an_inner_point_on_an_edge_makes_no_flat_triangle() {
        let m = mesh_of(square(0.0), vec![mp(5_000.0, 0.0, 1_000.0)]);
        let (points, triangles) = mesh_triangles(&m);
        for t in &triangles {
            let area = crate::polygon_area(&t.map(|i| points[i].at));
            assert!(area > 1.0, "flat triangle {t:?} in {triangles:?}");
        }
    }

    // --- cut_under_roof ---

    fn slab_block(outline: Vec<Point>, bottom: f64, top: f64) -> Block {
        let n = outline.len();
        Block {
            element: Id(3),
            role: None,
            outline,
            bottom: vec![bottom; n],
            top: vec![top; n],
        }
    }

    fn wall_block(a: (f64, f64), b: (f64, f64), t: f64, bottom: f64, top: f64) -> Block {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = dx.hypot(dy);
        let (nx, ny) = (-dy / l * t / 2.0, dx / l * t / 2.0);
        slab_block(
            vec![
                Point::new(a.0 - nx, a.1 - ny),
                Point::new(b.0 - nx, b.1 - ny),
                Point::new(b.0 + nx, b.1 + ny),
                Point::new(a.0 + nx, a.1 + ny),
            ],
            bottom,
            top,
        )
    }

    /// Plan area times mean height, by a signed fan (exact for affine
    /// heights, any simple polygon).
    fn volume(blocks: &[Block]) -> f64 {
        blocks
            .iter()
            .map(|b| {
                let o = &b.outline;
                let h = |k: usize| b.top[k] - b.bottom[k];
                let mut v = 0.0;
                for k in 1..o.len().saturating_sub(1) {
                    let a = ((o[k].x - o[0].x) * (o[k + 1].y - o[0].y)
                        - (o[k].y - o[0].y) * (o[k + 1].x - o[0].x))
                        / 2.0;
                    v += a * (h(0) + h(k) + h(k + 1)) / 3.0;
                }
                v.abs()
            })
            .sum()
    }

    fn plan_area(blocks: &[Block]) -> f64 {
        blocks.iter().map(|b| crate::polygon_area(&b.outline)).sum()
    }

    fn sane(pieces: &[Block]) {
        for b in pieces {
            assert!(b.outline.len() >= 3);
            assert_eq!(b.outline.len(), b.top.len());
            assert_eq!(b.outline.len(), b.bottom.len());
            for k in 0..b.outline.len() {
                assert!(b.outline[k].x.is_finite() && b.outline[k].y.is_finite());
                assert!(b.top[k].is_finite() && b.bottom[k].is_finite());
                assert!(b.top[k] >= b.bottom[k] - 1e-6, "top below bottom: {b:?}");
            }
        }
    }

    fn s45() -> Option<f64> {
        Some(45f64.to_radians())
    }

    fn roofs() -> Vec<Roof> {
        let mut all = Vec::new();
        for plane in [RoofPlane::Top, RoofPlane::Bottom] {
            for overhang in [0.0, 500.0] {
                for slopes in [
                    [s45(), None, s45(), None],
                    [s45(); 4],
                    [s45(), None, None, None],
                    [None; 4],
                ] {
                    let mut r = roof(slopes, overhang);
                    r.plane = plane;
                    all.push(r);
                }
            }
        }
        all
    }

    #[test]
    fn a_cut_never_adds_volume_or_breaks_a_block() {
        let arc = |radius: f64, k: usize| {
            let a = k as f64 * 0.2;
            Point::new(5_000.0 + radius * a.cos(), 3_000.0 + radius * a.sin())
        };
        let curved: Vec<Point> = (0..=12)
            .map(|k| arc(3_000.0, k))
            .chain((0..=12).rev().map(|k| arc(3_200.0, k)))
            .collect();
        let column = vec![
            Point::new(4_850.0, 2_850.0),
            Point::new(5_150.0, 2_850.0),
            Point::new(5_150.0, 3_150.0),
            Point::new(4_850.0, 3_150.0),
        ];
        let blocks = [
            wall_block(
                (-3_000.0, 3_000.0),
                (13_000.0, 3_000.0),
                200.0,
                0.0,
                9_000.0,
            ),
            wall_block((0.0, 0.0), (0.0, 6_000.0), 200.0, 0.0, 6_300.0),
            wall_block((10_000.0, 0.0), (0.0, 0.0), 200.0, 0.0, 6_300.0),
            wall_block(
                (-500.0, -500.0),
                (10_500.0, 6_500.0),
                300.0,
                -200.0,
                5_000.0,
            ),
            slab_block(curved, 0.0, 6_000.0),
            slab_block(column, 0.0, 6_000.0),
        ];
        for r in roofs() {
            for eaves in [-5_000.0, 0.0, 2_800.0, 3_000.0, 20_000.0] {
                for b in &blocks {
                    let pieces = cut_under_roof(b, &r, eaves);
                    sane(&pieces);
                    let (cut, whole) = (volume(&pieces), volume(std::slice::from_ref(b)));
                    assert!(
                        cut <= whole * (1.0 + 1e-9) + 1.0,
                        "volume {cut} > {whole}: {r:?} eaves {eaves} block {b:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_block_out_of_the_roof_or_under_a_high_roof_comes_out_whole() {
        let r = roof([s45(), None, s45(), None], 0.0);
        let far = wall_block((12_000.0, 0.0), (14_000.0, 0.0), 200.0, 0.0, 6_300.0);
        let out = cut_under_roof(&far, &r, 2_800.0);
        assert!((volume(&out) - volume(std::slice::from_ref(&far))).abs() < 1.0);
        let wall = wall_block((0.0, 0.0), (0.0, 6_000.0), 200.0, 0.0, 6_300.0);
        let out = cut_under_roof(&wall, &r, 20_000.0);
        assert!((volume(&out) - volume(&[wall])).abs() < 1.0);
        assert!(out.iter().all(|b| b.top.iter().all(|&t| t == 6_300.0)));
    }

    #[test]
    fn a_wall_crossing_the_roof_keeps_its_parts_out_whole() {
        let r = roof([None; 4], 0.0);
        // Flat Top roof, eaves 3000: underside 2800. The wall sticks out
        // 3000 at each end.
        let w = wall_block(
            (-3_000.0, 3_000.0),
            (13_000.0, 3_000.0),
            200.0,
            0.0,
            9_000.0,
        );
        let pieces = cut_under_roof(&w, &r, 3_000.0);
        sane(&pieces);
        let plan = plan_area(&pieces);
        assert!((plan - 16_000.0 * 200.0).abs() < 1.0, "plan {plan}");
        let outside = pieces
            .iter()
            .filter(|b| b.top.iter().all(|&t| t == 9_000.0))
            .count();
        assert_eq!(outside, 2, "{pieces:?}");
        let cut: Vec<Block> = pieces
            .iter()
            .filter(|b| b.top.iter().all(|&t| (t - 2_800.0).abs() < 1e-6))
            .cloned()
            .collect();
        assert_eq!(cut.len(), 1);
        assert!((plan_area(&cut) - 10_000.0 * 200.0).abs() < 1.0);
    }

    #[test]
    fn a_wall_under_the_overhang_only_is_cut_by_the_overhang() {
        let r = roof([s45(), None, s45(), None], 1_000.0);
        // Edge 0 is y = 0, rising inwards; the overhang reaches y = -1000.
        let w = wall_block((2_000.0, -500.0), (8_000.0, -500.0), 100.0, 0.0, 6_300.0);
        let pieces = cut_under_roof(&w, &r, 2_800.0);
        sane(&pieces);
        let underside = 2_800.0 - 450.0 - 200.0 / 45f64.to_radians().cos();
        assert!((top_of(&pieces) - underside).abs() < 1.0);
    }

    #[test]
    fn a_hipped_roof_cuts_a_column_at_the_hip_by_its_lowest_pane() {
        let s = Some(30f64.to_radians());
        let r = roof([s; 4], 0.0);
        let c = slab_block(
            vec![
                Point::new(900.0, 900.0),
                Point::new(1_100.0, 900.0),
                Point::new(1_100.0, 1_100.0),
                Point::new(900.0, 1_100.0),
            ],
            0.0,
            9_000.0,
        );
        let pieces = cut_under_roof(&c, &r, 3_000.0);
        sane(&pieces);
        let t = 30f64.to_radians().tan();
        let lift = 200.0 / 30f64.to_radians().cos();
        let lo = pieces
            .iter()
            .flat_map(|b| b.top.iter().copied())
            .fold(f64::MAX, f64::min);
        assert!((lo - (3_000.0 + 900.0 * t - lift)).abs() < 1.0, "{lo}");
        assert!((top_of(&pieces) - (3_000.0 + 1_100.0 * t - lift)).abs() < 1.0);
        let area = plan_area(&pieces);
        assert!((area - 200.0 * 200.0).abs() < 1.0, "plan kept once: {area}");
    }

    #[test]
    fn the_eaves_level_is_on_the_top_or_on_the_underside() {
        let mut r = roof([None; 4], 0.0);
        let w = wall_block((2_000.0, 3_000.0), (8_000.0, 3_000.0), 200.0, 0.0, 6_000.0);
        let hi = |r: &Roof| top_of(&cut_under_roof(&w, r, 3_000.0));
        assert!((hi(&r) - 2_800.0).abs() < 1e-6);
        r.plane = RoofPlane::Bottom;
        assert!((hi(&r) - 3_000.0).abs() < 1e-6);
    }

    #[test]
    fn a_roof_below_the_bottom_leaves_nothing() {
        let r = roof([None; 4], 0.0);
        let w = wall_block(
            (2_000.0, 3_000.0),
            (8_000.0, 3_000.0),
            200.0,
            3_000.0,
            6_000.0,
        );
        assert!(cut_under_roof(&w, &r, 3_000.0).is_empty());
    }

    #[test]
    fn a_top_exactly_on_the_underside_is_not_kept_twice() {
        let r = roof([None; 4], 0.0);
        let w = wall_block((2_000.0, 3_000.0), (8_000.0, 3_000.0), 200.0, 0.0, 2_800.0);
        let pieces = cut_under_roof(&w, &r, 3_000.0);
        sane(&pieces);
        assert_eq!(pieces.len(), 1, "{pieces:?}");
    }

    #[test]
    fn a_block_cut_under_two_roofs_follows_the_lower_one() {
        let flat = roof([None; 4], 0.0);
        let w = wall_block((2_000.0, 3_000.0), (8_000.0, 3_000.0), 200.0, 0.0, 9_000.0);
        let mut pieces = vec![w];
        for eaves in [3_000.0, 2_000.0] {
            pieces = pieces
                .iter()
                .flat_map(|b| cut_under_roof(b, &flat, eaves))
                .collect();
        }
        sane(&pieces);
        assert!((top_of(&pieces) - 1_800.0).abs() < 1e-6);
        assert!((plan_area(&pieces) - 6_000.0 * 200.0).abs() < 1.0);
    }
}
