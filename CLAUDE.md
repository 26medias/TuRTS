# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository state

This repository holds no code yet. It contains one document:
`RTS-Engine-Project-Specifications.md` (version 0.5, 2026-10-21). That document is the
source of record for all design decisions. Read it before you start any implementation work.

There is no build, lint, or test command yet. When the first Cargo workspace exists, add the
commands to this file.

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
