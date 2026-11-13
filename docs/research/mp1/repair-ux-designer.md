VERDICT=REPAIRED
---
## Replace: `## 5. The mod `frontier` (content)`
## 5. The mod `frontier` (content)

The content is in a new package `frontier`, not in `base`, because the frozen M2 stream test uses `base`
(`crates/turts_pack/tests/m3_m2_stream.rs:350-366`). All ids passed the game-designer's scan against engine
identifiers (`light` and `shell` collide and are not used).

Display names: Rifleman, Launcher, Engineer, Ore Hauler, Tank, Guard Turret, Command Centre, Power Station. Resources:
ore (an item), credits (a balance), power (a capacity, section 6.2). The screen, the hints, the alerts and the
play-test texts use these names only. The word "harvester" does not show to a player [player 8].

Status: the roster and the rules of S:782-805 are Decided. Names, statistics, timings, costs, repair accounting,
construction restrictions, the power multiplier and the simultaneous-destruction case are Open (S:807). Every number
in this section is a proposal to measure in a headless match. The full stat tables stay in GD parts 1 to 6. The table
below gives only the values that the gate-1 repair changed, with the reason. A balance ratio that fails in a headless
duel is a retune of these numbers, recorded in `docs/measurements/mp1.md`; it is not a red of DMG1. DMG1 is red only on
the formula and the table cells [rigour 6].

Numbers after gate 1 (game-designer pen, repair pass; the scripts are listed after the results):

| Item | Gate-1 value (GD) | Repaired value | Why |
| --- | --- | --- | --- |
| Starting credits | 1,200 | 1,200 | S:805 ("enough credits to establish power"); the other changes remove the holds |
| Ore Hauler cost, build time | 800, 20 s | 500, 15 s | a lost hauler can be replaced [player 5] |
| Ore Hauler load, gather, unload, speed | 200, 10/s, 40/s, 2.0 | 300, 15/s, 60/s, 2.4 | income on the main field 332 to 519 credits a minute [player H3] |
| Ore Hauler at the start | waits for an order | gathers from tick 0: the map placement carries a start order (decision D-F; engine mechanism: a placement `order`, sent through `apply_order` with the source `Engine`, row STO1). Today a placement has no order field (`crates/turts_mod_api/src/schema.rs:440-447`) | the first "+300" at about 30 s with no input [player H1] |
| Free income of the Command Centre | none | 3 credits each second while it is complete (decision D-L, engine capability `income`, row INC1) | the floor: a slot with 0 credits and no hauler gets a hauler in 166 s [player 5] |
| Rifleman build time | 6 s | 7 s | the spend rate of one unit queue (14.3 credits/s) is near one hauler plus the free income |
| Launcher build time | 9 s | 10 s | the same |
| Engineer cost | 300 | 250 | the same |
| Tank build time | 18 s | 20 s | the same (35 credits/s) |
| Guard Turret cost, range (health 600 unchanged) | 500, 8 | 600, 7 | defence is not twice as cost-efficient [player H3]; 7 still outranges the tank (6) and equals the Launcher |
| Damage cells rocket to structure, cannon to structure | 50 %, 100 % | 60 %, 125 % | tanks and launchers break buildings; formula unchanged (GD part 1) |
| Command Centre health | 2,000 | 3,000 | a broken base has time to answer |
| Build area | 10 from the edge of an own Command Centre or Power Station | 10 from the edge of the own Command Centre only (radius 12.5 from its centre) | no turret creep to the side and centre ore: those fields stay open ground, so careful players must meet there [player 10] |
| Ore nodes | main 3 x 1,500; side 2 x 1,500; centre 8 x 2,000 | main 3 x 5,000; side 2 x 3,000; centre 8 x 3,000 | the defended main field lasts 9.6 min with 3 haulers, 14.5 min with 2 |
| Placement gap | 0.5 (GD) or 2.0 (6.9) | `placement.gap` 2.0 as data; the packer refuses a gap below the largest mobile diameter + 0.2 (1.8 + 0.2 for the Tank and the Ore Hauler); units count for overlap, not for the gap | [mod L10] |
| Ore clearance | 2.0 from the node edge | 2.0, data `placement.node_clearance` | unchanged |
| Open sites for each slot | GD: 1 | 1, mod data (decision D-E); the structures queue holds up to 5 placed orders | [net 5] |
| Unit queue capacity | 5 | 5 | unchanged |
| Power | station makes 100; Command Centre uses 40; turret uses 30 | unchanged; build time of the Power Station 12 s (24 s at the start, because the start is in low power) | the tooltip shows the time at the current power (7.10) [player 7] |

Results of the scripts (first 180 s unless the row says otherwise; "hold" is a span in which a job cannot pay, with
gaps below 2 s merged, because free income lets a starved job pay in some ticks):

| Run | Gate-1 numbers | Repaired numbers |
| --- | --- | --- |
| S1, the player seat's ordinary script (power, 3 riflemen, turret, tank, 2 riflemen, tank, 2 launchers), hauler from tick 0 | longest hold 31 s, 3 holds | no hold; 445 credits at 180 s |
| S4, power and a second hauler first | longest hold 24 s | no hold |
| S7, the step order of the hint list (Player path) | longest hold 16 s, 7 holds | one hold of 4 s at 95 s |
| S5, tank rush (overspends on purpose) | 31 s | 25 s |
| S8, greedy (overspends on purpose), 300 s | holds up to 33 s; "Not enough credits" sounds with the rev-3 rule: 8 | holds up to 30 s; rev-3 rule: 89 sounds (free income ends and starts the state in most ticks); the rule of 7.9: 1 sound |
| L1b, the only hauler dies at 90 s; at 95 s the player cancels the tank job and trains a hauler | no hauler in 420 s | a new hauler at 110 s |
| L1, the same without the cancel (the hauler waits behind a starved tank job) | no hauler in 420 s | a new hauler at 333 s; hint X1 tells the player to cancel first |
| L2, the floor: 0 credits, no hauler, a hauler queued | never | at 166 s |
| One turret with its power share (690; gate-1: 590) against the cheapest winning attack of riflemen, launchers, tanks and engineers | 800 / 590 = 1.36 | 750 / 690 = 1.09 |
| Tanks only against one turret | 2 tanks: 1,400 / 590 = 2.37 | 2 tanks: 2.03; one tank and two riflemen win at 1.30; one tank alone loses (the turret keeps 100 of 600) |
| Tanks only against two turrets | 3 tanks: 1.78 | 3 tanks: 1.52 |
| match.py, careful against careful with 10 % less fight value | A wins at 11.8 to 14.0 min | A wins at 11.4 min |
| match.py, careful against turtle (6 turrets) | careful wins at 16.9 min | careful wins at 20.2 min |
| match.py, aggressor (2 tanks first) against careful | careful wins at 16.9 min | careful wins at 20.2 min |
| match.py, aggressor against turtle | aggressor wins at 2.9 min | aggressor wins at 3.5 min |
| match.py, two equal careful players | no end in 40 min; ore gone at 27.6 min | no end in 40 min; ore gone at 30.4 min |

What the results support, and what they do not:

- The first 3 minutes: no hold above 4 s for the ordinary scripts S1, S4 and S7. A hold comes only when the player
  overspends (S5, S8), and then the alert sounds one time (7.9) [player H3].
- The loss of the only hauler is not a silent loss: the alert "Ore Hauler destroyed" and hint X1 name the action, and
  the free income is a floor (L1b, L2) [player 5].
- Defence: a mixed attack needs 1.09 times the cost of one turret; a tank-only attack needs 2.03. The role lines (7.10)
  tell the player to bring a mix [player H3, player 6].
- Match length: unequal pairs end in 11 to 20 min with the repaired numbers. The macro model is crude: it has no
  tactics, so two exactly equal careful players stall in it with both number sets. The repaired build area forces both
  players onto open ground after 9.6 to 14.5 min. The headless match (section 14) and the play test measure the real
  length. This is a proposal, not a measurement.

Scripts (Python 3 standard library; run with `python3 -I`; the gate-1 player scripts are the control, and
`pacing.py old` gives the same holds as `docs/research/mp1/probes/player/play.out` S2):
`pacing.py [old|new]` (first minutes, the floor, the alert rule), `fights2.py [old|new]` (battles and the defence
ratio), `match.py` (macro match), all with their output in `results.txt`, now in
`/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mp1-pacing/`. The plan
copies them to `docs/research/mp1/probes/pacing/`.

The ratios that carry the balance (fights2.py, new):

- Fights last seconds and are readable: a rifleman kills a rifleman in 9 s; a tank kills a rifleman in 6 s.
- At equal cost, 7 riflemen lose to a tank and 8 win; 3 launchers kill a tank in 7.5 s; 5 riflemen kill 2 launchers in
  3 s.
