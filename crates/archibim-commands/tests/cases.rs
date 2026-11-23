//! The architect test cases of `tests/cases`, run through commands.
//!
//! Each test is named after its case. The cases are still `proposed`: these
//! tests check the prototype against them and will follow their wording once
//! an architect agrees them.

use archibim_commands::{Command, History};
use archibim_geometry::{
    LayerPlan, Phase, beam_outline, column_outline, contour_at, cuts_through, floor_contour_at,
    mesh_blocks, mesh_contours, mesh_triangles, model_blocks, model_prisms, plan_layers,
    polygon_area, roof_blocks, roof_faces, roof_overhang, stair_steps,
};
use archibim_model::{
    Align, Anchor, Beam, Column, Framing, Guide, GuideShape, Hand, Id, JunctionMode, Layer,
    LayerRole, LevelKind, LevelRef, Material, Mesh, MeshPoint, NominalSize, Opening, OpeningKind,
    Operation, Part, Performance, Point, Project, RenovationStatus, Roof, RoofEdge, RoofPlane,
    Slab, Stair, Story, Wall, WallReference,
};

use std::f64::consts::FRAC_PI_2;

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
            reference: WallReference::LeftFace,
            offset: 0.0,
            sweep: 0.0,
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
        anchor: Anchor::default(),
        angle: 0.0,
        width: 300.0,
        depth: 300.0,
        round: false,
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
        anchor: Anchor {
            width: Align::Middle,
            depth: Align::First,
        },
        level: level(f.story1),
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
        anchor: Anchor::default(),
        angle: 0.0,
        width: 300.0,
        depth: 300.0,
        round: false,
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
        anchor: Anchor {
            width: Align::Middle,
            depth: Align::First,
        },
        level: level(f.story1),
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

#[test]
fn walls_drawing_methods() -> Result {
    // The wall tool is tested in archibim-ui; here, the group command it
    // relies on: four walls in one step, nothing kept if one is refused.
    let mut f = Fixture::new()?;
    let concrete = f.material("concrete", 9)?;
    let corners = [(0.0, 0.0), (0.0, 4000.0), (6000.0, 4000.0), (6000.0, 0.0)];
    let mut walls: Vec<Wall> = Vec::new();
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        let id = f.wall(a, b, &[(concrete, 200.0)])?;
        walls.push(f.h.project().wall(id)?.clone());
    }
    for _ in 0..4 {
        f.h.undo()?;
    }
    let group = Command::Group(walls.iter().cloned().map(Command::AddWall).collect());
    f.h.run(group)?;
    assert_eq!(f.h.project().walls.len(), 4);
    f.h.undo()?;
    assert!(f.h.project().walls.is_empty(), "one undo, four walls");
    f.h.redo()?;
    assert_eq!(f.h.project().walls.len(), 4, "one redo, four walls");

    let mut flat = walls[0].clone();
    flat.id = f.h.new_id();
    flat.layers[0].material = Id(u64::MAX);
    let extra = Wall {
        id: f.h.new_id(),
        ..walls[1].clone()
    };
    let refused = Command::Group(vec![Command::AddWall(extra), Command::AddWall(flat)]);
    assert!(f.h.run(refused).is_err(), "a material that does not exist");
    assert_eq!(f.h.project().walls.len(), 4, "nothing of the group is kept");

    let point = Wall {
        id: f.h.new_id(),
        end: walls[0].start,
        ..walls[0].clone()
    };
    assert!(
        f.h.run(Command::AddWall(point)).is_err(),
        "a wall needs a length"
    );
    Ok(())
}

/// The smallest and largest y of every polygon of a wall.
fn y_range(plan: &[LayerPlan], wall: Id) -> (f64, f64) {
    plan.iter()
        .filter(|l| l.wall == wall)
        .flat_map(|l| l.polygons.iter().flatten())
        .fold((f64::MAX, f64::MIN), |(lo, hi), p| {
            (lo.min(p.y), hi.max(p.y))
        })
}

#[test]
fn walls_reference_line() -> Result {
    let mut f = Fixture::new()?;
    let render = f.material("render", 1)?;
    let insulation = f.material("insulation", 3)?;
    let concrete = f.material("concrete", 9)?;
    let plaster = f.material("plaster", 1)?;

    // On the axis, the line stays put when the wall gets thicker.
    let a = f.wall((0.0, 0.0), (5000.0, 0.0), &[(concrete, 200.0)])?;
    let mut wall = f.h.project().wall(a)?.clone();
    wall.reference = WallReference::Axis;
    f.h.run(Command::ReplaceWall(wall.clone()))?;
    let (lo, hi) = y_range(&f.plan()?, a);
    assert!(close(lo, -100.0) && close(hi, 100.0), "{lo} {hi}");
    wall.layers[0].thickness = 300.0;
    f.h.run(Command::ReplaceWall(wall))?;
    let (lo, hi) = y_range(&f.plan()?, a);
    assert!(close(lo, -150.0) && close(hi, 150.0), "{lo} {hi}");
    f.h.undo()?;
    f.h.undo()?;
    let (lo, hi) = y_range(&f.plan()?, a);
    assert!(close(lo, -200.0) && close(hi, 0.0), "undone: left face");
    f.h.undo()?;

    // On the outside face of the concrete: render and insulation outside
    // (to the left, y > 0), plaster inside.
    let b = f.wall_with(
        (0.0, 0.0),
        (5000.0, 0.0),
        vec![
            layer_of(LayerRole::Finish, render, 20.0),
            layer_of(LayerRole::Insulation, insulation, 140.0),
            layer_of(LayerRole::LoadBearing, concrete, 200.0),
            layer_of(LayerRole::Lining, plaster, 15.0),
        ],
    )?;
    let mut wall = f.h.project().wall(b)?.clone();
    wall.reference = WallReference::LoadBearingLeft;
    f.h.run(Command::ReplaceWall(wall.clone()))?;
    let (lo, hi) = y_range(&f.plan()?, b);
    assert!(close(hi, 160.0) && close(lo, -215.0), "{lo} {hi}");
    // Without load-bearing layers, back on the face on its side.
    wall.reference = WallReference::LoadBearingRight;
    for layer in &mut wall.layers {
        layer.role = LayerRole::Lining;
    }
    f.h.run(Command::ReplaceWall(wall))?;
    let (lo, hi) = y_range(&f.plan()?, b);
    assert!(close(lo, 0.0) && close(hi, 375.0), "{lo} {hi}");
    f.h.undo()?;
    f.h.undo()?;
    f.h.undo()?;

    // Two thicknesses on their axis meet on a mitre, no gap, no overlap.
    let c = f.wall((0.0, 0.0), (5000.0, 0.0), &[(concrete, 200.0)])?;
    let d = f.wall((5000.0, 0.0), (5000.0, -5000.0), &[(concrete, 300.0)])?;
    for id in [c, d] {
        let mut wall = f.h.project().wall(id)?.clone();
        wall.reference = WallReference::Axis;
        f.h.run(Command::ReplaceWall(wall))?;
    }
    let plan = f.plan()?;
    assert_eq!(
        shared_vertices(&layer(&plan, c, 0), &layer(&plan, d, 0)),
        2,
        "the corner closes"
    );
    // The L of axes 5000 by 5000, 200 then 300 thick, mitred.
    let expected = (5000.0 + 150.0) * 200.0 + (5000.0 + 100.0) * 300.0 - 200.0 * 300.0;
    assert!(close(total_area(&plan), expected), "{}", total_area(&plan));
    Ok(())
}

