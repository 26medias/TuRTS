RESULT=PASS

# Repair of L0-CORE (review `review-l0-core.md`: 1 High, 3 Medium)

Branch `mp1/l0-core`, worktree `/home/julien/Projects/TuRTS-mp1-l0-core`, pushed to origin at 0816a87 (not merged).
Commits after the reviewed head ec88338:
- 2c188c1 the repair (code, proof scripts, errata);
- b08a8f9 a message on one assert of a new test;
- 0816a87 the plant records.

Evidence logs: `/home/julien/Projects/TuRTS/target/mc-evidence/<label>/command.log`.

## Per finding

**H1 FocusAlert: done.**
- I verified first. `rtk proxy grep -rn FocusAlert crates` found no `FocusAlert` in `turts_mod_api`. A mod writes `ButtonAction::Item(Action)` (`crates/turts_mod_api/src/interface.rs:153-157`; design 6.10 note [9], D:1657-1659).
- `WidgetAction` is a client type (`widgets/primitives.rs`). They are two types, so the mod API did not change.
- Now `WidgetAction::FocusAlert { x: f32, y: f32, unit: Option<u32> }` (`primitives.rs`).
- `activation/alert.rs::activate(x, y, unit, queue, state, acquired)` receives the fields.
- The core input has the same fields: `alerts::Input { x, y, unit }`.
- Tests: `widgets::tests::stub_contract::new_widget_actions_activate_nothing_until_their_owner_builds_them` (two `FocusAlert` forms).
- Red first: with a `todo!()` body in the stub, the test fails (`l0core-fix-red-todo-client`, 2 failed). Wrong stub: plant `h1-focus-alert-stub-queues-an-input`.

**M1 intercept: done.**
- `placement::intercept(core, input_index, event, world, view, now_ms, effects) -> bool` runs first in the event loop of `Core::step`. A `true` makes the loop `continue`.
- Its body is `core.placement.taking_input`. `State::taking_input` defaults to `false`, and no production code assigns it. `run_proof.sh` greps this: 0 assignments.
- The stub text is corrected (`placement.rs` header).
- Tests (`crates/turts_interaction/src/hook_tests.rs`):
    - `intercept_that_takes_the_input_sends_no_order_and_changes_no_selection` runs both cases (`false`: the right click sends an order and the left click selects unit 2; `true`: no `Send`, selection unchanged).
    - `intercept_stub_takes_no_event_in_a_fresh_core`.
- Red first: plant `m1-intercept-ignores-the-flag` (red, `a right click in the placement mode must send no order`). Plant `m1-intercept-always-takes-the-input` (red).
- With the real stub (`false`) all other tests are green and the proof holds (see M3). The test world is `tests/common/mod.rs`, included in the crate under `#[cfg(test)]` (`extern crate self as turts_interaction`).

**M2 owner types wired once: done** (table below, also in `errata.md` entry 10).
- `placement.rs`, `hints.rs` and `alerts.rs` of `turts_interaction` each have `State` (a `Core` field). `placement` and `hints` also have `Output` (`Effect::Placement` and `Effect::Hint`) and `Facts` (`WorldView::placement_facts` and `hint_facts`, with defaults; `FrameWorld` fields filled by `core_adapter/facts/{placement,hint}.rs`). `placement`, `hints` and `alerts` have `Input` (`InputEvent::Placement`, `Hint`, `Alert`) and `on_input`.
- Client: `widgets/sources.rs` has `CoreFeed { placement: place::Feed, hint: hint::Feed }` and `SourceContext.core`. It is built from `CoreRuntime::core()` at the start of `sync_widget_tree`. Adapter: `core_adapter/effects/{placement,hint}.rs`. Visuals: `visuals/placement.rs` (`Feed`) and `visuals/effects.rs` (`Pool`), each with `register(app)` and `draw(..)` called in `draw_world_visuals` after `draw_queued_targets` and before `update_health_fills`; `VisualInputs` has the two resources.

| Shared edit point | Does an owner still edit it? | Reason |
| --- | --- | --- |
| `Core` fields, `Core::new` | No | the owner edits its own `State` |
| `InputEvent` and the `step` match | No | one variant for each owner; the owner makes its input kinds inside its `Input` |
| `Effect` and the adapter match | No for `UI-PLACE`, `UI-HINT` | `UI-ALERT` uses `CameraChanged` and `SelectionChanged`; `UI-RESULT` has no Core effect |
| `WorldView`, `FrameWorld` | No for placement, hints | the other owners read the replica |
| `SourceContext` | No for placement, hints | `UI-HUD`, `UI-RESULT`, `UI2-*` read `replica` and the store getters. A new kind of core state needs a new part: one field and its literals |
| `VisualInputs`, `draw_world_visuals` | No for `UI-PLACE`, `L-FX-PRES` | `PRES-MODELS` and `FX2` (stage 2, `render.rs`) are not wired |

- Tests: `l0_core_stubs.rs` (`owner_inputs_through_step_add_no_effect_and_change_no_state`, `owner_input_handlers_are_empty_until_their_owner_builds_them`, `world_gives_empty_owner_facts_by_default`), `adapter_tests.rs` (`effect_family_stubs_record_nothing_until_their_owner_builds_them`, `owner_effect_variants_carry_the_owner_types`).
- Red first: `todo!()` bodies give 5 interaction reds and 2 client reds (`l0core-fix-red-todo-int`, `l0core-fix-red-todo-client`).
- Plants:
    - `g-<owner>-update-adds-an-effect` and `g-<owner>-on-input-adds-an-effect` (the three owners);
    - `d-effect-family-placement-records` and `d-effect-family-hint-records`;
    - `v-placement-draw-adds-a-visual` and `v-effects-draw-adds-a-visual` (the scene oracle goes red, so the two call lines in `draw_world_visuals` are live).
