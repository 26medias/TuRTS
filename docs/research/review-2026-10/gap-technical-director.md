VERDICT=fail ITEMS=69 MISSING=16 PARTIAL=31 CONFLICT=8 RUNS=15 NOT_TRACED=4

# Gap audit: technical-director (architecture, decision register, 4.1 to 4.4, 12)

Seat lens: ownership. For each yardstick area, I ask: does a 4.1 module own it, does a 4.3 row split it,
does a register row give it a status, does a seat own it. Then I audit the architecture items and add my own gaps.
Spec = `RTS-Engine-Project-Specifications.md` v0.7 (809 lines), cited as S:line.
Inventories: A-xx = `inventory-engine.md` ids, B-nn = `inventory-player.md` Part A2 item numbers.
"M2D:n", "M3D:n", "M4D:n" = line n of the M2, M3, M4 design files (the vault uses this form, for example
`vault/Decisions/2026-10-27 M3 scope and orchestrator calls.md:23` "refines M2D:98 and M2D:509").

## Part 1: area ownership (one row per yardstick area)

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A1-IN | Input foundation: owner | partial | 4.1 Presentation lists "input" S:122. No 4.3 row (S:155-165 has 11 rows, no Input). No register row; D54 S:77 names keyboard/mouse only. Seat: bevy-rendering-engineer (design:110 "input"). | B | none | new control milestone before M4 | D54 | yes: input.rs:23-60 is the only input path | Module yes, policy split no, status no. |
| A2-PC | Player control and selection: owner | missing | No 4.1 module names selection. No 4.3 row. No D. Terms: select (hits are unrelated: "selected", "selection" of libraries), drag 0, control group 0, hotkey 0 (grep-terms.txt). No seat (design:128-164). | B | M4 draft K1 only (M4D:27) | new control milestone before M4 | A1-IN, A5-CA | yes: Selection(pub Option<u32>) economy_ui.rs:31 | No module, no row, no D, no seat. |
| A3-OR | Orders: owner | partial | 4.1 Simulation core "orders" S:121. 4.3 Abilities row "Orders, targeting" S:160. Group orders, batching, persistent intentions: Proposed S:342. 16.2 combat orders S:747 are mod policy for one mod. | B | M1 (MoveTo) | control milestone | OR11, OR24 | yes: MoveTarget(pub Vec2) world.rs:33 is one target, no queue | Engine order model is not specified. |
| A4-GM | Group movement: owner | partial | 4.3 Movement "formation primitives" S:156; D63 S:86 Baseline; 13.6. Seat world-engineer. | M | none | M4 nav or control milestone | OR24 | NOT TRACED | Owner exists; design is world-engineer's audit. |
| A5-CA | Camera: owner | partial | D37 S:60 Decided "mods control camera". S:101. 4.1 Presentation row S:122 does not name camera. No 4.3 row. 15 S:701 "camera/lighting API" Open. Seat bevy-rendering-engineer (design:110). | B | M2 fixed framing | control milestone | A1-IN, MP09 | yes: vault M2 camera framing note says "adds no zoom, pan" | Decided status, no module, no row. |
| A6-UI | User interface / HUD: owner | partial | 4.1 Presentation "UI" S:122. 4.3 Interface row S:164: engine "UI primitives and controlled state access", mod "resource displays, command panels, tooltips, screens". No D. No design seat. | B | M2, M3 text panels (M2D:513-529, M3D:46) | control milestone | TD-6, TD-8 | yes: see UI27 | Row exists; the built UI contradicts it. |
| A7-CO | Construction: owner | partial | 4.3 Construction row S:158; 16.2 S:745. No section; no seat row in design:128-164. | B | none (M3 K2 map-placed machines) | after control milestone | IN01, A5-CA | NOT TRACED | Placement ghost needs picking (IN01). |
| A8-PR | Production and technology: owner | present | 7.3 S:408-410, 8 S:422-428, 4.3 Progression S:161. Seat simulation-core-engineer (design:151). | B | M2, M3 | - | - | no | Owner present. |
| A9-EC | Economy: owner | present | 7 S:390-420, 4.3 Economy S:157. Seat simulation-core-engineer. | B | M2 | - | - | no | Owner present. |
| A10-CB | Combat: owner | partial | 4.3 Combat row S:159; 6.2 S:243 names combat. No spec section. No seat row (grep combat in design:128-164: 0 hits). | B | M3 K1 minimal | M7 or earlier | OR20 | NOT TRACED | Combat has a row but no section and no seat. |
| A11-FV | Fog and vision: owner | partial | 4.3 Visibility row S:162; 6.9 S:338; 13.5. No section. Seat simulation-core-engineer via 13.5 (design:156). | B | M5 (discovery only) | M5 | A20-TM | NOT TRACED | Fog rendering (UI14) has no owner. |
| A12-UN | Units: owner | partial | 4.2 S:136-145 compositions (no air, transport, garrison rows). 4.3 Entities S:155. | B | M1-M3 | - | MD01 | no | Illustrative list; acceptable. |
| A13-PF | Pathfinding: owner | present | D63 S:86, 13.6. Seat world-engineer. | M | M4 draft | M4 | - | no | Owner present. |
| A14-MF | Match flow: owner | partial | 4.3 Match rules S:163; D21-D28 S:44-51; 6.6-6.8; seat backend-engineer. Client screens (menu, lobby UI, score, defeat) have no module; "trusted client shell" S:130, S:384 is named but has no 4.1 row. | B | M9, M10 server side | M9 + shell decision | TD-2 | no | Server side owned; client side not. |
| A15-RO | Replays and observers: owner | partial | D41 S:64 "Replays ... secondary"; D30, D31 S:53-54; 6.11. No 4.1 module, no 4.3 row for replay. | M | M11 (spectators) | M11 | SV04 | no | Replay has status, no module. |
| A16-SV | Saves: owner | conflict | D41 S:64 Decided: no local saves. Server save contract Baseline S:253. No 4.1 module names persistence. Seat backend-engineer. | M | none | M10 or later | D41 | no | See SV01. |
| A17-MP | Maps and map tooling: owner | partial | D36 S:59, 5.6 S:211-215, D38 S:61. Terrain module S:125 owns heights only. Map format, metadata, editor: no module, no row. Editor: D43 S:66 + S:720 "Do not precommit to a large manual editor suite". | B | M1 MapDef, M4 heightfield | M4 + tooling decision | TD-8 | NOT TRACED | Map tooling has no owner. |
| A18-MD | Modding surface: owner | present | 5.1-5.7; D05 S:28, D15 S:38. Seat mod-runtime-engineer. | B | M1-M3 | - | - | no | Owner present; presentation part missing (UI27, MD09). |
| A19-BT | Bots: owner | present | 17 S:753-786; D45-D49. Seat ai-engineer. | B | M12 | - | BT03 | no | Owner present. |
| A20-TM | Teams and diplomacy: owner | partial | 4.3 Match rules "teams" S:163, "diplomacy" mod policy. No section. | B | none | M9 | MF02 | no | Owner by row only. |
| A21-AU | Audio: owner | partial | 4.1 Presentation "audio" S:122 is the only spec hit (grep audio: 1, sound: 0). No 4.3 row, no D, no 5.8 audio assets (S:225 lists model/animation only). Design table seat text has no audio (design:110); agent body has it (bevy-rendering-engineer.md:11). | B | none | control milestone (feedback) or M13 | MD09 | yes: no bevy_audio in client tree (run 4) | One word, no design. |
| A22-AN | Animation and effects: owner | present | D60-D62 S:83-85, 5.8, 13.4. Seat bevy-rendering-engineer. | B | M13 | - | - | no | Owner present. |
| A23-AC | Accessibility and settings: owner | missing | Terms: accessib, rebind, key binding, settings menu, colour-blind, subtitle: 0 real hits ("locali" hits are "localized"). No module, no row, no D, no seat. | M | none | control milestone (bindings) | TD-9 | no | Nothing. |
| A24-PE | Performance the player notices: owner | partial | 13; "immediate input acknowledgement" Baseline S:243. Seat performance-architect. | M | M7, M12 | - | OR23 | no | Local feedback (PE07) has no design. |

