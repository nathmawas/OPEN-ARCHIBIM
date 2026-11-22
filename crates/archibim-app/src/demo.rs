//! A demonstration house, built through commands: a rectangle of external
//! walls, two partitions crossing each other, a timber frame annex, a slab,
//! a column, a beam, a window and a door, a gabled roof, a shed roof over
//! the annex, a turning stair and the terrain around.

use archibim_commands::{Command, History};
use archibim_model::{
    Align, Anchor, Beam, Column, Framing, Hand, Id, Layer, LayerRole, LevelKind, LevelRef,
    Material, Mesh, MeshPoint, NominalSize, Opening, OpeningKind, Operation, Part, Performance,
    Point, Project, RenovationStatus, Roof, RoofEdge, RoofPlane, Slab, Stair, Story, Wall,
    WallReference,
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
    // ponytail: translated in the interface language; follow the project's
    // language and country once projects carry them, and write the
    // translated name to IFC.
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
    let tiles = material("mat.tiles", 1)?;
    let soil = material("mat.soil", 0)?;

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
    let finished = |story| LevelRef {
        story,
        kind: LevelKind::Finished,
        offset: 0.0,
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
    // A low curved garden wall south of the house, bowing away from it.
    let id = h.new_id();
    h.run(Command::AddWall(Wall {
        id,
        story: ground,
        start: Point::new(0.0, -11000.0),
        end: Point::new(10000.0, -11000.0),
        reference: WallReference::LeftFace,
        offset: 0.0,
        sweep: 60f64.to_radians(),
        layers: simple(concrete, 200.0),
        bottom: level(ground, 0.0),
        top: level(ground, 1000.0),
    }))?;
    let preset = |name, layers: &Vec<Layer>| Preset {
        name,
        layers: layers.clone(),
    };
    // ponytail: presets written here for the demo; load them as data, by
    // country and reviewed by architects (TOOLS.md, rule 17).
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
        roof: Roof {
            id: Id(0),
            story: ground,
            outline: Vec::new(),
            edges: vec![RoofEdge {
                slope: Some(35f64.to_radians()),
                overhang: 400.0,
            }],
            layers: vec![
                layer(LayerRole::Covering, tiles, 40.0),
                layer(LayerRole::Insulation, wool, 200.0),
                layer(LayerRole::LoadBearing, timber, 60.0),
            ],
            eaves: level(first, 0.0),
            plane: RoofPlane::Bottom,
        },
        mesh: Mesh {
            id: Id(0),
            story: ground,
            outline: Vec::new(),
            inner: Vec::new(),
            level: finished(ground),
            bottom: -1000.0,
            material: soil,
        },
        stair: Stair {
            id: Id(0),
            story: ground,
            path: Vec::new(),
            width: 900.0,
            bottom: finished(ground),
            top: finished(first),
            risers: None,
            material: concrete,
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

    let points = |xy: &[(f64, f64)]| xy.iter().map(|&(x, y)| Point::new(x, y)).collect();
    let main: Vec<Point> = points(&[
        (0.0, 0.0),
        (10000.0, 0.0),
        (10000.0, -8000.0),
        (0.0, -8000.0),
    ]);
    // Sloped on the long sides, gables on the short ones.
    let sloped = templates.roof.edges[0];
    let gable = RoofEdge {
        slope: None,
        overhang: 150.0,
    };
    let id = h.new_id();
    h.run(Command::AddRoof(Roof {
        id,
        edges: vec![sloped, gable, sloped, gable],
        outline: main,
        ..templates.roof.clone()
    }))?;
    // The annex: one plane, low on its far side.
    let id = h.new_id();
    h.run(Command::AddRoof(Roof {
        id,
        outline: points(&[
            (10000.0, -2000.0),
            (14000.0, -2000.0),
            (14000.0, -6000.0),
            (10000.0, -6000.0),
        ]),
        edges: vec![
            gable,
            RoofEdge {
                slope: Some(10f64.to_radians()),
                overhang: 400.0,
            },
            gable,
            RoofEdge {
                slope: None,
                overhang: 0.0,
            },
        ],
        ..templates.roof.clone()
    }))?;
    // A quarter-turn stair in the corner room, up to the first floor.
    let id = h.new_id();
    h.run(Command::AddStair(Stair {
        id,
        path: points(&[(800.0, -7100.0), (3300.0, -7100.0), (3300.0, -5550.0)]),
        ..templates.stair.clone()
    }))?;
    // The terrain, falling away to the south, below the ground floor.
    let at = |x, y, z| MeshPoint {
        at: Point::new(x, y),
        z,
    };
    let id = h.new_id();
    h.run(Command::AddMesh(Mesh {
        id,
        outline: vec![
            at(-6000.0, 6000.0, -200.0),
            at(20000.0, 6000.0, -600.0),
            at(20000.0, -16000.0, -2400.0),
            at(-6000.0, -16000.0, -1800.0),
        ],
        inner: vec![at(5000.0, -4000.0, -250.0), at(12000.0, -4000.0, -400.0)],
        bottom: -4000.0,
        ..templates.mesh.clone()
    }))?;
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

    #[test]
    fn the_wall_presets_are_simple_first_then_build_ups_with_known_materials() -> super::Result<()>
    {
        let (h, _, templates) = super::house()?;
        let presets = &templates.wall_presets;
        assert_eq!(presets.len(), 7);
        for (i, p) in presets.iter().enumerate() {
            assert_eq!(p.layers.len() == 1, i < 4, "{}: simple ones first", p.name);
            assert!(p.name.starts_with("preset.wall."), "{}", p.name);
            for l in &p.layers {
                assert!(
                    h.project().materials.iter().any(|m| m.id == l.material),
                    "{}: unknown material",
                    p.name
                );
            }
        }
        let total = |i: usize| presets[i].layers.iter().map(|l| l.thickness).sum::<f64>();
        assert_eq!(
            [total(0), total(1), total(2), total(3)],
            [200.0, 200.0, 200.0, 72.0]
        );
        // Matching by layers must be unambiguous: no two presets are equal.
        for (i, a) in presets.iter().enumerate() {
            for b in &presets[i + 1..] {
                assert_ne!(a.layers, b.layers, "{} and {}", a.name, b.name);
            }
        }
        // The names of the starting library are language pack keys.
        assert!(
            h.project()
                .materials
                .iter()
                .all(|m| m.name.starts_with("mat."))
        );
        Ok(())
    }
}
