//! Per-controller proposal collection. Invalid attempts still consume their attempt quota.
use crate::{controller::ProposalSink, types::*};
use std::collections::BTreeSet;

pub struct ProposalBuffer {
    controller: String,
    limit: u32,
    used: u32,
    max_actors: u32,
    keys: BTreeSet<TaskKey>,
    proposals: Vec<Proposal>,
}
impl ProposalBuffer {
    pub fn new(controller: impl Into<String>, limit: u32, max_actors: u32) -> Self {
        Self {
            controller: controller.into(),
            limit,
            used: 0,
            max_actors,
            keys: BTreeSet::new(),
            proposals: Vec::new(),
        }
    }
    pub fn used(&self) -> u32 {
        self.used
    }
    pub fn into_proposals(self) -> Vec<Proposal> {
        self.proposals
    }
}
impl ProposalSink for ProposalBuffer {
    fn propose(&mut self, proposal: Proposal) -> Result<(), ProposalRejection> {
        if self.used >= self.limit {
            return Err(ProposalRejection::Budget);
        }
        self.used += 1;
        if proposal.task.controller != self.controller
            || proposal.task.goal.is_empty()
            || proposal.task.instance.is_empty()
        {
            return Err(ProposalRejection::InvalidTask);
        }
        if proposal.actors.is_empty() {
            return Err(ProposalRejection::UnknownActor);
        }
        if proposal.actors.len() > self.max_actors as usize {
            return Err(ProposalRejection::Budget);
        }
        let mut actors = BTreeSet::new();
        if proposal.actors.iter().any(|unit| !actors.insert(*unit)) {
            return Err(ProposalRejection::DuplicateActor);
        }
        if self.keys.contains(&proposal.task) {
            return Err(ProposalRejection::DuplicateTask);
        }
        self.keys.insert(proposal.task.clone());
        self.proposals.push(proposal);
        Ok(())
    }
}
