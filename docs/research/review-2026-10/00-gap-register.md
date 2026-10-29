# Full spec review — consolidated gap register

Date: 2026-10-28. Status: phase 3 passed; re-check pass by both reviewers (technical-director: 3 blocking, producer: 4 blocking; all
incorporated, see `phase3-*.md`). **Decided by the CEO on 2026-10-29**: G1 adopted with the clause; G2 to G16 as the
team lean in section 3. See "CEO decisions" after the table in section 3.
Goal, verbatim from the CEO: **"Build the Ultimate RTS engine."** Read as the union of what StarCraft, Command &
Conquer, Age of Empires 2, Warcraft 3, Supreme Commander and Factorio-style industrial RTS give a player and a modder,
through the public mod API.

Method: two seats (`game-designer`, `player`) wrote reference inventories BEFORE they read the spec. Then all 16 seats
audited the spec, the register, the milestones and the M1 to M4 documents against those inventories. Each seat ran what
could be run. The orchestrator re-ran the load-bearing code claims (section 7). Seat reports: `gap-<seat>.md` in this
directory. Inventories: `inventory-engine.md` (355 items, 181 CORE), `inventory-player.md`. Yardstick A has 24 areas.

Budget used: 2 + 14 agent runs, 2 resumed turns, all Opus. Phase 3 adds 2 resumed turns.

## 1. Verdict

Sixteen seats, sixteen `fail`. The spec and the four milestone documents describe an authoritative RTS **simulation
and content platform**. They do not describe an RTS **game** a person can play, or that a modder can give a player
interface. The `player` seat's sentence stands: "the server could run the rules of all six games. A person could not
play any of them."

Of the 181 CORE items in yardstick A: 77 never appear in the spec, 61 appear as a word with no design, 43 are covered.
Of the 24 yardstick areas, 9 have no milestone at all (input, player control, camera, UI, construction, saves, teams
and match flow, audio, accessibility): 125 items, 67 CORE. The milestone table (S:674-688) has 0 of 13 rows that a
player could fail.

The spec is not garbage. Its server, package, accounting, terrain and scale design is sound and was built. Its flaw is
a missing half, and a process that could not see a missing half (section 6).

## 2. The ten findings that decide the plan