## Part 2: architecture items

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| IN01 | Screen-to-world picking | partial | S:610, S:688 name picking for rendering only. Code: ground_point intersects plane y=0, input.rs:14-21. | M | M4 draft (M4D:262) | M4 | heightfield | yes: input.rs:14-21 | Picking ignores terrain today. |
| IN02 | Action mapping layer | missing | hotkey 0, rebind 0, shortcut 1 (S:97 "privileged shortcut", unrelated). | M | none | control milestone | D54 | yes: input.rs reads raw MouseButton | Engine mechanism; mod gives default keys. |
| IN08 | Browser input capture | missing | Platform matrix Open S:702. MDN Keyboard API: reserved keys are "typically captured by the user agent", API "Experimental" (run 11). | M | none | control milestone, after a browser probe | IN02 | no | Ctrl+N control groups may not reach the page; probe first. |
| PC01 | Single select | partial | Not in spec. M2D:513 "left click selects one owned unit/static producer". economy_ui.rs:31. | M | M2 | - | IN01 | yes: one-slot type | Exists by milestone call only. |
| PC02 | Box select | wrong-milestone | Not in spec. M4D:27 K1 (draft, not gated). The M3 playtest already needed it (brief). | M | M4 draft | control milestone before M4 | IN01, OR24 | yes: economy_ui.rs:31 | Planned after the milestone that needed it. |
| PC08 | Control groups (B-11) | missing | control group 0, Ctrl 0. M4D:23 lists hotkeys and control groups out of scope. | M | none | control milestone | IN02, IN08, TD-9 | no | CORE for B-11. |
| PC17 | Ownership gate | present | S:243 validated commands; code NotOwner command.rs:278-279. | M | M1 | - | - | no | Works. |
| PC18 | Shared control | conflict | D52 S:75 Decided "only one controlling session per claimed slot". | B | none | M9 or later | D52 | no | Compatible only as a per-unit permission, not a second session on the slot. Needs decision. |
| PC19 | Selection hygiene; groups survive reload | missing | No term. D41 S:64 no local game saves; S:253 "Browser package caches are not local game saves". | M | none | control milestone | TD-9 | yes: validate_selection economy_ui.rs:1272 handles one id only | Persistence home for groups is undefined. |
| OR11 | Per-unit order queue (B-10) | missing | queue hits are production queues (S:142, S:410). "persistent intentions" Proposed S:342. Code: MoveTarget(pub Vec2) world.rs:33. | M | none | control milestone | - | yes: world.rs:33 | Shift-queue needs a sim order model. |
| OR20 | Ability / mod-defined order | partial | 4.3 Abilities row S:160. Wire PlayerCommandKind is a closed enum of 6 variants, messages.rs:54-78; wire_reason is exhaustive, outbound.rs:55. | B | none | M7 or control milestone | TD-6 | yes: closed enum | A mod cannot add an order without an engine edit and a protocol bump. |
| OR22 | Rejected-order feedback (B-17) | partial | Reject reasons on the wire messages.rs:92; panel shows the last result M2D:519. No reason-to-player-text rule. | B | M2 | control milestone | UI27 | no | Text exists; voice/alert path missing. |
| OR24 | Group order fan-out (B-6) | conflict | Proposed S:342 "Support group orders". Code: MoveTo carries Vec<UnitId> (messages.rs:55-58) but validate_move_to rejects the whole command if one unit is unknown, foreign or static (command.rs:274-282). | M | M1 | control milestone | - | yes: command.rs:274-282 | A box selection that holds one turret, or one unit that died in flight, moves nobody. |
| OR31 | Command budget | partial | Proposed S:342 budgets. Code: MAX_UNITS_PER_COMMAND = 256, command.rs:19. | M | M1 | M12 | D39 | yes: command.rs:19 | 256 is below D57's 1,000-unit benchmark; a 300-unit box move fails TooManyUnits. Budget is Open; do not pick a number without a measurement. |
| UI27 | Mod-defined HUD / UI definitions (MD08) | conflict | 4.3 Interface S:164: command panels and tooltips are mod policy under the Decided principle S:151. M2D:521-522: "command verbs/status strings are engine UI". turts_mod_api has 0 hits for camera, hotkey, icon, sound, audio, cursor, voice, portrait (run 3). | B | M2 | control milestone | TD-6, TD-8, D34 | yes: economy_ui.rs is engine-owned HUD | Milestone call drifted from S:164. Not a section 2 conflict. |
| MD09 | Audio definitions | missing | 5.8 S:225 model/animation sources only. No mod API field (run 3). | B | none | with A21-AU | A21-AU | no | |
| MD12 | Lobby settings schema | partial | D21 S:44 "configure mod-provided rules"; 6.6. Seat backend + game-designer. | B | none | M9 | - | no | Owner present. |
| MD14 | Event scripting | partial | D34 S:57 language Open; S:201. M2 has no script layer (vault M2 scope). | B | none | when a rule needs a script | D34 | no | Do not close D34. |
| RO05 | Replay/save version safety | present | D42 S:65; S:253; invariant 10 S:548. | M | none | M10 | - | no | |
| SV01 | Single-player save and load | conflict | D41 S:64 Decided: "no standalone solo mode or local game saves". S:253 allows server-hosted bot matches. turts_server is native only (wasm32 check excludes it, CLAUDE.md). | M | - | - | D41 | no | Skirmish in the browser needs a reachable server. Name, do not reopen. |
| MP02 | Map editor | conflict | D43 S:66, S:720 Decided "Do not precommit to a large manual editor suite". | M | none | tooling decision | MP01 | no | Soft conflict: a modder-facing editor has no planning home. |
| MP07 | Map distribution | present | Maps ship in packages: D20 S:43, 6.5. load_package accepts one map per package (M4D:28). | M | M1 | - | - | no | |
| BT03 | Same command path for AI and humans | present | S:761 "validated command facilities", S:774. Not an invariant. | M | M12 | - | - | no | See TD-5. |
| PE07 | Local feedback / latency hiding (OR23, B-30) | partial | "immediate input acknowledgement" Baseline S:243. No ground marker, no voice, no design. | M | M1 ack only | control milestone | A21-AU | no | |
| AN08 | Tick interpolation | present | S:243; M1D:299; interp.rs. | M | M1 | - | - | no | |
| CA01 | Classic RTS camera | present | D37 S:60; S:101 "expected for the default game". | B | M2 | - | - | no | Status only; no controls. |
| CA05 | Camera controls: pan, zoom, edge scroll, bounds (B-25) | missing | zoom 0; pan hits are unrelated words. Vault M2 camera framing: "adds no zoom, pan". | B | none | control milestone | IN02, MP09 | yes: render.rs camera fit | D37 says mods constrain; engine must supply controls. |
| CA15 | Height-aware camera | wrong-milestone | M4D:263 "camera fit includes the highest cell" only. | M | M4 draft | M4 | heightfield | no | Fit, not follow. |
| AU03 | Positional audio and limiting (AU07) | missing | sound 0, audio 1 (S:122). | M | none | control milestone or M13 | A21-AU | yes: add audio feature | |
| AC01 | Rebindable keys (B-77) | missing | rebind 0. | M | none | control milestone | IN02, TD-9 | no | |
| MF01 | Main menu / client shell | partial | "trusted client shell" S:130, S:384; no 4.1 row. | M | M1 join flow | shell decision | TD-2 | no | |

