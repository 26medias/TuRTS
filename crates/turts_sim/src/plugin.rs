//! The simulation plugin.

use bevy_app::{App, FixedPostUpdate, FixedPreUpdate, FixedUpdate, Plugin};
use bevy_ecs::prelude::*;
use bevy_time::{Fixed, Time};
use std::collections::BTreeSet;

use crate::SimClock;
use crate::command::{CommandQueue, CommandResults, drain_commands};
use crate::economy::events::{EconomyEvent, MatchStopped, StopReason};
use crate::economy::views::{economy_changes, world_view};
use crate::match_config::match_started;
use crate::movement::integrate_movement;
use crate::orders::run_orders;
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

/// The sets of the fixed step, in the order in which they run inside one tick (design 6.2, 6.4,
/// 6.7.8; the ruling `ruling-l0-sim-order.md`):
///
/// `FixedPreUpdate`: `Advance`, `Supply`, `Construction`, `Commands`. `FixedUpdate`: `Income`,
/// `Orders`, `Movement`, `Gather`, `Extract`, `Transport`, `Inserters`, `Production`, `Acquire`,
/// `Repair`, `Combat`, `Outcome`. `FixedPostUpdate`: `Collect`.
///
/// `Supply` runs before the commands, so a removal in tick t counts from tick t+1. `Construction`
/// places the held orders before the commands. `Production` is the one pay pass (producers and
/// sites, ascending unit id). `Supply`, `Income`, `Construction`, `Acquire`, `Repair` and
/// `Outcome` are the system slots of plan task `L0-SIM`: each holds one empty system until its
/// lane fills it, and the order above is pinned by the test
/// `the_sets_of_one_tick_run_in_the_pinned_order`. A system that must run between `Advance` and
/// `Commands` (the admission systems of the server) goes `.after(SimSet::Construction)`. Every
/// set has the run condition `match_started`: with no sim `MatchConfig`, no sim system runs and
/// the tick does not advance.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSet {
    Advance,
    Supply,
    Construction,
    Commands,
    Income,
    Orders,
    Movement,
    Gather,
    Extract,
    Transport,
    Inserters,
    Production,
    Acquire,
    Repair,
    Combat,
    Outcome,
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
            .init_resource::<crate::unit_index::UnitIndex>()
            .init_resource::<crate::economy::power::RateQ>()
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
            .init_resource::<crate::movement::MovementPools>()
            .init_resource::<crate::orders::PendingMovementOrders>()
            .init_resource::<crate::orders::MovementDispatchState>()
            .init_resource::<crate::economy::transaction::AuditTrail>()
            .init_resource::<ChangedThisTick>()
            .init_resource::<CommittedTick>()
            .init_resource::<CompletedWorld>()
            .init_resource::<PendingEconomyEvents>()
            .init_resource::<NextEventOrdinal>()
            .init_resource::<SpawnedThisTick>()
            .init_resource::<RemovedThisTick>()
            .init_resource::<crate::economy::views_m3::RemovedHistory>()
            .insert_resource(Slots::new(SLOT_COUNT))
            .insert_resource(SimClock {
                tick_hz: self.tick_hz,
            })
            .configure_sets(
                FixedPreUpdate,
                (
                    SimSet::Advance,
                    SimSet::Supply,
                    SimSet::Construction,
                    SimSet::Commands,
                )
                    .chain()
                    .run_if(match_started),
            )
            .configure_sets(
                FixedUpdate,
                (
                    SimSet::Income,
                    SimSet::Orders,
                    SimSet::Movement,
                    SimSet::Gather,
                    SimSet::Extract,
                    SimSet::Transport,
                    SimSet::Inserters,
                    SimSet::Production,
                    SimSet::Acquire,
                    SimSet::Repair,
                    SimSet::Combat,
                    SimSet::Outcome,
                )
                    .chain()
                    .run_if(match_started),
            )
            .configure_sets(FixedPostUpdate, SimSet::Collect.run_if(match_started))
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
                FixedPreUpdate,
                crate::economy::power::run_supply
                    .in_set(SimSet::Supply)
                    .run_if(not_stopped),
            )
            .add_systems(
                FixedUpdate,
                crate::economy::income::run_income
                    .in_set(SimSet::Income)
                    .run_if(not_stopped),
            )
            .add_systems(
                FixedPreUpdate,
                crate::economy::construction::run_construction
                    .in_set(SimSet::Construction)
                    .run_if(not_stopped),
            )
            .add_systems(
                FixedUpdate,
                crate::economy::acquire::run_acquire
                    .in_set(SimSet::Acquire)
                    .run_if(not_stopped),
            )
            .add_systems(
                FixedUpdate,
                crate::economy::repair::run_repair
                    .in_set(SimSet::Repair)
                    .run_if(not_stopped),
            )
            .add_systems(
                FixedUpdate,
                crate::outcome::run_outcome_check
                    .in_set(SimSet::Outcome)
                    .run_if(not_stopped),
            )
            .add_systems(
                FixedUpdate,
                run_orders.in_set(SimSet::Orders).run_if(not_stopped),
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
    let mut m3_changes = crate::economy::views_m3::m3_changes(
        &world.resource::<CompletedWorld>().0.m3,
        &completed.m3,
    );
    let mut events = std::mem::take(&mut world.resource_mut::<PendingEconomyEvents>().0);
    events.sort_by_key(|event: &EconomyEvent| event.ordinal);
    let mut healths = m3_changes.healths;
    let killed_units: BTreeSet<_> = events
        .iter()
        .filter_map(|event| match &event.kind {
            crate::economy::events::EconomyEventKind::Destroyed {
                unit,
                cause: crate::economy::events::RemoveCause::Killed { .. },
                ..
            } => Some(*unit),
            _ => None,
        })
        .collect();
    for unit in killed_units {
        healths.retain(|row| row.unit != unit);
        healths.push(crate::economy::views_m3::RawHealthAudit { unit, current: 0 });
    }
    healths.sort_by_key(|row| row.unit);
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
    m3_changes
        .enabled
        .retain(|change| !removed.contains(&change.unit));
    let batch = CommittedTick {
        tick,
        spawned,
        removed,
        positions: changed,
        nodes,
        economy: economy_changes,
        output_inventories: m3_changes.output_inventories,
        belts: m3_changes.belts,
        inserters: m3_changes.inserters,
        extractors: m3_changes.extractors,
        weapons: m3_changes.weapons,
        healths,
        research: m3_changes.research,
        enabled: m3_changes.enabled,
        events,
    };
    world.insert_resource(batch);
    world.insert_resource(CompletedWorld(completed));
    world.resource_mut::<NextEventOrdinal>().0 = Some(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headless_app;

    #[derive(Resource, Default)]
    struct Ran(Vec<SimSet>);

    /// The order of the sets inside one tick (design 6.2, 6.7.8; plan task `L0-SIM`; ruling
    /// `ruling-l0-sim-order.md`). Parallel lanes that put a system in a slot depend on this
    /// list, so the test pins it.
    /// `Supply` is the first set after the tick advance: a removal in tick t counts from tick
    /// t+1 (6.2). `Construction` places the held orders before the commands, so a cap freed in
    /// tick t is free for the placement of tick t+1. `Income` is the first set of the economy
    /// phase: it pays after the commands of the tick and before every per-tick charge.
    /// `Production` is the one pay pass, over producers and sites in ascending unit id.
    /// `Acquire` builds the grids, `Repair` heals before `Combat` damages, and `Outcome` runs
    /// after the death path of `Combat` and before `Collect`.
    const TICK_ORDER: [SimSet; 17] = [
        SimSet::Advance,
        SimSet::Supply,
        SimSet::Construction,
        SimSet::Commands,
        SimSet::Income,
        SimSet::Orders,
        SimSet::Movement,
        SimSet::Gather,
        SimSet::Extract,
        SimSet::Transport,
        SimSet::Inserters,
        SimSet::Production,
        SimSet::Acquire,
        SimSet::Repair,
        SimSet::Combat,
        SimSet::Outcome,
        SimSet::Collect,
    ];

    fn mark(app: &mut App, set: SimSet) {
        let system = move |mut ran: ResMut<Ran>| ran.0.push(set);
        match set {
            SimSet::Advance | SimSet::Supply | SimSet::Construction | SimSet::Commands => {
                app.add_systems(FixedPreUpdate, system.in_set(set));
            }
            SimSet::Collect => {
                app.add_systems(FixedPostUpdate, system.in_set(set));
            }
            _ => {
                app.add_systems(FixedUpdate, system.in_set(set));
            }
        }
    }

    #[test]
    fn the_sets_of_one_tick_run_in_the_pinned_order() {
        let mut app = headless_app(20);
        app.init_resource::<Ran>();
        // The marks are added in the reverse of the pinned order, so an unordered set cannot
        // land in the pinned place by the order of registration.
        for set in TICK_ORDER.into_iter().rev() {
            mark(&mut app, set);
        }
        app.update();
        assert_eq!(app.world().resource::<Ran>().0, TICK_ORDER);
    }

    #[test]
    fn each_slot_set_holds_a_system_in_its_schedule() {
        use bevy_ecs::schedule::{ScheduleLabel, SystemSet as _};
        let mut app = headless_app(20);
        app.update();
        let schedules = app.world().resource::<bevy_ecs::schedule::Schedules>();
        // One system at least: the empty system of the slot, or the system of the lane that
        // filled it. A lane that fills a slot may add more, so the test does not pin the count.
        // `Supply` and `Construction` run before the commands, so they are in `FixedPreUpdate`.
        let slots: [(SimSet, bevy_ecs::intern::Interned<dyn ScheduleLabel>); 7] = [
            (SimSet::Supply, FixedPreUpdate.intern()),
            (SimSet::Construction, FixedPreUpdate.intern()),
            (SimSet::Income, FixedUpdate.intern()),
            (SimSet::Acquire, FixedUpdate.intern()),
            (SimSet::Repair, FixedUpdate.intern()),
            (SimSet::Outcome, FixedUpdate.intern()),
            (SimSet::Collect, FixedPostUpdate.intern()),
        ];
        for (slot, label) in slots {
            let schedule = schedules
                .get(label)
                .unwrap_or_else(|| panic!("{slot:?}: no schedule {label:?}"));
            let systems = schedule
                .graph()
                .systems_in_set(slot.intern())
                .unwrap_or_else(|error| panic!("{slot:?} in {label:?}: {error:?}"));
            assert!(
                !systems.is_empty(),
                "the slot {slot:?} holds no system in {label:?}"
            );
        }
    }
}
