VERDICT=fail ITEMS=38 MISSING=15 PARTIAL=12 CONFLICT=3 RUNS=6 NOT_TRACED=5

Seat: producer. Areas: spec 14 (milestones), 15 (open decisions), 18 (next step). Spec v0.7 = S:line.
M4D = `docs/superpowers/specs/2026-10-28-m4-terrain-design.md`. M3D = `docs/superpowers/specs/2026-10-27-m3-industrial-design.md`.
"Milestone now" = a row of S:674-688. A milestone is a risk slice, not a date (S:672).
Dependency tags: (S) = read from the spec; (I) = inferred by me from the Demonstration column.

Note on the count: the engine yardstick has 24 item areas (A1-A24). A25 is its count section, with no items
(inventory-engine.md:522). Run R3 confirms 24 areas, 355 items, 181 CORE.

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A1 | Input foundation (IN, 10 items, 7 CORE) | missing | S:674-688 grep select/input/hotkey/camera/UI: no player hit (R1). S:122 one cell. | M | NONE | MC (player control) | M1 (I) | yes: `crates/turts_client/src/input.rs:44-45` reads only single left/right press | IN08 browser keys block PC08 |
| A2 | Player control and selection (PC, 23, 14 CORE) | missing | No row. M4D:27 K1 adds box select inside M4 (draft, not gated). | M (filters B) | NONE | MC | A1, M1 (I) | yes: `economy_ui.rs:31` `Selection(pub Option<u32>)` | PC17 ownership gate exists (M1) |
| A3 | Orders (OR, 31, 18 CORE) | partial | M1 move (S:676); attack-move only as mod rule S:747; group orders Proposed S:342 | M + B | M1 (move), M7 (combat) | MC (order model), MP (combat orders) | M1, M3 (I) | yes: queue, stop, hold not traced in code: NOT TRACED | order model needed by bots (S:761) |
| A4 | Group movement (GM, 13, 7 CORE) | partial | S:624, S:630 (Baseline nav); M12 "concentrated battles" S:687 | M | M4, M12 | MC (spread, arrival), M7/M12 (scale) | A3 (I) | NOT TRACED | GM01 spread has no row |
| A5 | Camera (CA, 17, 9 CORE) | missing | S:701 camera API Open; S:101 expected only | M (+B limits) | NONE | MC | M1 (I); CA15 needs M4 | NOT TRACED | vault M2 camera framing is not a milestone |
| A6 | User interface (UI, 30, 16 CORE) | missing | S:164 one row (Decided principle S:151) | B | NONE | MC (primitives), MP (default-mod HUD) | A2 (I) | yes: M3 UI is named buttons (playtest R:3, R:25) | |
| A7 | Construction (CO, 22, 8 CORE) | missing | M3D:28 K2: "Player construction: a separate milestone." No such row. M4D:21 excludes it. | B | NONE | MP | MC, M2 (I) | NOT TRACED | 16.2 needs it (S:745) |
| A8 | Production and technology (PR) | present | S:678 | B | M3 | M3 | M2 (I) | no | |
| A9 | Economy (EC) | present | S:677 | B | M2 | M2 | M1 (S) | no | |
| A10 | Combat (CB, 25, 10 CORE) | partial | M3D:27 K1 minimal combat; S:681 M7 combat | B | M3, M7, M12 | MP (player combat), M7/M12 (scale) | MC (I) | NOT TRACED | ownership drifted M7 to M3 |
| A11 | Fog of war and vision (FV, 13, 6 CORE) | partial | S:680 M5 is geology secrecy; M3D:472 health public "until the fog design" | M + B | M5 (partial) | MP or own row: needs decision | M1 (I) | NOT TRACED | unit fog has no home |
| A12 | Units (UN) | partial | S:676 one unit; air S:113 needs separate design | B | M1 | MB (breadth) | MP (I) | no | |
| A13 | Pathfinding (PF) | present | S:679, S:687 | M | M4, M12 | M4, M12 | M3 (I) | no | |
| A14 | Match flow (MF, 19, 11 CORE) | partial | S:684-685 rooms, slots; victory/score/menu no row | B | M9, M10 | MP (victory, end screen), M9/M10 | MP (I) | NOT TRACED | |
| A15 | Replays and observers (RO) | partial | S:686 spectators; replays secondary D41 S:64 | M | M11 | M11 | M9 (I) | no | |
| A16 | Saves (SV, 5, 3 CORE) | missing | S:694 "server save/restart" is additional validation, not a row; S:710 Open | M | NONE | own row after M7 | M7 (I) | NOT TRACED | |
| A17 | Maps (MP) | partial | S:679 terrain; S:694 generator, two-map switching not rows | B | M4 | M4 + later | M4 (I) | no | |
| A18 | Modding surface (MD) | present | S:677, S:683 | B | M2, M8 | M2, M8 | M1 (S) | no | |
| A19 | Bots (BT, 5, 3 CORE) | partial | S:687 "high-rate AI commands" = load only; S:694 bot is additional | B | M12 (load) | MP (first bot) | MC order model (I) | NOT TRACED | S:755 Decided |
| A20 | Teams and diplomacy (TM, 3, 3 CORE) | missing | "team" only S:748 rule; no row | B | NONE | MP (2 teams) or M9 | M9 (I) | NOT TRACED | |
| A21 | Audio (AU, 8, 5 CORE) | missing | S:122 one word; sound/voice 0 hits (grep-terms.txt) | B | NONE | MC (cues), M13 (assets) | MC (I) | NOT TRACED | |
| A22 | Animation and VFX (AN) | present | S:688 | B | M13 | M13 | M1 (I) | no | |
| A23 | Accessibility and settings (AC, 7, 2 CORE) | missing | rebind 0 hits | M | NONE | MC (rebind, settings) | A1 (I) | NOT TRACED | |
| A24 | Performance the player notices (PE) | partial | S:681, S:687, S:688; PE01, PE04 no row | M | M7, M12, M13 | MC adds PE04 measure | MC (I) | no | budgets Open S:700 |
| producer-1 | No milestone row for player control and HUD | missing | S:670-688 (R1) | M + B | NONE | MC, before M4 | M1, M3 (I) | yes (A1, A2) | first test that goes red: box select |
| producer-2 | No milestone row for the 16.2 playable mod | missing | S:694 additional only; vault M2 decision: "the full section 16.2 validation home stay open" | B | NONE | MP | MC, M2, M3 (I) | NOT TRACED | needs decision (CEO) |
| producer-3 | M12 demonstration needs a bot that no row builds | wrong-milestone | S:687 vs S:755 | B | M12 | MP builds bot; M12 scales it | MP (I) | no | demo needs a missing milestone |
| producer-4 | M7 needs "scripts on both targets"; no row selects scripting | wrong-milestone | S:681; S:705 Open; S:790; vault M2 decision defers language | M | M7 (implicit) | decision before MP or MB | none (S:790) | no | M7 would close an Open item in passing |
| producer-5 | S:15 has no row for input/selection ownership, order model, HUD primitives, audio, unit fog | missing | S:696-714 grep (R1) | both | n/a | add rows | n/a | no | needs decision on ownership |
| producer-6 | M4 draft puts control (K1, F9) inside a terrain milestone with no row and no decision ID | wrong-milestone | M4D:27, M4D:264-267, M4D:345 | M | M4 (draft) | MC | MC (I) | no | keep F9, move K1 |
| producer-7 | Unit fog of war has no home | partial | M3D:472; S:162 | M | NONE | needs decision | M1 (I) | NOT TRACED | |
| producer-8 | Combat ownership drift | partial | M3D:27 (M1 design gave combat to M7) | B | M3/M7 | MP | MC (I) | no | |
| producer-9 | Saves have no row | missing | S:694, S:710 | M | NONE | after M7 | M7 (I) | no | |
| producer-10 | `CLAUDE.md:167` says M1 to M12; spec has M13 (S:688) | missing | R5 | n/a | n/a | fix doc | n/a | no | Low |
| producer-11 | Genre breadth (heroes, abilities, transports, air, naval, formations) has no row | missing | S:111 no first-release commitment; D06 Baseline S:29 | B | NONE | MB (second mod, M2 pattern) | MP, scripting (I) | no | |
| producer-12 | D41: no standalone solo mode or local saves | conflict | S:64 vs SV01 | M | n/a | n/a | n/a | no | name only |
| producer-13 | D10 and S:113 exclude bridges | conflict | S:34, S:113 vs CNC bridges | M | n/a | n/a | n/a | no | name only |
| producer-14 | 16.1: no large manual editor suite in advance | conflict | S:720 vs MP02 map editor CORE | M | n/a | n/a | n/a | no | name only; timing, not a ban |