## Part 3: gaps the yardsticks did not ask about

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| technical-director-1 | No 4.1 module for the client interaction layer (selection, groups, action map, command composition, smart command, order feedback) | missing | 4.1 S:119-128 has 8 modules; Presentation S:122 is render/input/audio/UI; Multiplayer S:124 is wire. Neither owns client-only game state. | M | none | spec amendment before control milestone | - | yes: logic lives in economy_ui.rs and input.rs | Add a module row and a register row. |
| technical-director-2 | "Trusted client shell" has no module row | partial | Named S:130 and S:384 (identity, acquisition). Not in 4.1 table. | M | M1 | spec amendment | - | no | Owns identity, settings, menus, server browser. |
| technical-director-3 | 4.3 has no rows for Input, Camera, Selection/Control, Audio | missing | 4.3 rows S:155-165: Entities, Movement, Economy, Construction, Combat, Abilities, Progression, Visibility, Match rules, Interface, AI. | B | - | spec amendment | - | no | Without rows, nobody splits default keys (mod) from the action map (engine). |
| technical-director-4 | Invariant 1 covers only the gameplay API | partial | S:539 "same supported gameplay API". The default HUD is engine code (M2D:521-522), so invariant 1 cannot go red on UI. | B | - | spec amendment | UI27 | yes: economy_ui.rs | Extend invariant 1 to presentation, or add an invariant 15. |
| technical-director-5 | No invariant for client-local state and command parity | missing | S:173 covers detail level and view switching only. No invariant says selection, groups and camera never change outcomes, and that every UI action is a public command a bot or external model can send. | M | - | spec amendment | BT03 | no | Test: drive the same scenario through UI and through the headless command path; compare server results. |
| technical-director-6 | Order set is a closed engine enum | conflict | messages.rs:54-78 (6 variants); outbound.rs:55 exhaustive match; D05 S:28 Decided, D15 S:38 Baseline, 4.3 Abilities S:160. | M | M1-M3 | decision before abilities | OR20 | yes: messages.rs, command.rs, outbound.rs | Needs decision: content-id-keyed order envelope, or engine enum per order. |
| technical-director-7 | Group order semantics and per-command cap | conflict | All-or-nothing command.rs:274-282; cap 256 command.rs:19; D39 S:62 "Avoid arbitrary gameplay caps"; budgets Proposed S:342. | M | M1 | control milestone | OR24 | yes | Needs decision: per-unit filter versus atomic reject; cap from measurement. |
| technical-director-8 | Mod API version path for a presentation schema | partial | M3 K4 (API 0.2.1 additive, serde defaults); M4D:30 K3: 0.3.0 breaks the ^0.2 manifests of acme, base, cedar, haul, forge. ENGINE_API_VERSION crates/turts_mod_api/src/lib.rs:28. | M | M3 | every UI/audio/hotkey field | - | no | New presentation fields must be additive with defaults, or the fix carries a planned 0.3.0 migration. |
| technical-director-9 | Client-local persistence is undefined | missing | D41 S:64 "no local game saves"; S:253 caches are not saves; S:384 identity stored locally. No rule for bindings, settings, control groups. | M | - | spec amendment (clarify D41 scope) | - | no | Say that settings and bindings are not game saves. |
| technical-director-10 | Client interaction must run headless | partial | D43 S:66; 16.1 S:722 headless scenarios Proposed. Hit tests need camera and UI nodes (input.rs:23-60). turts_client has --headless (CLAUDE.md). | M | M1-M3 | control milestone | technical-director-1 | yes: input.rs couples picking to panel geometry | Agent tests must drive selection and orders with no window. |
| technical-director-11 | Design table and agent body disagree on the audio seat | partial | design:110 lists no audio; bevy-rendering-engineer.md:11 lists audio. | - | - | roster edit | - | no | Fix with the roster change. |
| technical-director-12 | Selection is owned by the economy UI module | partial | Selection economy_ui.rs:31 (file is 1,425 lines); M4D:357 flags client growth. | M | M2 | control milestone | technical-director-1 | yes | Structural, see lens answer 3. |
| technical-director-13 | Split between shell screens and mod screens undefined | missing | S:164 lists "screens" as mod policy; S:384 keeps identity in the trusted shell. No rule for who owns menu, server browser, lobby, score screen. | B | - | with technical-director-2 | - | no | |
| technical-director-14 | No combat section; combat spans 4.3, 6.2, 16.2 | partial | S:159, S:243, S:747. No section heading (heading list, run 1). | B | M3 K1 | spec amendment | - | no | Owner seat also missing (A10-CB). |

