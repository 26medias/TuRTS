//! Real economy logs for the ledger audit (pre-MP1 task A1b). Each test runs a package in
//! process, captures every record that the server sink writes (`Initial`, command boundaries,
//! operations, `Tick`), and gives the log to the validators. The fault cases of
//! `tests/m2/refutations.json` plant a defect in the simulation and run these tests.

use std::path::PathBuf;
use std::sync::OnceLock;

use turts_audit::EconomyLogRecord;
use turts_sim::{AuditTrail, Command, CommandKind, CommandQueue, SlotId, audit_state};

fn package_app(name: &str, tick_hz: u16) -> bevy_app::App {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../mods")
        .join(name);
    let bytes = turts_mod_runtime::pack::pack_dir(&path).expect("pack the package");
    let archive =
        turts_mod_runtime::archive::PackageArchive::parse(&bytes).expect("parse the package");
    let mut app = turts_sim::headless_app(tick_hz);
    turts_pack::load_package(app.world_mut(), &archive).expect("load the package");
    app.world_mut().resource_mut::<AuditTrail>().enable();
    app
}

/// Run `ticks` ticks. Before each tick, `commands` gives the commands of that tick from the
/// world state.
fn record(
    app: &mut bevy_app::App,
    ticks: u32,
    mut commands: impl FnMut(u32, &mut bevy::prelude::World) -> Vec<Command>,
) -> Vec<EconomyLogRecord> {
    let mut records = vec![EconomyLogRecord::Initial(audit_state(app.world_mut()))];
    for tick in 1..=ticks {
        for command in commands(tick, app.world_mut()) {
            app.world_mut()
                .resource_mut::<CommandQueue>()
                .0
                .push_back(command);
        }
        app.update();
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        assert!(trail.valid, "the audit capture stays valid");
        records.extend(
            trail
                .drain_commands()
                .into_iter()
                .map(EconomyLogRecord::CommandBoundary),
        );
        records.extend(
            trail
                .drain_operations()
                .into_iter()
                .map(EconomyLogRecord::Operation),
        );
        records.extend(
            trail
                .drain_ticks()
                .into_iter()
                .map(EconomyLogRecord::from_tick),
        );
    }
    records
}

fn events(records: &[EconomyLogRecord]) -> Vec<&turts_sim::EconomyEventKind> {
    records
        .iter()
        .filter_map(|record| match record {
            EconomyLogRecord::Operation(operation) => Some(&operation.events),
            _ => None,
        })
        .flatten()
        .map(|event| &event.kind)
        .collect()
}

fn produce(producer_id: u32, recipe: &str) -> CommandKind {
    CommandKind::Produce {
        producer_id,
        recipe_id: turts_mod_api::ContentId::parse(recipe).unwrap(),
    }
}

/// The Forge proving ground: drill, belts, arm loaders, the shell press, the turret, the lab and
/// two enemy drones. When the lab (unit 8) holds 10 shells, it gets a research job, which is
/// cancelled one tick later. When the lab holds 10 shells again, it gets the job again, and the
/// job completes. The log holds extraction, belt and inserter moves, machine jobs, reserve-then-charge
/// research jobs, a cancel with its refund, a completed research, shots with ammo, damage and a kill. The log is made one time for all
/// the tests of this file.
fn forge_log() -> &'static [EconomyLogRecord] {
    static LOG: OnceLock<Vec<EconomyLogRecord>> = OnceLock::new();
    LOG.get_or_init(|| {
        let mut app = package_app("forge", 20);
        let shell = turts_mod_api::ContentId::parse("forge:shell").unwrap();
        let mut phase = 0;
        let mut first_at = 0;
        let mut seq = 0;
        record(&mut app, 2_400, |tick, world| {
            let lab_shells = turts_sim::inventory_view(world, SlotId(0), 8)
                .ok()
                .and_then(|inventory| {
                    inventory
                        .items
                        .iter()
                        .find(|item| item.resource == shell)
                        .map(|item| item.on_hand)
                })
                .unwrap_or(0);
            let jobs = turts_sim::queue_view(world, SlotId(0), 8)
                .map(|queue| queue.jobs)
                .unwrap_or_default();
            let mut next = |kind| {
                seq += 1;
                Command {
                    slot: SlotId(0),
                    seq,
                    kind,
                }
            };
            match phase {
                0 | 2 if lab_shells >= 10 && jobs.is_empty() => {
                    phase += 1;
                    first_at = tick;
                    vec![next(produce(8, "forge:hardened_shells"))]
                }
                1 if tick == first_at + 1 => {
                    phase = 2;
                    let job = jobs
                        .first()
                        .map(|job| job.id)
                        .expect("the first research job");
                    vec![next(CommandKind::CancelProduction {
                        producer_id: 8,
                        job_id: turts_sim::JobId(job),
                    })]
                }
                _ => Vec::new(),
            }
        })
    })
}

