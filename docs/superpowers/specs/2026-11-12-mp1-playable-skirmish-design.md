# MP1: Playable skirmish for two people: design

Date: 2026-11-12. Revision 3. Status: DRAFT, before gate 1. Revision 2: section 6.9 from probe MP1-P1. Revision 3: the Yardstick coverage and Player path sections (qa-lead; the completeness gate passes), 12 new acceptance rows, section 16.
Author: the orchestrator, from five seat inputs under `docs/research/mp1/` (producer, game-designer,
technical-director, ux-designer, bevy-rendering-engineer). The inputs are agent output. This design takes
their conclusions; gate 1 checks each claim that the design depends on.

Short references: `S:n` is `RTS-Engine-Project-Specifications.md` line n. `GD`, `TD`, `UX`, `RE`, `PR` are the
five input files (`input-game-designer.md`, `input-technical-director.md`, `input-ux-designer.md`,
`input-bevy-rendering-engineer.md`, `input-producer.md`). `MCD` is the MC design
(`2026-10-29-mc-player-control-design.md`, revision 16).

## 1. Goal and exit

MP1 is the first real game on the engine. Roadmap row (S:723): two people finish a match with mouse and
keyboard; both build, power and fight; both see the result screen. The mod is the first playable mod of
specification section 16.2: one faction, three infantry types, one resource and a harvester, one tank, one
turret, a command centre, a power station (S:782-805). The ground is flat (G15). There is no bot, no fog of
war, and no terrain.

MP1 is done when:

1. Every acceptance row of section 12 is green, each with its recorded red.
2. Two people play one full match to the result screen and one person signs off (section 13).
3. The technical-director audit of the whole MP1 change finds no structural defect that is open.
4. The gates and the hosted CI are green in the form that the CEO decides for MC (decision note
   `vault/Decisions/2026-11-11 MC hosted CI without a GPU.md`).
5. `docs/measurements/mp1.md` records the numbers of section 12 (recorded, not judged).

## 2. Decisions for the CEO before gate 1 closes

Each line has the team recommendation. The design below assumes the recommendation; a different choice
changes the named sections.

| Id | Question | Options | Recommendation | Changes |
| --- | --- | --- | --- | --- |
| P1 | Real models in MP1? | (a) simple shapes; (b) one static model and one texture set for each unit and building, loaded from the mod package, with the shape as fallback; (c) wait for M13 | (b), with a cap of 9 models; the engine side is 5 to 7 runs because the model loader is already in the web build (RE part 1) | 8, 14 |
| P2 | Lobby | (a) two slots, ready only, team = slot; (b) team and colour choice | (a) for the exit; colour choice is cheap and stays; team choice moves to the milestone that needs teams (PR 5c) | 7.6 |
| P3 | Scope cuts to MP2 | engineer repair, rally point, cancel of construction, charge over time | Keep all four in MP1. They are not on the critical path (PR part 1), the engineer has no function without repair, and S:798 is Decided. Cut them only if the schedule slips, in this order: rally, cancel of construction, repair | 4, 6 |
| P4 | Payment | (a) pay at the order; (b) pay over time while the job runs | (b) (GD part 2, S:723, S:434); it is what makes "On hold: no credits" a readable state | 6.4 |
| P5 | Sound | none; a minimal set (fire, hit, death, built, low power, ready, error); voices | the minimal set | 8.4 |
| P6 | Names, tone, art direction | open in S:807 | Faction: the guard force of a frontier mining company (working name "Lantern Company"), a mirror match over a claim. Tone: grounded near future, no gore. Art: low-poly, clear top-down silhouettes, team colour on large surfaces. Plain function names for units | 5 |
| P7 | The two people of the play test | two people; or the CEO and one more person | two people, one of them not the CEO, so one tester is blind | 13 |
| P8 | A lost connection | (a) the match waits for ever; (b) a slot that stays disconnected for N seconds is defeated | (b) with N = 120 s as mod data (UX part 6); without it a two-player match cannot end | 6.7 |
| P9 | Units and buildings in the way | (a) a free drop-off point in the engine and a minimum gap between buildings as a mod rule; (b) bring the M4 route step into MP1 | (a): the probe shows single buildings pass and every gap of 2.0 gives 0 blocked units; (b) moves the first match later | 6.9 |

## 3. What exists and what MP1 adds

MC gave the engine: orders as content ids in an open envelope, group movement with slot pools, the HUD
widget set with mod layout, the client shell, control groups, key bindings with a settings menu, alerts,
and the pixels-only test harness.

The engine has none of these today (verified by the seats with grep; gate 1 checks again): construction,
power, armour and damage types, teams, a match end, a lobby, a result screen, a rally point, repair,
a fire event on the wire, a unit heading on the wire, per-entry keys in the build menu, an action scope for
keys (MCD:312 says actions have a scope; `crates/turts_mod_api/src/input.rs:10-14` has none).

Two M3 validation rules remain (`crates/turts_mod_runtime/src/content_m3_validation.rs:374-376`,
`:420-424`): "a unit with health requires removal" and "a unit with a queue cannot be removed". MC already
lifted "weapons must be static". S:809 says "three"; that text is stale and the specification update after
MC corrects it.

## 4. Scope

Types: E engine mechanism for all mods; C mod content; UI client; I test instrument.

| Id | Item | Type |
| --- | --- | --- |
| X1 | Lift the two M3 rules; destruction of a producer with an explicit policy for its queued jobs | E |
| X2 | Damage types and armour classes as mod data; automatic targeting for idle and hold units with an acquire range and a leash | E |
| X4 | Construction: a `Construct` order with content and a point, a site check, a site entity that owns its progress and can be attacked, a footprint that blocks | E |
| X4b | Cancel of construction with refund | E |
| X5 | Placement mode and ghost | UI + E (interaction core) |
| X6 | Power: supply and demand for each slot, a shortage rule for each capability | E + C + UI |
| X7 | Charge over time, with the stage "waiting for funds" | E |
| X8 | Repair: a `repairer` capability and a `Repair` behaviour, charged over time | E + C |
| X9 | Rally: `SetRally` on a producer; the engine sends the follow order through the normal order path | E + UI |
| X10 | Relations: `Ally` and `Enemy` beside the present slot filter; MP1 uses two teams of one | E |
| X11 | Match lifecycle: a declarative victory rule as mod data, an outcome for each slot, surrender, the disconnect rule | E + C |
| X12 | Lobby for two slots | UI + E (server room) |
| X13 | Result screen, owned by the mod, from engine sources | UI |
| X14 | Effects: fire, hit, death, building damage; named by content id | UI + E (wire event) |
| X15 | The mod `frontier`: 8 types, ore, the map "Twin Fields", HUD layout, guide, numbers | C |
| X16 | Models (decision P1) | E + C |
| X17 | Sound set (decision P5) | C |
| X18 | MC items handed to MP1: Button `font`, "Unbound", Controls groups and keyboard navigation, the alert list defects, a value predicate for labels, the grouped failure notice (O14), the no-GPU page | UI |
| X19 | One protocol step (4 to 5) and one mod API step (0.2.3 to 0.2.4) for all of MP1 | E |
| X20 | Instruments: a headless two-client match, acceptance rows, fault plants, a pixels-only two-player driver | I |
| X21 | The play test | I |
| X22 | Final regression and CI | I |
| X23 | Architecture audit before the merge to main | I |

