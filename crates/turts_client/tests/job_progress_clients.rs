//! L0-WIRE repair 2: the derived job progress of a client equals the sim at every tick.
//!
//! The wire carries the progress of a job as a rate record (`JobProgress`: `done`, `total`,
//! `per_tick`, `at_tick`), sent on change, and the client derives the elapsed ticks from it
//! (`Replica::job_elapsed_ticks`). The rule: the wire rate equals the rate that the sim applies in
//! the tick. This test runs the real sim and the real export, and applies the stream to real
//! `Replica` values for three clients: one that follows every delta, one that joins at a
//! Snapshot of tick N, and one that loses a run of deltas and then gets an unrequested Snapshot.
//! At every tick, for every job of the owner, the client value is the sim value.
//!
//! Stalled conditions (the sim does not advance a job whose stage is `Running` or later):
//! - a disabled producer, machine or research station keeps the stage `Running` and does not
//!   advance (`production.rs`, `machine.rs`, `research.rs`): scenarios `base` and `forge`;
//! - a job behind another job in the queue has the stage `Queued` and does not advance: `base`
//!   (four jobs in one queue);
//! - the tick in which a job starts (`Queued` to `Running`) does not advance: every start;
//! - a job whose output is blocked has the stage `OutputBlocked` and does not advance: the stage
//!   rule is a unit test of `job_step_per_tick` (`economy::production::tests`), because no
//!   package here can block an output;
//! - a producer without power (`L-PWR`) and a held job (`L-PAY`) have no writer yet: when they
//!   arrive they change `job_step_per_tick` (the rate) and the view, and this test is the one
//!   that must stay green;
//! - a machine with a missing input has no job, so it has no progress to derive.
//!
//! Where it lives: the test needs the real export (`turts_server/src/export.rs`, included by path)
//! and the real `Replica`. A dev-dependency of `turts_server` on `turts_client` unified the bevy
//! features of the server binary that the process tests start, and `mc_kill_replica` then lost
//! deltas under load (2 runs in 12 failed, against 0 in 12 without the edge). So the test sits in
//! the client crate, with dev-dependencies on `turts_sim` and `turts_pack`; the boundary rules read
//! the normal and build edges only.

#[path = "../../turts_server/src/export.rs"]
mod export;

use std::collections::BTreeMap;
use std::path::Path;

use bevy::platform::time::Instant;
use turts_client::content::ClientContent;
use turts_client::replica::{DeltaOutcome, Replica, ReplicaContext, SnapshotOutcome};
use turts_mod_runtime::archive::PackageArchive;
use turts_mod_runtime::pack::pack_dir;
use turts_net_protocol::{Snapshot, WorldDelta};
use turts_sim::economy::events::{committed_tick, completed_world};
use turts_sim::{
    Command, CommandKind, CommandQueue, SlotId, WorldView, headless_app, initialize_completed_world,
};

struct Tick {
    tick: u32,
    delta: WorldDelta,
    /// The elapsed ticks of every job of slot 0 in the sim.
    truth: BTreeMap<(u32, u64), u64>,
    /// Some job of slot 0 is `Running` in the sim but frozen by a disabled producer.
    frozen_running: bool,
}

struct Run {
    snapshot0: Snapshot,
    ticks: Vec<Tick>,
    snapshots: BTreeMap<u32, Snapshot>,
    archive: PackageArchive,
}

fn package(name: &str) -> PackageArchive {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mods")
        .join(name);
    PackageArchive::parse(&pack_dir(&dir).unwrap()).unwrap()
}

