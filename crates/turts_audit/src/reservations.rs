//! Reservations domain: ledger claims and the reserved amounts of the accounts.
//!
//! Rules:
//! - reservations: no account reserves more than it holds.
//! - reservations: the reserved amount of each account equals the sum of the active claim lines
//!   for that account, and each active claim line names an account that exists.
//! - reservations: an active claim has a token equal to its job id, and names a queued job.
//! - reservations: a job that has started has no active claim.
//! - reservations: a job holds a reservation token exactly when it has an active claim; the token
//!   names the job and holds the claim lines.
//! - reservations: a claim opens only in an operation with a `ProductionQueued` event for its job.
//! - reservations: an active claim closes exactly once, in an operation with a `ProductionStarted`
//!   or a `ProductionCancelled` event for its job, or a `Destroyed` event for the producer of its
//!   job. A cancel releases exactly the claim lines. A closed claim never opens again.

use std::collections::{BTreeMap, BTreeSet};

use turts_mod_api::ContentId;
use turts_sim::{EconomyEventKind, JobStage, ReservationClaimView};

use crate::{LedgerAudit, Step, add, differences, endpoint_key, endpoint_unit};

#[derive(Default)]
pub(crate) struct Reservations {
    closed: BTreeSet<u64>,
}

type AccountKey = ((u8, u32), ContentId);

fn active_claims(state: &LedgerAudit) -> BTreeMap<u64, (u64, &[turts_sim::ReservationLineView])> {
    state
        .ledger_claims
        .iter()
        .filter_map(|claim| match claim {
            ReservationClaimView::Active {
                map_job_id,
                token_job_id,
                lines,
            } => Some((*map_job_id, (*token_job_id, lines.as_slice()))),
            ReservationClaimView::Closed { .. } => None,
        })
        .collect()
}

/// (on_hand, reserved) of every account, by (endpoint kind, endpoint id) and resource.
fn accounts(state: &LedgerAudit) -> BTreeMap<AccountKey, (u64, u64)> {
    let mut result = BTreeMap::new();
    for row in &state.economy.balances {
        result.insert(
            ((0, u32::from(row.slot)), row.resource.clone()),
            (row.on_hand, row.reserved),
        );
    }
    for inventory in &state.economy.inventories {
        for item in &inventory.items {
            result.insert(
                ((1, inventory.unit), item.resource.clone()),
                (item.on_hand, item.reserved),
            );
        }
    }
    for output in &state.raw_output_inventories {
        for (item, account) in &output.items {
            result.insert(
                ((2, output.unit), item.clone()),
                (account.on_hand, account.reserved),
            );
        }
    }
    result
}

fn check_state(state: &LedgerAudit, label: &str) -> Result<(), String> {
    let accounts = accounts(state);
    let active = active_claims(state);
    let mut claimed: BTreeMap<AccountKey, i128> = BTreeMap::new();
    for (job, (token, lines)) in &active {
        if job != token {
            return Err(format!(
                "reservations: active claim {job} holds the token of job {token} after {label}"
            ));
        }
        let queued = state
            .raw_queues
            .iter()
            .flat_map(|queue| &queue.jobs)
            .any(|row| row.id == *job && row.stage == JobStage::Queued);
        if !queued {
            return Err(format!(
                "reservations: active claim {job} names no queued job after {label}"
            ));
        }
        for line in *lines {
            let key = (
                (endpoint_key(&line.endpoint), endpoint_unit(&line.endpoint)),
                line.resource.clone(),
            );
            if !accounts.contains_key(&key) {
                return Err(format!(
                    "reservations: active claim {job} names the missing account {:?} {} after {label}",
                    line.endpoint, line.resource
                ));
            }
            add(&mut claimed, key, i128::from(line.amount));
        }
    }
    let mut reserved: BTreeMap<AccountKey, i128> = BTreeMap::new();
    for (key, (on_hand, held)) in &accounts {
        if held > on_hand {
            return Err(format!(
                "reservations: account {:?} {} reserves {held} above its on_hand {on_hand} after {label}",
                key.0, key.1
            ));
        }
        if *held > 0 {
            reserved.insert(key.clone(), i128::from(*held));
        }
    }
    if let Some((key, held, claims)) = differences(&reserved, &claimed).into_iter().next() {
        return Err(format!(
            "reservations: account {:?} {} reserves {held}, its active claims total {claims} after {label}",
            key.0, key.1
        ));
    }
    for job in state.raw_queues.iter().flat_map(|queue| &queue.jobs) {
        let claim = active.get(&job.id);
        if job.stage != JobStage::Queued && claim.is_some() {
            return Err(format!(
                "reservations: started job {} still has an active claim after {label}",
                job.id
            ));
        }
        let token_matches = match (&job.reservation, claim) {
            (None, None) => true,
            (Some(token), Some((_, lines))) => token.job_id == job.id && token.lines == *lines,
            _ => false,
        };
        if !token_matches {
            return Err(format!(
                "reservations: job {} holds the reservation token {:?} and the active claim {:?} after {label}",
                job.id,
                job.reservation.as_ref().map(|token| token.job_id),
                claim.map(|(token, _)| *token)
            ));
        }
    }
    Ok(())
}

