//! The sim `MatchConfig` and the sim gate (design 6.7.5 and 6.7.11, erratum E12, rule R17).
//!
//! The room owns the match lifecycle. The sim gets one input from it before the start: this
//! resource. Every `SimSet` has the run condition `match_started` (`plugin.rs`): without the
//! resource, the tick does not advance and no sim system runs.
//!
//! Who inserts it:
//!
//! - `SimPlugin` inserts no config. A harness with no config gets 0 ticks and fails loudly.
//! - The test helper `headless_app` inserts `MatchConfig::immediate` in one line.
//! - The server inserts it by its start policy: at boot under `Immediate`, in the start step
//!   under a lobby policy.

use bevy_ecs::{prelude::Resource, system::Res};
use serde::{Deserialize, Serialize};

use crate::slots::SLOT_COUNT;

/// One lobby setting. MP1 has none (the mod settings schema is M9, design 6.7.11), so the type
/// has no value and `MatchConfig::settings` can only be empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchSetting {}

/// The part of the match configuration that the sim reads: `{ tick_hz, start_tick, teams,
/// settings }`. Colour never enters the sim, and the config holds no player id.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchConfig {
    /// Ticks in each second. The same value as `SimPlugin::tick_hz`.
    pub tick_hz: u16,
    /// The tick of the first step after the config exists. The first step moves the sim from
    /// tick 0 to tick 1, so the value is 1 under every start policy of MP1.
    pub start_tick: u32,
    /// The team of each slot, indexed by slot number (`turts_rules::relation::Teams`).
    pub teams: Vec<u8>,
    pub settings: Vec<MatchSetting>,
}

impl MatchConfig {
    /// The `Immediate` start policy: the match starts at once, every slot is its own team
    /// (`team = slot`), and no setting exists.
    pub fn immediate(tick_hz: u16) -> Self {
        Self {
            tick_hz,
            start_tick: 1,
            teams: (0..SLOT_COUNT).collect(),
            settings: Vec::new(),
        }
    }
}

/// The run condition of every `SimSet`: the sim `MatchConfig` exists.
pub fn match_started(config: Option<Res<MatchConfig>>) -> bool {
    config.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SimSet, Tick, headless_app};
    use bevy_app::{App, FixedPostUpdate, FixedPreUpdate, FixedUpdate};
    use bevy_ecs::prelude::*;
    use bevy_time::{TimePlugin, TimeUpdateStrategy};

    /// An app built by hand, as a harness with no `headless_app` would build it.
    fn bare_sim_app() -> App {
        let mut app = App::new();
        app.add_plugins((TimePlugin, crate::SimPlugin { tick_hz: 20 }));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        app
    }

    #[test]
    fn with_no_config_the_tick_does_not_advance_and_no_system_runs() {
        #[derive(Resource, Default)]
        struct Ran(Vec<&'static str>);
        let mut app = bare_sim_app();
        app.init_resource::<Ran>();
        for (name, set, schedule) in [
            ("advance", SimSet::Advance, 0),
            ("commands", SimSet::Commands, 0),
            ("orders", SimSet::Orders, 1),
            ("combat", SimSet::Combat, 1),
            ("collect", SimSet::Collect, 2),
        ] {
            let system = move |mut ran: ResMut<Ran>| ran.0.push(name);
            match schedule {
                0 => app.add_systems(FixedPreUpdate, system.in_set(set)),
                1 => app.add_systems(FixedUpdate, system.in_set(set)),
                _ => app.add_systems(FixedPostUpdate, system.in_set(set)),
            };
        }
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(app.world().resource::<Tick>().0, 0, "the tick advanced");
        assert!(
            app.world().resource::<Ran>().0.is_empty(),
            "a SimSet system ran with no config: {:?}",
            app.world().resource::<Ran>().0
        );
        // The config starts the match: the next update is tick 1.
        app.insert_resource(MatchConfig::immediate(20));
        app.update();
        assert_eq!(app.world().resource::<Tick>().0, 1);
        assert_eq!(
            app.world().resource::<Ran>().0,
            ["advance", "commands", "orders", "combat", "collect"]
        );
    }

    #[test]
    fn headless_app_inserts_the_immediate_config_and_ticks() {
        let mut app = headless_app(20);
        assert_eq!(
            app.world().get_resource::<MatchConfig>(),
            Some(&MatchConfig::immediate(20)),
            "headless_app must insert the Immediate config"
        );
        for expected in 1..=3 {
            app.update();
            assert_eq!(app.world().resource::<Tick>().0, expected);
        }
    }

    #[test]
    fn the_immediate_config_is_the_start_policy_of_the_design() {
        let config = MatchConfig::immediate(20);
        assert_eq!(config.tick_hz, 20);
        assert_eq!(config.start_tick, 1);
        assert_eq!(config.teams, [0, 1], "team = slot, one team for each slot");
        assert!(config.settings.is_empty());
    }
}