## Conflicts with the goal (Decided items; not reopened)

- D41 (S:64): saves live on servers; no standalone solo mode. The union bar has offline skirmish save (SV01).
- D10 (S:34) with S:113: no bridges with space under them. CNC maps use bridges.
- 16.1 Decided (S:720): no large manual editor suite in advance. MP02 map editor is CORE for a moddable engine.
  This is a timing rule, not a ban.
- S:111 (3.2, no status label): "No production commitment to all illustrative mechanics at the first release."
  This is in tension with "Ultimate". It limits release scope, not engine scope.

## Lens answer

(1) Areas with no milestone: 9 of 24 (IN, PC, CA, UI, CO, SV, TM, AU, AC). They hold 125 items, 67 CORE (R3).

(2) Proposed direction (not a design):

M1-M3 (done) -> MC Player control and HUD -> MP Playable default mod (16.2) with a first conventional bot ->
M4 Terrain -> M5 -> M6 -> MB Genre breadth (second mod) -> M7 -> M12. M8-M11 have no gameplay dependency
(I). M13 render probes run alongside (S:690).

- MC before M4 (dependency): M4 F9 needs a person to finish the demo with mouse and keyboard (M4D:345).
  So M4 needs MC. MC does not need M4. The client ground is flat today (`input.rs:14-21`). R4 shows a
  cycle when MC also needs terrain picking. Break the cycle: M4 replaces `ground_point` (M4D:262).
