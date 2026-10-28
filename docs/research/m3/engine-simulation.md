> **Research input, not a contract.** Written 2026-10-27 by the simulation-core seat during the M3 brainstorm. Where it conflicts with `docs/superpowers/specs/2026-10-27-m3-industrial-design.md`, the design wins. Facts F1-F21 were checked by that seat; reviewers re-check them.

# M3 "Industrial loop": engine design (simulation-core seat, brainstorm, 2026-10-27)

Status of this note: **Proposed**. It extends the M2 contracts in
`docs/superpowers/specs/2026-10-26-m2-economy-design.md` (M2D) sections 3, 4, 6 and 7. It does not
replace them. Every number in forge data is a PLACEHOLDER from the designer note. Repo at 9e1987c.

Abbreviations: S = `RTS-Engine-Project-Specifications.md`; M2D = M2 design; R = recon note
`scratchpad/m3-recon.md`; G = designer note `scratchpad/m3-designer.md`. Paths are relative to
`/home/julien/Projects/TuRTS`.

## 0. Facts this design depends on (re-checked, not taken from R or G)

| # | Fact | Evidence | How checked |
| --- | --- | --- | --- |
| F1 | Unit ids are allocated in map placement order, one by one, from `next_unit` | `crates/turts_sim/src/bootstrap.rs:152-264` (loop over `map.placements`, `allocate_unit` at :261), `crates/turts_sim/src/ids.rs:33-39` | read |
| F2 | Ids never get reused: allocators only increment, checked | `ids.rs:33-54`; `transaction.rs:255-263` | read |
| F3 | Tangent circles pass the boot overlap check (`<` on squared distance) | `crates/turts_sim/src/map.rs:180-192` | read |
| F4 | One inventory per unit; `Endpoint` has only `Slot(u8)` and `Inventory(u32)` | `crates/turts_sim/src/economy/ledger.rs:7-41` | read |
| F5 | `transact` wraps every transaction in `capture_operation` | `crates/turts_sim/src/economy/transaction.rs:234-239` | read |
| F6 | WriteSet stages accounts, claims, the whole `ProductionQueues` (clone), ids, spawns, events. No node, no despawn | `transaction.rs:206-216`, `:273-284`, `:410-498` | read |
| F7 | M2 gather extraction is two writes in one captured operation: `extract` commits the inventory credit, then a direct `remaining` write outside the transaction | `crates/turts_sim/src/economy/gather.rs:571-585`, `:811-835` | read |
| F8 | Phases: `SimSet` = Advance, Commands, Movement, Gather, Production, Collect; FixedUpdate chain Movement -> Gather -> Production | `crates/turts_sim/src/plugin.rs:29-37`, `:69-74` | read |
| F9 | `CommittedTick.removed` is always empty; no despawn exists | `plugin.rs:156` | read |
| F10 | `collect_changes` reads only live entities (`Query<(&UnitId, Ref<Position>)>`), so a despawned unit gives no position | `crates/turts_sim/src/world.rs:87-102` | read |
| F11 | Client rejects a delta with a position for a removed or unknown unit; client removes inventory, order and queue records of removed units | `crates/turts_client/src/replica.rs:414-435` | read |
| F12 | Economy diff emits only current records that changed; a vanished key emits nothing | `crates/turts_sim/src/economy/views.rs:422-460` | read |
| F13 | Audit log fails any state change between captured operations, except the movement phase | `crates/turts_server/src/economylog.rs:579-711` | read |
| F14 | `^0.2` accepts 0.2.1 and rejects 0.3.0; `^0.2.1` rejects 0.2.0 | probe `scratchpad/probe-schema` (semver 1.0.28, the locked version, `Cargo.lock:7622-7623`) | run |
| F15 | A new `#[serde(default)] Option` field loads old RON without the key; `deny_unknown_fields` still rejects a typo; a `required_option` field stays required | same probe (ron 0.12.2, serde 1.0.229) | run |
| F16 | All four mods require `engine_api: "^0.2"` | `mods/{acme,base,cedar,haul}/manifest.ron` | grep |
| F17 | Wire codec is postcard 1.1.3 (not self-describing: any field change breaks old decoders) | `Cargo.toml:25`; `crates/turts_net_protocol/Cargo.toml:13` | grep |
| F18 | Render places each unit with `Transform::from_translation` only (no rotation) | `crates/turts_client/src/render.rs:385-400` | read |
| F19 | E1 scan matches every production Rust identifier and every string literal against every mod local id, exact token, with a 5-word exception list | `crates/turts_checks/tests/support/mod.rs:192-222`, `:458-548` | read |
| F20 | Server `Produce` admission requires the `producer` capability | `crates/turts_server/src/session.rs:376-384`, `:319-334` | read |
| F21 | M2 conservation oracle counts only inventory `on_hand` and node `remaining`, plus charge, refund and conversion terms | `crates/turts_pack/tests/common/accounting.rs:546-597` | read |

Postcard sizes from probe `scratchpad/probe-wire` (postcard 1.1.3): a belt piece record with 4
`Option<String>` slots of `"forge:ore"` is 46 B full, 26 B half, 6 B empty; a small inserter record
is 4 B; a weapon record 6 B; a health record 2 B. Lightyear framing overhead: NOT TRACED.

## 1. Belt representation

### 1.1 Decision: discrete slots in a compact per-line array; pieces are units

- A belt **piece** is a unit (ECS entity with `UnitId`, `Owner`, `Position`, `UnitType`) with a
  new `conveyor` capability. Reason: `Remove`, ownership, footprint, replication of identity and
  the visual table already work per unit (S 4.2 composition). A piece never moves.
