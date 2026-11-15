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
            // An invalid duplicate cannot choose a winner by whichever proposal arrived first.
            self.proposals.retain(|old| old.task != proposal.task);
            return Err(ProposalRejection::DuplicateTask);
        }
        self.keys.insert(proposal.task.clone());
        self.proposals.push(proposal);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_mod_api::ContentId;
    fn proposal(unit: u32) -> Proposal {
        Proposal {
            source_basis: 0,
            task: TaskKey::new("controller", "goal", "0"),
            actors: vec![unit],
            order: BotOrder {
                content: ContentId::parse("test:order").unwrap(),
                target: BotTarget::None,
                queue: BotQueueMode::Replace,
                group: 0,
            },
            priority: 1,
            goal_order: 0,
        }
    }
    #[test]
    fn duplicate_tasks_remove_both_candidates_instead_of_keeping_the_first_actor() {
        for actors in [[1, 900], [900, 1]] {
            let mut sink = ProposalBuffer::new("controller", 3, 1);
            sink.propose(proposal(actors[0])).unwrap();
            assert_eq!(
                sink.propose(proposal(actors[1])),
                Err(ProposalRejection::DuplicateTask)
            );
            assert!(sink.into_proposals().is_empty());
        }
    }
    #[test]
    fn invalid_attempts_use_the_declared_proposal_budget() {
        let mut sink = ProposalBuffer::new("controller", 1, 1);
        let mut invalid = proposal(1);
        invalid.task.controller = "other".into();
        assert_eq!(sink.propose(invalid), Err(ProposalRejection::InvalidTask));
        assert_eq!(sink.propose(proposal(1)), Err(ProposalRejection::Budget));
        assert!(sink.into_proposals().is_empty());
    }
}
