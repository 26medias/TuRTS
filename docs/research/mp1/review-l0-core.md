VERDICT=FAIL HIGH=1

# Review of L0-CORE (candidate ec88338, base 9044820)

Reviewer: `technical-director` (boundary lens). Worktree: `/home/julien/Projects/TuRTS-mp1-l0-core-review`, detached at ec88338. Date: 2026-11-12.

Summary: the split of `Core::step` is behaviour-neutral (my own checks agree with the builder's proof, and the tests catch the faults that the proof cannot see). The fast gate is green. But the frozen client shape does not give each later owner its own file and one call line. One frozen shape (`WidgetAction::FocusAlert`) cannot carry what design 7.9 needs, so it would force a change request after `L0-EXIT`. That is the one High.

## Findings

### High

**H1. `WidgetAction::FocusAlert` has no field, so an alert row cannot say which alert it is.** Needs decision.
- `crates/turts_client/src/widgets/primitives.rs:41-42` declares `FocusAlert` as a unit variant. The arm at `crates/turts_client/src/widgets/activation.rs:197-199` calls `alert::activate(&mut queue, &state, acquired)`. The stub at `widgets/activation/alert.rs:7-11` gets no row, no entity, no point and no unit.
- Design 7.9 (D:2059-2061): "A click on a line moves the camera to its place (`FocusAlert`); 'Ore Hauler idle' also selects the hauler." Design 6.10 note [9] (D:1657-1659): "the engine gives each alert item the action `FocusAlert`". Every alert row then carries the same action value, and the activation cannot find the place or the unit.
- Today each alert row carries its place in the action: `WidgetAction::CenterCamera { x, y }` at `crates/turts_client/src/widgets/sources.rs:1031-1036`.
- Plan 5.1 item 5 (plan:467) freezes "the `WidgetAction` variants `Place { constructor, content }` and `FocusAlert`" at `L0-EXIT`. The builder followed the plan, but this shape forces `UI-ALERT` to file a change request (5.2), or to add a system parameter to `activate_widget`, whose lines `UI-HUD` and `UI-RESULT` also edit.
- Options: (a) `FocusAlert { x: f32, y: f32, unit: Option<u32> }`, the same form as `CenterCamera`, so the core action is complete for a headless agent (D74). (b) `FocusAlert { alert: u64 }`, an alert instance id that the core adapter resolves through the alert list. (c) Keep the unit variant and budget one change request at `UI-ALERT`. I recommend (a): it costs one variant now and needs no lookup.

### Medium

**M1. The hook position cannot do placement: a click is already dispatched when `placement::update` runs.**
- `crates/turts_interaction/src/lib.rs:237-241`: the hooks run after the event loop and the second `prune`. The stub contract at `placement.rs:3-5` says "`UI-PLACE` fills the body and adds no call line to `step`".
- Design 7.2 (D:1807): "Right click or Esc leaves placement and never gives an order." GHO1 plant (d) (D:2470): "the smart path runs before the placement mode: a right click in placement sends a move". A left click to place also runs `complete_selection_gesture` first (`lib.rs:605-606`) and changes the selection, so the Command Centre selection (the hint X4 condition, D:2016) is gone before the hook runs.
- Probe (run P1 below): with a print in `placement::update`, 27 of 149 hook calls with a `PointerUp` already held `Send(OrderEnvelope { order: "forge:order_move", ... })` in `effects`, and the selection was already changed.
- Fix: add now one pre-dispatch call at the top of the event loop, `if placement::intercept(self, input_index, event, world, view, now_ms, &mut effects) { continue; }`, with an empty body that returns `false`. Keep the post-loop `update` for the ghost. Correct the stub text.

**M2. Each owner still edits shared types, so "its own file and at most one call line" (plan:779) does not hold.**
- `Core` fields and `Core::new` (`lib.rs:35-53`, `:74-95`): no slot for the placement mode (`UI-PLACE`) or the hint state (`UI-HINT`, D:2011 "The interaction core evaluates them").
- `InputEvent` (`lib.rs:927`) and the one exhaustive match (`lib.rs:164-235`): `WidgetAction::Place` must reach the core as a new input. `place.rs` gets `&mut CoreInputQueue`, and no `InputEvent` variant exists for it. `UI-ALERT` needs the same for `FocusAlert`. `UI-PLACE` and `UI-ALERT` are concurrent (plan:975, plan:1006: no input path joins them), and both add a variant and an arm at the same place.
- `Effect` (`lib.rs:1036-1059`) and the exhaustive match in `drive_core_adapter` (`core_adapter.rs:1045-1070`): the same pair adds variants and arms side by side.
- `WorldView` (`world_view.rs:30-71`) has no sites, footprints, build area, placement gap, supply, held jobs, order results or replica tick. `UI-PLACE` (placement rule inputs) and `UI-HINT` (conditions, and "once per applied delta", plan:990) both add methods there and in `FrameWorld` (`core_adapter.rs:279`).
- `SourceContext` (`widgets/sources.rs:55-70`) has no core state, so `predicate_placing` (`sources/place.rs:7`) and `value_hint` (`sources/hint.rs:6`) cannot read anything. Each new field must go into 10 struct literals, one of them inside `sync_widget_tree` (`widgets/tree.rs:201`), whose writers are `UI-HUD` and `UI-ALERT` (plan:313). `UI-PLACE` is concurrent with both.
- `VisualInputs` (`visuals.rs:89-103`, destructured at `:308-322`): `L-FX-PRES` (an effect pool resource) and `UI-PLACE` add fields at the same lines (shape from `L0-SPLIT`).
- Fix: give each owner an empty type in its own file now: `Core { placement: placement::State, hints: hints::State }`; `InputEvent::Placement(placement::Input)` and `InputEvent::Alert(alerts::Input)`; `Effect::Placement(placement::Output)` and `Effect::Hint(hints::Output)`; one `SourceContext.core: &CoreFeed` field whose parts are owner types; one `VisualInputs` field per owner. Each owner then edits only its own file.

**M3. The purity proof does not compare call arguments, and it maps every `return` to `continue`.**
- `docs/research/mp1/l0-core/step_equal.py:88-105` inlines a method body and drops the arguments of the call. Lines 94-96 turn every `return` into `continue`, also in an inner loop.
- My mutations (run R3) that stay EQUAL: an inner `continue` changed to `return` in `on_restore_groups`; `input_index` changed to `0` at the `on_ui_action` call; `&mut wheel_camera_actions` changed to `&mut Vec::new()`; `now_ms` changed to `0` at the `on_pointer_down` call; the hook calls moved in front of the loop.
- The product is correct. My own check (R4) shows that every call passes its parameters by name and in order, and that the rest of `lib.rs` is token-equal to the base. The tests catch two of the blind mutations (F1, F2). Fix: compare each argument with its parameter name. Map `return` to `continue` only at loop depth 0 of the method.

### Low

**L1. Three dead activation stubs are unreachable and untested, and their signatures do not fit their owners.** `activation/cancel.rs:7-13`, `leave.rs:8-14`, `rally.rs:7-13`. Fault F4 (cancel queues an input) stays green. `CancelProduction` already has an arm (`activation.rs:135-185`). `leave_match` arrives as `WidgetAction::Input` (`turts_mod_api/tests/api_0_2_4.rs:524-526`). A rally is an order of the smart path, not an activation. These are file markers, not frozen shapes. Say so in the stub text, or delete them.

**L2. Three of the four effect stubs have no producer in the design.** Design 7.5 (D:1916) lists the core outputs as placement, `Ghost`, the hint state and `FocusAlert`. A match result comes from `MatchResult`, not from `Core`, so `apply_result` (`core_adapter/effects.rs:119-121`) has no producer. A camera move and a selection already exist as `CameraChanged` and `SelectionChanged`. The stubs take no payload (`effects.rs:109-121`), so each owner changes the signature anyway.

**L3. Alert trigger stubs return `Vec<u32>`, units only** (`feedback/alert_state.rs:9-17`). An alert with no unit and no `focus` (Lost connection, D:2068) cannot be expressed. This is a stage-2 shape (`UI2-CONN`), so `L0-EXIT2` can change it.

**L4. Typo "The the"** at `hints.rs:10` and `alerts.rs:10`.

## Behaviour neutrality of the split

- The 13 handler bodies match the old arms token for token. 13 `continue` became `return`. The one inner `continue` in `on_restore_groups` stays at `lib.rs:497`. The accumulators `ignored_wheel_inputs` and `wheel_camera_actions` stay in `step` and pass in by `&mut` (`lib.rs:161-162`, `:227-228`).
- Every early exit ended the arm, and the arm was the last statement of the loop body. So `return` from a method is the same as `continue` of the loop.
- Effect order: each handler pushes to the same `effects` in the same order. The hooks are empty and run at one point, after the second `prune` (`lib.rs:237-241`).
- The rest of `lib.rs` is token-equal to the base, apart from the three `pub mod` lines (R4).

## My faults and mutations (each one: red or green, an exact restore, then green)

| Id | Change | Result | Restore |
| --- | --- | --- | --- |
| F1 | `lib.rs` call of `on_wheel`: `&mut wheel_camera_actions` changed to `&mut Vec::new()` | RED exit 101: `authored_wheel_pan_is_one_actual_frame_pulse`, `tests/rebound_actions.rs:708` | `git checkout`, `cmp`=0, status empty |
| F2 | `lib.rs` `on_restore_groups`: inner `continue` changed to `return` | RED exit 101: `restore_groups_filters_bad_slots_foreign_dead_and_duplicate_ids`, `tests/groups.rs:188` | `git checkout`, `cmp`=0, status empty |
| P1 | probe: `placement::update` prints `effects` and the selection when a `PointerUp` is in the events | 149 prints, 27 with `Send(OrderEnvelope ...)` already in `effects` | `git checkout`, `cmp`=0, status empty |
| F4 | `activation/cancel.rs` pushes `UiAction { name: "fault" }` | GREEN (all `turts_client` tests): dead code, nothing calls it | `git checkout`, `cmp`=0, status empty |
| R3 | 8 mutations of the proof input (scratch harness) | 4 RED (statement swap in a handler, effect order changed, `&mut` target in a body, and the `minimap` target), 4 EQUAL (see M3) | not in the tree |

The fresh greens after the restores: the plant run R5 (green on clean source before each plant and after each restore) and the clean head `ec88338` with an empty `git status`.

## The 7 errata

The builder's list of 7 errata is not in any file that I found (I searched `docs/research/mp1/`, the plant records and the commit messages). I judged the 2 that the brief names. For the other 5, I judged the plan deviations that I found in the diff. NOT TRACED: whether these 5 are the builder's 5.

1. Plant (b) cannot be red in the scene oracle. **Accept.** `visuals/scene_tests.rs` has no widget or source reference (grep: 0 hits). A 0.2.3 layout cannot name a 0.2.4 variant, so no 0.2.3 path reaches a stub. The replacement plants `b-*` are red through the stub contract tests (records).
2. `activation/alert.rs` added. **Accept.** `FocusAlert` is a new `WidgetAction` (plan:772), and R-S gives each writer one file. But see H1.
3. No `ItemField` stubs (inferred). **Accept.** `Value::Item(field)` reads `item.get(field)` with no dispatch for each variant (`sources.rs:167-168`). `item_field_key` (`sources.rs:1254-1281`) is a key map. The owner's list source writes the field.
4. Effect stubs with no calling arm (inferred). **Accept as file markers.** The `Effect` variants do not exist yet. See L2.
5. The lead test of plant (a) is `km3.rs` `shift_click_toggles` (inferred). **Accept.** R5 shows it red: "toggle baseline must contain the clicked unit before Shift+click".
6. A token proof and a `#[test]` grep in addition to `--list` (inferred). **Accept as an addition.** The two `--list` logs of the builder differ by exactly the 3 new hook tests. See M3 for the blind spots.
7. Alert triggers in `feedback/alert_state.rs` and `alert_connection.rs` (inferred). **Accept.** This is the rule "a stub file of its owner task" (plan:772).

## What I ran (all through `exec_in_lane_v4.py`, head ec88338; logs in `/home/julien/Projects/TuRTS/target/mc-evidence/<label>/`)

| Label or command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `l0core-rev-interaction-green` `cargo test -p turts_interaction` | interaction tests green | any failure | exit 0 |
| `l0core-rev-client-green` `cargo test -p turts_client` | client green, the scene oracle included | any failure | exit 0 (249 lib tests, 1 ignored) |
| `l0core-rev-checks-green` `cargo test -p turts_checks` | boundary tests green | any failure | exit 0 |
| `l0core-rev-wasm-check` wasm32 check | builds for wasm32 | an error | exit 0 |
| `l0core-rev-boundary` `scripts/boundary.sh` | T1 edges | "boundary" failure | exit 0, "boundary ok" |
| `l0core-rev-proof-green` `run_proof.sh` | builder proof EQUAL, self-test OK, no test removed | DIFFERENT or removed | EQUAL 2261 = 2261, 4/4 red, +14 tests |
| `l0core-rev-clippy-two` `cargo clippy -p turts_interaction -p turts_client --all-targets -- -D warnings` | clean clippy at the head (the builder's run was at aade379, dirty) | a warning | exit 0 |
| `l0core-rev-plants-lib` `mp1_plants.py --files lib.rs --native-only` | plants (a) and (c) red and restored | a green plant | 2 red-restored |
| R3: scratch `mut_proof.py` | the proof sees control-flow and argument faults | EQUAL on a behaviour change | 4 EQUAL (M3) |
| R4: scratch `rest_equal.py` | `lib.rs` outside `step` unchanged; arguments equal the parameters | a token difference or a mismatch | equal (2368 = 2368); 13/13 OK |
| `l0core-rev-fault-wheelpan`, `l0core-rev-fault-restore-return`, `l0core-rev-probe-hook-position`, `l0core-rev-fault-cancel-stub` | F1, F2, P1, F4 | see the table above | see the table above |
| `rtk proxy diff` of the builder `--list` logs | step 1 removes no interaction test | a missing name | +3 hook tests only |
| plant record anchor check (`git apply --check` and anchor count) | the 12 records apply at ec88338 | count not 1 | 12/12 count 1 |

## NOT TRACED

- The builder's list of 7 errata (see above).
- A `--list` at clean 9044820 (the builder's "after1" log ran at 9044820 with the step-1 change in the tree).
- That the version table refuses a 0.2.3 manifest that names a 0.2.4 variant (I did not run it).
- Whether `On<Activate>` fires on a right click (open point 16).
- The 10 client plant records (I ran only the 2 for `lib.rs`).
- The pixel paths and headed runs (none in this task).

## What I changed

The repository: none (each fault was restored, `cmp`=0, and `git status` is empty at ec88338). Files: this report, and scratch files under the session scratchpad.
