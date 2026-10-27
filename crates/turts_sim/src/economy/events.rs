//! Committed domain events and immutable tick exports.

use bevy_ecs::prelude::Resource;
use turts_mod_api::ContentId;

use crate::{
    economy::views::{EconomyChangesView, WorldView},
    world::UnitState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomyEvent {
    pub tick: u32,
    pub ordinal: u64,
    pub owner_slot: u8,
    pub kind: EconomyEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostLine {
    pub source: EventSource,
    pub resource: ContentId,
    pub amount: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSource {
    Slot(u8),
    Inventory(u32),
}

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct CommittedTick {
    pub tick: u32,
    pub spawned: Vec<UnitState>,
    pub removed: Vec<u32>,
    pub positions: Vec<(u32, bevy_math::Vec2)>,
    pub nodes: Vec<crate::economy::views::NodeView>,
    pub economy: EconomyChangesView,
    pub events: Vec<EconomyEvent>,
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
