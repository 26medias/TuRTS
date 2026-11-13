VERDICT=REPAIRED
## Replace: `## 3. What exists and what MP1 adds`
## 3. What exists and what MP1 adds

MC gave the engine: orders as content ids in an open envelope, group movement with slot pools, the HUD
widget set with mod layout, the client shell, control groups, key bindings with a settings menu, alerts,
and the pixels-only test harness.

The engine has none of the items below today. The technical-director checked each one on the MC tree
(branch `mp1-design`, code equal to the MC integration branch) on 2026-11-11:

| Item | Evidence |
| --- | --- |
| Construction | `UnitDef` has no constructor or placement field (`crates/turts_mod_api/src/schema.rs:333-367`); a production output must be mobile (`crates/turts_mod_runtime/src/content.rs:557`) |
| Power | `ResourceKind` is `Balance` or `Item` only (`schema.rs`, `pub enum ResourceKind`) |
| Armour and damage types | `WeaponDef` has `damage: u64` only (`schema.rs`, `pub struct WeaponDef`) |
| Teams | `SlotRelation` is `Other`, `Same`, `Any` (`schema.rs:245-249`); `SLOT_COUNT` is the constant 2 (`crates/turts_sim/src/slots.rs:7`) |
| A match end from mod data | the only stop reason is `TickExhausted` (`crates/turts_sim/src/economy/events.rs:183-198`) |
| A lobby | the server claims a slot at `Hello` (`crates/turts_server/src/session.rs:152`) |
| A result screen, rally, repair | no `SetRally` or `Repair` behaviour (`crates/turts_mod_api/src/orders.rs`, `pub enum Behaviour`) |
| A fire event on the wire | the sim stages `Fired` for the shooter's slot only (`crates/turts_sim/src/economy/combat.rs:393-402`); `turts_net_protocol/src` has no `Fired` (grep) |
| A unit heading on the wire | `UnitDelta` carries the position only (RE part 3) |
| One key for each build entry | the build menu uses the key of the producer's Produce order for every row (`crates/turts_client/src/widgets/sources.rs:391-393`) |
| An action scope for keys | `ActionDef` has `name` and `default` only (`crates/turts_mod_api/src/input.rs:10-14`); MCD:312 says actions have a scope |
| A start order on a map placement | `Placement` has `unit`, `owner_slot`, `pos`, `contents`, `facing` (`schema.rs:440-447`) |
| Idle acquisition | an idle unit fires only inside its weapon range (`crates/turts_sim/src/economy/combat.rs:297-339`) |
| A UnitId-to-Entity index | 78 `iter_entities()` calls in `crates/turts_sim/src` (grep) |
| Order records as changes | each delta carries the full owner order table (`crates/turts_server/src/export.rs:190-195`) |
| Version-table presence from the written file | presence comes from parsed values (`crates/turts_mod_runtime/src/content_mc_validation.rs:65-68`) |
| Checked reason and cursor keys | the typos `queue_ful` and `plcae` pack with exit 0 (mod-builder gate run) |

The M3 validation rules. S:723 and S:809 both say that MP1 lifts "three" M3 rules. MC already lifted one of
them, "weapons must be static", with the version-table row `weapon with movement`, since 0.2.3
(`crates/turts_mod_runtime/src/version_table.rs:52-56`). Two rules remain:
`crates/turts_mod_runtime/src/content_m3_validation.rs:374-376` ("a unit with health requires removal") and
`:420-424` ("a unit with a queue cannot be removed"). MP1 lifts them (X1, section 6.4). The text "three" in
S:723 and S:809 is stale after MC. The specification edit that corrects both lines is in the CEO list of
section 2 [rigour 12].

MP1 adds one engine API step (0.2.3 to 0.2.4, section 6.10) and one protocol step (4 to 5, section 6.8). Each
mechanism of section 6 is exposed to all mods through that API (S 4.3, S 5.2).
---
## Replace: `### 6.1 Construction (X4, X4b, X5)`
### 6.1 Construction (X4, X4b, X5)

Status: S:178 (4.3, Decided principle) puts placement, footprints, progress and occupancy in the engine, and
"worker-built versus instant placement" and costs in the mod. The design below follows decision D-E.

**One home for each job** [rigour 1] [mod H2]:

- A constructor is a unit with the `constructor` capability (6.10 row K1): `buildings` (the production entries
  that it can build), `concurrent_sites` (the number of open sites that it can have at one time),
  `queue_capacity` (the number of held orders), `progress` and `when_short`.
- The player places a building with a `Construct` order (content and point). The constructor's structure
  queue holds each `Construct` order until the engine places its site. The queue holds orders only. It never
  holds progress or charge.
- The engine places a site at once when the constructor has fewer than `concurrent_sites` open sites and the
  slot has fewer than `open_sites_per_slot` open sites (6.10 row K6). Otherwise the order waits in the queue,
  in order. The engine checks the site when the order arrives (a refusal goes to the player in the
  `OrderResult`, per unit, D65) and again when it places the site (a refusal then is an engine-order failure,
  section 6.5).
- The site is an entity. It owns its progress, its charge and its job. Its footprint blocks from the tick it
  appears. It can be attacked. It neither provides nor consumes supply (6.2). Its weapon, producer and
  constructor capabilities are off until it is complete.
- The constructor counts only its open sites. When a site completes, is cancelled or is destroyed, the
  constructor can place its next held order.
- A site does not need its constructor after it appears. A site whose constructor is destroyed continues
  (`Autonomous` progress).
- `constructor` is a work capability. The rule "at most one work capability"
  (`content_m3_validation.rs:418`) gets one exemption: `constructor` beside `producer`. A constructor beside any
  other work capability is still refused. So the command centre has two queues that run at the same time: the
  producer queue for units and the constructor queue for structures. MP1 adds no named producer queues
  (decision in the CEO list).

**Progress.** `progress: Autonomous` is the only MP1 form: the site progresses by itself. `NeedsBuilder(reach)`
is the named later variant (a worker must stand within `reach` of the site). The schema has it in 0.2.4, and the
packer refuses it with the text "reserved for a later engine API" (6.10 row K1). A worker mod (MB) then needs no
schema change.

**Free sites are a grief path** [net 5]. Under charge over time (6.4), a site at progress 0 has charged 0, and
it holds ground. Two limits close this: `concurrent_sites` for each constructor and `open_sites_per_slot` for
each slot. Both are mod data. `open_sites_per_slot` is required when a package has a constructor, so no
engine default hides it (6.10 row K6).

**Placement rules are data** (6.10 rows K2 and K3) [mod L10]:

- The site footprint is fully inside the map.
- If `placement.in_build_area` is true: the site centre is within `build_area` of the footprint edge of a
  completed own unit that declares `build_area`.
- No overlap with any footprint, mobile units included.
- Between two static footprints, the edge distance is at least `max(gap of the new building, gap of the other
  building)`. A mobile unit counts by overlap only, not in the gap: the gap exists so that mobiles can pass
  between buildings (6.9), and a unit that walks past must not block a site.
- The edge distance to a resource node is at least `placement.node_clearance`.
- `placement.gap` is a number in the package. The packer refuses a gap below the largest diameter of a unit
  with movement in the resolved package set, plus 0.2. For the proposal of GD part 1 (tank radius 0.9) the
  smallest legal gap is 2.0, so the GD value 0.5 is refused.
- Footprints stay circles (`schema.rs:28-30`).

**Refusal causes** (reason keys, validated, 6.10 row H1) [player H4]: `site_outside_map`, `site_out_of_area`
("Too far from your base"), `site_blocked` (an overlap: "Something is in the way"), `site_too_close` (only the
gap blocks: "Too close to a building"), `site_near_resource` ("Too close to ore"), `site_cap` (the slot is at
`open_sites_per_slot`), `queue_full`. The texts are mod content. The checks run in this order, and the first
failure is the cause.

**One pure function, with visibility** [net 6]. `turts_rules::placement::check(site, view, visible)` gives the
verdict and the cause. `visible` is the viewer's visibility predicate. In MP1 it is always true. When a hidden
unit blocks a site (M5), the verdict is still a refusal, but the cause is `site_blocked` with no unit id, so a
refusal never names a hidden blocker. The server calls the function on the authoritative world. The
interaction core calls the same function on the replica for the ghost colour, the reason at the pointer, and the
gap and build-area rings (UX 7.2). The client holds no second copy of a rule.

**What the client verdict can promise** [net 9]. The client sees positions that are one tick plus the latency
old. For a scene with no motion and no new site between the client frame and the server tick, the client
verdict equals the server verdict. When they differ, the player sees the pending site, then the server refusal
with its reason at the pointer and the error sound, and the credits do not change.

**Cancel** (X4b). `CancelConstruction` on a site cancels it. A cancel of a held order removes it and refunds 0,
because it charged 0. Refunds: 6.4.
---
## Replace: `### 6.2 Power (X6)`
### 6.2 Power (X6)

Power is one use of a generic supply mechanism. A second mod can use the same mechanism for a population cap
[mod M5]. Status: S:426 (7.1) names "network quantity" resources, distributed "according to the selected network
model"; S:802 (16.2, Decided, D55) is the frontier rule; S:177 puts population rules in the mod.

