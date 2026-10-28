> **Research input, not a contract.** Written 2026-10-27 by the game-designer seat during the M3 brainstorm. Where it conflicts with `docs/superpowers/specs/2026-10-27-m3-industrial-design.md`, the design wins. All numbers are placeholders.

# M3 "Industrial loop" — content and demonstration design (game-designer, brainstorm, 2026-10-27)

Status of this note: **Proposed**. Every number is a **PLACEHOLDER** (test value) and is not a
balance decision. The engine shape (schema field names, belt representation, grid) belongs to
the engine designer; the RON below shows the content need, not a schema.

Sources read: spec = `RTS-Engine-Project-Specifications.md` (S); M2 design =
`docs/superpowers/specs/2026-10-26-m2-economy-design.md` (M2D); M2 research =
`docs/research/m2/economies-game-designer.md` (R2); packages under `mods/`.

## 0. Status facts this design depends on

| Fact | Where | Status |
| --- | --- | --- |
| M3 demonstration and acceptance: "Mine → belt → factory → inserter → ammunition turret → research upgrade"; "Inputs/outputs remain consistent under congestion, interruption, and cancellation" | S:678 | Proposed (milestone table is "not a committed delivery schedule", S:672) |
| Research, factories with inputs/outputs, belts, inserters are in scope | S:30 (D07) | Decided |
| Engine mechanisms vs mod policy; Combat row: engine = target queries, damage events; mod = weapons, armor, targeting restrictions; Progression row: engine = prerequisites, unlocks, modifiers | S:151, S:159, S:161 | Decided |
| Ammunition is a physical item | S:397 | Baseline (7.1 table) |
| Explicit cancellation/refund/interruption/output-blockage/destruction policies; three payment policies | S:406 | Baseline |
| Recipes: inputs, outputs, duration, eligible machines, optional operating conditions | S:410 | Baseline |
| Belts: movement, lanes where enabled, junctions, occupancy, backpressure; mod: speed, connections, appearance, permitted items; inserters: reach, rate, filters, stack size | S:414 | Baseline |
| Compact belt representation, not one ECS entity per item; segment representation, junction fairness, scheduling need benchmarks | S:416 | Baseline direction; details Open |
| Research is a graph; definitions separate from player state; mod defines costs, labs, rate, effects | S:424-426 | Baseline |
| Research cancellation, parallel research, exclusive branches, repeatable techs, team sharing, modifier stacking | S:428 | **Open** |
| No per-unit or per-item script per frame | S:183 | Baseline |
| Default game has no private privilege | S:189, S:27-28 (D04, D05), S:539 (invariant 1) | Decided |
| Reservations and transfers cannot double-spend | S:543 (invariant 5) | Baseline |
| Depleted deposits stay depleted | S:542 (invariant 4); M2D:280 | Baseline / M2 approved |
| Industry and research may be follow-on scenarios separate from the 16.2 game | S:751 | Decided text of 16.2 |
| Combat was assigned to M7 by the M1 design | `docs/superpowers/specs/2026-10-26-m1-foundation-design.md:46`; M2D:68-70 | M1 design scope note |
| M2 producer output is exactly one mobile unit | M2D:166 | M2 approved |
| M2 gatherer requires movement | M2D:158 | M2 approved |
| M2 commands: MoveTo, Gather, Produce, CancelProduction | M2D:366-371 | M2 approved |
| M2 cancel: refund floor(charged*ratio) to the original store; reject RefundDestinationFull, no silent loss | M2D:338-353 | M2 approved |
| M2 OutputBlocked holds progress, never recharges | M2D:321-323 | M2 approved |
| M2 removal/destruction excluded; future removal needs an explicit policy for held stock | M2D:306-308 | M2 approved |
| Inventory capacity counts on_hand, not available | M2D:214-215 | M2 approved |
| Default tick rate 20 Hz | `crates/turts_sim/src/lib.rs:26` | prototype constant |
| Rate budget carry formula | M2D:191-195 | M2 approved |
| Existing packages: base, haul, cedar, acme; no `forge` | `mods/*/manifest.ron` (listed 2026-10-27) | fact |
| Live playtest: objective and unit purpose were unclear to the user | `vault/Work Log/2026-10-27.md:86` | fact |

