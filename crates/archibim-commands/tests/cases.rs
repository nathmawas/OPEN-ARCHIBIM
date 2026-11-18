//! The architect test cases of `tests/cases`, run through commands.
//!
//! Each test is named after its case. The cases are still `proposed`: these
//! tests check the prototype against them and will follow their wording once
//! an architect agrees them.

use archibim_commands::{Command, History};
use archibim_geometry::{LayerPlan, plan_layers, polygon_area};
use archibim_model::{
    Hand, Id, JunctionMode, Layer, LevelKind, LevelRef, Material, NominalSize, Opening,
    OpeningKind, Operation, Part, Performance, Point, Project, RenovationStatus, Story, Wall,
};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

struct Fixture {
    h: History,
    story0: Id,
    story1: Id,
}

impl Fixture {
    fn new() -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let mut h = History::new(Project::new());
        let story0 = h.new_id();
        let story1 = h.new_id();
        h.run(Command::AddStory(Story {
            id: story0,
            name: "Ground floor".into(),
            finished_level: 0.0,
            floor_build_up: 0.0,
        }))?;
        h.run(Command::AddStory(Story {
            id: story1,
            name: "First floor".into(),
            finished_level: 2800.0,
            floor_build_up: 0.0,
        }))?;
        Ok(Self { h, story0, story1 })
    }

    fn material(
        &mut self,
        name: &str,
        priority: u8,
    ) -> std::result::Result<Id, Box<dyn std::error::Error>> {
        let id = self.h.new_id();
        self.h.run(Command::AddMaterial(Material {
            id,
            name: name.into(),
            priority,
        }))?;
        Ok(id)
    }

    fn wall(
        &mut self,
        start: (f64, f64),
        end: (f64, f64),
        layers: &[(Id, f64)],
    ) -> std::result::Result<Id, Box<dyn std::error::Error>> {
        let id = self.h.new_id();
        self.h.run(Command::AddWall(Wall {
            id,
            story: self.story0,
            start: Point::new(start.0, start.1),
            end: Point::new(end.0, end.1),
            layers: layers
                .iter()
                .map(|&(material, thickness)| Layer {
                    material,
                    thickness,
                    status: RenovationStatus::New,
                })
                .collect(),
            bottom: LevelRef {
                story: self.story0,
                kind: LevelKind::Structural,
                offset: 0.0,
            },
            top: LevelRef {
                story: self.story1,
                kind: LevelKind::Structural,
                offset: 0.0,
            },
        }))?;
        Ok(id)
    }

    fn plan(&self) -> std::result::Result<Vec<LayerPlan>, Box<dyn std::error::Error>> {
        Ok(plan_layers(self.h.project(), self.story0)?)
    }
}

fn layer(plan: &[LayerPlan], wall: Id, layer: usize) -> Vec<Vec<Point>> {
    plan.iter()
        .find(|l| l.wall == wall && l.layer == layer)
        .map(|l| l.polygons.clone())
        .unwrap_or_default()
}

fn total_area(plan: &[LayerPlan]) -> f64 {
    plan.iter()
        .flat_map(|l| &l.polygons)
        .map(|p| polygon_area(p))
        .sum()
}

