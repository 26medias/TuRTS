VERDICT=fail ITEMS=78 MISSING=36 PARTIAL=38 CONFLICT=0 RUNS=6 NOT_TRACED=77

# Gap audit, player seat

Seat: player. Mode: review-spec. Yardstick: B-1 to B-78 in `inventory-player.md` (A2 items 1 to 38, A2b items 39 to 78; B-n = item n).
The coordinator said "40 union items". The A2b list has 40 items (B-39 to B-78). I rank all 78, because the CORE control loop (B-1 to B-38) is what a player hits first. This is my call; the orchestrator can filter.
Verdict counts come from a Python count over the rows (`tools/rows.py`): 4 present, 38 partial, 36 missing, 0 conflict, 0 wrong-milestone.

## How to read the rank

- `m` = the minute of a normal match when a player first needs the item (0 = the first click).
- `H` = how much its absence hurts, 1 to 5. 5 = I stop playing. 4 = I fight the game all match. 3 = I miss it often. 2 = I miss it in some games. 1 = nice to have.
- `R` = rank. Sort: H high first, then m low first. The orchestrator orders the fix by R.
- Milestone codes. `M4` = needed before the M4 F9 player gate can pass. `FP` = a first-playable milestone for the 16.2 mod. No such milestone exists in S:674-688; S:694 names "playable default mod against a conventional bot" only as "Additional proposed validation". This needs decision. `FP+` = after FP, for the union bar. `-` = done.
- "M1-M3 change?" I did not open code (brief rule for this seat in phase 1; I kept it). Every code claim is NOT TRACED except where a milestone document states it.

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| B-1 | Click-select one unit | partial | S:610 'picking' (render check only); terms: select, click | both; split undecided (S:164) | M1-M3 client (single, not in spec) | M4 | picking (M4D section 9) | yes: Selection is one unit (economy_ui.rs:31 per BRIEF and M4D:57; code not opened: NOT TRACED) | R1 m0 H5. First click of every match. |
| B-2 | Box-select by drag | missing | terms: drag, box (noise only) | both; split undecided | M4D K1 (draft) | M4 | B-1 | yes: same as B-1 | R2 m0 H5. M4D:27 adds it; spec does not. |
| B-5 | Right-click natural order (move, attack, gather, repair) | missing | terms: click, right-click; S:747 lists orders only | both | M1-M3 client has right-click move (G:21) | M4 (move) / FP (attack, gather, repair) | B-1 | NOT TRACED | R3 m0 H5. Order by context is the language of the genre. |
| B-6 | One order to all selected units | partial | S:342 'group orders' (Proposed, cost paragraph) | engine | M4D K1 (one group MoveTo) | M4 | B-2 | no per M4D:265 (MoveTo carries a list); NOT TRACED | R4 m0 H5. |
| B-13 | Selection circle or box on screen | missing | terms: selection (only dependency hits S:305,356,564,664,790) | mod (look), engine (state) | M1-M3 client (not in spec) | M4 | B-1 | NOT TRACED | R5 m0 H5. I must see what I picked. |
| B-15 | Command card or build menu | partial | S:164 'command panels'; S:744 | mod | M2-M3 buttons (not in spec) | FP | B-14 | NOT TRACED | R6 m0 H5. |
| B-21 | Resource bar always on screen | partial | S:164 'Resource displays' | mod | M2 panel (not in spec) | FP | economy | NOT TRACED | R7 m0 H5. |
| B-25 | Camera pan and zoom | partial | S:60; S:101; S:701 (Open) | both | M3-M4 fixed camera fit (M4D:262) | M4 | none | NOT TRACED | R8 m0 H5. |
| B-33 | Workers gather and return on their own | present | S:743 (Decided); S:140 | both | M2 gatherers (M3D:357) | - | economy | no | R9 m0 H5. |
| B-17 | Refused-order reason for the player | partial | S:782 (bots, Proposed); S:243 | both | M4D H6 (fixed reason list) | M4 | wire reject reasons | NOT TRACED | R10 m1 H5. A silent refusal feels like a bug. |
| B-18 | Placement ghost (green or red) | partial | S:158; S:745; S:436 | both | none (M3D:28 no player construction) | FP | construction | NOT TRACED | R11 m1 H5. 16.2 construction cannot be played without it. |
| B-24 | Minimap with click-to-jump | partial | S:338 (data only) | both | none (M4D:22 out of scope) | FP | B-25, fog | NOT TRACED | R12 m1 H5. I cannot find the enemy without it. |
| B-11 | Control groups | missing | terms: control group; 'group' hits S:231,342,566,573,620,624,771 not player | both; split undecided | none (M4D:22 out of scope) | FP | B-2, B-12 | NOT TRACED | R13 m2 H5. No play with two armies or two bases. |
| B-23 | Power shown, low-power warning | partial | S:746 rule (Decided); no display | both | none (M3D:51 excludes power) | FP | utility model | NOT TRACED | R14 m3 H5. 16.2: I must know why the towers stopped. |
| B-8 | Attack-move | partial | S:747 (Decided, 16.2 rule); S:160 | both | none | FP | B-6, combat | NOT TRACED | R15 m4 H5. Rule decided; no input, no milestone. |
| B-28 | Health bars | missing | terms: health | mod (look) | M3 health exists (M3D:27); bars NOT TRACED | FP | health | NOT TRACED | R16 m4 H5. I must see who is losing. |
| B-32 | Units shoot on their own | present | S:747 (Decided, 16.2) | both | M3 turret (M3D:332) | FP | combat | no | R17 m4 H5. |
| B-29 | Alerts: under attack, ping, jump key | partial | S:338; S:634 (data only) | both | none | FP | B-24 | NOT TRACED | R18 m6 H5. I lose with no warning without it. |
| B-36 | Win and lose screen with the reason | partial | S:748 rule; S:163; terms: defeat, score | both | none (G:5 no victory screen) | FP | match rules | NOT TRACED | R19 m10 H5. A match must end. |
| B-14 | Selection panel (name, health, what it does) | partial | S:164 one row | mod | M2-M3 text panels (not in spec) | M4 | B-1 | NOT TRACED | R20 m0 H4. |
| B-3 | Shift add or remove in a selection | missing | terms: shift | both | none | M4 or FP | B-2 | NOT TRACED | R21 m1 H4. Needed as soon as a box picks one unit too many. |
| B-7 | Group movement without clumping | partial | S:156 'formation primitives'; S:624; S:630 | engine | none (M4D:22 defers clearance and collision) | FP | navigation (S:622-630) | NOT TRACED | R22 m1 H4. Units on one pixel look broken. |
| B-12 | Hotkeys for orders and production | missing | terms: hotkey, shortcut (S:97 noise); S:77 keyboard only | both; split undecided | none (M4D:22 out of scope) | FP | input layer | NOT TRACED | R23 m1 H4. |
| B-16 | Tooltip on hover (cost, time, text) | partial | S:164 'tooltips' | mod | M4D H4 (hover cell) | M4 | B-15 | NOT TRACED | R24 m1 H4. |
| B-19 | Production queue and progress bar | partial | S:142; S:158; S:745 | both | M3 research progress in ticks (G:13) | FP | B-15 | NOT TRACED | R25 m1 H4. |
| B-26 | Fog of war (black, grey, clear) | partial | S:162; S:634; terms: explored, shroud | both | none (M5 secrecy only) | FP | visibility | NOT TRACED | R26 m1 H4. Mechanism named; states not. |
| B-60 | Belts, inserters, item flow, contents overlay | present | D07 S:30; S:412-416 (overlay: missing) | both | M3 (M3D:11) | - | production | overlay NOT TRACED | R27 m1 H4. Overlay absent; G:11 sends me to panels. |
| B-9 | Stop and hold position | missing | terms: stop, hold position | both | none | FP | B-6 | NOT TRACED | R28 m2 H4. |
| B-20 | Rally point with a flag | missing | terms: rally (S:366 noise) | both | none | FP | B-19 | NOT TRACED | R29 m2 H4. |
| B-62 | Machine-problem alerts | missing | S:406 accounting only | both | none | FP+ industrial | B-29 | NOT TRACED | R30 m3 H4. M3 factory stops with no sign (G:15). |
| B-31 | A fight looks like a fight | partial | S:159; S:171; S:612 | mod | M3 hitscan, no projectiles (M3D:53) | FP | combat events | NOT TRACED | R31 m4 H4. |
| B-30 | Voice or sound on order | partial | S:122 'audio' only; terms: sound, voice | mod | none (M4D:22 no sound) | FP+ | audio | NOT TRACED | R32 m0 H3. Silence feels like lag. |
| B-35 | Cursor changes per target | missing | terms: cursor | mod | none | FP | B-5 | NOT TRACED | R33 m0 H3. |
| B-55 | Worker build queue with ghosts | missing | terms: ghost, queue | both | none | FP+ | B-10, B-18 | NOT TRACED | R34 m2 H3. |
| B-4 | Select all of one type (double-click) | missing | terms: double, type select | both | none | FP | B-2 | NOT TRACED | R35 m3 H3. |
| B-10 | Shift order queue and waypoints | missing | terms: shift, waypoint; 'queue' only production S:142 | both | none | FP | B-6 | NOT TRACED | R36 m3 H3. |
| B-37 | Cancel build and get money back | partial | S:406 (accounting); S:428 | both | M2-M3 CancelProduction (M3D:411) | FP | B-19 | NOT TRACED | R37 m3 H3. |
| B-69 | Ally ping, chat, team view | missing | terms: chat, ping (noise); S:163 diplomacy | both | none | FP+ | teams | NOT TRACED | R38 m3 H3. Multiplayer is core (D03). |
| B-27 | Last-seen building in the fog | partial | S:780 (bots only) | both | none | FP | B-26 | NOT TRACED | R39 m5 H3. |
| B-43 | Spells, target cursor, mana, autocast | partial | S:160 | both | none | FP+ | B-35 | NOT TRACED | R40 m5 H3. |
| B-47 | Sight blocked by height or trees | partial | S:461 | engine | none (M4D:22 no line of sight) | M4/M5 | terrain | NOT TRACED | R41 m5 H3. Height is in M4; sight is not. |
| B-73 | Select-all-army key | missing | terms: select all | both | none | FP+ | B-12 | NOT TRACED | R42 m5 H3. |
| B-41 | Stances and fire states | missing | terms: stance (noise), fire state | both | none | FP+ | B-32 | NOT TRACED | R43 m6 H3. |
| B-51 | Sell and repair buildings | partial | S:742 repair (Decided, 16.2); terms: sell | both | none | FP | B-15 | NOT TRACED | R44 m6 H3. Repair is a 16.2 rule. |
| B-38 | Menu, pause, surrender | partial | S:303 (all disconnect); terms: surrender | both | none | FP | match rules | NOT TRACED | R45 m8 H3. |
| B-59 | Blueprints, copy-paste, deconstruct box | missing | terms: blueprint, template, copy (noise) | both | none | FP+ industrial | B-18 | NOT TRACED | R46 m8 H3. |
| B-78 | Mod victory modes | present | S:163; S:290 (Decided, 6.6) | mod | none | FP | match rules | no | R47 m10 H3. |
| B-61 | Avatar control (WASD), hand crafting | missing | terms: avatar, character (S:9 noise) | both | none | FP+ | B-25 | NOT TRACED | R48 m0 H2. Only for a Factorio-like mod. |
| B-42 | Hero units, levels, items | partial | S:161 'hero levels' | mod | none | FP+ | progression | NOT TRACED | R49 m2 H2. CORE for a WC3-like mod only. |
| B-49 | Neutral creeps and shops | missing | terms: neutral, creep | mod | none | FP+ | AI | NOT TRACED | R50 m2 H2. |
| B-22 | Supply or population cap shown | partial | S:157 'population rules' (mod) | mod | none | FP+ | B-21 | NOT TRACED | R51 m3 H2. Not in 16.2 rules. |
| B-34 | Idle-worker button | missing | terms: idle (S:618, S:655 noise) | mod | none | FP+ | B-33 | NOT TRACED | R52 m3 H2. 16.2 has one harvester. |
| B-45 | Strategic zoom with role icons | partial | S:604 (render option) | both | none | FP+ | B-25 | NOT TRACED | R53 m5 H2. |
| B-54 | Walls by drag | missing | terms: wall (S:442 dirt walls only) | both | none | FP+ | B-18 | NOT TRACED | R54 m5 H2. |
| B-56 | Engineer assist; repeat build | missing | terms: assist, repeat (S:428 research only) | both | none | FP+ | B-19 | NOT TRACED | R55 m5 H2. |
| B-66 | Tech tree screen | partial | S:424 graph; no screen | mod | none | FP+ | progression | NOT TRACED | R56 m5 H2. |
| B-40 | Formations and formation drag | partial | S:156 (engine primitive) | both | none | FP+ | B-7 | NOT TRACED | R57 m6 H2. |
| B-44 | Tab sub-group cycling | missing | terms: tab, sub-group | both | none | FP+ | B-2 | NOT TRACED | R58 m6 H2. |
| B-53 | Garrison and transports | partial | S:180 'transporting passengers' | both | none | FP+ | B-5 | NOT TRACED | R59 m6 H2. |
| B-72 | Camera bookmarks | missing | terms: bookmark | both | none | FP+ | B-25 | NOT TRACED | R60 m6 H2. |
| B-39 | Patrol | missing | terms: patrol | both | none | FP+ | B-10 | NOT TRACED | R61 m8 H2. |
| B-52 | Capture or convert | missing | terms: capture | mod | none | FP+ | ownership | NOT TRACED | R62 m8 H2. |
| B-63 | Power graph | missing | S:519 aggregate model only | mod | none | FP+ | B-23 | NOT TRACED | R63 m8 H2. |
| B-76 | Pause | partial | S:303 server policy only | both | none | FP | match rules | NOT TRACED | R64 m8 H2. |
| B-65 | Age or tier advance, global horn | partial | S:426 unlocks | mod | M3 research (one tech) | FP+ | progression | NOT TRACED | R65 m9 H2. |
| B-70 | End score screen with graphs | missing | terms: score | mod | none | FP+ | B-36 | NOT TRACED | R66 m10 H2. |
| B-71 | Replays and observer mode | partial | D30 S:53, S:362-368 (observer); D41 S:64 replays secondary | both | M11 (S:686) | M11 | spectator feed | NOT TRACED | R67 m10 H2. Observer planned; replay secondary (Decided). |
| B-77 | Rebindable hotkeys | missing | terms: rebind | both | none | FP+ | B-12 | NOT TRACED | R68 m10 H2. |
| B-57 | Reclaim wrecks | missing | terms: reclaim, wreck | mod | none | FP+ | economy | NOT TRACED | R69 m5 H1. |
| B-58 | Adjacency bonus | missing | terms: adjacen | mod | none | FP+ | construction | NOT TRACED | R70 m5 H1. |
| B-46 | Radar blips | partial | S:618 (radar coverage, cost note) | both | none | FP+ | B-26 | NOT TRACED | R71 m8 H1. |
| B-74 | Force-fire | missing | terms: force | both | none | FP+ | B-8 | NOT TRACED | R72 m8 H1. |
| B-75 | Scatter | missing | terms: scatter | both | none | FP+ | B-7 | NOT TRACED | R73 m8 H1. |
| B-48 | Day and night | missing | terms: day, night | mod | none | FP+ | B-26 | NOT TRACED | R74 m9 H1. |
| B-50 | Veterancy | partial | S:180 'gaining experience' | mod | none | FP+ | combat | NOT TRACED | R75 m9 H1. |
| B-64 | Upkeep | partial | S:157 | mod | none | FP+ | economy | NOT TRACED | R76 m9 H1. |
| B-68 | Global announcement events | partial | S:201 script events | mod | none | FP+ | scripts | NOT TRACED | R77 m9 H1. |
| B-67 | Superweapon timer | missing | terms: superweapon, countdown | mod | none | FP+ | abilities | NOT TRACED | R78 m10 H1. |