| # | Finding | Evidence | Seats |
| --- | --- | --- | --- |
| F1 | **No player control, anywhere.** Click, box, shift-add, control groups, hotkeys, right-click order, stop, hold, selection visuals, rally, health bars: 0 hits each. The client holds one selected unit and reads no key. | S sweep; `economy_ui.rs:31`; `KeyCode` 0 hits; `input.rs:44-45` | player, game-designer, bevy-rendering, rigour |
| F2 | **No HUD and no presentation design.** Presentation is one cell (S:122), Interface one row (S:164). The mod API has 0 presentation fields; the HUD is engine code with content-specific English. No audio crate at all. | `schema.rs:48-51`; `economy_ui.rs:124-337`; `cargo tree -i bevy_audio` empty | bevy-rendering, mod-runtime, TD |
| F3 | **No order model.** Six closed command kinds; mods cannot add an order (against S:358, D05). No queue, no interruption rule, combat takes no order. Attack, attack-move, stop, hold, patrol, repair, build: no design, no code. | `messages.rs:54-78`; `command.rs:30-56`; `combat.rs:165-199` | sim-core, network, ai, mod-builder |
| F4 | **Group orders are all-or-nothing, capped at 256.** One foreign or static unit rejects the whole `MoveTo`. No per-unit result. A 300-unit box move fails. 40 units sent to one point stack on it (780 overlaps, probe). | `command.rs:274-283,19`; `tests/command.rs:233`; world probe | network, TD, sim-core, world |
| F5 | **The first playable mod (16.2, Decided) has no milestone, and shipped rules make it impossible.** Weapons must be static; a producer with health must have removal and a producer with a queue cannot have removal (so a command center cannot die, so the victory rule cannot fire); no building can be made in play; "charge over time" payment is missing. Construction was excluded by M2, M3 and M4 in turn. | `content_m3_validation.rs:375,382,427`; mod-builder `turts-pack` run (28 of 33 fields rejected); `schema.rs:390-393` | mod-builder, sim-core, game-designer, producer, ai |
| F6 | **No fog of war for units; no match lifecycle; no teams; no identity.** Server sends every unit, health and node to every slot. No lobby a player recognises, no outcome per slot, no defeat/surrender/score (0 hits). Combat filters by slot equality, so allies would be hit. Client picks an unauthenticated `u64`; reconnect on it would let any client take a slot. | `export.rs:28,104-118`; `combat.rs:182-186`; `messages.rs:21`; `session.rs:103-105` | network, backend, perf |
| F7 | **Modding surface cannot be expressed.** 98 of 181 CORE items need a declarative field; the schema has 10. All types `deny_unknown_fields`. Libraries are refused ("not supported at M1"), which blocks the Decided AI libraries. The data layer, not the missing script layer, is the main blocker (15 items need scripts). | mod-runtime probe (35 fields, all rejected); `manifest.rs:44-49` | mod-runtime, mod-builder |
| F8 | **Browser control constraints are undesigned.** Physical-key binding (AZERTY probe: `KeyA` = "q"), edge scroll on a fixed 1280x720 canvas, pointer/keyboard lock versus Esc, hidden-tab throttling with the two-tab playtest, per-origin key-binding storage. The Chromium source's reserved list (`accelerator_table.cc:347-359`) does not include Ctrl+1..9; this is NOT TRACED with real key input, and Firefox and Safari are NOT TRACED. The MC key design must probe it first. 0 of 12 browser tests send a key. | Chrome 156 probes; `index.html:8` | web-platform |
| F9 | **No budget for anything a player touches.** Input latency never measured (model: 151 ms at 50 ms RTT, 20 Hz). All M1 bandwidth is a no-fog upper bound. Pick pass parses a string per unit (11x slower than a dense scan). Replays need a decision: command log (small) requires server determinism S:239/243 does not promise. Order cost grows with world size (1.1 ms per `MoveTo` at 40,000 units). | perf probes `sel.rs`, `model.py`; sim-core run 4 | perf, sim-core |
| F10 | **The goal is not in the spec and the gates could not see the gap.** S:95 says "should be possible", twice. Twelve sentences set a lower bar. "Interface" has three meanings, "presentation" two, "input" two. `player`, `bevy-rendering-engineer` and `game-designer` sat at 0 of 8 gates. `m3-human.js` passed on the build the CEO could not play, because it reads controls through a test bridge, not pixels. | rigour sweep; qa-lead C1/C2 probes | rigour, qa-lead |

Smaller defects confirmed by the orchestrator's re-run: `turts-pack` accepts an unknown capability (`"scripts.server"`)
with exit 0, and a `kind: Library` package holding units and a map with exit 0 (against S:207 Decided).

## 3. Decisions that belong to the CEO

The team names these and does not decide them.

