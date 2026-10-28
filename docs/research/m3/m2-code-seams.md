> **Research input, not a contract.** Recon of the M2 code at commit 9e1987c, written 2026-10-27. Claims carry path:line citations; treat them as leads.

# M3 recon: M2 code seams (read-only; repo at 9e1987c, clean tree)

All paths are under /home/julien/Projects/TuRTS. Every claim has a path:line. Items marked NOT TRACED or NOT RUN were not checked.

## 1. Tick, map, position, phases

- Tick rate: `DEFAULT_TICK_HZ = 20`, comment "prototype constant" (crates/turts_sim/src/lib.rs:26). The server takes `tick_hz` from server.ron (crates/turts_server/src/config.rs:72, must be >0 at :86). The client has its own copy `LINK_TICK_HZ = 20` (crates/turts_client/src/lib.rs:54); Welcome also carries `tick_hz` (crates/turts_net_protocol/src/messages.rs:25-29).
- Slots: `SLOT_COUNT = 2` (crates/turts_sim/src/slots.rs:7).
- Map: continuous f32, not a grid. `MapBounds { size: Vec2 }` (crates/turts_sim/src/world.rs:35-39). `MapSize { x: f32, y: f32 }` (crates/turts_mod_api/src/schema.rs:34-37). The base map is 64 x 64 (mods/base/maps/flat.ron).
- Position: `Position(pub Vec2)` component (world.rs:21). Wire: `Pos { x: f32, y: f32 }` (crates/turts_net_protocol/src/messages.rs:14-17). `WorldPos` is the schema form (schema.rs:41-44).
- Footprint: only `Footprint::Circle { radius }` (schema.rs:28-30). Overlap tests are circle distance tests (crates/turts_sim/src/map.rs:180-192, production.rs:395-429).
- Phases per tick (crates/turts_sim/src/plugin.rs):
  - FixedPreUpdate: `begin_tick_audit` (:77-80), `advance_tick` (:81, set Advance), server `admit_player_commands` after Advance and before Commands (crates/turts_server/src/session.rs:69-74), `drain_commands` (set Commands, plugin.rs:83-85).
  - FixedUpdate, chained (:70-73): Movement -> Gather -> Production. Each has `run_if(not_stopped)` (:88-103).
  - FixedPostUpdate (:104-114): `collect_changes`, `publish_completed_tick`, `finish_tick_audit`, chained.
  - Server FixedLast: `send_command_results`, `send_deltas` (crates/turts_server/src/outbound.rs:14); `capture_completed_tick` (crates/turts_server/src/main.rs:103-108).
- `SimSet` enum has exactly Advance, Commands, Movement, Gather, Production, Collect (plugin.rs:29-37). A new phase needs a new variant and a new `configure_sets` chain entry (plugin.rs:70-73).
- Tick counter `Tick(u32)` (world.rs:33). Stop on exhaustion: `MatchStopped` (plugin.rs:119-134).

## 2. Economy storage model (all side tables keyed by u32 unit id, not ECS components)

- ECS components on a unit entity: only `UnitId(u32)`, `Owner(SlotId)`, `Position(Vec2)`, `UnitType(UnitTypeId)`, optional `MoveTarget` (world.rs:13-29).
- `EconomyLedger` resource (crates/turts_sim/src/economy/ledger.rs:34-41):
  - `balances: BTreeMap<(u8, ContentId), Account>`
  - `inventories: BTreeMap<u32, Inventory>`
  - `reservation_claims: BTreeMap<JobId, ReservationClaim>` (Active(Reservation) | Closed tombstone, :43-48).
