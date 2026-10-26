# M2 economy — approved design

Date: 2026-10-26. Revision 2. Status: **Gate 1 passed after repairs; approved by the user on 2026-10-26.**
Author: technical-director, build mode. Scope: one economy slice, no script layer.

Six independent gate lenses reviewed this design. The consumer and simulation blockers and
the API/network/rigour corrections are resolved; affected seats re-reviewed the repairs.
See [review record](../../research/m2/design-reviews.md). Gate passage establishes design
readiness, not implemented M2 behavior. The user subsequently approved this economy-only
slice and its M2 policies; the implementation plan is next.

## 1. Contract and status

M2 proves the proposed section 14 demonstration: a traditional global-balance economy and a
physical-item economy work through the same public facilities. It does not complete the first
playable default mod. The source of record remains `RTS-Engine-Project-Specifications.md` v0.7.

**Decided constraints:** D01 Rust/Bevy; D02/D03 browser-first multiplayer; D04/D05 no default-mod
privilege; D14 simple local simulation; D18 server authority; D25 permanent ownership; D27/D28
slots independent of spawning/gameplay eligibility; D34 shared sandboxed scripting direction
with declarative packages allowed; D35 exact package locks; D43 agent-first tools; D44/D55 the
full default-game roster and rules. See spec:24-84, 149-165, 177-209, 315-332, 724-751.

**Baseline preserved:** D15 and sections 5.2/6.10 versioned, Bevy-free mod API; 4.4 separate
simulation, replication and visuals; 6.1 fixed ticks; 6.9/13.7 authorized selective replication;
7.2 atomic accounting; section 12 invariants 1, 2, 5, 10, 11, 12 and 13.

**Approved M2 choices (user approval, 2026-10-26):** economy-only scope; no runtime in M2; two payment
policies; owner-only private economy replication; map-boot initialization; integer accounting;
refund failure behavior; gather fallback and output blockage rules. The user approved the
revision 2 design after gate 1. The schema and rules below are the M2 implementation contract;
they do not settle the retained long-term open decisions. The approval is also recorded in
`vault/Decisions/2026-10-26 M2 scope and no script layer.md`.

**Open and retained:** scripting language/sandbox/budgets (D34, 5.3, 15), consume-over-time
production, general factories/research/logistics, mod sharing/visibility rules, final tick rate,
map limits, performance budgets, desktop delivery, package format choices and full section 16.2
playable scheduling. M2 uses the existing M1 stack and format placeholders. It selects none of
these long-term choices.

Included: per-slot balances; capacity/filter inventories and reservations; finite surface nodes;
gather/carry/drop-off with Store or Convert; map-placed static producers; timed unit production;
ChargeOnQueue and ReserveThenCharge; stage-specific cancellation refunds; browser controls;
base plus an independent physical economy package; migrated acme regression.

Excluded: player construction, power/network quantities, combat, damage/destruction, repair,
victory, general hauling/Supply, item-output recipes, belts/inserters, research, terrain/geology,
fog, bot spending, reconnect, saves, dependency resolution and scripts. The broad loops in
`docs/research/m2/economies-game-designer.md` are research, not M2 requirements. No implicit
per-slot electricity model, delivered-material construction or monument victory enters this cut.

## 2. Verified baseline and changes to make

| Current source | Verified fact | M2 change |
| --- | --- | --- |
| `Cargo.toml:19` | Bevy 0.20.0; lightyear 0.31.0; postcard, RON, serde already pinned | Keep dependencies/forbidden edges unless the gated plan proves a need |
| `crates/turts_mod_api/src/schema.rs:43` | Exports has units/maps/visuals; UnitDef has speed/footprint; MapDef has placements | Extend public schema below |
| `crates/turts_mod_api/src/lib.rs:19`; `crates/turts_net_protocol/src/lib.rs:12` | Engine API 0.1.0, protocol 1 | API 0.2.0, protocol 2 |
| `crates/turts_pack/src/lib.rs:22` | One public package-to-world path | Validate/stage all new definitions and map state on this path |
| `crates/turts_sim/src/registry.rs:90`; `crates/turts_sim/src/map.rs:32` | Speed must be positive; map creates owned moving units | Explicit static capability; atomic boot with balances, contents and nodes |
| `crates/turts_sim/src/plugin.rs:19`; `crates/turts_sim/src/command.rs:23` | Tick/drain, movement, collect phases; MoveTo only | Ordered economy phases and commands below |
| `crates/turts_sim/src/harness.rs:11` | FixedTimesteps(1), warm-up update | Reuse this tick driver |
| `crates/turts_server/src/session.rs:183`; `crates/turts_server/src/outbound.rs:50` | Receipt ack then validation; position-only deltas | Unified command sequence and atomic world deltas |
| `crates/turts_client/src/replica.rs:79` | Snapshot/chain resync; deltas cannot create unknown units | Add entity lifecycle and private economy to same chain |
| `crates/turts_client/src/input.rs:1`; `crates/turts_client/src/render.rs:94` | Click moves all owned units; rendered entities follow replica | Selection/capability controls; nodes and spawned units |
| `crates/turts_checks/tests/no_content.rs:7` | Fixed base/acme/rifleman pattern | Derived, scoped E1 instrument |

M1 design `docs/superpowers/specs/2026-10-26-m1-foundation-design.md:45` assigns both economy and
16.2 roster to M2 while leaving combat to M7. This proposal replaces that M2 scope note only;
it does not reassign the full playable to M3 or M7. Its home needs a later product decision.

## 3. Public content schema (API 0.2)

