# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository state

This repository holds the M1 Cargo workspace and the specification.
`RTS-Engine-Project-Specifications.md` (version 0.7, 2026-10-26) is the source of record for
all design decisions. Read it before you start any implementation work. The M1 numbers
(bytes per tick, tick time, frame time, memory, download size) are in
`docs/measurements/m1.md`. They are recorded, not judged; the budgets stay open (spec 15).

Toolchain: `rust-toolchain.toml` pins stable with the
`wasm32-unknown-unknown` target. `scripts/setup-dev.sh` installs the user-level tools (trunk,
wasm-bindgen-cli, wasm-opt, flamegraph, cargo-llvm-lines, cargo-bloat). `scripts/setup-sudo.sh`
installs perf, heaptrack, and valgrind and needs root. CI is `.github/workflows/ci.yml`: fmt,
clippy, wasm32 check, native tests, the acceptance run, the T7 browser job, and the agent
roster checks. The build and test commands are in "Build and test commands" below.

Bevy version: **0.20** (orchestrator call, 2026-10-26; stable at that date). Ten installed Bevy
skills pin 0.19; read the `bevy-ecs` skill's 0.20 pitfalls before you use any 0.19 snippet.

GCP for benchmarks, testing, and dev (D57 to D59): use the operator's `default` gcloud
configuration (project, account, region, and zone are set there, not in this file). Record
machine type and image with every benchmark result. Delete test VMs when a run ends.

## What the project is

A mod-first real-time strategy (RTS) engine in Rust + Bevy. The browser WebAssembly client is
the primary product. A desktop executable is secondary. Multiplayer is a core requirement.
The official game is a default mod that uses only the public mod API.

## How to read the specification

Each statement in the spec carries a status. Respect the status:

- **Decided**: implement as written. Do not reopen without the user.
- **Baseline**: the working design. You can refine it during prototyping. State the refinement.
- **Open**: no choice has been made. Do not pick one silently. Prototype and report, or ask.
- **Proposed**: a suggestion, not a commitment.

Section 2 is the decision register (D01 to D59). Section 15 lists open decisions. Cite the
decision ID when a code choice depends on one.

## Architecture constraints that shape all code

These rules come from the spec and apply to every crate:

- **Authoritative server.** The server computes all simulation results. Clients send commands
  and present permitted state. Do not build deterministic client lockstep (D18, section 6.2).
- **Simulation runs headless.** The simulation core must run with no window and no rendering
  plugins. Presentation consumes simulation state. Visual meshes are never the authoritative
  terrain (section 4.1).
- **Engine mechanisms, mod policy.** The engine enforces integrity and exposes capabilities.
  Gameplay rules live in mods. The default mod gets no private shortcut. If the default mod
  needs a new engine feature, expose it to all mods (sections 4.3, 5.2).
- **Versioned mod API, not Bevy internals.** Mods see stable content identifiers, commands,
  and queries. They never see Bevy entities, schedules, or renderer details (D15, section 5.2).
- **Heavy work stays in Rust.** Pathfinding, inventories, transport, and production scheduling
  run in Rust. A mod must not run a script per unit or per item per frame (section 5.1).
- **Components, not class trees.** Entities are compositions of capabilities (section 4.2).
- **Terrain is a heightfield.** One surface elevation per horizontal location. No tunnels,
  caves, overhangs, or stacked surfaces. Buried deposits are stored apart from the surface.
  Underground infrastructure is a network, not a traversable space (D09 to D13).
- **Exact package lockfiles.** Dependencies resolve to exact versions and hashes before a match
  starts. Clients download the server's locked set (D35, section 5.5).
- **Slots are permanent.** A claimed player slot belongs to that player for the whole match and
  is never reassigned (D25).
- **Fixed simulation ticks**, independent of rendering (section 6.1).

## Logical modules (section 4.1)

Simulation core, presentation, mod runtime and API, multiplayer, terrain and geology,
production and logistics, utility networks, headless host. These are responsibility
boundaries, not a fixed crate layout. The M1 crate layout is in "Crate layout" below.

## Build and test commands

- `cargo test --workspace`: all tests. A test that needs a running server is `#[ignore]`; run
  it with `scripts/acceptance.sh`. A test that starts one binary on its own runs by default:
  `turts-pack`, the `--help` texts, and the headless client against a small fake HTTP server
  (`crates/turts_client/tests/rejected_exit.rs`).
- `bash scripts/boundary.sh`: the T1 forbidden-edge checks (design 4.1). The fixture must be red.
- `bash scripts/acceptance.sh`: builds both binaries and runs the ignored process tests in
  release, one at a time.
- `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown`: wasm32 check.
- `trunk build --release` in `crates/turts_client_web`: the browser build.
- `bash scripts/t7.sh`: T7, the browser join in headless Chrome. It needs trunk, brotli, node
  with `npm ci` done in `tests/browser`, and Chrome.
