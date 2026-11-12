//! Economy log audit.
//!
//! Two validators run side by side until task A1c of the pre-MP1 slice deletes the second:
//! - the ledger audit (`validate_ledger`): record shape, refusals, and one file for each ledger
//!   domain. Each rule holds on the log alone, whatever the movement model does. No rule
//!   computes a decision of the simulation again (no path, no target choice, no timing model);
//! - the replay validator (`replay.rs`, `validate_records`), which models the simulation again.
//!
//! `validate_side_by_side` runs the ledger audit first and returns its error first.
//!
//! The ledger audit reads each snapshot only through `turts_sim::LedgerAudit`, the ledger view of
//! `AuditState` (`turts_sim/src/economy/ledger_view.rs`).
//!
//! Rules of this file:
//! - shape: the log has exactly one `Initial` record, and it is the first record.
//! - shape: a command boundary result names the slot and the seq of its input.
//! - shape: each command boundary id has exactly one operation record, which starts from the
//!   boundary snapshot (a Command-phase operation also ends on it).
//! - shape: an operation has the Command phase only when a command boundary names its id.
//! - shape: the operations of an order span are consecutive Orders-phase operations.
//! - catalog: the catalog snapshot never changes.
//! - refusal: each operation of a refused command leaves the full logged view unchanged.
//! - refusal: the operation of a refused unit in an order span leaves the full logged view
//!   unchanged.
//! - refusal: a wire-only rejection (`Admission`) leaves the full logged view unchanged.

mod balances;
mod clock;
mod combat;
mod format;
mod industry;
mod inventories;
mod production;
mod replay;
mod reservations;
mod slots;

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader};

pub use format::EconomyLogRecord;
pub use replay::validate_records;
pub use turts_sim::{LedgerAudit, ledger_audit};

use turts_sim::{
    AuditState, CommandAudit, EconomyEvent, OperationAudit, economy::transaction::AuditPhase,
};

/// Test hook: with `TURTS_AUDIT_REPLAY=off`, `validate_side_by_side` runs the ledger audit only.
/// A fault case uses it to show that the ledger audit, and not the replay validator, finds a
/// fault. Task A1c deletes it with the replay validator.
pub const REPLAY_SWITCH: &str = "TURTS_AUDIT_REPLAY";

/// Decode a JSONL economy log and validate it with both validators.
pub fn read_jsonl<R: BufRead>(input: R) -> Result<Vec<EconomyLogRecord>, String> {
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
    validate_side_by_side(&records)?;
    Ok(records)
}

pub fn read_jsonl_file(path: impl AsRef<std::path::Path>) -> Result<Vec<EconomyLogRecord>, String> {
    let file = std::fs::File::open(path.as_ref())
        .map_err(|error| format!("open {}: {error}", path.as_ref().display()))?;
    read_jsonl(BufReader::new(file))
}

/// Run the ledger audit, then the replay validator. The first error is returned.
pub fn validate_side_by_side(records: &[EconomyLogRecord]) -> Result<(), String> {
    validate_ledger(records)?;
    if std::env::var(REPLAY_SWITCH).is_ok_and(|value| value == "off") {
        return Ok(());
    }
    validate_records(records)
}

/// One state change that the domains check: an operation, the gap between two records, or the
/// end of a tick (the gap from the last record of the tick to the `Tick` state).
pub(crate) struct Step<'a> {
    pub(crate) kind: StepKind,
    pub(crate) before: &'a LedgerAudit,
    pub(crate) after: &'a LedgerAudit,
    /// The events of an operation. Empty for a gap and for the end of a tick.
    pub(crate) events: &'a [EconomyEvent],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepKind {
    Operation(u64),
    Gap,
    TickEnd(u32),
}

impl Step<'_> {
    pub(crate) fn label(&self) -> String {
        match self.kind {
            StepKind::Operation(id) => format!("operation {id}"),
            StepKind::Gap => format!("the gap at clock {}", self.after.economy.tick),
            StepKind::TickEnd(tick) => format!("the end of tick {tick}"),
        }
    }
}

/// The state of the domains between records.
struct Domains {
    clock: clock::Clock,
    slots: slots::Slots,
    reservations: reservations::Reservations,
    production: production::Production,
}

