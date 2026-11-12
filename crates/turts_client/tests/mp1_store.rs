//! L0-STORE: the client store of the 6.8 records (ruling R-H; design 6.8, "No client prediction").
//!
//! The replica is the candidate and one side store that commits with it in one apply path. These
//! tests run through `Replica::apply_snapshot` and `Replica::apply_delta` only, with hand-built
//! protocol-5 messages, and read the records through the getters of the replica.
//!
//! Each test that reads the clone counter takes `serial()`: the counter is a process-wide value
//! and the tests of one binary run in parallel.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use bevy::platform::time::Instant;
use turts_client::content::ClientContent;
use turts_client::replica::{DeltaOutcome, Replica, ReplicaContext, SnapshotOutcome};
use turts_client::replica_store::store_clone_count;
use turts_mod_api::{ResourceDef, ResourceKind, SupplyCounts, SupplyScope, SupplySide};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{
    BalanceState, ConstructionQueue, HeldOrder, JobId, JobProgress, JobStage, JobState, MatchState,
    Outcome, OutcomeReason, Pos, QueueState, RateRecord, RecordState, SiteProgress, SlotConnection,
    SlotId, SlotOutcome, Snapshot, SupplyRecord, UnitId, UnitSnapshot, WorldDelta,
};
use turts_rules::Ratio;
use turts_rules::charge::{CancelCase, CancelRefunds};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

const POWER: &str = "base:power";
const OWN: u32 = 1;
const CONSTRUCTOR: u32 = 2;
const ENEMY: u32 = 3;
const SITE: u32 = 20;
const ENEMY_SITE: u32 = 21;

fn content() -> ClientContent {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
    let archive = PackageArchive::parse(&pack_dir(&base).expect("pack real base package"))
        .expect("parse base package");
    let mut content = ClientContent::from_archives(&[archive]).expect("resolve base catalog");
    // The base package has no `Supply` resource: the catalog of the test adds one.
    content.catalog.resources.insert(
        turts_mod_api::ContentId::parse(POWER).unwrap(),
        ResourceDef {
            id: "power".to_string(),
            name: "Power".to_string(),
            unit_name: "power".to_string(),
            kind: ResourceKind::Supply {
                scope: SupplyScope::Slot,
                counts: SupplyCounts::Completed,
            },
        },
    );
    content
}

fn replica() -> Replica {
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(content(), 0));
    replica
}

fn unit(id: u32, content: &str, slot: u8, site: bool) -> UnitSnapshot {
    UnitSnapshot {
        unit_id: UnitId(id),
        content_id: format!("base:{content}"),
        owner_slot: SlotId(slot),
        facing: None,
        pos: Pos {
            x: id as f32,
            y: 8.0,
        },
        site,
    }
}

fn rate(done: u64, total: u64, per_tick: u64, at_tick: u32) -> RateRecord {
    RateRecord {
        done,
        total,
        per_tick,
        at_tick,
        state: RecordState::Running,
        charged: vec![],
        held: false,
        held_since_tick: 0,
    }
}

fn job_state(id: u64, duration: u64) -> JobState {
    JobState {
        id: JobId(id),
        recipe: "base:train_rifleman".to_string(),
        stage: JobStage::Running,
        duration_ticks: duration,
        charged: vec![],
        reserved: vec![],
    }
}

fn held(content: &str, x: f32) -> HeldOrder {
    HeldOrder {
        content: format!("base:{content}"),
        point: Pos { x, y: 5.0 },
    }
}

fn supply(provided: u64, consumed: u64) -> SupplyRecord {
    SupplyRecord {
        slot: SlotId(0),
        resource: POWER.to_string(),
        provided,
        consumed,
    }
}

fn outcome(outcome: Outcome, reason: OutcomeReason, tick: u32) -> SlotOutcome {
    SlotOutcome {
        outcome,
        reason,
        tick,
    }
}

/// A Snapshot at `tick`: an own producer (with 1 job and its record), an own constructor, an enemy
/// unit, an own site (with its progress) and an enemy site; the records of the other tables.
fn snapshot(tick: u32) -> Snapshot {
    let mut snapshot = Snapshot {
        tick,
        units: vec![
            unit(OWN, "command_center", 0, false),
            unit(CONSTRUCTOR, "rifleman", 0, false),
            unit(ENEMY, "command_center", 1, false),
            unit(SITE, "command_center", 0, true),
            unit(ENEMY_SITE, "command_center", 1, true),
        ],
        economy: turts_net_protocol::EconomyState {
            balances: vec![BalanceState {
                resource: "base:credits".to_string(),
                on_hand: 500,
                reserved: 0,
            }],
            inventories: vec![],
            orders: vec![],
            queues: vec![QueueState {
                producer: UnitId(OWN),
                jobs: vec![job_state(44, 100)],
                enabled: true,
            }],
        },
        ..Default::default()
    };
    snapshot.records.jobs = vec![JobProgress {
        producer: UnitId(OWN),
        job: JobId(44),
        progress: rate(10, 100, 1, 10),
    }];
    snapshot.records.sites = vec![SiteProgress {
        unit: UnitId(SITE),
        progress: rate(5, 100, 1, 10),
    }];
    snapshot.records.construction_queues = vec![ConstructionQueue {
        constructor: UnitId(CONSTRUCTOR),
        orders: vec![held("command_center", 5.0), held("command_center", 6.0)],
    }];
    snapshot.records.supply = vec![supply(100, 40)];
    snapshot.records.match_state = Some(MatchState {
        connections: vec![
            SlotConnection {
                slot: SlotId(0),
                connected: true,
                since_tick: 1,
            },
            SlotConnection {
                slot: SlotId(1),
                connected: true,
                since_tick: 1,
            },
        ],
        outcomes: vec![None, None],
    });
    snapshot
}

