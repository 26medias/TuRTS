# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository state

This repository holds no code yet. It contains one document:
`RTS-Engine-Project-Specifications.md` (version 0.6, 2026-10-26). That document is the
source of record for all design decisions. Read it before you start any implementation work.

There is no Cargo workspace yet. Toolchain: `rust-toolchain.toml` pins stable with the
`wasm32-unknown-unknown` target. `scripts/setup-dev.sh` installs the user-level tools (trunk,
wasm-bindgen-cli, wasm-opt, flamegraph, cargo-llvm-lines, cargo-bloat). `scripts/setup-sudo.sh`
installs perf, heaptrack, and valgrind and needs root. CI is `.github/workflows/ci.yml`: fmt,
clippy, wasm32 check, native tests, and the agent roster checks. When the workspace exists, add
the build and test commands here.

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
boundaries, not a fixed crate layout. Choose the crate layout when you create the workspace
and record it here.

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

Roster check: `bash docs/agents/check.sh` (format, budget, standing-rules identity) and
`bash docs/agents/check_fixtures.sh` (proves the check can go red). Run both before you commit
an agent file. A session that started without `.claude/agents/` does not load the directory;
restart the session after the roster is first created.