Not in MP1: a bot (MP2), fog of war (M5), terrain (M4), routes round obstacles beyond section 6.9,
skeletal animation (M13), more than two slots, rooms (M9), scripting (MS).

## 5. The mod `frontier` (content)

The content is in a new package `frontier`, not in `base`, because the frozen M2 stream test uses `base`
(`crates/turts_pack/tests/m3_m2_stream.rs:350-366`). All ids below passed the game-designer's scan against
engine identifiers (`light` and `shell` collide and are not used).

Roster, statistics, the damage table, the economy numbers, the placement rules, the power rule and the map
are in GD parts 1 to 6 and are not repeated here. They are proposals to measure: MP1 adds a headless duel
scenario for each balance ratio and a harvester timing, and the numbers stay Open until measured (S:807).
The ratios that carry the balance (GD part 1):

- Fights last seconds and are readable: a rifleman kills a rifleman in 9 s.
- At equal cost, riflemen lose to a tank; three launchers kill a tank; riflemen kill launchers.
- One turret beats one tank and loses to two; a turret outranges a tank.
- One harvester earns about 330 credits a minute and pays for itself in about 2.4 minutes.
- One power station supplies the command centre and two turrets; the third turret needs a second station.

Power, in the words the player gets: "When your base uses more power than it makes, everything builds at
half speed and your turrets switch off."

Victory: at the end of a tick a team is alive if one of its slots has a command centre with health above 0.
One team alive: it wins. No team alive: a draw for every team that was alive at the start of the tick.

No private path: every rule above is data in the package. No content id of `frontier` appears in engine
code; the no-content scan checks it (row NC1).

## 6. Engine mechanisms

Each mechanism is exposed to all mods through the public mod API (S 4.3, 5.2). The owner crate and the
decision that keeps later milestones cheap are from TD part B.

### 6.1 Construction (X4, X4b, X5)

- The server checks the site and owns the progress. The client ghost uses the same pure function for its
  colour only; a difference gives a refusal with a reason for each unit (D65).
- The site is an entity. It owns its progress and its job, so several sites can progress, a worker unit can
  build in a later mod, and a site can be attacked. Progress inside the producer's queue is refused: it would
  be a private shape for this one mod.
- Placement rules are data: inside the map; within a build distance of an own completed building that
  extends the area; no overlap with a footprint, with a gap; a clearance from a resource node. Footprints
  stay circles (`crates/turts_mod_api/src/schema.rs:28-30`).
- The rule "at most one work capability" (`content_m3_validation.rs:418`) gets an exemption row, because the
  command centre produces units and constructs.

### 6.2 Power (X6)

A resource kind for a capacity with supply and demand, `scope: Slot` explicit so that M6 can add networks.
Each capability names what a shortage does to it (a production rate ratio, weapon disabled). No "power"
check is hard-coded in production or combat code.

### 6.3 Damage, armour, targeting (X2)

- A damage type on the weapon, an armour class on the unit, and a table type x class of integer ratios.
  Formula: `max(min, floor(damage x ratio) - flat)`. Integer rationals only (invariant 15).
- Idle acquisition: a weapon has `acquire {range, leash}`. Today an idle unit fires only inside its weapon
  range (`crates/turts_sim/src/combat.rs:297-339`), so an outranged idle unit dies in place.
- Target queries take a visibility predicate that is always true in MP1. M5 (fog) and MP2 (bots) use the
  same query. A UnitId-to-Entity index replaces the world scans (`orders.rs:3056,3085`).

### 6.4 Payment, cancel, destruction (X7, X4b, X1)

- `ChargeOverTime`: the total charge at a progress p is `floor(cost x p)`; each tick charges the difference.
  A tick that cannot pay does not advance; the balance never goes below 0. Refund on cancel is a ratio of what
  the job charged; the ratio is mod data.
- Destruction goes through the one death path (`transaction_m3.rs:354`, called from `combat.rs:405`), which
  cancels the jobs of the destroyed unit with an `on_destroy` ratio and releases its claims.
- Every charge and refund is a ledger event, so `turts_audit` sees it. No MP1 behaviour adds a branch to a
  validator (TD part E1).

### 6.5 Repair and rally (X8, X9)

- Repair writes health only inside a transaction and charges as a ledger event.
- Rally: on spawn the engine sends the follow order through `apply_order` with the source `Engine`. A replay
  of a command log derives it again (G9). A fake player envelope is refused.

### 6.6 Relations and teams (X10)

`SlotRelation` (`schema.rs:245-249`) gains `Ally` and `Enemy`; `Other` keeps its meaning. Export takes a viewer
group, so M5 can share vision inside a team.

### 6.7 Match lifecycle, lobby, result (X11, X12, X13)

- The victory rule is declarative mod data behind one trait, so MS can add a script form later. Outcomes are
  checked once for each tick, after all removals. A defeated slot keeps its slot (D25).
- The lobby state stays out of `turts_sim`; the sim gets a match configuration. A seat holds a human or, in
  MP2, a bot, on the same path. Colour never enters the sim.
- The result screen reads only the reliable match state and the match statistics, which are readable only
  after the match is over (before that they leak enemy state).
- Readiness is a start policy that the mod selects (S:335 forbids mandatory readiness in the engine).

### 6.8 Wire and API (X19)

- One protocol step, 4 to 5: construction, power, effects, match state and lobby records; rally on the order
  record; teams and colours at the welcome; new refusal reasons; and `unit_orders` as changes only (MC audit
  debt, `export.rs:190-195`). Everything goes through the per-slot export (`export.rs:25,106`).
- `effects: [Fired{unit, target, weapon}, Destroyed{unit}]`, visible to each client that may see the units
  (all clients in MP1). The client derives a hit from the health drop. Effects are cosmetic; the state comes
  from health and removal.
- One mod API step, 0.2.3 to 0.2.4, additive, with one version-table row for each feature.
- Lane 0 (section 10) adds every schema field, table row and wire record first, with stub refusals, so that
  the feature lanes do not all edit the same schema and message files.

### 6.9 Units and buildings in the way (decision P9)

Probe MP1-P1 measured today's behaviour (`docs/research/mp1/probe-obstacles.md`; native, deterministic, the
same result in two runs). Result: PASSABLE WITH LIMITS.

- One building is passable. One unit and twenty units pass a static footprint of radius 2.5 at every offset;
  they arrive 1.00 to 1.22 times later than on a free line. A click on a building ends on a ring round it.
- A compact base traps units. With a gap between footprints that is smaller than a unit's diameter, 20 to
  50 % of tanks stay `Blocked` inside the base, and a lone tank also. With the 0.5 gap of the first content
  proposal, 7 of 20 infantry stay too. With every gap at least 2.0, 0 units are `Blocked`.
- A building near the drop-off stops the economy. A power station 1 to 2 from the command centre on the ore
  side gives 0 harvester trips: the harvester stops `Blocked` and its gather order is cleared
  (`gather.rs:1126-1149`, `orders.rs:785-819`).

MP1 takes two measures, and no route search:

1. Engine (small, generic, stays valid at M4): the approach point of a receiver or a node is the nearest
   ring point that no static footprint blocks. About 40 lines in one file with one test (estimate of the probe,
   NOT TRACED).
