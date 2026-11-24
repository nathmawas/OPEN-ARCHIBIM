//! Selecting: how what is clicked or caught by an area changes the
//! selection, and which elements an area catches in plan.

use archibim_commands::Command;
use archibim_geometry::{
    LayerPlan, beam_outline, column_outline, contains, opening_outline, roof_overhang, stair_steps,
};
use archibim_model::{GuideShape, Id, Point, Project};
use egui::Modifiers;

use crate::Element;

/// How the selection tool draws an area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectMethod {
    /// Dragged from corner to corner: left to right takes the elements
    /// wholly inside, right to left the elements touched.
    Rectangle,
    /// Corners clicked, closed on the first one or with Enter.
    Polygon,
    /// A free line dragged, closed on release.
    Lasso,
}

/// How what is clicked or caught changes the selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Combine {
    /// It becomes the selection.
    Replace,
    /// It is added.
    Add,
    /// It is taken out.
    Remove,
    /// What was selected is taken out, the rest added.
    Invert,
}

impl Combine {
    /// From the keys held: Shift adds, Ctrl removes, Alt inverts, and Ctrl
    /// with Shift inverts too, where the system keeps Alt.
    pub(crate) fn from(m: Modifiers) -> Self {
        if m.alt || (m.command && m.shift) {
            Self::Invert
        } else if m.command {
            Self::Remove
        } else if m.shift {
            Self::Add
        } else {
            Self::Replace
        }
    }

    /// Changes `selection` with what was `caught`, keeping its order.
    pub(crate) fn apply(self, selection: &mut Vec<Id>, caught: &[Id]) {
        match self {
            Self::Replace => {
                selection.clear();
                selection.extend(caught);
            }
            Self::Add => {
                for id in caught {
                    if !selection.contains(id) {
                        selection.push(*id);
                    }
                }
            }
            Self::Remove => selection.retain(|id| !caught.contains(id)),
            Self::Invert => {
                for id in caught {
                    match selection.iter().position(|s| s == id) {
                        Some(i) => {
                            selection.remove(i);
                        }
                        None => selection.push(*id),
                    }
                }
            }
        }
    }
}

/// What an element draws in plan, as an area judges it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Drawn {
    /// Filled outlines: an area lying inside one touches it.
    Filled(Vec<Vec<Point>>),
    /// A closed line around nothing filled: a guide circle.
    Ring(Vec<Point>),
    /// An infinite line through two points: a guide line.
    Line(Point, Point),
}

/// Whether the area, a closed polygon, catches what is drawn: lying wholly
/// inside it if `whole`, else touching it.
pub(crate) fn caught(drawn: &Drawn, area: &[Point], whole: bool) -> bool {
    let touches_line =
        |poly: &[Point]| crosses(poly, area) || poly.iter().any(|&q| contains(area, q));
    match drawn {
        Drawn::Filled(polys) if whole => !polys.is_empty() && polys.iter().all(|p| inside(p, area)),
        Drawn::Filled(polys) => polys
            .iter()
            .any(|p| touches_line(p) || area.iter().any(|&q| contains(p, q))),
        Drawn::Ring(ring) if whole => inside(ring, area),
        Drawn::Ring(ring) => touches_line(ring),
        // An infinite line is never wholly inside.
        Drawn::Line(..) if whole => false,
        Drawn::Line(a, b) => {
            let side = |q: &Point| orient(*a, *b, *q);
            area.iter().any(|q| side(q) >= 0.0) && area.iter().any(|q| side(q) <= 0.0)
        }
    }
}

/// The elements the area catches, as the plan draws them: the faces of the
/// walls of the story shown, the outlines of the rest, and the guides if
/// they are `shown`.
// ponytail: every edge of every element against every edge of the area,
// on each frame of a drag; a spatial index when projects get large.
// Slabs, roofs and the rest of every story count, as the plan draws them
// all; keep those of the view's story once the plan filters them.
pub(crate) fn caught_in(
    project: &Project,
    plan: &[LayerPlan],
    area: &[Point],
    whole: bool,
    shown: bool,
) -> Vec<Id> {
    if area.len() < 3 {
        return Vec::new();
    }
    drawn(project, plan, shown)
        .into_iter()
        .filter(|(_, d)| caught(d, area, whole))
        .map(|(id, _)| id)
        .collect()
}

