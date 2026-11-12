//! Headless test harness.

use bevy_app::App;
use bevy_time::{TimePlugin, TimeUpdateStrategy};

use crate::match_config::MatchConfig;
use crate::plugin::SimPlugin;

/// An app with the simulation and no window. Each `app.update()` on the returned app runs
/// exactly one fixed tick: the fixed clock advances by one timestep per update. The call makes
/// one warm-up update first, because the first update runs zero fixed ticks.
///
/// `SimPlugin` inserts no `MatchConfig` and a sim with no config does not tick (rule R17, design
/// 6.7.5). This helper inserts the `Immediate` config, in the one line below. A harness that
/// builds `SimPlugin` by hand must insert a config too, or it gets 0 ticks.
pub fn headless_app(tick_hz: u16) -> App {
    let mut app = App::new();
    app.add_plugins((TimePlugin, SimPlugin { tick_hz }));
    app.insert_resource(MatchConfig::immediate(tick_hz));
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update();
    app
}
