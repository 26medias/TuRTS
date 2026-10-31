mod support;
#[path = "support/wire_sender.rs"]
mod wire_sender;

#[path = "../../turts_pack/tests/common/accounting.rs"]
#[allow(dead_code)]
mod accounting;

#[path = "../src/economylog.rs"]
mod economylog;

// Unit tests cover typed JSONL replay; ignored process tests below exercise the live
// server sink and the public Task 15 acceptance scenarios.
fn package_files(
    root: &std::path::Path,
    dir: &std::path::Path,
    files: &mut std::collections::BTreeMap<String, Vec<u8>>,
) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            package_files(root, &path, files);
        } else {
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(relative, std::fs::read(path).unwrap());
        }
    }
}

fn economy_fixture_app_at(tick_hz: u16) -> bevy_app::App {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../turts_pack/tests/fixtures/economy");
    let mut files = std::collections::BTreeMap::new();
    package_files(&root, &root, &mut files);
    let package = turts_mod_runtime::pack::pack_files(&files).unwrap();
    let archive = turts_mod_runtime::archive::PackageArchive::parse(&package).unwrap();
    let mut app = turts_sim::headless_app(tick_hz);
    turts_pack::load_package(app.world_mut(), &archive).unwrap();
    app
}

fn advanced_clock(state: &turts_sim::AuditState, tick: u32) -> turts_sim::AuditState {
    assert_eq!(state.economy.tick.checked_add(1), Some(tick));
    let mut advanced = state.clone();
    advanced.economy.tick = tick;
    advanced
}

#[test]
fn live_audit_reader_ignores_only_unterminated_tail_and_waits_for_tick() {
    use economylog::{EconomyLogRecord, EconomyLogWriter};
    use std::io::Write;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use turts_sim::{AuditTrail, economy::views::audit_state};

    let mut app = economy_fixture_app_at(5);
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let mut writer = EconomyLogWriter::new(Vec::new());
    let initial = audit_state(app.world_mut());
    writer
        .write_initial(app.world().resource::<AuditTrail>(), initial)
        .unwrap();
    for _ in 0..3 {
        app.update();
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        for command in trail.drain_commands() {
            writer.write_command(&trail, command).unwrap();
        }
        for operation in trail.drain_operations() {
            writer.write_operation(&trail, operation).unwrap();
        }
        let tick = trail.drain_ticks().pop().expect("one completed test tick");
        writer.write_tick(&trail, tick).unwrap();
    }
    writer.flush().unwrap();
    let complete = writer.into_inner();

    let mut prefixes = Vec::new();
    let mut prefix = Vec::new();
    for line in complete.split_inclusive(|byte| *byte == b'\n') {
        prefix.extend_from_slice(line);
        let value: serde_json::Value = serde_json::from_slice(line).unwrap();
        if value["record"] == "tick" {
            prefixes.push(prefix.clone());
        }
    }
    assert!(prefixes.len() >= 2, "test log has multiple completed ticks");
    let first = &prefixes[0];
    let second = &prefixes[1];
    let second_segment = &second[first.len()..];
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "turts-live-audit-{}-{nonce}.jsonl",
        std::process::id()
    ));

    std::fs::write(&path, first).unwrap();
    let half = second_segment.len() / 2;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    file.write_all(&second_segment[..half]).unwrap();
    file.flush().unwrap();
    let decoded = read_live_complete_tick_prefix(&path).unwrap();
    assert_eq!(
        decoded
            .iter()
            .filter(|record| matches!(record, EconomyLogRecord::Tick { .. }))
            .count(),
        1,
        "unterminated JSON tail is excluded"
    );
    drop(file);

    let path_for_writer = path.clone();
    let completion = second_segment[half..].to_vec();
    let append = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(60));
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(path_for_writer)
            .unwrap();
        file.write_all(&completion).unwrap();
        file.flush().unwrap();
    });
    let decoded = wait_for_audit_tick(&path, 2);
    append.join().unwrap();
    assert!(
        decoded
            .iter()
            .any(|record| matches!(record, EconomyLogRecord::Tick { tick: 2, .. }))
    );

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    file.write_all(b"{malformed complete record}\n").unwrap();
    file.flush().unwrap();
    assert!(
        read_live_complete_tick_prefix(&path)
            .unwrap_err()
            .contains("decode complete JSONL line"),
        "malformed newline-terminated records remain fatal even after a valid tick"
    );
    let last_state = decoded
        .iter()
        .rev()
        .find_map(|record| match record {
            EconomyLogRecord::Tick { state, .. } => Some(state.clone()),
            _ => None,
        })
        .unwrap();
    let input = turts_sim::Command {
        slot: turts_sim::SlotId(0),
        seq: 77,
        kind: turts_sim::CommandKind::MoveTo {
            unit_ids: vec![1],
            target: bevy_math::Vec2::new(2.0, 2.0),
        },
    };
    let open_boundary = EconomyLogRecord::CommandBoundary(turts_sim::CommandAudit {
        operation_id: 900,
        operation_end_id: None,
        result: turts_sim::CommandResult {
            slot: input.slot,
            seq: input.seq,
            outcome: Err(turts_sim::RejectReason::EmptyCommand),
            units: Default::default(),
        },
        input,
        before: last_state.clone(),
        after: last_state,
    });
    let invalid_claim = EconomyLogRecord::Claim {
        tick: 999,
        player_id: 99,
        slot: 0,
    };
    let semantic_path = path.with_file_name(format!("turts-live-audit-semantic-{nonce}.jsonl"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&semantic_path)
        .unwrap();
    file.write_all(second).unwrap();
    for record in [&invalid_claim, &open_boundary] {
        serde_json::to_writer(&mut file, record).unwrap();
        file.write_all(b"\n").unwrap();
    }
    file.flush().unwrap();
    assert!(
        read_live_complete_tick_prefix(&semantic_path)
            .unwrap_err()
            .contains("discontinuous tick"),
        "an invalid complete Claim is not hidden by a later open command"
    );
    drop(file);

    let initial_only_path =
        path.with_file_name(format!("turts-live-audit-initial-only-{nonce}.jsonl"));
    let initial_line = complete
        .split_inclusive(|byte| *byte == b'\n')
        .next()
        .unwrap();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&initial_only_path)
        .unwrap();
    file.write_all(initial_line).unwrap();
    serde_json::to_writer(&mut file, &invalid_claim).unwrap();
    file.write_all(b"\n").unwrap();
    serde_json::to_writer(&mut file, &open_boundary).unwrap();
    file.write_all(b"\n").unwrap();
    file.flush().unwrap();
    assert!(
        read_live_complete_tick_prefix(&initial_only_path)
            .unwrap_err()
            .contains("discontinuous tick"),
        "a complete invalid record before the first Tick is not hidden"
    );

    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(semantic_path).unwrap();
    std::fs::remove_file(initial_only_path).unwrap();
}