#[test]
fn structure_placed_point() -> Result {
    let mut f = Fixture::new()?;
    let concrete = f.material("concrete", 9)?;
    let level = |story| LevelRef {
        story,
        kind: LevelKind::Finished,
        offset: 0.0,
    };
    let column = Column {
        id: f.h.new_id(),
        story: f.story0,
        at: Point::new(1000.0, 1000.0),
        anchor: Anchor {
            width: Align::First,
            depth: Align::First,
        },
        angle: 0.0,
        width: 300.0,
        depth: 200.0,
        round: false,
        material: concrete,
        bottom: level(f.story0),
        top: level(f.story1),
    };
    let bounds = |points: &[Point]| {
        points.iter().fold(
            (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
            |(x0, y0, x1, y1), p| (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y)),
        )
    };
    let (x0, y0, x1, y1) = bounds(&column_outline(&column));
    assert!(close(x0, 1000.0) && close(x1, 1300.0), "right of the click");
    assert!(close(y0, 1000.0) && close(y1, 1200.0), "behind the click");
    let turned = Column {
        angle: std::f64::consts::FRAC_PI_2,
        ..column.clone()
    };
    let (x0, y0, x1, y1) = bounds(&column_outline(&turned));
    assert!(
        close(x0, 800.0) && close(x1, 1000.0),
        "turned around the click"
    );
    assert!(
        close(y0, 1000.0) && close(y1, 1300.0),
        "turned around the click"
    );
    let round = Column {
        round: true,
        anchor: Anchor::default(),
        width: 400.0,
        ..column.clone()
    };
    let (x0, y0, x1, y1) = bounds(&column_outline(&round));
    assert!(
        close(x0, 800.0) && close(x1, 1200.0),
        "diameter 400 round the click"
    );
    assert!(close(y0, 800.0) && close(y1, 1200.0));
    let wider = Column {
        width: 500.0,
        ..column.clone()
    };
    assert!(
        close(bounds(&column_outline(&wider)).0, 1000.0),
        "kept point"
    );

    let beam = Beam {
        id: f.h.new_id(),
        story: f.story0,
        start: Point::new(0.0, 0.0),
        end: Point::new(4000.0, 0.0),
        anchor: Anchor {
            width: Align::First,
            depth: Align::Last,
        },
        width: 200.0,
        height: 400.0,
        material: concrete,
        level: level(f.story1),
    };
    f.h.run(Command::AddBeam(beam.clone()))?;
    let (_, y0, _, y1) = bounds(&beam_outline(&beam).ok_or("no length")?);
    assert!(close(y0, -200.0) && close(y1, 0.0), "right of the line");
    let prism = model_prisms(f.h.project(), Phase::Proposed)?
        .into_iter()
        .find(|p| p.element == beam.id)
        .ok_or("no beam volume")?;
    assert!(close(prism.bottom, 2800.0), "bottom on the level");
    assert!(close(prism.top, 3200.0));
    Ok(())
}

#[test]
fn walls_curved() -> Result {
    let mut f = Fixture::new()?;
    let concrete = f.material("concrete", 9)?;
    // A half circle of radius 3000 on its axis, 200 thick.
    let id = f.wall((3000.0, 0.0), (-3000.0, 0.0), &[(concrete, 200.0)])?;
    let mut wall = f.h.project().wall(id)?.clone();
    wall.sweep = std::f64::consts::PI;
    wall.reference = WallReference::Axis;
    f.h.run(Command::ReplaceWall(wall.clone()))?;
    let area = total_area(&f.plan()?);
    let exact = std::f64::consts::PI / 2.0 * (3100.0_f64.powi(2) - 2900.0_f64.powi(2));
    assert!((area - exact).abs() / exact < 1e-3, "{area} {exact}");
    // Made thicker, the axis stays at radius 3000.
    wall.layers[0].thickness = 300.0;
    f.h.run(Command::ReplaceWall(wall.clone()))?;
    let far = f
        .plan()?
        .iter()
        .flat_map(|l| l.polygons.iter().flatten())
        .map(|p| (p.x * p.x + p.y * p.y).sqrt())
        .fold(0.0_f64, f64::max);
    assert!((far - 3150.0).abs() < 1e-6, "{far}");
    // Volumes follow the plan.
    assert!(!model_prisms(f.h.project(), Phase::Proposed)?.is_empty());
    let opening = Opening {
        id: f.h.new_id(),
        wall: id,
        kind: OpeningKind::Interior,
        position: 1000.0,
        sill: 0.0,
        height: 2000.0,
        frame: 50.0,
        parts: vec![Part {
            width: 800.0,
            operation: Operation::Fixed,
        }],
        nominal: NominalSize::ClearOpening,
    };
    assert!(
        f.h.run(Command::AddOpening(opening)).is_err(),
        "openings in curved walls come later"
    );
    wall.sweep = std::f64::consts::TAU;
    assert!(
        f.h.run(Command::ReplaceWall(wall)).is_err(),
        "not a full turn"
    );
    Ok(())
}

fn rect_outline(w: f64, d: f64) -> Vec<Point> {
    [(0.0, 0.0), (w, 0.0), (w, d), (0.0, d)]
        .iter()
        .map(|&(x, y)| Point::new(x, y))
        .collect()
}

