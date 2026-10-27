pub mod events;
pub mod gather;
pub mod ledger;
pub mod production;
pub mod transaction;
pub mod views;

use crate::ids::NodeId;
use crate::world::Position;
use bevy_ecs::prelude::Resource;
use turts_mod_api::ContentId;

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct SurfaceNodes(pub(crate) Vec<NodeRecord>);
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NodeRecord {
    pub(crate) id: NodeId,
    pub(crate) content_id: ContentId,
    pub(crate) position: Position,
    pub(crate) remaining: u64,
}
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct EconomyOrders(pub(crate) std::collections::BTreeMap<u32, GatherOrder>);
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GatherOrder {
    pub(crate) node: Option<NodeId>,
    pub(crate) deposit: Option<NodeId>,
    pub(crate) receiver: Option<u32>,
    pub(crate) stage: gather::GatherStage,
    pub(crate) carry: gather::RateCarry,
}