#[test]
fn decoded_moving_gatherer_replays_frozen_command_and_automatic_effect() {
    use std::io::Cursor;
    use turts_sim::{
        AuditTrail, Command, CommandKind, CommandQueue, SlotId, economy::views::audit_state,
    };

    let mut app = economy_fixture_app_at(5);
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let expected_command = Command {
        slot: SlotId(0),
        seq: 71,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    let mut writer = economylog::EconomyLogWriter::new(Vec::new());
    {
        let trail = app.world_mut().resource_mut::<AuditTrail>();
        assert!(trail.valid);
        writer.write_initial(&trail, initial.clone()).unwrap();
    }

    // Capture clock advance, command admission, movement, gather operations, and
    // publication. The stock worker starts outside range and must travel before mining.
    for tick_number in 1..=24 {
        if tick_number == 1 {
            app.world_mut()
                .resource_mut::<CommandQueue>()
                .0
                .push_back(expected_command.clone());
        }
        app.update();
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        assert!(trail.valid);
        let commands = trail.drain_commands();
        let operations = trail.drain_operations();
        let tick = trail.drain_ticks().pop().expect("one completed tick");
        assert_eq!(tick.tick, tick_number);
        if tick_number == 1 {
            assert_eq!(tick.before, initial);
            assert_eq!(commands.len(), 1);
            assert_eq!(operations.len(), 3, "command plus both gatherer operations");
            writer.write_command(&trail, commands[0].clone()).unwrap();
        } else {
            assert!(commands.is_empty());
            assert_eq!(operations.len(), 2, "both gatherer operations");
        }
        for operation in operations {
            writer.write_operation(&trail, operation).unwrap();
        }
        writer.write_tick(&trail, tick).unwrap();
    }
    writer.flush().unwrap();

    // Replay only decoded JSONL. The expected command and automatic subjects are
    // frozen from the fixture inputs; observed audit fields are only evidence.
    let decoded = economylog::read_jsonl(Cursor::new(writer.into_inner())).unwrap();
    economylog::validate_records(&decoded).unwrap();
    let decoded_initial = match &decoded[0] {
        economylog::EconomyLogRecord::Initial(state) => state.clone(),
        _ => panic!("first record must be Initial"),
    };
    assert_eq!(decoded_initial, initial);
    let decoded_command = decoded
        .iter()
        .find_map(|record| match record {
            economylog::EconomyLogRecord::CommandBoundary(command) => Some(command.clone()),
            _ => None,
        })
        .expect("decoded command boundary");
    assert_eq!(decoded_command.input, expected_command);

    let mut tick_start = decoded_initial;
    let mut modeled_extraction = 0;
    for tick_number in 1..=24 {
        // R14 anchors TickAudit.before at t-1 while command/operation.before is at t.
        // This predicted start comes only from the previous modeled tick state.
        let modeled_start = if tick_number == 1 {
            advanced_clock(&tick_start, tick_number)
        } else {
            tick_start.clone()
        };
        let mut oracle = accounting::AccountingOracle::new(&modeled_start, &catalog);
        let operations = decoded
            .iter()
            .filter_map(|record| match record {
                economylog::EconomyLogRecord::Operation(operation)
                    if operation.before.economy.tick == tick_number =>
                {
                    Some(operation)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let expected_gatherers = [3_u32, 5_u32];
        let mut automatic_index = 0;
        let mut found_command = false;
        for operation in operations {
            if operation.operation_id == decoded_command.operation_id {
                assert_eq!(tick_number, 1);
                assert!(!found_command, "one frozen command is recorded once");
                assert_eq!(operation.before, modeled_start);
                oracle
                    .observe(
                        &accounting::ExpectedOperation::Command(expected_command.clone()),
                        &operation.before,
                        &operation.after,
                        &operation.events,
                        Some(&decoded_command.result),
                    )
                    .unwrap();
                found_command = true;
            } else {
                let gatherer = *expected_gatherers
                    .get(automatic_index)
                    .expect("fixture automatic subject list");
                automatic_index += 1;
                if tick_number == 2 && gatherer == 3 {
                    let before = tick_start
                        .raw_units
                        .iter()
                        .find(|unit| unit.unit_id == 3)
                        .unwrap();
                    let moved = operation
                        .before
                        .raw_units
                        .iter()
                        .find(|unit| unit.unit_id == 3)
                        .unwrap();
                    assert_ne!(
                        before.pos.x.to_bits(),
                        moved.pos.x.to_bits(),
                        "the worker actually advances toward its node before gather capture"
                    );
                }
                oracle
                    .observe(
                        &accounting::ExpectedOperation::Automatic {
                            tick: tick_number,
                            phase: accounting::AutomaticPhase::Gather,
                            subject: accounting::AutomaticSubject::Gatherer(gatherer),
                        },
                        &operation.before,
                        &operation.after,
                        &operation.events,
                        None,
                    )
                    .unwrap();
            }
        }
        if tick_number == 1 {
            assert!(found_command);
            oracle
                .assert_command_records(std::slice::from_ref(&decoded_command))
                .unwrap();
        }
        assert_eq!(automatic_index, expected_gatherers.len());

        let (tick_before, tick_state, tick_events) = decoded
            .iter()
            .find_map(|record| match record {
                economylog::EconomyLogRecord::Tick {
                    tick,
                    before,
                    state,
                    events,
                } if *tick == tick_number => Some((before, state, events)),
                _ => None,
            })
            .expect("decoded completed tick");
        assert_eq!(tick_before, &tick_start);
        let mut expected_published = oracle.modeled_state().pending_events.clone();
        expected_published.sort_by_key(|event| event.ordinal);
        assert_eq!(tick_events, &expected_published);
        let mut expected_tick_state = oracle.modeled_state().clone();
        expected_tick_state.pending_events.clear();
        expected_tick_state.next_event_ordinal = Some(0);
        assert_eq!(tick_state, &expected_tick_state);
        modeled_extraction += oracle.extraction;
        tick_start = expected_tick_state;
    }
    assert!(
        modeled_extraction > 0,
        "frozen fixture produces independent extraction progress"
    );

    // Mutate records after actual encoding/decoding. Both tick-state and event
    // association defects must be rejected by the typed stream validator.
    let mut bad_tick_state = decoded.clone();
    let tick_record = bad_tick_state
        .iter_mut()
        .find_map(|record| match record {
            economylog::EconomyLogRecord::Tick { state, tick, .. } if *tick == 24 => Some(state),
            _ => None,
        })
        .unwrap();
    tick_record.economy.tick += 1;
    assert!(economylog::validate_records(&bad_tick_state).is_err());

    let mut bad_economy_state = decoded.clone();
    let worker_inventory = bad_economy_state
        .iter_mut()
        .find_map(|record| match record {
            economylog::EconomyLogRecord::Tick { state, tick, .. } if *tick == 24 => state
                .economy
                .inventories
                .iter_mut()
                .find(|inventory| inventory.unit == 3),
            _ => None,
        })
        .unwrap();
    worker_inventory.items[0].on_hand += 1;
    assert!(economylog::validate_records(&bad_economy_state).is_err());

    let mut missing_event = decoded.clone();
    let tick_record = missing_event
        .iter_mut()
        .find_map(|record| match record {
            economylog::EconomyLogRecord::Tick { events, tick, .. } if *tick == 24 => Some(events),
            _ => None,
        })
        .unwrap();
    tick_record.clear();
    assert!(economylog::validate_records(&missing_event).is_err());
}

#[test]
fn initial_snapshot_round_trip_detects_a_lost_field() {
    let mut app = turts_sim::headless_app(10);
    let state = turts_sim::audit_state(app.world_mut());
    let encoded = serde_json::to_value(&state).unwrap();
    let mut lossy = encoded.clone();
    lossy.as_object_mut().unwrap().remove("claims");
    assert!(serde_json::from_value::<turts_sim::AuditState>(lossy).is_err());
    assert_eq!(
        serde_json::from_value::<turts_sim::AuditState>(encoded).unwrap(),
        state
    );
}

#[test]
fn decoded_fractional_movement_uses_captured_fixed_timestep() {
    use bevy_math::Vec2;
    use std::io::Cursor;
    use turts_sim::{
        AuditTrail, Command, CommandKind, CommandQueue, Position, SlotId, UnitId,
        economy::views::audit_state,
    };

    let mut app = economy_fixture_app_at(30);
    let worker_entity = app
        .world()
        .iter_entities()
        .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 3))
        .expect("stock fixture worker")
        .id();
    app.world_mut()
        .entity_mut(worker_entity)
        .insert(Position(Vec2::new(0.01, 1.0)));
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let initial = audit_state(app.world_mut());
    let command = Command {
        slot: SlotId(0),
        seq: 72,
        kind: CommandKind::MoveTo {
            unit_ids: vec![3],
            target: Vec2::new(24.0, 1.0),
        },
    };
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(command);
    app.update();

    let mut trail = app.world_mut().resource_mut::<AuditTrail>();
    assert!(trail.valid);
    let commands = trail.drain_commands();
    let operations = trail.drain_operations();
    let tick = trail.drain_ticks().pop().expect("captured fixed tick");
    assert_eq!(commands.len(), 1);
    assert_eq!(operations.len(), 3);
    assert_eq!(tick.tick, 1);
    let mut writer = economylog::EconomyLogWriter::new(Vec::new());
    writer.write_initial(&trail, initial).unwrap();
    writer.write_command(&trail, commands[0].clone()).unwrap();
    for operation in operations {
        writer.write_operation(&trail, operation).unwrap();
    }
    writer.write_tick(&trail, tick).unwrap();
    writer.flush().unwrap();

    let decoded = economylog::read_jsonl(Cursor::new(writer.into_inner())).unwrap();
    let movement = decoded
        .iter()
        .find_map(|record| match record {
            economylog::EconomyLogRecord::Tick { state, .. } => state
                .raw_units
                .iter()
                .find(|unit| unit.unit_id == 3)
                .map(|unit| unit.pos.x),
            _ => None,
        })
        .expect("decoded worker movement");
    let fixed_delta = std::time::Duration::from_secs_f64(1.0 / 30.0).as_secs_f32();
    assert_eq!(movement.to_bits(), (0.01_f32 + 3.0 * fixed_delta).to_bits());

    let mut corrupted = decoded;
    let moved = corrupted
        .iter_mut()
        .find_map(|record| match record {
            economylog::EconomyLogRecord::Tick { state, .. } => {
                state.raw_units.iter_mut().find(|unit| unit.unit_id == 3)
            }
            _ => None,
        })
        .unwrap();
    moved.pos.x = f32::from_bits(moved.pos.x.to_bits() + 1);
    assert!(economylog::validate_records(&corrupted).is_err());
}

#[derive(Clone)]
enum FrozenKind {
    MoveTo {
        units: Vec<u32>,
        target_bits: [u32; 2],
    },
    Gather {
        unit: u32,
        node: u32,
    },
    Produce {
        producer: u32,
        recipe: &'static str,
    },
    CancelFirstJob {
        producer: u32,
    },
    Cancel {
        producer: u32,
        job: u64,
    },
}

#[derive(Clone)]
struct FrozenCommand {
    slot: u8,
    seq: u32,
    kind: FrozenKind,
}

fn independent_public_model(
    package_name: &str,
) -> (turts_sim::AuditState, turts_mod_api::ContentCatalog) {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../mods")
        .join(package_name);
    let mut files = std::collections::BTreeMap::new();
    package_files(&root, &root, &mut files);
    let archive_bytes = turts_mod_runtime::pack::pack_files(&files).unwrap();
    independent_public_model_from_archive(&archive_bytes, 20)
}

fn independent_public_model_from_archive(
    archive_bytes: &[u8],
    tick_hz: u16,
) -> (turts_sim::AuditState, turts_mod_api::ContentCatalog) {
    use turts_sim::economy::views::audit_state;
    let archive = turts_mod_runtime::archive::PackageArchive::parse(archive_bytes).unwrap();
    let mut app = turts_sim::headless_app(tick_hz);
    turts_pack::load_package(app.world_mut(), &archive).unwrap();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    (initial, catalog)
}

fn pack_e8_validation_variant(into: &std::path::Path) -> std::path::PathBuf {
    let root = support::mods_dir().join("base");
    let mut files = std::collections::BTreeMap::new();
    package_files(&root, &root, &mut files);

    let manifest = String::from_utf8(files.get("manifest.ron").unwrap().clone()).unwrap();
    let manifest = manifest
        .replace(
            "visuals: [\"visuals/command_center.ron\", \"visuals/harvester.ron\", \"visuals/rifleman.ron\", \"visuals/ore.ron\"],",
            "visuals: [\"visuals/command_center.ron\", \"visuals/harvester.ron\", \"visuals/rifleman.ron\", \"visuals/ore.ron\", \"visuals/timber.ron\"],",
        )
        .replace(
            "resources: [\"resources/ore.ron\", \"resources/credits.ron\"],",
            "resources: [\"resources/ore.ron\", \"resources/credits.ron\", \"resources/tokens.ron\", \"resources/timber.ron\"],",
        )
        .replace(
            "nodes: [\"nodes/ore.ron\"],",
            "nodes: [\"nodes/ore.ron\", \"nodes/timber.ron\"],",
        )
        .replace(
            "production: [\"production/rifleman.ron\", \"production/harvester.ron\"],",
            "production: [\"production/rifleman.ron\", \"production/harvester.ron\", \"production/free_hire.ron\"],",
        );
    files.insert("manifest.ron".into(), manifest.into_bytes());

    let center = String::from_utf8(files.get("units/command_center.ron").unwrap().clone()).unwrap();
    assert!(center.contains("recipes: [\"base:train_rifleman\", \"base:build_harvester\"]"));
    files.insert(
        "units/command_center.ron".into(),
        center
            .replace(
                "recipes: [\"base:train_rifleman\", \"base:build_harvester\"]",
                "recipes: [\"base:build_harvester\"]",
            )
            .into_bytes(),
    );

    files.insert(
        "resources/tokens.ron".into(),
        b"(id: \"tokens\", name: \"Tokens\", unit_name: \"token\", kind: Balance)".to_vec(),
    );
    files.insert(
        "resources/timber.ron".into(),
        b"(id: \"timber\", name: \"Timber\", unit_name: \"timber\", kind: Item)".to_vec(),
    );
    files.insert(
        "nodes/timber.ron".into(),
        b"(id: \"timber_deposit\", name: \"Timber Deposit\", item: \"base:timber\", radius: 1.0)"
            .to_vec(),
    );
    files.insert(
        "visuals/timber.ron".into(),
        b"(target: Node(\"base:timber_deposit\"), shape: Cuboid(x: 2.0, y: 1.0, z: 2.0))".to_vec(),
    );

    let map = String::from_utf8(files.get("maps/flat.ron").unwrap().clone()).unwrap();
    let map = map
        .replace(
            "(balance: \"base:credits\", amount: 500)",
            "(balance: \"base:credits\", amount: 500), (balance: \"base:tokens\", amount: 0)",
        )
        .replace(
            "    ],\n    slot_economy:",
            "        (node: \"base:timber_deposit\", pos: (x: 32.0, y: 32.0), remaining: 100),\n    ],\n    slot_economy:",
        );
    files.insert("maps/flat.ron".into(), map.into_bytes());

    files.insert(
        "production/harvester.ron".into(),
        b"(id: \"build_harvester\", name: \"Build Harvester\", output_unit: \"base:harvester\", duration_ms: 15000, costs: [(resource: \"base:credits\", amount: 100, source: SlotBalance), (resource: \"base:tokens\", amount: 1, source: SlotBalance)], payment: ChargeOnQueue, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 2), output_blocked: (numerator: 1, denominator: 1)))".to_vec(),
    );
    let center = String::from_utf8(files.get("units/command_center.ron").unwrap().clone())
        .expect("public command center definition is UTF-8")
        .replace(
            "\"base:build_harvester\"], queue_capacity:",
            "\"base:build_harvester\", \"base:free_hire\"], queue_capacity:",
        );
    assert!(center.contains("base:free_hire"));
    files.insert("units/command_center.ron".into(), center.into_bytes());
    files.insert(
        "production/free_hire.ron".into(),
        b"(id: \"free_hire\", name: \"Free Hire\", output_unit: \"base:harvester\", duration_ms: 60000, costs: [], payment: ReserveThenCharge, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1)))".to_vec(),
    );

    let bytes = turts_mod_runtime::pack::pack_files(&files).unwrap();
    turts_mod_runtime::archive::PackageArchive::parse(&bytes)
        .expect("the E8 multi-cost/incompatible-node public package is valid");
    std::fs::create_dir_all(into).unwrap();
    let path = into.join("base-e8-variant.tar");
    std::fs::write(&path, bytes).unwrap();
    path
}

fn sim_command(input: &FrozenCommand, state: &turts_sim::AuditState) -> turts_sim::Command {
    use turts_sim::{Command, CommandKind, SlotId};
    let kind = match &input.kind {
        FrozenKind::MoveTo { units, target_bits } => CommandKind::MoveTo {
            unit_ids: units.clone(),
            target: bevy_math::Vec2::new(
                f32::from_bits(target_bits[0]),
                f32::from_bits(target_bits[1]),
            ),
        },
        FrozenKind::Gather { unit, node } => CommandKind::Gather {
            unit_id: *unit,
            node_id: turts_sim::NodeId(*node),
        },
        FrozenKind::Produce { producer, recipe } => CommandKind::Produce {
            producer_id: *producer,
            recipe_id: turts_mod_api::ContentId::parse(recipe).unwrap(),
        },
        FrozenKind::CancelFirstJob { producer } => {
            let job = state
                .economy
                .queues
                .iter()
                .find(|queue| queue.producer == *producer)
                .and_then(|queue| queue.jobs.first())
                .expect("prior model has the referenced produced job");
            CommandKind::CancelProduction {
                producer_id: *producer,
                job_id: turts_sim::ids::JobId(job.id),
            }
        }
        FrozenKind::Cancel { producer, job } => CommandKind::CancelProduction {
            producer_id: *producer,
            job_id: turts_sim::ids::JobId(*job),
        },
    };
    Command {
        slot: SlotId(input.slot),
        seq: input.seq,
        kind,
    }
}

fn command_input_matches(expected: &turts_sim::Command, actual: &turts_sim::Command) -> bool {
    use turts_sim::CommandKind;
    if expected.slot != actual.slot || expected.seq != actual.seq {
        return false;
    }
    match (&expected.kind, &actual.kind) {
        (
            CommandKind::MoveTo {
                unit_ids: expected_units,
                target: expected_target,
            },
            CommandKind::MoveTo {
                unit_ids: actual_units,
                target: actual_target,
            },
        ) => {
            expected_units == actual_units
                && expected_target.x.to_bits() == actual_target.x.to_bits()
                && expected_target.y.to_bits() == actual_target.y.to_bits()
        }
        (
            CommandKind::Gather {
                unit_id: expected_unit,
                node_id: expected_node,
            },
            CommandKind::Gather {
                unit_id: actual_unit,
                node_id: actual_node,
            },
        ) => expected_unit == actual_unit && expected_node == actual_node,
        (
            CommandKind::Produce {
                producer_id: expected_producer,
                recipe_id: expected_recipe,
            },
            CommandKind::Produce {
                producer_id: actual_producer,
                recipe_id: actual_recipe,
            },
        ) => expected_producer == actual_producer && expected_recipe == actual_recipe,
        (
            CommandKind::CancelProduction {
                producer_id: expected_producer,
                job_id: expected_job,
            },
            CommandKind::CancelProduction {
                producer_id: actual_producer,
                job_id: actual_job,
            },
        ) => expected_producer == actual_producer && expected_job == actual_job,
        _ => false,
    }
}

fn order_input_matches_frozen(
    expected: &FrozenCommand,
    actual: &turts_sim::Command,
    initial: &turts_sim::AuditState,
) -> bool {
    use turts_mod_api::Behaviour;
    use turts_sim::{CommandKind, OrderTargetSim, QueueMode};
    let CommandKind::Order(order) = &actual.kind else {
        return false;
    };
    if actual.slot.0 != expected.slot || actual.seq != expected.seq {
        return false;
    }
    let (order_id, behaviour, units, target) = match &expected.kind {
        FrozenKind::MoveTo { units, target_bits } => (
            "base:order_move",
            Behaviour::Move,
            units.clone(),
            OrderTargetSim::Point(bevy_math::Vec2::new(
                f32::from_bits(target_bits[0]),
                f32::from_bits(target_bits[1]),
            )),
        ),
        FrozenKind::Gather { unit, node } => (
            "base:order_gather",
            Behaviour::Gather,
            vec![*unit],
            OrderTargetSim::Node(turts_sim::NodeId(*node)),
        ),
        FrozenKind::Produce { producer, recipe } => (
            "base:order_produce",
            Behaviour::Produce,
            vec![*producer],
            OrderTargetSim::Content(turts_mod_api::ContentId::parse(recipe).unwrap()),
        ),
        FrozenKind::CancelFirstJob { producer } => {
            let first_job = initial.allocators.next_values().2;
            (
                "base:order_cancel",
                Behaviour::CancelProduction,
                vec![*producer],
                OrderTargetSim::Job(turts_sim::JobId(first_job)),
            )
        }
        FrozenKind::Cancel { producer, job } => (
            "base:order_cancel",
            Behaviour::CancelProduction,
            vec![*producer],
            OrderTargetSim::Job(turts_sim::JobId(*job)),
        ),
    };
    let target_matches = match (&order.target, &target) {
        (OrderTargetSim::Point(actual), OrderTargetSim::Point(expected)) => {
            actual.x.to_bits() == expected.x.to_bits() && actual.y.to_bits() == expected.y.to_bits()
        }
        (actual, expected) => actual == expected,
    };
    order.order.to_string() == order_id
        && order.behaviour == behaviour
        && order.units == units
        && target_matches
        && order.queue == QueueMode::Replace
        && order.group == 0
}

fn replay_task4_order_log(
    records: &[economylog::EconomyLogRecord],
    expected_initial: &turts_sim::AuditState,
    expected_commands: &[FrozenCommand],
    expected_claims: &[(u8, u64)],
) {
    use economylog::EconomyLogRecord;

    economylog::validate_records(records).unwrap_or_else(|error| {
        panic!("Task4 catalog-derived order replay rejected the process log: {error}")
    });
    let initial = records
        .iter()
        .find_map(|record| match record {
            EconomyLogRecord::Initial(state) => Some(state),
            _ => None,
        })
        .expect("process log has its captured initial state");
    assert_eq!(
        initial, expected_initial,
        "process boot matches public catalog"
    );

    let mut actual_claims = records
        .iter()
        .filter_map(|record| match record {
            EconomyLogRecord::Claim {
                player_id, slot, ..
            } => Some((*slot, *player_id)),
            _ => None,
        })
        .collect::<Vec<_>>();
    actual_claims.sort_unstable();
    let mut expected_claims = expected_claims
        .iter()
        .map(|(slot, player_id)| (*slot, *player_id))
        .collect::<Vec<_>>();
    expected_claims.sort_unstable();
    assert_eq!(
        actual_claims, expected_claims,
        "process Hello claims match fixture"
    );

    let commands = records
        .iter()
        .filter_map(|record| match record {
            EconomyLogRecord::CommandBoundary(command) => Some(command),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        commands.len(),
        expected_commands.len(),
        "every admitted frozen envelope has exactly one command boundary"
    );
    let mut unmatched = expected_commands.iter().collect::<Vec<_>>();
    for command in commands {
        let index = unmatched
            .iter()
            .position(|expected| {
                expected.slot == command.input.slot.0 && expected.seq == command.input.seq
            })
            .expect("every domain command maps to a frozen protocol envelope");
        let expected = unmatched.remove(index);
        assert!(
            order_input_matches_frozen(expected, &command.input, expected_initial),
            "slot {} seq {} matches its frozen order, target, subjects, queue and group",
            expected.slot,
            expected.seq
        );
    }
    assert!(
        unmatched.is_empty(),
        "every expected domain command was captured"
    );
}

fn mutate_order_command_after(
    records: &mut [economylog::EconomyLogRecord],
    seq: u32,
    mut mutate: impl FnMut(&mut turts_sim::AuditState),
) {
    use economylog::EconomyLogRecord;
    let operation_id = records
        .iter()
        .find_map(|record| match record {
            EconomyLogRecord::CommandBoundary(command) if command.input.seq == seq => {
                Some(command.operation_id)
            }
            _ => None,
        })
        .expect("fixture command boundary exists");
    let mut changed_boundary = false;
    let mut changed_operation = false;
    for record in records {
        match record {
            EconomyLogRecord::CommandBoundary(command) if command.operation_id == operation_id => {
                mutate(&mut command.after);
                changed_boundary = true;
            }
            EconomyLogRecord::Operation(operation) if operation.operation_id == operation_id => {
                mutate(&mut operation.after);
                changed_operation = true;
            }
            _ => {}
        }
    }
    assert!(changed_boundary && changed_operation);
}

fn protocol_command(input: &FrozenCommand) -> turts_net_protocol::PlayerCommand {
    use turts_net_protocol::{OrderEnvelope, OrderTarget, PlayerCommand, Pos, QueueMode, UnitId};
    let (order, units, target) = match &input.kind {
        FrozenKind::MoveTo { units, target_bits } => (
            "base:order_move",
            units.iter().copied().map(UnitId).collect(),
            OrderTarget::Point(Pos {
                x: f32::from_bits(target_bits[0]),
                y: f32::from_bits(target_bits[1]),
            }),
        ),
        FrozenKind::Produce { producer, recipe } => (
            "base:order_produce",
            vec![UnitId(*producer)],
            OrderTarget::Content((*recipe).to_owned()),
        ),
        FrozenKind::Gather { unit, node } => (
            "base:order_gather",
            vec![UnitId(*unit)],
            OrderTarget::Node(turts_net_protocol::NodeId(*node)),
        ),
        FrozenKind::Cancel { producer, job } => (
            "base:order_cancel",
            vec![UnitId(*producer)],
            OrderTarget::Job(turts_net_protocol::JobId(*job)),
        ),
        FrozenKind::CancelFirstJob { .. } => {
            panic!("trace-only dynamic cancellation is not a static wire fixture")
        }
    };
    PlayerCommand {
        seq: input.seq,
        order: OrderEnvelope {
            order: order.into(),
            units,
            target,
            queue: QueueMode::Replace,
            group: 0,
        },
    }
}

fn unit_id_for_owner_and_content(
    initial: &turts_sim::AuditState,
    slot: u8,
    content_id: &str,
) -> u32 {
    initial
        .economy
        .units
        .iter()
        .find(|unit| unit.owner_slot == slot && unit.content_id == content_id)
        .map(|unit| unit.unit_id)
        .expect("frozen public boot world has the requested owner/content unit")
}

fn assert_ack_before_every_rejection(report: &wire_sender::Report) {
    for attempt in &report.attempts {
        let end = attempt
            .terminal_cursor
            .expect("attempt has terminal result");
        let terminal = &report.receipts[end];
        if terminal.kind != "rejected" {
            continue;
        }
        let ack = report
            .receipts
            .get(
                attempt
                    .ack_cursor
                    .expect("attempt captured its actual Ack cursor"),
            )
            .filter(|receipt| receipt.seq == attempt.seq && receipt.kind == "ack")
            .expect("each admitted rejected envelope has its receipt Ack");
        assert!(ack.reliable && terminal.reliable);
        assert!(
            (1..32_768).contains(
                &terminal
                    .message_id
                    .unwrap()
                    .wrapping_sub(ack.message_id.unwrap())
            ),
            "actual ReliableChannel MessageIds put Ack before rejection for seq {}",
            attempt.seq
        );
    }
}

fn command_attempt_receipt<'a>(
    report: &'a wire_sender::Report,
    attempt_index: usize,
    kind: &str,
) -> &'a wire_sender::Receipt {
    let attempt = report
        .attempts
        .get(attempt_index)
        .expect("configured command has one actual send interval");
    let end = report
        .attempts
        .get(attempt_index + 1)
        .map_or(report.receipts.len(), |next| next.start_cursor);
    assert!(end >= attempt.start_cursor);
    report.receipts[attempt.start_cursor..end]
        .iter()
        .find(|receipt| receipt.seq == attempt.seq && receipt.kind == kind)
        .expect("receipt belongs to this command's fresh send interval")
}

#[derive(Debug, Default)]
struct ReplaySummary {
    extraction: u128,
    delivery: u128,
    conversion: u128,
    charge: u128,
    refunds: u128,
    outputs: u64,
    cancellations: u64,
}

fn wait_for_owner_rifle_output(path: &std::path::Path) -> (u32, u32) {
    use economylog::EconomyLogRecord;
    use std::time::{Duration, Instant};

    let started = Instant::now();
    loop {
        let records = read_live_complete_tick_prefix(path)
            .unwrap_or_else(|error| panic!("decode complete live audit prefix: {error}"));
        for record in &records {
            let EconomyLogRecord::Tick { tick, state, .. } = record else {
                continue;
            };
            if let Some(unit) = state
                .economy
                .units
                .iter()
                .find(|unit| unit.content_id == "base:rifleman" && unit.owner_slot == 0)
            {
                return (*tick, unit.unit_id);
            }
        }
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "owner 0's first rifleman appears in a completed, flushed audit tick"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn audit_state_at(path: &std::path::Path, tick: u32) -> turts_sim::AuditState {
    use economylog::EconomyLogRecord;
    let records = wait_for_audit_tick(path, tick);
    let matching: Vec<_> = records
        .iter()
        .filter_map(|record| match record {
            EconomyLogRecord::Tick {
                tick: found, state, ..
            } if *found == tick => Some(state),
            _ => None,
        })
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "one validated completed audit tick {tick}"
    );
    matching[0].clone()
}

fn audit_last_tick(path: &std::path::Path) -> u32 {
    use economylog::EconomyLogRecord;
    read_complete_economy_log(path)
        .expect("decode complete economy audit log")
        .iter()
        .filter_map(|record| match record {
            EconomyLogRecord::Tick { tick, .. } => Some(*tick),
            _ => None,
        })
        .max()
        .expect("audit log has completed ticks")
}

fn owner_unit_orders_from_audit(
    state: &turts_sim::AuditState,
    slot: u8,
) -> Vec<turts_net_protocol::UnitOrderRecord> {
    use turts_mod_api::Behaviour;
    use turts_net_protocol as wire;
    use turts_sim::{OrderTargetSim, RejectReason as R};

    fn reject_reason(reason: R) -> wire::CommandRejectReason {
        use wire::CommandRejectReason as W;
        match reason {
            R::EmptyCommand => W::EmptyCommand,
            R::TooManyUnits => W::TooManyUnits,
            R::DuplicateUnit => W::DuplicateUnit,
            R::UnknownUnit => W::UnknownUnit,
            R::NotOwner => W::NotOwner,
            R::NotMobile => W::NotMobile,
            R::TargetNotFinite => W::TargetNotFinite,
            R::TargetOutsideMap => W::TargetOutsideMap,
            R::MissingCapability => W::MissingCapability,
            R::UnknownNode => W::UnknownNode,
            R::IncompatibleItem => W::IncompatibleItem,
            R::NodeDepleted => W::NodeDepleted,
            R::UnknownRecipe => W::UnknownRecipe,
            R::RecipeNotEnabled => W::RecipeNotEnabled,
            R::QueueFull => W::QueueFull,
            R::InsufficientBalance | R::UnknownBalance => W::InsufficientBalance,
            R::InsufficientItems | R::UnknownInventory | R::InvalidAmount => W::InsufficientItems,
            R::UnknownJob => W::UnknownJob,
            R::RefundDestinationFull => W::RefundDestinationFull,
            R::RefundDestinationOverflow => W::RefundDestinationOverflow,
            R::ArithmeticOverflow => W::ArithmeticOverflow,
            R::IdExhausted => W::IdExhausted,
            R::BadSequence => W::BadSequence,
            R::NotRemovable => W::NotRemovable,
            R::NotToggleable => W::NotToggleable,
            R::PrerequisitesMissing => W::PrerequisitesMissing,
            R::AlreadyResearched => W::AlreadyResearched,
            R::ResearchInProgress => W::ResearchInProgress,
            R::IntegrityViolation => W::Internal,
            R::NotApplicable => W::NotApplicable,
            R::TargetInvalid => W::TargetInvalid,
            R::TargetGone => W::TargetGone,
            R::OutOfRange => W::OutOfRange,
            R::NotQueueable => W::NotQueueable,
            R::Blocked => W::Blocked,
        }
    }

    let target = |target: &OrderTargetSim| match target {
        OrderTargetSim::Invalid => panic!("invalid order target in captured unit state"),
        OrderTargetSim::None => wire::OrderTarget::None,
        OrderTargetSim::Point(point) => wire::OrderTarget::Point(wire::Pos {
            x: point.x,
            y: point.y,
        }),
        OrderTargetSim::Unit(unit) => wire::OrderTarget::Unit(wire::UnitId(*unit)),
        OrderTargetSim::Node(node) => wire::OrderTarget::Node(wire::NodeId(node.0)),
        OrderTargetSim::Job(job) => wire::OrderTarget::Job(wire::JobId(job.0)),
        OrderTargetSim::Content(content) => wire::OrderTarget::Content(content.to_string()),
    };
    let summary = |order: &turts_sim::Order| wire::OrderSummary {
        order: order.order.to_string(),
        target: target(&order.target),
    };

    state
        .raw_units
        .iter()
        .filter(|unit| unit.owner_slot == slot)
        .filter_map(|unit| {
            let unit_type = state
                .unit_types
                .iter()
                .find(|kind| kind.type_id == unit.unit_type)?;
            let orders = state
                .unit_orders
                .iter()
                .find(|orders| orders.unit_id == unit.unit_id)
                .map(|orders| orders.orders.clone())
                .unwrap_or_default();
            let owns_order_component = !orders.eq(&turts_sim::UnitOrders::default())
                || state.catalog.as_ref().is_some_and(|catalog| {
                    catalog.order_definitions.iter().any(|definition| {
                        definition.unit_capabilities.iter().any(|capability| {
                            capability.unit == unit_type.content_id
                                && (capability.movement || capability.weapon)
                        })
                    })
                });
            if !owns_order_component {
                return None;
            }
            let activity = if orders
                .last_failure
                .as_ref()
                .is_some_and(|failure| failure.reason == R::Blocked)
            {
                wire::Activity::Blocked
            } else if let Some(current) = &orders.current {
                match current.behaviour {
                    Behaviour::Move if unit.move_target.is_some() => wire::Activity::Moving,
                    Behaviour::Move => wire::Activity::Pending,
                    Behaviour::Attack => wire::Activity::Attacking,
                    Behaviour::Hold => wire::Activity::Holding,
                    Behaviour::Gather => wire::Activity::Gathering,
                    _ => wire::Activity::Pending,
                }
            } else if !orders.queue.is_empty() {
                wire::Activity::Pending
            } else if unit.move_target.is_some() {
                wire::Activity::Moving
            } else {
                wire::Activity::Idle
            };
            Some(wire::UnitOrderRecord {
                unit: wire::UnitId(unit.unit_id),
                activity,
                current: orders.current.as_ref().map(summary),
                queued: orders.queue.iter().map(summary).collect(),
                last_failure: orders
                    .last_failure
                    .as_ref()
                    .map(|failure| wire::OrderFailure {
                        order: failure.order.to_string(),
                        reason: reject_reason(failure.reason),
                        tick: failure.tick,
                    }),
            })
        })
        .collect()
}

fn owner_snapshot_from_audit(
    state: &turts_sim::AuditState,
    slot: u8,
) -> turts_net_protocol::Snapshot {
    use turts_mod_api::Cost;
    use turts_net_protocol as wire;
    use turts_sim::economy::gather::{GatherStage as SimGatherStage, WaitReason};
    use turts_sim::economy::production::JobStage as SimJobStage;

    let world = &state.economy;
    wire::Snapshot {
        tick: world.tick,
        units: world
            .units
            .iter()
            .map(|unit| wire::UnitSnapshot {
                unit_id: wire::UnitId(unit.unit_id),
                content_id: unit.content_id.clone(),
                owner_slot: wire::SlotId(unit.owner_slot),
                pos: wire::Pos {
                    x: unit.pos.x,
                    y: unit.pos.y,
                },
                facing: unit.facing,
            })
            .collect(),
        nodes: world
            .nodes
            .iter()
            .map(|node| wire::NodeState {
                id: wire::NodeId(node.id.0),
                content_id: node.content_id.to_string(),
                pos: wire::Pos {
                    x: node.pos.x,
                    y: node.pos.y,
                },
                remaining: node.remaining,
            })
            .collect(),
        economy: wire::EconomyState {
            balances: world
                .economy
                .balances
                .iter()
                .filter(|balance| balance.slot == slot)
                .map(|balance| wire::BalanceState {
                    resource: balance.resource.to_string(),
                    on_hand: balance.on_hand,
                    reserved: balance.reserved,
                })
                .collect(),
            inventories: world
                .economy
                .inventories
                .iter()
                .filter(|inventory| inventory.owner_slot == slot)
                .map(|inventory| wire::InventoryState {
                    unit: wire::UnitId(inventory.unit),
                    items: inventory
                        .items
                        .iter()
                        .map(|item| wire::BalanceState {
                            resource: item.resource.to_string(),
                            on_hand: item.on_hand,
                            reserved: item.reserved,
                        })
                        .collect(),
                    port: wire::InventoryPort::Main,
                })
                .collect(),
            orders: world
                .economy
                .orders
                .iter()
                .filter(|order| order.owner_slot == slot)
                .map(|order| wire::OrderState {
                    unit: wire::UnitId(order.unit),
                    node: order.node.map(|node| wire::NodeId(node.0)),
                    receiver: order.receiver.map(wire::UnitId),
                    stage: match order.stage {
                        SimGatherStage::MovingToNode => wire::GatherStage::ToNode,
                        SimGatherStage::Gathering => wire::GatherStage::Gathering,
                        SimGatherStage::MovingToReceiver => wire::GatherStage::ToDropOff,
                        SimGatherStage::Unloading => wire::GatherStage::Unloading,
                        SimGatherStage::Idle => wire::GatherStage::Idle,
                        SimGatherStage::Waiting { reason } => wire::GatherStage::Waiting {
                            reason: match reason {
                                WaitReason::NoReceiver => wire::GatherWaitReason::NoDropOff,
                                WaitReason::NoDeposit => wire::GatherWaitReason::NoDeposit,
                                WaitReason::DestinationFull => {
                                    wire::GatherWaitReason::DestinationFull
                                }
                                WaitReason::DestinationOverflow => {
                                    wire::GatherWaitReason::DestinationOverflow
                                }
                            },
                        },
                    },
                })
                .collect(),
            queues: world
                .economy
                .queues
                .iter()
                .filter(|queue| queue.owner_slot == slot)
                .map(|queue| wire::QueueState {
                    producer: wire::UnitId(queue.producer),
                    jobs: queue
                        .jobs
                        .iter()
                        .map(|job| wire::JobState {
                            id: wire::JobId(job.id),
                            recipe: job.recipe.to_string(),
                            stage: match job.stage {
                                SimJobStage::Queued => wire::JobStage::Queued,
                                SimJobStage::Running => wire::JobStage::Running,
                                SimJobStage::OutputBlocked => wire::JobStage::OutputBlocked,
                            },
                            elapsed_ticks: job.elapsed_ticks,
                            duration_ticks: job.duration_ticks,
                            charged: job
                                .charged
                                .iter()
                                .map(|line| Cost {
                                    source: line.source,
                                    resource: line.resource.to_string(),
                                    amount: line.amount,
                                })
                                .collect(),
                            reserved: job
                                .reserved
                                .iter()
                                .map(|line| Cost {
                                    source: line.source,
                                    resource: line.resource.to_string(),
                                    amount: line.amount,
                                })
                                .collect(),
                        })
                        .collect(),
                    enabled: true,
                })
                .collect(),
        },
        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
        unit_orders: owner_unit_orders_from_audit(state, slot),
    }
}

fn owner_report_projection_from_audit(
    state: &turts_sim::AuditState,
    slot: u8,
) -> serde_json::Value {
    let snapshot = owner_snapshot_from_audit(state, slot);
    serde_json::json!({
        "tick": snapshot.tick,
        "units": snapshot.units.iter().map(|unit| serde_json::json!({
            "unit_id": unit.unit_id.0,
            "content_id": unit.content_id,
            "owner_slot": unit.owner_slot.0,
            "facing": unit.facing,
            "pos": [unit.pos.x, unit.pos.y],
        })).collect::<Vec<_>>(),
        "nodes": snapshot.nodes,
        "economy": snapshot.economy,
        // M2 has no M3 factory/combat state; retain explicit empty wire families so later
        // nonempty additions still diverge from this independently audit-derived projection.
        "belts": snapshot.belts,
        "inserters": snapshot.inserters,
        "extractors": snapshot.extractors,
        "weapons": snapshot.weapons,
        "healths": snapshot.healths,
        "research": snapshot.research,
        "health_trace": [],
    })
}

fn canonicalize_wire_snapshot(value: &mut serde_json::Value) {
    fn sort_by(array: &mut serde_json::Value, key: &str) {
        if let Some(items) = array.as_array_mut() {
            items.sort_by_key(|item| item[key].to_string());
        }
    }
    sort_by(&mut value["units"], "unit_id");
    sort_by(&mut value["nodes"], "id");
    sort_by(&mut value["economy"]["balances"], "resource");
    sort_by(&mut value["economy"]["inventories"], "unit");
    sort_by(&mut value["economy"]["orders"], "unit");
    sort_by(&mut value["economy"]["queues"], "producer");
    if let Some(inventories) = value["economy"]["inventories"].as_array_mut() {
        for inventory in inventories {
            sort_by(&mut inventory["items"], "resource");
        }
    }
    if let Some(queues) = value["economy"]["queues"].as_array_mut() {
        for queue in queues {
            sort_by(&mut queue["jobs"], "id");
            if let Some(jobs) = queue["jobs"].as_array_mut() {
                for job in jobs {
                    sort_by(&mut job["charged"], "resource");
                    sort_by(&mut job["reserved"], "resource");
                }
            }
        }
    }
}

fn assert_report_matches_own_audit_tick(
    log_path: &std::path::Path,
    report: &serde_json::Value,
) -> u32 {
    let slot = report["slot"].as_u64().expect("client Welcome slot") as u8;
    let tick = report["replicated_world"]["tick"]
        .as_u64()
        .expect("client final replicated tick") as u32;
    let state = audit_state_at(log_path, tick);
    let mut expected = owner_report_projection_from_audit(&state, slot);
    let mut actual = report["replicated_world"].clone();
    canonicalize_wire_snapshot(&mut expected);
    canonicalize_wire_snapshot(&mut actual);
    assert_eq!(
        actual, expected,
        "client slot {slot} full public and private projection matches independently validated audit tick {tick}"
    );
    tick
}

fn assert_delayed_first_snapshot_then_successor(raw_path: &std::path::Path, hold_ticks: u32) {
    let text =
        std::fs::read_to_string(raw_path).expect("client wrote raw response timing evidence");
    let records: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).expect("raw DTO JSONL record decodes"))
        .collect();
    let snapshots: Vec<_> = records
        .iter()
        .enumerate()
        .filter(|(_, record)| record["kind"] == "snapshot")
        .collect();
    assert!(
        snapshots.len() >= 3,
        "initial, delayed response, and successor Snapshot arrived"
    );
    let (first_index, first_response) = snapshots[1];
    let (successor_index, successor) = snapshots[2];
    let first_tick = first_response["dto"]["tick"].as_u64().unwrap();
    let successor_tick = successor["dto"]["tick"].as_u64().unwrap();
    let intervening: Vec<u32> = records[first_index + 1..successor_index]
        .iter()
        .filter(|record| record["kind"] == "world_delta")
        .map(|record| record["dto"]["tick"].as_u64().unwrap() as u32)
        .collect();
    assert!(
        intervening.len() >= hold_ticks as usize,
        "the first response was held while {hold_ticks} WorldDeltas arrived"
    );
    assert!(
        intervening
            .iter()
            .any(|tick| *tick >= first_tick as u32 + hold_ticks),
        "the first held response crossed its configured release tick"
    );
    assert!(
        successor_tick > first_tick,
        "successor response follows the delayed response in server ticks"
    );
}

