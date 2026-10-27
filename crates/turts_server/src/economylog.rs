//! Task-local trusted JSONL encoding and typed replay checks for economy audits.
//!
//! This module is intentionally not included from `main.rs`: it is preparation
//! for a later wiring task and does not create a runtime diagnostic interface.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Write};

use serde::{Deserialize, Serialize};
use turts_sim::{AuditState, CommandAudit, EconomyEvent, OperationAudit, TickAudit};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "record", content = "data", rename_all = "snake_case")]
pub(crate) enum EconomyLogRecord {
    Initial(AuditState),
    CommandBoundary(CommandAudit),
    Operation(OperationAudit),
    Tick {
        tick: u32,
        before: AuditState,
        state: AuditState,
        events: Vec<EconomyEvent>,
    },
}

impl EconomyLogRecord {
    #[allow(dead_code)]
    fn from_tick(audit: TickAudit) -> Self {
        Self::Tick {
            tick: audit.tick,
            before: audit.before,
            state: audit.after,
            events: audit.events,
        }
    }
}

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
        self.write(&EconomyLogRecord::Initial(state))
    }

    #[allow(dead_code)]
    pub(crate) fn write_command(
        &mut self,
        trail: &turts_sim::AuditTrail,
        audit: CommandAudit,
    ) -> Result<(), std::io::Error> {
        ensure_valid(trail)?;
        self.write(&EconomyLogRecord::CommandBoundary(audit))
    }

    #[allow(dead_code)]
    pub(crate) fn write_operation(
        &mut self,
        trail: &turts_sim::AuditTrail,
        audit: OperationAudit,
    ) -> Result<(), std::io::Error> {
        ensure_valid(trail)?;
        self.write(&EconomyLogRecord::Operation(audit))
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

    pub(crate) fn into_inner(self) -> W {
        self.output
    }
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

pub(crate) fn read_jsonl<R: BufRead>(input: R) -> Result<Vec<EconomyLogRecord>, String> {
    let mut records = Vec::new();
    for (index, line) in input.lines().enumerate() {
        let line = line.map_err(|error| format!("read JSONL line {}: {error}", index + 1))?;
        if line.trim().is_empty() {
            return Err(format!("JSONL line {} is empty", index + 1));
        }
        let record = serde_json::from_str(&line)
            .map_err(|error| format!("decode JSONL line {}: {error}", index + 1))?;
        records.push(record);
    }
    validate_records(&records)?;
    Ok(records)
}

#[allow(dead_code)]
pub(crate) fn read_jsonl_file(
    path: impl AsRef<std::path::Path>,
) -> Result<Vec<EconomyLogRecord>, String> {
    let file = std::fs::File::open(path.as_ref())
        .map_err(|error| format!("open {}: {error}", path.as_ref().display()))?;
    read_jsonl(BufReader::new(file))
}

/// Check record shape, R14 clock transitions, command-operation association, and
/// the published tick boundary. Operation expectations remain external to this log.
pub(crate) fn validate_records(records: &[EconomyLogRecord]) -> Result<(), String> {
    let Some(EconomyLogRecord::Initial(initial)) = records.first() else {
        return Err("first economy log record must be Initial".into());
    };
    if records
        .iter()
        .skip(1)
        .any(|record| matches!(record, EconomyLogRecord::Initial(_)))
    {
        return Err("economy log contains more than one Initial record".into());
    }

    let mut cursor = initial.clone();
    let mut tick_start = initial.clone();
    let mut commands = BTreeMap::new();
    let mut operations = BTreeSet::new();
    let mut last_operation: Option<u64> = None;
    let mut last_tick = initial.economy.tick;
    let mut tick_events = initial.pending_events.clone();

    for record in records.iter().skip(1) {
        match record {
            EconomyLogRecord::Initial(_) => unreachable!("initial count checked above"),
            EconomyLogRecord::CommandBoundary(command) => {
                if command.result.slot != command.input.slot
                    || command.result.seq != command.input.seq
                {
                    return Err(format!(
                        "command result identity differs from input for operation {}",
                        command.operation_id
                    ));
                }
                if commands.insert(command.operation_id, command).is_some() {
                    return Err(format!(
                        "duplicate command operation id {}",
                        command.operation_id
                    ));
                }
            }
            EconomyLogRecord::Operation(operation) => {
                if !operations.insert(operation.operation_id) {
                    return Err(format!("duplicate operation id {}", operation.operation_id));
                }
                if last_operation
                    .is_some_and(|previous| previous.checked_add(1) != Some(operation.operation_id))
                {
                    return Err(format!(
                        "operation id {} is out of order or discontinuous",
                        operation.operation_id
                    ));
                }
                last_operation = Some(operation.operation_id);

                if operation.before.economy.tick != operation.after.economy.tick {
                    return Err(format!(
                        "operation {} changes the simulation clock",
                        operation.operation_id
                    ));
                }
                validate_clock_continuity(&cursor, &operation.before, "operation")?;
                if operation
                    .events
                    .iter()
                    .any(|event| event.tick != operation.before.economy.tick)
                {
                    return Err(format!(
                        "operation {} contains an event from another tick",
                        operation.operation_id
                    ));
                }
                let mut expected_pending = operation.before.pending_events.clone();
                expected_pending.extend(operation.events.iter().cloned());
                if operation.after.pending_events != expected_pending {
                    return Err(format!(
                        "operation {} event contribution differs from its after state",
                        operation.operation_id
                    ));
                }
                if let Some(command) = commands.remove(&operation.operation_id) {
                    if command.before != operation.before
                        || command.after != operation.after
                        || validate_clock_continuity(
                            &cursor,
                            &operation.before,
                            "command operation",
                        )
                        .is_err()
                    {
                        return Err(format!(
                            "command/operation state mismatch for id {}",
                            operation.operation_id
                        ));
                    }
                }
                cursor = operation.after.clone();
                tick_events.extend(operation.events.iter().cloned());
            }
            EconomyLogRecord::Tick {
                tick,
                before,
                state,
                events,
            } => {
                if last_tick.checked_add(1) != Some(*tick) {
                    return Err(format!("tick {tick} is out of order or discontinuous"));
                }
                if before != &tick_start {
                    return Err(format!(
                        "tick {tick} before snapshot differs from prior tick state"
                    ));
                }
                if before.economy.tick.checked_add(1) != Some(*tick) || state.economy.tick != *tick
                {
                    return Err(format!("tick {tick} state clock is inconsistent"));
                }
                tick_events.sort_by_key(|event| event.ordinal);
                if events != &tick_events {
                    return Err(format!(
                        "tick {tick} published events differ from operation contributions"
                    ));
                }
                if !events
                    .windows(2)
                    .all(|pair| pair[0].ordinal < pair[1].ordinal)
                    || events.iter().any(|event| event.tick != *tick)
                {
                    return Err(format!(
                        "tick {tick} events are misordered or from another tick"
                    ));
                }
                if !state.pending_events.is_empty() || state.next_event_ordinal != Some(0) {
                    return Err(format!(
                        "tick {tick} did not drain and reset pending events"
                    ));
                }
                validate_tick_state(&cursor, state, *tick)?;
                cursor = state.clone();
                tick_start = state.clone();
                tick_events.clear();
                last_tick = *tick;
            }
        }
    }
    if let Some(operation_id) = commands.keys().next() {
        return Err(format!(
            "command operation id {operation_id} has no matching operation"
        ));
    }
    Ok(())
}

fn validate_clock_continuity(
    previous: &AuditState,
    current: &AuditState,
    label: &str,
) -> Result<(), String> {
    if previous == current {
        return Ok(());
    }
    let Some(next_tick) = previous.economy.tick.checked_add(1) else {
        return Err(format!("{label} clock overflow"));
    };
    let mut advanced = previous.clone();
    advanced.economy.tick = next_tick;
    if &advanced == current {
        Ok(())
    } else {
        Err(format!(
            "{label} before snapshot is discontinuous from the prior modeled state"
        ))
    }
}

/// A completed tick may move units between recorded operations. Ignore only the
/// movement-facing coordinates/targets; all economy, ownership, order, event, and id
/// state must still follow from the preceding typed record.
fn validate_tick_state(previous: &AuditState, state: &AuditState, tick: u32) -> Result<(), String> {
    let mut expected = previous.clone();
    expected.economy.tick = tick;
    expected.pending_events.clear();
    expected.next_event_ordinal = Some(0);

    for expected_unit in &mut expected.raw_units {
        let Some(actual) = state
            .raw_units
            .iter()
            .find(|unit| unit.unit_id == expected_unit.unit_id)
        else {
            return Err(format!(
                "tick {tick} removes raw unit {}",
                expected_unit.unit_id
            ));
        };
        expected_unit.pos = actual.pos.clone();
        expected_unit.move_target = actual.move_target.clone();
    }
    for expected_unit in &mut expected.units {
        let Some(actual) = state
            .units
            .iter()
            .find(|unit| unit.unit_id == expected_unit.unit_id)
        else {
            return Err(format!(
                "tick {tick} removes unit {}",
                expected_unit.unit_id
            ));
        };
        expected_unit.pos = actual.pos.clone();
    }
    for expected_unit in &mut expected.economy.units {
        let Some(actual) = state
            .economy
            .units
            .iter()
            .find(|unit| unit.unit_id == expected_unit.unit_id)
        else {
            return Err(format!(
                "tick {tick} removes world unit {}",
                expected_unit.unit_id
            ));
        };
        expected_unit.pos = actual.pos.clone();
    }
    if expected != *state {
        return Err(format!(
            "tick {tick} after snapshot differs from the prior operation state"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{EconomyLogRecord, EconomyLogWriter, read_jsonl, validate_records};
    use std::io::Cursor;
    use turts_mod_api::WorldPos;
    use turts_sim::economy::views::RawUnitAudit;
    use turts_sim::{AuditState, AuditTrail, headless_app};

    fn initial() -> AuditState {
        let mut app = headless_app(10);
        turts_sim::audit_state(app.world_mut())
    }

    #[test]
    fn initial_state_round_trips_without_field_loss() {
        let state = initial();
        let original = EconomyLogRecord::Initial(state.clone());
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
        assert_eq!(decoded_state, &state);
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
    fn reader_rejects_malformed_and_empty_lines() {
        assert!(
            read_jsonl(Cursor::new(b"{bad}\n".as_slice()))
                .unwrap_err()
                .contains("line 1")
        );
        assert!(
            read_jsonl(Cursor::new(b"\n".as_slice()))
                .unwrap_err()
                .contains("empty")
        );
    }

    #[test]
    fn replay_validation_requires_initial_and_matching_operation() {
        assert!(validate_records(&[]).unwrap_err().contains("Initial"));
        let state = initial();
        let records = vec![EconomyLogRecord::Initial(state)];
        validate_records(&records).unwrap();
    }
}
