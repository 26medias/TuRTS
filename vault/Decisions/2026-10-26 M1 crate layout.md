---
type: decision
date: 2026-10-26
tags:
  - turts
  - decision
  - architecture
ai-first: true
source: mcp
status: proposed
---

## For future Claude
Six crates plus a data package for M1, with forbidden dependency edges tested by `cargo tree`. Proposed by technical-director (Fable); orchestrator accepts as a studio call pending gate 1 on the M1 design.

| Crate | One line |
| --- | --- |
| `turts_mod_api` | versioned content ids, unit definition schema, command and query envelopes, `ENGINE_API_VERSION`. No Bevy. |
| `turts_net_protocol` | wire messages, `NET_PROTOCOL_VERSION`. Transport-free. No Bevy. |
| `turts_sim` | authoritative world, fixed ticks, one unit. Depends on `bevy_app`, `bevy_ecs`, `bevy_time` only. |
| `turts_mod_runtime` | manifest and lockfile parse, cache, hash check, declarative load. No Bevy, no scripts. |
| `turts_server` (bin, native) | `MinimalPlugins` app; serves advertisement, lockfile, transport endpoint. |
| `turts_client` (bin, wasm32) | `DefaultPlugins` app; presents replicated state; sends commands. Never depends on `turts_sim`. |
| `mods/base/` | the default mod as a data package, not a crate. |

**Forbidden edges, each a `cargo tree` test:** `sim` on `bevy_render`, `bevy_winit`, `bevy_pbr`; `sim` on `mod_runtime`; `mod_api` and `net_protocol` on any `bevy*`; `client` on `sim`.
**Fixed tick:** server `Time::<Fixed>::from_hz(TICK_HZ)`; commands in `FixedPreUpdate`, sim in `FixedUpdate`, snapshots in `FixedPostUpdate`. Client has zero `FixedUpdate` systems.
**CI fix needed:** the wasm32 check must exclude `turts_server`.
**Open items the TD flagged as tie-breaks, taken as studio calls instead (reversible prototype choices):** WebSocket inside the two binaries only; `TICK_HZ` one constant, 20, "measure at M1"; RON for package files; `turts_` prefix.