fn delta(tick: u32) -> WorldDelta {
    WorldDelta {
        tick,
        base_tick: tick - 1,
        ..Default::default()
    }
}

fn applied(replica: &mut Replica, snapshot: &Snapshot) {
    assert_eq!(
        replica.apply_snapshot(snapshot, Instant::now()),
        SnapshotOutcome::Applied { replayed: 0 }
    );
}

/// The contract: every getter returns the record values of the Snapshot and then of a delta.
#[test]
fn the_getters_return_the_records_of_a_snapshot_and_a_delta() {
    let _guard = serial();
    let now = Instant::now();
    let mut replica = replica();
    applied(&mut replica, &snapshot(10));

    // The Snapshot.
    assert_eq!(replica.store().tick(), 10);
    assert_eq!(replica.supply(POWER, SupplySide::Provided), Some(100));
    assert_eq!(replica.supply(POWER, SupplySide::Consumed), Some(40));
    assert_eq!(replica.supply("base:credits", SupplySide::Provided), None);
    assert_eq!(
        replica.construction_queue(CONSTRUCTOR),
        &[held("command_center", 5.0), held("command_center", 6.0)]
    );
    assert!(replica.construction_queue(OWN).is_empty());
    assert_eq!(replica.sites().collect::<Vec<_>>(), vec![SITE, ENEMY_SITE]);
    assert!(replica.is_site(SITE) && !replica.is_site(OWN));
    assert_eq!(replica.site_progress(SITE), Some(&rate(5, 100, 1, 10)));
    assert_eq!(replica.site_progress(ENEMY_SITE), None);
    assert_eq!(replica.job_progress(OWN, 44), Some(&rate(10, 100, 1, 10)));
    assert_eq!(replica.job_elapsed_ticks(OWN, 44), 10);
    assert_eq!(replica.own_outcome(), None);
    assert_eq!(replica.outcome(), None);
    assert_eq!(replica.outcome_reason(), None);
    assert_eq!(replica.match_clock_ticks(4), 6);

    // One delta: ticks 11..=15 move the values, the outcome arrives.
    for tick in 11..=14 {
        assert_eq!(
            replica.apply_delta(&delta(tick), now),
            DeltaOutcome::Applied
        );
    }
    let mut change = delta(15);
    change.records.supply = vec![supply(120, 130)];
    change.records.sites = vec![SiteProgress {
        unit: UnitId(SITE),
        progress: rate(30, 100, 2, 15),
    }];
    change.records.construction_queues = vec![ConstructionQueue {
        constructor: UnitId(CONSTRUCTOR),
        orders: vec![held("command_center", 7.0)],
    }];
    change.records.match_state = Some(MatchState {
        connections: vec![],
        outcomes: vec![
            Some(outcome(Outcome::Defeat, OutcomeReason::KeyUnitsLost, 15)),
            Some(outcome(
                Outcome::Victory,
                OutcomeReason::OpponentsDefeated,
                15,
            )),
        ],
    });
    assert_eq!(replica.apply_delta(&change, now), DeltaOutcome::Applied);
    assert_eq!(replica.store().tick(), 15);
    assert_eq!(replica.supply(POWER, SupplySide::Provided), Some(120));
    assert_eq!(replica.supply(POWER, SupplySide::Consumed), Some(130));
    assert_eq!(
        replica.construction_queue(CONSTRUCTOR),
        &[held("command_center", 7.0)]
    );
    assert_eq!(replica.site_progress(SITE), Some(&rate(30, 100, 2, 15)));
    // The job record of tick 10 is evaluated at the last applied tick, 15.
    assert_eq!(replica.job_elapsed_ticks(OWN, 44), 15);
    assert_eq!(replica.job_value(OWN, 44), Some(15));
    assert_eq!(replica.site_value(SITE), Some(30));
    assert_eq!(replica.own_outcome().map(|o| o.tick), Some(15));
    assert_eq!(replica.outcome(), Some(Outcome::Defeat));
    assert_eq!(replica.outcome_reason(), Some(OutcomeReason::KeyUnitsLost));
    assert_eq!(replica.match_state().connections.len(), 0);
    assert_eq!(replica.match_clock_ticks(4), 11);

    // The item fields `Paid` and `Refund` go through `turts_rules::progress`: a job of cost 200 at
    // value 15 of 100 has paid 30; a running job refunds 1/2 of that.
    assert_eq!(replica.job_paid(OWN, 44, 200), Some(30));
    let refunds = CancelRefunds {
        running: Ratio::new(1, 2).unwrap(),
        output_blocked: Ratio::ONE,
        destroyed: Ratio::ZERO,
    };
    assert_eq!(
        replica.job_refund(OWN, 44, 200, CancelCase::Running, &refunds),
        Some(15)
    );
    assert_eq!(
        replica.job_refund(OWN, 44, 200, CancelCase::HeldOrQueued, &refunds),
        Some(0)
    );
    assert_eq!(replica.site_paid(SITE, 100), Some(30));
    assert_eq!(
        replica.site_refund(SITE, 100, CancelCase::Destroyed, &refunds),
        Some(0)
    );
    assert_eq!(replica.job_paid(OWN, 99, 200), None);
}

