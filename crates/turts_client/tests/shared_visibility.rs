//! SV6 native activation probes use the ordinary client replica and public catalog.
use bevy::platform::time::Instant;
use turts_client::{
    content::ClientContent,
    replica::{Replica, ReplicaContext},
};
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{visibility::*, visibility_transaction::*, *};

fn setup() -> (Replica, StateTransactionStamp, StateTransactionPayload) {
    let archive = PackageArchive::parse(
        &pack_dir(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../mods/visibility_probe"),
        )
        .unwrap(),
    )
    .unwrap();
    let content = ClientContent::from_archives(&[archive]).unwrap();
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(content, 0));
    let basis = KnowledgeBasis {
        match_id: 7,
        map: ContentId::parse("test:map").unwrap(),
        tick: 10,
        phase: ObservationPhase::Completed,
    };
    let stamp = StateTransactionStamp {
        basis: basis.clone(),
        transaction_id: 1,
        parent_transaction_id: None,
    };
    let knowledge = KnowledgeSnapshot {
        basis,
        viewer: 0,
        grid: VisibilityGrid {
            size: WorldPos { x: 8., y: 8. },
            cell_size: 4.,
            columns: 2,
            rows: 2,
        },
        observed: vec![3],
        explored: vec![3],
        remembered_units: vec![],
        remembered_nodes: vec![],
        retention_truncated: false,
    };
    let state = Snapshot {
        tick: 10,
        healths: vec![HealthRecord {
            unit: UnitId(1),
            current: 100,
        }],
        units: vec![UnitSnapshot {
            unit_id: UnitId(1),
            content_id: "visibility_probe:scout".into(),
            owner_slot: SlotId(1),
            pos: Pos { x: 5., y: 1. },
            facing: None,
            site: false,
        }],
        ..Default::default()
    };
    (
        replica,
        stamp,
        StateTransactionPayload::Snapshot {
            state: Box::new(state),
            knowledge: Some(knowledge),
            event_units: vec![],
        },
    )
}
fn hidden_patch(stamp: &StateTransactionStamp) -> (StateTransactionStamp, StateTransactionPayload) {
    let stamp = StateTransactionStamp {
        basis: KnowledgeBasis {
            tick: 13,
            ..stamp.basis.clone()
        },
        transaction_id: 2,
        parent_transaction_id: Some(1),
    };
    let knowledge = KnowledgeDelta {
        basis: stamp.basis.clone(),
        viewer: 0,
        coverage: vec![CoverageRun {
            first_cell: 1,
            cell_count: 1,
            observed: false,
            explored: true,
        }],
        hidden_units: vec![1],
        hidden_nodes: vec![],
        revealed_units: vec![],
        revealed_nodes: vec![],
        remembered_units: vec![RememberedUnit {
            unit: 1,
            content: ContentId::parse("visibility_probe:scout").unwrap(),
            owner: 1,
            position: WorldPos { x: 5., y: 1. },
            facing: None,
            site: false,
            last_seen_tick: 10,
        }],
        remembered_nodes: vec![],
        forgotten_units: vec![],
        forgotten_nodes: vec![],
        retention_truncated: false,
    };
    let payload = StateTransactionPayload::Patch {
        state: Box::new(WorldDelta {
            tick: 13,
            base_tick: 10,
            ..Default::default()
        }),
        knowledge: Some(knowledge),
        event_units: vec![],
    };
    (stamp, payload)
}
#[test]
fn interrupted_transport_keeps_old_view_until_complete_activation_and_ack() {
    let (mut replica, stamp, payload) = setup();
    let mut assembler = StateTransactionAssembler::new(7, stamp.basis.map.clone(), 1024);
    let mut encoded = encode_transaction(stamp.clone(), &payload, 1024).unwrap();
    let mut staged = None;
    while let Some(frame) = encoded.next_frame() {
        assert!(replica.units.is_empty());
        assert!(replica.knowledge.is_none());
        match frame {
            StateTransactionFrame::Begin(frame) => assembler.begin(frame, 0).unwrap(),
            StateTransactionFrame::Chunk(frame) => assembler.chunk(&frame).unwrap(),
            StateTransactionFrame::End(frame) => staged = assembler.end(&frame).unwrap(),
        }
        assert!(assembler.active().is_none());
    }
    let staged = staged.unwrap();
    assert!(replica.activate_transaction(&staged.stamp, &staged.payload, Instant::now()));
    assert!(replica.units.contains_key(&1));
    assert_eq!(assembler.activate(&stamp).unwrap().stamp, stamp);
}
#[test]
fn hide_removes_live_interpolation_and_health_without_inventing_death() {
    let (mut replica, stamp, payload) = setup();
    assert!(replica.activate_transaction(&stamp, &payload, Instant::now()));
    assert_eq!(replica.healths[0].current, 100);
    assert_eq!(replica.health_trace[&1].last().unwrap().current, 100);
    let (stamp, payload) = hidden_patch(&stamp);
    assert!(replica.activate_transaction(&stamp, &payload, Instant::now()));
    assert!(!replica.units.contains_key(&1));
    assert!(replica.sample(1, 13.).is_none());
    assert!(replica.healths.is_empty());
    assert!(replica.health_trace.is_empty());
    assert!(replica.visual_events.is_empty());
    assert_eq!(
        replica.knowledge.as_ref().unwrap().remembered_units[0].last_seen_tick,
        10
    );
}
#[test]
fn invalid_late_record_rejects_coverage_memory_and_live_changes_together() {
    let (mut replica, stamp, payload) = setup();
    assert!(replica.activate_transaction(&stamp, &payload, Instant::now()));
    let original = replica.knowledge.clone();
    let (stamp, mut payload) = hidden_patch(&stamp);
    if let StateTransactionPayload::Patch { state, .. } = &mut payload {
        state.healths.push(HealthRecord {
            unit: UnitId(999),
            current: 1,
        });
    }
    assert!(!replica.activate_transaction(&stamp, &payload, Instant::now()));
    assert_eq!(replica.knowledge, original);
    assert!(replica.units.contains_key(&1));
    assert_eq!(replica.last_applied_tick, Some(10));
}