Top of the fix order (H5, first minutes): B-1 click-select, B-2 box-select, B-5 right-click order, B-6 group order, B-13 selection circle, B-25 camera, B-15 command card, B-21 resource bar, B-33 gather (present), B-17 refusal reason, B-18 placement ghost, B-24 minimap, B-11 control groups, B-8 attack-move, B-23 power warning, B-28 health bars, B-32 auto-fire (present), B-29 alerts, B-36 win screen.

## Player gaps the yardstick did not ask about

| id | item | verdict | evidence | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| player-1 | A human playability gate on every milestone that has a player | missing | S:674-688 acceptance column: no human or player check. M3D: 0 hits for human, person, playtest, readab, usab. M4D:345-349 adds F9 | both (process) | M4D F9 (draft) | every milestone with a demo | none | yes: M3 closed with no usability verdict (R:25) | The CEO found the gap by playing. No gate asked a person. |
| player-2 | The M4 gate is played by the `player` agent, not a person | partial | M4D:345 "the `player` agent plays the quarry demo" | process | M4D F9 | M4: needs decision | player-1 | no | An agent is not a human hand. It reads labels it was told about. Needs decision: a person, or the agent plus a person. |
| player-3 | M4 cut line removes what F9 needs | conflict (inside M4D) | M4D:363 cuts H8 (the guide) first and the scout rover third. M4D:345-347 F9 needs "the in-game guide" and "send the rover through the passage". M4D:364 says F9 is never cut | process | M4D | M4 | none | no | If the cut happens, F9 cannot pass as written. |
| player-4 | "Use the opened passage" has no felt result for the player | partial | M4D:252-254: the rover can already use the saddle; the passage only "shortens its route". The excavator is the unit that "cannot cross, then can" (M4D:306), but F9 sends the rover (M4D:347) | mod (demo design) | M4D | M4 | none | no | A choice with no consequence. I would not notice the passage mattered. |
| player-5 | A game loop in each demo: something stops me, a way to win | missing | G:5 "no construction, AI opponent or victory screen". M4D:232 "one player slot"; M4D:21-22 no opponent. | mod | none | FP | B-36, AI (S:753) | no | By my four-part test, M3 and M4 are not games. That is fine for a tech demo. It is not fine to call them playable. |
| player-6 | Player words, not studio words, in the game | missing | G:13 "progress in ticks", "`Applied`"; G:3 "All20 implementation entries, C1–C4"; G:23 map coordinates | mod | none | M4 | none | yes: M3 panel text (G:13); code NOT TRACED | |
| player-7 | Start and restart a match from inside the game | missing | G:27 "Restart the server for a fresh match"; G:15 "Start a fresh match after experimenting with removal" | both | none | FP | rooms (S:286-305) | NOT TRACED | Rooms (M9) plan it on the server; the player path is not planned. |
| player-8 | The game starts in my normal browser | partial | G:33 ordinary Chrome gives "No available adapters" and a renderer panic; S:612 WebGPU or WebGL fallback is Open | engine | none | M4 | S:702 platform matrix | NOT TRACED | Not a player rule to fix, but I could not play. |