/// Every kind of ledger event that the domains check is in the Forge log, so the cases that read
/// it can go red.
#[test]
fn forge_log_holds_every_ledger_event_kind() {
    use turts_sim::EconomyEventKind as K;
    let records = forge_log();
    let kinds = events(records);
    let count = |matches: fn(&K) -> bool| kinds.iter().filter(|kind| matches(kind)).count();
    let counts = [
        (
            "Extracted",
            count(|kind| matches!(kind, K::Extracted { .. })),
        ),
        (
            "ProductionQueued",
            count(|kind| matches!(kind, K::ProductionQueued { .. })),
        ),
        (
            "ProductionStarted",
            count(|kind| matches!(kind, K::ProductionStarted { .. })),
        ),
        (
            "ProductionCancelled",
            count(|kind| matches!(kind, K::ProductionCancelled { .. })),
        ),
        (
            "ItemsProduced",
            count(|kind| matches!(kind, K::ItemsProduced { .. })),
        ),
        ("Fired", count(|kind| matches!(kind, K::Fired { .. }))),
        (
            "Destroyed",
            count(|kind| matches!(kind, K::Destroyed { .. })),
        ),
        (
            "ResearchCompleted",
            count(|kind| matches!(kind, K::ResearchCompleted { .. })),
        ),
    ];
    eprintln!("forge log: {} records, events {counts:?}", records.len());
    for (name, count) in counts {
        assert!(count > 0, "the Forge log holds no {name} event");
    }
}

#[test]
fn forge_log_passes_the_ledger_audit() {
    turts_audit::validate_ledger(forge_log())
        .unwrap_or_else(|error| panic!("the ledger audit rejects the Forge log: {error}"));
}

#[test]
fn forge_log_passes_both_validators() {
    turts_audit::validate_side_by_side(forge_log())
        .unwrap_or_else(|error| panic!("a validator rejects the Forge log: {error}"));
}

/// The base package: both harvesters gather, slot 0 trains two riflemen (a slot balance charge
/// on queue) and cancels the second while it waits (a full refund), and slot 1 asks for a
/// harvester that it cannot pay (a refused command). The first rifleman spawns. The log is made
/// one time for all the tests of this file.
fn base_log() -> &'static [EconomyLogRecord] {
    static LOG: OnceLock<Vec<EconomyLogRecord>> = OnceLock::new();
    LOG.get_or_init(|| {
        let mut app = package_app("base", 20);
        let command = |slot, seq, kind| Command {
            slot: SlotId(slot),
            seq,
            kind,
        };
        record(&mut app, 600, |tick, world| match tick {
            1 => vec![
                command(
                    0,
                    1,
                    CommandKind::Gather {
                        unit_id: 2,
                        node_id: turts_sim::NodeId(1),
                    },
                ),
                command(
                    1,
                    1,
                    CommandKind::Gather {
                        unit_id: 4,
                        node_id: turts_sim::NodeId(2),
                    },
                ),
                command(0, 2, produce(1, "base:train_rifleman")),
                command(0, 3, produce(1, "base:train_rifleman")),
                command(1, 2, produce(3, "base:build_harvester")),
            ],
            3 => {
                let job = turts_sim::queue_view(world, SlotId(0), 1)
                    .ok()
                    .and_then(|queue| queue.jobs.last().map(|job| job.id))
                    .expect("the second rifleman job waits");
                vec![command(
                    0,
                    4,
                    CommandKind::CancelProduction {
                        producer_id: 1,
                        job_id: turts_sim::JobId(job),
                    },
                )]
            }
            _ => Vec::new(),
        })
    })
}

#[test]
fn base_log_holds_charges_refunds_refusals_and_a_spawn() {
    use turts_sim::EconomyEventKind as K;
    let records = base_log();
    let kinds = events(records);
    let refused = records
        .iter()
        .filter(|record| {
            matches!(record, EconomyLogRecord::CommandBoundary(command) if command.result.outcome.is_err())
        })
        .count();
    let count = |matches: fn(&K) -> bool| kinds.iter().filter(|kind| matches(kind)).count();
    let counts = [
        (
            "Extracted",
            count(|kind| matches!(kind, K::Extracted { .. })),
        ),
        (
            "Converted",
            count(|kind| matches!(kind, K::Converted { .. })),
        ),
        (
            "ProductionQueued",
            count(|kind| matches!(kind, K::ProductionQueued { .. })),
        ),
        (
            "ProductionCancelled",
            count(|kind| matches!(kind, K::ProductionCancelled { .. })),
        ),
        (
            "ProductionCompleted",
            count(|kind| matches!(kind, K::ProductionCompleted { .. })),
        ),
        ("refused commands", refused),
    ];
    eprintln!("base log: {} records, {counts:?}", records.len());
    for (name, count) in counts {
        assert!(count > 0, "the base log holds no {name}");
    }
}

#[test]
fn base_log_passes_both_validators() {
    turts_audit::validate_side_by_side(base_log())
        .unwrap_or_else(|error| panic!("a validator rejects the base log: {error}"));
}
