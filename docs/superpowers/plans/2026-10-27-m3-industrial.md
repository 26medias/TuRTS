# M3 Industrial Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Plan status:** Gate 2 passed. The user confirmed the calls K1-K8 in the spec (section 1.1) on 2026-10-27. Task 0 may start.

**Goal:** Prove mine → belt → factory → inserter → ammunition turret → research upgrade with a conservation oracle, through a new `forge` data package, the authoritative server and the browser client.

**Architecture:** Extend the M2 single-writer transaction service with new subjects (belt lines, inserters, extractors, machines, weapons, healths, research) that each run in one fixed phase and commit one transaction per subject per tick. Items on belts are compact slot arrays. A hand, a magazine and a machine output are ordinary inventories. The oracle checks conservation from audit states and events, and parses the package source itself.

**Tech Stack:** Rust 2024, Bevy 0.20.0, lightyear 0.31.0, RON 0.12.2, serde, postcard 1.1.3, Trunk and Playwright-core (as M2).

**Spec:** `docs/superpowers/specs/2026-10-27-m3-industrial-design.md` (the spec, "SPEC" below). Rationale and derivations: `docs/research/m3/*.md` (research, not contract). Product spec: `RTS-Engine-Project-Specifications.md` v0.7. M2 design (M2D): `docs/superpowers/specs/2026-10-26-m2-economy-design.md`.

## Global Constraints

- `ENGINE_API_VERSION = "0.2.1"`; `NET_PROTOCOL_VERSION = 3`; `forge` package version `0.1.0`, `engine_api: "^0.2.1"`. Existing packages (base, haul, acme, cedar) stay `engine_api: "^0.2"` and load unchanged.
- Every new schema field is `#[serde(default)]`. `deny_unknown_fields` stays.
- Tick rate 20 Hz. Durations convert with `ceil(duration_ms * tick_hz / 1000)`. Repeat rate and duration checks at 10 and 30 Hz (as M2).
- One subject, one tick, one `transact` per new phase (SPEC 12.1). No state change outside a captured operation.
- Phase order: `Movement → Gather → Extract → Transport → Inserters → Production → Combat` (SPEC 11).
- Amounts are `u64`; sums use checked `u128` then checked conversion (M2 rule). Ids are never reused.
- Items are never entities. No Bevy type in `turts_mod_api`, `turts_net_protocol`, `turts_mod_runtime`. `turts_client` never depends on `turts_sim`. Keep `scripts/boundary.sh` green.
- No engine code names a forge item, recipe, technology or unit. Hostility, ammo item, research target and removal contents are package data.
- A unit has at most one work capability (SPEC 3.1). `Remove` is not allowed on a unit with a queue.
- New logic goes into new files (`economy/{conveyor,inserter,extractor,combat,research,machine}.rs`, `turts_mod_runtime/src/content_m3.rs`). Do not grow `transaction.rs`, `ingest.rs` or `session.rs` beyond the hooks that the task names.
- Tests of `pub(crate)` items (`split_line`, `remove_unit`, `step_line`, `Transaction`) live in a `#[cfg(test)] mod tests` inside the source file, as M2 does (`transaction.rs:823`). Files under `crates/turts_sim/tests/` use only the public `turts_sim` API (`headless_app`, `AuditTrail`, `AuditState`, the commands). Where a task lists a test file, read it as "or the in-file test module" when the item is `pub(crate)`.
- Types, variants and fields (for example `Endpoint::Output`) added in Task 3a but first used in a later task carry `#[allow(dead_code)]` with the comment `// used from Task N`; the task that uses them removes the allow (strict Clippy runs with `-D warnings`).
- Wire record types end in `Record`; sim state types are `pub(crate)` and never share a name with a wire type.
- Code style: 4 spaces per indent level.

## Review Focus

Failure modes that the SPEC implies and that a task's main test might not reach. Each has a test in the owning task.

1. Removing the first or last piece of a belt line, or the only piece (line split edge cases). Owner: Task 3 (split unit test) and Task 7 (items keep moving after a split).
2. A belt `slots_per_second` above the tick rate (several steps in one tick) and the rate at 10 and 30 Hz. Owner: Task 7.
3. A client that joins late must see belt contents, hands and healths from the snapshot alone. Owner: Task 13b and Task 15a.
4. `Produce(lab, technology)` without enough shells must reject with `InsufficientItems` and leave the queue and claims unchanged. Owner: Task 10.
5. Removing a unit that is the `receiver` of an M2 gatherer order (SPEC 8.3, NOT TRACED). Owner: Task 11.

## Execution contract