- One turret beats one tank; a tank with two riflemen beats one turret; two tanks kill a turret in 10 s.
- One Ore Hauler earns about 519 credits a minute on the main field and pays for itself in about 1 min.
- One Power Station supplies the Command Centre and two turrets; the third turret needs a second station.

Power, in the words the player gets: "When your base uses more power than it makes, everything builds at half speed
and your turrets switch off."

Victory: at the end of a tick a team is alive if one of its slots has a Command Centre with health above 0. One team
alive: it wins. No team alive: a draw for every team that was alive at the start of the tick. The disconnect clause
is mod data in the victory rule: a slot that stays disconnected for 120 s (counted in ticks) is defeated (decision
D-D). A package with no clause gets no disconnect defeat (D28).

Interface content (in `mods/frontier/interface/`): the goal line and the hint list of 7.8 (texts in the Player path),
the alert rules of 7.9, the reason texts of 7.2, the role lines of 7.10, the key defaults of 7.4, the lobby palette
and the result screen of 7.6.

No private path: every rule above is data in the package. No content id of `frontier` appears in engine code; the
no-content scan checks it (row NC1).
---
## Replace: `## 7. Player interface`
## 7. Player interface

This block is the whole section 7, with its subsections 7.0 to 7.10. Status: D65, D67 (line (a) and its constraint)
and D73 are Decided lines with Proposed designs; D66, D70 and D74 are Proposed (S:94-103). Every new item is an id, a
number or a string; the engine supplies widgets and the mod places them (D67). Each new engine item of this section is
for all mods. Section 6.10 (the API table of decision D-I) holds its field, type, default, validation, error text and
version-table row; 7.5 is the interface input to that table.

### 7.0 Terms

- Player slot (slot): the D25 slot. The match start claims it (decision D-B); under the start policy `Immediate` the
  claim stays at Hello. A claimed slot is never released in the match.
- Lobby seat (seat): a reservation in the lobby that the room can release. It becomes a claimed slot at the match
  start. A claimed slot is never called a seat.
- Team: a set of player slots. MP1 has two teams of one slot (decision P2 a).
- Team material: the material that a model names in `team_material` (8.1). The client replaces it with the slot
  material. (Rev 3 called it a "material slot".)
- Site: the server entity of a building under construction. It owns its progress, its charge and its job, it has
  health and a footprint, and it can be attacked (6.1, decision D-E).
- Pending site: the client marker from the left click until the server result. It is not a site and charges nothing.
- Placed order: a `Construct` order with its point in the structures queue that waits for an open site.
- Queue: the list of jobs or placed orders of one producer, named by a queue id (`structures`, `units`). A lane
  (section 10) is a worktree and its task chain, not a queue.
- Alert: a line in the alert list that an alert rule makes (7.9); it can have a sound, a minimap ping and a place.
- Notice: the text of a refused order: at the pointer, at a site, or the grouped failure line (ruling O14). A notice
  is not an alert.
- Hint: the one hint line of 7.8.
- Done, for a hint: its done condition held in one client update (7.8). `done:` is used only as the yardstick
  disposition; "MP1 is done" (section 1) is the exit.
- Click to play: the button that ends the start screen. The clock of the 60 s rule starts when the player presses it.

[rigour 11]

### 7.1 The player path

The oracle list of the play test is the section `## Player path` at the end of this design: lobby lines L1 to L5,
the hint list H1 to H7, F and X1 to X3, and match steps 1 to 19. The testers never see it. The first three steps are
the lesson of the CEO play of MC on 2026-11-11: the goal is on the screen in one line, one hint line names the first
action and its key, the screen shows a reward when the action is done, and nothing sounds before the player acts.

The 60 s rule of PT3: the clock starts when the player presses "Click to play" in that window. It ends at the first
client update in which a site of an own Power Station exists (a server entity; a pending site does not end it). PT3
is red when the end comes later than 60.0 s, or never. The client records both times in `turts_debug.hints`
(`click_ms`, and for each hint `shown_ms`, `done_ms`, and for H1 also `site_ms`) [rigour 10].

### 7.2 Construction

- Start: select the Command Centre, then click a build entry or press its key (7.4). This starts placement on the
  client only; the server gets no order (GHO1). The left click sends `Construct{content, point}`.
- Ghost: the unit's own visual, translucent, with its footprint ring, in the slot colour.
- Rings while the player places [player H4]:
    - the build-area ring of the own Command Centre: a solid line in the slot colour at radius 12.5 (footprint 2.5
      plus `extends_area` 10);
    - a gap ring round each building and site within 15 of the pointer: a thin dashed line at its footprint plus
      `placement.gap`;
    - a clearance ring round each ore node within 15 of the pointer: a thin dashed line at its radius plus
      `placement.node_clearance`.
