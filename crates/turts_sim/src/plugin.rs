//! The simulation plugin.

use bevy_app::{App, FixedPostUpdate, FixedPreUpdate, FixedUpdate, Plugin};
use bevy_ecs::prelude::*;
use bevy_time::{Fixed, Time};

use crate::command::{CommandQueue, CommandResults, drain_commands};
use crate::movement::integrate_movement;
use crate::registry::UnitTypes;
use crate::slots::{SLOT_COUNT, Slots};
use crate::world::{ChangedThisTick, Tick, collect_changes};

/// Adds the simulation to an app that has `bevy_time::TimePlugin`. `MapBounds` is not inserted
/// here: `load_map` inserts it.
pub struct SimPlugin {
    pub tick_hz: u16,
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        assert!(self.tick_hz > 0, "tick_hz must be above 0");
        app.insert_resource(Time::<Fixed>::from_hz(f64::from(self.tick_hz)))
            .init_resource::<Tick>()
            .init_resource::<UnitTypes>()
            .init_resource::<CommandQueue>()
            .init_resource::<CommandResults>()
            .init_resource::<ChangedThisTick>()
            .insert_resource(Slots::new(SLOT_COUNT))
            .add_systems(FixedPreUpdate, (advance_tick, drain_commands).chain())
            .add_systems(FixedUpdate, integrate_movement)
            .add_systems(FixedPostUpdate, collect_changes);
    }
}

/// FixedPreUpdate: count the tick.
fn advance_tick(mut tick: ResMut<Tick>) {
    tick.0 += 1;
}
