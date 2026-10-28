# M3 industrial loop — design

Date: 2026-10-27. Revision 1. Status: **Gates 1 and 2 passed. The user confirmed calls K1 to K8 on 2026-10-27.** Ready for implementation.
Authors: orchestrator, from three brainstorm notes (see section 17). Implementer: a Codex agent.
Source of record for the product: `RTS-Engine-Project-Specifications.md` (S), version 0.7.
This design extends the M2 design (`docs/superpowers/specs/2026-10-26-m2-economy-design.md`, M2D).
It does not replace it.

## 1. Contract and status

M3 proves the section 14 demonstration (S:678): **mine → belt → factory → inserter → ammunition
turret → research upgrade**, with the acceptance statement "inputs and outputs remain consistent
under congestion, interruption, and cancellation".

M3 is a **graybox proof of the engine mechanisms**, driven by a new mod package `forge`. It is not
a playable game and not the 16.2 default mod. All forge numbers are placeholders. No number is a
balance decision.

### 1.1 Calls that the orchestrator made

The user asked the orchestrator to write the spec and plan and to delegate the implementation. The
user did not answer the questions below. Each is the orchestrator's call, based on the brainstorm
recommendations. **The user confirmed all eight on 2026-10-27 ("the eight calls are fine").**

| # | Call | Reason | Alternative |
| --- | --- | --- | --- |
| K1 | M3 includes a minimal combat slice: health, a hitscan weapon with ammunition, destruction. The M1 design (`2026-10-26-m1-foundation-design.md:46`) and M2D:68-70 assign combat to M7. No Decided row forbids the change, and S:672 says the milestones are "not a committed delivery schedule". | The ammunition turret in S:678 cannot exist without it. | Turret as an ammunition sink: drop section 8.1 to 8.3; the end state changes. |
| K2 | Machines are placed by the map. Players do not build. Players can `Remove` and `SetEnabled`. | Construction roughly doubles M3 and overlaps the open work for 16.2 (S:745). | Player construction: a separate milestone. |
| K3 | `Remove` destroys the contents of the removed unit and counts them. This is the only removal policy in M3. | Spill to ground needs world item piles. Refund needs a balance, and forge has none. | Later policies are added as new enum values. |
| K4 | Engine API goes to **0.2.1**, additive, with `#[serde(default)]` on every new field. Existing packages load unchanged. This refines two approved M2 rules: M2D:98 ("every list is required, even when empty") and M2D:509 ("no silent v0.1 defaults"). The refined rule applies to the new 0.2.1 fields only. The version scheme is still Open (S:708); semver is the M1 placeholder (`turts_mod_runtime/src/manifest.rs:8`). | Probe: `^0.2` accepts 0.2.1 and rejects 0.3.0; old RON loads with new defaults (research note F14, F15). | 0.3.0 with required keys: edits four manifests, fixtures and a 2,083-line generator for no M3 benefit. |
| K5 | Wire protocol goes from 2 to 3. | postcard is not self-describing; any field change breaks old decoders. | None. |
| K6 | Research uses the M2 `ReserveThenCharge` policy and the M2 production queue. The rules for repeat, duplicate and stacking are **M3 engine prototype limits**. They are fixed in engine code. They do **not** settle the Open policies in S:428. | Reuse. S:428 says to design these when a concrete game needs them. A package field cannot change these rules in M3. | Make each rule a `TechnologyDef` field that accepts one value only; or consume-over-time payment (deferred, as in M2). |
| K7 | An inserter picks only when the destination has room now. It does not reserve that room. | Items stay out of hands at a jam, and the inserter never picks an item the destination cannot take. | Pick without a check: hands hold items at a jam. |
| K8 | After a successful `CancelProduction` on a machine, an enabled machine with enough input starts a new job **in the same tick**. | Commands run before the production phase (M2D:242-244). To stop a machine, disable it first. | Next-tick restart: needs a new rule. |

### 1.2 Included

- Static extractor (drill) on a finite surface node.
- Compact belt with discrete slots and backpressure.
- Inserter with a hand that is an inventory.
- Item-output recipe machine (press) with blocked-output behavior.
- Ammunition turret: health, hitscan weapon, atomic shot, destruction.
- Research station (lab), one technology, one weapon-damage modifier.
- Commands `Remove` and `SetEnabled`.
- Package `forge`, API 0.2.1, protocol 3, owner-only replication of the new state.
- Client text panels for the new state and the controls that the demonstration needs.
- A conservation oracle, 17 scenario cases, a mutation sweep, and process and browser acceptance.

### 1.3 Excluded

Power and network quantities; player construction and placement; junctions, splitters, lanes,
belt turns; inserter filters beyond the inventory `accepts` list; more than one recipe per
machine; more than one turret or weapon type; projectiles, armor, damage types, turret rotation;
ground item piles; consume-over-time payment; research beyond one technology, repeatable research,
team sharing; hauling; fog and visibility; terrain and geology; bots; victory; balances in forge;
scripts. Combat at scale stays in M7. Item visibility to enemies waits for the fog design.

### 1.4 Decision IDs this design depends on

D04 and D05 (the official game and mods use the same public facilities: `forge` gets no private
facility); D07 (research, factories, belts, inserters are in scope); D14 (simple, local
simulation); D15 (Bevy-free versioned API); D18 (authoritative server); D35 (exact lockfiles:
`forge` must pack and lock like any package); D39 (no arbitrary gameplay caps: the slot, piece
and item limits in section 3.1 are prototype validation limits); D44 and D55 (the 16.2 default
game is not changed by M3); D60 (simulation, replication and visuals stay separate: belt items
are sim state; client panels read the replica only). The register runs D01 to D64. `CLAUDE.md`
says D01 to D59; that is out of date.

## 2. Verified baseline

The simulation-core seat checked these facts against commit `9e1987c`. Gate reviewers re-check
them. Full table with 21 facts: `docs/research/m3/engine-simulation.md` section 0.