These are serde data types in `turts_mod_api`; no Bevy type or schedule crosses this boundary.
Existing PackageId, ContentId, WorldPos, Footprint::Circle, MapSize and Shape remain. `Id` below
means a full ContentId in references and a local string in definition `id` fields. Amounts are
unsigned integers in each resource's smallest declared unit. Names/units are display strings,
not arithmetic scale factors. Duplicate keys/ids/cost endpoints are rejected before maps are
built; do not let deserialization silently overwrite them.

Author-facing package and wire references are quoted qualified strings, as in M1. In the
schema below `ContentRef = String` at serialization boundaries,
not the existing derived structured serde representation of the internal ContentId type.
Validate each reference with ContentId::parse, validate its package with PackageId::parse,
then resolve it against the staged registry before constructing internal typed ids. Package
ids are nonempty `a-z`, `0-9`, `_`; local ids are nonempty and contain no colon; references
have exactly one colon. Reject empty parts, invalid package characters and unknown references
with the source file/field. Do not accept unchecked structured ids. These are M1 prototype
identifier rules, not a final identifier standard. Section 7's content references use the same
quoted-string contract. The complete public RON/package example is the companion
[M2 authoring example](2026-10-26-m2-authoring-example.md); it defines every retained form
needed to serialize a package without reading engine internals.
Definition ids are unique across resource, unit, node and production kinds within a package;
map ids are separate. Resolve each reference against its required definition kind.

```rust
type ContentRef = String; // Checked qualified reference; normalized to ContentId after validation.
// Exports replaces the old struct; every list is required, even when empty.
struct Exports {
    units: Vec<String>, maps: Vec<String>, visuals: Vec<String>,
    resources: Vec<String>, nodes: Vec<String>, production: Vec<String>,
}
enum ResourceKind { Balance, Item }
struct ResourceDef { id: String, name: String, unit_name: String, kind: ResourceKind }
struct MovementDef { speed: f32 }                 // finite, >0, M1 bounds retained
struct InventoryDef { capacity: u64, accepts: Vec<ContentRef> }
struct GathererDef {
    items: Vec<ContentRef>, gather_per_second: u64, unload_per_second: u64,
    interaction_range: f32,
}
enum DropOffEffect { Store, Convert { balance: ContentRef, per_item: u64 } }
struct DropOffRule { item: ContentRef, effect: DropOffEffect }
struct DropOffDef { rules: Vec<DropOffRule> }
struct ProducerDef {
    recipes: Vec<ContentRef>, queue_capacity: u16, spawn_offsets: Vec<WorldPos>,
}
struct UnitDef {
    id: String, name: String, footprint: Footprint,
    movement: Option<MovementDef>, inventory: Option<InventoryDef>,
    gatherer: Option<GathererDef>, drop_off: Option<DropOffDef>,
    producer: Option<ProducerDef>,
}
struct NodeDef { id: String, name: String, item: ContentRef, radius: f32 }
enum CostSource { SlotBalance, ProducerInventory }
struct Cost { resource: ContentRef, amount: u64, source: CostSource }
enum PaymentPolicy { ChargeOnQueue, ReserveThenCharge }
struct RefundRatio { numerator: u32, denominator: u32 }
struct CancelRefunds {
    queued: RefundRatio, running: RefundRatio, output_blocked: RefundRatio,
}
struct ProductionDef {
    id: String, name: String, output_unit: ContentRef, duration_ms: u64,
    costs: Vec<Cost>, payment: PaymentPolicy, cancel_refunds: CancelRefunds,
}
struct ItemStack { item: ContentRef, amount: u64 }
struct BalanceAmount { balance: ContentRef, amount: u64 }
struct Placement {
    unit: ContentRef, owner_slot: u8, pos: WorldPos, contents: Vec<ItemStack>,
}
struct NodePlacement { node: ContentRef, pos: WorldPos, remaining: u64 }
struct SlotEconomyInit { slot: u8, balances: Vec<BalanceAmount> }
struct MapDef {
    id: String, size: MapSize, placements: Vec<Placement>,
    nodes: Vec<NodePlacement>, slot_economy: Vec<SlotEconomyInit>,
}
enum VisualTarget { Unit(ContentRef), Node(ContentRef) }
struct VisualDef { target: VisualTarget, shape: Shape }
```

One item occupies one capacity unit; no mass/volume/stack-size system in M2. Empty `accepts`
accepts nothing; explicit lists avoid an accidental all-items wildcard. Resource definitions
can include unused types. Every Balance resource creates a zero account for every configured
slot at boot; no Balance definitions means no accounts. SlotEconomyInit overrides these zeros
once. It creates neither an account nor assets when a player claims a slot. Map initial items
are explicit; newly produced units start with empty inventories and no orders/queued jobs.

Static entities have movement=None. MoveTo rejects an entire group containing a static unit;
client movement controls list only mobile owned units. Gatherer requires movement and inventory;
its items must be Item resources accepted by that inventory. Store requires a destination
inventory accepting its item. Convert requires a Balance resource and positive `per_item`; no
input inventory is required on a converting receiver. One rule per input item. Producers are
static in M2. A recipe may be enabled on multiple producers. Each cost is positive, unique by
(source, resource), and kind-correct; ProducerInventory requires that producer's inventory.
Refund ratios have denominator>0 and numerator<=denominator. Under ReserveThenCharge, queued
refund must be 1/1 because no charge has happened and reservations always release in full.
Output is exactly one mobile unit, with no producer capability. No recursive spawn chain runs
without a submitted production command. Positive duration, rates, capacity and queue length;
nonempty spawn offsets and recipe references; finite range>=0, finite positive node/footprint
radii; known resource kinds; same-package references while dependencies remain unsupported.

