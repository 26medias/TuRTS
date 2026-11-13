RESULT=PASS

# Repair of L0-SIM (build mode, simulation-core-engineer)

Worktree `/home/julien/Projects/TuRTS-mp1-l0-sim`, branch `mp1/l0-sim`, pushed to origin (451f7b6..665a6e7). Cache folder: `mc-mp1-l0-sim`. Evidence logs: `/home/julien/Projects/TuRTS/target/mc-evidence/l0sim-fix-*/command.log`. The ruling `ruling-l0-sim-order.md` was read, and its section 5 was applied.

## Commits (on 451f7b6, oldest first)

- 8da440c fix(sim): one holder for each UnitId, UnitId immutable (M1, M2)
- 4150868 perf(sim): the per-unit scans read the UnitId index (M4)
- a0f04ca fix(sim): production counts progress in Q units with `rate_step`; `steps_at_rate` deleted (M3)
- 7634e15 fix(sim): the tick order of the ruling (H1, H2, L3)
- 665a6e7 docs(mp1): plant record, 30 plants (the file `docs/research/mp1/plants/l0-sim.json`)

Each message ends with the Co-Authored-By line. The red of each test was run on the tree before the fix (the test was in the tree, the fix was not). The red logs are named below. The tests and the fix are in the same commit.

## Findings

| Id | State | Test (red evidence) |
| --- | --- | --- |
| H1 | done | `plugin::tests::the_sets_of_one_tick_run_in_the_pinned_order` (17 sets, Supply and Construction before Commands) and `each_slot_set_holds_a_system_in_its_schedule`. Red on the old chain: `l0sim-fix-red-order/command.log` (left `[Construction, Supply, Advance, Commands, Repair, ...]`; the slot test panicked at the Supply lookup). Green: `l0sim-fix-green-order2`. |
| H2 | done (as the ruling orders) | Production is the one pay pass; the Construction set no longer sits beside it. The frozen order is the test above. The stubs stay empty (`construction.rs`, `income.rs`, new `repair.rs`). The walk over sites and producers is `L-PAY` (the ruling says so). Plants (o1) and (o2) are red on a wrong order. |
| M1 | done | `unit_index::tests::a_second_holder_of_one_id_panics`, `a_second_holder_panics_in_a_world_that_simplugin_did_not_build`, and `the_same_holder_may_insert_its_own_id_again`. The old `two_holders` test was replaced. Red on 451f7b6: `l0sim-fix-red-m1m2/command.log` ("test did not panic as expected" for both). Code: `unit_index.rs:53-58` (`claim`). |
| M2 | done | `unit_index::tests::the_unit_id_component_is_immutable` compares `<UnitId as Component>::Mutability` with `Immutable`. Red: same log (TypeId differs). Code: `world.rs:19` (`immutable`). All crates compile: `cargo clippy --workspace --all-targets` exits 0 (this covers sim, server, pack, audit, checks and the others). |
| M3 | done | `steps_at_rate` and its test are deleted (`turts_rules/src/progress.rs`, `tests/progress.rs`). `run_production` uses `rate_step(Q, rate)` (`production.rs:299-302`). Q is the new resource `RateQ` (`power.rs:53-64`, default 1, set by `L-PWR`). Tests: `a_job_at_a_low_rate_takes_duration_over_rate_ticks_at_every_start_phase` (1/2 with duration 3: 6 ticks at each start phase; 1/3 with duration 10: 30), `a_job_that_changes_rate_keeps_the_progress_it_made` (1/1 then 1/2: 15). Red on the old code: `l0sim-fix-red-m3/command.log` (5 ticks, not 6, at first tick 2; 14, not 15). The test `a_job_advances_at_the_rate_of_the_power_stub` was updated: Q is 2 there, and each tick adds 1 at 1/2. |
| M4 | done | Four tests, each red on the scan: `the_owner_lookup_reads_the_unit_index_and_does_not_scan` (gather), `the_extractor_pass_reads_the_unit_index_and_does_not_scan`, `the_arm_lookups_read_the_unit_index_and_do_not_scan` (inserter), `the_machine_recipe_lookup_reads_the_unit_index_and_does_not_scan`. Red: `l0sim-fix-red-m4c/command.log` (4 FAILED). The four `find_unit` helpers are deleted, and every call site uses `unit_entity` (gather has 11 sites). `rtk proxy grep -n find_unit` in the four files gives none. |
| M5 | not built; owner named | Plan line 687 "Footprint and supply resource ids to indices". At this head: `Footprint` is an inline enum with one variant, `Circle { radius }` (`turts_mod_api/src/schema.rs:28-30`), so it has no id to resolve. A grep for `provides`/`consumes` in `crates/turts_mod_api/src` gives nothing: the supply fields (design row U5) arrive with plan task `L0-API1` (plan line 644) and the supply rule with `L-PWR` (plan line 913). The one parse of a content id in a tick that the design names is `economy/combat.rs:270` (damage type, armour): plan line 879 `L-DMG`. NEEDS DECISION: the plan finding table (line 1348) names "L0-SIM install indices" for regate-performance M4, but the fields do not exist yet. I propose `L0-API1`/`L-PWR` for supply and `L-DMG` for the combat parse. |
| L1 | owed to `L-ROOM0` | A server with no config is silent. At this head `main.rs:76` always inserts `Immediate`, so the case cannot happen. The log line belongs with the lobby (design 6.7.5). |
| L2 | owed to the P9 lane | `production.rs:446` (`try_output`): one scan and one Vec of circles for each blocked producer per tick. More than 10 lines (it needs a grid), so it is not repaired here. |
| L3 | done | `income.rs`, `construction.rs`, `power.rs`, `plugin.rs` doc comments follow the ruling. |

