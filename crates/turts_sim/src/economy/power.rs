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

/// The system slot of the supply computation: the first set after the tick advance, in
/// `FixedPreUpdate` before the commands (it runs before every removal path of the tick, so a
/// removal in tick t counts at tick t+1, design 6.2). Empty until `L-PWR`.
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

/// The full rate (1/1): the rate of a machine, of a research station and of a producer with no
/// shortage. The view-level rate of the wire uses it until `L-PWR`.
pub(crate) fn full_rate() -> Ratio {
    turts_rules::supply::rate_factor(None, false)
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

/// Q, the unit of job progress (design 6.4, "Progress in integers"): the least common multiple of
/// every `Rate` denominator in the catalog. A job has `total = duration_ticks x Q`, and a tick
/// adds `rate_step(Q, rate)`, a whole number. It is 1 while no capability has a `when_short`
/// `Rate` (every catalog today), so each tick adds 1 at full rate. `L-PWR` sets it at catalog
/// install; the packer refuses a Q above 65,536.
#[derive(bevy_ecs::prelude::Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RateQ(pub(crate) u64);

impl Default for RateQ {
    fn default() -> Self {
        Self(1)
    }
}

/// The Q of this world (1 when the world has no `RateQ`).
pub(crate) fn rate_q(world: &World) -> u64 {
    world.get_resource::<RateQ>().map_or(1, |q| q.0)
}

/// Test hook: a world with this resource gets its answers from it, so a test can show that a
/// call site reads the stub. It exists in test builds of this crate only.
#[cfg(test)]
#[derive(bevy_ecs::prelude::Resource, Debug, Clone, Copy)]
pub(crate) struct PowerProbe {
    pub(crate) weapon_enabled: bool,
    pub(crate) rate: Ratio,
}
