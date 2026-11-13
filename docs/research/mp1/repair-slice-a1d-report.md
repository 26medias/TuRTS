RESULT=PASS

# A1d repair pass: report (backend-engineer, 2026-11-12)

Branch `pre-mp1/a1d-oracle`, pushed. Head `e2ba109c79d38df38bd51202f804b68d829ffd57` (3 commits on `afb9c6de`; the new one is `e2ba109c`, [skip ci]). Not merged. Lane `/home/julien/Projects/TuRTS-pre-a1d`, cache `target/mc-pre-a1d` (it was gone at the start of the pass; I made it again, empty). Evidence labels `pre-a1d-r-*` under `/home/julien/Projects/TuRTS/target/mc-evidence/`. Scratch scripts: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/a1d/`.

No work-list item stopped. Two things go beyond the text of the ruling and are named here (section 6).

## 1. Revert (work items 1 and 2)

`git show afb9c6de:<path> | cmp - <path>` at HEAD, exit code of each:

| File | cmp |
| --- | --- |
| `crates/turts_pack/tests/common/conservation.rs` | 0 |
| `crates/turts_pack/tests/common/mod.rs` | 0 |
| `crates/turts_pack/tests/common/forge_support.rs` | 0 |
| `crates/turts_pack/tests/forge_cases.rs` | 0 |
| `crates/turts_pack/tests/forge_fixed_point.rs` | 0 |
| `crates/turts_pack/tests/forge_renamed_clone.rs` | 0 |
| `crates/turts_pack/tests/forge_timing.rs` | 0 |
| `crates/turts_pack/Cargo.toml` | 0 |
| `crates/turts_audit/src/lib.rs` | 0 |

- `crates/turts_audit/src/source.rs` is gone. `grep -rn 'turts_audit::source' crates` exits 1.
- `crates/turts_audit/Cargo.toml`: `ron` removed, `bevy*` removal kept. `cargo metadata --locked` (`pre-a1d-r-meta-clean`): deps of `turts_audit` are serde, serde_json, turts_mod_api, turts_net_protocol, turts_sim; `ron present: False`.
- Instrument check: a planted `ron` line makes `cargo metadata --locked` fail (`pre-a1d-r-meta-planted`, exit 101, "cannot update the lock file"); restored, `git status` empty.
- `git diff afb9c6de HEAD -- Cargo.lock` shows only `- "bevy"`, `- "bevy_app"`, `- "bevy_math"` (the `turts_audit` block).

## 2. L1: the three row plants write real state

Commit `e2ba109c`. All three rows now plant in `crates/turts_sim/src/movement/ecs.rs` (same anchor as the A1c sweep row, `.map_or(Vec2::ZERO, |bounds| bounds.size);` then `let tick_hz = world`):
- `A1d-run-constants-durations`: the real `ProductionDurations` grows by 1 tick each step (reviewer OWN-D).
- `A1d-run-constants-resources`: `world.remove_resource::<ProductionDurations>()` (reviewer OWN-R).
- `A1d-run-constants-unit-types`: a new entry is registered in the real `UnitTypes` each step through `UnitRegistry::register_unit_type` (the ruling's one-file form; a `UnitDef` literal in `ecs.rs`). I did not use the reviewer's OWN-U (it needs a helper in `registry.rs`, two files).
- `test_a1d_run_constants_rows_name_their_instrument` (`tests/m2/test_refutations.py`) now asserts the plant path `movement/ecs.rs`, that `views.rs` is absent from the row, and that the replacement writes through `world.`. Red first: `AssertionError: 'crates/turts_sim/src/economy/views.rs' != 'crates/turts_sim/src/movement/ecs.rs'` (`pre-a1d-r-reg-red`); green: 21 tests OK (`pre-a1d-r-reg-green`). Row ids are unique (59 rows).
- Registry runner (`pre-a1d-r-m2-rows`, 5 of 5 RED-RESTORED, `"failed": []`): `A1d-run-constants-durations` "tick 1: the compiled durations changed inside the run"; `-unit-types` "tick 1: the unit type registry changed inside the run"; `-resources` "tick 1: the resource presence changed inside the run"; also `E8-mixed-cost-partial-commit` ("rejected seq 5 is write neutral") and `A1c-movement-writes-only-movement-state`.
- A2b1 conflict (L2): this row set is appended at the end of the array. The task that merges second keeps both sets. Not tried here (NOT TRACED).

## 3. M1: the run-constant writer scan (work item 4)

Files: `crates/turts_checks/tests/run_constant_writers.rs` (new), `crates/turts_checks/tests/support/writers.rs` (new, shared visitor, `pub mod writers;` in `support/mod.rs`), `crates/turts_checks/tests/fixtures/run_constant_writers/planted.rs` (new), `movement_writers.rs` (now uses the shared visitor), `fixtures/movement_writers/planted.rs` (three new planted lines).

- New arm: `ResMut<T>`, `Mut<T>`, `NonSendMut<T>` (kind `mut-param`) in the shared visitor, so both scans have it.
  - Red first: the planted lines 43, 47, 51 were added to `movement_writers.rs` `scan_finds_every_planted_write` while the old private visitor was still in place: `left` lacked the three `mut-param` hits (`pre-a1d-r-mw-red`). Green after the switch (`pre-a1d-r-mw-green`).
  - The movement table needed no change: production has no `ResMut`/`Mut`/`NonSendMut` of a movement type (the scan found 0 new hits).
- Types: `MapBounds`, `ProductionDurations`, `UnitTypes` (all write kinds); `remove_resource` of the 14 types of `AuditResourcePresence` (`economy/views.rs:1176-1191`, kind `remove_resource`). The test `presence_types_are_the_fields_of_audit_resource_presence` keeps the list equal to `views.rs`.
- Roots: `crates/turts_sim/src`, `crates/turts_server/src`, `crates/turts_pack/src`. Production filter: no `#[cfg(test)]`, no `tests` directory, no listed test-module file.
- Tests: 3 (`scan_finds_every_planted_write`, `presence_types_are_the_fields_of_audit_resource_presence`, `run_constant_writers_match_the_pinned_multiset`). First run was red because the pins were empty and wrong guesses (`pre-a1d-r-rc-first`, `-second`); green at `pre-a1d-r-rc-green`.