/// The binding ruling for `RateRecord`: `done`, `total` and `per_tick` count raw units of 1/Q
/// tick, and the client derives ticks with `base_ticks_done`. Q = 2: a job of 100 ticks has
/// `total` 200; at rate 1/2 `per_tick` is 1, so 20 ticks give 20 raw units and 10 ticks.
#[test]
fn the_elapsed_ticks_of_a_job_are_derived_from_raw_units_of_one_over_q_tick() {
    let _guard = serial();
    let now = Instant::now();
    let mut replica = replica();
    let mut first = snapshot(10);
    first.records.jobs[0].progress = rate(0, 200, 1, 10);
    applied(&mut replica, &first);
    for tick in 11..=30 {
        assert_eq!(
            replica.apply_delta(&delta(tick), now),
            DeltaOutcome::Applied
        );
    }
    assert_eq!(replica.job_value(OWN, 44), Some(20), "raw units");
    assert_eq!(replica.job_elapsed_ticks(OWN, 44), 10, "ticks of base time");
}

/// A record for a job that no queue holds is dropped as the message applies, and so is a record
/// of a job that the queue holds only in a later message.
#[test]
fn a_job_record_for_a_job_that_no_queue_holds_is_dropped() {
    let _guard = serial();
    let mut replica = replica();
    applied(&mut replica, &snapshot(10));
    let mut orphan = delta(11);
    orphan.records.jobs = vec![JobProgress {
        producer: UnitId(OWN),
        job: JobId(99),
        progress: rate(1, 100, 1, 11),
    }];
    assert_eq!(
        replica.apply_delta(&orphan, Instant::now()),
        DeltaOutcome::Applied
    );
    assert_eq!(
        replica.job_progress(OWN, 99),
        None,
        "job_progress: a record of a job that no queue holds stays"
    );
    assert!(replica.job_progress(OWN, 44).is_some());
}

/// A job that waits for funds, and a held record, keep their value (6.4, E21) whatever `per_tick`
/// says.
#[test]
fn a_waiting_or_held_job_keeps_its_value() {
    let _guard = serial();
    let now = Instant::now();
    for stalled in [
        |record: &mut RateRecord| {
            record.state = RecordState::WaitingForFunds("base:credits".into())
        },
        |record: &mut RateRecord| {
            record.held = true;
            record.held_since_tick = 10;
        },
    ] {
        let mut replica = replica();
        let mut first = snapshot(10);
        first.records.jobs[0].progress = rate(10, 100, 1, 10);
        stalled(&mut first.records.jobs[0].progress);
        applied(&mut replica, &first);
        for tick in 11..=20 {
            assert_eq!(
                replica.apply_delta(&delta(tick), now),
                DeltaOutcome::Applied
            );
        }
        assert_eq!(
            replica.job_elapsed_ticks(OWN, 44),
            10,
            "a waiting or held job keeps its value"
        );
    }
}

/// A Snapshot that carries no match state reads as the empty state; a delta with `None` changes
/// nothing (ruling for `L0-STORE`).
#[test]
fn a_missing_match_state_reads_as_the_empty_state_and_a_delta_none_is_no_change() {
    let _guard = serial();
    let now = Instant::now();
    let mut replica = replica();
    let mut first = snapshot(10);
    first.records.match_state = None;
    applied(&mut replica, &first);
    assert_eq!(replica.match_state(), &MatchState::default());
    let mut set = delta(11);
    set.records.match_state = Some(MatchState {
        connections: vec![],
        outcomes: vec![Some(outcome(
            Outcome::Draw,
            OutcomeReason::Disconnected,
            11,
        ))],
    });
    assert_eq!(replica.apply_delta(&set, now), DeltaOutcome::Applied);
    assert_eq!(replica.outcome(), Some(Outcome::Draw));
    assert_eq!(replica.apply_delta(&delta(12), now), DeltaOutcome::Applied);
    assert_eq!(replica.outcome(), Some(Outcome::Draw), "None is no change");
}

/// A delta that fails to apply leaves the candidate and the store at the old tick. The delta has a
/// valid store change and fails on its economy, after the store change is staged.
#[test]
fn a_failed_delta_leaves_both_the_candidate_and_the_store_at_the_old_tick() {
    let _guard = serial();
    let now = Instant::now();
    let mut replica = replica();
    applied(&mut replica, &snapshot(10));
    let before_store = replica.store().clone();
    let mut bad = delta(11);
    bad.records.supply = vec![supply(999, 999)];
    bad.records.construction_queues = vec![ConstructionQueue {
        constructor: UnitId(CONSTRUCTOR),
        orders: vec![],
    }];
    // `reserved > on_hand` is not a valid balance: the economy check fails after the merge.
    bad.economy.balances = vec![BalanceState {
        resource: "base:credits".to_string(),
        on_hand: 1,
        reserved: 2,
    }];
    assert_eq!(
        replica.apply_delta(&bad, now),
        DeltaOutcome::ResyncRequested
    );
    assert_eq!(replica.last_applied_tick, Some(10));
    assert_eq!(
        replica.store().tick(),
        10,
        "a failed delta: the store stays at the tick of the candidate"
    );
    assert_eq!(
        replica.store(),
        &before_store,
        "a failed delta: the store is unchanged"
    );
    assert_eq!(replica.supply(POWER, SupplySide::Provided), Some(100));
    assert_eq!(replica.construction_queue(CONSTRUCTOR).len(), 2);
}

