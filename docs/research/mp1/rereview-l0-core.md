VERDICT=PASS

# Re-review of the L0-CORE repair (candidate 0816a87; first review at ec88338)

Reviewer: `technical-director` (boundary lens). Worktree `/home/julien/Projects/TuRTS-mp1-l0-core-review`, detached at 0816a87. Date: 2026-11-12. Commits after ec88338: 2c188c1 (the repair), b08a8f9, 0816a87 (plant records).

Summary: H1, M1, M3, L1, L2 and L4 are closed. M2 is closed for every concurrent stage-1 pair. Two shared points remain, and an input order separates each one, so they are not High (M-R1, L-R2). The repair adds no new High. The split stays behaviour-neutral. The repaired proof is red on all 13 of my mutations. My 2 new product faults are red. All 27 plant records are red and restored. The merge with `mp1/integration` f372c77 is clean.

## 1. The first-review findings

| Finding | Status | Evidence |
| --- | --- | --- |
| H1 `FocusAlert` without data | Closed | `WidgetAction::FocusAlert { x, y, unit }` (`widgets/primitives.rs:44-48`); the arm passes the fields (`widgets/activation.rs:154-156`); the core input `alerts::Input { x, y, unit }` (`turts_interaction/src/alerts.rs:24-28`). No mod type changed: `FocusAlert` is not in `turts_mod_api`, and a mod writes `ButtonAction::Item(Action)` (design D:1657-1659). The plan 5.1 item 5 text must change (errata entry 8). |
| M1 hook too late | Closed | `placement::intercept` is the first statement of the event loop (`lib.rs:181-183`). The test `hook_tests.rs:13-51` runs both cases, so it can fail. My fault F5 is red in 92 tests. |
| M2 shared edit points | Closed for the concurrent pairs; two ordered points remain | See section 2. |
| M3 proof blind spots | Closed | My 4 blind mutations of the first review plus the moved hooks are now DIFFERENT, and so are my 8 new ones (run R3). |
| L1 dead stubs | Closed | `cancel.rs` holds the moved arm (299 = 299 tokens). `leave.rs` is live for `leave_match`. `rally.rs` is deleted. No stub has `#[allow(dead_code)]` now. The only one left is on the `#[cfg(test)]` module `test_common` (`lib.rs:23-25`). |
| L2 effect stubs with no producer | Closed | `apply_alert` and `apply_result` are removed. See 3e. |
| L3 alert trigger return type | Left to `UI-ALERT`: **accept** | `UI-ALERT` is the only stage-1 writer of `feedback.rs` (plan 3.4, plan:317). The return type is in its own files (`feedback/alert_state.rs`, and the tail of `collect_alert_events` that it owns). `UI2-CONN` comes later (stage 2). No frozen shape depends on it. |
| L4 typo | Closed | The headers are rewritten (`hints.rs:1-15`, `alerts.rs:1-14`). |

## 2. M2: each stage-1 owner, its files, and any shared line that remains

| Owner | Its own files now | A shared line that remains? |
| --- | --- | --- |
| `UI-PLACE` | `turts_interaction/src/placement.rs`; client `activation/place.rs`, `core_adapter/effects/placement.rs`, `core_adapter/facts/placement.rs`, `widgets/sources/place.rs`, `visuals/placement.rs` | Yes, if it follows plan:977 ("the footprint list rebuilt per applied delta, not per frame"): see M-R1. |
| `UI-HINT` | `hints.rs`; `effects/hint.rs`, `facts/hint.rs`, `sources/hint.rs` | One line in `drive_core_adapter` pushes `InputEvent::Hint` (nothing pushes it today), plus the per-delta cache (M-R1). That is within "at most one call line" (plan:779). It merges after `UI-PLACE` (plan:986). |
| `UI-ALERT` | `alerts.rs`, `activation/alert.rs`, `feedback.rs`, `feedback/alert_state.rs`, `audio.rs` | `alert_rows` in `sources.rs` (`:1060` area) must emit `FocusAlert` in place of `CenterCamera`. `UI-HUD` edits other lines of `sources.rs` (build menu), and `UI-ALERT` is stacked on `UI-HUD`, so the two are ordered. `tree.rs` is ordered in the same way (R-S). |
| `UI-HUD` | `sources/hud.rs`, `activation/cancel.rs` (now live), `values.rs`, `tree.rs` | None beyond the plan's own list. |
| `UI-RESULT` | `sources/result.rs`, `activation/leave.rs`, `client_shell/notices.rs` | `LeaveMatch` is a wire message (design D:1039, D:1165). `leave::activate` gets only `&mut CoreInputQueue` (`activation/leave.rs:8-12`), so `UI-RESULT` must add a system parameter to `activate_widget` (`activation.rs:51-57`): see L-R2. |
| `L-FX-PRES` | `visuals/effects.rs` (`Pool`, `register`, `draw`), `debug.rs` (its own field function, plan 3.4:315) | None from L0-CORE. `register(app)` can add the system that fills `Pool` from the replica. The visuals index in plan:967 edits other concern files, but that is outside L0-CORE. |
| `UI-LABEL` | not wired by L0-CORE | One concern call line and maybe one `VisualInputs` field. No other stage-1 owner edits `VisualInputs` now, so there is no conflict. |