| Fact | Evidence |
| --- | --- |
| Tick rate 20 Hz. Map is continuous f32, not a grid. | `crates/turts_sim/src/lib.rs:26`; `world.rs:35` |
| Unit ids are allocated one by one in map placement order. Ids are never reused. | `bootstrap.rs:152-264`; `ids.rs:33-54` |
| Tangent circles pass the boot overlap check. | `map.rs:180-192` |
| One inventory per unit. `Endpoint` has `Slot(u8)` and `Inventory(u32)` only. | `economy/ledger.rs:7-41` |
| Every transaction runs inside `capture_operation`. | `economy/transaction.rs:234-239` |
| M2 gather writes the node outside the transaction. M3 must not copy this. | `economy/gather.rs:571-585`, `:811-835` |
| `SimSet` = Advance, Commands, Movement, Gather, Production, Collect. | `plugin.rs:29-37`, `:69-74` |
| No despawn exists. `CommittedTick.removed` is always empty. | `plugin.rs:156` |
| The client already handles `removed`. | `turts_client/src/replica.rs:414-435` |
| The audit log fails any state change between captured operations, except the movement phase. | `turts_server/src/economylog.rs:579-711` |
| The M2 conservation oracle counts only inventory `on_hand` and node `remaining`. | `turts_pack/tests/common/accounting.rs:546-597` |
| The no-content scan (E1) matches production identifiers and string literals against mod local ids, as exact tokens. | `turts_checks/tests/support/mod.rs:458-548` |
| Durations convert at load: `ceil(duration_ms * tick_hz / 1000)`. | M2D:191 |

`NOT TRACED` (a task must test it): whether the M2 gather code tolerates a removed receiver
(`docs/research/m3/engine-simulation.md` section 5.4, row `EconomyOrders`).

## 3. Public content schema (API 0.2.1)

`ENGINE_API_VERSION = "0.2.1"`. New fields are additive and carry `#[serde(default)]`. The four
existing packages keep `engine_api: "^0.2"` and load unchanged. `forge` declares `^0.2.1`.
This refines two M2 rules (M2D:98 and M2D:509, see K4): "every list is required, even when empty"
applies to the M2 fields only.

| Struct | New fields (all optional or default) |
| --- | --- |
| `Exports` | `recipes: Vec<String>`, `technologies: Vec<String>` |
| `UnitDef` | `output_inventory`, `extractor`, `conveyor`, `inserter`, `machine`, `research`, `health`, `weapon`, `removal` |
| `Placement` | `facing: Option<Facing>` |
| `MapDef` | `belts: Vec<BeltLinePlacement>` |

```rust
enum Facing { PosX, NegX, PosY, NegY }
struct ExtractorDef { per_second: u64, interaction_range: f32 }
struct ConveyorDef { slots_per_piece: u8 /* 1..=16 */, slots_per_second: u64, accepts: Vec<ContentRef> }
struct InserterDef { reach: f32, swing_ms: u64, return_ms: u64 }   // hand = the unit's inventory
struct RunRefunds { running: RefundRatio, output_blocked: RefundRatio }
struct RecipeDef { id: String, name: String, inputs: Vec<ItemStack>, outputs: Vec<ItemStack>,
                   duration_ms: u64, cancel_refunds: RunRefunds }
struct MachineDef { recipe: ContentRef }                           // one recipe, auto-repeat
struct HealthDef { max: u64 }                                      // > 0
enum SlotRelation { Other, Same, Any }
struct TargetFilter { slots: SlotRelation }
struct AmmoDef { item: ContentRef, per_shot: u64 }
struct WeaponDef { range: f32, cooldown_ms: u64, damage: u64, ammo: Option<AmmoDef>, targets: TargetFilter }
enum ContentsPolicy { Destroy }
struct RemovalDef { by_command: bool, contents: ContentsPolicy }
enum WeaponStat { Damage }
enum EffectDef { WeaponModifier { unit: ContentRef, stat: WeaponStat, add: u64 } }
struct TechnologyDef { id: String, name: String, prerequisites: Vec<ContentRef>, duration_ms: u64,
                       costs: Vec<Cost>, payment: PaymentPolicy, cancel_refunds: CancelRefunds,
                       effects: Vec<EffectDef> }
struct ResearchStationDef { technologies: Vec<ContentRef>, queue_capacity: u16 }
struct BeltLinePlacement { conveyor: ContentRef, owner_slot: u8, start: WorldPos, facing: Facing, pieces: u16 }
```

### 3.1 Validation (portable path and sim path)

Errors use the M2 `ModError` form (package, file, field).

- A machine needs an `inventory` that accepts every input item and an `output_inventory` that
  accepts every output item. Sum of inputs ≤ input capacity. Sum of outputs ≤ output capacity.
  Inputs and outputs are non-empty Item resources. A machine is static.
- A unit with `health` must declare `removal`. A weapon is static.
- A weapon with `ammo` needs the ammo item in the unit's `inventory.accepts`.
- `WeaponModifier.unit` must name a unit definition that has a `weapon`.
- Technology prerequisites form an acyclic graph inside the package (depth-first check). M2 cost
  and refund rules apply (M2D:162-165). `ProducerInventory` means the station's inventory.
- Definition ids are unique across the resource, unit, node, production, recipe and technology
  kinds within a package. Map and visual ids stay in their own namespaces (M2D:93). Cross-package
  references stay rejected.
- **Version honesty (invariant 10).** A package that uses any 0.2.1 field, kind or export list must
  declare an `engine_api` requirement that excludes 0.2.0 (for example `^0.2.1`). Otherwise
  validation rejects it with a `ModError` on the manifest `engine_api` field. A `^0.2` package that
  uses an M3 field would claim a compatibility that is false. Add a test for this.
- **Removal flags.** The belt piece and the inserter declare `removal { by_command: true,
  contents: Destroy }`. The drone declares `by_command: false`. The drill, press, turret and lab
  declare no `removal`, so `Remove` rejects them with `NotRemovable`.
- **Prototype constraint:** a unit has at most one work capability among gatherer, producer,
  machine, research, extractor, inserter, conveyor, weapon. `Remove` is not allowed on a unit
  that has a queue (producer, machine, research). These limits bound the test matrix. S:4.2
  composition stays the long-term rule.
- Item count: at most 65,535 Item resources per world (`ItemIx` is `u16`).

## 4. Belts

A belt **piece** is a unit with the `conveyor` capability. It never moves. **Items are not
entities** (S:416). A belt **line** is one compact record. Positions are discrete slots.

```rust
type ItemIx = u16;                      // 0 = empty; 1.. = index+1 into ItemTable (sorted by ContentId)
struct BeltLine { def: ContentId, owner: u8, pieces: Vec<u32> /* upstream→downstream UnitIds */,
                  slots: Vec<ItemIx> /* len = pieces × slots_per_piece; index 0 = upstream end */,
                  slots_per_piece: u8, slots_per_second: u64, carry: RateCarry }
struct Conveyors { lines: BTreeMap<u32, BeltLine> /* key = first piece UnitId */,
                   piece_line: BTreeMap<u32, (u32, u16)> /* derived */ }
```

