//! The power seams of design 6.2 (plan task `L0-SIM`, ruling R-X).
//!
//! Lane 0 puts three things here so that the power lane (`L-PWR`) edits this file only:
//!
//! - the system slot `run_supply` (`SimSet::Supply`), where the one supply computation of each
//!   tick will run;
//! - `weapon_enabled`, called by `combat.rs` at the fire check and at the targeting step;
//! - `rate_factor`, called by `production.rs` where a job advances (and by the constructor where
//!   a site advances).
//!
//! The bodies are stubs: a weapon is enabled and the rate is 1/1. They go through
//! `turts_rules::supply`, the one copy of the shortage rule, with "no `when_short` entry and no
//! shortage". `L-PWR` replaces the arguments with the slot supply result and the `when_short`
//! entry of the capability. No rule of power is written here.

use bevy_ecs::world::World;
use turts_rules::Ratio;

/// The system slot of the supply computation: the first set of the economy phase (it runs
/// before every removal path of the tick, so a removal in tick t counts at tick t+1, design
/// 6.2). Empty until `L-PWR`.
pub(crate) fn run_supply() {}

/// False when the weapon of `unit` is disabled by a supply shortage: it neither fires nor
/// acquires (design 6.2, `Disable`). The stub says "enabled".
pub(crate) fn weapon_enabled(world: &World, unit: u32) -> bool {
    #[cfg(test)]
    if let Some(probe) = world.get_resource::<PowerProbe>() {
        return probe.weapon_enabled;
    }
    let _ = (world, unit);
    turts_rules::supply::weapon_enabled(None, false)
}

/// The rate at which a job of the producer `unit` (or a site of the constructor `unit`)
/// advances: 1/1 at full rate, the `Rate(ratio)` of its `when_short` entry in a shortage
/// (design 6.2). The stub says "full rate".
pub(crate) fn rate_factor(world: &World, unit: u32) -> Ratio {
    #[cfg(test)]
    if let Some(probe) = world.get_resource::<PowerProbe>() {
        return probe.rate;
    }
    let _ = (world, unit);
    turts_rules::supply::rate_factor(None, false)
}

/// Test hook: a world with this resource gets its answers from it, so a test can show that a
/// call site reads the stub. It exists in test builds of this crate only.
#[cfg(test)]
#[derive(bevy_ecs::prelude::Resource, Debug, Clone, Copy)]
pub(crate) struct PowerProbe {
    pub(crate) weapon_enabled: bool,
    pub(crate) rate: Ratio,
}