- One branch per task, named `codex/m3-t<N>`, based on the current integration branch `codex/m3-integration` (create it from `main` at Task 0). Merge a task only after its tests, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` (the CI command, `.github/workflows/ci.yml:25`), `cargo check --workspace --all-targets`, `cargo test --workspace` and `bash scripts/boundary.sh` pass. On a schema or wire task, also run `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown`.
- Build binaries one at a time: `cargo build -p turts_server`, `cargo build -p turts_client`. Do not run concurrent un-hashed binary builds on the shared target directory.
- Stage explicit paths only. Never `git add -A`.
- A task that must change a frozen interface stops and reports before any dependent task starts.
- Parallel lanes: after Task 4, Tasks 6, 7 and 9 may run in separate worktrees. All other tasks follow the dependency graph below. The coordinator composes shared-file hunks (`transaction.rs`, `plugin.rs`, `views.rs`, `command.rs`).
- Record in `vault/Work Log/` each task's tests, red-to-green evidence, changed paths and the commit hash.

```text
T0 → T1 → {T2, T3a}          (T3a needs only the Task 1 types, so it may run beside T2)
T2 → T5 ; T3a → T3 ; {T3, T5} → T4
T4 → {T6, T7, T9} ; {T6, T7} → T8 ; T9 → T10 ; {T10, T3} → T11
{T7, T8, T11} → T12 → T13a → {T13b, T14}   (T14 grows case by case as phases land)
T13b → T15a → T15b ; {T14, T15b} → T16
```

Critical path: T0 → T1 → T3a → T3 → T4 → T7 → T8 → T12 → T13a → T13b → T15a → T15b → T16.

The run sheet (agents, reviewers, validators, waves, ledger) is in `vault/M3 Execution Board.md`.

---

### Task 0: M2 operation-stream baseline

**Files:**
- Create: `crates/turts_pack/tests/m3_m2_stream.rs`, `tests/m3/m2_operation_stream.json`
- Modify: none in `src/`. This task runs on commit `9e1987c` content, before any M3 edit.

**Interfaces:**
- Consumes: `turts_sim::AuditTrail::drain_operations()` and `OperationAudit { operation_id, before, after, events }` (`crates/turts_sim/src/economy/transaction.rs:44-49`), reachable from `turts_pack` tests as in `crates/turts_pack/tests/common/accounting.rs:9-34`. (`economylog` lives in the binary-only `turts_server` crate and is not reachable.)
- Produces: `tests/m3/m2_operation_stream.json`: for each package in `base, haul, acme, cedar`, for each tick of a 400-tick headless run, the number of captured operations and, for each operation, the list of its event kinds and whether `before == after`.

- [ ] **Step 1:** Write test `m2_operation_stream_matches_baseline`. It boots each package through `turts_pack::load_package`, drives a command script (for base, haul and cedar the Gather and Produce scripts that `economy_accounting.rs` and `economy_variants.rs` already use for that package; for acme a `MoveTo`), runs 400 ticks, and compares the stream to the JSON file. With env `UPDATE_M3_BASELINE=1` it writes the file instead of comparing. It asserts that base, haul and cedar streams contain at least one operation (a file of zeros would prove nothing).
- [ ] **Step 2:** Run `UPDATE_M3_BASELINE=1 cargo test -p turts_pack --test m3_m2_stream`. Expected: file written.
- [ ] **Step 3:** Run `cargo test -p turts_pack --test m3_m2_stream`. Expected: PASS. Then plant one extra empty operation per tick in `capture_operation` (temporary, uncommitted), rerun, expect FAIL, and revert it.
- [ ] **Step 4:** Create branch `codex/m3-integration`; commit `tests/m3/m2_operation_stream.json` and the test.

### Task 1: Schema types for API 0.2.1

**Files:**
- Modify: `crates/turts_mod_api/src/schema.rs`, `content.rs`, `lib.rs`; `crates/turts_mod_runtime/src/archive.rs`, `manifest.rs`, `visuals.rs`; `scripts/boundary.sh` (add the no-Bevy rule for `turts_mod_runtime`; the fixture check stays red); every file where `cargo check --workspace --all-targets` reports a struct literal of a changed type (about 66 sites in 11 files, for example `crates/turts_server/src/session.rs:709,803` and `crates/turts_client/src/content.rs:13,26`)
- Test: `crates/turts_mod_api/tests/schema_m3.rs`, existing package tests keep their assertions (only struct literals gain the new fields)

**Interfaces:**
- Produces: every type in SPEC 3 (`Facing`, `ExtractorDef`, `ConveyorDef`, `InserterDef`, `RunRefunds`, `RecipeDef`, `MachineDef`, `HealthDef`, `SlotRelation`, `TargetFilter`, `AmmoDef`, `WeaponDef`, `ContentsPolicy`, `RemovalDef`, `WeaponStat`, `EffectDef`, `TechnologyDef`, `ResearchStationDef`, `BeltLinePlacement`), new `UnitDef` / `Placement` / `MapDef` / `Exports` fields (all `#[serde(default)]`), `ENGINE_API_VERSION = "0.2.1"`, `ContentCatalog` and `Resolved*` accessors for recipes and technologies, archive and manifest readers for the new export lists.