New file: `crates/turts_sim/src/economy/conveyor.rs`. New files for the other sections:
`economy/{inserter,extractor,combat,research}.rs` in `turts_sim`, and `content_m3.rs` in
`turts_mod_runtime/src`.

### 4.1 Placement without a grid

The map stays continuous. M3 adds one grid-like concept: the four-way `Facing`.

- A line is declared in map data by `BeltLinePlacement`. Order comes from the list, not from
  geometry. There is no adjacency inference.
- `start` is the centre of piece 0. Pitch = the conveyor footprint diameter. Piece `i` is at
  `start + facing_vec * pitch * i`.
- Boot allocates piece unit ids **after** all `placements`, line by line, upstream to downstream.
- Each piece passes the normal bounds and overlap checks. Tangent pieces pass.
- Lines do not connect to each other in M3.

### 4.2 Step rule

Per tick, per line in ascending line key:

1. `steps = carry.take_budget(slots_per_second, tick_hz)` (the M2D:191-195 rule). The carry
   accumulates every tick for every line.
2. Repeat `steps` times: for `i` from `len-2` down to `0`: if `slots[i] != 0 && slots[i+1] == 0`,
   move the item to `i+1`. The last slot never moves. The line end holds items.

The scan goes downstream first, so a compressed run moves as one block. There is no lag of one
tick per piece. An item moves at most one slot per step. A belt step creates and deletes nothing,
so it writes no ledger account. Each line step is one captured operation (section 12.1).

Cost: N pieces × K slots reads per step. Demo: 6 × 4 = 24 reads per step, one step per 5 ticks.
Large-scale cost is **not measured**. M7 benchmarks decide whether to switch to a gap list.

### 4.3 Endpoints on a belt

- **Drop** onto a piece: only into the first slot of that piece (`p*K`), only if it is empty and
  the item is in `accepts`. Otherwise the inserter waits.
- **Pick** from a piece: the downstream-most occupied slot of that piece whose item the hand and
  the destination accept. One item.
- **Removal** of piece `p`: the items in slots `p*K..(p+1)*K` are destroyed and counted. The line
  splits: `pieces[..p]` keeps the old key; `pieces[p+1..]` gets the key `pieces[p+1]`. Both halves
  copy the old carry. Empty halves are dropped. `piece_line` is rebuilt for the changed lines.

## 5. Inserters

Capability `inserter: InserterDef` plus the existing `inventory` capability. **The inventory is
the hand.** Stack size = hand capacity. Filter = hand `accepts`. A held item is ordinary
`on_hand` stock, so interruption never loses it and the ledger, oracle and replication count it
with no new code.

```rust
struct InserterState { source: Option<u32>, destination: Option<u32>, stage: ArmStage, elapsed: u32 }
enum ArmStage { AtSource(Option<ArmWait>), ToDestination, AtDestination(ArmWait), Returning }
enum ArmWait { NoSource, NoDestination, SourceEmpty, DestinationFull }
struct Inserters(BTreeMap<u32, InserterState>);
```

### 5.1 Endpoint resolution (boot)

Pick point = `pos - facing_vec * reach`. Drop point = `pos + facing_vec * reach`. A point resolves
to the single non-inserter unit whose footprint circle contains it (squared centre distance ≤
radius squared). Belt pieces count. Zero or more than one match rejects the map with a `ModError`
on `placements[i].facing`. Source and destination must have the inserter's owner. The source needs
an output inventory, an inventory or a conveyor. The destination needs an inventory or a
conveyor. After boot, endpoints never re-resolve. When a target is removed, the endpoint becomes
`None`.

Source side of a unit = its **output inventory** if it has one, else its main inventory.
Destination side = its main inventory. So: press input = main, press output = output; drill =
main; turret magazine = main; lab = main.

### 5.2 Cycle

Durations convert with the `ceil` rule. One transaction per inserter per tick.

- `AtSource`: try to pick. Candidates = items in the source with `on_hand - reserved > 0`, in
  ascending `ContentId`, that the hand accepts and the destination accepts, **and** the
  destination has room now (inventory: `used < capacity`; belt: first slot of the piece empty).
  Amount = min(available, hand free capacity). From a belt: one item. On success: stage
  `ToDestination`, `elapsed = 0`. On failure: stay, with a wait reason.
- `ToDestination`: `elapsed += 1`. At `elapsed == swing_ticks`, try the drop in the same tick.
  Drop amount = min(held, destination free room). Hand empty: `Returning`, `elapsed = 0`. Else
  `AtDestination(DestinationFull or NoDestination)` and retry every tick.
- `Returning`: `elapsed += 1`. At `elapsed == return_ticks`: `AtSource(None)` and try the pick in
  the same tick.

Forge cycle: 5 + 5 = 10 ticks = 2 items/s. The pick does not reserve destination room (K7). If
another inserter fills the destination during the swing, the arm waits holding the item.

### 5.3 Tie-break

Inserters run in ascending `UnitId`. Each pick is its own committed transaction, so the second
inserter sees the first result. Two inserters cannot take one item (S:543). Unit id order equals
map placement order (fact in section 2). The map author fixes the winner by list order. This is a
repeatable rule, not fairness (S:416 keeps fairness open).

## 6. Item recipes and machines

Item recipes are a new content kind `RecipeDef`, run by the unit capability `machine`.
Machine jobs live in the M2 `ProductionQueues` (queue length ≤ 1) and use the M2 `ProductionJob`,
`charge_costs`, `refund_charged`, `CancelProduction` and the `OutputBlocked` stage. This follows
S:410 (shared primitives, different queue semantics).

Rejected: extending `ProductionDef` with an output enum. `output_unit` is required in
`schema.rs:235-243`; changing it breaks every existing package.

Ledger changes: add `Endpoint::Output(u32)` and
`EconomyLedger.output_inventories: BTreeMap<u32, Inventory>`. Generalize `Transaction::inventory`,
`inventory_used`, `validate_endpoint`, `account` and `commit` to take an `Endpoint`. Keep the
`u32` forms as wrappers for M2 callers.

Per machine, in ascending UnitId, when enabled, in `run_production`:

1. Queue empty: one transaction: `charge_costs` of the inputs from `Endpoint::Inventory(machine)`;
   allocate a `JobId`; push a job with `stage: Running`, `elapsed_ticks: 0`, `charged = inputs`,
   `reservation: None`; event `ProductionStarted`. Not enough input: no write, status
   `InsufficientInput`.
2. Running: advance as M2 (`production.rs:279-291`). At `elapsed == duration`: stage
   `OutputBlocked` and try output in the same tick (M2D:321-323).
