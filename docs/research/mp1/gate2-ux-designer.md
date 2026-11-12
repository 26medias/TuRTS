# MP1 plan gate 2: player interface (ux-designer)

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS HIGH=3

The plan covers every item of D section 7 and the Player path, and the D line pointers are good. Three defects decide what a person sees at C1. My scratch notes are in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/gate2-ux/ (`evidence.txt`, `items.py`, `items.out`).

**Two sources could not be found.** The E1 to E10 list and the 10-point first-60-seconds list are NOT TRACED. "E10" has 0 hits in every worktree copy of the MC design, and `git log --all -S'objective marker'` returns nothing. The first-60-seconds list has 0 hits in the vault, memory and the mc-fix-pt1 specs. I used the list in your prompt. I built the check list below from D:1763-1764, D:2461 and `docs/research/mc/pt1-player-seat-first-minutes.md` section F(2).

## Findings, High first

- **H1 (P:22, P:169). The unit types look the same in stage 1.**
  - Stage-1 units are shapes from a two-variant enum (`schema.rs:48-51`, Capsule or Cuboid). `VisualDef` has no colour and no label (`:501-504`).
  - E3 (name labels) and E10 (shapes that show the type) are not in stage 1.
  - MC failed on exactly this ("Nothing has a name … the same blue blob", pt1 seat item 4). At C1 the person must find the Ore Hauler, and must tell a Rifleman from a Launcher and a Tank, in the first 3 minutes.
  - Repair: add to `C-UNITS`: "Write one shape for each type, all different in form, radius and height (table in the report)." Add to `C1-PLAYER`: "From one screenshot, name each own unit type."
  - Or, **needs decision**: a name-label world style in stage 1 (`UI-HUD`, plus 1 to 2 runs).
- **H2 (P:847, P:195). The C1 start can fail the same way the CEO's MC start failed.**
  - The sheet says "flags NOT TRACED", and `WEB-NOGPU` is stage 2. The CEO's MC start failed here (`pt1-playlog.md:7`).
  - The flags are traced: `--enable-gpu --use-angle=vulkan --enable-features=Vulkan --enable-unsafe-webgpu` (`vault/Reports/2026-10-28 M3 browser adapter startup.md:5`).
  - Repair: put that exact Chrome command in P:847. `scripts/mp1-play.sh` prints it. Move `WEB-NOGPU` to stage 1 (2 runs, merge after `L0-DBG`, because both edit `web-lib.rs`).
- **H3 (P:771-777). `C-UI` has no checks at all.**
  - It has no step, no red, no gate and no plant, which breaks P:74. No test checks the words.
  - `forge_guide.rs` exists (`crates/turts_pack/tests/forge_guide.rs`); no `frontier` equivalent is in any task.
  - Repair: add a step: "Red: `crates/turts_pack/tests/frontier_texts.rs`. Each hint, alert, reason, role and result text equals its D text, byte for byte (7.2, 7.6, 7.9, 7.10, Player path). Each step hint that needs the Command Centre contains 'select your Command Centre'. Each key letter in a hint equals the default binding of that action in `input/input.ron`. Gate: F-PACK. Plant: change one hint word; the test is red."
  - The reviewer stays `player`.
- **M1 (P:443-447, P:517). Lane 0 does not compile in its own order.**
  - `L0-API1` adds variants to matches that have no `_` arm. I checked each by grep: `widgets/values.rs:201-233`, `sources.rs:64`, `:120`, `:184`, `:1215`, and `feedback.rs:620-837`. `L0-API1` does not name these files, and `L0-CORE`, which runs later, leaves out `feedback.rs`. So the `L0-API1` gate (F-RUST, F-WASM) goes red.
  - `WidgetAction` (`primitives.rs:13-20`) needs a placement action (wanted by both `UI-HUD` and `UI-PLACE`) and `FocusAlert` (wanted by `UI-ALERT`). No task owns either.
  - Repair: `L0-API1` Step 2 adds arms that keep the 0.2.3 behaviour in each listed match. `L0-CORE` Files adds `feedback.rs`, `widgets/primitives.rs` and `widgets/activation.rs`, with the `WidgetAction` variants `Place{constructor,content}` and `FocusAlert`.
  - The contract task is already in lane 0 (`L0-API1` plus `L0-CORE`). No new task is needed.
- **M2 (P:775). No task writes the stage-1 sounds.**
  - Stage 1 needs one placeholder sound for each alert family (D:2049, D:2348) and the GHO1 error sound.
  - `C-UI` writes only `interface/` and `input/`.
  - Repair: `C-UI` also writes `mods/frontier/assets/sounds/` (6 sounds copied from `forge`) and their `assets.ron` entries.
- **M3 (P:743-747, section 3.4). The MC PT1 repair is not in the MP1 base.**
  - `codex/mc-fix-pt1` (`feedback.rs` +16: one alert for each wait) is not an ancestor of `pre-mp1/integration` or `mp1-design`. The plan names it 0 times.
  - Repair: add a 3.4 row: "`feedback.rs`: the PT1 repair, merged forward; `UI-ALERT` keeps the one-alert-per-wait test."
- **M4 (P:176). The first person check comes late.**
  - The first headed run that shows `frontier` is `I-PIX1` (offset 27-31). `C1-PLAYER` then also waits for `REG2-1`, which is 2 more headed runs. A FAIL loops through all of it again.
  - Repair: the inputs of `C1-PLAYER` become `I-PIX1`. Its first-minute screenshots run before `REG2-1`; the full play runs after it.
