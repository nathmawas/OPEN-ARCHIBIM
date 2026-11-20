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
    for inner in &mesh.inner {
        let Some(t) = triangles
            .iter()
            .position(|&[a, b, c]| contains(&[points[a].at, points[b].at, points[c].at], inner.at))
        else {
            continue;
        };
        let [a, b, c] = triangles.swap_remove(t);
        let k = points.len();
        points.push(*inner);
        triangles.extend([[a, b, k], [b, c, k], [c, a, k]]);
    }
    (points, triangles)
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
        2.0 * self.riser + self.going
    }
}

/// The steps of a stair from `bottom` to `top`: the risers share the height
/// equally, the treads share the walking line; the upper floor is the last
/// tread.
pub fn stair_steps(stair: &Stair, bottom: f64, top: f64) -> StairSteps {
    let rise = top - bottom;
    let risers = stair
        .risers
        .unwrap_or_else(|| ((rise / RISER_TARGET).round() as u32).max(1));
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
}