| Id | Decision | Options | Team lean |
| --- | --- | --- | --- |
| G1 | Goal sentence in 3.1. | Proposed: "A mod must be able to reproduce the player-facing mechanics of the reference RTS games, within the terrain exclusions of 3.2 and D10 (no underground movement, no bridges with traversable space beneath), and a person must be able to play the default game with mouse and keyboard." Alternative: the same sentence without the exclusion clause, which reopens D10. | adopt with the clause |
| G2 | D41 (no offline skirmish, no local saves). All six reference games have both. | keep; reopen; or "server-in-process" (a local headless server for solo play) | reopen or server-in-process |
| G3 | D52 (one controlling session per slot) blocks shared control (AoE co-op). | keep; allow N sessions per slot as mod policy | keep now, note it |
| G4 | S:720 / D43 (no large editor suite precommitted) versus a map/trigger editor, CORE for a moddable engine. | keep and add a "when tooling becomes a need" trigger; or plan an editor milestone | add the trigger |
| G5 | Engine/mod line for the HUD and controls (S:164 is silent on selection, groups, camera, minimap). | (a) engine widgets, mod layout and content; (b) full mod UI runtime; (c) engine-fixed HUD | (a) |
| G6 | Group orders: atomic (today) or per-unit acceptance with per-unit reasons (genre rule). | A / B | B |
| G7 | Mod-defined orders: order type as content id in an open envelope (protocol redesign at the next bump). | yes / no | yes |
| G8 | Bot runtime: Rust bot framework exposed to all mods (S:189) or wait for the script layer. | framework now / wait | framework now |
| G9 | Replays: command log (needs server determinism promise) or state stream (620x larger). | command log / state / defer | command log, add the determinism requirement |
| G10 | Milestone resequence (section 4), with its token budget: 180 to 285 agent runs before M4 and 525 to 875 to M7 (producer re-estimate at the M3 rate; M3 used 75 against an estimate of 47). | adopt / amend | adopt |
| G11 | Roster: add a `ux-designer` seat; add `player` and `bevy-rendering-engineer` to every gate for a milestone with a client. | yes / no | yes |
| G12 | Who plays the gate: the `player` agent, a person, or both. | agent / person / both | both; a person signs off |
| G14 | Scripting runtime: hold the language Open until the MS spike (sequence row 7), or select now. | spike / select now | spike, placed before MB |
| G15 | Does "playable" (MP1) include terrain? If yes, M4 moves before MP1 and the budget before a person plays a full match grows by the M4 estimate. | no / yes | no |
| G16 | Approve the statuses of the new register rows D65 to D74 as Proposed, with the seats in section 5 item 5. | approve / amend | approve |
| G13 | Networking library. S:352, D18 and S:704 keep it Open ("not selected"), but `Cargo.toml:24` pins lightyear `=0.31.0` and M1 to M3 are built on it. | select lightyear and close the Open item; keep it Open and plan a re-evaluation gate | select, with the M1 spike (`docs/measurements/m1-network-spike.md`) as the evidence |

**CEO decisions (2026-10-29).** G1: adopt with the clause. G2 to G16: the team lean, as written in the last column.
For G2 the lean names two options; the orchestrator records it as **reopen D41 in the "server-in-process" form**: solo
play and local saves run against a local headless server (a separate process or a wasm module, never a `turts_client`
dependency on `turts_sim`). The CEO sees this reading in the report and can correct it. The decisions take effect in the
spec amendment (version 0.8) and in the milestone order of section 4.

**G5 follow-up (CEO, 2026-10-29).** The CEO asked whether line (a) limits the interface a mod can build. Answer recorded as
a D67 design constraint, line (a) kept: a mod places any interface element at any position and size, with its own images,
fonts, and cursors; the engine fixes no layout. Widgets are primitives, not a fixed HUD. Mod code does not paint pixels at
run time (5.1, D15); a missing primitive is added to the engine for all mods (4.3). Option (b) stays rejected.

Note on G6 and G7: they refine Proposed or Baseline text (S:342, D15, S:358), so the team can decide them as register
rows D65 and D73 and the CEO only sees them. G8 rests on D48/S:759 ("libraries provide the decision machinery"). G2
and G9 must say where the server simulation runs for solo play and replays: `turts_client` never depends on
`turts_sim` (M1D:94), so "server-in-process" means a separate headless server process or wasm module, not a client crate
dependency.

## 4. Proposed milestone sequence (direction, for the producer to plan)

Dependency reasons, not preferences. M1 to M3 stay done.

The producer's dependency check (`producer/deps2.py`) runs clean on this order.