- **Resource kind.** `Supply(scope: Slot, counts: Completed)` (6.10 row S1). `scope: Slot` is one network for
  each slot; M6 adds `Network` scope for the utility networks of S:549, with no change to the unit fields.
- **Units.** `provides` and `consumes`: lists of (supply resource, amount) (6.10 row U5). The field name is not
  `power`, because `power` is the frontier resource id and the no-content scan would match it.
- **Counts.** `Completed`: a unit counts when it exists and is not a site. `Queued` (also count the outputs of
  queued jobs) is named in 0.2.4 and refused as reserved. MP1 builds `Completed` only.
- **Computation.** Once for each tick, at the start of the economy phase, the engine sums `provides` and
  `consumes` for each slot and supply resource. A slot is short of a resource when consumed is greater than
  provided. All effects in that tick use that one result. A removal in tick t takes effect at the computation of
  tick t+1.
- **Shortage effect for each capability.** Each capability states its own effect in `when_short` (6.10 row U6):
  `Rate(ratio)` (progress of a producer job or a site runs at the ratio), `Disable` (the weapon neither fires
  nor acquires), and `BlockStart` (no new job starts), which is named and refused as reserved. A capability
  with no `when_short` entry is not affected. So frontier writes `Rate(1/2)` on the producer and the
  constructor, and `Disable` on the turret weapon; the rifleman weapon has no entry and is not affected; the
  engineer's repair has no entry, so repair is not slowed (GD part 4).
- **No hard-coded power.** No "power" check exists in production, combat or construction code. The code reads
  the slot supply result and the capability's `when_short` rules. NC1 checks the content ids.
- **Shortage disable is not SetEnabled.** A weapon that is disabled by a shortage keeps its player `enabled`
  state, so recovery restores the state that the player set.
- **On the wire and in the HUD.** A `SupplyRecord{slot, resource, provided, consumed}` goes to the owner on change
  (6.8). The HUD source is `Supply(resource, Provided | Consumed)` and the predicate `SupplyShort(resource)` (6.10
  rows H7, H10). The alert triggers are `SupplyShort(resource)` and `SupplyRestored(resource)`, and the credits
  trigger is `InsufficientFunds(resource)` (6.10 row H4). No engine name uses a frontier word such as "low power"
  or "credits".
- **The population cap of a later mod.** It needs `counts: Queued` and `BlockStart`. Both have their names in
  0.2.4, so adding them later is additive and changes no field.
---
## Replace: `### 6.3 Damage, armour, targeting (X2)`
### 6.3 Damage, armour, targeting (X2)

Status: S:179 (4.3) puts target queries and damage events in the engine, and weapons, armour, damage types and
targeting restrictions in the mod. Decisions D-G and D-J apply.

**One damage formula.** The design (`max(min, floor(d x ratio) - flat)`) and GD:68 (`max(1, floor(d x percent /
100))`) disagree [mod H1]. This design takes the design form, because it holds the GD form as a case (flat 0,
minimum 1, ratio = percent/100) and also covers flat armour:

    base = weapon.damage + modifier          (the research modifier of today, combat.rs:371-374, checked add)
    raw  = floor(base x num / den)           (num/den: the cell of (weapon damage type, target armour class))
    hit  = max(minimum, raw saturating_sub armour.flat)
    applied = min(hit, health)               (as today, combat.rs:376)

- All arithmetic is integer, with a u128 intermediate for `base x num`. No float takes part.
- The reason is exact authoring, not an invariant: health and damage are already `u64` (`HealthDef.max`,
  `WeaponDef.damage`), and a float ratio gives a different value for the same data. Probe (this repair):
  `floor(100 x 0.29)` in floating point is 28; the integer form `100 x 29 / 100` is 29. Invariant 15 (S:581) is
  the same-build replay rule; any deterministic arithmetic meets it, so it is not cited for integers here
  [rigour 12].
- A package with a damage table must give one cell for each pair of damage type and armour class. A missing
  cell is a pack error, not a default (6.10 row C3) [mod M6].
- A package with no damage table keeps the damage of today (`hit = base`).
- `minimum` is on the damage table. `flat` is on the unit armour (6.10 rows C4, U1).
- The formula is one function in `turts_rules`, used by the sim and by the client tooltip.

**Acquire is on the unit** [mod M6]. `acquire: (range, leash)` belongs to the unit, not to the weapon: a leash
is a property of the unit's movement, and a later unit with two weapons has one leash. `Fired.weapon` is the
weapon index on the unit (0 in MP1, where a unit has one weapon) (6.8).

**Idle acquisition.** A unit with `acquire` that is idle, with a weapon that is ready and not disabled and has
no valid target, takes the nearest enemy (distance, then unit id) inside `acquire.range` that the visibility
predicate allows. It chases while its distance from its idle point is at most `leash`, and then it returns to the
idle point. An ordered target comes first (`combat.rs:323-326`). A unit without `acquire` keeps the behaviour of
today: it fires only inside its weapon range (`crates/turts_sim/src/economy/combat.rs:297-339`) [rigour 12, the
path].

**Retaliation and hold (decision D-G)** [rigour 5]:

- The engine records the attacker of each unit that takes damage in a tick (the lowest attacker unit id when
  several hit it in one tick).
- An idle unit with `acquire` targets its attacker if the attacker is inside its leash (the same chase rule as
  idle acquisition).
- A holding unit fires back at its attacker only when the attacker is inside its weapon range. A holding unit
  never moves.
- A unit with no weapon, with a disabled weapon, or with an order that is not Hold does not react.
- Retaliation applies only to units with `acquire`, so no 0.2.3 package changes behaviour (6.10 row U4).

**Acquisition cost (decision D-J)** [perf H3]:

- Candidates come from one grid for each relation: one grid for each team, built once for each tick, with the
  units of that team. A unit queries only the grids of the teams that are `Enemy` to it. Friends are never
  candidates.
- A query runs only when the weapon is ready and has no valid target, and at most once every
  `ACQUIRE_PERIOD_TICKS` ticks for each unit, staggered by unit id (a unit queries in tick t when
  `(t + unit_id) mod K = 0`). K is an engine constant, 4 at the MP1 tick rate (Baseline, recorded in
  `docs/measurements/mp1.md`; the tick rate stays Open, S 15).
- The grid cell is the smallest weapon range in the catalog (slice part A3).
- Today `choose_target` runs before the cooldown check (`combat.rs:201`, `:229`). The new path runs the query
  after the readiness check. Units without `acquire` keep the order of today, because the attack-move state needs
  acquisition during the cooldown (`combat.rs:224-228`).
- Row TGT1 asserts acquisition within K ticks of an enemy entering the range.

**Visibility predicate.** Every target query (acquisition, retaliation, attack validation, repair targets, the
placement check of 6.1) takes the viewer's visibility predicate. It is always true in MP1. M5 (fog) and MP2
(bots) use the same queries.

**Damage types and armour on the wire.** No new record: health is public (`export.rs:180`), and the client derives
a hit from the health drop. The tooltip shows the cell value from the catalog through `turts_rules`.
---
## Replace: `### 6.4 Payment, cancel, destruction (X7, X4b, X1)`
### 6.4 Payment, cancel, destruction (X7, X4b, X1)

**Payment over time (decision D-A)** [rigour 2]. S:801 (16.2, Decided, D55) says: "Pay credits, select a valid
site, then construction progresses over time". Decision D-A changes this order: the player selects a valid
site, and the construction progresses over time while it pays. This is a change of a Decided specification line.
It is in the CEO list of section 2, with the specification edit, and the CEO can overrule it. S:434 (7.2) names
"consume over time" as a valid production policy, so the mechanism itself is inside the specification.

`PaymentPolicy::ChargeOverTime` (6.10 row P1) applies to production jobs (units) and to sites (buildings):

- **Progress in integers.** A job has `done` and `total = duration_ticks x Q`. `duration_ticks` comes from the
  conversion of today (`try_ms_to_ticks`, `crates/turts_sim/src/boot_m3.rs:591`). `Q` is the least common
  multiple of every `Rate` denominator in the catalog, so each tick adds the whole number `Q x num / den` (`Q` at
  full rate). The packer refuses `Q` above 65,536 and a `total` that overflows u64.
- **Charge.** For each cost entry, the total charged at `done` is `floor(cost x done / total)`, computed with a
  u128 intermediate. `cost x done` is a product of two u64 values, so it always fits in u128, and the result is at
  most `cost`. Each tick charges the difference to the previous total [perf M3].
- **All costs or none.** For a job with more than one cost entry, a tick charges every difference or none. If
  any source cannot pay its difference, the tick charges nothing and the job does not advance. Its stage is
  `WaitingForFunds(resource)`, with the first resource in cost order that cannot pay. It resumes on its own.
  The balance never goes below 0.
- **Order inside a tick.** Jobs pay in ascending unit id of their owner (producer or site), then in queue order.
- **Proof run (this repair).** A model of these rules (scratch `td-repair/charge.py`, 20,000 cases, costs up
  to u64::MAX, five rates, random stalls) gave a charged sum equal to the cost in every case, and no negative
  difference.