`turts_mod_runtime` parses all lists, resolves references and validates portable rules;
`turts_sim` validates spatial/slot/state rules. `turts_pack` runs the combined server and client
paths before writing an archive. Complete registration and map validation occur in a staging
registry/world before any live registry, amount, entity or id allocator changes. A failed load
leaves the caller's previous world unchanged. Every unit and node definition has exactly one
visual; server loading never deserializes visuals. Preserve ModError package/file/field/allowed
names, including cost index and unknown-reference allowed lists. Validate all files, not only
map-reachable definitions. M1 one-map/two-slot shape, circle footprints, package isolation,
reproducible tar/hash checks and dependency rejection remain prototype constraints.

## 4. Accounting, operations and tick order

Balances, node remainder, on-hand items, reserved amounts, charged costs, refund totals and
job progress use u64. Capacity sums and all intermediate multiply/add operations use checked
u128, then checked conversion to u64. Never wrap, saturate or use floating-point resource
amounts. Reject an overflowing command with ArithmeticOverflow and no write. A background
operation which cannot credit a destination waits with a reason, retaining its source. Tick
(u32), UnitId/NodeId (u32) and JobId (u64) allocations are checked and never reused; exhaustion
halts that operation. Tick exhaustion stops the match with a host diagnostic before wrap.

Durations convert at package load: `duration_ticks = ceil(duration_ms * tick_hz / 1000)`,
checked and >=1. A rate uses `budget=(carry+per_second)/tick_hz`, with remainder modulo tick_hz.
Only active gathering/unloading phases accumulate fractional carry; whole unused budget is
lost, never banked for a later burst. New phase/order resets its rate carry. Job progress is
integer elapsed ticks. Geometry alone keeps M1 finite f32 coordinates/radii and movement.

Proposed engine records, internal to `turts_sim`:

```rust
struct Account { on_hand: u64, reserved: u64 }
struct Inventory { capacity: u64, items: BTreeMap<ContentId, Account> }
// Balance accounts use the same Account; inventory capacity counts on_hand, not available.
struct ReservationLine { endpoint: Endpoint, resource: ContentId, amount: u64 }
enum Endpoint { Slot(u8), Inventory(u32) }
struct Reservation { job_id: u64, lines: Vec<ReservationLine> }
enum JobStage { Queued, Running, OutputBlocked }
struct ProductionJob {
    id: u64, recipe: ContentId, stage: JobStage, elapsed_ticks: u64,
    duration_ticks: u64, charged: Vec<ReservationLine>,
    reservation: Option<Reservation>,
}
```

Available = on_hand-reserved; 0<=reserved<=on_hand. Reservations are claims within stock, not
extra stock or a second inventory. Only their job can consume/release them. Job records snapshot
immutable recipe/payment/refund data (or retain its frozen registry reference). No mutable
package reload exists. A queue owns its JobIds and reservations. No command accepts an owner
slot from the client; server session authority supplies it.

All writes use a single-writer transaction service. It resolves stable ids, validates access,
capability, quantities, filters, free capacity, available stock and checked arithmetic against
the latest committed state; builds a complete write set including id allocation, job changes,
spawn/despawn and event records; then commits all or none. Private staged changes cannot be
observed by replication or another operation. Never use deferred ECS Commands for half of an
accounting transaction. A plain account/inventory query cannot return mutable references.
Multi-endpoint transfers reserve no hidden extra stock and debit/credit exactly the same
amount. A transfer with source==destination or zero amount rejects. Reject duplicate
`(endpoint, resource)` lines; different resources may share one endpoint in a mixed cost.
Failed extraction,
conversion, payment, reservation, transfer or refund changes nothing, including the event log.

Public engine operations use stable ids and immutable views: BalanceView(slot,resource),
InventoryView(unit), NodeView(node), QueueView(producer), OrderView(unit), plus validated
QueueProduction, CancelProduction, AssignGather, InterruptOrder. InventoryTransfer(from,to,item,
amount) is a trusted scenario/engine facility: same owner, filters/capacity and available stock
still apply. M2 has no player teleport-transfer command and no hauling controller. E4 calls
this facility in a headless scenario. Future runtime exposure must add capability checks;
this design exposes no arbitrary credit/debit or world-mutation command to players/packages.

Per fixed tick, use explicit chained sets with committed state between sets:

1. FixedPreUpdate: increment tick; freeze admitted command batch, ordered by server admission
   ordinal. Within a session commands share one sequence space across all kinds. Drain in
   order; each successful transaction is visible to the next command, including cancellation.
2. FixedUpdate: integrate movement in ascending UnitId order; resolve gather intentions,
   extraction and delivery in ascending UnitId order. Each gatherer does at most one economic
   action and one phase transition per tick; reaching a target allows work on the next tick.
3. FixedUpdate after gathering: process producers in ascending UnitId, FIFO within each queue.
   A producer starts at most one head job, or advances one running job, or retries one blocked
   output. A newly started job has progress zero; first advancement is next tick. A completed
   job tries output immediately. Its successor cannot start until the next tick.
4. FixedPostUpdate: collect the complete committed end-of-tick change batch and immutable
   domain events. FixedLast exports filtered state and command results. No write occurs in
   the export path. Snapshot requests read a completed tick, never a partly drained tick.

Thus commands use tick-start balances; gathering income is available to producer start this
tick and new commands next tick. A second producer cannot reserve the same stock. This is a
repeatable server tie-break, not client lockstep or a claim of long-term fairness at scale.