- **Items are not entities** (S:416). A belt **line** is one compact record in a new resource
  `Conveyors`. Items are slot values in a flat array.
- Positions on a line are **discrete slots**. Each piece has `slots_per_piece` slots (mod data,
  1..=16). One slot holds zero or one item. Line capacity = pieces x slots_per_piece.

Rejected: continuous gap representation (Factorio "distance to next item"). It gives O(1) moving
cost per line, but it needs fractional positions, a second integer scale, and gap arithmetic in
the conservation oracle. M3 needs exact counts and cheap tests more than throughput. The slot
array can change to a gap list later behind the same views; M7 benchmarks decide (S:416 says the
segment representation needs benchmarks).

### 1.2 Data layout (`crates/turts_sim/src/economy/conveyor.rs`, new)

```rust
/// Item index: 0 = empty, 1.. = position+1 in ItemTable (Item resources sorted by ContentId).
pub(crate) type ItemIx = u16;
#[derive(Resource)] pub(crate) struct ItemTable(pub(crate) Vec<ContentId>); // built at boot
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct BeltLine {
    pub(crate) def: ContentId,          // conveyor unit definition
    pub(crate) owner: u8,
    pub(crate) pieces: Vec<u32>,        // UnitIds, upstream -> downstream
    pub(crate) slots: Vec<ItemIx>,      // len = pieces.len() * slots_per_piece; index 0 = upstream end
    pub(crate) slots_per_piece: u8,
    pub(crate) slots_per_second: u64,
    pub(crate) carry: RateCarry,        // reuse gather.rs:66-86
}
#[derive(Resource, Default, Clone, PartialEq, Eq)]
pub(crate) struct Conveyors {
    pub(crate) lines: BTreeMap<u32, BeltLine>,     // key = first (upstream) piece UnitId
    pub(crate) piece_line: BTreeMap<u32, (u32, u16)>, // piece -> (line key, piece index); derived
}
```

`ItemTable` must hold at most 65,535 items (boot check). Views convert `ItemIx` to `ContentRef`.

### 1.3 Connection without a grid

The map stays continuous f32 (`MapBounds`, `world.rs:35-39`). M3 adds one grid-like concept only:
a four-way **`Facing`** enum `{ PosX, NegX, PosY, NegY }` for belt lines and inserters.

- No adjacency inference. A belt line is placed explicitly in map data:
  `BeltLinePlacement { conveyor: ContentRef, owner_slot: u8, start: WorldPos, facing: Facing, pieces: u16 }`.
  `start` is the center of piece 0. Piece pitch = conveyor footprint diameter (no separate length
  field, so no float equality check). Piece i center = `start + facing_vec * pitch * i`.
- Order inside a line comes from the list, not from geometry. Lines do not connect to each
  other in M3 (no junctions, turns or splitters; G section 5).
- Boot allocates belt piece unit ids **after** all `placements`, line by line in list order,
  upstream to downstream (extends F1). Each piece passes the normal bounds and overlap checks
  (tangent pieces pass, F3).

Rejected: (a) free direction vectors: render has no rotation (F18) and endpoint arithmetic loses
exactness; (b) adjacency by position: needs an epsilon and a spatial search.

### 1.4 Movement per tick

Belt phase, per line in ascending line key:

1. `steps = line.carry.take_budget(slots_per_second, tick_hz)` (M2D:191-195 rule). The carry
   accumulates every tick for every line: a belt is a clock, it is never "inactive".
2. Repeat `steps` times: for `i` from `len-2` down to `0`: if `slots[i] != 0 && slots[i+1] == 0`
   then move the item to `i+1`. The last slot (`len-1`) never moves: the line end holds items.

Because the scan goes downstream to upstream, a compressed run moves as one block in the step
where its head moves. There is **no lag of one tick per piece**. One item moves at most one slot
per step. Backpressure is structural: an item moves only into an empty slot, so a blocked head
stops the whole run behind it in the same step. Nothing is created or deleted, so the belt phase
writes no ledger account.

Per-tick cost: for N pieces with K slots, each step reads N*K slots and writes at most N*K.
Steps per tick = slots_per_second / tick_hz (carry rule). Demo: 6 pieces x 4 slots = 24 reads per
step, one step per 5 ticks, so about 5 slot reads per tick. 5,000 pieces x 4 slots = 20,000 reads
per step: cost NOT MEASURED. A full line with a blocked head still costs a full scan; S:616
(sleep with explicit wake-up) is M7 work. Each line is one captured operation (section 9).

### 1.5 Endpoints on a belt

- Drop (inserter to belt): only into the **first slot of the target piece** (slot `p*K`), and
  only if it is empty and the item is in the conveyor `accepts`. Else wait.
- Pick (belt to inserter): the **downstream-most occupied slot of the target piece** whose item
  the hand accepts and the destination accepts.
- Removal of a piece at index p: the items in slots `p*K..(p+1)*K` are destroyed and counted
  (section 7). The line splits: `pieces[..p]` keeps the old key; `pieces[p+1..]` gets the key
  `pieces[p+1]`. Both halves copy the old carry. Empty halves are dropped. `piece_line` is rebuilt
  for the changed lines only.

## 2. Inserter

### 2.1 Data

Unit capability `inserter: InserterDef { reach: f32, swing_ms: u64, return_ms: u64 }` plus the
existing `inventory` capability, which is **the hand**. Stack size = hand inventory capacity.
Filter = hand inventory `accepts` (M2 rule: an empty list accepts nothing, M2D:150-151).

Reason for hand = inventory: the hand then uses the M2 ledger, transfers, capacity checks,
replication (`InventoryState`, owner-only) and the oracle stock count with no new code. An item in
a hand is ordinary `on_hand` stock. Interruption never loses it.

