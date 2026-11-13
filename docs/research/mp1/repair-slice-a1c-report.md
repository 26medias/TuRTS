RESULT=PASS

# Repair of task A1c after the review (backend-engineer, 2026-11-12)

Branch `pre-mp1/a1c-delete`, pushed. HEAD 3c53b96 (a merge commit of `origin/pre-mp1/integration` 86a5324 is inside). Not merged into integration.
Review: `docs/research/mp1/review-slice-a1c.md`. Evidence: `/home/julien/Projects/TuRTS/target/mc-evidence/pre-a1c-fix-*`. Every command ran through `exec_in_lane_v4.py`.
Hand-over file (items 4 and 5): `docs/research/mp1/a1c-handover.md` on the branch (the slice branch has `docs/research/mp1/`, so the file is there).

## Items

1. **High, N10 (and N13): DONE.**
   - New ledger rule `crates/turts_audit/src/orders.rs`, called for each command boundary (`lib.rs`). It is a pure catalog rule: it compares the logged order command with `before.catalog.order_definitions`.
   - Fields ported from `validate_order_policy` (`8722f8e:crates/turts_audit/src/replay.rs:1789-1925`), each named in its message: one definition (no missing, no duplicate); no duplicate capability entries; behaviour; target kind; allowed units (as a set); queue depth; policy present; policy id, behaviour, target kind, fire, engage, units (as a set), queue depth, unit capabilities (as a set); behaviour/target pair; Remove capability against the loaded unit policy (the unit type now comes from `economy.units`, because the ledger view has no `unit_types`).
   - Unit tests in `orders.rs` (5): each field red with its name, missing catalog and definition, duplicate, bad pair, Remove.
   - Conversion test `to_sim_copies_every_field_of_the_order_definition` (`crates/turts_server/src/orders.rs`): every order of muster and forge; expected values from the catalog, queue depth set to a value no package uses (7 and 9).
   - Red on N10 and N13 in both instruments (`pre-a1c-fix-t-N10`, `-N13`): the ledger text "orders: order command 1 differs from its immutable catalog definition in allowed units" / "... in queue depth" in 6 `mc_orders` log tests, `ord3_full_scene...`, `e9_base_gather_trace...`, `muster_log_passes_the_ledger_audit`; and the conversion test.
   - Side fix: the Muster fixture of `ledger_logs.rs` built orders with no policy; it now takes them from the server conversion (`#[path] mod server_orders`).
2. **Medium, N3: DONE.**
   - Rule (read from `movement/frame.rs:1095-1100` and `orders/run.rs:314-330`): a resting unit loses its progress when its pool is done, unless its current order is an Attack with a point. The arrival window is a tick end with the order still current and no `MoveTarget`.
   - `crates/turts_sim/tests/movement_resting.rs`: (a) a rush group (4 troopers) keeps `MoveProgress` with activity `Resting` at its slot in every window, also for the last lander; at the end no overlap. (b) A `order_move` group: the last lander holds no progress, and at the end no progress, no target, no pool. Both red on N3 (`pre-a1c-fix-t-N3`, `pre-a1c-fix-n3`). I found that a finished Move completes its order, so "a finished Move stays a Move" is not observable; the test states the observable rule.
   - Wider question: **chosen instrument: a sweep test, not a writer scan.** `movement::ecs::tests::the_movement_system_writes_only_movement_state` (in `movement/ecs.rs`) calls `integrate_movement` on the installed world and between 130 ticks of a muster scenario (rush, move, legacy single move) and compares the full `AuditState` without the allowed fields. Why: a syn scan cannot see `insert(progress)` (the type is a variable), and `movement_writers.rs` already pins the counts for the scanned types; the sweep sees a write to any audited field. Known gap (stated in the code): a constant write that already holds at every snapshot after tick 0.
   - Field list (module doc of `ecs.rs`): the movement system writes only `Position`, `MoveTarget`, `MoveProgress`, `PlainMoveProgress` (until A2b2), `PhysicalEscape` and the resource `MovementPools`. It reads `UnitOrders` (resting set only), `UnitType`, `Owner`, `UnitId`, `MapBounds`, `SimClock`, `Time<Fixed>`.
   - Plants: owner write (red at tick 0, `pre-a1c-fix-sweep-owner2`) and bounds grow (red, `-sweep-bounds2`). The first owner plant was green until the tick-0 call was added.
3. **Low, phase `Other`: DONE by naming the test.** The ledger cannot know a label of an operation with no command. The label is set at `economy/transaction.rs` (`capture_unit_order_operation`); plant `Orders` -> `Other` there is red in `orders::tests::depleted_gather_writes_last_failure`, `orders::tests::queued_gather_that_depletes_before_activation_fails_and_idles` and `tests/orders.rs::mixed_order_command_audit_covers_every_per_unit_operation` (`pre-a1c-fix-phase-source`). The comment at `economylog.rs:245` now says so.
4. **Low, `full_state`: DONE.** The doc comment of `full_state` (`economy.rs`) states the blind spot and where the property is held. The log was not widened. Hand-over text for A1d is in `a1c-handover.md` section 1.
5. **Low, E8 reorder and dependencies: DONE** in `a1c-handover.md` sections 2 and 3 (the three `Cargo.lock` lines under `turts_audit`: `"bevy"`, `"bevy_app"`, `"bevy_math"`).