/// What every element draws in plan, guides only if `shown`.
fn drawn(project: &Project, plan: &[LayerPlan], shown: bool) -> Vec<(Id, Drawn)> {
    let mut walls: Vec<(Id, Vec<Vec<Point>>)> = Vec::new();
    for layer in plan {
        match walls.iter_mut().find(|(id, _)| *id == layer.wall) {
            Some((_, polys)) => polys.extend(layer.polygons.iter().cloned()),
            None => walls.push((layer.wall, layer.polygons.clone())),
        }
    }
    let filled = |id, polys| (id, Drawn::Filled(polys));
    let mut all: Vec<(Id, Drawn)> = walls.into_iter().map(|(id, p)| filled(id, p)).collect();
    all.extend(
        project
            .slabs
            .iter()
            .map(|s| filled(s.id, vec![s.outline.clone()])),
    );
    all.extend(project.openings.iter().filter_map(|o| {
        opening_outline(project, o.id)
            .ok()
            .map(|p| filled(o.id, vec![p]))
    }));
    all.extend(
        project
            .columns
            .iter()
            .map(|c| filled(c.id, vec![column_outline(c)])),
    );
    all.extend(
        project
            .beams
            .iter()
            .filter_map(|b| beam_outline(b).map(|p| filled(b.id, vec![p]))),
    );
    all.extend(project.stairs.iter().filter_map(|s| {
        let (bottom, top) = (project.level(&s.bottom).ok()?, project.level(&s.top).ok()?);
        Some(filled(s.id, stair_steps(s, bottom, top).treads))
    }));
    all.extend(
        project
            .roofs
            .iter()
            .map(|r| filled(r.id, vec![roof_overhang(r)])),
    );
    all.extend(
        project
            .meshes
            .iter()
            .map(|m| filled(m.id, vec![m.outline.iter().map(|p| p.at).collect()])),
    );
    for g in project.guides.iter().filter(|_| shown) {
        let d = match g.shape {
            GuideShape::Line { a, b } => Drawn::Line(a, b),
            // ponytail: a circle as 64 chords; finer if architects catch
            // the gap.
            GuideShape::Circle { centre, radius } => Drawn::Ring(
                (0..64)
                    .map(|i| {
                        let a = f64::from(i) * std::f64::consts::TAU / 64.0;
                        Point::new(centre.x + radius * a.cos(), centre.y + radius * a.sin())
                    })
                    .collect(),
            ),
        };
        all.push((g.id, d));
    }
    all
}

/// The command that deletes the elements `ids`, in one undo step: a wall
/// takes the openings it hosts with it, and nothing is removed twice.
pub(crate) fn removal(project: &Project, ids: &[Id]) -> Option<Command> {
    let mut removed: Vec<Id> = Vec::new();
    let mut commands = Vec::new();
    for element in ids.iter().filter_map(|&id| Element::find(project, id)) {
        // The model refuses a wall that still hosts openings: they go
        // first.
        let hosted = project
            .openings
            .iter()
            .filter(|o| matches!(&element, Element::Wall(w) if w.id == o.wall))
            .map(|o| (o.id, Command::RemoveOpening(o.id)));
        for (id, command) in hosted.chain([(element.id(), element.remove())]) {
            if !removed.contains(&id) {
                removed.push(id);
                commands.push(command);
            }
        }
    }
    match commands.len() {
        0 | 1 => commands.pop(),
        _ => Some(Command::Group(commands)),
    }
}

/// Every point inside the area, and no edge crossing it.
fn inside(poly: &[Point], area: &[Point]) -> bool {
    !poly.is_empty() && poly.iter().all(|&q| contains(area, q)) && !crosses(poly, area)
}

/// Whether an edge of one closed polygon meets an edge of the other.
fn crosses(a: &[Point], b: &[Point]) -> bool {
    edges(a).any(|(p, q)| edges(b).any(|(r, s)| segments_meet(p, q, r, s)))
}

fn edges(poly: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    let n = poly.len();
    (0..n).map(move |i| (poly[i], poly[(i + 1) % n]))
}