- The site check runs in this order. The first check that fails gives the reason; the ghost shows one reason only.
  Each reason is a reason key with a mod text (keys are checked at pack time: an unknown key is refused with a "did you
  mean" hint [mod M8]):

| Check | Reason key | Text at the pointer | What the ghost shows |
| --- | --- | --- | --- |
| A part of the footprint is outside the map | `site_outside_map` | "Outside the map" | stripes |
| The centre is farther than `extends_area` from the edge of the own Command Centre | `site_too_far` | "Too far from your base" | stripes; the build-area ring |
| The footprint overlaps a unit, a building or a site | `site_blocked` | "Something is in the way" | stripes |
| No overlap, but the gap to a building or a site is below `placement.gap` | `site_near_building` | "Too close to a building" | stripes; the gap ring of the ghost and of that building turn vermilion, so the missing distance shows between them |
| The distance to the edge of an ore node is below `node_clearance` | `site_near_ore` | "Too close to ore" | stripes; the clearance ring of that node turns vermilion |
| The structures queue holds 5 placed orders | `queue_full` | "Construction queue is full (5/5)" | stripes |

- Valid: a solid ghost ring in the slot colour. Invalid: Okabe vermilion `#D55E00`, diagonal stripes, the `forbidden`
  cursor and the text. Colour is never the only signal.
- Input: left click places. Shift and left click places and keeps the ghost for one more. Right click or Esc leaves
  placement and never gives an order. No rotation (circles, `crates/turts_mod_api/src/schema.rs:28-30`).
- Preview against the server verdict [net 9]: the ghost uses the same pure rule as the server (`turts_rules`), with
  the client replica. The replica is at least one tick plus the latency old (gate1-network.md:78), so the client cannot
  see a unit that walks in, or a site that the other player places in the same tick. On a mismatch the player sees:
  the pending site; then the refusal; the pending site goes; the reason text shows at the site point for 3 s, with one
  error sound; the credits do not change; the hint does not advance (7.8). The client never shows a site before the
  server result (D18).
- Fog (M5): a reason never names a hidden blocker; the placement rule takes the visibility predicate of the target
  queries, which is always true in MP1 [net 6].
- A placed order that waits for an open site shows a ghost outline with its number at its point ("2"); its queue row
  says "Waiting to start".
- Progress: a world progress bar on the site; the site model rises (8.2).
- Cancel: select the site and press C ("Cancel construction"), or right-click its queue row. The tooltip shows
  "Refund: N credits" before the click (N is what the site paid, times the mod ratio). After the server result, "+N"
  rises at the site.

### 7.3 Production

- The Command Centre has two named queues that run at the same time: `structures` (placed orders, capacity 5; one open
  site for each slot, mod data) and `units` (jobs, capacity 5). The left click of 7.2 carries the point, so a
  structures entry waits only for an open site; then the site owns progress, charge and job (decision D-E).
- Each entry shows a count badge and a fill on its icon. A queue row shows the icon, a bar and the seconds left at the
  current power.
- Row states, each in words: "On hold: no credits" (a striped bar); "Slowed: low power"; "Waiting to start" (a placed
  order); "Waiting: exit blocked" (a complete unit job with no free exit point). "On hold: no credits" shows from the
  first tick in which the job cannot pay until the job pays in every tick for 1 s, so it does not flicker under the free
  income.
- Pay over time (decision D-A): entries stay enabled while their queue has space; no cost blocks them at the order. The
  tooltip says "Cost N credits, paid while it builds". The row tooltip says "Paid N of M credits".
- A full queue disables the entry with "Queue is full (5/5)" (units) or "Construction queue is full (5/5)".
- Cancel: right-click a row, or right-click the entry (the newest job of that type). "+N" rises at the Command Centre
  with the refund sound (7.10). A waiting job or order refunds 0, because it paid 0.
- Rally: with the Command Centre selected, a right click on the ground sets the rally (a smart rule before Move), or L
  then a click. A flag and a dashed line from the Command Centre show while it is selected (owner only), with a pulse
  when it is set. A rally on ore makes new Ore Haulers gather; other new units attack-move to the flag.

### 7.4 Keys

The build menu needs one key for each entry; today every row shows the key of the producer's Produce order
(`crates/turts_client/src/widgets/sources.rs:391-393`). The key check is global (no action scope), so each MP1 default
key is unique across all cards. The action scope of MCD:312 is not built in MP1; the MC design text is corrected to
say so. All defaults are plain letters; none is in `RESERVED_BROWSER_KEYS` (`crates/turts_mod_api/src/input.rs:51-68`).

| Action | Key | Where |
| --- | --- | --- |
| Move | M | card |
| Stop | S | card |
| Hold | H | card |
| Attack, attack-move | A | card |
| Gather | G | card |
| Repair | R | card |
| Set rally | L | card |
| Cancel (selected site, or the newest job) | C | card |
| Power Station | P | build menu |
| Guard Turret | T | build menu |
| Rifleman | I | build menu |
| Launcher | V | build menu |
| Engineer | E | build menu |
| Ore Hauler | O | build menu |
| Tank | K | build menu |

Shift and a train key queues five. Esc leaves placement, else opens the menu. The camera, selection and group keys
stay as in MC; KEY1 checks the full list against them (the MC defaults: NOT TRACED here). Each tooltip names its key
("[P]"); a key with no binding shows "Unbound" (MCX1).

### 7.5 HUD sources and schema items

Input for the table of section 6.10 (decision D-I). Each item is additive in API 0.2.4. Presence for the version table
means that the field is written in the source file (decision D-I). When an item is absent, the 0.2.3 behaviour stays.

- Start: `InterfaceDef.start_selection: Option<ContentRef>` (default none: no selection, as today). The camera keeps
  `CameraStart::OwnedCentre` (`crates/turts_mod_api/src/interface.rs:327`).
- Hints (7.8): `InterfaceDef.hints: Vec<HintDef>` (default empty), `hint_fallback: Option<String>`, `Value::Hint`,
  `Predicate::HintShown`. Today `Source` and `Predicate` have no hint (`interface.rs:216-240`).
- Alerts (7.9): `AlertDef.kind: Event | State` (default `Event`, the 0.2.3 behaviour), `clear_after: f32` (default 10,
  `State` only, at least 0), `units: Vec<ContentRef>` (default empty: all units), `select_on_click: bool` (default
  false); the placeholders `{unit}` and `{slot}` in `text` (an unknown placeholder is refused); triggers
  `SupplyShort(res)`, `SupplyRestored(res)`, `InsufficientFunds(res)`, `ConstructionComplete`, `SlotDisconnected`,
  `SlotReconnected` [mod M5]; the alert row as a button with the core action `FocusAlert` (D74: an agent drives it
  headless).
- Power and match: `Value::Supply(res, Used | Made)`, `Predicate::SupplyShort(res)`, `Value::Connection` (the banner
  of a disconnected slot), `Value::Outcome`, `Value::OutcomeReason`, `Value::MatchStats`, `Predicate::MatchOver`.
- Fields: primary `Armour`, `Activity`, `Cargo`; item `Role`, `BuildTimeNow`, `PowerChange`, `CausesShortage`,
  `State`, `Paid`, `Refund`, `Badge`; predicates `ItemHas(field)` and `PrimaryHasValue(field)` (MCX1).
- Content: `UnitPresentation.role: Option<String>` (today `icon`, `portrait`, `responses` only:
  `interface.rs:410-415`); production entry `hotkey` and `card`; Button `font` and `size`.
- World styles: `placement` (valid, invalid, stripes), `build_area_ring`, `gap_ring`, `progress_bar`, `cargo_bar`,
  `rally`, `float_text` (with a sound id).
- Cursor contexts `place`, `forbidden`, `repair`, `rally`, and the reason keys of 7.2 plus `match_not_started` and
  `slot_defeated`: an unknown key is refused at pack time with a "did you mean" hint [mod M8].
- Core (D74): placement mode and the effect `Ghost{content, point, valid, reason}`; the hint state; `FocusAlert`. All
  run with no renderer.

The power source is named `Supply`, because the same source gives the supply cap of a later mod.

### 7.6 Lobby and result screen

Owners: the shell owns the lobby and the "Leave" screen; the mod owns the result screen. This split is D70, which is
Proposed (S:99) [rigour 12]. MP1 has no settings schema (MD12 is `later:M9`): the lobby shows the seats, the colour
from the mod palette, and Ready [rigour 12]. The server state machine (Boot, Lobby, Countdown, Running, Ended, then
exit 0; decision D-C) and its cases are in 6.7. The words below are what each player sees in each case. "Player 1"
and "Player 2" are slot labels (the `{slot}` text).

| Case of 6.7 | This window | The other window |
| --- | --- | --- |
| Lobby, one seat taken | "Slot 1: You — Not ready", "Slot 2: Open — waiting for a player"; status "Waiting for a second player." | none |
| A second player takes a seat | "Slot 2: Player 2 — Not ready"; status "Press Ready to start." | "Slot 2: You — Not ready"; the same status |
| One player is ready | "You are ready. Waiting for Player 2 to press Ready." | "Player 1 is ready. Press Ready to start." |
| Both ready: Countdown | "Match starts in 3", "2", "1" | the same |
| A player changes colour or clears Ready during Countdown | "Countdown stopped: Player 2 is not ready." | the change clears the ready state of that player only |
| A player leaves the lobby or the countdown | the seat row is "Slot 2: Open — waiting for a player"; status "Player 2 left. Waiting for a second player." | none |
| A seated player reloads the page in the lobby | the same seat, "Not ready" (the browser keeps the player id from the seat on, backend M6) | "Player 2 — Not ready" |
| Running starts: the claim of every seated slot (decision D-B) | "Click to play" | the same |
| A new player id during Running | shell screen: "This match has started and has no open slot. Wait for it to end, then reload this page." | nothing |
| The other player's session ends during Running | alert "Player 2 lost connection."; banner "Waiting 1:59 for Player 2 to return." that counts down in match time | none |
| The other player returns | alert "Player 2 is back."; the banner goes | the match continues in the own slot (MC SLOT1) |
| Ended | "Victory", "Defeat" or "Draw" within 1 s; after 3 s the result screen | the same |
| A claimed player connects after the end | the result screen of that match | none |
| A new player id after the end | shell screen: "The match is over. Reload this page in a few seconds to join the next match." | none |
| The page cannot reach the server (the next process is not up yet, D-C) | shell screen: "The server does not answer. It can be starting the next match. Reload this page in a few seconds." | none |

Notes: the session end comes up to about 15 s after a close (backend M7, native probe; the browser lag is NOT TRACED),
so the banner can start late; the 2:00 is counted in ticks from the logged disconnect (decision D-D). A lobby timeout is
a 6.7 choice; if 6.7 sets one, the shell screen says "The room closed: no second player came. Reload this page to open
it again."

Result reasons (decision D-K). The engine gives an outcome and a reason code for each slot (MAT1); the mod gives the
words:

| Reason | Winner | Loser |
| --- | --- | --- |
| The key units of the other team are lost | "Victory. You destroyed the enemy Command Centre." | "Defeat. Your Command Centre was destroyed." |
| No team alive in the same tick | "Draw. Both Command Centres were destroyed at the same moment." | the same |
| Surrender | "Victory. Player 1 surrendered." | "Defeat. You surrendered." |
| The disconnect clause | "Victory. Player 2 left the match and did not return within 2:00." | (on a later connect) "Defeat. You were away for more than 2:00." |

The result screen (mod layout): a dim full canvas, the headline, the reason line, the match time, one row for each
slot (colour, label, outcome, credits gathered, credits spent, units trained, units lost, enemy units destroyed,
buildings built, buildings lost), and one button "Leave" (`Input("leave_match")`). There is no rematch button in MP1
(decision D-K) [player 9]. "Leave" opens the shell screen: "The match is over. This server starts a new match. Reload
this page to join it. If the page shows an error, wait a few seconds and reload again."

The surrender menu item: "Surrender" asks "Surrender and end the match? You will be defeated." with "Cancel" and
"Surrender".

### 7.7 MC items

Button `font` and `size`; "Unbound"; Controls groups and keyboard navigation (search waits for M8); the alert list clip
marks and row pitch; a value predicate so that "x1 - HP" does not show for a unit with no health; the grouped failure
notice of ruling O14 ("3 of 8 units: Cannot reach that place", one sound, a click selects the failed units); and a plain
page when the browser gives no GPU adapter (the CEO met a panic with a console message only, 2026-11-11). The page
text: "TuRTS cannot start: this browser gives no graphics adapter (WebGPU). Turn on hardware acceleration in the
browser settings. On Linux, Chrome has WebGPU off: open chrome://flags, turn on "Unsafe WebGPU Support", and restart
Chrome."

### 7.8 Goal line and hints

- The goal line is a mod `Text` widget with the literal "Destroy the enemy Command Centre" (no new engine item). It
  shows from the first frame after "Click to play" until the result.
- The Command Centre is selected in the first frame (`start_selection`), so its build menu shows.
- The guide does not open at the start. The hint line teaches the first actions.
- One hint line shows at a time, at a place that the mod sets. The full list with its words is in the Player path.
- Hint kinds: a step hint teaches the next action in order; a situation hint answers a state (no Ore Hauler, low power,
  jobs on hold); the fallback line names an action that the player can always do.
- Selection rule, in each client update: the first situation hint whose start holds and whose done does not hold;
  else the first step that is not done and whose start holds; else the fallback line, if a step is not done; else no
  line.
- A step whose done condition already holds when the list reaches it is done at once and does not show.
- Done: a step is done in the first client update in which its done condition holds. It stays done when the
  condition holds through a site, a complete building or an order that the server accepted. When it holds only
  through a pending site, the line changes at once (so placing a building names the next action in the same frame),
  but if the server refuses that order, the step is not done and its line shows again [player H2, rigour 10].
- Conditions are client facts from the replica and the own order results: own units of a type (pending, site,
  complete or any), an own job of a type, no own unit of a type, a rally of the own producer, an accepted own order of
  a type, a supply short, a job on hold for N s, another hint done, a hint shown for N s, and `Not`. The interaction
  core evaluates them, so an agent reads the hint state headless (D74).
- The client records the times in `turts_debug.hints` (7.1).

### 7.9 Alerts

Two kinds of alert rule [player H3]:

- Event: one line for each event, coalesced by place (`coalesce_radius`); the sound plays at most once in each
  `cooldown` seconds for the rule. This is the 0.2.3 behaviour (`crates/turts_client/src/feedback.rs:849-853`).
- State: the line and the sound come when the state starts. A state that holds in the first replica after "Click to
  play" is not a start, so nothing sounds at tick 0. The line stays while the state holds, and nothing sounds again. The
  state ends only after it stays false for `clear_after` seconds. When it starts again, the line comes back; the sound
  plays only if the last sound of the rule is at least `cooldown` seconds old. So a state that clears at each delivery
  and comes back 5 s later gives one line and one sound in each `cooldown` window.

Rev 3 had one rule form, a cooldown per rule (`crates/turts_mod_api/src/interface.rs:375-384`), and MC content gave
seven rules a cooldown of 1.0 s (`mods/forge/interface/hud.ron:130-136`): that was "it beeps non stop".

The rules of `frontier`. Each text names a thing on the screen, and what to do when there is something to do. A click
on a line moves the camera to its place (`FocusAlert`); "Ore Hauler idle" also selects the hauler.

| Rule | Trigger | Kind | cooldown, clear_after | Text | Sound, ping | Place |
| --- | --- | --- | --- | --- | --- | --- |
| Construction complete | `ConstructionComplete` | Event | 2 s | "{unit} complete." | yes, yes | the building |
| Unit ready | `ProductionComplete` | Event | 2 s | "{unit} ready." | yes, no | the new unit |
| Low power | `SupplyShort(power)` | State | 60 s, 10 s | "Low power: everything builds at half speed and turrets are off. Build a Power Station (P)." | yes, yes | an offline turret, else the Command Centre |
| Power restored | `SupplyRestored(power)` | Event | 30 s | "Power restored." | no, no | the Command Centre |
| Not enough credits | `InsufficientFunds(credits)` | State | 90 s, 15 s | "Not enough credits: jobs are on hold. Your Ore Haulers bring more." | yes, no | the Command Centre |
| Base under attack | `OwnUnitDamaged`, units Command Centre, Power Station, Guard Turret | Event | 20 s | "Base under attack. Click to see it." | yes, yes | the damaged building |
| Ore Hauler under attack | `OwnUnitDamaged`, units Ore Hauler | Event | 15 s | "Ore Hauler under attack. Click to see it." | yes, yes | the hauler |
| Ore Hauler destroyed | `OwnUnitDestroyed`, units Ore Hauler | Event | 5 s | "Ore Hauler destroyed. Train a new one at the Command Centre (O)." | yes, yes | the place of the loss |
| Ore Hauler idle | `GathererWaiting(5.0)` | State | 60 s, 5 s | "Ore Hauler idle. Select it and right-click ore." | yes, yes | the hauler |
| Lost connection | `SlotDisconnected` | State | 1 s, 0 s | "{slot} lost connection." | yes, no | none |
| Back | `SlotReconnected` | Event | 1 s | "{slot} is back." | no, no | none |

### 7.10 Reading a fight

- Role line: each unit tooltip and build entry shows the mod `role` text [player 6]:
    - Rifleman: "Strong against infantry. Weak against tanks and buildings."
    - Launcher: "Strong against tanks and Ore Haulers. Weak against riflemen."
    - Engineer: "Repairs vehicles and buildings. Has no weapon."
    - Ore Hauler: "Brings ore to the Command Centre. Has no weapon."
    - Tank: "Strong against riflemen, vehicles and buildings. Weak against launchers."
    - Guard Turret: "Strong against tanks and infantry. Needs power. Cannot move."
    - Power Station: "Makes 100 power."
    - Command Centre: "Trains units and builds buildings. Makes 3 credits each second. Lose it and you lose."
- A reduced hit shows a small hit effect: the client computes the table ratio from the weapon of `Fired`, the armour
  class of the target and the damage table of the package; a ratio below 1/2 picks `hit_reduced` (8.3) [player 6].
- The tooltip of a build entry shows the build time at the current power: "Build time 12 s", or under a shortage
  "Build time 12 s (24 s now: low power)" [player 7].
- Each delivery shows "+N" at the receiver with the delivery sound (8.4). N comes from the hauler's load that the
  replica shows before and after the unload (owner-visible); the export of the receiver id is NOT TRACED. The free
  income shows no float; it shows only in the credits counter.
---
## Replace: `## 8. Presentation`
## 8. Presentation

This block is the whole section 8, with its subsections 8.1 to 8.5. From RE, with the gate-1 changes.

### 8.1 Models (decision P1, option b)

- `models` in the package assets (`AssetsDef`, `crates/turts_mod_api/src/assets.rs:5-10`, has images, fonts, sounds and
  cursors only), and an optional `model` on `VisualDef` (`crates/turts_mod_api/src/schema.rs:501-504`):
  `{asset, team_material, turret_node, muzzle_node}`. `shape` stays required: the fallback while a model loads, and the
  content cut line.
- Status: S:247 keeps the model formats and features Open. The checks below are the MP1 validation profile, not the
  final engine format; a later milestone can widen it with a version-table row [rigour 12].
- The validator uses `gltf = "=1.4.1"`: `bevy_gltf` 0.20 asks for `1.4.0` (`bevy_gltf-0.20.0/Cargo.toml:122-123`) and
  the workspace lock has `1.4.1` (`Cargo.lock:3929-3930`), so the validator parses what the client parses.
- The browser loads from the package bytes in memory (Bevy 0.20 `MemoryAssetReader`,
  `bevy_asset-0.20.0/src/io/memory.rs:290`), with no fetch.
- Team colour: the client replaces the team material with the slot material at spawn.
- The client builds one flat template for each model and spawns its own entities; it does not spawn scenes.
- The MC packages keep their shapes, so the MC pixel rows do not change.

The asset brief as pack-time checks [mod M9]. Each refusal is a `ModError` with the package, the file and the field;
the texts follow the style of the existing asset checks (`crates/turts_mod_runtime/src/content_mc_validation.rs:175`,
`:212`). The numbers in brackets are the caps of decision D-M (CEO).

| Check | Error text (example) |
| --- | --- |
| The file is GLB version 2 | `assets/models/tank.glb: not a GLB version 2 file` |
| No `uri` in buffers or images | `assets/models/tank.glb: buffers[0] has an external uri; a model must embed all its data` |
| No `extensionsRequired` | `assets/models/tank.glb: extensionsRequired lists KHR_draco_mesh_compression; an MP1 model accepts no required extension` |
| No skins, animations, cameras or lights | `assets/models/tank.glb: has 1 skin; an MP1 model accepts no skins` (the same form for the others) |
| Embedded images are PNG, within the existing image caps (2048 px, `content_mc_validation.rs:17`) | `assets/models/tank.glb: image 0 is image/jpeg; an MP1 model accepts embedded PNG only` |
| At most 2 materials | `assets/models/tank.glb: has 3 materials; maximum is 2 (body and the team material)` |
| Triangles, from the index accessor counts [8,000 for one model] | `assets/models/tank.glb: 9,412 triangles; maximum is 8,000 for one model` |
| Bytes [4 MiB for one model; 16 MiB for all models] | `assets/models/tank.glb: 4,500,000 bytes; maximum is 4,194,304 bytes` and `assets/assets.ron: models: total 17,000,000 bytes exceed the 16,777,216 byte package limit` |
| The team material exists | `visuals/tank.ron: model.team_material: material `team` is not in assets/models/tank.glb; materials are: body` |
| Each named node exists (`turret_node`, `muzzle_node`, when given; the `frontier` names are `turret` and `muzzle`) | `visuals/tank.ron: model.turret_node: node `turet` is not in assets/models/tank.glb; did you mean `turret`?` |
| Scale: the XZ radius from the POSITION bounds is 0.8 to 1.2 times the footprint radius | `assets/models/tank.glb: radius 2.31 is outside 0.72 to 1.08 (0.8 to 1.2 x the footprint radius 0.9 of frontier:tank)` |
| Pivot: the lowest point is at ground level, min y from -0.05 to 0.05 | `assets/models/tank.glb: lowest point y = -0.40; the pivot must be at ground level` |

Advice, not checked (the validator cannot measure it): forward is glTF +Z (Blender -Y) and +Y is up; the team material
covers at least 15 % of the top view; body colours are neutral greys and browns, with no saturated green and none of
the slot hues; textures 512 px for units and 1024 px for the Command Centre; triangles about 1,500 for infantry, 3,000
for vehicles and 8,000 for buildings (RE part 6).

### 8.2 Heading and aim

The client derives the yaw of a unit from its movement, with a turn rate; no wire change. A turret node aims from the
`Fired` event. Walking animation needs skinning and stays at M13; infantry gets a small bob. A site model rises from
the ground with its progress.

### 8.3 Effects

- Pooled unlit meshes for the muzzle flash, the full hit (`hit`) and the reduced hit (`hit_reduced`, smaller and
  dimmer, 7.10), a line for a tracer, an expanding burst for a death, smoke below 50 % and fire below 25 % health on a
  building, dust on a site, an arc for repair, an offline mark on an unpowered turret, and the "+N" float.
- One pool of 256 instances. The pool never grows: when it is full, a new effect takes the oldest instance, and no
  effect spawns an entity of its own [perf L1]. No compute shader, so WebGL2 stays possible.
- Effects are never replayed: the client drops an effect whose tick is older than the presentation tick minus K
  (K = 4 ticks, Proposed), so a resync after a lost delta gives no burst [net 7].
- A mod names each effect by content id; the server never reads them. Effect colours avoid the slot hues of the pixel
  drivers.

### 8.4 Sound

Effect sounds need a small engine part, as section 16 says: an effect definition names a sound id; the engine plays it
at the place of the effect; the gain falls with the distance from the camera look-at point (to 0 at the mod
`audible_range`); the effects category plays at most the mod cap at once (`effects_voices`, default 8), and a sound
beyond the cap is dropped. X14 takes this engine part [rigour 12]. The audio engine and its categories exist (D69,
Proposed; the alert cap of two at `crates/turts_client/src/audio.rs:795`). The sound set is mod content: the minimal
set of decision P5 (fire, hit, death, construction complete, low power, unit ready, error), plus the reduced hit and
the delivery sound with the "+N" [player answer 5].

### 8.5 Measurement first

- The first presentation task measures the frame time at 400 units with primitives and with models, in the browser on
  the real GPU (RE part 5: 10.7 to 43 ms is an estimate, not a measurement).
- The debug state is built only when the harness asks for it, at most 4 times a second, and for the last presented
  frame. Today the client rebuilds it in each frame (`crates/turts_client/src/lib.rs:147`,
  `crates/turts_client/src/debug.rs:441`); at 400 units that costs 4.4 to 5.1 ms native (gate1-performance.md:15).
  VIS3 records the frame time with the debug state off and on [perf H1].
- The play-test floor (Proposed; decision D-N): frame time p95 at or below 50 ms and p50 at or below 33 ms at 400
  units on the play-test machine. PT3 does not start below it [perf L2].
---
## Replace row: `GOAL1`
| GOAL1 | In the first presented frame after the press of "Click to play", in each window: the own Command Centre is inside the viewport; it is selected (the selection panel shows "Command Centre"); the goal line says "Destroy the enemy Command Centre"; the hint line shows the H1 text of the Player path. No alert line and no alert sound come before the first input of the player. With no input, the Ore Hauler drives to the ore and the first "+300" shows before 40 s. A pixels-only driver that does only what the hint lines say presses P and left-clicks inside the build-area ring: the hint line shows the H2 text within 2 presented frames of the frame with the pending site, and a Power Station site exists before 60 s from the press (`turts_debug.hints.H1.site_ms` at most 60,000) | Plants, each red: (a) `start_selection` ignored: no unit selected; (b) the done condition of H1 wired to `Complete` instead of `Pending` or `Site`: H1 stays about 24 s after the click; (c) a state alert that counts the state of the first replica as a start: "Low power" sounds at tick 0; (d) the hint source evaluated only at the start: H2 never shows; (e) the map placement without its start order: no "+300" before 40 s | pixels, two windows |
---
## New row, after `GOAL1`
| STO1 | A map placement with a start order (`order: Gather`, decision D-F) gathers from tick 0 with no client input: the first delivery comes within 1 s of the first-trip time of the model (29.7 s, `pacing.py new`); the command log records the order at tick 0 with the source `Engine`; a replay of the log gives the same delivery tick; the order has no seq, costs no token and gives no `OrderResult` | (a) the field parsed and not used: no delivery in 60 s; (b) the start order sent as a player envelope: it shows in the admission evidence with a seq | headless |
---
## Replace row: `GHO1`
| GHO1 | P, or a click on the build entry, starts placement; the server order log has no new line. The ghost follows the pointer, and the build-area ring of the own Command Centre shows (radius 12.5). Over a valid point: a solid ghost ring in the slot colour. At one fixed point for each cause on a test map: stripes, the `forbidden` cursor, and the exact text of 7.2 at the pointer: "Outside the map", "Too far from your base", "Something is in the way" (over a unit, and over a building), "Too close to a building" (the gap only: the gap rings of the ghost and of the building show in vermilion), "Too close to ore" (the clearance ring shows). Right click and Esc end placement: no new order line, and the credits do not change. Left click places: a pending site shows in the same frame; after the server result a world progress bar grows on the site until it is complete. Shift and left click places and keeps the ghost. Mismatch case (a scenario unit walks onto the point after the client verdict): the pending site goes, "Something is in the way" shows at the site point, one error sound plays, the credits do not change, and the hint does not advance | (a) the gap folded into the overlap check (the rev-3 text): the gap-only point shows "Something is in the way"; (b) the old ore text "Cannot build on ore"; (c) the world style `build_area_ring` or `gap_ring` not drawn: no ring pixels at radius 12.5, or none round the building at the gap-only point; (d) the smart path runs before the placement mode: a right click in placement sends a move (an order line); (e) the client reads a fixed gap of 0.5, not `placement.gap`: the ghost is valid at the gap-only point and the server refuses | pixels |
---
## Replace row: `HUD5`
| HUD5 | Forced states in a scenario, each read as exact text: "Credits 1200"; "Power 40 used / 100 made"; under a shortage "Low power: 130 used / 100 made" with the mark; queue rows "On hold: no credits" with a striped bar, "Slowed: low power", "Waiting to start", "Waiting: exit blocked"; a disabled entry "Queue is full (5/5)". A starved job under the free income (it pays in some ticks and not in others) shows "On hold: no credits" in each of 10 frames sampled over 1 s | (a) the `Supply` fields `Used` and `Made` swapped: "Power 100 used / 40 made"; (b) the row state without its 1 s hold: the words are absent in some of the 10 frames; (c) the job stage `WaitingForFunds` mapped to the low-power words | pixels |
---
## Replace row: `HUD6`
| HUD6 | The tooltip of each build entry, exact: the name; the role line of 7.10; "Cost N credits, paid while it builds"; "Build time T s", or under a forced shortage "Build time T s (2T s now: low power)"; the power change ("+100 power" or "Uses 30 power"), and "This causes low power" when it applies; the key ("[P]"). A disabled entry names its cause ("Queue is full (5/5)"). The info panel of one selected unit shows its armour ("Armour: Vehicle"); for the Ore Hauler also "Load 150/300 ore" and the activity ("Harvesting", "Returning", "Unloading", "Waiting: field empty", "Idle") | (a) the tooltip time from the base time only: under forced low power the Power Station tooltip says 12 s and the job takes 24 s; (b) `Role` not bound in the `frontier` tooltip: no role line; (c) the `Reason` field empty for `queue_full`: a disabled entry with no cause | pixels |
---
## Replace row: `ALR2`
| ALR2 | The rules of 7.9 give their lines with the exact texts, sounds and pings: event alerts (Construction complete, Unit ready, Base under attack, Ore Hauler under attack, Ore Hauler destroyed, Power restored, the return of a slot) and state alerts (Low power, Not enough credits, Ore Hauler idle, a lost connection). From tick 0 to the first input of the player: no alert line and no alert sound. A state alert: one line and one sound when the state starts; no sound while it holds; it ends only after `clear_after` s clear; a new start inside `cooldown` shows the line with no sound. Forced case A (headless and pixels): 0 credits, a tank job, the free income on, and one hauler that delivers every 35 s, for 300 s: at most one "Not enough credits" sound in each 90 s window (`turts_debug.audio`). Forced case B: a hit on an Ore Hauler gives "Ore Hauler under attack" and not "Base under attack". A click on a line moves the camera look-at to within 2 world units of the place of the alert; a click on "Ore Hauler idle" also selects the hauler | (a) the rev-3 rule (one sound at each start of the state) in place of 7.9: case A gives more than one sound in a window (run S8 of section 5: 89 starts in 300 s); (b) the state of the first replica counted as a start: "Low power" sounds at tick 0; (c) the `units` filter not used: a hauler hit gives "Base under attack"; (d) the alert row a text and not a button: a click does nothing (today no alert row takes a click) | headless and pixels |
---
## Replace row: `SMT1`
| SMT1 | MP1 smart rules and cursors. Command Centre selected: a right click on the ground sets the rally (a flag and a line show; no unit moves; no notice); a right click on ore sets a gather rally. Engineer selected: over a damaged own building the cursor is `repair`, and a right click repairs it (the health bar rises); over an undamaged own building the cursor is the default cursor. Ore Hauler selected: over ore the cursor is `gather`. In placement the cursor is `place` or `forbidden`. The oracle reads `turts_debug.cursor` for each context | (a) the smart table puts Move before SetRally: a right click on the ground with the Command Centre selected gives the notice "cannot move"; (b) the repair context tests "own" and not "own and damaged": the `repair` cursor over an undamaged building; (c) one cursor for the placement contexts: `place` over an invalid point | pixels |
---
## Replace row: `AUD2`
| AUD2 | The sound set plays from the mod package: fire, hit, reduced hit, death, construction complete, low power, unit ready, error, and the delivery sound with each "+N" (one for each delivery). A weapon sound is positional: a shot far from the camera look-at point has a lower gain than a near shot, and a shot beyond `audible_range` has gain 0. With 40 units that fire at once, no more effect sounds play at once than `effects_voices` (`turts_debug.audio`) | (a) the effect sound played at the listener: equal gain near and far; (b) the cap counted for each sound id: with two weapon sound ids, twice the cap plays; (c) an effect that names a sound the package lacks: the packer refuses with `effects/frontier.ron: rifle_fire.sound: unknown sound `frontier:rifle_shot`; did you mean `frontier:rifle_shots`?`; (d) the "+N" float with no sound | pixels |
---
## Replace row: `KEY1`
| KEY1 | Each build entry has its own key, shown in its tooltip and its build-menu row (the table of 7.4); no two MP1 default bindings are equal (the check is global; there is no action scope); no default key is in `RESERVED_BROWSER_KEYS` (`crates/turts_mod_api/src/input.rs:51-68`); in a pixels run each key starts the placement or the job of its entry | (a) today's build menu: every row shows the key of the producer's Produce order (`crates/turts_client/src/widgets/sources.rs:391-393`); (b) a package with two actions on KeyP: the packer refuses with `input/input.ron: actions: KeyP is the default of build_power_station and pause; each default key must be unique`; (c) a default on Ctrl and KeyT: refused as a browser key | headless and pixels |
---
## Replace row: `MCX1`
| MCX1 | The MC items of 7.7, one check each: a Button with `font` and `size` renders in that font and size; an action with no key shows "Unbound" in the menu, the tooltip and the binding rows; the Controls menu shows its groups and Up, Down, Enter, Delete and Esc work; the alert list row pitch equals the row height plus the gap within 1 px at UI scale 1.0 and 2.0, and no pixel of a row shows above the list top; a unit with no health shows no "x1 - HP"; a partly refused group order shows "3 of 8 units: Cannot reach that place" with one error sound, and a click on the line selects the failed units | Each MC defect put back; each one exists in the MC build 8d6902c (PT1 person run 1 and the O15 list), so each red can happen | pixels |
---
## Replace row: `NOGPU1`
| NOGPU1 | With no graphics adapter (Chrome started with WebGPU off), the page shows the plain text of 7.7, with how to turn WebGPU on, and the console shows no panic | The MC build 8d6902c: a panic with a console message only (PT1 person run 1) | pixels |
---
## Replace row: `PT3`
| PT3 | The two-person play test of section 13, on a play-test machine that passes the VIS3 floor. The clock starts when the player presses "Click to play" in that window; the 60 s rule ends at the first client update with an own Power Station site (7.1), not at a pending site. A person signs off | Red when a tester cannot find an object, an action or a reason (the tester says so aloud, or the observer sees no progress for 60 s), or when no Power Station site exists 60.0 s after the press. The timing instrument can go red: GOAL1 plant (b) moves `H1.done_ms` late and plant (d) leaves it absent, and the PT3 timing script reports red on both | people |
---
## Replace row: `PRD1`
| PRD1 | The Command Centre runs one site from its structures queue and one job from its units queue at the same time (decision D-E). A second placed order waits ("Waiting to start") while one site is open (open-site cap 1, mod data) and opens when that site completes. A sixth placed order is refused with `queue_full`. A new unit appears at an exit point of the producer. With every exit point blocked, the complete job waits ("Waiting: exit blocked"), charges nothing more, and the unit appears when an exit point is free | (a) the producer advances only its first queue with an active job: the unit job stops while a site progresses; (b) the open-site cap not counted: two sites progress at once; (c) the exit check ignores static footprints: a unit appears inside the footprint of the Command Centre | headless |
---
## New row, after `PRD1`
| INC1 | The Command Centre adds 3 credits each second (mod data `income`, decision D-L) only while it is complete and has health above 0; each period is one ledger event, so `turts_audit` balances; a slot with 0 credits and no Ore Hauler that queues a hauler gets it in at most 170 s (section 5: 166 s); a destroyed Command Centre stops the income in the same tick | (a) the income added outside a ledger event: the audit is red; (b) income from a site or from a destroyed Command Centre; (c) a float rate: the total after 600 s is not 1,800 | headless |
---
## Replace row: `FX1`
| FX1 | A `Fired{unit, target}` event, with the weapon as an index (net 7, perf M1), reaches each client that may see the shooter or the target (both clients in MP1). In window B an enemy turret turns to its target and its muzzle flash shows; a death shows its effect; a reduced hit (table ratio below 1/2, for example a bullet on a tank) shows `hit_reduced` and a full hit shows `hit` (`turts_debug.effects.recent`). With 400 riflemen that fire, the effect pool stays at 256 and the effect entity count does not grow. After a T8 resync, no effect older than the presentation tick minus K plays | (a) today's event path: the event is staged for the shooter's slot only (`crates/turts_sim/src/economy/combat.rs:393-395`), so window B sees no enemy fire; (b) the pool grows past 256, or one entity spawns for each shot; (c) `hit_reduced` never chosen; (d) the kept deltas of a resync replay their effects: a burst after the resync | headless and pixels |
---
## Replace row: `MDL1`
| MDL1 | The `frontier` package with models loads in the browser from the package bytes in memory: the browser network log has no request for a `.glb` path. Each check of 8.1 refuses its plant at pack time with its error text. The `shape` shows until the model is ready: with a test hook that delays readiness by 500 ms, the pixel finder sees the shape at the unit's screen point in each frame of the delay. The team material shows the slot colour | (a) a URL fetch of the model; (b) for each check of 8.1, a plant that packs with exit 0; (c) the shape removed at spawn before the model is ready: frames with no unit pixels; (d) the team material not replaced: the body colour shows where the slot colour must be | headless and pixels |
---
## Replace row: `VIS3`
| VIS3 | Frame time p50 and p95 at 400 units, with primitives and with models, with the debug state off and on (8.5), in `docs/measurements/mp1.md` (recorded, not judged). The play-test floor (Proposed, decision D-N): p95 at or below 50 ms and p50 at or below 33 ms on the play-test machine; PT3 does not start below it | The recorded numbers have no red. The floor check is red on a build with the test hook `frame_stall_ms=60` (a busy wait of 60 ms in each frame), which shows that the check can fail | pixels |
---
## Replace: `## 13. Play test`
## 13. Play test

- Who: the `player` agent plays first, through the harness, with the goals text only. Then two people play in two
  windows or on two machines; one of them is not the CEO, so one tester is blind (decision P7). One person signs off.
- Machine: it passes the VIS3 floor (8.5).
- The observer has the Player path; the testers do not. The observer answers no question about the game, reads nothing
  to the testers, and writes the time, the step and the words each time a tester says that they do not know what to do.
- Goals text, match 1. Both testers get this text only [player 8]:

  "Play one match against the other person. Win: destroy the enemy Command Centre. Use only the mouse and the keyboard.
  Say aloud what you try to do, and say when you do not know what to do next. The observer does not answer questions
  about the game. Stop at the result screen, or after 40 minutes."

  "Command Centre" is the word of the goal line on the screen (7.8). The text does not name power, ore, units or
  turrets: the hint lines must teach them.
- Match 2, after match 1 [player 11]. To A: "Play a second match. When you want to, open the menu (Esc) and choose
  Surrender." To B: "Play a second match as usual."
- Match 3 [player 11; decision D-H]. To B: "Play a third match. After about two minutes, click the reload button of
  the browser (the round arrow) in this same tab, and play on. If the pointer stays inside the game, press Esc first.
  Later, close this tab and do not come back." To A: "Play a third match as usual." The browser keeps the player id in
  sessionStorage (`crates/turts_client_web/src/lib.rs:242`, `:266`): a reload in the same tab keeps it, and a closed tab
  loses it (gate1-network.md:27).
- Each match is a new server process (decision D-C). Between matches the testers reload the page; the "Leave" screen
  says so (7.6).
- Where a first-time player most likely stops, and the design answer:
    1. The player does not find building: the Command Centre is selected at the start, the goal line shows, and hint H1
       names the key and the ring.
    2. A silent stall from no credits or low power: the Ore Hauler gathers from tick 0; the numbers of section 5 give no
       hold above 4 s in the ordinary scripts; the row states; the alert rule of 7.9 (one sound for each state start,
       never at tick 0); the situation hints X1 to X3.
    3. A red ghost with no cause: the reason names the real cause, a ring shows the gap, and right click and Esc leave.
    4. The player does not see who beats whom: the role lines and the reduced-hit effect (7.10).
- Record for each tester: the hint times of `turts_debug.hints`, each spoken stop with its time and step, the result
  screen, and the answer to "Would you play a second match?".
---
## Replace: `## 16. Cover that the yardstick does not ask for (from the coverage work)`
## 16. Cover that the yardstick does not ask for (from the coverage work)

The yardstick marks an objectives panel as COMMON, not CORE, so the completeness gate never asks for a visible goal
or a first action. That is the cause class of the CEO's failed MC play. MP1 covers it with its own rows:

- GOAL1, STO1 and PT3: the goal line, the selected Command Centre, the Ore Hauler that gathers from tick 0, the hint
  list of 7.8 with a defined "done", and the 60 s rule of 7.1.
- ALR2: the two alert kinds of 7.9. A state alert (low power, no credits, an idle Ore Hauler, a lost connection)
  sounds when the state starts, never at tick 0, never while it holds, and not again inside the rule `cooldown` when
  the state clears and comes back. An event alert (unit ready, construction complete, under attack) sounds at most once
  in each `cooldown`. MC row ALR1 stays green while a state fires again at each cooldown, which is how "it beeps non
  stop" passed MC.
- Refusals (OR22): a refused order names a cause that the player can act on, near the pointer. GHO1 covers placement
  with one reason for each real cause and the rings that show it; MCX1 covers the grouped notice; the same rule holds
  for every refused order.
- The floor (INC1): the free income of the Command Centre, so the loss of the only Ore Hauler is not a silent loss.
- TGT2: an idle unit that takes damage targets its attacker inside its leash; a holding unit fires back only inside
  its weapon range and never moves (decision D-G). Without it, a unit that is hit from beyond its acquire range does not
  react. With the repaired numbers no `frontier` weapon outranges an acquire range, but a later mod needs the rule, and
  the hold case needs it in MP1. X2 takes this.
- Sound (AUD2): effect sounds need a small engine part (an effect names a sound; the engine plays it at the place of
  the effect, with a cap); 8.4 says the same. X14 takes the engine part.
- Reading a fight (HUD6, FX1): role lines, the reduced-hit effect, and the build time at the current power.
- The play-test floor (VIS3): a frame time that a person can play with, before PT3 starts.
- Lobby (decision P2 a): teams and team choice stay later; the starting credits setting is not in MP1.
- 57 MC rows are `done` and rest on REG2 alone. MC is not closed: its person play failed and Tasks 23 and 24 are open.
  MP1 gate 2 checks again that each `done` path is green on the tree that MP1 starts from.
---
## Replace: `## Player path`
## Player path

This is the oracle list of PT3. The testers never see it: they get the goals texts of section 13. The source is UX
part 1, corrected by PT1 person run 1 (2026-11-11: no goal, no first action, an alert sound that repeats) and by
gate 1. It assumes decisions P2 (a), P5, P8 (b), D-A to D-H and D-K, D-L. Setup: two browser windows A and B, or two
machines; mouse and keyboard only. The brackets name the rows of section 12 that check the step.

Lines L1 to L5 are the lobby. They come first, but their time depends on the second person, so they have no clock.
The 60 s rule of 7.1 starts when a tester presses "Click to play": a person with no knowledge of the game must place
the Power Station site from the screen alone before 60.0 s.

- L1. A: the load screen shows each phase and a progress bar (MC SH1).
- L2. A: the lobby shows the goal line "Destroy the enemy Command Centre", the rows "Slot 1: You — Not ready" and
  "Slot 2: Open — waiting for a player", and the status line "Waiting for a second player." (LOB1).
- L3. B joins. Both windows show the row "Player 2 — Not ready" for slot 2, and the status "Press Ready to start." Each
  slot has a different default colour (LOB1, LOB2).
- L4. A picks the colour of B: the swatch shows a cross, and its tooltip says "Player 2 has this colour". A then picks
  a free colour, and both windows show it (LOB2).
- L5. Both press Ready. The rows show "Ready", then "Match starts in 3", "2", "1". A change by a player clears the
  ready state of that player and shows "Countdown stopped: Player 2 is not ready." At the end of the countdown both
  windows show "Click to play" (LOB1).

The hint list (mod data of `frontier`; the rules are in 7.8). One line shows at a time.

| Id | Kind | Text on the screen | Starts when | Done when |
| --- | --- | --- | --- | --- |
| H1 | step | "Build a Power Station: press P, then left-click inside the ring around your Command Centre." | the first client update after "Click to play" | an own Power Station is pending (not refused), a site, or complete |
| H2 | step | "While it builds, train a Rifleman: press I." | H1 is done | an own Rifleman job is in the units queue, or an own Rifleman exists |
| H3 | step | "Select your Command Centre, then right-click the ground: new units walk there." | H2 is done | the own Command Centre has a rally point |
| H4 | step | "Train a second Ore Hauler for more credits: press O." | H3 is done | an own Ore Hauler job is in the queue, or 2 own Ore Haulers exist |
| H5 | step | "Build a Guard Turret near your ore field: press T." | H4 is done, and an own Power Station is complete | an own Guard Turret is pending (not refused), a site, or complete |
| H6 | step | "Drag a box round your units, press A, then left-click the ground: they attack on the way." | H5 is done, and 4 own Riflemen, Launchers or Tanks exist | the server accepts an own attack order on a point |
| H7 | step | "Find the enemy Command Centre and destroy it. Look for it on the minimap." | H6 is done | it has shown for 30 s |
| F | fallback | "Train more units while you wait: Rifleman (I), Launcher (V), Tank (K)." | a step is not done, and no step can start | a step can start |
| X1 | situation | "Your Ore Hauler is gone. Select your Command Centre and press O. Cancel other unit jobs first (right-click them), so it starts at once." | no own Ore Hauler and no Ore Hauler job | an Ore Hauler job is in the queue, or an Ore Hauler exists |
| X2 | situation | "Low power: build another Power Station (P)." | H1 is done, the base uses more power than it makes, and no own Power Station is pending or a site | the base uses no more power than it makes, or a Power Station is pending or a site |
| X3 | situation | "Jobs are on hold: you need credits. Wait for your Ore Haulers, or cancel a job (right-click it)." | a job has been "On hold: no credits" for 10 s | no job is on hold |

The steps:

1. In the first frame after "Click to play": the camera shows the own Command Centre, and it is selected. The top line
   says "Destroy the enemy Command Centre". The HUD shows "Credits 1200" and "Low power: 40 used / 0 made", because the
   Command Centre uses power and nothing makes it. No alert line shows and nothing sounds. The Ore Hauler drives to the
   ore by itself (GOAL1, STO1, HUD5, ALR2).
2. The hint line shows H1. The tester presses P or clicks the entry. The ghost follows the pointer, and the ring of the
   build area shows round the Command Centre. The tester left-clicks inside the ring where the ghost ring is solid
   (GOAL1, GHO1).
3. The reward shows at once: the pending site shows in the same frame, and the hint changes to H2 in the same frame.
   Then the site shows a progress bar that grows for 24 s (the tooltip said "24 s now: low power"), and the credits
   count down. At about 30 s the Ore Hauler unloads: "+300" rises at the Command Centre with the delivery sound, and
   the credits go up by 300. When the station is complete, the alert "Power Station complete." shows with its sound,
   the power text shows "Power 40 used / 100 made", and "Low power" goes (GOAL1, GHO1, ALR2, PWR1, HUD6).
4. The tester presses I (H2): the Rifleman job runs while the station builds. Then H3: the tester right-clicks the
   ground with the Command Centre selected. A flag shows with a line from the Command Centre; no unit moves. The new
   Rifleman walks to the flag, and the alert "Rifleman ready." shows (SMT1, RLY1, ALR2, PRD1).
5. The tester hovers the Guard Turret entry. The tooltip shows the name, "Strong against tanks and infantry. Needs
   power. Cannot move.", "Cost 600 credits, paid while it builds", the build time, "Uses 30 power" and "[T]" (HUD6).
6. The tester presses T. Over the Ore Hauler the ghost shows stripes and "Something is in the way"; close to the
   Command Centre, "Too close to a building", with the two gap rings in vermilion; close to an ore node, "Too close to
   ore", with its clearance ring; outside the ring, "Too far from your base". A right click ends placement: the ghost
   goes, and the credits do not change (GHO1).
7. The tester presses O (H4). The tester selects the Ore Hauler: the info panel shows "Harvesting" and the load goes
   up. A right click on another ore node shows the gather cursor, then the gather marker (SMT1, HUD6).
8. The tester places a Guard Turret near the ore field (H5). With the Command Centre selected, the tester presses I
   three times. The entry shows the badge "3". The first row of the units queue shows its progress and the seconds
   left. The site in the structures queue progresses at the same time (HUD5, PRD1).
9. The tester right-clicks the active job in the queue. The job goes, "+N" rises at the Command Centre, and the
   credits go up by N, the amount that the job paid. A cancel of a waiting job gives back nothing, because it paid
   nothing (PAY2, HUD5).
10. The tester builds turrets until the base uses more power than it makes. The power text shows "Low power: 130 used
    / 100 made" with a mark. The alert "Low power: ..." shows once, with one sound. The queue rows say "Slowed: low
    power". Each turret shows an offline mark. Hint X2 shows. A second Power Station clears all of these, and "Power
    restored." shows with no sound (PWR1, HUD5, ALR2, GOAL1).
11. The credits reach 0 during a job. The row says "On hold: no credits", with a striped bar, and the alert "Not enough
    credits: ..." shows once, with one sound. The job crawls on the free income and runs at full speed after the next
    delivery. When the state comes back after a delivery, nothing sounds again inside 90 s (PAY1, HUD5, ALR2, INC1).
12. B drags a box round its units, presses A and left-clicks near the base of A (H6). The attack cursor shows, then a
    cross marker at once. The units move, and they fire by themselves at enemies in range (TGT1, MC ORD4).
13. In the fight: muzzle flashes, tracers, full and reduced hits, and health bars on damaged units. A weapon sound near
    the camera is louder than a far one. A gets "Base under attack. Click to see it." with a minimap ping; a click on
    the line moves the camera to the base. When B's launchers hit A's Ore Hauler, A gets "Ore Hauler under attack.
    Click to see it." (FX1, AUD2, ALR2).
14. An enemy turret turns to its target and fires (FX1).
15. Units die with a death effect, and they leave the selection and their groups. If the Ore Hauler of A dies, A gets
    "Ore Hauler destroyed. Train a new one at the Command Centre (O)." and hint X1. A partly refused order shows "3 of 8
    units: Cannot reach that place" with one error sound; a click on the line selects the failed units (FX1, MCX1,
    ALR2, GOAL1).
