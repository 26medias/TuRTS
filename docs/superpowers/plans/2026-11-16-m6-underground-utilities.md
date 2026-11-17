# M6 underground utilities: implementation plan

Spec: `docs/superpowers/specs/2026-11-16-m6-underground-utilities.md`. Reviewer: the gate-1 `world-engineer`.
Branch `codex/m6-underground-utilities` in the one worktree `TuRTS-m6-utilities`. No other worktree.
Builds use `~/.codex/bin/turts-cargo` under `flock -s /tmp/turts-quiet-performance.lock`, 4 jobs.
Evidence: `/home/julien/Projects/TuRTS/target/mc-evidence/m6-utilities/`.

## Ownership and order

| Step | Owner | Files (only these) | Depends on | Check |
| --- | --- | --- | --- | --- |
| C0 contract | root | `turts_mod_api/src/utilities.rs`, `turts_rules/src/utilities.rs`, `turts_net_protocol/src/utilities.rs` (done); then the variant ripple below | gate 2 | workspace `check` and wasm32 check compile |
| A1 runtime + fixture | builder A1 (Sonnet) | `turts_mod_runtime/src/utilities.rs` (new) + call sites in `archive.rs`/`unbuilt.rs`; `mods/utility_yard/` | C0 | UT1 |
| A2 sim | builder A2 (Sonnet) | `turts_sim/src/utilities/` (new); hooks in `orders/apply.rs`, `economy/power.rs`, `economy/transaction.rs` (WriteSet), `economy/views.rs` (WorldView), `terrain/state.rs` + `terrain/mod.rs`, `plugin.rs`, `bootstrap.rs` | C0 | UT2-UT5, UT7 |
| B wire + server | root | `turts_server/src/export_utilities.rs` (new) + its call sites in `export.rs`, `export_visibility.rs`, `outbound_knowledge.rs`; privacy tests | C0, A2's `owner_view(&WorldView, slot)` | UT6 |
| C client | builder C (Sonnet) | `turts_interaction/src/utility.rs` (new) + `lib.rs` hooks; `turts_client/src/utilities/` (new) + replica, widget source, input hooks | C0 | client unit tests, wasm32 check, UT8 driver |
| D browser + launcher + report | root | `tests/browser/utilities.js`, `scripts` launcher copy, report, README | A, B, C | UT8, cold-play |

A1, A2 and C run in parallel after C0. B starts when A publishes `owner_view`. Shared files get small hooks only.
Root runs the checks of each step. No per-task review. One code-running foundation review at the end.

## C0 contract ripple (root)

- `Exports.utilities: Vec<String>` (default, skip if empty). `ContentCatalog.utilities: BTreeMap<ContentId, ResolvedUtility>`.
- `SupplyScope::Network`. `Behaviour::UtilityRoute`. `TargetKind::UtilityRoute`.
  `Source::UtilityRoutes { order }` in the interface schema.
- `OrderTarget::UtilityRoute(UtilityRouteTarget)` on the wire. `CommandRejectReason` and sim `RejectReason`
  get `UtilityNotObserved` and `UtilityLimitReached`.
- `SnapshotRecords.utilities` and `DeltaRecords.utilities`: `Option<UtilityView>`, `#[serde(default)]`.
- `ENGINE_API_VERSION = "0.2.18"`, appended to `RELEASED_ENGINE_APIS`. `NET_PROTOCOL_VERSION = 11`.
- Version-table rows `since_api: "0.2.18"`: raw `utilities` export, `Variant("SupplyScope::Network")`,
  `Variant("Behaviour::UtilityRoute")`, `Variant("TargetKind::UtilityRoute")`, `Variant("Source::UtilityRoutes")`.
- Fix every exhaustive match with the obvious arm (refuse, `false`, or "None") so the workspace compiles.
  The packer refuses the new behaviour until A wires it (existing unbuilt rule), then A removes that refusal.

## A: runtime validation (spec "Public authoring")