- [ ] **Step 1:** Write `forge_shaped_ron_parses` (a RON fixture of a unit with `machine`, `output_inventory`, `weapon`, `health`, `removal`; a technology; a recipe; a map with `belts` and `facing`) and `old_ron_still_loads` (every file under `mods/{base,haul,acme,cedar}` parses unchanged) and `unknown_field_typo_rejected`.
- [ ] **Step 2:** Run both: expect FAIL (types missing).
- [ ] **Step 3:** Add the types and fields. Do not derive `Default` on `UnitDef`, `Footprint` or other public structs (it would invent defaults in the public mod API). Add the new fields explicitly to each existing struct literal. Run `cargo check --workspace --all-targets` so test and example sites show up. Do not edit any file under `mods/`. Bump `ENGINE_API_VERSION` and update only version literals in tests that assert the engine version (not the `^0.2` requirement).
- [ ] **Step 4:** Run `cargo test -p turts_mod_api -p turts_mod_runtime`. Expected: PASS, including all M2 package tests.
- [ ] **Step 5:** Commit explicit paths.

### Task 2: Portable validation for the new kinds

**Files:**
- Create: `crates/turts_mod_runtime/src/content_m3.rs`, `crates/turts_mod_runtime/tests/content_m3.rs`
- Modify: `crates/turts_mod_runtime/src/content.rs` (call into `content_m3`), `archive.rs` (call site)

**Interfaces:**
- Consumes: Task 1 types.
- Produces: `pub(crate) fn validate_m3(manifest: &Manifest, catalog: &ContentCatalog) -> Result<(), ModError>`, called from the place where `validate_content` (`content.rs:88`) is called, after the manifest and the content are both parsed. Do not change a public signature; if `validate_content` needs the manifest's `engine_api`, add a new function and keep the old one as a wrapper.

- [ ] **Step 1:** Write one reject test per rule of SPEC 3.1, each asserting package, file and field in the `ModError`: `machine_needs_matching_inventories`, `machine_inputs_exceed_input_capacity`, `machine_outputs_exceed_output_capacity`, `machine_must_be_static`, `health_requires_removal`, `weapon_must_be_static`, `weapon_ammo_item_not_accepted`, `modifier_unit_without_weapon`, `prerequisite_cycle`, `duplicate_id_across_recipe_and_unit`, `two_work_capabilities_on_one_unit`, `removal_on_unit_with_queue`, `item_count_over_u16`, `m3_field_with_caret_0_2_rejected` (a package that declares `^0.2` and uses `weapon`), plus `forge_style_package_accepted` and `map_and_visual_ids_stay_separate`.
- [ ] **Step 2:** Run: expect FAIL.
- [ ] **Step 3:** Implement `validate_m3` with a depth-first cycle check; add the version-honesty rule in `validate_m3` (a package that uses an M3 field must declare an `engine_api` requirement that excludes 0.2.0). `validate_manifest` runs before the content is read, so it cannot hold this rule.
- [ ] **Step 4:** Run `cargo test -p turts_mod_runtime`. Expected: PASS.
- [ ] **Step 5:** Commit explicit paths.

### Task 3a: State types, ledger shape and reject reasons

**Files:**
- Create: `crates/turts_sim/src/economy/{conveyor,inserter,extractor,combat,research}.rs` (state types only, no systems)
- Modify: `crates/turts_sim/src/economy/{ledger,mod,transaction,views,production,events}.rs` (the eight exhaustive `Endpoint` matches at `transaction.rs:324,375,419`, `views.rs:473,485,527,623`, `production.rs:486`; plus the new `Output` variants of `EventSource`, `events.rs:73`, and `ReservationEndpointView`, `views.rs:151`), `crates/turts_sim/src/command.rs`, `crates/turts_net_protocol/src/messages.rs`, `crates/turts_server/src/outbound.rs`
- Test: `crates/turts_sim/tests/m3_shapes.rs`, `crates/turts_net_protocol/tests/reasons_m3.rs`, and an in-file `#[cfg(test)]` module in `outbound.rs` for `wire_reason`

**Interfaces:**
- Produces: `Endpoint::Output(u32)` with matching `EventSource::Output(u32)` and `ReservationEndpointView::Output(u32)`; every existing exhaustive `Endpoint` match handles `Output` (as an inventory-like endpoint); `EconomyLedger.output_inventories: BTreeMap<u32, Inventory>`; the state types of SPEC 4-9 (`ItemIx`, `BeltLine`, `Conveyors`, `Inserters`, `InserterState`, `ArmStage`, `ArmWait`, `ExtractorState`, `Healths`, `WeaponState`, `ResearchState`, `Disabled`) as `pub(crate)` with `Default`, `Clone`, `PartialEq`; the new sim `RejectReason` values `NotRemovable`, `NotToggleable`, `PrerequisitesMissing`, `AlreadyResearched`, `ResearchInProgress`, `IntegrityViolation` with `as_str`; the wire `CommandRejectReason` values for the first five plus `Internal` (`ALL`, `as_str`); `wire_reason` maps `IntegrityViolation` to `Internal`.

