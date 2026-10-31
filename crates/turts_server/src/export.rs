//! Pure projection of immutable simulation views into protocol 3 public state.
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
        views_m3::{
            RawBeltAudit, RawExtractorAudit, RawHealthAudit, RawInserterAudit, RawResearchAudit,
            RawWeaponAudit,
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
                .map(|record| inventory_state(record, wire::InventoryPort::Main))
                .chain(
                    view.m3
                        .raw_output_inventories
                        .iter()
                        .filter(|record| owner_of(view, record.unit) == Some(slot.0))
                        .map(output_inventory_state),
                )
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
                .map(|record| queue_state(record, is_enabled(view, record.producer)))
                .collect(),
        },
        belts: view
            .m3
            .raw_belts
            .iter()
            .flat_map(|record| belt_records(record, view, slot))
            .collect(),
        inserters: view
            .m3
            .raw_inserters
            .iter()
            .filter(|record| owner_of(view, record.unit) == Some(slot.0))
            .map(inserter_state)
            .collect(),
        extractors: view
            .m3
            .raw_extractors
            .iter()
            .filter(|record| owner_of(view, record.unit) == Some(slot.0))
            .map(extractor_state)
            .collect(),
        weapons: view
            .m3
            .raw_weapons
            .iter()
            .filter(|record| owner_of(view, record.unit) == Some(slot.0))
            .map(weapon_state)
            .collect(),
        healths: view.m3.raw_healths.iter().map(health_state).collect(),
        research: research_records(&view.m3.research, Some(slot)),
        unit_orders: view
            .unit_orders
            .iter()
            .filter(|record| record.owner_slot == slot.0)
            .map(unit_order_state)
            .collect(),
    }
}

/// Build the owner-specific absolute replacements for one committed simulation tick.
pub fn delta_for(batch: &CommittedTick, view: &WorldView, slot: SlotId) -> wire::WorldDelta {
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
                .map(|record| inventory_state(record, wire::InventoryPort::Main))
                .chain(
                    batch
                        .output_inventories
                        .iter()
                        .filter(|record| owner_of(view, record.unit) == Some(slot.0))
                        .map(output_inventory_state),
                )
                .collect(),
            orders: batch
                .economy
                .orders
                .iter()
                .filter(|record| record.owner_slot == slot.0)
                .map(order_state)
                .collect(),
            queues: queue_replacements(batch, view, slot),
        },
        belts: batch
            .belts
            .iter()
            .flat_map(|record| belt_records(record, view, slot))
            .collect(),
        inserters: batch
            .inserters
            .iter()
            .filter(|record| owner_of(view, record.unit) == Some(slot.0))
            .map(inserter_state)
            .collect(),
        extractors: batch
            .extractors
            .iter()
            .filter(|record| owner_of(view, record.unit) == Some(slot.0))
            .map(extractor_state)
            .collect(),
        weapons: batch
            .weapons
            .iter()
            .filter(|record| owner_of(view, record.unit) == Some(slot.0))
            .map(weapon_state)
            .collect(),
        healths: batch.healths.iter().map(health_state).collect(),
        research: research_records(&view.m3.research, Some(slot))
            .into_iter()
            .filter(|record| {
                batch
                    .research
                    .iter()
                    .any(|changed| changed.slot == record.slot.0)
            })
            .collect(),
        unit_orders: view
            .unit_orders
            .iter()
            .filter(|record| record.owner_slot == slot.0)
            .map(unit_order_state)
            .collect(),
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
        facing: unit.facing,
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

