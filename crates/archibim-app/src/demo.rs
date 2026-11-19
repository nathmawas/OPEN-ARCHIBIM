//! A demonstration house, built through commands: a rectangle of external
//! walls, two partitions crossing each other, a timber frame annex, a slab,
//! a column, a beam, a window and a door.

use archibim_commands::{Command, History};
use archibim_model::{
    Beam, Column, Framing, Hand, Id, Layer, LayerRole, LevelKind, LevelRef, Material, NominalSize,
    Opening, OpeningKind, Operation, Part, Performance, Point, Project, RenovationStatus, Slab,
    Story, Wall,
};
use archibim_ui::Templates;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// The demonstration house, the story to show, and the elements the tools
/// draw.
pub fn house() -> Result<(History, Id, Templates)> {
    let mut h = History::new(Project::new());
    let ground = h.new_id();
    let first = h.new_id();
    for (id, level) in [(ground, 0.0), (first, 2800.0)] {
        h.run(Command::AddStory(Story {
            id,
            name: String::new(),
            finished_level: level,
            floor_build_up: 0.0,
        }))?;
    }
    let mut material = |priority| -> Result<Id> {
        let id = h.new_id();
        h.run(Command::AddMaterial(Material {
            id,
            name: String::new(),
            priority,
        }))?;
        Ok(id)
    };
    let render = material(1)?;
    let insulation = material(3)?;
    let concrete = material(9)?;
    let plaster = material(1)?;
    let brick = material(5)?;
    let fibre = material(2)?;
    let wool = material(2)?;
    let timber = material(6)?;
    let osb = material(4)?;
    let board = material(1)?;

    let layer = |role, material, thickness| Layer {
        role,
        material,
        thickness,
        status: RenovationStatus::New,
        framing: None,
    };
    // Layers are listed from the reference line (left face) to the right.
    // The rectangle runs clockwise, so the first layer is outside.
    let external = vec![
        layer(LayerRole::Finish, render, 20.0),
        layer(LayerRole::Insulation, insulation, 140.0),
        layer(LayerRole::LoadBearing, concrete, 200.0),
        layer(LayerRole::Finish, plaster, 15.0),
    ];
    let partition = vec![
        layer(LayerRole::Finish, plaster, 15.0),
        layer(LayerRole::LoadBearing, brick, 100.0),
        layer(LayerRole::Finish, plaster, 15.0),
    ];
    let timber_frame = vec![
        layer(LayerRole::Finish, render, 15.0),
        layer(LayerRole::Insulation, fibre, 60.0),
        Layer {
            framing: Some(Framing {
                material: timber,
                width: 45.0,
                spacing: 600.0,
            }),
            ..layer(LayerRole::Insulation, wool, 145.0)
        },
        layer(LayerRole::Lining, osb, 12.0),
        layer(LayerRole::Finish, board, 13.0),
    ];

    let t = 375.0;
    let walls = [
        ((0.0, 0.0), (10000.0, 0.0), &external),
        ((10000.0, 0.0), (10000.0, -8000.0), &external),
        ((10000.0, -8000.0), (0.0, -8000.0), &external),
        ((0.0, -8000.0), (0.0, 0.0), &external),
        ((4000.0, -t), (4000.0, -8000.0 + t), &partition),
        ((t, -4500.0), (10000.0 - t, -4500.0), &partition),
        ((10000.0, -2000.0), (14000.0, -2000.0), &timber_frame),
        ((14000.0, -2000.0), (14000.0, -6000.0), &timber_frame),
        ((14000.0, -6000.0), (10000.0, -6000.0), &timber_frame),
    ];
    let level = |story, offset| LevelRef {
        story,
        kind: LevelKind::Structural,
        offset,
    };
    let mut ids = Vec::new();
    for (start, end, layers) in walls {
        let id = h.new_id();
        ids.push(id);
        h.run(Command::AddWall(Wall {
            id,
            story: ground,
            start: Point::new(start.0, start.1),
            end: Point::new(end.0, end.1),
            layers: layers.clone(),
            bottom: level(ground, 0.0),
            top: level(first, 0.0),
        }))?;
    }

    let templates = Templates {
        wall: h.project().walls.first().cloned().ok_or("no wall")?,
        slab: Slab {
            id: Id(0),
            story: ground,
            outline: Vec::new(),
            layers: vec![layer(LayerRole::LoadBearing, concrete, 200.0)],
            top: level(ground, 0.0),
        },
        column: Column {
            id: Id(0),
            story: ground,
            at: Point::new(0.0, 0.0),
            width: 300.0,
            depth: 300.0,
            material: concrete,
            bottom: level(ground, 0.0),
            top: level(first, 0.0),
        },
        beam: Beam {
            id: Id(0),
            story: ground,
            start: Point::new(0.0, 0.0),
            end: Point::new(1.0, 0.0),
            width: 200.0,
            height: 400.0,
            material: concrete,
            top: level(first, 0.0),
        },
        window: Opening {
            id: Id(0),
            wall: Id(0),
            kind: OpeningKind::Exterior(Performance::default()),
            position: 0.0,
            sill: 900.0,
            height: 1350.0,
            frame: 60.0,
            parts: vec![Part {
                width: 1080.0,
                operation: Operation::SideHung(Hand::Left),
            }],
            nominal: NominalSize::ClearOpening,
        },
        door: Opening {
            id: Id(0),
            wall: Id(0),
            kind: OpeningKind::Interior,
            position: 0.0,
            sill: 0.0,
            height: 2040.0,
            frame: 45.0,
            parts: vec![Part {
                width: 830.0,
                operation: Operation::SideHung(Hand::Right),
            }],
            nominal: NominalSize::ClearOpening,
        },
    };

    let slab = Slab {
        id: h.new_id(),
        outline: vec![
            Point::new(0.0, 0.0),
            Point::new(10000.0, 0.0),
            Point::new(10000.0, -8000.0),
            Point::new(0.0, -8000.0),
        ],
        ..templates.slab.clone()
    };
    h.run(Command::AddSlab(slab))?;
    let column = Column {
        id: h.new_id(),
        at: Point::new(12000.0, -4000.0),
        ..templates.column.clone()
    };
    h.run(Command::AddColumn(column))?;
    let beam = Beam {
        id: h.new_id(),
        start: Point::new(10000.0, -4000.0),
        end: Point::new(14000.0, -4000.0),
        ..templates.beam.clone()
    };
    h.run(Command::AddBeam(beam))?;
    let window = Opening {
        id: h.new_id(),
        wall: ids[0],
        position: 2000.0,
        ..templates.window.clone()
    };
    h.run(Command::AddOpening(window))?;
    let door = Opening {
        id: h.new_id(),
        wall: ids[4],
        position: 1200.0,
        ..templates.door.clone()
    };
    h.run(Command::AddOpening(door))?;
    Ok((h, ground, templates))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_demo_house_has_a_plan() -> super::Result<()> {
        let (h, story, _) = super::house()?;
        let plan =
            archibim_geometry::plan_layers(h.project(), story, archibim_geometry::Phase::Proposed)?;
        assert!(plan.iter().any(|l| !l.members.is_empty()), "studs shown");
        Ok(())
    }
}