## 1. Content set: new package `forge`

Package id `forge`, kind `Game`, engine_api `^0.3` (placeholder; the API bump is the engineer's
call). Pure physical-item economy: no Balance resource, no power (like `haul`,
`mods/haul/manifest.ron`). It is a separate package, not part of `base`, because 16.2 lets
industry be a follow-on scenario (S:751) and `base` carries the decided C&C rules (S:738-749).

### 1.1 Resources (7.1 kind)

| Id | Name | Kind | Notes |
| --- | --- | --- | --- |
| `forge:ore` | Ore | Item (physical) | Mined, carried on belt, recipe input |
| `forge:shell` | Shell | Item (physical) | Recipe output; turret ammunition; research cost |

One item = one capacity unit (M2 rule, M2D:150). No stack-size-per-item system; "stack size" is
an inserter property only (S:414).

### 1.2 World and entity definitions (all numbers PLACEHOLDER, 20 Hz)

| Id | Role | Capabilities and numbers |
| --- | --- | --- |
| `forge:ore_vein` | finite surface node | item ore; radius 1.0; map remaining **300** (depletion fixture: **21**) |
| `forge:drill` | static extractor | footprint r 1.2; extracts ore from a node in range (M2 range formula, M2D:273-274), interaction_range 0.5; **2 ore/s** (1 per 10 ticks); output inventory cap **4**, accepts [ore] |
| `forge:belt` | belt piece | length 1.0 world unit; **4 slots** per piece; items advance **1 slot per 5 ticks** (4 slots/s = 1 piece/s, max throughput 4 items/s); accepts [ore, shell]; straight chain only; facing given by map |
| `forge:inserter` | inserter | footprint r 0.4; reach: pick point 1.0 behind, drop point 1.0 ahead along facing; **stack 1**; cycle **10 ticks** (5 to drop, 5 to return) = **2 items/s**; no filter (empty filter = any item the destination accepts) |
| `forge:press` | factory machine | footprint r 1.2; static; one recipe, auto-repeat while enabled; input inventory cap **6**, accepts [ore]; output inventory cap **4**, accepts [shell] |
| `forge:turret` | ammunition turret | footprint r 1.2; static; ammo inventory cap **5**, accepts [shell]; weapon: range **8.0**, cooldown **20 ticks** (1 shot/s), **1 shell per shot**, damage **10**, instant hit, auto-target nearest hostile with health in range (tie: lowest UnitId) |
| `forge:lab` | laboratory | footprint r 1.2; static; input inventory cap **10**, accepts [shell]; runs one research job at a time |
| `forge:drone` | damageable target | footprint r 0.6; mobile, speed **2.0**; health **100**; no weapon, no inventory |

### 1.3 Recipe and research

| Id | Machine | Inputs | Output | Duration | Payment | Cancel refunds |
| --- | --- | --- | --- | --- | --- | --- |
| `forge:make_shells` | press | 2 ore from press input | 1 shell into press output | **2000 ms** (40 ticks) | charge at start (consume inputs when the job starts; the M2 ReserveThenCharge start step, with no separate queue wait) | running **1/1**, output_blocked **1/1** |
| `forge:hardened_shells` (research) | lab | 10 shells from lab input | technology state "researched" for the owner slot | **10000 ms** (200 ticks) | ReserveThenCharge (M2D:312-323) | queued **1/1** (M2 rule M2D:164-165), running **1/2** |

Research effect: **+10 damage** (additive) to the weapon of `forge:turret` units of the owner
slot. Base 10 → 20. Prerequisites: `[]` (keep the field so the graph shape exists, S:424).

Research policies for `forge` only (S:428 says these are **Open**; these are package-local
proposals, not engine rules, **needs decision** for the engine API shape): per-slot state; not
repeatable; a Research request for a technology that is researched or already queued rejects;
cancel uses the declared refund ratios; one modifier only, so no stacking question arises.

