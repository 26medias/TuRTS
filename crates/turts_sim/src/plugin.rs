//! The simulation plugin.

use bevy_app::{App, FixedPostUpdate, FixedPreUpdate, FixedUpdate, Plugin};
use bevy_ecs::prelude::*;
use bevy_time::{Fixed, Time};
use std::collections::BTreeSet;

use crate::SimClock;
use crate::command::{CommandQueue, CommandResults, drain_commands};
use crate::economy::events::{EconomyEvent, MatchStopped, StopReason};
use crate::economy::views::{economy_changes, world_view};
use crate::movement::integrate_movement;
use crate::registry::UnitTypes;
use crate::slots::{SLOT_COUNT, Slots};
use crate::world::snapshot;
use crate::world::{ChangedThisTick, Tick, collect_changes};
use crate::{
    economy::{
        EconomyOrders, SurfaceNodes,
        events::{
            CommittedTick, CompletedWorld, NextEventOrdinal, PendingEconomyEvents, RemovedThisTick,
            SpawnedThisTick, StopDiagnostic,
        },
        ledger::EconomyLedger,
        production::{ProductionQueues, run_production},
    },
    ids::IdAllocators,
};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSet {
    Advance,
    Commands,
    Movement,
    Gather,
    Extract,
    Transport,
    Inserters,
    Production,
    Combat,
    Collect,
}

/// Adds the simulation to an app that has `bevy_time::TimePlugin`. `MapBounds` is not inserted
/// here: `load_map` inserts it.
pub struct SimPlugin {
    pub tick_hz: u16,
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        assert!(self.tick_hz > 0, "tick_hz must be above 0");
        crate::economy::extractor::install(app);
        crate::economy::conveyor::install(app);
        crate::economy::inserter::install(app);
        crate::economy::combat::install(app);
        app.insert_resource(Time::<Fixed>::from_hz(f64::from(self.tick_hz)))
            .init_resource::<Tick>()
            .init_resource::<UnitTypes>()
            .init_resource::<IdAllocators>()
            .init_resource::<EconomyLedger>()
            .init_resource::<crate::economy::conveyor::Conveyors>()
            .init_resource::<crate::economy::conveyor::ItemTable>()
            .init_resource::<crate::economy::inserter::Inserters>()
            .init_resource::<crate::economy::extractor::Extractors>()
            .init_resource::<crate::economy::combat::Weapons>()
            .init_resource::<crate::economy::combat::Healths>()
            .init_resource::<crate::economy::research::ResearchState>()
            .init_resource::<crate::economy::research::Disabled>()
            .init_resource::<SurfaceNodes>()
            .init_resource::<EconomyOrders>()
            .init_resource::<ProductionQueues>()
            .init_resource::<CommandQueue>()
            .init_resource::<CommandResults>()
            .init_resource::<crate::economy::transaction::AuditTrail>()
            .init_resource::<ChangedThisTick>()
            .init_resource::<CommittedTick>()
            .init_resource::<CompletedWorld>()
            .init_resource::<PendingEconomyEvents>()
            .init_resource::<NextEventOrdinal>()
            .init_resource::<SpawnedThisTick>()
            .init_resource::<RemovedThisTick>()
            .insert_resource(Slots::new(SLOT_COUNT))
            .insert_resource(SimClock {
                tick_hz: self.tick_hz,
            })
            .configure_sets(FixedPreUpdate, (SimSet::Advance, SimSet::Commands).chain())
            .configure_sets(
                FixedUpdate,
                (
                    SimSet::Movement,
                    SimSet::Gather,
                    SimSet::Extract,
                    SimSet::Transport,
                    SimSet::Inserters,
                    SimSet::Production,
                    SimSet::Combat,
                )
                    .chain(),
            )
            .configure_sets(FixedPostUpdate, SimSet::Collect)
            .add_systems(
                FixedPreUpdate,
                crate::economy::transaction::begin_tick_audit
                    .in_set(SimSet::Advance)
                    .before(advance_tick),
            )
            .add_systems(FixedPreUpdate, advance_tick.in_set(SimSet::Advance))
            .add_systems(
                FixedPreUpdate,
                drain_commands.in_set(SimSet::Commands).after(advance_tick),
            )
            .add_systems(
                FixedUpdate,
                integrate_movement
                    .in_set(SimSet::Movement)
                    .run_if(not_stopped),
            )
            .add_systems(
                FixedUpdate,
                crate::economy::gather::run_gather
                    .in_set(SimSet::Gather)
                    .run_if(not_stopped),
            )
            .add_systems(
                FixedUpdate,
                run_production
                    .in_set(SimSet::Production)
                    .run_if(not_stopped),
            )
            .add_systems(
                FixedPostUpdate,
                (
                    collect_changes,
                    publish_completed_tick,
                    crate::economy::transaction::finish_tick_audit,
                )
                    .chain()
                    .in_set(SimSet::Collect)
                    .run_if(not_stopped),
            );
    }
}

/// FixedPreUpdate: count the tick.
fn advance_tick(world: &mut World) {
    if world.contains_resource::<MatchStopped>() {
        return;
    }
    let next = world.resource::<Tick>().0.checked_add(1);
    if let Some(tick) = next {
        world.resource_mut::<Tick>().0 = tick;
    } else {
        world.insert_resource(MatchStopped {
            reason: StopReason::TickExhausted,
        });
        world.insert_resource(StopDiagnostic(
            StopReason::TickExhausted.as_str().to_string(),
        ));
    }
}

fn not_stopped(stopped: Option<Res<MatchStopped>>) -> bool {
    stopped.is_none()
}

fn publish_completed_tick(world: &mut World) {
    let tick = world.resource::<Tick>().0;
    let changed = world.resource::<ChangedThisTick>().0.clone();
    let mut completed = world_view(world);
    completed.tick = tick;
    completed.units = snapshot(world);
    let nodes = completed.nodes.clone();
    let economy_changes = economy_changes(
        &world.resource::<CompletedWorld>().0.economy,
        &completed.economy,
    );
    let mut events = std::mem::take(&mut world.resource_mut::<PendingEconomyEvents>().0);
    events.sort_by_key(|event: &EconomyEvent| event.ordinal);
    let mut spawned = std::mem::take(&mut world.resource_mut::<SpawnedThisTick>().0);
    let mut removed = std::mem::take(&mut world.resource_mut::<RemovedThisTick>().0);
    let overlap = removed
        .iter()
        .copied()
        .filter(|unit| spawned.iter().any(|state| state.unit_id == *unit))
        .collect::<BTreeSet<_>>();
    debug_assert!(
        overlap.is_empty(),
        "a unit cannot be spawned and removed in one tick"
    );
    spawned.retain(|state| !overlap.contains(&state.unit_id));
    removed.retain(|unit| !overlap.contains(unit));
    removed.sort_unstable();
    let batch = CommittedTick {
        tick,
        spawned,
        removed,
        positions: changed,
        nodes,
        economy: economy_changes,
        events,
    };
    world.insert_resource(batch);
    world.insert_resource(CompletedWorld(completed));
    world.resource_mut::<NextEventOrdinal>().0 = Some(0);
}