## 5. Gathering state machine

Command `Gather(seq, unit_id, node_id)` checks owned mobile gatherer, a known finite surface
node and compatible item. Node availability is public in this M2 fixture; no geology secrecy
claim. A currently empty node rejects NodeDepleted without replacing a previous order. A valid
new Gather replaces the old intention, retains cargo and has no reservation. Full cargo or
cargo of another item accepted by its filter goes to delivery first. M2 inventories may hold
several types; delivery processes the lowest ContentId with unreserved cargo first.

Order stages: ToNode -> Gathering -> ToDropOff -> Unloading -> ToNode. Observable blocked/idle
reasons: NoDropOff, DestinationFull, DestinationOverflow, NoDeposit, Depleted. Target approach
point is at the sum of actor/target radii plus interaction_range along the line from target
to actor (positive-x direction when centers coincide), clipped to a point inside map bounds.
Range uses center distance<=radii sum+interaction_range. Movement uses the M1 straight-line
integrator. This adds no pathfinder/avoidance guarantee. Reject an invalid/unreachable outside
approach; the order waits visibly if movement cannot reach range. Nodes have no collision body
beyond interaction/production occupancy checks.

Extraction transaction: `n=min(rate_budget, node.remaining, cargo.free_capacity)`. Remove n
from node and add n to cargo together. A full cargo or depleted node initiates delivery, even
for a partial load. Nodes stay at remaining=0; never reinitialize from NodeDef. No passive
regeneration, excavating, hidden intervals or node respawn. Two gatherers racing the last item
cannot both receive it. Reservations in cargo are unavailable for delivery (ordinary M2 gather
cargo has none).

Drop-off selection: nearest same-owner receiver with a matching declared rule and currently
positive acceptance, squared distance then UnitId tie-break. A Store receiver whose inventory
is the cargo source is ineligible: never select a self-transfer, even when the gatherer also
declares a Store capability. Other gatherers may still deliver to it. Store acceptance includes filter,
free capacity and overflow; Convert acceptance includes remaining balance headroom divided by
per_item. No account auto-creation. Recheck at arrival and on every unload transaction. Partial
unload is allowed by rate budget and current acceptance. Store atomically transfers cargo;
Convert atomically consumes cargo and credits its owner's balance by n*per_item. No global
resource-name branch. Destination inputs cannot be used as a hidden remote production pool.

If all destinations are absent/full/overflowed, retain cargo and wait, report the reason,
and retry selection each tick; never discard, convert remotely or extract beyond cargo capacity.
If a chosen receiver stops accepting, select another. After cargo empties, reselect nearest
nondepleted compatible node (distance then NodeId); the originally named node is preferred
while nondepleted. If none remain, idle with NoDeposit/Depleted and keep the intention for
inspection; new Gather is needed to resume after a later scenario change. At boot with no
node, nothing auto-starts and no yield appears.

MoveTo interrupts gathering/unloading immediately in the command phase, clears intention and
rate carry, retains all cargo, and changes only the movement target. A new Gather can deliver
that retained cargo. Disconnect leaves server orders running; it changes neither ownership
nor balances. Destruction is excluded: no gameplay destruction command, damage system or
silently chosen cargo-loss/refund rule. Future removal must resolve held stock/reservations
through an explicit policy before it can delete an economic entity.

## 6. Production, cancellation and output

`Produce(seq, producer_id, recipe_id)` queues one output. Validate owner, enabled recipe,
queue capacity, every cost source and available amount. ChargeOnQueue debits every cost now;
ReserveThenCharge reserves every cost now. Both operations are atomic across the full vector.
Insufficient funds/items rejects the request; it does not insert an unpaid waiting job. A JobId
is allocated only on success and is returned in CommandApplied. Queue capacity includes the
head and OutputBlocked job. No player pause command or operating conditions exist in M2.

At ReserveThenCharge start, consume exactly the job's held reservations, debit on_hand and
release reserved amounts atomically; record charged lines. ChargeOnQueue start debits nothing.
Reservations do not expire. Only the head runs. Running advances one tick per eligible tick;
output blockage holds elapsed_ticks at duration_ticks and blocks later starts. It never charges
again or restarts progress.

Output placement: try producer-defined offsets in list order, relative to producer center.
First candidate whose entire output circle is in bounds and does not overlap any existing unit
or nondepleted surface node wins (tangency allowed). Positions after this tick's movement and
earlier producer completions count. No valid candidate => OutputBlocked. No created UnitId,
output entity or completion event until the spawn commits. Successful spawn removes the head
job atomically and emits Completed once. Production and map/synthetic loading use one shared
checked UnitId allocator; production must not create a second counter. Record explicit full
spawn entries at commit: M1 ChangedThisTick excludes fresh Position components
(`crates/turts_sim/src/world.rs:81`) and cannot be the spawn instrument. No hidden output entity sits outside the world.
M1 movement still has no collision avoidance; this is an output occupancy rule, not a new
movement collision system. Map boot checks fixture placements fit in bounds; intersecting
initial fixtures are rejected so spawn-block tests start from defined occupancy.

`CancelProduction(seq, producer_id, job_id)` may target any job of an owned producer. Refund
amount per charged line is floor(charged*numerator/denominator), using the ratio for its current
stage. Refund goes to exactly the original slot account or producer inventory. The rest stays
consumed. There is no cash-to-items conversion or refund to a different store.