### 1.4 Map `forge:range` (32 x 16, PLACEHOLDER coordinates)

Slot 0 owns the industry. Slot 1 owns two drones. UnitId order must give inserter B a lower id
than inserter C (tie-break below). That map placement order sets UnitId order is **NOT
TRACED** in code.

| Entity | Owner | Position | Facing | Contents |
| --- | --- | --- | --- | --- |
| ore_vein | — | (3.5, 8) | — | remaining 300 |
| drill | 0 | (6, 8) | — | empty |
| inserter D (drill→belt) | 0 | (8, 8) | +x | empty |
| belt pieces 0..5 | 0 | (9..14, 8) | +x | empty |
| inserter A (belt→press) | 0 | (15, 8) | +x | empty |
| press | 0 | (17, 8) | — | empty |
| inserter B (press→turret) | 0 | (17, 10) | +y | empty |
| inserter C (press→lab) | 0 | (17, 6) | −y | empty |
| turret | 0 | (17, 12) | — | empty |
| lab | 0 | (17, 4) | — | empty |
| drone A | 1 | (24, 12) | — | health 100; distance 7.0 from turret: in range |
| drone B | 1 | (30, 4) | — | health 100; distance 15.3: out of range |

Inserter endpoint rule (content need): an inserter picks from the **output** side of the
entity whose footprint contains its pick point, or from the belt piece at that point; it
drops into the **input** side of the entity at its drop point, or onto the belt piece there.
Drill output = its inventory. Turret and lab have input only. Press has both.

## 2. Demonstration scenario

### 2.1 Recommendation: pre-placed layout, no player construction

Everything in 1.4 is placed by the map, like M2 static producers (M2D:41-43, 46). Player
actions are commands on existing entities only.

| Option | Engine cost |
| --- | --- |
| **Pre-placed (recommended)** | Map schema for belt pieces with facing and inserters with facing; boot validation of belt chains and inserter endpoints; topology update when a piece is removed. |
| Player-built | All of the above plus: placement command, grid snapping and rotation, footprint and belt occupancy checks, cost and payment for buildings (forge has no balance, so material delivery or a new rule), construction progress (S:158), endpoint re-resolution on every build, client ghost/drag/rotate UI, and validation of every placement request. Roughly doubles M3 and overlaps the open construction work for 16.2 (S:745). |

Commands M3 needs (all owner-checked by the server, M2D:218):

| Command | Status | Use |
| --- | --- | --- |
| MoveTo | exists (M2D:367) | slot 1 moves drone B into range |
| Produce / CancelProduction | exist (M2D:369-370) | start research at the lab; cancel research; cancel the running press job |
| `SetEnabled(entity, bool)` | new, **should** | reversible interruption: pause drill, inserter, press, turret, lab |
| `Remove(entity)` | new, **must** for the destroy case | remove an own belt piece or inserter; contents follow the removal policy (3.0) |

### 2.2 Timeline (ESTIMATE from placeholder rates, NOT measured)

| ~Time | Event |
| --- | --- |
| 0.5 s | first ore in drill; inserter D puts it on belt piece 0 |
| 6.8 s | first ore reaches belt end (24 slots x 5 ticks = 6 s transit) |
| ~7.5 s | press starts first job (2 ore) |
| ~9.6 s | first shell; inserter B carries it to the turret; turret fires at drone A |
| ~13 s | press input full (6); inserter A now limited to 1 ore/s; belt starts to compress |
| ~25-29 s | belt full; inserter D blocks holding 1 ore; drill buffer full (4); **drill OutputBlocked** (first visible congestion) |
| ~28 s | drone A destroyed by the **10th** shell (100 / 10) |
| ~38-40 s | turret ammo full (5); inserter B blocks holding 1 shell; inserter C now gets every shell |
| ~60 s | lab holds 10 shells; player (or test) sends Produce(lab, hardened_shells) |
| ~70 s | research complete; turret damage 20 |
| ~71 s | slot 1 sends MoveTo(drone B → (24, 14)); distance 7.3 from turret |
| ~75-80 s | drone B destroyed by the **5th** shell (100 / 20) |
| ~80-95 s | lab refills to 10; press output full (4); press job OutputBlocked; whole chain static |

