//! Pure projection of immutable simulation views into protocol 2 public state.
//!
//! Private economy records are filtered by owner before any wire values are built.

use turts_mod_api::Cost;
use turts_net_protocol::messages as wire;
use turts_net_protocol::messages;
use turts_sim::{
    CommittedTick, SlotId, WorldView,
    economy::{
        gather::{GatherStage, WaitReason},
        production::JobStage,
        views::{
            BalanceView, CostView, InventoryView, ItemView, JobView, NodeView, OrderView, QueueView,
        },
    },
    world::UnitState,
};

/// Build the owner-specific reliable snapshot from one completed world view.
pub fn snapshot_for(view: &WorldView, slot: SlotId) -> wire::Snapshot {
    wire::Snapshot {
        tick: view.tick,
        units: view.units.iter().map(unit_snapshot).collect(),
        nodes: view.nodes.iter().map(node_state).collect(),
        economy: wire::EconomyState {
            balances: view
                .economy
                .balances
                .iter()
                .filter(|record| owns_balance(record, slot))
                .map(balance_state)
                .collect(),
            inventories: view
                .economy
                .inventories
                .iter()
                .filter(|record| record.owner_slot == slot.0)
                .map(inventory_state)
                .collect(),
            orders: view
                .economy
                .orders
                .iter()
                .filter(|record| record.owner_slot == slot.0)
                .map(order_state)
                .collect(),
            queues: view
                .economy
                .queues
                .iter()
                .filter(|record| record.owner_slot == slot.0)
                .map(queue_state)
                .collect(),
        },
    }
}

/// Build the owner-specific absolute replacements for one committed simulation tick.
pub fn delta_for(batch: &CommittedTick, slot: SlotId) -> wire::WorldDelta {
    wire::WorldDelta {
        tick: batch.tick,
        base_tick: batch.tick.saturating_sub(1),
        spawned: batch.spawned.iter().map(unit_snapshot).collect(),
        removed: batch
            .removed
            .iter()
            .copied()
            .map(messages::UnitId)
            .collect(),
        positions: batch
            .positions
            .iter()
            .map(|(id, pos)| messages::UnitDelta {
                unit_id: messages::UnitId(*id),
                pos: messages::Pos { x: pos.x, y: pos.y },
            })
            .collect(),
        nodes: batch.nodes.iter().map(node_state).collect(),
        economy: wire::EconomyChanges {
            balances: batch
                .economy
                .balances
                .iter()
                .filter(|record| owns_balance(record, slot))
                .map(balance_state)
                .collect(),
            inventories: batch
                .economy
                .inventories
                .iter()
                .filter(|record| record.owner_slot == slot.0)
                .map(inventory_state)
                .collect(),
            orders: batch
                .economy
                .orders
                .iter()
                .filter(|record| record.owner_slot == slot.0)
                .map(order_state)
                .collect(),
            queues: batch
                .economy
                .queues
                .iter()
                .filter(|record| record.owner_slot == slot.0)
                .map(queue_state)
                .collect(),
        },
    }
}

fn owns_balance(record: &BalanceView, slot: SlotId) -> bool {
    record.slot == slot.0
}

fn unit_snapshot(unit: &UnitState) -> messages::UnitSnapshot {
    messages::UnitSnapshot {
        unit_id: messages::UnitId(unit.unit_id),
        content_id: unit.content_id.clone(),
        owner_slot: messages::SlotId(unit.owner_slot),
        pos: messages::Pos {
            x: unit.pos.x,
            y: unit.pos.y,
        },
    }
}

fn node_state(node: &NodeView) -> wire::NodeState {
    wire::NodeState {
        id: wire::NodeId(node.id.0),
        content_id: node.content_id.to_string(),
        pos: messages::Pos {
            x: node.pos.x,
            y: node.pos.y,
        },
        remaining: node.remaining,
    }
}

fn balance_state(balance: &BalanceView) -> wire::BalanceState {
    wire::BalanceState {
        resource: balance.resource.to_string(),
        on_hand: balance.on_hand,
        reserved: balance.reserved,
    }
}

fn item_state(item: &ItemView) -> wire::BalanceState {
    wire::BalanceState {
        resource: item.resource.to_string(),
        on_hand: item.on_hand,
        reserved: item.reserved,
    }
}

fn inventory_state(inventory: &InventoryView) -> wire::InventoryState {
    wire::InventoryState {
        unit: messages::UnitId(inventory.unit),
        items: inventory.items.iter().map(item_state).collect(),
    }
}

fn order_state(order: &OrderView) -> wire::OrderState {
    wire::OrderState {
        unit: messages::UnitId(order.unit),
        node: order.node.map(|id| wire::NodeId(id.0)),
        receiver: order.receiver.map(messages::UnitId),
        stage: match order.stage {
            GatherStage::MovingToNode => wire::GatherStage::ToNode,
            GatherStage::Gathering => wire::GatherStage::Gathering,
            GatherStage::MovingToReceiver => wire::GatherStage::ToDropOff,
            GatherStage::Unloading => wire::GatherStage::Unloading,
            GatherStage::Idle => wire::GatherStage::Idle,
            GatherStage::Waiting { reason } => wire::GatherStage::Waiting {
                reason: match reason {
                    WaitReason::NoReceiver => wire::GatherWaitReason::NoDropOff,
                    WaitReason::NoDeposit => wire::GatherWaitReason::NoDeposit,
                    WaitReason::DestinationFull => wire::GatherWaitReason::DestinationFull,
                    WaitReason::DestinationOverflow => wire::GatherWaitReason::DestinationOverflow,
                },
            },
        },
    }
}

fn queue_state(queue: &QueueView) -> wire::QueueState {
    wire::QueueState {
        producer: messages::UnitId(queue.producer),
        jobs: queue.jobs.iter().map(job_state).collect(),
    }
}

fn job_state(job: &JobView) -> wire::JobState {
    wire::JobState {
        id: wire::JobId(job.id),
        recipe: job.recipe.to_string(),
        stage: match job.stage {
            JobStage::Queued => wire::JobStage::Queued,
            JobStage::Running => wire::JobStage::Running,
            JobStage::OutputBlocked => wire::JobStage::OutputBlocked,
        },
        elapsed_ticks: job.elapsed_ticks,
        duration_ticks: job.duration_ticks,
        charged: job.charged.iter().map(cost).collect(),
        reserved: job.reserved.iter().map(cost).collect(),
    }
}

fn cost(line: &CostView) -> Cost {
    Cost {
        source: line.source,
        resource: line.resource.to_string(),
        amount: line.amount,
    }
}
