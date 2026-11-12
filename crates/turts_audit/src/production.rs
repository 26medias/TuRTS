//! Production domain: production, machine and research jobs (`raw_queues`), and spawned units.
//!
//! Rules:
//! - production: a job enters a queue only in an operation with a `ProductionQueued` or a
//!   `ProductionStarted` event for that job.
//! - production: a job leaves its queue only in an operation with an end event for that job
//!   (`ProductionCompleted`, `ProductionCancelled`, `ItemsProduced` or `ResearchCompleted`), or a
//!   `Destroyed` event for its producer.
//! - production: a job has at most one `ProductionStarted` event in the log.
//! - production: a job never goes back to an earlier stage, and its elapsed ticks never decrease
//!   and never pass its duration.
//! - production: a cancel refunds, for each charge line of the job, the charged amount times the
//!   catalog refund ratio of the job stage, rounded down.
//! - production: a unit enters the world only in an operation with a `ProductionCompleted` event
//!   that names it.

use std::collections::{BTreeMap, BTreeSet};

use turts_mod_api::RefundRatio;
use turts_sim::{EconomyEventKind, JobStage};

use crate::{LedgerAudit, Step};

#[derive(Default)]
pub(crate) struct Production {
    started: BTreeSet<u64>,
}

fn stage_rank(stage: JobStage) -> u8 {
    match stage {
        JobStage::Queued => 0,
        JobStage::Running => 1,
        JobStage::OutputBlocked => 2,
    }
}

/// The refund ratio of a job stage, from the catalog of the log.
fn refund_ratio(
    catalog: &LedgerAudit,
    recipe: &turts_mod_api::ContentId,
    stage: JobStage,
) -> Option<RefundRatio> {
    let catalog = catalog.catalog.as_ref()?;
    let by_stage = |refunds: &turts_mod_api::CancelRefunds| match stage {
        JobStage::Queued => refunds.queued,
        JobStage::Running => refunds.running,
        JobStage::OutputBlocked => refunds.output_blocked,
    };
    if let Some(row) = catalog.recipes.iter().find(|row| &row.id == recipe) {
        return Some(by_stage(&row.refunds));
    }
    if let Some(row) = catalog.technologies.iter().find(|row| &row.id == recipe) {
        return Some(by_stage(&row.refunds));
    }
    let row = catalog.item_recipes.iter().find(|row| &row.id == recipe)?;
    match stage {
        JobStage::Queued => None,
        JobStage::Running => Some(row.refunds.running),
        JobStage::OutputBlocked => Some(row.refunds.output_blocked),
    }
}

