---
name: bevy020-ci-facts
description: Verified Bevy 0.20.0 docs.rs names for fixed-tick scheduling, the headless test driver that is correct, lightyear 0.31 dependency-tree facts, and the CI wasm32 check that a native-only server crate will break
metadata:
  type: project
---

Verified 2026-10-26 on docs.rs bevy 0.20.0 (latest): FixedPreUpdate, FixedUpdate,
FixedPostUpdate, FixedMain, RunFixedMainLoop, RunFixedMainLoopSystems
{BeforeFixedMainLoop, FixedMainLoop, AfterFixedMainLoop}, ScheduleRunnerPlugin
::run_loop/run_once, MinimalPlugins = TaskPoolPlugin + FrameCountPlugin +
TimePlugin + ScheduleRunnerPlugin, Time<Fixed>::from_hz/from_seconds/
from_duration, World::run_schedule, App::add_message (add_event not on the
page). bevy default features include bevy_render, bevy_winit, bevy_pbr, bevy_ui.
Sub-crates bevy_app, bevy_ecs, bevy_time, bevy_state 0.20.0 exist.

Headless test driver (docs.rs bevy_time 0.20.0, struct.Fixed and
enum.TimeUpdateStrategy): the fixed clock becomes the generic `Time` only while
`run_fixed_main_schedule` runs FixedMain. A direct `World::run_schedule(FixedUpdate)`
skips that swap, so `Res<Time>` holds the virtual delta, not the tick period.
Use `TimeUpdateStrategy::FixedTimesteps(n)` + `app.update()`: "a call to
App::update will always run the fixed loop exactly n times". My 2026-10-26
research note (scratchpad m1/crates.md) recommended run_schedule(FixedUpdate);
that was wrong.

lightyear 0.31.0 with features std, replication, websocket, netcode and
client or server: `cargo tree -e normal -i bevy_render|bevy_pbr` prints nothing
on native and on wasm32; `lightyear_web` pulls bevy_winit on wasm32 only.
`cargo tree -i X` exits with "did not match any packages" on stderr when X is
absent from the whole lockfile; a grep -c over stderr counts that line.
tar 0.4.46 (default-features = false) checks on wasm32-unknown-unknown.
bevy_asset 0.20.0 depends on ron ^0.12 (crates.io API).

CI (.github/workflows/ci.yml:27) runs `cargo check --workspace --target
wasm32-unknown-unknown`; a native-only server crate needs `--exclude`.

**Why:** Gate reviews on the first workspace will cite these names and this CI line.
**How to apply:** Re-check docs.rs before citing if Bevy moves past 0.20.
