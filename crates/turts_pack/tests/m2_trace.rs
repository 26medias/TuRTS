//! The trace emitter of the pre-MP1 re-freeze (task Q1b).
//!
//! The ignored test `m2_trace_writes_four_packages` runs the four packages of the
//! frozen M2 operation stream (`m3_m2_stream.rs`) with the same command sequence and
//! the same drain points. It writes, to the directory `TURTS_M2_TRACE_OUT`:
//!
//! - `<package>.jsonl`: one trace for each package, in the format
//!   `turts-refreeze-trace/4` (the reference text is the docstring at the top of
//!   `tests/m3/refreeze_classify.py`);
//! - `m2_operation_stream.json`: the operation stream of the same run, with the
//!   bytes that `UPDATE_M3_BASELINE=1` makes `m3_m2_stream.rs` write.
//!
//! Rules of this file:
//! - The sha256 of this file is the `emitter_sha256` of every header, so the
//!   classifier can check that two runs used the same emitter. The file holds no
//!   value that changes from one run to the next (time, path, commit).
//! - It reads the movement state only through `turts_sim::movement::probe`
//!   (plan R11). It never names a movement type that task A2b2 deletes, so the
//!   file stays equal on every build of the slice.
//! - It needs the cargo feature `test-probes` of `turts_sim`. The dev-dependency
//!   of `turts_pack` turns it on.
//! - Units are sorted by unit id. Floats are written with `to_bits`.
//! - The stream never sets `separated` or `was_projected` to true and never has a pool
//!   or a plain progress. The cases of `crates/turts_sim/tests/probe_flags.rs` show
//!   that the probe values work.
//! - `TURTS_M2_TRACE_OUT` must be an absolute path. A relative path would resolve
//!   under the crate folder and write untracked files into the tree, so the test
//!   refuses it.

use bevy_app::App;
use bevy_ecs::world::World;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
use turts_mod_api::{Behaviour, ContentCatalog, ContentId};
use turts_mod_runtime::{archive::PackageArchive, hash::sha256_hex, pack::pack_dir};
use turts_pack::load_package;
use turts_sim::{
    Applied, AuditTrail, Command, CommandKind, EconomyEventKind, MoveTarget, NodeId, Owner,
    Position, SlotId, UnitId, UnitOrders, Vec2, apply_command, audit_state, headless_app,
    movement::probe, order_view,
};

const TICKS: u32 = 400;
const PACKAGES: [&str; 4] = ["base", "haul", "acme", "cedar"];
const FORMAT: &str = "turts-refreeze-trace/4";
const EMITTER_SOURCE: &[u8] = include_bytes!("m2_trace.rs");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn out_dir() -> PathBuf {
    let value = std::env::var("TURTS_M2_TRACE_OUT")
        .expect("TURTS_M2_TRACE_OUT names the directory that receives the traces");
    let path = PathBuf::from(&value);
    assert!(
        path.is_absolute(),
        "TURTS_M2_TRACE_OUT must be an absolute path (got {value:?}): a relative path would \
         resolve under crates/turts_pack/ and write untracked files into the tree"
    );
    path
}

fn emitter_sha256() -> String {
    sha256_hex(EMITTER_SOURCE)
        .strip_prefix("sha256:")
        .expect("sha256_hex has the prefix sha256:")
        .to_owned()
}

fn event_kind(kind: &EconomyEventKind) -> String {
    let value = serde_json::to_value(kind).expect("event kind serializes");
    match value {
        Value::String(name) => name,
        Value::Object(fields) => {
            fields
                .into_iter()
                .next()
                .expect("externally tagged event kind has one variant key")
                .0
        }
        _ => panic!("externally tagged event kind serializes as a string or object"),
    }
}

fn load(package: &str) -> (App, ContentCatalog) {
    let dir = root().join("mods").join(package);
    let bytes = pack_dir(&dir).unwrap_or_else(|error| panic!("pack {package}: {error}"));
    let archive = PackageArchive::parse(&bytes).expect("packed package parses");
    let mut app = headless_app(20);
    load_package(app.world_mut(), &archive).unwrap();
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    app.world_mut().resource_mut::<AuditTrail>().enable();
    (app, catalog)
}