- `Account { on_hand: u64, reserved: u64 }` (ledger.rs:24-27).
- `Inventory { capacity: u64, accepted_items: BTreeSet<ContentId>, items: BTreeMap<ContentId, Account> }` (ledger.rs:29-33). Capacity is one u64 total over all items. There are no slots.
- `Endpoint::{Slot(u8), Inventory(u32)}` (ledger.rs:8-11).
- Nodes: `SurfaceNodes(Vec<NodeRecord>)` resource, `NodeRecord { id: NodeId, content_id, position, remaining: u64 }` (crates/turts_sim/src/economy/mod.rs:13-21).
- Gather state: `EconomyOrders(BTreeMap<u32, GatherOrder>)` (mod.rs:22-31).
- Production: `ProductionQueues(BTreeMap<u32, Vec<ProductionJob>>)`; `ProductionJob { id: JobId, recipe, stage: Queued|Running|OutputBlocked, elapsed_ticks, duration_ticks, charged, reservation }` (crates/turts_sim/src/economy/production.rs:17-34).
- Integer types: u64 for amounts, balances, node remaining, job progress; u32 for UnitId, NodeId, Tick; u64 for JobId (crates/turts_sim/src/ids.rs:4-7). Checked u128 for sums (transaction.rs:346-367). Spec statement: spec M2 section 4, docs/superpowers/specs/2026-10-26-m2-economy-design.md:183-189.
- Rate budget: `RateCarry(u16)`, `take_budget(per_second, tick_hz)` (crates/turts_sim/src/economy/gather.rs:67-86).
- Durations: ms -> ticks, ceil, at package load (crates/turts_sim/src/bootstrap.rs:292-317).
- Transaction API (crates/turts_sim/src/economy/transaction.rs):
  - `transact(world, |tx| ...)` (:234). It stages `WriteSet { accounts, claims, queues, next_job_id, next_unit_id, spawned, events, next_event_ordinal }` (:206-216) and `commit()` applies all (:410-498).
  - Tx methods: `allocate_unit_id`, `stage_spawn`, `queues()` (clones the whole `ProductionQueues`, :273-280), `stage_queues`, `allocate_job_id`, `stage_event`, `account`, `inventory`, `inventory_used`, `validate_endpoint`, `stage_account` (:254-408).
  - Free functions: `reserve_costs` (:597), `charge_costs` (:651), `consume_reservation` (:690), `release_reservation` (:698), `refund_charged` (:758), `inventory_transfer` (:502, pub).
  - `inventory_transfer` has NO production caller. Only transaction.rs unit tests and pack tests call it (rtk proxy grep, section "Commands run"). It is a trusted scenario facility (spec :234-238).
  - Gather uses private `extract`, `store`, `convert` (gather.rs:811, 839, 899).
- Spawn through a transaction: `SpawnWrite` carries `inventory`, `gatherer: bool`, `producer: bool` flags (transaction.rs:218-227). Commit inserts the side-table rows by hand (:440-478). A third side table for M3 needs a third flag here, in `stage_package` (bootstrap.rs:243-257), in `spawn_synthetic_units` (lib.rs:96-126), and in `try_output` (production.rs:456-470). Four places repeat the same "if capability then insert row" code.
- Events: `EconomyEventKind` has Extracted, Stored, Converted, ProductionQueued/Started/Completed/Cancelled (crates/turts_sim/src/economy/events.rs:20-63).

## 3. Package schema and validation

