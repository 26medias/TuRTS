//! Task-local trusted JSONL encoding and typed replay checks for economy audits.
//!
//! The optional runtime sink is enabled by `main.rs` only when `--economy-log`
//! is supplied. The Task 15 acceptance harness also includes this module for
//! typed replay and process-level evidence checks.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use bevy::prelude::Resource;

use serde::{Deserialize, Serialize};
use turts_net_protocol::CommandRejectReason;
use turts_sim::{AuditState, AuditTrail, CommandAudit, EconomyEvent, OperationAudit, TickAudit};

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
    /// An accepted Hello claim, independently applied to the prior model's slot vector.
    Claim {
        tick: u32,
        player_id: u64,
        slot: u8,
    },
    /// A wire-only refusal; this is deliberately not represented as a typed command audit.
    Admission {
        ordinal: u64,
        connection: u64,
        player_id: u64,
        slot: u8,
        seq: u32,
        tick: u32,
        reason: CommandRejectReason,
        wire_format: String,
        raw_envelope: Vec<u8>,
        before: AuditState,
        after: AuditState,
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
            before,
            after,
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
            if let Err(error) = self
                .writer
                .write(&EconomyLogRecord::CommandBoundary(command))
            {
                self.fail(format!("write command audit: {error}"));
                return;
            }
        }
        for operation in operations {
            if let Err(error) = self.writer.write(&EconomyLogRecord::Operation(operation)) {
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
    // Movement runs once after command admission and before the automatic systems.
    let mut movement_pending = true;

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
                let is_command = commands.contains_key(&operation.operation_id);
                if is_command {
                    validate_clock_continuity(&cursor, &operation.before, "command operation")?;
                } else {
                    validate_operation_continuity(&cursor, &operation.before, movement_pending)?;
                    movement_pending = false;
                }
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
                if let Some(command) = commands.remove(&operation.operation_id)
                    && (command.before != operation.before
                        || command.after != operation.after
                        || validate_clock_continuity(
                            &cursor,
                            &operation.before,
                            "command operation",
                        )
                        .is_err())
                {
                    return Err(format!(
                        "command/operation state mismatch for id {}",
                        operation.operation_id
                    ));
                }
                cursor = operation.after.clone();
                tick_events.extend(operation.events.iter().cloned());
            }
            EconomyLogRecord::Claim {
                tick,
                player_id,
                slot,
            } => {
                if *tick != cursor.economy.tick {
                    return Err(format!(
                        "claim for player {player_id} has a discontinuous tick"
                    ));
                }
                if cursor.claims.contains(&Some(*player_id)) {
                    return Err(format!("player {player_id} claimed more than one slot"));
                }
                let expected_slot = cursor.claims.iter().position(Option::is_none);
                if expected_slot != Some(usize::from(*slot)) {
                    return Err(format!(
                        "player {player_id} claim is not the first open slot"
                    ));
                }
                let Some(claim) = cursor.claims.get_mut(usize::from(*slot)) else {
                    return Err(format!(
                        "player {player_id} claimed out-of-range slot {slot}"
                    ));
                };
                *claim = Some(*player_id);
                let Some(tick_claim) = tick_start.claims.get_mut(usize::from(*slot)) else {
                    return Err(format!(
                        "tick baseline has no slot {slot} for player {player_id}"
                    ));
                };
                *tick_claim = Some(*player_id);
            }
            EconomyLogRecord::Admission {
                ordinal,
                connection: _,
                player_id,
                slot,
                seq: _,
                tick,
                reason: _,
                wire_format,
                raw_envelope,
                before,
                after,
            } => {
                if *ordinal == 0 || wire_format != "postcard" || raw_envelope.is_empty() {
                    return Err(
                        "admission evidence is missing its ordinal or raw wire bytes".into(),
                    );
                }
                if before != after {
                    return Err(format!(
                        "wire-only rejection for player {player_id}, slot {slot}, tick {tick} changed audited state"
                    ));
                }
                validate_clock_continuity(&cursor, before, "wire admission")?;
                if before.economy.tick != *tick {
                    return Err(format!(
                        "wire admission tick {tick} differs from its snapshot"
                    ));
                }
                cursor = after.clone();
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
                validate_tick_state(&cursor, state, *tick, movement_pending)?;
                cursor = state.clone();
                tick_start = state.clone();
                tick_events.clear();
                last_tick = *tick;
                movement_pending = true;
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

fn validate_operation_continuity(
    previous: &AuditState,
    current: &AuditState,
    movement_pending: bool,
) -> Result<(), String> {
    if movement_pending {
        return validate_movement_transition(previous, current);
    }
    validate_clock_continuity(previous, current, "operation")
}

fn same_pos(a: &turts_mod_api::WorldPos, b: &turts_mod_api::WorldPos) -> bool {
    a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits()
}

fn same_optional_pos(
    a: &Option<turts_mod_api::WorldPos>,
    b: &Option<turts_mod_api::WorldPos>,
) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => same_pos(a, b),
        (None, None) => true,
        _ => false,
    }
}

