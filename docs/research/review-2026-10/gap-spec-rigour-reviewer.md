VERDICT=fail ITEMS=31 MISSING=11 PARTIAL=16 CONFLICT=2 RUNS=9 NOT_TRACED=4

Seat: spec-rigour-reviewer. Areas: section 12 (second), 6.7 (second), 6.8 (second), whole document for rigour.
Spec: RTS-Engine-Project-Specifications.md v0.7, 809 lines, 64 register rows (D01 to D64), 54 rows with "Decided" in the status cell, 14 invariants, 13 milestone rows (M1 to M13). Counts from run R7.

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A-PC17 | Ownership gate (order only own units) | partial | S:243 "Clients submit validated commands"; S:305 admin "does not grant control of another player's units"; inv. 13 S:551 "access controls" (generic). No rule says a command on another slot's unit is rejected. | mechanism | M1 (built) | spec text now | 6.8 ownership | No. Client rejects foreign selection (crates/turts_client/src/economy_ui.rs:1421); server path NOT TRACED beyond CLAUDE.md:111-113. | Write the gate as an invariant in section 12. |
| A-PC18 | Shared control (ally orders your units) | conflict | D52 S:75 "only one controlling session per claimed slot" blocks the AoE DE co-op form (two humans, one slot). SC2/WC3 form (cross-slot permission): searched "share control", "shared control", "co-op": 0 hits in S. | both | none | M10 | PC17, D52 | No | Name the conflict with D52; the cross-slot form needs a mod-granted access rule. |
| A-MF02 | Lobby (slots, ready, team, colour, faction, start position) | partial | S:288 count/message/password (Decided); S:290 settings schema, teams as example (Baseline); S:311 readiness "suggestions only"; colour, faction pick: 0 hits. | both | M9 | M9 | MD12 | No | Lobby fields beyond 6.6 have no owner. |
| A-MF05 | AI in slots | partial | S:301, S:786 creator assigns bots (Decided). S:780 difficulty and registration Open. "playable default mod against a conventional bot" only in Proposed extension S:694. | policy + library | none | before first playable | 17, M10 | No | No milestone owns a playable bot. |
| A-MF06 | Start conditions | present | S:317, S:332 (Decided, mod-owned); S:749 first-mod assets (D55). | policy | M9/M10 | M9/M10 | 6.8 | No | |
| A-MF07 | Load barrier and synchronized start | partial | S:311: countdown "suggestions only"; "the loading barrier and failure handling are implementation concerns". No acceptance in M9 S:684. | mechanism | M9 | M9 | 6.5 | NOT TRACED | Barrier is engine work with no test. |
| A-MF08 | Victory and defeat | partial | Rule is mod policy S:163, S:748. Engine "lifecycle" S:163, S:309 is not defined: no match-end event, no per-participant result. "defeat": 0 hits. | both | none | M9 | MF10 | NOT TRACED | |
| A-MF09 | Defeated player observes | missing | Searched "defeat", "eliminat", "observe": 0 player hits. S:328 "Spectating does not claim a player slot"; a defeated owner keeps a claimed slot (D25), so the switch to observer vision is undefined. | both | none | M11 | 10.3 secrecy, D25 | No | |
| A-MF10 | Team victory | partial | S:748 "player or team"; S:163 engine "Players, teams"; teams only an example setting S:290. | both | none | M9 | TM01 | No | |
| A-MF14 | Disconnect handling (wait, pause, drop) | partial | S:303 all-disconnect policy; S:330 mod owns disconnected assets; D28 S:51 bans engine "disconnect gameplay". No wait or pause mechanism. | both | M10 | M10 | UI18 | No | D28 can be read to forbid the mechanism too: needs decision on wording. |
| A-MF15 | Reconnect | present | S:328 "restores the correct player on reconnect" (Decided); S:685 M10 evidence; S:710 implementation Open. | mechanism | M10 | M10 | D50 identity | No (grep "reconnect" in crates: 0 hits, expected before M10) | |
| A-MF19 | Co-op versus AI | partial | Bots S:301 + teams S:290; no text says a bot can be an ally. | policy | none | M9 | MF05, TM01 | No | |
| A-UI18 | Pause by a player | missing | Searched "pause": S:303 only (server policy when everyone disconnects). | both | none | M9 | sim tick control | NOT TRACED | |
| A-UI19 | Surrender and leave | missing | Searched "surrender", "concede", "resign", "forfeit": 0 hits. S:313 "departed player" only. | both | none | M9 | MF08, D25 | No | Leaving keeps the slot claimed (D25); concede needs an engine command. |
| A-TM01 | Teams | partial | S:163 (Decided principle), S:290 example (Baseline). | both | none | M9 | MF02 | No | |
| B-36 | Win and lose screen with reason | partial | Rule S:748; screen: "defeat", "score", "screen" (player sense) 0 hits. | policy on mechanism | none | first playable | MF08 | No | |
| B-38 | Pause, menu, surrender | missing | "menu", "surrender": 0 hits. | both | none | first playable | UI18, UI19 | No | |
| rigour-1 | Goal text | missing | S:95 bar is "possible"/"should be possible"; "ultimate" hits S:700, S:774 only (other senses). | scope | none | spec 3.1 now | CEO goal | No | See Lens answer. |
| rigour-2 | Owner and status of player control (selection, order input, groups, hotkeys, feedback) | missing | S:164 gives mods "command panels"; S:122 gives Presentation "input". No line assigns selection or input to engine or mod. "drag", "hotkey", "control group", "click": 0 hits (grep-terms.txt). | both | none | M2-level, before M4 | 4.3 | Yes: Selection(Option<u32>) crates/turts_client/src/economy_ui.rs:31 | Needs decision. |
| rigour-3 | "Interface" has three meanings | partial | S:42 index protocol; S:97 mod API; S:164 player UI domain. | n/a | n/a | spec now | | No | Split the terms. |
| rigour-4 | "presentation" has two scopes | partial | S:122 module = rendering, input, audio, UI; S:60/S:101 D37 = camera, lighting, visual settings that "mods control". | n/a | n/a | spec now | D37 | No | With S:122, D37 can hand input to mods. |
| rigour-5 | "UI primitives", "controlled state access", "command panels" undefined | missing | One hit each, S:164. No list, no API, no milestone. | mechanism | none | spec now | rigour-2 | No | |
| rigour-6 | "input" has two meanings | partial | Player: S:122, S:243. Factory: S:30, S:141, S:143, S:410, S:678. | n/a | n/a | spec now | | No | |
| rigour-7 | "playable" undefined, no acceptance | missing | S:724 "First playable default mod", S:694, S:755. No definition, no milestone row. | n/a | none | add row | rigour-2 | No | |
| rigour-8 | Milestone acceptance a player can fail | missing | R4: 0 of 13 rows (2 regex hits are false: S:676 "selected packaging", S:678 factory "Inputs"). | n/a | none | every milestone with a client | | No | See Lens answer. |
| rigour-9 | No player-facing invariant in section 12 | missing | Inv. 1 to 14 S:539-552: none on command authority to a human, refused-order reason, or input-to-effect. S:782 "rejected-command reasons" is Proposed, bots only. | mechanism | none | spec 12 now | PC17 | NOT TRACED | |
| rigour-10 | Non-status words carry status | partial | "expected" S:60, S:101; "Selected" S:338; "suggestions only" S:311; "recommendations" S:722. | n/a | n/a | spec now | S:11-16 | No | Map each to one of the four terms. |
| rigour-11 | Lightyear status drift | conflict | S:352 "First prototype recommendation, not selected" vs Cargo.toml:24 lightyear "=0.31.0" in use M1-M3. | n/a | M1 | spec now | D18 | No | Needs decision: record as Baseline, or keep Open with a prototype tag. |
| rigour-12 | Reference list differs from the CEO list | partial | S:9 names Minecraft and Cities: Skylines; omits Supreme Commander. | scope | n/a | spec 1 now | | No | |
| rigour-13 | Participant outcome state (defeated, conceded) | missing | 6.8 table S:319-324 has 4 slot states plus connectivity S:326; no outcome state. | mechanism | none | M9/M10 | MF08 | No | Must not change slot ownership (D25, inv. 11). |
| rigour-14 | CLAUDE.md drift | partial | See list below. | n/a | n/a | now | | Yes: CLAUDE.md | |

