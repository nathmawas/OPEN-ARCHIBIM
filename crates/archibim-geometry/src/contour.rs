//! The contour around a point: the inner faces of a loop of walls, or the
//! outline of a slab. Used to draw a slab or walls on what already exists.

use archibim_model::{Id, Point, Project};

use crate::{Line, TOL, add, contains, cross, dot, intersect, length, mul, polygon_area, sub};

/// The smallest closed contour of story `story` around `p`: the faces of
/// the walls that enclose it, or else the outline of the slab under it.
/// Anticlockwise.
// ponytail: straight walls only, and walls must meet end to end or end on
// another wall; walls crossing each other do not split the loop yet.
pub fn contour_at(project: &Project, story: Id, p: Point) -> Option<Vec<Point>> {
    wall_loop(project, story, p).or_else(|| {
        project
            .slabs
            .iter()
            .filter(|s| s.story == story && contains(&s.outline, p))
            .min_by(|a, b| {
                polygon_area(&a.outline)
                    .abs()
                    .total_cmp(&polygon_area(&b.outline).abs())
            })
            .map(|s| anticlockwise(s.outline.clone()))
    })
}

fn anticlockwise(mut points: Vec<Point>) -> Vec<Point> {
    if signed_area(&points) < 0.0 {
        points.reverse();
    }
    points
}

fn signed_area(points: &[Point]) -> f64 {
    let n = points.len();
    (0..n)
        .map(|i| cross(points[i], points[(i + 1) % n]))
        .sum::<f64>()
        / 2.0
}

/// One wall between two nodes of the graph, with its two face lines.
struct Edge {
    from: usize,
    to: usize,
    /// The face on the left of `from` to `to`, then the one on the right.
    faces: [Line; 2],
}