impl Reservations {
    pub(crate) fn check(&mut self, step: &Step<'_>) -> Result<(), String> {
        let label = step.label();
        check_state(step.after, &label)?;
        let before = active_claims(step.before);
        let after = active_claims(step.after);
        for job in after.keys() {
            if before.contains_key(job) {
                continue;
            }
            if self.closed.contains(job) {
                return Err(format!(
                    "reservations: closed claim {job} opens again in {label}"
                ));
            }
            let queued = step.events.iter().any(|event| {
                matches!(&event.kind, EconomyEventKind::ProductionQueued { job: queued, .. } if queued == job)
            });
            if !queued {
                return Err(format!(
                    "reservations: claim {job} opens with no ProductionQueued event in {label}"
                ));
            }
        }
        for (job, (_, lines)) in &before {
            if after.contains_key(job) {
                continue;
            }
            if !self.closed.insert(*job) {
                return Err(format!(
                    "reservations: claim {job} is released more than one time in {label}"
                ));
            }
            let producer = step
                .before
                .raw_queues
                .iter()
                .find(|queue| queue.jobs.iter().any(|row| row.id == *job))
                .map(|queue| queue.producer);
            let mut closed_by_event = false;
            for event in step.events {
                match &event.kind {
                    EconomyEventKind::ProductionStarted { job: started, .. } if started == job => {
                        closed_by_event = true;
                    }
                    EconomyEventKind::ProductionCancelled {
                        job: cancelled,
                        released,
                        ..
                    } if cancelled == job => {
                        let mut released_lines: Vec<_> = released
                            .iter()
                            .map(|line| {
                                (
                                    format!("{:?}", line.source),
                                    line.resource.clone(),
                                    line.amount,
                                )
                            })
                            .collect();
                        let mut claim_lines: Vec<_> = lines
                            .iter()
                            .map(|line| {
                                (
                                    format!("{:?}", event_source(&line.endpoint)),
                                    line.resource.clone(),
                                    line.amount,
                                )
                            })
                            .collect();
                        released_lines.sort();
                        claim_lines.sort();
                        if released_lines != claim_lines {
                            return Err(format!(
                                "reservations: the cancel of job {job} releases other lines than its claim in {label}"
                            ));
                        }
                        closed_by_event = true;
                    }
                    EconomyEventKind::Destroyed { unit, .. } if Some(*unit) == producer => {
                        closed_by_event = true;
                    }
                    _ => {}
                }
            }
            if !closed_by_event {
                return Err(format!(
                    "reservations: claim {job} closes with no start or cancel event in {label}"
                ));
            }
        }
        Ok(())
    }
}

fn event_source(endpoint: &turts_sim::ReservationEndpointView) -> turts_sim::EventSource {
    match endpoint {
        turts_sim::ReservationEndpointView::Slot(slot) => turts_sim::EventSource::Slot(*slot),
        turts_sim::ReservationEndpointView::Inventory(unit) => {
            turts_sim::EventSource::Inventory(*unit)
        }
        turts_sim::ReservationEndpointView::Output(unit) => turts_sim::EventSource::Output(*unit),
    }
}