fn producer_recipe(app: &mut App) -> (u32, ContentId) {
    let state = audit_state(app.world_mut());
    let catalog = &app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0;
    let (producer_type, recipe_id) = catalog
        .units
        .iter()
        .find_map(|(id, unit)| {
            unit.producer.as_ref().map(|producer| {
                let recipe = producer
                    .recipes
                    .iter()
                    .min_by_key(|recipe| {
                        catalog.production[*recipe]
                            .costs
                            .iter()
                            .map(|cost| cost.amount)
                            .sum::<u64>()
                    })
                    .expect("producer has a recipe");
                (id, recipe.clone())
            })
        })
        .expect("package has a producer");
    let producer = state
        .units
        .iter()
        .find(|unit| unit.content_id == producer_type.to_string() && unit.owner_slot == 0)
        .expect("slot 0 owns a producer")
        .unit_id;
    (producer, recipe_id)
}

/// The behaviour name: the kind of an order in the trace. A content id holds a
/// colon, which the format reads as the separator of kind and stage.
fn behaviour_name(behaviour: Behaviour) -> &'static str {
    match behaviour {
        Behaviour::Move => "Move",
        Behaviour::Stop => "Stop",
        Behaviour::Hold => "Hold",
        Behaviour::Attack => "Attack",
        Behaviour::Gather => "Gather",
        Behaviour::Produce => "Produce",
        Behaviour::CancelProduction => "CancelProduction",
        Behaviour::Remove => "Remove",
        Behaviour::SetEnabled(_) => "SetEnabled",
        // API 0.2.4: no packed package orders these yet.
        Behaviour::Construct => "Construct",
        Behaviour::CancelConstruction => "CancelConstruction",
        Behaviour::SetRally => "SetRally",
        Behaviour::Repair => "Repair",
        Behaviour::Surrender => "Surrender",
    }
}

/// The name of a gather stage in the trace: `GatherStage::as_str` with the
/// brackets of a waiting reason written as a hyphen (`Waiting-NoDeposit`).
fn stage_name(stage: &str) -> String {
    stage.replace('(', "-").replace(')', "")
}

struct UnitRow {
    id: u32,
    owner: SlotId,
    position: Vec2,
    target: Option<Vec2>,
    orders: Option<UnitOrders>,
}

/// All units, sorted by unit id. Never the iteration order of the world.
fn unit_rows(world: &mut World) -> Vec<UnitRow> {
    let mut query = world.query::<(
        &UnitId,
        &Owner,
        &Position,
        Option<&MoveTarget>,
        Option<&UnitOrders>,
    )>();
    let mut rows: Vec<_> = query
        .iter(world)
        .map(|(id, owner, position, target, orders)| UnitRow {
            id: id.0,
            owner: owner.0,
            position: position.0,
            target: target.map(|target| target.0),
            orders: orders.cloned(),
        })
        .collect();
    rows.sort_by_key(|row| row.id);
    rows
}

fn bits(value: f32) -> String {
    format!("{:08x}", value.to_bits())
}

fn point_bits(point: Vec2) -> Value {
    json!({"x_bits": bits(point.x), "y_bits": bits(point.y)})
}

/// `none`, or `<Behaviour>:<stage>`. The stage is the gather stage of the unit
/// (`-` when the unit has no gather order).
fn order_text(world: &World, row: &UnitRow) -> String {
    let Some(order) = row
        .orders
        .as_ref()
        .and_then(|orders| orders.current.as_ref())
    else {
        return "none".to_owned();
    };
    let stage = order_view(world, row.owner, row.id)
        .map_or_else(|_| "-".to_owned(), |view| stage_name(view.stage.as_str()));
    format!("{}:{stage}", behaviour_name(order.behaviour))
}