## Conflicts with the goal (Decided items; named, not reopened)

1. **D41** (S:64, Decided): no standalone solo mode, no local saves. Blocks SV01 and offline skirmish. Browser skirmish needs a reachable server, because turts_server is native only.
2. **D43 + 16.1** (S:66, S:720, Decided): no precommitted manual editor suite. MP02 (map editor) is CORE for a moddable engine in yardstick A. No milestone may plan a modder-facing editor without a CEO call.
3. **D52** (S:75, Decided): one controlling session per claimed slot. Blocks PC18 shared control and two-player-one-slot modes if they use sessions. A per-unit permission model is compatible.
4. **D06** (S:29, Baseline, not Decided): "not arbitrary game genres". MP10 custom modes (tower defence, hero arenas) and B-61 avatar control sit on that line. The boundary needs a statement, not a reopen.
5. Milestone drift, not a section 2 item: M2D:521-522 ("command verbs/status strings are engine UI") departs from the Decided 4.3 principle S:151 and the Interface row S:164.

No Decided item blocks drag-select, control groups, hotkeys, camera controls or audio. The gap is absence, not a decision.

## Lens answer

**(1) Areas with no 4.1 module and no 4.3 row.** Player control and selection (A2), camera (A5: D37 gives status, 4.1 S:122 does not name camera), replays (A15), saves (A16), map tooling (A17), accessibility and settings (A23). Input (A1) and audio (A21) have a 4.1 word (S:122) but no 4.3 row and no register row. Match flow (A14) has a 4.3 row (S:163) but its client screens have no module. Register rows the fix needs (numbers follow D64, S:87; status is a proposal for the CEO):

