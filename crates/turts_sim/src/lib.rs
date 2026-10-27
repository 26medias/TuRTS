//! Authoritative headless world: fixed ticks, flat map, unit spawning, movement, command validation.

pub mod bootstrap;
pub mod command;
pub mod economy;
pub mod harness;
pub mod ids;
pub mod map;
pub mod movement;
pub mod plugin;
pub mod registry;
pub mod slots;
pub mod world;

#[derive(bevy_ecs::prelude::Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimClock {
    pub tick_hz: u16,
}

/// Fixed simulation ticks per second.
pub const DEFAULT_TICK_HZ: u16 = 20; // prototype constant: measure at M1

pub use bevy_math::Vec2;
pub use bootstrap::{StagedPackage, install_package, stage_package};
pub use command::{
    Command, CommandKind, CommandQueue, CommandResult, CommandResults, RejectReason,
};
pub use economy::ledger::EconomyLedger;
pub use economy::production::JobStage;
pub use economy::views::{
    AuditState, BalanceView, CostView, InventoryView, ItemView, JobView, NodeView, OrderView,
    QueueView, UnitTypeView, UnitView, WorldView, audit_state, balance_view, inventory_view,
    node_view, order_view, queue_view, world_view,
};
pub use economy::{EconomyOrders, SurfaceNodes};
pub use harness::headless_app;
pub use ids::{IdAllocators, JobId, NodeId};
pub use map::load_map;
pub use plugin::SimPlugin;
pub use registry::{UnitTypeEntry, UnitTypes};
pub use slots::{ClaimError, SlotId, Slots};
pub use world::{
    ChangedThisTick, MapBounds, MoveTarget, Owner, Position, Tick, UnitId, UnitState, UnitType,
    gameplay_entity_count, snapshot,
};

/// Spawn diagnostic units with the shared checked unit allocator.
pub fn spawn_synthetic_units(
    world: &mut bevy_ecs::world::World,
    unit_type: turts_mod_api::UnitTypeId,
    positions: &[Vec2],
) -> Result<Vec<u32>, String> {
    let entry = world
        .resource::<UnitTypes>()
        .get(unit_type)
        .ok_or("unknown synthetic unit type")?;
    if entry.movement.is_none() {
        return Err("synthetic units require a mobile unit type".into());
    }
    let mut alloc = *world.resource::<IdAllocators>();
    let mut ids = Vec::with_capacity(positions.len());
    for _ in positions {
        ids.push(alloc.allocate_unit()?);
    }
    for (id, pos) in ids.iter().zip(positions) {
        world.spawn((
            UnitId(*id),
            world::UnitType(unit_type),
            world::Owner(crate::SlotId(0)),
            world::Position(*pos),
        ));
    }
    world.insert_resource(alloc);
    Ok(ids)
}
