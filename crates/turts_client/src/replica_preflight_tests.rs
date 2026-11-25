//! T-31: the staging preflight on a 256 x 256 all-visible map with 1,000 units.
//! Before the fix, the client refused the first patch and each resync snapshot of this state.
use super::*;
use std::path::{Path, PathBuf};
use turts_mod_api::{KnowledgeBasis, ObservationPhase, TerrainCell, TerrainProvenance};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::visibility::{
    MAX_PENDING_STATE_BYTES, StateTransactionPayload, StateTransactionStamp,
};
use turts_net_protocol::{
    Activity, HealthRecord, InventoryPort, InventoryState, SlotId, TerrainGroundAnchor,
    TerrainMaterialId, TerrainSnapshot, TerrainSurfaceRecord, UnitDelta, UnitId, UnitOrderRecord,
    UnitSnapshot,
};

const CELLS: u32 = 256;
const UNITS: u32 = 1_000;

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// The Quarry ridge package with the map change of `tests/net/flock_group1000.py`: a flat map of
/// 256 x 256 cells. No visibility policy, so every cell is visible.
fn wide_flat_content() -> ClientContent {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/quarry_ridge");
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let call = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("turts-t31-preflight-{}-{call}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&root, &dir);
    let field = dir.join("maps/field.ron");
    let mut text = std::fs::read_to_string(&field).unwrap();
    let start = text.find("initial_heights:[").unwrap() + "initial_heights:[".len();
    let end = start + text[start..].find("],floor").unwrap();
    text.replace_range(start..end, "");
    for (old, new) in [
        ("size:(x:64.0,y:48.0)", "size:(x:256.0,y:256.0)"),
        ("columns:64,rows:48", "columns:256,rows:256"),
        // The bands must end at the highest initial surface, which is now 0.
        ("(bottom:-2,top:4,", "(bottom:-2,top:0,"),
    ] {
        assert_eq!(text.matches(old).count(), 1, "{old}");
        text = text.replace(old, new);
    }
    std::fs::write(&field, text).unwrap();
    let archive = PackageArchive::parse(&pack_dir(&dir).unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let content = ClientContent::from_archives(&[archive]).unwrap();
    let layout = content.terrain.as_ref().unwrap();
    assert_eq!((layout.grid.columns, layout.grid.rows), (CELLS, CELLS));
    content
}

fn basis(tick: u32) -> KnowledgeBasis {
    KnowledgeBasis {
        match_id: 31,
        map: ContentId::parse("quarry_ridge:field").unwrap(),
        tick,
        phase: ObservationPhase::Completed,
    }
}

fn stamp(tick: u32, id: u64, parent: Option<u64>) -> StateTransactionStamp {
    StateTransactionStamp {
        basis: basis(tick),
        transaction_id: id,
        parent_transaction_id: parent,
    }
}

fn unit_pos(index: u32) -> Pos {
    Pos {
        x: 20.0 + (index % 40) as f32 * 1.2,
        y: 113.0 + (index / 40) as f32 * 1.2,
    }
}

/// The permitted state of the group1000 fixture at rest: every cell, 1,000 Walkers with
/// health, an empty inventory, an idle order record and a ground anchor.
fn snapshot(content: &ClientContent, tick: u32) -> StateTransactionPayload {
    let materials = content
        .catalog
        .terrain_materials
        .keys()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let soil = materials
        .iter()
        .position(|id| id == "quarry_ridge:soil_layer")
        .unwrap() as u32;
    let ids = (1..=UNITS).map(UnitId);
    let mut state = turts_net_protocol::Snapshot {
        tick,
        units: ids
            .clone()
            .map(|unit_id| UnitSnapshot {
                unit_id,
                content_id: "quarry_ridge:walker".into(),
                owner_slot: SlotId(0),
                pos: unit_pos(unit_id.0 - 1),
                facing: None,
                site: false,
            })
            .collect(),
        healths: ids
            .clone()
            .map(|unit| HealthRecord { unit, current: 100 })
            .collect(),
        unit_orders: ids
            .clone()
            .map(|unit| UnitOrderRecord {
                unit,
                activity: Activity::Idle,
                current: None,
                queued: vec![],
                last_failure: None,
                rally: None,
            })
            .collect(),
        ..Default::default()
    };
    state.economy.inventories = ids
        .clone()
        .map(|unit| InventoryState {
            unit,
            port: InventoryPort::Main,
            items: vec![],
        })
        .collect();
    state.records.terrain = Some(TerrainSnapshot {
        materials,
        surfaces: (0..CELLS * CELLS)
            .map(|index| TerrainSurfaceRecord {
                cell: TerrainCell {
                    x: index % CELLS,
                    y: index / CELLS,
                },
                elevation: 0,
                material: Some(TerrainMaterialId(soil)),
                provenance: Some(TerrainProvenance::Natural),
                observed_tick: 0,
                current: true,
            })
            .collect(),
        anchors: ids
            .map(|unit| TerrainGroundAnchor {
                unit,
                elevation: 0,
                observed_tick: tick,
                current: true,
            })
            .collect(),
        work: vec![],
    });
    StateTransactionPayload::Snapshot {
        state: Box::new(state),
        knowledge: None,
        event_units: vec![],
    }
}

fn patch(tick: u32, positions: Vec<UnitDelta>) -> StateTransactionPayload {
    StateTransactionPayload::Patch {
        state: Box::new(WorldDelta {
            tick,
            base_tick: tick - 1,
            positions,
            ..Default::default()
        }),
        knowledge: None,
        event_units: vec![],
    }
}

fn joined_replica(content: &ClientContent) -> Replica {
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(content.clone(), 0));
    assert!(
        replica.activate_transaction(&stamp(8, 1, None), &snapshot(content, 8), Instant::now()),
        "the join snapshot"
    );
    assert_eq!(replica.units.len(), UNITS as usize);
    replica
}

#[test]
fn a_wide_all_visible_map_with_1000_units_accepts_a_patch_and_a_resync_snapshot() {
    let content = wide_flat_content();
    let mut replica = joined_replica(&content);
    assert!(
        replica.activate_transaction(&stamp(9, 2, Some(1)), &patch(9, vec![]), Instant::now()),
        "a patch at rest after the join"
    );
    assert!(
        replica.activate_transaction(&stamp(10, 3, None), &snapshot(&content, 10), Instant::now()),
        "a resync snapshot while the old terrain is resident"
    );
    assert_eq!(replica.last_applied_tick, Some(10));
}

#[test]
fn a_patch_larger_than_the_space_left_in_the_budget_is_refused() {
    let content = wide_flat_content();
    let mut replica = joined_replica(&content);
    let positions = (1..=UNITS)
        .map(|id| UnitDelta {
            unit_id: UnitId(id),
            pos: unit_pos(id),
        })
        .collect::<Vec<_>>();
    // Other transactions in flight hold all of the budget except the bare decoded size of this
    // patch's position table. The patch, its candidate and the resident state cannot fit.
    let room = positions.len() * std::mem::size_of::<UnitDelta>();
    let reserved = MAX_PENDING_STATE_BYTES as usize - room;
    let payload = patch(9, positions);
    assert!(
        !replica.activate_transaction_reserved(
            &stamp(9, 2, Some(1)),
            &payload,
            Instant::now(),
            reserved
        ),
        "the staging guard must refuse a patch that does not fit"
    );
    assert_eq!(replica.last_applied_tick, Some(8));
    assert!(
        replica.activate_transaction_reserved(&stamp(9, 2, Some(1)), &payload, Instant::now(), 0),
        "the same patch is valid when the budget is free"
    );
    assert_eq!(
        replica.units[&1].samples.back(),
        Some(&(9, unit_pos(1))),
        "the patch moved unit 1"
    );
}

#[test]
fn the_preflight_charges_each_table_row_of_a_patch() {
    let content = wide_flat_content();
    let replica = joined_replica(&content);
    let rows = |count: usize| {
        patch(
            9,
            (0..count)
                .map(|index| UnitDelta {
                    unit_id: UnitId(index as u32 % UNITS + 1),
                    pos: unit_pos(index as u32 % UNITS),
                })
                .collect(),
        )
    };
    // The bare decoded rows alone are larger than the whole budget (T-39 M2, Nora's probe).
    let over = MAX_PENDING_STATE_BYTES as usize / std::mem::size_of::<UnitDelta>() + 1;
    assert!(
        replica.preflight_transaction(&rows(over), 0).is_err(),
        "a patch of {over} position rows passed the preflight with a free budget"
    );
    // A large patch that fits is not refused: the charge is not far above the real size.
    assert!(
        replica.preflight_transaction(&rows(100_000), 0).is_ok(),
        "a patch of 100,000 position rows was refused with a free budget"
    );
}