3. Output: one transaction credits every output line to `Endpoint::Output(machine)`, all or none
   (filter and capacity); removes the job; event `ItemsProduced { machine, job, outputs }`. No
   room: no write, retry every tick, never recharge.
4. The next job starts on the next tick. One cycle = `duration_ticks + 1` ticks.

`CancelProduction(machine, job)` uses the M2 path (`production.rs:145-231`) with the
`running` / `output_blocked` ratios of `RunRefunds`. The refund goes to
`Endpoint::Inventory(machine)`. If that inventory lacks room, reject `RefundDestinationFull` and
change nothing (M2D:350-353). K8: an enabled machine with enough input restarts in the same tick.

## 7. Extractor (drill)

Capability `extractor` on a static unit with an `inventory`. Boot binds it to the nearest node in
range: squared centre distance ≤ (unit radius + node radius + `interaction_range`)² (M2D:271-273);
tie by NodeId. No node in range rejects the map. The node item must be accepted by the inventory.

State: `ExtractorState { node: Option<NodeId>, carry: RateCarry, status: Extracting | Full | Depleted }`.

Per tick, in ascending UnitId, when enabled: `budget = take_budget(per_second)`;
`n = min(budget, node.remaining, free capacity)` (M2D:278). If `n > 0`, in **one** transaction:
`remaining -= n`, inventory `on_hand += n`, event `Extracted { node, cargo: drill, item, amount }`
(the M2 event kind, `economy/events.rs:20-63`). Unused budget is lost (M2D:193-194). Nodes never
refill (S:542).

Do not copy the M2 gather write (section 2). Add a node overlay to the `WriteSet`
(`nodes: BTreeMap<NodeId, u64>`) and one shared helper `extract_from_node(tx, node, endpoint, item, n)`.
M3 does not change the M2 gather code.

## 8. Combat and removal

### 8.1 State

`Healths(BTreeMap<u32, u64>)` (current; initial = max).
`WeaponState { cooldown: u32, status: Ready | Cooling | NoTarget | OutOfAmmo, last_target: Option<u32> }`.

### 8.2 Shot (Combat phase)

Per weapon, ascending UnitId, one transaction per tick, when enabled:

1. If `cooldown > 0`: `cooldown -= 1`.
2. If `cooldown == 0`: select the target among units with a health row whose owner matches the
   filter relative to the weapon owner and whose squared centre distance ≤ range². The smallest
   squared distance wins; tie by lowest UnitId. Select again every shot (stateless).
3. No target: status `NoTarget`. Ammo `available < per_shot`: status `OutOfAmmo`, no shot,
   cooldown stays 0.
4. Shot, all in **one** transaction: debit `per_shot` from `Endpoint::Inventory(weapon)`;
   `damage = base + modifier(owner slot, weapon unit def, Damage)` (checked add);
   `applied = min(health, damage)`; `health -= applied`; `cooldown = cooldown_ticks`; event
   `Fired { weapon, target, item, ammo, damage, applied }`. If health reaches 0, stage
   `remove_unit(target, Killed { by: weapon })` in the same transaction. Surplus damage is not
   carried.

Hostility is mod data (`SlotRelation`), not an engine rule (S:159, S:163).

### 8.3 Removal (the first despawn in the engine)

One function `remove_unit(tx, unit, cause)` stages a `RemoveWrite`. In the same commit:

| Store | Action |
| --- | --- |
| Main and output inventories (hands, magazines) | every `on_hand` is destroyed and listed in the event. `reserved` must be 0, else reject `IntegrityViolation` (sim-internal) |
| Belt piece | slots of the piece destroyed and listed; line split (4.3) |
| `ProductionQueues` row | none can exist (validation); assert |
| Inserter rows | the removed row is deleted; every other inserter with `source` or `destination == unit` gets `None` |
| Extractor, weapon, health rows | deleted |
| `EconomyOrders` row | deleted; gatherers whose `receiver == unit` re-select on their next tick (M2D:297). **NOT TRACED** — a task must test it |
| Disabled set | id removed |
| ECS entity | `world.despawn(entity)` |
| `RemovedThisTick` (new resource) | push the id; `publish_completed_tick` drains it into `CommittedTick.removed` (replaces the literal at `plugin.rs:156`) |
| Unit id | never reused |

Event: `Destroyed { unit, cause: Removed | Killed { by }, items: Vec<DestroyedLine { location, item, amount }> }`.
A unit that is spawned and removed in one tick is dropped from both lists. No M3 path does this;
assert it.

## 9. Research

A research station has a `ProductionQueues` row. `Produce(station, technology)` reuses the M2
command. `queue_production` resolves the id as a technology when the unit has `research`.

Checks, in order, after the M2 ownership check: technology listed by the station
(`RecipeNotEnabled`); prerequisites complete for the slot (`PrerequisitesMissing`); not complete
for the slot (`AlreadyResearched`); not in any queue of the same slot (`ResearchInProgress`).
Payment, start, advance and cancel use the M2 code paths. Completion never blocks: one
transaction removes the job, inserts `(slot, technology)` into `ResearchState`, and emits
`ResearchCompleted { station, job, slot, technology }`. Server admission
(`turts_server/src/session.rs:376-384`) must accept a unit with `research` as well as `producer`.

`ResearchState { completed: BTreeSet<(u8, ContentId)> }`. Definitions stay in the catalog. Player
state stays in `ResearchState` (S:424). Modifiers are **computed**, never stored:
`modifier(slot, unit_def, stat) = sum of add` over completed technologies of the slot and over
the effects that name `(unit_def, stat)` (checked add). Stacking = sum. Forge has one effect, so
the Open question of stacking (S:428) does not arise in the demonstration. It stays Open.

These research rules are **M3 engine prototype limits** (K6): per-slot state; not repeatable; no
parallel duplicate per slot; stacking is a sum. They are fixed in engine code and no package field
changes them. They do **not** close the Open policies in S:428 (cancellation, parallel research,
exclusive branches, repeatable technologies, team sharing, modifier stacking). Those need a user
decision before a second game uses research. Tests name these rules as M3 limits.

## 10. Commands

```rust
// sim CommandKind and wire PlayerCommandKind
Remove { unit_id: UnitId }
SetEnabled { unit_id: UnitId, enabled: bool }
```

`Remove`: ownership check first (M2D:385-386); then `removal` declared with `by_command: true`
(else `NotRemovable`); then `remove_unit(tx, unit, Removed)`.