- `cargo run -p turts_pack -- <dir>` (the `turts-pack` binary): check the package directory
  `<dir>` on the server path and the client path, then write `<dir>.tar` next to it and print
  `<archive path> <sha256 hash>`. On a problem it prints the `ModError` (package, file,
  field) on stderr, writes no archive, and exits with 2. `turts-pack --help` lists the checks.
- `turts_server <server.ron> [options]`: the headless host. `turts_server --help` lists the
  options, the `server.ron` fields, and the HTTP endpoints. A full `server.ron` and the
  endpoint paths are in design Appendix A. The options:
  - `--tick-log P`: test hook. One line `tick,wall_ns,unit_id,x,y` for each unit and tick.
    Off when `--synthetic-units` is above 0.
  - `--stats P`: test hook. Rewrites one JSON line of statistics each second and at exit.
  - `--synthetic-units N`: add N units of the first unit type; each gets a new move target
    every 40 ticks. For load runs.
  - `--run-ticks N`: exit with 0 after N ticks.
- `turts_client --server URL --ws URL [options]`: the native client. `turts_client --help`
  lists everything below. `--server` and `--ws` are required.
  - `--headless`: no window, no rendering. `--player-id N`: a fixed player id (default: random).
  - `--run-secs S`: test hook. Stop after S seconds. Without it, a joined client runs until
    it is killed.
  - `--out P`: test hook. At the end, write the report as JSON to P: `welcomed`,
    `rejected_reason`, `slot`, `disconnected_after_connect`, `command_results`,
    `final_positions`, the resync counters, and the debug state.
  - `--frame-log P`: test hook. One record for each presented frame.
  - `--move X,Y`: test hook. Send one `MoveTo` to X,Y. It moves all the units that the slot of
    this client owns; `--move-unit ID` moves only that unit (it may belong to another slot, so
    the server rejects it). `--move-after-ticks N` sets the wait after `Welcome` (default 5).
  - `--hello-hash H`, `--protocol-version N`: test hooks. Send H as the lockfile hash, or N as
    the wire protocol version, in `Hello`. The server rejects a wrong value.
  - `--drop-delta-ticks a,b,c`, `--drop-arrival-delta`, `--hold-resync-snapshots N`: test hooks
    for the lost-delta case (T8). They ignore the `UnitDeltas` at the tick offsets a, b, c from
    `Welcome`; ignore the first `UnitDeltas` that puts a unit at the `--move` target; and
    hold each resync `Snapshot` back by N ticks.
  - Exit codes: 0 ended well; 1 the client rejected the advertisement, a package, or the join
    (`turts_client: <ModError>` on stderr, the same text as `rejected_reason`; a client with a
    window exits with 1 only at `--run-secs`); 2 the command line is wrong.
- `python3 tests/net/relay.py <listen port> <target port> --out P`: a TCP relay that counts the
  bytes in each direction.
- `node tests/browser/measure.js <page url> <label> [chrome flags]` in `tests/browser`: the
  browser figures. The exact commands for every M1 figure are in `docs/measurements/m1.md`.
- Build each binary with its own `cargo build -p <crate>`. A workspace-wide build unifies the
  client's `bevy` features into the server.

## Crate layout

Source: `docs/superpowers/specs/2026-10-26-m1-foundation-design.md` section 4.1. Crates are
under `crates/`. Packages are under `mods/`.

| Crate | Responsibility | Forbidden dependencies |
| --- | --- | --- |
| `turts_mod_api` | Content ids, package schema types, `UnitRegistry`, `ENGINE_API_VERSION`, `ModError`. | Any `bevy*` crate. |
| `turts_net_protocol` | Wire messages, `NET_PROTOCOL_VERSION`. No transport. | Any `bevy*` crate. |
| `turts_sim` | Authoritative headless world: fixed ticks, flat map, movement, command validation. | `bevy_render`, `bevy_winit`, `bevy_pbr`, `turts_mod_runtime`. |
| `turts_mod_runtime` | Archive read, manifest and lockfile parse, hash, validation, `pack_dir`, and the client path check `visuals_by_unit`. | Any `bevy*` crate. |
| `turts_pack` (library and binary) | `load_package`, the one path from a package to the world, called by the server and by the packer; the `turts-pack` binary. It needs `turts_sim`, so it cannot live in `turts_mod_runtime`. | Not a dependency of `turts_client`. |
| `turts_net_lightyear` | The shared lightyear `ProtocolPlugin`: one registration order for both ends. | `turts_sim`, `turts_mod_runtime`. |
| `turts_server` (binary) | Headless host: HTTP endpoints, lightyear server, snapshots and deltas, tick log, `--stats`, `--synthetic-units`. | `bevy_render`, `bevy_winit`, `bevy_pbr`. |
| `turts_client` (library and binary) | All client logic: fetch, acquisition, join, replica, interpolation, presentation, input. | `turts_sim`. |
| `turts_client_web` (cdylib) | Thin wasm32 entry for the browser client. Sets `window.turts_debug`, `turts_move`, and `turts_memory_bytes`. | None of its own. |
| `turts_checks` | Test-only: T1 boundary test and the no-content scan. | Not a dependency of any crate. |