| Order | Milestone | Why here | Acceptance a player can fail |
| --- | --- | --- | --- |
| 0 | **QA0 Instrument and schema groundwork** (short qa-lead slice) | The completeness gate and the pixels-only driver (section 5) go into CI before any new milestone is gated. Fix the two `turts-pack` validation defects. Open the schema for additive growth (F7) and lift the "dependencies not supported at M1" refusal to a real library check, so later milestones do not each re-open the manifest. | Completeness gate red on M1 to M4 as they stand; green on the QA0 fixture. |
| 1 | **MC Player control and client foundation** | M4's own gate needs mouse-and-keyboard play; MC needs no terrain. Replaces, not extends: single `Selection`, gather-else-move handler, per-frame camera fit, one unit material, panel model, flat-plane picking (keep picking by footprint). Includes: action map on physical keys, browser capture and focus, click/box/shift/type select, control groups, camera pan/zoom/edge/minimap-click, order queue with stop/hold/attack/attack-move, selection and status visuals, health bars, reasons, event/alert channel, team colours. Includes the order envelope redesign and per-unit group results **if** G6 and G7 are decided as the team leans; otherwise MC extends the six commands and the redesign waits. Flat-ground group movement comes here: destination spreading, local avoidance, clean arrival, unreachable fallback, because units that stack on one point (B-7) is a CORE defect that the first full-match playtest hits. | Blind playtest (section 5): tester with only the build completes the M3 industrial demo with mouse and keyboard; 40 units sent to one point arrive as a group. |
| 2 | **MP1 Playable skirmish (16.2) for two people** | Needs MC orders and the M3 economy. Lifts the three M3 rules that make 16.2 impossible (F5). Adds construction with placement ghost, power, damage and armour, engineer repair (S:742, Decided), rally, charge-over-time payment, teams, match lifecycle and outcome per slot, victory and defeat screen, a minimal two-slot lobby (team, colour, ready). Identity and per-slot settings in depth stay in M9 and M10; MP1 takes only what two people need to finish a match. | Two people finish a match with mouse and keyboard and both see the result screen. |
| 3 | **MP2 First bot** | Needs MP1 orders and lifecycle. A Rust bot framework exposed to all mods (G8) drives a slot. Remembered sightings for bots and humans alike. | A person finishes a match against the bot. |
| 4 | **M4 Terrain and geology** (draft kept, amended) | Keep the terrain core. Move K1/H9 to MC. Fix: cut list versus F9 (guide and rover are never cut), "opened passage" must have a felt result (the excavator is the unit that cannot cross), traversal keyed on a movement class (air later), static footprints block cells, queued units start moving, chunk and navigation state keyed by map id, group id carried into the search queue. | Ridge demo blind playtest. |
| 5 | **M4b Group movement on terrain** | Collision, push, chokes, slopes, cliff edges. Before any M7/M12 number, or those benchmarks measure the wrong movement. | A group crosses the ridge passage without stacking or walking through walls. |
| 6 | M5 Discovery, now with **unit fog of war** (F6) and last-seen state for humans and bots alike. | Needs teams (MP1). | Two players see different things. |
| 7 | **MS Scripting selection spike** | S:705 keeps the language Open; M7 needs "scripts on both targets" (S:681) and MB needs triggers and abilities beyond data (15 CORE items, F7). The spike selects and measures; it does not build the full runtime. | Decision recorded with browser and native measurements. |
| 8 | **MB Genre breadth**: a StarCraft-like second mod from the public API only (`mod-builder` builds it). | Proves "Ultimate", not only 16.2. Needs MP1, M5, M4b and MS. Does not need M6. | Modder builds it with no engine edit. |
| 9 | M6 Underground utilities. | as before; it blocks nothing above | as before |
| 10 | M7 Integrated workload, with fog on and input latency measured (F9 budgets). | after MB | p99 numbers recorded |
| 11–14 | M8 (plus main menu, profile, key-binding store), M9 (per-slot settings in depth, pause, speed, results), M10 (identity first, reconnect, saves), M11 (defeated players observe; replays per G9). | as the backend seat lists | each has a player check |
| 15–16 | M12 AI at scale (bot exists since MP2), M13 assets. | as before | as before |

Cost, producer's re-estimate at the M3 rate (about 3.75 agent runs per plan entry; M3 used 75 runs against an
estimate of 47): MC 60 to 100 runs; MP1 plus MP2 95 to 150; everything before M4 180 to 285; the total to M7 525 to
875, which is 7 to 12 times M3. These are estimates with the assumption stated. The CEO paused M2 over token spend,
so the budget is part of G10.

