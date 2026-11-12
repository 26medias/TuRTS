//! The reorder grace is the most fixed ticks that one server frame can run. The bound is computed
//! here from the Bevy rule itself, not from a formula: a real Bevy app with `Time<Virtual>` and
//! `Time<Fixed>` runs one frame with the worst overstep left by the frame before (one timestep
//! less 1 ns) and one frame of a very long stall, and the test counts the `FixedUpdate` runs.

use std::time::Duration;

use bevy::app::{App, FixedUpdate};
use bevy::ecs::system::ResMut;
use bevy::prelude::{Resource, Time};
use bevy::time::{Fixed, TimePlugin, TimeUpdateStrategy};
use turts_client::replica::max_catchup_ticks;
use turts_net_lightyear::tick_duration;

#[derive(Resource, Default)]
struct Runs(u32);

/// The most `FixedUpdate` runs in one frame for a tick rate, by the Bevy rule.
fn most_ticks_in_one_frame(hz: u16) -> u32 {
    let timestep = tick_duration(hz);
    let mut app = App::new();
    app.add_plugins(TimePlugin)
        .insert_resource(Time::<Fixed>::from_duration(timestep))
        .init_resource::<Runs>()
        .add_systems(FixedUpdate, |mut runs: ResMut<Runs>| runs.0 += 1);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.update(); // the first update starts the clocks
    // The worst overstep that frames can leave: one timestep less 1 ns. It runs no tick. A frame
    // adds at most the clamp (250 ms), so a long timestep takes several frames.
    let mut left = timestep - Duration::from_nanos(1);
    while !left.is_zero() {
        let step = left.min(Duration::from_millis(250));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(step));
        app.update();
        left -= step;
    }
    assert_eq!(app.world().resource::<Runs>().0, 0, "hz {hz}: no tick yet");
    // A stall of 30 s: `Time<Virtual>` clamps the frame to its max_delta.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(30)));
    app.update();
    app.world().resource::<Runs>().0
}

#[test]
fn the_grace_covers_the_most_ticks_of_one_frame_at_every_tick_rate() {
    let mut loose = Vec::new();
    for hz in 1u16..=240 {
        let most = most_ticks_in_one_frame(hz);
        let grace = max_catchup_ticks(hz);
        assert!(
            grace >= most,
            "hz {hz}: one frame runs {most} ticks, the grace is {grace}"
        );
        if grace != most {
            loose.push((hz, most, grace));
        }
    }
    // The bound is tight: it is the Bevy rule, not a margin.
    assert!(
        loose.is_empty(),
        "the grace equals the Bevy maximum at every rate; (hz, bevy, grace): {loose:?}"
    );
}

#[test]
fn the_grace_at_20_hz_is_5_ticks() {
    assert_eq!(most_ticks_in_one_frame(20), 5);
    assert_eq!(max_catchup_ticks(20), 5);
}
