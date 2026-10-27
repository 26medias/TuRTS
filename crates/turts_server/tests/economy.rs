#[path = "../../turts_pack/tests/common/accounting.rs"]
#[allow(dead_code)]
mod accounting;

#[path = "../src/economylog.rs"]
mod economylog;

// Preparation tests exercise only the standalone typed JSONL layer. Process acceptance
// tests are intentionally deferred until the driver and live server wiring exist.
#[test]
fn typed_initial_and_command_record_replay_with_frozen_command_input() {
    use turts_sim::{AuditTrail, Command, CommandKind, CommandQueue, SlotId};

    let mut app = turts_sim::headless_app(10);
    app.world_mut().resource_mut::<AuditTrail>().enable();
    app.update();
    {
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        trail.drain_operations();
        trail.drain_ticks();
    }
    let catalog = turts_mod_api::ContentCatalog {
        resources: Default::default(),
        units: Default::default(),
        nodes: Default::default(),
        production: Default::default(),
    };
    let command = Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::MoveTo {
            unit_ids: Vec::new(),
            target: bevy_math::Vec2::ZERO,
        },
    };
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(command.clone());
    app.update();

    let mut trail = app.world_mut().resource_mut::<AuditTrail>();
    assert!(trail.valid);
    let command_record = trail.drain_commands().pop().expect("command audit");
    let operation_record = trail.drain_operations().pop().expect("operation audit");
    assert_eq!(command_record.operation_id, operation_record.operation_id);
    // The boundary snapshot seeds the replay cursor; the expected command itself is
    // frozen above from test input and is not inferred from this observed record.
    let initial = command_record.before.clone();
    let mut oracle = accounting::AccountingOracle::new(&initial, &catalog);
    oracle
        .observe(
            &accounting::ExpectedOperation::Command(command),
            &command_record.before,
            &command_record.after,
            &operation_record.events,
            Some(&command_record.result),
        )
        .expect("independent oracle accepts the independently frozen input");
    oracle
        .assert_command_records(std::slice::from_ref(&command_record))
        .unwrap();

    let records = vec![
        economylog::EconomyLogRecord::Initial(initial),
        economylog::EconomyLogRecord::CommandBoundary(command_record),
        economylog::EconomyLogRecord::Operation(operation_record),
    ];
    economylog::validate_records(&records).unwrap();

    // Corrupting the operation correlation or one retained state field must make
    // the typed replay fail, so a future mutation cannot silently pass the audit.
    let mut wrong_id = records.clone();
    let economylog::EconomyLogRecord::Operation(operation) = &mut wrong_id[2] else {
        panic!("expected operation record");
    };
    operation.operation_id += 1;
    assert!(
        economylog::validate_records(&wrong_id)
            .unwrap_err()
            .contains("no matching operation")
    );

    let mut lost_state = records.clone();
    let economylog::EconomyLogRecord::Operation(operation) = &mut lost_state[2] else {
        panic!("expected operation record");
    };
    operation.after.economy.tick = operation.after.economy.tick.wrapping_add(1);
    assert!(
        economylog::validate_records(&lost_state)
            .unwrap_err()
            .contains("state mismatch")
    );
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
