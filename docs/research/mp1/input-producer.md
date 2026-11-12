# MP1 input: producer (scope, sizing, dependency graph)

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=INPUT

Repository: /home/julien/Projects/TuRTS-mp1 (branch mp1-design). Mode: answer. Read-only.

## 1. MP1 scope

Types: E = engine mechanism, C = mod content, UI = client, I = test instrument. Exit = needed for the exit row (spec :723).

| Id | Item | Type | Spec line | Exit |
|---|---|---|---|---|
| X1 | Lift the two remaining M3 rules: "a unit with health requires removal" and "a unit with a queue cannot be removed" (content_m3_validation.rs:375, :423). Destruction of producers. Version-table rows. MC already lifted "weapons must be static" (plan:114; the row "weapon with movement" is at version_table.rs:51-55). So spec :809 is stale: it says "three". | E | :723, :809 | Y |
| X2 | Damage and armour as mod data. Automatic targeting for idle and hold units. The choice code exists at combat.rs:297. | E | :723, :179, :803 | Y |
| X19 | Protocol bump: lifecycle and lobby messages, damage and death events, construction state. Fix the per-tick full order table (audit :34). | E | :723 | Y |
| X11 | Match lifecycle. Outcome for each slot. Victory rule as mod data. A rule for simultaneous destruction. | E+C | :723, :804, :807 | Y |
| X12 | Two-slot lobby with ready, team and colour | UI+E | :723 | Y (ready only) |
| X13 | Victory and defeat screen, mod-owned (D70 :99) | UI | :723 | Y |
| X4 | Construction: an order, a site check, a footprint that blocks, progress over time, payment | E | :723, :801 | Y |
| X4b | Cancel and refund of construction (production cancel is from M3, :720) | E | :723 | MP2 |
| X5 | Placement ghost | UI | :723 | Y |
| X6 | Power: supply against demand; a shortage slows production and disables turrets; a power meter in the HUD | E+C+UI | :802 | Y |
| X7 | Charge-over-time payment | E | :723, :434 | MP2 (:801 "pay, then progress" is enough) |
| X8 | Engineer repair | E+C | :798 | MP2 |
| X9 | Rally point | E+UI | :723 | MP2 |
| X10 | Teams: team id and an ally filter (today combat filters by slot equality, gap F6) | E | :723, :804 | 1v1 does not need it. M5 needs it (:727) |
| X14 | Attack, hit and death effects, minimal | UI | :723 | Y |
| X15 | Default mod: 8 types, ore, a 2-player map, HUD layout, guide, numbers, starting assets | C | :784-805 | Y |
| X16 | Real static assets (decision 5b) | E+C | :737 | N |
| X17 | Sound set (the engine exists: turts_client/Cargo.toml:14, audio.rs) | C | D69 :98 | N |
| X18 | Items that MC moved to MP1: Tasks.md:216, :233-235 | UI | O14 :38, O15 :14,:16,:22 | N |
| X20 | Instruments: headless two-client match, acceptance rows, fault plants, pixels-only two-player driver, completeness mapping (:709) | I | :703, :709 | Y |
| X21-23 | Player check; CI and REG; technical-director audit before the merge to main | I | :703 | Y |

**Cut for the earliest first match:** X4b, X7, X8, X9, team and colour choice, X16, X17, X18.
- Saving: 23 to 39 runs (sums in est.py).
- Calendar saving: small, because no cut item is on the critical path (part 4). The cuts that shorten the calendar are a ready-only lobby and teams off the lifecycle path.
- Cost: the Decided engineer rule (:798) stays unmet until MP2, and the engineer has no function. Rally, charge-over-time and cancel move to MP2. The MP1 row content is Proposed until gate 1 (:701), so the MP1 spec can move them. Moving them postpones a Decided rule; it does not reopen it.

## 2. Pre-MP1 slice

**Recommendation:** a separate small slice. Start it at the MC merge, in parallel with the MP1 spec and gates. Give it a short design note and one gate with two lenses (technical-director, qa-lead). Do not put it in MP1 plan tasks 1 to 3.