| Current stage | ChargeOnQueue | ReserveThenCharge |
| --- | --- | --- |
| Queued, including head before start | Refund declared queued fraction | Release all reservations; no income/spend/refund entry |
| Running | Refund declared running fraction | Refund declared running fraction of charged input |
| OutputBlocked | Refund declared output_blocked fraction | Refund declared output_blocked fraction of charged input |
| Completed, cancelled or unknown id | Reject UnknownJob; never repeat refund | Same |

Preflight all refund destinations. Inventory full/filter conflict or balance overflow rejects
cancellation as RefundDestinationFull/RefundDestinationOverflow and leaves job, progress,
reservation and accounting unchanged. There is no silent loss, ground pile, pending refund or
capacity lease. A smaller permitted refund can fit when a full refund cannot. On success remove
job and release any remaining reservations in the same transaction. Refund rejection does not
pause a running job; its ordinary tick advancement still occurs. A same-batch Produce then
Cancel can cancel a queued head before start. Cancelling a waiting non-head uses Queued policy.
A Cancel received before the completion tick applies before progress and can prevent output;
one received after completion rejects. Disconnection is not cancellation.

## 7. Commands, replication and event boundary

Protocol 2 retains Hello/Welcome/JoinRejected, CommandAck and ResyncRequest. Replace separate
MoveTo transport messages with one `PlayerCommand { seq:u32, kind: PlayerCommandKind }`:

```rust
enum PlayerCommandKind {
    MoveTo { unit_ids: Vec<UnitId>, target: Pos },
    Gather { unit_id: UnitId, node_id: NodeId },
    Produce { producer_id: UnitId, recipe_id: ContentRef },
    CancelProduction { producer_id: UnitId, job_id: JobId },
}
struct CommandApplied { seq: u32, tick: u32, job_id: Option<JobId> }
struct CommandRejected { seq: u32, tick: u32, reason: CommandRejectReason }
```

Channel mapping is explicit: Hello, Welcome, JoinRejected, PlayerCommand, CommandAck,
CommandApplied, CommandRejected, Snapshot and ResyncRequest use the existing bidirectional
ReliableChannel with OrderedReliable delivery (message direction remains as declared).
WorldDelta alone uses server-to-client StateChannel with UnorderedUnreliable delivery.

CommandAck is sent on receipt only; application/rejection follows tick validation. Keep M1
MoveTo errors plus NotMobile, MissingCapability, UnknownNode, IncompatibleItem, NodeDepleted,
UnknownRecipe, RecipeNotEnabled, QueueFull, InsufficientBalance, InsufficientItems, UnknownJob,
RefundDestinationFull, RefundDestinationOverflow, ArithmeticOverflow, IdExhausted and BadSequence.
Wire reasons are a closed serde enum with stable snake_case UI/debug keys. Ownership checks
precede queue/cargo details for another owner's entity. A connection without a session cannot
submit commands. Receivers merge message kinds through the one queue. Sequence numbers increase
strictly per session; duplicate/out-of-order seq gets BadSequence and no reapplication. Client
must reconnect before sequence exhaustion (reconnect product support remains deferred). No
assumption that transport receipt means successful payment.

Use one atomic world chain, extending the existing position-delta mechanism, rather than two
independently applied streams which can show a debit without its spawned output:

```rust
struct Snapshot {
    tick: u32, units: Vec<UnitSnapshot>, nodes: Vec<NodeState>, economy: EconomyState,
}
struct WorldDelta {
    tick: u32, base_tick: u32, spawned: Vec<UnitSnapshot>, removed: Vec<UnitId>,
    positions: Vec<UnitDelta>, nodes: Vec<NodeState>, economy: EconomyChanges,
}
struct NodeState { id: NodeId, content_id: ContentRef, pos: Pos, remaining: u64 }
struct BalanceState { resource: ContentRef, on_hand: u64, reserved: u64 }
struct InventoryState { unit: UnitId, items: Vec<BalanceState> }
struct OrderState { unit: UnitId, node: Option<NodeId>, receiver: Option<UnitId>, stage: GatherStage }
struct QueueState { producer: UnitId, jobs: Vec<JobState> }
struct JobState {
    id: JobId, recipe: ContentRef, stage: JobStage, elapsed_ticks: u64,
    duration_ticks: u64, charged: Vec<Cost>, reserved: Vec<Cost>,
}
struct EconomyState {
    balances: Vec<BalanceState>, inventories: Vec<InventoryState>,
    orders: Vec<OrderState>, queues: Vec<QueueState>,
}
// Same four lists; each listed account/entity record replaces that whole record.
// Empty items/jobs, or an explicit Idle OrderState, clears the old contents.
struct EconomyChanges { /* balances, inventories, orders, queues as above */ }
```

NodeId(u32) and JobId(u64) are stable newtypes; UnitId remains u32. UnitSnapshot retains
content_id, owner_slot and pos. GatherStage carries ToNode, Gathering, ToDropOff, Unloading,
Idle and Waiting{reason}; reason is one of the section 5 stable statuses. Immutable definition
data provides capacity, capabilities, names, accepted items and recipe costs. Zero balance
accounts are included; empty inventories/queues/orders are explicit for owned capabilities.
Do not send reservation tokens or mutable references; aggregate reserved amounts and job lines
are sufficient views. Server-only reservation records remain authoritative.