Also done from the ruling: the admission systems `session.rs:120`, `session.rs:1123`, and `command_script.rs:45` use `.after(SimSet::Construction)`. New tests (red on the old order, green now, `l0sim-fix-red-order-server`, `green-order2`): `session::admission_tests::the_production_admission_system_is_ordered_against_every_set_before_the_commands` and `command_script::tests::the_script_system_is_ordered_against_every_set_before_the_commands`. They turn on the ambiguity check of `FixedPreUpdate` (error level). The old chain gave "9 pairs of systems with conflicting data access". After the change there is none.

## Dense `Vec<Option<Entity>>` by id: not taken

Ids are dense but not bounded. `IdAllocators::allocate_unit` only adds 1 and never reuses an id (`ids.rs:33-36`; start value 1 at `ids.rs:18`). A vector by id would keep one slot for every id ever allocated, so memory grows with churn, and tests may use any u32 (`set_next_values`, `ids.rs:28`). Memory for 3.6 M ids is about 29 MB at 8 bytes. The map stays. (The 3.6 M number is my estimate for 5,000 units at 1% deaths per tick for one hour at 20 Hz: NOT MEASURED.)

## Hash checks (`l0sim-fix-hashes`, exit 0): all equal

- trace base d4c73ebee90e0aef84e58df4e31787ef3756b7e5d3a589ffdde2224ccfcdd7b2: equal
- haul 805309b26e717dfe3daa0e5597ebd6f21adde7ef3f93ec71a3610bd97bff3148: equal
- acme 19dcf9a3759096c312c11a8e89c2c0ac272dd3ccd868aacf9ac8a4357b91d1d2: equal
- cedar 0db10e012cd1c3a41c68fb054a7ab39ec325451f4db4fb912f72dbddd5f33718: equal
- `tests/m3/m2_operation_stream.json` bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b: equal
- `probe_neutral`: `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`: equal

## Plants (`l0sim-fix-plants1`, exit 0)

`mp1_plants.py` (copy from `origin/mp1/integration`, self-test 51 of 51 ok): "30 plant(s) in 25 command group(s), 30 red-restored, 0 failed"; `source_unchanged` true. Plant (r1) `steps_at_rate` was dropped. Plants (d), (e1), (e2), (g2) were re-pinned (the code changed). 13 plants are new: (m1), (m2), (m3a), (m3b), (m4a) to (m4e), (o1) to (o5). The records carry `commit` 7634e15. I ran the runner with `--plants-dir` on one folder that holds only `l0-sim.json`: the other record files in the lane are in the old format (the reviewer found the same fault; it is not an L0-SIM matter).

## Gates (all through `exec_in_lane_v4.py`, no headed run)

| Gate | Label | Exit |
| --- | --- | --- |
| fmt --check | l0sim-fix-gates | 0 |
| clippy workspace all-targets, `-D warnings` (the CI command) | l0sim-fix-gates | 0 |
| test turts_sim, turts_server, turts_pack, turts_audit, turts_checks (972 passed, 0 failed, 33 ignored in all) | l0sim-fix-gates | 0, 0, 0, 0, 0 |
| wasm32 check | l0sim-fix-gates | 0 |
| `scripts/boundary.sh` | l0sim-fix-gates | 0 |
| `scripts/acceptance.sh`, `MC_JOBS=4` (30 passed, 0 failed) | l0sim-fix-acceptance | 0 |

The formula ratchet (`turts_checks`): only the name list changed (`steps_at_rate` out, `rate_step` in, `formula_ratchet.rs:~505`). The reason is that the rule is deleted. No count baseline changed. The ratchet is green.

## NOT TRACED

- T7 headed: not run, as the task says (the reviewer runs it).
- `acceptance.sh`: the review text says 31 ignored process tests; the log has 30 `test result` lines with 30 passed. I did not reconcile the 31 and 30 (some `test result` lines may cover a file with more than one test, or a test may be filtered).
- Whether `apply_order` can run inside the `capture_operation` of `try_output` (ruling item 4): `L-STO`.
- The 5,000-unit tick time (the review "not from this task" note): not touched, not measured.
- Each call site of the old gather `find_unit`: the one test covers `owner_of`; the other 10 sites are one text change (`find_unit(` to `unit_entity(`), and no test covers each one.
- `RateQ` is never set by production code (it is the seam for `L-PWR`); the job `elapsed_ticks` is in units of 1/Q tick, so the views would show mixed units if Q were above 1. Q is 1 in every catalog today. `L-PAY`/`L-WIRE` own the wire shape (done and total).
- The report file is not committed (as ordered).