- The empty `Facts` and `Feed` types cannot be observed, so they have no plant.

**M3 purity proof: done.** `docs/research/mp1/l0-core/step_equal.py` now has these checks.
- Each argument is its parameter name (57 checked).
- `return` becomes `continue` only at loop depth 0 (13).
- The hook calls are checked in place: `intercept` is the first statement of the loop; the three `update` calls follow the `prune` after the loop and come before the `let`; the three arms are present once.
- The self-test has 12 mutations, each DIFFERENT: the reviewer's five (inner `continue` to `return`, `input_index` to `0`, `&mut Vec::new()` at a call, `now_ms` to `0`, the hook calls moved) and 7 more.
- `run_proof.sh` result: 2261 tokens equal, `SELF-TEST OK`.
- Plant records `c-step-proof-sees-a-dropped-return`, `c-step-proof-sees-an-argument-changed-to-0`, `c-step-proof-sees-the-intercept-removed`.

**L1 dead stubs: done.**
- `cancel.rs` now holds the arm `CancelProduction`, moved unchanged. `move_equal.py`: 299 tokens equal, and DIFFERENT on a changed body. Oracle tests: `widgets_lifecycle` cancel tests (green).
- `leave.rs` is reached through `activate_widget` for `WidgetAction::Input { name: "leave_match" }`. Control test `other_input_actions_still_reach_the_core`.
- `rally.rs` is deleted: a rally is an order of the smart path, not an activation.
- Plants `l1-*`.
- The only `#[allow(dead_code)]` the branch adds is on the `#[cfg(test)]` module `test_common` in `turts_interaction/src/lib.rs`: it includes the integration-test helper file, and the unit tests use only part of it. No stub carries one.

**L2 effect stubs: done.** `apply_alert` and `apply_result` have no producer (D:1916, MatchResult, `CameraChanged`, `SelectionChanged`). I removed them. Plan lines for `UI-ALERT` and `UI-RESULT` must be struck (errata 13).

**L3 alert trigger stubs: not changed.**
- Reason: a unit-less alert needs a change in the tail of `collect_alert_events` (`units.retain`, the `units.is_empty()` exit, unit `0`). That is more than 15 lines and it is behaviour.
- `UI-ALERT` owns that tail. No frozen shape depends on it (errata 14).

**L4 typo: done.** The three stub headers were rewritten.

**Errata:** `docs/research/mp1/l0-core/errata.md` (committed): the 7 of the first pass and 11 new (numbers 8 to 18).

## No change of behaviour
- `visuals/scene_oracle.txt`: `cmp` against 9044820, exit 0.
- Existing test files: 0 lines deleted in `widgets/tests.rs`, `adapter_tests.rs`, `scene_tests.rs`, `tests/controls_rows.rs`, `tests/forge_research_tooltip.rs`, `tests/l0_core_stubs.rs` (`git diff --numstat`). The setup lines added: `core:` in 13 `SourceContext` literals of existing tests, two `init_resource` lines in `scene_tests.rs`. No expected value changed.
- Interaction test list: 115 before, 123 after, 0 removed, 8 added (all are my tests).
- Static test-name check (`static_tests.py`): 1849 at base, 1870 at head, 0 removed.

## Gates on the clean head 0816a87 (all exit 0, wrapper exit 0)
| Gate | Label |
| --- | --- |
| `cargo fmt --all -- --check` | `l0core-fix-gate-fmt` |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | `l0core-fix-gate-clippy` |
| `cargo test -p turts_client` (457 passed, 4 ignored) | `l0core-fix-gate-client` |
| `cargo test -p turts_interaction` | `l0core-fix-gate-interaction` |
| `cargo test -p turts_checks` (formula ratchet unchanged) | `l0core-fix-gate-checks` |
| `cargo test --workspace` (1898 passed, 36 ignored, 0 failed) | `l0core-fix-gate-workspace` |
| wasm32 check | `l0core-fix-gate-wasm` |
| `scripts/boundary.sh` | `l0core-fix-gate-boundary` |
| `driver_rule.py tests/browser` | `l0core-fix-gate-driver` |
| `mp1_plants.py --self-test` | `l0core-fix-gate-plantself` |
| `run_proof.sh` | exit 0 (no label; run in the shell) |

Plants: 27 records in `docs/research/mp1/plants/l0-core.json`. `l0core-fix-plants-run-1`: 26 red-restored, 1 failed (`m1-intercept-always-takes-the-input` expected text was wrong: the setup assert failed first). I named that assert and ran the 4 `placement.rs` records again: `l0core-fix-plants-run-2`, 4 of 4 red-restored. The two `m1-*` records name b08a8f9 as their commit; the other 25 name 2c188c1.

## NOT TRACED
- A `--list` at clean 9044820 (the base list is the `before` list of the first pass).
- The browser build and any pixel path: no headed run.
- The cost of `CoreFeed::from_core` in each frame (empty types now; no measure).
- A pack of a package with `leave_match` (the row is at `version_table.rs:446`; I did not run it).
- Which alert triggers have no unit (L3).
- The L0-SPLIT move A proof (`l0-split/run_proof.sh`) does not apply as it is: `draw_world_visuals` has two more calls. I did not re-run it. The scene oracle is the behaviour oracle (errata 16).
- The CI `agents` job and the roster check.