Snapshot is reliable at Welcome/resync and includes all fields from one completed tick.
WorldDelta uses the existing StateChannel every tick, including empty heartbeats, base_tick
=tick-1. Changes are absolute record replacements, never arithmetic deltas applied twice.
Apply a batch atomically: validate ids/content, spawn-before-position, private references and
base_tick; then replace/remove all included records. An unknown position/inventory target
without a spawn is a chain break. Ignore duplicate/stale batches. At a break request one resync,
buffer as in M1, freeze latest coherent world, apply a snapshot, drop <=snapshot tick and replay
only a contiguous buffered chain. Stale snapshots must not roll back an already coherent newer
world. Preserve initial Welcome/snapshot ordering and idle-resync recovery. Buffers have an
operator-configurable technical bound. On overflow discard the buffered tail, freeze the last
coherent world and mark a gap. Coalesce recovery behind the outstanding ResyncRequest; never
issue another while that response is pending. Continue retaining only a bounded contiguous
tail. When the reliable response arrives, clear the outstanding marker, apply a valid snapshot
and replay only its contiguous tail. If the discarded gap still prevents convergence, issue
one successor request; otherwise finish recovery. Stale responses cannot roll back state and
may cause that same single successor if recovery is still needed. Never apply a partial tail.
The numeric bound is a test/config parameter to record,
not a gameplay cap. Test loss at extraction, conversion, charging, refund and spawn, including a
lost final economy change followed by empty heartbeats and late snapshots.

Filter **before serialization** by receiving Session.slot, for both snapshots and deltas:
balances, inventories, order internals, queues/costs/progress are owner-only, including offscreen
owned entities. Other units' public identity/position and all M2 surface node state remain
visible as in M1's open-map fixture. No enemy economy lives in the client debug hook, encoded
payload or cache. This is the M2 authorization proposal, not a universal mod secrecy policy.
Tests inspect decoded messages for both slots, not just hidden UI. Sharing/fog policy waits for
its own design. Package map data is public in this fixture; M2 does not prove hidden deposits.
Register all protocol-2 messages in the shared `turts_net_lightyear` plugin in one fixed order;
both ends reject old protocol/API/package locks before admission.

Event boundary: `EconomyEvent { tick, ordinal, owner_slot, kind }`, emitted only after commit.
Kinds: Extracted(node,cargo,item,amount), Stored(from,to,item,amount),
Converted(from,to,item,amount,balance,credited), ProductionQueued(producer,job,recipe),
ProductionStarted(producer,job,charged), ProductionCompleted(producer,job,output_unit_id),
ProductionCancelled(producer,job,released,refunded). Include complete resource/source lines;
rejections are command results, not successful domain events. Events form one immutable batch
per completed tick, ordered by transaction ordinal; consuming a batch cannot mutate that tick.
The server diagnostic sink can write JSONL; no event archive, replay service or player event
stream is required. Events are not the replicated state source or a persistence contract.

A future runtime may subscribe to these typed batches and submit validated operations for a
later tick. M2 supplies no interpreter, guest executable, package event handlers, registered
callbacks, hooks per entity/item/tick, runtime subscription API or arbitrary closure execution.
Gather/production loops stay Rust. The batch boundary is a seam, not a claim of completed
section 5.4 sandbox/event budgets. Existing research `scripting-probe-results.txt` is archived
native/Node-V8 evidence, not a new browser run or a language selection; its source/machine
metadata is not self-contained in that report. D34 remains intact and deferred.

## 8. Data packages and browser use

All following values are **fixture proposals**, separate from budgets and game balance:

| Fixture | Resources and starting state per slot | Gather/drop-off | Production |
| --- | --- | --- | --- |
| base 0.2.0 | ore Item; credits Balance; 500 credits; one static command_center, one empty harvester | Cargo 100 ore; gather 20/s, unload 50/s, speed 2, range 0.5; CC Convert 1 ore -> 1 credit | rifleman cost 100 credits/5,000 ms; harvester 600/15,000 ms; ChargeOnQueue; refunds queued 1/1, running 1/2, blocked 1/1 |
| haul 0.2.0 | ore Item only; static depot with 100 ore/capacity 2,000; two empty workers | Cargo 10; gather 5/s, unload 10/s, speed 3, range 0.5; depot Store | worker cost 50 ore from that depot/6,000 ms; ReserveThenCharge; queued release, running 1/1, blocked 1/1 |
| acme 0.2.0 | No resources, nodes, accounts or economic capabilities | Existing scout movement/shape/map | No production |

Base uses a 64x64 map, haul 48x48, two slots with mirrored starts; finite nodes of 1,000 ore
per side. Define exact positions/radii/offsets in package data with at least two in-bounds free
spawn choices. Suggested fixture geometry: radius 2 producer at (8,8), gatherer at (12,8), node
radius 1 at (24,8) for base and (20,8) for haul; mirror x for slot 1. Thus base nodes have x=24/40
and haul nodes x=20/28, with no coincident mirrored nodes. Place haul's second worker at
(12,10), also mirrored. Output offsets are (0,4), (0,-4). Rifleman radius 0.4,
harvester radius 0.8, haul worker radius 0.5; each has a simple package visual. Base rifleman
speed 3; CC/depot movement None; queues capacity 5. All fixture types receive names in data.
No construction/power expense is available, so 500 credits proves neither the full starting
power rule nor the full 16.2 game. No match result is claimed when nodes run out.