- **Ledger events are coalesced.** A job writes one `Charged{job, resource, amount}` ledger event when it
  completes, when it is cancelled, and when it enters `WaitingForFunds`, with the sum since its previous event; it
  never writes one for each tick (400 events for one 20-second tank at 20 Hz is the cost this avoids) [perf M3].
  The job record holds its `charged` amount for each cost entry. The ledger rule of `turts_audit` uses it: in
  each record, the change of a balance equals the ledger events plus the negative change of `charged` over the
  jobs that pay from that balance. No MP1 behaviour adds a branch to a validator (TD part E1).

**Refunds.** A refund is `floor(charged x numerator / denominator)` for each cost entry, with a u128
intermediate, from the job's `cancel_refunds`:

| Case | Ratio | Note |
| --- | --- | --- |
| Cancel of a held order or a queued job | any | it charged 0, so the refund is 0 |
| Cancel of a running job or a site | `cancel_refunds.running` | |
| Cancel of a complete job that waits for a free exit | `cancel_refunds.output_blocked` | it charged the full cost |
| Destruction of the producer or of the site | `cancel_refunds.destroyed` (new, 6.10 row K5) | 0/1 when absent: a destroyed site refunds nothing (CON3) |

Each refund is one `Refunded{job, resource, amount}` ledger event.

**Destruction (X1).** It goes through the one death path (`crates/turts_sim/src/economy/transaction_m3.rs:354`,
called from `combat.rs:405`). In one captured operation it:

1. cancels the running and queued jobs of a destroyed producer and refunds each by `cancel_refunds.destroyed`;
2. refunds a destroyed site by the `cancel_refunds.destroyed` of its production entry;
3. drops the held `Construct` orders of a destroyed constructor (refund 0); its open sites continue (6.1);
4. releases every claim of the unit;
5. removes the footprint in the same tick (DES1).

The two lifted M3 rules (6.10 rows K7, K8): a unit with health and no `removal` is destroyed with its contents
(`ContentsPolicy::Destroy`); a unit with a queue (producer, machine, research or constructor) can be removed, with
the explicit job policy above (S:434: destruction policies are explicit, not side effects).
---
## Replace: `### 6.5 Repair and rally (X8, X9)`
### 6.5 Repair and rally (X8, X9)

**Repair.** S:798 (16.2, Decided): the engineer repairs buildings and vehicles. Fields: 6.10 row R1.

- A `repairer` unit has `rate_per_second`, `reach`, `cost` (a list of (resource, amount, per_health)), `targets`
  (a slot filter), `classes` (armour classes; empty means all) and `auto_range`.
- Cost is an integer ratio for each health point: `amount` for each `per_health` points. A cost of 0 is legal:
  `amount: 0`, or an empty `cost` list, gives free repair [mod L12].
- Heal for each tick: `floor(rate x t / tick_hz)` minus the same value at `t - 1`, where `t` counts the ticks of
  this repair order. Applied heal is at most `max - health`. Charge is by applied health: the total charged after
  `H` applied points is `floor(H x amount / per_health)`, and each tick charges the difference. A tick that cannot
  pay heals nothing, and `t` does not advance (stage `WaitingForFunds(resource)`). Several repairers on one
  target are independent, and the health cap stops all of them.
- Repair writes health only inside a transaction (`stage_health`) and charges as coalesced ledger events (6.4).
- The repairer approaches to `reach` of the target footprint edge through the one movement model; it adds no
  movement path (TD part E2).
- `repairer` is not a work capability: it has no queue, and a unit runs one order at a time, so a unit can have
  a weapon and a repairer. A later combat engineer then needs no rule change [mod L12].
- An idle repairer with `auto_range` repairs the nearest damaged target that its filter accepts inside that
  range, with the cadence and grid rule of 6.3.

**Rally.**

- `SetRally` on a producer stores a rally target: a point, a unit or a node. A `SetRally` on the producer itself
  clears it. The rally goes to the owner in the producer's order record (`UnitOrderRecord.rally`, 6.8).
- When the producer spawns a unit, the engine gives the new unit a follow order: the smart rule that the new unit
  would get for a right click on the rally target. The engine reads `orders/rules.ron` `rally` when it is present,
  else `smart` (6.10 row O7). So frontier can map the ground to attack-move for combat units and a node to gather
  for haulers, with no hauler special case.

**Engine orders** [net 8]. The follow order of a rally, the start order of a map placement (decision D-F, 6.10
row M3) and the placement of a held `Construct` order are engine orders:

- They go through `apply_order` (`crates/turts_sim/src/orders.rs:2610`) with the source `Engine`. They are not
  player envelopes; a fake player envelope is refused (TD part B5).
- They have no sequence number, cost no rate-limit token, and give no `OrderResult`. Today `OrderResult` is
  routed by (slot, seq) (`crates/turts_server/src/outbound.rs:26-31`); an engine order writes nothing into that
  list.
- A refusal shows in the unit's `last_failure` (`UnitOrderRecord.last_failure`,
  `crates/turts_net_protocol/src/messages.rs:136`). The client needs no order id: the follow order shows in
  `unit_orders.current`.
- The order log (`crates/turts_server/src/orderlog.rs`) records the source `Engine`.
- Engine orders are not inputs of the command log. A replay derives them again from the inputs (G9,
  invariant 15).
---
## Replace: `### 6.6 Relations and teams (X10)`
### 6.6 Relations and teams (X10)

Status: S:183 (4.3) puts players, teams and lifecycle in the engine; decision P2 (a): team = slot in MP1.

- `SlotRelation` (`schema.rs:245-249`) gains `Ally` and `Enemy` (6.10 row L1). Meanings:
    - `Same`: the own slot (it is the "Own" of GD:278, so no third new variant is needed).
    - `Ally`: a slot of the own team, the own slot included.
    - `Enemy`: a slot of another team.
    - `Other`: any other slot. It keeps its meaning. Code must not use `Other` to mean enemy (TD trap B8).
    - `Any`: every slot.
- In MP1 each slot is its own team, so `Ally` equals `Same` and `Enemy` equals `Other`. The frontier package
  writes `Enemy` on its weapons, so that the same package works when M5 adds teams of more than one slot.
- The team of each slot comes from the match configuration (6.7). Under the `Immediate` start policy, each slot
  is its own team.
- One function `turts_rules::relation(a, b, teams)` gives the relation. The target filters, the acquisition grids
  (6.3), retaliation, repair targets, the build area (6.1) and the victory check (6.7) use it.
- Export takes a viewer group (a team or a slot), so M5 can share vision inside a team. In MP1 the group is the
  slot.
- `Welcome` and `MatchStart` carry the team of each slot (6.8), so the client uses the same relation function.
---
## Replace: `### 6.8 Wire and API (X19)`
### 6.8 Wire and API (X19)

**One protocol step, 4 to 5** (`NET_PROTOCOL_VERSION`, `crates/turts_net_protocol/src/lib.rs:13`), for all of MP1.
Every record goes through the per-slot export (`crates/turts_server/src/export.rs:25`, `:106`). The visibility
filter is in one place: a per-viewer visible-unit set in export, used for units, positions, healths, sites and
effects. In MP1 the set holds every unit [net 6].

**The record list and the visibility class of each record** [net 6] [backend M5] [backend M6]:

| Record | Message and channel | Who gets it | Sent |
| --- | --- | --- | --- |
| Site existence and footprint: `UnitSnapshot.site: bool`; `sites_completed: Vec<UnitId>` | Snapshot, WorldDelta (state) | Public | at spawn and at completion |
| `SiteProgress{unit, done, total, per_tick, at_tick, stage, charged}` | WorldDelta | Owner | on change of rate or stage |
| Production job records, the same form as `SiteProgress` | WorldDelta | Owner | on change of rate or stage |
| `ConstructionQueue{constructor, orders: [(content, point)]}` | WorldDelta | Owner | on change |
| `SupplyRecord{slot, resource, provided, consumed}` | WorldDelta | Owner | on change |
| `WeaponRecord` with `ready_tick` in place of `cooldown` | WorldDelta | Owner (as today, `export.rs:174-177`) | on change |
| `UnitOrderRecord.rally: Option<RallyTarget>` | WorldDelta `unit_orders` | Owner (order record, `export.rs:96-100`) | on change |
| `unit_orders` changes and `orders_cleared: Vec<UnitId>` | WorldDelta | Owner | on change; the Snapshot keeps the full table |
| `effects`: `Fired{unit, target, weapon: u8}`, `Destroyed{unit}` | WorldDelta | each client that may see the unit (all clients in MP1) | at the tick |
| `effects`: `Credited{unit, resource, amount}` (a delivery or a refund at a unit, for the "+N" text) | WorldDelta | Owner | at the tick |
| `healths` | WorldDelta (as today) | Public in MP1 (`export.rs:180`) | on change |
| `LobbyState{revision, you, seats, settings, start}`; a seat is `{index, occupant: Open, Human{label} or Bot{content}, ready, colour}` | reliable | room members | on change |
| `Welcome` gains `teams`, `colours`, `match_id` | reliable | the joining player | once |
| `MatchStart{start_tick, teams, colours}` | reliable | all | once |
| `MatchState{phase, outcomes: [(slot, outcome, reason)], end_tick}` | reliable | Public | on change |
| `SlotConnection{slot, connected, since_tick}` | reliable | Public | on change |
| `MatchStats` | reliable | Public, and only after the end tick | once |
| New refusal reasons: the `site_*` keys of 6.1, `match_not_started`, `slot_defeated`, `match_over` | reliable `OrderResult` | the sender | for each envelope |
| Client to server: the order target `ContentAtPoint{content, point}`; `LobbyAction`; `Surrender` | reliable | the server | for each action |