#[test]
fn roofs_slopes_and_gables() -> Result {
    let mut f = Fixture::new()?;
    let tiles = f.material("tiles", 3)?;
    let insulation = f.material("insulation", 2)?;
    let timber = f.material("timber", 4)?;
    let eaves = LevelRef {
        story: f.story1,
        kind: LevelKind::Finished,
        offset: 0.0,
    };
    let roof_of = |f: &mut Fixture, slopes: [Option<f64>; 4], overhang: f64| Roof {
        id: f.h.new_id(),
        story: f.story1,
        outline: rect_outline(10_000.0, 6_000.0),
        edges: slopes
            .iter()
            .map(|&slope| RoofEdge { slope, overhang })
            .collect(),
        layers: [(tiles, 40.0), (insulation, 200.0), (timber, 60.0)]
            .iter()
            .map(|&(m, t)| layer_of(LayerRole::LoadBearing, m, t))
            .collect(),
        eaves,
        plane: RoofPlane::Top,
    };
    let s45 = Some(45f64.to_radians());
    let top_of = |blocks: &[archibim_geometry::Block]| {
        blocks
            .iter()
            .flat_map(|b| b.top.iter().copied())
            .fold(f64::MIN, f64::max)
    };

    // One sloped edge: shed; two opposite: gabled; all: hipped.
    let shed = roof_of(&mut f, [s45, None, None, None], 0.0);
    assert_eq!(roof_faces(&shed).len(), 1);
    let hipped = roof_of(&mut f, [s45; 4], 0.0);
    assert_eq!(roof_faces(&hipped).len(), 4);
    let gabled = roof_of(&mut f, [s45, None, s45, None], 0.0);
    assert_eq!(roof_faces(&gabled).len(), 2);

    // Ridge 3 m above the eaves level (the top of the build-up).
    let id = gabled.id;
    f.h.run(Command::AddRoof(gabled))?;
    let eaves_z = f.h.project().level(&eaves)?;
    assert!(close(eaves_z, 2_800.0));
    let blocks = roof_blocks(f.h.project().roof(id)?, eaves_z, Phase::Proposed);
    assert!((top_of(&blocks) - (eaves_z + 3_000.0)).abs() < 1.0);
    // The gables stop on the outline.
    let x = blocks.iter().flat_map(|b| &b.outline).map(|p| p.x);
    assert!(close(x.clone().fold(f64::MAX, f64::min), 0.0));
    assert!(close(x.fold(f64::MIN, f64::max), 10_000.0));

    // Overhang on the eaves: out past the edge, lower along the slope.
    let mut wide = f.h.project().roof(id)?.clone();
    wide.edges[0].overhang = 500.0;
    wide.edges[2].overhang = 500.0;
    f.h.run(Command::ReplaceRoof(wide.clone()))?;
    let (y0, y1) = ys(&[roof_overhang(&wide)]);
    assert!(close(y0, -500.0) && close(y1, 6_500.0));
    let blocks = roof_blocks(&wide, eaves_z, Phase::Proposed);
    let low = blocks
        .iter()
        .step_by(3) // the top layer of each plane
        .flat_map(|b| b.top.iter().copied())
        .fold(f64::MAX, f64::min);
    assert!((low - (eaves_z - 500.0)).abs() < 1.0, "{low}");

    // Eaves level on the top or on the underside.
    let mut flat = roof_of(&mut f, [None; 4], 0.0);
    f.h.run(Command::AddRoof(flat.clone()))?;
    assert!(close(
        top_of(&roof_blocks(&flat, eaves_z, Phase::Proposed)),
        eaves_z
    ));
    flat.plane = RoofPlane::Bottom;
    f.h.run(Command::ReplaceRoof(flat.clone()))?;
    assert!(close(
        top_of(&roof_blocks(&flat, eaves_z, Phase::Proposed)),
        eaves_z + 300.0
    ));
    // Layer thicknesses are square to the slope.
    let steep = Some(60f64.to_radians());
    let sloped = roof_of(&mut f, [steep, None, steep, None], 0.0);
    let b = roof_blocks(&sloped, eaves_z, Phase::Proposed);
    let rise = b[0].top[0] - b[0].bottom[0];
    assert!(
        (rise - 40.0 / 60f64.to_radians().cos()).abs() < 1e-6,
        "{rise}"
    );

    // Refused: vertical, negative slope, no area, wrong number of settings.
    // Accepted: flat slope, zero overhang, and a negative overhang (a roof
    // stopping behind a parapet): a design choice.
    for bad in [
        [Some(FRAC_PI_2), None, None, None],
        [Some(-0.1), None, None, None],
    ] {
        let r = roof_of(&mut f, bad, 0.0);
        assert!(f.h.run(Command::AddRoof(r)).is_err(), "slope {bad:?}");
    }
    let r = roof_of(&mut f, [s45; 4], -300.0);
    let inside = r.id;
    f.h.run(Command::AddRoof(r))?;
    let (y0, y1) = ys(&[roof_overhang(f.h.project().roof(inside)?)]);
    assert!((y0 - 300.0).abs() < 1e-6 && (y1 - 5_700.0).abs() < 1e-6);
    f.h.undo()?;
    let mut r = roof_of(&mut f, [s45; 4], 0.0);
    r.outline = rect_outline(10_000.0, 0.0);
    assert!(f.h.run(Command::AddRoof(r)).is_err(), "no area");
    let mut r = roof_of(&mut f, [s45; 4], 0.0);
    r.edges.pop();
    assert!(f.h.run(Command::AddRoof(r)).is_err(), "edge count");
    let r = roof_of(&mut f, [Some(0.0), None, None, None], 0.0);
    let zero = r.id;
    f.h.run(Command::AddRoof(r))?;

    // Undo restores the roof, step by step.
    f.h.undo()?; // zero slope roof
    assert!(f.h.project().roof(zero).is_err());
    f.h.undo()?; // flat on its underside
    assert_eq!(f.h.project().roofs.len(), 2);
    assert_eq!(f.h.project().roofs[1].plane, RoofPlane::Top);
    f.h.undo()?; // flat roof added
    f.h.undo()?; // overhang
    assert_eq!(f.h.project().roof(id)?.edges[0].overhang, 0.0);
    f.h.undo()?;
    assert!(f.h.project().roofs.is_empty());
    Ok(())
}

fn mesh_point(x: f64, y: f64, z: f64) -> MeshPoint {
    MeshPoint {
        at: Point::new(x, y),
        z,
    }
}

#[test]
fn terrain_mesh_from_survey() -> Result {
    let mut f = Fixture::new()?;
    let soil = f.material("soil", 1)?;
    let outline = [
        (0.0, 0.0),
        (10_000.0, 0.0),
        (10_000.0, 10_000.0),
        (0.0, 10_000.0),
    ]
    .map(|(x, y)| mesh_point(x, y, 0.0))
    .to_vec();
    let mut mesh = Mesh {
        id: f.h.new_id(),
        story: f.story0,
        outline,
        inner: vec![],
        level: LevelRef {
            story: f.story0,
            kind: LevelKind::Finished,
            offset: 0.0,
        },
        bottom: -1_000.0,
        material: soil,
    };
    f.h.run(Command::AddMesh(mesh.clone()))?;
    assert_eq!(mesh_triangles(&mesh).1.len(), 2);

    // A point inside at 2 m splits the terrain around it.
    mesh.inner = vec![mesh_point(5_000.0, 5_000.0, 2_000.0)];
    f.h.run(Command::ReplaceMesh(mesh.clone()))?;
    let (points, triangles) = mesh_triangles(&mesh);
    assert_eq!((points.len(), triangles.len()), (5, 4));
    // A point outside is kept but has no effect.
    mesh.inner.push(mesh_point(50_000.0, 0.0, 9_000.0));
    f.h.run(Command::ReplaceMesh(mesh.clone()))?;
    assert_eq!(f.h.project().mesh(mesh.id)?.inner.len(), 2);
    assert_eq!(mesh_triangles(&mesh).0.len(), 5);
    assert_eq!(mesh_triangles(&mesh).1.len(), 4);

    // Surface through every point, solid down to the flat underside.
    let blocks = mesh_blocks(&mesh, 0.0);
    assert_eq!(blocks.len(), 4);
    assert!(
        blocks
            .iter()
            .all(|b| b.bottom.iter().all(|&z| close(z, -1_000.0)))
    );
    let tops: Vec<f64> = blocks.iter().flat_map(|b| b.top.clone()).collect();
    assert!(tops.iter().any(|&z| close(z, 2_000.0)));
    assert!(tops.iter().all(|&z| (0.0..=2_000.0).contains(&z)));

    // Contours every 0.5 m: 0.5, 1 and 1.5 m, nothing at the foot or top.
    let contours = mesh_contours(&mesh, 0.0, 500.0);
    let mut heights: Vec<f64> = contours.iter().map(|c| c.0).collect();
    heights.sort_by(f64::total_cmp);
    heights.dedup();
    assert_eq!(heights, vec![500.0, 1_000.0, 1_500.0]);
    // The stronger line every 2.5 m is above this hill.
    assert!(mesh_contours(&mesh, 0.0, 2_500.0).is_empty());

    // An outline without area is refused.
    let mut bad = mesh.clone();
    for p in &mut bad.outline {
        p.at.y = 0.0;
    }
    assert!(f.h.run(Command::ReplaceMesh(bad)).is_err());

    // Undo restores the heights.
    f.h.undo()?;
    assert_eq!(f.h.project().mesh(mesh.id)?.inner.len(), 1);
    f.h.undo()?;
    assert!(f.h.project().mesh(mesh.id)?.inner.is_empty());
    f.h.undo()?;
    assert!(f.h.project().mesh(mesh.id).is_err());
    Ok(())
}