| proposed ID | subject | status proposal | Primary seat | Second |
| --- | --- | --- | --- | --- |
| D65 | Player control model: selection, control groups, smart command, per-unit order queue, group fan-out semantics. Engine mechanism; mod gives filters, caps, defaults. | Baseline after CEO call | new ux-designer seat | simulation-core-engineer (order queue) |
| D66 | Input action map: named actions, mod default bindings, player rebinding, browser key-capture policy. | Baseline after browser probe | ux-designer | web-platform-engineer |
| D67 | HUD model: engine UI primitives plus mod UI data (panels, tooltips, icons, cursors); command verbs from the mod. Must not need a script language (D34 Open). | Proposed | ux-designer | mod-runtime-engineer |
| D68 | Camera controls: pan, zoom, edge scroll, bounds, bookmarks in the engine; mod constraints per D37. Refines D37. | Baseline | bevy-rendering-engineer | ux-designer |
| D69 | Audio: positional playback, mix categories, voice and alert limiting in the engine; sound sets as mod content; extend 5.8. Formats Open. | Baseline; formats Open | bevy-rendering-engineer | mod-runtime-engineer |
| D70 | Client shell and match flow: shell-owned screens (identity, server browser, settings) versus mod-owned screens (HUD, victory, score). | Baseline | backend-engineer (server side), ux-designer (client side) | game-designer |
| D71 | Map tooling: versioned map format in the mod API; editor path (agent-built tool or in-client editor). Needs decision against D43/S:720. | needs decision | world-engineer | mod-runtime-engineer |
| D72 | Client-local persistence: bindings, settings, control groups are not game saves (clarifies D41 scope). | Baseline | web-platform-engineer | technical-director |
| D73 | Order envelope: mod-defined orders cross the wire as content-id-keyed commands (refines D15). | needs decision | mod-runtime-engineer | network-engineer |