- API version `ENGINE_API_VERSION = "0.2.0"` (crates/turts_mod_api/src/lib.rs:28). Manifest checks the requirement (crates/turts_mod_runtime/src/manifest.rs:23-42). Base manifest requires `"^0.2"` (mods/base/manifest.ron:6). Whether `^0.2` rejects 0.3.0: NOT RUN (semver caret rule says yes).
- Content kinds today: Resource (Balance|Item), Unit, Node, Production recipe, Map, Visual (schema.rs:112-318). `Exports` has six required lists (schema.rs:55-62, `deny_unknown_fields`, no `serde(default)`).
- Unit capabilities: movement, inventory, gatherer, drop_off, producer. All are `Option` and ALL are required keys in RON via `required_option` (schema.rs:175-193). Adding a capability field breaks every existing unit RON (4 mods plus test fixtures plus the generator crates/turts_checks/examples/m2_variants.rs).
- Ids: `PackageId` = `[a-z0-9_]+` (crates/turts_mod_api/src/ids.rs:7-28). `ContentId` = `<package>:<local>`, exactly one colon (ids.rs:30-62). Authoring text is `ContentRef = String` (content.rs:8). Definition ids are unique across all kinds in a package (`insert_id`, crates/turts_mod_runtime/src/content.rs:584-600). Cross-package references are rejected (content.rs:601-617). Manifest `dependencies` must be empty (manifest.rs:44-49). A package exports exactly one map (manifest.rs:51-59).
- Validation structure: one function `validate_content(origin, resources, units, nodes, production)` of 677 lines (crates/turts_mod_runtime/src/content.rs:88-559). Order: resources, id registration, units (footprint, movement, inventory, gatherer, drop_off, producer), nodes, production, then two cross-check passes (:486-557). Rules: gatherer needs movement and inventory (:185-190); producer must be static (:313); recipe output must be mobile and not a producer (:512).
- Where a new content kind goes (all of these):
  1. `Exports` + struct in schema.rs.
  2. `ContentCatalog` field and `Resolved*` type (crates/turts_mod_api/src/content.rs:84-91); re-export in lib.rs:10-25.
  3. `PackageArchive::<kind>()` accessor and `content()` call (crates/turts_mod_runtime/src/archive.rs:61-104).
  4. `validate_content` gains another positional parameter and loop. Callers: archive.rs:103 and crates/turts_mod_runtime/tests/content.rs:55, 102 (rtk proxy grep). The two session.rs literals are at lines 709 and 803, inside the test module (starts at 581).
  5. `validate_manifest` export path list (manifest.rs:61-68).
  6. Visuals: `visuals_by_content` knows only "unit" and "node" kinds (crates/turts_mod_runtime/src/visuals.rs:11-21, 26-29); `VisualTarget::{Unit, Node}` (schema.rs:309-312). Every unit and node MUST have a visual (visuals.rs:61-64).
  7. Client merge: `ClientContent::from_archives` copies four maps by hand (crates/turts_client/src/content.rs:32-70) and has a literal default (:13-18).
  8. Sim: `InstalledCatalog(ContentCatalog)` (crates/turts_sim/src/bootstrap.rs:24-25); `stage_package` picks the package name from the first unit/node/resource/production key (bootstrap.rs:50-57).
  9. Audit: `CatalogAudit` mirrors the catalog (crates/turts_sim/src/economy/views.rs:243-292).
  10. `ContentCatalog { .. }` literal constructors: crates/turts_client/src/content.rs (x2), turts_mod_api/src/content.rs (type), turts_mod_runtime/src/content.rs, crates/turts_pack/tests/economy_packages.rs, crates/turts_server/src/session.rs (x2, test code per rtk proxy grep counts).
- turts_pack: `load_package` = `archive.content()`, `archive.maps()`, `stage_package`, `install_package` (crates/turts_pack/src/lib.rs:13-38). `check_package` also runs `visuals_by_content` (lib.rs:41-46).
- E1 scan (no engine knowledge of content ids) reads every manifest and definition and scans production Rust literals: crates/turts_checks/tests/no_content.rs, tests/support/mod.rs (698 lines). New content names in M3 mods are covered by the derived scan; a new engine word that matches a mod local id would trip it. NOT RUN.

## 4. Command path

Client UI/input -> `PendingCommands`/`submit` (crates/turts_client/src/commands.rs:49-54) -> `send_commands_traced` assigns `seq` and sends `PlayerCommand` on `ReliableChannel` (commands.rs:110-158) -> server `receive_player_commands` acks receipt (`CommandAck`) and stages (crates/turts_server/src/session.rs:233-269) -> `admit_player_commands` checks sequence then calls `admit_kind` (session.rs:272-414) -> pushes sim `Command` on `CommandQueue` -> `drain_commands` (crates/turts_sim/src/command.rs:173-185) -> `apply_command` -> `apply_command_inner` match (command.rs:142-170) -> result in `CommandResults` -> `send_command_results` (crates/turts_server/src/outbound.rs:17-53) -> client `CommandResult` (crates/turts_client/src/ingest.rs:239-258).