2. Mod placement rule, as data: the gap between two footprints is at least the largest mobile diameter plus
   0.2 (2.0 for the tank of the proposal). The cost: bases are wider, and a player cannot build a wall.

Recorded limit, as MC ruling O14: MP1 has no route round a group of obstacles. A route over the static
footprints is the first step of M4 navigation (the M4 design, `2026-10-28-m4-terrain-design.md:178-191`), not
of M4b. A `Blocked` unit in physical escape can walk up to 1.154 into a footprint (MC design 6.5 permits it);
a player can see it; it stays until M4b.

Decision P9 for the CEO: accept these two measures (recommended), or bring the M4 route step into MP1 (400 to
700 lines in 5 to 7 files, estimate of the probe; it moves the first match later).

## 7. Player interface

From UX. Engine widgets with mod layout (D67); every new item is an id, a number or a string.

### 7.1 The player path

The oracle list of the play test is the section `## Player path` at the end of this design (lobby lines L1 to
L5 and match steps 1 to 19). The testers never see it. The first three steps are the lesson of the CEO play of
MC on 2026-11-11: the goal is on the screen in one line, one hint line names the first action, and the screen
shows a reward when the action is done. Row PT3 is red when a tester does not see the site of step 3 within
60 s of "Click to play".

### 7.2 Construction

Start from the build menu of the command centre. A ghost follows the pointer. A valid place shows a solid
ring in the slot colour; an invalid place shows stripes, the `forbidden` cursor and the reason in words at
the pointer ("Something is in the way", "Cannot build on ore", "Too far from your base", "Outside the map").
Colour is never the only signal. Left click places; Shift and left click places and keeps the ghost; right
click or Esc leaves placement and never gives an order. No rotation (circles).

### 7.3 Production

Two queues on the command centre, structures and units, which run at the same time (named queues for each
producer: engine mechanism). Each entry shows a count badge and its progress. Words: "On hold: no credits",
"Slowed: low power", "Queue is full (5/5)".

### 7.4 Keys

The build menu needs one key for each entry (today one key for all: `sources.rs:391-393`). The key check is
global today, so each MP1 key is unique across all cards (UX part 2 gives the table). The action scope of
MCD:312 is not built in MP1; the MC design text is corrected to say so.

### 7.5 HUD sources

New sources, primary fields, predicates, world styles, cursor contexts and alert triggers are listed in UX
part 8. The power source is named `Supply`, because the same source gives the supply cap of a later mod.

### 7.6 Lobby and result screen

The shell owns the lobby (room lifecycle that M9 grows); its fields come from the mod settings schema
(S:314). The mod owns the result screen (D70). Words and states: UX part 6.

### 7.7 MC items

Button `font` and `size`; "Unbound"; Controls groups and keyboard navigation (search waits for M8); the alert
list clip marks and row pitch; a value predicate so that "x1 - HP" does not show for a unit with no health;
the grouped failure notice of ruling O14 ("3 of 8 units: Cannot reach that place", one sound, a click selects
the failed units); and a plain page when the browser gives no GPU adapter (the CEO met a panic with a console
message only, 2026-11-11).

## 8. Presentation

From RE.

### 8.1 Models (decision P1, option b)

- `models` in the package assets, checked like the other asset kinds: GLB version 2, no external reference,
  no skin, no animation, triangle and size caps, the named nodes present. The validator uses the same parser
  version as the client.
- The browser loads from the package bytes in memory (Bevy 0.20 `MemoryAssetReader`), with no fetch.
- The visual keeps its `shape` as the fallback while a model loads and as the content cut line.
- Team colour: a material slot named `team`, replaced at spawn by the slot material.
- The client builds one flat template for each model and spawns its own entities; it does not spawn scenes.
- The MC packages keep their shapes, so the MC pixel rows do not change.

### 8.2 Heading and aim

The client derives the yaw of a unit from its movement, with a turn rate; no wire change. A turret node aims
from the `Fired` event. Walking animation needs skinning and stays at M13; infantry gets a small bob.

### 8.3 Effects

Pooled unlit meshes for flash and spark, a line for a tracer, an expanding burst for a death, smoke at a
health threshold for a building; a cap of 256 instances; no compute shader, so WebGL2 stays possible. A mod
names each effect by content id; the server never reads them.

### 8.4 Sound

The audio engine exists. MP1 adds the minimal set of decision P5 as mod content.

### 8.5 Measurement first

The first presentation task measures the frame time at 400 units with primitives and with models, in the
browser on the real GPU (RE part 5: 10.7 to 43 ms is an estimate, not a measurement).

## 9. Architecture

- New crate `turts_rules`: pure placement, relation and damage rules, with no Bevy type, used by the sim and
  by the interaction core, so the client holds no second copy of a rule.
- The pre-MP1 slice (`2026-11-12-pre-mp1-slice-design.md`) lands first: `turts_audit`, one movement model,
  the neighbour grid, and the file splits.
- After the pure moves: a behaviour handler table in `turts_sim/src/orders/`, so a new behaviour is one new
  file and one table line; and `Core::step` of the interaction core split by input kind.
- The crate table after MP1 is TD part D.
- Refused in any MP1 plan (TD part E): a validator branch for an MP1 behaviour; a second movement path; a
  default-mod rule name in engine code; a frozen-value change other than the one approved re-freeze; client
  authority over placement or outcome.

## 10. Lanes and order

A dependency graph, not waves; a task starts when its inputs are merged (PR part 4). One worktree and one
build cache for each lane; reviewers in their own worktrees; fast checks for each task; the slow checks one
time for each merged tree.

- Lane 0: the schema, version table and wire freeze with stub refusals; `turts_rules`; the handler table.
- Critical path (39 runs with the slice done during the gates): X1, X2, X6, X15, the headless match, the
  two-player browser driver, the play test, the final regression, the audit.
- Up to 7 lanes in parallel in the first half. The tail is serial, so each feature lane owns its own pixel
  rows and the final browser driver holds only the full match.
- The GPU is the scarce resource: two headed clients for one match is tested in the first two days.

## 11. Budget

At the MC rate (PR part 3): the slice 11 to 18 runs; MP1 tasks 85 to 139; gates 20 to 35; total 116 to 192
(124 to 208 with models). Build time on the critical path 55 to 160 hours. MP2, coarse: 51 to 85 runs. These
replace the early estimate of 95 to 150 runs for MP1 and MP2 together.

## 12. Acceptance rows

Each row names what is checked, the build on which it must be red, and where it runs. Gate 1 (qa-lead)
checks that each red is reachable. The row list is a first draft; the plan fixes the commands.