`install_utilities` beside `install_terrain` in `turts_mod_runtime`. Checks with field paths:
depth range; `0 < max_segment_length <= 64`; `reach > 0` and finite; `carries` resolves to a `Supply` resource
with `Network` scope; one utility per Network resource and one Network resource per utility; ports name unit
types, no duplicate unit, finite offset within `footprint radius + 2`; every unit with a Network resource in
`provides`/`consumes` has a port of its utility; costs name `Balance` resources and amounts are nonzero;
colours parse; constructor `when_short` with a Network resource is refused; a `UtilityRoute` order uses target
`UtilityRoute` and lists only unit types. Tests: one refusal case per check, plus `utility_yard` loads.

## A: simulation module `turts_sim/src/utilities/`

- `state.rs`: `UtilityState` resource. For each slot: `next_node`, `next_segment`, `nodes: BTreeMap<u32, Node{utility, pos}>`,
  `segments: BTreeMap<u32, Segment{utility, a: End, b: End, elevation, depth, cells: Vec<(u32,u32)>, state}>`,
  where `End` is `Node(u32)` or `Port(unit u32)`. A cell index `BTreeMap<(u32,u32), BTreeSet<(slot, segment)>>`.
  Installed by bootstrap when the catalog has utilities, removed on reset like terrain.
- `route.rs`: `apply_route(world, slot, command, unit_ids, entity_by_unit)`, called by one early branch in
  `orders::apply::apply_order`, like `cancel_sites`. Checks in the spec order. Surface reads use the current
  `TerrainState` surface; a map without terrain uses elevation 0 and one virtual flat grid of 1-unit cells over
  the map. Observation uses `terrain::step::observed` with the Completed phase of the current tick, the same
  helper as work orders, with the Completed basis of the previous tick. Payment uses `economy::transaction::transact` with `charge_costs` on `Endpoint::Slot`.
  The utility write is a new `WriteSet` field, applied in `Transaction::commit` next to `terrain_step`
  (transaction.rs:1189-1193), so the debit and the topology change are one audited operation.
- Terrain hook: `TerrainState.changed_cells: BTreeSet<Cell>` beside `dirty_chunks`, inserted by
  `apply_prevalidated` (state.rs:292) and cleared in `terrain::begin_tick` (mod.rs:148-152). Read-only accessor.
- New `SimSet::Utilities`, after `SimSet::Outcome` in the FixedUpdate chain (plugin.rs), and in `TICK_ORDER`
  (plugin.rs:509-528) so the order test pins it. One exclusive system
  `utilities::run_pass` runs in order: port removal, exposure, networks.
- Port removal: a port end whose unit is not a live completed own unit becomes a new junction at the old port
  point, in ascending segment id order; at the junction limit, that segment is deleted instead.
- `exposure.rs`: check the segments in the cell index for `changed_cells`, plus the segments made or split this
  tick (a `pending_check` set filled by `route.rs`). Recompute exposure over each segment's cells and apply the
  policy. `checked_segments` counter (operator-only accessor) for UT3.
- `network.rs`: a dirty set of (slot, utility) pairs, marked by route, split, removal, exposure change and
  port conversion. Rebuild components (union-find over conducting segments, junctions and port units) only for
  dirty pairs. Then enumerate all live completed port units every tick (a unit with no conducting segment is
  its own network) and sum totals in checked `u128` for every network from the live completed port units'
  `provides`/`consumes`, saturating the published `u64`. Write `next: UtilityNetworks { by_unit:
  BTreeMap<(u32 unit, ContentId resource), bool short>, rows }`.
- `power::run_supply` swaps `next` into `effective` (one call into `utilities::begin_tick`, outside the early
  return when `OperatingResources` is absent). All readers use
  `effective`. A port unit with no `effective` row is its own network (`consumes > provides` of itself).
- Power hooks (`economy/power.rs`): `OperatingResources::from_catalog` and `prepare_activation` skip Network
  resources. `weapon_enabled` and `rate_for` call `utilities::network_short(world, unit, resource)` for a Network
  resource. Producer view rates already use `rate_for` at capture (views.rs:527). A2 adds the utility state
  (topology plus `effective` networks) to `WorldView` in `world_view` (views.rs:770, :920) and its `PartialEq`
  (views.rs:707-755). `owner_view(&WorldView, slot)` builds the plain owner rows, so the server never needs `World`.
- Runtime refusals added in `install_utilities`: port on a unit with `movement`; producer or weapon Network
  `when_short` on a unit type with no port of that utility.
