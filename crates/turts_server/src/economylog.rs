//! Task-local trusted JSONL encoding and ledger audit checks for economy audits.
//!
//! The optional runtime sink is enabled by `main.rs` only when `--economy-log`
//! is supplied. The sink writes the ledger view of each snapshot (`LedgerAudit`, A1c). The
//! Task 15 acceptance harness also includes this module for ledger audit and process-level
//! evidence checks.

use std::io::{BufWriter, Write};
use std::path::Path;

use bevy::prelude::Resource;

use turts_audit::EconomyLogRecord;
use turts_net_protocol::CommandRejectReason;
use turts_sim::{AuditState, AuditTrail, CommandAudit, OperationAudit, TickAudit};

pub(crate) struct EconomyLogWriter<W> {
    output: W,
}

impl<W: Write> EconomyLogWriter<W> {
    pub(crate) fn new(output: W) -> Self {
        Self { output }
    }

    fn write(&mut self, record: &EconomyLogRecord) -> Result<(), std::io::Error> {
        serde_json::to_writer(&mut self.output, record)?;
        self.output.write_all(b"\n")
    }

    pub(crate) fn write_initial(
        &mut self,
        trail: &turts_sim::AuditTrail,
        state: AuditState,
    ) -> Result<(), std::io::Error> {
        ensure_valid(trail)?;
        self.write(&EconomyLogRecord::initial(&state))
    }

    #[allow(dead_code)]
    pub(crate) fn write_command(
        &mut self,
        trail: &turts_sim::AuditTrail,
        audit: CommandAudit,
    ) -> Result<(), std::io::Error> {
        ensure_valid(trail)?;
        self.write(&EconomyLogRecord::command(audit))
    }

    #[allow(dead_code)]
    pub(crate) fn write_operation(
        &mut self,
        trail: &turts_sim::AuditTrail,
        audit: OperationAudit,
    ) -> Result<(), std::io::Error> {
        ensure_valid(trail)?;
        self.write(&EconomyLogRecord::operation(audit))
    }

    #[allow(dead_code)]
    pub(crate) fn write_tick(
        &mut self,
        trail: &turts_sim::AuditTrail,
        audit: TickAudit,
    ) -> Result<(), std::io::Error> {
        ensure_valid(trail)?;
        self.write(&EconomyLogRecord::from_tick(audit))
    }

    pub(crate) fn flush(&mut self) -> Result<(), std::io::Error> {
        self.output.flush()
    }

    #[allow(dead_code)] // Used by the standalone JSONL tests to inspect captured bytes.
    pub(crate) fn into_inner(self) -> W {
        self.output
    }
}

/// Process-local file sink. It is created only when `--economy-log` is present.
#[derive(Resource)]
#[allow(dead_code)] // Standalone logger tests include this module without the server process sink.
pub(crate) struct EconomyLogSink {
    writer: EconomyLogWriter<BufWriter<std::fs::File>>,
    pub(crate) error: Option<String>,
}

#[allow(dead_code)] // Process-only sink methods are intentionally absent from standalone logger tests.
impl EconomyLogSink {
    pub(crate) fn create(
        path: impl AsRef<Path>,
        trail: &AuditTrail,
        initial: AuditState,
    ) -> Result<Self, std::io::Error> {
        ensure_valid(trail)?;
        let file = std::fs::File::create(path)?;
        let mut sink = Self {
            writer: EconomyLogWriter::new(BufWriter::new(file)),
            error: None,
        };
        sink.writer.write_initial(trail, initial)?;
        sink.writer.flush()?;
        Ok(sink)
    }

    pub(crate) fn write_claim(&mut self, tick: u32, player_id: u64, slot: u8) {
        self.write_record(EconomyLogRecord::Claim {
            tick,
            player_id,
            slot,
        });
    }

    #[allow(clippy::too_many_arguments)] // Preserve the explicit fields of the trusted admission record.
    pub(crate) fn write_admission(
        &mut self,
        ordinal: u64,
        connection: u64,
        player_id: u64,
        slot: u8,
        seq: u32,
        tick: u32,
        reason: CommandRejectReason,
        raw_envelope: Vec<u8>,
        before: AuditState,
        after: AuditState,
    ) {
        self.write_record(EconomyLogRecord::Admission {
            ordinal,
            connection,
            player_id,
            slot,
            seq,
            tick,
            reason,
            wire_format: "postcard".to_owned(),
            raw_envelope,
            before: turts_sim::ledger_audit(&before),
            after: turts_sim::ledger_audit(&after),
        });
    }