fn ys(polygons: &[Vec<Point>]) -> (f64, f64) {
    let ys = polygons.iter().flatten().map(|p| p.y);
    (
        ys.clone().fold(f64::MAX, f64::min),
        ys.fold(f64::MIN, f64::max),
    )
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn shared_vertices(a: &[Vec<Point>], b: &[Vec<Point>]) -> usize {
    let near = |p: &Point, q: &Point| (p.x - q.x).abs() < 1e-6 && (p.y - q.y).abs() < 1e-6;
    a.iter()
        .flatten()
        .filter(|p| b.iter().flatten().any(|q| near(p, q)))
        .count()
}

/// External wall of walls-t-junction-priority, along x, outside at y > 0.
fn external_wall(
    f: &mut Fixture,
    brick_priority: u8,
) -> std::result::Result<(Id, Id, [Id; 4]), Box<dyn std::error::Error>> {
    let render = f.material("render", 1)?;
    let insulation = f.material("insulation", 3)?;
    let concrete = f.material("concrete", 9)?;
    let plaster = f.material("plaster", 1)?;
    let brick = f.material("brick", brick_priority)?;
    let a = f.wall(
        (0.0, 0.0),
        (6000.0, 0.0),
        &[
            (render, 20.0),
            (insulation, 140.0),
            (concrete, 200.0),
            (plaster, 15.0),
        ],
    )?;
    let b = f.wall(
        (3000.0, -3000.0),
        (3000.0, -375.0),
        &[(plaster, 15.0), (brick, 100.0), (plaster, 15.0)],
    )?;
    Ok((a, b, [render, insulation, concrete, brick]))
}

#[test]
fn walls_l_corner_same_build_up() -> Result {
    let mut f = Fixture::new()?;
    let render = f.material("render", 1)?;
    let insulation = f.material("insulation", 3)?;
    let concrete = f.material("concrete", 9)?;
    let plaster = f.material("plaster", 1)?;
    let build_up = [
        (render, 20.0),
        (insulation, 140.0),
        (concrete, 200.0),
        (plaster, 15.0),
    ];
    let a = f.wall((0.0, 0.0), (5000.0, 0.0), &build_up)?;
    let b = f.wall((5000.0, 0.0), (5000.0, -5000.0), &build_up)?;
    let plan = f.plan()?;
    for i in 0..4 {
        assert_eq!(
            shared_vertices(&layer(&plan, a, i), &layer(&plan, b, i)),
            2,
            "layer {i} joins the same layer"
        );
    }
    let t = 375.0;
    assert!(
        close(total_area(&plan), 10000.0 * t - t * t),
        "no overlap, no gap"
    );
    Ok(())
}

#[test]
fn walls_t_junction_priority() -> Result {
    let mut f = Fixture::new()?;
    let (a, b, _) = external_wall(&mut f, 5)?;
    let plan = f.plan()?;
    for structural in 0..3 {
        assert_eq!(
            layer(&plan, a, structural).len(),
            1,
            "layer {structural} of the external wall is not cut"
        );
    }
    assert_eq!(
        layer(&plan, a, 3).len(),
        2,
        "the brick interrupts the external plaster"
    );
    assert!(
        close(ys(&layer(&plan, b, 1)).1, -360.0),
        "the brick stops against the concrete"
    );
    assert!(
        close(ys(&layer(&plan, b, 0)).1, -375.0),
        "the plaster stops against the plaster"
    );
    assert!(
        close(ys(&layer(&plan, b, 2)).1, -375.0),
        "the plaster stops against the plaster"
    );
    Ok(())
}

#[test]
fn walls_priority_change() -> Result {
    let mut f = Fixture::new()?;
    let (_, b, [.., brick]) = external_wall(&mut f, 5)?;
    let changed = f.h.run(Command::SetMaterialPriority {
        material: brick,
        priority: 10,
    })?;
    assert!(
        changed.contains(&b),
        "the walls using the material are reported as changed"
    );
    let plan = f.plan()?;
    assert!(
        ys(&layer(&plan, b, 1)).1 > -360.0,
        "the brick now cuts into the concrete"
    );
    Ok(())
}

#[test]
fn walls_x_crossing_priority() -> Result {
    let mut f = Fixture::new()?;
    let plaster = f.material("plaster", 1)?;
    let concrete = f.material("concrete", 9)?;
    let brick = f.material("brick", 5)?;
    let a = f.wall(
        (0.0, 0.0),
        (6000.0, 0.0),
        &[(plaster, 15.0), (concrete, 180.0), (plaster, 15.0)],
    )?;
    let b = f.wall(
        (3000.0, 3000.0),
        (3000.0, -3000.0),
        &[(plaster, 15.0), (brick, 100.0), (plaster, 15.0)],
    )?;
    let plan = f.plan()?;
    assert_eq!(
        layer(&plan, a, 1).len(),
        1,
        "the concrete wall is continuous"
    );
    let brick_pieces = layer(&plan, b, 1);
    assert_eq!(brick_pieces.len(), 2, "the brick is cut on both sides");
    let ends: Vec<f64> = brick_pieces
        .iter()
        .map(|p| ys(std::slice::from_ref(p)))
        .flat_map(|(lo, hi)| [lo, hi])
        .collect();
    assert!(
        ends.iter().any(|y| close(*y, -15.0)) && ends.iter().any(|y| close(*y, -195.0)),
        "it stops at the concrete faces"
    );
    Ok(())
}

#[test]
fn walls_equal_priority_junction() -> Result {
    let mut f = Fixture::new()?;
    let plaster = f.material("plaster", 1)?;
    let brick = f.material("brick", 5)?;
    let build_up = [(plaster, 15.0), (brick, 200.0), (plaster, 15.0)];
    let a = f.wall((0.0, 0.0), (5000.0, 0.0), &build_up)?;
    let b = f.wall((5000.0, 0.0), (5000.0, -5000.0), &build_up)?;
    let t = 230.0;
    let mitre = f.plan()?;
    assert_eq!(
        shared_vertices(&layer(&mitre, a, 1), &layer(&mitre, b, 1)),
        2,
        "mitre by default"
    );

    f.h.run(Command::SetJunctionMode {
        a,
        b,
        mode: Some(JunctionMode::InFront(a)),
    })?;
    let front = f.plan()?;
    let a_max_x = layer(&front, a, 1)
        .iter()
        .flatten()
        .map(|p| p.x)
        .fold(f64::MIN, f64::max);
    assert!(
        close(a_max_x, 5000.0),
        "the wall in front runs to the outer face"
    );
    assert!(
        close(ys(&layer(&front, b, 1)).1, -t),
        "the other wall stops against it"
    );
    assert!(
        close(total_area(&front), 10000.0 * t - t * t),
        "no overlap, no gap"
    );

    f.h.undo()?;
    assert_eq!(f.plan()?, mitre, "back to the mitre");
    Ok(())
}

#[test]
fn stories_finished_and_structural_level() -> Result {
    let mut f = Fixture::new()?;
    let concrete = f.material("concrete", 9)?;
    f.h.run(Command::SetStoryLevels {
        story: f.story1,
        finished_level: 3000.0,
        floor_build_up: 80.0,
    })?;
    let wall = f.wall((0.0, 0.0), (5000.0, 0.0), &[(concrete, 200.0)])?;
    let window = f.h.new_id();
    f.h.run(Command::AddOpening(Opening {
        id: window,
        wall,
        kind: OpeningKind::Exterior(Performance::default()),
        position: 1000.0,
        sill: 1000.0,
        height: 1200.0,
        frame: 60.0,
        parts: vec![Part {
            width: 1000.0,
            operation: Operation::Fixed,
        }],
        nominal: NominalSize::ClearOpening,
    }))?;
    assert_eq!(f.h.project().wall_extent(wall)?, (0.0, 2920.0));
    let changed = f.h.run(Command::SetStoryLevels {
        story: f.story1,
        finished_level: 3000.0,
        floor_build_up: 120.0,
    })?;
    assert!(changed.contains(&wall));
    assert_eq!(f.h.project().story(f.story1)?.structural_level(), 2880.0);
    assert_eq!(
        f.h.project().wall_extent(wall)?,
        (0.0, 2880.0),
        "the wall stops at the new structural level"
    );
    // The window sits on story 0 here; its sill follows the finished level only.
    assert_eq!(f.h.project().sill_level(window)?, 1000.0);
    Ok(())
}

fn door(
    f: &mut Fixture,
    nominal: NominalSize,
) -> std::result::Result<Id, Box<dyn std::error::Error>> {
    let concrete = f.material("concrete", 9)?;
    let wall = f.wall((0.0, 0.0), (5000.0, 0.0), &[(concrete, 200.0)])?;
    let id = f.h.new_id();
    f.h.run(Command::AddOpening(Opening {
        id,
        wall,
        kind: OpeningKind::Interior,
        position: 1000.0,
        sill: 0.0,
        height: 2150.0,
        frame: 68.0,
        parts: vec![Part {
            width: 900.0,
            operation: Operation::SideHung(Hand::Right),
        }],
        nominal,
    }))?;
    Ok(id)
}

#[test]
fn windows_add_side_panel() -> Result {
    let mut f = Fixture::new()?;
    let id = door(&mut f, NominalSize::ClearOpening)?;
    let before = f.h.project().opening(id)?.wall_opening_width();
    f.h.run(Command::AddOpeningPart {
        opening: id,
        part: Part {
            width: 400.0,
            operation: Operation::Fixed,
        },
        at_left: true,
    })?;
    let o = f.h.project().opening(id)?;
    assert_eq!(
        o.parts[1],
        Part {
            width: 900.0,
            operation: Operation::SideHung(Hand::Right)
        },
        "the leaf is unchanged"
    );
    assert_eq!(o.height, 2150.0);
    assert_eq!(o.frame, 68.0);
    assert!(
        close(o.wall_opening_width(), before + 400.0 + 68.0),
        "the hole grows by the panel and its mullion"
    );

    let mut fixed = Fixture::new()?;
    let id = door(&mut fixed, NominalSize::WallOpening)?;
    let before = fixed.h.project().opening(id)?.wall_opening_width();
    fixed.h.run(Command::AddOpeningPart {
        opening: id,
        part: Part {
            width: 200.0,
            operation: Operation::Fixed,
        },
        at_left: true,
    })?;
    let o = fixed.h.project().opening(id)?;
    assert!(
        close(o.wall_opening_width(), before),
        "a fixed overall size is kept"
    );
    assert!(
        close(o.parts[1].width, 900.0 - 200.0 - 68.0),
        "the leaf gives the width"
    );
    Ok(())
}

#[test]
fn windows_change_part_opening() -> Result {
    let mut f = Fixture::new()?;
    let id = door(&mut f, NominalSize::ClearOpening)?;
    for at_left in [true, false] {
        f.h.run(Command::AddOpeningPart {
            opening: id,
            part: Part {
                width: 600.0,
                operation: Operation::Fixed,
            },
            at_left,
        })?;
    }
    let before = f.h.project().opening(id)?.clone();
    f.h.run(Command::SetPartOperation {
        opening: id,
        part: 1,
        operation: Operation::Sliding,
    })?;
    let after = f.h.project().opening(id)?;
    assert_eq!(after.parts[1].operation, Operation::Sliding);
    assert_eq!(after.parts[0], before.parts[0]);
    assert_eq!(after.parts[2], before.parts[2]);
    assert_eq!(after.wall_opening_width(), before.wall_opening_width());
    Ok(())
}

#[test]
fn windows_nominal_size_reference() -> Result {
    let mut f = Fixture::new()?;
    let id = door(&mut f, NominalSize::ClearOpening)?;
    let mut o = f.h.project().opening(id)?.clone();
    o.set_frame(90.0)?;
    assert_eq!(o.clear_width(), 900.0, "the clear opening stays");
    assert!(
        close(o.wall_opening_width(), 900.0 + 180.0),
        "the hole follows"
    );
    Ok(())
}

#[test]
fn openings_interior_exterior() -> Result {
    let mut f = Fixture::new()?;
    let id = door(&mut f, NominalSize::ClearOpening)?;
    let before = f.h.project().opening(id)?.clone();
    let exterior = OpeningKind::Exterior(Performance {
        thermal_uw: Some(1.3),
        ..Performance::default()
    });
    f.h.run(Command::SetOpeningKind {
        opening: id,
        kind: exterior,
    })?;
    let after = f.h.project().opening(id)?;
    assert_eq!(after.kind, exterior);
    assert_eq!(
        (&after.parts, after.height, after.frame),
        (&before.parts, before.height, before.frame),
        "sizes and parts kept"
    );
    Ok(())
}

#[test]
fn undo_and_redo_restore_the_project() -> Result {
    let mut f = Fixture::new()?;
    let (_, _, [.., brick]) = external_wall(&mut f, 5)?;
    f.h.run(Command::SetMaterialPriority {
        material: brick,
        priority: 10,
    })?;
    let end = f.h.project().clone();
    while f.h.undo()?.is_some() {}
    assert!(f.h.project().walls.is_empty());
    assert!(f.h.project().materials.is_empty());
    assert!(f.h.project().stories.is_empty());
    while f.h.redo()?.is_some() {}
    assert_eq!(f.h.project(), &end, "identifiers included");
    Ok(())
}