Who acts: slot 0 (browser) sends one Produce at the lab; slot 1 (second browser tab, or the
existing headless client with `--move X,Y --move-unit ID --move-after-ticks N`, see
`CLAUDE.md` client options) moves drone B. A headless test drives the same commands.

### 2.3 Observable end state (exact oracle, timing-independent)

Measured effect of research: **shells fired at drone A = 10; shells fired at drone B = 5.**
Damage per shot before the completion tick = 10, from the completion tick = 20.

The final jam is a fixed point. Its counts do not depend on timing, only on capacities:

| Location | Final count |
| --- | --- |
| turret ammo | 5 shell |
| inserter B hand, inserter C hand | 1 + 1 shell |
| lab input | 10 shell (unused; see 6) |
| press output | 4 shell |
| press head job | OutputBlocked, 2 ore charged |
| press input | 6 ore |
| inserter A hand, inserter D hand | 1 + 1 ore |
| belt | 24 ore (6 pieces x 4 slots, if the representation packs to full slots) |
| drill buffer | 4 ore |
| shells produced | 10 + 5 + 10 (research) + 5 + 2 + 10 + 4 = **46** |
| ore consumed | 46 x 2 + 2 = **94** |
| ore extracted | 94 + 6 + 2 + 24 + 4 = **130** |
| node remaining | 300 − 130 = **170** |

If the engineer picks a belt representation that does not pack 4 items per piece, replace 24
by its declared belt capacity; the equations still hold.

## 3. Congestion, interruption and cancellation cases

### 3.0 Conserved quantities (check after every committed operation and every tick)

- **O (ore):** `node_initial = node_remaining + drill_buffer + Σ inserter hands(ore) + belt(ore)
  + press_input.on_hand + ore_charged_net + ore_destroyed`, where `ore_charged_net = Σ charged
  − Σ refunded`.
- **S (shell):** `shells_completed = press_output + Σ hands(shell) + belt(shell) + turret_ammo +
  lab_input.on_hand + shells_fired + shells_research_net + shells_destroyed`.
- **D (damage):** for each drone, `health_initial − health_now = min(health_initial, Σ damage of
  shots at it)`; exactly one Destroyed event at health 0; zero shots at it after that tick.
- **R (reservation):** `0 ≤ reserved ≤ on_hand` on every inventory (M2D:214); a shell or ore is
  picked by at most one inserter (S:543).
- `*_destroyed` is a ledger written by the removal policy. Proposed policy for M3: **Destroy
  with ledger** (items on a removed piece or in a removed inserter hand are deleted and
  counted in a Destroyed event). Spill to ground needs world item piles: cut. **needs decision**.

### 3.1 Cases

