# MP1: Playable skirmish for two people: design

Date: 2026-11-12. Revision 2. Status: DRAFT, before gate 1. Revision 2: section 6.9 from probe MP1-P1.
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

The oracle list of the play test is UX part 1 (23 steps from the page load to the result screen). The
testers never see it.

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