/// `IgnoredStale` and `Discarded` change nothing.
#[test]
fn a_stale_snapshot_and_a_discarded_delta_change_nothing() {
    let _guard = serial();
    let now = Instant::now();
    let mut replica = replica();
    applied(&mut replica, &snapshot(20));
    let before_store = replica.store().clone();
    let mut stale = snapshot(15);
    stale.records.supply = vec![supply(1, 1)];
    assert_eq!(
        replica.apply_snapshot(&stale, now),
        SnapshotOutcome::IgnoredStale
    );
    let mut same_tick = snapshot(20);
    same_tick.records.supply = vec![supply(2, 2)];
    assert_eq!(
        replica.apply_snapshot(&same_tick, now),
        SnapshotOutcome::IgnoredStale
    );
    let mut old = delta(20);
    old.base_tick = 19;
    old.records.supply = vec![supply(3, 3)];
    assert_eq!(replica.apply_delta(&old, now), DeltaOutcome::Discarded);
    assert_eq!(replica.store(), &before_store);
    assert_eq!(replica.store().tick(), 20);
    assert_eq!(replica.supply(POWER, SupplySide::Provided), Some(100));
}

/// A Snapshot that is not valid changes nothing.
#[test]
fn an_invalid_snapshot_changes_nothing() {
    let _guard = serial();
    let mut replica = replica();
    applied(&mut replica, &snapshot(10));
    let before_store = replica.store().clone();
    let mut bad = snapshot(11);
    bad.records.supply = vec![SupplyRecord {
        slot: SlotId(1),
        ..supply(1, 1)
    }];
    assert_eq!(
        replica.apply_snapshot(&bad, Instant::now()),
        SnapshotOutcome::Invalid
    );
    assert_eq!(replica.store(), &before_store);
    assert_eq!(replica.last_applied_tick, Some(10));
}

/// Case (1): a resync where a retained delta fails. The Snapshot and the first retained delta are
/// valid and change the store; the second retained delta fails. The display and the store stay at
/// tick 10.
#[test]
fn a_resync_where_a_retained_delta_fails_leaves_the_store_unchanged() {
    let _guard = serial();
    let now = Instant::now();
    let mut replica = replica();
    applied(&mut replica, &snapshot(10));
    let before_store = replica.store().clone();
    // A gap: the delta of tick 12 does not extend tick 10.
    let mut twelve = delta(12);
    twelve.records.supply = vec![supply(112, 40)];
    assert_eq!(
        replica.apply_delta(&twelve, now),
        DeltaOutcome::ResyncRequested
    );
    let mut thirteen = delta(13);
    thirteen.records.supply = vec![supply(113, 40)];
    // The economy of this delta is not valid.
    thirteen.economy.balances = vec![BalanceState {
        resource: "base:credits".to_string(),
        on_hand: 1,
        reserved: 2,
    }];
    assert_eq!(replica.apply_delta(&thirteen, now), DeltaOutcome::Buffered);
    let mut resync = snapshot(11);
    resync.records.supply = vec![supply(111, 40)];
    assert_eq!(
        replica.apply_snapshot(&resync, now),
        SnapshotOutcome::ResyncRequested { replayed: 0 }
    );
    assert_eq!(replica.last_applied_tick, Some(10));
    assert_eq!(
        replica.store().tick(),
        10,
        "case (1): the store stays at the tick of the candidate"
    );
    assert_eq!(
        replica.store(),
        &before_store,
        "case (1): the store is unchanged"
    );
    assert_eq!(replica.supply(POWER, SupplySide::Provided), Some(100));
}

/// Case (3): a resync with 2 replayed deltas. The store equals the candidate after the last one,
/// and equals the store of a client that followed every delta.
#[test]
fn a_resync_with_two_replayed_deltas_ends_with_the_store_of_a_continuous_client() {
    let _guard = serial();
    let now = Instant::now();
    let mut eleven = delta(11);
    eleven.records.supply = vec![supply(111, 41)];
    let mut twelve = delta(12);
    twelve.records.supply = vec![supply(112, 42)];
    twelve.records.sites = vec![SiteProgress {
        unit: UnitId(SITE),
        progress: rate(9, 100, 3, 12),
    }];
    let mut thirteen = delta(13);
    thirteen.records.supply = vec![supply(113, 43)];
    thirteen.records.construction_queues = vec![ConstructionQueue {
        constructor: UnitId(CONSTRUCTOR),
        orders: vec![],
    }];

    let mut continuous = replica();
    applied(&mut continuous, &snapshot(10));
    for step in [&eleven, &twelve, &thirteen] {
        assert_eq!(continuous.apply_delta(step, now), DeltaOutcome::Applied);
    }

    let mut resynced = replica();
    applied(&mut resynced, &snapshot(10));
    // The delta of tick 11 is lost. Ticks 12 and 13 are buffered, then the Snapshot of tick 11.
    assert_eq!(
        resynced.apply_delta(&twelve, now),
        DeltaOutcome::ResyncRequested
    );
    assert_eq!(resynced.apply_delta(&thirteen, now), DeltaOutcome::Buffered);
    let mut at_eleven = snapshot(11);
    at_eleven.records.supply = vec![supply(111, 41)];
    assert_eq!(
        resynced.apply_snapshot(&at_eleven, now),
        SnapshotOutcome::Applied { replayed: 2 }
    );
    assert_eq!(resynced.last_applied_tick, Some(13));
    assert_eq!(resynced.store().tick(), 13);
    assert_eq!(resynced.store(), continuous.store());
    assert_eq!(resynced.supply(POWER, SupplySide::Provided), Some(113));
    assert_eq!(resynced.site_progress(SITE), Some(&rate(9, 100, 3, 12)));
    assert!(resynced.construction_queue(CONSTRUCTOR).is_empty());
}