## Conflicts with the goal (Decided items; not reopened)

1. D52 S:75 "only one controlling session per claimed slot" blocks AoE DE co-op (two humans control one player). Conflict with A-PC18.
2. D41 S:64 "no standalone solo mode or local game saves" blocks offline skirmish and A-SV01 single-player save. Solo play against bots on a server stays possible. Conflict in part.
3. D28 S:51 "Do not impose ... disconnect gameplay through the engine" can be read to forbid an engine pause/wait mechanism (A-MF14, A-UI18). Wording conflict, not intent.
4. D43 S:66 with S:720 "Do not precommit to a large manual editor suite" can be used against a modder map/scenario editor (WC3, SC2). Lens: modder side of the union.

Decided statements a reader could use to argue player control is out of scope:
- D37 S:60 "mods control camera, lighting, and presentation. Classic RTS view is expected, not enforced." With S:122 (Presentation includes "input" and "UI"), a reader gives all input to mods.
- D05 S:28 with the Decided principle S:151 and the Interface row S:164: mods own "command panels, tooltips, screens"; engine owns undefined "UI primitives".
- D44 S:67 "one-faction C&C-style test mod": "test" lowers the bar of the first game.
- D55 S:78 with S:747: move, attack, attack-move are rules "for this default mod only", "not engine-wide policies". A reader concludes the engine owes no order set.
- D04 S:27 "The official game is a default mod": with D05, a reader puts every player-facing element in the mod.