/// The stage of the gather cycle of a unit: `GatherStage::as_str` (brackets as a
/// hyphen), or `none` when the unit has no gather cycle. It is read through
/// `order_view`, the economy view of the gather order, so a legacy Gather (which
/// leaves `UnitOrders` empty and `order` at `none`) shows its stage too.
fn gather_stage(world: &World, row: &UnitRow) -> String {
    order_view(world, row.owner, row.id).map_or_else(
        |_| "none".to_owned(),
        |view| stage_name(view.stage.as_str()),
    )
}

fn unit_record(world: &World, package: &str, tick: u32, row: &UnitRow) -> Value {
    let queue: Vec<&str> = row
        .orders
        .as_ref()
        .map(|orders| {
            orders
                .queue
                .iter()
                .map(|order| behaviour_name(order.behaviour))
                .collect()
        })
        .unwrap_or_default();
    let last_failure = row
        .orders
        .as_ref()
        .and_then(|orders| orders.last_failure.as_ref())
        .map_or(Value::Null, |failure| {
            json!({
                "order": failure.order.to_string(),
                "reason": failure.reason.as_str(),
                "tick": failure.tick,
            })
        });
    json!({
        "record": "unit",
        "package": package,
        "tick": tick,
        "unit": row.id,
        "x_bits": bits(row.position.x),
        "y_bits": bits(row.position.y),
        "target": row.target.map_or(Value::Null, point_bits),
        "activity": probe::activity(world, row.id),
        "state": probe::unit_state(world, row.id),
        "order": order_text(world, row),
        "gather_stage": gather_stage(world, row),
        "queue": queue,
        "last_failure": last_failure,
        "separated": probe::separated(world, row.id),
        "was_projected": probe::was_projected(world, row.id),
    })
}

/// How a `MoveTo` command reached the movement code, as the world shows it right
/// after the command: `direct` (the unit has its `MoveTarget` at once, nothing is
/// pending), `per_unit` (a pending envelope, and the command has more than one
/// unit: `command.rs` takes that route for `unit_ids.len() > 1`), `envelope` (a
/// pending envelope for one unit) or `none`. It is read from the world, not from
/// the shape of the command, so a reroute of the single `MoveTo` (task A2c) shows.
fn observe_move_to_route(world: &mut World, unit_count: usize, unit: u32) -> &'static str {
    if probe::pending_movement_orders(world) > 0 {
        if unit_count > 1 {
            "per_unit"
        } else {
            "envelope"
        }
    } else if unit_rows(world)
        .iter()
        .any(|row| row.id == unit && row.target.is_some())
    {
        "direct"
    } else {
        "none"
    }
}

struct OpRecord {
    id: u64,
    unit: Option<u32>,
    phase: String,
    events: Vec<Value>,
    kinds: Vec<String>,
    before_equals_after: bool,
    move_to_route: Option<&'static str>,
}

struct PackageRun {
    trace: String,
    /// The stream of the package: `{"package", "ticks"}`, as `m3_m2_stream.rs` builds it.
    stream: Value,
}

fn line(value: &Value) -> String {
    let mut text = serde_json::to_string(value).expect("trace record serializes");
    text.push('\n');
    text
}

