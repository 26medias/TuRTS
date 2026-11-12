//! L0-STORE: the clone cost of the progress records, before and after the move into the side
//! store (`docs/measurements/mp1.md`, "The client store of the 6.8 records").
//!
//! Two scenarios, each applied through `Replica::apply_delta` with no resync:
//! - `base`: the real sim and the real export (package `base`, one Command Centre, 4 jobs, 300
//!   ticks), as in `job_progress_clients.rs`;
//! - `scale`: a hand-built world with 40 producers of 4 jobs (160 progress records) and 100 empty
//!   deltas, so the table size dominates.
//!
//! The counter: `replica_store::store_clone_count()` counts each clone of the store. The store is
//! not cloned for each delta, so the count is 0 for both scenarios. Before the move the same
//! tables were a field of the candidate and the candidate was cloned twice for each applied delta
//! (`candidate_at` and `stage_delta`); the "before" figures in the measurement note come from this
//! test with a temporary counter at those two places (`probe` below).

#[path = "../../turts_server/src/export.rs"]
mod export;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use bevy::platform::time::Instant;
use turts_client::content::ClientContent;
use turts_client::replica::{DeltaOutcome, Replica, ReplicaContext, SnapshotOutcome};
use turts_mod_runtime::archive::PackageArchive;
use turts_mod_runtime::pack::pack_dir;
use turts_net_protocol::{
    BalanceState, EconomyChanges, EconomyState, JobId, JobProgress, JobStage, JobState, Pos,
    QueueState, RateRecord, RecordState, SlotId, Snapshot, UnitId, UnitSnapshot, WorldDelta,
};
use turts_sim::economy::events::{committed_tick, completed_world};
use turts_sim::{
    Command, CommandKind, CommandQueue, SlotId as SimSlot, headless_app, initialize_completed_world,
};

static SERIAL: Mutex<()> = Mutex::new(());

/// The counter is process-wide: the two scenarios run one at a time.
fn serial() -> MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The clone counters `(clones, table entries cloned)` of the side store.
mod probe {
    pub fn read(replica: &turts_client::replica::Replica) -> (u64, u64) {
        let clones = turts_client::replica_store::store_clone_count();
        // Entries cloned by the store clones: each clone copies the whole store.
        (clones, clones * replica.store().record_count() as u64)
    }
}

fn package(name: &str) -> PackageArchive {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mods")
        .join(name);
    PackageArchive::parse(&pack_dir(&dir).unwrap()).unwrap()
}

fn replica_for(archive: &PackageArchive) -> Replica {
    let content = ClientContent::from_archives(std::slice::from_ref(archive)).unwrap();
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(content, 0));
    replica
}

/// The bytes of one table entry that a clone copies: the key and the record (heap parts are
/// empty in these scenarios: `charged` is empty).
fn entry_bytes() -> usize {
    std::mem::size_of::<((u32, u64), RateRecord)>()
}

#[test]
fn base_scenario_applies_300_deltas_with_no_store_clone() {
    let _guard = serial();
    let archive = package("base");
    let mut app = headless_app(20);
    turts_pack::load_package(app.world_mut(), &archive).unwrap();
    initialize_completed_world(app.world_mut());
    let view0 = completed_world(app.world()).clone();
    let now = Instant::now();
    let mut client = replica_for(&archive);
    client.apply_snapshot(&export::snapshot_for(&view0, SimSlot(0)), now);
    let producer = view0
        .units
        .iter()
        .find(|unit| unit.content_id == "base:command_center" && unit.owner_slot == 0)
        .unwrap()
        .unit_id;
    for seq in 1..=4u32 {
        app.world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .push_back(Command {
                slot: SimSlot(0),
                seq,
                kind: CommandKind::Produce {
                    producer_id: producer,
                    recipe_id: turts_mod_api::ContentId::parse("base:train_rifleman").unwrap(),
                },
            });
    }
    let before = probe::read(&client);
    let mut records_seen = 0u64;
    for tick in 1..=300u32 {
        app.update();
        let batch = committed_tick(app.world()).clone();
        let view = completed_world(app.world()).clone();
        let delta = export::delta_for(&batch, &view, SimSlot(0));
        assert_eq!(client.apply_delta(&delta, now), DeltaOutcome::Applied);
        records_seen += client.store().record_count() as u64;
        let _ = tick;
    }
    let after = probe::read(&client);
    let clones = after.0 - before.0;
    let entries = after.1 - before.1;
    eprintln!(
        "STORECOST base: 300 deltas, {clones} store clones, {entries} table entries cloned, {} B cloned ({} B per entry), {records_seen} record-ticks held",
        entries as usize * entry_bytes(),
        entry_bytes()
    );
    assert_eq!(clones, 0, "the apply path clones the store");
    assert!(records_seen > 0, "the scenario holds job records");
}

fn scale_snapshot(producers: u32, jobs: u64) -> Snapshot {
    let mut snapshot = Snapshot {
        tick: 10,
        economy: EconomyState {
            balances: vec![BalanceState {
                resource: "base:credits".to_string(),
                on_hand: 500,
                reserved: 0,
            }],
            inventories: vec![],
            orders: vec![],
            queues: vec![],
        },
        ..Default::default()
    };
    for producer in 1..=producers {
        snapshot.units.push(UnitSnapshot {
            site: false,
            unit_id: UnitId(producer),
            content_id: "base:command_center".to_string(),
            owner_slot: SlotId(0),
            pos: Pos {
                x: producer as f32,
                y: 8.0,
            },
            facing: None,
        });
        let mut queue = QueueState {
            producer: UnitId(producer),
            jobs: vec![],
            enabled: true,
        };
        for job in 0..jobs {
            let id = u64::from(producer) * 100 + job;
            queue.jobs.push(JobState {
                id: JobId(id),
                recipe: "base:train_rifleman".to_string(),
                stage: JobStage::Queued,
                duration_ticks: 100,
                charged: vec![],
                reserved: vec![],
            });
            snapshot.records.jobs.push(JobProgress {
                producer: UnitId(producer),
                job: JobId(id),
                progress: RateRecord {
                    done: 0,
                    total: 100,
                    per_tick: 0,
                    at_tick: 10,
                    state: RecordState::Running,
                    charged: vec![],
                    held: false,
                    held_since_tick: 0,
                },
            });
        }
        snapshot.economy.queues.push(queue);
    }
    snapshot
}

#[test]
fn scale_scenario_applies_100_deltas_with_no_store_clone() {
    let _guard = serial();
    let archive = package("base");
    let mut client = replica_for(&archive);
    let now = Instant::now();
    assert_eq!(
        client.apply_snapshot(&scale_snapshot(40, 4), now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    let held = client.store().record_count();
    assert_eq!(held, 160, "40 producers of 4 jobs");
    let before = probe::read(&client);
    for tick in 11..=110u32 {
        let delta = WorldDelta {
            tick,
            base_tick: tick - 1,
            economy: EconomyChanges::default(),
            ..Default::default()
        };
        assert_eq!(client.apply_delta(&delta, now), DeltaOutcome::Applied);
    }
    let after = probe::read(&client);
    let clones = after.0 - before.0;
    let entries = after.1 - before.1;
    eprintln!(
        "STORECOST scale: 100 deltas, {held} records held, {clones} store clones, {entries} table entries cloned, {} B cloned ({} B per entry)",
        entries as usize * entry_bytes(),
        entry_bytes()
    );
    assert_eq!(clones, 0, "the apply path clones the store");
}