fn parse_complete_raw_dto_prefix(bytes: &[u8]) -> Result<Vec<serde_json::Value>, String> {
    let Some(last_newline) = bytes.iter().rposition(|byte| *byte == b'\n') else {
        return Ok(Vec::new());
    };
    let complete = std::str::from_utf8(&bytes[..=last_newline])
        .map_err(|error| format!("complete raw DTO prefix is not UTF-8: {error}"))?;
    complete
        .lines()
        .enumerate()
        .map(|(index, line)| {
            if line.is_empty() {
                return Err(format!("raw DTO JSONL line {} is empty", index + 1));
            }
            serde_json::from_str(line).map_err(|error| {
                format!("decode complete raw DTO JSONL line {}: {error}", index + 1)
            })
        })
        .collect()
}

#[test]
fn raw_dto_prefix_rejects_empty_completed_lines_and_ignores_only_unterminated_tail() {
    let complete = br#"{"kind":"snapshot","dto":{"tick":0}}"#;
    let mut bytes = complete.to_vec();
    bytes.push(b'\n');
    bytes.extend_from_slice(br#"{"kind":"world_delta""#);
    let records = parse_complete_raw_dto_prefix(&bytes).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["dto"]["tick"], 0);

    let mut blank_complete_line = complete.to_vec();
    blank_complete_line.extend_from_slice(b"\n\n");
    assert!(
        parse_complete_raw_dto_prefix(&blank_complete_line)
            .unwrap_err()
            .contains("raw DTO JSONL line 2 is empty")
    );
}