## Conflicts with the goal

- No Decided row in S:22-87 blocks any of B-1 to B-78. CONFLICT=0 counts spec conflicts only. player-3 is a conflict inside the M4 draft, not in the spec.
- S:164 (Decided principle, S:151) gives the mod "command panels, tooltips, screens" and the engine "UI primitives". It does not say who owns selection, control groups, hotkeys, or camera input. This is not a conflict. It is a hole: needs decision.
- D37 (S:60) says mods control the camera. D54 (S:77) says keyboard and mouse first. Neither says what the keys and buttons do. Not a conflict.
- D41 (S:64) makes replays secondary. A player of SC2, AoE2, WC3, or SupCom expects replays. This is a Decided scope choice, not a block. I name it; I do not reopen it.

## Lens answer: what "playable" means, as a test a milestone must pass

Rule for every gate. A tester gets the game in a normal browser, the in-game text only, and a stop watch. No guide file, no coordinates, no chat with the studio. The tester writes what they did, in order. The gate fails when the tester cannot find an object, an action, or a reason. It also fails when the tester must wait more than 60 s with nothing to do and no sign of progress.

### M3 industrial demo (as in M3D:11 and S:678; slot 0 factory, slot 1 drones, G:7-23)

Slot 0, 10 minutes:
1. In 1 minute, point at and name the drill, belt, arm loader, press, turret, and lab. Say which are mine.
2. In 2 minutes, say where ore goes, step by step, from the vein to the turret and the lab, by looking only.
3. Click the lab before it has shells. Read why research is not ready, in words, with the number missing.
4. See a sign that shells arrive at the lab (a count that rises), so the wait has progress.
5. Start research with a click. See it start, see progress in seconds or a bar, and see it end.
6. Find, with no help, where the turret damage shows, and see it go from 10 to 20.
7. Disable one belt piece. Say what stops and why, from the screen. Enable it. See the chain restart.
8. Try one order the game refuses. Read the reason on the screen.
9. Undo a mistake (Remove) or restart the match from inside the game, with no server restart.

