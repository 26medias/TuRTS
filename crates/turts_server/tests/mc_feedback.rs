//! Task15 ORD2 headless path through the installed Muster catalog and real simulation units.

#[path = "../src/orders.rs"]
#[allow(dead_code)]
mod orders;

use std::{collections::BTreeMap, path::PathBuf};

use bevy_math::Vec2;
use turts_mod_api::{Placement, SourceDef, WorldPos};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{
    OrderEnvelope, OrderTarget, PlayerCommand, Pos, QueueMode, UnitId as ProtocolUnitId,
};
use turts_sim::{
    CommandKind, Position, UnitTypes, apply_command, headless_app, install_package, stage_package,
};

fn muster() -> PackageArchive {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
    PackageArchive::parse(&pack_dir(&path).expect("pack actual Muster package"))
        .expect("parse actual Muster archive")
}

fn positions(app: &mut bevy_app::App) -> BTreeMap<u32, Vec2> {
    let world = app.world_mut();
    let mut query = world.query::<(&turts_sim::UnitId, &Position)>();
    query
        .iter(world)
        .map(|(id, position)| (id.0, position.0))
        .collect()
}

#[test]
fn group_order_uses_muster_unit_types_and_moves_all_eligible_mobile_owners() {
    let archive = muster();
    let catalog = archive.content().expect("resolve actual Muster catalog");
    let (map_file, mut map) = archive
        .maps()
        .expect("read actual Muster map")
        .into_iter()
        .next()
        .expect("Muster exports a map");
    map.placements = (0..290)
        .map(|index| Placement {
            unit: "muster:trooper".into(),
            owner_slot: 0,
            pos: WorldPos {
                x: 2.0 + (index % 20) as f32,
                y: 27.0 + (index / 20) as f32,
            },
            contents: Vec::new(),
            facing: None,
        })
        .chain((0..5).map(|index| Placement {
            unit: "muster:post".into(),
            owner_slot: 0,
            pos: WorldPos {
                x: [30.0, 35.0, 50.0, 55.0, 60.0][index],
                y: 22.0,
            },
            contents: Vec::new(),
            facing: None,
        }))
        .chain((0..5).map(|index| Placement {
            unit: "muster:trooper".into(),
            owner_slot: 1,
            pos: WorldPos {
                x: 40.0 + index as f32,
                y: 22.0,
            },
            contents: Vec::new(),
            facing: None,
        }))
        .collect();

    let mut app = headless_app(20);
    let staged = stage_package(
        app.world(),
        catalog.clone(),
        SourceDef {
            file: map_file,
            value: map,
        },
        20,
    )
    .expect("stage real Muster definitions with the controlled fixture map");
    install_package(app.world_mut(), staged);

    let unit_types = app.world().resource::<UnitTypes>().clone();
    let before = positions(&mut app);
    let ids = before
        .keys()
        .copied()
        .map(ProtocolUnitId)
        .collect::<Vec<_>>();
    assert_eq!(
        ids.len(),
        300,
        "the controlled map contains 300 real Muster units"
    );

    let command = PlayerCommand {
        seq: 1,
        order: OrderEnvelope {
            order: "muster:order_move".into(),
            units: ids,
            target: OrderTarget::Point(Pos { x: 55.0, y: 40.0 }),
            queue: QueueMode::Replace,
            group: 0,
        },
    };
    let converted = orders::to_sim(turts_sim::SlotId(0), &command, &catalog)
        .expect("Muster move resolves through its real order definition");
    assert_eq!(converted.len(), 1);
    assert!(matches!(converted[0].kind, CommandKind::Order(_)));
    let result = apply_command(app.world_mut(), &converted[0]);
    assert_eq!(result.units.accepted.len(), 290);
    assert_eq!(
        result
            .units
            .refused
            .iter()
            .filter(|(_, reason)| *reason == turts_sim::RejectReason::NotApplicable)
            .count(),
        5
    );
    assert_eq!(
        result
            .units
            .refused
            .iter()
            .filter(|(_, reason)| *reason == turts_sim::RejectReason::NotOwner)
            .count(),
        5
    );

    app.update();
    let after = positions(&mut app);
    let moved_owned_mobile = result
        .units
        .accepted
        .iter()
        .filter(|id| {
            before
                .get(id)
                .zip(after.get(id))
                .is_some_and(|(before, after)| before.distance_squared(*after) > 0.0001)
        })
        .count();
    assert_eq!(
        moved_owned_mobile, 290,
        "every accepted own trooper changes its actual simulation Position on the next tick"
    );
    for (id, position) in &before {
        let unit = app
            .world()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|unit| unit.0 == *id)
            })
            .unwrap();
        let owner = unit.get::<turts_sim::Owner>().unwrap().0;
        let content = unit
            .get::<turts_sim::UnitType>()
            .and_then(|kind| unit_types.get(kind.0))
            .unwrap()
            .content_id
            .clone();
        if owner == turts_sim::SlotId(1) || content == "muster:post" {
            assert_eq!(
                after.get(id),
                Some(position),
                "refused unit {id} does not move on the simulation tick"
            );
        }
    }
}