    pub(crate) fn capture_completed_tick(&mut self, world: &mut bevy::prelude::World) {
        if self.error.is_some() {
            return;
        }
        let (enabled, valid, commands, operations, ticks) = {
            let Some(mut trail) = world.get_resource_mut::<AuditTrail>() else {
                self.fail("economy audit trail is missing".to_owned());
                return;
            };
            let enabled = trail.enabled;
            let valid = trail.valid;
            if !enabled || !valid {
                self.fail("economy audit capture became disabled or invalid".to_owned());
                return;
            }
            (
                enabled,
                valid,
                trail.drain_commands(),
                trail.drain_operations(),
                trail.drain_ticks(),
            )
        };
        if !enabled || !valid {
            self.fail("economy audit capture became disabled or invalid".to_owned());
            return;
        }
        for command in commands {
            if let Err(error) = self.writer.write(&EconomyLogRecord::command(command)) {
                self.fail(format!("write command audit: {error}"));
                return;
            }
        }
        for operation in operations {
            if let Err(error) = self.writer.write(&EconomyLogRecord::operation(operation)) {
                self.fail(format!("write operation audit: {error}"));
                return;
            }
        }
        for tick in ticks {
            if let Err(error) = self.writer.write(&EconomyLogRecord::from_tick(tick)) {
                self.fail(format!("write tick audit: {error}"));
                return;
            }
        }
        if let Err(error) = self.writer.flush() {
            self.fail(format!("flush economy log: {error}"));
        }
    }

    fn write_record(&mut self, record: EconomyLogRecord) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self
            .writer
            .write(&record)
            .and_then(|()| self.writer.flush())
        {
            self.fail(format!("write economy log: {error}"));
        }
    }

    fn fail(&mut self, message: String) {
        eprintln!("turts_server: economy log evidence failed: {message}");
        self.error = Some(message);
    }
}

#[allow(dead_code)] // The standalone reader tests include, but do not run, the server process sink.
pub(crate) fn capture_completed_tick(world: &mut bevy::prelude::World) {
    let Some(mut sink) = world.remove_resource::<EconomyLogSink>() else {
        return;
    };
    sink.capture_completed_tick(world);
    world.insert_resource(sink);
}

