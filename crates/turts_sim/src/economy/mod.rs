pub mod combat;
pub mod conveyor;
pub mod events;
pub mod extractor;
pub mod gather;
pub mod inserter;
pub mod ledger;
pub mod production;
pub mod research;
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

#[cfg(test)]
mod m3_resource_tests {
    use bevy_ecs::world::World;

    use super::{
        combat::{Healths, Weapons},
        conveyor::Conveyors,
        extractor::Extractors,
        inserter::Inserters,
        research::{Disabled, ResearchState},
    };

    #[test]
    fn m3_side_tables_are_empty_world_resources_by_default() {
        let mut world = World::new();
        world.insert_resource(Conveyors::default());
        world.insert_resource(Inserters::default());
        world.insert_resource(Extractors::default());
        world.insert_resource(Weapons::default());
        world.insert_resource(Healths::default());
        world.insert_resource(ResearchState::default());
        world.insert_resource(Disabled::default());

        assert!(world.resource::<Conveyors>().lines.is_empty());
        assert!(world.resource::<Inserters>().0.is_empty());
        assert!(world.resource::<Extractors>().0.is_empty());
        assert!(world.resource::<Weapons>().0.is_empty());
        assert!(world.resource::<Healths>().0.is_empty());
        assert!(world.resource::<ResearchState>().completed.is_empty());
        assert!(world.resource::<Disabled>().0.is_empty());
    }
}