```rust
pub(crate) struct InserterState {
    pub(crate) source: Option<u32>,      // UnitId; None after the target is removed
    pub(crate) destination: Option<u32>,
    pub(crate) stage: ArmStage,
    pub(crate) elapsed: u32,             // ticks in the current moving stage
}
pub(crate) enum ArmStage { AtSource(Option<ArmWait>), ToDestination, AtDestination(ArmWait), Returning }
pub(crate) enum ArmWait { NoSource, NoDestination, SourceEmpty, DestinationFull }
#[derive(Resource)] pub(crate) struct Inserters(pub(crate) BTreeMap<u32, InserterState>);
```

### 2.2 Endpoint resolution (boot, `turts_sim` spatial validation)

Pick point = `pos - facing_vec * reach`; drop point = `pos + facing_vec * reach`. A point
resolves to the one non-inserter unit whose footprint circle contains it (squared center distance
<= radius squared). Belt pieces count. Zero or more than one match rejects the map with a
`ModError` on `placements[i].facing`. The source must have an output inventory, an inventory or a
conveyor; the destination must have an inventory or a conveyor; else the map rejects. Source unit
and destination unit must have the inserter's owner (M2 same-owner transfer rule, `transaction.rs:526-533`).

After boot, endpoints never re-resolve. When a target is removed, the endpoint becomes `None`.
With no construction, no other unit can contain the point (boot rejected ambiguity).

Source side of a unit: its **output inventory** if it has one, else its main inventory. Destination
side: its main inventory. So press = input (main) + output; drill = main; turret magazine = main;
lab = main.

### 2.3 Cycle (one transaction per inserter per tick)

- `AtSource`: try to pick. Candidate items = items in the source with `available =
  on_hand - reserved > 0`, in ascending `ContentId`, that the hand accepts and the destination
  accepts by filter, **and** the destination has free room now (inventory: `used < capacity`;
  belt: first slot of the piece empty). Amount = min(available, hand free capacity); from a belt,
  one item. On success: stage `ToDestination`, `elapsed = 0`. On failure: stay with a wait reason.
- `ToDestination`: `elapsed += 1`. When `elapsed == swing_ticks`, try the drop **in the same
  tick**. Drop amount = min(held, destination free room). When the hand is empty: `Returning`,
  `elapsed = 0`. Else `AtDestination(DestinationFull or NoDestination)`, retry each tick.
- `Returning`: `elapsed += 1`. When `elapsed == return_ticks`: `AtSource(None)` and try the pick
  **in the same tick**.

Cycle = swing_ticks + return_ticks when nothing blocks. Forge: 5 + 5 = 10 ticks = 2 items/s.

The pick checks the destination room but does **not** reserve it (M2D:352: no capacity lease). If
another inserter fills the destination during the swing, the arm waits holding the item.
Rejected: the designer's "pick without a destination check". It leaves items in hands at every
jam and picks items that the destination cannot take.

Effect on G section 2.3: with this rule the forge final jam has **empty hands** (each destination
has one feeder, so nothing fills it during a swing). The fixed point becomes: turret 5, lab 10,
press output 4, press input 6, belt 24, drill 4, hands 0, head job OutputBlocked with 2 ore
charged. Shells produced = 15 fired + 10 research + 5 + 10 + 4 = 44; ore charged = 44*2 + 2 = 90;
ore extracted = 90 + 6 + 24 + 4 = 124; node remaining = 176. The designer must confirm.

### 2.4 Tie-break between inserters

Inserters run in ascending `UnitId`. Each pick is its own committed transaction, so the second
inserter sees the first one's result. Two inserters cannot take the same item (S:543). Unit id
order equals map placement order (F1), so the map author fixes the order: list inserter B before
inserter C. This is a repeatable tie-break, not fairness (S:416 keeps junction fairness open).

## 3. Item-output recipes and machines

### 3.1 Decision: new `recipes` kind + `machine` capability; reuse queues, jobs, refunds

Item recipes get a **new content kind** `RecipeDef`, run by a new unit capability `machine`.
Machine jobs live in the M2 `ProductionQueues` (`production.rs:33-34`) with queue length at most 1,
use the M2 `ProductionJob` struct, `charge_costs`, `refund_charged`, `CancelProduction` and the
`OutputBlocked` stage. This follows S:410: shared primitives, different queue semantics.

Rejected: extend `ProductionDef` with an output enum. `output_unit` is a required non-`Option`
field (`crates/turts_mod_api/src/schema.rs:235-243`); changing its type breaks every existing
package RON and forces API 0.3.0 plus a migration of four mods, fixtures and the 2,083-line
generator `crates/turts_checks/examples/m2_variants.rs` (R section 8).

```rust
struct RunRefunds { running: RefundRatio, output_blocked: RefundRatio }
struct RecipeDef {
    id: String, name: String, inputs: Vec<ItemStack>, outputs: Vec<ItemStack>,
    duration_ms: u64, cancel_refunds: RunRefunds,
}
struct MachineDef { recipe: ContentRef }      // one recipe, auto-repeat
// UnitDef gains: #[serde(default)] output_inventory: Option<InventoryDef>, machine: Option<MachineDef>
```

Validation (`turts_mod_runtime`): machine needs `inventory` that accepts every input item and
`output_inventory` that accepts every output item; inputs and outputs are nonempty Item
resources; sum of inputs <= input capacity; sum of outputs <= output capacity (else the machine
can never run or never finish); machine is static.

### 3.2 Machine semantics (in `run_production`, Production phase)