/// The loop of `operation_stream()` in `m3_m2_stream.rs`, with the same drain
/// points, plus the trace records of each tick.
fn traced_stream(
    package: &str,
    app: &mut App,
    initial: Vec<turts_sim::OperationAudit>,
    initial_route: Option<&'static str>,
    header: Value,
) -> PackageRun {
    let mut trace = line(&header);
    let mut ticks = Vec::with_capacity(TICKS as usize);
    let mut ids = Vec::new();
    let mut progress_events = 0;
    let mut production_began = false;
    let mut production_started = false;
    let initial_ids: Vec<u64> = initial.iter().map(|op| op.operation_id).collect();
    let mut pending = initial;

    for tick in 1..=TICKS {
        app.update();
        pending.extend(
            app.world_mut()
                .resource_mut::<AuditTrail>()
                .drain_operations(),
        );

        // The state after the update of this tick, before any command of the
        // harness (the Produce command below does not move a unit).
        trace.push_str(&line(&json!({
            "record": "world",
            "package": package,
            "tick": tick,
            "pools": probe::pool_count(app.world()),
            "pending_movement_orders": probe::pending_movement_orders(app.world()),
        })));
        for row in unit_rows(app.world_mut()) {
            trace.push_str(&line(&unit_record(app.world(), package, tick, &row)));
        }

        let delivered = pending.iter().flat_map(|op| &op.events).any(|event| {
            matches!(
                event.kind,
                EconomyEventKind::Stored { .. } | EconomyEventKind::Converted { .. }
            )
        });
        if delivered {
            progress_events += pending
                .iter()
                .flat_map(|op| &op.events)
                .filter(|event| {
                    matches!(
                        event.kind,
                        EconomyEventKind::Extracted { .. }
                            | EconomyEventKind::Stored { .. }
                            | EconomyEventKind::Converted { .. }
                    )
                })
                .count();
        }

        if delivered && !production_started {
            let (producer_id, recipe_id) = producer_recipe(app);
            let result = apply_command(
                app.world_mut(),
                &Command {
                    slot: SlotId(0),
                    seq: 2,
                    kind: CommandKind::Produce {
                        producer_id,
                        recipe_id,
                    },
                },
            );
            assert!(
                matches!(result.outcome, Ok(Applied { job_id: Some(_) })),
                "{package} Produce command must queue after Gather delivery: {result:?}"
            );
            pending.extend(
                app.world_mut()
                    .resource_mut::<AuditTrail>()
                    .drain_operations(),
            );
            production_started = true;
        }

        let operations = pending
            .drain(..)
            .map(|op| {
                let kinds = op
                    .events
                    .iter()
                    .map(|event| event_kind(&event.kind))
                    .collect::<Vec<_>>();
                production_began |= kinds.iter().any(|kind| kind == "ProductionStarted");
                OpRecord {
                    id: op.operation_id,
                    unit: op.unit_id,
                    phase: format!("{:?}", op.phase),
                    events: op
                        .events
                        .iter()
                        .map(|event| serde_json::to_value(event).expect("event serializes"))
                        .collect(),
                    kinds,
                    before_equals_after: op.before == op.after,
                    move_to_route: initial_ids
                        .contains(&op.operation_id)
                        .then_some(initial_route)
                        .flatten(),
                }
            })
            .collect::<Vec<_>>();
        ids.extend(operations.iter().map(|op| op.id));
        for op in &operations {
            let mut record = json!({
                "record": "op",
                "package": package,
                "tick": tick,
                "op_id": op.id,
                "unit": op.unit,
                "phase": op.phase,
                "events": op.events,
                "before_equals_after": op.before_equals_after,
            });
            if let Some(route) = op.move_to_route {
                record["move_to_route"] = json!(route);
            }
            trace.push_str(&line(&record));
        }
        ticks.push(json!({
            "tick": tick,
            "operation_count": operations.len(),
            "operations": operations.iter().map(|op| json!({
                "operation_id": op.id,
                "event_kinds": op.kinds,
                "before_equals_after": op.before_equals_after,
            })).collect::<Vec<_>>(),
        }));
    }

    assert!(
        ids.windows(2).all(|pair| pair[0] < pair[1]),
        "{package} operation identities stay in captured order"
    );
    if package != "acme" {
        assert!(progress_events > 0, "{package} Gather made real progress");
        assert!(production_started, "{package} Produce command was accepted");
        assert!(
            production_began,
            "{package} production started in {TICKS} ticks"
        );
    }
    PackageRun {
        trace,
        stream: json!({"package": package, "ticks": ticks}),
    }
}