| # | Case | Trigger | Invariant | Conserved |
| --- | --- | --- | --- | --- |
| C1 | Belt full → drill stops | default rates (drill 2/s > press 1 ore/s) | drill shows OutputBlocked; extraction n = min(rate, node, free buffer) (M2D:278); belt slots never exceed capacity | O |
| C2 | Press output blocked | drone A dead, turret full, lab full or reserved | job holds elapsed = duration, no recharge, no duplicate output (M2D:321-323); exactly one shell when space returns | O, S |
| C3 | Backpressure chain | C2 continues | each upstream stage stops in order: press input full → inserter A holds 1 → belt full → inserter D holds 1 → drill blocked | O, S |
| C4 | Cancel press job mid-run | CancelProduction(press, job) while Running | refund floor(2 x 1/1) = 2 ore to press input; if input has < 2 free, reject RefundDestinationFull and change nothing (M2D:350-353); auto-repeat restarts next tick if ≥ 2 ore and enabled | O |
| C5 | Cancel press job while OutputBlocked | same, stage OutputBlocked | refund 2 ore; no shell appears; never refund twice (M2D:348) | O, S |
| C6 | Cancel research mid-run | CancelProduction(lab, job) while Running | refund floor(10 x 1/2) = 5 shells into lab; reject if lab lacks room (inserter C refills the lab during research, so a late cancel can reject); tech stays unresearched; damage stays 10 | S |
| C7 | Cancel research while queued | Produce then Cancel in one batch (M2D:356) | all reservations released; no charge | S, R |
| C8 | Pause and resume | SetEnabled(x, false) then true for drill, inserter, press, lab, turret | no extraction, transfer, charge, refund, output or shot while paused; progress and hand item held; resume continues from the same progress | O, S |
| C9 | Remove belt piece with items | Remove(piece 3) | items on piece 3 → destroyed ledger; pieces 0-2 end at piece 2 and stop; pieces 4-5 drain into inserter A; nothing re-creates piece 3 | O |
| C10 | Remove inserter holding an item | Remove(inserter A) mid-swing | hand item → destroyed ledger; press starves visibly (reason) | O |
| C11 | Ore depleted | fixture node remaining 21 | drill idles Depleted; node stays 0 (S:542); downstream drains; 10 shells made; 1 ore stays in press input with reason "insufficient input" | O |
| C12 | Two inserters, one source | B and C both pick from press output | each shell taken once; tie-break lowest UnitId per tick (engine tie-break, like M2D:257-258) | S, R |
| C13 | Turret ammo starvation | default supply 0.5 shell/s < fire 1/s | turret shows OutOfAmmo; shots ≤ shells delivered; no shot without a shell | S, D |
| C14 | Overkill and destruction | last shot on drone | shell consumed; surplus damage not carried; one Destroyed event; turret retargets next eligible tick | S, D |
| C15 | Research completes mid-volley | variant: drone B in range during research | shots before completion tick deal 10, from completion tick deal 20 | D |
| C16 | Target leaves range | MoveTo drone out of range mid-volley | turret stops, keeps ammo | S |

Each case needs a deliberate defect that turns it red (M2 practice, M2D:535-552): for example
drop an item on a removed piece without a ledger entry (O fails); let two inserters take one
shell (S fails); recharge on blocked retry (O fails); apply damage without ammo (S fails).

## 4. Engine mechanism versus mod policy

| Capability | Engine mechanism | Mod policy (forge data) | Exists? |
| --- | --- | --- | --- |
| Item resources | resource registry, Item kind | ore, shell | yes (M2D:103-104) |
| Finite node | extraction transaction, never refills | remaining, radius | yes (M2D:278-281) |
| Static extractor (drill) | extract into own inventory at a rate, block when full | rate, range, buffer cap | **gap** (gatherer needs movement, M2D:158) |
| Belt transport | compact chain, slot occupancy, advance, backpressure, split on removal | speed, slots, length, accepted items, facing | **gap** (S:414-416) |
| Inserter | pick/drop between inventory and belt endpoints, hand, cycle, block with item held | reach, cycle, stack, filter | **gap**; M2 trusted InventoryTransfer (M2D:235) can be the atomic step |
| Item-output recipe | job with input/output inventories, auto-repeat, OutputBlocked hold | inputs, outputs, duration, refunds, machine list | **partial** (M2 jobs; output must be a unit, M2D:166) |
| Weapon with ammo | target query, cooldown timer, atomic shot = consume ammo + damage event | range, cooldown, damage, ammo item and amount, target filter | **gap** |
| Health and destruction | health component, damage event, Destroyed event, despawn | health values | **gap** |
| Hostility filter | target query with a filter | "other slot" for forge | **gap**; must be data, not an engine rule (S:163 diplomacy is mod policy) |
| Research job | reuse producer queue with ProducerInventory cost; output = technology state | cost, duration, lab, refunds, prerequisites | **partial** (S:410 shared primitives) |
| Modifier | apply a numeric modifier to a stat of named definitions for a slot | +10 damage on forge:turret | **gap** (S:161) |
| Removal | delete entity after contents policy | Destroy with ledger | **gap** (M2D:306-308) |
| Pause | enabled flag checked by each machine phase | which entities allow it | **gap** |
| Tie-break between inserters | ascending UnitId per tick | none in M3 | engine integrity; fairness Open (S:416) |