16. The Engineer of A right-clicks a damaged own building. The repair cursor and the repair effect show, the health
    bar rises, and the credits go down. At 0 credits the repair stops (SMT1, REP1).
17. A Command Centre is destroyed. Within 1 s, both windows show "Victory" or "Defeat". After 3 s, the result screen
    shows the reason ("Victory. You destroyed the enemy Command Centre." or "Defeat. Your Command Centre was
    destroyed."), the match time, both slots, the outcomes and the statistics. "Leave" opens the shell screen "The
    match is over. This server starts a new match. Reload this page to join it. If the page shows an error, wait a few
    seconds and reload again." (MAT1, RES1).
18. In a second match (instruction of section 13): A opens the menu and selects "Surrender". The text "Surrender and
    end the match? You will be defeated." shows. Cancel changes nothing. Confirm defeats A; window A shows "Defeat. You
    surrendered." and window B shows "Victory. Player 1 surrendered." (SUR1).
19. In a third match (instruction of section 13; decision D-H): B clicks the reload button of the browser in the same
    tab. When the server session of B ends, A shows the alert "Player 2 lost connection." and the banner "Waiting 1:59
    for Player 2 to return.", which counts down in match time. B reloads within 2:00, rejoins its slot, and A shows
    "Player 2 is back."; the match goes on. Later B closes its tab and does not come back: after 2:00, B is defeated,
    and A shows "Victory. Player 2 left the match and did not return within 2:00." (MAT1, ALR2, MC SLOT1).
---
## Not done

The same text is in `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mp1-pacing/repair-ux.md`; the script outputs are in `results.txt` beside it.

Scope of my blocks: the blocks for `## 7. Player interface` and `## 8. Presentation` replace the whole section with
all its subsections, up to the next `## ` heading. Section 7 now has 7.0 to 7.10.

Gate findings in my area that I did not close, or closed only in part:

- [player 10] Players meet before minute 11: closed in part. The build area (Command Centre only) leaves the side and
  centre fields open, and the defended ore runs out at 9.6 to 14.5 min. The macro model still gives no end for two
  exactly equal careful players (it has no tactics). The headless match must measure it.
- [player H3] match length: the model gives 11 to 20 min for unequal pairs. It does not separate the gate-1 and the
  repaired numbers strongly at match level; the clear gains are in the first minutes, the floor and the defence ratio.
- [mod M5] `counts: Completed | Queued` and the shortage effect `BlockStart`: section 6.2, not mine. I used the trigger
  names `SupplyShort(res)`, `SupplyRestored(res)` and `InsufficientFunds(res)`.
- [perf M2] the pruning of `health_trace`: client replica, not in my sections.
- [rigour 4] CB05 in the yardstick table, and [rigour 7] exit item 1: not my sections. VIS3 now has a floor check with a
  red, so exit item 1 can keep "every row".
- [rigour 9] the Changes column of section 2: not mine. My blocks touch it: P1 (8.1, MDL1, VIS3), P4 (7.3, HUD5, HUD6,
  steps 3, 9, 11), P5 (8.4, AUD2), P8 (7.6, step 19).
- Section 4 needs scope items for the new engine parts named here: the hint list and its conditions (7.8), the alert
  kinds and `FocusAlert` (7.9), the free income (INC1), the start order of a placement (STO1), the role field and the
  `{unit}` and `{slot}` placeholders. Section 6.10 needs their rows (7.5 is the input).
- 7.3 reads decision D-E as "the left click carries the point; the order waits in the structures queue only for an open
  site". If 6.1 keeps an order with no point (place it later), 7.3 needs one more entry state, "Ready to place".
- The result words need a reason code on the outcome record (MAT1, 6.7) and a connection record for "is back" (6.8).
- The scripts are in the scratchpad; the plan copies them to `docs/research/mp1/probes/pacing/`.
- NOT TRACED: whether two Ore Haulers on one node slow each other; the MC camera and group default keys (KEY1 checks
  them); whether the client captures F5 (section 13 uses the reload button instead); the browser disconnect lag; the
  export of the receiver id for "+N".
---
## Decisions for the CEO

- D-L. The floor after the loss of the only Ore Hauler (player 5). Options: (a) a free income of 3 credits each second
  from a complete Command Centre: a new engine capability `income` for all mods, one ledger event for each period (row
  INC1); a hauler from 0 credits in 166 s; (b) a cheaper Ore Hauler only (500): no floor at 0 credits (run L2: never);
  (c) two Ore Haulers at the start: the loss of both is still silent. Recommendation: (a) with the cheaper hauler; it is
  the only option that is a floor, and it costs one small capability.
- D-M. Model caps (mod M9, RE:24). Options: (a) one cap for each model, 8,000 triangles and 4 MiB, and 16 MiB for all
  models, with the class numbers of RE part 6 as art advice; (b) caps for each class (infantry 1,500, vehicles 3,000,
  buildings 8,000): the engine cannot know a class, so the mod must declare a class map, which is more API; (c) no cap in
  MP1: the art loop and the frame time have no bound. Recommendation: (a); decide before lane 0, because the caps are
  version-table rows.
- D-N. The play-test floor (perf L2): frame time p95 at or below 50 ms and p50 at or below 33 ms at 400 units on the
  play-test machine, as an entry condition of PT3 (Proposed, not a spec budget; the budgets of spec 15 stay Open).
  Options: (a) accept it; (b) no floor: a person can play a slow build and fail PT3 for the frame time. Recommendation:
  (a).
