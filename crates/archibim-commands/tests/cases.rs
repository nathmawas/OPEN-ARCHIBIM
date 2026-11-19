//! The architect test cases of `tests/cases`, run through commands.
//!
//! Each test is named after its case. The cases are still `proposed`: these
//! tests check the prototype against them and will follow their wording once
//! an architect agrees them.

use archibim_commands::{Command, History};
use archibim_geometry::{LayerPlan, Phase, model_prisms, plan_layers, polygon_area};
use archibim_model::{
    Beam, Column, Framing, Hand, Id, JunctionMode, Layer, LayerRole, LevelKind, LevelRef, Material,
    NominalSize, Opening, OpeningKind, Operation, Part, Performance, Point, Project,
    RenovationStatus, Slab, Story, Wall,
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
        let layers = layers
            .iter()
            .map(|&(material, thickness)| layer_of(LayerRole::LoadBearing, material, thickness))
            .collect();
        self.wall_with(start, end, layers)
    }

    fn wall_with(
        &mut self,
        start: (f64, f64),
        end: (f64, f64),
        layers: Vec<Layer>,
    ) -> std::result::Result<Id, Box<dyn std::error::Error>> {
        let id = self.h.new_id();
        self.h.run(Command::AddWall(Wall {
            id,
            story: self.story0,
            start: Point::new(start.0, start.1),
            end: Point::new(end.0, end.1),
            layers,
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
        Ok(plan_layers(self.h.project(), self.story0, Phase::Proposed)?)
    }
}

fn layer_of(role: LayerRole, material: Id, thickness: f64) -> Layer {
    Layer {
        role,
        material,
        thickness,
        status: RenovationStatus::New,
        framing: None,
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

#[test]
fn walls_timber_frame_studs() -> Result {
    let mut f = Fixture::new()?;
    let render = f.material("render", 1)?;
    let fibre = f.material("wood fibre board", 2)?;
    let wool = f.material("wood wool", 2)?;
    let timber = f.material("timber", 6)?;
    let osb = f.material("OSB", 4)?;
    let board = f.material("plasterboard", 1)?;
    let studs = |spacing| Layer {
        framing: Some(Framing {
            material: timber,
            width: 45.0,
            spacing,
        }),
        ..layer_of(LayerRole::Insulation, wool, 145.0)
    };
    let build_up = |spacing| {
        vec![
            layer_of(LayerRole::Finish, render, 15.0),
            layer_of(LayerRole::Insulation, fibre, 60.0),
            studs(spacing),
            layer_of(LayerRole::Lining, osb, 12.0),
            layer_of(LayerRole::Lining, board, 13.0),
        ]
    };
    let wall = f.wall_with((0.0, 0.0), (5000.0, 0.0), build_up(600.0))?;
    let plan = f.plan()?;
    let members = plan
        .iter()
        .find(|l| l.wall == wall && l.layer == 2)
        .map(|l| l.members.clone())
        .unwrap_or_default();
    // Centres at 22.5, 622.5 ... 4822.5, plus one at the far end, 4977.5.
    assert_eq!(members.len(), 10, "a stud at each end and every 600");
    assert!(
        members.iter().all(|m| close(polygon_area(m), 45.0 * 145.0)),
        "each stud is 45 by 145"
    );
    let xs = |m: &Vec<Point>| m.iter().map(|p| p.x).fold(f64::MAX, f64::min);
    assert!(close(xs(&members[0]), 0.0) && close(xs(&members[1]), 600.0));
    assert!(
        plan.iter()
            .filter(|l| l.layer != 2)
            .all(|l| l.members.is_empty()),
        "only the insulation holds studs"
    );

    f.h.run(Command::SetWallLayers {
        wall,
        layers: build_up(400.0),
    })?;
    let plan = f.plan()?;
    let members = plan
        .iter()
        .find(|l| l.wall == wall && l.layer == 2)
        .map(|l| l.members.len())
        .unwrap_or(0);
    assert_eq!(
        members, 14,
        "redrawn every 400: 13 from the start, one at the end"
    );
    Ok(())
}

#[test]
fn renovation_replace_insulation() -> Result {
    let mut f = Fixture::new()?;
    let stone = f.material("stone", 8)?;
    let wool = f.material("mineral wool", 2)?;
    let board = f.material("plasterboard", 1)?;
    let existing = |role, material, thickness| Layer {
        status: RenovationStatus::Existing,
        ..layer_of(role, material, thickness)
    };
    let before = vec![
        existing(LayerRole::LoadBearing, stone, 450.0),
        existing(LayerRole::Insulation, wool, 80.0),
        existing(LayerRole::Lining, board, 13.0),
    ];
    let wall = f.wall_with((0.0, 0.0), (5000.0, 0.0), before.clone())?;
    let window = f.h.new_id();
    f.h.run(Command::AddOpening(Opening {
        id: window,
        wall,
        kind: OpeningKind::Exterior(Performance::default()),
        position: 2000.0,
        sill: 900.0,
        height: 1200.0,
        frame: 60.0,
        parts: vec![Part {
            width: 1000.0,
            operation: Operation::TiltAndTurn(Hand::Left),
        }],
        nominal: NominalSize::ClearOpening,
    }))?;

    let mut after = before.clone();
    after[1].status = RenovationStatus::ToDemolish;
    after[2].status = RenovationStatus::ToDemolish;
    after.push(layer_of(LayerRole::Insulation, wool, 120.0));
    after.push(layer_of(LayerRole::Lining, board, 13.0));
    f.h.run(Command::SetWallLayers {
        wall,
        layers: after,
    })?;

    assert_eq!(f.h.project().walls.len(), 1, "it stays one wall");
    assert_eq!(
        f.h.project().opening(window)?.wall,
        wall,
        "with the same window in it"
    );
    let depth = |phase| -> std::result::Result<f64, Box<dyn std::error::Error>> {
        let plan = plan_layers(f.h.project(), f.story0, phase)?;
        Ok(-ys(&plan
            .iter()
            .flat_map(|l| l.polygons.clone())
            .collect::<Vec<_>>())
        .0)
    };
    assert!(close(depth(Phase::Existing)?, 543.0), "existing state");
    assert!(close(depth(Phase::Proposed)?, 583.0), "new state");

    f.h.undo()?;
    assert_eq!(
        f.h.project().wall(wall)?.layers,
        before,
        "undo brings the build-up back"
    );
    Ok(())
}

#[test]
fn openings_cut_the_wall() -> Result {
    let mut f = Fixture::new()?;
    let render = f.material("render", 1)?;
    let insulation = f.material("insulation", 3)?;
    let concrete = f.material("concrete", 9)?;
    let plaster = f.material("plaster", 1)?;
    let wall = f.wall(
        (0.0, 0.0),
        (5000.0, 0.0),
        &[
            (render, 20.0),
            (insulation, 140.0),
            (concrete, 200.0),
            (plaster, 15.0),
        ],
    )?;
    let id = f.h.new_id();
    let window = Opening {
        id,
        wall,
        kind: OpeningKind::Exterior(Performance::default()),
        position: 1900.0,
        sill: 900.0,
        height: 1350.0,
        frame: 60.0,
        parts: vec![Part {
            width: 1080.0,
            operation: Operation::SideHung(Hand::Left),
        }],
        nominal: NominalSize::ClearOpening,
    };
    f.h.run(Command::AddOpening(window.clone()))?;
    let plan = f.plan()?;
    for i in 0..4 {
        assert_eq!(layer(&plan, wall, i).len(), 2, "layer {i} is cut");
    }
    let t = 375.0;
    assert!(
        close(total_area(&plan), (5000.0 - 1200.0) * t),
        "the hole is 1200 wide"
    );
    f.h.run(Command::ReplaceOpening(Opening {
        position: 3000.0,
        ..window
    }))?;
    let xs: Vec<f64> = layer(&f.plan()?, wall, 2)
        .iter()
        .flatten()
        .map(|p| p.x)
        .collect();
    assert!(xs.iter().any(|&x| close(x, 3000.0)), "the cut follows");
    Ok(())
}

#[test]
fn slabs_columns_and_beams_are_added_edited_and_undone() -> Result {
    let mut f = Fixture::new()?;
    let concrete = f.material("concrete", 9)?;
    let level = |story| LevelRef {
        story,
        kind: LevelKind::Structural,
        offset: 0.0,
    };
    let slab = Slab {
        id: f.h.new_id(),
        story: f.story0,
        outline: [
            (0.0, 0.0),
            (4000.0, 0.0),
            (4000.0, 2000.0),
            (2000.0, 2000.0),
            (2000.0, 4000.0),
            (0.0, 4000.0),
        ]
        .iter()
        .map(|&(x, y)| Point::new(x, y))
        .collect(),
        layers: vec![layer_of(LayerRole::LoadBearing, concrete, 200.0)],
        top: level(f.story0),
    };
    f.h.run(Command::AddSlab(slab.clone()))?;
    let flat = Slab {
        id: f.h.new_id(),
        outline: vec![Point::new(0.0, 0.0), Point::new(1000.0, 0.0)],
        ..slab.clone()
    };
    assert!(f.h.run(Command::AddSlab(flat)).is_err(), "no area, refused");

    let column = Column {
        id: f.h.new_id(),
        story: f.story0,
        at: Point::new(1000.0, 1000.0),
        width: 300.0,
        depth: 300.0,
        material: concrete,
        bottom: level(f.story0),
        top: level(f.story1),
    };
    f.h.run(Command::AddColumn(column.clone()))?;
    let beam = Beam {
        id: f.h.new_id(),
        story: f.story0,
        start: Point::new(0.0, 0.0),
        end: Point::new(4000.0, 0.0),
        width: 200.0,
        height: 400.0,
        material: concrete,
        top: level(f.story1),
    };
    f.h.run(Command::AddBeam(beam.clone()))?;

    let mut thick = slab.clone();
    thick.layers[0].thickness = 250.0;
    f.h.run(Command::ReplaceSlab(thick))?;
    f.h.run(Command::ReplaceColumn(Column {
        width: 400.0,
        ..column.clone()
    }))?;
    f.h.run(Command::ReplaceBeam(Beam {
        height: 500.0,
        ..beam.clone()
    }))?;
    let p = f.h.project();
    assert!(close(p.slab(slab.id)?.thickness(), 250.0));
    assert!(close(p.column(column.id)?.width, 400.0));
    assert!(close(p.beam(beam.id)?.height, 500.0));

    for _ in 0..3 {
        f.h.undo()?;
    }
    let p = f.h.project();
    assert_eq!(p.slab(slab.id)?, &slab);
    assert_eq!(p.column(column.id)?, &column);
    assert_eq!(p.beam(beam.id)?, &beam);
    f.h.run(Command::RemoveColumn(column.id))?;
    f.h.undo()?;
    assert_eq!(f.h.project().column(column.id)?, &column, "delete undone");
    Ok(())
}

#[test]
fn views_3d_model() -> Result {
    let mut f = Fixture::new()?;
    let concrete = f.material("concrete", 9)?;
    let wall = f.wall((0.0, 0.0), (5000.0, 0.0), &[(concrete, 200.0)])?;
    let level = |story| LevelRef {
        story,
        kind: LevelKind::Structural,
        offset: 0.0,
    };
    let window = Opening {
        id: f.h.new_id(),
        wall,
        kind: OpeningKind::Exterior(Performance::default()),
        position: 1900.0,
        sill: 900.0,
        height: 1350.0,
        frame: 60.0,
        parts: vec![Part {
            width: 1080.0,
            operation: Operation::SideHung(Hand::Left),
        }],
        nominal: NominalSize::ClearOpening,
    };
    f.h.run(Command::AddOpening(window))?;
    let square = |x: f64, y: f64, half: f64| {
        vec![
            Point::new(x - half, y - half),
            Point::new(x + half, y - half),
            Point::new(x + half, y + half),
            Point::new(x - half, y + half),
        ]
    };
    let slab = f.h.new_id();
    f.h.run(Command::AddSlab(Slab {
        id: slab,
        story: f.story0,
        outline: square(2500.0, 2000.0, 2500.0),
        layers: vec![layer_of(LayerRole::LoadBearing, concrete, 200.0)],
        top: level(f.story0),
    }))?;
    let column = f.h.new_id();
    f.h.run(Command::AddColumn(Column {
        id: column,
        story: f.story0,
        at: Point::new(2500.0, 2000.0),
        width: 300.0,
        depth: 300.0,
        material: concrete,
        bottom: level(f.story0),
        top: level(f.story1),
    }))?;
    let beam = f.h.new_id();
    f.h.run(Command::AddBeam(Beam {
        id: beam,
        story: f.story0,
        start: Point::new(0.0, 2000.0),
        end: Point::new(5000.0, 2000.0),
        width: 200.0,
        height: 400.0,
        material: concrete,
        top: level(f.story1),
    }))?;

    let volume =
        |p: &Project, element: Id| -> std::result::Result<f64, Box<dyn std::error::Error>> {
            Ok(model_prisms(p, Phase::Proposed)?
                .iter()
                .filter(|q| q.element == element)
                .map(|q| polygon_area(&q.outline) * (q.top - q.bottom))
                .sum())
        };
    let heights =
        |p: &Project, element: Id| -> std::result::Result<(f64, f64), Box<dyn std::error::Error>> {
            let prisms = model_prisms(p, Phase::Proposed)?;
            let of = prisms.iter().filter(|p| p.element == element);
            Ok((
                of.clone().map(|p| p.bottom).fold(f64::MAX, f64::min),
                of.map(|p| p.top).fold(f64::MIN, f64::max),
            ))
        };
    assert_eq!(
        heights(f.h.project(), wall)?,
        (0.0, 2800.0),
        "wall from floor to floor"
    );
    assert!(
        close(
            volume(f.h.project(), wall)?,
            (5000.0 * 2800.0 - 1200.0 * 1350.0) * 200.0
        ),
        "the window leaves its hole, the wall stays under and over it"
    );
    assert_eq!(
        heights(f.h.project(), slab)?,
        (-200.0, 0.0),
        "slab under its top"
    );
    assert_eq!(heights(f.h.project(), column)?, (0.0, 2800.0));
    assert_eq!(
        heights(f.h.project(), beam)?,
        (2400.0, 2800.0),
        "beam under its top"
    );

    f.h.undo()?; // the beam
    f.h.undo()?; // the column
    f.h.undo()?; // the slab
    f.h.undo()?; // the window
    assert!(
        close(volume(f.h.project(), wall)?, 5000.0 * 2800.0 * 200.0),
        "undo fills it"
    );
    Ok(())
}