impl Domains {
    fn step(&mut self, step: &Step<'_>, initial: &LedgerAudit) -> Result<(), String> {
        self.clock.check(step)?;
        self.slots.check(step)?;
        combat::check(step)?;
        balances::check(step)?;
        self.reservations.check(step)?;
        self.production.check(step, initial)?;
        industry::check(step, initial)?;
        inventories::check(step, initial)?;
        Ok(())
    }
}

struct OrderSpan {
    end_id: u64,
    refused_units: BTreeSet<u32>,
    refused_command: bool,
}

fn refused_command(command: &CommandAudit) -> bool {
    command.result.units.accepted.is_empty()
        && (command.result.outcome.is_err()
            || matches!(command.input.kind, turts_sim::CommandKind::Order(_)))
}

/// The ledger audit of one complete economy log.
///
/// A snapshot that is equal to the previous snapshot is not converted again, and a step with no
/// change and no events is checked by the clock only: every domain rule holds on such a step.
pub fn validate_ledger(records: &[EconomyLogRecord]) -> Result<(), String> {
    let Some(EconomyLogRecord::Initial(initial_raw)) = records.first() else {
        return Err("first economy log record must be Initial".into());
    };
    if records
        .iter()
        .skip(1)
        .any(|record| matches!(record, EconomyLogRecord::Initial(_)))
    {
        return Err("economy log contains more than one Initial record".into());
    }
    let initial = ledger_audit(initial_raw);
    let mut domains = Domains {
        clock: clock::Clock::new(&initial),
        slots: slots::Slots::new(&initial),
        reservations: reservations::Reservations::default(),
        production: production::Production::default(),
    };
    let mut cursor = initial.clone();
    let mut cursor_raw: &AuditState = initial_raw;
    let mut commands: BTreeMap<u64, &CommandAudit> = BTreeMap::new();
    let mut span: Option<OrderSpan> = None;

    for record in records.iter().skip(1) {
        match record {
            EconomyLogRecord::Initial(_) => unreachable!("the Initial count is checked above"),
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
                let id = operation.operation_id;
                if cursor_raw != &operation.before {
                    let before = ledger_audit(&operation.before);
                    check_catalog(&initial, &before, id)?;
                    domains.step(
                        &Step {
                            kind: StepKind::Gap,
                            before: &cursor,
                            after: &before,
                            events: &[],
                        },
                        &initial,
                    )?;
                    cursor = before;
                }
                cursor_raw = &operation.before;
                let command = commands.remove(&id);
                check_span(operation, command, &mut span)?;
                if operation.before == operation.after {
                    let unchanged = Step {
                        kind: StepKind::Operation(id),
                        before: &cursor,
                        after: &cursor,
                        events: &operation.events,
                    };
                    if operation.events.is_empty() {
                        domains.clock.check(&unchanged)?;
                    } else {
                        domains.step(&unchanged, &initial)?;
                    }
                } else {
                    let after = ledger_audit(&operation.after);
                    check_catalog(&initial, &after, id)?;
                    check_refusal(operation, command, span.as_ref(), &cursor, &after)?;
                    domains.step(
                        &Step {
                            kind: StepKind::Operation(id),
                            before: &cursor,
                            after: &after,
                            events: &operation.events,
                        },
                        &initial,
                    )?;
                    cursor = after;
                    cursor_raw = &operation.after;
                }
                if span.as_ref().is_some_and(|open| open.end_id == id) {
                    span = None;
                }
            }
            EconomyLogRecord::Claim {
                tick,
                player_id,
                slot,
            } => {
                domains
                    .slots
                    .claim(*tick, *player_id, *slot, cursor.economy.tick)?;
            }
            EconomyLogRecord::Admission {
                ordinal,
                player_id,
                slot,
                tick,
                wire_format,
                raw_envelope,
                before,
                after,
                ..
            } => {
                slots::check_admission(
                    *ordinal,
                    wire_format,
                    raw_envelope,
                    *tick,
                    before.economy.tick,
                )?;
                let before_view = ledger_audit(before);
                if before_view != ledger_audit(after) {
                    return Err(format!(
                        "wire-only rejection for player {player_id}, slot {slot}, tick {tick} changed audited state"
                    ));
                }
                if cursor_raw != before {
                    check_catalog(&initial, &before_view, u64::from(*tick))?;
                    domains.step(
                        &Step {
                            kind: StepKind::Gap,
                            before: &cursor,
                            after: &before_view,
                            events: &[],
                        },
                        &initial,
                    )?;
                    cursor = before_view;
                }
                cursor_raw = after;
            }
            EconomyLogRecord::Tick {
                tick,
                before,
                state,
                events,
            } => {
                if span.is_some() {
                    return Err("order command operation span is incomplete".into());
                }
                let state_view = ledger_audit(state);
                check_catalog(&initial, &state_view, u64::from(*tick))?;
                domains
                    .clock
                    .tick(*tick, before.economy.tick, &state_view, events)?;
                domains.step(
                    &Step {
                        kind: StepKind::TickEnd(*tick),
                        before: &cursor,
                        after: &state_view,
                        events: &[],
                    },
                    &initial,
                )?;
                cursor = state_view;
                cursor_raw = state;
            }
        }
    }
    if let Some(operation_id) = commands.keys().next() {
        return Err(format!(
            "command operation id {operation_id} has no matching operation"
        ));
    }
    if span.is_some() {
        return Err("order command operation span is incomplete".into());
    }
    Ok(())
}

