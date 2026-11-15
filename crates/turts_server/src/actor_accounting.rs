//! Immutable completed-basis accounting for trusted participant receipts.
use turts_bot::{ActorResult, EmittedCommand};
use turts_sim::receipts::AdmittedWork;

#[derive(Debug, Clone)]
pub(crate) struct TrackedRequest {
    pub command: EmittedCommand,
    pub result: Option<ActorResult>,
    pub admitted_work: Option<AdmittedWork>,
}

impl TrackedRequest {
    pub(crate) fn new(command: EmittedCommand) -> Self {
        Self {
            command,
            result: None,
            admitted_work: None,
        }
    }

    pub(crate) fn finalize(&mut self, result: ActorResult, work: Option<AdmittedWork>) {
        self.result = Some(result);
        self.admitted_work = work;
    }
}