The builder's remaining points: `PRES-MODELS` and `FX2` are stage 2 (`render.rs`), and they are not frozen until `L0-EXIT2` (plan 5.1 item 7). A new kind of core state needs one `CoreFeed` field and one line in `from_core` (`widgets/sources.rs:78-96`), not the 22 literals, because the literals use `CoreFeed::empty()`. **Both are acceptable for `L0-EXIT`.** Neither touches a concurrent pair.

## 3. New defects of the repair

### Medium

**M-R1. The fact builders are stateless and run every frame. The per-delta rule of the plan needs a shared edit.**
- `frame_world` calls `facts::placement::build(replica, content)` and `facts::hint::build(replica, content)` (`core_adapter.rs:567-568`). `frame_world` runs in each `drive_core_adapter` call (`core_adapter.rs:773`), so it runs every frame. Each `build` returns a new `Facts` by value. It has no store for the last result and no applied-delta signal.
- The plan: `UI-PLACE` "the footprint list rebuilt per applied delta, not per frame" (plan:977). The `UI-HINT` counter test allows "at most 1 hint evaluation for each applied delta" (plan:990). The stub text itself promises the same (`facts/placement.rs:2-3`).
- To keep a cache, the owner must change the `frame_world` call lines or its parameters. `UI-PLACE` and `UI-HINT` both edit those lines. They are ordered (`UI-HINT` merges after `UI-PLACE`, plan:986), so this is a rebase, not a concurrent conflict.
- Fix (small): `build(&mut Facts, replica, content)`, which updates a `Facts` kept in `CoreRuntime` (one field for each owner type, wired now), so the owner can skip the work when the replica tick did not change.

### Low

**L-R1. The `taking_input` contract says "takes every input". Read literally, that breaks bookkeeping.**
- `placement.rs:24` says `taking_input` is "`true` while the mode takes every input". The intercept runs before the `match` (`lib.rs:181-183`). So a literal "every input" also swallows `KeyUp`, `FocusLost`, `PointerLeft` and `UiCapture`, which leaves keys and buttons held. It also swallows `InputEvent::Placement` itself, so `on_input` cannot run while the mode is on.
- It does not stop the camera: `camera_controller.step` reads `events` on its own (`lib.rs:287-299`). An intercepted `Wheel` also skips the `ignored_wheel_inputs` push of `on_wheel`, so Ctrl+wheel then reaches the camera.
- The body is the owner's file, so this is a text fix: say that `intercept` takes only the placement clicks and Esc, and never the bookkeeping events or `InputEvent::Placement`.
- Not run (code read). It cannot happen today, because nothing sets the flag (`run_proof.sh` grep).

**L-R2. `leave::activate` cannot send `LeaveMatch` with its parameters.** `UI-RESULT` adds one parameter to `activate_widget`. No concurrent owner edits that header, so there is no conflict.

**L-R3. The `taking_input` grep in `run_proof.sh` is blind to struct-literal initialisation.** It matches `taking_input\s*=[^=]` only. My fault F5 (a `Default` with `taking_input: true`) passes the grep (exit 1, no match). The tests catch it: 92 failures, among them `intercept_stub_takes_no_event_in_a_fresh_core`. No fix is needed; the test is the instrument.