| Row | Check | Red build | Runs |
| --- | --- | --- | --- |
| NC1 | No content id of `frontier` in engine code | a `frontier` id in a crate source | headless |
| VAL1 | The two M3 rules are lifted with their version-table rows; a package for API 0.2.3 with the new content is refused | a row missing | headless |
| DMG1 | Damage follows the type x class table with integer arithmetic; each balance ratio of section 5 holds in a headless duel | a float multiplier; a swapped table cell | headless |
| TGT1 | An idle unit acquires an enemy inside its acquire range and returns at its leash; hold never moves | acquisition off | headless |
| CON1 | A site is refused for each placement cause with its reason; a valid site appears, progresses, charges over time, and completes | the site check off; progress in the producer queue | headless |
| CON2 | Client ghost validity equals the server result for 1,000 seeded points | the client uses its own copy of a rule | headless |
| CON3 | Cancel refunds the charged amount by the ratio; a destroyed site refunds nothing; claims are released | refund of the full cost | headless |
| PWR1 | Supply and demand count completed buildings only; a shortage slows production by the ratio and disables turrets; recovery restores both | a hard-coded check; sites counted | headless |
| PAY1 | Charge over time: the total at completion equals the cost; the balance never goes below 0; a job that cannot pay waits and resumes | charge at the order | headless |
| REP1 | Repair raises health at the rate, charges for each point, stalls with no credits | a health write outside a transaction | headless |
| RLY1 | A new unit follows the rally through the normal order path; a replay of the command log gives the same result | a fake player envelope | headless |
| MAT1 | Victory, defeat and draw for each case, the simultaneous one included; surrender; the disconnect rule | outcome from the unit list on the client | headless, two clients |
| LOB1 | Two clients reach the lobby, set ready, and the match starts; a change clears ready; a leave reopens the slot text | start with one ready | pixels, two windows |
| RES1 | Both windows show the result screen within 1 s of the end, with the outcome and the statistics | statistics readable before the end | pixels, two windows |
| FX1 | A `Fired` event reaches each client that may see the unit; an enemy turret aims; a death shows its effect | events to the owner only | headless and pixels |
| HUD5 | The power text, the low-power state, the queue states and the credits counter show the words of section 7 | a missing source | pixels |
| KEY1 | Each build entry has its own key; no MP1 key conflicts | one key for all entries | headless |
| OBS1 | A harvester completes 20 trips with a building 1 to 2 from the command centre on the ore side; with the minimum gap rule, 0 of 20 tanks are `Blocked` in a compact base; the site check refuses a smaller gap | the approach point ignores statics; the gap rule off | headless |
| MDL1 | A package with a model loads in the browser from memory; a damaged model is refused at pack time; the shape shows until the model is ready | a URL fetch; no refusal | headless and pixels |
| VIS3 | Frame time p50 and p95 at 400 units, primitives and models | recorded, not judged | pixels |
| MCX1 | The MC items of section 7.7, one check each | each defect put back | pixels |
| NOGPU1 | With no adapter the page shows the plain message | the panic | pixels |
| PT3 | The two-person play test | a tester cannot find an object, an action or a reason, or 60 s pass with no progress | people |
| REG2 | All MC rows, the fault sets, the frozen values after the one re-freeze | any regression | all |
| GOAL1 | In the first presented frame after "Click to play": the camera shows the own command centre, and it is selected; the top line says "Destroy the enemy command centre"; one hint line names one first action and its key. A hint line shows while its condition holds and goes when the action is done; then the next hint shows. A pixels-only driver that does only what the hint lines name places a Power Station and sees the next hint within 60 s of "Click to play" | The MC `forge` start: a guide page, no objective, no selected building (PT1 person run 1, build 8d6902c); or a hint that stays after its action is done | pixels, two windows |
| GHO1 | P, or a click on the build entry, starts placement. The ghost follows the pointer. Over a valid point: a solid ring in the slot colour. Over each invalid cause (a unit, ore, too far, outside the map): stripes, the `forbidden` cursor, and the reason text of section 7.2 at the pointer. Right click and Esc end placement: the server order log has no new line, and the credits do not change. Left click places: a pending site shows in the same frame; after the server result, a world progress bar grows on the site until it is complete. Shift and left click places and keeps the ghost | One ghost style for valid and invalid; no reason text; a right click in placement sends a move | pixels |
| PRD1 | The command centre runs one structure job and one unit job at the same time, each in its own named queue. A sixth job in a full queue of 5 is refused with `queue_full`. A new unit appears at an exit point of the producer. With every exit point blocked, the completed job waits, charges nothing more, and the unit appears when an exit point is free | One shared queue; a unit that appears inside a footprint or on another unit | headless |
| PAY2 | Cancel of production under charge over time: a cancel of the active job refunds what the job charged, multiplied by the mod ratio; a cancel of a waiting job refunds 0, because the job charged 0; the ledger (`turts_audit`) balances after each cancel | Refund of the full cost; a refund for a job that charged nothing | headless |
| DES1 | The one death path removes a destroyed unit that has health and a queue (the two M3 rules lifted, VAL1). It cancels the queued jobs with the `on_destroy` ratio, releases the claims, and the ledger balances. In the same tick the footprint is free: a site on the same point passes the site check, and a unit moves across the point | A second removal path; jobs left on a removed unit; the footprint kept after the death | headless |
| ALR2 | Forced states give one alert line each, with its sound, and a minimap ping where the state has a place: Construction complete, Unit ready, Low power, Not enough credits, Base under attack, Harvester idle, a lost connection. A state that persists (low power for 60 s; 0 credits for 60 s) gives one line and one sound; the alert comes again only after the state clears and returns. A click on an alert line centres the camera on its place | The alert repeats at each cooldown while the state persists (the rule form of `mods/forge/interface/hud.ron:130-136`); a click on the line does nothing | headless and pixels |
| HUD6 | The tooltip of a build entry shows the name, the cost with "paid while it builds", the build time, the power change, and the key. A disabled entry names its cause ("Queue is full (5/5)"). The info panel of one selected unit shows its armour class; for the harvester it also shows the load and the activity ("Harvesting", "Returning", "Waiting: field empty", "Idle") | A tooltip without the cost or the key; a disabled entry with no cause | pixels |
| SMT1 | MP1 smart rules and cursors. Command centre selected: a right click on the ground sets the rally (a flag and a line show; no unit moves); a right click on ore sets a gather rally. Engineer selected: over a damaged own building the cursor is `repair`, and a right click repairs it (the health bar rises). Harvester selected: over ore the cursor is `gather`. In placement the cursor is `place` or `forbidden`. The oracle reads `turts_debug.cursor` for each context | One cursor for all contexts; a right click on the ground with the command centre selected gives a refusal or a move | pixels |
| AUD2 | The sound set of decision P5 plays from the mod package: fire, hit, death, construction complete, low power, unit ready, error. A weapon sound is positional: a shot far from the camera look-at point has a lower gain than a near shot. With 40 units that fire at once, no more weapon sounds play at once than the mod cap (`turts_debug.audio`) | A missing sound id; equal gain near and far; no cap | pixels |
| LOB2 | Colour choice in the lobby: each slot picks a colour from the mod palette. A colour that the other slot holds shows a cross and the tooltip "Player 2 has this colour", and the server refuses it. The match uses the chosen colours for the units, the rings and the minimap in both windows | Two slots with one colour; the client decides the colour | pixels, two windows |
| SUR1 | The menu item "Surrender" asks "Surrender and end the match? You will be defeated." Cancel changes nothing. Confirm defeats the slot, and both windows show the outcome and then the result screen | Surrender with no confirmation; the other window shows no result | pixels, two windows |
| TGT2 | An idle or holding unit that takes damage targets its attacker inside its leash; a unit with no weapon does not | retaliation off | headless |

## 13. Play test