#[test]
fn stairs_walking_line() -> Result {
    let mut f = Fixture::new()?;
    let concrete = f.material("concrete", 9)?;
    let level = |story| LevelRef {
        story,
        kind: LevelKind::Finished,
        offset: 0.0,
    };
    let mut stair = Stair {
        id: f.h.new_id(),
        story: f.story0,
        path: vec![Point::new(0.0, 0.0), Point::new(3_750.0, 0.0)],
        width: 900.0,
        bottom: level(f.story0),
        top: level(f.story1),
        risers: None,
        material: concrete,
    };
    f.h.run(Command::AddStair(stair.clone()))?;
    let steps_of = |f: &Fixture, s: &Stair| {
        let p = f.h.project();
        Ok::<_, archibim_model::ModelError>(stair_steps(s, p.level(&s.bottom)?, p.level(&s.top)?))
    };

    let steps = steps_of(&f, &stair)?;
    assert_eq!(steps.risers, 16);
    assert!(close(steps.riser, 175.0) && close(steps.going, 250.0));
    assert_eq!(steps.treads.len(), 15);
    assert!(close(steps.blondel(), 600.0));
    // The upper floor is the last tread: one riser above the last tread.
    assert!(close(
        *steps.heights.last().ok_or("no tread")?,
        2_800.0 - 175.0
    ));
    assert!(close(polygon_area(&steps.treads[0]), 250.0 * 900.0));

    // By hand: 14 risers, the going follows; never refused.
    stair.risers = Some(14);
    f.h.run(Command::ReplaceStair(stair.clone()))?;
    let steps = steps_of(&f, &stair)?;
    assert!(close(steps.riser, 200.0) && close(steps.going, 3_750.0 / 13.0));
    assert_eq!(steps.treads.len(), 13);
    assert!(steps.blondel() > 640.0, "outside the comfort range, kept");
    stair.risers = Some(1);
    f.h.run(Command::ReplaceStair(stair.clone()))?;
    assert!(steps_of(&f, &stair)?.treads.is_empty());
    let zero = Stair {
        risers: Some(0),
        ..stair.clone()
    };
    assert!(f.h.run(Command::ReplaceStair(zero)).is_err());

    // A turn is the same element: treads wrap around the corner.
    stair.risers = Some(5);
    stair.path = vec![
        Point::new(0.0, 0.0),
        Point::new(1_000.0, 0.0),
        Point::new(1_000.0, 1_000.0),
    ];
    f.h.run(Command::ReplaceStair(stair.clone()))?;
    let steps = steps_of(&f, &stair)?;
    assert_eq!(steps.treads.len(), 4);
    assert!(close(steps.going, 500.0));
    // Tread 2 ends on the corner; the one after it runs up the second flight.
    assert_eq!(steps.treads[1].len(), 4);
    assert!(steps.treads[2].iter().all(|p| p.x > 500.0));

    // Changing the story heights changes the steps, keeping width and
    // material.
    f.h.run(Command::SetStoryLevels {
        story: f.story1,
        finished_level: 3_000.0,
        floor_build_up: 0.0,
    })?;
    let steps = steps_of(&f, &stair)?;
    assert!(close(steps.riser, 600.0));
    assert_eq!(f.h.project().stair(stair.id)?.width, 900.0);
    assert_eq!(f.h.project().stair(stair.id)?.material, concrete);

    // Undo restores the stair.
    f.h.undo()?; // story heights
    f.h.undo()?; // turn
    assert_eq!(f.h.project().stair(stair.id)?.risers, Some(1));
    f.h.undo()?;
    f.h.undo()?;
    assert_eq!(f.h.project().stair(stair.id)?.risers, None);
    f.h.undo()?;
    assert!(f.h.project().stair(stair.id).is_err());
    // A stair needs a walking line and a width.
    let mut bad = stair.clone();
    bad.path.truncate(1);
    assert!(f.h.run(Command::AddStair(bad)).is_err());
    stair.width = 0.0;
    assert!(f.h.run(Command::AddStair(stair)).is_err());
    Ok(())
}

#[test]
fn guides_are_added_edited_cleared_and_undone() -> Result {
    let mut f = Fixture::new()?;
    let line = GuideShape::Line {
        a: Point::new(0.0, 0.0),
        b: Point::new(1000.0, 0.0),
    };
    let circle = GuideShape::Circle {
        centre: Point::new(500.0, 500.0),
        radius: 300.0,
    };
    let (l, c) = (f.h.new_id(), f.h.new_id());
    let story = f.story0;
    let guide = |id, shape| Guide { id, story, shape };
    let (gl, gc) = (guide(l, line), guide(c, circle));
    assert_eq!(f.h.run(Command::AddGuide(gl.clone()))?, vec![l]);
    f.h.run(Command::AddGuide(gc.clone()))?;
    let two = f.h.project().clone();
    // Edit, then undo and redo the edit.
    let moved = Guide {
        shape: GuideShape::Circle {
            centre: Point::new(0.0, 0.0),
            radius: 50.0,
        },
        ..gc.clone()
    };
    assert_eq!(f.h.run(Command::ReplaceGuide(moved.clone()))?, vec![c]);
    assert_eq!(f.h.project().guide(c)?, &moved);
    f.h.undo()?;
    assert_eq!(f.h.project(), &two);
    f.h.redo()?;
    assert_eq!(f.h.project().guide(c)?, &moved);
    f.h.undo()?;
    // A refused command changes nothing.
    let bad = Guide {
        shape: GuideShape::Circle {
            centre: Point::new(0.0, 0.0),
            radius: 0.0,
        },
        ..gc.clone()
    };
    assert!(f.h.run(Command::ReplaceGuide(bad)).is_err());
    assert!(f.h.run(Command::AddGuide(gl.clone())).is_err(), "same id");
    assert!(f.h.run(Command::RemoveGuide(Id(9999))).is_err());
    // The story keeps its guides.
    assert!(f.h.run(Command::RemoveStory(f.story0)).is_err());
    assert_eq!(f.h.project(), &two);
    // Clearing the story's guides is one undo step.
    f.h.run(Command::Group(vec![
        Command::RemoveGuide(l),
        Command::RemoveGuide(c),
    ]))?;
    assert!(f.h.project().guides.is_empty());
    f.h.undo()?;
    // Identifiers kept; a restored guide goes back at the end of the list,
    // as for every other element.
    let mut back = f.h.project().guides.clone();
    back.sort_by_key(|g| g.id.0);
    assert_eq!(back, two.guides);
    // A group that fails half way is rolled back whole.
    let after = f.h.project().clone();
    let r = f.h.run(Command::Group(vec![
        Command::RemoveGuide(l),
        Command::RemoveGuide(Id(9999)),
    ]));
    assert!(r.is_err());
    assert_eq!(f.h.project(), &after);
    // The two additions undo and redo one after the other.
    f.h.undo()?;
    f.h.undo()?;
    assert!(f.h.project().guides.is_empty());
    f.h.redo()?;
    f.h.redo()?;
    let mut back = f.h.project().guides.clone();
    back.sort_by_key(|g| g.id.0);
    assert_eq!(back, two.guides);
    Ok(())
}