Places to add a command variant:
1. Sim enum `CommandKind` (command.rs:30-48) and match (command.rs:143-164).
2. Protocol enum `PlayerCommandKind` (crates/turts_net_protocol/src/messages.rs:52-69).
3. Server `admit_kind` match (session.rs:340-412). It is exhaustive. Ownership and capability pre-checks live here for Produce only (:355-384).
4. Sim domain function with ownership check (pattern: `queue_production`/`cancel_production`, production.rs:37-231; `assign_gather`, gather.rs:88).
5. New `RejectReason` values: sim enum + `as_str` (command.rs:71-132), protocol `CommandRejectReason` + `ALL` + `as_str` (messages.rs:81-161), `wire_reason` exhaustive match (outbound.rs:55-85). Today the sim has 26 reasons and the wire has 23; `UnknownInventory`, `InvalidAmount` collapse to `InsufficientItems` and `UnknownBalance` collapses to `InsufficientBalance` (outbound.rs:74-76, 83).
6. Client producers of the command: input.rs:125-136 (pointer), economy_ui.rs:395-415 (`UiAction` -> `PlayerCommandKind`), scenario.rs `TraceCommand` (trace JSON, :31-46), client_web lib.rs:207, 666 (browser move shim).
7. Client result display: `append_last_result` (economy_ui.rs:325), debug JSON (crates/turts_client/src/debug.rs:164).
8. Tests: crates/turts_net_protocol/tests/wire.rs, crates/turts_sim/tests/command.rs, crates/turts_server/src/session.rs admission tests (:663+), crates/turts_server/tests/economy.rs (E8), crates/turts_client/tests/commands.rs.
- No new lightyear message registration is needed for a new command variant: `PlayerCommand` is one envelope (crates/turts_net_lightyear/src/protocol_plugin.rs:36-37).
- Protocol version: `NET_PROTOCOL_VERSION = 2` (crates/turts_net_protocol/src/lib.rs:12); the server checks it first in `judge_hello` (session.rs:89-92).
- Audit: `CommandAudit.input` stores the whole `Command` by serde (transaction.rs:35-41), so a new variant is logged without extra code. Every command and every `capture_operation` calls `audit_state()` twice (transaction.rs:93, 103, 139, 149).

## 5. Replication path

- Publish: `publish_completed_tick` (plugin.rs:140-165) builds `world_view(world)` (views.rs:353-419: clones the whole `EconomyLedger`, `EconomyOrders`, `ProductionQueues`, builds an owner map by querying all units) and `snapshot(world)` (world.rs:56-74: all units, sorted), then `economy_changes(prev, current)` (views.rs:422-460: absolute-record diff for four lists), then `CommittedTick` (events.rs:78-87) and `CompletedWorld` (events.rs:97-98).
- `CommittedTick.removed` is always `Vec::new()` (plugin.rs:156). Nothing in sim despawns a unit (rtk proxy grep for `despawn` in turts_sim/src gives no match). Client code and tests handle `removed` (crates/turts_client/src/replica.rs:390-425; crates/turts_client/tests/economy_replica.rs:412) but the server never sends it.
- `CommittedTick.nodes` is ALL nodes, every tick, not a diff (plugin.rs:146, 158). Units: `spawned` (public) plus `positions` (only changed). No per-unit state beyond `pos` (UnitSnapshot, messages.rs:35-40). No orientation, no health.
- Wire: `Snapshot { tick, units, nodes, economy: EconomyState }`, `WorldDelta { tick, base_tick, spawned, removed, positions, nodes, economy: EconomyChanges }` (messages.rs:168-184). `EconomyState`/`EconomyChanges` have four lists: balances, inventories, orders, queues (crates/turts_net_protocol/src/economy.rs:123-139). Channels: Snapshot reliable-ordered; WorldDelta unordered-unreliable (protocol_plugin.rs:15-25, 32-35).
- Owner-only filter: `snapshot_for` and `delta_for` filter by `owner_slot == slot` BEFORE serialization (crates/turts_server/src/export.rs:21-111). Units and nodes are public (export.rs:24-25, 64-79).
- Server send: `send_snapshots` on `SnapshotDue` (session.rs:541-564); `send_deltas` per session per tick, `StateChannel` (outbound.rs:87-119).
- Client: `Replica` holds `units`, `nodes`, `economy` (replica.rs:88-106). `stage_delta` CLONES the whole `Candidate` (units, nodes, economy) for every delta (replica.rs:389) and `merge_by` is a linear `position` search (replica.rs:837-852). `valid_economy` (replica.rs:584-805) re-validates the entire economy against the catalog on every delta (replica.rs:469). Client unit samples are a VecDeque pruned to 16 ticks (replica.rs:543, 449-456).
- Places to add a replicated field or entity kind:
  1. Sim state + view struct + `WorldView`/`EconomyStateView`/`EconomyChangesView` (views.rs:316-338), `world_view`, `economy_changes` (key fn + sort), `AuditState` (views.rs:295-315).
  2. `CommittedTick` (events.rs:78) and `publish_completed_tick` (plugin.rs:140).
  3. Protocol: `EconomyState`/`EconomyChanges` or new fields on `Snapshot`/`WorldDelta`; bump `NET_PROTOCOL_VERSION`.
  4. Server `snapshot_for`, `delta_for` and the owner filter (export.rs).
  5. Client replica: `Candidate` (replica.rs:79-84), `Replica` fields, `empty_economy` (:545), `stage_snapshot` (:344), `candidate_at` (:378), `stage_delta` (:387), `valid_economy` (:584), `merge_economy` (:807), `commit` (:476); removal clean-up of a unit's side records (:414-425).
  6. Client ingest: `merge_economy_baseline` is a second hand-written copy of the merge, for the four lists (crates/turts_client/src/ingest.rs:509-554); loss-injection matchers `matching_world_changes` (:556+), `has_charged_line_transition` (:652), `economy_increased` (:720) are keyed to M2 event semantics.
  7. Render: `sync_units` (crates/turts_client/src/render.rs:357-409) is one entity per unit, one shared material, no rotation; `sync_nodes` (:438-495); shape table keyed by content id from visuals.
  8. UI/debug: `panel_model` (crates/turts_client/src/economy_ui.rs:74+), `debug_json` (debug.rs:31), `snapshot_json` (live_view.rs:130), `publish_scene` and `publish_ui` (crates/turts_client_web/src/lib.rs:243, 362).
  9. Browser test: `assertPublicProjectionSchema` asserts exact key sets with `deepEqual` for view, economy, balances, inventories, orders, queues, units, nodes (tests/browser/m2.js:356-376). Any new key fails it until the test changes.

