use crate::{economy::ledger::ReservationLine, ids::JobId};
use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use turts_mod_api::ContentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStage {
    Queued,
    Running,
    OutputBlocked,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductionJob {
    pub(crate) id: JobId,
    pub(crate) recipe: ContentId,
    pub(crate) stage: JobStage,
    pub(crate) elapsed_ticks: u64,
    pub(crate) duration_ticks: u64,
    pub(crate) charged: Vec<ReservationLine>,
    pub(crate) reservation: Option<super::ledger::Reservation>,
}
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProductionQueues(pub(crate) std::collections::BTreeMap<u32, Vec<ProductionJob>>);
