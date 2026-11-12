//! Committed domain events and immutable tick exports.

use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use turts_mod_api::ContentId;

use crate::{
    economy::{
        views::{EconomyChangesView, UnitOrderView, WorldView},
        views_m3::{
            RawBeltAudit, RawEnabledAudit, RawExtractorAudit, RawHealthAudit, RawInserterAudit,
            RawOutputInventoryAudit, RawResearchAudit, RawWeaponAudit,
        },
    },
    world::UnitState,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EconomyEvent {
    pub tick: u32,
    pub ordinal: u64,
    pub owner_slot: u8,
    pub kind: EconomyEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EconomyEventKind {
    Extracted {
        node: u32,
        cargo: u32,
        item: ContentId,
        amount: u64,
    },
    Stored {
        from: u32,
        to: u32,
        item: ContentId,
        amount: u64,
    },
    Converted {
        from: u32,
        to: u32,
        item: ContentId,
        amount: u64,
        balance: ContentId,
        credited: u64,
    },
    ProductionQueued {
        producer: u32,
        job: u64,
        recipe: ContentId,
    },
    ProductionStarted {
        producer: u32,
        job: u64,
        charged: Vec<CostLine>,
    },
    ProductionCompleted {
        producer: u32,
        job: u64,
        output_unit_id: u32,
    },
    ProductionCancelled {
        producer: u32,
        job: u64,
        released: Vec<CostLine>,
        refunded: Vec<CostLine>,
    },
    Destroyed {
        unit: u32,
        cause: RemoveCause,
        items: Vec<DestroyedLine>,
    },
    ItemsProduced {
        machine: u32,
        job: u64,
        outputs: Vec<CostLine>,
    },
    ResearchCompleted {
        station: u32,
        job: u64,
        slot: u8,
        technology: ContentId,
    },
    Fired {
        weapon: u32,
        target: u32,
        item: Option<ContentId>,
        ammo: u64,
        damage: u64,
        applied: u64,
    },
    EnabledChanged {
        unit: u32,
        enabled: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoveCause {
    Removed,
    Killed { by: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DestroyedLocation {
    Inventory(u32),
    Output(u32),
    BeltPiece(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestroyedLine {
    pub location: DestroyedLocation,
    pub item: ContentId,
    pub amount: u64,
}

/// Unit ids removed by the current tick; publication drains this into `CommittedTick`.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RemovedThisTick(pub(crate) Vec<u32>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostLine {
    pub source: EventSource,
    pub resource: ContentId,
    pub amount: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSource {
    Slot(u8),
    Inventory(u32),
    Output(u32),
}

/// The job state of a site or a job record (design 1.1). The shape is frozen in lane 0; the
/// writers are empty until `L-PAY`, `L-CON1` and `L-PWR`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordStateRow {
    Running,
    Slowed,
    WaitingForFunds(ContentId),
    OutputBlocked,
    Held,
}

/// One rate record of a site or a production job (design 6.8). `unit` is the site, or the
/// producer when `job` is `Some`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateRecordRow {
    pub unit: u32,
    pub owner_slot: u8,
    pub job: Option<u64>,
    pub done: u64,
    pub total: u64,
    pub per_tick: u64,
    pub at_tick: u32,
    pub state: RecordStateRow,
    pub charged: Vec<crate::economy::views::CostView>,
    pub held: bool,
    pub held_since_tick: u32,
}

/// The held `Construct` orders of one constructor (design 6.1), complete.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructionQueueRow {
    pub constructor: u32,
    pub owner_slot: u8,
    pub orders: Vec<(ContentId, bevy_math::Vec2)>,
}

/// The supply of one slot and one resource (design 6.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplyRow {
    pub slot: u8,
    pub resource: ContentId,
    pub provided: u64,
    pub consumed: u64,
}

/// One effect of the tick (design 6.8). `Credited` belongs to the slot `owner_slot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectRow {
    Fired {
        unit: u32,
        target: u32,
        weapon: u8,
    },
    Destroyed {
        unit: u32,
    },
    Credited {
        unit: u32,
        owner_slot: u8,
        resource: ContentId,
        amount: u64,
    },
}

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct CommittedTick {
    pub tick: u32,
    pub spawned: Vec<UnitState>,
    pub removed: Vec<u32>,
    pub positions: Vec<(u32, bevy_math::Vec2)>,
    pub nodes: Vec<crate::economy::views::NodeView>,
    pub economy: EconomyChangesView,
    pub output_inventories: Vec<RawOutputInventoryAudit>,
    pub belts: Vec<RawBeltAudit>,
    pub inserters: Vec<RawInserterAudit>,
    pub extractors: Vec<RawExtractorAudit>,
    pub weapons: Vec<RawWeaponAudit>,
    pub healths: Vec<RawHealthAudit>,
    pub research: Vec<RawResearchAudit>,
    pub enabled: Vec<RawEnabledAudit>,
    pub events: Vec<EconomyEvent>,
    /// The complete record of each unit whose order record changed in this tick, in unit id
    /// order (design 6.8, `unit_orders` as changes). The writer is `orders::writer`.
    pub unit_orders: Vec<UnitOrderView>,
    /// The units whose order record left the view, with their owner slot. A unit that this tick
    /// removes is not listed.
    pub orders_cleared: Vec<(u32, u8)>,
    /// Sites that completed in this tick. The writer is `L-CON1`; it is empty in lane 0.
    pub sites_completed: Vec<u32>,
    /// The site records that changed. Empty in lane 0 (`L-CON1`).
    pub site_progress: Vec<RateRecordRow>,
    /// The job records that changed. Empty in lane 0 (`L-PAY`).
    pub job_progress: Vec<RateRecordRow>,
    /// The constructor queues that changed. Empty in lane 0 (`L-CON1`).
    pub construction_queues: Vec<ConstructionQueueRow>,
    /// The supply rows that changed. Empty in lane 0 (`L-PWR`).
    pub supply: Vec<SupplyRow>,
    /// The effects of this tick. Empty in lane 0 (`L-FX-WIRE`).
    pub effects: Vec<EffectRow>,
}

pub fn committed_tick(world: &bevy_ecs::world::World) -> &CommittedTick {
    world.resource::<CommittedTick>()
}

pub fn completed_world(world: &bevy_ecs::world::World) -> &WorldView {
    &world.resource::<CompletedWorld>().0
}

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct CompletedWorld(pub WorldView);

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct PendingEconomyEvents(pub Vec<EconomyEvent>);

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NextEventOrdinal(pub Option<u64>);

impl Default for NextEventOrdinal {
    fn default() -> Self {
        Self(Some(0))
    }
}

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct SpawnedThisTick(pub Vec<UnitState>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    TickExhausted,
}

impl StopReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TickExhausted => "tick_exhausted",
        }
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchStopped {
    pub reason: StopReason,
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct StopDiagnostic(pub String);

/// Append one already committed domain operation to this tick's immutable event batch.
pub fn record_committed_event(
    world: &mut bevy_ecs::world::World,
    owner_slot: u8,
    kind: EconomyEventKind,
) -> Result<u64, crate::RejectReason> {
    let ordinal = world
        .resource::<NextEventOrdinal>()
        .0
        .ok_or(crate::RejectReason::ArithmeticOverflow)?;
    let next = ordinal.checked_add(1);
    world.resource_mut::<NextEventOrdinal>().0 = next;
    let tick = world.resource::<crate::world::Tick>().0;
    world
        .resource_mut::<PendingEconomyEvents>()
        .0
        .push(EconomyEvent {
            tick,
            ordinal,
            owner_slot,
            kind,
        });
    Ok(ordinal)
}
