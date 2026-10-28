//! Pure JSON command-trace parsing and deterministic step scheduling.
//!
//! This preparation module is intentionally unwired. The host drives it from Welcome,
//! coherent ticks and CommandResults, and reports PlayerCommand envelopes only after the
//! central sender has actually allocated and sent them.

use std::collections::BTreeMap;
use std::path::Path;

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use turts_net_protocol::{JobId, PlayerCommand, PlayerCommandKind, UnitId};

use crate::ingest::CommandResult;

/// State-triggered single Produce test hook. The coordinator may observe the watermark
/// in the native live report, while the opponent receives only its own replica.
#[derive(Resource, Debug, Default)]
pub(crate) struct ProduceHook {
    pub(crate) request: Option<(u32, String)>,
    pub(crate) inventory_gate: Option<(u32, String, u64)>,
    pub(crate) sent: bool,
    pub(crate) research_completed_tick: Option<u32>,
}

impl ProduceHook {
    pub(crate) fn from_options(options: &crate::options::ClientOptions) -> Self {
        Self {
            request: options.produce.clone(),
            inventory_gate: options.produce_when_inventory.clone(),
            ..Default::default()
        }
    }
}

/// Return the one command only after a coherent owner replica proves its precondition.
pub(crate) fn produce_command_if_ready(
    replica: &crate::replica::Replica,
    owner_slot: u8,
    request: &(u32, String),
    inventory_gate: Option<&(u32, String, u64)>,
) -> Option<PlayerCommandKind> {
    replica.last_applied_tick?;
    let (producer, technology) = request;
    if replica.units.get(producer)?.owner_slot != owner_slot {
        return None;
    }
    if let Some((gate_unit, item, required)) = inventory_gate {
        if gate_unit != producer {
            return None;
        }
        let available = replica
            .economy
            .inventories
            .iter()
            .find(|inventory| {
                inventory.unit.0 == *gate_unit
                    && inventory.port == turts_net_protocol::InventoryPort::Main
            })?
            .items
            .iter()
            .find(|balance| balance.resource == *item)?
            .on_hand
            .checked_sub(
                replica
                    .economy
                    .inventories
                    .iter()
                    .find(|inventory| {
                        inventory.unit.0 == *gate_unit
                            && inventory.port == turts_net_protocol::InventoryPort::Main
                    })?
                    .items
                    .iter()
                    .find(|balance| balance.resource == *item)?
                    .reserved,
            )?;
        if available < *required {
            return None;
        }
    }
    Some(PlayerCommandKind::Produce {
        producer_id: turts_net_protocol::UnitId(*producer),
        recipe_id: technology.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TraceStep {
    pub(crate) after_ticks: u32,
    pub(crate) command: TraceCommand,
    pub(crate) require_result: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "command",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum TraceCommand {
    Submit(PlayerCommandKind),
    CancelAppliedJob {
        producer_id: UnitId,
        produced_by_step: usize,
    },
    RepeatLastEnvelope,
    ArmDropWorldChanges {
        kinds: Vec<DropWorldChange>,
    },
}

/// Task-local names shared with the later ingest wiring; these are predicates over deltas,
/// not protocol messages or a second player command language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DropWorldChange {
    Extraction,
    Conversion,
    Charge,
    Refund,
    Spawn,
    FinalChange,
}

pub(crate) fn parse_drop_world_changes(text: &str) -> Result<Vec<DropWorldChange>, String> {
    let mut kinds = Vec::new();
    for name in text.split(',') {
        let kind = match name {
            "extraction" => DropWorldChange::Extraction,
            "conversion" => DropWorldChange::Conversion,
            "charge" => DropWorldChange::Charge,
            "refund" => DropWorldChange::Refund,
            "spawn" => DropWorldChange::Spawn,
            "final_change" => DropWorldChange::FinalChange,
            _ => return Err(format!("unknown WorldDelta change kind `{name}`")),
        };
        if kinds.contains(&kind) {
            return Err(format!("duplicate WorldDelta change kind `{name}`"));
        }
        kinds.push(kind);
    }
    if kinds.is_empty() {
        return Err("at least one WorldDelta change kind is required".into());
    }
    Ok(kinds)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TraceBinding {
    pub(crate) step_index: usize,
    pub(crate) action: TraceCommand,
    pub(crate) envelope: Option<PlayerCommand>,
    pub(crate) result_cursor: Option<usize>,
    pub(crate) result: Option<CommandResult>,
    pub(crate) arm_tick: Option<u32>,
}

#[derive(Resource, Debug, Default)]
pub(crate) struct ScenarioState {
    steps: Vec<TraceStep>,
    driver: Option<TraceDriver>,
    pub(crate) enabled: bool,
    pub(crate) error: Option<String>,
    pub(crate) bindings: Vec<TraceBinding>,
}

impl ScenarioState {
    pub(crate) fn from_path(path: Option<&Path>) -> Self {
        let Some(path) = path else {
            return Self::default();
        };
        let json = match std::fs::read_to_string(path) {
            Ok(json) => json,
            Err(error) => {
                return Self {
                    enabled: true,
                    error: Some(format!(
                        "cannot read command trace {}: {error}",
                        path.display()
                    )),
                    ..Self::default()
                };
            }
        };
        match parse_trace(&json) {
            Ok(steps) => Self {
                steps,
                enabled: true,
                ..Self::default()
            },
            Err(error) => Self {
                enabled: true,
                error: Some(format!("invalid command trace {}: {error}", path.display())),
                ..Self::default()
            },
        }
    }

    pub(crate) fn record_action(&mut self, step_index: usize, action: TraceCommand) {
        if self
            .bindings
            .iter()
            .any(|binding| binding.step_index == step_index)
        {
            return;
        }
        self.bindings.push(TraceBinding {
            step_index,
            action,
            envelope: None,
            result_cursor: None,
            result: None,
            arm_tick: None,
        });
    }

    pub(crate) fn record_trace_action(&mut self, step_index: usize) {
        if let Some(step) = self.steps.get(step_index) {
            self.record_action(step_index, step.command.clone());
        }
    }

    pub(crate) fn record_arm(&mut self, step_index: usize, action: TraceCommand, tick: u32) {
        self.record_action(step_index, action);
        if let Some(binding) = self
            .bindings
            .iter_mut()
            .find(|b| b.step_index == step_index)
        {
            binding.arm_tick = Some(tick);
        }
    }

    pub(crate) fn record_sent(
        &mut self,
        step_index: usize,
        envelope: PlayerCommand,
        result_cursor: usize,
    ) -> Result<(), String> {
        let Some(binding) = self
            .bindings
            .iter_mut()
            .find(|b| b.step_index == step_index)
        else {
            return Err(format!("trace step {step_index} has no report binding"));
        };
        let envelope_for_driver = envelope.clone();
        binding.envelope = Some(envelope);
        binding.result_cursor = Some(result_cursor);
        self.driver
            .as_mut()
            .ok_or_else(|| {
                "trace sender reported an envelope before the driver started".to_owned()
            })?
            .record_sent(step_index, envelope_for_driver, result_cursor)?;
        Ok(())
    }

    pub(crate) fn observe_results(&mut self, results: &[CommandResult]) {
        for binding in &mut self.bindings {
            if binding.result.is_some() {
                continue;
            }
            let (Some(envelope), Some(cursor)) = (&binding.envelope, binding.result_cursor) else {
                continue;
            };
            binding.result = results
                .iter()
                .enumerate()
                .skip(cursor.min(results.len()))
                .find_map(|(_, result)| match result {
                    CommandResult::Applied { seq, .. } | CommandResult::Rejected { seq, .. }
                        if *seq == envelope.seq =>
                    {
                        Some(result.clone())
                    }
                    CommandResult::Acked { .. }
                    | CommandResult::Applied { .. }
                    | CommandResult::Rejected { .. } => None,
                });
        }
    }

    pub(crate) fn is_complete(&self) -> bool {
        !self.enabled || self.driver.as_ref().is_some_and(TraceDriver::is_complete)
    }

    pub(crate) fn drive(
        &mut self,
        tick: u32,
        welcome_tick: u32,
        results: &[CommandResult],
    ) -> Result<Vec<TraceAction>, String> {
        if !self.enabled || self.error.is_some() {
            return Ok(Vec::new());
        }
        let driver = self.driver.get_or_insert_with(|| {
            TraceDriver::new(self.steps.clone(), welcome_tick)
                .expect("trace validated during option loading")
        });
        driver.update(tick, results)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TraceAction {
    Submit {
        step_index: usize,
        kind: PlayerCommandKind,
    },
    RepeatLastEnvelope {
        step_index: usize,
        envelope: PlayerCommand,
    },
    ArmDropWorldChanges {
        step_index: usize,
        arm_tick: u32,
        kinds: Vec<DropWorldChange>,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum StepOutcome {
    AppliedJob { producer_id: UnitId, job_id: JobId },
    AppliedWithoutJob,
    Rejected,
}

#[derive(Debug, Clone, PartialEq)]
struct SentEnvelope {
    envelope: PlayerCommand,
    result_cursor: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum IssuedAction {
    Submit {
        step_index: usize,
        kind: PlayerCommandKind,
    },
    Repeat {
        step_index: usize,
        envelope: PlayerCommand,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TraceDriver {
    steps: Vec<TraceStep>,
    step_index: usize,
    base_tick: u32,
    issued: Option<IssuedAction>,
    awaiting_result: Option<(usize, u32, usize)>,
    last_sent: Option<SentEnvelope>,
    last_sent_terminal: bool,
    outcomes: BTreeMap<usize, StepOutcome>,
    result_cursor: usize,
}

pub(crate) fn parse_trace(json: &str) -> Result<Vec<TraceStep>, String> {
    let steps: Vec<TraceStep> = serde_json::from_str(json).map_err(|error| error.to_string())?;
    validate_steps(&steps)?;
    Ok(steps)
}

impl TraceDriver {
    pub(crate) fn new(steps: Vec<TraceStep>, welcome_tick: u32) -> Result<Self, String> {
        validate_steps(&steps)?;
        Ok(Self {
            steps,
            step_index: 0,
            base_tick: welcome_tick,
            issued: None,
            awaiting_result: None,
            last_sent: None,
            last_sent_terminal: false,
            outcomes: BTreeMap::new(),
            result_cursor: 0,
        })
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.step_index == self.steps.len()
    }

    /// Produce the next host action(s) for this coherent tick. `results` is a cumulative
    /// CommandResults snapshot; the cursor prevents an old same-sequence result from
    /// satisfying a later RepeatLastEnvelope step.
    pub(crate) fn update(
        &mut self,
        tick: u32,
        results: &[CommandResult],
    ) -> Result<Vec<TraceAction>, String> {
        let cursor = if results.len() < self.result_cursor {
            0
        } else {
            self.result_cursor
        };
        let mut matching_result = None;
        let start = cursor.min(results.len());
        for (result_index, result) in results.iter().enumerate().skip(start) {
            let terminal = match result {
                CommandResult::Applied { seq, tick, job_id } => Some((*seq, *tick, Some(*job_id))),
                CommandResult::Rejected { seq, tick, .. } => Some((*seq, *tick, None)),
                CommandResult::Acked { .. } => None,
            };
            let Some((seq, result_tick, job_id)) = terminal else {
                continue;
            };
            if self
                .last_sent
                .as_ref()
                .is_some_and(|sent| sent.envelope.seq == seq && result_index >= sent.result_cursor)
            {
                self.last_sent_terminal = true;
            }
            if let Some((step_index, expected_seq, after_send_cursor)) = self.awaiting_result
                && seq == expected_seq
                && result_index >= after_send_cursor
            {
                matching_result = Some((step_index, result_tick, job_id));
                break;
            }
        }
        self.result_cursor = results.len();

        if let Some((step_index, result_tick, job_id)) = matching_result {
            let step = &self.steps[step_index];
            let outcome = match (&step.command, job_id) {
                (
                    TraceCommand::Submit(PlayerCommandKind::Produce { producer_id, .. }),
                    Some(Some(job_id)),
                ) => StepOutcome::AppliedJob {
                    producer_id: *producer_id,
                    job_id,
                },
                (_, Some(_)) => StepOutcome::AppliedWithoutJob,
                (_, None) => StepOutcome::Rejected,
            };
            self.outcomes.insert(step_index, outcome);
            self.awaiting_result = None;
            self.base_tick = result_tick;
            self.step_index = step_index + 1;
        }

        if self.awaiting_result.is_some() || self.issued.is_some() {
            return Ok(Vec::new());
        }

        let mut actions = Vec::new();
        loop {
            let Some(step) = self.steps.get(self.step_index) else {
                return Ok(actions);
            };
            let due_tick = self
                .base_tick
                .checked_add(step.after_ticks)
                .ok_or_else(|| format!("trace step {} tick offset overflows", self.step_index))?;
            if tick < due_tick {
                return Ok(actions);
            }
            let index = self.step_index;
            match &step.command {
                TraceCommand::ArmDropWorldChanges { kinds } => {
                    actions.push(TraceAction::ArmDropWorldChanges {
                        step_index: index,
                        arm_tick: tick,
                        kinds: kinds.clone(),
                    });
                    self.base_tick = tick;
                    self.step_index += 1;
                }
                TraceCommand::Submit(kind) => {
                    let kind = kind.clone();
                    actions.push(TraceAction::Submit {
                        step_index: index,
                        kind: kind.clone(),
                    });
                    self.issued = Some(IssuedAction::Submit {
                        step_index: index,
                        kind,
                    });
                    return Ok(actions);
                }
                TraceCommand::CancelAppliedJob {
                    producer_id,
                    produced_by_step,
                } => {
                    let job_id = match self.outcomes.get(produced_by_step) {
                        Some(StepOutcome::AppliedJob {
                            producer_id: produced_producer,
                            job_id,
                        }) if produced_producer == producer_id => *job_id,
                        Some(StepOutcome::Rejected) => {
                            return Err(format!(
                                "trace step {index} references rejected production step {produced_by_step}"
                            ));
                        }
                        Some(StepOutcome::AppliedWithoutJob) => {
                            return Err(format!(
                                "trace step {index} references production step {produced_by_step} without a JobId"
                            ));
                        }
                        Some(StepOutcome::AppliedJob { .. }) => {
                            return Err(format!(
                                "trace step {index} producer differs from referenced step {produced_by_step}"
                            ));
                        }
                        None => {
                            return Err(format!(
                                "trace step {index} references production step {produced_by_step} without an Applied result"
                            ));
                        }
                    };
                    let kind = PlayerCommandKind::CancelProduction {
                        producer_id: *producer_id,
                        job_id,
                    };
                    actions.push(TraceAction::Submit {
                        step_index: index,
                        kind: kind.clone(),
                    });
                    self.issued = Some(IssuedAction::Submit {
                        step_index: index,
                        kind,
                    });
                    return Ok(actions);
                }
                TraceCommand::RepeatLastEnvelope => {
                    let Some(sent) = self.last_sent.as_ref() else {
                        return Err(format!(
                            "trace step {index} repeats before any actual envelope was sent"
                        ));
                    };
                    if !self.last_sent_terminal {
                        return Ok(actions);
                    }
                    let envelope = sent.envelope.clone();
                    actions.push(TraceAction::RepeatLastEnvelope {
                        step_index: index,
                        envelope: envelope.clone(),
                    });
                    self.issued = Some(IssuedAction::Repeat {
                        step_index: index,
                        envelope,
                    });
                    return Ok(actions);
                }
            }
        }
    }

    /// Record only the exact envelope observed at the central sender after allocation and send.
    /// The driver never predicts a sequence number or converts a queued body into a sent envelope.
    pub(crate) fn record_sent(
        &mut self,
        step_index: usize,
        envelope: PlayerCommand,
        result_cursor_after_send: usize,
    ) -> Result<(), String> {
        let issued = self.issued.take().ok_or_else(|| {
            "trace sender reported an envelope without a pending action".to_owned()
        })?;
        let (expected_step, require_result) = match &issued {
            IssuedAction::Submit {
                step_index: expected_step,
                kind,
            } => {
                if envelope.kind != *kind {
                    self.issued = Some(issued);
                    return Err(format!(
                        "trace step {step_index} sent a command different from its frozen input"
                    ));
                }
                (*expected_step, self.steps[*expected_step].require_result)
            }
            IssuedAction::Repeat {
                step_index: expected_step,
                envelope: expected_envelope,
            } => {
                if envelope != *expected_envelope {
                    self.issued = Some(issued);
                    return Err(format!(
                        "trace step {step_index} changed the repeated PlayerCommand envelope"
                    ));
                }
                (*expected_step, self.steps[*expected_step].require_result)
            }
        };
        if step_index != expected_step || step_index != self.step_index {
            self.issued = Some(issued);
            return Err(format!(
                "trace sender step {step_index} does not match pending step {expected_step}"
            ));
        }
        self.last_sent = Some(SentEnvelope {
            envelope: envelope.clone(),
            result_cursor: result_cursor_after_send,
        });
        self.last_sent_terminal = false;
        if require_result {
            self.awaiting_result = Some((step_index, envelope.seq, result_cursor_after_send));
        } else {
            self.step_index += 1;
        }
        Ok(())
    }
}

fn validate_steps(steps: &[TraceStep]) -> Result<(), String> {
    if steps.is_empty() {
        return Err("command trace must contain at least one step".into());
    }
    for (index, step) in steps.iter().enumerate() {
        match &step.command {
            TraceCommand::CancelAppliedJob {
                producer_id,
                produced_by_step,
            } => {
                let Some(produced) = steps.get(*produced_by_step) else {
                    return Err(format!(
                        "trace step {index} references missing production step {produced_by_step}"
                    ));
                };
                let is_matching_produce = matches!(
                    &produced.command,
                    TraceCommand::Submit(PlayerCommandKind::Produce {
                        producer_id: submitted_producer,
                        ..
                    }) if submitted_producer == producer_id
                );
                if *produced_by_step >= index || !produced.require_result || !is_matching_produce {
                    return Err(format!(
                        "trace step {index} must reference an earlier required Produce for the same producer"
                    ));
                }
            }
            TraceCommand::ArmDropWorldChanges { kinds } => {
                if step.require_result || kinds.is_empty() {
                    return Err(format!(
                        "trace arm step {index} must have kinds and require_result=false"
                    ));
                }
                let unique = kinds
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>();
                if unique.len() != kinds.len() {
                    return Err(format!("trace arm step {index} has duplicate change kinds"));
                }
                let Some(next) = steps.get(index + 1) else {
                    return Err(format!("trace arm step {index} has no following action"));
                };
                if next.after_ticks != 0
                    || !matches!(
                        next.command,
                        TraceCommand::Submit(_) | TraceCommand::CancelAppliedJob { .. }
                    )
                {
                    return Err(format!(
                        "trace arm step {index} must be followed by a zero-offset submit or cancel"
                    ));
                }
            }
            TraceCommand::Submit(_) | TraceCommand::RepeatLastEnvelope => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod produce_hook_tests {
    use std::collections::VecDeque;

    use super::produce_command_if_ready;
    use crate::replica::{Replica, UnitReplica};
    use turts_net_protocol::{
        BalanceState, EconomyState, InventoryPort, InventoryState, PlayerCommandKind, UnitId,
    };

    #[test]
    fn scenario_waits_for_replica_state() {
        let request = (7, "forge:hardened_shells".to_owned());
        let gate = (7, "forge:shell".to_owned(), 10);
        let mut replica = Replica::new(20);
        replica.units.insert(
            7,
            UnitReplica {
                content_id: "forge:proving_lab".into(),
                owner_slot: 0,
                facing: None,
                samples: VecDeque::new(),
            },
        );
        replica.economy = EconomyState {
            balances: vec![],
            inventories: vec![InventoryState {
                unit: UnitId(7),
                port: InventoryPort::Main,
                items: vec![BalanceState {
                    resource: "forge:shell".into(),
                    on_hand: 10,
                    reserved: 0,
                }],
            }],
            orders: vec![],
            queues: vec![],
        };
        assert!(produce_command_if_ready(&replica, 0, &request, Some(&gate)).is_none());
        replica.last_applied_tick = Some(41);
        assert_eq!(
            produce_command_if_ready(&replica, 0, &request, Some(&gate)),
            Some(PlayerCommandKind::Produce {
                producer_id: UnitId(7),
                recipe_id: "forge:hardened_shells".into()
            })
        );
        assert!(produce_command_if_ready(&replica, 1, &request, Some(&gate)).is_none());
    }
}