Slot 1, 3 minutes:
10. Tell my drones from the enemy machines by colour or mark.
11. Find the turret on the screen, with no coordinates.
12. Move the far drone into range with a click. See hits land and the health drop. See the drone die.
13. Know whether the enemy's research is done from something I can see (hit size, a message), not from a person.

### M4 ridge demo (as in M4D:8-27, 230-284, 345-349)

One slot, 10 minutes, the five guide lines only (M4D:245):
1. In 1 minute, find the excavator, the scout rover, the ridge, the saddle, and the cut zone by their labels.
2. Hover a ridge cell. Read its height and material. Hover the survey post area. Read why I cannot dig there.
3. Drag a box over both units. See both selected. Right-click east. See the excavator refuse with "No route" and the rover take the saddle.
4. Select the excavator. Press Dig. Drag over the cut zone. See the target outline and "step n of 4".
5. Watch Ore rise to 16 and Spoil to 8. See the cells change shade and colour as layers go.
6. See the passage open (a visible change), with no number to read.
7. Order the excavator east again. See it go through the passage. (This is the unit that "cannot cross, then can", M4D:306.)
8. Order a dig when cargo is full. Read "Cargo full".
9. Order the rover to the east target. See it take the shorter path through the passage.

The gate fails on any step the tester cannot do in time. Steps 3 and 7 test M4's claim that navigation updates; step 7 is what makes "use opened passage" felt (player-4).

