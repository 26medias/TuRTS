---
type: decision
date: 2026-10-28
tags:
  - turts
  - decision
  - review
ai-first: true
source: mcp
status: approved
---

## For future Claude
The CEO stated the project goal on 2026-10-28: **"Build the Ultimate RTS engine."** This is the bar for every spec and plan. It is higher than spec section 3.1 ("allow mods to build games with different combinations" of mechanics). Read "Ultimate" as the union of what the genre does (StarCraft, Command & Conquer, Age of Empires, Warcraft 3, Supreme Commander, Factorio-style industrial RTS), moddable through the public API.

**Trigger.** During M4 design the orchestrator found that no spec, plan or milestone covers the core RTS control loop: drag-select, group orders, control groups, hotkeys. The client selects one unit at a time (`crates/turts_client/src/economy_ui.rs:31`). Four milestones and all gate reviews missed it. The CEO: "I do not trust the specs, I do not trust the team and I do not trust you. I want a full review of the project specs and plans. I want the whole team involved. I want the specs to match the project goals."

**Root cause (orchestrator's reading).** Every gate compared the spec with itself (rigour, architecture, cost, instrument). No gate compared the spec with an outside inventory of what an RTS is. Consistency passed; completeness was never tested.

**Decision.** Stop M4 design (draft at `docs/superpowers/specs/2026-10-28-m4-terrain-design.md`, uncommitted, not gated). Run a full review in three phases:
1. Reference inventory, written by `game-designer` and `player` BEFORE they read the spec (so the spec cannot anchor them).
2. Gap audit by every seat against that inventory: present / partial / missing / wrong milestone, plus own-domain gaps.
3. Consolidation into a gap register with spec amendments and a corrected milestone list, reviewed by `technical-director` and `producer`, then decided by the CEO.

Budget stated: about 20 agent runs on Opus. Scratch: `scratchpad/review/`. Results land in `docs/research/review-2026-10/` and a vault report.

## CEO decisions on the register (2026-10-29)

The register (`docs/research/review-2026-10/00-gap-register.md`, section 3) named sixteen decisions G1 to G16. The CEO decided: **G1 adopt with the clause; G2 to G16 as the team lean.** In effect:

- **G1** Goal sentence goes into spec 3.1: a mod must be able to reproduce the player-facing mechanics of the reference RTS games, within the terrain exclusions of 3.2 and D10; a person must be able to play the default game with mouse and keyboard.
- **G2** D41 reopened in the "server-in-process" form: solo play and local saves run against a local headless server (separate process or wasm module; `turts_client` never depends on `turts_sim`). Orchestrator's reading of a two-option lean; the CEO can correct it.
- **G3** D52 kept; shared control noted for later. **G4** D43 kept with a "when tooling becomes a need" trigger. **G13** lightyear selected; the Open item closes with the M1 spike as evidence. **G14** scripting language stays Open until the MS spike.
- **G5** HUD line (a): engine widgets, mod layout and content. **G6** per-unit order acceptance with per-unit reasons. **G7** order type as a content id in an open envelope. **G8** Rust bot framework now, open to all mods. **G9** replays as a command log; the server gets a determinism requirement.
- **G10** Milestone order QA0 → MC → MP1 → MP2 → M4 → M4b → M5 → MS → MB → M6 → M7 → M8 to M13; budget 180 to 285 agent runs before M4, 525 to 875 to M7. **G15** "playable" does not include terrain.
- **G11** `ux-designer` seat; `player` and `bevy-rendering-engineer` at every client gate. **G12** agent and person play; a person signs off. **G16** D65 to D74 approved as Proposed with their seats.

**Added 2026-10-29:** the CEO decided "add stop and hold" to the 16.2 combat order list (Decided under D55; the gap register finding F3 named them as genre CORE).

**Added 2026-10-29, D67 constraint (G5 kept at line a).** The CEO asked how mods get full freedom over their interface (menus, info panels, avatars) and proposed a canvas layer over the map. Answer: a mod-painted canvas is option (b), rejected by 5.1 (no per-frame script), D15 (no renderer details) and the browser frame budget. Line (a) gives the same freedom with a widget tree: the mod places any element at any position and size, with its own images, fonts and cursors; the engine fixes no layout and draws the primitives (box, image, text, bar, button, list, grid, tooltip, cursor, minimap view). A missing primitive is added for all mods (4.3). CEO: "record it as a D67 design constraint, keep (a)". Recorded in spec D67, section 15 HUD row, history 0.8, and the gap register. The widget set, schema, image formats and limits are the MC spec's work (`ux-designer`, `mod-runtime-engineer`).

Next: spec amendment to version 0.8 (goal sentence, D65 to D74, section 14 and 15 rows, term fixes, D41 and S:352 updates), gated by `spec-rigour-reviewer` and `player`; CLAUDE.md stale lines; then QA0.