/// The store is not cloned for each delta: 100 deltas, each with a store change, clone it 0 times.
#[test]
fn a_hundred_deltas_clone_the_store_zero_times() {
    let _guard = serial();
    let now = Instant::now();
    let mut replica = replica();
    applied(&mut replica, &snapshot(10));
    let before = store_clone_count();
    for tick in 11..=110u32 {
        let mut change = delta(tick);
        change.records.supply = vec![supply(u64::from(tick), 40)];
        change.records.sites = vec![SiteProgress {
            unit: UnitId(SITE),
            progress: rate(u64::from(tick), 1000, 1, tick),
        }];
        assert_eq!(replica.apply_delta(&change, now), DeltaOutcome::Applied);
    }
    assert_eq!(
        store_clone_count() - before,
        0,
        "the apply path clones the store"
    );
    // The deltas did change the store.
    assert_eq!(replica.supply(POWER, SupplySide::Provided), Some(110));
    assert_eq!(replica.store().tick(), 110);
    // The counter counts: one explicit clone adds one.
    let _copy = replica.store().clone();
    assert_eq!(store_clone_count() - before, 1);
}

/// The removal rule of each table (see the module doc of `replica_store`): what removes an entry
/// when its unit is removed.
#[test]
fn each_table_drops_its_entry_when_its_unit_goes() {
    let _guard = serial();
    let now = Instant::now();
    let fresh = || {
        let mut replica = replica();
        applied(&mut replica, &snapshot(10));
        replica
    };

    // `sites` and `site_progress`: the unit is removed.
    let mut replica = fresh();
    let mut gone = delta(11);
    gone.removed = vec![UnitId(SITE), UnitId(ENEMY_SITE)];
    assert_eq!(replica.apply_delta(&gone, now), DeltaOutcome::Applied);
    assert_eq!(replica.sites().count(), 0, "sites: a removed unit stays");
    assert_eq!(
        replica.site_progress(SITE),
        None,
        "site_progress: a removed unit stays"
    );

    // `sites` and `site_progress`: the site completes.
    let mut replica = fresh();
    let mut done = delta(11);
    done.records.sites_completed = vec![UnitId(SITE)];
    assert_eq!(replica.apply_delta(&done, now), DeltaOutcome::Applied);
    assert_eq!(
        replica.sites().collect::<Vec<_>>(),
        vec![ENEMY_SITE],
        "sites: a completed site stays"
    );
    assert_eq!(
        replica.site_progress(SITE),
        None,
        "site_progress: a completed site stays"
    );

    // `construction_queues`: the constructor is removed; an empty record also clears it.
    let mut replica = fresh();
    let mut gone = delta(11);
    gone.removed = vec![UnitId(CONSTRUCTOR)];
    assert_eq!(replica.apply_delta(&gone, now), DeltaOutcome::Applied);
    assert!(
        replica.construction_queue(CONSTRUCTOR).is_empty(),
        "construction_queues: a removed constructor stays"
    );
    assert_eq!(
        replica.store().record_count(),
        2,
        "the job and the site remain"
    );
    let mut replica = fresh();
    let mut cleared = delta(11);
    cleared.records.construction_queues = vec![ConstructionQueue {
        constructor: UnitId(CONSTRUCTOR),
        orders: vec![],
    }];
    assert_eq!(replica.apply_delta(&cleared, now), DeltaOutcome::Applied);
    assert!(
        replica.construction_queue(CONSTRUCTOR).is_empty(),
        "construction_queues: an empty record stays"
    );

    // `job_progress`: the producer is removed.
    let mut replica = fresh();
    let mut gone = delta(11);
    gone.removed = vec![UnitId(OWN)];
    assert_eq!(replica.apply_delta(&gone, now), DeltaOutcome::Applied);
    assert_eq!(
        replica.job_progress(OWN, 44),
        None,
        "job_progress: the record of a removed producer stays"
    );
    assert_eq!(replica.store().job_keys().count(), 0);

    // `job_progress`: the queue no longer holds the job.
    let mut replica = fresh();
    let mut finished = delta(11);
    finished.economy.queues = vec![QueueState {
        producer: UnitId(OWN),
        jobs: vec![],
        enabled: true,
    }];
    assert_eq!(replica.apply_delta(&finished, now), DeltaOutcome::Applied);
    assert_eq!(
        replica.job_progress(OWN, 44),
        None,
        "job_progress: the record of a job that the queue no longer holds stays"
    );

    // `supply` and `match_state` name slots, never units: a removal does not touch them.
    let mut replica = fresh();
    let mut gone = delta(11);
    gone.removed = vec![UnitId(OWN), UnitId(CONSTRUCTOR), UnitId(ENEMY)];
    assert_eq!(replica.apply_delta(&gone, now), DeltaOutcome::Applied);
    assert_eq!(replica.supply(POWER, SupplySide::Provided), Some(100));
    assert_eq!(replica.match_state().connections.len(), 2);
}