- **M5. Two things a person meets in the first 3 minutes are not in stage 1.**
  - The alert-list row pitch and clip defect (part of MCX1, `UI2-MC`, stage 2) shows with the first alert, at about 40 s.
  - A right click with the Command Centre selected gives `no_applicable_order` (`smart.rs:170`). `frontier` has no text for it, and E7 (a notice near the pointer) is not in MP1.
  - Repair: move the alert-list part of MCX1 into `UI-ALERT`. `C-UI` writes the text "The Command Centre cannot move. Select units, then right-click."
- **M6 (P:841-850, P:922-931). No first-minute check in C1 or PT3.** The plan has 0 hits for "first 60" or "first minute". Add this text to both:
  > First-60-s check (observer, from the screen): (1) the person says the goal within 5 s; (2) points at the own Command Centre and the Ore Hauler; (3) names the first action from the hint; (4) no alert line or sound before the first input; (5) P or the entry shows the ghost and the ring; (6) a red ghost shows a reason the person can repeat; (7) the click shows the site and a new hint in the same frame; (8) "+300" and the credit change by 40 s; (9) the person says the next action; (10) no raw id, "[]", "Slot 0", "x1 - HP" or key list on screen. Record `turts_debug.hints`. Any miss is a FAIL routed by the section 8 table.
- **M7 (P:894). Stage 2 has no named writer for `mods/frontier/interface/`.** HUD6/s2, the RES1/s2 statistics rows and the connection banner all need `frontier` layout lines. Repair: `C2-CONTENT` owns `mods/frontier/interface/` in stage 2 and merges after `UI2-HUD`, `UI2-CONN`, `UI2-MC` and `UI2-RESULT`.
- **Low.**
  - L1: the lobby goal line (D:2972) is a shell screen, but D:1986 makes the goal line a mod widget. Needs decision for `L-LOBBY2-UI`.
  - L2: hints name keys as plain letters (D:2990-2997), so they are wrong after a rebind. D says so; record a `{key:action}` placeholder for M8.
  - L3: the turret offline mark is `FX2` (D:2152), stage 2, but step 10 shows it.
  - L4: `economy_ui.rs`, named in CLAUDE.md, does not exist in either client tree.
  - Point 16: I confirm the lead. `UI-HUD` run 1 must probe whether `Activate` fires on a right click (P:1092).

## Interface items (task, wave, stage, enough to build)

| Item | Task | Wave | Stage | Enough |
|---|---|---|---|---|
| Goal line | C-UI | W6 | 1 | yes (D:1986), no check (H3) |
| H1, H2, H4-H7, F, X1-X4 data | C-UI | W6 | 1 | yes (P:773) |
| Hint engine, X4, `start_selection` | UI-HINT | W9 | 1 | yes |
| H3 | C2-CONTENT | W20 | 2 | yes |
| Ghost, rings, reasons | UI-PLACE plus C-UI texts | W8 | 1 | yes; the held-order "2" outline has no row |
| Queue rows, cancel | UI-HUD | W9 | 1 | partly (point 16 NOT TRACED) |
| Tooltip role and cost lines | UI-HUD plus C-UI | W9 | 1 | yes |
| Alert kinds, sound gap | UI-ALERT | W9 | 1 | yes |
| Alert family sounds | none | - | 1 | no (M2) |
| Result screen, outcome and reason | UI-RESULT plus C-UI | W9 | 1 | yes |
| Result statistics | UI2-RESULT (layout writer unnamed) | W19 | 2 | partly (M7) |
| "Waiting for the other player" | L-LOBBY-C1 | W8 | 1 | yes (P:586) |
| Lobby | L-LOBBY2-UI | W18 | 2 | yes, except L1 |
| NOGPU1 | WEB-NOGPU | W17 | 2 | yes; stage is wrong (H2) |
| "+N" | L-FX-PRES | W8 | 1 | yes |
| C1 instruction sheet | none (only the sign-off note) | - | 1 | no (H2) |

## E1 to E10 and the own-slot predicate

| Request | Where in MP1 | Met at C1 in the first 3 min? |
|---|---|---|
| E1 guide pages | not in MP1 (guide is C2-CONTENT) | no; the hints replace it |
| E2 hide no-op order, "OFF" | not in MP1 | no |
| E3 name labels | not in MP1 | **yes** (H1) |
| E4 objective marker | not in MP1 | no; H7 points to the minimap |
| E5 `Format` drops an empty value | not in MP1 | low risk if C-UI avoids empty fields |
| E6 alert list, inventory text | UI2-MC, stage 2 | **yes, alert list** (M5) |
| E7 notice near the pointer | placement reasons only (UI-PLACE) | **yes**, the right click (M5) |
| E8 alert filter by wait reason | `GathererIdle` split only | no |
| E9 control-group row as one line | not in MP1 | low; C-UI layout |
| E10 shapes that show the type | models are stage 2 | **yes** (H1) |
| Own-slot predicate | `PrimaryOwned` exists (`interface.rs:235`) | no |

## Overrun risk

- `UI-PLACE` (5 runs): first deliver the headless `Ghost` with CON2 by run 2, then the rings. Have `qa-lead` build the GHO1 one-point-per-cause map as a fixture first.
- `UI-HUD` (5 runs): probe point 16 in run 1, and review at half way, after HUD5.

## What I ran

| Command | Claim tested | What would refute it | Observed |
|---|---|---|---|
| `items.py` over P | each item has an owner | NONE for an item | goal line, family sounds, first-60-s list: none |
| grep `_ =>` in the dispatch matches | P:517, lane 0 compiles | a wildcard arm | 0 in every match |
| `git merge-base` mc-fix-pt1 against both bases | the base has the PT1 repair | exit 0 | exit 1 for each |
| grep E10 and "first 60" in every worktree | the source lists exist | a hit | 0 hits |
| `wc -l` on the S45 widget files | the split | - | `sources.rs` 2455, `tree.rs` 996, `values.rs` 378 lines |

What I changed: none.