fn inventory_state(inventory: &InventoryView, port: wire::InventoryPort) -> wire::InventoryState {
    wire::InventoryState {
        unit: messages::UnitId(inventory.unit),
        port,
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

fn queue_state(queue: &QueueView, enabled: bool) -> wire::QueueState {
    wire::QueueState {
        producer: messages::UnitId(queue.producer),
        enabled,
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

fn owner_of(view: &WorldView, unit: u32) -> Option<u8> {
    view.units
        .iter()
        .find(|row| row.unit_id == unit)
        .map(|row| row.owner_slot)
}

fn is_enabled(view: &WorldView, unit: u32) -> bool {
    view.m3.disabled.binary_search(&unit).is_err()
}

fn queue_replacements(
    batch: &CommittedTick,
    view: &WorldView,
    slot: SlotId,
) -> Vec<wire::QueueState> {
    let mut producers: std::collections::BTreeSet<u32> = batch
        .economy
        .queues
        .iter()
        .filter(|row| row.owner_slot == slot.0)
        .map(|row| row.producer)
        .collect();
    producers.extend(
        batch
            .enabled
            .iter()
            .filter(|row| owner_of(view, row.unit) == Some(slot.0))
            .map(|row| row.unit),
    );
    producers
        .into_iter()
        .filter_map(|producer| {
            let changed = batch
                .economy
                .queues
                .iter()
                .find(|row| row.producer == producer && row.owner_slot == slot.0);
            changed
                .or_else(|| {
                    view.economy
                        .queues
                        .iter()
                        .find(|row| row.producer == producer && row.owner_slot == slot.0)
                })
                .map(|row| queue_state(row, is_enabled(view, producer)))
        })
        .collect()
}

fn output_inventory_state(
    row: &turts_sim::economy::views_m3::RawOutputInventoryAudit,
) -> wire::InventoryState {
    wire::InventoryState {
        unit: messages::UnitId(row.unit),
        port: wire::InventoryPort::Output,
        items: row
            .items
            .iter()
            .map(|(resource, account)| wire::BalanceState {
                resource: resource.to_string(),
                on_hand: account.on_hand,
                reserved: account.reserved,
            })
            .collect(),
    }
}

fn belt_records<'a>(
    row: &'a RawBeltAudit,
    view: &'a WorldView,
    slot: SlotId,
) -> impl Iterator<Item = wire::BeltPieceRecord> + 'a {
    row.pieces
        .iter()
        .filter(move |piece| owner_of(view, piece.unit) == Some(slot.0))
        .map(|piece| wire::BeltPieceRecord {
            piece: messages::UnitId(piece.unit),
            slots: piece
                .slots
                .iter()
                .map(|slot| slot.item.as_ref().map(ToString::to_string))
                .collect(),
        })
}

fn inserter_state(row: &RawInserterAudit) -> wire::InserterRecord {
    use turts_sim::economy::views_m3::{RawInserterStageAudit as S, RawInserterWaitAudit as W};
    let wait = |reason| match reason {
        W::NoSource => wire::InserterWaitReason::NoSource,
        W::NoDestination => wire::InserterWaitReason::NoDestination,
        W::SourceEmpty => wire::InserterWaitReason::SourceEmpty,
        W::DestinationFull => wire::InserterWaitReason::DestinationFull,
    };
    wire::InserterRecord {
        unit: messages::UnitId(row.unit),
        enabled: row.enabled,
        stage: match row.stage {
            S::AtSource(reason) => wire::InserterStage::AtSource(reason.map(wait)),
            S::ToDestination => wire::InserterStage::ToDestination,
            S::AtDestination(reason) => wire::InserterStage::AtDestination(wait(reason)),
            S::Returning => wire::InserterStage::Returning,
        },
        elapsed: row.elapsed,
        source: row.source.map(messages::UnitId),
        destination: row.destination.map(messages::UnitId),
    }
}

fn extractor_state(row: &RawExtractorAudit) -> wire::ExtractorRecord {
    use turts_sim::economy::views_m3::RawExtractorStatusAudit as S;
    wire::ExtractorRecord {
        unit: messages::UnitId(row.unit),
        enabled: row.enabled,
        node: row.node.map(wire::NodeId),
        status: match row.status {
            S::Extracting => wire::ExtractorStatus::Extracting,
            S::Full => wire::ExtractorStatus::Full,
            S::Depleted => wire::ExtractorStatus::Depleted,
        },
    }
}

fn weapon_state(row: &RawWeaponAudit) -> wire::WeaponRecord {
    use turts_sim::economy::views_m3::RawWeaponStatusAudit as S;
    wire::WeaponRecord {
        unit: messages::UnitId(row.unit),
        enabled: row.enabled,
        cooldown: row.cooldown,
        damage: row.effective_damage.map_err(command_reject_reason),
        status: match row.status {
            S::Ready => wire::WeaponStatus::Ready,
            S::Cooling => wire::WeaponStatus::Cooling,
            S::NoTarget => wire::WeaponStatus::NoTarget,
            S::OutOfAmmo => wire::WeaponStatus::OutOfAmmo,
        },
        last_target: row.last_target.map(messages::UnitId),
    }
}

fn command_reject_reason(reason: turts_sim::RejectReason) -> wire::CommandRejectReason {
    use turts_sim::RejectReason as R;
    match reason {
        R::EmptyCommand => wire::CommandRejectReason::EmptyCommand,
        R::TooManyUnits => wire::CommandRejectReason::TooManyUnits,
        R::DuplicateUnit => wire::CommandRejectReason::DuplicateUnit,
        R::UnknownUnit => wire::CommandRejectReason::UnknownUnit,
        R::NotOwner => wire::CommandRejectReason::NotOwner,
        R::NotMobile => wire::CommandRejectReason::NotMobile,
        R::TargetNotFinite => wire::CommandRejectReason::TargetNotFinite,
        R::TargetOutsideMap => wire::CommandRejectReason::TargetOutsideMap,
        R::MissingCapability => wire::CommandRejectReason::MissingCapability,
        R::UnknownNode => wire::CommandRejectReason::UnknownNode,
        R::IncompatibleItem => wire::CommandRejectReason::IncompatibleItem,
        R::NodeDepleted => wire::CommandRejectReason::NodeDepleted,
        R::UnknownRecipe => wire::CommandRejectReason::UnknownRecipe,
        R::RecipeNotEnabled => wire::CommandRejectReason::RecipeNotEnabled,
        R::QueueFull => wire::CommandRejectReason::QueueFull,
        R::InsufficientBalance | R::UnknownBalance => {
            wire::CommandRejectReason::InsufficientBalance
        }
        R::InsufficientItems | R::UnknownInventory | R::InvalidAmount => {
            wire::CommandRejectReason::InsufficientItems
        }
        R::UnknownJob => wire::CommandRejectReason::UnknownJob,
        R::RefundDestinationFull => wire::CommandRejectReason::RefundDestinationFull,
        R::RefundDestinationOverflow => wire::CommandRejectReason::RefundDestinationOverflow,
        R::ArithmeticOverflow => wire::CommandRejectReason::ArithmeticOverflow,
        R::IdExhausted => wire::CommandRejectReason::IdExhausted,
        R::BadSequence => wire::CommandRejectReason::BadSequence,
        R::NotRemovable => wire::CommandRejectReason::NotRemovable,
        R::NotToggleable => wire::CommandRejectReason::NotToggleable,
        R::PrerequisitesMissing => wire::CommandRejectReason::PrerequisitesMissing,
        R::AlreadyResearched => wire::CommandRejectReason::AlreadyResearched,
        R::ResearchInProgress => wire::CommandRejectReason::ResearchInProgress,
        R::IntegrityViolation => wire::CommandRejectReason::Internal,
        R::NotApplicable => wire::CommandRejectReason::NotApplicable,
        R::TargetInvalid => wire::CommandRejectReason::TargetInvalid,
        R::TargetGone => wire::CommandRejectReason::TargetGone,
        R::OutOfRange => wire::CommandRejectReason::OutOfRange,
        R::NotQueueable => wire::CommandRejectReason::NotQueueable,
        R::Blocked => wire::CommandRejectReason::Blocked,
    }
}

fn unit_order_state(record: &turts_sim::economy::views::UnitOrderView) -> wire::UnitOrderRecord {
    use turts_sim::economy::views::{
        UnitOrderActivityView as Activity, UnitOrderSummaryView as Summary,
        UnitOrderTargetView as Target,
    };
    let summary = |summary: &Summary| wire::OrderSummary {
        order: summary.order.clone(),
        target: match &summary.target {
            Target::None => wire::OrderTarget::None,
            Target::Point(point) => wire::OrderTarget::Point(messages::Pos {
                x: point.x,
                y: point.y,
            }),
            Target::Unit(unit) => wire::OrderTarget::Unit(messages::UnitId(*unit)),
            Target::Node(node) => wire::OrderTarget::Node(messages::NodeId(node.0)),
            Target::Job(job) => wire::OrderTarget::Job(messages::JobId(job.0)),
            Target::Content(content) => wire::OrderTarget::Content(content.to_string()),
        },
    };
    wire::UnitOrderRecord {
        unit: messages::UnitId(record.unit_id),
        activity: match record.activity {
            Activity::Idle => wire::Activity::Idle,
            Activity::Pending => wire::Activity::Pending,
            Activity::Moving => wire::Activity::Moving,
            Activity::Attacking => wire::Activity::Attacking,
            Activity::Holding => wire::Activity::Holding,
            Activity::Gathering => wire::Activity::Gathering,
            Activity::Blocked => wire::Activity::Blocked,
        },
        current: record.current.as_ref().map(summary),
        queued: record.queued.iter().map(summary).collect(),
        last_failure: record
            .last_failure
            .as_ref()
            .map(|failure| wire::OrderFailure {
                order: failure.order.clone(),
                reason: command_reject_reason(failure.reason),
                tick: failure.tick,
            }),
    }
}

fn health_state(row: &RawHealthAudit) -> wire::HealthRecord {
    wire::HealthRecord {
        unit: messages::UnitId(row.unit),
        current: row.current,
    }
}

fn research_records(
    rows: &[RawResearchAudit],
    only_slot: Option<SlotId>,
) -> Vec<wire::ResearchRecord> {
    let mut by_slot = std::collections::BTreeMap::<u8, Vec<String>>::new();
    for row in rows
        .iter()
        .filter(|row| only_slot.is_none_or(|slot| row.slot == slot.0))
    {
        by_slot
            .entry(row.slot)
            .or_default()
            .push(row.technology.to_string());
    }
    by_slot
        .into_iter()
        .map(|(slot, completed)| wire::ResearchRecord {
            slot: messages::SlotId(slot),
            completed,
        })
        .collect()
}

fn cost(line: &CostView) -> Cost {
    Cost {
        source: line.source,
        resource: line.resource.to_string(),
        amount: line.amount,
    }
}