Dependencies:
- P1 (audit crate) changes economylog.rs.
- P2 (one movement model) changes sim orders.rs (plain-move code :3101-3652, nearly half of the 3,673 production lines), movement.rs, and economylog.rs (`validate_pending_movement_dispatch` :4282).
- So the order is P1 first. Then P2 deletes no validator branch. Then P3, the grid cell at movement.rs:1402-1403, in the same step function.

| MP1 task | economylog.rs / turts_audit | sim orders.rs | movement.rs |
|---|---|---|---|
| X4 | yes (payment) | yes | yes (footprints) |
| X6 | yes, if power is in the ledger | no | no |
| X7 | yes | no | no |
| X8 | yes | yes | no |
| X9 | no | yes | no |

X1, X2, X10 to X19 (except X17) and X20h touch none of these files.

Why not tasks 1 to 3:
- As plan tasks, P1 to P3 put 14.5 runs in series before X4. That makes the critical path 46 to 55 runs.
- Done during the gates, the slice leaves the path at 39 runs.
- The CEO already decided what the slice does (audit :47-48), so a full Anvil buys little.
- Constraint: audit :51 puts this work after the MC merge. T23 and T24 are NOT STARTED (board :92-93).

## 3. Sizing at the MC rate

MC actuals (ledger, 135 runs):
- Run types: 55 build, 58 review, 22 repair (41/43/16 %).
- Task runs: 135 against a plan of 106 (plan:227). T23 and T24 are still open. Projection: about 140 to 145 runs, 1.3 to 1.4 times the plan.
- With gates (23 + 19, plan:228-229): about 177 to 187 runs. That is under the plan total of 212 (:231) and 1.8 to 3.1 times the early 60 to 100.
- Overruns: T16 22/6, T22 11/3, T20 7/3, T18 5/2. Underruns: T4 2/8, T15 3/5.
- Lane time per run: 1.4 to 2.9 h. T16 had about 30 h between the T15 and T16 merges, a lower bound. T6 had about 23 h between the T4 and T6 merges. MC ran under the single lease until C391.

| Item | MC analog (actual runs) | Runs (B/R/rep about 40/45/15) | Lane hours |
|---|---|---|---|
| P1 | T8 (3) + T4 (2) | 4-6 | 8-12 |
| P2 | T6 (8) | 5-8 | 10-20 |
| P3 | T6 runs 46-50 | 2-4 | 4-8 |
| X1 | T3 (5) | 3-5 | 6-10 |
| X2 | T5 (4) | 4-7 | 8-14 |
| X19 | T2a+2b (4) | 3-5 | 6-10 |
| X11 | T7 (4) + T2 | 6-9 | 12-18 |
| X12 | T16 (22) | 6-14 | 12-30 |
| X13 | T16 screens + T12 | 3-5 | 6-10 |
| X4 | T4 + T6 | 6-10 | 12-24 |
| X5 | T14 (3) / T11 (3) | 4-6 | 8-12 |
| X6 | T5 + T12 | 4-6 | 8-12 |
| X7 / X8 / X9 / X10 | T4 / T5 / T14 / T7 | 2-4 / 3-4 / 2-3 / 2-3 | 4-8 each |
| X14 | T13 (7) / T15 (3) | 4-7 | 8-14 |
| X15 | T17+T18+T19 (13) | 8-12 | 16-24 |
| X16 | RUN57 + T13 | 8-16 | 16-32 |
| X17 / X18 | T15 / RUN131-135 (5) | 2-3 / 4-6 | 4-12 |
| X20 | T20+T21+T22 (25) | 12-20 | 24-40 |
| X21 / X22 / X23 | T23 (plan 3) / T24 / RUN126-127 | 3-5 / 2-3 / 2 | 4-10 |
| Gates | MC 42 | 20-35 | - |

**MP1 totals:**
- Slice 11-18 + tasks 85-139 + gates 20-35 = **116-192 runs**. With assets: **124-208**. With the cuts: **101-169**.
- Calendar: critical path 39 to 55 runs × 1.4 to 2.9 h = **55 to 160 h** of build. Gates and the person play come on top.