- [ ] **Step 1:** Write `wire_roundtrip_new_reasons` (every new reason round-trips through postcard and `as_str`), `wire_reason_covers_every_sim_reason` (exhaustive over sim reasons), `output_inventory_defaults_empty`.
- [ ] **Step 2:** Run: expect FAIL.
- [ ] **Step 3:** Add the types and reasons. Do not add systems or phases. Do not change the wire protocol version yet.
- [ ] **Step 4:** Run `cargo test -p turts_sim -p turts_net_protocol` and `cargo check --workspace`. Expected: PASS.
- [ ] **Step 5:** Commit explicit paths.

### Task 3: Transaction and removal infrastructure

**Files:**
- Modify: `crates/turts_sim/src/economy/transaction.rs`, `mod.rs`, `plugin.rs`
- Test: in-file `#[cfg(test)] mod tests` in `transaction.rs` (the items are `pub(crate)`)

**Interfaces:**
- Consumes: Task 3a types.
- Produces: `Transaction::inventory(Endpoint)`, `inventory_used(Endpoint)`, `validate_endpoint`, `account`, `commit` generalized to `Endpoint` (the `u32` forms stay as wrappers); `WriteSet` overlays `BTreeMap<K, Option<V>>` for nodes, conveyor lines, inserters, extractors, weapons, healths, research, disabled; `fn extract_from_node(tx: &mut Transaction, node: NodeId, to: Endpoint, item: ContentId, n: u64) -> Result<(), RejectReason>`; `fn remove_unit(tx: &mut Transaction, unit: u32, cause: RemoveCause) -> Result<(), RejectReason>`; `RemoveCause::{Removed, Killed { by: u32 }}`; `RemovedThisTick` resource drained by `publish_completed_tick` into `CommittedTick.removed`; event `Destroyed { unit, cause, items: Vec<DestroyedLine> }`; `fn split_line(line: &BeltLine, piece_index: usize) -> (Option<BeltLine>, Option<BeltLine>)`.

- [ ] **Step 1:** Write tests: `remove_unit_destroys_and_lists_contents` (inventory, output inventory, hand), `remove_unit_with_reserved_stock_rejects_integrity_violation`, `remove_unit_clears_inserter_endpoints`, `failed_write_leaves_state_equal`, `write_to_missing_endpoint_is_integrity_violation` (M3 paths only; M2 behavior unchanged), `extract_from_node_is_atomic`, `removed_this_tick_reaches_committed_tick`, `split_line_first_middle_last_and_only_piece`.
- [ ] **Step 2:** Run: expect FAIL.
- [ ] **Step 3:** Implement. Use per-key overlays; do not clone whole tables. `split_line` follows SPEC 4.3.
- [ ] **Step 4:** Run `cargo test -p turts_sim` and `cargo check --workspace`. Expected: PASS with all M2 tests.
- [ ] **Step 5:** Commit explicit paths.

### Task 5: Package `forge` and the E1 scanner

**Files:**
- Create: `mods/forge/**` (manifest, resources, nodes, units, recipes, technologies, maps, visuals), `mods/forge/NAMES.md`, `tests/m3/packages/forge_locked/**`
- Modify: `crates/turts_checks/tests/support/mod.rs` (export list `:87-94`, `stray_ron` `:333-350`), `crates/turts_checks/tests/no_content.rs`
- Test: `crates/turts_mod_runtime/tests/forge_package.rs`

**Interfaces:**
- Consumes: Tasks 1, 2.
- Produces: package `forge` per SPEC 13 (content table, map table, removal flags), local ids that pass E1 (renamed as needed; final names in `NAMES.md`); fixture package `forge_locked` (forge plus a second technology that has the first as prerequisite).

- [ ] **Step 1:** Write `forge_packs_and_validates` (uses `turts_mod_runtime::pack_dir` and content validation only; it does **not** boot the sim, because `turts_pack::load_package` boots it and Task 4 does not exist yet), `forge_archive_hash_is_stable_across_two_packs`, `e1_scans_forge_tokens` and three E1 plant tests: a production file holding a forge recipe id, a technology id, a unit id must each turn E1 red (use the existing plant pattern in `no_content.rs`).
- [ ] **Step 2:** Run: expect FAIL.
- [ ] **Step 3:** Author the package from SPEC 13.1 and 13.2 including the removal flags. Run E1 first; rename any local id that equals an engine identifier. Extend the scanner for the `recipes` and `technologies` kinds.
- [ ] **Step 4:** Run `cargo test -p turts_mod_runtime --test forge_package` and `cargo test -p turts_checks`. Expected: PASS.
- [ ] **Step 5:** Commit explicit paths.

### Task 4: Boot

**Files:**
- Modify: `crates/turts_sim/src/{bootstrap,lib,plugin,registry,map}.rs`; the phase files from Task 3a get empty `install` functions
- Test: `crates/turts_sim/tests/boot_m3.rs`, `crates/turts_pack/tests/forge_boot.rs`