- A seat never carries a `player_id`. The id is a bearer value (`session.rs:152-156` gives a held slot to any
  `Hello` with the id), and today only `Hello` carries it (`messages.rs:21`) [backend M5].
- Message order: with a lobby, `Hello`, then `LobbyState`, then `Welcome` and `Snapshot` at the match start;
  under `Immediate`, `Hello`, then `Welcome`, as today. The lifecycle and the claim boundary are in 6.7
  (decisions D-B, D-C, D-D) [backend M6] [net 1].
- `MatchState.outcomes` names the reason of each outcome (key units lost, surrender, disconnect, draw), so the
  result screen can show "Player 2 surrendered" (decision D-K) [player M9].
- `MatchStats` is in no message before the end tick. A wire check asserts it, because a pixels driver cannot see
  data that the client holds and does not show [net 10].

**`Fired`** [net 7] [perf M1] [mod M6]. `Fired{unit, target, weapon}` has the weapon index on the unit and no
string. It drops the `damage`, `ammo` and `applied` fields of the sim event (`economy/events.rs:85-92`), and it
leaves out any id that the viewer may not see. Measured by the network lens: 6 B with an index against 20 B with a
content id.

**Records with a start tick and a rate** [perf M1]. A site, job or weapon record carries a value at a tick
(`done` at `at_tick`, or `ready_tick`) and a rate (`per_tick`). It is sent only when the rate or the stage
changes, not each tick. The client evaluates it at the tick of its last applied delta, never later, so this is not
prediction. Measured by the performance lens: `WeaponRecord.cooldown` changes every tick, about 500 records for
each tick at 1,000 units in combat.

**`unit_orders` as changes** [net 4] [perf M1]. Today each delta carries the full owner table
(`export.rs:190-195`); it is 64 to 83 % of the delta (network lens) and 77 % of the moving delta at 1,000 units
(performance lens). In protocol 5:

- A delta carries the complete record of each unit whose record changed, from the writer's change list (section
  9), plus `orders_cleared` for each unit whose record left the view.
- A record leaves the view when the sim gives no record for the unit: `views.rs` returns none for a unit without
  `UnitOrders` (`crates/turts_sim/src/economy/views.rs:558-569`). Today the client clears such a unit only because
  each delta replaces the whole table (`crates/turts_client/src/replica.rs:523-525`).
- The Snapshot keeps the full table. The replica merges deltas in chain order only.
- Row ORDW1 (section 12) plants the trap.

**Effects are never replayed.** Effects ride the unreliable state channel
(`crates/turts_net_lightyear/src/protocol_plugin.rs:20-25`). After a resync the client applies the kept deltas in
one commit. The client presents an effect only from a delta that it applies in normal order; a delta that it
applies in a resync batch gives no effect. Nothing in the state depends on an effect: removal and health are the
state.

**The rate-limit bucket runs first for every envelope** [net 5]. Today a `BadSequence` refusal and
`production_precheck` come before the bucket (`crates/turts_server/src/session.rs:403-418`, bucket at `:419-424`),
so a refused envelope costs no token. In protocol 5 the bucket runs first. No new bucket class. Engine orders
(6.5) do not pass admission.

**No client prediction** [net 11]. A grep finds no prediction in the client; progress comes from replicated
values. The replica is the only source of credits, progress, supply and outcomes (D18). A bar never goes past the
value of the last applied delta. The pending site is the one predicted object: it shows from the click until the
reliable `OrderResult`.

**One mod API step, 0.2.3 to 0.2.4** (`ENGINE_API_VERSION`, `crates/turts_mod_api/src/lib.rs:52`), additive. The
full table is section 6.10.

**Lane 0 freezes the shapes first** (section 10): every schema field, version-table row and wire record of 6.8
and 6.10, with the writers empty. A 0.2.4 feature that its lane has not built yet is refused by the packer from
one list, `UNBUILT_FEATURES`, with the text "`<feature>` is not built in this engine build". Each feature lane
removes its own line. So the feature lanes do not all edit `schema.rs`, `version_table.rs` and `messages.rs`.
---
## Replace: `### 6.9 Units and buildings in the way (decision P9)`
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

1. Engine (small, generic, stays valid at M4): the approach point of a receiver or a node is the nearest ring
   point that no static footprint blocks. About 40 lines in one file with one test (estimate of the probe,
   NOT TRACED). This changes a behaviour that has no field. It applies to a package of edition 0.2.4 only
   (6.10 row T1), so the frozen M2 stream (package `base`, `^0.2.3`) and every MC package keep their values.
2. Mod placement rule, as data: `placement.gap`, at least the largest mobile diameter plus 0.2 (2.0 for the tank of
   the proposal); the packer refuses a smaller gap (6.1, 6.10 row K2). The cost: bases are wider, and a player
   cannot build a wall.

**Entry condition from the performance lens** [perf H4]. Slice part A3 (two neighbour grids) is an entry condition
for any MP1 content with a footprint above the mobile radius. The command centre (2.5), the power station (1.5)
and the turret (1.0) are above it (infantry 0.4, tank 0.9). Measured by the performance lens on a loaded host
(ratios hold, absolute values about 2x high): one static unit of radius 2.5 raised the tick of 5,000 movers from
42.7 to 195.5 ms (4.6x), and of 1,000 movers from 6.9 to 11.0 ms. The slice plan already puts A3 "before MP1 content
adds a command centre or a long-range tank" (slice plan section 5, row E3). So no frontier package with these
footprints merges, and no MP1 measurement uses one, before A3 merges (section 10).

Recorded limit, as MC ruling O14: MP1 has no route round a group of obstacles. A route over the static
footprints is the first step of M4 navigation (the M4 design, `2026-10-28-m4-terrain-design.md:178-191`), not
of M4b. A `Blocked` unit in physical escape can walk up to 1.154 into a footprint (MC design 6.5 permits it);
a player can see it; it stays until M4b.

Decision P9 for the CEO: accept these two measures (recommended), or bring the M4 route step into MP1 (400 to
700 lines in 5 to 7 files, estimate of the probe; it moves the first match later).
---
## New section, after `### 6.9 Units and buildings in the way (decision P9)`: `### 6.10 Public mod API 0.2.4`
### 6.10 Public mod API 0.2.4

Status: D15 (Baseline): mods target a versioned engine API, never Bevy entities, schedules or renderer details.
S:207-211 (5.2) and S 5.5: manifests declare engine API compatibility. Decision D-I: this table is the full step
from 0.2.3 to 0.2.4. Lane 0 builds it; gate 1 checks it [mod H1]. Every type below is a number, a string, a
content id, a list or an enum of these. No Bevy type crosses the boundary.

**Conventions for every row.**

- Content references are qualified ids (`frontier:tank`). Local ids follow the existing local id rule.
- Error text form, as today: `<package>/<file>: <field path>: <message>` (`ModError` with package, file, field;
  for example `frontier/units/command_center.ron: removal: a unit with a queue cannot be removed`).
- Every name lookup (content reference, damage type, armour class, reason key, cursor context, state key, action
  name, model node) that fails adds the list of allowed names and the hint "; did you mean `<name>`?" for the
  nearest name at edit distance 2 or less, with the rule of `crates/turts_mod_runtime/src/parse.rs:23-29`. Today a
  map unit reference has no list and no hint [mod M7].
- A new struct field is optional in RON (`#[serde(default)]`). The required fields of today stay required.
- `Ratio` is `(numerator: u32, denominator: u32)`, the shape of `RefundRatio` today (`schema.rs:404-407`);
  `denominator` is above 0.
- Each new export group and each new id kind is added to the no-content scan kind list
  (`crates/turts_checks/tests/support/mod.rs:86-100`), so NC1 can plant one id of each new kind [mod H3].

**Version-table detection** (decision D-I) [mod M4]:

- RAW: the field is written in the source file. The packer parses each file a second time as `ron::Value` and walks
  its keys. Probe (this repair, ron 0.12.2, the workspace version, `Cargo.toml:15`): `ron::Value` keeps the keys of
  `armour: None` and `orders: []`, and it drops enum variant names (a unit variant becomes `Unit`). So RAW finds
  fields, not variants. RAW applies to every row, the 0.2.2 and 0.2.3 rows included; today `^0.2.2` with
  `orders: []` packs, and a 0.2.2 engine refuses that file (mod-builder gate run). No repo package changes: all
  of them require `^0.2.3` (`mods/*/manifest.ron`).
- VAR: the parsed value holds the new variant. Rule: no default produces a new variant, so a parsed variant is a
  written variant.
- COMBO: a combination of written fields on one parsed unit, as the `weapon with movement` row of today
  (`version_table.rs:52-56`).
- ED (edition): the root package's edition is the lowest released API that its `engine_api` requirement admits.
  An ED row selects a behaviour for edition 0.2.4 and refuses nothing. A `^0.2.3` package keeps the 0.2.3
  behaviour.
- A package with no 0.2.4 field keeps the 0.2.3 behaviour. The changed behaviours with no field are listed in rows
  T1 to T3; idle acquisition and retaliation (TGT2) are not among them, because they need the `acquire` field
  (row U4).