Allowed entries (multiset by file, kind, type, count). Each was read in code:

| Entry | Kind, type, count | Reason (verified at) |
| --- | --- | --- |
| `crates/turts_sim/src/map.rs` | construct `MapBounds` 1 | `load_map` direct map loading: `world.insert_resource(MapBounds {..})` at `map.rs:154`, at boot |
| `crates/turts_sim/src/bootstrap.rs` | construct `MapBounds` 1, `ProductionDurations` 1, `UnitTypes` 1 | package staging: `UnitTypes::from_catalog` `:164`, `MapBounds {` `:424`, `ProductionDurations(durations)` `:434`; `install_package` inserts them at `:442`, `:443`, `:450`, at boot |
| `crates/turts_sim/src/plugin.rs` | method `UnitTypes` 1 | `init_resource::<UnitTypes>()` at `:61`, the empty registry at app build |

Ruling items that I checked and that did not hold as written (so no pin):
- `boot_m3.rs:1172-1178` is in `#[cfg(test)] mod tests` (`boot_m3.rs:626-627`), so the scan does not read it.
- `economy/transaction.rs:1807-1822` (the four `remove_resource` calls) is in `#[cfg(test)] mod tests` (`transaction.rs:1104-1105`), so the scan does not read it either. No production `remove_resource` of a presence type exists in the three roots (grep of `remove_resource` in `crates/*/src`: only `ReplicaState`, `EconomyLogSink`, `SpreadIndexCache`, `IdAllocators`/`Tick`/... in the test module).

Known gap (also in the file header): `install_package` inserts `staged.types`, `staged.bounds`, `staged.durations` as plain values; the scan sees them only through the constructors in staging.

