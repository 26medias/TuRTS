#[path = "../../turts_pack/tests/common/accounting.rs"]
#[allow(dead_code)]
mod accounting;

#[path = "../src/economylog.rs"]
mod economylog;

// Preparation tests exercise only the standalone typed JSONL layer. Process acceptance
// tests are intentionally deferred until the driver and live server wiring exist.
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

fn economy_fixture_app() -> bevy_app::App {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../turts_pack/tests/fixtures/economy");
    let mut files = std::collections::BTreeMap::new();
    package_files(&root, &root, &mut files);
    let map = files.get_mut("maps/test.ron").unwrap();
    let map_text = String::from_utf8(std::mem::take(map)).unwrap();
    assert_eq!(map_text.matches("pos: (x: 12.0, y: 8.0)").count(), 1);
    *map = map_text
        .replace("pos: (x: 12.0, y: 8.0)", "pos: (x: 22.0, y: 8.0)")
        .into_bytes();
    let package = turts_mod_runtime::pack::pack_files(&files).unwrap();
    let archive = turts_mod_runtime::archive::PackageArchive::parse(&package).unwrap();
    let mut app = turts_sim::headless_app(5);
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
fn decoded_mixed_tick_replays_frozen_gather_command_and_automatic_effect() {
    use std::io::Cursor;
    use turts_sim::{
        AuditTrail, Command, CommandKind, CommandQueue, SlotId, economy::views::audit_state,
    };

    let mut app = economy_fixture_app();
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

    // Capture the actual fixed-tick ordering: clock advance, command admission,
    // gather operations, publication. Tick one arms the worker; tick two extracts.
    for tick_number in 1..=2 {
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
    for tick_number in 1..=2 {
        // R14 anchors TickAudit.before at t-1 while the first command/operation
        // boundary is at t. Derive this one clock step from the prior model.
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
            economylog::EconomyLogRecord::Tick { state, tick, .. } if *tick == 2 => Some(state),
            _ => None,
        })
        .unwrap();
    tick_record.economy.tick += 1;
    assert!(economylog::validate_records(&bad_tick_state).is_err());

    let mut bad_economy_state = decoded.clone();
    let worker_inventory = bad_economy_state
        .iter_mut()
        .find_map(|record| match record {
            economylog::EconomyLogRecord::Tick { state, tick, .. } if *tick == 2 => state
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
            economylog::EconomyLogRecord::Tick { events, tick, .. } if *tick == 2 => Some(events),
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