- VAL1 needs one red package for each RAW row (with an explicit `None` or `[]`), for each VAR row and for each COMBO
  row, and one `^0.2.3` and `^0.2.4` pair for each ED row.

**Exports**

| # | File | Field | Type | Default when absent | Validation | Error text | Version row and detection |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A1 | `manifest.ron` | `exports.combat` | list of paths | `[]` | each file exists and parses; one damage table in the resolved package set | `frontier/manifest.ron: exports.combat[1]: a match has one damage table; combat/damage.ron already declares it` | `exports.combat` since 0.2.4, RAW |
| A2 | `manifest.ron` | `exports.match` | list of paths | `[]`: no victory clause, start policy `Immediate`, no site cap | at most one file in the resolved package set; the root Game package owns it | `frontier/manifest.ron: exports.match[1]: a match has one match file` | `exports.match`, RAW |
| A3 | `manifest.ron` | `exports.visual_effects` | list of paths | `[]` | each file exists and parses | as A1 | `exports.visual_effects`, RAW |

**Damage table** (file of `exports.combat`, type `CombatDef`)

| # | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- |
| C1 | `damage_types` | list of local ids | required | at least 1; unique | `frontier/combat/damage.ron: damage_types[2]: duplicate damage type rocket` | covered by A1 |
| C2 | `armour_classes` | list of local ids | required | at least 1; unique | as C1 | covered by A1 |
| C3 | `cells` | list of `(damage: ContentRef, armour: ContentRef, ratio: Ratio)` | required | each reference resolves; each pair of type and class has exactly one cell | `frontier/combat/damage.ron: cells: no cell for damage frontier:rocket and armour frontier:structure`; `cells[4].damage: unknown damage type frontier:rockt; did you mean frontier:rocket?`; `cells[7]: second cell for frontier:bullet and frontier:vehicle` | covered by A1 |
| C4 | `minimum` | u64 | required | none | parser text `missing field minimum` | covered by A1 |

**Unit, weapon and supply fields** (files of `exports.units` and `exports.resources`)

| # | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- |
| U1 | `armour` | `Option<(class: ContentRef, flat: u64)>`; `flat` defaults to 0 | `None`: no class | class resolves; required for a unit with `health` when the package has a damage table | `frontier/units/tank.ron: armour: a unit with health needs an armour class, because the package declares a damage table` | `armour`, RAW |
| U2 | `weapon.damage_type` | `Option<ContentRef>` | `None`: the damage of today | resolves; required when the package has a damage table | `frontier/units/rifleman.ron: weapon.damage_type: unknown damage type frontier:bulet; did you mean frontier:bullet?` | `weapon.damage_type`, RAW |
| U3 | `weapon.damage` (exists) | u64 | required | at least 1 (today 0 packs) | `frontier/units/rifleman.ron: weapon.damage: must be positive` | T3, ED |
| U4 | `acquire` | `Option<(range: f32, leash: f32)>` | `None`: the behaviour of today (fire only inside weapon range; no retaliation) | needs `weapon`; `range` finite and at least the weapon range; `leash` finite and at least 0 | `frontier/units/rifleman.ron: acquire.range: 4.0 is below the weapon range 5.0`; `acquire: needs weapon` | `acquire`, RAW |
| U5 | `provides`, `consumes` | list of `(resource: ContentRef, amount: u64)` | `[]` | resource is a `Supply` resource; amount above 0; one entry for each resource in each list | `frontier/units/power_station.ron: provides[0].resource: frontier:credits is not a Supply resource` | `provides`, `consumes`, RAW |
| U6 | `weapon.when_short`, `producer.when_short`, `constructor.when_short` | list of `(resource: ContentRef, effect: Rate(Ratio) or Disable or BlockStart)` | `[]`: not affected | `Supply` resource; `Rate` with 0 < numerator <= denominator, on producer and constructor only; `Disable` on weapon only; `BlockStart` reserved | `frontier/units/guard_turret.ron: weapon.when_short[0].effect: Rate applies to producer and constructor only`; `...: BlockStart is reserved for a later engine API` | `*.when_short`, RAW |
| S1 | resource `kind` | new variant `Supply(scope: Slot, counts: Completed or Queued)` | none | `scope` is `Slot` (M6 adds `Network`); `counts: Queued` reserved | `frontier/resources/power.ron: kind.counts: Queued is reserved for a later engine API` | `kind: Supply`, VAR |

**Construction and payment**

| # | File | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- | --- |
| K1 | units | `constructor` | `Option<(buildings: [ContentRef], concurrent_sites: u16, queue_capacity: u16, progress: Autonomous or NeedsBuilder(reach: f32), when_short)>` | `None` | `buildings` are production ids whose output has `placement`; a production id is in at most one of `producer.recipes` and `constructor.buildings` in the package set; `concurrent_sites` >= 1; `queue_capacity` >= 1; `NeedsBuilder` reserved; beside `producer` allowed, beside any other work capability refused | `frontier/units/command_center.ron: constructor.buildings[0]: unknown production frontier:build_powr; did you mean frontier:build_power?`; `constructor.progress: NeedsBuilder is reserved for a later engine API`; `machine: a unit may have at most one work capability; constructor is allowed only beside producer` | `constructor`, RAW; `constructor beside producer`, COMBO |
| K2 | units | `placement` | `Option<(gap: f32, node_clearance: f32, in_build_area: bool, site_health: Ratio)>`; `site_health` defaults to 1/1 | `None`: the unit is not buildable | static units only; `gap` finite and at least the largest diameter of a unit with movement in the package set plus 0.2; `node_clearance` finite and at least 0; `site_health` numerator <= denominator | `frontier/units/power_station.ron: placement.gap: 0.5 is below 2.0, the largest mobile diameter (frontier:tank, 1.8) plus 0.2` | `placement`, RAW |
| K3 | units | `build_area` | `Option<f32>` | `None` | finite and above 0; if a `placement` has `in_build_area: true`, some unit has `build_area` | `frontier/units/power_station.ron: placement.in_build_area: no unit declares build_area, so no site can be placed` | `build_area`, RAW |
| K4 | production | `output_unit` of a building | existing | none | for a production id in `constructor.buildings`: the output is static and has `placement` (it lifts `content.rs:557` for these ids only) | `frontier/production/build_guard_turret.ron: output_unit: a building output needs placement` | `production with a static output`, COMBO |
| K5 | production | `cancel_refunds.destroyed` | `Ratio` | 0/1: no refund | numerator <= denominator | `frontier/production/build_guard_turret.ron: cancel_refunds.destroyed: the numerator is above the denominator` | `cancel_refunds.destroyed`, RAW |
| K6 | match file | `open_sites_per_slot` | u16 | required when a unit in the package set has `constructor` | at least 1 | `frontier/manifest.ron: exports.match: a package with a constructor needs open_sites_per_slot in its match file` | covered by A2 |
| K7 | units | lifted rule: `health` without `removal` | combination | death destroys the contents (`ContentsPolicy::Destroy`) | none | none | `health without removal`, COMBO |
| K8 | units | lifted rule: `removal` with a queue (producer, machine, research, constructor) | combination | destruction refunds by `cancel_refunds.destroyed` (6.4) | none | none | `removal with a queue`, COMBO |
| P1 | production | `payment: ChargeOverTime` | new variant of `PaymentPolicy` (`schema.rs:398`) | none | production entries only | `frontier/technologies/armour.ron: payment: ChargeOverTime is not supported for technologies` | `payment: ChargeOverTime`, VAR |

**Orders, repair and relations** (files of `exports.orders` and `exports.units`)

| # | File | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- | --- |
| O1 | order | `behaviour: Construct` | variant | none | target `ContentAtPoint`; each listed unit has `constructor` | `frontier/orders/order_construct.ron: units[0]: frontier:tank has no constructor` | `Construct`, VAR |
| O2 | order | `behaviour: CancelConstruction` | variant | none | target `None`; each listed unit has `placement` | as O1 | `CancelConstruction`, VAR |
| O3 | order | `behaviour: SetRally` | variant | none | target `Point`, `Unit`, `PointOrUnit` or `Node`; each listed unit has `producer` | as O1 | `SetRally`, VAR |
| O4 | order | `behaviour: Repair` | variant | none | target `Unit`; each listed unit has `repairer` | as O1 | `Repair`, VAR |
| O5 | order | `target: ContentAtPoint` | new variant of `TargetKind` | none | only with `Construct` | `frontier/orders/order_x.ron: target: ContentAtPoint needs behaviour Construct` | `ContentAtPoint`, VAR |
| O6 | `orders/rules.ron` | smart target `OwnDamaged`; `needs: Constructor` and `needs: Repairer` | new variants of `SmartTarget` and `Capability` | none | `OwnDamaged` with a `Repair` order; `Ground` with `needs: Producer` only for a `SetRally` order | `frontier/orders/rules.ron: smart[5].order: Ground with needs Producer needs a SetRally order` | each variant, VAR |
| O7 | `orders/rules.ron` | `rally` | `Option<[SmartOrder]>` | `None`: the follow order comes from `smart` | each order resolves | as the smart list of today | `rally`, RAW |
| O8 | production | `hotkey` | `Option<String>` (an action name) | `None`: the key of the producer's Produce order (today, `sources.rs:391-393`) | the action exists in the input map; the default keys of all actions are unique across all cards (the check is global today, UX F1) | `frontier/production/train_rifleman.ron: hotkey: unknown action key_rifelman; did you mean key_rifleman?` | `hotkey`, RAW |
| R1 | units | `repairer` | `Option<(rate_per_second: u64, reach: f32, cost: [(resource, amount: u64, per_health: u64)], targets: TargetFilter, classes: [ContentRef], auto_range: Option<f32>)>` | `None` | `rate_per_second` > 0; `reach` finite > 0; `per_health` > 0; `amount` >= 0 and `cost: []` legal (free repair); `classes` resolve to armour classes, `[]` means all; `auto_range` finite and at least `reach`; not a work capability | `frontier/units/engineer.ron: repairer.cost[0].per_health: must be positive` | `repairer`, RAW |
| L1 | any target filter | `slots: Ally`, `slots: Enemy` | new variants of `SlotRelation` | none | none | none | each variant, VAR |