`SetEnabled`: ownership check; the unit must have extractor, inserter, machine, research,
producer or weapon (else `NotToggleable`). Same value: applied, no change, no event. A change is
one transaction that updates `Disabled(BTreeSet<u32>)` and emits `EnabledChanged { unit, enabled }`.

| Capability | While disabled |
| --- | --- |
| extractor | no budget taken; carry reset to 0 on disable (M2D:193) |
| inserter | stage, elapsed and hand frozen |
| machine, research, producer | head job frozen: no start, advance or output. `CancelProduction` still works |
| weapon | cooldown frozen; no shot |

Conveyors are not toggleable in M3.

New reject reasons: `NotRemovable`, `NotToggleable`, `PrerequisitesMissing`, `AlreadyResearched`,
`ResearchInProgress`. Add them to the sim enum and `as_str`, to the wire `CommandRejectReason`
(`ALL`, `as_str`), and to `wire_reason` (`turts_server/src/outbound.rs:55-85`).
`IntegrityViolation` is sim-internal: no valid command produces it, and `wire_reason` maps it to a new wire reason `Internal` so that the exhaustive match compiles.

Command path (every place a variant needs an edit): sim `CommandKind` and the `command.rs:143`
match; protocol `PlayerCommandKind`; `session.rs:339` `admit_kind` (exhaustive match); reject
enums; client producers (`input.rs`, `economy_ui.rs`, `scenario.rs`); tests. No new lightyear
registration is needed. Source: `docs/research/m3/m2-code-seams.md` section 4.

## 11. Tick order

```
FixedPreUpdate : Advance → Commands
FixedUpdate    : Movement → Gather → Extract → Transport → Inserters → Production → Combat
FixedPostUpdate: Collect
```

New `SimSet` variants `Extract`, `Transport`, `Inserters`, `Combat` join the chain at
`plugin.rs:70-73`, each with `run_if(not_stopped)`. Reasons:

- Commands first (M2D:242-244): `Remove` and `SetEnabled` apply before every phase.
- Extract before Transport and Inserters: drill output is pickable in the same tick.
- Transport before Inserters: an item that reaches the belt end is pickable in the same tick. An
  item dropped this tick moves at the next step, never twice in one tick.
- Inserters before Production: an input dropped this tick can start a machine this tick. Machine
  output is pickable next tick.
- Production before Combat: a research that completes this tick changes damage for shots in the
  same tick, and ammunition dropped this tick can fire.
- Combat last: targets have this tick's positions. Destruction is visible from the next tick.

A phase with no subjects captures no operation. **M2 packages must produce the same operation
stream as before.** This is a regression test (F2).

## 12. Audit, replication and protocol

### 12.1 Audit rule

For every new phase: **one subject, one tick, one `transact`** (stage change and accounting
together). This keeps the audit rule (section 2) valid with no special case. The belt step,
inserter advance and weapon cooldown are writes, so they also go through `transact`.

`WriteSet` gains per-key overlays `BTreeMap<K, Option<V>>` (`None` = delete) for nodes, conveyor
lines, inserters, extractors, weapons, healths, research and disabled, plus
`removed: Vec<RemoveWrite>`. Do not clone whole tables. `AuditState` (`views.rs:295-315`) gains
`raw_output_inventories`, `raw_belts`, `raw_inserters`, `raw_extractors`, `raw_weapons`,
`raw_healths`, `research`, `disabled`, `removed_units`. The catalog audit mirror gains recipes and
technologies.

### 12.2 Protocol 3

`NET_PROTOCOL_VERSION` 2 → 3 (`turts_net_protocol/src/lib.rs:13`).

| Record | Content | Visibility |
| --- | --- | --- |
| `UnitSnapshot.facing: Option<Facing>` | static | public |
| `healths: Vec<HealthRecord { unit, current }>` in Snapshot and WorldDelta | changed only. Public to every slot: a **proposal**, like item visibility, until the fog design | public (proposal) |
| `belts: Vec<BeltPieceRecord { piece, slots: Vec<Option<ContentRef>> }>` | one record per piece | owner-only |
| `inserters: Vec<InserterRecord { unit, enabled, stage, elapsed, source, destination }>` | | owner-only |
| `extractors: Vec<ExtractorRecord { unit, enabled, node, status }>` | | owner-only |
| `weapons: Vec<WeaponRecord { unit, enabled, cooldown, damage, status, last_target }>` | `damage` = effective value | owner-only |
| `QueueState.enabled: bool` | added field | owner-only |
| `research: Vec<ResearchRecord { slot, completed }>` | per slot | owner-only |
| `InventoryState.port: Main \| Output` | added field | owner-only |
| `WorldDelta.removed` | now filled. A unit removed in tick t also appears in `healths` of the same delta with its last value (0 after a kill), so a client can count the killing hit. The client applies `healths` before `removed` | public |

Naming: wire types live in `turts_net_protocol` and end in `Record`. The sim types in section 4 to 9
(`InserterState`, `WeaponState`, `ExtractorState`) are `pub(crate)` in `turts_sim`. The two sets
must not share names.

All records are absolute replacements keyed by unit (or slot) and use the generic `replacements`
diff (`views.rs:422-460`). Change detection is a value diff of the views, not Bevy `Changed<T>`.
Item visibility to enemy slots is a **proposal**: owner-only until the fog design.

Demo estimate for slot 0: about 150-250 B per tick payload (belt 6 pieces × 26-46 B every 5 ticks,
inserters, weapon, queue records). Framing overhead **not traced**. Instrument: `tests/net/relay.py`.
Slot 1 gets only public data. This is an estimate, not a budget.

Client edit points for a new record kind (`docs/research/m3/m2-code-seams.md` section 5): sim
views and `AuditState`; `CommittedTick`; protocol Snapshot and WorldDelta; `export.rs`; client
`replica.rs` (`Candidate`, `stage_snapshot`, `stage_delta`, `valid_economy`, `merge_economy`);
`ingest.rs` `merge_economy_baseline` (a second hand-written copy of the merge); `render.rs`;
`economy_ui.rs`; `debug.rs`; `tests/browser/m2.js:356` (asserts exact key sets). Use **one generic
keyed-record merge helper** to avoid nine copies.

## 13. Package `forge` and the demonstration

Package id `forge`, kind `Game`, `engine_api: "^0.2.1"`. Pure physical items: no balance, no
power. A separate package, because 16.2 lets industry be a follow-on scenario (S:751).

All local ids below are **working names**. The E1 scan matches mod local ids against engine
identifiers as exact tokens. The engine will have tokens such as `inserter`, `range`, `belts`,
`conveyor`. Task T5 runs E1 first and renames any colliding id, then records the final names in
`mods/forge/NAMES.md`. Suggested collision-free names are in the third column.

