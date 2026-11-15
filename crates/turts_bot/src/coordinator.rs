//! Exact planning obligations reconciled only with their authoritative account basis.
use crate::{controller::CoordinatorLimits, types::*};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use turts_mod_api::{PaymentPolicy, WorldPos};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingCommitment {
    pub request: RequestKey,
    pub task: TaskKey,
    pub declared: Vec<AccountAmount>,
    pub unpaid: Vec<AccountAmount>,
    pub work: Option<WorkIdentity>,
    pub payment: Option<PaymentPolicy>,
    pub observed_done: u64,
    pub created_at: Watermark,
    pub represented: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileError {
    Incomplete,
    StaleBasis,
    ChangedBasis,
    InvalidAccount,
    DuplicateEffect,
    ForeignActor,
    FutureEffect,
    InvalidEffect,
    ConflictingResult,
    UnknownRequest,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReconcileReport {
    pub retired: Vec<RequestKey>,
    pub represented: Vec<RequestKey>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedProposal {
    pub task: TaskKey,
    pub reason: ProposalRejection,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Selection {
    pub commands: Vec<EmittedCommand>,
    pub refused: Vec<RejectedProposal>,
}

pub struct BotCoordinator {
    actor: BotActor,
    limits: CoordinatorLimits,
    next_sequence: Option<u32>,
    basis: Option<BasisId>,
    watermark: Option<Watermark>,
    accounts: BTreeMap<SpendKey, AccountObservation>,
    pending: BTreeMap<RequestKey, PendingCommitment>,
    results: BTreeMap<RequestKey, ActorResult>,
    ready: bool,
}
impl BotCoordinator {
    pub fn new(actor: BotActor, limits: CoordinatorLimits) -> Self {
        Self {
            actor,
            limits,
            next_sequence: Some(0),
            basis: None,
            watermark: None,
            accounts: BTreeMap::new(),
            pending: BTreeMap::new(),
            results: BTreeMap::new(),
            ready: false,
        }
    }
    /// A restored/test sequence may start higher, but never wrap after its last value.
    pub fn with_sequence(
        actor: BotActor,
        limits: CoordinatorLimits,
        next_sequence: Option<u32>,
    ) -> Self {
        let mut coordinator = Self::new(actor, limits);
        coordinator.next_sequence = next_sequence;
        coordinator
    }
    pub fn actor(&self) -> &BotActor {
        &self.actor
    }
    pub fn pending(&self) -> Vec<PendingCommitment> {
        self.pending.values().cloned().collect()
    }
    pub fn has_task(&self, task: &TaskKey) -> bool {
        self.pending.values().any(|entry| &entry.task == task)
    }
    pub fn next_sequence(&self) -> Option<u32> {
        self.next_sequence
    }
    pub fn observe_result(&mut self, result: ActorResult) -> Result<(), ReconcileError> {
        if result.request.actor != self.actor {
            return Err(ReconcileError::ForeignActor);
        }
        let Some(pending) = self.pending.get(&result.request) else {
            return if self
                .next_sequence
                .is_none_or(|next| result.request.sequence < next)
            {
                Ok(())
            } else {
                Err(ReconcileError::UnknownRequest)
            };
        };
        if result.effect_watermark < pending.created_at {
            return Err(ReconcileError::InvalidEffect);
        }
        if let Some(old) = self.results.get(&result.request) {
            return if old == &result {
                Ok(())
            } else {
                Err(ReconcileError::ConflictingResult)
            };
        }
        self.results.insert(result.request.clone(), result);
        Ok(())
    }
    /// Accounts, witnesses, and commitments advance together or remain unchanged.
    pub fn reconcile(
        &mut self,
        basis: BasisId,
        batch: &ReconcileBatch,
    ) -> Result<ReconcileReport, ReconcileError> {
        let result = self.prepare_reconcile(basis, batch);
        match result {
            Ok((accounts, pending, report)) => {
                self.accounts = accounts;
                self.pending = pending;
                self.results.retain(|key, _| self.pending.contains_key(key));
                self.basis = Some(basis);
                self.watermark = Some(batch.watermark);
                self.ready = true;
                Ok(report)
            }
            Err(error) => {
                self.ready = false;
                Err(error)
            }
        }
    }
    fn prepare_reconcile(
        &self,
        basis: BasisId,
        batch: &ReconcileBatch,
    ) -> Result<
        (
            BTreeMap<SpendKey, AccountObservation>,
            BTreeMap<RequestKey, PendingCommitment>,
            ReconcileReport,
        ),
        ReconcileError,
    > {
        if !batch.complete {
            return Err(ReconcileError::Incomplete);
        }
        if self.watermark.is_some_and(|old| batch.watermark < old) {
            return Err(ReconcileError::StaleBasis);
        }
        let mut accounts = BTreeMap::new();
        for account in &batch.accounts {
            if account.reserved > account.on_hand
                || !valid_endpoint(&account.account.endpoint, self.actor.slot)
                || accounts
                    .insert(account.account.clone(), account.clone())
                    .is_some()
            {
                return Err(ReconcileError::InvalidAccount);
            }
        }
        if self.watermark == Some(batch.watermark)
            && (self.basis != Some(basis) || self.accounts != accounts)
        {
            return Err(ReconcileError::ChangedBasis);
        }
        let mut included = BTreeMap::new();
        for effect in &batch.included {
            if effect.request.actor != self.actor {
                return Err(ReconcileError::ForeignActor);
            }
            if effect.effect_watermark > batch.watermark {
                return Err(ReconcileError::FutureEffect);
            }
            if included.insert(effect.request.clone(), effect).is_some() {
                return Err(ReconcileError::DuplicateEffect);
            }
        }
        let mut unapplied = BTreeSet::new();
        for key in &batch.unapplied {
            if key.actor != self.actor {
                return Err(ReconcileError::ForeignActor);
            }
            if included.contains_key(key) || !unapplied.insert(key.clone()) {
                return Err(ReconcileError::DuplicateEffect);
            }
        }
        let mut pending = self.pending.clone();
        let mut report = ReconcileReport::default();
        let mut retire = Vec::new();
        for (key, entry) in &mut pending {
            let Some(effect) = included.get(key) else {
                if entry.represented
                    || !unapplied.contains(key)
                    || self
                        .results
                        .get(key)
                        .is_some_and(|result| result.effect_watermark <= batch.watermark)
                {
                    return Err(ReconcileError::Incomplete);
                }
                continue;
            };
            if effect.effect_watermark < entry.created_at {
                return Err(ReconcileError::InvalidEffect);
            }
            if let Some(result) = self.results.get(key)
                && (result.effect_watermark != effect.effect_watermark
                    || result.accepted == effect.refused
                    || result.work.as_ref().is_some_and(|work| {
                        effect
                            .work
                            .as_ref()
                            .is_none_or(|observed| &observed.identity != work)
                    }))
            {
                return Err(ReconcileError::ConflictingResult);
            }
            let paid = amounts(&effect.paid).map_err(|_| ReconcileError::InvalidEffect)?;
            let reserved = amounts(&effect.reserved).map_err(|_| ReconcileError::InvalidEffect)?;
            let declared = amounts(&entry.declared).map_err(|_| ReconcileError::InvalidEffect)?;
            if paid
                .keys()
                .chain(reserved.keys())
                .any(|account| !declared.contains_key(account))
            {
                return Err(ReconcileError::InvalidEffect);
            }
            if effect.refused
                && (!effect.terminal
                    || !paid.is_empty()
                    || !reserved.is_empty()
                    || effect.work.is_some()
                    || effect.pending_placement)
            {
                return Err(ReconcileError::InvalidEffect);
            }
            if let Some(work) = &effect.work {
                if work.actor_unit != key.unit
                    || work.done > work.total
                    || work.done < entry.observed_done
                    || work.payment != entry.payment
                    || entry.work.as_ref().is_some_and(|old| old != &work.identity)
                    || amounts(&work.paid).map_err(|_| ReconcileError::InvalidEffect)? != paid
                    || amounts(&work.reserved).map_err(|_| ReconcileError::InvalidEffect)?
                        != reserved
                {
                    return Err(ReconcileError::InvalidEffect);
                }
                if work.payment == Some(PaymentPolicy::ChargeOverTime) {
                    if work.total == 0 || !reserved.is_empty() {
                        return Err(ReconcileError::InvalidEffect);
                    }
                    for (account, full) in &declared {
                        let expected = (u128::from(*full) * u128::from(work.done)
                            / u128::from(work.total)) as u64;
                        if paid.get(account).copied().unwrap_or(0) != expected {
                            return Err(ReconcileError::InvalidEffect);
                        }
                    }
                }
                entry.work = Some(work.identity.clone());
                entry.observed_done = work.done;
            }
            let mut remaining = Vec::new();
            for (account, full) in declared {
                let represented = paid
                    .get(&account)
                    .copied()
                    .unwrap_or(0)
                    .checked_add(reserved.get(&account).copied().unwrap_or(0))
                    .ok_or(ReconcileError::InvalidEffect)?;
                let amount = full
                    .checked_sub(represented)
                    .ok_or(ReconcileError::InvalidEffect)?;
                if amount > 0 {
                    if entry.represented
                        && amount
                            > entry
                                .unpaid
                                .iter()
                                .find(|old| old.account == account)
                                .map_or(0, |old| old.amount)
                    {
                        return Err(ReconcileError::InvalidEffect);
                    }
                    remaining.push(AccountAmount { account, amount });
                }
            }
            if effect.refused || effect.terminal || remaining.is_empty() {
                retire.push(key.clone());
                report.retired.push(key.clone());
            } else {
                if effect.work.is_none()
                    && (!effect.pending_placement || !paid.is_empty() || !reserved.is_empty())
                {
                    return Err(ReconcileError::InvalidEffect);
                }
                entry.unpaid = remaining;
                entry.represented = true;
                report.represented.push(key.clone());
            }
        }
        for key in retire {
            pending.remove(&key);
        }
        Ok((accounts, pending, report))
    }
    pub fn select(
        &mut self,
        basis: BasisId,
        proposals: Vec<Proposal>,
        verifier: &dyn ProposalVerifier,
    ) -> Selection {
        let mut out = Selection::default();
        let mut candidates = Vec::new();
        let mut task_counts = BTreeMap::new();
        for proposal in &proposals {
            *task_counts.entry(proposal.task.clone()).or_insert(0_usize) += 1;
        }
        for proposal in proposals {
            let reject = if !self.ready || self.basis != Some(basis) {
                Some(ProposalRejection::AccountingUnresolved)
            } else if self.has_task(&proposal.task) || task_counts[&proposal.task] > 1 {
                Some(ProposalRejection::DuplicateTask)
            } else {
                None
            };
            if let Some(reason) = reject {
                out.refused.push(RejectedProposal {
                    task: proposal.task,
                    reason,
                });
                continue;
            }
            match verifier
                .verify(basis, &proposal)
                .and_then(|verified| validate_verified(&proposal, verified, self.actor.slot))
            {
                Ok(verified) => candidates.push((proposal, verified)),
                Err(reason) => out.refused.push(RejectedProposal {
                    task: proposal.task,
                    reason,
                }),
            }
        }
        candidates.sort_by(|(a, av), (b, bv)| rank_cmp(a, av, b, bv));
        let mut assigned = self
            .pending
            .values()
            .filter(|entry| !entry.represented)
            .map(|entry| entry.request.unit)
            .collect::<BTreeSet<_>>();
        let mut commands = 0_u32;
        let mut units = 0_u32;
        let mut i = 0;
        while i < candidates.len() {
            let mut end = i + 1;
            while end < candidates.len()
                && rank_cmp(
                    &candidates[i].0,
                    &candidates[i].1,
                    &candidates[end].0,
                    &candidates[end].1,
                )
                .is_eq()
            {
                end += 1;
            }
            if end > i + 1 {
                for (proposal, _) in &candidates[i..end] {
                    out.refused.push(RejectedProposal {
                        task: proposal.task.clone(),
                        reason: ProposalRejection::EqualRank,
                    });
                }
            } else {
                let (proposal, verified) = &candidates[i];
                match self.emit(proposal, verified, &assigned, commands, units) {
                    Ok(command) => {
                        commands += 1;
                        units += command.units.len() as u32;
                        assigned.extend(command.units.iter().copied());
                        out.commands.push(command);
                    }
                    Err(reason) => out.refused.push(RejectedProposal {
                        task: proposal.task.clone(),
                        reason,
                    }),
                }
            }
            i = end;
        }
        out
    }
    fn emit(
        &mut self,
        proposal: &Proposal,
        verified: &VerifiedProposal,
        assigned: &BTreeSet<UnitId>,
        commands: u32,
        units: u32,
    ) -> Result<EmittedCommand, ProposalRejection> {
        let count = u32::try_from(verified.members.len()).map_err(|_| ProposalRejection::Budget)?;
        if commands >= self.limits.commands
            || units
                .checked_add(count)
                .is_none_or(|count| count > self.limits.affected_units)
        {
            return Err(ProposalRejection::Budget);
        }
        if verified
            .members
            .iter()
            .any(|member| assigned.contains(&member.unit))
        {
            return Err(ProposalRejection::ConflictingAssignment);
        }
        let envelopes = self
            .pending
            .keys()
            .map(|key| key.sequence)
            .collect::<BTreeSet<_>>()
            .len();
        if envelopes >= self.limits.pending_requests as usize {
            return Err(ProposalRejection::Budget);
        }
        let sequence = self
            .next_sequence
            .ok_or(ProposalRejection::SequenceExhausted)?;
        let mut required = BTreeMap::<SpendKey, u128>::new();
        let mut touched_accounts = BTreeSet::new();
        let mut pending_accounts = BTreeSet::new();
        for entry in self.pending.values() {
            for amount in &entry.unpaid {
                pending_accounts.insert(amount.account.clone());
                *required.entry(amount.account.clone()).or_default() += u128::from(amount.amount);
            }
        }
        for member in &verified.members {
            for amount in &member.costs {
                pending_accounts.insert(amount.account.clone());
                touched_accounts.insert(amount.account.clone());
                let old = required.entry(amount.account.clone()).or_default();
                *old = old
                    .checked_add(u128::from(amount.amount))
                    .ok_or(ProposalRejection::ArithmeticOverflow)?;
            }
        }
        if pending_accounts.len() > self.limits.pending_accounts as usize {
            return Err(ProposalRejection::Budget);
        }
        for (account, amount) in required {
            if !touched_accounts.contains(&account) {
                continue;
            }
            let observed = self
                .accounts
                .get(&account)
                .ok_or(ProposalRejection::NotObserved)?;
            let available = observed
                .on_hand
                .checked_sub(observed.reserved)
                .ok_or(ProposalRejection::InvalidCosts)?;
            if amount > u128::from(available) {
                return Err(ProposalRejection::InsufficientResources(account));
            }
        }
        let watermark = self
            .watermark
            .ok_or(ProposalRejection::AccountingUnresolved)?;
        for member in &verified.members {
            let request = RequestKey {
                actor: self.actor.clone(),
                sequence,
                unit: member.unit,
            };
            self.pending.insert(
                request.clone(),
                PendingCommitment {
                    request,
                    task: proposal.task.clone(),
                    declared: member.costs.clone(),
                    unpaid: member.costs.clone(),
                    work: None,
                    payment: member.payment,
                    observed_done: 0,
                    created_at: watermark,
                    represented: false,
                },
            );
        }
        self.next_sequence = sequence.checked_add(1);
        let mut members = verified.members.iter().collect::<Vec<_>>();
        // IDs only canonicalize a complete same-position group for serialization; never truncate it.
        members.sort_by(|a, b| position_cmp(&a.anchor, &b.anchor).then(a.unit.cmp(&b.unit)));
        Ok(EmittedCommand {
            actor: self.actor.clone(),
            sequence,
            task: proposal.task.clone(),
            units: members.into_iter().map(|member| member.unit).collect(),
            order: proposal.order.clone(),
        })
    }
}
fn valid_endpoint(endpoint: &SourceEndpoint, slot: u8) -> bool {
    match endpoint {
        SourceEndpoint::SlotBalance(owner) => *owner == slot,
        SourceEndpoint::Inventory { unit, .. } => *unit != 0,
    }
}
fn amounts(rows: &[AccountAmount]) -> Result<BTreeMap<SpendKey, u64>, ProposalRejection> {
    let mut out = BTreeMap::new();
    for row in rows {
        if row.amount == 0 || out.insert(row.account.clone(), row.amount).is_some() {
            return Err(ProposalRejection::InvalidCosts);
        }
    }
    Ok(out)
}
fn validate_verified(
    proposal: &Proposal,
    verified: VerifiedProposal,
    slot: u8,
) -> Result<VerifiedProposal, ProposalRejection> {
    if proposal.actors.is_empty()
        || proposal.task.controller.is_empty()
        || proposal.task.goal.is_empty()
        || proposal.task.instance.is_empty()
    {
        return Err(ProposalRejection::InvalidTask);
    }
    let actors = proposal.actors.iter().copied().collect::<BTreeSet<_>>();
    if actors.len() != proposal.actors.len() {
        return Err(ProposalRejection::DuplicateActor);
    }
    let mut members = BTreeSet::new();
    for member in &verified.members {
        if member.unit == 0 || !actors.contains(&member.unit) || !members.insert(member.unit) {
            return Err(ProposalRejection::DuplicateActor);
        }
        if !member.anchor.x.is_finite() || !member.anchor.y.is_finite() {
            return Err(ProposalRejection::InvalidAnchor);
        }
        for account in amounts(&member.costs)?.keys() {
            if !valid_endpoint(&account.endpoint, slot)
                || matches!(account.endpoint,SourceEndpoint::Inventory{unit,..} if unit!=member.unit)
            {
                return Err(ProposalRejection::InvalidCosts);
            }
        }
    }
    if actors != members {
        return Err(ProposalRejection::InvalidCosts);
    }
    Ok(verified)
}
fn coordinate_cmp(a: f32, b: f32) -> std::cmp::Ordering {
    if a == b {
        std::cmp::Ordering::Equal
    } else {
        a.total_cmp(&b)
    }
}
fn position_cmp(a: &WorldPos, b: &WorldPos) -> std::cmp::Ordering {
    coordinate_cmp(a.x, b.x).then_with(|| coordinate_cmp(a.y, b.y))
}
fn anchor(verified: &VerifiedProposal) -> &WorldPos {
    &verified
        .members
        .iter()
        .min_by(|a, b| position_cmp(&a.anchor, &b.anchor))
        .expect("verified proposal has actors")
        .anchor
}
fn rank_cmp(
    a: &Proposal,
    av: &VerifiedProposal,
    b: &Proposal,
    bv: &VerifiedProposal,
) -> std::cmp::Ordering {
    b.priority
        .cmp(&a.priority)
        .then_with(|| position_cmp(anchor(av), anchor(bv)))
        .then(a.goal_order.cmp(&b.goal_order))
}