The `player` agent plays through the harness with the goals only. Two people play in two windows or on two
machines; one person signs off. Goals text (UX part 9): "Play one match against the other person. Each of you
starts with a command centre, a harvester and some credits. Win by destroying the other command centre. Get
power, collect credits from ore, train an army, build a turret, and fight. Use only the mouse and the
keyboard. Say aloud when you do not know what to do next. Stop at the result screen, or after 40 minutes."

The three places where a first-time player most likely stops (UX part 9), each with its design answer: the
player does not find building (the command centre is selected at the start, with an objective line); a
silent stall from no credits or low power (the row states and alerts of section 7.3); a red ghost with no
cause (the reason at the pointer; right click and Esc cancel).

## 14. Risks and the early tests

| Risk | Early test, in the first two days |
| --- | --- |
| Units cannot pass buildings | Probe MP1-P1 (running) |
| The lobby and screens overrun, as the client shell did in MC (22 runs against 6) | A pixels-only stub: two clients reach the lobby, press ready, the match starts; red if the stub needs more than 4 runs |
| One GPU for a two-client match | Two headed clients for 10 minutes with combat; frame p95 and the wait for the GPU lock |
| The numbers do not end a match, or end it too fast | A scripted headless match with the draft numbers |
| Models cost too much frame time | The 400-unit measurement of section 8.5 |
| Art production has no natural end | The cap of decision P1 and the shape fallback |
| The re-freeze and the audit change have a long tail of moved fault plants | The slice gate runs the fault sets with the replay validator disabled |

## 15. Open points for gate 1

1. Section 6.9 has the probe result; the 40-line estimate of the engine measure is NOT TRACED.
2. The pre-MP1 slice gate is in progress; its findings can change section 9.
3. Two M2-era open decisions touch MP1 lightly and stay Open: the tick rate and the numeric budgets (S 15).
   MP1 records numbers and promises none.
4. The MP2 row asks for remembered sightings, but fog comes at M5 (PR part 3): a sequencing question for the
   MP2 design, not for MP1.

## 16. Cover that the yardstick does not ask for (from the coverage work)

The yardstick marks an objectives panel as COMMON, not CORE, so the completeness gate never asks for a visible
goal or a first action. That is the cause class of the CEO's failed MC play. MP1 covers it with its own rows:

- GOAL1 and PT3: the goal line, the first action hint, and the 60 s rule of section 7.1.
- ALR2: an alert for a state that lasts (low power, no credits, a harvester idle) shows and sounds one time when
  the state starts, and not again while it lasts. An event alert (unit ready, construction complete, under
  attack with its cooldown) sounds each time. The engine tells the start of a state from its repeat; a long
  cooldown in content is not the rule. MC row ALR1 stays green while a state fires again at each cooldown,
  which is how "it beeps non stop" passed MC.
- Refusals (OR22): a refused order names a cause that the player can act on, near the pointer. GHO1 covers
  placement; MCX1 covers the grouped notice; the same rule holds for every refused order.
- TGT2 (new): an idle or holding unit that takes damage targets its attacker inside its leash. Without it, with
  the proposed numbers, a rifleman (acquire range 7) under a turret (range 8) does not react. X2 takes this.
- Sound (AUD2): effect sounds need a small engine part (an effect names a sound; the engine plays it at the
  place of the effect, with a cap). Section 8.4 said "content only"; that was wrong. X14 takes the engine part.
- Lobby (decision P2 a): teams and team choice stay later; the starting credits setting is not in MP1.
- 57 MC rows are `done` and rest on REG2 alone. MC is not closed: its person play failed and Tasks 23 and 24
  are open. MP1 gate 2 checks again that each `done` path is green on the tree that MP1 starts from.

## Yardstick coverage

One row for each of the 181 CORE items of `docs/qa/yardstick-a.tsv`, in file order. `in` rows name a
row of section 12. `done:` rows name the evidence of an earlier milestone: for an MC item it is the MC
row file or test, and the note names the MC row. MC acceptance is not closed (PT1 person run 1 failed on
2026-11-11), so REG2 runs every MC row again, and a red MC row makes MP1 red. `later:` rows name a
milestone after MP1 in `docs/qa/milestones.tsv`; the note gives the reason and the MP1 part with its row,
if MP1 builds a part. Where a yardstick dependency is later than MP1, the item is `later:` even when MP1
builds its flat-ground part. The dispositions assume the recommendations of section 2 (P1 b, P2 a, P4 b,
P5 minimal set, P8 b).