### 13.1 Content (all numbers PLACEHOLDER; 20 Hz)

| Role | Working id | Suggested id | Numbers |
| --- | --- | --- | --- |
| Item | `forge:ore` | `forge:ore` | one item = one capacity unit |
| Item | `forge:shell` | `forge:shell` | ammunition, recipe output, research cost |
| Node | `forge:ore_vein` | `forge:ore_vein` | radius 1.0; remaining 300 (depletion fixture: 21) |
| Drill | `forge:drill` | `forge:bore_drill` | r 1.2; 2 ore/s; interaction_range 0.5; inventory cap 4, accepts ore |
| Belt piece | `forge:belt` | `forge:chain_belt` | r 0.5 (pitch 1.0); 4 slots/piece; 4 slots/s; accepts ore, shell |
| Inserter | `forge:inserter` | `forge:arm_loader` | r 0.4; reach 1.0; swing 250 ms; return 250 ms; hand cap 1 |
| Press | `forge:press` | `forge:shell_press` | r 1.2; input cap 6 (ore); output cap 4 (shell) |
| Recipe | `forge:make_shells` | `forge:make_shells` | 2 ore → 1 shell; 2000 ms; cancel refunds running 1/1, output_blocked 1/1 |
| Turret | `forge:turret` | `forge:shell_turret` | r 1.2; magazine cap 5 (shell); range 8.0; cooldown 1000 ms; 1 shell/shot; damage 10; targets `Other` |
| Lab | `forge:lab` | `forge:proving_lab` | r 1.2; input cap 10 (shell) |
| Technology | `forge:hardened_shells` | `forge:hardened_shells` | 10 shell; 10000 ms; ReserveThenCharge; queued refund 1/1, running 1/2; effect +10 damage to the turret unit |
| Drone | `forge:drone` | `forge:target_drone` | r 0.6; speed 2.0; health 100; removal `by_command: false`, Destroy |
| Map | `forge:range` | `forge:proving_ground` | 32 × 16 |

`SlotRelation::Other` makes a weapon target units of every slot except its owner's.

Removal flags (section 3.1): the belt piece and the inserter declare `removal { by_command: true, contents: Destroy }`; the drone declares `by_command: false`; the drill, press, turret and lab declare no `removal`.

### 13.2 Map (PLACEHOLDER coordinates; list order sets UnitId order)

Slot 0 owns the industry. Slot 1 owns two drones. Inserter B must be listed before inserter C.

| Entity | Owner | Position | Facing |
| --- | --- | --- | --- |
| ore vein (node, remaining 300) | | (3.5, 8) | |
| drill | 0 | (6, 8) | |
| inserter D (drill → belt) | 0 | (8, 8) | PosX |
| belt line, 6 pieces, start (9, 8) | 0 | (9..14, 8) | PosX |
| inserter A (belt → press) | 0 | (15, 8) | PosX |
| press | 0 | (17, 8) | |
| inserter B (press → turret) | 0 | (17, 10) | PosY |
| inserter C (press → lab) | 0 | (17, 6) | NegY |
| turret | 0 | (17, 12) | |
| lab | 0 | (17, 4) | |
| drone A | 1 | (24, 12), 7.0 from turret: in range | |
| drone B | 1 | (30, 4), 15.3 from turret: out of range | |

The orchestrator checked by hand that every pick and drop point falls inside exactly one
footprint and that no footprints overlap. Task T4 proves it by booting the map.

### 13.3 Demonstration

Players take these actions: slot 0 sends `Produce(lab, hardened_shells)` once the lab holds 10
shells. Slot 1 sends `MoveTo(drone B, (24, 14))` (7.28 from the turret) after research
completes. Tests use the headless client options `--move`, `--move-unit`, `--move-after-ticks`.

Timeline (tick-model estimate from the placeholder rates, **not measured** in the real sim): first
ore on the belt at tick 15 (0.75 s); first shell at tick 170 (8.5 s); belt full at about 25-29 s; drone A destroyed at about 28 s by the 10th
shell; lab holds 10 shells at about 60 s; research completes at about 70 s; drone B destroyed at
about 75-80 s by the 5th shell; chain static at about 95 s.

**Measured effect of research: shells fired at drone A = 10; shells fired at drone B = 5.**
Damage per shot before the completion tick = 10; from the completion tick = 20.

### 13.4 Final jam (fixed point)

The jam is a function of the capacities, not of timing. Hand-derived with the K7 pick rule
(empty hands at the jam). **Not run.** Task T14 asserts it. If a run disagrees, the engineer
finds the cause by analysis. Nobody edits the number to match a run.

| Location | Final count |
| --- | --- |
| turret magazine | 5 shell |
| lab input | 10 shell |
| press output | 4 shell |
| press input | 6 ore |
| press head job | `OutputBlocked`, 2 ore charged |
| belt | 24 ore (6 pieces × 4 slots) |
| drill buffer | 4 ore |
| every inserter hand | 0 |
| shells produced | 15 fired + 10 research + 5 + 10 + 4 = **44** |
| ore charged | 44 × 2 + 2 = **90** |
| ore extracted | 90 + 6 + 24 + 4 = **124** |
| node remaining | 300 − 124 = **176** |

## 14. Cases and conservation

### 14.1 Checks

The oracle (15.2) runs these checks after **every** captured operation. Each check has an id and a
failure text. The M2 refutation runner needs the failure text for each plant.