**Interfaces:**
- Consumes: Tasks 3a, 3, 5.
- Produces: `fn install_capability_rows(...)` used by package staging, synthetic units and spawn commit; boot expands `BeltLinePlacement` (ids after all `placements`, upstream to downstream); inserter endpoint resolution (SPEC 5.1); extractor binding (SPEC 7); `ItemTable`; `fn ms_to_ticks(ms: u64, tick_hz: u32) -> u64` (ceil); new `SimSet::{Extract, Transport, Inserters, Combat}` in the chain, with empty systems.

- [ ] **Step 1:** Write `forge_map_boots_with_expected_ids` (through `turts_pack::load_package`; unit and belt piece ids as in SPEC 13.2 order), `turts_pack_binary_accepts_forge` (`cargo run -p turts_pack -- mods/forge` exits 0), `inserter_endpoints_resolve_as_table` (A: belt piece 5 → press input; B and C: press output → turret / lab; D: drill → belt piece 0), `ambiguous_endpoint_rejects_with_moderror_on_facing`, `drill_binds_to_nearest_node_and_no_node_rejects`, `overlapping_footprint_rejects`, `ms_to_ticks_ceils_at_10_20_30_hz`. Also run `cargo test -p turts_pack --test m3_m2_stream` (Task 0): it must stay green.
- [ ] **Step 2:** Run: expect FAIL.
- [ ] **Step 3:** Implement. Empty phases capture no operation.
- [ ] **Step 4:** Run `cargo test -p turts_sim -p turts_pack` and `cargo check --workspace`. Expected: PASS.
- [ ] **Step 5:** Commit explicit paths.

### Task 6: Extract phase

**Files:** `crates/turts_sim/src/economy/extractor.rs`, test `crates/turts_sim/tests/extractor.rs`

**Interfaces:** Consumes Task 3 `extract_from_node`, Task 3a `ExtractorState` (bound at boot by Task 4). Produces `fn run_extract(...)` in `SimSet::Extract`; event `Extracted` (M2 kind).

- [ ] **Step 1:** Write `extractor_rate_with_carry_exact` (literal expected extract ticks for `per_second = 2` at 20 Hz: one ore at ticks 10, 20, 30...), `extractor_stops_when_full` (status `Full`, no write), `extractor_depletes` (remaining 21 → 21 ore then `Depleted`, node stays 0), `disabled_extractor_takes_no_budget_and_resets_carry`, `extractor_rate_at_10_and_30_hz`.
- [ ] **Step 2-4:** Run to fail; implement; run `cargo test -p turts_sim --test extractor`: PASS.
- [ ] **Step 5:** Commit explicit paths.

### Task 7: Transport phase

**Files:** `crates/turts_sim/src/economy/conveyor.rs`, tests in an in-file `#[cfg(test)]` module of `conveyor.rs` (`step_line` is `pub(crate)`), plus `crates/turts_sim/tests/conveyor.rs` for tick-level cases through `headless_app`

**Interfaces:** Consumes `BeltLine`. Produces `fn run_transport(...)` in `SimSet::Transport`; pure `fn step_line(line: &mut BeltLine, steps: u64)`.

- [ ] **Step 1:** Write `step_line_moves_run_as_block` (items at slots 0..2, head free: all move one slot in one step), `step_line_blocked_head_holds_run`, `last_slot_never_moves`, `transit_time_exact` (literal: an item placed in slot 0 of a 6×4 line at 4 slots/s and 20 Hz reaches slot 23 after exactly 23 steps; assert 23 steps, not a tick count that depends on the drop tick), `steps_per_tick_above_one` (slots_per_second 40 at 20 Hz = 2 steps per tick), `rate_at_10_and_30_hz`, `no_loss_over_1000_ticks` (count preserved), `items_keep_moving_after_split` (Review Focus 1), `line_clock_runs_when_empty` (carry accumulates).
- [ ] **Step 2-4:** Run to fail; implement; PASS.
- [ ] **Step 5:** Commit explicit paths.

### Task 8: Inserter phase

**Files:** `crates/turts_sim/src/economy/inserter.rs`, test `crates/turts_sim/tests/inserter.rs`

**Interfaces:** Consumes Tasks 3, 4, 6, 7. Produces `fn run_inserters(...)` in `SimSet::Inserters`.

- [ ] **Step 1:** Write `period_is_ten_ticks` (literal: pick at tick t, drop at t+5, next pick at t+10), `pick_requires_destination_room_now` (K7), `held_item_kept_when_destination_fills`, `belt_drop_only_into_first_slot_of_piece`, `belt_pick_takes_downstream_most_item`, `lowest_unit_id_wins_shared_source` (two inserters, one shell: the lower id gets it), `target_removed_sets_endpoint_none_and_hand_kept`, `disabled_inserter_frozen`, `hand_cap_two_moves_two_items` (stack size).
- [ ] **Step 2-4:** Run to fail; implement; PASS.
- [ ] **Step 5:** Commit explicit paths.

### Task 9: Machines and item recipes

**Files:** `crates/turts_sim/src/economy/production.rs` (hooks only), new code in `economy/machine.rs`; test `crates/turts_sim/tests/machine.rs`