Plants, each applied, red in the named scan, restored with `git checkout`, `git status --porcelain` empty:

| Plant | Scan and result |
| --- | --- |
| SRV-B: `get_resource_mut::<turts_sim::MapBounds>()` in `session.rs` `admit_player_commands` | `run_constant_writers`: `("crates/turts_server/src/session.rs", "method", "MapBounds") pinned 0, found 1 at session.rs:383` (`pre-a1d-r-plant-srvb`) |
| `ResMut<MapBounds>` parameter in `session.rs` | `run_constant_writers`: `"mut-param", "MapBounds" ... found 1` (`pre-a1d-r-plant-srvresmut`) |
| `ResMut<MovementPools>` parameter in `economy/combat.rs` | `movement_writers`: `"mut-param", "MovementPools" ... found 1 at combat.rs:58` (`pre-a1d-r-plant-combatpools`) |
| `remove_resource::<UnitTypes>()` in `economy/combat.rs` | `run_constant_writers`: `"method", "UnitTypes" ... found 1 at combat.rs:59` (`pre-a1d-r-plant-combatremove`) |
| the `ResMut` and `NonSendMut` names removed from the shared arm list | both `scan_finds_every_planted_write` tests red (`pre-a1d-r-plant-armoff`) |

## 4. Known gap in `no_content.rs` (work item 5)

Header added at `crates/turts_checks/tests/no_content.rs:1-9`: whole-word match, compound names (`ore_items`) not split, 49 identifiers (193 uses, 29 files) in the ruling's probe, top `client_shell`; owner `qa-lead`, deadline before the first MP1 package with new content ids merges. The count is the ruling's (run R5); I did not repeat the probe (NOT TRACED).

## 5. Gates (all through `exec_in_lane_v4.py`; head content = `e2ba109c`; each `wrapper_exit` 0 unless named)

| Gate | Label | Result |
| --- | --- | --- |
| `cargo build -p turts_checks --example m2_variants --locked` (first) | `pre-a1d-r-build-m2variants` | exit 0 |
| `cargo test -p turts_audit -p turts_sim -p turts_server -p turts_pack -p turts_checks -p turts_client --locked --no-fail-fast` | `pre-a1d-r-gate-tests` | exit 0, 1300 passed, 0 failed, 33 ignored (1297 + 3 new) |
| `cargo test -p turts_sim --locked --features test-probes` | `pre-a1d-r-gate-probes` | exit 0, 351 passed |
| release `kernel_reference` | `pre-a1d-r-gate-kernel-release` | exit 0, 2 passed |
| combined set `-p turts_sim -p turts_audit -p turts_checks -p turts_pack -p turts_server --features turts_sim/test-probes` | `pre-a1d-r-combo-base` | exit 0, 868 passed, 0 failed |
| wasm32 check | `pre-a1d-r-gate-wasm` | exit 0 |
| `bash scripts/boundary.sh` | `pre-a1d-r-gate-boundary` | exit 0 |
| `driver_rule.py tests/browser` | `pre-a1d-r-gate-driver` | exit 0 |
| `tests/m2/test_refutations.py` | `pre-a1d-r-gate-unittest` | exit 0, 21 OK |
| fmt check, clippy `--workspace --all-targets --locked -- -D warnings` | `pre-a1d-r-fmt-final`, `pre-a1d-r-clippy-final` | exit 0, exit 0 |
| `--set m3-all --workers 9 --alsa` | `pre-a1d-r-m3-all` | 17 of 17 RED-RESTORED, `"failed": []`, 145 s (C1 `RES: inventory 1 uses 5 > 4`, C6 `REFUND: job 1 refunds ...`, C10 `DANGLING: ...`, C13 `SHOT: ...`, C14/C15 `DMG`) |
| `--set ci-native` 5 rows (see section 2) | `pre-a1d-r-m2-rows` | 5 of 5 RED-RESTORED |
| `bash scripts/acceptance.sh`, one time, MC_JOBS=4 | `pre-a1d-r-acceptance` | exit 0, 1279 s (load 12 to 25 during the run) |

