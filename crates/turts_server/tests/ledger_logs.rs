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
/// world state. The audit state at the end of every `sample_every` ticks, and at the end of the
/// last tick, is kept for the field test.
fn record(
    app: &mut bevy_app::App,
    ticks: u32,
    sample_every: u32,
    mut commands: impl FnMut(u32, &mut bevy::prelude::World) -> Vec<Command>,
) -> Captured {
    let mut records = vec![EconomyLogRecord::Initial(audit_state(app.world_mut()))];
    let mut states = Vec::new();
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
        for audit in trail.drain_ticks() {
            if audit.tick % sample_every == 0 || audit.tick == ticks {
                states.push(audit.after.clone());
            }
            records.push(EconomyLogRecord::from_tick(audit));
        }
    }
    Captured { records, states }
}

/// A captured log and the audit states at the end of some of its ticks (`record`).
struct Captured {
    records: Vec<EconomyLogRecord>,
    states: Vec<turts_sim::AuditState>,
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
    &forge().records
}

fn forge() -> &'static Captured {
    static LOG: OnceLock<Captured> = OnceLock::new();
    LOG.get_or_init(|| {
        let mut app = package_app("forge", 20);
        let shell = turts_mod_api::ContentId::parse("forge:shell").unwrap();
        let mut phase = 0;
        let mut first_at = 0;
        let mut seq = 0;
        record(&mut app, 2_400, 100, |tick, world| {
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
    &base().records
}

fn base() -> &'static Captured {
    static LOG: OnceLock<Captured> = OnceLock::new();
    LOG.get_or_init(|| {
        let mut app = package_app("base", 20);
        let command = |slot, seq, kind| Command {
            slot: SlotId(slot),
            seq,
            kind,
        };
        record(&mut app, 600, 50, |tick, world| match tick {
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

/// A refused command must leave the pending move queues unchanged. The sim names them "audited
/// command state" (`orders/pending.rs`, `PendingMovementOrders`), so the refusal rule reads them
/// (A1b review, probe F4: a refused command that changes `next_pool_id` was green in the ledger).
/// The test changes the state of the command boundary and of its one operation in the same way,
/// so that the shape rules stay true and only the refusal rule can find the change.
#[test]
fn refused_command_that_changes_pending_movement_orders_is_rejected() {
    let mut records = base_log().to_vec();
    let refused = records
        .iter()
        .find_map(|record| match record {
            EconomyLogRecord::CommandBoundary(command)
                if command.result.units.accepted.is_empty() && command.result.outcome.is_err() =>
            {
                Some((
                    command.operation_id,
                    command.input.slot.0,
                    command.input.seq,
                ))
            }
            _ => None,
        })
        .expect("the base log holds a refused command");
    let mut changed = 0;
    for record in &mut records {
        match record {
            EconomyLogRecord::CommandBoundary(command) if command.operation_id == refused.0 => {
                command.after.pending_movement_orders.next_pool_id += 1;
                changed += 1;
            }
            EconomyLogRecord::Operation(operation) if operation.operation_id == refused.0 => {
                operation.after.pending_movement_orders.next_pool_id += 1;
                changed += 1;
            }
            _ => {}
        }
    }
    assert_eq!(changed, 2, "one boundary and one operation are changed");
    let error = turts_audit::validate_ledger(&records)
        .expect_err("the ledger audit rejects a refused command that changes the pending queues");
    assert!(
        error.contains(&format!(
            "refused command of slot {} seq {} changed the logged view in operation {}",
            refused.1, refused.2, refused.0
        )),
        "the refusal rule names the command: {error}"
    );
}

/// The Muster package: forty troopers and a harvester on the `field` map. At tick 1 all the
/// troopers get a group move; at tick 10 the first ten get a second move that queues behind the
/// first; at tick 12 the harvester gets a legacy single move. The log holds order state, spread
/// pools, pending move queues and the plain move of one unit (no ledger event). The log is made
/// one time for all the tests of this file.
fn muster() -> &'static Captured {
    use turts_mod_api::{ContentId, OrderDef, PackageId};
    use turts_sim::{OrderCommand, OrderTargetSim, QueueMode, Vec2};
    static LOG: OnceLock<Captured> = OnceLock::new();
    LOG.get_or_init(|| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
        let bytes = turts_mod_runtime::pack::pack_dir(&path).expect("pack Muster");
        let archive =
            turts_mod_runtime::archive::PackageArchive::parse(&bytes).expect("parse Muster");
        let catalog = archive.content().expect("the Muster catalog resolves");
        let order = |units: Vec<u32>, point: Vec2, group: u32, queue: QueueMode| {
            let (id, definition): (ContentId, &OrderDef) = catalog
                .orders
                .iter()
                .find(|(id, _)| id.package.0 == "muster" && id.local == "order_move")
                .map(|(id, definition)| (id.clone(), definition))
                .expect("Muster order_move exists");
            Command {
                slot: SlotId(0),
                seq: group,
                kind: CommandKind::Order(Box::new(OrderCommand {
                    order: id,
                    behaviour: definition.behaviour,
                    target_kind: definition.target,
                    policy: None,
                    units,
                    target: OrderTargetSim::Point(point),
                    queue,
                    group,
                    allowed_units: definition.units.clone(),
                    queue_depth: catalog.order_queue_depths[&PackageId("muster".into())],
                })),
            }
        };
        let mut app = package_app("muster", 20);
        record(&mut app, 60, 10, |tick, _| match tick {
            1 => vec![order(
                (1..=40).collect(),
                Vec2::new(40.0, 30.0),
                1,
                QueueMode::Replace,
            )],
            10 => vec![order(
                (1..=10).collect(),
                Vec2::new(30.0, 20.0),
                2,
                QueueMode::Append,
            )],
            12 => vec![Command {
                slot: SlotId(0),
                seq: 3,
                kind: CommandKind::MoveTo {
                    unit_ids: vec![41],
                    target: Vec2::new(34.0, 26.0),
                },
            }],
            _ => Vec::new(),
        })
    })
}

#[test]
fn muster_log_passes_the_ledger_audit() {
    turts_audit::validate_ledger(&muster().records)
        .unwrap_or_else(|error| panic!("the ledger audit rejects the Muster log: {error}"));
}

fn empty_json(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::Array(items) => items.is_empty(),
        serde_json::Value::Object(fields) => fields.is_empty(),
        _ => false,
    }
}

/// The kept fields that every sampled `Tick` state of a fixture leaves empty. The lists are exact:
/// a field that fills or empties makes the test red, so a list cannot hide a dropped field.
const FORGE_EMPTY: &[&str] = &[
    "disabled",
    "movement_pools",
    "movement_progress",
    "pending_events",
    "pending_movement_orders",
    "physical_escapes",
    "plain_movement_progress",
    "raw_orders",
    "unit_orders",
];
const BASE_EMPTY: &[&str] = &[
    "disabled",
    "ledger_claims",
    "movement_pools",
    "movement_progress",
    "pending_events",
    "pending_movement_orders",
    "physical_escapes",
    "plain_movement_progress",
    "raw_belts",
    "raw_extractors",
    "raw_healths",
    "raw_inserters",
    "raw_output_inventories",
    "raw_weapons",
    "removed_units",
    "research",
    "unit_orders",
];
const MUSTER_EMPTY: &[&str] = &[
    "disabled",
    "ledger_claims",
    "pending_events",
    "physical_escapes",
    "raw_belts",
    "raw_extractors",
    "raw_inserters",
    "raw_output_inventories",
    "raw_queues",
    "removed_units",
    "research",
];
/// The kept fields that no fixture fills, in the order of `LEDGER_KEPT_FIELDS`: `physical_escapes`
/// (a unit that pushes out of a crowd), `disabled` (a power outage) and `pending_events` (events
/// that wait inside a tick). Only the equality of their JSON is checked here.
const NEVER_FILLED: &[&str] = &["physical_escapes", "disabled", "pending_events"];

/// The ledger view of real `Tick` states keeps every field of the audit state, with the same
/// JSON, and each kept field that a fixture can fill is not empty in at least one sampled state.
/// The small fixture of the sim unit test has no order and no inventory, so a field that it
/// leaves empty is not checked there (A1b review, probe F5: `unit_orders: Vec::new()` stayed
/// green there).
#[test]
fn ledger_view_keeps_every_field_of_real_tick_states() {
    use std::collections::BTreeSet;
    let fixtures: [(&str, &Captured, &[&str]); 3] = [
        ("forge", forge(), FORGE_EMPTY),
        ("base", base(), BASE_EMPTY),
        ("muster", muster(), MUSTER_EMPTY),
    ];
    let mut filled_somewhere = BTreeSet::new();
    for (name, captured, expected_empty) in fixtures {
        let mut empty_in_all: BTreeSet<&str> = turts_sim::LEDGER_KEPT_FIELDS.into_iter().collect();
        for state in &captured.states {
            let ledger = serde_json::to_value(turts_sim::ledger_audit(state)).unwrap();
            let mut audit = serde_json::to_value(state).unwrap();
            for field in turts_sim::LEDGER_DROPPED_FIELDS {
                audit.as_object_mut().unwrap().remove(field);
            }
            let differing: Vec<&str> = turts_sim::LEDGER_KEPT_FIELDS
                .into_iter()
                .filter(|field| ledger.get(field) != audit.get(field))
                .collect();
            assert!(
                differing.is_empty() && ledger == audit,
                "{name}: the ledger view of a Tick state differs from the audit state without the dropped fields: {differing:?}"
            );
            for field in turts_sim::LEDGER_KEPT_FIELDS {
                if ledger.get(field).is_some_and(|value| !empty_json(value)) {
                    empty_in_all.remove(field);
                    filled_somewhere.insert(field);
                }
            }
        }
        eprintln!("{name}: kept fields empty in every sampled Tick state: {empty_in_all:?}");
        assert_eq!(
            empty_in_all,
            expected_empty.iter().copied().collect::<BTreeSet<_>>(),
            "{name}: the kept fields that no sampled Tick state fills differ from the exact list"
        );
    }
    let never: Vec<&str> = turts_sim::LEDGER_KEPT_FIELDS
        .into_iter()
        .filter(|field| !filled_somewhere.contains(field))
        .collect();
    assert_eq!(
        never, NEVER_FILLED,
        "the kept fields that no fixture fills differ from the exact list"
    );
}

/// The browser audit oracle reads `economy.units[].facing` (`tests/browser/m2-audit.js`,
/// `projectAuditOwnerWorld`). The Forge state has units with a facing, so a ledger view that
/// drops it is red here, and in the browser case that reads the Forge log (A1c plant (b)).
#[test]
fn ledger_view_keeps_the_facing_that_the_browser_oracle_reads() {
    let state = forge().states.last().expect("the Forge run keeps a state");
    let ledger = serde_json::to_value(turts_sim::ledger_audit(state)).unwrap();
    let with_facing = ledger["economy"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|unit| unit.get("facing").is_some_and(|facing| !facing.is_null()))
        .count();
    assert!(
        with_facing > 0,
        "the Forge Tick state holds no unit with a facing, so the facing is not checked"
    );
}
