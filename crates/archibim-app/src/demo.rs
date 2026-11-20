//! A demonstration house, built through commands: a rectangle of external
//! walls, two partitions crossing each other, a timber frame annex, a slab,
//! a column, a beam, a window and a door.

use archibim_commands::{Command, History};
use archibim_model::{
    Align, Anchor, Beam, Column, Framing, Hand, Id, Layer, LayerRole, LevelKind, LevelRef,
    Material, NominalSize, Opening, OpeningKind, Operation, Part, Performance, Point, Project,
    RenovationStatus, Slab, Story, Wall, WallReference,
};
use archibim_ui::{Preset, Templates};

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
    // Names are language pack keys: the starting library shows in the
    // user's language; names the user types show as typed.
    let mut material = |name: &str, priority| -> Result<Id> {
        let id = h.new_id();
        h.run(Command::AddMaterial(Material {
            id,
            name: name.to_owned(),
            priority,
        }))?;
        Ok(id)
    };
    let render = material("mat.render", 1)?;
    let insulation = material("mat.insulation", 3)?;
    let concrete = material("mat.concrete", 9)?;
    let plaster = material("mat.plaster", 1)?;
    let brick = material("mat.brick", 5)?;
    let fibre = material("mat.wood_fibre", 2)?;
    let wool = material("mat.mineral_wool", 2)?;
    let timber = material("mat.timber", 6)?;
    let osb = material("mat.osb", 4)?;
    let board = material("mat.plasterboard", 1)?;
    let block = material("mat.concrete_block", 7)?;

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
            reference: WallReference::LeftFace,
            offset: 0.0,
            sweep: 0.0,
            layers: layers.clone(),
            bottom: level(ground, 0.0),
            top: level(first, 0.0),
        }))?;
    }

    let simple = |material, thickness| vec![layer(LayerRole::LoadBearing, material, thickness)];
    let preset = |name, layers: &Vec<Layer>| Preset {
        name,
        layers: layers.clone(),
    };
    let wall_presets = vec![
        preset("preset.wall.concrete", &simple(concrete, 200.0)),
        preset("preset.wall.block", &simple(block, 200.0)),
        preset("preset.wall.brick", &simple(brick, 200.0)),
        preset("preset.wall.partition", &simple(board, 72.0)),
        preset("preset.wall.external", &external),
        preset("preset.wall.masonry_partition", &partition),
        preset("preset.wall.timber_frame", &timber_frame),
    ];
    let templates = Templates {
        wall: h.project().walls.first().cloned().ok_or("no wall")?,
        wall_presets,
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
            anchor: Anchor::default(),
            angle: 0.0,
            width: 300.0,
            depth: 300.0,
            round: false,
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
            anchor: Anchor {
                width: Align::Middle,
                depth: Align::First,
            },
            level: level(first, 0.0),
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