**Interfaces:** Consumes Tasks 3, 4. Produces machine start/advance/output inside `run_production`; event `ItemsProduced { machine, job, outputs }`; `CancelProduction` on a machine uses `RunRefunds`.

- [ ] **Step 1:** Write `cycle_is_duration_plus_one` (literal: 41 ticks for the press), `insufficient_input_status_no_write`, `output_blocked_holds_progress_no_recharge`, `one_shell_when_room_returns`, `cancel_running_refunds_floor_charged_times_ratio` (2 ore), `cancel_output_blocked_refunds_and_credits_no_output`, `cancel_rejects_refund_destination_full_and_changes_nothing`, `enabled_machine_restarts_same_tick_after_cancel` (K8), `disabled_machine_frozen_but_cancel_works`.
- [ ] **Step 2-4:** Run to fail; implement; PASS.
- [ ] **Step 5:** Commit explicit paths.

### Task 10: Research

**Files:** `crates/turts_sim/src/economy/research.rs`, hooks in `production.rs`, `crates/turts_server/src/session.rs:376-384`; test `crates/turts_sim/tests/research.rs`

**Interfaces:** Consumes Task 9. Produces `Produce(station, technology)` handling for a `research` unit (it uses the reject reasons `PrerequisitesMissing`, `AlreadyResearched`, `ResearchInProgress` that Task 3a added); `ResearchCompleted { station, job, slot, technology }`; `fn modifier(research: &ResearchState, catalog: &ContentCatalog, slot: u8, unit_def: &ContentId, stat: WeaponStat) -> u64`.

- [ ] **Step 1:** Write `research_completes_after_ceil_duration` (200 ticks), `produce_without_shells_rejects_insufficient_items_and_leaves_claims` (Review Focus 4), `already_researched_rejects`, `second_request_same_slot_rejects_in_progress`, `locked_prerequisite_rejects`, `cancel_running_refunds_half`, `cancel_queued_refunds_all_and_releases_claim`, `modifier_is_computed_from_completed_set`, `server_admits_produce_for_research_unit`.
- [ ] **Step 2-4:** Run to fail; implement; PASS.
- [ ] **Step 5:** Commit explicit paths.

### Task 11: Combat

**Files:** `crates/turts_sim/src/economy/combat.rs`, test `crates/turts_sim/tests/combat.rs`

**Interfaces:** Consumes Tasks 3, 10. Produces `fn run_combat(...)` in `SimSet::Combat`; event `Fired { weapon, target, item, ammo, damage, applied }`.

- [ ] **Step 1:** Write `weapon_period_is_twenty_ticks`, `shot_debits_ammo_and_damages_in_one_operation`, `nearest_target_wins_tie_lowest_unit_id`, `out_of_ammo_no_shot`, `overkill_applied_is_min_and_target_removed_once`, `retarget_after_kill`, `hostility_other_ignores_same_owner`, `research_bonus_applies_in_completion_tick` (Production runs before Combat), `target_out_of_range_no_shot`, `removed_gatherer_receiver_is_tolerated` (Review Focus 5: remove a Store that an M2 gatherer targets, run 100 ticks, no panic, the gatherer re-selects or waits, conservation holds).
- [ ] **Step 2-4:** Run to fail; implement; PASS. If the gatherer test fails, fix the smallest M2 hook and report it.
- [ ] **Step 5:** Commit explicit paths.

### Task 12: Commands `Remove` and `SetEnabled`

**Files:** `crates/turts_sim/src/command.rs`, `crates/turts_net_protocol/src/messages.rs`, `crates/turts_server/src/session.rs` (`admit_kind`)

**Interfaces:** Consumes Task 3a reasons and Task 3 `remove_unit`. Produces sim `CommandKind::{Remove, SetEnabled}`, wire `PlayerCommandKind::{Remove, SetEnabled}`, `Disabled(BTreeSet<u32>)` handling, event `EnabledChanged { unit, enabled }`.

- [ ] **Step 1:** Write `remove_requires_ownership_then_removal_flag`, `remove_drone_by_command_rejected_not_removable`, `set_enabled_noop_when_same_value`, `set_enabled_not_toggleable_for_belt`, `wire_roundtrip_new_commands`, `server_admit_kind_covers_new_variants`, `cross_owner_commands_rejected_not_owner`.
- [ ] **Step 2-4:** Run to fail; implement; PASS.
- [ ] **Step 5:** Commit explicit paths.

### Task 13a: Views and audit (sim side)

**Files:** `crates/turts_sim/src/economy/{views,events}.rs`, `plugin.rs` (`CommittedTick` and its literal at `plugin.rs:153`), `crates/turts_server/src/economylog.rs`, `crates/turts_server/tests/economy_export.rs` (the `CommittedTick` literals at `:336` and `:568`)
**Test:** `crates/turts_sim/tests/views_m3.rs`

