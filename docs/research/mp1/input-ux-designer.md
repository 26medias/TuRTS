# MP1 input: ux-designer (player interface)

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=INPUT

Scope note: this is a build-mode design input with nine parts, so it is longer than 600 words. I changed no repository file. The probe files are in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mp1probe/ (trees/*.ron, probe-output.txt).

Status of the lines this input touches: D65, D67 and D73 are Decided lines with Proposed designs (S:94, S:96, S:102). D66 and D70 are Proposed (S:95, S:99). Every choice below keeps per-unit reasons (D65), uses engine widgets with mod layout (D67), and uses content-id orders (D73).

## Facts that this input rests on (verified)

- F1. The input schema has no action scope. `ActionDef` has only `name` and `default` (crates/turts_mod_api/src/input.rs:10-14). The rebind conflict check is global: `key_move` on KeyS conflicts with `key_stop` (client_shell.rs:2810-2822). But design 3.1 says that "an action has ... a scope (world, card, shell)" (mc design:312). This is a design-to-code gap. Consequence: in MP1 each default key must be unique across all cards.
- F2. Command-card cells are absolute. The Grid puts a `CommandCard` item at its `CardCell` (widgets/mod.rs:1100-1107). Two orders with the same cell overlap. So each order needs its own cell.
- F3. `BuildMenu` gives one key for all rows: the key of the producer's Produce order (sources.rs:391-393, :439, :522). MP1 needs one key for each entry.
- F4. Footprints are circles only (schema.rs:28-30).
- F5. `SelectionUnits` gives one item per selected unit, with Count = the count of that type (sources.rs:260-297). So hud.ron:11 shows one row "Rifleman ×12 …" for each of 12 units. Health is null when a unit has no record (sources.rs:285-293), and null displays as "" (sources.rs:1149). That gives "Proving Lab ×1 —  HP".
- F6. `current_hotkey` returns "" when an action has no key (sources.rs:805-816). The shell menu already shows "Unbound" (client_shell.rs:2035-2038).
- F7. Button labels use the font of `tooltip_style` at a fixed 14 px (widgets/mod.rs:627-633). `Button` has no `font` field (interface.rs:66-71). The probe confirms this.
- F8. The authored alert pitch is 28 + 3 = 31 px (hud.ron:88). The observed 62 px has a cause that is NOT TRACED (O15:22). The tooltip view is a separate absolute entity (tooltip.rs:46-59), so it does not cause the pitch.
- F9. The mod API and the protocol have no power, supply, outcome or lobby type (grep; the only match is a key name, input.rs:260).
- F10. Notices are already grouped by content id and reason, but only inside one result or one replica update (feedback.rs:200-227, :264-313). There is one sound per update (feedback.rs:288-301).
- F11. Rules: the command centre produces units and manages construction (S:800). Construction is "Pay credits, select a valid site, then construction progresses over time" (S:801). Low power slows production and disables turrets (S:802). Readiness toggles were "suggestions only" (S:335). Teams are mod options (S:314). A claimed slot stays claimed through a disconnect (S:323, S:348).

## 1. Player path, MP1 (the oracle list of the play test; testers never see it)

Setup: two browser windows (A and B), or two machines. Mouse and keyboard only. The steps are in order. Steps 1 to 15 fit in the first 5 minutes; the steps after that complete the match.

1. A: the load screen shows each phase and a progress bar (existing, mc design:948).
2. A: the lobby shows "Slot 1: You — Not ready", "Slot 2: Open — waiting for a player", and the status line "Waiting for a second player".
3. B joins. Both windows show "Slot 2: Player 2 — Not ready". Defaults are team 1 and team 2, and palette colours 0 and 1.
4. A sets team 2. Both windows show "Both players are on team 2. Choose different teams to start." The Ready button stays available, but the start waits.
5. A picks B's colour: the swatch shows a cross, and its tooltip says "Player 2 has this colour".
6. Both press Ready. The rows show "Ready", then "Match starts in 3", "2", "1". A change by a player clears the ready state of that player.
7. Match start: "Click to play" (existing). The camera centres on the own command centre, and the command centre is already selected. The HUD shows "Credits N", "Power 0/0", and the objective line "Destroy the enemy command centre".
8. A hovers the Power Station entry. The tooltip shows the name, "600 credits, paid while it builds", the time, "+100 power" and "[P]". The numbers come from the rules seat.
9. A presses P. The ghost follows the cursor, solid, in the slot colour. Over the harvester the ghost shows stripes and the text "Something is in the way". Over ore it says "Cannot build on ore". Far out it says "Too far from your base".
10. A left-clicks a valid spot. In the same frame a pending site shows, and the ghost goes. After the round trip the site shows a progress bar at 0 %, the credits count down, and the queue row shows "Power Station 3 %".
11. A presses T, then right-clicks. The ghost goes, and the credits do not change.
12. The station is done: the alert "Construction complete" shows with a sound, and the HUD shows "Power 0/100".
13. A right-clicks ore with the harvester selected. The gather cursor shows, then the gather marker. The load bar over the harvester fills. At the command centre, "+N" rises from the unload point, and the credits go up by N.
14. A selects the command centre and presses I three times. The entry shows a badge "3". The first queue row shows its progress and the seconds left.
15. A right-clicks the ground. A flag shows with a line from the command centre. The new rifleman walks to the flag. The alert "Unit ready" shows.
16. A right-clicks the third queued job. The job goes, "+paid amount" rises at the command centre, and the counter goes up.
17. A builds a turret. Power used becomes more than power produced. The power text turns to "Low power" with a mark, the alert "Low power" shows once, the queue rows say "Slowed: low power", and the turret shows an offline mark.
18. The credits reach 0 during a job. The row says "On hold: no credits" with a striped bar, and the alert "Not enough credits" shows once. The job continues after the next delivery.
19. B selects units, presses A and left-clicks near A's base. The attack cursor shows, then a cross marker at once. The units move and fire at enemies in range by themselves.
20. In the fight: muzzle flashes, tracers, impacts, and health bars on damaged units. A gets "Unit under attack" or "Base under attack", with a minimap ping. A click on the alert line centres the camera.
21. Units die with a death effect, and they leave the selection and their groups. A partly refused order shows "3 of 8 units: Cannot reach that place" with one error sound.
22. A's engineer right-clicks a damaged own building. The repair cursor and the repair effect show, and the health bar rises.
23. A command centre is destroyed. Within 1 s, both windows show "Victory" or "Defeat". After 3 s, the result screen shows both slots, the outcomes and the stats. "Leave" opens the shell screen "The match is over. Reload this page to join the next match."

## 2. Construction

- Start: from the build menu of the command centre (S:800), by click or by the entry key. I did not take the alternative, a builder unit: in 16.2 the engineer only repairs (S:798).
- Order: place first, then the site builds over time while the credits drain (S:801 puts the site before the progress). I did not take build-then-place (the sidebar convention). It is a mod policy, so the game-designer can choose it, and then the UI needs a "Ready to place" entry state.
- Ghost: the engine draws the unit's own visual, translucent, with a footprint ring. It is styled by the mod (`world.placement`). Valid: a solid ring in the slot colour. Invalid: Okabe vermilion `#D55E00`, diagonal stripes, the `forbidden` cursor, and a reason text beside the cursor. Colour is never the only cue.
- Invalid causes and their texts (mod `reasons`; the rules seat picks the set): `site_blocked` "Something is in the way"; `site_outside_map` "Outside the map"; `site_on_resource` "Cannot build on ore"; `site_too_far` "Too far from your base"; `queue_full` "Construction queue is full (n/n)".
- Rotation: none in MP1, because the footprints are circles (F4). I did not take rectangle footprints with R to rotate: that is engine scope with no MP1 need.
- Input: left click places. Shift+left click places and keeps the same ghost for one more. Right click or Esc cancels the ghost, as target mode does (mc design:421-423). During placement, right click never gives an order.
- Check: the interaction core (D74) computes the ghost validity from the replica, and the server decides (D18). A server refusal shows "Power Station: Something is in the way". Under fog (M5), the refusal must not say which hidden unit blocks the site.
- Queue: one active site, and further placed sites wait in order (C&C convention). Waiting sites show as ghost outlines with a number.
- Progress: a world progress bar on the site (`world.progress_bar`), and the model rises or shows scaffolding (rendering seat). The HUD has a `Sites` list with icon, %, state and click-to-centre, visible with any selection.
- Cancel: select the site, then "Cancel construction" (C), or right-click its queue row. The tooltip shows "Refund: N credits" before the click. After the result, "+N" rises from the site.
- Repair: the engineer's card has "Repair" (R, target mode, `repair` cursor). The engineer's right click on a damaged own building or vehicle repairs it (new smart rule). Feedback: a repair effect on the target, the health bar rises, and the tooltip names the cost. If the rules consume the engineer, the tooltip says "The engineer is used up".
- Keys (globally unique, because of F1; plain letters, so the browser does not take them; reserved list at input.rs:51-60). Kept: M move, S stop, H hold, A attack and attack-move, G gather, C cancel. New: P power station, T turret, I rifle infantry, V anti-vehicle infantry, E engineer, O harvester ("ore"), K tank, R repair, L rally. Re-check the table when the names are final. Shift+train key queues five.
- Card cells (unique, because of F2): row 0 move, stop, hold, attack; row 1 gather, repair; row 2 rally, cancel. The build and train entries go in their own `Grid(BuildMenu)` in index order.

## 3. Production

- Queues: two queues on the command centre, structures and units, which run in parallel (C&C convention). This needs named queues for each producer (engine mechanism; production API, S:763); a structure must not wait behind a tank. I did not take one shared queue. Capacity: 5 for each queue (the rules seat sets it).
- Display: each entry button shows a count badge and a fill on the active job's icon. The queue list shows icon, bar and seconds. Left click on the entry adds one job; Shift adds five; right click on the entry or on a queue row cancels the newest job of that type.
- Rally: right click on the ground with the producer selected (a new smart rule, placed before move), or L then a click. On the ground: a flag and a dashed line from the producer, owner-only, shown while the producer is selected, with a pulse when set. A rally on ore makes new harvesters gather.
- Words: the entry tooltip says "Costs N credits, paid while it builds. You have M." A paused row says "On hold: no credits". A row under low power says "Slowed: low power". A full queue disables the entry with "Queue is full (5/5)". Alerts: "Not enough credits" and "Low power" (10 s cooldown).
- Charge-over-time: entries stay enabled while the queue has space, and no up-front cost blocks them. The row tooltip shows "Paid N of M". At 0 credits, the bar shows grey stripes and stops.

## 4. Economy and power HUD

- Credits: `Text(Format("Credits {0}", [Balance(<credits>)]))` works today; probe tree A parses. No income rate: harvester deliveries come in steps, so a rate number jumps. A "+N" float on each delivery shows the income instead. The alternative not taken is a net rate number.
- Power: a vertical bar of used against produced, plus "Power used/produced". In a shortage, a second widget shows "Low power" in vermilion with a mark. This needs the new source `Supply(<id>)` (fields Produced, Used) and the predicate `SupplyShort(<id>)`. Probe tree B fails today with `Power` "Unexpected variant". The name `Supply` is generic on purpose: the same source gives the supply cap of the MB mod.
- Harvester: the info panel shows "Load 300/700 ore" and the activity ("Harvesting", "Returning", "Unloading", "Waiting: field empty", "Idle"). A world load bar shows over a selected gatherer. New: `SelectionPrimary(Activity)`, `SelectionPrimary(Cargo)` and `world.cargo_bar`. New alert trigger: `GathererIdle(s)`, "Harvester idle".

## 5. Combat feedback

- Health bars: keep the menu mode, with "damaged or selected" as the default (mc design:854-856). Add a 0.5 s white chip for the health just lost. Scale the bar width to the footprint, because hud.ron:128 fixes 48 px. The armour class shows in the info panel ("Armour: Heavy"), and weapon tooltips show "Strong against: Vehicles". Armour does not show on world bars.
- Effects for the rendering seat, with the purpose of each:
    - (1) Muzzle flash: who fires.
    - (2) Tracer or projectile: who fires at whom.
    - (3) Impact, full or reduced (from the damage multiplier): to read armour, for example rifles against a tank.
    - (4) Unit turns to face its target: to read the engagement.
    - (5) Death: infantry fall and fade in 1 s; vehicles explode and leave a wreck or scorch for 10 s; buildings collapse with smoke. This confirms the kill.
    - (6) Smoke below 50 % and fire below 25 % on buildings: base health without a click.
    - (7) Turret offline mark: low power.
    - (8) Positional weapon sounds with a cap (deferred from MC, mc design:937).
  Effects (1) and (3) need a permitted hit event for each shot (network seat).
- Alerts: `OwnUnitDamaged` with a new optional `units` filter on `AlertDef`, so the mod can write "Base under attack" and "Harvester under attack". A minimap ping comes with each (exists).
- Attack-move: A, then the `attack` cursor, then a left click on the ground gives a cross marker. An attack on a unit flashes a ring on the target. The waypoint lines exist.
- Grouped failure notice (O14): the client keys each failure by the gesture `group` that it sent (it knows the units of each envelope) and by the reason. Within 2 s, one line updates its count: "3 of 8 units: Cannot reach that place" (several units), or "Rifleman: Target out of range" (one unit). There is one error sound for each group and reason window. A click on the line selects the failed units. D65 holds: each unit keeps its own reason, and different reasons give different lines. This needs the source `Notices` (text, count, select action) and number-neutral reason texts (hud.ron:147 "The unit cannot reach…" must change).

## 6. Lobby and result screen

- Lobby owner: the shell. It is room lifecycle that M9 grows. The fields come from the mod settings schema (S:314): `team` (Choice) and `colour` (from `slot_colours`; give 8, because hud.ron:127 has 2). Ready is a mod-selected start policy, `AllReady(countdown 3 s)`, because S:335 forbids mandatory readiness in the engine. Words: see steps 2 to 6. A leave before the start shows "Player 2 left. Waiting for a player to join." I did not take a mod-owned lobby: it needs pre-match widget sources before M9.
- Result screen owner: the mod (D70, S:99). The engine gives `Outcome` (Victory, Defeat or Draw for each slot), `MatchStats` (for each slot: units trained, lost and destroyed; buildings built and lost; credits gathered; time), and the predicates `MatchOver` and `OutcomeIs(..)`. Stats are readable only after `MatchOver`, because before that they leak enemy state. Layout (mod): a dim full canvas, the headline, one row for each slot, and "Leave", which is `Input("leave_match")`. M8 and M9 can send that shell action to a room list.
- Leaver: a menu item "Surrender" asks "Surrender and end the match? You will be defeated." A disconnect shows "Player 2 lost connection. Waiting 2:00 for them to return." Needs decision (game-designer, rules): a slot that stays disconnected for N s is defeated (recommended; the engine gives a `SlotDisconnected(s)` hook), or the match waits for ever (not recommended: with two players nobody can finish).

## 7. Deferred MC items

- Button `font` and `size`: optional fields. The defaults are the tooltip font at 14 px. Add a version-table row. P1, engine.
- "Unbound": one engine label function for `current_hotkey`, `binding_rows` and the menu. `Item(Title)` gives "Unbound". A hotkey badge shows nothing. P1, engine.
- Controls menu: P1, groups (Orders, Build and train, Camera, Selection, Groups, Interface). P2, keyboard navigation (Up and Down, Enter changes, Delete unbinds, Esc closes). P3, search: it is the first text field (mc design:339), so defer it to M8 (S:732). Engine.
- Alert clip marks and the 62/31 px pitch: the rendering seat. Test: at UI scale 1.0 and 2.0, the rendered pitch equals h + gap within 1 px, and no pixel of row 1 shows above the list top. P1, because MP1 adds more alerts. Engine.
- "Proving Lab ×1 — HP": add the predicate `ItemHas(<field>)` and the predicate `PrimaryHasValue(<field>)`, which replaces the `PrimaryHas(Movement)` work-round at hud.ron:23. Probe tree C fails today. Also add the source `SelectionTypes`, one item for each type (F5). P1 for the predicate, P2 for the source. Engine, plus a mod fix.

## 8. New schema items (all are ids, numbers and strings; no Bevy type)

Engine:
- Sources: `Supply(id)`; `Sites`; `ProductionQueue(queue id)`; `Notices`; `SelectionTypes`; `Outcome`; `MatchStats`.
- Primary fields: `Activity`, `Cargo`, `Armour`.
- Item fields: `State`, `Paid`, `Refund`, `Badge` (the count queued).
- Predicates: `ItemHas`, `PrimaryHasValue`, `SupplyShort`, `MatchOver`, `OutcomeIs`.
- World styles: `placement`, `progress_bar`, `cargo_bar`, `rally`, `float_text`, and a health bar width scaled to the footprint.
- Cursor contexts: `place`, `repair`, `rally`.
- Alert triggers: `ConstructionComplete`, `UnitReady` (or reuse `ProductionComplete`), `LowPower`, `NoCredits`, `GathererIdle(s)`, `SlotDisconnected(s)`; and the `AlertDef.units` filter.
- Button `font` and `size`.
- Core: a placement mode and the effect `Ghost{content, point, valid, reason}` (D74, headless).
- Behaviours: `Place`, `SetRally`, `Repair`. Smart targets: `OwnDamaged`, `Ground` with needs `Producer`.
- Lobby settings schema and a start policy.

Mod: an entry `hotkey` and `card` field on production entries (F3; the field is engine schema, the values are mod content), reason texts, alert rules, the layout, and the result screen.

Bevy and private-path risk: none, if the ghost uses the unit's compiled visual by content id and the effects are named by id. Flag: `Bar(direction: Clockwise)` for icon fills is optional (P2).

## 9. MP1 play test

- Who: the `player` agent drives the headless core and the oracles; two people play in two windows, and one person signs off.
- Goals text: "Play one match against the other person. Each of you starts with a command centre, a harvester and some credits. Win by destroying the other command centre. Get power, collect credits from ore, train an army, build a turret, and fight. Use only the mouse and the keyboard. Say aloud when you do not know what to do next. Stop at the result screen, or after 40 minutes."
- Most likely stops: (1) the player does not find building, because it is on the command centre (fix: the command centre is selected at the start, and an objective and hint line shows). (2) Silent stalls from no credits or low power (fix: the row states, the alerts and the power colour). (3) A red ghost with no clear cause, or no known way out (fix: the reason at the cursor; right click and Esc cancel).

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| scratchpad crate parses 5 RON trees with `turts_mod_api` | D67: today's schema places a card top-left, a build menu left-centre, a queue at Frac and the minimap bottom-right (mc design:806) | tree A fails | A OK |
| same | a power bar, `ItemHas`, Button `font` and a construction source are absent (my parts 4, 7, 8) | B to E parse | all four ERR "Unexpected variant/field" |
| grep `scope` in input.rs and turts_interaction | mc design:312 says actions have a scope | a scope field found | none (F1) |
| grep power, outcome and lobby in mod_api and protocol | MP1 needs new types | an existing type | none (F9) |

Not checked: audio formats, the lobby protocol (backend seat), whether Firefox takes plain letters (NOT TRACED), and the cause of the 62 px pitch (NOT TRACED).

What I changed: none (repository). Scratchpad only.

Open questions for the orchestrator:
- (a) The card scope gap (F1): fix the design text, or add the scope at MB.
- (b) The leaver rule (part 6): needs decision by the game-designer.
- (c) Named queues for each producer: technical-director.
