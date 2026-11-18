//! A demonstration house, built through commands: a rectangle of external
//! walls, two partitions crossing each other, and a timber frame annex.

use archibim_commands::{Command, History};
use archibim_model::{
    Framing, Id, Layer, LayerRole, LevelKind, LevelRef, Material, Point, Project, RenovationStatus,
    Story, Wall,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// The demonstration house and the story to show.
pub fn house() -> Result<(History, Id)> {
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
    for (start, end, layers) in walls {
        let id = h.new_id();
        h.run(Command::AddWall(Wall {
            id,
            story: ground,
            start: Point::new(start.0, start.1),
            end: Point::new(end.0, end.1),
            layers: layers.clone(),
            bottom: LevelRef {
                story: ground,
                kind: LevelKind::Structural,
                offset: 0.0,
            },
            top: LevelRef {
                story: first,
                kind: LevelKind::Structural,
                offset: 0.0,
            },
        }))?;
    }
    Ok((h, ground))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_demo_house_has_a_plan() -> super::Result<()> {
        let (h, story) = super::house()?;
        let plan =
            archibim_geometry::plan_layers(h.project(), story, archibim_geometry::Phase::Proposed)?;
        assert!(plan.iter().any(|l| !l.members.is_empty()), "studs shown");
        Ok(())
    }
}