## Lens answer

Q1. Does 3.1 state "Build the Ultimate RTS engine"? No. S:95 asks only that combinations be "possible". S:95 lists simulation domains and no player control. "ultimate" (R1): 0 hits in the goal sense.

Q2. Sentences in sections 1, 3, 14, 18 with a lower or narrower bar:
- S:7 lists engine features (multiplayer, modding, logistics, terrain, utilities); no player control.
- S:9 "general capabilities can span traditional RTS mechanics": "can span" is a possibility bar; "mechanics" excludes control.
- S:18 "not a complete production schedule or a finished low-level API design."
- S:95 "should be possible" (twice): possibility, not quality.
- S:99 client "must drive compatibility, loading, memory, and rendering validation": no input or usability validation.
- S:101 "A classic RTS camera is expected for the default game but is not an engine restriction": "expected" has no status.
- S:111 "No production commitment to all illustrative mechanics at the first release."
- S:672 "These milestones organize implementation risk": risk, not goal coverage. Section title S:670 makes all milestones Proposed.
- S:676 M1 evidence "presents server results": output only, no input.
- S:694 "playable default mod against a conventional bot" is a Proposed extension; "not a commitment".
- S:790 "The core product decisions are sufficient to begin prototypes": lists settled items with no player control; next step is crowd/navigation probes only.
- S:790 "implement the public APIs against the first mod": the bar is the D44 test mod.

Q3. Undefined terms that hide the gap:
- "Interface" S:164 (also S:42, S:97 in other senses). It needs to mean: the player-facing layer, with an engine part (selection, order input, camera input, feedback events) and a mod part (layout, panels).
- "presentation" S:122 vs S:60/S:101/S:701. It needs one scope; input must be named apart from visual settings.
- "UI primitives" S:164. It needs a list (widgets? selection set? hit test?) and a status.
- "input" S:122, S:243. It needs a player sense distinct from factory inputs S:141, S:678.
- "command panels" S:164. It needs to say which engine data feeds it (available orders per selection, cost, refusal reason).