fn check_catalog(initial: &LedgerAudit, state: &LedgerAudit, id: u64) -> Result<(), String> {
    if state.catalog != initial.catalog {
        return Err(format!(
            "catalog: record {id} changed the immutable catalog snapshot"
        ));
    }
    Ok(())
}

fn check_span(
    operation: &OperationAudit,
    command: Option<&CommandAudit>,
    span: &mut Option<OrderSpan>,
) -> Result<(), String> {
    let id = operation.operation_id;
    if let Some(open) = span.as_ref() {
        if command.is_some() || operation.phase != AuditPhase::Orders || id > open.end_id {
            return Err(format!(
                "order command operation span ended unexpectedly at {id}"
            ));
        }
        return Ok(());
    }
    let Some(command) = command else {
        if operation.phase == AuditPhase::Command {
            return Err(format!(
                "operation {id} has Command phase without a command"
            ));
        }
        return Ok(());
    };
    if command.before != operation.before {
        return Err(format!(
            "command boundary {id} does not start from its operation snapshot"
        ));
    }
    match operation.phase {
        AuditPhase::Command => {
            if command.operation_end_id.is_some() || command.after != operation.after {
                return Err(format!("command/operation state mismatch for id {id}"));
            }
        }
        AuditPhase::Orders => {
            let end_id = command.operation_end_id.unwrap_or(id);
            if end_id < id {
                return Err(format!(
                    "order command {id} has an invalid operation span start"
                ));
            }
            *span = Some(OrderSpan {
                end_id,
                refused_units: command
                    .result
                    .units
                    .refused
                    .iter()
                    .map(|(unit, _)| *unit)
                    .collect(),
                refused_command: refused_command(command),
            });
        }
        AuditPhase::Other | AuditPhase::PostMovementOrders => {
            return Err(format!(
                "command operation {id} has the {:?} phase",
                operation.phase
            ));
        }
    }
    Ok(())
}

fn check_refusal(
    operation: &OperationAudit,
    command: Option<&CommandAudit>,
    span: Option<&OrderSpan>,
    before: &LedgerAudit,
    after: &LedgerAudit,
) -> Result<(), String> {
    if before == after {
        return Ok(());
    }
    if let Some(command) = command
        && operation.phase == AuditPhase::Command
        && refused_command(command)
    {
        return Err(format!(
            "refused command of slot {} seq {} changed the logged view in operation {}",
            command.input.slot.0, command.input.seq, operation.operation_id
        ));
    }
    if let Some(open) = span {
        if let Some(unit) = operation.unit_id
            && open.refused_units.contains(&unit)
        {
            return Err(format!(
                "refused unit {unit} changed state inside its order operation"
            ));
        }
        if open.refused_command {
            return Err(format!(
                "refused order command changed the logged view in operation {}",
                operation.operation_id
            ));
        }
    }
    Ok(())
}

/// Add `amount` to `map[key]`, as a signed total.
pub(crate) fn add<K: Ord>(map: &mut BTreeMap<K, i128>, key: K, amount: i128) {
    *map.entry(key).or_default() += amount;
}

