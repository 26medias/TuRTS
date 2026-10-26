//! Headless test harness.

use bevy_app::App;
use bevy_time::{TimePlugin, TimeUpdateStrategy};

use crate::plugin::SimPlugin;

/// An app with the simulation and no window. Each `app.update()` on the returned app runs
/// exactly one fixed tick: the fixed clock advances by one timestep per update. The call makes
/// one warm-up update first, because the first update runs zero fixed ticks.
pub fn headless_app(tick_hz: u16) -> App {
    let mut app = App::new();
    app.add_plugins((TimePlugin, SimPlugin { tick_hz }));
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update();
    app
}
