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

struct ActiveOrderSpan {
    command: CommandAudit,
    end_id: u64,
    last_unit_id: Option<u32>,
    next_subject_index: usize,
    subjects_with_operations: BTreeSet<u32>,
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
    let mut active_order_command: Option<ActiveOrderSpan> = None;
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
                if matches!(command.input.kind, turts_sim::CommandKind::Order(_)) {
                    validate_order_policy(command)?;
                    validate_order_result(command)?;
                }
                if commands.insert(command.operation_id, command).is_some() {
                    return Err(format!(
                        "duplicate command operation id {}",
                        command.operation_id
                    ));
                }
            }
            EconomyLogRecord::Operation(operation) => {
                if active_order_command.is_some() && commands.contains_key(&operation.operation_id)
                {
                    return Err(format!(
                        "overlapping order command span starts at operation {}",
                        operation.operation_id
                    ));
                }
                if let Some(active) = &active_order_command
                    && (operation.operation_id > active.end_id
                        || operation.phase != turts_sim::economy::transaction::AuditPhase::Orders)
                {
                    return Err(format!(
                        "order command operation span ended unexpectedly at {}",
                        operation.operation_id
                    ));
                }
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
                if operation.before.catalog != initial.catalog
                    || operation.after.catalog != initial.catalog
                {
                    return Err(format!(
                        "operation {} changed the immutable catalog snapshot",
                        operation.operation_id
                    ));
                }
                let command_start = commands.remove(&operation.operation_id);
                let is_command = command_start.is_some();
                match operation.phase {
                    turts_sim::economy::transaction::AuditPhase::Command => {
                        if !is_command {
                            return Err(format!(
                                "operation {} has Command phase without a command",
                                operation.operation_id
                            ));
                        }
                        validate_clock_continuity(&cursor, &operation.before, "command operation")?;
                    }
                    turts_sim::economy::transaction::AuditPhase::Orders => {
                        validate_operation_phase(
                            &cursor,
                            &operation.before,
                            movement_pending,
                            operation.phase,
                        )?;
                    }
                    turts_sim::economy::transaction::AuditPhase::Other => {
                        if is_command {
                            return Err(format!(
                                "command operation {} has Other phase",
                                operation.operation_id
                            ));
                        }
                        validate_operation_phase(
                            &cursor,
                            &operation.before,
                            movement_pending,
                            operation.phase,
                        )?;
                        movement_pending = false;
                    }
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
                let was_active_order_span = active_order_command.is_some();
                let mut started_order_span = false;
                if let Some(command) = command_start {
                    match operation.phase {
                        turts_sim::economy::transaction::AuditPhase::Command => {
                            if command.operation_end_id.is_some()
                                || command.before != operation.before
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
                            if matches!(command.input.kind, turts_sim::CommandKind::Order(_)) {
                                validate_command_phase_order(command, operation)?;
                            }
                        }
                        turts_sim::economy::transaction::AuditPhase::Orders => {
                            if active_order_command.is_some() {
                                return Err(format!(
                                    "overlapping order command span starts at operation {}",
                                    operation.operation_id
                                ));
                            }
                            let end_id = command.operation_end_id.unwrap_or(command.operation_id);
                            if end_id < command.operation_id
                                || (command.operation_end_id.is_some()
                                    && end_id == command.operation_id)
                                || command.before != operation.before
                            {
                                return Err(format!(
                                    "order command {} has an invalid operation span start",
                                    command.operation_id
                                ));
                            }
                            if end_id == operation.operation_id {
                                let mut span = ActiveOrderSpan {
                                    command: command.clone(),
                                    end_id,
                                    last_unit_id: None,
                                    next_subject_index: 0,
                                    subjects_with_operations: BTreeSet::new(),
                                };
                                validate_order_operation(&mut span, operation)?;
                                validate_order_span_end(&span, operation)?;
                            } else {
                                let mut span = ActiveOrderSpan {
                                    command: command.clone(),
                                    end_id,
                                    last_unit_id: None,
                                    next_subject_index: 0,
                                    subjects_with_operations: BTreeSet::new(),
                                };
                                validate_order_operation(&mut span, operation)?;
                                active_order_command = Some(span);
                                started_order_span = true;
                            }
                        }
                        turts_sim::economy::transaction::AuditPhase::Other => unreachable!(),
                    }
                }
                if !started_order_span && let Some(mut span) = active_order_command.take() {
                    validate_order_operation(&mut span, operation)?;
                    if operation.operation_id == span.end_id {
                        validate_order_span_end(&span, operation)?;
                    } else {
                        active_order_command = Some(span);
                    }
                }
                if operation.phase == turts_sim::economy::transaction::AuditPhase::Orders
                    && !is_command
                    && !was_active_order_span
                    && active_order_command.is_none()
                {
                    validate_automatic_order_operation(operation)?;
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
    if active_order_command.is_some() {
        return Err("order command operation span is incomplete".into());
    }
    Ok(())
}

fn validate_order_result(command: &CommandAudit) -> Result<(), String> {
    let turts_sim::CommandKind::Order(order) = &command.input.kind else {
        return Ok(());
    };
    if order.units.is_empty() {
        return if command.result.units.accepted.is_empty()
            && command.result.units.refused.is_empty()
            && command.result.outcome == Err(turts_sim::RejectReason::EmptyCommand)
        {
            Ok(())
        } else {
            Err("empty order envelope has nonempty results or wrong outcome".into())
        };
    }
    if order.units.len() > turts_net_protocol::MAX_ORDER_UNITS {
        return if command.result.units.accepted.is_empty()
            && command.result.units.refused.is_empty()
            && command.result.outcome == Err(turts_sim::RejectReason::TooManyUnits)
        {
            Ok(())
        } else {
            Err("oversized order envelope has nonempty results or wrong outcome".into())
        };
    }
    let requested = order.units.iter().copied().collect::<BTreeSet<_>>();
    if requested.len() != order.units.len() {
        return if command.result.units.accepted.is_empty()
            && command.result.units.refused.is_empty()
            && command.result.outcome == Err(turts_sim::RejectReason::DuplicateUnit)
        {
            Ok(())
        } else {
            Err("duplicate order envelope has nonempty results or wrong outcome".into())
        };
    }
    let accepted = &command.result.units.accepted;
    let refused = &command.result.units.refused;
    let mut outcomes = BTreeMap::new();
    if !accepted.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err("accepted order subjects are not in unit-id order".into());
    }
    for unit in accepted {
        if outcomes.insert(*unit, None).is_some() {
            return Err(format!("order result duplicates unit {unit}"));
        }
    }
    if !refused.windows(2).all(|pair| pair[0].0 < pair[1].0) {
        return Err("refused order subjects are not in unit-id order".into());
    }
    for (unit, reason) in refused {
        if outcomes.insert(*unit, Some(*reason)).is_some() {
            return Err(format!("order result duplicates unit {unit}"));
        }
    }
    if outcomes.keys().copied().collect::<BTreeSet<_>>() != requested {
        return Err("order results do not partition the unique requested subjects".into());
    }
    let expected_outcome = if accepted.is_empty() {
        Err(refused
            .first()
            .map_or(turts_sim::RejectReason::NotApplicable, |(_, reason)| {
                *reason
            }))
    } else {
        Ok(turts_sim::Applied {
            job_id: command
                .result
                .outcome
                .as_ref()
                .ok()
                .and_then(|applied| applied.job_id),
        })
    };
    if command.result.outcome != expected_outcome {
        return Err("order aggregate outcome disagrees with the per-unit result model".into());
    }
    Ok(())
}

fn expected_order_refusal(
    order: &turts_sim::OrderCommand,
    unit_id: u32,
    before: &AuditState,
    sender_slot: u8,
) -> Result<Option<turts_sim::RejectReason>, String> {
    use turts_sim::RejectReason as R;
    let Some(unit) = before.raw_units.iter().find(|unit| unit.unit_id == unit_id) else {
        return Ok(Some(R::UnknownUnit));
    };
    if unit.owner_slot != sender_slot {
        return Ok(Some(R::NotOwner));
    }
    let content_id = before
        .unit_types
        .iter()
        .find(|ty| ty.type_id == unit.unit_type)
        .map(|ty| ty.content_id.as_str());
    let catalog_order = before
        .catalog
        .as_ref()
        .and_then(|catalog| {
            catalog
                .order_definitions
                .iter()
                .find(|definition| definition.id == order.order)
        })
        .ok_or_else(|| format!("order {} is absent from the immutable catalog", order.order))?;
    let capability = content_id.and_then(|content| {
        catalog_order
            .unit_capabilities
            .iter()
            .find(|entry| entry.unit == content)
    });
    let has_capability = match order.behaviour {
        turts_mod_api::Behaviour::Move => capability.is_some_and(|entry| entry.movement),
        turts_mod_api::Behaviour::Stop | turts_mod_api::Behaviour::Hold => {
            capability.is_some_and(|entry| entry.movement || entry.weapon)
        }
        turts_mod_api::Behaviour::Attack => match &order.target {
            turts_sim::OrderTargetSim::Unit(_) => capability.is_some_and(|entry| entry.weapon),
            turts_sim::OrderTargetSim::Point(_) => capability.is_some_and(|entry| entry.movement),
            _ => false,
        },
        turts_mod_api::Behaviour::Gather => capability.is_some_and(|entry| entry.gatherer),
        turts_mod_api::Behaviour::Produce => {
            capability.is_some_and(|entry| entry.producer || entry.research)
        }
        turts_mod_api::Behaviour::CancelProduction => {
            capability.is_some_and(|entry| entry.producer || entry.research)
        }
        turts_mod_api::Behaviour::Remove => capability.is_some_and(|entry| entry.removable),
        turts_mod_api::Behaviour::SetEnabled(_) => capability.is_some_and(|entry| entry.toggleable),
    };
    if !has_capability {
        return Ok(Some(match order.behaviour {
            turts_mod_api::Behaviour::Gather
            | turts_mod_api::Behaviour::Produce
            | turts_mod_api::Behaviour::CancelProduction => R::MissingCapability,
            turts_mod_api::Behaviour::Remove => R::NotRemovable,
            turts_mod_api::Behaviour::SetEnabled(_) => R::NotToggleable,
            _ => R::NotApplicable,
        }));
    }
    if !catalog_order.units.is_empty()
        && !content_id.is_some_and(|id| catalog_order.units.iter().any(|allowed| allowed == id))
    {
        return Ok(Some(R::NotApplicable));
    }
    let target_matches = match (&order.target_kind, &order.target) {
        (turts_mod_api::TargetKind::Point, turts_sim::OrderTargetSim::Point(_))
        | (turts_mod_api::TargetKind::Unit, turts_sim::OrderTargetSim::Unit(_))
        | (turts_mod_api::TargetKind::PointOrUnit, turts_sim::OrderTargetSim::Point(_))
        | (turts_mod_api::TargetKind::PointOrUnit, turts_sim::OrderTargetSim::Unit(_))
        | (turts_mod_api::TargetKind::Node, turts_sim::OrderTargetSim::Node(_))
        | (turts_mod_api::TargetKind::Job, turts_sim::OrderTargetSim::Job(_))
        | (turts_mod_api::TargetKind::Content, turts_sim::OrderTargetSim::Content(_))
        | (turts_mod_api::TargetKind::None, turts_sim::OrderTargetSim::None) => true,
        (turts_mod_api::TargetKind::Content, turts_sim::OrderTargetSim::Invalid)
            if order.behaviour == turts_mod_api::Behaviour::Produce =>
        {
            true
        }
        _ => false,
    };
    if !target_matches {
        return Ok(Some(R::TargetInvalid));
    }
    match (&order.behaviour, &order.target) {
        (turts_mod_api::Behaviour::Move, turts_sim::OrderTargetSim::Point(point))
            if !point.is_finite() =>
        {
            return Ok(Some(R::TargetNotFinite));
        }
        (turts_mod_api::Behaviour::Move, turts_sim::OrderTargetSim::Unit(target))
        | (turts_mod_api::Behaviour::Attack, turts_sim::OrderTargetSim::Unit(target))
            if !before.raw_units.iter().any(|unit| unit.unit_id == *target) =>
        {
            return Ok(Some(R::TargetGone));
        }
        (turts_mod_api::Behaviour::Gather, turts_sim::OrderTargetSim::Node(node))
            if !before.nodes.iter().any(|entry| entry.id == *node) =>
        {
            return Ok(Some(R::UnknownNode));
        }
        (turts_mod_api::Behaviour::Gather, turts_sim::OrderTargetSim::Node(node)) => {
            let node_state = before
                .nodes
                .iter()
                .find(|entry| entry.id == *node)
                .expect("the preceding guard found the node");
            if node_state.remaining == 0 {
                return Ok(Some(R::NodeDepleted));
            }
            if order.queue == turts_sim::QueueMode::Replace {
                let Some(catalog) = &before.catalog else {
                    return Ok(Some(R::UnknownNode));
                };
                let Some(node_definition) = catalog
                    .nodes
                    .iter()
                    .find(|definition| definition.id == node_state.content_id)
                else {
                    return Ok(Some(R::UnknownNode));
                };
                let Some(gatherer) = content_id
                    .and_then(|content| {
                        catalog
                            .units
                            .iter()
                            .find(|definition| definition.id.to_string() == content)
                    })
                    .and_then(|definition| definition.gatherer.as_ref())
                else {
                    return Ok(Some(R::MissingCapability));
                };
                if !gatherer.items.contains(&node_definition.item) {
                    return Ok(Some(R::IncompatibleItem));
                }
            }
        }
        (turts_mod_api::Behaviour::Produce, turts_sim::OrderTargetSim::Invalid) => {
            return Ok(Some(R::UnknownRecipe));
        }
        (turts_mod_api::Behaviour::Produce, turts_sim::OrderTargetSim::Content(recipe)) => {
            let Some(catalog) = &before.catalog else {
                return Ok(Some(R::UnknownRecipe));
            };
            let unit_definition = content_id.and_then(|content| {
                catalog
                    .units
                    .iter()
                    .find(|unit| unit.id.to_string() == content)
            });
            let Some(unit_definition) = unit_definition else {
                return Ok(Some(R::MissingCapability));
            };
            if let Some((technologies, capacity)) = &unit_definition.research {
                let Some(technology) = catalog
                    .technologies
                    .iter()
                    .find(|entry| entry.id == *recipe)
                else {
                    return Ok(Some(R::UnknownRecipe));
                };
                if !technologies.contains(recipe) {
                    return Ok(Some(R::RecipeNotEnabled));
                }
                if technology.prerequisites.iter().any(|prerequisite| {
                    !before
                        .research
                        .iter()
                        .any(|entry| entry.slot == sender_slot && entry.technology == *prerequisite)
                }) {
                    return Ok(Some(R::PrerequisitesMissing));
                }
                if before
                    .research
                    .iter()
                    .any(|entry| entry.slot == sender_slot && entry.technology == *recipe)
                {
                    return Ok(Some(R::AlreadyResearched));
                }
                let stations = catalog
                    .units
                    .iter()
                    .filter(|unit| {
                        unit.research
                            .as_ref()
                            .is_some_and(|(enabled, _)| enabled.contains(recipe))
                    })
                    .filter_map(|unit| {
                        before
                            .raw_units
                            .iter()
                            .find(|raw| {
                                raw.owner_slot == sender_slot
                                    && before.unit_types.iter().any(|ty| {
                                        ty.type_id == raw.unit_type
                                            && ty.content_id == unit.id.to_string()
                                    })
                            })
                            .map(|raw| raw.unit_id)
                    })
                    .collect::<BTreeSet<_>>();
                if before.raw_queues.iter().any(|queue| {
                    stations.contains(&queue.producer)
                        && queue.jobs.iter().any(|job| job.recipe == *recipe)
                }) {
                    return Ok(Some(R::ResearchInProgress));
                }
                let queue_len = before
                    .raw_queues
                    .iter()
                    .find(|queue| queue.producer == unit_id)
                    .map_or(0, |queue| queue.jobs.len());
                if queue_len >= usize::from(*capacity) {
                    return Ok(Some(R::QueueFull));
                }
                return Ok(expected_job_cost_refusal(
                    before,
                    unit_id,
                    sender_slot,
                    &technology.costs,
                ));
            }
            let Some((recipes, capacity, _)) = unit_definition.producer.as_ref() else {
                return Ok(Some(R::MissingCapability));
            };
            let Some(recipe_definition) = catalog.recipes.iter().find(|entry| entry.id == *recipe)
            else {
                return Ok(Some(R::UnknownRecipe));
            };
            if !recipes.contains(recipe) {
                return Ok(Some(R::RecipeNotEnabled));
            }
            let queue_len = before
                .raw_queues
                .iter()
                .find(|queue| queue.producer == unit_id)
                .map_or(0, |queue| queue.jobs.len());
            if queue_len >= usize::from(*capacity) {
                return Ok(Some(R::QueueFull));
            }
            return Ok(expected_job_cost_refusal(
                before,
                unit_id,
                sender_slot,
                &recipe_definition.costs,
            ));
        }
        (turts_mod_api::Behaviour::CancelProduction, turts_sim::OrderTargetSim::Job(job))
            if !before.raw_queues.iter().any(|queue| {
                queue.producer == unit_id && queue.jobs.iter().any(|entry| entry.id == job.0)
            }) =>
        {
            return Ok(Some(R::UnknownJob));
        }
        _ => {}
    }
    if matches!(
        order.behaviour,
        turts_mod_api::Behaviour::Produce
            | turts_mod_api::Behaviour::CancelProduction
            | turts_mod_api::Behaviour::Remove
            | turts_mod_api::Behaviour::SetEnabled(_)
    ) {
        return Ok(None);
    }
    if order.queue == turts_sim::QueueMode::Append {
        let unit_orders = unit_orders_for(before, unit_id);
        if unit_orders.current.as_ref().is_some_and(|current| {
            matches!(
                current.behaviour,
                turts_mod_api::Behaviour::Hold | turts_mod_api::Behaviour::Gather
            )
        }) || unit_orders.queue.iter().any(|queued| {
            matches!(
                queued.behaviour,
                turts_mod_api::Behaviour::Hold | turts_mod_api::Behaviour::Gather
            )
        }) || before
            .raw_orders
            .iter()
            .any(|entry| entry.unit == unit_id && entry.node.is_some())
        {
            return Ok(Some(R::NotQueueable));
        }
        if unit_orders.queue.len() >= usize::from(catalog_order.queue_depth) {
            return Ok(Some(R::QueueFull));
        }
    }
    Ok(None)
}

fn expected_job_cost_refusal(
    before: &AuditState,
    producer: u32,
    owner_slot: u8,
    costs: &[turts_sim::CostView],
) -> Option<turts_sim::RejectReason> {
    use turts_mod_api::CostSource;
    use turts_sim::RejectReason as R;
    for cost in costs {
        match cost.source {
            CostSource::SlotBalance => {
                let Some(balance) = before.economy.balances.iter().find(|balance| {
                    balance.slot == owner_slot && balance.resource == cost.resource
                }) else {
                    return Some(R::UnknownBalance);
                };
                if balance.on_hand.saturating_sub(balance.reserved) < cost.amount {
                    return Some(R::InsufficientBalance);
                }
            }
            CostSource::ProducerInventory => {
                let Some(inventory) = before
                    .economy
                    .inventories
                    .iter()
                    .find(|inventory| inventory.unit == producer)
                else {
                    return Some(R::UnknownInventory);
                };
                let Some(item) = inventory
                    .items
                    .iter()
                    .find(|item| item.resource == cost.resource)
                else {
                    return Some(R::InsufficientItems);
                };
                if item.on_hand.saturating_sub(item.reserved) < cost.amount {
                    return Some(R::InsufficientItems);
                }
            }
        }
    }
    None
}

fn validate_order_policy(command: &CommandAudit) -> Result<(), String> {
    let turts_sim::CommandKind::Order(order) = &command.input.kind else {
        return Ok(());
    };
    let catalog = command.before.catalog.as_ref().ok_or_else(|| {
        format!(
            "order command {} has no immutable content catalog",
            command.operation_id
        )
    })?;
    let definitions: Vec<_> = catalog
        .order_definitions
        .iter()
        .filter(|definition| definition.id == order.order)
        .collect();
    if definitions.len() != 1 {
        return Err(format!(
            "order command {} has a missing or duplicate catalog definition",
            command.operation_id
        ));
    }
    let definition = definitions[0];
    let canonical_units = canonical_order_units(&definition.units);
    let command_units = canonical_order_units(&order.allowed_units);
    let definition_capabilities = canonical_capabilities(&definition.unit_capabilities);
    if canonical_units.windows(2).any(|pair| pair[0] == pair[1])
        || command_units.windows(2).any(|pair| pair[0] == pair[1])
        || definition_capabilities
            .windows(2)
            .any(|pair| pair[0].unit == pair[1].unit)
    {
        return Err(format!(
            "order command {} has duplicate catalog filter or capability entries",
            command.operation_id
        ));
    }
    if order.behaviour != definition.behaviour
        || order.target_kind != definition.target
        || command_units != canonical_units
        || order.queue_depth != definition.queue_depth
    {
        return Err(format!(
            "order command {} differs from its immutable catalog definition",
            command.operation_id
        ));
    }
    let policy = order.policy.as_ref().ok_or_else(|| {
        format!(
            "order command {} omits its captured catalog definition",
            command.operation_id
        )
    })?;
    if policy.id != definition.id
        || policy.behaviour != definition.behaviour
        || policy.target != definition.target
        || canonical_order_units(&policy.units) != canonical_units
        || policy.queue_depth != definition.queue_depth
        || canonical_capabilities(&policy.unit_capabilities) != definition_capabilities
    {
        return Err(format!(
            "order command {} differs from its immutable catalog definition",
            command.operation_id
        ));
    }
    let target_allowed = match order.behaviour {
        turts_mod_api::Behaviour::Move => matches!(
            policy.target,
            turts_mod_api::TargetKind::Point
                | turts_mod_api::TargetKind::Unit
                | turts_mod_api::TargetKind::PointOrUnit
        ),
        turts_mod_api::Behaviour::Attack => matches!(
            policy.target,
            turts_mod_api::TargetKind::Point
                | turts_mod_api::TargetKind::Unit
                | turts_mod_api::TargetKind::PointOrUnit
        ),
        turts_mod_api::Behaviour::Stop
        | turts_mod_api::Behaviour::Hold
        | turts_mod_api::Behaviour::Remove
        | turts_mod_api::Behaviour::SetEnabled(_) => {
            policy.target == turts_mod_api::TargetKind::None
        }
        turts_mod_api::Behaviour::Gather => policy.target == turts_mod_api::TargetKind::Node,
        turts_mod_api::Behaviour::Produce => policy.target == turts_mod_api::TargetKind::Content,
        turts_mod_api::Behaviour::CancelProduction => {
            policy.target == turts_mod_api::TargetKind::Job
        }
    };
    if !target_allowed {
        return Err(format!(
            "order command {} has an invalid behavior/target policy pair",
            command.operation_id
        ));
    }
    Ok(())
}

fn canonical_order_units(units: &[String]) -> Vec<String> {
    let mut canonical = units.to_vec();
    canonical.sort();
    canonical
}

fn canonical_capabilities(
    capabilities: &[turts_sim::OrderUnitCapabilitiesAudit],
) -> Vec<turts_sim::OrderUnitCapabilitiesAudit> {
    let mut canonical = capabilities.to_vec();
    canonical.sort_by(|left, right| left.unit.cmp(&right.unit));
    canonical
}

fn validate_order_operation(
    span: &mut ActiveOrderSpan,
    operation: &OperationAudit,
) -> Result<(), String> {
    let turts_sim::CommandKind::Order(order) = &span.command.input.kind else {
        return Err("order operation span is attached to a non-order command".into());
    };
    let unit_id = operation.unit_id.ok_or_else(|| {
        format!(
            "order operation {} omits its unit subject",
            operation.operation_id
        )
    })?;
    if !order.units.contains(&unit_id) {
        return Err(format!(
            "order operation {} names unit {unit_id} outside its envelope",
            operation.operation_id
        ));
    }
    let mut requested = order.units.clone();
    requested.sort_unstable();
    while span.next_subject_index < requested.len() && requested[span.next_subject_index] < unit_id
    {
        let prior_subject = requested[span.next_subject_index];
        if span
            .command
            .result
            .units
            .accepted
            .binary_search(&prior_subject)
            .is_ok()
        {
            return Err(format!(
                "accepted order for unit {prior_subject} has no per-unit operation"
            ));
        }
        let reason = span
            .command
            .result
            .units
            .refused
            .iter()
            .find(|(subject, _)| *subject == prior_subject)
            .map(|(_, reason)| *reason)
            .ok_or_else(|| format!("order subject {prior_subject} has no result"))?;
        let expected = expected_order_refusal(
            order,
            prior_subject,
            &operation.before,
            span.command.input.slot.0,
        )?;
        if expected != Some(reason) {
            return Err(format!(
                "refusal for unit {prior_subject} differs from independent validation: expected {expected:?}, got {reason:?}"
            ));
        }
        span.next_subject_index += 1;
    }
    if span.last_unit_id.is_some_and(|last| last >= unit_id) {
        return Err(format!(
            "order operation subjects are not in unit-id order at {unit_id}"
        ));
    }
    let accepted_count = span
        .command
        .result
        .units
        .accepted
        .iter()
        .filter(|id| **id == unit_id)
        .count();
    let refusal = span
        .command
        .result
        .units
        .refused
        .iter()
        .find(|(id, _)| *id == unit_id)
        .map(|(_, reason)| *reason);
    let refused_count = usize::from(refusal.is_some());
    if accepted_count + refused_count != 1 {
        return Err(format!(
            "order operation {unit_id} is absent or duplicated in per-unit results"
        ));
    }
    span.last_unit_id = Some(unit_id);
    if requested.get(span.next_subject_index) == Some(&unit_id) {
        span.next_subject_index += 1;
    }
    if !span.subjects_with_operations.insert(unit_id) {
        return Err(format!(
            "unit {unit_id} has multiple order operations in one envelope"
        ));
    }

    let expected_refusal =
        expected_order_refusal(order, unit_id, &operation.before, span.command.input.slot.0)?;
    if accepted_count == 1 && expected_refusal.is_some() {
        return Err(format!(
            "accepted order for unit {unit_id} should be refused as {expected_refusal:?}"
        ));
    }
    if accepted_count == 0 && refusal != expected_refusal {
        return Err(format!(
            "refusal for unit {unit_id} differs from independent validation: expected {expected_refusal:?}, got {refusal:?}"
        ));
    }

    if accepted_count == 0 {
        if operation.before != operation.after {
            return Err(format!(
                "refused unit {unit_id} changed state inside its order operation"
            ));
        }
        return Ok(());
    }

    let before_orders = unit_orders_for(&operation.before, unit_id);
    let after_orders = unit_orders_for(&operation.after, unit_id);
    let before_target = raw_move_target(&operation.before, unit_id);
    let after_target = raw_move_target(&operation.after, unit_id);
    let mut expected_orders = before_orders.clone();
    let mut expected_target = before_target.clone();
    match order.behaviour {
        turts_mod_api::Behaviour::Move => {
            let point = match &order.target {
                turts_sim::OrderTargetSim::Point(point) => *point,
                turts_sim::OrderTargetSim::Unit(target_id) => {
                    let Some(target) = operation
                        .before
                        .raw_units
                        .iter()
                        .find(|unit| unit.unit_id == *target_id)
                    else {
                        return Err(format!(
                            "accepted Move for unit {unit_id} has a missing unit target"
                        ));
                    };
                    bevy::math::Vec2::new(target.pos.x, target.pos.y)
                }
                _ => {
                    return Err(format!(
                        "accepted Move for unit {unit_id} has an invalid target"
                    ));
                }
            };
            let plain_cleanup = order.queue == turts_sim::QueueMode::Replace
                && order.group == 0
                && order.units.len() == 1
                && (before_orders.current.is_some()
                    || !before_orders.queue.is_empty()
                    || before_orders.last_failure.is_some()
                    || before_orders != Default::default());
            if plain_cleanup {
                expected_orders = Default::default();
                expected_target = Some(clamped_audit_point(&operation.before, point));
            } else if order.queue == turts_sim::QueueMode::Replace {
                expected_orders = turts_sim::UnitOrders {
                    current: Some(order_record(order)),
                    ..Default::default()
                };
                expected_target = Some(clamped_audit_point(&operation.before, point));
            } else if before_orders.current.is_none() && before_target.is_none() {
                expected_orders.current = Some(order_record(order));
                expected_target = Some(clamped_audit_point(&operation.before, point));
            } else {
                expected_orders.queue.push_back(order_record(order));
            }
        }
        turts_mod_api::Behaviour::Stop => {
            expected_orders = Default::default();
            expected_target = None;
        }
        turts_mod_api::Behaviour::Hold => {
            if order.queue == turts_sim::QueueMode::Replace {
                expected_orders = turts_sim::UnitOrders {
                    current: Some(order_record(order)),
                    ..Default::default()
                };
                expected_target = None;
            } else {
                expected_orders.queue.push_back(order_record(order));
            }
        }
        turts_mod_api::Behaviour::Gather => {
            if order.queue == turts_sim::QueueMode::Replace {
                expected_orders = turts_sim::UnitOrders {
                    current: Some(order_record(order)),
                    ..Default::default()
                };
                expected_target = None;
            } else if before_orders.current.is_none() && before_target.is_none() {
                expected_orders.current = Some(order_record(order));
                expected_target = None;
            } else {
                expected_orders.queue.push_back(order_record(order));
            }
        }
        turts_mod_api::Behaviour::Produce
        | turts_mod_api::Behaviour::CancelProduction
        | turts_mod_api::Behaviour::Remove
        | turts_mod_api::Behaviour::SetEnabled(_)
        | turts_mod_api::Behaviour::Attack => {}
    }
    if after_orders != expected_orders {
        return Err(format!(
            "accepted order for unit {unit_id} produced unexpected UnitOrders"
        ));
    }
    if order.behaviour != turts_mod_api::Behaviour::Remove
        && let (Some(before), Some(after)) = (
            operation
                .before
                .raw_units
                .iter()
                .find(|unit| unit.unit_id == unit_id),
            operation
                .after
                .raw_units
                .iter()
                .find(|unit| unit.unit_id == unit_id),
        )
        && before.pos != after.pos
    {
        return Err(format!(
            "Orders operation for unit {unit_id} moved its actor before Movement"
        ));
    }
    if order.behaviour != turts_mod_api::Behaviour::Remove
        && operation
            .before
            .units
            .iter()
            .find(|unit| unit.unit_id == unit_id)
            != operation
                .after
                .units
                .iter()
                .find(|unit| unit.unit_id == unit_id)
    {
        return Err(format!(
            "Orders operation for unit {unit_id} changed its public position/state view"
        ));
    }
    if order.behaviour == turts_mod_api::Behaviour::Remove {
        if operation
            .after
            .raw_units
            .iter()
            .any(|unit| unit.unit_id == unit_id)
        {
            return Err(format!(
                "accepted Remove for unit {unit_id} retained the unit"
            ));
        }
    } else if after_target != expected_target {
        return Err(format!(
            "accepted order for unit {unit_id} produced an unexpected MoveTarget"
        ));
    }
    for old in &operation.before.unit_orders {
        if old.unit_id != unit_id && unit_orders_for(&operation.after, old.unit_id) != old.orders {
            return Err(format!(
                "order operation for unit {unit_id} changed unit {} orders",
                old.unit_id
            ));
        }
    }
    for old in &operation.before.raw_units {
        if old.unit_id != unit_id
            && operation
                .after
                .raw_units
                .iter()
                .find(|new| new.unit_id == old.unit_id)
                != Some(old)
        {
            return Err(format!(
                "order operation for unit {unit_id} changed unit {} movement",
                old.unit_id
            ));
        }
    }
    validate_legacy_gather_order_state(order, unit_id, &operation.before, &operation.after)?;
    validate_set_enabled_state(
        order,
        unit_id,
        &operation.before,
        &operation.after,
        &operation.events,
    )?;
    validate_immediate_economy_order(
        order,
        unit_id,
        &operation.before,
        &operation.after,
        &operation.events,
    )?;
    Ok(())
}

fn validate_automatic_order_operation(operation: &OperationAudit) -> Result<(), String> {
    let unit_id = operation.unit_id.ok_or_else(|| {
        format!(
            "automatic Orders operation {} omits its unit subject",
            operation.operation_id
        )
    })?;
    let actor = operation
        .before
        .raw_units
        .iter()
        .find(|unit| unit.unit_id == unit_id)
        .ok_or_else(|| format!("automatic Orders operation names absent unit {unit_id}"))?;
    let orders_before = unit_orders_for(&operation.before, unit_id);
    let mut orders_after = orders_before.clone();
    let tick = operation.before.economy.tick;
    let mut expected = operation.before.clone();
    let current = orders_before.current.clone();

    match current {
        Some(order)
            if order.behaviour == turts_mod_api::Behaviour::Move
                && let turts_sim::OrderTargetSim::Unit(target_id) = order.target =>
        {
            let target = operation
                .before
                .raw_units
                .iter()
                .find(|unit| unit.unit_id == target_id);
            match target {
                None => {
                    orders_after.current = None;
                    orders_after.last_failure = Some(turts_sim::OrderFailure {
                        order: order.order,
                        reason: turts_sim::RejectReason::TargetGone,
                        tick,
                    });
                    set_audit_move_target(&mut expected, unit_id, None)?;
                    start_next_audited_order(&mut expected, unit_id, &mut orders_after)?;
                }
                Some(target) if actor.pos == target.pos => {
                    orders_after.current = None;
                    set_audit_move_target(&mut expected, unit_id, None)?;
                    start_next_audited_order(&mut expected, unit_id, &mut orders_after)?;
                }
                Some(target)
                    if raw_move_target(&operation.before, unit_id) != Some(target.pos.clone()) =>
                {
                    set_audit_move_target(&mut expected, unit_id, Some(target.pos.clone()))?;
                }
                Some(_) => {
                    return Err(format!(
                        "automatic Orders operation for unit {unit_id} records an unchanged follow target"
                    ));
                }
            }
        }
        Some(order)
            if order.behaviour == turts_mod_api::Behaviour::Move
                && matches!(order.target, turts_sim::OrderTargetSim::Point(_))
                && actor.move_target.is_none() =>
        {
            orders_after.current = None;
            start_next_audited_order(&mut expected, unit_id, &mut orders_after)?;
        }
        Some(order)
            if order.behaviour == turts_mod_api::Behaviour::Gather
                && let turts_sim::OrderTargetSim::Node(node) = order.target
                && operation
                    .before
                    .nodes
                    .iter()
                    .any(|entry| entry.id == node && entry.remaining == 0) =>
        {
            orders_after.current = None;
            orders_after.last_failure = Some(turts_sim::OrderFailure {
                order: order.order,
                reason: turts_sim::RejectReason::NodeDepleted,
                tick,
            });
            clear_audited_gather_cycle(&mut expected, unit_id)?;
            start_next_audited_order(&mut expected, unit_id, &mut orders_after)?;
        }
        None if !orders_before.queue.is_empty() && actor.move_target.is_none() => {
            start_next_audited_order(&mut expected, unit_id, &mut orders_after)?;
        }
        _ => {
            return Err(format!(
                "automatic Orders operation for unit {unit_id} has no valid completion, failure or queued-start transition"
            ));
        }
    }

    store_audited_unit_orders(&mut expected, unit_id, orders_after);
    if !operation.events.is_empty() {
        return Err(format!(
            "automatic Orders operation for unit {unit_id} emitted an economy event"
        ));
    }
    if expected != operation.after {
        return Err(format!(
            "automatic Orders operation for unit {unit_id} differs from its independently modeled transition"
        ));
    }
    Ok(())
}

fn start_next_audited_order(
    state: &mut AuditState,
    unit_id: u32,
    orders: &mut turts_sim::UnitOrders,
) -> Result<(), String> {
    loop {
        let Some(next) = orders.current.take().or_else(|| orders.queue.pop_front()) else {
            return Ok(());
        };
        orders.current = Some(next.clone());
        let actor = state
            .raw_units
            .iter()
            .find(|unit| unit.unit_id == unit_id)
            .ok_or_else(|| format!("queued order starts on absent unit {unit_id}"))?;
        match (next.behaviour, &next.target) {
            (turts_mod_api::Behaviour::Move, turts_sim::OrderTargetSim::Point(point)) => {
                set_audit_move_target(state, unit_id, Some(clamped_audit_point(state, *point)))?;
                return Ok(());
            }
            (turts_mod_api::Behaviour::Move, turts_sim::OrderTargetSim::Unit(target_id)) => {
                if let Some(target) = state
                    .raw_units
                    .iter()
                    .find(|unit| unit.unit_id == *target_id)
                {
                    let point = clamped_audit_point(
                        state,
                        bevy::math::Vec2::new(target.pos.x, target.pos.y),
                    );
                    set_audit_move_target(state, unit_id, Some(point))?;
                    return Ok(());
                }
                orders.current = None;
                orders.last_failure = Some(turts_sim::OrderFailure {
                    order: next.order,
                    reason: turts_sim::RejectReason::TargetGone,
                    tick: state.economy.tick,
                });
            }
            (turts_mod_api::Behaviour::Hold, _) => {
                set_audit_move_target(state, unit_id, None)?;
                return Ok(());
            }
            (turts_mod_api::Behaviour::Gather, turts_sim::OrderTargetSim::Node(node)) => {
                let failure = audited_gather_start_failure(state, actor, *node);
                if let Some(reason) = failure {
                    orders.current = None;
                    orders.last_failure = Some(turts_sim::OrderFailure {
                        order: next.order,
                        reason,
                        tick: state.economy.tick,
                    });
                    continue;
                }
                let node_view = state.nodes.iter().find(|entry| entry.id == *node).unwrap();
                let item = state
                    .catalog
                    .as_ref()
                    .and_then(|catalog| {
                        catalog
                            .nodes
                            .iter()
                            .find(|definition| definition.id == node_view.content_id)
                    })
                    .map(|definition| definition.item.clone())
                    .ok_or_else(|| format!("queued Gather node {node:?} has no catalog item"))?;
                let inventory = state
                    .economy
                    .inventories
                    .iter()
                    .find(|inventory| inventory.unit == unit_id)
                    .ok_or_else(|| format!("queued Gather unit {unit_id} has no inventory"))?;
                let used = inventory
                    .items
                    .iter()
                    .map(|entry| u128::from(entry.on_hand))
                    .sum::<u128>();
                let has_other_item = inventory
                    .items
                    .iter()
                    .any(|entry| entry.on_hand > entry.reserved && entry.resource != item);
                let must_deliver = used >= u128::from(inventory.capacity) || has_other_item;
                set_audited_gather_cycle(state, unit_id, *node, must_deliver)?;
                set_audit_move_target(state, unit_id, None)?;
                return Ok(());
            }
            _ => {
                orders.current = None;
                orders.last_failure = Some(turts_sim::OrderFailure {
                    order: next.order,
                    reason: turts_sim::RejectReason::NotApplicable,
                    tick: state.economy.tick,
                });
            }
        }
    }
}

fn audited_gather_start_failure(
    state: &AuditState,
    actor: &turts_sim::economy::views::RawUnitAudit,
    node: turts_sim::NodeId,
) -> Option<turts_sim::RejectReason> {
    use turts_sim::RejectReason as R;
    if !state
        .unit_types
        .iter()
        .find(|unit_type| unit_type.type_id == actor.unit_type)
        .is_some_and(|unit_type| unit_type.movement_speed.is_some())
    {
        return Some(R::NotMobile);
    }
    let Some(node_view) = state.nodes.iter().find(|entry| entry.id == node) else {
        return Some(R::TargetGone);
    };
    if node_view.remaining == 0 {
        return Some(R::NodeDepleted);
    }
    let Some(catalog) = &state.catalog else {
        return Some(R::MissingCapability);
    };
    let unit_content = state
        .unit_types
        .iter()
        .find(|entry| entry.type_id == actor.unit_type)
        .map(|entry| entry.content_id.as_str());
    let gatherer = unit_content.and_then(|content| {
        catalog
            .units
            .iter()
            .find(|unit| unit.id.to_string() == content)
    });
    let Some(gatherer) = gatherer.and_then(|unit| unit.gatherer.as_ref()) else {
        return Some(R::MissingCapability);
    };
    let Some(node_def) = catalog
        .nodes
        .iter()
        .find(|definition| definition.id == node_view.content_id)
    else {
        return Some(R::TargetGone);
    };
    if !gatherer.items.contains(&node_def.item) {
        return Some(R::IncompatibleItem);
    }
    if !state
        .raw_orders
        .iter()
        .any(|entry| entry.unit == actor.unit_id)
        || !state
            .economy
            .inventories
            .iter()
            .any(|entry| entry.unit == actor.unit_id)
    {
        return Some(R::MissingCapability);
    }
    None
}

fn set_audited_gather_cycle(
    state: &mut AuditState,
    unit_id: u32,
    node: turts_sim::NodeId,
    must_deliver: bool,
) -> Result<(), String> {
    let raw = state
        .raw_orders
        .iter_mut()
        .find(|entry| entry.unit == unit_id)
        .ok_or_else(|| format!("queued Gather unit {unit_id} has no raw EconomyOrders row"))?;
    raw.node = Some(node);
    raw.deposit = Some(node);
    raw.receiver = None;
    raw.stage = if must_deliver {
        turts_sim::economy::gather::GatherStage::MovingToReceiver
    } else {
        turts_sim::economy::gather::GatherStage::MovingToNode
    };
    raw.rate_carry = 0;
    let public = state
        .economy
        .orders
        .iter_mut()
        .find(|entry| entry.unit == unit_id)
        .ok_or_else(|| format!("queued Gather unit {unit_id} has no public EconomyOrders row"))?;
    public.node = Some(node);
    public.receiver = None;
    public.stage = raw.stage;
    Ok(())
}

fn clear_audited_gather_cycle(state: &mut AuditState, unit_id: u32) -> Result<(), String> {
    if let Some(raw) = state
        .raw_orders
        .iter_mut()
        .find(|entry| entry.unit == unit_id)
    {
        raw.node = None;
        raw.deposit = None;
        raw.receiver = None;
        raw.stage = turts_sim::economy::gather::GatherStage::Idle;
        raw.rate_carry = 0;
    }
    if let Some(public) = state
        .economy
        .orders
        .iter_mut()
        .find(|entry| entry.unit == unit_id)
    {
        public.node = None;
        public.receiver = None;
        public.stage = turts_sim::economy::gather::GatherStage::Idle;
    }
    set_audit_move_target(state, unit_id, None)
}

fn set_audit_move_target(
    state: &mut AuditState,
    unit_id: u32,
    target: Option<turts_mod_api::WorldPos>,
) -> Result<(), String> {
    let unit = state
        .raw_units
        .iter_mut()
        .find(|unit| unit.unit_id == unit_id)
        .ok_or_else(|| format!("order transition names absent unit {unit_id}"))?;
    unit.move_target = target;
    Ok(())
}

fn store_audited_unit_orders(state: &mut AuditState, unit_id: u32, orders: turts_sim::UnitOrders) {
    state.unit_orders.retain(|entry| entry.unit_id != unit_id);
    if orders != Default::default() {
        state
            .unit_orders
            .push(turts_sim::economy::views::RawUnitOrdersAudit { unit_id, orders });
        state.unit_orders.sort_by_key(|entry| entry.unit_id);
    }
}

fn validate_legacy_gather_order_state(
    order: &turts_sim::OrderCommand,
    unit_id: u32,
    before: &AuditState,
    after: &AuditState,
) -> Result<(), String> {
    use turts_mod_api::Behaviour;
    let should_clear = matches!(order.behaviour, Behaviour::Move | Behaviour::Stop);
    let mut expected_raw = before
        .raw_orders
        .iter()
        .find(|row| row.unit == unit_id)
        .cloned();
    let mut expected_view = before
        .economy
        .orders
        .iter()
        .find(|row| row.unit == unit_id)
        .cloned();
    if should_clear {
        if let Some(row) = &mut expected_raw {
            row.node = None;
            row.deposit = None;
            row.receiver = None;
            row.stage = turts_sim::economy::gather::GatherStage::Idle;
            row.rate_carry = 0;
        }
        if let Some(row) = &mut expected_view {
            row.node = None;
            row.receiver = None;
            row.stage = turts_sim::economy::gather::GatherStage::Idle;
        }
    }
    if order.behaviour == Behaviour::Gather {
        let turts_sim::OrderTargetSim::Node(node) = &order.target else {
            if after.raw_orders.iter().find(|row| row.unit == unit_id) != expected_raw.as_ref() {
                return Err(format!(
                    "Gather for unit {unit_id} changed M2 order state with a wrong target"
                ));
            }
            return Ok(());
        };
        let after_orders = unit_orders_for(after, unit_id);
        let current_is_gather = after_orders.current.as_ref().is_some_and(|current| {
            current.behaviour == Behaviour::Gather
                && current.target == turts_sim::OrderTargetSim::Node(*node)
        });
        if current_is_gather {
            let depleted = before
                .nodes
                .iter()
                .any(|entry| entry.id == *node && entry.remaining == 0);
            if !depleted {
                if let Some(row) = &mut expected_raw {
                    row.node = Some(*node);
                    row.deposit = Some(*node);
                    row.receiver = None;
                    row.stage = turts_sim::economy::gather::GatherStage::MovingToNode;
                    row.rate_carry = 0;
                } else {
                    return Err(format!(
                        "Gather for unit {unit_id} has no M2 EconomyOrders row"
                    ));
                }
                if let Some(row) = &mut expected_view {
                    row.node = Some(*node);
                    row.receiver = None;
                    row.stage = turts_sim::economy::gather::GatherStage::MovingToNode;
                }
            } else if let Some(row) = &mut expected_raw {
                row.node = None;
                row.deposit = None;
                row.receiver = None;
                row.stage = turts_sim::economy::gather::GatherStage::Idle;
                row.rate_carry = 0;
                if let Some(view) = &mut expected_view {
                    view.node = None;
                    view.receiver = None;
                    view.stage = turts_sim::economy::gather::GatherStage::Idle;
                }
            }
        }
    }
    if after.raw_orders.iter().find(|row| row.unit == unit_id) != expected_raw.as_ref()
        || after.economy.orders.iter().find(|row| row.unit == unit_id) != expected_view.as_ref()
    {
        let reason = if order.behaviour == Behaviour::Gather {
            "did not assign the M2 gather cycle"
        } else {
            "did not clear the M2 gather cycle"
        };
        return Err(format!("order for unit {unit_id} {reason}"));
    }
    Ok(())
}

fn validate_set_enabled_state(
    order: &turts_sim::OrderCommand,
    unit_id: u32,
    before: &AuditState,
    after: &AuditState,
    operation_events: &[EconomyEvent],
) -> Result<(), String> {
    let turts_mod_api::Behaviour::SetEnabled(enabled) = order.behaviour else {
        return Ok(());
    };
    let was_enabled = before.disabled.binary_search(&unit_id).is_err();
    let changed = was_enabled != enabled;
    let mut expected = before.clone();
    if enabled {
        expected.disabled.retain(|unit| *unit != unit_id);
    } else if !expected.disabled.contains(&unit_id) {
        expected.disabled.push(unit_id);
        expected.disabled.sort_unstable();
    }
    for row in expected
        .raw_extractors
        .iter_mut()
        .filter(|row| row.unit == unit_id)
    {
        row.enabled = enabled;
        if !enabled {
            row.rate_carry = 0;
        }
    }
    for row in expected
        .raw_inserters
        .iter_mut()
        .filter(|row| row.unit == unit_id)
    {
        row.enabled = enabled;
    }
    for row in expected
        .raw_weapons
        .iter_mut()
        .filter(|row| row.unit == unit_id)
    {
        row.enabled = enabled;
    }
    expected.economy.m3.disabled.clone_from(&expected.disabled);
    expected
        .economy
        .m3
        .raw_extractors
        .clone_from(&expected.raw_extractors);
    expected
        .economy
        .m3
        .raw_inserters
        .clone_from(&expected.raw_inserters);
    expected
        .economy
        .m3
        .raw_weapons
        .clone_from(&expected.raw_weapons);
    let expected_event = if changed {
        let ordinal = before
            .next_event_ordinal
            .ok_or("SetEnabled has no event ordinal")?;
        expected.next_event_ordinal = Some(
            ordinal
                .checked_add(1)
                .ok_or("SetEnabled event ordinal overflow")?,
        );
        let event = EconomyEvent {
            tick: before.economy.tick,
            ordinal,
            owner_slot: before
                .raw_units
                .iter()
                .find(|unit| unit.unit_id == unit_id)
                .map_or(0, |unit| unit.owner_slot),
            kind: turts_sim::EconomyEventKind::EnabledChanged {
                unit: unit_id,
                enabled,
            },
        };
        expected.pending_events.push(event.clone());
        Some(event)
    } else {
        None
    };
    if operation_events != expected_event.iter().cloned().collect::<Vec<_>>() {
        return Err(format!(
            "SetEnabled for unit {unit_id} has the wrong state-change event"
        ));
    }
    if expected != *after {
        return Err(format!(
            "SetEnabled for unit {unit_id} changed state outside enabled policy"
        ));
    }
    Ok(())
}

fn validate_immediate_economy_order(
    order: &turts_sim::OrderCommand,
    unit_id: u32,
    before: &AuditState,
    after: &AuditState,
    operation_events: &[EconomyEvent],
) -> Result<(), String> {
    match (&order.behaviour, &order.target) {
        (turts_mod_api::Behaviour::Produce, turts_sim::OrderTargetSim::Content(recipe)) => {
            validate_produce_order(unit_id, recipe, before, after, operation_events)
        }
        (turts_mod_api::Behaviour::CancelProduction, turts_sim::OrderTargetSim::Job(job)) => {
            validate_cancel_order(unit_id, *job, order, before, after, operation_events)
        }
        _ => Ok(()),
    }
}

fn validate_produce_order(
    producer: u32,
    recipe_id: &turts_mod_api::ContentId,
    before: &AuditState,
    after: &AuditState,
    events: &[EconomyEvent],
) -> Result<(), String> {
    before
        .catalog
        .as_ref()
        .ok_or("Produce has no captured immutable catalog")?;
    let job_policy = captured_order_job_policy(before, producer, recipe_id)?;
    let actor = before
        .raw_units
        .iter()
        .find(|unit| unit.unit_id == producer)
        .ok_or("Produce producer is absent from prior state")?;
    let owner = actor.owner_slot;
    let next_job = before.allocators.next_values().2;
    let next_job_after = next_job.checked_add(1).ok_or("Produce JobId overflow")?;
    let duration = before
        .compiled_durations
        .iter()
        .find(|duration| duration.recipe == *recipe_id)
        .map(|duration| duration.ticks)
        .ok_or("Produce duration is absent from captured catalog")?;
    let new_job = match job_policy.payment {
        turts_mod_api::PaymentPolicy::ChargeOnQueue => next_raw_job(
            producer,
            owner,
            recipe_id,
            &job_policy.costs,
            next_job,
            duration,
            true,
        )?,
        turts_mod_api::PaymentPolicy::ReserveThenCharge => next_raw_job(
            producer,
            owner,
            recipe_id,
            &job_policy.costs,
            next_job,
            duration,
            false,
        )?,
    };
    let expected_event = event_for_order(
        before,
        producer,
        turts_sim::EconomyEventKind::ProductionQueued {
            producer,
            job: next_job,
            recipe: recipe_id.clone(),
        },
    )?;
    if events != [expected_event.clone()] {
        return Err("Produce event differs from requested recipe and allocated JobId".into());
    }
    let mut expected = before.clone();
    let queue = expected
        .raw_queues
        .iter_mut()
        .find(|queue| queue.producer == producer)
        .ok_or("Produce producer has no raw queue")?;
    queue.jobs.push(new_job.0.clone());
    expected.raw_queues.sort_by_key(|queue| queue.producer);
    let public_queue = expected
        .economy
        .queues
        .iter_mut()
        .find(|queue| queue.producer == producer)
        .ok_or("Produce producer has no public queue")?;
    public_queue.jobs.push(new_job.1.clone());
    expected.economy.queues.sort_by_key(|queue| queue.producer);
    let (next_unit, next_node, _) = before.allocators.next_values();
    expected
        .allocators
        .set_next_values(next_unit, next_node, next_job_after);
    for cost in &job_policy.costs {
        let endpoint = match cost.source {
            turts_mod_api::CostSource::SlotBalance => {
                turts_sim::ReservationEndpointView::Slot(owner)
            }
            turts_mod_api::CostSource::ProducerInventory => {
                turts_sim::ReservationEndpointView::Inventory(producer)
            }
        };
        let reserved = job_policy.payment == turts_mod_api::PaymentPolicy::ReserveThenCharge;
        adjust_order_cost(
            &mut expected,
            &endpoint,
            &cost.resource,
            if reserved {
                0
            } else {
                -i128::from(cost.amount)
            },
            if reserved { i128::from(cost.amount) } else { 0 },
        )?;
    }
    if job_policy.payment == turts_mod_api::PaymentPolicy::ReserveThenCharge {
        let lines = job_policy
            .costs
            .iter()
            .map(|cost| turts_sim::ReservationLineView {
                endpoint: match cost.source {
                    turts_mod_api::CostSource::SlotBalance => {
                        turts_sim::ReservationEndpointView::Slot(owner)
                    }
                    turts_mod_api::CostSource::ProducerInventory => {
                        turts_sim::ReservationEndpointView::Inventory(producer)
                    }
                },
                resource: cost.resource.clone(),
                amount: cost.amount,
            })
            .collect();
        expected
            .ledger_claims
            .push(turts_sim::ReservationClaimView::Active {
                map_job_id: next_job,
                token_job_id: next_job,
                lines,
            });
    }
    expected.append_order_event(expected_event);
    if expected != *after {
        return Err(
            "Produce changed state outside the catalog-derived job, payment and event delta".into(),
        );
    }
    Ok(())
}

struct ExpectedQueuedJob(turts_sim::economy::views::RawJobAudit, turts_sim::JobView);

#[derive(Debug, Clone)]
struct OrderJobPolicy {
    costs: Vec<turts_sim::CostView>,
    payment: turts_mod_api::PaymentPolicy,
    refunds: turts_mod_api::CancelRefunds,
}

fn captured_order_job_policy(
    before: &AuditState,
    producer: u32,
    id: &turts_mod_api::ContentId,
) -> Result<OrderJobPolicy, String> {
    let catalog = before
        .catalog
        .as_ref()
        .ok_or("order job has no captured catalog")?;
    let unit_type = before
        .raw_units
        .iter()
        .find(|unit| unit.unit_id == producer)
        .and_then(|unit| {
            before
                .unit_types
                .iter()
                .find(|kind| kind.type_id == unit.unit_type)
        })
        .ok_or("order job producer type is absent")?;
    let unit = catalog
        .units
        .iter()
        .find(|unit| unit.id.to_string() == unit_type.content_id)
        .ok_or("order job producer definition is absent")?;
    if unit.research.is_some() {
        let technology = catalog
            .technologies
            .iter()
            .find(|technology| technology.id == *id)
            .ok_or("order job technology is absent from captured catalog")?;
        Ok(OrderJobPolicy {
            costs: technology.costs.clone(),
            payment: technology.payment,
            refunds: technology.refunds,
        })
    } else {
        let recipe = catalog
            .recipes
            .iter()
            .find(|recipe| recipe.id == *id)
            .ok_or("order job recipe is absent from captured catalog")?;
        Ok(OrderJobPolicy {
            costs: recipe.costs.clone(),
            payment: recipe.payment,
            refunds: recipe.refunds,
        })
    }
}

fn next_raw_job(
    producer: u32,
    owner_slot: u8,
    recipe_id: &turts_mod_api::ContentId,
    costs: &[turts_sim::CostView],
    id: u64,
    duration: u64,
    charge_now: bool,
) -> Result<ExpectedQueuedJob, String> {
    let charged = if charge_now {
        costs
            .iter()
            .map(|cost| turts_sim::CostView {
                source: cost.source,
                resource: cost.resource.clone(),
                amount: cost.amount,
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let reserved = if charge_now {
        Vec::new()
    } else {
        costs
            .iter()
            .map(|cost| turts_sim::CostView {
                source: cost.source,
                resource: cost.resource.clone(),
                amount: cost.amount,
            })
            .collect::<Vec<_>>()
    };
    let raw_lines = |reserved: bool| {
        costs
            .iter()
            .filter(|_| reserved)
            .map(|cost| turts_sim::ReservationLineView {
                endpoint: match cost.source {
                    turts_mod_api::CostSource::SlotBalance => {
                        turts_sim::ReservationEndpointView::Slot(owner_slot)
                    }
                    turts_mod_api::CostSource::ProducerInventory => {
                        turts_sim::ReservationEndpointView::Inventory(producer)
                    }
                },
                resource: cost.resource.clone(),
                amount: cost.amount,
            })
            .collect::<Vec<_>>()
    };
    let raw_charged = if charge_now {
        raw_lines(true)
    } else {
        Vec::new()
    };
    let reservation_lines = if charge_now {
        Vec::new()
    } else {
        raw_lines(true)
    };
    let raw = turts_sim::economy::views::RawJobAudit {
        id,
        recipe: recipe_id.clone(),
        stage: turts_sim::economy::production::JobStage::Queued,
        elapsed_ticks: 0,
        duration_ticks: duration,
        charged: raw_charged,
        reservation: (!charge_now).then_some(turts_sim::economy::views::RawReservationAudit {
            job_id: id,
            lines: reservation_lines,
        }),
    };
    let public = turts_sim::JobView {
        id,
        recipe: recipe_id.clone(),
        stage: turts_sim::economy::production::JobStage::Queued,
        elapsed_ticks: 0,
        duration_ticks: duration,
        charged,
        reserved,
    };
    Ok(ExpectedQueuedJob(raw, public))
}

fn adjust_order_cost(
    state: &mut AuditState,
    endpoint: &turts_sim::ReservationEndpointView,
    resource: &turts_mod_api::ContentId,
    on_hand_delta: i128,
    reserved_delta: i128,
) -> Result<(), String> {
    let add = |value: u64, delta: i128| -> Result<u64, String> {
        u64::try_from(
            i128::from(value)
                .checked_add(delta)
                .ok_or("order cost overflow")?,
        )
        .map_err(|_| "order cost balance underflow/overflow".into())
    };
    match endpoint {
        turts_sim::ReservationEndpointView::Slot(slot) => {
            let balance = state
                .economy
                .balances
                .iter_mut()
                .find(|row| row.slot == *slot && row.resource == *resource)
                .ok_or("order cost slot balance missing")?;
            balance.on_hand = add(balance.on_hand, on_hand_delta)?;
            balance.reserved = add(balance.reserved, reserved_delta)?;
        }
        turts_sim::ReservationEndpointView::Inventory(unit) => {
            let inventory = state
                .economy
                .inventories
                .iter_mut()
                .find(|row| row.unit == *unit)
                .ok_or("order cost inventory missing")?;
            let item = inventory
                .items
                .iter_mut()
                .find(|item| item.resource == *resource)
                .ok_or("order cost inventory item missing")?;
            item.on_hand = add(item.on_hand, on_hand_delta)?;
            item.reserved = add(item.reserved, reserved_delta)?;
        }
        turts_sim::ReservationEndpointView::Output(_) => {
            return Err("production cost cannot use output inventory".into());
        }
    }
    Ok(())
}

fn validate_cancel_order(
    producer: u32,
    job: turts_sim::JobId,
    _order: &turts_sim::OrderCommand,
    before: &AuditState,
    after: &AuditState,
    events: &[EconomyEvent],
) -> Result<(), String> {
    let queue = before
        .raw_queues
        .iter()
        .find(|queue| queue.producer == producer)
        .ok_or("Cancel producer queue is absent")?;
    let job_index = queue
        .jobs
        .iter()
        .position(|entry| entry.id == job.0)
        .ok_or("Cancel target job is absent from prior state")?;
    let target = &queue.jobs[job_index];
    let job_policy = captured_order_job_policy(before, producer, &target.recipe)?;
    let (ratio, refunds, releases) = match target.stage {
        turts_sim::economy::production::JobStage::Queued => (
            job_policy.refunds.queued,
            if job_policy.payment == turts_mod_api::PaymentPolicy::ChargeOnQueue {
                target.charged.clone()
            } else {
                Vec::new()
            },
            target
                .reservation
                .as_ref()
                .map_or_else(Vec::new, |reservation| reservation.lines.clone()),
        ),
        turts_sim::economy::production::JobStage::Running => (
            job_policy.refunds.running,
            target.charged.clone(),
            Vec::new(),
        ),
        turts_sim::economy::production::JobStage::OutputBlocked => (
            job_policy.refunds.output_blocked,
            target.charged.clone(),
            Vec::new(),
        ),
    };
    let owner = before
        .raw_units
        .iter()
        .find(|unit| unit.unit_id == producer)
        .map(|unit| unit.owner_slot)
        .ok_or("Cancel producer is absent from prior state")?;
    let refund_events = refunds
        .iter()
        .map(|line| {
            Ok(turts_sim::CostLine {
                source: event_source_from_endpoint(&line.endpoint),
                resource: line.resource.clone(),
                amount: refund_amount(line.amount, ratio)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let released_events = releases
        .iter()
        .map(|line| turts_sim::CostLine {
            source: event_source_from_endpoint(&line.endpoint),
            resource: line.resource.clone(),
            amount: line.amount,
        })
        .collect::<Vec<_>>();
    let expected_event = event_for_order(
        before,
        producer,
        turts_sim::EconomyEventKind::ProductionCancelled {
            producer,
            job: job.0,
            released: released_events,
            refunded: refund_events.clone(),
        },
    )?;
    if events != [expected_event.clone()] {
        return Err("Cancel event differs from the prior job and catalog refund policy".into());
    }
    let mut expected = before.clone();
    expected
        .raw_queues
        .iter_mut()
        .find(|queue| queue.producer == producer)
        .unwrap()
        .jobs
        .remove(job_index);
    expected
        .economy
        .queues
        .iter_mut()
        .find(|queue| queue.producer == producer)
        .ok_or("Cancel public queue is absent")?
        .jobs
        .retain(|entry| entry.id != job.0);
    for (line, event_line) in refunds.iter().zip(&refund_events) {
        if event_line.amount != 0 {
            adjust_order_cost(
                &mut expected,
                &line.endpoint,
                &line.resource,
                i128::from(event_line.amount),
                0,
            )?;
        }
    }
    for line in &releases {
        adjust_order_cost(
            &mut expected,
            &line.endpoint,
            &line.resource,
            0,
            -i128::from(line.amount),
        )?;
    }
    if target.reservation.is_some() {
        for claim in &mut expected.ledger_claims {
            if matches!(claim, turts_sim::ReservationClaimView::Active { map_job_id, .. } if *map_job_id == job.0)
            {
                *claim = turts_sim::ReservationClaimView::Closed { map_job_id: job.0 };
            }
        }
    }
    expected.append_order_event(expected_event);
    if expected != *after {
        return Err("Cancel changed state outside requested job removal, refund, reservation release and event".into());
    }
    let _ = owner;
    Ok(())
}

fn refund_amount(amount: u64, ratio: turts_mod_api::RefundRatio) -> Result<u64, String> {
    if ratio.denominator == 0 {
        return Err("refund ratio denominator is zero".into());
    }
    u64::try_from(
        u128::from(amount)
            .checked_mul(u128::from(ratio.numerator))
            .ok_or("refund multiplication overflow")?
            / u128::from(ratio.denominator),
    )
    .map_err(|_| "refund amount overflow".into())
}

fn event_source_from_endpoint(
    endpoint: &turts_sim::ReservationEndpointView,
) -> turts_sim::EventSource {
    match endpoint {
        turts_sim::ReservationEndpointView::Slot(slot) => turts_sim::EventSource::Slot(*slot),
        turts_sim::ReservationEndpointView::Inventory(unit) => {
            turts_sim::EventSource::Inventory(*unit)
        }
        turts_sim::ReservationEndpointView::Output(unit) => turts_sim::EventSource::Output(*unit),
    }
}

fn event_for_order(
    before: &AuditState,
    unit_id: u32,
    kind: turts_sim::EconomyEventKind,
) -> Result<EconomyEvent, String> {
    Ok(EconomyEvent {
        tick: before.economy.tick,
        ordinal: before
            .next_event_ordinal
            .ok_or("order event has no event ordinal")?,
        owner_slot: before
            .raw_units
            .iter()
            .find(|unit| unit.unit_id == unit_id)
            .map_or(0, |unit| unit.owner_slot),
        kind,
    })
}

trait AppendOrderEvent {
    fn append_order_event(&mut self, event: EconomyEvent);
}

impl AppendOrderEvent for AuditState {
    fn append_order_event(&mut self, event: EconomyEvent) {
        self.next_event_ordinal = event.ordinal.checked_add(1);
        self.pending_events.push(event);
    }
}

fn validate_order_span_end(
    span: &ActiveOrderSpan,
    operation: &OperationAudit,
) -> Result<(), String> {
    if span.command.after != operation.after {
        return Err(format!(
            "order command/operation state mismatch for id {}",
            operation.operation_id
        ));
    }
    let accepted = &span.command.result.units.accepted;
    let refused = &span.command.result.units.refused;
    let mut subjects = BTreeSet::new();
    for unit in accepted {
        if !subjects.insert(*unit) || refused.iter().any(|(refused, _)| refused == unit) {
            return Err(format!(
                "order command has duplicate or conflicting result for unit {unit}"
            ));
        }
        if !span.subjects_with_operations.contains(unit) {
            return Err(format!(
                "accepted unit {unit} has no per-unit order operation"
            ));
        }
    }
    for (unit, _) in refused {
        if !subjects.insert(*unit) {
            return Err(format!(
                "order command has duplicate result for unit {unit}"
            ));
        }
    }
    let turts_sim::CommandKind::Order(order) = &span.command.input.kind else {
        return Err("order span has non-order input".into());
    };
    let expected_job_id =
        if order.behaviour == turts_mod_api::Behaviour::Produce && !accepted.is_empty() {
            Some(turts_sim::JobId(
                span.command.before.allocators.next_values().2,
            ))
        } else {
            None
        };
    let actual_job_id = span
        .command
        .result
        .outcome
        .as_ref()
        .ok()
        .and_then(|applied| applied.job_id);
    if actual_job_id != expected_job_id {
        return Err(format!(
            "order command has aggregate JobId {actual_job_id:?}, expected {expected_job_id:?}"
        ));
    }
    let requested = order.units.iter().copied().collect::<BTreeSet<_>>();
    if requested.len() != order.units.len() || requested != subjects {
        return Err("order per-unit results do not cover the unique envelope subjects".into());
    }
    let mut sorted_requested = order.units.clone();
    sorted_requested.sort_unstable();
    let mut next_subject_index = span.next_subject_index;
    while next_subject_index < sorted_requested.len() {
        let unit_id = sorted_requested[next_subject_index];
        if accepted.binary_search(&unit_id).is_ok() {
            return Err(format!(
                "accepted order for unit {unit_id} has no per-unit operation"
            ));
        }
        let reason = refused
            .iter()
            .find(|(subject, _)| *subject == unit_id)
            .map(|(_, reason)| *reason)
            .ok_or_else(|| format!("order subject {unit_id} has no result"))?;
        let expected =
            expected_order_refusal(order, unit_id, &operation.after, span.command.input.slot.0)?;
        if expected != Some(reason) {
            return Err(format!(
                "refusal for unit {unit_id} differs from independent validation: expected {expected:?}, got {reason:?}"
            ));
        }
        next_subject_index += 1;
    }
    Ok(())
}

fn validate_command_phase_order(
    command: &CommandAudit,
    operation: &OperationAudit,
) -> Result<(), String> {
    let turts_sim::CommandKind::Order(order) = &command.input.kind else {
        unreachable!()
    };
    let accepted = &command.result.units.accepted;
    let refused = &command.result.units.refused;
    if accepted.is_empty() {
        let turts_sim::CommandKind::Order(order) = &command.input.kind else {
            unreachable!()
        };
        for (unit, reason) in refused {
            let expected =
                expected_order_refusal(order, *unit, &command.before, command.input.slot.0)?;
            if expected != Some(*reason) {
                return Err(format!(
                    "all-refused order unit {unit} has reason {reason:?}, expected {expected:?}"
                ));
            }
        }
        return if operation.before == operation.after {
            Ok(())
        } else {
            Err("all-refused order command changed simulation state".into())
        };
    }
    let plain_move = order.behaviour == turts_mod_api::Behaviour::Move
        && order.queue == turts_sim::QueueMode::Replace
        && order.group == 0
        && order.units.len() == 1;
    let plain_gather = order.behaviour == turts_mod_api::Behaviour::Gather
        && order.queue == turts_sim::QueueMode::Replace
        && order.group == 0;
    if !plain_move && !plain_gather {
        return Err("accepted order used an undocumented Command-phase shortcut".into());
    }
    let requested = order.units.iter().copied().collect::<BTreeSet<_>>();
    let accepted_set = accepted.iter().copied().collect::<BTreeSet<_>>();
    let refused_set = refused
        .iter()
        .map(|(unit, _)| *unit)
        .collect::<BTreeSet<_>>();
    if requested.len() != order.units.len()
        || accepted_set.len() != accepted.len()
        || refused_set.len() != refused.len()
        || !accepted_set.is_disjoint(&refused_set)
        || accepted_set
            .union(&refused_set)
            .copied()
            .collect::<BTreeSet<_>>()
            != requested
    {
        return Err("plain order command results do not cover the unique envelope subjects".into());
    }
    let mut sorted_accepted = accepted.clone();
    sorted_accepted.sort_unstable();
    if sorted_accepted != *accepted {
        return Err("plain order command results are not in unit-id order".into());
    }
    for unit_id in accepted {
        if let Some(reason) =
            expected_order_refusal(order, *unit_id, &operation.before, command.input.slot.0)?
        {
            return Err(format!(
                "accepted plain order for unit {unit_id} should be refused as {reason:?}"
            ));
        }
        if operation
            .before
            .raw_units
            .iter()
            .find(|unit| unit.unit_id == *unit_id)
            .zip(
                operation
                    .after
                    .raw_units
                    .iter()
                    .find(|unit| unit.unit_id == *unit_id),
            )
            .is_some_and(|(before, after)| before.pos != after.pos)
            || operation
                .before
                .units
                .iter()
                .find(|unit| unit.unit_id == *unit_id)
                != operation
                    .after
                    .units
                    .iter()
                    .find(|unit| unit.unit_id == *unit_id)
        {
            return Err(format!(
                "plain order for unit {unit_id} moved its actor before Movement"
            ));
        }
        let expected_order = order_record(order);
        if plain_move {
            let turts_sim::OrderTargetSim::Point(point) = &order.target else {
                return Err("accepted plain Move has a non-point target".into());
            };
            if unit_orders_for(&operation.before, *unit_id)
                != unit_orders_for(&operation.after, *unit_id)
                || raw_move_target(&operation.after, *unit_id)
                    != Some(clamped_audit_point(&operation.before, *point))
            {
                return Err(format!(
                    "plain Move for unit {unit_id} changed order state or missed its target"
                ));
            }
        } else {
            let expected = turts_sim::UnitOrders {
                current: Some(expected_order),
                ..Default::default()
            };
            if unit_orders_for(&operation.after, *unit_id) != expected {
                return Err(format!(
                    "plain Gather for unit {unit_id} did not become current"
                ));
            }
            let turts_sim::OrderTargetSim::Node(node) = order.target else {
                return Err("accepted plain Gather has a non-node target".into());
            };
            let depleted = operation
                .before
                .nodes
                .iter()
                .any(|entry| entry.id == node && entry.remaining == 0);
            let assigned = operation
                .after
                .economy
                .orders
                .iter()
                .any(|entry| entry.unit == *unit_id && entry.node == Some(node));
            if assigned == depleted {
                return Err(format!(
                    "plain Gather for unit {unit_id} did not assign its node"
                ));
            }
        }
    }
    for (unit_id, _) in refused {
        let reason = refused
            .iter()
            .find(|(unit, _)| unit == unit_id)
            .map(|(_, reason)| *reason)
            .unwrap();
        let expected =
            expected_order_refusal(order, *unit_id, &operation.before, command.input.slot.0)?;
        if expected != Some(reason) {
            return Err(format!(
                "refused plain order for unit {unit_id} has reason {reason:?}, expected {expected:?}"
            ));
        }
        if raw_move_target(&operation.before, *unit_id)
            != raw_move_target(&operation.after, *unit_id)
            || unit_orders_for(&operation.before, *unit_id)
                != unit_orders_for(&operation.after, *unit_id)
            || operation
                .before
                .economy
                .orders
                .iter()
                .find(|entry| entry.unit == *unit_id)
                != operation
                    .after
                    .economy
                    .orders
                    .iter()
                    .find(|entry| entry.unit == *unit_id)
        {
            return Err(format!("refused plain order changed unit {unit_id}"));
        }
    }
    Ok(())
}

fn order_record(order: &turts_sim::OrderCommand) -> turts_sim::Order {
    turts_sim::Order {
        order: order.order.clone(),
        behaviour: order.behaviour,
        target: order.target.clone(),
        group: order.group,
    }
}

fn unit_orders_for(state: &AuditState, unit_id: u32) -> turts_sim::UnitOrders {
    state
        .unit_orders
        .iter()
        .find(|entry| entry.unit_id == unit_id)
        .map_or_else(Default::default, |entry| entry.orders.clone())
}

fn raw_move_target(state: &AuditState, unit_id: u32) -> Option<turts_mod_api::WorldPos> {
    state
        .raw_units
        .iter()
        .find(|unit| unit.unit_id == unit_id)
        .and_then(|unit| unit.move_target.clone())
}

fn clamped_audit_point(state: &AuditState, point: bevy::math::Vec2) -> turts_mod_api::WorldPos {
    let (x, y) = state.bounds.as_ref().map_or((point.x, point.y), |bounds| {
        (point.x.clamp(0.0, bounds.x), point.y.clamp(0.0, bounds.y))
    });
    turts_mod_api::WorldPos { x, y }
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

fn validate_operation_phase(
    previous: &AuditState,
    current: &AuditState,
    movement_pending: bool,
    phase: turts_sim::economy::transaction::AuditPhase,
) -> Result<(), String> {
    match phase {
        turts_sim::economy::transaction::AuditPhase::Command
        | turts_sim::economy::transaction::AuditPhase::Orders => {
            validate_clock_continuity(previous, current, "orders operation")
        }
        turts_sim::economy::transaction::AuditPhase::Other => {
            validate_operation_continuity(previous, current, movement_pending)
        }
    }
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
                        // Match Bevy Fixed::from_hz: Duration stores the reciprocal in f64,
                        // then Time<Fixed>::delta_secs exposes that duration as f32.
                        let delta = std::time::Duration::from_secs_f64(1.0 / f64::from(tick_hz))
                            .as_secs_f32();
                        let result = turts_sim::movement::step(
                            &turts_sim::movement::MovementInput {
                                position,
                                target: target_vec,
                                speed,
                            },
                            delta,
                        );
                        if result.reached {
                            (result.position, None)
                        } else {
                            (result.position, Some(target.clone()))
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
    use super::{
        EconomyLogRecord, EconomyLogWriter, read_jsonl, validate_operation_continuity,
        validate_records,
    };
    use std::io::Cursor;
    use turts_mod_api::WorldPos;
    use turts_sim::economy::views::RawUnitAudit;
    use turts_sim::{AuditState, AuditTrail, headless_app};

    fn initial() -> AuditState {
        let mut app = headless_app(10);
        turts_sim::audit_state(app.world_mut())
    }

    type OrderCapabilityRow = (&'static str, bool, bool, bool, bool, bool, bool, bool);

    fn order_policy(
        id: &str,
        behaviour: turts_mod_api::Behaviour,
        target: turts_mod_api::TargetKind,
        capabilities: Vec<OrderCapabilityRow>,
    ) -> turts_sim::OrderPolicyAudit {
        turts_sim::OrderPolicyAudit {
            id: turts_mod_api::ContentId::parse(id).unwrap(),
            behaviour,
            target,
            units: vec![],
            queue_depth: 16,
            unit_capabilities: capabilities
                .into_iter()
                .map(
                    |(
                        unit,
                        movement,
                        weapon,
                        gatherer,
                        producer,
                        research,
                        removable,
                        toggleable,
                    )| {
                        turts_sim::OrderUnitCapabilitiesAudit {
                            unit: unit.into(),
                            movement,
                            weapon,
                            gatherer,
                            producer,
                            research,
                            removable,
                            toggleable,
                        }
                    },
                )
                .collect(),
        }
    }

    fn single_order_records(
        mut before: AuditState,
        mut after: AuditState,
        command: turts_sim::Command,
        accepted: Vec<u32>,
        refused: Vec<(u32, turts_sim::RejectReason)>,
        phase: turts_sim::economy::transaction::AuditPhase,
    ) -> Vec<EconomyLogRecord> {
        let applied_job_id = matches!(
            &command.kind,
            turts_sim::CommandKind::Order(order) if order.behaviour == turts_mod_api::Behaviour::Produce && !accepted.is_empty()
        ).then(|| turts_sim::JobId(before.allocators.next_values().2));
        if let turts_sim::CommandKind::Order(order) = &command.kind {
            let definition = order
                .policy
                .as_ref()
                .map(|policy| turts_sim::CatalogOrderAudit {
                    id: policy.id.clone(),
                    behaviour: policy.behaviour,
                    target: policy.target,
                    units: policy.units.clone(),
                    queue_depth: policy.queue_depth,
                    unit_capabilities: policy.unit_capabilities.clone(),
                })
                .expect("test Order commands carry their catalog definition");
            let catalog =
                before
                    .catalog
                    .get_or_insert_with(|| turts_sim::economy::views::CatalogAudit {
                        resource_kinds: vec![],
                        nodes: vec![],
                        units: vec![],
                        recipes: vec![],
                        order_definitions: vec![],
                        technologies: vec![],
                        item_recipes: vec![],
                    });
            if !catalog
                .order_definitions
                .iter()
                .any(|existing| existing.id == order.order)
            {
                catalog.order_definitions.push(definition);
                catalog
                    .order_definitions
                    .sort_by(|left, right| left.id.cmp(&right.id));
            }
            after.catalog.clone_from(&before.catalog);
        }
        let boundary = turts_sim::CommandAudit {
            operation_id: 1,
            operation_end_id: None,
            input: command.clone(),
            result: turts_sim::CommandResult {
                slot: command.slot,
                seq: command.seq,
                outcome: if accepted.is_empty() {
                    Err(refused
                        .first()
                        .map_or(turts_sim::RejectReason::NotApplicable, |(_, reason)| {
                            *reason
                        }))
                } else {
                    Ok(turts_sim::Applied {
                        job_id: applied_job_id,
                    })
                },
                units: turts_sim::UnitOutcomes { accepted, refused },
            },
            before: before.clone(),
            after: after.clone(),
        };
        vec![
            EconomyLogRecord::Initial(before.clone()),
            EconomyLogRecord::CommandBoundary(boundary),
            EconomyLogRecord::Operation(turts_sim::OperationAudit {
                operation_id: 1,
                phase,
                unit_id: (phase == turts_sim::economy::transaction::AuditPhase::Orders)
                    .then_some(1),
                before,
                after,
                events: vec![],
            }),
        ]
    }

    fn one_moving_unit_state() -> AuditState {
        let mut state = initial();
        state.raw_units = vec![RawUnitAudit {
            unit_id: 1,
            owner_slot: 0,
            unit_type: 0,
            pos: WorldPos { x: 2.0, y: 2.0 },
            move_target: None,
        }];
        state.unit_types = vec![turts_sim::UnitTypeView {
            type_id: 0,
            content_id: "fixture:walker".into(),
            movement_speed: Some(3.0),
            radius: 0.5,
        }];
        state.units = vec![turts_sim::UnitView {
            unit_id: 1,
            content_id: "fixture:walker".into(),
            owner_slot: 0,
            pos: WorldPos { x: 2.0, y: 2.0 },
        }];
        state
    }

    fn automatic_move_log(
        orders: turts_sim::UnitOrders,
        target_units: &[(u32, bevy::math::Vec2)],
    ) -> Vec<EconomyLogRecord> {
        use std::collections::BTreeMap;
        use turts_mod_api::{
            ContentCatalog, Footprint, OrderDef, PackageId, ResolvedUnit, TargetKind, UnitDef,
            UnitRegistry,
        };
        use turts_sim::{Owner, Position, UnitType, UnitTypes};

        let mut app = headless_app(20);
        let package = PackageId::parse("fixture").unwrap();
        let kind = app
            .world_mut()
            .resource_mut::<UnitTypes>()
            .register_unit_type(
                &package,
                UnitDef {
                    id: "walker".into(),
                    name: "Walker".into(),
                    footprint: Footprint::Circle { radius: 0.5 },
                    movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
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
        let unit_id = turts_mod_api::ContentId::new(&package, "walker");
        let move_id = turts_mod_api::ContentId::new(&package, "move");
        let mut catalog = ContentCatalog {
            resources: BTreeMap::new(),
            units: BTreeMap::new(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: BTreeMap::new(),
            source_files: BTreeMap::new(),
        };
        catalog.units.insert(
            unit_id.clone(),
            ResolvedUnit {
                id: unit_id,
                name: "Walker".into(),
                footprint: Footprint::Circle { radius: 0.5 },
                movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
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
        );
        catalog.orders.insert(
            move_id.clone(),
            OrderDef {
                id: move_id.local.clone(),
                name: "Move".into(),
                behaviour: turts_mod_api::Behaviour::Move,
                target: TargetKind::PointOrUnit,
                fire: None,
                engage: None,
                icon: String::new(),
                hotkey: String::new(),
                cursor: None,
                card: None,
                units: vec![],
                description: String::new(),
            },
        );
        app.world_mut()
            .insert_resource(turts_sim::bootstrap::InstalledCatalog(catalog));
        app.world_mut().spawn((
            turts_sim::UnitId(1),
            UnitType(kind),
            Owner(turts_sim::SlotId(0)),
            Position(bevy::math::Vec2::new(2.0, 2.0)),
            orders,
        ));
        for (unit_id, position) in target_units {
            app.world_mut().spawn((
                turts_sim::UnitId(*unit_id),
                UnitType(kind),
                Owner(turts_sim::SlotId(0)),
                Position(*position),
                turts_sim::UnitOrders::default(),
            ));
        }
        app.world_mut().resource_mut::<AuditTrail>().enable();
        let mut records = vec![EconomyLogRecord::Initial(turts_sim::audit_state(
            app.world_mut(),
        ))];
        app.update();
        let (operations, ticks) = {
            let mut audit = app.world_mut().resource_mut::<AuditTrail>();
            (audit.drain_operations(), audit.drain_ticks())
        };
        records.extend(operations.into_iter().map(EconomyLogRecord::Operation));
        records.extend(ticks.into_iter().map(EconomyLogRecord::from_tick));
        records
    }

    fn actual_queued_gather_log() -> Vec<EconomyLogRecord> {
        use std::collections::BTreeMap;
        use turts_mod_api::{
            ContentCatalog, ContentId, Footprint, MapDef, MapSize, NodePlacement, OrderDef,
            Placement, ResolvedGatherer, ResolvedInventory, ResolvedNode, ResolvedUnit,
            ResourceDef, ResourceKind, SourceDef, TargetKind, WorldPos,
        };

        let mut app = headless_app(20);
        let package = turts_mod_api::PackageId::parse("fixture").unwrap();
        let item = ContentId::new(&package, "iron");
        let node = ContentId::new(&package, "deposit");
        let worker = ContentId::new(&package, "worker");
        let gather = ContentId::new(&package, "gather");
        let catalog = ContentCatalog {
            resources: BTreeMap::from([(
                item.clone(),
                ResourceDef {
                    id: "iron".into(),
                    name: "Iron".into(),
                    unit_name: "ore".into(),
                    kind: ResourceKind::Item,
                },
            )]),
            units: BTreeMap::from([(
                worker.clone(),
                ResolvedUnit {
                    id: worker.clone(),
                    name: "Worker".into(),
                    footprint: Footprint::Circle { radius: 0.5 },
                    movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                    inventory: Some(ResolvedInventory {
                        capacity: 2,
                        accepts: vec![item.clone()],
                    }),
                    gatherer: Some(ResolvedGatherer {
                        items: vec![item.clone()],
                        gather_per_second: 20,
                        unload_per_second: 20,
                        interaction_range: 1.0,
                    }),
                    drop_off: None,
                    producer: None,
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
            )]),
            nodes: BTreeMap::from([(
                node.clone(),
                ResolvedNode {
                    id: node.clone(),
                    name: "Deposit".into(),
                    item: item.clone(),
                    radius: 0.5,
                },
            )]),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::from([(
                gather.clone(),
                OrderDef {
                    id: gather.local.clone(),
                    name: "Gather".into(),
                    behaviour: turts_mod_api::Behaviour::Gather,
                    target: TargetKind::Node,
                    fire: None,
                    engage: None,
                    icon: String::new(),
                    hotkey: String::new(),
                    cursor: None,
                    card: None,
                    units: vec![],
                    description: String::new(),
                },
            )]),
            order_queue_depths: BTreeMap::new(),
            source_files: BTreeMap::new(),
        };
        let map = MapDef {
            id: "field".into(),
            size: MapSize { x: 20.0, y: 20.0 },
            placements: vec![Placement {
                unit: worker.to_string(),
                owner_slot: 0,
                pos: WorldPos { x: 2.0, y: 2.0 },
                contents: vec![],
                facing: None,
            }],
            nodes: vec![NodePlacement {
                node: node.to_string(),
                pos: WorldPos { x: 3.1, y: 2.0 },
                remaining: 1,
            }],
            slot_economy: vec![],
            belts: vec![],
        };
        let staged = turts_sim::stage_package(
            app.world(),
            catalog,
            SourceDef {
                file: "maps/field.ron".into(),
                value: map,
            },
            20,
        )
        .unwrap();
        turts_sim::install_package(app.world_mut(), staged);
        let entity = app
            .world()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == 1)
            })
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(entity)
            .insert(turts_sim::UnitOrders {
                queue: std::collections::VecDeque::from([turts_sim::Order {
                    order: gather,
                    behaviour: turts_mod_api::Behaviour::Gather,
                    target: turts_sim::OrderTargetSim::Node(turts_sim::NodeId(1)),
                    group: 1,
                }]),
                ..Default::default()
            });
        app.world_mut().resource_mut::<AuditTrail>().enable();
        let mut records = vec![EconomyLogRecord::Initial(turts_sim::audit_state(
            app.world_mut(),
        ))];
        for _ in 0..40 {
            app.update();
            let (operations, ticks) = {
                let mut audit = app.world_mut().resource_mut::<AuditTrail>();
                (audit.drain_operations(), audit.drain_ticks())
            };
            records.extend(operations.into_iter().map(EconomyLogRecord::Operation));
            records.extend(ticks.into_iter().map(EconomyLogRecord::from_tick));
            let state = turts_sim::audit_state(app.world_mut());
            if state
                .nodes
                .iter()
                .any(|node| node.id == turts_sim::NodeId(1) && node.remaining == 0)
                && state.unit_orders.iter().any(|entry| {
                    entry.unit_id == 1
                        && entry.orders.last_failure.as_ref().is_some_and(|failure| {
                            failure.reason == turts_sim::RejectReason::NodeDepleted
                        })
                })
            {
                break;
            }
        }
        records
    }

    fn one_order_command(
        id: &str,
        behaviour: turts_mod_api::Behaviour,
        target_kind: turts_mod_api::TargetKind,
        target: turts_sim::OrderTargetSim,
        policy: turts_sim::OrderPolicyAudit,
    ) -> turts_sim::Command {
        turts_sim::Command {
            slot: turts_sim::SlotId(0),
            seq: 1,
            kind: turts_sim::CommandKind::Order(Box::new(turts_sim::OrderCommand {
                order: turts_mod_api::ContentId::parse(id).unwrap(),
                behaviour,
                target_kind,
                policy: Some(policy),
                units: vec![1],
                target,
                queue: turts_sim::QueueMode::Replace,
                group: 1,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        }
    }

    fn producer_order_state(with_job: bool) -> (AuditState, turts_mod_api::ContentId) {
        use turts_sim::economy::views::{
            CatalogAudit, CatalogRecipeAudit, CatalogUnitAudit, DurationAudit, QueueView,
            RawJobAudit, RawQueueAudit,
        };
        let recipe_id = turts_mod_api::ContentId::parse("fixture:recipe").unwrap();
        let producer_id = turts_mod_api::ContentId::parse("fixture:producer").unwrap();
        let output_id = turts_mod_api::ContentId::parse("fixture:unit").unwrap();
        let mut state = one_moving_unit_state();
        state.unit_types[0].content_id = producer_id.to_string();
        state.units[0].content_id = producer_id.to_string();
        state.catalog = Some(CatalogAudit {
            resource_kinds: vec![],
            nodes: vec![],
            units: vec![CatalogUnitAudit {
                id: producer_id,
                radius: 0.5,
                movement_speed: None,
                inventory: None,
                gatherer: None,
                drop_off: vec![],
                producer: Some((vec![recipe_id.clone()], 4, vec![])),
                research: None,
            }],
            recipes: vec![CatalogRecipeAudit {
                id: recipe_id.clone(),
                output_unit: output_id,
                duration_ms: 1000,
                costs: vec![],
                payment: turts_mod_api::PaymentPolicy::ChargeOnQueue,
                refunds: turts_mod_api::CancelRefunds {
                    queued: turts_mod_api::RefundRatio {
                        numerator: 1,
                        denominator: 1,
                    },
                    running: turts_mod_api::RefundRatio {
                        numerator: 1,
                        denominator: 1,
                    },
                    output_blocked: turts_mod_api::RefundRatio {
                        numerator: 1,
                        denominator: 1,
                    },
                },
            }],
            order_definitions: vec![],
            technologies: vec![],
            item_recipes: vec![],
        });
        state.compiled_durations = vec![DurationAudit {
            recipe: recipe_id.clone(),
            ticks: 20,
        }];
        let job = with_job.then_some(RawJobAudit {
            id: 1,
            recipe: recipe_id.clone(),
            stage: turts_sim::JobStage::Queued,
            elapsed_ticks: 0,
            duration_ticks: 20,
            charged: vec![],
            reservation: None,
        });
        state.raw_queues = vec![RawQueueAudit {
            producer: 1,
            jobs: job.clone().into_iter().collect(),
        }];
        state.economy.queues = vec![QueueView {
            producer: 1,
            owner_slot: 0,
            jobs: job
                .map(|job| {
                    vec![turts_sim::JobView {
                        id: job.id,
                        recipe: job.recipe,
                        stage: job.stage,
                        elapsed_ticks: job.elapsed_ticks,
                        duration_ticks: job.duration_ticks,
                        charged: vec![],
                        reserved: vec![],
                    }]
                })
                .unwrap_or_default(),
        }];
        (state, recipe_id)
    }

    #[test]
    fn validator_accepts_an_orders_operation_before_movement() {
        use turts_mod_api::{Footprint, PackageId, UnitDef, UnitRegistry};
        use turts_sim::{
            MoveTarget, Order, OrderTargetSim, Owner, Position, SlotId, UnitId, UnitOrders,
            UnitType, UnitTypes, Vec2,
        };
        let mut app = headless_app(10);
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
        assert!(validate_records(&records).is_ok());

        if let EconomyLogRecord::Operation(operation) = &mut records[1] {
            operation.phase = turts_sim::economy::transaction::AuditPhase::Other;
        } else {
            panic!("captured unit order did not decode as an operation");
        }
        assert!(
            validate_records(&records)
                .unwrap_err()
                .contains("unit 1 movement differs from its prior target and speed")
        );
    }

    #[test]
    fn validator_checks_full_multi_operation_order_command_span() {
        use turts_sim::economy::transaction::AuditPhase;
        use turts_sim::economy::views::RawUnitOrdersAudit;
        use turts_sim::{
            Command, CommandKind, CommandResult, Order, OrderCommand, OrderTargetSim, QueueMode,
            SlotId, UnitOrders,
        };
        let mut before = initial();
        before.raw_units = (1..=2)
            .map(|unit_id| RawUnitAudit {
                unit_id,
                owner_slot: 0,
                unit_type: 0,
                pos: WorldPos {
                    x: 1.0 + unit_id as f32,
                    y: 1.0,
                },
                move_target: None,
            })
            .collect();
        before.unit_types = vec![turts_sim::UnitTypeView {
            type_id: 0,
            content_id: "fixture:walker".into(),
            movement_speed: Some(3.0),
            radius: 0.5,
        }];
        before.units = (1..=2)
            .map(|unit_id| turts_sim::UnitView {
                unit_id,
                content_id: "fixture:walker".into(),
                owner_slot: 0,
                pos: WorldPos {
                    x: 1.0 + unit_id as f32,
                    y: 1.0,
                },
            })
            .collect();
        let make_state = |ids: &[u32]| {
            let mut state = before.clone();
            state.unit_orders = ids
                .iter()
                .map(|unit_id| RawUnitOrdersAudit {
                    unit_id: *unit_id,
                    orders: UnitOrders {
                        current: Some(Order {
                            order: turts_mod_api::ContentId::parse("fixture:move").unwrap(),
                            behaviour: turts_mod_api::Behaviour::Move,
                            target: OrderTargetSim::Point(bevy_math::Vec2::new(8.0, 8.0)),
                            group: 1,
                        }),
                        ..Default::default()
                    },
                })
                .collect();
            for unit_id in ids {
                state
                    .raw_units
                    .iter_mut()
                    .find(|unit| unit.unit_id == *unit_id)
                    .unwrap()
                    .move_target = Some(WorldPos { x: 8.0, y: 8.0 });
            }
            state
        };
        let after_first = make_state(&[1]);
        let after_last = make_state(&[1, 2]);
        let command = Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: turts_mod_api::ContentId::parse("fixture:move").unwrap(),
                behaviour: turts_mod_api::Behaviour::Move,
                units: vec![1, 2],
                target_kind: turts_mod_api::TargetKind::Point,
                policy: Some(turts_sim::OrderPolicyAudit {
                    id: turts_mod_api::ContentId::parse("fixture:move").unwrap(),
                    behaviour: turts_mod_api::Behaviour::Move,
                    target: turts_mod_api::TargetKind::Point,
                    units: vec![],
                    queue_depth: 16,
                    unit_capabilities: vec![turts_sim::OrderUnitCapabilitiesAudit {
                        unit: "fixture:walker".into(),
                        movement: true,
                        weapon: false,
                        gatherer: false,
                        producer: false,
                        research: false,
                        removable: false,
                        toggleable: false,
                    }],
                }),
                target: OrderTargetSim::Point(bevy_math::Vec2::new(8.0, 8.0)),
                queue: QueueMode::Replace,
                group: 1,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let mut records = single_order_records(
            before.clone(),
            after_last.clone(),
            command.clone(),
            vec![1, 2],
            vec![],
            AuditPhase::Orders,
        );
        let authoritative_catalog = match &records[0] {
            EconomyLogRecord::Initial(state) => state.catalog.clone(),
            _ => unreachable!(),
        };
        let mut after_first = after_first;
        let mut after_last = after_last;
        after_first.catalog.clone_from(&authoritative_catalog);
        after_last.catalog.clone_from(&authoritative_catalog);
        if let EconomyLogRecord::CommandBoundary(boundary) = &mut records[1] {
            boundary.operation_end_id = Some(2);
        }
        if let EconomyLogRecord::Operation(operation) = &mut records[2] {
            operation.after = after_first.clone();
        }
        records.push(EconomyLogRecord::Operation(turts_sim::OperationAudit {
            operation_id: 2,
            phase: AuditPhase::Orders,
            unit_id: Some(2),
            before: after_first.clone(),
            after: after_last.clone(),
            events: vec![],
        }));
        validate_records(&records).unwrap_or_else(|error| panic!("{error}"));

        let mut wrong_subject = records.clone();
        if let EconomyLogRecord::Operation(operation) = &mut wrong_subject[2] {
            operation.unit_id = Some(99);
        }
        assert!(
            validate_records(&wrong_subject)
                .unwrap_err()
                .contains("outside its envelope")
        );

        let mut incomplete = records.clone();
        if let EconomyLogRecord::CommandBoundary(command) = &mut incomplete[1] {
            command.operation_end_id = None;
        }
        assert!(
            validate_records(&incomplete)
                .unwrap_err()
                .contains("state mismatch")
        );

        let mut wrong_phase = incomplete;
        if let EconomyLogRecord::Operation(operation) = &mut wrong_phase[3] {
            operation.phase = AuditPhase::Other;
        }
        if let EconomyLogRecord::CommandBoundary(command) = &mut wrong_phase[1] {
            command.operation_end_id = Some(2);
        }
        assert!(
            validate_records(&wrong_phase)
                .unwrap_err()
                .contains("span ended unexpectedly")
        );

        let mut nested = records.clone();
        if let EconomyLogRecord::CommandBoundary(command) = &mut nested[1] {
            command.after = after_first.clone();
        }
        let EconomyLogRecord::CommandBoundary(mut nested_boundary) = records[1].clone() else {
            unreachable!()
        };
        nested_boundary.operation_id = 2;
        nested_boundary.operation_end_id = Some(2);
        nested_boundary.input.seq = 2;
        nested_boundary.result.seq = 2;
        nested_boundary.before = after_first.clone();
        nested_boundary.after = after_last.clone();
        nested.insert(2, EconomyLogRecord::CommandBoundary(nested_boundary));
        assert!(
            validate_records(&nested)
                .unwrap_err()
                .contains("overlapping order command span")
        );

        let legacy_with_span = initial();
        let command = Command {
            slot: SlotId(0),
            seq: 3,
            kind: CommandKind::MoveTo {
                unit_ids: vec![],
                target: bevy_math::Vec2::ZERO,
            },
        };
        let boundary = turts_sim::CommandAudit {
            operation_id: 1,
            operation_end_id: Some(1),
            input: command.clone(),
            result: CommandResult {
                slot: command.slot,
                seq: command.seq,
                outcome: Err(turts_sim::RejectReason::EmptyCommand),
                units: Default::default(),
            },
            before: legacy_with_span.clone(),
            after: legacy_with_span.clone(),
        };
        let invalid_legacy_span = vec![
            EconomyLogRecord::Initial(legacy_with_span.clone()),
            EconomyLogRecord::CommandBoundary(boundary),
            EconomyLogRecord::Operation(turts_sim::OperationAudit {
                operation_id: 1,
                phase: AuditPhase::Command,
                unit_id: None,
                before: legacy_with_span.clone(),
                after: legacy_with_span,
                events: vec![],
            }),
        ];
        assert!(
            validate_records(&invalid_legacy_span)
                .unwrap_err()
                .contains("state mismatch")
        );
    }

    fn valid_point_move_records() -> Vec<EconomyLogRecord> {
        let mut before = one_moving_unit_state();
        before
            .raw_units
            .push(turts_sim::economy::views::RawUnitAudit {
                unit_id: 2,
                owner_slot: 0,
                unit_type: 0,
                pos: WorldPos { x: 8.0, y: 8.0 },
                move_target: None,
            });
        before.units.push(turts_sim::UnitView {
            unit_id: 2,
            content_id: "fixture:walker".into(),
            owner_slot: 0,
            pos: WorldPos { x: 8.0, y: 8.0 },
        });
        let order = turts_sim::Order {
            order: turts_mod_api::ContentId::parse("fixture:move").unwrap(),
            behaviour: turts_mod_api::Behaviour::Move,
            target: turts_sim::OrderTargetSim::Point(bevy_math::Vec2::new(8.0, 8.0)),
            group: 1,
        };
        let mut after = before.clone();
        after.unit_orders = vec![turts_sim::economy::views::RawUnitOrdersAudit {
            unit_id: 1,
            orders: turts_sim::UnitOrders {
                current: Some(order),
                ..Default::default()
            },
        }];
        after.raw_units[0].move_target = Some(WorldPos { x: 8.0, y: 8.0 });
        let command = one_order_command(
            "fixture:move",
            turts_mod_api::Behaviour::Move,
            turts_mod_api::TargetKind::Point,
            turts_sim::OrderTargetSim::Point(bevy_math::Vec2::new(8.0, 8.0)),
            order_policy(
                "fixture:move",
                turts_mod_api::Behaviour::Move,
                turts_mod_api::TargetKind::Point,
                vec![(
                    "fixture:walker",
                    true,
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                )],
            ),
        );
        let records = single_order_records(
            before.clone(),
            after.clone(),
            command,
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        before = match &records[0] {
            EconomyLogRecord::Initial(state) => state.clone(),
            _ => unreachable!(),
        };
        assert_eq!(before.raw_units[0].pos, after.raw_units[0].pos);
        records
    }

    #[test]
    fn validator_rejects_a_move_order_that_teleports_its_actor() {
        let mut records = valid_point_move_records();
        assert!(
            validate_records(&records).is_ok(),
            "unmodified Move log must be a valid baseline"
        );
        if let EconomyLogRecord::Operation(operation) = &mut records[2] {
            operation.after.raw_units[0].pos = WorldPos { x: 7.0, y: 7.0 };
            operation.after.units[0].pos = WorldPos { x: 7.0, y: 7.0 };
        }
        if let EconomyLogRecord::CommandBoundary(command) = &mut records[1] {
            command.after.raw_units[0].pos = WorldPos { x: 7.0, y: 7.0 };
            command.after.units[0].pos = WorldPos { x: 7.0, y: 7.0 };
        }
        assert!(
            validate_records(&records)
                .unwrap_err()
                .contains("moved its actor before Movement"),
            "Orders phase must reject actor teleportation"
        );
    }

    #[test]
    fn validator_rejects_a_forged_automatic_orders_completion() {
        let order_id = turts_mod_api::ContentId::parse("fixture:move").unwrap();
        let mut before = one_moving_unit_state();
        before.catalog = Some(turts_sim::economy::views::CatalogAudit {
            resource_kinds: vec![],
            nodes: vec![],
            units: vec![],
            recipes: vec![],
            order_definitions: vec![turts_sim::economy::views::CatalogOrderAudit {
                id: order_id.clone(),
                behaviour: turts_mod_api::Behaviour::Move,
                target: turts_mod_api::TargetKind::Point,
                units: vec![],
                queue_depth: 16,
                unit_capabilities: vec![turts_sim::OrderUnitCapabilitiesAudit {
                    unit: "fixture:walker".into(),
                    movement: true,
                    weapon: false,
                    gatherer: false,
                    producer: false,
                    research: false,
                    removable: false,
                    toggleable: false,
                }],
            }],
            technologies: vec![],
            item_recipes: vec![],
        });
        before.unit_orders = vec![turts_sim::economy::views::RawUnitOrdersAudit {
            unit_id: 1,
            orders: turts_sim::UnitOrders {
                current: Some(turts_sim::Order {
                    order: order_id,
                    behaviour: turts_mod_api::Behaviour::Move,
                    target: turts_sim::OrderTargetSim::Point(bevy_math::Vec2::new(8.0, 8.0)),
                    group: 1,
                }),
                ..Default::default()
            },
        }];
        let mut after = before.clone();
        after.unit_orders.clear();
        let mut records = vec![
            EconomyLogRecord::Initial(before.clone()),
            EconomyLogRecord::Operation(turts_sim::OperationAudit {
                operation_id: 1,
                phase: turts_sim::economy::transaction::AuditPhase::Orders,
                unit_id: Some(1),
                before,
                after,
                events: vec![],
            }),
        ];
        assert!(
            validate_records(&records).is_ok(),
            "a Move completing after Movement is a valid automatic Orders transition"
        );
        let mut wrong_actor = records.clone();
        if let EconomyLogRecord::Operation(operation) = &mut wrong_actor[1] {
            operation.unit_id = Some(99);
        }
        assert!(
            validate_records(&wrong_actor)
                .unwrap_err()
                .contains("automatic Orders operation names absent unit 99"),
            "an automatic order operation must identify the unit it changes"
        );
        if let EconomyLogRecord::Operation(operation) = &mut records[1] {
            let forged_position = WorldPos { x: 7.0, y: 7.0 };
            operation.after.raw_units[0].pos = forged_position.clone();
            operation.after.units[0].pos = forged_position;
        }
        assert!(
            validate_records(&records)
                .unwrap_err()
                .contains("automatic Orders operation for unit 1 differs from its independently modeled transition"),
            "automatic Orders records cannot teleport an actor"
        );
    }

    #[test]
    fn validator_replays_automatic_queued_gather_activation() {
        use turts_sim::economy::views::{
            CatalogAudit, CatalogGathererAudit, CatalogNodeAudit, CatalogUnitAudit, InventoryView,
            ItemView, NodeView, OrderView, RawOrderAudit,
        };

        let order_id = turts_mod_api::ContentId::parse("fixture:gather").unwrap();
        let node_id = turts_sim::NodeId(7);
        let node_content = turts_mod_api::ContentId::parse("fixture:node").unwrap();
        let item = turts_mod_api::ContentId::parse("fixture:item").unwrap();
        let mut before = one_moving_unit_state();
        before.catalog = Some(CatalogAudit {
            resource_kinds: vec![],
            nodes: vec![CatalogNodeAudit {
                id: node_content.clone(),
                item: item.clone(),
                radius: 2.0,
            }],
            units: vec![CatalogUnitAudit {
                id: turts_mod_api::ContentId::parse("fixture:walker").unwrap(),
                radius: 0.5,
                movement_speed: Some(3.0),
                inventory: None,
                gatherer: Some(CatalogGathererAudit {
                    items: vec![item.clone()],
                    gather_per_second: 1,
                    unload_per_second: 1,
                    interaction_range: 1.0,
                }),
                drop_off: vec![],
                producer: None,
                research: None,
            }],
            recipes: vec![],
            order_definitions: vec![turts_sim::economy::views::CatalogOrderAudit {
                id: order_id.clone(),
                behaviour: turts_mod_api::Behaviour::Gather,
                target: turts_mod_api::TargetKind::Node,
                units: vec![],
                queue_depth: 16,
                unit_capabilities: vec![turts_sim::OrderUnitCapabilitiesAudit {
                    unit: "fixture:walker".into(),
                    movement: true,
                    weapon: false,
                    gatherer: true,
                    producer: false,
                    research: false,
                    removable: false,
                    toggleable: false,
                }],
            }],
            technologies: vec![],
            item_recipes: vec![],
        });
        before.nodes = vec![NodeView {
            id: node_id,
            content_id: node_content,
            pos: WorldPos { x: 5.0, y: 5.0 },
            remaining: 10,
        }];
        before.economy.inventories = vec![InventoryView {
            unit: 1,
            owner_slot: 0,
            capacity: 100,
            items: vec![ItemView {
                resource: item,
                on_hand: 0,
                reserved: 0,
            }],
        }];
        before.raw_orders = vec![RawOrderAudit {
            unit: 1,
            node: None,
            deposit: None,
            receiver: None,
            stage: turts_sim::economy::gather::GatherStage::Idle,
            rate_carry: 0,
        }];
        before.economy.orders = vec![OrderView {
            unit: 1,
            owner_slot: 0,
            node: None,
            receiver: None,
            stage: turts_sim::economy::gather::GatherStage::Idle,
        }];
        before.unit_orders = vec![turts_sim::economy::views::RawUnitOrdersAudit {
            unit_id: 1,
            orders: turts_sim::UnitOrders {
                queue: std::collections::VecDeque::from([turts_sim::Order {
                    order: order_id.clone(),
                    behaviour: turts_mod_api::Behaviour::Gather,
                    target: turts_sim::OrderTargetSim::Node(node_id),
                    group: 1,
                }]),
                ..Default::default()
            },
        }];
        let mut after = before.clone();
        after.unit_orders[0].orders = turts_sim::UnitOrders {
            current: Some(turts_sim::Order {
                order: order_id,
                behaviour: turts_mod_api::Behaviour::Gather,
                target: turts_sim::OrderTargetSim::Node(node_id),
                group: 1,
            }),
            ..Default::default()
        };
        after.raw_orders[0].node = Some(node_id);
        after.raw_orders[0].deposit = Some(node_id);
        after.raw_orders[0].stage = turts_sim::economy::gather::GatherStage::MovingToNode;
        after.economy.orders[0].node = Some(node_id);
        after.economy.orders[0].stage = turts_sim::economy::gather::GatherStage::MovingToNode;
        let records = vec![
            EconomyLogRecord::Initial(before.clone()),
            EconomyLogRecord::Operation(turts_sim::OperationAudit {
                operation_id: 1,
                phase: turts_sim::economy::transaction::AuditPhase::Orders,
                unit_id: Some(1),
                before,
                after,
                events: vec![],
            }),
        ];
        assert!(
            validate_records(&records).is_ok(),
            "starting an accepted queued Gather must set both UnitOrders and M2 gather state"
        );
        let mut missing_gather_assignment = records.clone();
        if let EconomyLogRecord::Operation(operation) = &mut missing_gather_assignment[1] {
            operation.after.economy.orders[0].node = None;
            operation.after.economy.orders[0].stage = turts_sim::economy::gather::GatherStage::Idle;
        }
        assert!(
            validate_records(&missing_gather_assignment)
                .unwrap_err()
                .contains("independently modeled transition"),
            "automatic Gather activation must retain the matching M2 economy assignment"
        );
    }

    #[test]
    fn validator_accepts_real_follow_refresh_and_target_gone_queue_continuation() {
        let move_order = turts_mod_api::ContentId::parse("fixture:move").unwrap();
        let follow_records = automatic_move_log(
            turts_sim::UnitOrders {
                current: Some(turts_sim::Order {
                    order: move_order.clone(),
                    behaviour: turts_mod_api::Behaviour::Move,
                    target: turts_sim::OrderTargetSim::Unit(2),
                    group: 1,
                }),
                ..Default::default()
            },
            &[(2, bevy::math::Vec2::new(5.0, 2.0))],
        );
        assert_eq!(
            follow_records
                .iter()
                .filter(|record| matches!(record, EconomyLogRecord::Operation(_)))
                .count(),
            1,
            "the real run_orders system captures a follow-target refresh"
        );
        assert!(
            validate_records(&follow_records).is_ok(),
            "the actual follow-target operation and movement tick validate"
        );

        let queued_records = automatic_move_log(
            turts_sim::UnitOrders {
                current: Some(turts_sim::Order {
                    order: move_order.clone(),
                    behaviour: turts_mod_api::Behaviour::Move,
                    target: turts_sim::OrderTargetSim::Unit(99),
                    group: 1,
                }),
                queue: std::collections::VecDeque::from([turts_sim::Order {
                    order: move_order,
                    behaviour: turts_mod_api::Behaviour::Move,
                    target: turts_sim::OrderTargetSim::Point(bevy::math::Vec2::new(8.0, 2.0)),
                    group: 1,
                }]),
                ..Default::default()
            },
            &[],
        );
        assert_eq!(
            queued_records
                .iter()
                .filter(|record| matches!(record, EconomyLogRecord::Operation(_)))
                .count(),
            1,
            "the real run_orders system captures target-gone failure and next queued start"
        );
        assert!(
            validate_records(&queued_records).is_ok(),
            "the actual TargetGone failure, queue continuation and movement tick validate"
        );
    }

    #[test]
    fn validator_accepts_real_queued_gather_activation_and_depletion() {
        let records = actual_queued_gather_log();
        let order_operations = records
            .iter()
            .filter_map(|record| match record {
                EconomyLogRecord::Operation(operation)
                    if operation.phase == turts_sim::economy::transaction::AuditPhase::Orders =>
                {
                    Some(operation)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let final_tick = records.iter().rev().find_map(|record| match record {
            EconomyLogRecord::Tick { state, .. } => Some(state),
            _ => None,
        });
        let final_status = final_tick.map(|state| {
            (
                state.economy.tick,
                state
                    .raw_units
                    .first()
                    .map(|unit| (unit.pos.clone(), unit.move_target.clone())),
                state
                    .raw_orders
                    .first()
                    .map(|order| (order.stage, order.node)),
                state.nodes.first().map(|node| node.remaining),
            )
        });
        assert!(
            order_operations.len() >= 2,
            "real scheduler capture includes queued Gather activation and depletion: {final_status:?}"
        );
        assert!(
            order_operations.iter().any(|operation| {
                operation.after.unit_orders.iter().any(|entry| {
                    entry.unit_id == 1
                        && entry.orders.last_failure.as_ref().is_some_and(|failure| {
                            failure.reason == turts_sim::RejectReason::NodeDepleted
                        })
                })
            }),
            "the real depletion operation records the expected failure"
        );
        assert!(
            validate_records(&records).is_ok(),
            "actual queued Gather, M2 harvest, depletion and economy records validate"
        );
        let mut forged_gather = records.clone();
        let activation = forged_gather
            .iter_mut()
            .find_map(|record| match record {
                EconomyLogRecord::Operation(operation)
                    if operation.phase == turts_sim::economy::transaction::AuditPhase::Orders
                        && operation.after.unit_orders.iter().any(|entry| {
                            entry.unit_id == 1
                                && entry.orders.current.as_ref().is_some_and(|order| {
                                    order.behaviour == turts_mod_api::Behaviour::Gather
                                })
                        }) =>
                {
                    Some(operation)
                }
                _ => None,
            })
            .unwrap();
        activation.after.raw_orders[0].node = None;
        activation.after.raw_orders[0].deposit = None;
        activation.after.raw_orders[0].stage = turts_sim::economy::gather::GatherStage::Idle;
        activation.after.economy.orders[0].node = None;
        activation.after.economy.orders[0].stage = turts_sim::economy::gather::GatherStage::Idle;
        assert!(
            validate_records(&forged_gather)
                .unwrap_err()
                .contains("independently modeled transition"),
            "real Gather activation cannot omit its M2 gather assignment"
        );
    }

    #[test]
    fn validator_rejects_plain_move_actor_teleport() {
        let before = one_moving_unit_state();
        let point = bevy_math::Vec2::new(8.0, 8.0);
        let mut after = before.clone();
        after.raw_units[0].move_target = Some(WorldPos { x: 8.0, y: 8.0 });
        let mut command = one_order_command(
            "fixture:move",
            turts_mod_api::Behaviour::Move,
            turts_mod_api::TargetKind::Point,
            turts_sim::OrderTargetSim::Point(point),
            order_policy(
                "fixture:move",
                turts_mod_api::Behaviour::Move,
                turts_mod_api::TargetKind::Point,
                vec![(
                    "fixture:walker",
                    true,
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                )],
            ),
        );
        if let turts_sim::CommandKind::Order(order) = &mut command.kind {
            order.group = 0;
        }
        let valid = single_order_records(
            before.clone(),
            after.clone(),
            command,
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Command,
        );
        assert!(
            validate_records(&valid).is_ok(),
            "plain Move command-phase baseline must validate"
        );
        let mut forged = valid;
        let position = WorldPos { x: 7.0, y: 7.0 };
        if let EconomyLogRecord::CommandBoundary(boundary) = &mut forged[1] {
            boundary.after.raw_units[0].pos = position.clone();
            boundary.after.units[0].pos = position.clone();
        }
        if let EconomyLogRecord::Operation(operation) = &mut forged[2] {
            operation.after.raw_units[0].pos = position.clone();
            operation.after.units[0].pos = position;
        }
        assert!(
            validate_records(&forged)
                .unwrap_err()
                .contains("plain order for unit 1 moved its actor")
        );
    }

    #[test]
    fn validator_rejects_all_refused_order_results_with_the_wrong_reason() {
        let before = one_moving_unit_state();
        let command = one_order_command(
            "fixture:move",
            turts_mod_api::Behaviour::Move,
            turts_mod_api::TargetKind::Point,
            turts_sim::OrderTargetSim::Point(bevy_math::Vec2::new(8.0, 8.0)),
            order_policy(
                "fixture:move",
                turts_mod_api::Behaviour::Move,
                turts_mod_api::TargetKind::Point,
                vec![(
                    "fixture:walker",
                    true,
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                )],
            ),
        );
        let records = single_order_records(
            before.clone(),
            before,
            command,
            vec![],
            vec![(1, turts_sim::RejectReason::NotOwner)],
            turts_sim::economy::transaction::AuditPhase::Command,
        );
        assert!(
            validate_records(&records)
                .unwrap_err()
                .contains("all-refused order unit 1 has reason NotOwner"),
            "eligible own Move cannot be refused as NotOwner"
        );
    }

    #[test]
    fn validator_rejects_wrong_refused_subject_reason_in_mixed_envelope() {
        let mut before = one_moving_unit_state();
        before
            .raw_units
            .push(turts_sim::economy::views::RawUnitAudit {
                unit_id: 2,
                owner_slot: 1,
                unit_type: 0,
                pos: WorldPos { x: 6.0, y: 6.0 },
                move_target: None,
            });
        before.units.push(turts_sim::UnitView {
            unit_id: 2,
            content_id: "fixture:walker".into(),
            owner_slot: 1,
            pos: WorldPos { x: 6.0, y: 6.0 },
        });
        let mut after = before.clone();
        after.raw_units[0].move_target = Some(WorldPos { x: 8.0, y: 8.0 });
        after.unit_orders = vec![turts_sim::economy::views::RawUnitOrdersAudit {
            unit_id: 1,
            orders: turts_sim::UnitOrders {
                current: Some(turts_sim::Order {
                    order: turts_mod_api::ContentId::parse("fixture:move").unwrap(),
                    behaviour: turts_mod_api::Behaviour::Move,
                    target: turts_sim::OrderTargetSim::Point(bevy_math::Vec2::new(8.0, 8.0)),
                    group: 1,
                }),
                ..Default::default()
            },
        }];
        let command = one_order_command(
            "fixture:move",
            turts_mod_api::Behaviour::Move,
            turts_mod_api::TargetKind::Point,
            turts_sim::OrderTargetSim::Point(bevy_math::Vec2::new(8.0, 8.0)),
            order_policy(
                "fixture:move",
                turts_mod_api::Behaviour::Move,
                turts_mod_api::TargetKind::Point,
                vec![(
                    "fixture:walker",
                    true,
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                )],
            ),
        );
        let mut records = single_order_records(
            before,
            after,
            command,
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        if let EconomyLogRecord::CommandBoundary(boundary) = &mut records[1]
            && let turts_sim::CommandKind::Order(order) = &mut boundary.input.kind
        {
            order.units.push(2);
            boundary
                .result
                .units
                .refused
                .push((2, turts_sim::RejectReason::NotOwner));
        }
        validate_records(&records).unwrap_or_else(|error| panic!("{error}"));
        if let EconomyLogRecord::CommandBoundary(boundary) = &mut records[1] {
            boundary.result.units.refused[0].1 = turts_sim::RejectReason::MissingCapability;
        }
        let error = validate_records(&records).unwrap_err();
        assert!(
            error.contains("refusal for unit 2 differs from independent validation"),
            "{error}"
        );
    }

    #[test]
    fn validator_rejects_forged_order_target_policy() {
        let mut records = valid_point_move_records();
        assert!(
            validate_records(&records).is_ok(),
            "unmodified catalog-backed order must validate"
        );
        if let EconomyLogRecord::CommandBoundary(command) = &mut records[1]
            && let turts_sim::CommandKind::Order(order) = &mut command.input.kind
        {
            order.target_kind = turts_mod_api::TargetKind::Unit;
        }
        assert!(
            validate_records(&records)
                .unwrap_err()
                .contains("differs from its immutable catalog definition"),
            "command target kind must match captured catalog policy"
        );
    }

    #[test]
    fn validator_rejects_missing_or_duplicate_catalog_order_definitions() {
        fn alter_catalog(state: &mut AuditState, duplicate: bool) {
            let definitions = &mut state.catalog.as_mut().unwrap().order_definitions;
            if duplicate {
                definitions.push(definitions[0].clone());
            } else {
                definitions.clear();
            }
        }

        let mut missing = valid_point_move_records();
        assert!(
            validate_records(&missing).is_ok(),
            "baseline order audit must validate"
        );
        for record in &mut missing {
            match record {
                EconomyLogRecord::Initial(state) => alter_catalog(state, false),
                EconomyLogRecord::CommandBoundary(command) => {
                    alter_catalog(&mut command.before, false);
                    alter_catalog(&mut command.after, false);
                }
                EconomyLogRecord::Operation(operation) => {
                    alter_catalog(&mut operation.before, false);
                    alter_catalog(&mut operation.after, false);
                }
                _ => {}
            }
        }
        assert!(
            validate_records(&missing)
                .unwrap_err()
                .contains("missing or duplicate catalog definition"),
            "Order commands require their catalog definition"
        );

        let mut duplicate = valid_point_move_records();
        for record in &mut duplicate {
            match record {
                EconomyLogRecord::Initial(state) => alter_catalog(state, true),
                EconomyLogRecord::CommandBoundary(command) => {
                    alter_catalog(&mut command.before, true);
                    alter_catalog(&mut command.after, true);
                }
                EconomyLogRecord::Operation(operation) => {
                    alter_catalog(&mut operation.before, true);
                    alter_catalog(&mut operation.after, true);
                }
                _ => {}
            }
        }
        assert!(
            validate_records(&duplicate)
                .unwrap_err()
                .contains("missing or duplicate catalog definition"),
            "duplicate Order definitions must not create ambiguous authority"
        );
    }

    #[test]
    fn validator_rejects_jointly_forged_order_and_embedded_policy() {
        let mut records = valid_point_move_records();
        assert!(
            validate_records(&records).is_ok(),
            "unmodified catalog-backed order must validate"
        );
        if let EconomyLogRecord::CommandBoundary(command) = &mut records[1]
            && let turts_sim::CommandKind::Order(order) = &mut command.input.kind
        {
            order.target_kind = turts_mod_api::TargetKind::Unit;
            order.policy.as_mut().unwrap().target = turts_mod_api::TargetKind::Unit;
            order.target = turts_sim::OrderTargetSim::Unit(2);
        }
        if let EconomyLogRecord::CommandBoundary(command) = &mut records[1] {
            command.after.unit_orders[0]
                .orders
                .current
                .as_mut()
                .unwrap()
                .target = turts_sim::OrderTargetSim::Unit(2);
        }
        if let EconomyLogRecord::Operation(operation) = &mut records[2] {
            operation.after.unit_orders[0]
                .orders
                .current
                .as_mut()
                .unwrap()
                .target = turts_sim::OrderTargetSim::Unit(2);
        }
        let error = validate_records(&records).unwrap_err();
        assert!(
            error.contains("differs from its immutable catalog definition"),
            "{error}"
        );
    }

    #[test]
    fn validator_rejects_catalog_mutation_during_order_operation() {
        let mut records = valid_point_move_records();
        assert!(
            validate_records(&records).is_ok(),
            "unmodified catalog-backed order must validate"
        );
        if let EconomyLogRecord::Operation(operation) = &mut records[2] {
            operation.after.catalog.as_mut().unwrap().order_definitions[0].queue_depth = 17;
        }
        if let EconomyLogRecord::CommandBoundary(command) = &mut records[1] {
            command.after.catalog.as_mut().unwrap().order_definitions[0].queue_depth = 17;
        }
        let error = validate_records(&records).unwrap_err();
        assert!(
            error.contains("changed the immutable catalog snapshot"),
            "{error}"
        );
    }

    #[test]
    fn audit_catalog_captures_loaded_order_policy_and_package_queue_depth() {
        use std::collections::BTreeMap;
        use turts_mod_api::{
            ContentCatalog, ContentId, Footprint, MovementDef, OrderDef, PackageId, TargetKind,
        };
        let mut app = headless_app(20);
        let order_id = ContentId::parse("fixture:move").unwrap();
        let unit_id = ContentId::parse("fixture:walker").unwrap();
        let mut catalog = ContentCatalog {
            resources: BTreeMap::new(),
            units: BTreeMap::new(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: BTreeMap::from([(PackageId::parse("fixture").unwrap(), 3)]),
            source_files: BTreeMap::new(),
        };
        catalog.units.insert(
            unit_id.clone(),
            turts_mod_api::ResolvedUnit {
                id: unit_id.clone(),
                name: "Walker".into(),
                footprint: Footprint::Circle { radius: 0.5 },
                movement: Some(MovementDef { speed: 3.0 }),
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
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
        );
        catalog.orders.insert(
            order_id.clone(),
            OrderDef {
                id: order_id.local.clone(),
                name: "Move".into(),
                behaviour: turts_mod_api::Behaviour::Move,
                target: TargetKind::Point,
                fire: None,
                engage: None,
                icon: String::new(),
                hotkey: String::new(),
                cursor: None,
                card: None,
                units: vec![unit_id.to_string()],
                description: String::new(),
            },
        );
        app.world_mut()
            .insert_resource(turts_sim::bootstrap::InstalledCatalog(catalog));
        let audit = turts_sim::audit_state(app.world_mut());
        let captured = audit.catalog.unwrap().order_definitions;
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].id, order_id);
        assert_eq!(captured[0].behaviour, turts_mod_api::Behaviour::Move);
        assert_eq!(captured[0].target, TargetKind::Point);
        assert_eq!(captured[0].units, vec!["fixture:walker"]);
        assert_eq!(captured[0].queue_depth, 3);
        assert_eq!(captured[0].unit_capabilities[0].unit, "fixture:walker");
        assert!(captured[0].unit_capabilities[0].movement);
    }

    #[test]
    fn validator_rejects_stop_and_gather_with_incorrect_legacy_gather_state() {
        use turts_sim::economy::{gather::GatherStage, views::RawOrderAudit};
        let mut before = one_moving_unit_state();
        before.raw_orders = vec![RawOrderAudit {
            unit: 1,
            node: Some(turts_sim::NodeId(7)),
            deposit: Some(turts_sim::NodeId(7)),
            receiver: None,
            stage: GatherStage::MovingToNode,
            rate_carry: 0,
        }];
        before.economy.orders = vec![turts_sim::economy::views::OrderView {
            unit: 1,
            owner_slot: 0,
            node: Some(turts_sim::NodeId(7)),
            receiver: None,
            stage: GatherStage::MovingToNode,
        }];
        let mut stopped = before.clone();
        stopped.unit_orders.clear();
        stopped.raw_units[0].move_target = None;
        let stop = one_order_command(
            "fixture:stop",
            turts_mod_api::Behaviour::Stop,
            turts_mod_api::TargetKind::None,
            turts_sim::OrderTargetSim::None,
            order_policy(
                "fixture:stop",
                turts_mod_api::Behaviour::Stop,
                turts_mod_api::TargetKind::None,
                vec![(
                    "fixture:walker",
                    true,
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                )],
            ),
        );
        let stop_records = single_order_records(
            before.clone(),
            stopped,
            stop,
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );

        let mut gather_before = one_moving_unit_state();
        let node_content = turts_mod_api::ContentId::parse("fixture:node").unwrap();
        let item = turts_mod_api::ContentId::parse("fixture:item").unwrap();
        gather_before.catalog = Some(turts_sim::economy::views::CatalogAudit {
            resource_kinds: vec![],
            nodes: vec![turts_sim::economy::views::CatalogNodeAudit {
                id: node_content.clone(),
                item: item.clone(),
                radius: 2.0,
            }],
            units: vec![turts_sim::economy::views::CatalogUnitAudit {
                id: turts_mod_api::ContentId::parse("fixture:walker").unwrap(),
                radius: 0.5,
                movement_speed: Some(3.0),
                inventory: None,
                gatherer: Some(turts_sim::economy::views::CatalogGathererAudit {
                    items: vec![item],
                    gather_per_second: 1,
                    unload_per_second: 1,
                    interaction_range: 1.0,
                }),
                drop_off: vec![],
                producer: None,
                research: None,
            }],
            recipes: vec![],
            order_definitions: vec![],
            technologies: vec![],
            item_recipes: vec![],
        });
        gather_before
            .nodes
            .push(turts_sim::economy::views::NodeView {
                id: turts_sim::NodeId(7),
                content_id: node_content,
                pos: WorldPos { x: 20.0, y: 20.0 },
                remaining: 3,
            });
        gather_before.raw_orders = vec![RawOrderAudit {
            unit: 1,
            node: None,
            deposit: None,
            receiver: None,
            stage: GatherStage::Idle,
            rate_carry: 0,
        }];
        let mut gather_after = gather_before.clone();
        gather_after.unit_orders = vec![turts_sim::economy::views::RawUnitOrdersAudit {
            unit_id: 1,
            orders: turts_sim::UnitOrders {
                current: Some(turts_sim::Order {
                    order: turts_mod_api::ContentId::parse("fixture:gather").unwrap(),
                    behaviour: turts_mod_api::Behaviour::Gather,
                    target: turts_sim::OrderTargetSim::Node(turts_sim::NodeId(7)),
                    group: 1,
                }),
                ..Default::default()
            },
        }];
        let gather = one_order_command(
            "fixture:gather",
            turts_mod_api::Behaviour::Gather,
            turts_mod_api::TargetKind::Node,
            turts_sim::OrderTargetSim::Node(turts_sim::NodeId(7)),
            order_policy(
                "fixture:gather",
                turts_mod_api::Behaviour::Gather,
                turts_mod_api::TargetKind::Node,
                vec![(
                    "fixture:walker",
                    true,
                    false,
                    true,
                    false,
                    false,
                    false,
                    false,
                )],
            ),
        );
        let gather_records = single_order_records(
            gather_before,
            gather_after,
            gather,
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        assert!(
            validate_records(&stop_records)
                .unwrap_err()
                .contains("did not clear the M2 gather cycle"),
            "Stop must clear M2 EconomyOrders"
        );
        let gather_error = validate_records(&gather_records).unwrap_err();
        assert!(
            gather_error.contains("did not assign the M2 gather cycle"),
            "Gather must assign M2 EconomyOrders, got: {gather_error}"
        );
    }

    #[test]
    fn validator_rejects_fabricated_produce_cancel_and_set_enabled_operations() {
        let (before, recipe) = producer_order_state(false);
        let mut produced = before.clone();
        let queued_event = super::event_for_order(
            &before,
            1,
            turts_sim::EconomyEventKind::ProductionQueued {
                producer: 1,
                job: before.allocators.next_values().2,
                recipe: recipe.clone(),
            },
        )
        .unwrap();
        produced.pending_events.push(queued_event.clone());
        produced.next_event_ordinal = queued_event.ordinal.checked_add(1);
        let job_id = before.allocators.next_values().2;
        produced.raw_queues[0]
            .jobs
            .push(turts_sim::economy::views::RawJobAudit {
                id: job_id,
                recipe: recipe.clone(),
                stage: turts_sim::JobStage::Queued,
                elapsed_ticks: 0,
                duration_ticks: 20,
                charged: vec![],
                reservation: None,
            });
        produced.economy.queues[0].jobs.push(turts_sim::JobView {
            id: job_id,
            recipe: recipe.clone(),
            stage: turts_sim::JobStage::Queued,
            elapsed_ticks: 0,
            duration_ticks: 20,
            charged: vec![],
            reserved: vec![],
        });
        let (next_unit, next_node, _) = before.allocators.next_values();
        produced
            .allocators
            .set_next_values(next_unit, next_node, job_id + 1);
        let produce = one_order_command(
            "fixture:produce",
            turts_mod_api::Behaviour::Produce,
            turts_mod_api::TargetKind::Content,
            turts_sim::OrderTargetSim::Content(recipe.clone()),
            order_policy(
                "fixture:produce",
                turts_mod_api::Behaviour::Produce,
                turts_mod_api::TargetKind::Content,
                vec![(
                    "fixture:producer",
                    false,
                    false,
                    false,
                    true,
                    false,
                    false,
                    true,
                )],
            ),
        );
        let mut valid_produce_records = single_order_records(
            before.clone(),
            produced.clone(),
            produce.clone(),
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        if let EconomyLogRecord::Operation(operation) = &mut valid_produce_records[2] {
            operation.events = vec![queued_event.clone()];
        }
        assert!(
            validate_records(&valid_produce_records).is_ok(),
            "catalog-derived zero-cost Produce baseline must validate"
        );
        let mut wrong_job_id = valid_produce_records.clone();
        if let EconomyLogRecord::CommandBoundary(command) = &mut wrong_job_id[1]
            && let Ok(applied) = &mut command.result.outcome
        {
            applied.job_id = Some(turts_sim::JobId(job_id + 1));
        }
        assert!(
            validate_records(&wrong_job_id)
                .unwrap_err()
                .contains("aggregate JobId"),
            "accepted Produce must report the first catalog-derived JobId"
        );

        let (mut costed_before, recipe) = producer_order_state(false);
        let resource = turts_mod_api::ContentId::parse("fixture:coin").unwrap();
        let cost = turts_sim::CostView {
            source: turts_mod_api::CostSource::SlotBalance,
            resource: resource.clone(),
            amount: 5,
        };
        costed_before
            .catalog
            .as_mut()
            .unwrap()
            .resource_kinds
            .push((resource.clone(), turts_mod_api::ResourceKind::Balance));
        costed_before.catalog.as_mut().unwrap().recipes[0].costs = vec![cost.clone()];
        costed_before.economy.balances.push(turts_sim::BalanceView {
            slot: 0,
            resource: resource.clone(),
            on_hand: 100,
            reserved: 0,
        });
        let costed_job_id = costed_before.allocators.next_values().2;
        let costed_event = super::event_for_order(
            &costed_before,
            1,
            turts_sim::EconomyEventKind::ProductionQueued {
                producer: 1,
                job: costed_job_id,
                recipe: recipe.clone(),
            },
        )
        .unwrap();
        let charged_line = turts_sim::ReservationLineView {
            endpoint: turts_sim::ReservationEndpointView::Slot(0),
            resource: resource.clone(),
            amount: 5,
        };
        let mut costed_after = costed_before.clone();
        costed_after.pending_events.push(costed_event.clone());
        costed_after.next_event_ordinal = costed_event.ordinal.checked_add(1);
        costed_after.economy.balances[0].on_hand = 95;
        costed_after.raw_queues[0]
            .jobs
            .push(turts_sim::economy::views::RawJobAudit {
                id: costed_job_id,
                recipe: recipe.clone(),
                stage: turts_sim::JobStage::Queued,
                elapsed_ticks: 0,
                duration_ticks: 20,
                charged: vec![charged_line],
                reservation: None,
            });
        costed_after.economy.queues[0]
            .jobs
            .push(turts_sim::JobView {
                id: costed_job_id,
                recipe: recipe.clone(),
                stage: turts_sim::JobStage::Queued,
                elapsed_ticks: 0,
                duration_ticks: 20,
                charged: vec![cost],
                reserved: vec![],
            });
        let (next_unit, next_node, _) = costed_before.allocators.next_values();
        costed_after
            .allocators
            .set_next_values(next_unit, next_node, costed_job_id + 1);
        let costed_command = one_order_command(
            "fixture:produce",
            turts_mod_api::Behaviour::Produce,
            turts_mod_api::TargetKind::Content,
            turts_sim::OrderTargetSim::Content(recipe.clone()),
            order_policy(
                "fixture:produce",
                turts_mod_api::Behaviour::Produce,
                turts_mod_api::TargetKind::Content,
                vec![(
                    "fixture:producer",
                    false,
                    false,
                    false,
                    true,
                    false,
                    false,
                    true,
                )],
            ),
        );
        let mut valid_costed = single_order_records(
            costed_before.clone(),
            costed_after.clone(),
            costed_command.clone(),
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        if let EconomyLogRecord::Operation(operation) = &mut valid_costed[2] {
            operation.events = vec![costed_event.clone()];
        }
        assert!(
            validate_records(&valid_costed).is_ok(),
            "ChargeOnQueue baseline must debit exactly the declared cost"
        );
        let mut wrong_payment = costed_after;
        wrong_payment.economy.balances[0].on_hand = 100;
        wrong_payment.raw_queues[0].jobs[0].charged.clear();
        wrong_payment.economy.queues[0].jobs[0].charged.clear();
        let mut wrong_payment_records = single_order_records(
            costed_before,
            wrong_payment,
            costed_command,
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        if let EconomyLogRecord::Operation(operation) = &mut wrong_payment_records[2] {
            operation.events = vec![costed_event];
        }
        let error = validate_records(&wrong_payment_records).unwrap_err();
        assert!(
            error.contains("Produce changed state outside the catalog-derived job"),
            "{error}"
        );

        let mut missing_job = produced;
        missing_job.raw_queues[0].jobs.clear();
        missing_job.economy.queues[0].jobs.clear();
        missing_job
            .allocators
            .set_next_values(next_unit, next_node, job_id);
        let mut produce_records = single_order_records(
            before.clone(),
            missing_job,
            produce,
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        if let EconomyLogRecord::Operation(operation) = &mut produce_records[2] {
            operation.events = vec![queued_event];
        }
        let produce_error = validate_records(&produce_records).unwrap_err();
        assert!(
            produce_error.contains("Produce changed state outside the catalog-derived job"),
            "{produce_error}"
        );

        let (before, _recipe) = producer_order_state(true);
        let job = &before.raw_queues[0].jobs[0];
        let canceled_event = super::event_for_order(
            &before,
            1,
            turts_sim::EconomyEventKind::ProductionCancelled {
                producer: 1,
                job: 1,
                released: vec![],
                refunded: vec![],
            },
        )
        .unwrap();
        let mut canceled = before.clone();
        canceled.pending_events.push(canceled_event.clone());
        canceled.next_event_ordinal = canceled_event.ordinal.checked_add(1);
        canceled.raw_queues[0].jobs.clear();
        canceled.economy.queues[0].jobs.clear();
        let cancel = one_order_command(
            "fixture:cancel",
            turts_mod_api::Behaviour::CancelProduction,
            turts_mod_api::TargetKind::Job,
            turts_sim::OrderTargetSim::Job(turts_sim::JobId(job.id)),
            order_policy(
                "fixture:cancel",
                turts_mod_api::Behaviour::CancelProduction,
                turts_mod_api::TargetKind::Job,
                vec![(
                    "fixture:producer",
                    false,
                    false,
                    false,
                    true,
                    false,
                    false,
                    true,
                )],
            ),
        );
        let mut valid_cancel_records = single_order_records(
            before.clone(),
            canceled.clone(),
            cancel.clone(),
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        if let EconomyLogRecord::Operation(operation) = &mut valid_cancel_records[2] {
            operation.events = vec![canceled_event.clone()];
        }
        assert!(
            validate_records(&valid_cancel_records).is_ok(),
            "catalog-derived zero-cost Cancel baseline must validate"
        );

        let (mut costed_cancel_before, _recipe) = producer_order_state(true);
        let resource = turts_mod_api::ContentId::parse("fixture:coin").unwrap();
        let cost = turts_sim::CostView {
            source: turts_mod_api::CostSource::SlotBalance,
            resource: resource.clone(),
            amount: 5,
        };
        let charged = turts_sim::ReservationLineView {
            endpoint: turts_sim::ReservationEndpointView::Slot(0),
            resource: resource.clone(),
            amount: 5,
        };
        costed_cancel_before
            .catalog
            .as_mut()
            .unwrap()
            .resource_kinds
            .push((resource.clone(), turts_mod_api::ResourceKind::Balance));
        costed_cancel_before.catalog.as_mut().unwrap().recipes[0].costs = vec![cost.clone()];
        costed_cancel_before
            .economy
            .balances
            .push(turts_sim::BalanceView {
                slot: 0,
                resource: resource.clone(),
                on_hand: 95,
                reserved: 0,
            });
        costed_cancel_before.raw_queues[0].jobs[0].charged = vec![charged.clone()];
        costed_cancel_before.economy.queues[0].jobs[0].charged = vec![cost];
        let refunded_event = super::event_for_order(
            &costed_cancel_before,
            1,
            turts_sim::EconomyEventKind::ProductionCancelled {
                producer: 1,
                job: 1,
                released: vec![],
                refunded: vec![turts_sim::CostLine {
                    source: turts_sim::EventSource::Slot(0),
                    resource: resource.clone(),
                    amount: 5,
                }],
            },
        )
        .unwrap();
        let mut costed_canceled = costed_cancel_before.clone();
        costed_canceled.raw_queues[0].jobs.clear();
        costed_canceled.economy.queues[0].jobs.clear();
        costed_canceled.economy.balances[0].on_hand = 100;
        costed_canceled.pending_events.push(refunded_event.clone());
        costed_canceled.next_event_ordinal = refunded_event.ordinal.checked_add(1);
        let costed_cancel = one_order_command(
            "fixture:cancel",
            turts_mod_api::Behaviour::CancelProduction,
            turts_mod_api::TargetKind::Job,
            turts_sim::OrderTargetSim::Job(turts_sim::JobId(1)),
            order_policy(
                "fixture:cancel",
                turts_mod_api::Behaviour::CancelProduction,
                turts_mod_api::TargetKind::Job,
                vec![(
                    "fixture:producer",
                    false,
                    false,
                    false,
                    true,
                    false,
                    false,
                    true,
                )],
            ),
        );
        let mut costed_cancel_records = single_order_records(
            costed_cancel_before.clone(),
            costed_canceled.clone(),
            costed_cancel.clone(),
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        if let EconomyLogRecord::Operation(operation) = &mut costed_cancel_records[2] {
            operation.events = vec![refunded_event.clone()];
        }
        assert!(
            validate_records(&costed_cancel_records).is_ok(),
            "catalog-derived queued Cancel must refund charged cost"
        );
        let mut missing_refund = costed_canceled;
        missing_refund.economy.balances[0].on_hand = 95;
        let mut missing_refund_records = single_order_records(
            costed_cancel_before,
            missing_refund,
            costed_cancel,
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        if let EconomyLogRecord::Operation(operation) = &mut missing_refund_records[2] {
            operation.events = vec![refunded_event];
        }
        let refund_error = validate_records(&missing_refund_records).unwrap_err();
        assert!(
            refund_error.contains("Cancel changed state outside requested job removal, refund"),
            "Cancel must credit catalog-derived refunds: {refund_error}"
        );

        let mut cancel_records = single_order_records(
            before.clone(),
            {
                let mut missing_cancel = before.clone();
                missing_cancel.pending_events.push(canceled_event.clone());
                missing_cancel.next_event_ordinal = canceled_event.ordinal.checked_add(1);
                missing_cancel
            },
            cancel,
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        if let EconomyLogRecord::Operation(operation) = &mut cancel_records[2] {
            operation.events = vec![canceled_event];
        }
        let cancel_error = validate_records(&cancel_records).unwrap_err();
        assert!(
            cancel_error.contains("Cancel changed state outside requested job removal"),
            "{cancel_error}"
        );

        let mut before = one_moving_unit_state();
        before.unit_types[0].content_id = "fixture:machine".to_owned();
        let disable = one_order_command(
            "fixture:disable",
            turts_mod_api::Behaviour::SetEnabled(false),
            turts_mod_api::TargetKind::None,
            turts_sim::OrderTargetSim::None,
            order_policy(
                "fixture:disable",
                turts_mod_api::Behaviour::SetEnabled(false),
                turts_mod_api::TargetKind::None,
                vec![(
                    "fixture:machine",
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                    true,
                )],
            ),
        );
        let records = single_order_records(
            before.clone(),
            before.clone(),
            disable,
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        let error = validate_records(&records).unwrap_err();
        assert!(
            error.contains("SetEnabled for unit 1 has the wrong state-change event"),
            "{error}"
        );

        let idempotent = one_order_command(
            "fixture:enable",
            turts_mod_api::Behaviour::SetEnabled(true),
            turts_mod_api::TargetKind::None,
            turts_sim::OrderTargetSim::None,
            order_policy(
                "fixture:enable",
                turts_mod_api::Behaviour::SetEnabled(true),
                turts_mod_api::TargetKind::None,
                vec![(
                    "fixture:machine",
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                    true,
                )],
            ),
        );
        let records = single_order_records(
            before.clone(),
            before,
            idempotent,
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        assert!(
            validate_records(&records).is_ok(),
            "already-enabled idempotent SetEnabled must validate without an event"
        );
    }

    #[test]
    fn validator_accepts_zero_cost_reserve_then_charge_produce_and_cancel() {
        let (mut before, recipe) = producer_order_state(false);
        before.catalog.as_mut().unwrap().recipes[0].payment =
            turts_mod_api::PaymentPolicy::ReserveThenCharge;
        let job_id = before.allocators.next_values().2;
        let queued_event = super::event_for_order(
            &before,
            1,
            turts_sim::EconomyEventKind::ProductionQueued {
                producer: 1,
                job: job_id,
                recipe: recipe.clone(),
            },
        )
        .unwrap();
        let mut produced = before.clone();
        produced.pending_events.push(queued_event.clone());
        produced.next_event_ordinal = queued_event.ordinal.checked_add(1);
        produced.raw_queues[0]
            .jobs
            .push(turts_sim::economy::views::RawJobAudit {
                id: job_id,
                recipe: recipe.clone(),
                stage: turts_sim::JobStage::Queued,
                elapsed_ticks: 0,
                duration_ticks: 20,
                charged: vec![],
                reservation: Some(turts_sim::economy::views::RawReservationAudit {
                    job_id,
                    lines: vec![],
                }),
            });
        produced.economy.queues[0].jobs.push(turts_sim::JobView {
            id: job_id,
            recipe: recipe.clone(),
            stage: turts_sim::JobStage::Queued,
            elapsed_ticks: 0,
            duration_ticks: 20,
            charged: vec![],
            reserved: vec![],
        });
        produced
            .ledger_claims
            .push(turts_sim::ReservationClaimView::Active {
                map_job_id: job_id,
                token_job_id: job_id,
                lines: vec![],
            });
        let (next_unit, next_node, _) = before.allocators.next_values();
        produced
            .allocators
            .set_next_values(next_unit, next_node, job_id + 1);
        let produce = one_order_command(
            "fixture:produce",
            turts_mod_api::Behaviour::Produce,
            turts_mod_api::TargetKind::Content,
            turts_sim::OrderTargetSim::Content(recipe.clone()),
            order_policy(
                "fixture:produce",
                turts_mod_api::Behaviour::Produce,
                turts_mod_api::TargetKind::Content,
                vec![(
                    "fixture:producer",
                    false,
                    false,
                    false,
                    true,
                    false,
                    false,
                    true,
                )],
            ),
        );
        let mut produce_records = single_order_records(
            before.clone(),
            produced.clone(),
            produce,
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        if let EconomyLogRecord::Operation(operation) = &mut produce_records[2] {
            operation.events = vec![queued_event];
        }
        assert!(
            validate_records(&produce_records).is_ok(),
            "ReserveThenCharge with no cost lines still captures Some(empty) reservation"
        );

        let canceled_event = super::event_for_order(
            &produced,
            1,
            turts_sim::EconomyEventKind::ProductionCancelled {
                producer: 1,
                job: job_id,
                released: vec![],
                refunded: vec![],
            },
        )
        .unwrap();
        let mut canceled = produced.clone();
        canceled.raw_queues[0].jobs.clear();
        canceled.economy.queues[0].jobs.clear();
        canceled.ledger_claims[0] = turts_sim::ReservationClaimView::Closed { map_job_id: job_id };
        canceled.pending_events.push(canceled_event.clone());
        canceled.next_event_ordinal = canceled_event.ordinal.checked_add(1);
        let cancel = one_order_command(
            "fixture:cancel",
            turts_mod_api::Behaviour::CancelProduction,
            turts_mod_api::TargetKind::Job,
            turts_sim::OrderTargetSim::Job(turts_sim::JobId(job_id)),
            order_policy(
                "fixture:cancel",
                turts_mod_api::Behaviour::CancelProduction,
                turts_mod_api::TargetKind::Job,
                vec![(
                    "fixture:producer",
                    false,
                    false,
                    false,
                    true,
                    false,
                    false,
                    true,
                )],
            ),
        );
        let mut cancel_records = single_order_records(
            produced.clone(),
            canceled.clone(),
            cancel.clone(),
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        if let EconomyLogRecord::Operation(operation) = &mut cancel_records[2] {
            operation.events = vec![canceled_event.clone()];
        }
        assert!(
            validate_records(&cancel_records).is_ok(),
            "Cancel closes an empty reservation claim even though it releases no account lines"
        );

        let mut leaked_claim = canceled;
        leaked_claim.ledger_claims[0] = produced.ledger_claims[0].clone();
        let mut leaked_claim_records = single_order_records(
            produced,
            leaked_claim,
            cancel,
            vec![1],
            vec![],
            turts_sim::economy::transaction::AuditPhase::Orders,
        );
        if let EconomyLogRecord::Operation(operation) = &mut leaked_claim_records[2] {
            operation.events = vec![canceled_event];
        }
        assert!(
            validate_records(&leaked_claim_records)
                .unwrap_err()
                .contains("reservation release and event"),
            "leaving the empty claim active after Cancel is rejected"
        );
    }

    #[test]
    fn validator_replays_a_real_order_move_operation_by_subject() {
        use turts_mod_api::{Footprint, PackageId, UnitDef, UnitRegistry};
        use turts_sim::{
            Command, CommandKind, CommandQueue, OrderCommand, OrderTargetSim, QueueMode,
            UnitOrders, UnitTypes,
        };
        let mut app = headless_app(20);
        let kind = app
            .world_mut()
            .resource_mut::<UnitTypes>()
            .register_unit_type(
                &PackageId("test".into()),
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
        let unit_id = turts_mod_api::ContentId::parse("test:walker").unwrap();
        let order_id = turts_mod_api::ContentId::parse("test:move").unwrap();
        let mut catalog = turts_mod_api::ContentCatalog {
            resources: std::collections::BTreeMap::new(),
            units: std::collections::BTreeMap::new(),
            nodes: std::collections::BTreeMap::new(),
            production: std::collections::BTreeMap::new(),
            recipes: std::collections::BTreeMap::new(),
            technologies: std::collections::BTreeMap::new(),
            orders: std::collections::BTreeMap::new(),
            order_queue_depths: std::collections::BTreeMap::new(),
            source_files: std::collections::BTreeMap::new(),
        };
        catalog.units.insert(
            unit_id.clone(),
            turts_mod_api::ResolvedUnit {
                id: unit_id,
                name: "Walker".into(),
                footprint: Footprint::Circle { radius: 0.5 },
                movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
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
        );
        catalog.orders.insert(
            order_id.clone(),
            turts_mod_api::OrderDef {
                id: order_id.local.clone(),
                name: "Move".into(),
                behaviour: turts_mod_api::Behaviour::Move,
                target: turts_mod_api::TargetKind::Point,
                fire: None,
                engage: None,
                icon: String::new(),
                hotkey: String::new(),
                cursor: None,
                card: None,
                units: vec![],
                description: String::new(),
            },
        );
        app.world_mut()
            .insert_resource(turts_sim::bootstrap::InstalledCatalog(catalog));
        app.world_mut().spawn((
            turts_sim::UnitId(1),
            turts_sim::UnitType(kind),
            turts_sim::Owner(turts_sim::SlotId(0)),
            turts_sim::Position(bevy_math::Vec2::new(2.0, 2.0)),
            UnitOrders::default(),
        ));
        app.world_mut().resource_mut::<AuditTrail>().enable();
        let initial_state = turts_sim::audit_state(app.world_mut());
        let command = Command {
            slot: turts_sim::SlotId(0),
            seq: 1,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: turts_mod_api::ContentId::parse("test:move").unwrap(),
                behaviour: turts_mod_api::Behaviour::Move,
                units: vec![1],
                target_kind: turts_mod_api::TargetKind::Point,
                policy: Some(turts_sim::OrderPolicyAudit {
                    id: turts_mod_api::ContentId::parse("test:move").unwrap(),
                    behaviour: turts_mod_api::Behaviour::Move,
                    target: turts_mod_api::TargetKind::Point,
                    units: vec![],
                    queue_depth: 16,
                    unit_capabilities: vec![turts_sim::OrderUnitCapabilitiesAudit {
                        unit: "test:walker".into(),
                        movement: true,
                        weapon: false,
                        gatherer: false,
                        producer: false,
                        research: false,
                        removable: false,
                        toggleable: false,
                    }],
                }),
                target: OrderTargetSim::Point(bevy_math::Vec2::new(12.0, 12.0)),
                queue: QueueMode::Replace,
                group: 1,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        app.world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .push_back(command);
        turts_sim::drain_commands(app.world_mut());
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        let commands = trail.drain_commands();
        let operations = trail.drain_operations();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].operation_end_id, None);
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].unit_id, Some(1));
        let mut records = vec![
            EconomyLogRecord::Initial(initial_state),
            EconomyLogRecord::CommandBoundary(commands[0].clone()),
            EconomyLogRecord::Operation(operations[0].clone()),
        ];
        assert!(validate_records(&records).is_ok());
        if let EconomyLogRecord::Operation(operation) = &mut records[2] {
            operation.unit_id = Some(99);
        }
        assert!(
            validate_records(&records)
                .unwrap_err()
                .contains("outside its envelope")
        );
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
    fn audit_rejects_state_change_between_operations_for_new_tables() {
        use turts_sim::economy::views_m3::RawBeltAudit;
        let previous_operation_after = initial();
        let mut next_operation_before = previous_operation_after.clone();
        next_operation_before.raw_belts.push(RawBeltAudit {
            first_piece: 91,
            definition: turts_mod_api::ContentId::parse("fixture:belt").unwrap(),
            owner_slot: 0,
            slots_per_piece: 1,
            slots_per_second: 1,
            rate_carry: 0,
            raw_slots: Vec::new(),
            pieces: Vec::new(),
        });
        let error =
            validate_operation_continuity(&previous_operation_after, &next_operation_before, false)
                .unwrap_err();
        assert!(error.contains("discontinuous"));
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