/// The keys where two signed maps differ, with both values (a missing key is zero).
pub(crate) fn differences<K: Ord + Clone>(
    actual: &BTreeMap<K, i128>,
    expected: &BTreeMap<K, i128>,
) -> Vec<(K, i128, i128)> {
    let keys: BTreeSet<&K> = actual.keys().chain(expected.keys()).collect();
    keys.into_iter()
        .filter_map(|key| {
            let left = actual.get(key).copied().unwrap_or(0);
            let right = expected.get(key).copied().unwrap_or(0);
            (left != right).then(|| (key.clone(), left, right))
        })
        .collect()
}

/// `after - before` for each key of two signed maps. Zero entries are removed.
pub(crate) fn delta<K: Ord + Clone>(
    before: &BTreeMap<K, i128>,
    after: &BTreeMap<K, i128>,
) -> BTreeMap<K, i128> {
    let mut result = after.clone();
    for (key, value) in before {
        *result.entry(key.clone()).or_default() -= value;
    }
    result.retain(|_, value| *value != 0);
    result
}

/// The charge lines of every job, by job id (`raw_queues`).
pub(crate) fn job_charges(state: &LedgerAudit) -> BTreeMap<u64, &[turts_sim::ReservationLineView]> {
    state
        .raw_queues
        .iter()
        .flat_map(|queue| &queue.jobs)
        .map(|job| (job.id, job.charged.as_slice()))
        .collect()
}

/// The job ids that are in `before` and not in `after` (a job that ended in the step).
pub(crate) fn ended_jobs(before: &LedgerAudit, after: &LedgerAudit) -> BTreeSet<u64> {
    let after_jobs: BTreeSet<u64> = after
        .raw_queues
        .iter()
        .flat_map(|queue| queue.jobs.iter().map(|job| job.id))
        .collect();
    before
        .raw_queues
        .iter()
        .flat_map(|queue| queue.jobs.iter().map(|job| job.id))
        .filter(|id| !after_jobs.contains(id))
        .collect()
}

/// The charge increase of each job in a step, as signed amounts by (endpoint, resource).
/// A job that is new in `after` charged all its lines in the step. A job that ended in the step
/// charged nothing.
pub(crate) fn new_charges(
    before: &LedgerAudit,
    after: &LedgerAudit,
) -> Vec<(
    turts_sim::ReservationEndpointView,
    turts_mod_api::ContentId,
    i128,
)> {
    let old = job_charges(before);
    let mut result = Vec::new();
    for (job, lines) in job_charges(after) {
        let mut previous: BTreeMap<(u8, u32, &turts_mod_api::ContentId), i128> = BTreeMap::new();
        for line in old.get(&job).copied().unwrap_or_default() {
            add(
                &mut previous,
                (
                    endpoint_key(&line.endpoint),
                    endpoint_unit(&line.endpoint),
                    &line.resource,
                ),
                i128::from(line.amount),
            );
        }
        let mut current: BTreeMap<(u8, u32, &turts_mod_api::ContentId), i128> = BTreeMap::new();
        for line in lines {
            add(
                &mut current,
                (
                    endpoint_key(&line.endpoint),
                    endpoint_unit(&line.endpoint),
                    &line.resource,
                ),
                i128::from(line.amount),
            );
        }
        for ((kind, unit, resource), amount) in delta(&previous, &current) {
            result.push((endpoint_of(kind, unit), resource.clone(), amount));
        }
    }
    result
}

pub(crate) fn endpoint_key(endpoint: &turts_sim::ReservationEndpointView) -> u8 {
    match endpoint {
        turts_sim::ReservationEndpointView::Slot(_) => 0,
        turts_sim::ReservationEndpointView::Inventory(_) => 1,
        turts_sim::ReservationEndpointView::Output(_) => 2,
    }
}

pub(crate) fn endpoint_unit(endpoint: &turts_sim::ReservationEndpointView) -> u32 {
    match endpoint {
        turts_sim::ReservationEndpointView::Slot(slot) => u32::from(*slot),
        turts_sim::ReservationEndpointView::Inventory(unit)
        | turts_sim::ReservationEndpointView::Output(unit) => *unit,
    }
}

fn endpoint_of(kind: u8, unit: u32) -> turts_sim::ReservationEndpointView {
    match kind {
        0 => turts_sim::ReservationEndpointView::Slot(unit as u8),
        1 => turts_sim::ReservationEndpointView::Inventory(unit),
        _ => turts_sim::ReservationEndpointView::Output(unit),
    }
}
