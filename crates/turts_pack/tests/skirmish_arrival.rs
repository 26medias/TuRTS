//! Arrival priority through real package data, commands, ECS movement, and completed state.
use std::{collections::BTreeMap, path::Path};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{Command, CommandKind, SlotId, Vec2, apply_command, completed_world, headless_app};

#[test]
fn the_first_arrival_keeps_the_point_after_ids_and_command_order_are_exchanged() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
    let mut previous = None;
    for swap_ids in [false, true] {
        for reverse_commands in [false, true] {
            let mut map = archive.maps().unwrap().remove(0).1;
            if swap_ids {
                map.placements.swap(1, 2);
            }
            let mut app = headless_app(20);
            let staged = turts_sim::stage_package(
                app.world(),
                archive.content().unwrap(),
                turts_mod_api::SourceDef {
                    file: "maps/field.ron".into(),
                    value: map,
                },
                20,
            )
            .unwrap();
            turts_sim::install_package(app.world_mut(), staged);
            turts_sim::initialize_completed_world(app.world_mut());
            let mut subjects = completed_world(app.world())
                .units
                .iter()
                .filter(|unit| unit.owner_slot == 0 && unit.content_id != "skirmish:base")
                .map(|unit| unit.unit_id)
                .collect::<Vec<_>>();
            if reverse_commands {
                subjects.reverse();
            }
            let goal = Vec2::new(28.0, 24.0);
            for (seq, id) in subjects.iter().enumerate() {
                assert!(
                    apply_command(
                        app.world_mut(),
                        &Command {
                            slot: SlotId(0),
                            seq: seq as u32,
                            kind: CommandKind::MoveTo {
                                unit_ids: vec![*id],
                                target: goal
                            }
                        }
                    )
                    .outcome
                    .is_ok()
                );
            }
            for _ in 0..400 {
                app.update();
            }
            let positions = completed_world(app.world())
                .units
                .iter()
                .filter(|unit| unit.owner_slot == 0)
                .map(|unit| (unit.content_id.clone(), unit.pos))
                .collect::<BTreeMap<_, _>>();
            assert_eq!(
                positions["skirmish:soldier"], goal,
                "the faster soldier lost the empty point"
            );
            assert!(
                positions["skirmish:worker"].distance(goal) >= 1.0 - 1e-5,
                "the later worker overlapped the soldier"
            );
            if let Some(previous) = &previous {
                assert_eq!(
                    &positions, previous,
                    "ids or command order changed the visible result"
                );
            }
            previous = Some(positions);
        }
    }
}