#[test]
fn earlier_event_positions_are_accepted_only_for_all_visible_fallback() {
    for restricted in [false, true] {
        let (mut replica, stamp, mut payload) = setup();
        if let StateTransactionPayload::Snapshot {
            state,
            knowledge,
            event_units,
        } = &mut payload
        {
            if !restricted {
                *knowledge = None;
            }
            event_units.push(EventUnitObservation {
                basis: KnowledgeBasis {
                    tick: stamp.basis.tick - 1,
                    ..stamp.basis.clone()
                },
                unit: state.units[0].clone(),
            });
        }
        assert_eq!(
            replica.activate_transaction(&stamp, &payload, Instant::now()),
            !restricted
        );
        if restricted {
            assert!(replica.last_applied_tick.is_none());
        }
    }
}

#[test]
fn public_two_human_scouting_fixture_loads_through_the_normal_engine() {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/visibility_probe");
    let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
    let maps = archive.maps().unwrap();
    assert_eq!(
        maps[0].1.visibility.as_ref().unwrap().sharing,
        VisionSharing::OwnSlot
    );
    let mut app = turts_sim::headless_app(20);
    turts_pack::load_package(app.world_mut(), &archive).unwrap();
    assert!(turts_sim::completed_world(app.world()).units.is_empty());
    turts_sim::visibility::initialize_context(app.world_mut(), 7, &[0, 1]).unwrap();
    turts_sim::initialize_completed_world(app.world_mut());
    let world = turts_sim::completed_world(app.world());
    assert_eq!(world.units.len(), 2);
    assert_eq!(world.nodes.len(), 1);
    assert_eq!(
        world
            .units
            .iter()
            .map(|unit| unit.owner_slot)
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([0, 1])
    );
}

#[test]
fn observed_killing_hit_remains_available_for_alerts_in_atomic_views() {
    for restricted in [false, true] {
        let (mut replica, stamp, mut payload) = setup();
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../mods/visibility_probe");
        let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
        replica.set_context(ReplicaContext::new(
            ClientContent::from_archives(&[archive]).unwrap(),
            0,
        ));
        let unit = if let StateTransactionPayload::Snapshot {
            state, knowledge, ..
        } = &mut payload
        {
            state.units[0].content_id = "visibility_probe:scout".into();
            state.units[0].owner_slot = SlotId(0);
            state.healths = vec![HealthRecord {
                unit: UnitId(1),
                current: 100,
            }];
            if !restricted {
                *knowledge = None;
            }
            state.units[0].clone()
        } else {
            unreachable!()
        };
        assert!(replica.activate_transaction(&stamp, &payload, Instant::now()));
        let (stamp, mut patch) = hidden_patch(&stamp);
        if let StateTransactionPayload::Patch {
            state,
            knowledge,
            event_units,
        } = &mut patch
        {
            state.removed = vec![UnitId(1)];
            state.healths = vec![HealthRecord {
                unit: UnitId(1),
                current: 0,
            }];
            state
                .records
                .effects
                .push(Effect::Destroyed { unit: UnitId(1) });
            if restricted {
                let delta = knowledge.as_mut().unwrap();
                delta.coverage.clear();
                delta.hidden_units.clear();
                delta.remembered_units.clear();
            } else {
                *knowledge = None;
            }
            event_units.push(EventUnitObservation {
                basis: stamp.basis.clone(),
                unit,
            });
        }
        assert!(replica.activate_transaction(&stamp, &patch, Instant::now()));
        assert_eq!(replica.health_trace[&1].last().unwrap().current, 0);
        assert!(!replica.units.contains_key(&1));
    }
}