impl Production {
    pub(crate) fn check(&mut self, step: &Step<'_>, initial: &LedgerAudit) -> Result<(), String> {
        let label = step.label();
        let jobs =
            |state: &LedgerAudit| -> BTreeMap<u64, (u32, turts_sim::economy::views::RawJobAudit)> {
                state
                    .raw_queues
                    .iter()
                    .flat_map(|queue| {
                        queue
                            .jobs
                            .iter()
                            .map(|job| (job.id, (queue.producer, job.clone())))
                    })
                    .collect()
            };
        let before = jobs(step.before);
        let after = jobs(step.after);
        let names_job = |job: u64, start: bool| {
            step.events.iter().any(|event| match &event.kind {
                EconomyEventKind::ProductionQueued { job: id, .. }
                | EconomyEventKind::ProductionStarted { job: id, .. } => start && *id == job,
                EconomyEventKind::ProductionCompleted { job: id, .. }
                | EconomyEventKind::ProductionCancelled { job: id, .. }
                | EconomyEventKind::ItemsProduced { job: id, .. }
                | EconomyEventKind::ResearchCompleted { job: id, .. } => !start && *id == job,
                _ => false,
            })
        };
        for (job, (producer, row)) in &after {
            match before.get(job) {
                None => {
                    if !names_job(*job, true) {
                        return Err(format!(
                            "production: job {job} entered the queue of producer {producer} with no event in {label}"
                        ));
                    }
                }
                Some((_, old)) => {
                    if stage_rank(row.stage) < stage_rank(old.stage)
                        || row.elapsed_ticks < old.elapsed_ticks
                    {
                        return Err(format!(
                            "production: job {job} went back from {:?} at {} ticks to {:?} at {} ticks in {label}",
                            old.stage, old.elapsed_ticks, row.stage, row.elapsed_ticks
                        ));
                    }
                }
            }
            if row.elapsed_ticks > row.duration_ticks {
                return Err(format!(
                    "production: job {job} is at {} ticks of a duration of {} in {label}",
                    row.elapsed_ticks, row.duration_ticks
                ));
            }
        }
        for (job, (producer, _)) in &before {
            if after.contains_key(job) {
                continue;
            }
            let destroyed = step.events.iter().any(|event| {
                matches!(&event.kind, EconomyEventKind::Destroyed { unit, .. } if unit == producer)
            });
            if !names_job(*job, false) && !destroyed {
                return Err(format!(
                    "production: job {job} left the queue of producer {producer} with no end event in {label}"
                ));
            }
        }
        for event in step.events {
            match &event.kind {
                EconomyEventKind::ProductionStarted { job, .. } => {
                    if !self.started.insert(*job) {
                        return Err(format!(
                            "production: job {job} has more than one ProductionStarted event ({label})"
                        ));
                    }
                }
                EconomyEventKind::ProductionCancelled { job, refunded, .. } => {
                    let Some((_, row)) = before.get(job) else {
                        return Err(format!(
                            "production: cancelled job {job} is not in a queue before {label}"
                        ));
                    };
                    let mut expected = Vec::new();
                    if !row.charged.is_empty() {
                        let ratio = refund_ratio(initial, &row.recipe, row.stage).ok_or_else(|| {
                            format!(
                                "production: the catalog has no refund ratio for {} at {:?} ({label})",
                                row.recipe, row.stage
                            )
                        })?;
                        if ratio.denominator == 0 {
                            return Err(format!(
                                "production: the refund ratio of {} has a zero denominator",
                                row.recipe
                            ));
                        }
                        for line in &row.charged {
                            let amount = u128::from(line.amount) * u128::from(ratio.numerator)
                                / u128::from(ratio.denominator);
                            expected.push((
                                crate::endpoint_key(&line.endpoint),
                                crate::endpoint_unit(&line.endpoint),
                                line.resource.clone(),
                                amount,
                            ));
                        }
                    }
                    let mut actual: Vec<_> = refunded
                        .iter()
                        .map(|line| {
                            let (kind, unit) = match line.source {
                                turts_sim::EventSource::Slot(slot) => (0, u32::from(slot)),
                                turts_sim::EventSource::Inventory(unit) => (1, unit),
                                turts_sim::EventSource::Output(unit) => (2, unit),
                            };
                            (kind, unit, line.resource.clone(), u128::from(line.amount))
                        })
                        .collect();
                    actual.sort();
                    expected.sort();
                    if actual != expected {
                        return Err(format!(
                            "production: job {job} refunds {actual:?}, its charges and the catalog ratio give {expected:?} ({label})"
                        ));
                    }
                }
                _ => {}
            }
        }
        let units_before: BTreeSet<u32> = step
            .before
            .raw_units
            .iter()
            .map(|unit| unit.unit_id)
            .collect();
        for unit in &step.after.raw_units {
            if units_before.contains(&unit.unit_id) {
                continue;
            }
            let produced = step.events.iter().any(|event| {
                matches!(&event.kind, EconomyEventKind::ProductionCompleted { output_unit_id, .. } if *output_unit_id == unit.unit_id)
            });
            if !produced {
                return Err(format!(
                    "production: unit {} entered the world with no ProductionCompleted event in {label}",
                    unit.unit_id
                ));
            }
        }
        Ok(())
    }
}