**Interfaces:** Produces `AuditState` fields from SPEC 12.1 using **public** `Raw...Audit` view types (not the `pub(crate)` sim state): `raw_output_inventories`, `raw_belts`, `raw_inserters`, `raw_extractors`, `raw_weapons`, `raw_healths`, `research`, `disabled`, `removed_units`; `CommittedTick` gains the new record lists and `removed`; a killed unit appears in the `healths` list of its removal tick with its last value (0).

- [ ] **Step 1:** Write `audit_rejects_state_change_between_operations_for_new_tables`, `equal_value_write_makes_no_change_record`, `killed_unit_has_final_health_zero_in_its_removal_tick`, `views_expose_no_pub_crate_types`.
- [ ] **Step 2-4:** Run to fail; implement; run `cargo test -p turts_sim` and `cargo test -p turts_pack --test m3_m2_stream`: PASS.
- [ ] **Step 5:** Commit explicit paths.

### Task 13b: Protocol 3 and export

**Files:** `crates/turts_net_protocol/src/{lib,messages}.rs`, `crates/turts_server/src/{export,outbound}.rs`
**Test:** `crates/turts_server/tests/export_m3.rs`, `crates/turts_net_protocol/tests/protocol3.rs`

**Interfaces:** Produces wire records from SPEC 12.2 (`HealthRecord`, `BeltPieceRecord`, `InserterRecord`, `ExtractorRecord`, `WeaponRecord`, `ResearchRecord`, `QueueState.enabled`, `InventoryState.port`, `UnitSnapshot.facing`); `NET_PROTOCOL_VERSION = 3`; the export filter keeps owner-only records off other slots.

- [ ] **Step 1:** Write `snapshot_for_slot1_has_no_owner_only_records`, `delta_for_slot1_has_healths_and_removed_only`, `late_join_snapshot_has_belts_hands_healths` (Review Focus 3), `killed_unit_delta_has_final_health_zero_and_removed`, `protocol_version_is_3_and_old_hello_rejected`, `postcard_roundtrip_every_new_record`.
- [ ] **Step 2-4:** Run to fail; implement; PASS. Run the wasm check and `cargo test -p turts_pack --test m3_m2_stream`.
- [ ] **Step 5:** Commit explicit paths.

### Task 14: Oracle and scenario suite