#[test]
fn roofs_walls_cut_under() -> Result {
    let mut f = Fixture::new()?;
    let concrete = f.material("concrete", 5)?;
    let timber = f.material("timber", 4)?;
    let s45 = Some(45f64.to_radians());
    let roof = Roof {
        id: f.h.new_id(),
        story: f.story1,
        outline: rect_outline(10_000.0, 6_000.0),
        edges: [s45, None, s45, None]
            .iter()
            .map(|&slope| RoofEdge {
                slope,
                overhang: 0.0,
            })
            .collect(),
        layers: vec![layer_of(LayerRole::LoadBearing, timber, 300.0)],
        eaves: LevelRef {
            story: f.story1,
            kind: LevelKind::Finished,
            offset: 0.0,
        },
        plane: RoofPlane::Top,
    };
    let roof_id = roof.id;
    f.h.run(Command::AddRoof(roof))?;
    // Walls 6300 high, drawn clockwise so that they lie under the roof: a
    // gable, an eaves wall, and one out of the roof.
    let mut walls = Vec::new();
    for (start, end) in [
        ((0.0, 0.0), (0.0, 6_000.0)),
        ((10_000.0, 0.0), (0.0, 0.0)),
        ((12_000.0, 0.0), (14_000.0, 0.0)),
    ] {
        let id = f.wall(start, end, &[(concrete, 200.0)])?;
        let mut w = f.h.project().wall(id)?.clone();
        w.top.offset = 3_500.0;
        f.h.run(Command::ReplaceWall(w))?;
        walls.push(id);
    }
    let tops =
        |p: &Project, id: Id| -> std::result::Result<(f64, f64), Box<dyn std::error::Error>> {
            let t = model_blocks(p, Phase::Proposed)?
                .into_iter()
                .filter(|b| b.element == id)
                .flat_map(|b| b.top)
                .fold((f64::MAX, f64::MIN), |(lo, hi), t| (lo.min(t), hi.max(t)));
            Ok(t)
        };
    let near = |a: f64, b: f64| (a - b).abs() < 1.0;
    for &w in &walls {
        assert_eq!(tops(f.h.project(), w)?, (6_300.0, 6_300.0), "uncut");
    }
    // The underside: 2800 at the eaves, less the build-up square to a 45
    // degree slope, rising 1 for 1 to the ridge at y = 3000.
    let low = 2_800.0 - 300.0 / 45f64.to_radians().cos();
    // The roof offers to cut the two walls rising through it.
    assert_eq!(
        cuts_through(f.h.project(), roof_id)?,
        vec![walls[0], walls[1]]
    );
    let cuts: Vec<Command> = walls
        .iter()
        .map(|&element| Command::SetCut {
            element,
            by: roof_id,
            on: true,
        })
        .collect();
    f.h.run(Command::Group(cuts))?;
    assert!(cuts_through(f.h.project(), roof_id)?.is_empty(), "all cut");
    let (lo, hi) = tops(f.h.project(), walls[0])?;
    assert!(near(lo, low) && near(hi, low + 3_000.0), "gable: {lo} {hi}");
    let (lo, hi) = tops(f.h.project(), walls[1])?;
    assert!(near(lo, low) && near(hi, low + 200.0), "eaves: {lo} {hi}");
    assert_eq!(tops(f.h.project(), walls[2])?, (6_300.0, 6_300.0), "out");
    // The wall keeps its own height.
    assert!(close(f.h.project().wall_extent(walls[0])?.1, 6_300.0));
    // The cut follows the roof: raised by 1000, the gable point too.
    let mut raised = f.h.project().roof(roof_id)?.clone();
    raised.eaves.offset = 1_000.0;
    f.h.run(Command::ReplaceRoof(raised))?;
    let (_, hi) = tops(f.h.project(), walls[0])?;
    assert!(near(hi, (low + 4_000.0).min(6_300.0)), "{hi}");
    f.h.undo()?;
    // A roof under the wall's bottom leaves nothing of it there.
    let mut sunk = f.h.project().roof(roof_id)?.clone();
    sunk.eaves.offset = -5_000.0;
    f.h.run(Command::ReplaceRoof(sunk))?;
    let left: Vec<_> = model_blocks(f.h.project(), Phase::Proposed)?
        .into_iter()
        .filter(|b| b.element == walls[1])
        .collect();
    assert!(left.is_empty(), "the eaves wall is all above the roof");
    f.h.undo()?;
    // Deleting the roof releases the walls; undo brings the cuts back.
    f.h.run(Command::RemoveRoof(roof_id))?;
    assert!(f.h.project().cuts().is_empty());
    assert_eq!(tops(f.h.project(), walls[0])?, (6_300.0, 6_300.0));
    f.h.undo()?;
    assert_eq!(f.h.project().cuts().len(), 3);
    // Deleting a cut wall and undoing brings its cut back.
    f.h.run(Command::RemoveWall(walls[2]))?;
    assert_eq!(f.h.project().cuts().len(), 2);
    f.h.undo()?;
    assert_eq!(f.h.project().cuts().len(), 3);
    // One undo step stops all three cuts.
    f.h.undo()?;
    assert!(f.h.project().cuts().is_empty());
    // A column is cut too; a slab is not something to cut.
    let column = f.h.new_id();
    f.h.run(Command::AddColumn(Column {
        id: column,
        story: f.story0,
        at: Point::new(5_000.0, 3_000.0),
        anchor: Anchor::default(),
        angle: 0.0,
        width: 300.0,
        depth: 300.0,
        round: false,
        material: concrete,
        bottom: f.h.project().wall(walls[0])?.bottom,
        top: f.h.project().wall(walls[0])?.top,
    }))?;
    f.h.run(Command::SetCut {
        element: column,
        by: roof_id,
        on: true,
    })?;
    let (_, hi) = tops(f.h.project(), column)?;
    assert!(near(hi, low + 3_000.0), "column under the ridge: {hi}");
    assert!(
        f.h.run(Command::SetCut {
            element: roof_id,
            by: roof_id,
            on: true
        })
        .is_err()
    );
    Ok(())
}

