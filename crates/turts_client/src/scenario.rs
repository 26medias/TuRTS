//! Pure JSON command-trace parsing and deterministic step scheduling.
//!
//! This preparation module is intentionally unwired. The host drives it from Welcome,
//! coherent ticks and CommandResults, and reports PlayerCommand envelopes only after the
//! central sender has actually allocated and sent them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use turts_net_protocol::{JobId, PlayerCommand, PlayerCommandKind, UnitId};

use crate::ingest::CommandResult;

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
    last_sent: Option<PlayerCommand>,
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
        if let Some((step_index, expected_seq, after_send_cursor)) = self.awaiting_result {
            let result_start = cursor.max(after_send_cursor).min(results.len());
            for result in &results[result_start..] {
                match result {
                    CommandResult::Applied { seq, tick, job_id } if *seq == expected_seq => {
                        matching_result = Some((step_index, *tick, Some(*job_id)));
                        break;
                    }
                    CommandResult::Rejected { seq, tick, .. } if *seq == expected_seq => {
                        matching_result = Some((step_index, *tick, None));
                        break;
                    }
                    CommandResult::Acked { .. }
                    | CommandResult::Applied { .. }
                    | CommandResult::Rejected { .. } => {}
                }
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
                    let envelope = self.last_sent.clone().ok_or_else(|| {
                        format!("trace step {index} repeats before any actual envelope was sent")
                    })?;
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
        if matches!(issued, IssuedAction::Submit { .. }) {
            self.last_sent = Some(envelope.clone());
        }
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