- `owner_view(&WorldView, slot) -> UtilityOwnerView` (sim-side plain types) for B, in ascending id order.
- Tests (in `utilities/tests.rs`): UT2, UT3 (all three policies, cover vs elevation through real paid work
  orders on `utility_yard`, checked-segment counter), UT4 (each refusal leaves state and balances byte-equal;
  foreign and absent ids give equal results), UT5 (exact totals for chains, splits, several networks; reversed
  creation and command order; same-build replay; slot supply regression).
- UT7: an `#[ignore]` measurement test that builds 2 slots × 512 segments and 512 junctions, edits 16 cells
  under lines each tick for 2,000 ticks, and writes p50/p99/max pass time and view bytes to a JSON path from
  an env var; plus a synthetic 8-slot harness over the same pass functions. Root runs both under the
  exclusive lock.

## A: fixture `mods/utility_yard/`

Copy `mods/terrain_stockyard` (art, models, credits unchanged). Rename ids. Add `resources/power.ron`
(Supply, Network), `utilities/power_line.ron`, units `generator` (provides 10) and `workshop` (consumes 6,
producer of one porter, `when_short` Rate 1/4), depot as builder in `orders/route.ron`, HUD list
`UtilityRoutes`, and a map: generator west, workshop east, a slot-1 base apart, ridge in the middle, starting
loam balance for line costs and fill. README states the player steps and limits.

Audit: `turts_audit` keeps refusing `UtilityRoute` logs (spec limit). UT5 replay compares `UtilityState`.

## B: wire and server

`export_utilities.rs`: map `owner_view` to `UtilityView`; put it in `SnapshotRecords.utilities` for every
snapshot path (AllVisible `snapshot_for` and the permitted baseline), and in `DeltaRecords.utilities` only
when it differs from the last view sent to that viewer (keep the last view per viewer next to the existing
per-viewer baseline). `orders::to_sim` maps the target. Reset clears per-viewer state.
Tests: UT6 paired worlds; viewer 1 encoded snapshot and every delta byte-equal; viewer 0 sees own rows only;
a slot-1 command naming a slot-0 node id gets the same result as an absent id; resync and reconnect.

## C: client

- `turts_interaction/src/utility.rs`: `State { view_on, route: Option<RouteMode{order, utility, depth, start}> }`.
  `InputEvent::UtilityRoute{order, utility}` enters route mode and turns the view on. Engine actions
  `utility_view` (U), `utility_deeper` (]), `utility_shallower` ([) in the engine input map; `cancel`, right
  click, focus loss end route mode. Left click resolves the endpoint by the snap order of the spec from the
  `WorldView` utility rows, then a second click sends `OrderEnvelope{target: UtilityRoute, queue: Replace}`
  with the selected eligible units. Pure tests for each transition.
- `turts_client/src/utilities/`: `store.rs` stages and commits `UtilityView` with the transaction (replica
  hook beside terrain); `render.rs` gizmo lines at `elevation × step` world height, symbols, port squares,
  dashed disconnected lines; `fade.rs` swaps `TerrainTile` materials to one blended copy at alpha 0.35 while
  the view is on and back after; `panel.rs` hover inspection text; widget source `UtilityRoutes`.
- The scene bridge adds `utilities` (segments with screen points, ports with state) for the oracle only.
- Checks: client lib tests, interaction tests, wasm32 check, Clippy.

## D: browser, launcher, report (root)

`tests/browser/utilities.js` (role `instrument`, real pointer and keys; reads the bridges only to find points
and to assert). Prepare with `scripts/mp1-play.sh --prepare PREP --package mods/utility_yard`. Run UT8 at
both zoom limits, then the three cold-play probes (first-click 1280x720 and 1024x768, buttons) on the final
prepared build. Copy `play-skirmish.sh` into `play-utilities.sh` pointing at the utility prepared build.
Report and README rows. Final gates once: fmt, clippy, changed-crate tests, wasm32 check, boundary, agents
checks, acceptance script.

## Risks

- Shared hot files (`apply.rs`, `power.rs`, `plugin.rs`, `views.rs`, `replica.rs`): hooks only, to keep the
  root's later merge small.
- The terrain dirty-chunk lifecycle decides when exposure can read changed cells; A proves the chosen point.