**(2) Roster.** It needs a new seat, not a widened one. Evidence: no seat text names UX, HUD, controls or interaction (run 8: 0 hits in the design and 16 agent files). The two candidates for widening sit on the wrong side of a line. `game-designer` owns default-mod rules (design:114), which is mod policy; the control loop is mostly engine mechanism (A1, A2, A3 are M or B). `bevy-rendering-engineer` (design:110) already holds rendering, 5.8, 13.4 and presentation primitives; it builds primitives, it does not own a player-facing control design, and it was the Primary that existed through all eight gates that missed the loop. `player` judges and "does not propose architecture" (design:123). Proposal: a `ux-designer` seat (interaction, controls, HUD, feedback, accessibility), Primary on the new 4.1 interaction module and the 4.3 Interface/Input rows, Second on 3.1 camera; `player` stays the consumer lens. Cost: `docs/agents/check.sh:18` holds a fixed NAMES list and fails on unknown names (check.sh:155-159), so the change edits check.sh, design section 4, the CLAUDE.md lens mapping and the roster budget.

**(3) Client crate.** The CLAUDE.md layout gives turts_client one responsibility list ("fetch, acquisition, join, replica, interpolation, presentation, input"). It does not give the structure for a full RTS front end. The review needs a client architecture decision (a D-row, Baseline). Evidence: Selection lives in the economy UI file (economy_ui.rs:31); picking hard-codes the plane y=0 (input.rs:14-21); hit tests depend on panel geometry (input.rs:23-60); turts_mod_api has no presentation schema (run 3); the client tree has no audio crate (run 4); command verbs are engine strings (M2D:521-522). Direction: separate a client interaction core with no renderer dependency (selection, groups, action map, command composition from turts_mod_api capabilities) from presentation and from the shell, and add a T1 forbidden-edge rule for it, so agents and bots can drive it headless.