Ledger: add `Endpoint::Output(u32)` and `EconomyLedger.output_inventories: BTreeMap<u32, Inventory>`.
Generalize `Transaction::inventory`, `inventory_used`, `validate_endpoint`, `account` and `commit`
to take an `Endpoint` (keep the `u32` forms as wrappers for M2 callers).

Per machine, in ascending UnitId (machines are keys of `ProductionQueues`, so the M2 loop visits
them), when enabled:

1. Queue empty: one transaction: `charge_costs(inputs from Endpoint::Inventory(machine))`, allocate
   a JobId, push a job with `stage: Running`, `elapsed_ticks: 0`, `charged = inputs`,
   `reservation: None`; event `ProductionStarted`. Not enough input: no write, status
   `InsufficientInput`. This is "charge when processing starts" with zero queue wait (S:406).
2. Running: advance as M2 (`production.rs:279-291`). At `elapsed == duration`: stage
   `OutputBlocked` and try output in the same tick (M2D:321-323).
3. Output: one transaction credits every output line to `Endpoint::Output(machine)`, all or
   none (filter + capacity), removes the job, event `ItemsProduced { machine, job, outputs }`. No
   room: no write; retry every tick; never recharge.
4. The successor starts on the next tick (M2D:251). So one cycle = duration_ticks + 1 ticks.

