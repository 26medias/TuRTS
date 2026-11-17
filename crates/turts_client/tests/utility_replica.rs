//! M6: the owner utility view commits with the state transaction, and only whole.
use bevy::platform::time::Instant;
use std::collections::BTreeMap;
use turts_client::{
    content::ClientContent,
    replica::{DeltaOutcome, Replica, ReplicaContext, SnapshotOutcome},
};
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{visibility::*, *};

const LINE: &str = "visibility_probe:line";

fn catalog_utility() -> ResolvedUtility {
    ResolvedUtility {
        id: ContentId::parse(LINE).unwrap(),
        name: "Line".into(),
        carries: ContentId::parse("visibility_probe:power").unwrap(),
        ports: BTreeMap::new(),
        depth: UtilityDepth {
            default: 2,
            min: 1,
            max: 4,
        },
        reach: 10.0,
        max_segment_length: 16.0,
        segment_cost: vec![],
        exposure: UtilityExposure::Disconnect,
        appearance: UtilityAppearance {
            colour: "#ffcc00".into(),
            exposed_colour: "#ff4400".into(),
            junction: UtilitySymbol::Circle,
            labels: UtilityLabels {
                powered: "Powered".into(),
                short: "No power".into(),
                unconnected: "Unconnected".into(),
                exposed: "Exposed".into(),
            },
        },
    }
}