**(4) M1 to M3 constraints on the fix.** There are no "M2D/M3D" IDs; the vault uses M2D:n as line references (run 10). The constraints are:
- M1D:94 and CLAUDE.md: turts_client never depends on turts_sim. Smart-command and order-applicability logic must use turts_mod_api content data, not sim code.
- M1D:131: presentation reads ReplicaState and never writes it; client has no FixedUpdate.
- M1D:193-194: protocol coordinates are ground-plane x, y. Picking must convert from the heightfield (M4).
- M2D:513-516: single select, one click queues one unit. The fix replaces this call.
- M2D:521-522: command verbs are engine UI. The fix must reverse this to meet S:164.
- M2D:526-530 and tests/browser/m2.js (exact key sets): HUD changes break the E9 actual-text oracle; plan oracle edits per task.
- Vault M2 camera framing: panel-fit framing with no pan or zoom. Camera controls replace it.
- M3 K4, K5 and M4D:30 K3: API 0.2.x is additive with serde defaults; 0.3.0 breaks five packages. Wire protocol bumps per milestone.
- command.rs:19 and :274-282: the 256 cap and atomic group reject.
- economy_ui.rs:1405: "selection is local, never a server command". Keep this; it supports technical-director-5.

## What I ran

| # | command | claim tested | result that would refute | observed |
| --- | --- | --- | --- | --- |
| 1 | `rtk proxy grep -n -E '^#{2,4} '` on spec | sections cited exist; no combat or input section | a heading for combat, input, camera or audio | 52 headings; none for combat, input, camera, audio |
| 2 | Read S:117-165 | 4.1 Presentation names camera; 4.3 has Input/Camera rows | "camera" in S:122; a row for input or camera | S:122 has no camera; 11 rows, none for input, camera, audio, selection |
| 3 | `rtk proxy grep -rn -i -E 'camera\|hotkey\|icon\|sound\|audio\|cursor\|keybind\|hud\|portrait\|voice' crates/turts_mod_api/src` | mod API has a presentation schema | any hit | 0 hits |
| 4 | `cargo tree -p turts_client -e normal -i bevy_audio` (native and `--target wasm32-unknown-unknown`); control `-i bevy_ui`; `grep -c bevy_audio Cargo.lock` | client has an audio path | a tree printed for bevy_audio | "nothing to print" on both targets; bevy_ui prints (command can print); bevy_audio is in the lockfile (2), so the name is valid |
| 5 | `sed -n 255,296p crates/turts_sim/src/command.rs` | group MoveTo fans out per unit | per-unit filtering | one unknown, foreign or static unit rejects all |
| 6 | `grep MAX_UNITS_PER_COMMAND` | no fixed per-command cap | no constant | 256 at command.rs:19 |
| 7 | Read messages.rs:1-95 | orders are open to mods | a generic or content-id order variant | 6 fixed variants |
| 8 | `rtk proxy grep -i -E 'UX\|HUD\|controls designer\|interaction design\|usability'` on design + `.claude/agents/*.md`; audio count | a seat owns UX/HUD; seat texts agree on audio | any hit; equal audio counts | 0 UX hits; audio 0 in design table, 1 in bevy-rendering-engineer.md |
| 9 | `rtk proxy grep -n` on docs/agents/check.sh | a new seat needs no check edit | no fixed name list | NAMES at :18; unknown names fail at :155-159 |
| 10 | Python id count over milestone designs; `grep -rn -o -E 'M[0-9]+D[0-9]+'` over docs, vault | M2D/M3D decision IDs exist | any id | 0 ids; "M2D:98" line-ref form in vault M3 scope note |
| 11 | WebFetch MDN Keyboard API | browser pages can always capture Ctrl+N | text saying all keys are capturable | reserved keys "typically captured by the user agent"; API "Experimental"; fullscreen and Ctrl+digit not confirmed |
| 12 | `rtk proxy grep -i -E 'accessib\|rebind\|key binding\|settings menu\|...'` on spec | spec covers settings and accessibility | a real hit | only "localized" false positives |
| 13 | Read M2D:509-532 | single select and engine verbs are milestone calls | different text | lines 513 and 521-522 confirmed |
| 14 | Read economy_ui.rs:1398-1422 | selection is client-local by test | no such assertion | assertion at :1405 |
| 15 | `grep` turts_sim for an order queue | sim has a per-unit order queue | a queue type | MoveTarget(pub Vec2) world.rs:33 only |

## NOT TRACED

1. A4-GM, A7-CO, A10-CB, A11-FV, A17-MP code impact: I did not read the sim or client code for these areas; other seats own them.
2. Chromium Keyboard Lock fullscreen requirement, Firefox and Safari support, and Ctrl+digit capture: not confirmed by the MDN page (run 11). A browser probe must decide.
3. Whether tests/browser/m2.js still asserts exact key sets at :358-364: taken from my 2026-10-27 memory note, not re-read.
4. Roster description budget headroom for a 17th seat: not measured.