| Check id | Rule |
| --- | --- |
| `BAL-ore`, `BAL-shell` | For each Item resource r and each operation: `Δstock(r) = refunds(r) + produced(r) − charged(r) − converted(r) − destroyed(r) − fired(r)`. `stock(r)` = main and output inventory `on_hand` (hands and magazines included) + belt slots holding r + node `remaining` for nodes of item r. `Δstock` comes from the before and after `AuditState`. The counters come from the events of that operation. The check fails when state and events disagree. |
| `SHOT` | The oracle parses `mods/forge` (and the fixture packages) **itself**, with the `turts_mod_api` types and RON, not with `turts_pack::load_package` or the sim bootstrap. It does not read `AuditState.catalog`. There is exactly one `ProductionStarted` per `JobId`. For every start, `ItemsProduced` and `Fired` event, the amounts equal the amounts in the oracle's own parse of the recipe, technology cost or weapon, and the matching before/after inventory change equals the same amounts. Charges at job start have no event (`transaction.rs:651`), so the oracle checks the inventory delta of the start operation against the catalog cost. A `Fired` event debits exactly `per_shot` from the weapon inventory in the same operation. |
| `RES` | `0 ≤ reserved ≤ on_hand` on every account. The sum of Active claim lines equals each `reserved`. **Every Active claim names a job that is present in a queue. Every job in stage `Queued` with the `ReserveThenCharge` policy has exactly one Active claim. A job in stage `Running` or later holds none** (the claim closes at start, `transaction.rs:750-752`). `used ≤ capacity` for every inventory. |
| `DMG` | Per target: `max − current = sum(applied)` over its `Fired` events. `applied = min(health_before, damage)`. `damage = base + sum of add` of the technologies whose `ResearchCompleted.tick ≤ Fired.tick` for the shooter's slot (a research that completes in tick t counts for a shot in tick t, because Production runs before Combat). The oracle takes `base` and `add` from its own parse of `mods/forge`. Exactly one `Destroyed { Killed }` at health 0. No `Fired` at a unit after its `Destroyed`. **The damage-by-tick rule is evaluated once at the end of each tick over all operations of that tick**, not per operation: under the C15 plant the shot operation precedes `ResearchCompleted` inside the same tick, and only the end-of-tick view can see it. |
| `REFUND` | For each `ProductionCancelled`, refunded = floor(charged × ratio of the job's stage). The ratios come from the oracle's own parse. |
| `DANGLING` | After a removal, no side table, endpoint or claim names the removed id. |
| `STATE` | Named scenario assertions that conservation cannot see (a disabled unit does not change; no `Fired` while the target is out of range; the tie-break order). Each assertion has its own failure text. |

`S` always means the product specification in this document. The old per-item label `S` is now
`BAL-shell`.

### 14.2 Cases

Each case names the check that must go red when its plant is applied (15.3). Every fixture
distance and tick is chosen so that its plant is visible (see C15 and C16).

| # | Case | Trigger | Invariant | Red check |
| --- | --- | --- | --- | --- |
| C1 | Belt full, drill stops | default rates | drill `Full`; belt never exceeds capacity | `RES` (`used ≤ capacity`) |
| C2 | Press output blocked | targets full | job holds elapsed = duration; no recharge; one shell when room returns | `BAL-ore` |
| C3 | Backpressure chain | C2 continues | stages stop in order: press input, inserter A holds none (K7), belt, drill | `BAL-ore` |
| C4 | Cancel press job, running | `CancelProduction`, fixture: press input has room for at least 4 ore, so the refund fits | refund floor(2 × 1/1) = 2 ore; reject `RefundDestinationFull` if input lacks room; same-tick restart (K8) | `BAL-ore`, `REFUND` |
| C5 | Cancel press job, `OutputBlocked` | same | refund 2 ore; no shell appears; never refund twice | `BAL-shell` |
| C6 | Cancel research, running | `CancelProduction(lab)` | refund floor(10 × 1/2) = 5 shell; tech stays unresearched; damage stays 10 | `REFUND` (ratio from the oracle's own parse) |
| C7 | Cancel research, queued | `Produce` then `CancelProduction` in one batch | reservations released; no charge | `RES` (Active claim without a queued job) |
| C8 | Pause and resume | `SetEnabled` false then true on drill, inserter, press, lab, turret | no change in a disabled unit across N ticks; resume continues from the same progress | `STATE` |
| C9 | Remove belt piece with items | `Remove(piece 3)` | its items are listed in `Destroyed`; line splits | `BAL-ore` |
| C10 | Remove inserter holding an item | `Remove(inserter A)` mid-swing | hand item destroyed and counted; press starves | `DANGLING` |
| C11 | Ore depleted | fixture node remaining 21 | drill `Depleted`; node stays 0; 10 shells; 1 ore stays in press input | `BAL-ore` |
| C12 | Two inserters, one source | B and C pick from press output | the lowest UnitId inserter gets the first shell; each later shell goes to one inserter only (the ledger forbids a double take: that guard is not a plant target) | `STATE` (tie-break order) |
| C13 | Turret ammo starvation | supply below fire rate | `OutOfAmmo`; no shot without a shell | `SHOT` |
| C14 | Overkill and destruction | last shot | shell consumed; surplus not carried; one `Destroyed`; retarget | `DMG` |
| C15 | Research completes in the shot tick | fixture: target health ≥ 400; magazine preloaded by the trusted transfer facility; `Produce` tick chosen so that a shot falls on the completion tick | the shot in the completion tick deals 20; earlier shots deal 10. **Precondition assert: a `Fired` exists at the `ResearchCompleted` tick.** The test fails if the precondition fails | `DMG` |
| C16 | Target leaves range | fixture target moved to **12.0** from the turret (between range 8 and 2 × range 16) | no `Fired` while distance > range; ammo kept | `STATE` |
| C17 | Ownership and rejection (S section 12, invariant 13) | slot 1 sends `Remove` and `SetEnabled` for slot 0 units; slot 0 sends `Remove` for the press (no `removal`) and `SetEnabled` for a belt piece; `Produce` of a researched, queued or locked technology (the locked case uses the fixture package `tests/m3/packages/forge_locked`, which adds a second technology with the first as prerequisite) | each command is rejected with the exact reason (`NotOwner`, `NotRemovable`, `NotToggleable`, `AlreadyResearched`, `ResearchInProgress`, `PrerequisitesMissing`); state and audit are unchanged; a command from the owner still applies | `STATE` |

## 15. Acceptance and instruments

### 15.1 Acceptance F1-F11

Every item has a command, a pass condition and the build on which it goes red.

| ID | Check | Goes red when |
| --- | --- | --- |
| F1 | E1 no-content scan. **The scanner changes:** its export list (`turts_checks/tests/support/mod.rs:87-94`) and its `stray_ron` rule (`:333-350`) learn the `recipes` and `technologies` kinds, so forge does not make E1 error out. Plants: a production Rust file that holds a forge recipe id; one that holds a forge technology id; one that holds a forge unit id. E2 renamed-clone: the same commands on a renamed forge give the same trace. | any of the three plants stays green, or a rename changes the trace |
| F2 | M2 regression: all M2 acceptance tests pass unchanged (only version literals change). **Baseline first:** task T0 records the per-tick operation count and kinds for every M2 package on commit `9e1987c` in `tests/m3/m2_operation_stream.json`, before the first M3 edit. The test compares against this file. | a new phase captures an operation for a package that has no such subject |
| F3 | `turts-pack mods/forge` exits 0; the forge map boots; unit ids, endpoints and node binding match 13.2; an ambiguous endpoint rejects with a `ModError`; a `^0.2` package that uses an M3 field rejects (3.1) | endpoint resolution or version honesty is wrong |
| F4 | Exact timing tests in `turts_sim`. **Expected values are written as literals in the test**, derived by hand from the spec rules: belt transit tick, compression, inserter period (10 ticks), machine cycle (`duration + 1`), weapon period (20 ticks), extractor rate with carry. They do not read the catalog audit copy. | any timing rule drifts |
| F5 | Scenario cases C1-C17 with the oracle after every operation | a plant of 15.3 is applied |
| F6 | Fixed-point end state of 13.4 | a capacity or rule is wrong |
| F7 | Research effect, coarse: shots at drone A = 10, at drone B = 5. (The exact tick is C15.) | the modifier is missing |
| F8 | Replication privacy: decoded snapshot and delta for both slots; owner-only records absent for slot 1; `removed` filled; protocol 3 round trip | a record leaks or a field is missing |
| F9 | Mutation sweep: every plant in `tests/m3/refutations.json`: baseline green, plant red, restored green | a plant stays green |
| F10 | Browser: client joins, shows the panels, sends `Produce(lab)`, `SetEnabled`, `Remove` with Acked and Applied results (T7 pattern) | the client does not show or send |
| F11 | Process acceptance: server plus two headless clients. **Client actions are triggered by replica state, not by time**: slot 0 sends `Produce(lab, technology)` once its replica shows 10 shells in the lab. Research stays owner-only. The test coordinator observes the owner client's replica confirmation of completion, then releases a test signal that causes slot 1 to send `MoveTo`. Slot 1 never receives slot 0's private research records. This resolves the original F11 privacy conflict by explicit user choice on 2026-10-27. The test also fails (test invalid) when the recorded `MoveTo` applied tick is not later than the `ResearchCompleted` tick. The client report gains a **health trace** per unit (public `healths`). The test derives shots from it: drone A shows ten steps of −10; drone B shows five steps of −20. | the end-to-end path breaks, or the test runs in the wrong order |

### 15.2 Oracle design

Do **not** extend the M2 predictor (`accounting.rs:1541-2683`, a second implementation). Add
`crates/turts_pack/tests/common/conservation.rs`. It reads `AuditState` and the events of each
captured operation, and it parses the package source files (`mods/forge`, fixture packages) on
its own. It checks section 14.1 after **every** operation. It cross-checks every event against the
before/after state change of the same operation. It never trusts an event field alone, and never
trusts the catalog copy inside `AuditState`.

Add exact unit tests for timing (F4). Each check gets one hand-made bad `AuditState` that must
fail (instrument self-test).

Instrument fixes in the code that M3 builds on: `Transaction::commit` silently drops a write to a
missing inventory (`transaction.rs:419-423`). In M3 code paths, a write to a missing endpoint is an
`IntegrityViolation` error. M2 behavior stays the same.

### 15.3 Plants (one per case)

| Case | Plant |
| --- | --- |
| C1 | disable extractor inventory-capacity enforcement at all three defenses: phase free-space clipping, the `extract_from_node` capacity guard, and transaction preflight |
| C2 | recharge the inputs on each blocked retry |
| C3 | belt step ignores the occupancy of slot `i+1` (overwrite) |
| C4 | call `refund_charged` twice |
| C5 | cancel in `OutputBlocked` also credits the output |
| C6 | in `turts_sim` bootstrap, install the `running` refund ratio as 1/1 instead of 1/2 (a load-time defect; the oracle's own parse of the RON does not share this code, so `REFUND` sees it) |
| C7 | skip `release_reservation` |
| C8 | an inserter advances while disabled |
| C9 | delete slots without listing them in `Destroyed` |
| C10 | keep the hand inventory row after despawn |
| C11 | restore `remaining` from the map value at 0 |
| C12 | visit inserters in descending UnitId |
| C13 | fire with `available < per_shot` and no debit |
| C14 | skip the removal at health 0 |
| C15 | run Combat before Production |
| C16 | compare distance to `2 * range` |
| C17 | skip the ownership check in `Remove` and `SetEnabled` |

Register all plants in `tests/m3/refutations.json` with the M2 runner pattern
(`tests/m2/refutations.py`). Each entry carries the semantic failure text of its red check. A
plant that the sweep cannot turn red is a defect in the instrument, not a reason to drop the plant.

## 16. Risks, cut line, open items

### 16.1 Risks

1. **Client and replication cost doubles the milestone.** Each new record touches about nine
   client places (section 12.2). Mitigation: one generic keyed-record merge helper; text panels
   only.
2. **The test instrument grows into a second simulator.** Mitigation: conservation by state and
   events plus small exact timing tests (15.2). A reviewer who asks for a full predictor must name
   the defect that conservation and the timing tests miss.
3. **E1 goes red on forge names.** Mitigation: T5 runs E1 on forge first (section 13).
4. **The first despawn ripples.** Mitigation: one `remove_unit` function and the Dangling check.
5. **Large files grow.** `transaction.rs` is 1,546 lines, `ingest.rs` 1,517, `session.rs` 1,274.
   New logic goes into new files (`conveyor.rs`, `inserter.rs`, `extractor.rs`, `combat.rs`,
   `research.rs`, `content_m3.rs`).

### 16.2 Cut line (in order, only if the milestone proves too large)

1. `SetEnabled` (C8).
2. Drone B and `MoveTo`: measure the research effect by `WeaponRecord.damage` and shots per kill on
   a fixture target.
3. Browser rendering of belt items: show per-piece counts instead.
4. `Remove` as a player command: keep `remove_unit` as a trusted scenario facility, like
   `inventory_transfer` (M2D:234-237).

**Must not be cut:** compact belt with backpressure; item recipe with `OutputBlocked`; inserter
hand as inventory; atomic shot with removal; a research that changes a stat; conservation checks
with plants; E1 and the M2 regression.

### 16.3 Open and retained

Belt representation at scale (gap list), junction fairness, scheduling and sleeping (S:416, S:616);
research policies (S:428); item visibility to enemies; consume-over-time payment; tick rate and all
performance budgets. M3 selects none of them. M3 records measurements only if a task produces
them, with machine type, and claims no budget.

## 17. Sources

- `docs/research/m3/content-designer.md`: content and demonstration (game-designer seat).
- `docs/research/m3/engine-simulation.md`: engine design, 21 verified facts, probes
  (simulation-core seat).
- `docs/research/m3/m2-code-seams.md`: recon of the M2 code.
- Plan: `docs/superpowers/plans/2026-10-27-m3-industrial.md` (written after gate 1).