fn replica(with_utility: bool) -> Replica {
    let archive = PackageArchive::parse(
        &pack_dir(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../mods/visibility_probe"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut content = ClientContent::from_archives(&[archive]).unwrap();
    if with_utility {
        let utility = catalog_utility();
        content
            .catalog
            .utilities
            .insert(utility.id.clone(), utility);
    }
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(content, 0));
    replica
}

fn node(id: u32, x: f32) -> UtilityNodeRecord {
    UtilityNodeRecord {
        id,
        utility: LINE.into(),
        pos: Pos { x, y: 2.0 },
    }
}

fn view(nodes: &[u32]) -> UtilityView {
    UtilityView {
        nodes: nodes.iter().map(|id| node(*id, *id as f32)).collect(),
        segments: nodes
            .windows(2)
            .map(|pair| UtilitySegmentRecord {
                id: pair[0] * 100 + pair[1],
                utility: LINE.into(),
                a: UtilityEndRecord::Node(pair[0]),
                b: UtilityEndRecord::Node(pair[1]),
                elevation: -2,
                depth: 2,
                state: UtilitySegmentState::Conducting,
            })
            .collect(),
        networks: vec![],
    }
}

fn basis(tick: u32) -> turts_mod_api::KnowledgeBasis {
    KnowledgeBasis {
        match_id: 7,
        map: ContentId::parse("test:map").unwrap(),
        tick,
        phase: ObservationPhase::Completed,
    }
}

fn grid() -> VisibilityGrid {
    VisibilityGrid {
        size: WorldPos { x: 8., y: 8. },
        cell_size: 4.,
        columns: 2,
        rows: 2,
    }
}

fn knowledge(tick: u32) -> KnowledgeSnapshot {
    KnowledgeSnapshot {
        basis: basis(tick),
        viewer: 0,
        grid: grid(),
        observed: vec![3],
        explored: vec![3],
        remembered_units: vec![],
        remembered_nodes: vec![],
        retention_truncated: false,
    }
}

fn snapshot(tick: u32, utilities: Option<UtilityView>) -> Snapshot {
    Snapshot {
        tick,
        units: vec![UnitSnapshot {
            unit_id: UnitId(1),
            content_id: "visibility_probe:scout".into(),
            owner_slot: SlotId(0),
            pos: Pos { x: 5., y: 1. },
            facing: None,
            site: false,
        }],
        records: SnapshotRecords {
            utilities,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn stamp(tick: u32, id: u64, parent: Option<u64>) -> StateTransactionStamp {
    StateTransactionStamp {
        basis: basis(tick),
        transaction_id: id,
        parent_transaction_id: parent,
    }
}

fn snapshot_payload(tick: u32, utilities: Option<UtilityView>) -> StateTransactionPayload {
    StateTransactionPayload::Snapshot {
        state: Box::new(snapshot(tick, utilities)),
        knowledge: Some(knowledge(tick)),
        event_units: vec![],
    }
}

fn patch_payload(tick: u32, base: u32, utilities: Option<UtilityView>) -> StateTransactionPayload {
    StateTransactionPayload::Patch {
        state: Box::new(WorldDelta {
            tick,
            base_tick: base,
            records: DeltaRecords {
                utilities,
                ..Default::default()
            },
            ..Default::default()
        }),
        knowledge: Some(KnowledgeDelta {
            basis: basis(tick),
            viewer: 0,
            coverage: vec![],
            hidden_units: vec![],
            hidden_nodes: vec![],
            revealed_units: vec![],
            revealed_nodes: vec![],
            remembered_units: vec![],
            remembered_nodes: vec![],
            forgotten_units: vec![],
            forgotten_nodes: vec![],
            retention_truncated: false,
        }),
        event_units: vec![],
    }
}

fn activate(
    replica: &mut Replica,
    stamp: &StateTransactionStamp,
    payload: &StateTransactionPayload,
) -> bool {
    replica.activate_transaction(stamp, payload, Instant::now())
}

#[test]
fn a_snapshot_commits_the_view_with_the_units() {
    let mut replica = replica(true);
    let rows = view(&[1, 2, 3]);
    assert!(activate(
        &mut replica,
        &stamp(10, 1, None),
        &snapshot_payload(10, Some(rows.clone()))
    ));
    assert!(replica.units.contains_key(&1));
    assert_eq!(replica.utilities, Some(rows));
}

#[test]
fn a_patch_with_a_view_replaces_the_whole_view_and_none_keeps_it() {
    let mut replica = replica(true);
    assert!(activate(
        &mut replica,
        &stamp(10, 1, None),
        &snapshot_payload(10, Some(view(&[1, 2, 3])))
    ));
    // `None` is no change.
    assert!(activate(
        &mut replica,
        &stamp(11, 2, Some(1)),
        &patch_payload(11, 10, None)
    ));
    assert_eq!(replica.utilities, Some(view(&[1, 2, 3])));
    // `Some` replaces everything: the old rows are gone, even when the new view is empty.
    assert!(activate(
        &mut replica,
        &stamp(12, 3, Some(2)),
        &patch_payload(12, 11, Some(view(&[7, 8])))
    ));
    assert_eq!(replica.utilities, Some(view(&[7, 8])));
    assert!(activate(
        &mut replica,
        &stamp(13, 4, Some(3)),
        &patch_payload(13, 12, Some(UtilityView::default()))
    ));
    assert_eq!(replica.utilities, Some(UtilityView::default()));
}

#[test]
fn a_bad_view_refuses_the_whole_snapshot() {
    for bad in [
        // The same junction id twice.
        UtilityView {
            nodes: vec![node(1, 1.0), node(1, 2.0)],
            ..Default::default()
        },
        // A segment to a junction that is not in the view.
        UtilityView {
            nodes: vec![node(1, 1.0)],
            segments: vec![UtilitySegmentRecord {
                id: 1,
                utility: LINE.into(),
                a: UtilityEndRecord::Node(1),
                b: UtilityEndRecord::Node(9),
                elevation: -1,
                depth: 1,
                state: UtilitySegmentState::Conducting,
            }],
            networks: vec![],
        },
        // A utility type that the catalog does not have.
        UtilityView {
            nodes: vec![UtilityNodeRecord {
                id: 1,
                utility: "visibility_probe:unknown".into(),
                pos: Pos { x: 0.0, y: 0.0 },
            }],
            ..Default::default()
        },
        // A position that is not finite.
        UtilityView {
            nodes: vec![node(1, f32::NAN)],
            ..Default::default()
        },
    ] {
        let mut replica = replica(true);
        assert!(!activate(
            &mut replica,
            &stamp(10, 1, None),
            &snapshot_payload(10, Some(bad))
        ));
        assert!(replica.units.is_empty(), "no unit commits without the view");
        assert_eq!(replica.utilities, None);
    }
}

#[test]
fn a_bad_patch_view_changes_nothing() {
    let mut replica = replica(true);
    assert!(activate(
        &mut replica,
        &stamp(10, 1, None),
        &snapshot_payload(10, Some(view(&[1, 2])))
    ));
    let bad = UtilityView {
        nodes: vec![node(5, 1.0), node(5, 2.0)],
        ..Default::default()
    };
    assert!(!activate(
        &mut replica,
        &stamp(11, 2, Some(1)),
        &patch_payload(11, 10, Some(bad))
    ));
    assert_eq!(replica.utilities, Some(view(&[1, 2])));
    assert_eq!(replica.last_applied_tick, Some(10));
}

#[test]
fn the_snapshot_holds_a_view_exactly_when_the_catalog_has_a_utility_type() {
    let mut without = replica(false);
    assert!(!activate(
        &mut without,
        &stamp(10, 1, None),
        &snapshot_payload(10, Some(UtilityView::default()))
    ));
    assert!(activate(
        &mut without,
        &stamp(10, 1, None),
        &snapshot_payload(10, None)
    ));
    assert_eq!(without.utilities, None);
    let mut with = replica(true);
    assert!(!activate(
        &mut with,
        &stamp(10, 1, None),
        &snapshot_payload(10, None)
    ));
    // A delta view is also refused when the catalog has no utility type.
    assert!(!activate(
        &mut without,
        &stamp(11, 2, Some(1)),
        &patch_payload(11, 10, Some(UtilityView::default()))
    ));
}

#[test]
fn a_new_context_clears_the_view() {
    let mut replica = replica(true);
    assert!(activate(
        &mut replica,
        &stamp(10, 1, None),
        &snapshot_payload(10, Some(view(&[1, 2])))
    ));
    assert!(replica.utilities.is_some());
    let content = {
        let archive = PackageArchive::parse(
            &pack_dir(
                &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../mods/visibility_probe"),
            )
            .unwrap(),
        )
        .unwrap();
        ClientContent::from_archives(&[archive]).unwrap()
    };
    replica.set_context(ReplicaContext::new(content, 0));
    assert_eq!(
        replica.utilities, None,
        "a reset keeps no row of the old match"
    );
}

#[test]
fn the_plain_snapshot_and_delta_path_carries_the_view_too() {
    // A match with no terrain and no visibility transaction uses `apply_snapshot`/`apply_delta`.
    let mut replica = replica(true);
    let now = Instant::now();
    assert_eq!(
        replica.apply_snapshot(&snapshot(10, Some(view(&[1, 2]))), now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    assert_eq!(replica.utilities, Some(view(&[1, 2])));
    let mut delta = WorldDelta {
        tick: 11,
        base_tick: 10,
        ..Default::default()
    };
    assert_eq!(replica.apply_delta(&delta, now), DeltaOutcome::Applied);
    assert_eq!(replica.utilities, Some(view(&[1, 2])), "None is no change");
    delta = WorldDelta {
        tick: 12,
        base_tick: 11,
        records: DeltaRecords {
            utilities: Some(view(&[4, 5, 6])),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(replica.apply_delta(&delta, now), DeltaOutcome::Applied);
    assert_eq!(replica.utilities, Some(view(&[4, 5, 6])));
}