/// Validate exactly one fixed movement phase. Identity, ownership, economy,
/// orders, and events stay exact; positions and targets must follow prior targets,
/// catalog speed, and tick rate.
fn validate_movement_transition(previous: &AuditState, current: &AuditState) -> Result<(), String> {
    let previous_tick = previous.economy.tick;
    if current.economy.tick != previous_tick
        && previous_tick.checked_add(1) != Some(current.economy.tick)
    {
        return Err("movement transition skips or repeats a simulation tick".into());
    }
    let tick_hz = previous
        .tick_hz
        .filter(|hz| *hz > 0)
        .ok_or("movement transition has no valid tick rate")?;
    if current.tick_hz != previous.tick_hz || current.catalog != previous.catalog {
        return Err("movement transition changed clock or catalog".into());
    }

    let mut expected = previous.clone();
    expected.economy.tick = current.economy.tick;
    for expected_unit in &mut expected.raw_units {
        let actual = current
            .raw_units
            .iter()
            .find(|unit| unit.unit_id == expected_unit.unit_id)
            .ok_or_else(|| format!("movement removed raw unit {}", expected_unit.unit_id))?;
        let position = bevy::math::Vec2::new(expected_unit.pos.x, expected_unit.pos.y);
        let (next, next_target) = match expected_unit.move_target.as_ref() {
            Some(target) => {
                let speed = expected
                    .unit_types
                    .iter()
                    .find(|kind| kind.type_id == expected_unit.unit_type)
                    .and_then(|kind| kind.movement_speed);
                match speed {
                    None => (position, None),
                    Some(speed) => {
                        let target_vec = bevy::math::Vec2::new(target.x, target.y);
                        let offset = target_vec - position;
                        let distance = offset.length();
                        // Match Bevy Fixed::from_hz: Duration stores the reciprocal in f64,
                        // then Time<Fixed>::delta_secs exposes that duration as f32.
                        let delta = std::time::Duration::from_secs_f64(1.0 / f64::from(tick_hz))
                            .as_secs_f32();
                        let step = speed * delta;
                        if distance <= step {
                            (target_vec, None)
                        } else {
                            (position + offset * (step / distance), Some(target.clone()))
                        }
                    }
                }
            }
            None => (position, None),
        };
        let next_pos = turts_mod_api::WorldPos {
            x: next.x,
            y: next.y,
        };
        let next_move_target = next_target;
        if !same_pos(&actual.pos, &next_pos)
            || !same_optional_pos(&actual.move_target, &next_move_target)
        {
            return Err(format!(
                "unit {} movement differs from its prior target and speed",
                expected_unit.unit_id
            ));
        }
        expected_unit.pos = actual.pos.clone();
        expected_unit.move_target = actual.move_target.clone();
    }
    for expected_unit in &mut expected.units {
        let actual = current
            .units
            .iter()
            .find(|unit| unit.unit_id == expected_unit.unit_id)
            .ok_or_else(|| format!("movement removed unit {}", expected_unit.unit_id))?;
        let raw = current
            .raw_units
            .iter()
            .find(|unit| unit.unit_id == actual.unit_id)
            .ok_or_else(|| format!("movement has no raw unit {}", actual.unit_id))?;
        if !same_pos(&actual.pos, &raw.pos) {
            return Err(format!(
                "unit {} snapshots disagree after movement",
                actual.unit_id
            ));
        }
        expected_unit.pos = actual.pos.clone();
    }
    for expected_unit in &mut expected.economy.units {
        let actual = current
            .economy
            .units
            .iter()
            .find(|unit| unit.unit_id == expected_unit.unit_id)
            .ok_or_else(|| format!("movement removed world unit {}", expected_unit.unit_id))?;
        let raw = current
            .raw_units
            .iter()
            .find(|unit| unit.unit_id == actual.unit_id)
            .ok_or_else(|| format!("movement has no raw unit {}", actual.unit_id))?;
        if actual.pos.x.to_bits() != raw.pos.x.to_bits()
            || actual.pos.y.to_bits() != raw.pos.y.to_bits()
        {
            return Err(format!(
                "world unit {} snapshots disagree after movement",
                actual.unit_id
            ));
        }
        expected_unit.pos = actual.pos;
    }
    if expected != *current {
        return Err("movement transition changed nonmovement state".into());
    }
    Ok(())
}

/// A completed tick may move units if no automatic operation captured the
/// movement phase. Economy, ownership, order, event, and id state must remain exact.
fn validate_tick_state(
    previous: &AuditState,
    state: &AuditState,
    tick: u32,
    movement_pending: bool,
) -> Result<(), String> {
    let mut expected = previous.clone();
    expected.economy.tick = tick;
    expected.pending_events.clear();
    expected.next_event_ordinal = Some(0);
    if movement_pending {
        validate_movement_transition(&expected, state)?;
    } else if expected != *state {
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

        let mut wrong_slot = decoded;
        let EconomyLogRecord::Claim { slot, .. } = &mut wrong_slot[1] else {
            panic!("claim record")
        };
        *slot = 1;
        assert!(validate_records(&wrong_slot).is_err());
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
                before: state.clone(),
                after: state.clone(),
            })
            .unwrap();
        let decoded = read_jsonl(Cursor::new(writer.into_inner())).unwrap();
        assert!(matches!(decoded[1], EconomyLogRecord::Admission { .. }));

        let mut changed = decoded;
        let EconomyLogRecord::Admission { after, .. } = &mut changed[1] else {
            panic!("admission record")
        };
        after.economy.tick = 1;
        assert!(validate_records(&changed).is_err());
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