fn run(
    name: &str,
    ticks: u32,
    mut script: impl FnMut(&mut bevy::app::App, u32, &WorldView, &mut u32),
) -> Run {
    let archive = package(name);
    let mut app = headless_app(20);
    turts_pack::load_package(app.world_mut(), &archive).unwrap();
    initialize_completed_world(app.world_mut());
    let view0 = completed_world(app.world()).clone();
    let snapshot0 = export::snapshot_for(&view0, SlotId(0));
    let mut seq = 0u32;
    let mut out = Vec::new();
    let mut snapshots = BTreeMap::new();
    for tick in 1..=ticks {
        let before = completed_world(app.world()).clone();
        script(&mut app, tick, &before, &mut seq);
        app.update();
        let batch = committed_tick(app.world()).clone();
        let view = completed_world(app.world()).clone();
        let delta = export::delta_for(&batch, &view, SlotId(0));
        let truth = view
            .economy
            .queues
            .iter()
            .filter(|queue| queue.owner_slot == 0)
            .flat_map(|queue| {
                queue
                    .jobs
                    .iter()
                    .map(move |job| ((queue.producer, job.id), job.elapsed_ticks))
            })
            .collect();
        let frozen_running = view.economy.queues.iter().any(|queue| {
            queue.owner_slot == 0
                && view.m3.disabled.binary_search(&queue.producer).is_ok()
                && queue
                    .jobs
                    .iter()
                    .any(|job| job.stage == turts_sim::JobStage::Running)
        });
        snapshots.insert(tick, export::snapshot_for(&view, SlotId(0)));
        out.push(Tick {
            tick,
            delta,
            truth,
            frozen_running,
        });
    }
    Run {
        snapshot0,
        ticks: out,
        snapshots,
        archive,
    }
}

fn push(app: &mut bevy::app::App, seq: &mut u32, kind: CommandKind) {
    *seq += 1;
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: *seq,
            kind,
        });
}

fn replica(archive: &PackageArchive) -> Replica {
    let content = ClientContent::from_archives(std::slice::from_ref(archive)).unwrap();
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(content, 0));
    replica
}

/// Check the three clients at every tick. `late` is the tick of the Snapshot that the late client
/// joins at; the lossy client loses the deltas `lost`, sees the delta after them (a gap: it asks for
/// a resync and buffers the next three deltas), and then takes the Snapshot of the second tick after
/// the loss with the retained deltas to replay (it replays 2 and skips 2).
fn check(run: &Run, late: u32, lost: std::ops::RangeInclusive<u32>) -> (usize, usize) {
    let now = Instant::now();
    let mut continuous = replica(&run.archive);
    let mut joined = replica(&run.archive);
    let mut lossy = replica(&run.archive);
    continuous.apply_snapshot(&run.snapshot0, now);
    lossy.apply_snapshot(&run.snapshot0, now);
    // The gap: the delta of `gap_first` does not extend the chain; the deltas up to `answer_after`
    // are retained; the Snapshot of `answer_tick` answers, and the deltas after it replay.
    let (gap_first, answer_tick, answer_after) = (lost.end() + 1, lost.end() + 2, lost.end() + 4);
    let mut replayed_seen = false;
    let mut compared = 0;
    let mut frozen_compared = 0;
    for entry in &run.ticks {
        let tick = entry.tick;
        continuous.apply_delta(&entry.delta, now);
        if tick == late {
            joined.apply_snapshot(&run.snapshots[&tick], now);
        } else if tick > late {
            joined.apply_delta(&entry.delta, now);
        }
        let lossy_active = if lost.contains(&tick) {
            // The deltas are lost.
            false
        } else if tick == gap_first {
            assert_eq!(
                lossy.apply_delta(&entry.delta, now),
                DeltaOutcome::ResyncRequested,
                "tick {tick}: the lossy client sees a gap"
            );
            false
        } else if tick > gap_first && tick <= answer_after {
            assert_eq!(
                lossy.apply_delta(&entry.delta, now),
                DeltaOutcome::Buffered,
                "tick {tick}"
            );
            if tick == answer_after {
                assert_eq!(
                    lossy.apply_snapshot(&run.snapshots[&answer_tick], now),
                    SnapshotOutcome::Applied { replayed: 2 },
                    "the Snapshot of tick {answer_tick} replays the retained deltas"
                );
                replayed_seen = true;
                true
            } else {
                false
            }
        } else {
            assert_eq!(
                lossy.apply_delta(&entry.delta, now),
                DeltaOutcome::Applied,
                "tick {tick}"
            );
            true
        };
        for (name, client, active) in [
            ("continuous", &continuous, true),
            ("late join", &joined, tick >= late),
            ("lossy", &lossy, lossy_active),
        ] {
            if !active {
                continue;
            }
            assert_eq!(
                client.last_applied_tick,
                Some(tick),
                "{name} at tick {tick}"
            );
            // The store commits with the candidate: the same tick, for each of the three clients.
            assert_eq!(
                client.store().tick(),
                tick,
                "{name} at tick {tick}: the store tick"
            );
            // The tables hold a record for exactly the jobs of the sim: none missing, none stale.
            assert_eq!(
                client.store().job_keys().collect::<Vec<_>>(),
                entry
                    .truth
                    .keys()
                    .map(|&(producer, job)| (producer, job))
                    .collect::<Vec<_>>(),
                "{name} at tick {tick}: the job record table"
            );
            for (&(producer, job), &elapsed) in &entry.truth {
                compared += 1;
                frozen_compared += usize::from(entry.frozen_running);
                assert_eq!(
                    client.job_elapsed_ticks(producer, job),
                    elapsed,
                    "{name} at tick {tick}: job {job} of producer {producer} (sim {elapsed}, a \
                     disabled producer holds a Running job: {})",
                    entry.frozen_running
                );
            }
        }
    }
    assert!(replayed_seen, "the lossy client replayed a resync");
    (compared, frozen_compared)
}