**L-R4. Errata entry 16 is incomplete.** `l0-split/run_proof.sh` at the head fails move B too, not only move A. In run R6, with the repair lines removed, move A is EQUAL. Move B tokens are equal (2951 = 2951), but the proof prints `NOT-CALLED: placement_facts`, `NOT-CALLED: hint_facts` (the new `WorldView` methods in the same file) and DIFFERENT. No gate runs either script (grep of `.github` and `scripts`: 0 hits).

### Checked and correct

- 3a. **Hook position for GHO1 (d) and each input kind.** Each input kind reaches `step` as an event: pointer, key, wheel, minimap, UI action, `ui_select`, and the harness sequences. The intercept sees each one before its handler, so the place is right. The camera keeps its own path (L-R1), and the design does not ask to stop the camera in placement.
- 3b. **`CoreFeed::from_core` cost.** `place::Feed` and `hint::Feed` are empty structs (`sources/place.rs:10`, `sources/hint.rs:9`). So `CoreFeed` has size 0 and no heap work: one resource lookup for each `sync_widget_tree` run (`tree.rs:201-224`). Order: `drive_core_adapter` runs `.before(UiPresentationSet)` (`core_adapter.rs:1347-1351`), and `sync_widget_tree` is in that set (`widgets/mod.rs:85-96`), so the feed is from the same frame. Not measured. The future cost is the owner's (the client cost rule, errata entry 10).
- 3c. **`CancelProduction` moved to `cancel.rs`.** The body is token-equal (299 = 299, 4 `return` each, `run_proof.sh`). Only the binding types change (`Res` to `&`, by auto-deref), and it compiles. No change of behaviour.
- 3d. **`leave_match`.** Before the repair, `Input("leave_match")` went to the core as `UiAction`. No branch of `dispatch_named_action` matches that name (`lib.rs` around 786-940), so nothing happened, unless an order hotkey was "leave_match". A package that writes it is refused as unbuilt 0.2.4 (`turts_mod_runtime/src/variants.rs:304-309`, `unbuilt.rs:182`). So no package that packs today changes behaviour. My fault F6 shows that the test of the guard is live.
- 3e. **`rally.rs` deleted, and `apply_alert`/`apply_result` removed.** By the design, rally is a smart rule before Move, or L then a click (D:1845-1846), which is the core path, not an activation. Two plan texts name the removed items: plan:771 and plan:1245 (`UI2-HUD`: "rally flag and line (`widgets/activation/rally.rs`)"), plan:1008 (`UI-ALERT`: "the alert family function") and plan:1018 (`UI-RESULT`: "the result family function"). The design core outputs are placement, `Ghost`, the hint state and `FocusAlert` (D:1916). `FocusAlert` gives the existing `CameraChanged` and `SelectionChanged`, which the adapter already reads after `step`. **Accept**, with the plan errata 11 and 13 applied by REC. `UI2-HUD` is stage 2.

## 4. Neutrality

- `visuals/scene_oracle.txt`: `cmp` against 9044820 gives exit 0. `scene_oracle_matches_the_committed_fixture ... ok` in `l0core-rerev-client-green`.
- Existing test files: 0 lines deleted from 9044820 to 0816a87 (`git diff --numstat`, each file `N 0`). The lines added inside existing tests are setup only: 10 `core: sources::CoreFeed::empty()` in `widgets/tests.rs`, 1 in `sources.rs` tests, 1 each in `tests/controls_rows.rs` and `tests/forge_research_tooltip.rs`, and 2 `init_resource` lines in `scene_tests.rs`. No assert or expected value changed.
- **L0-SPLIT move A cover is not lost** (run R6). I removed the 4 repair lines from `visuals.rs` (2 destructure fields, 2 `draw` calls) and ran `l0-split/stmt_equal.py`: RESULT EQUAL. Move B tokens are also equal (see L-R4). The two new calls are live: plants `v-*` are red in the scene oracle.

## 5. The proof and my faults

Run R3 (scratch harness; the CLI of the repaired `step_equal.py`, unmutated EQUAL). All 13 mutations are DIFFERENT:

