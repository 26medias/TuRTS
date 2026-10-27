use crate::{
    economy::{
        EconomyOrders, SurfaceNodes,
        ledger::EconomyLedger,
        production::{ProductionJob, ProductionQueues},
    },
    ids::{IdAllocators, NodeId},
    slots::Slots,
    world::{Owner, Position, UnitId, UnitType},
};
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use turts_mod_api::{ContentId, WorldPos};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceView {
    pub slot: u8,
    pub resource: ContentId,
    pub on_hand: u64,
    pub reserved: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemView {
    pub resource: ContentId,
    pub on_hand: u64,
    pub reserved: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryView {
    pub unit: u32,
    pub owner_slot: u8,
    pub capacity: u64,
    pub items: Vec<ItemView>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderView {
    pub unit: u32,
    pub owner_slot: u8,
    pub node: Option<NodeId>,
    pub receiver: Option<u32>,
    pub stage: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostView {
    pub resource: ContentId,
    pub amount: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobView {
    pub id: u64,
    pub recipe: ContentId,
    pub stage: crate::economy::production::JobStage,
    pub elapsed_ticks: u64,
    pub duration_ticks: u64,
    pub charged: Vec<CostView>,
    pub reserved: Vec<CostView>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueView {
    pub producer: u32,
    pub owner_slot: u8,
    pub jobs: Vec<JobView>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeView {
    pub id: NodeId,
    pub content_id: ContentId,
    pub pos: WorldPos,
    pub remaining: u64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct UnitTypeView {
    pub content_id: String,
    pub movement_speed: Option<f32>,
    pub radius: f32,
}
#[derive(Debug, Clone, PartialEq)]
pub struct UnitView {
    pub unit_id: u32,
    pub content_id: String,
    pub owner_slot: u8,
    pub pos: WorldPos,
}
#[derive(Debug, Clone, PartialEq)]
pub struct AuditState {
    pub ledger_claims: Vec<(u64, bool)>,
    pub unit_types: Vec<UnitTypeView>,
    pub units: Vec<UnitView>,
    pub nodes: Vec<NodeView>,
    pub economy: WorldView,
    pub allocators: IdAllocators,
    pub claims: Vec<Option<u64>>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldView {
    pub balances: Vec<BalanceView>,
    pub inventories: Vec<InventoryView>,
    pub orders: Vec<OrderView>,
    pub queues: Vec<QueueView>,
    pub nodes: Vec<NodeView>,
}

pub fn world_view(world: &mut World) -> WorldView {
    let economy = world
        .get_resource::<EconomyLedger>()
        .cloned()
        .unwrap_or_default();
    let orders = world
        .get_resource::<EconomyOrders>()
        .cloned()
        .unwrap_or_default();
    let queues = world
        .get_resource::<ProductionQueues>()
        .cloned()
        .unwrap_or_default();
    let owners = owner_map(world);
    let mut view = WorldView::default();
    for ((slot, resource), account) in economy.balances {
        view.balances.push(BalanceView {
            slot,
            resource,
            on_hand: account.on_hand,
            reserved: account.reserved,
        });
    }
    for (unit, inventory) in economy.inventories {
        let owner_slot = owners.get(&unit).copied().unwrap_or_default();
        view.inventories.push(InventoryView {
            unit,
            owner_slot,
            capacity: inventory.capacity,
            items: inventory
                .items
                .into_iter()
                .map(|(resource, account)| ItemView {
                    resource,
                    on_hand: account.on_hand,
                    reserved: account.reserved,
                })
                .collect(),
        });
    }
    for (unit, order) in orders.0 {
        view.orders.push(OrderView {
            unit,
            owner_slot: owners.get(&unit).copied().unwrap_or_default(),
            node: order.node,
            receiver: order.receiver,
            stage: order.stage,
        });
    }
    for (producer, jobs) in queues.0 {
        view.queues.push(QueueView {
            producer,
            owner_slot: owners.get(&producer).copied().unwrap_or_default(),
            jobs: jobs.into_iter().map(job_view).collect(),
        });
    }
    view.nodes = node_views(world);
    view
}
fn job_view(job: ProductionJob) -> JobView {
    JobView {
        id: job.id.0,
        recipe: job.recipe,
        stage: job.stage,
        elapsed_ticks: job.elapsed_ticks,
        duration_ticks: job.duration_ticks,
        charged: job
            .charged
            .into_iter()
            .map(|line| CostView {
                resource: line.resource,
                amount: line.amount,
            })
            .collect(),
        reserved: job.reservation.map_or_else(Vec::new, |r| {
            r.lines
                .into_iter()
                .map(|line| CostView {
                    resource: line.resource,
                    amount: line.amount,
                })
                .collect()
        }),
    }
}
fn owner_map(world: &mut World) -> std::collections::BTreeMap<u32, u8> {
    let mut result = std::collections::BTreeMap::new();
    let mut q = world.query::<(&UnitId, &Owner)>();
    for (unit, owner) in q.iter(world) {
        result.insert(unit.0, owner.0.0);
    }
    result
}
fn node_views(world: &World) -> Vec<NodeView> {
    world
        .get_resource::<SurfaceNodes>()
        .map_or_else(Vec::new, |nodes| {
            nodes
                .0
                .iter()
                .map(|n| NodeView {
                    id: n.id,
                    content_id: n.content_id.clone(),
                    pos: WorldPos {
                        x: n.position.0.x,
                        y: n.position.0.y,
                    },
                    remaining: n.remaining,
                })
                .collect()
        })
}
pub fn audit_state(world: &mut World) -> AuditState {
    let mut q = world.query::<(&UnitId, &UnitType, &Owner, &Position)>();
    let types = world.get_resource::<crate::UnitTypes>();
    let mut units: Vec<_> = q
        .iter(world)
        .map(|(id, kind, owner, pos)| UnitView {
            unit_id: id.0,
            content_id: types
                .and_then(|types| types.get(kind.0))
                .map_or_else(String::new, |t| t.content_id.clone()),
            owner_slot: owner.0.0,
            pos: WorldPos {
                x: pos.0.x,
                y: pos.0.y,
            },
        })
        .collect();
    units.sort_by_key(|unit| unit.unit_id);
    let nodes = node_views(world);
    let unit_types = world
        .get_resource::<crate::UnitTypes>()
        .map_or_else(Vec::new, |types| {
            types
                .entries()
                .iter()
                .map(|entry| UnitTypeView {
                    content_id: entry.content_id.clone(),
                    movement_speed: entry.movement.as_ref().map(|movement| movement.speed),
                    radius: entry.radius,
                })
                .collect()
        });
    let economy = world_view(world);
    let allocators = *world
        .get_resource::<IdAllocators>()
        .unwrap_or(&IdAllocators {
            next_unit: 1,
            next_node: 1,
            next_job: 1,
        });
    let claims = world
        .get_resource::<Slots>()
        .map_or_else(Vec::new, Slots::owners);
    let ledger_claims = world
        .get_resource::<EconomyLedger>()
        .map_or_else(Vec::new, |ledger| {
            ledger
                .reservation_claims
                .iter()
                .map(|(id, claim)| {
                    (
                        id.0,
                        matches!(claim, crate::economy::ledger::ReservationClaim::Active(_)),
                    )
                })
                .collect()
        });
    AuditState {
        ledger_claims,
        unit_types,
        units,
        nodes,
        economy,
        allocators,
        claims,
    }
}

/// Return an owned balance snapshot. The shared World borrow cannot expose writable ledger state.
pub fn balance_view(
    world: &World,
    slot: crate::SlotId,
    resource: &ContentId,
) -> Result<BalanceView, crate::RejectReason> {
    let ledger = world
        .get_resource::<EconomyLedger>()
        .ok_or(crate::RejectReason::UnknownBalance)?;
    let account = ledger
        .balances
        .get(&(slot.0, resource.clone()))
        .ok_or(crate::RejectReason::UnknownBalance)?;
    Ok(BalanceView {
        slot: slot.0,
        resource: resource.clone(),
        on_hand: account.on_hand,
        reserved: account.reserved,
    })
}

/// Return an owned inventory snapshot after checking the owner's slot first.
pub fn inventory_view(
    world: &World,
    slot: crate::SlotId,
    unit_id: u32,
) -> Result<InventoryView, crate::RejectReason> {
    let owner = world
        .iter_entities()
        .find_map(|entity| {
            (entity.get::<UnitId>().is_some_and(|unit| unit.0 == unit_id))
                .then(|| entity.get::<Owner>().map(|owner| owner.0))?
        })
        .ok_or(crate::RejectReason::UnknownInventory)?;
    if owner != slot {
        return Err(crate::RejectReason::NotOwner);
    }
    let inventory = world
        .get_resource::<EconomyLedger>()
        .and_then(|ledger| ledger.inventories.get(&unit_id))
        .ok_or(crate::RejectReason::UnknownInventory)?;
    Ok(InventoryView {
        unit: unit_id,
        owner_slot: owner.0,
        capacity: inventory.capacity,
        items: inventory
            .items
            .iter()
            .map(|(resource, account)| ItemView {
                resource: resource.clone(),
                on_hand: account.on_hand,
                reserved: account.reserved,
            })
            .collect(),
    })
}

/// Return an owned surface node snapshot.
pub fn node_view(world: &World, id: NodeId) -> Result<NodeView, crate::RejectReason> {
    world
        .get_resource::<SurfaceNodes>()
        .and_then(|nodes| nodes.0.iter().find(|node| node.id == id))
        .map(|node| NodeView {
            id: node.id,
            content_id: node.content_id.clone(),
            pos: WorldPos {
                x: node.position.0.x,
                y: node.position.0.y,
            },
            remaining: node.remaining,
        })
        .ok_or(crate::RejectReason::UnknownNode)
}

/// Return a producer queue snapshot after checking the producer's owner.
pub fn queue_view(
    world: &World,
    slot: crate::SlotId,
    unit_id: u32,
) -> Result<QueueView, crate::RejectReason> {
    let owner = unit_owner(world, unit_id)?;
    if owner != slot {
        return Err(crate::RejectReason::NotOwner);
    }
    let jobs = world
        .get_resource::<ProductionQueues>()
        .and_then(|queues| queues.0.get(&unit_id))
        .ok_or(crate::RejectReason::UnknownUnit)?;
    Ok(QueueView {
        producer: unit_id,
        owner_slot: owner.0,
        jobs: jobs.iter().cloned().map(job_view).collect(),
    })
}

/// Return an order snapshot after checking the unit's owner.
pub fn order_view(
    world: &World,
    slot: crate::SlotId,
    unit_id: u32,
) -> Result<OrderView, crate::RejectReason> {
    let owner = unit_owner(world, unit_id)?;
    if owner != slot {
        return Err(crate::RejectReason::NotOwner);
    }
    let order = world
        .get_resource::<EconomyOrders>()
        .and_then(|orders| orders.0.get(&unit_id))
        .ok_or(crate::RejectReason::UnknownUnit)?;
    Ok(OrderView {
        unit: unit_id,
        owner_slot: owner.0,
        node: order.node,
        receiver: order.receiver,
        stage: order.stage.clone(),
    })
}

fn unit_owner(world: &World, unit_id: u32) -> Result<crate::SlotId, crate::RejectReason> {
    world
        .iter_entities()
        .find_map(|entity| {
            (entity.get::<UnitId>().is_some_and(|unit| unit.0 == unit_id))
                .then(|| entity.get::<Owner>().map(|owner| owner.0))?
        })
        .ok_or(crate::RejectReason::UnknownUnit)
}