/// Twice the signed area of the triangle `a`, `b`, `c`: positive when `c`
/// is to the left of `a` to `b`.
fn orient(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Whether the segments `ab` and `cd` meet, touching ends included.
fn segments_meet(a: Point, b: Point, c: Point, d: Point) -> bool {
    let apart = |p: f64, q: f64, r: f64, s: f64| p.max(q) < r.min(s) || r.max(s) < p.min(q);
    if apart(a.x, b.x, c.x, d.x) || apart(a.y, b.y, c.y, d.y) {
        return false;
    }
    orient(c, d, a) * orient(c, d, b) <= 0.0 && orient(a, b, c) * orient(a, b, d) <= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn square(x: f64, y: f64, size: f64) -> Vec<Point> {
        vec![
            pt(x, y),
            pt(x + size, y),
            pt(x + size, y + size),
            pt(x, y + size),
        ]
    }

    #[test]
    fn shift_adds_ctrl_removes_alt_or_ctrl_shift_inverts() {
        let m = |alt, command, shift| Modifiers {
            alt,
            command,
            shift,
            ..Modifiers::NONE
        };
        assert_eq!(Combine::from(m(false, false, false)), Combine::Replace);
        assert_eq!(Combine::from(m(false, false, true)), Combine::Add);
        assert_eq!(Combine::from(m(false, true, false)), Combine::Remove);
        assert_eq!(Combine::from(m(true, false, false)), Combine::Invert);
        assert_eq!(Combine::from(m(false, true, true)), Combine::Invert);
    }

    #[test]
    fn a_pick_replaces_adds_removes_or_inverts_the_selection() {
        let (a, b, c) = (Id(1), Id(2), Id(3));
        let with = |how: Combine, caught: &[Id]| {
            let mut s = vec![a, b];
            how.apply(&mut s, caught);
            s
        };
        assert_eq!(with(Combine::Replace, &[c]), vec![c]);
        assert_eq!(with(Combine::Replace, &[]), Vec::<Id>::new(), "empty space");
        assert_eq!(with(Combine::Add, &[b, c]), vec![a, b, c]);
        assert_eq!(with(Combine::Remove, &[b, c]), vec![a]);
        assert_eq!(with(Combine::Invert, &[b, c]), vec![a, c]);
    }

    #[test]
    fn whole_needs_every_outline_inside_touched_any_contact() {
        let area = square(0.0, 0.0, 100.0);
        let inner = Drawn::Filled(vec![square(10.0, 10.0, 20.0)]);
        let across = Drawn::Filled(vec![square(90.0, 10.0, 20.0)]);
        let outside = Drawn::Filled(vec![square(200.0, 10.0, 20.0)]);
        // A wall of two layers, one of them half out.
        let two = Drawn::Filled(vec![square(10.0, 10.0, 20.0), square(90.0, 40.0, 20.0)]);
        assert!(caught(&inner, &area, true));
        assert!(!caught(&across, &area, true));
        assert!(!caught(&two, &area, true));
        assert!(caught(&across, &area, false));
        assert!(caught(&two, &area, false));
        assert!(!caught(&outside, &area, false));
        assert!(!caught(&outside, &area, true));
    }

    #[test]
    fn an_area_inside_a_filled_outline_touches_it_not_a_ring() {
        let slab = square(0.0, 0.0, 1000.0);
        let area = square(400.0, 400.0, 100.0);
        assert!(caught(&Drawn::Filled(vec![slab.clone()]), &area, false));
        assert!(!caught(&Drawn::Filled(vec![slab.clone()]), &area, true));
        assert!(!caught(&Drawn::Ring(slab), &area, false));
    }

    #[test]
    fn a_concave_area_does_not_take_what_lies_in_its_notch() {
        // A U open to the top: its notch runs from x 40 to 60, above y 20.
        let u = vec![
            pt(0.0, 0.0),
            pt(100.0, 0.0),
            pt(100.0, 100.0),
            pt(60.0, 100.0),
            pt(60.0, 20.0),
            pt(40.0, 20.0),
            pt(40.0, 100.0),
            pt(0.0, 100.0),
        ];
        // From one arm to the other across the notch: corners inside,
        // edges crossing out.
        let bridge = Drawn::Filled(vec![vec![
            pt(10.0, 50.0),
            pt(90.0, 50.0),
            pt(90.0, 60.0),
            pt(10.0, 60.0),
        ]]);
        assert!(!caught(&bridge, &u, true));
        assert!(caught(&bridge, &u, false));
        assert!(!caught(
            &Drawn::Filled(vec![square(45.0, 50.0, 5.0)]),
            &u,
            false
        ));
    }

    #[test]
    fn a_guide_line_is_touched_when_it_crosses_the_area_never_whole() {
        let area = square(0.0, 0.0, 100.0);
        let through = Drawn::Line(pt(-500.0, 50.0), pt(-400.0, 50.0));
        let beside = Drawn::Line(pt(-500.0, 150.0), pt(-400.0, 150.0));
        assert!(caught(&through, &area, false));
        assert!(!caught(&through, &area, true));
        assert!(!caught(&beside, &area, false));
    }

    #[test]
    fn segments_meet_at_a_crossing_or_an_end_not_apart() {
        let (a, b) = (pt(0.0, 0.0), pt(10.0, 10.0));
        assert!(segments_meet(a, b, pt(0.0, 10.0), pt(10.0, 0.0)));
        assert!(segments_meet(a, b, pt(10.0, 10.0), pt(20.0, 0.0)), "end");
        assert!(segments_meet(a, b, pt(5.0, 5.0), pt(20.0, 20.0)), "overlap");
        assert!(!segments_meet(a, b, pt(11.0, 11.0), pt(20.0, 20.0)));
        assert!(
            !segments_meet(a, b, pt(0.0, 1.0), pt(9.0, 10.0)),
            "parallel"
        );
    }
}