Equal values:
- Trace sha256 (`pre-a1d-r-hash-trace`): base `d4c73ebee90e0aef84e58df4e31787ef3756b7e5d3a589ffdde2224ccfcdd7b2`, haul `805309b26e717dfe3daa0e5597ebd6f21adde7ef3f93ec71a3610bd97bff3148`, acme `19dcf9a3759096c312c11a8e89c2c0ac272dd3ccd868aacf9ac8a4357b91d1d2`, cedar `0db10e012cd1c3a41c68fb054a7ab39ec325451f4db4fb912f72dbddd5f33718`: all four equal.
- `tests/m3/m2_operation_stream.json` `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b`: equal.
- `probe_neutral.sh` (`pre-a1d-r-hash-probe-neutral`): `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`, feature off equals on, exit 0.
- N12 (bounds grow, `movement/ecs.rs`) on the combined set (`pre-a1d-r-combo-n12`): 38 red, including `decoded_moving_gatherer_replays_frozen_command_and_automatic_effect`, `e9_base_gather_trace_replays_node_depletion_in_process`, `full_state_rebuilds_the_audit_state` and `movement::ecs::tests::the_movement_system_writes_only_movement_state`. Restored, `git status` empty.

## 6. Additions beyond the ruling text (named, not silent)

1. The new scan also has the method `resource_scope` (it gives a `Mut<T>` of a resource), next to the `WRITE_METHODS` and `remove_resource`. The movement scan keeps its seven methods.
2. The shared visitor moved out of `movement_writers.rs` into `support/writers.rs`, so that both scans use one arm. The pinned movement table and its counts are unchanged (the test is green with no edit of a count).

## 7. NOT TRACED

- The A2b1 merge conflict in `tests/m2/refutations.json` (L2): not tried.
- The compound-name probe of the ruling (R5): not repeated.
- The reviewer's C6 probe was not run again in this pass; the C6 registry row is red in `m3-all` with the `REFUND` text.
- The reviewer's plants OWN-D, OWN-R and OWN-U as such were not applied. The three registry rows plant the same writes (OWN-D, OWN-R) and a one-file unit-type write, and run red through the registry runner.
- The 8 `ci-browser` cases and EL1 in release (owed to the reviewer, as before; no browser was started).
- `plant.py list` in my scratch chain printed a traceback (it still named the deleted `source.rs`). It touched no repository file.
- The doc edits for D, TDI and the plan (section 3 of the ruling) are not made here; they are on `mp1-design`.

## What I changed

On `pre-mp1/a1d-oracle` (diff of the new commit vs `a921096b` is the revert, the L1 rows and test, the scan files and the `no_content.rs` header). This report file, not committed.

# Follow-up (re-check findings M1, L1, M2 text, L3; 2026-11-12)

FOLLOWUP=PASS

New head `28e551d9e9d4096e3fb4cd67d366d28d125dce33` on `pre-mp1/a1d-oracle` (one commit on `e2ba109c`, [skip ci]), pushed. Evidence labels `pre-a1d-f-*`. Test-only change in `crates/turts_checks/tests/`; no production code.

`git diff --stat e2ba109c..HEAD`:

```
 .../tests/fixtures/movement_writers/planted.rs           |  8 ++++++++
 .../tests/fixtures/run_constant_writers/planted.rs       | 12 ++++++++++++
 crates/turts_checks/tests/movement_writers.rs            |  9 +++++++--
 crates/turts_checks/tests/run_constant_writers.rs        | 13 ++++++++++---
 crates/turts_checks/tests/support/writers.rs             | 16 ++++++++++++++--
 5 files changed, 51 insertions(+), 7 deletions(-)
```

## 1. M1: two more methods