| id | disposition | test | item | note |
| --- | --- | --- | --- | --- |
| IN01 | done:tests/browser/rows/SEL1.json | - | Screen-to-world picking | MC SEL1; REG2 runs it again |
| IN02 | done:tests/browser/rows/KM1.json | - | Action mapping layer | MC KM1; REG2 runs it again |
| IN03 | done:crates/turts_interaction/tests/km3.rs | - | Modifier semantics | MC KM3; REG2 runs it again |
| IN04 | done:crates/turts_interaction/tests/km3.rs | - | Double-click detection | MC KM3; REG2 runs it again |
| IN05 | in | KEY1 | Hotkey per command button | MC HUD2. MP1: one key for each build entry |
| IN08 | done:tests/browser/rows/KM2.json | - | Browser input capture | MC KM2; REG2 runs it again |
| IN09 | later:M9 | - | Text input mode | as MC |
| PC01 | done:tests/browser/rows/SEL1.json | - | Single select | MC SEL1; REG2 runs it again |
| PC02 | done:tests/browser/rows/SEL2.json | - | Box (drag) select | MC SEL2; REG2 runs it again |
| PC04 | done:tests/browser/rows/SEL2.json | - | Shift add and remove | MC SEL2; REG2 runs it again |
| PC05 | done:tests/browser/rows/SEL3.json | - | Type select on screen | MC SEL3; REG2 runs it again |
| PC08 | done:tests/browser/rows/SEL4.json | - | Control group assign and recall | MC SEL4; REG2 runs it again |
| PC09 | done:tests/browser/rows/SEL4.json | - | Control group add | MC SEL4; REG2 runs it again |
| PC10 | done:tests/browser/rows/SEL4.json | - | Group double-tap centres camera | MC SEL4; REG2 runs it again |
| PC16 | done:tests/browser/rows/SEL1.json | - | Inspect foreign entities | MC SEL1; REG2 runs it again |
| PC17 | done:tests/browser/rows/ORD2.json | - | Ownership gate | MC ORD2; REG2 runs it again |
| PC19 | done:tests/browser/rows/SEL4.json | - | Selection hygiene | MC SEL4; REG2 runs it again |
| PC20 | done:tests/browser/rows/SEL1.json | - | Hover highlight | MC SEL1; REG2 runs it again |
| PC21 | in | SMT1 | Context cursor | MC HUD4. MP1: place, repair, rally cursors |
| PC22 | in | SMT1 | Smart command | MC ORD5. MP1: rally and repair smart rules |
| PC23 | done:tests/browser/rows/VIS1.json | - | Selection visual | MC VIS1; REG2 runs it again |
| OR01 | done:tests/browser/rows/ORD5.json | - | Move | MC ORD5; REG2 runs it again |
| OR02 | done:tests/browser/rows/ORD3.json | - | Stop | MC ORD3; REG2 runs it again |
| OR03 | done:tests/browser/rows/ORD4.json | - | Attack target | MC ORD4; REG2 runs it again |
| OR04 | done:tests/browser/rows/ORD4.json | - | Attack-move | MC ORD4; REG2 runs it again |
| OR05 | done:tests/browser/rows/ORD4.json | - | Force move | MC ORD4; REG2 runs it again |
| OR08 | in | TGT1 | Hold position | MC ORD3. MP1: hold with idle acquisition |
| OR11 | done:tests/browser/rows/ORD3.json | - | Order queue per unit | MC ORD3; REG2 runs it again |
| OR12 | done:tests/browser/rows/ORD3.json | - | Shift-queued orders | MC ORD3; REG2 runs it again |
| OR15 | in | RLY1 | Rally point | MC later:MP1. Visible flag: SMT1 |
| OR16 | in | CON1 | Build order | MC later:MP1. The producer gets the order (C&C form); a worker that walks to the site: not in MP1 |
| OR17 | done:tests/browser/rows/ORD5.json | - | Gather order | MC ORD5; REG2 runs it again |
| OR20 | later:MB | - | Ability order | as MC |
| OR22 | in | MCX1 | Order validation feedback | MC ORD6. MP1: grouped failure notice (O14); also CON1 reasons |
| OR23 | done:tests/browser/rows/ORD7.json | - | Order acknowledgement | MC ORD7; REG2 runs it again |
| OR24 | done:tests/browser/rows/ORD2.json | - | Group order fan-out | MC ORD2; REG2 runs it again |
| OR25 | done:tests/browser/rows/GM40.json | - | Group destination spread | MC GM40; REG2 runs it again |
| OR26 | later:MP2 | - | Order state machine | MC later:MP1. Needs CB05 (later:MP2). MP1 part: TGT1 |
| OR31 | done:tests/browser/rows/ORD6.json | - | Command rate limit | MC ORD6; REG2 runs it again |
| GM01 | done:tests/browser/rows/GM40.json | - | Destination spreading | MC GM40; REG2 runs it again |
| GM04 | done:crates/turts_server/tests/mc_group.rs | - | Local avoidance | MC GMA; REG2 runs it again |
| GM06 | done:crates/turts_server/tests/mc_group.rs | - | Unit collision size | MC GMA; REG2 runs it again |
| GM07 | later:M4b | - | Push idle friends | as MC |
| GM09 | done:tests/browser/rows/GM40.json | - | Arrival | MC GM40; REG2 runs it again |
| GM10 | done:crates/turts_server/tests/mc_group.rs | - | Unreachable target | MC GMU; REG2 runs it again |
| GM11 | later:M4b | - | Choke traffic | as MC |
| CA01 | done:tests/browser/rows/CAM1.json | - | Classic RTS camera | MC CAM1; REG2 runs it again |
| CA02 | done:tests/browser/rows/CAM1.json | - | Edge scroll | MC CAM1; REG2 runs it again |
| CA03 | done:tests/browser/rows/CAM1.json | - | Key scroll | MC CAM1; REG2 runs it again |
| CA08 | done:tests/browser/rows/MM1.json | - | Minimap click | MC MM1; REG2 runs it again |
| CA09 | done:tests/browser/rows/MM1.json | - | Minimap order | MC MM1; REG2 runs it again |
| CA13 | done:tests/browser/rows/CAM1.json | - | Bounds clamp | MC CAM1; REG2 runs it again |
| CA14 | done:tests/browser/rows/CAM1.json | - | Centre on selection | MC CAM1; REG2 runs it again |
| CA15 | later:M4 | - | Height-aware camera | as MC |
| CA16 | done:tests/browser/rows/SH1.json | - | Scroll speed setting | MC SH1; REG2 runs it again |
| UI01 | in | KEY1 | Command card | MC HUD2. MP1: unique card cells and keys |
| UI02 | in | MCX1 | Selection panel | MC HUD2. MP1: value predicate, no 'x1 - HP' |
| UI03 | in | HUD6 | Info panel and portrait | MC HUD2. MP1: armour, load and activity |
| UI04 | in | HUD5 | Resource bar | MC HUD2. MP1: credits and power |
| UI05 | done:tests/browser/rows/MM1.json | - | Minimap | MC MM1; REG2 runs it again |
| UI08 | done:tests/browser/rows/MM1.json | - | Minimap alert flash | MC MM1; REG2 runs it again |
| UI09 | in | HUD6 | Build menu | MC HUD2. MP1: build menu of the command centre |
| UI10 | in | HUD5 | Production queue display | MC HUD2. MP1: two queues, badges, row states |
| UI11 | in | HUD6 | Tooltips | MC HUD2. MP1: cost, power change and key in the tooltip |
| UI12 | done:tests/browser/rows/VIS1.json | - | Health bars | MC VIS1; REG2 runs it again |
| UI14 | later:M5 | - | Fog rendering | as MC |
| UI16 | in | ALR2 | Event alerts | MC ALR1. MP1: new alerts; one alert for a state that persists |
| UI17 | later:M9 | - | Chat | as MC |
| UI19 | in | SUR1 | Surrender and leave | MC later:MP1. Headless: MAT1 |
| UI22 | in | MCX1 | In-game options menu | MC SH1. MP1: Controls groups, keyboard navigation, Unbound |
| UI23 | in | SMT1 | Cursor set | MC HUD4. MP1: new cursor contexts |
| CO01 | in | GHO1 | Placement ghost | MC later:MP1 |
| CO02 | later:M4 | - | Footprint validity | MC later:MP1. Needs CO03. MP1 part (flat ground): GHO1, CON2 |
| CO03 | later:M4 | - | Placement rules | MC later:MP1. Needs MP01 (map format, M4). MP1 part (flat ground): CON1, CON2 |
| CO04 | in | CON1 | Construction model | MC later:MP1 |
| CO07 | in | CON1 | Under construction state | MC later:MP1. Attack on a site: CON3 |
| CO08 | in | CON3 | Cancel and refund | MC later:MP1 |
| CO20 | in | DES1 | Destruction clears footprint | MC later:MP1 |
| CO21 | later:M4b | - | Placement vs units | MC later:MP1. Needs GM07 (push, M4b) and CO03. MP1 part: CON1 refuses a site over a unit |
| PR01 | done:crates/turts_pack/tests/economy_production.rs | - | Production queue | as MC |
| PR02 | in | PAY2 | Cancel and refund | MC done. MP1: refund of what the job charged |
| PR03 | in | PAY1 | Payment policy | MC done. MP1: charge over time (decision P4 b) |
| PR04 | in | PRD1 | Parallel production | MC later:MP1 |
| PR05 | in | PRD1 | Spawn and exit | MC later:MP1 |
| PR06 | in | RLY1 | Rally applied | MC later:MP1 |
| PR07 | done:crates/turts_pack/tests/forge_cases.rs | - | Prerequisites | as MC |
| PR08 | done:crates/turts_pack/tests/forge_cases.rs | - | Research and upgrades | as MC |
| PR10 | in | HUD6 | Unavailable reason | MC HUD2. MP1: queue full and no-credit causes |
| PR15 | done:crates/turts_pack/tests/forge_cases.rs | - | Modifier application | as MC |
| PR17 | later:MB | - | Asymmetric factions | as MC |
| EC01 | in | PWR1 | Resource types | MC done. MP1: a capacity resource kind for power |
| EC02 | in | OBS1 | Gather cycle | MC ORD5. MP1: free approach point at the receiver |
| EC03 | done:crates/turts_pack/tests/economy_gather.rs | - | Drop-off choice | MC later:MP1. M2 test nearest_receiver_wins_before_id_tie_break (:471) |
| EC04 | done:tests/browser/rows/ORD3.json | - | Node depletion | MC ORD3; REG2 runs it again |
| EC05 | done:crates/turts_pack/tests/economy_gather.rs | - | Auto-resume and seek | MC later:MP1. M2 test depleted_original_reselects_nearest_node_after_delivery (:656) |
| EC10 | in | PAY1 | Insufficient funds | MC done. MP1: the job waits for funds and resumes |
| CB01 | done:crates/turts_pack/tests/forge_cases.rs | - | Health and death | as MC |
| CB02 | done:crates/turts_pack/tests/forge_cases.rs | - | Weapons | as MC |
| CB03 | in | TGT1 | Auto-target | MC ORD4. MP1: acquire range and leash |
| CB04 | later:MP2 | - | Target priority | MC later:MP1. MP1 rule: ordered target, then the nearest enemy (TGT1); no threat priority |
| CB05 | later:MP2 | - | Retaliation | MC later:MP1. Not in X2; see the weak-cover list |
| CB07 | in | DMG1 | Damage and armour types | MC later:MP1 |
| CB09 | later:MB | - | Splash damage | as MC |
| CB10 | later:MB | - | Projectiles | as MC |
| CB21 | in | PWR1 | Defensive buildings | MC done. MP1: a turret is off in a power shortage |
| CB25 | later:MB | - | Bonus damage | as MC |
| FV01 | later:M5 | - | Black shroud | as MC |
| FV02 | later:M5 | - | Fog of war | as MC |
| FV03 | later:M5 | - | Sight radius | as MC |
| FV04 | later:M5 | - | Server-side visibility | as MC |
| FV05 | later:M5 | - | Shared vision | as MC |
| FV10 | later:M5 | - | Last-seen structures | as MC |
| UN01 | later:M4 | - | Ground units and movement classes | as MC |
| UN07 | later:MB | - | Abilities | as MC |
| UN11 | later:MB | - | Workers | MC later:MP1. No unit gathers and builds in 16.2 (S:798-800) |
| UN13 | in | DES1 | Death and removal | MC done. MP1: removal of a unit with health and a queue (X1) |
| PF01 | done:tests/browser/rows/GM40.json | - | Sensible path | MC GM40; REG2 runs it again |
| PF02 | later:M4 | - | Repath on change | as MC |
| PF03 | later:M4 | - | Movement layers | as MC |
| PF04 | in | OBS1 | No stuck units | MC GMU. MP1: 0 of 20 tanks Blocked in a compact base |
| PF05 | done:tests/browser/rows/ORD7.json | - | Instant start | MC ORD7; REG2 runs it again |
| PF10 | later:M7 | - | Mass pathing | as MC |
| MF01 | later:M8 | - | Main menu | as MC |
| MF02 | later:M9 | - | Lobby | MC later:MP1. Needs MD12 (later:M9). MP1 part: LOB1, LOB2 (two slots, colour, ready; team = slot, decision P2 a) |
| MF03 | later:M9 | - | Map choice | as MC |
| MF04 | later:M9 | - | Game settings | as MC |
| MF05 | later:MP2 | - | AI in slots | as MC |
| MF06 | later:M4 | - | Start conditions | MC later:MP1. Needs MP01 (M4). MP1 part: fixed starts of Twin Fields, MAT1 |
| MF07 | in | LOB1 | Load and countdown | MC later:MP1. Countdown, one start for both |
| MF08 | in | MAT1 | Victory and defeat | MC later:MP1 |
| MF10 | later:M5 | - | Team victory | MC later:MP1. Needs TM01 (later:M5) |
| MF12 | in | RES1 | Score screen | MC later:M9. MP1: units, buildings, credits, time; no graphs, no APM |
| MF14 | in | MAT1 | Disconnect handling | MC later:M10. MP1: defeat after 120 s disconnected (decision P8 b) |
| SV01 | later:M10 | - | Single-player save and load | as MC |
| SV04 | later:M10 | - | Save version safety | as MC |
| SV05 | later:M10 | - | Mod save data | as MC |
| MP01 | later:M4 | - | Map format | as MC |
| MP02 | later:MB | - | Map editor | as MC |
| MP05 | later:M9 | - | Map metadata | as MC |
| MP07 | done:crates/turts_server/tests/acceptance.rs | - | Map distribution | MC later:M8. The client downloads the locked package with its maps from the server since M1 (t2a_package_parity) |
| MP09 | done:crates/turts_server/tests/mc_group.rs | - | Playable bounds | MC GMU; REG2 runs it again |
| MD01 | in | VAL1 | Unit definition | MC MOD1. MP1: armour, power, placement, repairer, acquire fields |
| MD02 | in | DMG1 | Weapon and projectile definition | MC later:MB. MP1: damage type, acquire, effect ids; projectile travel stays MB (CB10) |
| MD03 | in | CON1 | Building definition | MC later:MP1. Power: PWR1 |
| MD04 | later:MB | - | Ability definition | as MC |
| MD05 | done:crates/turts_pack/tests/forge_cases.rs | - | Upgrade and tech definition | as MC |
| MD06 | later:MB | - | Faction definition | as MC |
| MD07 | in | DMG1 | Damage and armour tables | MC later:MP1 |
| MD08 | in | KEY1 | UI definitions | MC HUD3. MP1: entry hotkey and card fields |
| MD09 | done:tests/browser/rows/AUD1.json | - | Audio definitions | MC AUD1; REG2 runs it again |
| MD10 | in | MAT1 | Victory definitions | MC later:MP1 |
| MD11 | later:MP2 | - | AI definitions | as MC |
| MD12 | later:M9 | - | Lobby settings schema | as MC |
| MD17 | in | MDL1 | Asset pipeline | MC later:M13. MP1: static models and textures (decision P1 b); animation stays M13 |
| MD18 | done:crates/turts_pack/tests/bad_packages.rs | - | Validation messages | as MC |
| BT01 | later:MP2 | - | Skirmish AI | as MC |
| BT02 | later:M12 | - | Difficulty levels | as MC |
| BT03 | later:MP2 | - | Same command path | as MC |
| TM01 | later:M5 | - | Teams | MC later:MP1. Decision P2 a: team = slot; M5 needs teams (S:727) |
| TM02 | later:M5 | - | Alliance effects | MC later:MP1. Needs TM01. MP1 part: Enemy relation in TGT1 |
| TM03 | in | LOB2 | Player colours | MC VIS1. MP1: colour choice in the lobby |
| AU01 | done:tests/browser/rows/AUD1.json | - | Unit responses | MC AUD1; REG2 runs it again |
| AU02 | in | ALR2 | Alerts | MC ALR1. MP1: alert sounds once for each state |
| AU03 | in | AUD2 | Positional effects | MC later:MP1. Decision P5 sound set |
| AU06 | done:tests/browser/rows/AUD1.json | - | Volume categories | MC AUD1; REG2 runs it again |
| AU07 | in | AUD2 | Sound limiting | MC later:MP1 |
| AN01 | later:M13 | - | Unit animations | as MC |
| AN02 | later:MB | - | Weapon effects | MC later:MP1. Needs CB10 (projectiles, MB). MP1 part: FX1 |
| AN03 | in | GHO1 | Construction visuals | MC later:MP1 |
| AN05 | in | FX1 | Death visuals | MC later:MP1 |
| AN06 | done:tests/browser/rows/VIS1.json | - | Decals for orders | MC VIS1; REG2 runs it again |
| AN07 | in | MDL1 | Team colour on models | MC VIS1. MP1: material slot team on models (decision P1 b) |
| AN08 | done:crates/turts_client/tests/interp.rs | - | Tick interpolation | as MC |
| AN11 | later:M13 | - | Instancing and level of detail | as MC |
| AC05 | done:tests/browser/rows/CAM2.json | - | Cursor confine | MC CAM2; REG2 runs it again |
| AC06 | later:M13 | - | Graphics settings | as MC |
| PE01 | done:tests/browser/rows/ORD7.json | - | Order responsiveness | MC ORD7; REG2 runs it again |
| PE02 | later:M7 | - | Stable frame rate | as MC |
| PE03 | later:M7 | - | Unit count | as MC |
| PE04 | done:tests/browser/rows/SEL2.json | - | Large selection speed | MC SEL2; REG2 runs it again |
| PE07 | done:tests/browser/rows/ORD7.json | - | Latency hiding | MC ORD7; REG2 runs it again |