**Match file and map** (file of `exports.match`, type `MatchDef`; semantics in 6.7)

| # | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- |
| M1 | `victory` | list of clauses: `LastTeamStanding(key_units: [ContentRef])`, `Surrender`, `DisconnectDefeat(after_seconds: u32)` | `[]`: the engine never ends the match (today) | `key_units` at least 1, resolve, have `health`; `after_seconds` > 0; each clause at most once | `frontier/match/rules.ron: victory[0].key_units[0]: unknown unit frontier:comand_center; did you mean frontier:command_center?` | covered by A2 |
| M2 | `start_policy` | `Immediate` or `AllReady(countdown_seconds: u32)` | `Immediate`, as today (claim at `Hello`) [backend H3] | `countdown_seconds` <= 60 | `frontier/match/rules.ron: start_policy.countdown_seconds: must be 60 or less` | covered by A2 |
| M3 | map `placements[].start_order` | `Option<(order: ContentRef, target: Node(u32) or Point(WorldPos))>` (decision D-F) [player H1] | `None` | the order exists and lists the unit; the target kind matches the order; the node index is below the number of map nodes | `frontier/maps/twin_fields.ron: placements[1].start_order.target: node index 40 is outside the 18 map nodes` | `start_order`, RAW |

- With no `DisconnectDefeat` clause, a disconnect changes nothing: D28 (Decided) keeps disconnect gameplay out of
  the engine; the engine gives only the connection facts (decision D-D) [mod L11] [backend H4].
- With no `Surrender` clause, the engine refuses `Surrender` with `not_applicable`.
- No lobby settings schema is in 0.2.4: MD12 is `later:M9`, and S:314 (Baseline) stays for M9 [rigour 12]. The lobby
  colour is an index into `slot_colours` (row H15). Team choice is not in MP1 (decision P2 a).

**Interface** (file of `exports.interface`; UX part 8)

| # | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- |
| H1 | `reasons` keys | strings | — | each key is in `REASON_KEYS`: the wire reason keys (`turts_net_protocol/src/messages.rs`, `as_str`), the client keys of today (`no_applicable_order`, `queue_disabled`, `already_researching`) and the new keys of 6.1 and 6.8 | `frontier/interface/hud.ron: reasons.queue_ful: unknown reason key; did you mean queue_full?` [mod M8] | T2, ED |
| H2 | `cursors` keys | strings | — | each key is in `CURSOR_CONTEXTS`: `default`, `select`, `move`, `attack`, `gather`, `forbidden`, `place`, `repair`, `rally` | `frontier/interface/hud.ron: cursors.plcae: unknown cursor context; did you mean place?` [mod M8] | T2, ED; the keys `place`, `repair`, `rally`, RAW |
| H3 | `states` | map from state key to text | `{}`: the engine English text | keys in `STATE_KEYS`: `waiting_for_funds`, `slowed`, `disabled`, `output_blocked`, `held`, and `activity.<name>` for each wire `Activity`; a text can hold `{resource}` | as H1 | `states`, RAW; keys T2, ED |
| H4 | `alerts[].trigger` | new variants `ConstructionComplete`, `SupplyShort(ContentRef)`, `SupplyRestored(ContentRef)`, `InsufficientFunds(ContentRef)`, `GathererIdle(f32)`, `SlotDisconnected` | none | `Supply*` names a `Supply` resource; `InsufficientFunds` names a `Balance` or `Item`; seconds finite > 0. A unit is ready: the existing `ProductionComplete` | `frontier/interface/hud.ron: alerts[2].trigger: frontier:ore is not a Supply resource` | each variant, VAR |
| H5 | `alerts[].units` | list of `ContentRef` | `[]`: all units | resolve | as M1 | `alerts.units`, RAW |
| H6 | `alerts[].rearm` | `Option<f32>` seconds | required for the state triggers `SupplyShort`, `InsufficientFunds`, `GathererIdle` | finite and at least `cooldown`. A state alert sounds when the state starts, and again only after the state stayed clear for `rearm` seconds [player H3] | `frontier/interface/hud.ron: alerts[3].rearm: a state trigger needs rearm` | `alerts.rearm`, RAW |
| H7 | `Value` and `Source` | new variants `Supply(ContentRef, Provided or Consumed)`, `ConstructionQueue`, `Sites`, `Notices`, `SelectionTypes`, `Outcome`, `MatchStats`, `MatchClock`, `Hint` | none | references resolve | as M1 | each variant, VAR (walk of the parsed widget tree) |
| H8 | `PrimaryField` | `Activity`, `Cargo`, `Armour` | none | none | none | each, VAR |
| H9 | `ItemField` | `State`, `Paid`, `Refund`, `Badge`, `SupplyChange`; `Time` shows the time at the current rate [player M7] | none | none | none | each, VAR |
| H10 | `Predicate` | `ItemHas(ItemField)`, `PrimaryHasValue(PrimaryField)`, `SupplyShort(ContentRef)`, `MatchOver`, `OutcomeIs(Victory or Defeat or Draw)`, `Placing` | none | references resolve | as M1 | each, VAR |
| H11 | `world.placement`, `world.progress_bar`, `world.cargo_bar`, `world.rally`, `world.float_text` | optional styles (colours `#RRGGBB`, sizes, font id; `placement` has `valid`, `invalid`, `gap_ring`, `area_ring`) [player H4] | `None`: the engine style (valid `#56B4E9`, invalid `#D55E00` with stripes, white rings) | colours parse; font exists | `frontier/interface/hud.ron: world.placement.invalid: not a #RRGGBB colour` | each, RAW |
| H12 | `world.health_bar.scale_to_footprint` | bool | `false` | none | none | RAW |
| H13 | `Button` `font`, `size` (X18) | `Option<String>`, `Option<u16>` | `None`: the tooltip font at 14 px (today, `widgets/mod.rs:627-633`) | font exists; size 8 to 64 | as the font check of today | `Button.font`, `Button.size`, RAW |
| H14 | `hints` | list of `(id, text, starts: MatchStarted or After(id) or event, ends: event)`; events: `Placed(ContentRef)`, `Completed(ContentRef)`, `Trained(ContentRef)`, `Delivered`, `SupplyShort(ContentRef)`, `SupplyRestored(ContentRef)`, `Ordered(ContentRef)` (decision D-F) | `[]` | ids unique; `After` names an earlier hint; references resolve. The `Hint` value shows the first hint that started and did not end | `frontier/interface/hud.ron: hints[2].starts: After(build_power) names no earlier hint` | `hints`, RAW |
| H15 | `slot_colours` (exists) | list of colours | required | with `AllReady`, at least as many different colours as slots; the server refuses a colour index that another seat holds | `frontier/interface/hud.ron: slot_colours: the lobby needs 2 different colours; it has 1` | none (existing field) |
| H16 | `effect_voices` | `Option<u16>` | `None`: 8 | 1 to 64; the cap of weapon sounds that play at one time (AUD2) | `frontier/interface/hud.ron: effect_voices: must be 1 to 64` | `effect_voices`, RAW |

**Assets, visuals and effects** (RE parts 1 and 4; semantics in 8.1 and 8.3)

| # | File | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- | --- |
| V1 | assets | `models` | list of `(id, path)` | `[]` | GLB version 2; no `uri`; no `extensionsRequired`; no skins, animations, cameras or lights; the triangle cap of the unit class and the byte caps (CEO list) | `frontier/assets/assets.ron: models[2]: 9,412 triangles exceed the 3,000 vehicle cap` | `models`, RAW |
| V2 | visuals | `model` | `Option<(asset, team_material: Option<String>, turret_node: Option<String>, muzzle_node: Option<String>)>` | `None`: the shape only | the asset exists; the named material and nodes exist in the GLB | `frontier/visuals/tank.ron: model.turret_node: the model has no node turet; did you mean turret?` | `model`, RAW |
| V3 | visual effects file | each effect | `(id, kind: Flash or Spark or Tracer or Burst or Smoke, colour, size: f32, ms: u32, sound: Option<String>)` | none | ids unique; sound exists | `frontier/fx/effects.ron: effects[1].sound: unknown sound asset fx_shot` | covered by A3 |
| V4 | visuals | `effects` | `Option<(fire, hit, death: Option<String>, damaged: Option<(effect, below: Ratio)>)>` | `None`: no effect | ids exist | as V3 | `effects`, RAW |

