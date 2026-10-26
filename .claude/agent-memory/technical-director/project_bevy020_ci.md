---
name: bevy020-ci-facts
description: Verified Bevy 0.20.0 docs.rs names for fixed-tick scheduling, and the CI wasm32 check that a native-only server crate will break
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

CI (.github/workflows/ci.yml:27) runs `cargo check --workspace --target
wasm32-unknown-unknown`; a native-only server crate needs `--exclude`.

**Why:** Gate reviews on the first workspace will cite these names and this CI line.
**How to apply:** Re-check docs.rs before citing if Bevy moves past 0.20.