Cancel: `CancelProduction(machine, job)` uses the M2 path (`production.rs:145-231`) with
`running`/`output_blocked` ratios of `RunRefunds`, refund to `Endpoint::Inventory(machine)`,
reject `RefundDestinationFull` with no change. Commands run before the Production phase, so an
enabled machine with enough input starts a new job **in the same tick** as a successful cancel.
To stop a machine, the player disables it first. (G C4 says "next tick"; this design says "same
tick". The designer must confirm.)

### 3.3 Schema version: API 0.2.1, additive, with serde defaults

Decision: `ENGINE_API_VERSION = "0.2.1"`. All new fields are additive with `#[serde(default)]`:

| Struct | New fields (all `#[serde(default)]`) |
| --- | --- |
| `Exports` | `recipes: Vec<String>`, `technologies: Vec<String>` |
| `UnitDef` | `output_inventory`, `extractor`, `conveyor`, `inserter`, `machine`, `research`, `health`, `weapon`, `removal` (all `Option`) |
| `Placement` | `facing: Option<Facing>` |
| `MapDef` | `belts: Vec<BeltLinePlacement>` |

Evidence: F14, F15. The four existing packages keep `^0.2` and load unchanged. Forge declares
`engine_api: "^0.2.1"`, so an engine at 0.2.0 rejects forge. This refines one M2 rule: "every list
is required, even when empty" (M2D:98) now applies to the M2 fields only. Rejected: 0.3.0 with
required keys. It forces edits to four manifests, every unit RON, the fixtures that assert `^0.2`
(`crates/turts_pack/tests/economy_package_boot.rs:445`, `crates/turts_mod_runtime/tests/archive.rs:122`,
`crates/turts_client/tests/acquire.rs:69`) and the generator, for no M3 benefit.

Prototype constraint for M3 (state it in the spec): a unit has **at most one work capability**
among gatherer, producer, machine, research, extractor, inserter, conveyor, weapon. Removal is not
allowed on a unit with a queue (producer, machine, research). These caps bound the test matrix;
S 4.2 composition stays the long-term rule.

## 4. Static extractor on a node (drill)

Capability `extractor: ExtractorDef { per_second: u64, interaction_range: f32 }` on a static unit
with `inventory`. Boot binds the extractor to the nearest node in range (squared center distance
<= (unit radius + node radius + interaction_range)^2, M2D:271-273 formula; tie by NodeId). No
node in range rejects the map. The node item must be an Item that the inventory accepts.

State: `ExtractorState { node: Option<NodeId>, carry: RateCarry, status: ExtractStatus }`,
`ExtractStatus { Extracting, Full, Depleted }`.

Per tick (Extract phase, ascending UnitId, one transaction each), when enabled:
`budget = take_budget(per_second)`; `n = min(budget, node.remaining, free capacity)` (M2D:278);
if `n > 0`, in **one** transaction: `remaining -= n`, inventory `on_hand += n`, event `Extracted
{ node, cargo: drill, item, amount: n }` (the M2 event kind, `events.rs:20-63`). Whole unused
budget is lost (M2D:193-194). Nodes never refill (S:542, M2D:280).

Do not copy F7. Add a node overlay to the WriteSet (`nodes: BTreeMap<NodeId, u64>`) and one shared
`extract_from_node(tx, node, endpoint, item, n)` helper. M3 does not change the M2 gather code.

## 5. Minimal combat

Needs decision (CEO): G section 7 item 1 pulls a minimal combat slice from M7 into M3. The design
below assumes yes. If no, cut items 5.2 to 5.4 and the turret becomes an ammunition sink.

### 5.1 Schema

```rust
struct HealthDef { max: u64 }                                  // > 0
enum SlotRelation { Other, Same, Any }                         // hostility = mod data (S:159, S:163)
struct TargetFilter { slots: SlotRelation }
struct AmmoDef { item: ContentRef, per_shot: u64 }             // Item accepted by the unit inventory
struct WeaponDef { range: f32, cooldown_ms: u64, damage: u64, ammo: Option<AmmoDef>, targets: TargetFilter }
enum ContentsPolicy { Destroy }                                // the only M3 policy
struct RemovalDef { by_command: bool, contents: ContentsPolicy }
```

Validation: a unit with `health` must declare `removal` (death must use a declared policy, G
section 4 risk 4). Weapon is static in M3.

### 5.2 State

`Healths(BTreeMap<u32, u64>)` (current; initial = max). `WeaponState { cooldown: u32, status:
WeaponStatus, last_target: Option<u32> }`, `WeaponStatus { Ready, Cooling, NoTarget, OutOfAmmo }`.

### 5.3 Shot (Combat phase, ascending UnitId, one transaction per weapon per tick)

When enabled:
1. If `cooldown > 0`: `cooldown -= 1`.
2. If `cooldown == 0`: select the target: units with a health row, owner matching the filter
   relative to the weapon owner, squared center distance <= range squared; the smallest squared
   distance wins; tie by lowest UnitId. Re-select every shot (stateless). Rejected: sticky
   targets (more state; the C15 oracle would depend on history).
3. No target: status `NoTarget`, no write except status. Ammo `available < per_shot`: status
   `OutOfAmmo`, no shot, cooldown stays 0.
4. Shot, all in one transaction: debit `per_shot` from `Endpoint::Inventory(weapon)`; `damage =
   base + modifiers(owner slot, weapon unit def, WeaponDamage)` (checked add); `applied =
   min(health, damage)`; `health -= applied`; `cooldown = cooldown_ticks`; event
   `Fired { weapon, target, item, ammo: per_shot, damage, applied }`. If health reaches 0, stage
   the removal of the target in the same transaction (section 7, cause `Killed { by: weapon }`).
   Surplus damage is not carried.

With cooldown 20 ticks the weapon fires at t, t+20, t+40: 1 shot/s.

### 5.4 Removal (first despawn in the engine)

One function `remove_unit(tx, unit, cause)` stages a `RemoveWrite`. In the same commit:

| Store | Action |
| --- | --- |
| Main and output inventories (includes an inserter hand, a magazine) | every `on_hand` is destroyed and listed in the event. `reserved` must be 0, else reject `IntegrityViolation` (no queue holder can be removed, section 3.3, so no claim points here) |
| Belt piece | slots of the piece destroyed and listed; line split (1.5) |
| `ProductionQueues` row | none can exist (validation); assert |
| Inserter rows | the removed inserter row is deleted; every other inserter with `source` or `destination == unit` gets `None` |
| Extractor, weapon, health rows | deleted |
| `EconomyOrders` row | deleted; other gatherers whose `receiver == unit` re-select on their next tick (M2D:297). Whether the gather code tolerates a missing receiver: NOT TRACED; the removal task must test it |
| Disabled set | id removed |
| ECS entity | `world.despawn(entity)` |
| `RemovedThisTick` (new resource) | push the id; `publish_completed_tick` drains it into `CommittedTick.removed` (replaces the literal at `plugin.rs:156`) |
| Unit id | never reused (F2) |

Event `Destroyed { unit, cause: Removed | Killed { by }, items: Vec<DestroyedLine { location, item, amount }> }`.
A despawned unit gives no position (F10), and its economy records vanish from the views (F12), so
the client's existing removal path (F11) stays valid. If a unit is spawned and removed in one tick,
drop it from both lists (no M3 path does this; assert it).

## 6. Research

### 6.1 Schema

```rust
enum WeaponStat { Damage }                                     // closed engine enum: stats that exist as capability fields
enum EffectDef { WeaponModifier { unit: ContentRef, stat: WeaponStat, add: u64 } }
struct TechnologyDef {
    id: String, name: String, prerequisites: Vec<ContentRef>, duration_ms: u64,
    costs: Vec<Cost>, payment: PaymentPolicy, cancel_refunds: CancelRefunds, effects: Vec<EffectDef>,
}
struct ResearchStationDef { technologies: Vec<ContentRef>, queue_capacity: u16 }  // UnitDef.research
```

Validation: prerequisites form an acyclic graph (DFS) inside the package; `WeaponModifier.unit`
has a `weapon`; M2 cost and refund rules apply (M2D:162-165); `ProducerInventory` means the
station's inventory.

### 6.2 Jobs reuse the production queue

A research station has a `ProductionQueues` row. `Produce(station, technology)` reuses the M2
command: `queue_production` resolves the id as a technology when the unit has `research`. Extra
checks, in this order, after the M2 ownership check: technology listed by the station
(`RecipeNotEnabled`), prerequisites complete for the slot (`PrerequisitesMissing`), not complete for
the slot (`AlreadyResearched`), not in any queue of the same slot (`ResearchInProgress`). Payment,
start, advance and cancel are the M2 code paths. Completion (the output step) never blocks: one
transaction removes the job, inserts `(slot, technology)` into `ResearchState`, event
`ResearchCompleted { station, job, slot, technology }`. Server admission (F20) must accept a unit
with `research` as well as `producer`.

These forge rules are **package-scoped proposals** for open policies (S:428: cancellation,
parallel research, repeatable, sharing, stacking are Open). The engine M3 shape: per-slot state,
not repeatable, no parallel duplicate per slot. Needs decision before a second game uses research.

### 6.3 State and modifiers

`ResearchState { completed: BTreeSet<(u8, ContentId)> }`. Definitions stay in the catalog;
player state stays here (S:424). Modifiers are **computed**, not stored:
`modifier(slot, unit_def, stat) = sum of add over completed technologies of slot, over their
effects that name (unit_def, stat)`, checked. Rejected: a stored modifier table (a second source of
truth). Stacking = sum. This is the only M3 rule and forge has one effect, so the open stacking
question (S:428) does not arise; state it as Open in the spec.

### 6.4 Tick phases

```
FixedPreUpdate : Advance -> Commands
FixedUpdate    : Movement -> Gather -> Extract -> Transport -> Inserters -> Production -> Combat
FixedPostUpdate: Collect
```

New `SimSet` variants: `Extract`, `Transport`, `Inserters`, `Combat`, added to the chain at
`plugin.rs:70-73`, each with `run_if(not_stopped)`. Reasons:

- Commands first (M2D:242-244): `Remove` and `SetEnabled` take effect before every phase.
- Extract before Transport and Inserters: drill output is pickable in the same tick (same rule as
  "gather income is visible to producer start", M2D:256-257).
- Transport before Inserters: an item that reaches the belt end is pickable in the same tick; an
  item dropped this tick moves at the next step, never twice in one tick.
- Inserters before Production: an input dropped this tick can start a machine this tick; machine
  output is pickable next tick.
- Production before Combat: a research that completes this tick changes damage for shots this
  tick (G C15: "from the completion tick deal 20"); ammunition dropped this tick can fire.
- Combat last: targets have this tick's positions; destruction is visible from the next tick.

A phase with no subjects captures no operation, so M2 packages produce the **same operation
stream** as before. That is a regression test (section 9).

## 7. Remove and SetEnabled

```rust
// sim CommandKind and wire PlayerCommandKind
Remove { unit_id: UnitId }
SetEnabled { unit_id: UnitId, enabled: bool }
```

`Remove`: ownership check first (M2D:385-386), then `removal` declared with `by_command: true`
(else `NotRemovable`), then `remove_unit(tx, unit, Removed)` (5.4). Contents policy `Destroy`: every
item is deleted and counted in the `Destroyed` event. Rejected for M3: spill to ground (needs world
item piles) and refund to owner (no destination in a no-balance mod). Needs decision: G section 7
item 3 (recommendation: Destroy, mod-declared).

`SetEnabled`: ownership check, then the unit must have extractor, inserter, machine, research,
producer or weapon (else `NotToggleable`). Same value: applied, no change, no event. Change: one
transaction updates `Disabled(BTreeSet<u32>)`, event `EnabledChanged { unit, enabled }`. While
disabled:

| Capability | Effect |
| --- | --- |
| extractor | no budget taken; carry reset to 0 on disable (M2D:193) |
| inserter | stage, elapsed and hand frozen |
| machine, research, producer | head job frozen: no start, advance or output; `CancelProduction` still works |
| weapon | cooldown frozen; no shot |

Conveyors are not toggleable in M3 (frozen pieces inside a moving line add a rule for no M3 case).

New reject reasons (sim enum + `as_str`, wire `CommandRejectReason` + `ALL` + `as_str`, `wire_reason`
at `crates/turts_server/src/outbound.rs:55-85`): `NotRemovable`, `NotToggleable`,
`PrerequisitesMissing`, `AlreadyResearched`, `ResearchInProgress`. `IntegrityViolation` stays
sim-internal (never a client result).

## 8. Replication and protocol

`NET_PROTOCOL_VERSION` 2 -> 3 (`crates/turts_net_protocol/src/lib.rs:13`). Reason: postcard is not
self-describing (F17); every new field breaks old decoders.

| Record (wire) | Content | Visibility |
| --- | --- | --- |
| `UnitSnapshot.facing: Option<Facing>` | static | public |
| `Snapshot.healths`, `WorldDelta.healths: Vec<HealthState { unit, current }>` | delta: changed only | public |
| `EconomyState/Changes.belts: Vec<BeltPieceState { piece, slots: Vec<Option<ContentRef>> }>` | one record per piece | owner-only |
| `.inserters: Vec<InserterState { unit, enabled, stage, elapsed, source, destination }>` | | owner-only |
| `.extractors: Vec<ExtractorState { unit, enabled, node, status }>` | | owner-only |
| `.weapons: Vec<WeaponState { unit, enabled, cooldown, damage, status, last_target }>` | `damage` = effective value, so the client does not recompute modifiers | owner-only |
| `QueueState.enabled: bool` | added field | owner-only |
| `.research: Vec<ResearchState { slot, completed: Vec<ContentRef> }>` | per slot | owner-only (slot == session slot) |
| `WorldDelta.removed` | now filled | public |

Hand contents, magazines and machine output inventories use `InventoryState` (output inventories:
add `port: InventoryPort { Main, Output }` to `InventoryState`). Belt contents are owner-only, like
M2 inventories (M2D:449-453). This is a proposal; physical-item visibility to enemies waits for the
fog design. All records are absolute replacements keyed by unit (or slot) and use the generic
`replacements` diff (F12). Change detection is a value diff of views, not Bevy `Changed<T>`, so an
equal-value write never makes a delta.

Bytes per tick for the demo, slot 0 (ESTIMATE from probe sizes, payload only, framing NOT TRACED):
belts 6 pieces x 26-46 B every 5 ticks = 32-56 B/tick average, 160-280 B on a step tick;
inserters 4 x ~6 B while moving = ~24 B; weapon ~6 B; press and lab `QueueState` (M2 record with
elapsed) ~50 B each while running; health 2-4 B per hit. Total about 150-250 B/tick. Slot 1 gets
only public data: positions, health, removed. Instrument: `tests/net/relay.py` byte counter.

## 9. Audit log and test instruments

### 9.1 Audit

- Rule for every new phase: **one subject, one tick, one `transact`** (stage change and
  accounting together). This keeps F13 valid with no special case. The belt step, inserter stage
  advance and weapon cooldown are writes, so they also go through `transact`.
- WriteSet: add per-key overlays `BTreeMap<K, Option<V>>` (None = delete) for nodes, conveyor
  lines, inserters, extractors, weapons, healths, research, disabled; and `removed: Vec<RemoveWrite>`.
  Do not clone whole tables (the M2 `queues()` clone, `transaction.rs:273-280`, stays as is).
- `AuditState` (`views.rs:295-315`) gains `raw_output_inventories`, `raw_belts`, `raw_inserters`,
  `raw_extractors`, `raw_weapons`, `raw_healths`, `research`, `disabled`, `removed_units`. The
  catalog audit mirror gains recipes and technologies.
- `validate_records` needs no new rule if every new write is in a captured operation (F13).

### 9.2 Oracle: conservation by state and events, not a second implementation

Do not extend the M2 predictor (`accounting.rs:1541-2683`, a second implementation of gather and
production) to belts, inserters or combat. Add `crates/turts_pack/tests/common/conservation.rs`.
It reads only `AuditState` and the events of each captured operation and checks after **every**
operation:

- **O/S (per Item resource r):** `initial(r) + refunds(r) + produced(r) = stock(r) + charged(r) +
  converted(r) + destroyed(r) + fired(r)`, where `stock(r)` = main + output inventory `on_hand`
  (includes hands and magazines) + belt slots holding r + node `remaining` for nodes of item r.
  `produced` from `ItemsProduced`; `fired` from `Fired.ammo`; `destroyed` from `Destroyed.items`.
- **R:** `0 <= reserved <= on_hand` on every account; the sum of active claim lines equals each
  `reserved`; `used <= capacity` for every inventory; a belt slot holds at most one item.
- **D:** per target, `max - current = sum(applied)` over its `Fired` events; `applied =
  min(health_before, damage)`; `damage = base + sum of add of technologies completed for the
  shooter's slot at or before that operation` (the oracle computes this from the catalog and
  `ResearchCompleted` events, not from sim code); exactly one `Destroyed { Killed }` at health 0;
  no `Fired` at a unit after its `Destroyed`.
- **Refund:** for each `ProductionCancelled`, refunded = floor(charged x ratio of the job's stage)
  from the catalog.
- **Dangling:** after a removal, no side table, endpoint or claim names the removed id.

Plus exact unit tests in `turts_sim` for timing that conservation cannot see: belt transit tick,
inserter period, machine cycle (duration+1), weapon period, extractor rate with carry.
Plus the fixed-point end state (2.3) in a forge scenario test. Plus the regression: every M2
E-test passes unchanged (only protocol and API version literals change).

### 9.3 Deliberate defects (one per case; each must turn its named check red)

| Case | Plant | Red check |
| --- | --- | --- |
| C1 belt full, drill stops | drop the capacity check in `extract_from_node` | R `used <= capacity` |
| C2 press output blocked | recharge inputs on each blocked retry | O |
| C3 backpressure | belt step ignores occupancy of `i+1` (overwrite) | O (item lost) |
| C4 cancel running | call `refund_charged` twice | O and Refund |
| C5 cancel blocked | cancel in OutputBlocked also credits the output | S |
| C6 cancel research | use the `running` ratio 1/1 instead of 1/2 | Refund |
| C7 queued research cancel | skip `release_reservation` | R (claim sum) |
| C8 pause/resume | inserter advances while disabled | scenario: no state change in a disabled unit across N ticks |
| C9 remove belt piece | delete slots without listing them in `Destroyed` | O |
| C10 remove inserter | keep the hand inventory row after despawn | Dangling |
| C11 depletion | restore `remaining` from the map value at 0 | O (stock rises) |
| C12 two inserters | pick from `on_hand` read before the tick (stale snapshot) | S (duplicate) |
| C13 ammo starvation | fire with `available < per_shot`, no debit | S |
| C14 overkill | skip removal at health 0 | D (shot after 0 / no Destroyed) |
| C15 research mid-volley | run Combat before Production | D (damage at completion tick) |
| C16 leaves range | compare distance to `2 * range` | scenario: no `Fired` while distance > range |

Instrument self-tests: each conservation check gets one hand-made bad `AuditState` that must fail.
Register all plants in `tests/m3/refutations.json` with the M2 runner pattern
(`tests/m2/refutations.py`, R section 7).

## 10. Decomposition (one Codex agent per task)

| # | Task | Crate and files | Test that proves it | Depends |
| --- | --- | --- | --- | --- |
| T1 | Schema types for API 0.2.1 (section 3.3, 5.1, 6.1), `ContentCatalog` and `Resolved*`, archive accessors, manifest export paths | `turts_mod_api` `schema.rs`, `content.rs`, `lib.rs`; `turts_mod_runtime` `archive.rs`, `manifest.rs` | all existing package tests pass unchanged; a forge-shaped RON fixture parses | none |
| T2 | Portable validation of new kinds and capabilities; id uniqueness across 5 kinds; prerequisite DFS | `turts_mod_runtime/src/content.rs` (split new rules into `content_m3.rs`), `tests/content.rs` | one reject test per rule with package/file/field | T1 |
| T3 | Ledger and transaction: `Endpoint::Output`, output inventories, per-key overlays, node overlay, `extract_from_node`, `RemoveWrite` + despawn + `RemovedThisTick` | `turts_sim/src/economy/{ledger,transaction}.rs`, `plugin.rs` | transaction unit tests: atomic remove with destroyed lines; failed write leaves state equal | none |
| T4 | Boot: one `install_capability_rows` used by `stage_package`, `spawn_synthetic_units`, spawn commit; belt line expansion; inserter endpoint resolution; extractor node binding; `ItemTable`; ms->ticks helper; new `SimSet` chain | `turts_sim/src/{bootstrap,lib,plugin}.rs`, new `economy/{conveyor,inserter,extractor,combat}.rs` stubs | forge map boots; unit ids, endpoints, nodes asserted; ambiguous endpoint rejects | T1, T2, T3 |
| T5 | `mods/forge` package (renamed ids, see risk 3), visuals, E1 pass | `mods/forge/**`, `crates/turts_checks/tests` | `turts-pack mods/forge` exits 0; E1 green | T2 |
| T6 | Extract phase | `economy/extractor.rs` | C1, C11 headless; rate-with-carry exact | T4 |
| T7 | Transport phase | `economy/conveyor.rs` | transit tick exact; compression; no loss over 1,000 ticks | T4 |
| T8 | Inserter phase | `economy/inserter.rs` | period = 10 ticks; C12; held item kept when destination fills | T6, T7 |
| T9 | Machine recipes in `run_production`; generalized cancel and output | `economy/production.rs` | C2, C4, C5; cycle = duration+1 | T4 |
| T10 | Research: station queue, Produce checks, completion, computed modifier, server admission | `economy/production.rs`, new `economy/research.rs`, `turts_server/src/session.rs` | C6, C7; reject reasons | T9 |
| T11 | Combat: health, weapon, targeting, atomic shot, kill via `remove_unit` | `economy/combat.rs` | C13, C14, C16, C15 | T3, T4, T10 |
| T12 | Commands `Remove`, `SetEnabled`, reject reasons (sim, wire, server admission, outbound map) | `turts_sim/src/command.rs`, `turts_net_protocol/src/messages.rs`, `turts_server/src/{session,outbound}.rs` | C8, C9, C10; wire round trip | T7, T8, T11 |
| T13 | Views, `AuditState`, `CommittedTick`, protocol 3 records, export filter | `turts_sim/src/economy/views.rs`, `plugin.rs`, `turts_net_protocol`, `turts_server/src/export.rs` | decoded snapshot and delta for both slots: owner-only records absent for slot 1 | T6-T12 |
| T14 | Conservation oracle and forge scenario suite C1-C16, fixed point, instrument self-tests | `turts_pack/tests/common/conservation.rs`, `turts_pack/tests/forge_*.rs` | all checks after every operation; self-tests red | T13 (helper can start after T4) |
| T15 | Client replica, ingest merge, validation, facing rotation, text panels (belt counts, statuses, health, research button, enable/remove) | `turts_client/src/{replica,ingest,render,economy_ui,debug}.rs`, `turts_client_web` | client replica tests; one browser case | T13 |
| T16 | M3 refutation registry and runner, acceptance list, process test (server + two headless clients) | `tests/m3/`, `scripts/`, `turts_server/tests/` | every plant baseline green, red, restored green | T14, T15 |

Parallel groups: {T1, T3} first; then {T2}; then {T4, T5}; then {T6, T7, T9} in parallel; T8
after T6+T7; T10 after T9; T11 after T10; T14 helper may start after T4 and grow per case.
Critical path: T1 -> T2 -> T4 -> T7 -> T8 -> T12 -> T13 -> T15 -> T16 (9 tasks).

## 11. Risks and cut line

1. **Client and replication cost doubles the milestone.** Each new record touches about nine
   client places (R section 5: `Candidate`, `valid_economy`, `merge_economy`, the second merge copy
   in `ingest.rs:509-554`, browser `deepEqual` schema at `tests/browser/m2.js:356-376`, UI, debug).
   Mitigation: one generic keyed-record merge helper in T15; text panels only.
2. **The test instrument grows into a second simulator.** The M2 oracle is 3,188 lines.
   Mitigation: conservation by state and events plus small exact timing tests (9.2). A reviewer
   who asks for a full predictor must name the defect that conservation and the timing tests miss.
3. **E1 goes red on forge names.** E1 matches exact identifiers (F19). The engine will have the
   identifiers `inserter`, `range` (weapon field) and others; G names a unit `forge:inserter` and
   the map `forge:range`. Forge local ids must not equal an engine identifier or literal (for
   example `forge:arm_loader`, `forge:proving_ground`). Run E1 on forge in T5, before sim work
   lands. Also first despawn ripples: one `remove_unit` and the Dangling check contain it.

Cut line, in order (extends G section 5): (1) `SetEnabled`; (2) drone B and MoveTo (measure the
research effect by `WeaponState.damage` and shots per kill on a fixture target); (3) browser
rendering of belt items (show per-piece counts); (4) `Remove` as a player command (keep
`remove_unit` as a trusted scenario facility, like `inventory_transfer`, M2D:234-237). Must not
cut: compact belt with backpressure, item recipe with OutputBlocked, inserter hand as inventory,
atomic shot with removal, research that changes a stat, conservation checks with plants, E1 and
the M2 regression.

## 12. Commands I ran

- `cargo run --offline -q` in `scratchpad/probe-schema` (semver caret rule, serde default with RON): F14, F15.
- `cargo run --offline -q` in `scratchpad/probe-wire` (postcard record sizes): section 8.
- `rtk proxy grep -n` on the spec and M2D headings and lines; `Endpoint::` and `inventories` site
  counts in `turts_sim/src`; E1 exception list and scanner; `engine_api` in mods and crates;
  postcard and semver in `Cargo.lock`/`Cargo.toml`; whole-word scan for forge names in `crates/*/src`.
- Read: `plugin.rs`, `ledger.rs`, `economy/mod.rs`, `production.rs`, `transaction.rs` (1-822),
  `bootstrap.rs` (1-330), `ids.rs`, `map.rs` (170-192), `schema.rs` (1-330), `gather.rs`
  (60-100, 195-235, 555-620, 800-900), `manifest.rs` (15-70), `views.rs` (286-345, 422-460),
  `accounting.rs` (540-650), `economylog.rs` (575-720), `replica.rs` (400-470), `world.rs` (76-102),
  `render.rs` (286-300, 380-400), `export.rs` (15-60), `session.rs` (319-414), `messages.rs` (1-60).
- NOT RUN: `cargo test`, any build of the workspace, any benchmark. No repository file changed.