- `get_resource_or_init` and `get_resource_or_insert_with` are now in `WRITE_METHODS` of `run_constant_writers.rs` (11 names) and of `movement_writers.rs` (9 names).
- Fixture lines: `fixtures/run_constant_writers/planted.rs:43` (`get_resource_or_init::<UnitTypes>()`) and `:47` (`get_resource_or_insert_with::<MapBounds>(..)`); `fixtures/movement_writers/planted.rs:60` and `:64` (the same two names on `MovementPools`). The method hit is on the line of the call.
- Red first, with the names missing from the lists (`pre-a1d-f-fix-red`): both `scan_finds_every_planted_write` tests failed; `left` lacked the `(43, "method", "UnitTypes")`, `(47, "method", "MapBounds")` hits and the two movement hits. Green with the names in the lists (`pre-a1d-f-fix-green2`, `cargo test -p turts_checks --locked`: exit 0). One expected line number of mine was wrong in the first green try (59 and 63 instead of 60 and 64); I fixed the numbers, not the scan.

## 2. L1: `&mut` line in the run-constant fixture

- `fixtures/run_constant_writers/planted.rs:50`: `fn bounds_ref(bounds: &mut MapBounds)`; expected hit `(50, "&mut", "MapBounds")`.
- Mutant W1 (`node.mutability.is_some()` to `is_none()` at `support/writers.rs:106`, the `&mut` arm) on the committed tree (`pre-a1d-f-w1`): `run_constant_writers::scan_finds_every_planted_write` FAILED (it was green under W1 before), `movement_writers::scan_finds_every_planted_write` FAILED, and both pinned tests FAILED. Restored with `git checkout`; `git status --porcelain` empty.

## 3. M2 text: known gaps

`crates/turts_checks/tests/support/writers.rs` header ("Known gap (A1d re-check ...)") now lists the forms that the scan does not see: a value received and inserted with no `&mut`/`ResMut`/turbofish/constructor; a copy that is changed and inserted again (`let mut b = *world.resource::<T>()` then `insert_resource(b)`, also through `Commands`); a type alias; `use ... as`; a write in a `macro_rules!` body. It says that code review is the only cover today, and that a runtime guard (change ticks of the run constants and the presence set) is an OPEN item, owner: technical-director decision. The guard is not built.

## 4. L3: correction to section 3 of this report

The report cited `economy/transaction.rs:1104-1105` as the test module of the four `remove_resource` calls. That is a different module. The calls (`transaction.rs:1807-1822`) are in `#[cfg(test)] mod raw_audit_capture_tests` (`transaction.rs:1638-1639`, ends `:1828`), as the re-check says. The conclusion (the scan does not read them) is unchanged. No code change.

## 5. The real tree with the new method names

Both pinned tests are green at `28e551d9` (`run_constant_writers_match_the_pinned_multiset`, `movement_state_writers_match_the_pinned_multiset`; `pre-a1d-f-gate-checks`: 38 tests passed in `turts_checks`). The scan lists a hit as a difference from the pins, so a real use of the new names would be red. Real hits of `get_resource_or_init` or `get_resource_or_insert_with` on a run constant or on a movement type: 0. No legal existing use, so nothing to stop on and nothing pinned.

## 6. Gates (each `wrapper_exit` 0)

| Gate | Label | Result |
| --- | --- | --- |
| `cargo test -p turts_checks --locked` | `pre-a1d-f-gate-checks` | exit 0, 38 passed |
| `cargo test -p turts_audit -p turts_sim -p turts_server -p turts_pack -p turts_checks -p turts_client --locked --no-fail-fast` | `pre-a1d-f-gate-tests` | exit 0, 1300 passed, 33 ignored |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | `pre-a1d-f-clippy` | exit 0 |
| `cargo fmt --all --check` | `pre-a1d-f-fmt-final` | exit 0 |
| `bash scripts/boundary.sh` | `pre-a1d-f-boundary` | exit 0 |

No `acceptance.sh` run: the change is test-only in `turts_checks` and touches no simulation, server or log code. The earlier acceptance run (`pre-a1d-r-acceptance`, exit 0 at `e2ba109c`) still stands for the other code.