Q4. Milestone table S:674-688: no acceptance evidence that a player can fail. 13 rows; 0 name a human task or a player-judged result (R4). M3 S:678 evidence passed while the CEO "could identify no objects" (vault/Reports/2026-10-28 M3 human playtest repair.md, per inventory-player.md:416). That is the instrument defect: the table cannot go red on the CEO's complaint.

Q5. CLAUDE.md drift (each line is stale against the tree or the spec):
- CLAUDE.md:7 "holds the M1 Cargo workspace": M3 is merged (commit 7da21e0, 2026-10-28).
- CLAUDE.md:9-11 cites only docs/measurements/m1.md: m2.md and m3.md exist.
- CLAUDE.md:42 "(D01 to D59)": register runs to D64 (S:87; R7 count 64).
- CLAUDE.md:75 and :131-147 "M1 crate layout": turts_sim row says "flat map, movement, command validation"; the crate now holds economy/, boot_m3.rs, command_m3.rs. turts_client row omits economy_ui, scenario, live_view.
- CLAUDE.md:155-162 open list omits the Presentation row of S:701 (camera/lighting API) and says nothing of player control.
- CLAUDE.md:167 "milestones M1 to M12": spec has M13 (S:688). "M1 is the first target": M1 to M3 are done.
- CLAUDE.md:219 "re-check them when Cargo.toml pins a version": Cargo.toml:19 already pins bevy "=0.20.0".
- CLAUDE.md:27-33 "What the project is" does not state the CEO goal.
- Not stale (checked): :8 version 0.7 matches S:3; :20 Bevy 0.20 matches Cargo.toml:19; :188 sixteen agents matches 16 files.

## What I ran

| # | command | claim tested | refuting result | observed |
| --- | --- | --- | --- | --- |
| R1 | rtk proxy grep -rn -i ultimate (spec, CLAUDE.md, specs/) | S:95 states the CEO goal | a goal-sense hit | S:700, S:774 only, other senses |
| R2 | rigour_probe.py term hits | S:164 terms are defined elsewhere | a second hit with a definition | 1 hit each for UI primitives, command panel, controlled state access |
| R3 | python \bpresentation\b, \binputs?\b | one meaning each | all hits one sense | two senses each (lines in table) |
| R4 | rigour_probe.py milestone rows | some acceptance is player-judged | a row naming a human task | 0 of 13 (2 false regex hits) |
| R5 | rtk proxy grep share control/co-op (spec, inventories) | spec covers PC18 | a spec hit | 0 spec hits |
| R6 | cat slots.rs; sed economy_ui.rs:25-40 | M1-M3 code matches 6.8 / one-unit selection | blocked/closed states; multi-select | open/claimed only (slots.rs:24); Selection(Option<u32>) |
| R7 | python counts of D rows, invariants, milestones | CLAUDE.md:42, :167 counts | 59 rows, 12 milestones | 64, 14, 13 |
| R8 | ls crates, docs/measurements; grep Cargo.toml | CLAUDE.md layout and pins current | matching lists | drift listed in Q5 |
| R9 | grep lightyear spec + Cargo.toml | S:352 "not selected" matches practice | lightyear absent from Cargo.toml | Cargo.toml:24 =0.31.0 |

Probe script: /tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/review/rigour_probe.py

## NOT TRACED

1. Server-side rejection of a foreign-unit command (A-PC17): read only CLAUDE.md:111-113, not server code.
2. Load barrier behaviour in M1-M3 (A-MF07).
3. Whether the M4 draft adds any player-control item (not read).
4. Engine pause or tick-freeze support in turts_sim (A-UI18).

What I changed in the repository: none.