## 5. Process fixes (so this cannot happen again)

1. **Completeness gate** (qa-lead C1, built and tested): a coverage matrix over the 181 CORE items; every milestone spec
   must map each in-scope item to a named acceptance test and carry a `Player path` section; dependency order is
   checked. It went red on M1 to M4, green on a fixture, red on 5 of 5 mutations. Add to CI.
2. **Driver/oracle split** (qa-lead C2): a browser test driver may use only pointer events, key events and screenshots;
   bridges (`turts_ui`, `turts_move`) belong to the oracle only. Red on `m3-human.js` and `t7.js` today. Add to CI.
3. **Blind playtest gate** (player): tester has the build, the in-game guide and the pixels. Fails if an object, action
   or reason cannot be found, or 60 s pass with no sign of progress. A person signs off (G12).
4. **Panel rule**: `player` and `bevy-rendering-engineer` at every gate for a milestone with a client; a `ux-designer`
   seat (G11). Remove the "no playable loop, player not picked" rule.
5. **Register rows D65 to D74** (technical-director): D65 control model, D66 input map, D67 HUD model, D68 camera
   controls, D69 audio, D70 client shell, D71 map tooling, D72 client-local persistence, D73 order envelope. Each row
   gets a status (Proposed until the CEO decides), a seat per the technical-director's table (`ux-designer` D65, D66,
   D67; `bevy-rendering-engineer` D68, D69; `backend-engineer` with `ux-designer` D70; `world-engineer` D71;
   `web-platform-engineer` D72; `mod-runtime-engineer` D73) and a link to its G item. The client architecture decision becomes D74
   (interaction core with no renderer dependency, driven headless by agents and bots), with a T1 forbidden-edge test.
6. **Spec text**: the goal sentence (G1); one meaning each for "Interface", "presentation", "input"; S:352 goes to the
   CEO as G13; fix S:790 ("desktop-first" contradicts D02); add input/HUD/order-model/fog rows to section 15.
   `CLAUDE.md`: eight stale lines (rigour report).
7. **Client architecture decision**: an interaction core with no renderer dependency that agents and bots drive
   headless (technical-director).

## 6. Why eight gates missed it (qa-lead, confirmed by the orchestrator)

Every lens took the spec's own claim list as the bar. Rigour hunted contradictions; the engine lens came from the
Demonstration column; the producer ordered the demonstrations; qa-lead asked "can this test go red" of each claim
and never asked whether the claim list was complete. The one seat whose job is to say "I cannot play this" was
excluded by rule when there was no playable loop, which is exactly when it was needed. The orchestrator wrote the
M3 and M4 documents inside the same frame and did not step outside it. The fix is section 5: an outside yardstick
and a gate that reads pixels.

## 7. What the orchestrator re-ran

`KeyCode` 0 hits in the client; `cargo tree -p turts_client -i bevy_audio` empty; `content_m3_validation.rs:375,382,427`
rules present; `export.rs:25-29` unfiltered units; `manifest.rs:44-49` dependencies refused; `turts-pack` on the
mod-builder's C&C package with `capabilities: ["scripts.server"]` exits 0 and with `kind: Library` exits 0; spec term
counts for health (0), control group (0), minimap (1), hotkey (0; the regex `hotkey|keybind|shortcut` gives 1),
rally (1).

NOT TRACED by the orchestrator: the per-seat probe numbers (latency model, pick-pass timings, 40-unit stack, A*
costs, geology loop); the browser reserved-key list beyond the quoted Chromium lines; all per-game yardstick facts.

## 8. What happens to the M4 draft

`docs/superpowers/specs/2026-10-28-m4-terrain-design.md` is committed as a draft ON HOLD and is not gated. G10 is
adopted (2026-10-29), so it is amended per section 4 row 4 and gated after MC, MP1 and MP2. K1 (drag-select) and H9
leave it for MC.