Workspace-wide: no `lightyear_prediction`, no `bevy_replicon`. `vendor/aeronet_websocket` is a
copy of the crate with TCP no-delay on the server socket (`vendor/aeronet_websocket/VENDORED.md`).
It is not a member; `[patch.crates-io]` in `Cargo.toml` uses it. The fixture crate
`tests/boundary-fixture/` is not a member. It depends on `bevy_render` on purpose, so the
boundary check can prove it goes red.

## Decisions that are still open

Do not assume any of these. Prototype browser-first and report measurements:

- Scripting language (Rhai is a candidate only).
- Networking library and transport (WebSockets, WebTransport, WebRTC are candidates).
- Desktop delivery method (native Bevy or a wrapper such as Tauri).
- Tick rate, numeric budgets, map dimensions, supported browser versions.
- Archive format, registry protocol, signature formats.

## Roadmap

Section 14 lists validation milestones M1 to M12. M1 is the first target: a browser client, an
authoritative headless server, and one mod-defined unit. Section 16.2 defines the first
playable mod: one faction, three infantry types, one resource and harvester, one tank, one
turret, a command center, and a power station.

## Development workflow

Development is agent-first (D43, section 16.1). Build tools when a real need appears during
implementation, testing, debugging, or profiling. Do not build a large manual editor suite in
advance. Favor headless scenario execution, machine-readable schemas, and structured
diagnostics.

Hosting benchmarks (D57, D58): 4 players/1,000 units/1 map and 8 players/5,000 units/2 maps on
a 4-vCPU, 8-GB server. Measure, do not promise capacity.

## Code style

Use 4 spaces per indentation level, including in Rust. Write clean, DRY code (section 4.2).

## Agent team

Sixteen project subagents live in `.claude/agents/`. Their design is
`docs/superpowers/specs/2026-10-25-studio-agent-team-design.md`. Section 4 of that design
holds the ownership table: which seat is Primary and Second for each spec section. The main
session is the orchestrator. It picks the lenses per gate with these rules.

- **Lens mapping.** Rigour: `spec-rigour-reviewer`. Engine/domain: the Primary of the section
  the artefact touches, from the ownership table. Boundary/sequencing: `technical-director`
  for architecture, `producer` for plans. Consumer: `mod-builder` when the artefact touches
  4.3, 5, or 6.5; `player` when it touches 3, 6.6, 16.2, or 17; otherwise the Second of the
  section.
- **Selection rule.** Four lenses per gate. When an artefact spans several subsystems, add one
  domain seat per extra subsystem and say so. Six agents is the cap for one gate; above six,
  ask the user. When a default lens has no claim to test in the artefact, drop it and say why.
- **Milestone artefacts.** For a spec or plan on a milestone (spec section 14), the engine
  lens is the Primary of the first subsystem in the Demonstration column, and each further
  subsystem adds its Primary. For M1: `web-platform-engineer`, `network-engineer`,
  `mod-runtime-engineer`.
- **Heavy review.** `performance-architect` joins any gate where cost, scale, or a numeric
  budget is at stake (spec 6.2, 6.9, 13, M7, M12). It is an allowed fifth lens.
- **Minimum prompt.** Artefact path, scope, the lens question, the review-mode rule, and the
  scratchpad path. The agent body carries the rest.
- **Reading reports.** Record line 1 (`VERDICT=...`) and the High findings of each report in
  your working notes. The rest stays in the report.
- **Fan-out reading.** `Explore` (built-in) for raw search. Domain seats for conclusions.
- **Independent refutation.** `performance-architect` or `qa-lead` to try to break a claim.
- **Build.** Domain builder for the crate. `qa-lead` writes or reviews the test instrument.
- **Verify.** `qa-lead` runs. `player` and `mod-builder` judge.
- **Git.** Never `git add -A` while an agent runs. Stage explicit paths. Memory files under
  `.claude/agent-memory/` are staged on purpose, not by accident.

Third-party skills, their caveats, and the deferred and rejected ones are listed in
`docs/agents/skills.md`. Ten Bevy skills pin 0.19; re-check them when `Cargo.toml` pins a version.

The studio keeps its work log, decisions, task board, knowledge, and the daily CEO report in a
dedicated Obsidian vault at `vault/` in this repository (committed; public; no secrets).
Log every decision and every day of work there; write the daily report at the end of each
working session.

Roster check: `bash docs/agents/check.sh` (format, budget, standing-rules identity) and
`bash docs/agents/check_fixtures.sh` (proves the check can go red). Run both before you commit
an agent file. A session that started without `.claude/agents/` does not load the directory;
restart the session after the roster is first created.