- MP after MC (dependency): 16.2 needs attack, attack-move (S:747) and site select (S:745). These are
  player inputs. MP needs M2 harvest (S:743) and M3 health and weapon (M3D:42).
- MP does not need M4-M6. S:751: excavation and utilities "may be separate follow-on scenarios".
- The first bot goes in MP. S:755 is Decided. M12 needs a bot (S:687). The bot uses the MC order path (S:761).
- MP before M4 is risk order, not a dependency. The open risk is human play (playtest R:3, R:25).
  Needs decision (CEO): does "playable" include terrain?
- M4 as drafted: keep GEO, NAV, LOCAL (S:679, D08-D11). Move K1 and H9 to MC. Keep F9.
- M7 after MP and MB: M7 integrates combat and scripts (S:681). They get designs first.

(3) Cost (estimates, not facts). M3 used 75 runs against an estimate of 47 (`vault/M3 Execution Board.md:145,147`).
MC: 0.5-1x M3. MP: 1-2x M3 (six bot controllers, S:765-772). Correction before M4: 1.5-3x M3
(about 115-225 runs). Same order of magnitude as M3. Full union: about 10x M3. M3 took about 2 days.

(4) Urgent Open decisions: Scripts (S:705, D34); Presentation camera API and UI primitive set (S:701, S:164);
Platform matrix browsers, for the Ctrl+1..9 keys (S:702); Default game numbers (S:712); the 16.2 home
(vault M2 decision). Absent from S:15, needs decision: input and selection ownership; order model; unit fog.

## What I ran

| id | command | claim tested | result that would refute | observed |
| --- | --- | --- | --- | --- |
| R1 | `sed -n 670,714p S \| rtk proxy grep -i -E 'select\|input\|hotkey\|camera\|UI\|control\|order\|playable'` | No milestone or S:15 row covers player control (S:674-714) | A row that names selection, input, HUD or camera control | Hits are noise ("Inputs/outputs", "select library"); camera API only in S:701 |
| R2 | `rtk proxy grep -n 'Selection(' economy_ui.rs`; `input.rs` | Client selects one unit (M4D:57) | A set-type selection | `economy_ui.rs:31` `Option<u32>`; `input.rs:44-45` single press |
| R3 | `python3 -I producer/deps.py` (count) | Yardstick has 25 areas (brief) | 25 item areas | 24 item areas, 355 items, 181 CORE; 9 NONE areas, 125 items, 67 CORE |
| R4 | same script (graph, Kahn sort, DFS probe) | The proposed order has no cycle | A cycle | Acyclic; probe edge MC->M4 gives a cycle (detector can go red) |
| R5 | `sed -n` on S:122,151,164,243,342,751; grep `M1 to M12` | Cited lines say what I claim; CLAUDE.md milestone range | Text differs | All match; `CLAUDE.md:167` says M1 to M12 |
| R6 | `rtk proxy grep 'agent runs'` in vault | M3 used about 75 runs | Another total | 75 actual, 47 estimate (Board:145,147) |

Output: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/review/producer/deps.out`.

## NOT TRACED

1. Browser support for capture of Ctrl+1..9 (Keyboard Lock API).
2. Token share of the week for the current model mix (memory gives M1: about 60 runs = 17 % all models).
3. All cost multipliers in (3). They are estimates.
4. Whether the sim command path has an order queue, stop or hold (only `command.rs` via M4D:43-46).
5. Whether D41 forbids an offline skirmish save, or only a local save file.