Package layout extends M1 with resources/*.ron, nodes/*.ron, production/*.ron, all declared in
Exports. Both economies select data, no .rs/Cargo.toml/native hooks. A third independent author
must be able to express a balance-only cost, inventory-only cost, Store/Convert receiver and
both payment policies from sections 3-7 and the complete companion authoring example. No dependency library or special package identity is
required. Role-swap fixtures must add needed Balance definitions/inventories/cost endpoints;
simply swapping enums into ill-typed content is not a valid E3 test.

API migration is intentionally breaking: ENGINE_API_VERSION=0.2.0, NET_PROTOCOL_VERSION=2;
base/acme/haul package version 0.2.0, engine_api='^0.2'. The workspace crate release version may
follow 0.2.0 in the implementation plan; it must agree with the advertised compatibility policy.
Explicitly migrate every manifest export list, unit movement/options, map contents/slot_economy/
nodes and VisualTarget; acme uses empty lists/None. No silent v0.1 defaults or save migration.
Regenerate package archives, hash/size locks, advertisement fixtures and protocol tests; never
reuse an old lock hash. Old API/protocol rejection and malformed new fields remain tested.

Generic browser interaction: left click selects one owned unit/static producer (ground clears);
right click ground moves selected mobile units; right click a node with a selected gatherer
submits Gather. Hit test unit/node circles in map coordinates; UI clicks never issue world
orders. Producer selection displays data-derived recipe buttons; one click queues one unit.
Queue rows display JobId, recipe name, stage, progress, reserved/charged source and cancel button.
Display account available/on-hand/reserved; selected inventory on-hand/reserved/capacity; node
remaining; gather status/wait reason; last command result. No balance panel appears when the
package defines no Balance resources. Labels/types come from downloaded definitions; command
verbs/status strings are engine UI. Show source hints such as 'this inventory' rather than a
false pooled total. Disable known-inapplicable actions, but always let the server validate.

Use the same Rust client logic native and wasm; a thin browser DOM panel may present these
controls and call the existing wasm bridge pattern. It reads replica/views only. Native window
controls must expose the same actions through a generic panel; headless driver submits the same
command types. The builder may choose Bevy UI for both instead; that is a presentation-only
choice and must keep the E9 actual-text oracle. No invented account values, local economic
prediction or debug-only interaction counts as completion. Preserve M1 scene/camera and
position interpolation tests. Extend browser hooks with permitted economy and panel/action
state; E9 must also inspect actual visible DOM/Text nodes.

## 9. Exact acceptance contract E1-E10

These refine the producer report's E1-E10 without renumbering. QA owns the instruments and
must prove each can fail by the named deliberate defect. E2/E3/E6 require nonzero extraction,
delivery, payment, cancellation and at least one completed spawn where applicable; equality
of empty logs or zero-state invariants is a failure. Use fixed package/seed/command traces,
canonical sorted JSONL and integer assertions, not wall-clock timing as an accounting oracle.

| ID | Input and required result | Refutation plant / red build |
| --- | --- | --- |
| E1 derived no-content | Parse every mods manifest/definition; nonempty token/provenance list includes package/local/qualified ids, names, resource types and package paths. Scan production Rust literals/paths and distinctive identifiers; full qualified ids, package archive paths and exact distinctive name/local-id literals block. Common schema words such as acme's `field` require an explicit reviewed generic-word allowlist with provenance; do not substring-match every Rust identifier. Comments/test fixtures are separately scoped. A malformed/missing definition or empty derived set fails. | Plant qualified id, new ore/credit/harvester literal and a new alt package path in temporary source, including inline-comment code. All must fail. Generic `field` variable must pass. Rename fixtures E2 backstop generic-token exceptions. |
| E2 renamed clone | Generate a valid clone of base AND a third physical package: change package/local ids, names and every reference with a recorded mapping. Same commands => identical ticks, stages, amounts, outcomes and events after mapping; positive gather/delivery/spawn required. | Add package-id branch, fixed content lookup or default balance; renamed run diverges or fails. No-op simulation fails positive-progress oracle. |
| E3 role swap | Valid fixture copies exchange Store/Convert and ChargeOnQueue/ReserveThenCharge, adding correct resources/cost sources/inventories. Both policies run against balance AND item costs. Behavior follows data, including reserve vs debit timing and cancellation. | Bind policy to package/kind, ignore declared refund ratio or hard-code conversion; exact trace differs. |
| E4 locality | Put ore only in depot A; B Produce rejects InsufficientItems and A/B remain unchanged at command boundary. Trusted atomic transfer A->B decreases A/increases B by n. B then produces consuming B only; A stays at its post-transfer count. | Pool all owner's stores; reject valid local transfer; or debit A during B production. This repairs the research's contradictory 'A unchanged after transfer' wording. |
| E5 kind separation | Base production debits credits and no item inventory; base conversion consumes cargo ore and increases the credits account. Haul has zero balance accounts and spends local items. Acme remains empty. | Auto-create credits/default account, treat item stock as account, or spend cargo remotely. |
| E6 accounting and interruption | Assert equations below after every committed operation and tick. For two jobs contending for last stock under ReserveThenCharge, require one accepted enqueue, one reservation and unchanged on-hand stock before start. Under ChargeOnQueue, require one accepted enqueue, one debit and zero reservations. Both reject the other enqueue for insufficient available stock (receipt ack is not acceptance). Test mixed-cost atomic failure, capacity/filter/access, failed transfer, partial unload, finite depletion, full/no receivers, MoveTo interrupt with cargo, cancel Queued/Running/OutputBlocked for both policies and full/overflow refund rejection; start/blocked retry never recharges. Include a gatherer with its own Store and a separate valid depot: it must skip the self-transfer and deliver to the depot. At least one refund and successful completion. | Remove reservation check; debit before validating second cost; recreate depleted node; permit overflow; lose cargo; duplicate conversion/refund/spawn; ignore destination capacity; select self as Store receiver and stall. Each named plant must be observed red. |
| E7 claim neutrality | At same simulation boundary, claim each slot and compare entity/node set, accounts, inventories, reservations and queues to preclaim. Include initialized and empty slots. Claims create/change none; ownership claims alone differ. | Grant balance/cargo/entity on claim or reset map economics. Use paused tick boundary/control run so unrelated progress is not mistaken for claim mutation. |
| E8 validation/authority | Foreign gatherer/producer/job, unknown ids, incompatible node, disabled recipe, static movement, malformed amounts/NaN/out-of-map movement, overfull queue, sequence duplicate, insufficient funds, old API/protocol. Receipt ack precedes rejection for admitted command envelopes. Full transactional write set unchanged; automatic work compared against no-command control. | Skip owner/recipe/capacity/sequence check; partial commit; expose foreign queue details in rejection. Compare rejection at command boundary, not across unrelated gather/progress ticks. |
| E9 browser control and resync | Same browser build runs base/haul/acme: actual visible names/types match package data; haul/acme show no balances; select/gather/queue/cancel through real controls, observe server amounts, stages and rendered new units. Two clients receive only own private economics, including late snapshot. Drop world deltas at extraction, conversion, payment/refund/spawn and final stop; hold snapshot; final full state converges without double effects or resync storm. Add a small-buffer overflow case with a delayed snapshot: stored batches never exceed the configured bound, outstanding requests never exceed one, and a successor is sent only after the prior response if a gap remains. Assert already-connected and late/resync clients see the same produced id/type/owner exactly once; failed/cancelled jobs spawn none. Inspect encoded/decoded payload authorization and actual panel text, not debug data alone. | Hard-coded label; absent/broken buttons; hidden enemy state still sent; position-only snapshot; suppress output spawn; failure to spawn from delta; double refund on replay; missing final-loss recovery; unbounded buffering or duplicate outstanding resync requests; debug-only panel. |
| E10 public path and independent author | mods has no .rs/Cargo.toml; server and packer use only turts_pack::load_package. mod-builder authors a third economy from this document and its complete public authoring companion without engine edits/private instructions; E2/E6 pass on it, and pack/browser loading succeeds with useful errors on bad refs. | Native/default hook, bypass loader or missing schema/rule documentation forces engine edit/author guess. Fixture package starts from these public documents, not a copied native test world. |

Accounting equations, per resource and explicit operation reason (checked integer diagnostic
counters, not gameplay resources):

- Items: `initial_items + extracted + created_by_explicit_operation = on_hand_all_inventories
  + net_production_consumed + converted + destroyed`. M2 has no generic item creation or
  destruction; those terms stay zero. `net_production_consumed = gross_item_charges - refunds`.
  Node remainder is not inventory: separately `initial_node = remaining + extracted` per node.
- Balances: `current = initial + conversion_income + refunds - gross_charges` per slot/type.
- Reservations: sum job reservation lines equals endpoint reserved; reserved<=on_hand; do not
  add reserved to on_hand in the conservation equation. Refunds reclassify consumed stock;
  they are not new extraction. Converted items are an explicit sink; initial stock is counted.

Additional required regressions run under these IDs: malformed package leaves no live state;
old API/protocol rejected; resource-kind mismatch; duplicate ids/keys; checked MAX arithmetic;
multiple-resource reservation rollback; refund rounding; blocked output occupancy/ID allocation;
headless boundary checks and acme M1 T1-T8 expectations with protocol/schema fixtures updated.
Generalize T7's one-unit expectation to package placements plus committed outputs; keep its
actual mesh/camera oracle. The M1 convergence threshold remains an existing regression setting,
not a newly approved supported-capacity budget.

## 10. Evidence, planning seam and remaining questions

Use 20 Hz, one tick/update headless, two slots and fixture dimensions above for correctness.
Repeat duration/rate/rounding cases at 10 and 30 Hz. Proposed bounded completion test timeout is
2,000 simulated ticks; enlarge fixture-specific traces explicitly if needed. Randomized rename
and accounting scenarios record seed and input; never interpret a timeout as an allowed stalled
job. Report bytes/tick, snapshot size, tick distribution, browser frame/memory/download figures
with machine/browser/build/package hashes if measured. No performance gate, capacity promise,
new GCP run or runtime selection follows from these fixture values (spec 13.2/15).

Implementation responsibilities preserve M1 boundaries: mod-runtime-engineer schema/archive
validation; simulation-core-engineer ledger, inventory, transaction/order/production systems;
network-engineer protocol/export/resync; game-designer base data; mod-builder haul plus independent
third package; bevy-rendering-engineer/web-platform-engineer client; qa-lead refutation plants.
The next artifact is the gated implementation plan, including tests before mechanisms and
browser validation. This document changes no product code.

No unanswered product question blocks planning this approved economy slice.
Approved calls: map boot initialization; owner-only replication; integer
rate/duration units; queued reservation/charge-at-start; reject cancellation when refund cannot
fit; retain cargo/retry full receivers; FIFO blocked output and ordered tie-breaks; API/protocol
breaking migration; no scripts. These are approved M2 choices, not changes to the source
specification's long-term open decisions.

Full section 16.2 playable scheduling remains an open CEO/product call: construction, power,
combat, engineer repair and victory still need a validation home and their own policies. The
scripting language goes to decision when a concrete rule needs scripts, with reproducible
browser/native probes and sandbox requirements; archived microbenchmarks do not decide it.
Desktop packaging and repository/package licence remain carried open questions. They do not
silently expand M2 or authorize implementing a runtime.