- S:247 (5.8, Baseline) keeps the exact supported glTF features Open. Rows V1 and V2 are the MP1 subset only,
  Proposed for MP1 [rigour 12]. The byte and triangle caps are a CEO decision (section 2, RE:24) [mod M9].
- RE part 6 (forward +Z, pivot at the ground centre, scale 0.8 to 1.2 of the footprint, the `team` material) is
  advice in section 8.1, not a pack check, except the named nodes and the material of row V2.

**Edition rows** (behaviours with no field)

| # | Behaviour | Edition 0.2.3 (and older) | Edition 0.2.4 |
| --- | --- | --- | --- |
| T1 | The approach point of a receiver or a node (6.9) | today | the nearest ring point that no static footprint blocks |
| T2 | Unknown `reasons`, `cursors` and `states` keys | pack (today) | refused (H1 to H3) |
| T3 | Weapon damage 0 | packs (today) | refused (U3) |

**What this table chose where the inputs disagree** [mod H1]:

| Point | Inputs | Chosen | Why |
| --- | --- | --- | --- |
| Damage formula | design `max(min, floor(d x ratio) - flat)`; GD `max(1, floor(d x percent / 100))` | the design form (6.3) | it holds the GD form as a case |
| Home of `acquire` | weapon (design); unit (GD) | unit | one leash for each unit |
| Power | `Network` and `UnitDef.power` (GD); `Capacity` (design) | `Supply(scope: Slot)` with `provides` and `consumes` | one name for power and a population cap; `power` is a frontier id |
| Construction order | `Place` (GD, UX); `Construct` (design) | `Construct` | it says what it does; `Placement` is already a map type name |
| Structure queue | `lanes` (GD); named producer queues (design 7.3) | the constructor queue (6.1); no named producer queues in 0.2.4 | one home for each job (D-E); CEO list |
| Payment name | `ConsumeOverTime` (GD); `ChargeOverTime` (design) | `ChargeOverTime`, stage `WaitingForFunds` | the design name |
| Relations | Enemy, Ally, Own (GD); Ally, Enemy (design) | `Ally`, `Enemy`; `Same` is "own" | no duplicate of `Same` |
| Home of victory, disconnect, start policy | map `match` (GD); not stated (design) | the `match` export file | the map refuses `match` today; one file for the match rules |
| Alert names | `LowPower`, `NoCredits` (UX) | `SupplyShort(res)`, `SupplyRestored(res)`, `InsufficientFunds(res)` | no frontier words in the engine |
| Unit ready | `UnitReady` or `ProductionComplete` (UX) | `ProductionComplete` (exists) | no duplicate |
| Lobby palette | `slot_colours` (UX); "mod palette" (design) | `slot_colours` index | one list of colours |

**Additive later, recorded now** [mod L12]: square footprints (a new `Footprint` variant), batches, a hold-a-point
victory clause, `counts: Queued`, `BlockStart`, `NeedsBuilder`, `Network` scope, named producer queues,
`consume_on_use` on a repairer. Each one is a new variant or an optional field, so a 0.2.4 package stays valid.
---
## Replace: `## 9. Architecture`
## 9. Architecture

**Crates.** The crate table after MP1 is TD part D, with these rows added or changed (each row is checked by
`scripts/boundary.sh`, and each new row has a fixture case that goes red):

| Crate | Logical module (S 4.1) | Change | Forbidden dependencies |
| --- | --- | --- | --- |
| `turts_rules` (new) | simulation core, used by presentation | pure rules: placement check with the visibility predicate, `relation`, the damage formula, the charge and heal difference functions, the shortage effect. Own f32 point type, because the lockfile has two glam versions (TD part B) | any `bevy*`, every workspace crate, `wgpu`, `winit`; host and wasm32 rows |
| `turts_mod_api` | mod runtime and API | the 0.2.4 schema of 6.10, `REASON_KEYS`, `CURSOR_CONTEXTS`, `STATE_KEYS` | any `bevy*` |
| `turts_mod_runtime` | mod runtime and API | RAW presence, the 0.2.4 rows, the edition, `UNBUILT_FEATURES` | any `bevy*` |
| `turts_net_protocol` | multiplayer | protocol 5 (6.8) | any `bevy*` |
| `turts_sim` | simulation core; production and logistics | construction, supply, damage, relations, match state, `UnitIndex` | `bevy_render`, `bevy_winit`, `bevy_pbr`, `turts_mod_runtime`, `turts_audit` |
| `turts_interaction` | presentation and player interface | placement mode; `Core::step` split by input kind; uses `turts_rules` | any `bevy*`, `turts_sim`, `wgpu`, `winit`, `turts_audit` |
| `turts_server` | headless host (owner: `backend-engineer`) | `room.rs`; match lifecycle (6.7) | `bevy_render`, `bevy_winit`, `bevy_pbr` |

The simulation stays headless: `turts_sim` and `turts_rules` have no rendering crate on host or wasm32.
`simulation-core-engineer` owns the headless-runnable core; `backend-engineer` owns the headless host.
`turts_rules` builds for wasm32 (`cargo check -p turts_rules --target wasm32-unknown-unknown`).

**One copy of each rule.** The sim and the interaction core call `turts_rules`; the client holds no second copy
of a placement, relation or damage rule. An instrument for CON2: a grep fails when placement arithmetic appears in
`turts_client` or `turts_interaction` outside calls to `turts_rules`. The placement arithmetic compares squared
distances and uses no square root, no trigonometry and no fused multiply-add, so it gives the same f32 result on
wasm32 and native.

**UnitId index in lane 0** [perf H2]. `UnitIndex`: a map from `UnitId` to `Entity`, with one insert at each spawn
and one remove at each removal. It is used for lookup only and is never iterated, so the tick order does not
depend on it. Lane 0 uses it at these sites before any feature lane adds a lookup (the performance lens measured a
scan at 5.0 µs at 1,000 entities and 25.9 µs at 5,000, against 0.026 µs for a map lookup):

1. `crates/turts_sim/src/economy/production.rs:282`: one scan for each active producer in each tick.
2. `crates/turts_sim/src/economy/transaction_m3.rs:635` (`find_unit_entity`), and `:524` (the ordered attackers of
   a removed unit: a reverse set, kept where an ordered target is set or cleared; if those writers are not in one
   place, lane 0 keeps the scan and records its cost).
3. `crates/turts_sim/src/economy/gather.rs:1206` (`find_unit`): about 10 calls for each gatherer in each tick.
4. `crates/turts_sim/src/economy/combat.rs:94` and 5. `crates/turts_sim/src/orders.rs:703`: a full scan and maps
   in each tick. They become one per-tick unit frame (positions, owners, teams), built once and used by combat,
   engagement and the acquisition grids of 6.3. This part belongs to the targeting lane and rebases on slice parts
   A2c and A3.
6. `crates/turts_server/src/export.rs:299` (`owner_of`): a linear scan for each record and slot (4.1 to 7.2 ms for
   each tick at 5,000 units and 2 slots, performance lens). `view.units` is sorted by unit id
   (`crates/turts_sim/src/world.rs:61`), so a binary search replaces it.
7. `crates/turts_sim/src/orders.rs:3056`, `:3085` (`validate_target`): once for each command, so not hot; they use
   the index because the index exists.

**String ids become indices at install** [perf M4]. Damage type, armour class, acquire, footprint and the supply
resources resolve to small integer indices when the catalog is installed. No tick or frame parses a content id.
Today `combat.rs:265` parses one for each weapon in each tick, and `crates/turts_client/src/visuals.rs:648` one for
each unit in each frame. The CON2 ghost reads a footprint list that is rebuilt for each applied delta, not for each
frame.

**New records enter the committed tick from their writer** [perf M5]. Each new record kind (sites, supply, job
progress, effects, order changes) enters `CommittedTick` (`crates/turts_sim/src/economy/events.rs:138`) from the
change list of the code that writes it. No new full view and no diff of the world for each tick (the idle tick
already costs 3.0 ms at 1,000 units, performance lens).

**The client debug state** [perf H1] [perf M2]. Decision: lane 0 of MP1, not the slice. The slice plan is gated,
and its client task S45 is a pure move; the debug state touches the harness bridges that every MP1 pixel row uses.

- Today `rebuild_debug_state` runs in `Last` for each frame (`crates/turts_client/src/lib.rs:147`), and the web
  client copies it for each frame (`crates/turts_client_web/src/lib.rs:166-170`, `:372`). Measured (native): 4.4
  ms and 148 KB at 400 units; 96.8 ms and 1.78 MB at 5,000 units.
- After lane 0 it is built only when the harness asks for it, at most 4 times each second. VIS3 records the frame
  time with the debug state off and on. Every MC browser row stays green (REG2).
- `health_trace` (`crates/turts_client/src/replica.rs:1182`) is pruned to a window. MP1 adds no field to the
  replica candidate.

**Behaviour handler table.** After the pure moves of the slice: a table in `turts_sim/src/orders/`, so a new
behaviour (Construct, CancelConstruction, SetRally, Repair) is one new file and one table line.

**The API bump.** `ENGINE_API_VERSION` 0.2.4 and `RELEASED_ENGINE_APIS` gain one entry
(`crates/turts_mod_api/src/lib.rs:52-55`). The literal "0.2.3" appears in `crates/turts_mod_api/src/lib.rs`,
`crates/turts_mod_runtime/src/version_table.rs` and `crates/turts_mod_runtime/tests/t2c_package_rejects.rs`
(grep); lane 0 checks each test that rewrites an `engine_api` text, so that no test turns into a no-op.