## Player path

This is the oracle list of PT3. The testers never see it: they get the goals text of section 13. The source is
UX part 1, corrected by PT1 person run 1 (2026-11-11: no goal, no first action, an alert sound that repeats).
It assumes decisions P2 (a), P4 (b), P5 and P8 (b). Setup: two browser windows A and B, or two machines;
mouse and keyboard only. The brackets name the rows of section 12 that check the step.

Lines L1 to L5 are the lobby. They come first, but their time depends on the second person, so they have no
clock. The clock of steps 1 to 3 starts at the click on "Click to play". Steps 1 to 3 are the first 60
seconds: a person with no knowledge of the game must do them from the screen alone. PT3 is red when a tester
does not see the site of step 3 within 60 s of that click.

- L1. A: the load screen shows each phase and a progress bar (MC SH1).
- L2. A: the lobby shows the goal line "Destroy the enemy command centre", the rows "Slot 1: You — Not ready"
  and "Slot 2: Open — waiting for a player", and the status line "Waiting for a second player" (LOB1).
- L3. B joins. Both windows show "Slot 2: Player 2 — Not ready". Each slot has a different default colour
  (LOB1, LOB2).
- L4. A picks the colour of B: the swatch shows a cross, and its tooltip says "Player 2 has this colour". A then
  picks a free colour, and both windows show it (LOB2).