## What I would have refused in the M3 Playtest Guide

- G:5 "There is no construction, AI opponent or victory screen". Nothing stops me and I cannot win. That is a demo, not a game.
- G:9 "Use the named **Select** buttons in the left panel". I pick units on the map, not from a list of buttons.
- G:13 "wait until its Main inventory has ten available shells". With G:35 "roughly one minute", that is a minute of waiting with nothing to do.
- G:13 "running progress in ticks" and "Watch **Last command** for `Applied`". Ticks and `Applied` are studio words. I want seconds, a bar, and "Research started".
- G:15 "**Remove** ... destroys their contents. Start a fresh match ... if the factory can no longer supply itself." One wrong click ends my session, and the fix is outside the game.
- G:21 "Move it only after the owner confirms completion; the second client does not receive the factory owner's research records." I must ask another person what the screen should tell me.
- G:23 "The drone positions are (24,12) and (30,4); the turret is at (17,12)." If I need coordinates, the screen failed.
- G:27 "Restart the server for a fresh match; do not open a third client". A player has no server. A third tab must say why it cannot join.
- G:33 "ordinary Chrome can expose WebGPU but return no adapter, causing ... a renderer panic". The game must start in my normal browser, or tell me why it cannot.
- G:3 "All20 implementation entries, C1–C4 and final review repairs are accepted." Studio words in a player guide.
- G:19 drones take hits and never fight back. The drone player has no aim except to die.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `rtk proxy grep -n -i -o` over 80 terms (phase 1, `grep-terms.txt`) | S has no player selection or input rules | a hit for drag, hotkey, control group, click in a player sense | 0 player hits; noise listed |
| same, 40 terms, batches 2 to 4 (phase 1) | S names fog, alerts, minimap only as data (S:338, S:634) | a section that describes a minimap or alert the player uses | only data mentions |
| `rtk proxy grep -n -i -E -o` 16 terms (tab, assist, reclaim, bookmark, scatter, wasd ...) | rows B-44, B-56, B-57, B-72, B-75, B-61 are missing | any hit in a player sense | 0 player hits; double S:543, countdown S:311, force, repeat are noise |
| `rtk proxy grep -n -i -E "human|person|playtest|readab|usab|label"` on M3D | M3 had no human gate (player-1) | an acceptance line with a person | only M3D:607 (label, noise) |
| `rtk proxy grep -n -E "H8|F9|Never cut|Cut line"` on M4D | M4 cut line removes what F9 needs (player-3) | H8 and the rover not in the cut line, or F9 not needing them | M4D:363 cuts H8 first, rover third; M4D:345-347 needs both |
| `python3 -I tools/rows.py` | the VERDICT counts | a mismatch with the table | 78 rows: 4 present, 38 partial, 36 missing |

## NOT TRACED

- Every "M1-M3 change?" cell marked NOT TRACED: I did not open code. The single-unit `Selection` at `crates/turts_client/src/economy_ui.rs:31` is from BRIEF.md and M4D:57, not from my read.
- The minute and hurt scores are my play judgement. They are not measured.
- Part A game facts in `inventory-player.md` are from play memory, not from manuals.
- I read only the M3 and M4 design sections named above (M3D:9-66, 501-535 by grep; M4D:8-40, 230-290, 334-364). The plans in `docs/superpowers/plans/` were not read.

What I changed: none in the repository. Written: this file, `tools/rows.py`, `tools/table.md` in the scratchpad.