**MP2 (coarse):**
- Bot framework 10-16, bot content 6-10, remembered sightings 4-8, UnitId index (Tasks:227, audit :35) 3-5, bot slot in the lobby 3-5, instruments 8-14, audit 2. Gates 15-25.
- Total: **51-85 runs**. Calendar: about 35 to 100 h.
- MP1 + MP2 = 175 to 293 runs, against the early estimate of 95 to 150 (gap register :113).
- Sequencing finding: MP2 (:724) asks for remembered sightings, but fog comes at M5 (:727). With no fog, nothing hides a unit. needs decision in the MP2 spec.

## 4. Dependency graph

Model: scratchpad file graph4.py, no cycle. Inputs are inferred unless the item names a spec line.

| Item | Inputs | Hot files | GPU |
|---|---|---|---|
| X1 | MC | content_m3_validation.rs, version_table.rs, schema.rs, combat.rs | no |
| X19 | MC | messages.rs, export.rs, replica.rs | no |
| X15a (map, units, HUD on the current schema) | MC | mods/ only | no |
| X16 | MC | assets.rs, schema.rs, visuals.rs, Cargo.lock | yes |
| X18 | MC | client_shell.rs, widgets/mod.rs | yes |
| X2 | X1 | schema.rs, combat.rs | no |
| X4 | X1, P1, P2 | sim orders.rs, movement.rs, schema.rs, turts_audit | no |
| X9 | P2 | sim orders.rs, turts_interaction lib.rs | yes |
| X11 | X1, X19 (X10 only if teams stay) | session.rs, schema.rs | no |
| X20h (harness) | X19 | tests/, acceptance.sh | no |
| X6 | X1, X2, P1 | combat.rs, machine.rs, schema.rs | no |
| X14 | X2, X19 | visuals.rs, feedback.rs | yes |
| X12, X13 | X11 | client_shell.rs, client_screens.rs, session.rs | yes |
| X5 | X4, X19 | lib.rs `Core::step` (:137), visuals.rs | yes |
| X7, X8 | X4 (X8 also X2) | orders.rs, combat.rs, turts_audit | no |
| X15 (bind the new fields) | X15a, X4, X6, P3 | mods/ | no |
| X20a | X20h, X11, X15 | tests/ | no |
| X20b | X20a, X5, X12-X14, (X16c-X18) | tests/browser | yes |
| X21 → X22 → X23 | in series | ci.yml | X21 yes |

Critical path with the slice done during the gates and teams off the path: X1 → X2 → X6 → X15 → X20a → X20b → X21 → X22 → X23 = **39 runs**. With the slice in series: 55 runs.

Lanes the graph allows (graph4.py):

| Run offset | Lanes | Items that need the GPU |
|---|---|---|
| 0-4 | 6 | 2 |
| 4-8 | 7 | 2 |
| 8-12 | 4 | 1 |
| 12-16 | 7 | 4 |
| 16-20 | 4 | 2 |
| 20 onward | 1-2 | 1 |

The tail X20b → X23 is 16.5 runs, all in series. Each feature lane must own its own pixel rows (as R10 did, plan:123), so that X20b holds only the full-match driver.

Three hot files that limit parallelism (economylog.rs and movement.rs are handled by the slice):
1. **sim orders.rs** (7,036 lines; 12 MC commits). Touched by X4, X8, X9 and P2. Split: P2 deletes the plain-move code. Then split by behaviour: queue, move, attack, gather, build, repair. The behaviour dispatch now sits in `run_orders` :697 and `apply_order` :2610.
2. **schema.rs + version_table.rs**. Touched by X1, X2, X4, X6, X16 and X18. All version rows are added at one place (:21-55). Split: one schema file and one row list for each domain. Then edits are additive and do not conflict.
3. **client_shell.rs** (4,807 lines; production code to :2115). Touched by X12, X13 and X18. Split: bindings menu, settings, focus overlay, pointer lock into separate files. Put the lobby and the result screen in new files.

## 5. Decisions before gate 1 of MP1