/// A record that names a unit that the same message removes, or a unit that is not valid for the
/// record, is refused: the delta asks for a resync and the store stays.
#[test]
fn a_record_that_names_a_gone_or_wrong_unit_is_refused() {
    let _guard = serial();
    let now = Instant::now();
    type Edit = Box<dyn Fn(&mut WorldDelta)>;
    let cases: Vec<(&str, Edit)> = vec![
        (
            "site progress of a removed site",
            Box::new(|d| {
                d.removed = vec![UnitId(SITE)];
                d.records.sites = vec![SiteProgress {
                    unit: UnitId(SITE),
                    progress: rate(1, 100, 1, 11),
                }];
            }),
        ),
        (
            "site progress of a unit that is not a site",
            Box::new(|d| {
                d.records.sites = vec![SiteProgress {
                    unit: UnitId(OWN),
                    progress: rate(1, 100, 1, 11),
                }];
            }),
        ),
        (
            "site progress of an enemy site",
            Box::new(|d| {
                d.records.sites = vec![SiteProgress {
                    unit: UnitId(ENEMY_SITE),
                    progress: rate(1, 100, 1, 11),
                }];
            }),
        ),
        (
            "site progress twice",
            Box::new(|d| {
                let row = SiteProgress {
                    unit: UnitId(SITE),
                    progress: rate(1, 100, 1, 11),
                };
                d.records.sites = vec![row.clone(), row];
            }),
        ),
        (
            "completion of a unit that is not a site",
            Box::new(|d| d.records.sites_completed = vec![UnitId(OWN)]),
        ),
        (
            "completion of a removed site",
            Box::new(|d| {
                d.removed = vec![UnitId(SITE)];
                d.records.sites_completed = vec![UnitId(SITE)];
            }),
        ),
        (
            "completion twice",
            Box::new(|d| d.records.sites_completed = vec![UnitId(SITE), UnitId(SITE)]),
        ),
        (
            "construction queue of a removed constructor",
            Box::new(|d| {
                d.removed = vec![UnitId(CONSTRUCTOR)];
                d.records.construction_queues = vec![ConstructionQueue {
                    constructor: UnitId(CONSTRUCTOR),
                    orders: vec![held("command_center", 1.0)],
                }];
            }),
        ),
        (
            "construction queue of an unknown unit",
            Box::new(|d| {
                d.records.construction_queues = vec![ConstructionQueue {
                    constructor: UnitId(777),
                    orders: vec![],
                }];
            }),
        ),
        (
            "construction queue of an enemy unit",
            Box::new(|d| {
                d.records.construction_queues = vec![ConstructionQueue {
                    constructor: UnitId(ENEMY),
                    orders: vec![],
                }];
            }),
        ),
        (
            "held order with a point that is not finite",
            Box::new(|d| {
                d.records.construction_queues = vec![ConstructionQueue {
                    constructor: UnitId(CONSTRUCTOR),
                    orders: vec![HeldOrder {
                        content: "base:command_center".to_string(),
                        point: Pos {
                            x: f32::NAN,
                            y: 1.0,
                        },
                    }],
                }];
            }),
        ),
        (
            "held order with an unknown content",
            Box::new(|d| {
                d.records.construction_queues = vec![ConstructionQueue {
                    constructor: UnitId(CONSTRUCTOR),
                    orders: vec![held("no_such_unit", 1.0)],
                }];
            }),
        ),
        (
            "supply of another slot",
            Box::new(|d| {
                d.records.supply = vec![SupplyRecord {
                    slot: SlotId(1),
                    ..supply(1, 1)
                }];
            }),
        ),
        (
            "supply of a resource that is not a Supply resource",
            Box::new(|d| {
                d.records.supply = vec![SupplyRecord {
                    resource: "base:credits".to_string(),
                    ..supply(1, 1)
                }];
            }),
        ),
        (
            "supply of an unknown resource",
            Box::new(|d| {
                d.records.supply = vec![SupplyRecord {
                    resource: "base:nothing".to_string(),
                    ..supply(1, 1)
                }];
            }),
        ),
        (
            "supply twice",
            Box::new(|d| d.records.supply = vec![supply(1, 1), supply(2, 2)]),
        ),
        (
            "match state with a slot twice",
            Box::new(|d| {
                let row = SlotConnection {
                    slot: SlotId(0),
                    connected: true,
                    since_tick: 1,
                };
                d.records.match_state = Some(MatchState {
                    connections: vec![row, row],
                    outcomes: vec![],
                });
            }),
        ),
        (
            "job progress of a removed producer",
            Box::new(|d| {
                d.removed = vec![UnitId(OWN)];
                d.records.jobs = vec![JobProgress {
                    producer: UnitId(OWN),
                    job: JobId(44),
                    progress: rate(1, 100, 1, 11),
                }];
            }),
        ),
    ];
    for (name, edit) in cases {
        let mut replica = replica();
        applied(&mut replica, &snapshot(10));
        let before_store = replica.store().clone();
        let mut bad = delta(11);
        edit(&mut bad);
        assert_eq!(
            replica.apply_delta(&bad, now),
            DeltaOutcome::ResyncRequested,
            "{name}"
        );
        assert_eq!(replica.store(), &before_store, "{name}");
        assert_eq!(replica.last_applied_tick, Some(10), "{name}");
    }
}