Private-shortcut risks (each is a finding if the engine does it):

1. The turret's ammo item must be data (`ammo: forge:shell, per_shot: 1`). An engine literal
   "ammo" or "shell" is a shortcut. The derived E1 scan (M2D:543) must include forge tokens.
2. "Hostile = other slot" must be a mod-declared filter, not a built-in rule.
3. The research effect must target definitions by id, not a built-in "turret" kind.
4. The removal contents policy must be declared, not an accidental despawn.
5. Run the E2 renamed-clone check (M2D:544) on forge: same commands → same trace.

No per-item or per-unit script is needed: every rule above is definition plus composition (S:179-183).

## 5. Scope cuts

Out of M3: power and network quantities; player construction; junctions, splitters, lanes,
belt turns, underground belts; inserter filters (field kept, empty); multiple recipes and
recipe selection; multiple turret or weapon types; projectiles, armor, damage types, turret
rotation; ground item piles; consume-over-time payment; research graph beyond one node,
repeatable research, team sharing; fog and visibility; terrain and geology; hauling (S:420);
bots; victory; balances in forge; removal of machines that hold inventories (Remove rejects
press, turret, lab, drill with NotRemovable).

If the milestone is too large, cut in this order:
1. `SetEnabled` (C8); interruption stays covered by removal, depletion and target loss.
2. Drone B and MoveTo; measure the research effect by the turret damage view and shots per
   kill on a third fixture target instead.
3. Browser belt rendering of moving items; show per-piece counts and machine stage text.
4. `Remove` as a player command; keep it as a trusted headless scenario facility (like
   InventoryTransfer, M2D:235-237).

Must not cut: compact belt with backpressure; item-output recipe with OutputBlocked;
inserter with held item and blocking; atomic shot (ammo + damage) and destruction; research
completion that changes a stat; ledgers O/S/D/R with red plants; the removal case (headless
at least); the no-content and renamed-clone checks.

## 6. Loop walk and player view

Arrows (capability or gap): node→drill: extractor (gap). drill→belt: inserter (gap).
belt: compact transport (gap). belt→press: inserter (gap). press: item recipe (partial).
press→turret: inserter (gap). turret→drone: weapon + health (gap). press→lab: inserter (gap).
lab→tech: research job (partial). tech→turret: modifier (gap). drone→destroyed: Destroyed
event (gap). Every arrow has a named capability; none needs a script.

First minute: the player watches the chain fill; first fire at ~10 s; first loss (enemy) at
~28 s. First choice: shells to defense or to research. With `SetEnabled`, the player can
pause inserter B to feed the lab early; without it, the lab only gets shells once the turret
is full. Dead ends: after the one research, the lab still absorbs 10 shells (sink with no
use); the press stops for good when both drones are dead. These are acceptable for a
demonstration and are not a game. Unclear-to-player risks: a rejected cancel
(RefundDestinationFull) looks like a bug; give the reason text. The playtest showed the
objective was unclear (`vault/Work Log/2026-10-27.md:86`): show one objective line from
package data and the blocked reason on each machine.

## 7. Decisions for the CEO

1. **Minimal combat in M3.** The ammunition turret needs health, an instant-hit ammo weapon,
   damage and destruction, which the M1 design put in M7 (M1 design:46). Recommendation: pull
   this minimal slice into M3; projectiles, armor and combat at scale stay in M7.
2. **Pre-placed layout, no player construction.** Recommendation: pre-placed map plus
   `Remove` (must) and `SetEnabled` (should). Player construction goes to the 16.2 playable
   follow-on.
3. **Removal contents policy.** Recommendation: Destroy with ledger, mod-declared, as the only
   M3 option. Alternatives: spill to ground (needs world item piles) or refund to the owner
   (no destination in a no-balance mod).

Also needs decision (lower priority): research payment, ReserveThenCharge reuse
(recommended) versus consume-over-time (S:406; deferred from M2,
`docs/research/m2/scope-producer.md:93`); research cancellation and duplicate-queue rules
(S:428 Open).