- The 5 from my first review: an inner `continue` changed to `return`; `input_index` changed to `0`; `&mut Vec::new()` at a call; `now_ms` changed to `0`; the hooks moved in front of the loop.
- 8 new ones:
    - placement and hints `update` swapped;
    - the intercept's `continue` changed to `break`;
    - `&mut Vec::new()` as `effects` of a `hints::update` call;
    - the intercept `input_index` changed to `0`;
    - the `Placement` arm with `now_ms` changed to `0`;
    - the `Alert` arm calling `hints::on_input`;
    - the intercept body widened (`INTERCEPT-BODY`);
    - (the moved hooks are counted in the first list).

Product faults (each one: an exact restore with `git checkout`, `cmp`=0, an empty status, then a fresh green in `l0core-rerev-green-after-faults`, exit 0):

| Id | Fault | Result |
| --- | --- | --- |
| F5 | `placement::State` `Default` gives `taking_input: true` (`placement.rs:25-28`) | RED exit 101, 92 tests (`intercept_stub_takes_no_event_in_a_fresh_core`, `arrow_pans`, ...). The `run_proof` grep stays blind (L-R3). |
| F6 | the guard `name == "leave_match"` changed to `"leave_matc"` (`activation.rs:67`) | RED exit 101: `new_widget_actions_activate_nothing_until_their_owner_builds_them`, `widgets/tests.rs:1771`, "`Input { name: "leave_match" }` must queue no input". |

Plants: `mp1_plants.py --files <the 17 files of the 27 records> --native-only` gave `27 plant(s) in 8 command group(s), 27 red-restored, 0 failed` (exit 0, source unchanged).

## 6. Merge check

`git merge-tree --write-tree --name-only f372c77 0816a87`: exit 0, tree `20000c6`, no conflicted file. The merge base is 9044820. Integration adds only the E-GPU commits, and no file is changed on both sides (`comm -12` of the two `--name-only` lists is empty). A build of the merged tree is NOT TRACED.

## What I ran (runner `exec_in_lane_v4.py`, `--head 0816a87`; logs in `/home/julien/Projects/TuRTS/target/mc-evidence/<label>/`)

| Label or command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `l0core-rerev-int-green` | interaction tests green | a failure | 123 passed |
| `l0core-rerev-client-green` | client tests green, the scene oracle included | a failure | 457 passed, 4 ignored |
| `l0core-rerev-checks` | `turts_checks` green | a failure | exit 0 |
| `l0core-rerev-wasm` | wasm32 check | an error | exit 0 |
| `l0core-rerev-boundary` | T1 edges | a failure | "boundary ok" |
| `l0core-rerev-clippy` (2 crates, `-D warnings`) | clean | a warning | exit 0 |
| `l0core-rerev-proof` | `run_proof.sh` claims (errata 15) | DIFFERENT, or self-test not OK | EQUAL 2261, 57 arguments, 12 of 12 red, move 299 EQUAL, no `taking_input` set, 0 tests removed |
| R3 scratch `rr_mut.py` | the proof sees my 13 mutations | EQUAL | 13 DIFFERENT |
| R6 scratch `l0-split/stmt_equal.py` without the repair lines | move A cover kept | DIFFERENT | A EQUAL; B tokens equal, NOT-CALLED only |
| `l0core-rerev-fault-default-taking` | F5 | green | red, 92 |
| `l0core-rerev-fault-leave-guard` | F6 | green | red, 1 |
| `l0core-rerev-green-after-faults` | exact restore | red | exit 0 |
| `l0core-rerev-plants-all` | 27 records red and restored | a failed plant | 27 of 27 red-restored |
| `cmp` of the fixture with 9044820 | fixture byte-equal | exit 1 | exit 0 |
| `git diff --numstat 9044820 0816a87` of the test files | no deletion | a number above 0 in column 2 | all 0 |
| `git merge-tree` f372c77 0816a87 | clean merge | a conflict | exit 0 |

## NOT TRACED

- A build of the merged tree.
- The pixel path and headed runs.
- A pack of a package that writes `leave_match` (I read the refusal code; I did not run a pack).
- The frame cost of a filled `CoreFeed` (the owner's).
- Which alert triggers have no unit (L3).
- That `On<Activate>` fires on a right click (open point 16).

## What I changed

The repository: none (every fault was restored, `cmp`=0, the status is empty at 0816a87). Files: this report and scratch files under the session scratchpad.