**Refused in any MP1 plan** (TD part E): a validator branch for an MP1 behaviour; a second movement path; a
default-mod rule name in engine code; a frozen-value change other than the one approved re-freeze; client
authority over placement or outcome; an Open item closed without a measurement or a decision (tick rate, numeric
budgets, transport, a scripting form).
---
## Replace: `## 10. Lanes and order`
## 10. Lanes and order

A dependency graph, not waves; a task starts when its inputs are merged (PR part 4). One worktree and one build
cache for each lane; reviewers in their own worktrees; fast checks for each task; the slow checks one time for each
merged tree. The GPU is the scarce resource: two headed clients for one match is tested in the first two days.

**Start points from the pre-MP1 slice** (slice plan section 5; slice design revision 6, section 9 item 15):

| Point | Slice tasks merged first | MP1 work that can start |
| --- | --- | --- |
| now | none | L-LOBBY (server room; it does not touch the sim) |
| E1 | S1, S2, S3, S45, D0p (the pure moves and the audit crate) | lane 0a; L-DMG, L-PWR, L-PAY, L-FX, L-MATCH, L-DES. Every sim write is inside a captured operation (`capture_operation`, `crates/turts_sim/src/economy/transaction.rs:105`). An edit of `economy/combat.rs` rebases on A2c and A3 |
| E2 | A1c (the replay validator is deleted) | lane 0b; L-CON, L-RLY, L-REP, L-TGT (they add orders) |
| A3 | A3 | L-CONTENT tests and measurements with the frontier footprints (6.9, perf H4) |

**Lanes:**

- **Lane 0a** (E1): the 0.2.4 schema of 6.10 with `UNBUILT_FEATURES`; RAW presence, the edition and every
  version-table row; the protocol 5 records of 6.8 with empty writers; `turts_rules` with its tests; `UnitIndex` and
  hot sites 1, 2, 6 and 7 of section 9; string ids to indices; the client debug state on request and the pruned
  health trace; the `Core::step` split.
- **Lane 0b** (E2): the behaviour handler table in `orders/`; hot site 3 (`gather.rs`, which rebases on slice
  parts A2b1 and A2b2).
- **Feature lanes:** L-DMG (X2 damage), L-PWR (X6), L-PAY (X7), L-FX (X14), L-MATCH (X11, backend), L-DES (X1), L-CON
  (X4, X4b, X5 server part), L-TGT (X2 targeting, hot sites 4 and 5), L-RLY (X9), L-REP (X8), L-LOBBY (X12, backend),
  L-UI (X5 ghost, X13, the HUD of section 7, X18), L-CONTENT (X15), L-PRES (X16, X17), and the instruments (X20).
- Each feature lane owns its own pixel rows; the final browser driver holds only the full match.

**Two stages, CEO order of 2026-11-11** (orchestrator brief; the vault note is NOT TRACED): first one thin
complete playable loop with simple shapes, which a person plays at a checkpoint; then the breadth.

**Stage 1: the thin loop.** A person builds power, harvests, trains, fights, wins and sees the result screen,
with simple shapes.

| Item | Stage 1 part |
| --- | --- |
| X19 | all of it, in lanes 0a and 0b |
| X1 | the two lifted rules and destruction |
| X2 | the damage table, armour and idle acquisition (retaliation, TGT2, is stage 2) |
| X4 | construction with held orders and sites (cancel, X4b, is stage 2) |
| X5 | the ghost with valid and invalid styles and the reason at the pointer |
| X6 | supply, the shortage effects, the power text |
| X7 | charge over time for jobs and sites |
| X10 | `Ally`, `Enemy`, team = slot |
| X11 | `LastTeamStanding`, defeat and draw (surrender and disconnect are stage 2) |
| X13 | the outcome and its reason (statistics are stage 2) |
| X14 | the `Fired` record and one tracer line for each shot; death by removal |
| X15 | the 8 types, Twin Fields, the credits and power HUD, the goal line and the hint list, the start gather order of decision D-F, simple shapes |
| X20 | the headless two-client match and the pixels driver of the stage 1 path |

Stage 1 runs with the start policy `Immediate`; the lobby lane can land earlier, but stage 1 does not wait for it.

**Checkpoint C1.** A person plays one full match in two browser windows with simple shapes: the hauler gathers
from tick 0, the person places a power station from the first hint, trains units, fights, destroys the other
command centre, and both windows show the outcome with its reason. Conditions: lanes 0a and 0b, the stage 1 lanes
and slice part A3 are merged; the stage 1 rows are green (VAL1 and NC1 for the stage 1 rows, DMG1, TGT1, CON1, CON2,
PWR1, PAY1, DES1, OBS1, MAT1 for victory and draw, GOAL1, GHO1 without the cancel part, HUD5, ORDW1). The person signs
off C1 in the vault. When C1 fails, stage 2 does not start until its cause is fixed.

**Stage 2: the breadth.** X4b, X8, X9, X12, the rest of X2 (TGT2), X11 (surrender, the disconnect clause,
statistics), X13 (statistics), the rest of X14, X16, X17, X18; then X21 (the play test), X22 (the final regression)
and X23 (the architecture audit).

The producer re-estimates section 11 and the critical path for the two stages.
---
## New row, after `TGT2`
| ORDW1 | `unit_orders` as changes (6.8). A plain move ends on a unit that then has no `UnitOrders`; the client drops the delta that clears it (T8 hook `--drop-delta-ticks`), then resyncs. After the resync the client shows no current order for the unit. With 200 idle units for 100 ticks, the `unit_orders` bytes of each delta are 0 | No `orders_cleared` list (the client keeps the stale order); or the full owner table in each delta | headless, two clients |
---
## Not done

- [rigour 4] CB05 `later:MP2` with no weak-cover list: it is a row of the Yardstick coverage section, not mine.
  With 6.3 (D-G), the row is `| CB05 | in | TGT2 |`.
- [rigour 6], [net 9], [net 10], [mod M4]: the plants for rows DMG1 (damage 100, cell 29/100: a float gives 28,
  the integer form 29), CON2 (a client copy with gap 1.8, plus the section 9 grep), VAL1 (one red package for each
  6.10 row, explicit `None` and `[]` included), RES1 (the wire check of 6.8), and the CON2 limit of 6.1. Section 12
  rows other than ORDW1 are not mine; the row owner copies these plants.
- [mod H3] NC1 row text: 6.10 adds the new kinds to the scan list; the bare-id rule for repo mods (the "units" probe)
  needs the NC1 owner and the modder guide.
- [mod M9] The asset brief of RE part 6 in section 8.1: not my section.
- [rigour 1] Rows PRD1 and section 7.3 still say "named queues"; with 6.1 they say "the unit queue of the producer
  and the structure queue of the constructor".
- 6.7 (backend) must use the field names of 6.10 rows M1 and M2 and the records of 6.8 (`MatchStart`,
  `MatchState`, `SlotConnection`, `LobbyState`), or the orchestrator reconciles the two texts.
- Player findings H2 (the hint list content), M6 (a role line), M8 and L11 (goals text): content and UX sections.
- NOT TRACED: the 40-line estimate of 6.9; the effect of the bucket-first rule on the MC row ORD6 (REG2 shows it).
- Runs of this repair: ron 0.12.2 `ron::Value` probe (scratch `td-ronprobe/`: keys kept for `None` and `[]`, variants become `Unit`); charge and float model (scratch `td-repair/charge.py`: 20,000 cases, 0 bad; float 28, integer 29); greps and reads of each cited line on `mp1-design` d729a97. The full text of this reply is also at /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/td-repair/out.md. Repository: no file changed (`git status --porcelain` empty in TuRTS-mp1). Memory: new `/home/julien/Projects/TuRTS/.claude/agent-memory/technical-director/project_mp1_repair.md`, an index line in `MEMORY.md`, and a correction in `project_mc_audit.md` (the version table has the `weapon with movement` row).
---
## Decisions for the CEO

1. Specification edits: S:801 under D-A ("select a valid site, then construction progresses over time while it
   pays"); S:723 and S:809 "three rules" to "two rules (MC lifted the third)". Recommendation: accept both.
2. No named producer queues in 0.2.4; the command centre has a producer queue and a constructor queue. (a) as
   written; (b) add `ProducerDef.queues` as well. Recommendation: (a); (b) is additive later.
3. Behaviours with no field (approach point, strict keys, damage 0): (a) edition by the lowest admitted API, as
   written; (b) apply to every package and take the change into the one re-freeze; (c) an opt-in field for each.
   Recommendation: (a), no frozen value moves.
4. Model caps (RE:24): 4 MiB for each GLB, 16 MiB in total, 1,500, 3,000 and 8,000 triangles. Proposed for MP1 only;
   S:247 stays Open. Recommendation: accept.
5. `repairer` is not a work capability; `Ally` includes the own slot; mobile units count in placement by overlap
   only; sites survive their constructor; `ACQUIRE_PERIOD_TICKS` is 4. Recommendation: accept each.
6. The debug-state fix goes into MP1 lane 0, not into the slice. Recommendation: accept.