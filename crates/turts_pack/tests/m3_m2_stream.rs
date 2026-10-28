use bevy_app::App;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};
use turts_mod_api::{ContentCatalog, ContentId};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_pack::load_package;
use turts_sim::{
    Applied, AuditTrail, Command, CommandKind, EconomyEventKind, NodeId, SlotId, Vec2,
    apply_command, audit_state, headless_app,
};

const TICKS: u32 = 400;
const PACKAGES: [&str; 4] = ["base", "haul", "acme", "cedar"];

#[derive(Debug, PartialEq)]
struct PackageStream {
    package: String,
    ticks: Vec<TickStream>,
}

#[derive(Debug, PartialEq)]
struct TickStream {
    tick: u32,
    operation_count: usize,
    operations: Vec<OperationStream>,
}

#[derive(Debug, PartialEq)]
struct OperationStream {
    operation_id: u64,
    event_kinds: Vec<String>,
    before_equals_after: bool,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn operation_stream(
    package: &str,
    app: &mut App,
    mut pending: Vec<turts_sim::OperationAudit>,
) -> PackageStream {
    let mut ticks = Vec::with_capacity(TICKS as usize);
    let mut ids = Vec::new();
    let mut progress_events = 0;
    let mut production_began = false;
    let mut production_started = false;

    for tick in 1..=TICKS {
        app.update();
        pending.extend(
            app.world_mut()
                .resource_mut::<AuditTrail>()
                .drain_operations(),
        );

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

        let mut operations = pending
            .drain(..)
            .map(|op| {
                let event_kinds = op
                    .events
                    .iter()
                    .map(|event| event_kind(&event.kind))
                    .collect::<Vec<_>>();
                production_began |= event_kinds.iter().any(|kind| kind == "ProductionStarted");
                OperationStream {
                    operation_id: op.operation_id,
                    event_kinds,
                    before_equals_after: op.before == op.after,
                }
            })
            .collect::<Vec<_>>();
        ids.extend(operations.iter().map(|op| op.operation_id));
        ticks.push(TickStream {
            tick,
            operation_count: operations.len(),
            operations: std::mem::take(&mut operations),
        });
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

    PackageStream {
        package: package.to_owned(),
        ticks,
    }
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

fn operation_totals(streams: &Value) -> Vec<(String, usize)> {
    streams
        .as_array()
        .expect("streams are an array")
        .iter()
        .map(|stream| {
            let package = stream["package"]
                .as_str()
                .expect("stream has a package")
                .to_owned();
            let operations = stream["ticks"]
                .as_array()
                .expect("stream has tick records")
                .iter()
                .map(|tick| {
                    tick["operation_count"]
                        .as_u64()
                        .expect("tick has an operation count") as usize
                })
                .sum();
            (package, operations)
        })
        .collect()
}

fn run_package(package: &str) -> PackageStream {
    let (mut app, catalog) = load(package);
    let mut initial_operations = Vec::new();
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
        let result = apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq: 1,
                kind: CommandKind::MoveTo {
                    unit_ids: vec![unit],
                    target: Vec2::new(30.0, 30.0),
                },
            },
        );
        assert!(
            result.outcome.is_ok(),
            "acme MoveTo command applies: {result:?}"
        );
        initial_operations.extend(
            app.world_mut()
                .resource_mut::<AuditTrail>()
                .drain_operations(),
        );
        let stream = operation_stream(package, &mut app, initial_operations);
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
        return stream;
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
    initial_operations.extend(
        app.world_mut()
            .resource_mut::<AuditTrail>()
            .drain_operations(),
    );
    operation_stream(package, &mut app, initial_operations)
}

#[test]
fn m2_operation_stream_matches_baseline() {
    let streams = PACKAGES
        .iter()
        .map(|package| run_package(package))
        .collect::<Vec<_>>();
    for package in ["base", "haul", "cedar"] {
        let stream = streams
            .iter()
            .find(|stream| stream.package == package)
            .unwrap();
        assert!(
            stream.ticks.iter().any(|tick| tick.operation_count > 0),
            "{package} stream has at least one captured operation"
        );
    }

    let baseline_path = root().join("tests/m3/m2_operation_stream.json");
    let actual = Value::Array(
        streams
            .iter()
            .map(|stream| {
                serde_json::json!({
                    "package": stream.package,
                    "ticks": stream.ticks.iter().map(|tick| serde_json::json!({
                        "tick": tick.tick,
                        "operation_count": tick.operation_count,
                        "operations": tick.operations.iter().map(|operation| serde_json::json!({
                            "operation_id": operation.operation_id,
                            "event_kinds": operation.event_kinds,
                            "before_equals_after": operation.before_equals_after,
                        })).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                })
            })
            .collect(),
    );
    if std::env::var("UPDATE_M3_BASELINE").as_deref() == Ok("1") {
        fs::create_dir_all(baseline_path.parent().unwrap()).expect("create baseline directory");
        fs::write(
            baseline_path,
            format!("{}\n", serde_json::to_string_pretty(&actual).unwrap()),
        )
        .expect("write M3 M2 baseline");
    } else {
        let expected: Value =
            serde_json::from_slice(&fs::read(&baseline_path).unwrap_or_else(|error| {
                panic!("read M3 M2 baseline {}: {error}", baseline_path.display())
            }))
            .expect("parse M3 M2 baseline");
        if actual != expected {
            panic!(
                "M2 operation stream changed; actual totals {:?}, baseline totals {:?}",
                operation_totals(&actual),
                operation_totals(&expected)
            );
        }
    }
}