fn ensure_valid(trail: &turts_sim::AuditTrail) -> Result<(), std::io::Error> {
    if !trail.enabled || !trail.valid {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "economy audit capture is disabled or invalid",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::EconomyLogWriter;
    use std::io::Cursor;
    use turts_audit::{EconomyLogRecord, read_jsonl, validate_ledger};
    use turts_mod_api::WorldPos;
    use turts_sim::economy::views::RawUnitAudit;
    use turts_sim::{AuditState, AuditTrail, headless_app};

    fn initial() -> AuditState {
        let mut app = headless_app(10);
        turts_sim::audit_state(app.world_mut())
    }

    /// The ledger audit has no movement model. It accepts the Orders-phase operation of a unit
    /// order, and it reads the phase as a shape rule: a Command phase needs a command boundary
    /// (the replay validator also rejected the phase `Other` here; that rule needed the model of
    /// the movement step, and the movement tests and the writer scan hold it now).
    #[test]
    fn ledger_accepts_an_orders_operation_before_movement() {
        use turts_mod_api::{Footprint, PackageId, UnitDef, UnitRegistry};
        use turts_sim::{
            MapBounds, MoveTarget, Order, OrderTargetSim, Owner, Position, SlotId, UnitId,
            UnitOrders, UnitType, UnitTypes, Vec2,
        };
        let mut app = headless_app(10);
        app.world_mut().insert_resource(MapBounds {
            size: Vec2::new(32.0, 32.0),
        });
        let kind = app
            .world_mut()
            .resource_mut::<UnitTypes>()
            .register_unit_type(
                &PackageId("probe".into()),
                UnitDef {
                    id: "walker".into(),
                    name: "Walker".into(),
                    movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    footprint: Footprint::Circle { radius: 0.5 },
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                },
            )
            .unwrap();
        app.world_mut().spawn((
            UnitId(1),
            UnitType(kind),
            Owner(SlotId(0)),
            Position(Vec2::ZERO),
            MoveTarget(Vec2::new(10.0, 0.0)),
            UnitOrders::default(),
        ));
        app.world_mut().spawn((
            UnitId(2),
            UnitType(kind),
            Owner(SlotId(0)),
            Position(Vec2::new(1.0, 1.0)),
            UnitOrders {
                current: Some(Order {
                    order: turts_mod_api::ContentId::parse("probe:move").unwrap(),
                    behaviour: turts_mod_api::Behaviour::Move,
                    target: OrderTargetSim::Point(Vec2::new(1.0, 1.0)),
                    group: 1,
                }),
                ..Default::default()
            },
        ));
        app.world_mut().resource_mut::<AuditTrail>().enable();
        let initial_state = turts_sim::audit_state(app.world_mut());
        app.update();
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        let operations = trail.drain_operations();
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].unit_id, Some(2));

        let mut writer = EconomyLogWriter::new(Vec::new());
        writer.write_initial(&trail, initial_state).unwrap();
        writer
            .write_operation(&trail, operations[0].clone())
            .unwrap();
        writer.flush().unwrap();
        let mut records = read_jsonl(Cursor::new(writer.output)).unwrap();
        assert_eq!(validate_ledger(&records), Ok(()));

        if let EconomyLogRecord::Operation(operation) = &mut records[1] {
            operation.phase = turts_sim::economy::transaction::AuditPhase::Command;
        } else {
            panic!("captured unit order did not decode as an operation");
        }
        assert!(
            validate_ledger(&records)
                .unwrap_err()
                .contains("operation 1 has Command phase without a command")
        );
    }

    #[test]
    fn initial_state_round_trips_without_field_loss() {
        let state = initial();
        let original = EconomyLogRecord::initial(&state);
        let mut trail = AuditTrail::default();
        trail.enable();
        let mut writer = EconomyLogWriter::new(Vec::new());
        writer.write_initial(&trail, state.clone()).unwrap();
        writer.flush().unwrap();
        let decoded = read_jsonl(Cursor::new(writer.output)).unwrap();
        assert_eq!(decoded, vec![original]);
        let EconomyLogRecord::Initial(decoded_state) = &decoded[0] else {
            panic!("wrong variant")
        };
        assert_eq!(decoded_state, &turts_sim::ledger_audit(&state));
        assert_eq!(decoded_state.raw_units, state.raw_units);
        assert_eq!(decoded_state.raw_inventories, state.raw_inventories);
        assert_eq!(decoded_state.raw_orders, state.raw_orders);
        assert_eq!(decoded_state.raw_queues, state.raw_queues);
        assert_eq!(decoded_state.economy, state.economy);
        assert_eq!(decoded_state.allocators, state.allocators);
        assert_eq!(decoded_state.claims, state.claims);
    }

    #[test]
    fn invalid_capture_is_not_written() {
        let mut writer = EconomyLogWriter::new(Vec::new());
        let mut invalid = AuditTrail::default();
        invalid.enable();
        invalid.valid = false;
        let error = writer.write_initial(&invalid, initial()).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(writer.output.is_empty());
    }

    #[test]
    fn tick_records_retain_before_after_and_events() {
        let mut app = headless_app(10);
        let mut trail = AuditTrail::default();
        trail.enable();
        app.world_mut().insert_resource(trail);
        app.update();
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        let tick = trail.drain_ticks().pop().expect("completed tick audit");
        let expected = EconomyLogRecord::from_tick(tick.clone());
        let mut writer = EconomyLogWriter::new(Vec::new());
        writer.write_initial(&trail, tick.before.clone()).unwrap();
        writer.write_tick(&trail, tick).unwrap();
        writer.flush().unwrap();
        let decoded = read_jsonl(Cursor::new(writer.output)).unwrap();
        assert_eq!(decoded[1], expected);
    }

    /// The sink writes the ledger view of each snapshot, not the full audit state: no snapshot
    /// in the bytes holds a field that `LedgerAudit` drops, and each field of a snapshot is a
    /// kept field (A1c; a sink that writes `AuditState` makes this test red).
    #[test]
    fn sink_writes_only_the_ledger_view_fields() {
        let mut app = headless_app(10);
        let mut trail = AuditTrail::default();
        trail.enable();
        app.world_mut().insert_resource(trail);
        let initial_state = turts_sim::audit_state(app.world_mut());
        app.update();
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        let tick = trail.drain_ticks().pop().expect("completed tick audit");
        let mut writer = EconomyLogWriter::new(Vec::new());
        writer.write_initial(&trail, initial_state).unwrap();
        writer.write_tick(&trail, tick).unwrap();
        writer.flush().unwrap();
        let text = String::from_utf8(writer.output).unwrap();
        let mut snapshots = 0;
        for line in text.lines() {
            let record: serde_json::Value = serde_json::from_str(line).unwrap();
            let data = &record["data"];
            let views: Vec<&serde_json::Value> = match record["record"].as_str().unwrap() {
                "initial" => vec![data],
                "tick" => vec![&data["before"], &data["state"]],
                other => panic!("unexpected record {other}"),
            };
            for view in views {
                for field in turts_sim::LEDGER_DROPPED_FIELDS {
                    assert!(
                        view.get(field).is_none(),
                        "the log holds the dropped field {field}"
                    );
                }
                for key in view.as_object().unwrap().keys() {
                    assert!(
                        turts_sim::LEDGER_KEPT_FIELDS.contains(&key.as_str()),
                        "the log holds the field {key}, which is not a ledger view field"
                    );
                }
                assert!(
                    view.get("economy").is_some(),
                    "the log keeps the economy field"
                );
                snapshots += 1;
            }
        }
        assert_eq!(
            snapshots, 3,
            "one Initial snapshot and the two snapshots of one Tick"
        );
    }

    #[test]
    fn raw_non_finite_positions_round_trip_as_exact_bits() {
        let mut state = initial();
        let x = 0x7fc0_0042;
        let y = 0xff80_0001;
        state.raw_units.push(RawUnitAudit {
            unit_id: 31,
            unit_type: 8,
            owner_slot: 2,
            pos: WorldPos {
                x: f32::from_bits(x),
                y: f32::from_bits(y),
            },
            move_target: Some(WorldPos {
                x: f32::from_bits(y),
                y: f32::from_bits(x),
            }),
        });
        let mut writer = EconomyLogWriter::new(Vec::new());
        let mut trail = AuditTrail::default();
        trail.enable();
        writer.write_initial(&trail, state).unwrap();
        let decoded = read_jsonl(Cursor::new(writer.output)).unwrap();
        let EconomyLogRecord::Initial(state) = &decoded[0] else {
            panic!("wrong variant")
        };
        let raw = &state.raw_units[0];
        assert_eq!(raw.pos.x.to_bits(), x);
        assert_eq!(raw.pos.y.to_bits(), y);
        let target = raw.move_target.as_ref().unwrap();
        assert_eq!(target.x.to_bits(), y);
        assert_eq!(target.y.to_bits(), x);
    }

    #[test]
    fn claims_are_modeled_from_initial_slot_state() {
        let state = initial();
        assert_eq!(state.claims, vec![None, None]);
        let mut writer = EconomyLogWriter::new(Vec::new());
        let mut trail = AuditTrail::default();
        trail.enable();
        writer.write_initial(&trail, state).unwrap();
        writer
            .write(&EconomyLogRecord::Claim {
                tick: 0,
                player_id: 902,
                slot: 0,
            })
            .unwrap();
        let decoded = read_jsonl(Cursor::new(writer.into_inner())).unwrap();
        assert!(matches!(
            decoded[1],
            EconomyLogRecord::Claim {
                slot: 0,
                player_id: 902,
                ..
            }
        ));

        // The ledger audit rejects a claim of a slot that is not configured. The choice of the
        // first open slot is not a ledger rule: `session.rs` tests hold it (A1c: the replay
        // validator also rejected a claim of slot 1 while slot 0 was open).
        let mut unknown_slot = decoded;
        let EconomyLogRecord::Claim { slot, .. } = &mut unknown_slot[1] else {
            panic!("claim record")
        };
        *slot = 9;
        assert!(
            validate_ledger(&unknown_slot)
                .unwrap_err()
                .contains("slots: player 902 claims slot 9, which is not configured")
        );
    }

    #[test]
    fn wire_admission_requires_lossless_neutral_evidence() {
        let state = initial();
        let mut writer = EconomyLogWriter::new(Vec::new());
        let mut trail = AuditTrail::default();
        trail.enable();
        writer.write_initial(&trail, state.clone()).unwrap();
        writer
            .write(&EconomyLogRecord::Admission {
                ordinal: 1,
                connection: 44,
                player_id: 902,
                slot: 0,
                seq: 7,
                tick: 0,
                reason: turts_net_protocol::CommandRejectReason::BadSequence,
                wire_format: "postcard".to_owned(),
                raw_envelope: vec![1, 2, 3],
                before: turts_sim::ledger_audit(&state),
                after: turts_sim::ledger_audit(&state),
            })
            .unwrap();
        let decoded = read_jsonl(Cursor::new(writer.into_inner())).unwrap();
        assert!(matches!(decoded[1], EconomyLogRecord::Admission { .. }));

        let mut changed = decoded;
        let EconomyLogRecord::Admission { after, .. } = &mut changed[1] else {
            panic!("admission record")
        };
        after.economy.tick = 1;
        assert!(
            validate_ledger(&changed).unwrap_err().contains(
                "wire-only rejection for player 902, slot 0, tick 0 changed audited state"
            )
        );
    }
}