#[test]
fn roofs_walls_cut_under_edge_cases() -> Result {
    let mut f = Fixture::new()?;
    let concrete = f.material("concrete", 5)?;
    let timber = f.material("timber", 4)?;
    let roof_at = |f: &mut Fixture, offset: f64, slope: Option<f64>| {
        let id = f.h.new_id();
        let roof = Roof {
            id,
            story: f.story1,
            outline: rect_outline(10_000.0, 6_000.0),
            edges: (0..4)
                .map(|_| RoofEdge {
                    slope,
                    overhang: 0.0,
                })
                .collect(),
            layers: vec![layer_of(LayerRole::LoadBearing, timber, 300.0)],
            eaves: LevelRef {
                story: f.story1,
                kind: LevelKind::Finished,
                offset,
            },
            plane: RoofPlane::Top,
        };
        f.h.run(Command::AddRoof(roof)).map(|_| id)
    };
    // Flat roofs: undersides at 2500 and 1500.
    let high = roof_at(&mut f, 0.0, None)?;
    let low = roof_at(&mut f, -1_000.0, None)?;
    let id = f.wall((1_000.0, 3_000.0), (9_000.0, 3_000.0), &[(concrete, 200.0)])?;
    let mut w = f.h.project().wall(id)?.clone();
    w.top.offset = 3_500.0;
    f.h.run(Command::ReplaceWall(w))?;
    let opening_id = f.h.new_id();
    f.h.run(Command::AddOpening(Opening {
        id: opening_id,
        wall: id,
        kind: OpeningKind::Interior,
        position: 1_000.0,
        sill: 0.0,
        height: 2_150.0,
        frame: 68.0,
        parts: vec![Part {
            width: 900.0,
            operation: Operation::SideHung(Hand::Right),
        }],
        nominal: NominalSize::ClearOpening,
    }))?;
    let top_of = |p: &Project| -> std::result::Result<f64, Box<dyn std::error::Error>> {
        let mut hi = f64::MIN;
        for b in model_blocks(p, Phase::Proposed)?
            .into_iter()
            .filter(|b| b.element == id)
        {
            for k in 0..b.top.len() {
                assert!(b.top[k].is_finite() && b.bottom[k].is_finite());
                assert!(b.top[k] >= b.bottom[k] - 1e-6, "top below bottom");
                hi = hi.max(b.top[k]);
            }
        }
        Ok(hi)
    };
    assert!(close(top_of(f.h.project())?, 6_300.0));
    // The strips over the head of the door are cut with the rest.
    f.h.run(Command::SetCut {
        element: id,
        by: high,
        on: true,
    })?;
    assert!(close(top_of(f.h.project())?, 2_500.0));
    // Two roofs: the lower one wins, in either order.
    f.h.run(Command::SetCut {
        element: id,
        by: low,
        on: true,
    })?;
    assert!(close(top_of(f.h.project())?, 1_500.0));
    // Undo and redo of the cut.
    f.h.undo()?;
    assert!(close(top_of(f.h.project())?, 2_500.0));
    f.h.redo()?;
    assert!(close(top_of(f.h.project())?, 1_500.0));
    // Removing the lower roof releases it; undo brings the cut back.
    f.h.run(Command::RemoveRoof(low))?;
    assert!(close(top_of(f.h.project())?, 2_500.0));
    f.h.undo()?;
    assert!(close(top_of(f.h.project())?, 1_500.0));
    f.h.redo()?;
    assert!(close(top_of(f.h.project())?, 2_500.0));
    f.h.undo()?;
    // Cutting twice under the same roof is one cut.
    f.h.run(Command::SetCut {
        element: id,
        by: low,
        on: true,
    })?;
    assert_eq!(f.h.project().cuts().len(), 2);
    // Refused: a slab, an unknown element, an unknown roof.
    let slab = Slab {
        id: f.h.new_id(),
        story: f.story0,
        outline: rect_outline(2_000.0, 2_000.0),
        layers: vec![layer_of(LayerRole::LoadBearing, concrete, 200.0)],
        top: LevelRef {
            story: f.story0,
            kind: LevelKind::Structural,
            offset: 0.0,
        },
    };
    let slab_id = slab.id;
    f.h.run(Command::AddSlab(slab))?;
    let before = f.h.project().cuts().to_vec();
    for (element, roof) in [(slab_id, high), (Id(9_999), high), (id, Id(9_999))] {
        assert!(
            f.h.run(Command::SetCut {
                element,
                by: roof,
                on: true
            })
            .is_err()
        );
    }
    assert_eq!(f.h.project().cuts(), &before[..]);
    // A curved wall under a sloped roof stays sane.
    let sloped = roof_at(&mut f, 0.0, Some(45f64.to_radians()))?;
    let arc = f.wall((3_000.0, 0.0), (-3_000.0, 0.0), &[(concrete, 200.0)])?;
    let mut a = f.h.project().wall(arc)?.clone();
    a.sweep = std::f64::consts::PI;
    a.reference = WallReference::Axis;
    a.top.offset = 3_500.0;
    f.h.run(Command::ReplaceWall(a))?;
    f.h.run(Command::SetCut {
        element: arc,
        by: sloped,
        on: true,
    })?;
    for b in model_blocks(f.h.project(), Phase::Proposed)?
        .into_iter()
        .filter(|b| b.element == arc)
    {
        for k in 0..b.top.len() {
            assert!(b.top[k].is_finite() && b.top[k] >= b.bottom[k] - 1e-6);
            assert!(b.top[k] <= 6_300.0 + 1e-6);
        }
    }
    Ok(())
}