**Files:**
- Modify: `crates/turts_pack/Cargo.toml` (add `ron` as a dev-dependency, same version as the workspace pin, for the oracle's own parse)
- Create: `crates/turts_pack/tests/common/conservation.rs`, `crates/turts_pack/tests/forge_cases.rs`, `crates/turts_pack/tests/forge_timing.rs`, `crates/turts_pack/tests/forge_fixed_point.rs`, fixture packages under `tests/m3/packages/` (C11 node 21; C15 high-health target with preloaded magazine; C16 target at 12.0; `forge_locked` with a second technology that has the first as prerequisite)

**Interfaces:** Consumes everything above. Produces `fn check_operation(op: &OperationAudit, oracle_catalog: &OracleCatalog) -> Result<(), CheckFailure>` implementing `BAL-ore`, `BAL-shell`, `SHOT`, `RES`, `REFUND`, `DANGLING`, and `struct OracleHistory` (the oracle's own memory: technologies completed per slot, learned from `ResearchCompleted` events of earlier ticks; destroyed units) and `fn check_tick(ops: &[OperationAudit], history: &mut OracleHistory, oracle_catalog: &OracleCatalog) -> Result<(), CheckFailure>` implementing `DMG` once at the end of each tick (SPEC 14.1); `OracleCatalog::parse(dir: &Path)` reads the package source files itself.

- [ ] **Step 1:** Write instrument self-tests first: one hand-made bad `AuditState` per check that must fail (`bal_ore_fails_on_lost_item`, `bal_shell_fails_on_duplicated_shell`, `shot_fails_when_event_ammo_zero`, `res_fails_on_active_claim_without_job`, `dmg_fails_on_wrong_damage`, `refund_fails_on_wrong_ratio`, `dangling_fails_on_leftover_row`).
- [ ] **Step 2:** Write `forge_cases.rs` with one test per case C1-C17 (SPEC 14.2) calling `check_operation` after every operation **and `check_tick` at the end of every tick** (the C14 and C15 plants are red only in `check_tick`), with the trigger and fixture the spec names. C15 asserts its precondition (a `Fired` at the completion tick). C16 uses distance 12.0.
- [ ] **Step 3:** Write `forge_renamed_clone_same_trace` in `crates/turts_pack/tests/forge_renamed_clone.rs` (F1/E2: pack a copy of forge with every local id renamed, run the same commands, map the renamed ids back in the events, and assert the same operation trace), and `forge_timing.rs` (F4) with literal expected ticks, `forge_fixed_point.rs` (F6, SPEC 13.4 table) and a test for F7 (10 and 5 shots).
- [ ] **Step 4:** Run `cargo test -p turts_pack --test forge_cases --test forge_timing --test forge_fixed_point`. Expected: PASS. If the fixed point disagrees, find the cause by analysis and report it; do not edit the SPEC number to match.
- [ ] **Step 5:** Commit explicit paths.

### Task 15a: Client replica and ingest

**Files:** `crates/turts_client/src/{replica,ingest,debug}.rs`
**Test:** `crates/turts_client/tests/replica_m3.rs`

**Interfaces:** Consumes Task 13b wire records. Produces one generic keyed-record merge helper used by `replica.rs` and `ingest.rs` (remove the second hand-written copy of the merge); the replica applies `healths` before `removed`; debug state exposes the new records and a per-unit `health_trace`.

- [ ] **Step 1:** Write `replica_merges_new_records_through_one_helper`, `removed_unit_clears_all_records`, `health_trace_includes_killing_hit`, `late_join_replica_matches_server_view`.
- [ ] **Step 2-4:** Run to fail; implement; PASS. Run `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown`.
- [ ] **Step 5:** Commit explicit paths.

### Task 15b: Client presentation, controls and test hooks

**Files:** `crates/turts_client/src/{render,economy_ui,input,options,scenario}.rs`, `crates/turts_client_web/src/lib.rs`, `tests/browser/m2.js` (the exact key-set assertions at `:356-376`), `tests/browser/m3.js`

**Interfaces:** Consumes Task 15a. Produces facing rotation for belts and inserters; text panels (belt piece counts, machine and inserter status, health, research button, enable and remove controls); `UiAction::{Remove, SetEnabled}`; headless report fields `health_trace` and `research_completed_tick`; **state-triggered test hooks** `--produce UNIT,TECH --produce-when-inventory UNIT,ITEM,N` and a coordinator-controlled move signal (the existing hooks wait a fixed number of ticks, `scenario.rs:17-21`). The coordinator releases the signal only after the owner client's live replica report confirms research completion; do not replicate private research to the opponent. The exact test-only signal/report interface is to be fixed before T15b.

- [ ] **Step 1:** Write `panel_model_shows_blocked_reason`, `scenario_waits_for_replica_state` (the hook sends nothing until the replica shows the state), `options_parse_new_hooks`, and a browser case `m3_panel_actions_send_commands`. Update the `m2.js` key sets to include the new fields and check that the M2 browser suite stays green.
- [ ] **Step 2-4:** Run to fail; implement; PASS. Run the wasm check.
- [ ] **Step 5:** Commit explicit paths.

### Task 16: Mutation sweep, process and browser acceptance

**Files:** `tests/m3/refutations.json`, `tests/m3/refutations.py`, `scripts/m3-refutations.sh`, `scripts/acceptance.sh`, `scripts/t7.sh`, `scripts/m3-browser.sh`, `crates/turts_server/tests/m3_process.rs`, `.github/workflows/ci.yml`

**Interfaces:** Consumes everything. Produces the F1-F11 evidence run and `docs/measurements/m3.md`.

- [ ] **Step 1:** Write `refutations.json` with the 17 plants of SPEC 15.3 (path, search text, replacement text, expected failure text of the red check). Reuse the M2 runner pattern (`tests/m2/refutations.py`).
- [ ] **Step 2:** Write `m3_process.rs`: server plus two headless clients; slot 0 uses `--produce-when-inventory`; the test coordinator observes completion in the owner client's live replica report and releases slot 1's move signal; fail when the recorded `MoveTo` applied tick is not later than `ResearchCompleted`. Assert the `health_trace`: drone A ten steps of −10, ending at 0; drone B five steps of −20, ending at 0.
- [ ] **Step 3:** Run `bash scripts/m3-refutations.sh`. Expected: every plant baseline green, plant red, restored green. Run `bash scripts/acceptance.sh`, `bash scripts/t7.sh`, `bash scripts/m3-browser.sh`, `bash scripts/boundary.sh`.
- [ ] **Step 4:** Write `docs/measurements/m3.md` with the results, the machine type and the commit. Record measurements only; claim no budget.
- [ ] **Step 5:** Commit explicit paths. Wait for CI on the pushed branch (the M2 lesson: per-task tests missed a red fmt step).

## Self-review notes

- Spec coverage: SPEC 3 → T1, T2, T3a; 4 → T3a, T4, T7; 5 → T3a, T4, T8; 6 → T9; 7 → T4, T6; 8 → T3, T11; 9 → T10; 10 → T3a, T12; 11 → T4 and all phase tasks; 12 → T13a, T13b; 13 → T5, T14; 14 → T14; 15 → T0, T5, T14, T16.
- Acceptance: F1 → T5 (E1) and T14 (E2); F2 → T0, T4, T13a, T13b; F3 → T2, T4, T5; F4 → T14; F5 to F7 → T14; F8 → T13b; F9 → T16; F10 → T15b, T16; F11 → T15b, T16.
- Order: Task 5 does not boot the sim, so it can run before Task 4. Task 4 boots the forge map and needs Tasks 3, 3a and 5.
- Wire reasons (`NotRemovable` and the others) are added once, in Task 3a, so no later task breaks the exhaustive `wire_reason` match.