fn wall_loop(project: &Project, story: Id, p: Point) -> Option<Vec<Point>> {
    let walls: Vec<_> = project.walls.iter().filter(|w| w.story == story).collect();
    // Each wall end becomes a node: the end itself, or its foot on the wall
    // it stops against.
    let mut nodes: Vec<Point> = Vec::new();
    let mut node = |q: Point| match nodes.iter().position(|n| length(sub(*n, q)) < TOL) {
        Some(i) => i,
        None => {
            nodes.push(q);
            nodes.len() - 1
        }
    };
    let on_host = |q: Point, skip: usize| {
        walls.iter().enumerate().find_map(|(j, h)| {
            let (a, b) = h.left_face()?;
            let d = mul(sub(h.end, h.start), 1.0 / length(sub(h.end, h.start)));
            let r = Point::new(d.y, -d.x);
            let t = dot(sub(q, h.start), d);
            let s = dot(sub(q, a), r);
            let len = length(sub(b, a));
            (j != skip && t > TOL && t < len - TOL && s > -TOL && s < h.thickness() + TOL)
                .then(|| (j, add(h.start, mul(d, t))))
        })
    };
    // Where each wall is cut by the walls that stop against it.
    let mut cuts: Vec<Vec<Point>> = walls.iter().map(|w| vec![w.start, w.end]).collect();
    let mut ends: Vec<[Point; 2]> = Vec::new();
    for (i, w) in walls.iter().enumerate() {
        let mut e = [w.start, w.end];
        for q in &mut e {
            if let Some((j, foot)) = on_host(*q, i) {
                cuts[j].push(foot);
                *q = foot;
            }
        }
        ends.push(e);
    }
    let mut edges = Vec::new();
    for (i, w) in walls.iter().enumerate() {
        let Some((a, b)) = w.left_face() else {
            continue;
        };
        let d = mul(sub(b, a), 1.0 / length(sub(b, a)));
        let r = Point::new(d.y, -d.x);
        let left = Line { p: a, d };
        let right = Line {
            p: add(a, mul(r, w.thickness())),
            d,
        };
        let mut along: Vec<Point> = cuts[i].clone();
        along[0] = ends[i][0];
        along[1] = ends[i][1];
        along.sort_by(|x, y| dot(sub(*x, w.start), d).total_cmp(&dot(sub(*y, w.start), d)));
        for pair in along.windows(2) {
            let (from, to) = (node(pair[0]), node(pair[1]));
            if from != to {
                edges.push(Edge {
                    from,
                    to,
                    faces: [left, right],
                });
                let reverse = |l: Line| Line {
                    p: l.p,
                    d: mul(l.d, -1.0),
                };
                edges.push(Edge {
                    from: to,
                    to: from,
                    faces: [reverse(right), reverse(left)],
                });
            }
        }
    }
    let angle = |e: &Edge| {
        let v = sub(nodes[e.to], nodes[e.from]);
        v.y.atan2(v.x)
    };
    // Each face of the graph, walked with the face on the left.
    let next = |e: usize| {
        let twin = angle(&Edge {
            from: edges[e].to,
            to: edges[e].from,
            faces: edges[e].faces,
        });
        let at = edges[e].to;
        edges
            .iter()
            .enumerate()
            .filter(|(_, f)| f.from == at)
            .map(|(k, f)| {
                let mut turn = twin - angle(f);
                if turn <= 1e-9 {
                    turn += std::f64::consts::TAU;
                }
                (k, turn)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(k, _)| k)
    };
    let mut seen = vec![false; edges.len()];
    let mut best: Option<(f64, Vec<Point>)> = None;
    for first in 0..edges.len() {
        if seen[first] {
            continue;
        }
        let mut cycle = vec![first];
        seen[first] = true;
        let mut e = first;
        while let Some(k) = next(e) {
            if k == first {
                break;
            }
            if seen[k] || cycle.len() > edges.len() {
                cycle.clear();
                break;
            }
            seen[k] = true;
            cycle.push(k);
            e = k;
        }
        let corners: Vec<Point> = cycle.iter().map(|&k| nodes[edges[k].from]).collect();
        let area = signed_area(&corners);
        if corners.len() < 3 || area <= 0.0 || !contains(&corners, p) {
            continue;
        }
        // The inner faces, met corner to corner.
        let n = cycle.len();
        let faces: Vec<Line> = cycle.iter().map(|&k| edges[k].faces[0]).collect();
        let outline: Vec<Point> = (0..n)
            .filter_map(|i| intersect(faces[(i + n - 1) % n], faces[i]))
            .collect();
        if outline.len() >= 3 && best.as_ref().is_none_or(|(a, _)| area < *a) {
            best = Some((area, outline));
        }
    }
    best.map(|(_, outline)| outline)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archibim_model::{
        Layer, LayerRole, LevelKind, LevelRef, Material, RenovationStatus, Slab, Story, Wall,
        WallReference,
    };

    fn project() -> Result<(Project, Id), archibim_model::ModelError> {
        let mut p = Project::new();
        let (story, material) = (Id(1), Id(2));
        p.insert_story(Story {
            id: story,
            name: String::new(),
            finished_level: 0.0,
            floor_build_up: 0.0,
        })?;
        p.insert_material(Material {
            id: material,
            name: String::new(),
            priority: 5,
        })?;
        let level = |offset| LevelRef {
            story,
            kind: LevelKind::Finished,
            offset,
        };
        let mut id = 10;
        let mut wall = |p: &mut Project, a: (f64, f64), b: (f64, f64), t: f64| {
            id += 1;
            p.insert_wall(Wall {
                id: Id(id),
                story,
                start: Point::new(a.0, a.1),
                end: Point::new(b.0, b.1),
                reference: WallReference::Axis,
                offset: 0.0,
                layers: vec![Layer {
                    role: LayerRole::LoadBearing,
                    material,
                    thickness: t,
                    status: RenovationStatus::New,
                    framing: None,
                }],
                bottom: level(0.0),
                top: level(2500.0),
            })
        };
        // A 10 by 6 m outline on its axis, 200 thick, split by a partition
        // 100 thick at x = 4000 that stops against both long walls.
        wall(&mut p, (0.0, 0.0), (10000.0, 0.0), 200.0)?;
        wall(&mut p, (10000.0, 0.0), (10000.0, 6000.0), 200.0)?;
        wall(&mut p, (10000.0, 6000.0), (0.0, 6000.0), 200.0)?;
        wall(&mut p, (0.0, 6000.0), (0.0, 0.0), 200.0)?;
        wall(&mut p, (4000.0, 100.0), (4000.0, 5900.0), 100.0)?;
        Ok((p, story))
    }

    fn bounds(points: &[Point]) -> (f64, f64, f64, f64) {
        points.iter().fold(
            (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
            |(a, b, c, d), q| (a.min(q.x), b.min(q.y), c.max(q.x), d.max(q.y)),
        )
    }

    #[test]
    fn a_click_inside_rooms_gives_their_inner_faces() -> Result<(), Box<dyn std::error::Error>> {
        let (p, story) = project()?;
        let left = contour_at(&p, story, Point::new(2000.0, 3000.0)).ok_or("no contour")?;
        let (x0, y0, x1, y1) = bounds(&left);
        let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
        assert!(near(x0, 100.0) && near(x1, 3950.0), "{left:?}");
        assert!(near(y0, 100.0) && near(y1, 5900.0), "{left:?}");
        assert!(signed_area(&left) > 0.0, "anticlockwise");
        let right = contour_at(&p, story, Point::new(7000.0, 3000.0)).ok_or("no contour")?;
        let (x0, _, x1, _) = bounds(&right);
        assert!(near(x0, 4050.0) && near(x1, 9900.0), "{right:?}");
        assert!(
            contour_at(&p, story, Point::new(-5000.0, 0.0)).is_none(),
            "outside"
        );
        Ok(())
    }

    #[test]
    fn without_walls_the_slab_under_the_point_is_used() -> Result<(), Box<dyn std::error::Error>> {
        let (mut p, story) = project()?;
        p.walls.clear();
        let square = vec![
            Point::new(0.0, 0.0),
            Point::new(0.0, 1000.0),
            Point::new(1000.0, 1000.0),
            Point::new(1000.0, 0.0),
        ];
        p.insert_slab(Slab {
            id: Id(99),
            story,
            outline: square,
            layers: vec![Layer {
                role: LayerRole::LoadBearing,
                material: Id(2),
                thickness: 200.0,
                status: RenovationStatus::New,
                framing: None,
            }],
            top: LevelRef {
                story,
                kind: LevelKind::Finished,
                offset: 0.0,
            },
        })?;
        let c = contour_at(&p, story, Point::new(500.0, 500.0)).ok_or("no contour")?;
        assert_eq!(c.len(), 4);
        assert!(signed_area(&c) > 0.0);
        Ok(())
    }
}
