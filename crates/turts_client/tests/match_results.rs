//! A real final tick must reach both owners and survive a late snapshot.
#![allow(dead_code)]
use bevy::app::{App, FixedPostUpdate, FixedPreUpdate, FixedUpdate};
use turts_client::{
    content::ClientContent,
    replica::{DeltaOutcome, Replica, ReplicaContext, SnapshotOutcome},
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::Outcome;
use turts_sim::{Command, CommandKind, SlotId, apply_command, committed_tick, completed_world};
#[path = "../../turts_server/src/export.rs"]
mod export;

fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedPreUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedPostUpdate);
}

#[test]
fn final_delta_and_late_snapshot_give_both_owners_the_server_result() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
    for (defeated, snapshot_only) in [(0, false), (1, false), (0, true), (1, true)] {
        let mut app = turts_sim::headless_app(20);
        turts_pack::load_package(app.world_mut(), &archive).unwrap();
        let content = ClientContent {
            catalog: archive.content().unwrap(),
            ..Default::default()
        };
        let mut replicas = Vec::new();
        for slot in [0, 1] {
            let mut replica = Replica::new(20);
            replica.set_context(ReplicaContext::new(content.clone(), slot));
            let snapshot = export::snapshot_for(completed_world(app.world()), SlotId(slot));
            assert!(matches!(
                replica.apply_snapshot(&snapshot, bevy::platform::time::Instant::now()),
                SnapshotOutcome::Applied { .. }
            ));
            assert!(replica.own_outcome().is_none());
            replicas.push(replica);
        }
        let base = completed_world(app.world())
            .units
            .iter()
            .find(|unit| unit.owner_slot == defeated && unit.content_id == "skirmish:base")
            .unwrap()
            .unit_id;
        assert!(
            apply_command(
                app.world_mut(),
                &Command {
                    slot: SlotId(defeated),
                    seq: 0,
                    kind: CommandKind::Remove { unit_id: base }
                }
            )
            .outcome
            .is_ok()
        );
        tick(&mut app);
        for slot in [0, 1] {
            let expected = if slot == defeated {
                Outcome::Defeat
            } else {
                Outcome::Victory
            };
            let delta = export::delta_for(
                committed_tick(app.world()),
                completed_world(app.world()),
                SlotId(slot),
            );
            if snapshot_only {
                // The terminal snapshot is reliable. No final delta needs to arrive first.
                let snapshot = export::snapshot_for(completed_world(app.world()), SlotId(slot));
                assert!(matches!(
                    replicas[usize::from(slot)]
                        .apply_snapshot(&snapshot, bevy::platform::time::Instant::now()),
                    SnapshotOutcome::Applied { .. }
                ));
            } else {
                let result = replicas[usize::from(slot)]
                    .apply_delta(&delta, bevy::platform::time::Instant::now());
                assert_eq!(result, DeltaOutcome::Applied, "final delta: {delta:?}");
            }
            assert_eq!(replicas[usize::from(slot)].outcome(), Some(expected));
            let mut rejoined = Replica::new(20);
            rejoined.set_context(ReplicaContext::new(content.clone(), slot));
            let snapshot = export::snapshot_for(completed_world(app.world()), SlotId(slot));
            assert!(matches!(
                rejoined.apply_snapshot(&snapshot, bevy::platform::time::Instant::now()),
                SnapshotOutcome::Applied { .. }
            ));
            assert_eq!(rejoined.outcome(), Some(expected));
            assert_eq!(
                rejoined.own_outcome(),
                replicas[usize::from(slot)].own_outcome()
            );
        }
    }
}