## 6. Health, damage, targeting, projectiles, combat, research

None exist.
- rtk proxy grep, case-insensitive, for `health|damage|projectile|combat|attack|weapon|turret|ammo|research|belt|inserter` over crates, tests, scripts, mods (.rs/.ron/.md/.js/.py/.sh) returned only prose in the M2 spec: spec :36 (general factories/research/logistics are open), :46-47 (excluded: combat, damage/destruction, repair, victory, item-output recipes, belts/inserters, research), :69-70, :306-308 (destruction excluded: "Future removal must resolve held stock/reservations through an explicit policy"), :599.
- No component for hit points, no target, no weapon, no projectile entity, no tech/research table, no unit despawn path, no ammunition item. Items are `ResourceKind::Item` with no behavior (schema.rs:112-116). The only unit "rifleman" has no capability (mods/base/units/rifleman.ron).
- Only `Position` changes per tick for units; no "dies", no refund of inventory on removal.

## 7. Test instruments

- Process acceptance: `scripts/acceptance.sh` builds the two release binaries, runs `unit_boot`, `hello`, `acceptance` (T-series: t2a, t2d, t3, t4, t5, t6, t6b, t8), and four exact economy tests (e8, e9 x2, e10) from crates/turts_server/tests/economy.rs (3645 lines), then T5 again. It rejects zero-test results (acceptance.sh:11-22) and checks the four required names exist (:33-45).
- Accounting oracle: crates/turts_pack/tests/common/accounting.rs (3188 lines) is a SECOND implementation of gather, production, output placement and movement (`predict_gather` :1541-2282, `production_advance` :2301, `output_attempt` :2437, `predicted_movement` :2683). `advance_accounted_tick` walks operations in fixed order: all gatherers, then production heads (:24-188). `AutomaticPhase` is `Gather | ProductionAdvance | OutputAttempt` (:190-195). `check_conservation` counts item stock only as inventory `on_hand` plus node `remaining` (:546-597) and balances with charge/refund/conversion terms (:598-648). Items on a belt, in an inserter hand or in a magazine would break the equation unless the oracle changes.
- E-tests: crates/turts_pack/tests/economy_accounting.rs (3118 lines, e6_*, e7_*, e8_*), economy_variants.rs (E2/E3 rename and role swap with generator crates/turts_checks/examples/m2_variants.rs, 2083 lines), economy_completion.rs, economy_gather.rs, economy_production.rs, economy_ordering.rs, economy_phase_order.rs, economy_packages.rs, economy_package_boot.rs, economy_boot.rs, economy_transactions.rs. E1 in crates/turts_checks/tests/no_content.rs. E10 in crates/turts_checks/tests/m2_public_path.rs and economy.rs e10.
- Audit log (crates/turts_server/src/economylog.rs, 903 lines): `EconomyLogRecord` = Initial | CommandBoundary | Operation | Claim | Admission | Tick (:19-50). `validate_records` (:306-526) replays continuity. Key hard-coding: movement is the one phase NOT captured as an operation, tracked by `movement_pending` (:326) and checked by `validate_movement_transition` (:579-692, includes a copy of the movement formula and `Time<Fixed>` delta). Any other state change between operations fails with "differs from the prior operation state" (:708-711). So every new automatic phase must run inside `capture_operation` (transaction.rs:78-118) per subject, like `run_gather` does per gatherer (gather.rs:214-230), and every new state must be in `AuditState` or it is invisible to the diff.
- `audit_state` is a full-world snapshot (views.rs:562-840) called twice per captured operation. Enabled by `--economy-log` (main.rs:94-101).
- Mutation/refutation sweeps: tests/m2/refutations.json has 41 cases (13 mutation, 8 product, 8 browser, 7 process, 5 self-test kinds) run by tests/m2/refutations.py (387 lines) via scripts/m2-refutations.sh `--case ID|product|all|ci-native|ci-browser`. Each case: anchor text + replacement in a scratch copy, run a named test, expect red, restore, expect green. Python tests: tests/m2/test_refutations.py, test_variants.py.
- Browser: scripts/m2-browser.sh (330 lines) runs 15 registered cases (base 4, haul 2, acme 2, cedar 2, loss families 5) via tests/browser/m2.js (1523 lines) with helpers m2-audit.js and node tests m2-audit.test.js, m2-projection.test.js. Case list is hard-coded in the script (m2-browser.sh:12-27) and counted == 15 (:31-35). Loss families: conversion, charge, refund, spawn, final_change.
- CI: .github/workflows/ci.yml jobs `check` (fmt, clippy -D warnings, wasm check, workspace tests, boundary, native refutations, acceptance) and `browser` (T7, M2 browser, browser refutations) and `agents`.
- Recorded M2 status: workspace tests 497 passed, 17 ignored; 33 native + 8 browser mutation cases (docs/measurements/m2.md:5-12).
- What M3 must extend: oracle (new phases, conservation terms), AuditState and validate_records, `AutomaticPhase`, refutation registry (new plants for belts/inserters/turret/research), E-test numbering (spec :533-552 defines E1-E10 for M2 only), browser case list and `assertPublicProjectionSchema`, `scripts/acceptance.sh` required test list.