/// A site that completes in the delta that carries its record: the record goes with the site.
#[test]
fn the_record_of_a_site_that_completes_in_the_same_delta_is_dropped() {
    let _guard = serial();
    let mut replica = replica();
    applied(&mut replica, &snapshot(10));
    let mut done = delta(11);
    done.records.sites = vec![SiteProgress {
        unit: UnitId(SITE),
        progress: rate(100, 100, 0, 11),
    }];
    done.records.sites_completed = vec![UnitId(SITE)];
    assert_eq!(
        replica.apply_delta(&done, Instant::now()),
        DeltaOutcome::Applied
    );
    assert!(!replica.is_site(SITE));
    assert_eq!(replica.site_progress(SITE), None);
}

/// A site that spawns in a delta is a site, with its progress, in the same delta.
#[test]
fn a_site_spawned_by_a_delta_is_a_site_with_its_record() {
    let _guard = serial();
    let mut replica = replica();
    applied(&mut replica, &snapshot(10));
    let mut spawn = delta(11);
    spawn.spawned = vec![unit(30, "command_center", 0, true)];
    spawn.records.sites = vec![SiteProgress {
        unit: UnitId(30),
        progress: rate(0, 50, 1, 11),
    }];
    assert_eq!(
        replica.apply_delta(&spawn, Instant::now()),
        DeltaOutcome::Applied
    );
    assert!(replica.is_site(30));
    assert_eq!(replica.site_progress(30), Some(&rate(0, 50, 1, 11)));
}

// The three-client pattern (job_progress_clients.rs) for every table: a truth model, a Snapshot
// built from it at any tick, and the deltas of a script that touches each table.

#[derive(Clone)]
struct Truth {
    tick: u32,
    units: BTreeMap<u32, UnitSnapshot>,
    jobs: Vec<(u64, RateRecord)>,
    sites: BTreeMap<u32, RateRecord>,
    queues: BTreeMap<u32, Vec<HeldOrder>>,
    supply: Option<SupplyRecord>,
    match_state: MatchState,
}

impl Truth {
    fn new() -> Self {
        let first = snapshot(10);
        Truth {
            tick: 10,
            units: first
                .units
                .iter()
                .map(|u| (u.unit_id.0, u.clone()))
                .collect(),
            jobs: vec![(44, rate(10, 100, 1, 10))],
            sites: BTreeMap::from([(SITE, rate(5, 100, 1, 10))]),
            queues: BTreeMap::from([(
                CONSTRUCTOR,
                vec![held("command_center", 5.0), held("command_center", 6.0)],
            )]),
            supply: Some(supply(100, 40)),
            match_state: first.records.match_state.unwrap(),
        }
    }

    /// The Snapshot of the state of this truth at its tick.
    fn snapshot(&self) -> Snapshot {
        let mut snapshot = Snapshot {
            tick: self.tick,
            units: self.units.values().cloned().collect(),
            economy: turts_net_protocol::EconomyState {
                balances: vec![BalanceState {
                    resource: "base:credits".to_string(),
                    on_hand: 500,
                    reserved: 0,
                }],
                inventories: vec![],
                orders: vec![],
                queues: if self.units.contains_key(&OWN) {
                    vec![QueueState {
                        producer: UnitId(OWN),
                        jobs: self
                            .jobs
                            .iter()
                            .map(|(id, _)| job_state(*id, 100))
                            .collect(),
                        enabled: true,
                    }]
                } else {
                    vec![]
                },
            },
            ..Default::default()
        };
        if self.units.contains_key(&OWN) {
            snapshot.records.jobs = self
                .jobs
                .iter()
                .map(|(id, progress)| JobProgress {
                    producer: UnitId(OWN),
                    job: JobId(*id),
                    progress: progress.clone(),
                })
                .collect();
        }
        snapshot.records.sites = self
            .sites
            .iter()
            .map(|(unit, progress)| SiteProgress {
                unit: UnitId(*unit),
                progress: progress.clone(),
            })
            .collect();
        snapshot.records.construction_queues = self
            .queues
            .iter()
            .map(|(constructor, orders)| ConstructionQueue {
                constructor: UnitId(*constructor),
                orders: orders.clone(),
            })
            .collect();
        snapshot.records.supply = self.supply.iter().cloned().collect();
        snapshot.records.match_state = Some(self.match_state.clone());
        snapshot
    }