/// `base`: one Command Centre, 4 jobs (3 wait behind the first), disabled while the first runs,
/// enabled again, a cancel, a quick toggle, then to completion and output.
#[test]
fn base_producer_jobs_equal_the_sim_through_disable_enable_cancel() {
    let run = run("base", 700, |app, tick, view, seq| {
        let producer = view
            .units
            .iter()
            .find(|unit| unit.content_id == "base:command_center" && unit.owner_slot == 0)
            .unwrap()
            .unit_id;
        match tick {
            1 => {
                for _ in 0..4 {
                    push(
                        app,
                        seq,
                        CommandKind::Produce {
                            producer_id: producer,
                            recipe_id: turts_mod_api::ContentId::parse("base:train_rifleman")
                                .unwrap(),
                        },
                    );
                }
            }
            30 | 200 | 205 => push(
                app,
                seq,
                CommandKind::SetEnabled {
                    unit_id: producer,
                    enabled: false,
                },
            ),
            90 | 201 | 207 => push(
                app,
                seq,
                CommandKind::SetEnabled {
                    unit_id: producer,
                    enabled: true,
                },
            ),
            120 => {
                if let Some(job) = view
                    .economy
                    .queues
                    .iter()
                    .find(|queue| queue.producer == producer)
                    .and_then(|queue| queue.jobs.last())
                {
                    push(
                        app,
                        seq,
                        CommandKind::CancelProduction {
                            producer_id: producer,
                            job_id: turts_sim::JobId(job.id),
                        },
                    );
                }
            }
            _ => {}
        }
    });
    let (compared, frozen) = check(&run, 150, 240..=259);
    assert!(compared > 1500, "many comparisons: {compared}");
    assert!(
        frozen > 100,
        "the test compares ticks in which a disabled producer holds a Running job: {frozen}"
    );
    // Bytes of the queue and job records in a delta (0 when the lists are empty).
    let bytes = |tick: u32| {
        let delta = &run.ticks[tick as usize - 1].delta;
        let size = |len: usize, encoded: usize| if len == 0 { 0 } else { encoded };
        size(
            delta.economy.queues.len(),
            postcard::to_vec::<_, 4096>(&delta.economy.queues)
                .unwrap()
                .len(),
        ) + size(
            delta.records.jobs.len(),
            postcard::to_vec::<_, 4096>(&delta.records.jobs)
                .unwrap()
                .len(),
        )
    };
    let steady: Vec<usize> = (31..90).map(bytes).collect();
    assert!(
        steady.iter().all(|b| *b == 0),
        "a frozen job costs no bytes between the disable and the enable: {steady:?}"
    );
    let steady_running: Vec<usize> = (125..159).map(bytes).collect();
    assert!(
        steady_running.iter().all(|b| *b == 0),
        "a running job costs no bytes between its changes: {steady_running:?}"
    );
    eprintln!(
        "JOBPROGRESS bytes: disable tick 30 {} B, enable tick 90 {} B, ticks 31..89 (disabled) 0 B, ticks 125..158 (running) 0 B",
        bytes(30),
        bytes(90)
    );
    eprintln!(
        "JOBPROGRESS base: {compared} comparisons, {frozen} of them with a frozen Running job"
    );
}

