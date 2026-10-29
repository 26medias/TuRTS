---
type: report
date: 2026-10-28
tags:
  - turts
  - report
  - review
ai-first: true
source: mcp
---

# Full spec review against the goal "Build the Ultimate RTS engine" — 2026-10-28

**Trigger.** During M4 design the orchestrator found that no spec, plan or milestone covers unit selection, group
orders, control groups or hotkeys. The CEO ordered a full review of the specs and plans by the whole team. See
[[Decisions/2026-10-28 Goal statement and full spec review]].

**Method.** Three phases. (1) `game-designer` and `player` wrote reference inventories of what the genre gives a
player and a modder, BEFORE reading the spec: 355 items, 181 CORE. (2) All 16 seats audited the spec, the register,
the milestones and the M1 to M4 documents against the inventories; each seat ran what could be run (schema probe
crate, `turts-pack` on a real C&C package, Chrome key probes, A* and stacking probes, latency model, a completeness
gate). (3) `technical-director` and `producer` reviewed the consolidated register; 7 blocking items, all incorporated;
re-check pending at the time of writing.

**Budget.** 2 + 14 Opus agent runs, 6 resumed turns. Estimated 20; actual 16 runs plus resumes.

**Verdict.** 16 seats, 16 `fail`. The spec describes a sound authoritative simulation and content platform, and that
half is built. It does not describe a game a person can play or a player interface a modder can define. 77 of 181
CORE items never appear; 61 appear as a word; 9 of 24 areas have no milestone; 0 of 13 milestone rows can be failed
by a player. Three shipped M3 validation rules make the Decided first playable mod impossible (weapons must be static;
a producer cannot die; nothing can be built in play). The server sends every unit to every slot (no fog). Group orders
fail if one unit is invalid. The schema rejected 35 of 35 fields a modder needs.

**Why the gates missed it.** Every lens took the spec's own claim list as the bar. `player`, `bevy-rendering-engineer`
and `game-designer` sat at 0 of 8 gates; the panel rule excluded `player` when there was "no playable loop". The M3
human browser test passed on the build the CEO could not play, because it reads controls through a test bridge.

**Output.** `docs/research/review-2026-10/00-gap-register.md` (10 findings, 16 CEO decisions G1-G16, a resequenced
milestone list QA0 → MC → MP1 → MP2 → M4 → M4b → M5 → MS → MB → M6 → M7 → M8-M13, process fixes), the two
inventories, 16 seat reports, 2 phase-3 reviews. Producer's cost re-estimate: 180 to 285 runs before M4; 525 to 875
to M7 (7 to 12 x M3).

**Status.** Decided on 2026-10-29: G1 adopted with the clause, G2 to G16 as the team lean (details in
[[Decisions/2026-10-28 Goal statement and full spec review]]). Review files committed. The M4 draft spec is on hold
until after MP2. Next: spec amendment to version 0.8 with a two-lens gate, CLAUDE.md fixes, QA0.