fn header(package: &str, app: &mut App) -> Value {
    let state = audit_state(app.world_mut());
    let units: Vec<Value> = state
        .units
        .iter()
        .map(|unit| {
            json!({
                "unit": unit.unit_id,
                "content_id": unit.content_id,
                "owner_slot": unit.owner_slot,
            })
        })
        .collect();
    let mut sorted = units.clone();
    sorted.sort_by_key(|unit| unit["unit"].as_u64());
    assert_eq!(
        units, sorted,
        "{package}: header units are sorted by unit id"
    );
    json!({
        "record": "header",
        "format": FORMAT,
        "package": package,
        "emitter_sha256": emitter_sha256(),
        "ticks": TICKS,
        "unit_count": units.len(),
        "units": units,
    })
}

fn run_package(package: &str) -> PackageRun {
    let (mut app, catalog) = load(package);
    let header = header(package, &mut app);
    if package == "acme" {
        let before = audit_state(app.world_mut());
        let unit = before
            .units
            .iter()
            .find(|unit| unit.owner_slot == 0)
            .expect("acme has a slot 0 unit")
            .unit_id;
        let before_pos = before
            .raw_units
            .iter()
            .find(|raw| raw.unit_id == unit)
            .expect("acme unit has a position")
            .pos
            .clone();
        let unit_ids = vec![unit];
        let result = apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq: 1,
                kind: CommandKind::MoveTo {
                    unit_ids: unit_ids.clone(),
                    target: Vec2::new(30.0, 30.0),
                },
            },
        );
        assert!(
            result.outcome.is_ok(),
            "acme MoveTo command applies: {result:?}"
        );
        let route = observe_move_to_route(app.world_mut(), unit_ids.len(), unit);
        let initial = app
            .world_mut()
            .resource_mut::<AuditTrail>()
            .drain_operations();
        let run = traced_stream(package, &mut app, initial, Some(route), header);
        let after = audit_state(app.world_mut());
        let after_pos = after
            .raw_units
            .iter()
            .find(|raw| raw.unit_id == unit)
            .expect("acme unit remains present")
            .pos
            .clone();
        assert!(
            before_pos != after_pos,
            "acme MoveTo changes the real unit position"
        );
        return run;
    }

    let state = audit_state(app.world_mut());
    let worker_type = catalog
        .units
        .iter()
        .find(|(_, unit)| unit.gatherer.is_some())
        .expect("package has a gatherer")
        .0
        .to_string();
    let worker = state
        .units
        .iter()
        .find(|unit| unit.content_id == worker_type && unit.owner_slot == 0)
        .expect("slot 0 owns a gatherer")
        .unit_id;
    let worker_pos = state
        .raw_units
        .iter()
        .find(|unit| unit.unit_id == worker)
        .expect("gatherer has position")
        .pos
        .clone();
    let node = state
        .nodes
        .iter()
        .min_by(|a, b| {
            let da = (a.pos.x - worker_pos.x).powi(2) + (a.pos.y - worker_pos.y).powi(2);
            let db = (b.pos.x - worker_pos.x).powi(2) + (b.pos.y - worker_pos.y).powi(2);
            da.total_cmp(&db)
        })
        .expect("package has a gather node")
        .id
        .0;
    let result = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::Gather {
                unit_id: worker,
                node_id: NodeId(node),
            },
        },
    );
    assert!(
        result.outcome.is_ok(),
        "{package} Gather command applies: {result:?}"
    );
    let initial = app
        .world_mut()
        .resource_mut::<AuditTrail>()
        .drain_operations();
    traced_stream(package, &mut app, initial, None, header)
}

#[test]
#[ignore = "writes the four traces to TURTS_M2_TRACE_OUT; run it with --ignored"]
fn m2_trace_writes_four_packages() {
    let out = out_dir();
    fs::create_dir_all(&out).expect("create the trace directory");
    let mut streams = Vec::new();
    for package in PACKAGES {
        let run = run_package(package);
        fs::write(out.join(format!("{package}.jsonl")), run.trace).expect("write the trace");
        streams.push(run.stream);
    }
    // The bytes that `UPDATE_M3_BASELINE=1` makes `m3_m2_stream.rs` write.
    fs::write(
        out.join("m2_operation_stream.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&Value::Array(streams)).expect("stream serializes")
        ),
    )
    .expect("write the operation stream");
}