- **a. Scripting.** It does not block MP1 or MP2. All rules in 16.2 are data or Rust mechanisms. MS is order 7 (:728). MB needs MS (:729). The bot is a Rust framework (G8, :724). Recommendation: keep it Open. A rule that needs code is a finding, not a script.
- **b. First real assets.** Options:
    - (1) Placeholders: Capsule and Cuboid (schema.rs:48-51).
    - (2) One static glTF and one texture set for each type. This is a new asset kind; the current kinds are images, fonts, sounds, cursors (assets.rs:5-10).
    - (3) Wait for M13 (:737).

  Option 2 touches Open items: importer features (:750), D61 formats (:90), packaging (:764). Record it as an interim Proposed format, not as the M13 choice. It fits in its own lane, off the critical path. It displaces 8 to 16 runs (about 9 %) and GPU time. It re-baselines the VIS1, SEL2 and SEL3 pixel rows; MC T13 took 7 runs. The content must be original (:782): who authors the models? NOT TRACED. Recommendation: option 2 with a cap, because a blind tester must tell the infantry types apart (:708).
- **c. Lobby.** Two slots, 1v1, ready only. The team field stays fixed (team = slot). Teams then move to MP2 or M5. Recommended.
- **d. Bots.** None in MP1. The headless match uses `--command-script` and `--actions`. Recommended.
- **e. Sound.** Options: none / a minimal set (fire, hit, death, built, low power, ready, error) / voices. Recommendation: the minimal set, 2-3 runs; the O14 follow-up asks for an error sound.
- **f. Names, art direction and numbers** (:807, :768). Recommendation: game-designer writes one page of names, palette and draft numbers before gate 1, and the CEO approves it. For simultaneous destruction, recommend "draw".
- **g. Charge-over-time.** Options: pay at the order (fits :801) or pay over time. Recommendation: pay at the order in MP1.
- **h. "Two people".** Options: two humans / the CEO and the player agent. needs decision.
- **i.** Keep or move X8, X9 and X4b.

## 6. Risks, each with an early test (first two days)

1. **Lobby and screens overrun as T16 did (22 runs against 6).** Test: a pixels-only stub row. Two clients reach the lobby, press ready, and the match starts. Red if the stub needs more than 4 runs.
2. **Units cannot pass buildings on flat ground.** Routing round obstacles is M4 or M4b scope (O14 :33), and navigation is Open (:752). Test: a headless probe on `muster` with the footprints of a command centre, a power station and a turret, and 20 units sent behind them. Count `Blocked` units and units that never arrive. If units are stuck, a scope decision is needed.
3. **One GPU for a two-client match.** Two headed Chrome have been tested on T7 only, with n=2 (RUN125). Test: two headed clients for 10 minutes with combat. Record frame p95 and the wait for the GPU lock.
4. **Tail of the re-freeze.** Fault plants in orders.rs, movement.rs and economylog.rs must move. Test: run the G-REF selector (plan:69-70) on those three files and count the cases. Run a P2 prototype and record the change of the M2 stream hash.
5. **The numbers do not end a match, or end it too fast.** Test: a scripted headless match with the draft numbers. Red if a rush cannot destroy a command centre in N minutes, or if a command centre dies before the first power station.

Also: the MC merge (T23, T24) gates the start, and hosted T7 is red on the software renderer (RUN134).

## What I ran

| Command | Claim tested | Result that would refute | Observed |
|---|---|---|---|
| grep content_m3_validation.rs | spec :809 names three live rules | the static-weapon rule still present | 2 rules left (:375, :423) |
| ledger parse (scratchpad ledger.txt) | MC used about 176 runs (audit :49) | ledger plus gates far from 176 | 135 + 42 = 177 |
| `git log` on the merge commits | lane time per run | - | windows 10-30 to 11-01 and 11-09 to 11-11 |
| commit count per hot file since bb611bd | audit list of hot files | low counts | 16 / 12 / 12 / 11 / 11 / 7 |
| graph4.py | the graph has no cycle | cycle error | no cycle; path 39 runs |
| grep power, construct, team, rally, gltf | these items are new work | matches found | none found |

## NOT TRACED

- The C391 text about worktrees for reviewers and the audit before each merge to main. C391 lines 29-39 hold the rest.
- Whether the Bevy "3d" feature includes glTF.
- Wall-clock hours of the gates.
- The detail of the RUN125 two-Chrome result beyond T7 with n=2.

## What I changed

None. Scratchpad files: /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/graph4.py, /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/est.py, /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/ledger.txt.

The report is longer than the 600-word rule, because the request asked for six dense parts.