#[test]
fn slabs_floor_on_bearing_walls() -> Result {
    // Layers from the outside in, the load-bearing one, and where the slab
    // edge sits from the outside face.
    for (outside, inset) in [(true, 160.0), (false, 20.0)] {
        let mut f = Fixture::new()?;
        let render = f.material("render", 1)?;
        let insulation = f.material("insulation", 3)?;
        let bearing = f.material("bearing", 9)?;
        let plaster = f.material("plaster", 1)?;
        let layers = if outside {
            vec![
                layer_of(LayerRole::Finish, render, 20.0),
                layer_of(LayerRole::Insulation, insulation, 140.0),
                layer_of(LayerRole::LoadBearing, bearing, 200.0),
                layer_of(LayerRole::Finish, plaster, 15.0),
            ]
        } else {
            vec![
                layer_of(LayerRole::Finish, render, 20.0),
                layer_of(LayerRole::LoadBearing, bearing, 200.0),
                layer_of(LayerRole::Insulation, insulation, 100.0),
                layer_of(LayerRole::Finish, plaster, 15.0),
            ]
        };
        // Clockwise, the first layer outside.
        let mut walls = Vec::new();
        for (a, b) in [
            ((0.0, 0.0), (0.0, 6_000.0)),
            ((0.0, 6_000.0), (10_000.0, 6_000.0)),
            ((10_000.0, 6_000.0), (10_000.0, 0.0)),
            ((10_000.0, 0.0), (0.0, 0.0)),
        ] {
            walls.push(f.wall_with(a, b, layers.clone())?);
        }
        let t: f64 = layers.iter().map(|l| l.thickness).sum();
        let partition = f.wall((4_000.0, t), (4_000.0, 6_000.0 - t), &[(bearing, 100.0)])?;

        let at = Point::new(2_000.0, 3_000.0);
        let outline = floor_contour_at(f.h.project(), f.story0, at).ok_or("no floor contour")?;
        let (x0, y0, x1, y1) = outline.iter().fold(
            (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
            |(a, b, c, d), q| (a.min(q.x), b.min(q.y), c.max(q.x), d.max(q.y)),
        );
        let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
        assert_eq!(outline.len(), 4, "{outline:?}");
        assert!(near(x0, inset) && near(y0, inset), "{outline:?}");
        assert!(near(x1, 10_000.0 - inset) && near(y1, 6_000.0 - inset));
        let room = contour_at(f.h.project(), f.story0, at).ok_or("no room")?;
        assert!(
            room.iter()
                .all(|q| q.x >= t - 1e-6 && q.x <= 4_000.0 + 1e-6)
        );

        let slab = Slab {
            id: f.h.new_id(),
            story: f.story1,
            outline,
            layers: vec![layer_of(LayerRole::LoadBearing, bearing, 200.0)],
            top: LevelRef {
                story: f.story1,
                kind: LevelKind::Structural,
                offset: 0.0,
            },
        };
        let id = slab.id;
        f.h.run(Command::AddSlab(slab))?;
        let mut through = walls.clone();
        through.push(partition);
        assert_eq!(cuts_through(f.h.project(), id)?, through);
        let joins = through
            .iter()
            .map(|&element| Command::SetCut {
                element,
                by: id,
                on: true,
            })
            .collect();
        f.h.run(Command::Group(joins))?;
        assert!(cuts_through(f.h.project(), id)?.is_empty(), "all joined");

        // The highest top of each layer role of the first wall.
        let tops = |p: &Project| -> std::result::Result<Vec<(LayerRole, f64)>, Box<dyn std::error::Error>> {
            let mut t: Vec<(LayerRole, f64)> = Vec::new();
            for b in model_blocks(p, Phase::Proposed)?
                .into_iter()
                .filter(|b| b.element == walls[0])
            {
                let role = b.role.ok_or("a wall layer has a role")?;
                let top = b.top.iter().copied().fold(f64::MIN, f64::max);
                match t.iter_mut().find(|(r, _)| *r == role) {
                    Some((_, h)) => *h = h.max(top),
                    None => t.push((role, top)),
                }
            }
            Ok(t)
        };
        let top_of = |t: &[(LayerRole, f64)], role| t.iter().find(|(r, _)| *r == role).map(|x| x.1);
        let joined = tops(f.h.project())?;
        // The bearing layer and what is inside stop under the slab; the
        // render, and the insulation outside, go on to the top.
        assert_eq!(top_of(&joined, LayerRole::LoadBearing), Some(2_600.0));
        let insulation_top = if outside { 2_800.0 } else { 2_600.0 };
        assert_eq!(top_of(&joined, LayerRole::Insulation), Some(insulation_top));
        assert_eq!(top_of(&joined, LayerRole::Finish), Some(2_800.0), "render");

        // Deleting the slab releases the walls; undo brings the joins back.
        f.h.run(Command::RemoveSlab(id))?;
        assert!(f.h.project().cuts().is_empty());
        assert_eq!(
            top_of(&tops(f.h.project())?, LayerRole::LoadBearing),
            Some(2_800.0)
        );
        f.h.undo()?;
        assert_eq!(f.h.project().cuts().len(), through.len());
        assert_eq!(tops(f.h.project())?, joined);
    }
    Ok(())
}

fn floor_slab(f: &mut Fixture, outline: Vec<Point>, material: Id) -> Slab {
    Slab {
        id: f.h.new_id(),
        story: f.story1,
        outline,
        layers: vec![layer_of(LayerRole::LoadBearing, material, 200.0)],
        top: LevelRef {
            story: f.story1,
            kind: LevelKind::Structural,
            offset: 0.0,
        },
    }
}

/// Bottom, highest top, and x range of a piece.
type Piece = (f64, f64, f64, f64);

/// The pieces of an element.
fn pieces_of(p: &Project, id: Id) -> std::result::Result<Vec<Piece>, Box<dyn std::error::Error>> {
    Ok(model_blocks(p, Phase::Proposed)?
        .into_iter()
        .filter(|b| b.element == id)
        .map(|b| {
            let xs = b.outline.iter().map(|q| q.x);
            (
                b.bottom.iter().copied().fold(f64::MAX, f64::min),
                b.top.iter().copied().fold(f64::MIN, f64::max),
                xs.clone().fold(f64::MAX, f64::min),
                xs.fold(f64::MIN, f64::max),
            )
        })
        .collect())
}

fn ring(f: &mut Fixture, layers: Vec<Layer>) -> Result {
    for (a, b) in [
        ((0.0, 0.0), (0.0, 6_000.0)),
        ((0.0, 6_000.0), (10_000.0, 6_000.0)),
        ((10_000.0, 6_000.0), (10_000.0, 0.0)),
        ((10_000.0, 0.0), (0.0, 0.0)),
    ] {
        f.wall_with(a, b, layers.clone())?;
    }
    Ok(())
}

fn floor_inset(f: &Fixture) -> std::result::Result<f64, Box<dyn std::error::Error>> {
    let c = floor_contour_at(f.h.project(), f.story0, Point::new(2_000.0, 3_000.0))
        .ok_or("no floor contour")?;
    Ok(c.iter().map(|q| q.x).fold(f64::MAX, f64::min))
}

#[test]
fn slabs_floor_contour_edge_cases() -> Result {
    let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
    // No bearing layer: the inner face.
    let mut f = Fixture::new()?;
    let m = f.material("m", 5)?;
    ring(
        &mut f,
        vec![
            layer_of(LayerRole::Finish, m, 20.0),
            layer_of(LayerRole::Insulation, m, 100.0),
            layer_of(LayerRole::Finish, m, 15.0),
        ],
    )?;
    assert!(near(floor_inset(&f)?, 135.0), "{}", floor_inset(&f)?);
    // A framed layer bears like a load-bearing one.
    let mut g = Fixture::new()?;
    let m = g.material("m", 5)?;
    let mut framed = layer_of(LayerRole::Insulation, m, 140.0);
    framed.framing = Some(Framing {
        material: m,
        width: 45.0,
        spacing: 600.0,
    });
    ring(
        &mut g,
        vec![
            layer_of(LayerRole::Finish, m, 20.0),
            framed,
            layer_of(LayerRole::Finish, m, 15.0),
        ],
    )?;
    assert!(near(floor_inset(&g)?, 20.0), "{}", floor_inset(&g)?);
    // A dangling wall sticking out of the group leaves the outline alone.
    let at = Point::new(2_000.0, 3_000.0);
    let before = floor_contour_at(g.h.project(), g.story0, at).ok_or("none")?;
    let bearing = vec![layer_of(LayerRole::LoadBearing, m, 200.0)];
    g.wall_with((10_000.0, 3_000.0), (14_000.0, 3_000.0), bearing)?;
    let after = floor_contour_at(g.h.project(), g.story0, at).ok_or("none after")?;
    assert!(after.iter().all(|q| q.x < 10_000.0), "{after:?}");
    assert!(before.len() <= after.len());
    Ok(())
}

#[test]
fn slabs_floor_on_bearing_walls_edge_cases() -> Result {
    let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
    let mut f = Fixture::new()?;
    let m = f.material("m", 5)?;
    // A wall across the slab's edge, one under the slab, one whose top is
    // the slab's bottom, a column, a beam.
    let across = f.wall((-3_000.0, 3_000.0), (5_000.0, 3_000.0), &[(m, 200.0)])?;
    let inside = f.wall((2_000.0, 1_000.0), (2_000.0, 2_000.0), &[(m, 200.0)])?;
    let low = f.wall((2_500.0, 1_000.0), (2_500.0, 2_000.0), &[(m, 200.0)])?;
    let mut w = f.h.project().wall(low)?.clone();
    w.top.offset = -200.0;
    f.h.run(Command::ReplaceWall(w))?;
    let wall = f.h.project().wall(across)?.clone();
    let column = f.h.new_id();
    f.h.run(Command::AddColumn(Column {
        id: column,
        story: f.story0,
        at: Point::new(5_000.0, 4_000.0),
        anchor: Anchor::default(),
        angle: 0.0,
        width: 300.0,
        depth: 300.0,
        round: false,
        material: m,
        bottom: wall.bottom,
        top: wall.top,
    }))?;
    let beam = f.h.new_id();
    f.h.run(Command::AddBeam(Beam {
        id: beam,
        story: f.story1,
        start: Point::new(1_000.0, 5_000.0),
        end: Point::new(4_000.0, 5_000.0),
        anchor: Anchor::default(),
        width: 200.0,
        height: 400.0,
        material: m,
        level: LevelRef {
            story: f.story1,
            kind: LevelKind::Structural,
            offset: 0.0,
        },
    }))?;
    let slab = floor_slab(&mut f, rect_outline(10_000.0, 6_000.0), m);
    let id = slab.id;
    f.h.run(Command::AddSlab(slab.clone()))?;
    let through = cuts_through(f.h.project(), id)?;
    assert!(through.contains(&across), "{through:?}");
    assert!(through.contains(&inside) && through.contains(&column));
    assert!(!through.contains(&low), "top = slab bottom: not joined");
    assert!(through.contains(&beam), "{through:?}");
    // Joined in one step: undo and redo of the group.
    let joins: Vec<Command> = through
        .iter()
        .map(|&element| Command::SetCut {
            element,
            by: id,
            on: true,
        })
        .collect();
    f.h.run(Command::Group(joins))?;
    let n = through.len();
    assert_eq!(f.h.project().cuts().len(), n);
    f.h.undo()?;
    assert!(f.h.project().cuts().is_empty());
    f.h.redo()?;
    assert_eq!(f.h.project().cuts().len(), n);
    // Across the edge: the part outside goes whole, the part inside stops
    // under the slab.
    let parts = pieces_of(f.h.project(), across)?;
    assert!(
        parts.iter().any(|&(lo, hi, x0, x1)| near(lo, 0.0)
            && near(hi, 2_800.0)
            && x1 <= 1e-6
            && x0 < -2_999.0),
        "whole outside: {parts:?}"
    );
    assert!(
        parts
            .iter()
            .all(|&(_, hi, x0, _)| x0 < -1e-6 || hi <= 2_600.0 + 1e-6),
        "{parts:?}"
    );
    let low_top = pieces_of(f.h.project(), low)?
        .iter()
        .map(|p| p.1)
        .fold(0.0, f64::max);
    assert_eq!(low_top, 2_600.0, "untouched");
    // Moving the slab away: the joined elements are reported, nothing is
    // left to interrupt; undo brings it back.
    let mut moved = slab.clone();
    moved.outline = moved
        .outline
        .iter()
        .map(|q| Point::new(q.x + 50_000.0, q.y))
        .collect();
    let changed = f.h.run(Command::ReplaceSlab(moved))?;
    assert!(changed.contains(&across) && changed.contains(&id));
    let gone = pieces_of(f.h.project(), across)?;
    assert!(gone.iter().all(|p| near(p.1, 2_800.0)), "{gone:?}");
    f.h.undo()?;
    let back = pieces_of(f.h.project(), across)?;
    assert!(back.iter().any(|p| near(p.1, 2_600.0)), "{back:?}");
    // RemoveSlab reports what it frees; undo brings the joins back.
    let changed = f.h.run(Command::RemoveSlab(id))?;
    assert!(changed.contains(&across) && changed.contains(&column));
    assert!(f.h.project().cuts().is_empty());
    f.h.undo()?;
    assert_eq!(f.h.project().cuts().len(), n);
    // A roof and a slab on the same wall: both apply.
    let roof = Roof {
        id: f.h.new_id(),
        story: f.story1,
        outline: rect_outline(10_000.0, 6_000.0),
        edges: vec![
            RoofEdge {
                slope: None,
                overhang: 0.0
            };
            4
        ],
        layers: vec![layer_of(LayerRole::LoadBearing, m, 300.0)],
        eaves: LevelRef {
            story: f.story1,
            kind: LevelKind::Finished,
            offset: -1_000.0,
        },
        plane: RoofPlane::Top,
    };
    let rid = roof.id;
    f.h.run(Command::AddRoof(roof))?;
    f.h.run(Command::SetCut {
        element: across,
        by: rid,
        on: true,
    })?;
    let tops: Vec<f64> = pieces_of(f.h.project(), across)?
        .iter()
        .map(|p| p.1)
        .collect();
    assert!(tops.iter().any(|&t| t < 1_800.0 + 1e-6), "roof: {tops:?}");
    assert!(tops.iter().any(|&t| near(t, 2_800.0)), "outside: {tops:?}");
    // Only a roof or a slab cuts.
    assert!(
        f.h.run(Command::SetCut {
            element: across,
            by: across,
            on: true
        })
        .is_err()
    );

    // A concave (L-shaped) slab: a wall in the notch is left alone, one in
    // an arm is interrupted.
    let mut f = Fixture::new()?;
    let m = f.material("m", 5)?;
    let notch = f.wall((8_000.0, 5_000.0), (9_000.0, 5_000.0), &[(m, 200.0)])?;
    let arm = f.wall((2_000.0, 5_000.0), (3_000.0, 5_000.0), &[(m, 200.0)])?;
    let l: Vec<Point> = [
        (0.0, 0.0),
        (10_000.0, 0.0),
        (10_000.0, 3_000.0),
        (5_000.0, 3_000.0),
        (5_000.0, 6_000.0),
        (0.0, 6_000.0),
    ]
    .iter()
    .map(|&(x, y)| Point::new(x, y))
    .collect();
    let slab = floor_slab(&mut f, l, m);
    let id = slab.id;
    f.h.run(Command::AddSlab(slab))?;
    assert_eq!(cuts_through(f.h.project(), id)?, vec![arm]);
    f.h.run(Command::SetCut {
        element: arm,
        by: id,
        on: true,
    })?;
    let arm_tops = pieces_of(f.h.project(), arm)?;
    assert!(arm_tops.iter().all(|p| p.1 <= 2_600.0 + 1e-6));
    let notch_tops = pieces_of(f.h.project(), notch)?;
    assert!(notch_tops.iter().all(|p| near(p.1, 2_800.0)));
    Ok(())
}