fn assert_first_raw_snapshot_matches_owner(
    log_path: &std::path::Path,
    raw_path: &std::path::Path,
    slot: u8,
) {
    use economylog::EconomyLogRecord;
    use std::time::{Duration, Instant};

    let started = Instant::now();
    loop {
        let bytes = match std::fs::read(raw_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => panic!("read raw DTO evidence: {error}"),
        };
        let records = parse_complete_raw_dto_prefix(&bytes)
            .unwrap_or_else(|error| panic!("decode complete raw DTO prefix: {error}"));
        if let Some(record) = records.iter().find(|record| record["kind"] == "snapshot") {
            let dto = &record["dto"];
            let tick = dto["tick"].as_u64().expect("raw Snapshot tick") as u32;
            let audit = read_live_complete_tick_prefix(log_path)
                .unwrap_or_else(|error| panic!("decode complete live audit prefix: {error}"));
            if let Some(state) = audit.iter().find_map(|record| match record {
                EconomyLogRecord::Initial(state) if tick == 0 && state.economy.tick == 0 => {
                    Some(state)
                }
                EconomyLogRecord::Tick {
                    tick: found, state, ..
                } if *found == tick => Some(state),
                _ => None,
            }) {
                let actual: turts_net_protocol::Snapshot = serde_json::from_value(dto.clone())
                    .expect("received Snapshot JSON decodes to its original typed DTO");
                let expected = owner_snapshot_from_audit(state, slot);
                assert_eq!(
                    actual, expected,
                    "raw Snapshot DTO equals independent owner projection at tick {tick}"
                );
                return;
            }
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "first raw Snapshot has a completed authoritative audit tick"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn assert_raw_wire_owner_records(
    log_path: &std::path::Path,
    raw_path: &std::path::Path,
    slot: u8,
    expected_late_output: Option<(u32, u32)>,
) {
    let text = std::fs::read_to_string(raw_path).expect("client wrote raw DTO evidence");
    let records: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).expect("raw DTO JSONL record decodes"))
        .collect();
    assert!(records.iter().any(|record| record["kind"] == "snapshot"));
    assert!(records.iter().any(|record| record["kind"] == "world_delta"));
    for record in &records {
        let kind = record["kind"].as_str().unwrap();
        let dto = &record["dto"];
        let tick = dto["tick"].as_u64().expect("raw DTO tick") as u32;
        let state = audit_state_at(log_path, tick);
        let expected = owner_snapshot_from_audit(&state, slot);
        let expected = serde_json::to_value(expected).unwrap();
        let economy = &dto["economy"];
        for balance in economy["balances"].as_array().expect("balance DTO list") {
            assert!(
                expected["economy"]["balances"]
                    .as_array()
                    .unwrap()
                    .contains(balance),
                "raw {kind} at tick {tick} contains only this owner's authoritative balance: {balance}"
            );
        }
        for (field, key) in [
            ("inventories", "unit"),
            ("orders", "unit"),
            ("queues", "producer"),
        ] {
            for item in economy[field].as_array().expect("private DTO list") {
                assert!(
                    expected["economy"][field]
                        .as_array()
                        .unwrap()
                        .contains(item),
                    "raw {kind} at tick {tick} has only owner-filtered {field} records: {item}"
                );
                assert!(item[key].is_number());
            }
        }
        if kind == "snapshot" {
            let actual: turts_net_protocol::Snapshot = serde_json::from_value(dto.clone())
                .expect("received Snapshot JSON decodes to its original typed DTO");
            let expected_raw = owner_snapshot_from_audit(&state, slot);
            assert_eq!(
                actual, expected_raw,
                "raw Snapshot DTO equals independent owner projection at tick {tick}"
            );
            if let Some((output_tick, output_id)) = expected_late_output {
                assert!(
                    tick >= output_tick,
                    "late Snapshot tick {tick} follows owner 0 output at {output_tick}"
                );
                let matches = dto["units"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|unit| unit["unit_id"].as_u64() == Some(output_id.into()))
                    .collect::<Vec<_>>();
                assert_eq!(
                    matches.len(),
                    1,
                    "late Snapshot carries prior owner's public output once"
                );
                assert_eq!(matches[0]["owner_slot"].as_u64(), Some(0));
            }
        } else {
            let base_tick = dto["base_tick"].as_u64().expect("raw delta base tick") as u32;
            assert!(base_tick < tick);
        }
    }
}

fn economy_log_path(server: &support::ServerProc) -> &std::path::Path {
    server
        .economy_log
        .as_deref()
        .expect("server launched with economy logging")
}

fn read_live_complete_tick_prefix(
    path: &std::path::Path,
) -> Result<Vec<economylog::EconomyLogRecord>, String> {
    use economylog::EconomyLogRecord;

    let bytes = std::fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let Some(last_newline) = bytes.iter().rposition(|byte| *byte == b'\n') else {
        return Ok(Vec::new());
    };
    let complete_bytes = &bytes[..=last_newline];
    let mut records = Vec::new();
    for (index, line) in complete_bytes.split(|byte| *byte == b'\n').enumerate() {
        if line.is_empty() {
            if index + 1 == complete_bytes.split(|byte| *byte == b'\n').count() {
                continue;
            }
            return Err(format!("JSONL line {} is empty", index + 1));
        }
        let line = std::str::from_utf8(line)
            .map_err(|error| format!("decode JSONL line {} as UTF-8: {error}", index + 1))?;
        let record = serde_json::from_str::<EconomyLogRecord>(line)
            .map_err(|error| format!("decode complete JSONL line {}: {error}", index + 1))?;
        records.push(record);
    }

    let last_tick = records
        .iter()
        .rposition(|record| matches!(record, EconomyLogRecord::Tick { .. }));
    let full_validation = economylog::validate_records(&records);
    if full_validation.is_ok() {
        return Ok(records);
    }
    let full_error = full_validation.unwrap_err();

    let suffix_start = last_tick.map_or(0, |index| index + 1);
    let completed_operations = records[suffix_start..]
        .iter()
        .filter_map(|record| match record {
            EconomyLogRecord::Operation(operation) => Some(operation.operation_id),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    let open_commands = records[suffix_start..]
        .iter()
        .filter_map(|record| match record {
            EconomyLogRecord::CommandBoundary(command)
                if !completed_operations.contains(&command.operation_id) =>
            {
                Some(command.operation_id)
            }
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    let Some(open_operation_id) = open_commands.first().copied() else {
        return Err(full_error);
    };
    if full_error != format!("command operation id {open_operation_id} has no matching operation") {
        return Err(full_error);
    }

    let stable_end = if let Some(last_tick) = last_tick {
        last_tick + 1
    } else {
        records
            .iter()
            .position(|record| {
                matches!(
                    record,
                    EconomyLogRecord::CommandBoundary(command)
                        if command.operation_id == open_operation_id
                )
            })
            .expect("open command id has a boundary record")
    };
    records.truncate(stable_end);
    economylog::validate_records(&records)?;
    Ok(records)
}

fn read_complete_economy_log(
    path: &std::path::Path,
) -> Result<Vec<economylog::EconomyLogRecord>, String> {
    read_live_complete_tick_prefix(path)
}

fn wait_for_audit_tick(
    path: &std::path::Path,
    required_tick: u32,
) -> Vec<economylog::EconomyLogRecord> {
    use economylog::EconomyLogRecord;
    use std::time::{Duration, Instant};

    let started = Instant::now();
    loop {
        let records = read_live_complete_tick_prefix(path)
            .unwrap_or_else(|error| panic!("decode complete live audit prefix: {error}"));
        let latest_tick = records
            .iter()
            .filter_map(|record| match record {
                EconomyLogRecord::Tick { tick, .. } => Some(*tick),
                _ => None,
            })
            .max();
        if latest_tick.is_some_and(|tick| tick >= required_tick) {
            return records;
        }
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "completed audit tick reaches requested client tick {required_tick}; latest is {latest_tick:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn replay_public_log(
    path: &std::path::Path,
    package_name: &str,
    player_claims: &[(u8, u64)],
    frozen: &[FrozenCommand],
) -> ReplaySummary {
    let (public_initial, catalog) = independent_public_model(package_name);
    replay_public_log_with_model(path, public_initial, catalog, player_claims, frozen, true)
}

fn replay_public_log_with_model(
    path: &std::path::Path,
    public_initial: turts_sim::AuditState,
    catalog: turts_mod_api::ContentCatalog,
    player_claims: &[(u8, u64)],
    frozen: &[FrozenCommand],
    require_positive: bool,
) -> ReplaySummary {
    use economylog::EconomyLogRecord;
    use std::collections::BTreeMap;
    let records = read_complete_economy_log(path).unwrap();
    economylog::validate_records(&records).unwrap();
    let decoded_initial = match &records[0] {
        EconomyLogRecord::Initial(state) => state.clone(),
        _ => unreachable!(),
    };
    assert_eq!(
        decoded_initial, public_initial,
        "process boot matches the frozen public package"
    );
    let claim_records = records
        .iter()
        .filter_map(|record| match record {
            EconomyLogRecord::Claim {
                tick,
                player_id,
                slot,
            } => Some((*tick, *slot, *player_id)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        claim_records.len(),
        player_claims.len(),
        "every frozen Hello has one accepted claim"
    );
    for (_, slot, player_id) in &claim_records {
        assert!(
            player_claims.contains(&(*slot, *player_id)),
            "claim matches the separately frozen Hello identity and expected Welcome slot"
        );
    }

    let command_audits: BTreeMap<u64, _> = records
        .iter()
        .filter_map(|record| match record {
            EconomyLogRecord::CommandBoundary(command) => Some((command.operation_id, command)),
            _ => None,
        })
        .collect();
    let mut unmatched = frozen.to_vec();
    let mut tick_start = public_initial.clone();
    let mut oracle = accounting::AccountingOracle::new(&public_initial, &catalog);
    let first_tick = decoded_initial.economy.tick + 1;
    let last_tick = records
        .iter()
        .filter_map(|record| match record {
            EconomyLogRecord::Tick { tick, .. } => Some(*tick),
            _ => None,
        })
        .max()
        .unwrap_or(decoded_initial.economy.tick);
    let mut command_records = Vec::new();

    for tick in first_tick..=last_tick {
        for (claim_tick, slot, player_id) in claim_records
            .iter()
            .filter(|(claim_tick, _, _)| *claim_tick == tick - 1)
        {
            oracle
                .apply_frozen_hello_claim(*claim_tick, *slot, *player_id)
                .unwrap();
            let claim = tick_start
                .claims
                .get_mut(usize::from(*slot))
                .expect("frozen Welcome slot is in the modeled slot table");
            assert_eq!(*claim, None, "frozen player claims one open slot");
            *claim = Some(*player_id);
        }
        let operations = records
            .iter()
            .filter_map(|record| match record {
                EconomyLogRecord::Operation(operation) if operation.before.economy.tick == tick => {
                    Some(operation)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let has_command = operations
            .iter()
            .any(|operation| command_audits.contains_key(&operation.operation_id));
        if has_command {
            oracle
                .advance_clock_for_command_tick(tick)
                .unwrap_or_else(|error| panic!("prepare modeled command tick {tick}: {error}"));
        }
        let mut expected_gatherers: Option<Vec<u32>> = None;
        let mut gather_index = 0;
        let mut expected_producers: Option<Vec<u32>> = None;
        let mut producer_index = 0;
        let mut pending_output: Option<(u32, u64)> = None;

        for operation in operations {
            if let Some(command_audit) = command_audits.get(&operation.operation_id) {
                let index = unmatched
                    .iter()
                    .position(|expected| {
                        expected.slot == command_audit.input.slot.0
                            && expected.seq == command_audit.input.seq
                    })
                    .expect("actual command has one separately frozen trace input");
                let expected_frozen = unmatched.remove(index);
                let mut expected_input = sim_command(&expected_frozen, oracle.modeled_state());
                let task4_order =
                    matches!(&command_audit.input.kind, turts_sim::CommandKind::Order(_));
                if task4_order {
                    assert!(
                        order_input_matches_frozen(
                            &expected_frozen,
                            &command_audit.input,
                            &public_initial,
                        ),
                        "operation {} matches frozen Order envelope including raw float bits",
                        operation.operation_id
                    );
                    if let turts_sim::CommandKind::MoveTo { target, .. } = &mut expected_input.kind
                        && let Some(bounds) = &operation.before.bounds
                    {
                        target.x = target.x.clamp(0.0, bounds.x);
                        target.y = target.y.clamp(0.0, bounds.y);
                    }
                } else {
                    assert!(
                        command_input_matches(&expected_input, &command_audit.input),
                        "operation {} matches frozen input including raw float bits",
                        operation.operation_id
                    );
                }
                oracle
                    .observe(
                        &if task4_order {
                            accounting::ExpectedOperation::Task4Order(command_audit.input.clone())
                        } else {
                            accounting::ExpectedOperation::Command(expected_input.clone())
                        },
                        &operation.before,
                        &operation.after,
                        &operation.events,
                        Some(&command_audit.result),
                    )
                    .unwrap_or_else(|error| {
                        panic!("command operation {}: {error}", operation.operation_id)
                    });
                let mut projected_audit = (*command_audit).clone();
                projected_audit.input = expected_input;
                command_records.push(projected_audit);
                continue;
            }

            let gatherers = expected_gatherers.get_or_insert_with(|| {
                oracle
                    .modeled_state()
                    .raw_orders
                    .iter()
                    .map(|order| order.unit)
                    .collect()
            });
            let (expected, producer_visit) =
                if let Some(unit) = gatherers.get(gather_index).copied() {
                    gather_index += 1;
                    (
                        accounting::ExpectedOperation::Automatic {
                            tick,
                            phase: accounting::AutomaticPhase::Gather,
                            subject: accounting::AutomaticSubject::Gatherer(unit),
                        },
                        None,
                    )
                } else {
                    let producers = expected_producers.get_or_insert_with(|| {
                        oracle
                            .modeled_state()
                            .economy
                            .queues
                            .iter()
                            .filter(|queue| !queue.jobs.is_empty())
                            .map(|queue| queue.producer)
                            .collect()
                    });
                    let (producer, job_id, phase) =
                        if let Some((producer, job_id)) = pending_output.take() {
                            (producer, job_id, accounting::AutomaticPhase::OutputAttempt)
                        } else {
                            let producer = *producers.get(producer_index).expect(
                                "automatic production has a separately modeled producer visit",
                            );
                            let job = oracle
                                .modeled_state()
                                .economy
                                .queues
                                .iter()
                                .find(|queue| queue.producer == producer)
                                .and_then(|queue| queue.jobs.first())
                                .expect("frozen prior queue has its FIFO head");
                            let phase = if job.stage == turts_sim::JobStage::OutputBlocked {
                                accounting::AutomaticPhase::OutputAttempt
                            } else {
                                accounting::AutomaticPhase::ProductionAdvance
                            };
                            (producer, job.id, phase)
                        };
                    (
                        accounting::ExpectedOperation::Automatic {
                            tick,
                            phase,
                            subject: accounting::AutomaticSubject::HeadJob {
                                producer,
                                job: job_id,
                            },
                        },
                        Some((producer, job_id, phase)),
                    )
                };
            if let Some((producer, job_id, accounting::AutomaticPhase::ProductionAdvance)) =
                producer_visit
                && operation.events.iter().any(|event| matches!(
                    &event.kind,
                    turts_sim::EconomyEventKind::ProductionStarted { producer: event_producer, job: event_job, .. }
                        if *event_producer == producer && *event_job == job_id
                ))
            {
                let mut forged_after = operation.after.clone();
                forged_after
                    .raw_queues
                    .iter_mut()
                    .find(|queue| queue.producer == producer)
                    .and_then(|queue| queue.jobs.iter_mut().find(|job| job.id == job_id))
                    .expect("captured production job exists")
                    .stage = turts_sim::JobStage::Queued;
                forged_after
                    .economy
                    .queues
                    .iter_mut()
                    .find(|queue| queue.producer == producer)
                    .and_then(|queue| queue.jobs.iter_mut().find(|job| job.id == job_id))
                    .expect("captured public production job exists")
                    .stage = turts_sim::JobStage::Queued;
                let mut corrupted_oracle = oracle.clone();
                assert!(
                    corrupted_oracle
                        .observe(
                            &expected,
                            &operation.before,
                            &forged_after,
                            &operation.events,
                            None,
                        )
                        .is_err(),
                    "independent accounting rejects ProductionStarted without the modeled job-stage transition"
                );
            }
            oracle
                .observe(
                    &expected,
                    &operation.before,
                    &operation.after,
                    &operation.events,
                    None,
                )
                .unwrap_or_else(|error| {
                    panic!("automatic operation {}: {error}", operation.operation_id)
                });
            if let Some((producer, job_id, phase)) = producer_visit {
                if phase == accounting::AutomaticPhase::ProductionAdvance {
                    let blocked = oracle
                        .modeled_state()
                        .economy
                        .queues
                        .iter()
                        .find(|queue| queue.producer == producer)
                        .and_then(|queue| queue.jobs.first())
                        .is_some_and(|head| {
                            head.id == job_id && head.stage == turts_sim::JobStage::OutputBlocked
                        });
                    if blocked {
                        pending_output = Some((producer, job_id));
                    } else {
                        producer_index += 1;
                    }
                } else {
                    producer_index += 1;
                }
            }
        }

        let (tick_before, published, state) = records
            .iter()
            .find_map(|record| match record {
                EconomyLogRecord::Tick {
                    tick: found,
                    before,
                    state,
                    events,
                } if *found == tick => Some((before, events, state)),
                _ => None,
            })
            .expect("completed tick record");
        assert_eq!(
            tick_before, &tick_start,
            "tick {tick} before is prior modeled state"
        );
        oracle
            .observe(
                &accounting::ExpectedOperation::TickBoundary { tick },
                tick_before,
                state,
                published,
                None,
            )
            .unwrap_or_else(|error| panic!("completed tick {tick}: {error}"));
        tick_start = state.clone();
    }
    oracle.assert_command_records(&command_records).unwrap();
    assert!(
        unmatched.is_empty(),
        "every frozen trace input reached exactly one server command audit"
    );
    let totals = ReplaySummary {
        extraction: oracle.extraction,
        delivery: oracle.delivery,
        conversion: oracle.conversion,
        charge: oracle.gross_charge,
        refunds: oracle.refunds,
        outputs: oracle.outputs,
        cancellations: oracle.cancellations,
    };
    if require_positive {
        assert!(
            totals.extraction
                + totals.delivery
                + totals.conversion
                + totals.charge
                + totals.refunds
                + u128::from(totals.outputs)
                + u128::from(totals.cancellations)
                > 0,
            "the process replay measured a concrete positive economic effect: {totals:?}"
        );
    }
    totals
}

fn trace_file(server: &support::ServerProc, file: &str) -> std::path::PathBuf {
    let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m2")
        .join(file);
    let frozen = server.dir.join(file);
    std::fs::copy(source, &frozen).expect("copy frozen trace input into run evidence");
    frozen
}

fn run_trace_client(
    server: &support::ServerProc,
    player_id: u64,
    trace: &std::path::Path,
    run_secs: u64,
    extra: &[&str],
) -> serde_json::Value {
    let out = server.dir.join(format!("client-{player_id}.json"));
    let ws = format!("ws://{}", server.ws_addr);
    let player = player_id.to_string();
    let duration = run_secs.to_string();
    let mut args = vec![
        "--headless",
        "--server",
        &server.http_url,
        "--ws",
        &ws,
        "--run-secs",
        &duration,
        "--player-id",
        &player,
        "--out",
        out.to_str().unwrap(),
        "--command-trace",
        trace.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    let child = support::run_client(&args);
    support::wait_json(child, &out, std::time::Duration::from_secs(run_secs + 20))
}

fn start_economy_server(
    package: &std::path::Path,
    package_dir: &std::path::Path,
) -> support::ServerProc {
    let log = package_dir.join("economy.jsonl");
    support::spawn_server(package, &["--economy-log", log.to_str().unwrap()])
}

#[test]
#[ignore = "real WebSocket process acceptance; run through scripts/acceptance.sh"]
fn e8_commands_are_authoritative_and_acked() {
    use turts_mod_api::{ContentId, CostSource};
    use turts_net_protocol::{CommandRejectReason, OrderTarget, PlayerCommand};

    let package_dir = support::test_dir("e8-economy");
    let package = support::pack("base", &package_dir);
    let server = start_economy_server(&package, &package_dir);
    let (public_initial, public_catalog) = independent_public_model("base");
    let mut modeled_claims = public_initial.claims.clone();
    let trace_slot = modeled_claims
        .iter()
        .position(Option::is_none)
        .expect("public boot has an open slot") as u8;
    modeled_claims[usize::from(trace_slot)] = Some(10_001);
    let wire_slot = modeled_claims
        .iter()
        .position(Option::is_none)
        .expect("the second frozen Hello has an open slot") as u8;

    let trace_center =
        unit_id_for_owner_and_content(&public_initial, trace_slot, "base:command_center");
    let trace = trace_file(&server, "base-trace-live-job.json");
    let trace_client = run_trace_client(&server, 10_001, &trace, 2, &[]);
    assert!(trace_client["welcomed"].as_bool().unwrap_or(false));
    assert_eq!(trace_client["trace"]["complete"], true);
    let trace_expected = vec![
        FrozenCommand {
            slot: trace_slot,
            seq: 1,
            kind: FrozenKind::Produce {
                producer: trace_center,
                recipe: "base:train_rifleman",
            },
        },
        FrozenCommand {
            slot: trace_slot,
            seq: 2,
            kind: FrozenKind::CancelFirstJob {
                producer: trace_center,
            },
        },
        FrozenCommand {
            slot: trace_slot,
            seq: 3,
            kind: FrozenKind::Produce {
                producer: trace_center,
                recipe: "base:train_rifleman",
            },
        },
    ];
    let live_foreign_job = trace_client["trace"]["bindings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|binding| binding["step"].as_u64() == Some(2))
        .and_then(|binding| binding["result"]["job_id"].as_u64())
        .expect("foreign cancellation uses the actual Applied JobId from the trace");

    // Freeze wire actors, catalog affordances, costs, and queue capacity from the public
    // package before sending any raw envelope. The Welcome slot is predicted from the
    // public claim order and checked against the actual wire Welcome below.
    let owner_harvester =
        unit_id_for_owner_and_content(&public_initial, wire_slot, "base:harvester");
    let foreign_harvester = public_initial
        .economy
        .units
        .iter()
        .find(|unit| unit.content_id == "base:harvester" && unit.owner_slot != wire_slot)
        .map(|unit| unit.unit_id)
        .expect("public boot has a foreign harvester");
    let owner_center =
        unit_id_for_owner_and_content(&public_initial, wire_slot, "base:command_center");
    let foreign_center =
        unit_id_for_owner_and_content(&public_initial, trace_slot, "base:command_center");
    let ore_node = public_initial
        .economy
        .nodes
        .iter()
        .find(|node| node.content_id == ContentId::parse("base:ore_deposit").unwrap())
        .expect("public fixture has an ore node")
        .id
        .0;
    let unknown_node = public_initial
        .economy
        .nodes
        .iter()
        .map(|node| node.id.0)
        .max()
        .unwrap()
        .checked_add(1)
        .unwrap();
    let position = |unit_id| {
        public_initial
            .economy
            .units
            .iter()
            .find(|unit| unit.unit_id == unit_id)
            .expect("unit was frozen from public boot state")
            .pos
    };
    let own_pos = position(owner_harvester);
    let foreign_pos = position(foreign_harvester);
    let center_pos = position(owner_center);
    let bounds = public_initial.bounds.as_ref().expect("public map bounds");
    let unknown_unit = public_initial
        .economy
        .units
        .iter()
        .map(|unit| unit.unit_id)
        .max()
        .unwrap()
        .checked_add(1)
        .unwrap();
    let center_id = ContentId::parse("base:command_center").unwrap();
    let center_def = public_catalog.units.get(&center_id).unwrap();
    let producer = center_def
        .producer
        .as_ref()
        .expect("the public command center can produce");
    let train_id = ContentId::parse("base:train_rifleman").unwrap();
    let build_id = ContentId::parse("base:build_harvester").unwrap();
    assert!(producer.recipes.contains(&train_id));
    assert!(producer.recipes.contains(&build_id));
    let capacity = usize::from(producer.queue_capacity);
    assert!(capacity > 0);
    let credits = ContentId::parse("base:credits").unwrap();
    let slot_one_balance = public_initial
        .economy
        .balances
        .iter()
        .find(|balance| balance.slot == wire_slot && balance.resource == credits)
        .expect("public Welcome slot starts with a modeled credit balance")
        .on_hand;
    let recipe_cost = |recipe: &ContentId| {
        public_catalog
            .production
            .get(recipe)
            .unwrap()
            .costs
            .iter()
            .find(|cost| cost.source == CostSource::SlotBalance && cost.resource == credits)
            .unwrap()
            .amount
    };
    let train_cost = recipe_cost(&train_id);
    let build_cost = recipe_cost(&build_id);
    assert!(u128::from(slot_one_balance) >= u128::from(train_cost) * capacity as u128);
    assert!(build_cost > slot_one_balance);
    let move_fixture = |seq, units: Vec<u32>, x: f32, y: f32| FrozenCommand {
        slot: wire_slot,
        seq,
        kind: FrozenKind::MoveTo {
            units,
            target_bits: [x.to_bits(), y.to_bits()],
        },
    };
    let produce_fixture = |seq, producer, recipe| FrozenCommand {
        slot: wire_slot,
        seq,
        kind: FrozenKind::Produce { producer, recipe },
    };
    let gather_fixture = |seq, unit, node| FrozenCommand {
        slot: wire_slot,
        seq,
        kind: FrozenKind::Gather { unit, node },
    };
    let cancel_fixture = |seq, producer, job| FrozenCommand {
        slot: wire_slot,
        seq,
        kind: FrozenKind::Cancel { producer, job },
    };
    let mut wire_cases: Vec<(FrozenCommand, bool)> = vec![
        (
            move_fixture(1, vec![owner_harvester], own_pos.x - 2.0, own_pos.y),
            true,
        ),
        (
            move_fixture(
                2,
                vec![foreign_harvester],
                foreign_pos.x + 2.0,
                foreign_pos.y,
            ),
            true,
        ),
        (move_fixture(3, vec![unknown_unit], 20.0, 8.0), true),
        (move_fixture(4, vec![], 20.0, 8.0), true),
        (
            FrozenCommand {
                slot: wire_slot,
                seq: 5,
                kind: FrozenKind::MoveTo {
                    units: vec![owner_harvester],
                    target_bits: [0x7fc0_1234, 8.0_f32.to_bits()],
                },
            },
            true,
        ),
        // The duplicate seq=1 and missing producer capability are wire-only admissions.
        (
            move_fixture(1, vec![owner_harvester], own_pos.x - 4.0, own_pos.y),
            false,
        ),
        (
            produce_fixture(6, owner_harvester, "base:train_rifleman"),
            false,
        ),
        // A qualified, capability-valid unknown recipe is a domain command rejection.
        (
            produce_fixture(7, owner_center, "missing:train_rifleman"),
            true,
        ),
        // Malformed qualified-reference syntax is rejected before domain command capture.
        (produce_fixture(8, owner_center, "malformed"), false),
        // The command center is owned but does not have the order's movement capability.
        (
            move_fixture(9, vec![owner_center], center_pos.x - 1.0, center_pos.y),
            true,
        ),
        (
            move_fixture(10, vec![owner_harvester], bounds.x + 1.0, own_pos.y),
            true,
        ),
        // A catalog-valid build exceeds the frozen slot balance without mutating it.
        (
            produce_fixture(11, owner_center, "base:build_harvester"),
            true,
        ),
    ];
    let queue_start = 12_u32;
    for offset in 0..capacity {
        wire_cases.push((
            produce_fixture(
                queue_start + offset as u32,
                owner_center,
                "base:train_rifleman",
            ),
            true,
        ));
    }
    let queue_full_seq = queue_start + capacity as u32;
    wire_cases.push((
        produce_fixture(queue_full_seq, owner_center, "base:train_rifleman"),
        true,
    ));
    wire_cases.push((
        gather_fixture(queue_full_seq + 1, owner_harvester, ore_node),
        true,
    ));
    wire_cases.push((
        gather_fixture(queue_full_seq + 2, foreign_harvester, ore_node),
        true,
    ));
    wire_cases.push((
        gather_fixture(queue_full_seq + 3, owner_harvester, unknown_node),
        true,
    ));
    wire_cases.push((
        produce_fixture(queue_full_seq + 4, foreign_center, "base:train_rifleman"),
        false,
    ));
    wire_cases.push((
        cancel_fixture(queue_full_seq + 5, foreign_center, live_foreign_job),
        true,
    ));
    wire_cases.push((
        cancel_fixture(queue_full_seq + 6, owner_center, u64::MAX),
        true,
    ));
    let commands: Vec<PlayerCommand> = wire_cases
        .iter()
        .map(|(fixture, _)| protocol_command(fixture))
        .collect();
    let mut all_frozen = trace_expected;
    all_frozen.extend(
        wire_cases
            .iter()
            .filter(|(_, expect_domain_audit)| *expect_domain_audit)
            .map(|(fixture, _)| fixture.clone()),
    );

    let wire =
        wire_sender::send_commands(&server.http_url, &server.ws_addr, 10_002, commands.clone());
    assert!(
        wire.welcomed && !wire.timed_out,
        "real sender joined and got every terminal result: {wire:?}"
    );
    assert_eq!(wire.welcome_slot, Some(wire_slot));
    assert_eq!(wire.attempts.len(), commands.len());
    assert!(
        wire.attempts
            .iter()
            .all(|attempt| attempt.terminal_cursor.is_some())
    );
    assert!(
        wire.reliable_count < 32_768,
        "all observed lifetime reliable messages remain in the unambiguous half-range"
    );
    for (index, command) in commands.iter().enumerate() {
        let attempt = &wire.attempts[index];
        assert_eq!(attempt.index, index);
        assert_eq!(attempt.seq, command.seq);
        let terminal = &wire.receipts[attempt.terminal_cursor.unwrap()];
        assert_eq!(terminal.seq, attempt.seq);
        assert!(matches!(terminal.kind, "applied" | "rejected"));
        assert!(terminal.reliable);
    }
    let assert_rejection = |index: usize, expected: CommandRejectReason| {
        let attempt = &wire.attempts[index];
        let terminal_cursor = attempt.terminal_cursor.unwrap();
        let terminal = &wire.receipts[terminal_cursor];
        assert_eq!(terminal.seq, attempt.seq);
        assert_eq!(
            terminal.kind, "rejected",
            "wire command seq {} must be rejected",
            attempt.seq
        );
        assert!(terminal.reliable);
        assert_eq!(
            terminal.reason,
            Some(expected),
            "wire command seq {} rejection reason",
            attempt.seq
        );
    };
    let seq2_ack = command_attempt_receipt(&wire, 1, "ack");
    let seq2_rejection = command_attempt_receipt(&wire, 1, "rejected");
    assert!(seq2_ack.reliable && seq2_rejection.reliable);
    assert!(
        (1..32_768).contains(
            &seq2_rejection
                .message_id
                .unwrap()
                .wrapping_sub(seq2_ack.message_id.unwrap())
        ),
        "actual reliable-channel MessageIds put the seq2 Ack before its rejection"
    );
    assert_eq!(command_attempt_receipt(&wire, 0, "applied").kind, "applied");
    assert_rejection(1, CommandRejectReason::NotOwner);
    assert_rejection(2, CommandRejectReason::UnknownUnit);
    assert_rejection(3, CommandRejectReason::EmptyCommand);
    assert_rejection(4, CommandRejectReason::TargetNotFinite);
    assert_rejection(5, CommandRejectReason::BadSequence);
    assert_rejection(6, CommandRejectReason::MissingCapability);
    assert_rejection(7, CommandRejectReason::UnknownRecipe);
    assert_rejection(8, CommandRejectReason::UnknownRecipe);
    assert_rejection(9, CommandRejectReason::NotApplicable);
    assert_eq!(
        command_attempt_receipt(&wire, 10, "applied").kind,
        "applied",
        "Task4 Move clamps finite outside points to the map edge"
    );
    assert_rejection(11, CommandRejectReason::InsufficientBalance);
    for index in 12..12 + capacity {
        assert_eq!(
            wire.receipts[wire.attempts[index].terminal_cursor.unwrap()].kind,
            "applied"
        );
    }
    assert_rejection(12 + capacity, CommandRejectReason::QueueFull);
    let gather_start = 13 + capacity;
    assert_eq!(
        command_attempt_receipt(&wire, gather_start, "applied").kind,
        "applied"
    );
    assert_rejection(gather_start + 1, CommandRejectReason::NotOwner);
    assert_rejection(gather_start + 2, CommandRejectReason::UnknownNode);
    assert_rejection(gather_start + 3, CommandRejectReason::NotOwner);
    assert_rejection(gather_start + 4, CommandRejectReason::NotOwner);
    assert_rejection(gather_start + 5, CommandRejectReason::UnknownJob);
    assert_ack_before_every_rejection(&wire);

    let decoded = read_complete_economy_log(economy_log_path(&server)).unwrap();
    let seq10_audit = decoded
        .iter()
        .find_map(|record| match record {
            economylog::EconomyLogRecord::CommandBoundary(command)
                if command.input.slot.0 == wire_slot && command.input.seq == 10 =>
            {
                Some(command)
            }
            _ => None,
        })
        .expect("outside finite point reaches the admitted Order audit");
    let seq10_operation = decoded
        .iter()
        .find_map(|record| match record {
            economylog::EconomyLogRecord::Operation(operation)
                if operation.operation_id == seq10_audit.operation_id =>
            {
                Some(operation)
            }
            _ => None,
        })
        .expect("clamped Move has a matching Orders operation");
    let bounds = public_initial.bounds.as_ref().expect("public map bounds");
    assert_eq!(
        seq10_operation
            .after
            .raw_units
            .iter()
            .find(|unit| unit.unit_id == owner_harvester)
            .unwrap()
            .pos,
        seq10_operation
            .before
            .raw_units
            .iter()
            .find(|unit| unit.unit_id == owner_harvester)
            .unwrap()
            .pos,
        "the Orders operation sets a target without teleporting the actor"
    );
    assert_eq!(
        seq10_operation
            .after
            .raw_units
            .iter()
            .find(|target| target.unit_id == owner_harvester)
            .unwrap()
            .move_target,
        Some(turts_mod_api::WorldPos {
            x: bounds.x,
            y: own_pos.y
        }),
        "the finite target is clamped to the exact public map edge"
    );
    assert!(
        seq10_operation
            .after
            .unit_orders
            .iter()
            .all(|orders| { orders.unit_id != owner_harvester }),
        "fresh plain Move keeps the zero-extra-order-state fast path"
    );
    let admissions = decoded
        .iter()
        .filter_map(|record| match record {
            economylog::EconomyLogRecord::Admission {
                reason,
                raw_envelope,
                before,
                after,
                ..
            } => {
                let command: PlayerCommand =
                    postcard::from_bytes(raw_envelope).expect("lossless postcard envelope");
                assert_eq!(
                    before, after,
                    "wire-only rejection leaves audited simulation unchanged"
                );
                Some((reason, command))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let admission = |seq: u32| {
        admissions
            .iter()
            .find(|(_, command)| command.seq == seq)
            .expect("frozen wire-only input has one admission record")
    };
    assert_eq!(admission(1).0, &CommandRejectReason::BadSequence);
    assert_eq!(admission(6).0, &CommandRejectReason::MissingCapability);
    assert_eq!(admission(8).0, &CommandRejectReason::UnknownRecipe);
    assert_eq!(
        admission(queue_full_seq + 4).0,
        &CommandRejectReason::NotOwner
    );
    assert!(matches!(
        &admission(8).1.order,
        order if order.order == "base:order_produce"
            && order.units == [turts_net_protocol::UnitId(owner_center)]
            && order.target == OrderTarget::Content("malformed".into())
    ));
    assert!(admissions.iter().all(|(_, command)| command.seq != 7));

    let seq7_audit = decoded
        .iter()
        .find_map(|record| match record {
            economylog::EconomyLogRecord::CommandBoundary(audit)
                if audit.input.slot.0 == wire_slot && audit.input.seq == 7 =>
            {
                Some(audit)
            }
            _ => None,
        })
        .expect("qualified unknown recipe reaches domain command capture");
    assert!(matches!(
        &seq7_audit.input.kind,
        turts_sim::CommandKind::Order(order)
            if order.order.to_string() == "base:order_produce"
                && order.behaviour == turts_mod_api::Behaviour::Produce
                && order.units == [owner_center]
                && order.target == turts_sim::OrderTargetSim::Content(
                    ContentId::parse("missing:train_rifleman").unwrap()
                )
    ));
    assert_eq!(
        seq7_audit.result.outcome,
        Err(turts_sim::RejectReason::UnknownRecipe)
    );
    let seq7_operation = decoded
        .iter()
        .find_map(|record| match record {
            economylog::EconomyLogRecord::Operation(operation)
                if operation.operation_id == seq7_audit.operation_id =>
            {
                Some(operation)
            }
            _ => None,
        })
        .expect("seq7 command audit correlates to one operation by operation_id");
    assert_eq!(seq7_operation.before, seq7_operation.after);
    assert!(seq7_operation.events.is_empty());
    assert!(decoded.iter().any(|record| matches!(
        record,
        economylog::EconomyLogRecord::CommandBoundary(audit)
            if audit.operation_id == seq7_audit.operation_id
                && audit.input.slot.0 == wire_slot && audit.input.seq == 7
    )));
    assert!(decoded.iter().all(|record| !matches!(
        record,
        economylog::EconomyLogRecord::CommandBoundary(audit)
            if audit.input.slot.0 == wire_slot && audit.input.seq == 8
    )));
    for (index, (fixture, reaches_domain)) in wire_cases.iter().enumerate() {
        if !reaches_domain
            || wire.receipts[wire.attempts[index].terminal_cursor.unwrap()].kind != "rejected"
        {
            continue;
        }
        let command = decoded
            .iter()
            .find_map(|record| match record {
                economylog::EconomyLogRecord::CommandBoundary(command)
                    if command.input.slot.0 == fixture.slot && command.input.seq == fixture.seq =>
                {
                    Some(command)
                }
                _ => None,
            })
            .expect("domain rejection has a direct logged command boundary");
        assert_eq!(
            command.before, command.after,
            "rejected seq {} is write neutral",
            fixture.seq
        );
        let operation = decoded
            .iter()
            .find_map(|record| match record {
                economylog::EconomyLogRecord::Operation(operation)
                    if operation.operation_id == command.operation_id =>
                {
                    Some(operation)
                }
                _ => None,
            })
            .expect("rejected command boundary correlates to one operation");
        assert_eq!(
            operation.before, operation.after,
            "rejected seq {} has no hidden partial write",
            fixture.seq
        );
        assert!(
            operation.events.is_empty(),
            "rejected seq {} emits no event",
            fixture.seq
        );
    }

    // Reconstruct every domain command from the frozen public fixture, while malformed,
    // duplicate-sequence, and missing-capability envelopes remain admission-only.
    replay_task4_order_log(
        &decoded,
        &public_initial,
        &all_frozen,
        &[(trace_slot, 10_001), (wire_slot, 10_002)],
    );
    let _economic_oracle = replay_public_log(
        economy_log_path(&server),
        "base",
        &[(trace_slot, 10_001), (wire_slot, 10_002)],
        &all_frozen,
    );

    let old_protocol = wire_sender::send_commands_with_protocol(
        &server.http_url,
        &server.ws_addr,
        10_003,
        turts_net_protocol::NET_PROTOCOL_VERSION
            .checked_sub(1)
            .expect("current protocol version is positive"),
        Vec::new(),
    );
    assert!(!old_protocol.welcomed && !old_protocol.timed_out);
    assert_eq!(
        old_protocol.join_rejected.as_deref(),
        Some("protocol version mismatch")
    );
    assert!(old_protocol.receipts.is_empty());
    assert!(old_protocol.reliable_count < 32_768);
    let final_records = read_complete_economy_log(economy_log_path(&server)).unwrap();
    assert!(
        final_records.iter().all(|record| !matches!(
            record,
            economylog::EconomyLogRecord::Claim {
                player_id: 10_003,
                ..
            }
        )),
        "old-protocol Hello is rejected before slot claim"
    );

    let advertisement: turts_net_protocol::Advertisement = serde_json::from_str(
        &ureq::get(&format!("{}/advertisement.json", server.http_url))
            .call()
            .unwrap()
            .into_body()
            .read_to_string()
            .unwrap(),
    )
    .unwrap();
    let mut old_api_lockfile = advertisement.lockfile_hash.into_bytes();
    let first = old_api_lockfile
        .first_mut()
        .expect("advertised lockfile hash is nonempty");
    *first = if *first == b'0' { b'1' } else { b'0' };
    let old_api = wire_sender::send_commands_with_hello(
        &server.http_url,
        &server.ws_addr,
        10_004,
        turts_net_protocol::NET_PROTOCOL_VERSION,
        Some(String::from_utf8(old_api_lockfile).unwrap()),
        Vec::new(),
    );
    assert!(!old_api.welcomed && !old_api.timed_out);
    assert_eq!(
        old_api.join_rejected.as_deref(),
        Some("lockfile hash mismatch")
    );
    assert!(old_api.receipts.is_empty());
    assert!(old_api.reliable_count < 32_768);
    let final_records = read_complete_economy_log(economy_log_path(&server)).unwrap();
    assert!(
        final_records.iter().all(|record| !matches!(
            record,
            economylog::EconomyLogRecord::Claim {
                player_id: 10_004,
                ..
            }
        )),
        "stale-API lockfile Hello is rejected before slot claim"
    );

    run_e8_public_variant_process();
}

fn run_e8_public_variant_process() {
    use turts_net_protocol::{CommandRejectReason, PlayerCommand};

    let package_dir = support::test_dir("e8-public-validation-variant");
    let package = pack_e8_validation_variant(&package_dir);
    let package_bytes = std::fs::read(&package).unwrap();
    let (initial, catalog) = independent_public_model_from_archive(&package_bytes, 20);
    let server = start_economy_server(&package, &package_dir);
    let slot = 0_u8;
    let player_id = 10_005_u64;
    let harvester = unit_id_for_owner_and_content(&initial, slot, "base:harvester");
    let center = unit_id_for_owner_and_content(&initial, slot, "base:command_center");
    let ore_node = initial
        .economy
        .nodes
        .iter()
        .find(|node| node.content_id.to_string() == "base:ore_deposit")
        .unwrap()
        .id
        .0;
    let timber_node = initial
        .economy
        .nodes
        .iter()
        .find(|node| node.content_id.to_string() == "base:timber_deposit")
        .expect("variant adds a public timber node")
        .id
        .0;
    let unknown_node = initial
        .economy
        .nodes
        .iter()
        .map(|node| node.id.0)
        .max()
        .unwrap()
        .checked_add(1)
        .unwrap();
    let producer = catalog
        .units
        .get(&turts_mod_api::ContentId::parse("base:command_center").unwrap())
        .unwrap()
        .producer
        .as_ref()
        .unwrap();
    assert!(
        !producer
            .recipes
            .contains(&turts_mod_api::ContentId::parse("base:train_rifleman").unwrap())
    );
    assert_eq!(
        catalog.production[&turts_mod_api::ContentId::parse("base:build_harvester").unwrap()]
            .costs
            .len(),
        2,
        "frozen public variant has two independently charged balance lines"
    );

    let frozen = vec![
        FrozenCommand {
            slot,
            seq: 1,
            kind: FrozenKind::Gather {
                unit: harvester,
                node: ore_node,
            },
        },
        FrozenCommand {
            slot,
            seq: 2,
            kind: FrozenKind::Gather {
                unit: harvester,
                node: timber_node,
            },
        },
        FrozenCommand {
            slot,
            seq: 3,
            kind: FrozenKind::Gather {
                unit: harvester,
                node: unknown_node,
            },
        },
        FrozenCommand {
            slot,
            seq: 4,
            kind: FrozenKind::Produce {
                producer: center,
                recipe: "base:train_rifleman",
            },
        },
        FrozenCommand {
            slot,
            seq: 5,
            kind: FrozenKind::Produce {
                producer: center,
                recipe: "base:build_harvester",
            },
        },
        FrozenCommand {
            slot,
            seq: 6,
            kind: FrozenKind::Cancel {
                producer: center,
                job: u64::MAX,
            },
        },
        FrozenCommand {
            slot,
            seq: 7,
            kind: FrozenKind::Produce {
                producer: center,
                recipe: "base:free_hire",
            },
        },
        FrozenCommand {
            slot,
            seq: 8,
            kind: FrozenKind::Cancel {
                producer: center,
                job: initial.allocators.next_values().2,
            },
        },
    ];
    let commands: Vec<PlayerCommand> = frozen.iter().map(protocol_command).collect();
    let wire = wire_sender::send_commands(&server.http_url, &server.ws_addr, player_id, commands);
    assert!(
        wire.welcomed && !wire.timed_out,
        "variant sender completed: {wire:?}"
    );
    assert_eq!(wire.welcome_slot, Some(slot));
    assert_ack_before_every_rejection(&wire);
    let expected = [
        None,
        Some(CommandRejectReason::IncompatibleItem),
        Some(CommandRejectReason::UnknownNode),
        Some(CommandRejectReason::RecipeNotEnabled),
        Some(CommandRejectReason::InsufficientBalance),
        Some(CommandRejectReason::UnknownJob),
        None,
        None,
    ];
    for (index, expected) in expected.iter().enumerate() {
        let attempt = &wire.attempts[index];
        let terminal = &wire.receipts[attempt.terminal_cursor.unwrap()];
        if let Some(reason) = expected {
            assert_eq!(terminal.kind, "rejected");
            assert_eq!(terminal.reason, Some(*reason));
        } else {
            assert_eq!(terminal.kind, "applied");
        }
    }

    let log = economy_log_path(&server);
    let records = read_complete_economy_log(log).unwrap();
    for frozen_command in frozen.iter().skip(1).take(5) {
        let audit = records
            .iter()
            .find_map(|record| match record {
                economylog::EconomyLogRecord::CommandBoundary(audit)
                    if audit.input.slot.0 == slot && audit.input.seq == frozen_command.seq =>
                {
                    Some(audit)
                }
                _ => None,
            })
            .expect("rejected fixture command is captured at its direct domain boundary");
        assert_eq!(
            audit.before, audit.after,
            "rejected seq {} is write neutral",
            audit.input.seq
        );
        let operation = records
            .iter()
            .find_map(|record| match record {
                economylog::EconomyLogRecord::Operation(operation)
                    if operation.operation_id == audit.operation_id =>
                {
                    Some(operation)
                }
                _ => None,
            })
            .expect("rejected fixture command has one correlated operation");
        assert_eq!(
            operation.before, operation.after,
            "rejected seq {} has no partial write",
            audit.input.seq
        );
        assert!(operation.events.is_empty());
    }
    replay_task4_order_log(&records, &initial, &frozen, &[(slot, player_id)]);
    let _economic_oracle = replay_public_log_with_model(
        log,
        initial.clone(),
        catalog,
        &[(slot, player_id)],
        &frozen,
        false,
    );
    let produced = records
        .iter()
        .find_map(|record| match record {
            economylog::EconomyLogRecord::CommandBoundary(command)
                if command.input.slot.0 == slot && command.input.seq == 7 =>
            {
                command
                    .result
                    .outcome
                    .as_ref()
                    .ok()
                    .and_then(|applied| applied.job_id)
                    .map(|job| (job.0, &command.after))
            }
            _ => None,
        })
        .expect("free ReserveThenCharge production is applied");
    let (job_id, produced_state) = produced;
    let raw_job = produced_state
        .raw_queues
        .iter()
        .find(|queue| queue.producer == center)
        .and_then(|queue| queue.jobs.iter().find(|job| job.id == job_id))
        .expect("captured zero-cost job remains queued at Produce boundary");
    assert_eq!(
        raw_job
            .reservation
            .as_ref()
            .map(|reservation| reservation.lines.as_slice()),
        Some([].as_slice()),
        "ReserveThenCharge captures Some(empty) even when the recipe costs nothing"
    );
    assert!(produced_state.ledger_claims.iter().any(|claim| matches!(
        claim,
        turts_sim::ReservationClaimView::Active { map_job_id, token_job_id, lines }
            if *map_job_id == job_id && *token_job_id == job_id && lines.is_empty()
    )));
    let canceled_state = records
        .iter()
        .find_map(|record| match record {
            economylog::EconomyLogRecord::CommandBoundary(command)
                if command.input.slot.0 == slot && command.input.seq == 8 =>
            {
                Some(&command.after)
            }
            _ => None,
        })
        .expect("zero-cost job Cancel is audited");
    assert!(canceled_state.ledger_claims.iter().any(|claim| matches!(
        claim,
        turts_sim::ReservationClaimView::Closed { map_job_id } if *map_job_id == job_id
    )));

    let mut missing_reservation = records.clone();
    mutate_order_command_after(&mut missing_reservation, 7, |state| {
        let job = state
            .raw_queues
            .iter_mut()
            .find(|queue| queue.producer == center)
            .and_then(|queue| queue.jobs.iter_mut().find(|job| job.id == job_id))
            .expect("captured zero-cost job exists");
        job.reservation = None;
    });
    assert!(
        economylog::validate_records(&missing_reservation)
            .unwrap_err()
            .contains("Produce changed state outside the catalog-derived job"),
        "captured Produce without its empty reservation token is rejected"
    );

    let mut mismatched_token = records.clone();
    mutate_order_command_after(&mut mismatched_token, 7, |state| {
        let claim = state
            .ledger_claims
            .iter_mut()
            .find(|claim| {
                matches!(
                    claim,
                    turts_sim::ReservationClaimView::Active { map_job_id, .. }
                        if *map_job_id == job_id
                )
            })
            .expect("active zero-cost claim exists");
        *claim = turts_sim::ReservationClaimView::Active {
            map_job_id: job_id,
            token_job_id: job_id + 1,
            lines: vec![],
        };
    });
    assert!(
        economylog::validate_records(&mismatched_token)
            .unwrap_err()
            .contains("Produce changed state outside the catalog-derived job"),
        "captured claim with a token for another JobId is rejected"
    );

    let mut leaked_claim = records.clone();
    mutate_order_command_after(&mut leaked_claim, 8, |state| {
        let claim = state
            .ledger_claims
            .iter_mut()
            .find(|claim| {
                matches!(
                    claim,
                    turts_sim::ReservationClaimView::Closed { map_job_id }
                        if *map_job_id == job_id
                )
            })
            .expect("Cancel closes the zero-cost reservation claim");
        *claim = turts_sim::ReservationClaimView::Active {
            map_job_id: job_id,
            token_job_id: job_id,
            lines: vec![],
        };
    });
    assert!(
        economylog::validate_records(&leaked_claim)
            .unwrap_err()
            .contains("Cancel changed state outside requested job removal"),
        "captured Cancel that leaks the empty claim is rejected"
    );
}

#[test]
#[ignore = "real two-owner WebSocket process acceptance; run through scripts/acceptance.sh"]
fn e9_two_owners_late_join_and_output_lifecycle() {
    let package_dir = support::test_dir("e9-two-owner-economy");
    let package = support::pack("base", &package_dir);
    let server = start_economy_server(&package, &package_dir);
    let trace0 = trace_file(&server, "base-output-trace.json");
    let trace1 = trace_file(&server, "base-output-trace-slot1.json");
    let out0 = server.dir.join("client-20-001.json");
    let raw0 = server.dir.join("client-20-001-wire.jsonl");
    let ws = format!("ws://{}", server.ws_addr);
    let player0 = "20001";
    let duration0 = "12";
    let child0 = support::run_client(&[
        "--headless",
        "--server",
        &server.http_url,
        "--ws",
        &ws,
        "--run-secs",
        duration0,
        "--player-id",
        player0,
        "--out",
        out0.to_str().unwrap(),
        "--command-trace",
        trace0.to_str().unwrap(),
        "--raw-world-log",
        raw0.to_str().unwrap(),
    ]);
    assert_first_raw_snapshot_matches_owner(economy_log_path(&server), &raw0, 0);
    let (owner0_output_tick, owner0_output_id) =
        wait_for_owner_rifle_output(economy_log_path(&server));
    let out1 = server.dir.join("client-20-002.json");
    let raw1 = server.dir.join("client-20-002-wire.jsonl");
    let player1 = "20002";
    let duration1 = "7";
    let child1 = support::run_client(&[
        "--headless",
        "--server",
        &server.http_url,
        "--ws",
        &ws,
        "--run-secs",
        duration1,
        "--player-id",
        player1,
        "--out",
        out1.to_str().unwrap(),
        "--command-trace",
        trace1.to_str().unwrap(),
        "--raw-world-log",
        raw1.to_str().unwrap(),
    ]);
    let report1 = support::wait_json(child1, &out1, std::time::Duration::from_secs(27));
    let report0 = support::wait_json(child0, &out0, std::time::Duration::from_secs(27));
    for report in [&report0, &report1] {
        assert_eq!(report["trace"]["complete"], true);
        let units = report["replicated_world"]["units"].as_array().unwrap();
        assert_eq!(
            units
                .iter()
                .filter(|unit| unit["content_id"] == "base:rifleman")
                .count(),
            2,
            "late join sees each output exactly once"
        );
        let credits = report["replicated_world"]["economy"]["balances"]
            .as_array()
            .unwrap();
        assert_eq!(
            credits.len(),
            1,
            "the snapshot exposes only the owner's balance"
        );
        assert_eq!(credits[0]["on_hand"], 400);
    }
    let expected = [
        FrozenCommand {
            slot: 0,
            seq: 1,
            kind: FrozenKind::Produce {
                producer: 1,
                recipe: "base:train_rifleman",
            },
        },
        FrozenCommand {
            slot: 1,
            seq: 1,
            kind: FrozenKind::Produce {
                producer: 3,
                recipe: "base:train_rifleman",
            },
        },
    ];
    let _oracle = replay_public_log(
        economy_log_path(&server),
        "base",
        &[(0, 20_001), (1, 20_002)],
        &expected,
    );
    let log = economy_log_path(&server);
    let tick0 = assert_report_matches_own_audit_tick(log, &report0);
    let tick1 = assert_report_matches_own_audit_tick(log, &report1);
    let last_tick = audit_last_tick(log);
    assert!(tick0 >= owner0_output_tick && tick1 >= owner0_output_tick);
    assert!(tick0 <= last_tick && tick1 <= last_tick);
    let final_state = audit_state_at(log, last_tick);
    let mut output_ids: Vec<_> = final_state
        .economy
        .units
        .iter()
        .filter(|unit| unit.content_id == "base:rifleman")
        .map(|unit| unit.unit_id)
        .collect();
    output_ids.sort_unstable();
    assert_eq!(
        output_ids.len(),
        2,
        "the independent final audit has two produced riflemen"
    );
    for report in [&report0, &report1] {
        let mut report_ids: Vec<_> = report["replicated_world"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|unit| unit["content_id"] == "base:rifleman")
            .map(|unit| unit["unit_id"].as_u64().unwrap() as u32)
            .collect();
        report_ids.sort_unstable();
        assert_eq!(
            report_ids, output_ids,
            "each owner sees the same two output identities once"
        );
    }
    assert_raw_wire_owner_records(log, &raw0, 0, None);
    assert_raw_wire_owner_records(log, &raw1, 1, Some((owner0_output_tick, owner0_output_id)));
}

fn run_replay_spawn_window_subcase() {
    use turts_net_protocol::UnitSnapshot;

    let package_dir = support::test_dir("e9-replay-spawn-window");
    let production_path = support::mods_dir().join("base/production/rifleman.ron");
    let original = std::fs::read_to_string(&production_path).unwrap();
    let short_recipe = original.replace("duration_ms: 5000", "duration_ms: 200");
    assert_ne!(
        short_recipe, original,
        "fixture shortens only public recipe time"
    );
    let package = support::pack_with(
        "base",
        "production/rifleman.ron",
        &short_recipe,
        &package_dir,
    );
    let archive_bytes = std::fs::read(&package).unwrap();
    let (initial, catalog) = independent_public_model_from_archive(&archive_bytes, 20);
    let server = start_economy_server(&package, &package_dir);
    let trace = trace_file(&server, "base-replay-spawn-trace.json");
    let player_id = 36_001;
    let raw_path = server.dir.join("client-36-001-wire.jsonl");
    let raw_arg = raw_path.to_str().unwrap();
    let report = run_trace_client(
        &server,
        player_id,
        &trace,
        4,
        &[
            "--raw-world-log",
            raw_arg,
            "--resync-buffer-batches",
            "32",
            "--hold-resync-snapshots",
            "12",
            "--hold-resync-snapshot-count",
            "1",
            "--drop-world-changes",
            "charge",
        ],
    );
    assert!(report["welcomed"].as_bool().unwrap_or(false));
    assert!(report["trace"]["complete"].as_bool().unwrap_or(false));
    assert_eq!(
        report["successor_requests"].as_u64(),
        Some(0),
        "duplicate-free retained replay completes without a successor response"
    );
    assert_eq!(report["snapshots_applied"].as_u64(), Some(2));
    assert_eq!(report["max_outstanding_requests"].as_u64(), Some(1));
    assert_eq!(report["buffer_overflows"].as_u64(), Some(0));
    assert!(report["max_buffered_batches"].as_u64().unwrap_or(u64::MAX) <= 32);
    let dropped = report["dropped_world_changes"].as_array().unwrap();
    assert_eq!(dropped.len(), 1);
    assert_eq!(dropped[0]["kind"], "charge");

    let log = economy_log_path(&server);
    let (output_tick, output_id) = wait_for_owner_rifle_output(log);
    let report_tick = assert_report_matches_own_audit_tick(log, &report);
    assert!(report_tick >= output_tick);
    let oracle = replay_public_log_with_model(
        log,
        initial,
        catalog,
        &[(0, player_id)],
        &[FrozenCommand {
            slot: 0,
            seq: 1,
            kind: FrozenKind::Produce {
                producer: 1,
                recipe: "base:train_rifleman",
            },
        }],
        true,
    );
    assert_eq!(
        oracle.outputs, 1,
        "independent replay observes one spawned output"
    );

    let text = std::fs::read_to_string(&raw_path).unwrap();
    let records: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).expect("raw received DTO is valid JSON"))
        .collect();
    let snapshots: Vec<_> = records
        .iter()
        .enumerate()
        .filter(|(_, item)| item["kind"] == "snapshot")
        .collect();
    assert_eq!(snapshots.len(), 2, "initial Snapshot and one held response");
    let (response_index, response) = snapshots[1];
    let response_tick = response["dto"]["tick"].as_u64().unwrap() as u32;
    assert!(
        response_tick < output_tick,
        "response Snapshot predates the output"
    );
    let audit_output = audit_state_at(log, output_tick)
        .economy
        .units
        .into_iter()
        .find(|unit| unit.unit_id == output_id)
        .expect("audited output unit exists at its committed tick");
    let spawn_deltas: Vec<_> = records
        .iter()
        .enumerate()
        .filter(|(index, item)| {
            *index > response_index
                && item["kind"] == "world_delta"
                && item["dto"]["spawned"].as_array().is_some_and(|spawned| {
                    spawned
                        .iter()
                        .any(|unit| unit["unit_id"].as_u64() == Some(output_id.into()))
                })
        })
        .collect();
    assert_eq!(
        spawn_deltas.len(),
        1,
        "one received WorldDelta carries the output"
    );
    let (spawn_index, spawn_record) = spawn_deltas[0];
    let spawn_tick = spawn_record["dto"]["tick"].as_u64().unwrap() as u32;
    assert_eq!(spawn_tick, output_tick);
    assert!(
        spawn_tick <= response_tick + 12,
        "spawn arrives during the delayed response window"
    );
    assert!(
        records[response_index + 1..].iter().any(|item| {
            item["kind"] == "world_delta"
                && item["dto"]["tick"]
                    .as_u64()
                    .is_some_and(|tick| tick >= u64::from(response_tick + 12))
        }),
        "WorldDeltas arrive through the full 12-tick held-response window"
    );
    let spawned = spawn_record["dto"]["spawned"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["unit_id"].as_u64() == Some(output_id.into()))
        .unwrap();
    let typed: UnitSnapshot = serde_json::from_value(spawned.clone()).unwrap();
    assert_eq!(typed.content_id, audit_output.content_id);
    assert_eq!(typed.owner_slot.0, audit_output.owner_slot);
    assert_eq!(typed.pos.x.to_bits(), audit_output.pos.x.to_bits());
    assert_eq!(typed.pos.y.to_bits(), audit_output.pos.y.to_bits());
    assert!(spawn_index > response_index);
    assert_raw_wire_owner_records(log, &raw_path, 0, None);
}

fn run_world_change_subcase(
    kind: &str,
    package_name: &str,
    trace_name: &str,
    player_id: u64,
    run_secs: u64,
    expected: &[FrozenCommand],
) {
    let package_dir = support::test_dir(&format!("e9-{kind}-world-change"));
    let package = support::pack(package_name, &package_dir);
    let server = start_economy_server(&package, &package_dir);
    let trace = trace_file(&server, trace_name);
    let raw_wire = server.dir.join(format!("client-{player_id}-wire.jsonl"));
    let raw_wire_arg = raw_wire.to_str().unwrap().to_owned();
    let kind_arg = kind.to_owned();
    let extra = vec![
        "--raw-world-log".to_owned(),
        raw_wire_arg,
        "--resync-buffer-batches".to_owned(),
        "3".to_owned(),
        "--hold-resync-snapshots".to_owned(),
        "12".to_owned(),
        "--hold-resync-snapshot-count".to_owned(),
        "1".to_owned(),
        "--drop-world-changes".to_owned(),
        kind_arg,
    ];
    let extra_refs: Vec<_> = extra.iter().map(String::as_str).collect();
    let report = run_trace_client(&server, player_id, &trace, run_secs, &extra_refs);
    assert_eq!(report["welcomed"], true, "{kind} case joined");
    assert_eq!(report["trace"]["complete"], true, "{kind} trace completed");
    assert!(
        report["buffer_overflows"].as_u64().unwrap_or(0) > 0,
        "{kind} delayed response exercised bounded-buffer overflow"
    );
    assert!(report["max_buffered_batches"].as_u64().unwrap_or(u64::MAX) <= 3);
    assert_eq!(report["max_outstanding_requests"].as_u64(), Some(1));
    assert_eq!(report["successor_requests"].as_u64(), Some(1));
    assert!(report["snapshots_applied"].as_u64().unwrap_or(0) >= 2);
    let dropped = report["dropped_world_changes"].as_array().unwrap();
    assert_eq!(
        dropped.len(),
        1,
        "{kind} predicate fires once in this subcase"
    );
    assert_eq!(dropped[0]["kind"], kind);
    let log = economy_log_path(&server);
    let tick = assert_report_matches_own_audit_tick(log, &report);
    assert!(tick <= audit_last_tick(log));
    assert_raw_wire_owner_records(log, &raw_wire, 0, None);
    assert_delayed_first_snapshot_then_successor(&raw_wire, 12);
    let oracle = replay_public_log(log, package_name, &[(0, player_id)], expected);
    match kind {
        "extraction" => assert!(oracle.extraction > 0),
        "conversion" => assert!(oracle.conversion > 0),
        "charge" => assert!(oracle.charge > 0),
        "refund" => assert!(oracle.refunds > 0),
        "spawn" => assert!(oracle.outputs > 0),
        "final_change" => assert!(oracle.charge + oracle.refunds + u128::from(oracle.outputs) > 0),
        _ => panic!("unknown WorldDelta class {kind}"),
    }
}

#[test]
#[ignore = "real recovery and WorldDelta-loss process acceptance; run through scripts/acceptance.sh"]
fn e9_loss_and_small_buffer_converge() {
    let package_dir = support::test_dir("e9-loss-economy");
    let package = support::pack("haul", &package_dir);
    let server = start_economy_server(&package, &package_dir);
    let trace = trace_file(&server, "haul-trace.json");
    let raw_wire = server.dir.join("client-30-001-wire.jsonl");
    let raw_wire_arg = raw_wire.to_str().unwrap();
    let report = run_trace_client(
        &server,
        30_001,
        &trace,
        16,
        &[
            "--raw-world-log",
            raw_wire_arg,
            "--resync-buffer-batches",
            "3",
            "--hold-resync-snapshots",
            "12",
            "--hold-resync-snapshot-count",
            "1",
            "--drop-world-changes",
            "extraction",
        ],
    );
    assert_eq!(report["welcomed"], true);
    assert!(
        report["buffer_overflows"].as_u64().unwrap_or(0) > 0,
        "buffer limit was exercised: {report}"
    );
    assert!(report["max_buffered_batches"].as_u64().unwrap_or(0) <= 3);
    assert_eq!(report["max_outstanding_requests"], 1);
    assert!(report["successor_requests"].as_u64().unwrap_or(0) <= 1);
    assert!(report["snapshots_applied"].as_u64().unwrap_or(0) >= 2);
    let dropped = report["dropped_world_changes"].as_array().unwrap();
    assert_eq!(
        dropped.len(),
        2,
        "initial arm and live midtrace rearm each fired once"
    );
    assert!(dropped.iter().all(|item| item["kind"] == "extraction"));
    assert!(
        dropped[1]["arm_tick"].as_u64() > dropped[0]["dropped_tick"].as_u64(),
        "the same kind was rearmed only after its prior matching batch fired and recovery advanced"
    );
    assert!(dropped[1]["dropped_tick"].as_u64() > dropped[1]["arm_tick"].as_u64());
    assert!(report["trace"]["complete"].as_bool().unwrap_or(false));
    let expected = [
        FrozenCommand {
            slot: 0,
            seq: 1,
            kind: FrozenKind::Gather { unit: 2, node: 1 },
        },
        FrozenCommand {
            slot: 0,
            seq: 2,
            kind: FrozenKind::Gather { unit: 2, node: 1 },
        },
    ];
    let log = economy_log_path(&server);
    let oracle = replay_public_log(log, "haul", &[(0, 30_001)], &expected);
    let tick = assert_report_matches_own_audit_tick(log, &report);
    assert!(tick <= audit_last_tick(log));
    assert_raw_wire_owner_records(log, &raw_wire, 0, None);
    assert_delayed_first_snapshot_then_successor(&raw_wire, 12);
    assert_eq!(report["successor_requests"].as_u64(), Some(1));
    assert!(
        oracle.extraction > 0,
        "independent replay sees extraction progress"
    );
    assert!(
        oracle.delivery > 0,
        "independent replay sees a completed delivery"
    );

    let base_initial = independent_public_model("base").0;
    let base_center = unit_id_for_owner_and_content(&base_initial, 0, "base:command_center");
    let gather = [FrozenCommand {
        slot: 0,
        seq: 1,
        kind: FrozenKind::Gather { unit: 2, node: 1 },
    }];
    run_world_change_subcase(
        "conversion",
        "base",
        "base-gather-trace.json",
        31_001,
        20,
        &gather,
    );
    let produce = [FrozenCommand {
        slot: 0,
        seq: 1,
        kind: FrozenKind::Produce {
            producer: base_center,
            recipe: "base:train_rifleman",
        },
    }];
    run_world_change_subcase(
        "charge",
        "base",
        "base-output-trace.json",
        32_001,
        4,
        &produce,
    );
    let cancel = [
        FrozenCommand {
            slot: 0,
            seq: 1,
            kind: FrozenKind::Produce {
                producer: base_center,
                recipe: "base:train_rifleman",
            },
        },
        FrozenCommand {
            slot: 0,
            seq: 2,
            kind: FrozenKind::CancelFirstJob {
                producer: base_center,
            },
        },
    ];
    run_world_change_subcase("refund", "base", "base-trace.json", 33_001, 4, &cancel);
    run_world_change_subcase(
        "spawn",
        "base",
        "base-output-trace.json",
        34_001,
        8,
        &produce,
    );
    run_world_change_subcase(
        "final_change",
        "base",
        "base-output-trace.json",
        35_001,
        4,
        &produce,
    );
    run_replay_spawn_window_subcase();
}

#[test]
#[ignore = "real public Cedar package process acceptance; run through scripts/acceptance.sh"]
fn e10_cedar_uses_public_pack_and_boot() {
    let package_dir = support::test_dir("e10-cedar-economy");
    let package = support::pack("cedar", &package_dir);
    let server = start_economy_server(&package, &package_dir);
    let trace = trace_file(&server, "cedar-trace.json");
    let report = run_trace_client(&server, 40_001, &trace, 12, &[]);
    assert_eq!(report["welcomed"], true);
    assert_eq!(report["trace"]["complete"], true);
    let world = &report["replicated_world"];
    assert!(world["tick"].as_u64().unwrap_or(0) > 0);
    assert!(
        world["economy"]["inventories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|inventory| {
                inventory["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["on_hand"].as_u64().unwrap_or(0) > 0)
            }),
        "public Cedar harvest reaches the owner's inventory"
    );
    let expected = [FrozenCommand {
        slot: 0,
        seq: 1,
        kind: FrozenKind::Gather { unit: 2, node: 1 },
    }];
    let oracle = replay_public_log(
        economy_log_path(&server),
        "cedar",
        &[(0, 40_001)],
        &expected,
    );
    assert!(
        oracle.extraction > 0,
        "frozen public Cedar catalog predicts extraction"
    );
    assert!(
        oracle.delivery > 0,
        "frozen public Cedar catalog predicts delivery"
    );

    let old_api_dir = support::test_dir("e10-cedar-old-engine-api");
    let old_api_package = support::pack_with(
        "cedar",
        "manifest.ron",
        &std::fs::read_to_string(support::mods_dir().join("cedar/manifest.ron"))
            .unwrap()
            .replace("engine_api: \"^0.2.3\"", "engine_api: \"^0.1\""),
        &old_api_dir,
    );
    let invalid_archive = std::fs::read(&old_api_package).unwrap();
    let parse_error = turts_mod_runtime::archive::PackageArchive::parse(&invalid_archive)
        .expect_err("container parses through to the unsupported engine_api field");
    assert_eq!(parse_error.field_path, "engine_api");
    let boot_error = support::expect_server_boot_failure(&old_api_package);
    assert!(
        boot_error.contains("engine api `^0.1` does not accept engine"),
        "actual server boot reports the public engine API incompatibility: {boot_error}"
    );
}

fn economy_fixture_app_with_second_yard() -> bevy_app::App {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../turts_pack/tests/fixtures/economy");
    let mut files = std::collections::BTreeMap::new();
    package_files(&root, &root, &mut files);
    let map = String::from_utf8(files.get("maps/test.ron").unwrap().clone()).unwrap();
    let needle = "owner_slot: 1, pos: (x: 52.0, y: 8.0), contents: [])], nodes:";
    let replacement = "owner_slot: 1, pos: (x: 52.0, y: 8.0), contents: []), (unit: \"probe:yard\", owner_slot: 0, pos: (x: 20.0, y: 20.0), contents: [(item: \"probe:timber\", amount: 100)])], nodes:";
    assert!(
        map.contains(needle),
        "fixture map has the frozen placement tail"
    );
    files.insert(
        "maps/test.ron".into(),
        map.replace(needle, replacement).into_bytes(),
    );
    let package = turts_mod_runtime::pack::pack_files(&files).unwrap();
    let archive = turts_mod_runtime::archive::PackageArchive::parse(&package).unwrap();
    let mut app = turts_sim::headless_app(20);
    turts_pack::load_package(app.world_mut(), &archive).unwrap();
    app
}

#[test]
fn stateful_oracle_keeps_claim_job_and_ordered_producers_across_ticks() {
    use std::collections::BTreeMap;
    use turts_sim::{
        AuditTrail, Command, CommandKind, CommandQueue, SlotId, Slots, economy::views::audit_state,
    };

    let mut app = economy_fixture_app_with_second_yard();
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = accounting::AccountingOracle::new(&initial, &catalog);
    assert_eq!(
        app.world_mut().resource_mut::<Slots>().claim(77).unwrap(),
        SlotId(0)
    );
    oracle.apply_frozen_hello_claim(0, 0, 77).unwrap();

    let frozen = [
        Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::MoveTo {
                unit_ids: vec![3],
                target: bevy_math::Vec2::new(18.0, 8.0),
            },
        },
        Command {
            slot: SlotId(0),
            seq: 2,
            kind: CommandKind::Produce {
                producer_id: 1,
                recipe_id: turts_mod_api::ContentId::parse("probe:train").unwrap(),
            },
        },
        Command {
            slot: SlotId(0),
            seq: 3,
            kind: CommandKind::Produce {
                producer_id: 6,
                recipe_id: turts_mod_api::ContentId::parse("probe:train").unwrap(),
            },
        },
    ];
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .extend(frozen.iter().cloned());

    let mut all_command_audits = Vec::new();
    for tick in 1..=2 {
        if tick == 2 {
            let prior = oracle.modeled_state();
            let job_id = prior
                .economy
                .queues
                .iter()
                .find(|queue| queue.producer == 1)
                .and_then(|queue| queue.jobs.first())
                .expect("frozen paid produce remains live across the accepted claim")
                .id;
            assert_eq!(
                app.world_mut().resource_mut::<Slots>().claim(88).unwrap(),
                SlotId(1)
            );
            oracle.apply_frozen_hello_claim(1, 1, 88).unwrap();
            let cancel = Command {
                slot: SlotId(0),
                seq: 4,
                kind: CommandKind::CancelProduction {
                    producer_id: 1,
                    job_id: turts_sim::ids::JobId(job_id),
                },
            };
            frozen_command_queue(&mut app, cancel);
        }

        app.update();
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        assert!(trail.valid);
        let command_audits = trail.drain_commands();
        let operations = trail.drain_operations();
        let tick_audit = trail.drain_ticks().pop().expect("completed captured tick");
        assert_eq!(tick_audit.tick, tick);
        let by_id: BTreeMap<_, _> = command_audits
            .iter()
            .map(|audit| (audit.operation_id, audit))
            .collect();

        oracle.advance_clock_for_command_tick(tick).unwrap();
        let mut gatherers: Option<Vec<u32>> = None;
        let mut gather_index = 0;
        let mut producers: Option<Vec<u32>> = None;
        let mut producer_index = 0;
        for operation in &operations {
            if let Some(audit) = by_id.get(&operation.operation_id) {
                let expected = if tick == 1 {
                    frozen
                        .iter()
                        .find(|command| command.seq == audit.input.seq)
                        .unwrap()
                        .clone()
                } else {
                    let state = oracle.modeled_state();
                    let job_id = state
                        .economy
                        .queues
                        .iter()
                        .find(|queue| queue.producer == 1)
                        .and_then(|queue| queue.jobs.first())
                        .unwrap()
                        .id;
                    Command {
                        slot: SlotId(0),
                        seq: 4,
                        kind: CommandKind::CancelProduction {
                            producer_id: 1,
                            job_id: turts_sim::ids::JobId(job_id),
                        },
                    }
                };
                assert_eq!(
                    audit.input, expected,
                    "command input is frozen independently"
                );
                oracle
                    .observe(
                        &accounting::ExpectedOperation::Command(expected),
                        &operation.before,
                        &operation.after,
                        &operation.events,
                        Some(&audit.result),
                    )
                    .unwrap();
                all_command_audits.push((*audit).clone());
                continue;
            }

            let expected_gatherers = gatherers.get_or_insert_with(|| {
                oracle
                    .modeled_state()
                    .raw_orders
                    .iter()
                    .map(|order| order.unit)
                    .collect()
            });
            let expected = if let Some(unit) = expected_gatherers.get(gather_index).copied() {
                gather_index += 1;
                accounting::ExpectedOperation::Automatic {
                    tick,
                    phase: accounting::AutomaticPhase::Gather,
                    subject: accounting::AutomaticSubject::Gatherer(unit),
                }
            } else {
                let expected_producers = producers.get_or_insert_with(|| {
                    oracle
                        .modeled_state()
                        .economy
                        .queues
                        .iter()
                        .filter(|queue| !queue.jobs.is_empty())
                        .map(|queue| queue.producer)
                        .collect()
                });
                let producer = *expected_producers.get(producer_index).unwrap();
                let job = oracle
                    .modeled_state()
                    .economy
                    .queues
                    .iter()
                    .find(|queue| queue.producer == producer)
                    .and_then(|queue| queue.jobs.first())
                    .unwrap();
                producer_index += 1;
                accounting::ExpectedOperation::Automatic {
                    tick,
                    phase: accounting::AutomaticPhase::ProductionAdvance,
                    subject: accounting::AutomaticSubject::HeadJob {
                        producer,
                        job: job.id,
                    },
                }
            };
            oracle
                .observe(
                    &expected,
                    &operation.before,
                    &operation.after,
                    &operation.events,
                    None,
                )
                .unwrap_or_else(|error| panic!("operation {}: {error}", operation.operation_id));
        }
        oracle
            .observe(
                &accounting::ExpectedOperation::TickBoundary { tick },
                &tick_audit.before,
                &tick_audit.after,
                &tick_audit.events,
                None,
            )
            .unwrap();
        if tick == 1 {
            assert_eq!(oracle.gross_charge, 200);
            assert_eq!(
                oracle
                    .modeled_state()
                    .raw_units
                    .iter()
                    .find(|unit| unit.unit_id == 3)
                    .unwrap()
                    .pos
                    .x
                    .to_bits(),
                (12.0_f32 + 3.0_f32 / 20.0_f32).to_bits(),
                "the first automatic capture follows the predicted worker movement"
            );
        }
    }
    assert_eq!(
        oracle.gross_charge, 200,
        "both modeled producer visits retain paid Produce provenance"
    );
    assert_eq!(
        oracle.refunds, 100,
        "the later Cancel refunds from the frozen live job"
    );
    assert_eq!(oracle.cancellations, 1);
    oracle.assert_command_records(&all_command_audits).unwrap();
    assert_eq!(all_command_audits.len(), 4);
}

#[test]
fn stateful_oracle_consumes_movement_phase_when_gather_sets_target() {
    use std::collections::BTreeMap;
    use turts_sim::{
        AuditTrail, Command, CommandKind, CommandQueue, SlotId, Slots, economy::views::audit_state,
    };

    let mut app = economy_fixture_app_at(5);
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = accounting::AccountingOracle::new(&initial, &catalog);
    assert_eq!(
        app.world_mut().resource_mut::<Slots>().claim(91).unwrap(),
        SlotId(0)
    );
    oracle.apply_frozen_hello_claim(0, 0, 91).unwrap();

    let frozen = Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(frozen.clone());
    let mut moved_at_tick_two = false;

    for tick in 1..=2 {
        app.update();
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        assert!(trail.valid);
        let command_audits = trail.drain_commands();
        let operations = trail.drain_operations();
        let tick_audit = trail.drain_ticks().pop().expect("completed captured tick");
        assert_eq!(tick_audit.tick, tick);
        let by_id: BTreeMap<_, _> = command_audits
            .iter()
            .map(|audit| (audit.operation_id, audit))
            .collect();

        if tick == 2 {
            let mut missing_movement =
                accounting::AccountingOracle::new(oracle.modeled_state(), &catalog);
            missing_movement
                .advance_clock_for_command_tick(tick)
                .unwrap();
            let active_target_without_step = missing_movement.modeled_state().clone();
            assert!(
                active_target_without_step
                    .raw_units
                    .iter()
                    .find(|unit| unit.unit_id == 3)
                    .unwrap()
                    .move_target
                    .is_some(),
                "the frozen prior model has an active Gather movement target"
            );
            assert!(
                missing_movement
                    .observe(
                        &accounting::ExpectedOperation::Automatic {
                            tick,
                            phase: accounting::AutomaticPhase::Gather,
                            subject: accounting::AutomaticSubject::Gatherer(3),
                        },
                        &active_target_without_step,
                        &active_target_without_step,
                        &[],
                        None,
                    )
                    .is_err(),
                "the first automatic capture must reject an active target with no predicted movement"
            );
        }
        oracle.advance_clock_for_command_tick(tick).unwrap();
        let mut automatic_index = 0;
        for operation in &operations {
            if let Some(audit) = by_id.get(&operation.operation_id) {
                assert_eq!(tick, 1);
                assert_eq!(audit.input, frozen);
                oracle
                    .observe(
                        &accounting::ExpectedOperation::Command(frozen.clone()),
                        &operation.before,
                        &operation.after,
                        &operation.events,
                        Some(&audit.result),
                    )
                    .unwrap();
                continue;
            }

            let gatherer = [3_u32, 5_u32][automatic_index];
            automatic_index += 1;
            if tick == 1 && gatherer == 3 {
                let unit = operation
                    .before
                    .raw_units
                    .iter()
                    .find(|unit| unit.unit_id == gatherer)
                    .unwrap();
                assert!(unit.move_target.is_none());
                let after = operation
                    .after
                    .raw_units
                    .iter()
                    .find(|unit| unit.unit_id == gatherer)
                    .unwrap();
                assert!(
                    after.move_target.is_some(),
                    "Gather starts travel during its automatic operation"
                );
                assert_eq!(unit.pos, after.pos, "movement waits until the next tick");
            }
            if tick == 1 && gatherer == 5 {
                let mut fabricated = operation.before.clone();
                fabricated
                    .raw_units
                    .iter_mut()
                    .find(|unit| unit.unit_id == 3)
                    .unwrap()
                    .pos
                    .x += 0.6;
                fabricated
                    .economy
                    .units
                    .iter_mut()
                    .find(|unit| unit.unit_id == 3)
                    .unwrap()
                    .pos
                    .x += 0.6;
                assert!(
                    oracle
                        .observe(
                            &accounting::ExpectedOperation::Automatic {
                                tick,
                                phase: accounting::AutomaticPhase::Gather,
                                subject: accounting::AutomaticSubject::Gatherer(gatherer),
                            },
                            &fabricated,
                            &operation.after,
                            &operation.events,
                            None,
                        )
                        .is_err(),
                    "a second movement discontinuity after Gather must be rejected"
                );
            }
            if tick == 2 && gatherer == 3 {
                let before = operation
                    .before
                    .raw_units
                    .iter()
                    .find(|unit| unit.unit_id == gatherer)
                    .unwrap();
                let prior = oracle
                    .modeled_state()
                    .raw_units
                    .iter()
                    .find(|unit| unit.unit_id == gatherer)
                    .unwrap();
                assert_ne!(before.pos.x.to_bits(), prior.pos.x.to_bits());
                moved_at_tick_two = true;
            }
            oracle
                .observe(
                    &accounting::ExpectedOperation::Automatic {
                        tick,
                        phase: accounting::AutomaticPhase::Gather,
                        subject: accounting::AutomaticSubject::Gatherer(gatherer),
                    },
                    &operation.before,
                    &operation.after,
                    &operation.events,
                    None,
                )
                .unwrap();
        }
        assert_eq!(automatic_index, 2);
        oracle
            .observe(
                &accounting::ExpectedOperation::TickBoundary { tick },
                &tick_audit.before,
                &tick_audit.after,
                &tick_audit.events,
                None,
            )
            .unwrap();
    }
    assert!(
        moved_at_tick_two,
        "worker movement is predicted before next tick's Gather"
    );
}

fn frozen_command_queue(app: &mut bevy_app::App, command: turts_sim::Command) {
    app.world_mut()
        .resource_mut::<turts_sim::CommandQueue>()
        .0
        .push_back(command);
}