- L5. Both press Ready. The rows show "Ready", then "Match starts in 3", "2", "1". A change by a player clears
  the ready state of that player. Both windows show "Click to play" (LOB1).

1. In the first frame after "Click to play": the camera shows the own command centre, and it is selected. The
   top line says "Destroy the enemy command centre". The HUD shows the credits and the power, with "Low power",
   because the command centre uses power and nothing makes it (GOAL1, HUD5).
2. One hint line names one first action and its key: "Build a Power Station: press P, then click near your
   command centre." The tester presses P or clicks the entry. The ghost follows the pointer. The tester
   left-clicks where the ring is solid (GOAL1, GHO1).
3. The reward shows at once: the site shows in the same frame, then a progress bar that grows, and the credits
   count down. When the station is complete, the alert "Construction complete" shows with its sound, the power
   text shows more power made than used, "Low power" goes, and the hint line names the next action (GOAL1,
   GHO1, ALR2, PWR1).
4. The tester hovers the Turret entry. The tooltip shows the name, the cost with "paid while it builds", the
   time, the power change and the key (HUD6).
5. The tester presses the Turret key. Over the harvester the ghost shows stripes and "Something is in the way";
   over ore, "Cannot build on ore"; far out, "Too far from your base". A right click ends placement: the ghost
   goes, and the credits do not change (GHO1).
6. The tester selects the Ore Hauler and right-clicks the ore field. The gather cursor shows, then the gather
   marker. The load in the info panel goes up. At the command centre, "+N" rises, and the credits go up by N
   (SMT1, HUD6, HUD5).
7. The tester selects the command centre and presses the Rifleman key three times. The entry shows the badge
   "3". The first row of the unit queue shows its progress and the seconds left. A structure job in the other
   queue progresses at the same time (HUD5, PRD1).
8. The tester right-clicks the ground with the command centre selected. A flag shows, with a line from the
   command centre. The new rifleman walks to the flag, and the alert "Unit ready" shows (SMT1, RLY1, ALR2).
9. The tester right-clicks the active job in the queue. The job goes, "+N" rises at the command centre, and the
   credits go up by N, the amount that the job paid. A cancel of a waiting job gives back nothing, because it
   paid nothing (PAY2, HUD5).
10. The tester builds turrets until the base uses more power than it makes. The power text shows "Low power"
    with a mark. The alert "Low power" shows once. The queue rows say "Slowed: low power". Each turret shows an
    offline mark. A second Power Station clears all of these (PWR1, HUD5, ALR2).
11. The credits reach 0 during a job. The row says "On hold: no credits", with a striped bar, and the alert "Not
    enough credits" shows once. The job continues after the next delivery (PAY1, HUD5, ALR2).
12. B selects units, presses A and left-clicks near the base of A. The attack cursor shows, then a cross marker at
    once. The units move, and they fire by themselves at enemies in range (TGT1, MC ORD4).
13. In the fight: muzzle flashes, tracers, impacts, and health bars on damaged units. A weapon sound near the
    camera is louder than a far one. A gets "Base under attack" with a minimap ping. A click on the alert line
    centres the camera on the base (FX1, AUD2, ALR2).
14. An enemy turret turns to its target and fires (FX1).
15. Units die with a death effect, and they leave the selection and their groups. A partly refused order shows
    "3 of 8 units: Cannot reach that place" with one error sound. A click on the line selects the failed units
    (FX1, MCX1).
16. The engineer of A right-clicks a damaged own building. The repair cursor and the repair effect show, the
    health bar rises, and the credits go down. At 0 credits the repair stops (SMT1, REP1).
17. A command centre is destroyed. Within 1 s, both windows show "Victory" or "Defeat". After 3 s, the result
    screen shows both slots, the outcomes and the statistics. "Leave" opens the shell screen "The match is over.
    Reload this page to join the next match." (MAT1, RES1).
18. In a second match: A opens the menu and selects "Surrender". The confirmation text shows. Cancel changes
    nothing. Confirm defeats A, and both windows show the result (SUR1).
19. In a third match: B closes its window. A shows "Player 2 lost connection. Waiting 2:00 for them to return."
    When B does not return within 2:00, B is defeated, and A shows the result. When B reloads within 2:00, B
    rejoins its slot and the match goes on (MAT1, ALR2, MC SLOT1).