## 8. File sizes (lines)

Top 10 source files (non-generated, crates + tests dirs; `wc -l`):
1. crates/turts_server/tests/economy.rs 3645
2. crates/turts_pack/tests/common/accounting.rs 3188
3. crates/turts_pack/tests/economy_accounting.rs 3118
4. crates/turts_checks/examples/m2_variants.rs 2083
5. crates/turts_sim/src/economy/transaction.rs 1546 (production code ends at 822; tests after :823)
6. tests/browser/m2.js 1523
7. crates/turts_client/src/ingest.rs 1517 (production ends 1042)
8. crates/turts_client/tests/economy_ui.rs 1514
9. crates/turts_pack/tests/economy_completion.rs 1335
10. crates/turts_sim/src/economy/gather.rs 1324 (production ends 1245)
(Generated crates/turts_client_web/dist/*.js 2596 is excluded.)
Production-source files over 800 lines (rtk proxy wc): transaction.rs 1546, ingest.rs 1517, gather.rs 1324, session.rs 1274 (prod ends 580), views.rs 966, economylog.rs 903 (prod ends 715), replica.rs 853, economy_ui.rs 846 (prod ends 815). Below 800: content.rs (runtime) 677, client_web lib.rs 682, production.rs 492, scenario.rs 600, plugin.rs 165, command.rs 281.
M3 hotspots: transaction.rs (new accounting flows, SpawnWrite), gather.rs and production.rs patterns, views.rs (every new state), plugin.rs, session.rs `admit_kind`, ingest.rs and replica.rs (new replicated records), economy_ui.rs, content.rs (runtime validation). The test hotspots (economy.rs, accounting.rs, economy_accounting.rs, m2_variants.rs, m2.js) are already 1500-3600 lines.

## 9. Obstacles for belts and inserters

Each item has a source line. "Obstacle" means M3 design must decide, not that M3 is blocked.
1. No grid. Positions are free f32; footprints are circles; overlap is O(N) distance checks over all entities (production.rs:395-429, map.rs:43-64). A belt needs adjacency, direction and cell snapping; none exists. The spec says terrain is a heightfield (CLAUDE.md) but there is no terrain code in the scoped crates (NOT TRACED outside scope).
2. No construction. All units are map-placed at boot (`stage_package` bootstrap.rs:152-265); the only runtime spawn is recipe output (production.rs:441-480). Spec excludes player construction (spec :46).
3. Recipes output a unit only. The validator requires `output_unit` to be mobile and not a producer (runtime content.rs:512-519). There is no item-output recipe, no consume-over-time, no multi-input/multi-output recipe (spec :36, :47). `ProducerInventory` cost is paid at queue or start (production.rs:112-120, 268-278), not consumed during the run.
4. Producers must be static, gatherers must be mobile (content.rs:313, 185). No capability exists for a static non-producer with a work cycle. Static units can hold inventory and drop-off (command_center) but cannot move items.
5. Item movement between inventories is not exposed: `inventory_transfer` is pub but has no production caller (transaction.rs:502). Its owner check requires the same owner on both ends (:526-533), runs as one atomic step with no duration.
6. Inventory capacity is one u64 total over all items, no slots, no per-item stack (ledger.rs:29-33, transaction.rs:553). No ordering of items, no position of an item. An item is a count, not an entity: there is no per-item identity anywhere. A belt that needs item positions must be a new representation, and it must be in the accounting (oracle `check_conservation` counts only inventories and nodes, accounting.rs:546-597).
7. Reservations are keyed by `JobId` and tied to production jobs (ledger.rs:40, 43-48; transaction.rs:597-757). An inserter claim on a source item or a target slot has no existing token type. `Endpoint` has two variants only (ledger.rs:8-11).
8. All entity lookups scan the world: `find_unit` (gather.rs:1194), `owner_of`, `inventory_owner` (transaction.rs:584-593), `queue_production` (production.rs:44-55), `try_output` (production.rs:334-347). 29 `iter_entities`/`find_unit` call sites in turts_sim/src (rtk proxy grep). Cost is O(units) per lookup, per operation.
9. Per-tick full-state cost: `world_view` clones ledger, orders, queues and queries all units (views.rs:353-419); `snapshot` builds all units (world.rs:56); `Transaction::queues()` clones all queues for each production transaction (transaction.rs:273-280); with auditing on, `audit_state` runs twice per operation (transaction.rs:93, 103). Cost grows with total state, not with change. No measurement of per-tick time exists in docs/measurements/m2.md (budgets are open, m2.md:5, 9).
10. Replication: node list is resent whole every tick (plugin.rs:146). Delta economy is diffed by absolute record (views.rs:422-460). Per-item state would need either a record per item (large) or a compact per-belt record. Client deep-clones everything per delta (replica.rs:389) and has no `removed` from server (plugin.rs:156).
11. Unit entities carry only 4-5 components and only `pos` is replicated each tick (world.rs:87-101, messages.rs:41-45). No direction/orientation, no per-unit status (working, blocked) outside `OrderState`/`QueueState`. Render shows one capsule/cuboid per unit with one shared material and no rotation (render.rs:393-396, mesh_for :291-296).
12. Destruction/removal: spec requires an explicit policy before an economic entity can be deleted (spec :306-308). There is no despawn. Interruption/cancellation semantics for belts and inserters have no M2 precedent except gather interrupt (`MoveTo` clears order, retains cargo: command.rs:216-226) and production cancel refund ratios (production.rs:177-193).
13. Congestion precedent in M2: blocked output (`OutputBlocked`, FIFO block, retry each tick, production.rs:254-319), gather `Waiting{reason}` with retained cargo (events GatherStage in protocol economy.rs:79-99). These are the existing "backpressure" patterns.
14. Content schema rigidity: `Exports` has no `serde(default)` and `UnitDef` capabilities are `required_option` (schema.rs:55-62, 175-185), so each new content kind or capability is a breaking package change; API bump and migration of 4 mods (acme, base, cedar, haul) plus fixtures. M2 did this once (spec :52-70 migration row).
15. Phase order is fixed and chained: Movement -> Gather -> Production (plugin.rs:70-73). An item-moving phase must pick its place relative to Gather (income) and Production (start). M2 spec says gather income is visible to producer start in the same tick (spec :256-258).
16. Tick rate 20 Hz and unit granularity are prototype constants (lib.rs:26). Belt speed in items/s needs the same ceil-division-plus-carry rule as gather (gather.rs:67-86; bootstrap.rs:292-317) or a decision.

## Commands run

- `rtk proxy find crates mods tests scripts -type f (rs|ron|sh|js|py|toml) | xargs wc -l | sort -rn | head -50`
- `rtk proxy ls` of crates/*/src, crates/*/tests, mods, tests, scripts
- `wc -l` on src files of turts_sim, turts_mod_api, turts_mod_runtime, turts_net_protocol, turts_server, turts_client, turts_pack; and the split between production and test code via `rtk proxy grep -n -m1 '#\[cfg(test)\]'`
- Read: turts_sim plugin.rs, lib.rs, map.rs, world.rs, movement.rs, bootstrap.rs, command.rs, ids.rs, economy/{mod,ledger,events,production}.rs, transaction.rs (1-620), views.rs (172-519), gather.rs (195-310, 811-840); turts_mod_api schema.rs, lib.rs, content.rs, ids.rs; turts_mod_runtime content.rs, archive.rs, manifest.rs, pack.rs, visuals.rs; turts_pack lib.rs; turts_net_protocol messages.rs, economy.rs, lib.rs; turts_net_lightyear protocol_plugin.rs; turts_server session.rs (62-94, 190-600), outbound.rs, export.rs, economylog.rs (1-40, 300-715), main.rs (60-131); turts_client replica.rs (14-140, 340-553, 554-640, 800-853), render.rs (286-495), commands.rs (1-165), economy_ui.rs (20-100), content.rs, ingest.rs (460-560); turts_pack tests/common/accounting.rs (20-80, 186-280, 546-650); mods/base files; spec docs/superpowers/specs/2026-10-26-m2-economy-design.md (1-140, 181-330, 360-480, 533-603); docs/measurements/m2.md (1-80); scripts/acceptance.sh, scripts/m2-refutations.sh, scripts/m2-browser.sh (1-60); tests/m2/refutations.py (1-60), refutations.json (head); tests/m2/variants.py; tests/browser/m2.js (356-390, outline); .github/workflows/ci.yml (1-120).
- `rtk proxy grep -nE` outlines of: transaction.rs, session.rs, views.rs, gather.rs, economylog.rs, accounting.rs, and client files replica/ingest/economy_ui/render/commands/input/lib/debug/live_view/scenario.
- `rtk proxy grep -rn` for: `despawn|.removed|removed:`; the M3-term list (health/damage/...); `ContentCatalog {`; `Exports {`; `PlayerCommandKind::|CommandKind::` (file lists and src line lists); `inventory_transfer`; `iter_entities|find_unit`; `SLOT_COUNT`; `tick_hz`; `add_systems|configure_sets`.
- `python3 -c` to count and list the 41 cases in tests/m2/refutations.json.
- NOT RUN: any cargo build/test, any script, semver check of `^0.2` vs 0.3.0, any benchmark. I changed no repo file and ran no `git add`.