## Registry

New rows (ci-native): `A1c-order-conversion-allowed-units` (N10), `A1c-order-conversion-queue-depth` (N13), `A1c-movement-resting-set` (N3), `A1c-movement-writes-only-movement-state`. Each ran red with its pinned text and restored clean (`pre-a1c-fix-row-n10`, `-n13`, `-n3`, `-sweep2`). The first sweep row had `--exact` with a short name and ran 0 tests (green by mistake); the row now names `movement::ecs::tests::...` and is red. `test_a1c_repair_rows_name_their_instrument` was added to `tests/m2/test_refutations.py`; `test_refutations.py` is green (`pre-a1c-fix-g-registry`).

## Fault table (merged head, combined set of sim, audit, checks, pack, server with `--features turts_sim/test-probes`; each plant: anchor count 1, restore by `git checkout`, tree clean after)

| Fault | Red in (first tests; count of red tests) |
| --- | --- |
| F1 first open slot | `session::tests::judge_accepts_a_matching_hello_and_claims_the_first_open_slot`, `claim_same_player_twice_is_rejected` (11) |
| F2 Position write in `combat.rs` | `movement_writers` count pin among 18 tests |
| F3 cost - 1 | accounting oracle, 38 tests (ledger green: by decision) |
| F4 refused order changes `next_pool_id` | `session::admission_tests::multi_produce_all_invalid_subjects_keep_per_unit_priority` (1) |
| F5 `unit_orders` empty | `ledger_view_keeps_every_field_of_real_tick_states`, `queued_group_attack_spreads_and_strictly_replays`, `e9_base_gather_trace_replays_node_depletion_in_process` (3) |
| N1 ECS speed | 32 tests, e.g. `a_pooled_group_reads_pool_activity_real_flag_separated_and_pool_count` |
| N2 ECS radius | 13 tests, e.g. `authored_force_move_never_fires_and_hold_remains_stationary_while_firing` |
| N3 resting set | `a_member_that_lands_after_a_point_attack_keeps_its_resting_progress`, `a_member_that_lands_after_a_move_holds_no_progress_and_the_pool_goes` (2) |
| N4 spawn position | 11 tests, e.g. `e6_every_committed_operation_and_tick_conserves` |
| N5 spawn type | 11 tests, same set |
| N6 dispatch cursor | `orders::tests::first_cohort_turn_uses_global_tick_then_idle_ticks_keep_rotation` (1) |
| N7 gather reach | 19 tests, e.g. `decoded_moving_gatherer_replays_frozen_command_and_automatic_effect` |
| N8 facing | `ledger_view_keeps_the_facing_that_the_browser_oracle_reads`, `late_join_snapshot_has_belts_hands_healths`, `public_forge_boot_and_remove_retain_authored_facing_and_m2_omission` (3) |
| N9 NotOwner -> NotApplicable | 7 tests, e.g. `per_unit_results_for_a_mixed_envelope` |
| N10 `allowed_units` empty | `orders::tests::to_sim_copies_every_field_of_the_order_definition` plus the ledger in `muster_log_passes_the_ledger_audit`, `queued_group_attack_spreads_and_strictly_replays`, `ord3_full_scene_blocked_gather_trip_validates` (11) |
| N12 bounds grow | 35 tests incl. the in-process oracle and `full_state_rebuilds_the_audit_state`; log-based oracle still blind (hand-over); the movement sweep is also red |
| N13 queue depth 255 | the same family as N10 (13) |
| P331 phase `Other` | the capture site plant: 3 sim tests (item 3) |

Labels: `pre-a1c-fix-t-<F1..N13>`. The reviewer's anchors still count 1 after the merge.

## Gates (merged head; exit codes)

- `cargo fmt --check` 0. Clippy `--workspace --all-targets --locked -D warnings` 0 (after one `type_complexity` fix). wasm32 check 0. `scripts/boundary.sh` 0. `driver_rule.py` 0. Node pure tests 0.
- `cargo test -p turts_audit -p turts_sim -p turts_server -p turts_pack -p turts_checks -p turts_client --features turts_sim/test-probes`: 1297 passed, 0 failed, 33 ignored.
- `cargo test -p turts_sim --locked --features test-probes`: 351 passed on the merged head (`pre-a1c-fix-merged-sim`).
- `cargo test -p turts_sim --locked --release --features test-probes --test kernel_reference`: 2 passed, 48.5 s (`pre-a1c-fix-merged-kernel`).
- `bash scripts/acceptance.sh` on 3c53b96: exit 0, 769 s (EL1, E8, E9, E10, DET1 included).
- Hashes on the merged head: traces base d4c73ebe..., haul 805309b2..., acme 19dcf9a3..., cedar 0db10e01... equal; stream bcbf72c2... equal; `probe_neutral` `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`.

## Not done / NOT TRACED

- The 8 headed `ci-browser` cases (still owed to the reviewer).
- The exploit of N10 on a play scenario and the play effect of N3 were not run; the instruments are tests, not scenarios.
- The parallel registry runner was not used (disk at 97%); each new row was run by its own plant and restore.
- `kernel_reference` was run on the merged head before the last three test-only commits (the later commits change no sim source).