/// `forge`: machines and research stations. Every owned unit with a queue is disabled for a while
/// and enabled again; the late client joins in the disabled window.
#[test]
fn forge_machine_and_research_jobs_equal_the_sim_through_disable_and_enable() {
    let run = run("forge", 400, |app, tick, view, seq| {
        let own_queues: Vec<u32> = view
            .economy
            .queues
            .iter()
            .filter(|queue| queue.owner_slot == 0)
            .map(|queue| queue.producer)
            .collect();
        match tick {
            60 | 220 => {
                for unit in own_queues {
                    push(
                        app,
                        seq,
                        CommandKind::SetEnabled {
                            unit_id: unit,
                            enabled: false,
                        },
                    );
                }
            }
            120 | 260 => {
                for unit in own_queues {
                    push(
                        app,
                        seq,
                        CommandKind::SetEnabled {
                            unit_id: unit,
                            enabled: true,
                        },
                    );
                }
            }
            _ => {}
        }
    });
    let (compared, frozen) = check(&run, 90, 150..=169);
    assert!(
        compared > 100,
        "the forge world has running jobs: {compared}"
    );
    assert!(
        frozen > 0,
        "a machine or station holds a Running job while disabled: {frozen}"
    );
    eprintln!(
        "JOBPROGRESS forge: {compared} comparisons, {frozen} of them with a frozen Running job"
    );
}

/// A real ordered attack through a kill (trooper 40 at dummy 46, the MM1 pair of
/// `mc_kill_replica`): every delta of the export applies to a real `Replica` of the owner, with no
/// resync. A delta that the replica rejects names a unit that the same delta removes, or breaks
/// another rule of the replica; the process test sees it as a stall.
#[test]
fn an_ordered_attack_kill_stream_applies_to_the_owner_replica_without_a_rejection() {
    let archive = package("muster");
    let mut app = headless_app(20);
    turts_pack::load_package(app.world_mut(), &archive).unwrap();
    initialize_completed_world(app.world_mut());
    let view0 = completed_world(app.world()).clone();
    let now = Instant::now();
    let mut client = replica(&archive);
    client.apply_snapshot(&export::snapshot_for(&view0, SlotId(0)), now);
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::Order(Box::new(turts_sim::OrderCommand {
                order: turts_mod_api::ContentId::parse("muster:order_attack").unwrap(),
                behaviour: turts_mod_api::Behaviour::Attack,
                target_kind: turts_mod_api::TargetKind::Unit,
                policy: None,
                units: vec![40],
                target: turts_sim::OrderTargetSim::Unit(46),
                queue: turts_sim::QueueMode::Replace,
                group: 0,
                allowed_units: vec!["muster:trooper".into()],
                queue_depth: 16,
            })),
        });
    let mut killed = false;
    for tick in 1..=900u32 {
        app.update();
        let batch = committed_tick(app.world()).clone();
        let view = completed_world(app.world()).clone();
        killed |= batch.removed.contains(&46);
        let delta = export::delta_for(&batch, &view, SlotId(0));
        assert_eq!(
            client.apply_delta(&delta, now),
            DeltaOutcome::Applied,
            "tick {tick}: the owner replica rejects the delta (removed {:?}, weapons {:?})",
            delta.removed,
            delta.weapons
        );
    }
    assert!(killed, "the attack kills the dummy");
}
