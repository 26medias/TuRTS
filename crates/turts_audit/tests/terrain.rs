//! Real public-package commands, plus independent coherent-tamper counterexamples.
use turts_audit::{EconomyLogRecord, validate_records};
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{
    AuditState, AuditTrail, Command, CommandKind, CommandQueue, EconomyEventKind, OrderTargetSim,
    QueueMode, SlotId,
};
fn id(local: &str) -> ContentId {
    ContentId::parse(&format!("quarry:{local}")).unwrap()
}
fn rect() -> TerrainRect {
    TerrainRect {
        min: TerrainCell { x: 3, y: 1 },
        max: TerrainCell { x: 4, y: 2 },
    }
}
struct Fixture {
    app: bevy_app::App,
    unit: u32,
    seq: u32,
}
impl Fixture {
    fn new(reverse: bool) -> Self {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/quarry");
        let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
        let catalog = archive.content().unwrap();
        let (file, mut map) = archive.maps().unwrap().remove(0);
        map.visibility = None; // Economic-step replay is independent of the separate sight model.
        // Small public map normalization avoids retaining repeated 768-column snapshots.
        // Definitions still come from the real packaged mod and pass its runtime validator.
        map.size = MapSize { x: 8.0, y: 8.0 };
        map.placements.retain(|row| {
            row.owner_slot == 0
                && (row.unit == id("walker").to_string() || row.unit == id("depot").to_string())
        });
        for placement in &mut map.placements {
            placement.pos = if placement.unit == id("walker").to_string() {
                WorldPos { x: 1.5, y: 1.5 }
            } else {
                WorldPos { x: 1.5, y: 4.5 }
            };
        }
        let terrain = map.terrain.as_mut().unwrap();
        terrain.columns = 8;
        terrain.rows = 8;
        terrain.initial_heights.clear();
        let TerrainGenerationDef::Version1 { bands } = &mut terrain.generation;
        bands.last_mut().unwrap().top = 0;
        if reverse {
            map.placements.reverse();
        }
        let resolved = turts_mod_runtime::terrain::validate_terrain_map(
            &PackageId("quarry".into()),
            &file,
            &map,
            &catalog,
        )
        .unwrap();
        let mut app = turts_sim::headless_app(20);
        let staged = turts_sim::stage_package_with_terrain(
            app.world(),
            catalog,
            SourceDef { file, value: map },
            20,
            id("field"),
            resolved,
            Some(turts_sim::terrain::PrivateTerrainSeed::new(17)),
        )
        .unwrap();
        turts_sim::install_package(app.world_mut(), staged);
        turts_sim::visibility::initialize_context(app.world_mut(), 91, &[0, 1]).unwrap();
        app.update();
        let unit = turts_sim::audit_state(app.world_mut())
            .raw_units
            .iter()
            .find(|unit| unit.owner_slot == 0 && unit.pos == WorldPos { x: 1.5, y: 1.5 })
            .unwrap()
            .unit_id;
        app.world_mut().resource_mut::<AuditTrail>().enable();
        Self { app, unit, seq: 0 }
    }
    fn work(&mut self, tool: &str, elevation: i32) {
        let state = turts_sim::audit_state(self.app.world_mut());
        let policy = state
            .catalog
            .as_ref()
            .unwrap()
            .order_definitions
            .iter()
            .find(|row| row.id == id("order_work"))
            .unwrap();
        self.seq += 1;
        let command = Command {
            slot: SlotId(0),
            seq: self.seq,
            kind: CommandKind::Order(Box::new(turts_sim::OrderCommand {
                order: policy.id.clone(),
                behaviour: policy.behaviour,
                target_kind: policy.target,
                policy: Some(turts_sim::OrderPolicyAudit {
                    id: policy.id.clone(),
                    behaviour: policy.behaviour,
                    target: policy.target,
                    fire: policy.fire,
                    engage: policy.engage,
                    units: policy.units.clone(),
                    queue_depth: policy.queue_depth,
                    unit_capabilities: policy.unit_capabilities.clone(),
                }),
                units: vec![self.unit],
                target: OrderTargetSim::TerrainWork {
                    tool: id(tool),
                    cells: rect(),
                    target_elevation: elevation,
                },
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: policy.units.clone(),
                queue_depth: policy.queue_depth,
            })),
        };
        self.app
            .world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .push(command);
    }
    fn tick(&mut self, records: &mut Vec<EconomyLogRecord>, count: u32) {
        for _ in 0..count {
            self.app.update();
            let mut trail = self.app.world_mut().resource_mut::<AuditTrail>();
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
    }
}
fn captured(reverse: bool) -> (Vec<EconomyLogRecord>, u32) {
    let mut fixture = Fixture::new(reverse);
    let mut records = vec![EconomyLogRecord::Initial(turts_sim::audit_state(
        fixture.app.world_mut(),
    ))];
    for (tool, height, ticks) in [
        ("spade", -1, 35),
        ("backfill", 0, 15),
        ("spade", -1, 35),
        ("backfill", 0, 15),
    ] {
        fixture.work(tool, height);
        fixture.tick(&mut records, ticks);
    }
    (records, fixture.unit)
}
fn save(name: &str, records: &[EconomyLogRecord]) {
    if let Some(root) = std::env::var_os("TURTS_TERRAIN_AUDIT_OUT") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join(format!("{name}.jsonl")),
            records
                .iter()
                .map(|record| serde_json::to_string(record).unwrap() + "\n")
                .collect::<String>(),
        )
        .unwrap();
    }
}
fn states(record: &mut EconomyLogRecord, change: &mut impl FnMut(&mut AuditState)) {
    match record {
        EconomyLogRecord::Initial(state) => change(state),
        EconomyLogRecord::CommandBoundary(command) => {
            change(&mut command.before);
            change(&mut command.after);
        }
        EconomyLogRecord::Operation(operation) => {
            change(&mut operation.before);
            change(&mut operation.after);
        }
        EconomyLogRecord::Tick { before, state, .. } => {
            change(before);
            change(state);
        }
        _ => {}
    }
}
fn final_state(records: &[EconomyLogRecord]) -> &AuditState {
    records
        .iter()
        .rev()
        .find_map(|record| {
            if let EconomyLogRecord::Tick { state, .. } = record {
                Some(state)
            } else {
                None
            }
        })
        .unwrap()
}
#[test]
fn actual_package_terrain_steps_conserve_resources_depletion_and_failed_fill() {
    let (records, unit) = captured(false);
    save("ordinary-dig-fill-noyield-failed-fill", &records);
    validate_records(&records).unwrap_or_else(|error| panic!("terrain stream: {error}"));
    let steps = records
        .iter()
        .filter_map(|record| {
            if let EconomyLogRecord::Operation(operation) = record {
                Some(&operation.events)
            } else {
                None
            }
        })
        .flatten()
        .filter(|event| matches!(event.kind, EconomyEventKind::TerrainStepCommitted { .. }))
        .count();
    assert_eq!(steps, 3);
    let final_state = final_state(&records);
    let column = final_state
        .raw_terrain
        .as_ref()
        .unwrap()
        .columns_state
        .iter()
        .find(|column| column.cell == rect().min)
        .unwrap();
    assert_eq!(
        (
            column.initial_surface,
            column.natural_top,
            column.surface,
            column.revision
        ),
        (0, -1, -1, 3)
    );
    assert!(column.placed.is_empty());
    let soil = final_state
        .economy
        .inventories
        .iter()
        .find(|inventory| inventory.unit == unit)
        .unwrap()
        .items
        .iter()
        .find(|item| item.resource == id("soil"))
        .map_or(0, |item| item.on_hand);
    assert_eq!(soil, 0);
}
#[test]
fn coherent_terrain_account_yield_depletion_and_revision_tampering_is_rejected() {
    let (records, unit) = captured(false);
    for kind in 0..3 {
        let mut tampered = records.clone();
        for record in &mut tampered {
            states(record, &mut |state| {
                let raw = state.raw_terrain.as_mut().unwrap();
                let column = raw
                    .columns_state
                    .iter_mut()
                    .find(|column| column.cell == rect().min)
                    .unwrap();
                if column.revision == 0 {
                    return;
                }
                match kind {
                    0 => {
                        if let Some(item) = state
                            .economy
                            .inventories
                            .iter_mut()
                            .find(|inventory| inventory.unit == unit)
                            .unwrap()
                            .items
                            .iter_mut()
                            .find(|item| item.resource == id("soil"))
                        {
                            item.on_hand += 1;
                        }
                    }
                    1 => {
                        column.natural_top = -2;
                        if let Some(first) = column.placed.first_mut() {
                            first.bottom = -2;
                        } else {
                            column
                                .placed
                                .push(turts_sim::terrain::TerrainIntervalAudit {
                                    bottom: -2,
                                    top: column.surface,
                                    material: id("soil_layer"),
                                });
                        }
                    }
                    _ => {
                        column.revision += 1;
                        for work in &mut raw.work {
                            if work.unit == unit
                                && work.source_revision.is_some_and(|revision| revision > 0)
                            {
                                work.source_revision = work
                                    .source_revision
                                    .and_then(|revision| revision.checked_add(1));
                            }
                        }
                    }
                }
            });
            let change_events = |events: &mut Vec<turts_sim::EconomyEvent>| {
                if kind == 0 {
                    for event in events {
                        if let EconomyEventKind::TerrainStepCommitted {
                            placed: false,
                            outputs,
                            ..
                        } = &mut event.kind
                        {
                            for output in outputs {
                                output.amount += 1;
                            }
                        }
                    }
                }
            };
            match record {
                EconomyLogRecord::Operation(operation) => change_events(&mut operation.events),
                EconomyLogRecord::Tick { events, .. } => change_events(events),
                _ => {}
            }
            states(record, &mut |state| {
                change_events(&mut state.pending_events)
            });
        }
        let error = validate_records(&tampered)
            .expect_err("coherent terrain forgery must fail independent math");
        assert!(error.contains("terrain"), "wrong rejection: {error}");
    }
}
#[test]
fn same_seed_command_tape_and_real_allocator_inversion_preserve_physical_edits() {
    let (first, first_id) = captured(false);
    let (again, _) = captured(false);
    let (inverted, inverted_id) = captured(true);
    assert_ne!(first_id, inverted_id);
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(&again).unwrap(),
        "same private seed and ordinary tape reproduce every capture"
    );
    validate_records(&inverted).unwrap();
    let physical = |records: &[EconomyLogRecord]| {
        records.iter().filter_map(|record|if let EconomyLogRecord::Tick{state,..}=record{
        Some(serde_json::json!({"columns":state.raw_terrain.as_ref().unwrap().columns_state,"inventories":state.economy.inventories.iter().map(|inventory|{
            let actor=state.raw_units.iter().find(|unit|unit.unit_id==inventory.unit).unwrap();serde_json::json!({"slot":actor.owner_slot,"position":actor.pos,"items":inventory.items})
        }).collect::<Vec<_>>()}))
    }else{None}).map(|mut value|{value["inventories"].as_array_mut().unwrap().sort_by_key(|row|serde_json::to_string(row).unwrap());value}).collect::<Vec<_>>()
    };
    assert_eq!(physical(&first), physical(&inverted));
    save("real-allocator-inverted", &inverted);
}