    /// Advance the truth by one tick of the script and return the delta of that tick.
    fn step(&mut self) -> WorldDelta {
        self.tick += 1;
        let tick = self.tick;
        let mut d = delta(tick);
        match tick {
            12 => {
                let unit = unit(40, "command_center", 0, true);
                self.units.insert(40, unit.clone());
                d.spawned = vec![unit];
                self.sites.insert(40, rate(0, 80, 1, 12));
                d.records.sites = vec![SiteProgress {
                    unit: UnitId(40),
                    progress: rate(0, 80, 1, 12),
                }];
            }
            14 => {
                let orders = vec![held("command_center", 9.0)];
                self.queues.insert(CONSTRUCTOR, orders.clone());
                d.records.construction_queues = vec![ConstructionQueue {
                    constructor: UnitId(CONSTRUCTOR),
                    orders,
                }];
            }
            16 => {
                self.supply = Some(supply(120, 130));
                d.records.supply = vec![supply(120, 130)];
            }
            20 => {
                let progress = rate(21, 100, 2, 20);
                self.sites.insert(SITE, progress.clone());
                d.records.sites = vec![SiteProgress {
                    unit: UnitId(SITE),
                    progress,
                }];
            }
            24 => {
                self.match_state.outcomes = vec![
                    None,
                    Some(outcome(Outcome::Defeat, OutcomeReason::Surrendered, 24)),
                ];
                d.records.match_state = Some(self.match_state.clone());
            }
            30 => {
                self.sites.remove(&SITE);
                // The completed site stays a unit; it is no longer a site.
                self.units.get_mut(&SITE).unwrap().site = false;
                d.records.sites_completed = vec![UnitId(SITE)];
            }
            33 => {
                self.queues.remove(&CONSTRUCTOR);
                d.records.construction_queues = vec![ConstructionQueue {
                    constructor: UnitId(CONSTRUCTOR),
                    orders: vec![],
                }];
            }
            36 => {
                self.units.remove(&40);
                self.sites.remove(&40);
                self.units.remove(&ENEMY_SITE);
                d.removed = vec![UnitId(40), UnitId(ENEMY_SITE)];
            }
            40 => {
                self.jobs.clear();
                d.economy.queues = vec![QueueState {
                    producer: UnitId(OWN),
                    jobs: vec![],
                    enabled: true,
                }];
            }
            44 => {
                self.units.remove(&CONSTRUCTOR);
                self.queues.remove(&CONSTRUCTOR);
                d.removed = vec![UnitId(CONSTRUCTOR)];
            }
            _ => {}
        }
        d
    }
}

/// Three clients: one follows every delta, one joins at a Snapshot, one loses a run of deltas and
/// then gets an unrequested Snapshot. At every tick the store of each equals the store of the
/// continuous client, and equals what the truth holds.
#[test]
fn a_late_join_and_a_lossy_client_end_with_the_store_of_a_continuous_client() {
    let _guard = serial();
    let now = Instant::now();
    let late = 22u32;
    let lost = 26u32..=31;
    let resync_at = lost.end() + 1;
    let mut truth = Truth::new();
    let mut continuous = replica();
    let mut joined = replica();
    let mut lossy = replica();
    applied(&mut continuous, &truth.snapshot());
    applied(&mut lossy, &truth.snapshot());
    let mut compared = 0;
    for _ in 11..=50u32 {
        let step = truth.step();
        let tick = truth.tick;
        let snapshot = truth.snapshot();
        assert_eq!(
            continuous.apply_delta(&step, now),
            DeltaOutcome::Applied,
            "tick {tick}"
        );
        if tick == late {
            applied(&mut joined, &snapshot);
        } else if tick > late {
            assert_eq!(
                joined.apply_delta(&step, now),
                DeltaOutcome::Applied,
                "tick {tick}"
            );
        }
        if lost.contains(&tick) {
            // The deltas are lost.
        } else if tick == resync_at {
            applied(&mut lossy, &snapshot);
        } else {
            assert_eq!(
                lossy.apply_delta(&step, now),
                DeltaOutcome::Applied,
                "tick {tick}"
            );
        }
        // The continuous client holds what the truth holds: the store of a Snapshot of the truth.
        let mut fresh = replica();
        applied(&mut fresh, &snapshot);
        for (name, client, active) in [
            ("continuous", &continuous, true),
            ("late join", &joined, tick >= late),
            ("lossy", &lossy, !lost.contains(&tick)),
        ] {
            if !active {
                continue;
            }
            compared += 1;
            assert_eq!(
                client.last_applied_tick,
                Some(tick),
                "{name} at tick {tick}"
            );
            assert_eq!(client.store().tick(), tick, "{name} at tick {tick}");
            assert_eq!(
                client.store(),
                fresh.store(),
                "{name} at tick {tick}: the store differs from the store of a Snapshot of the truth"
            );
        }
    }
    assert!(compared > 100, "compared {compared}");
    // The script ended with these facts.
    assert_eq!(continuous.sites().count(), 0, "no site is left");
    assert_eq!(continuous.supply(POWER, SupplySide::Consumed), Some(130));
    assert_eq!(continuous.outcome(), None, "slot 0 has no outcome");
    assert_eq!(
        continuous.match_state().outcomes[1].map(|o| o.reason),
        Some(OutcomeReason::Surrendered)
    );
    assert!(continuous.construction_queue(CONSTRUCTOR).is_empty());
    assert_eq!(continuous.store().job_keys().count(), 0);
}
