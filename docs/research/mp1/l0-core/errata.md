# L0-CORE: errata of the plan and the design (first pass and repair)

Source: the build of task `L0-CORE` and the repair after its review
(`docs/research/mp1/review-l0-core.md`). Each entry says what the plan or the design says, what
the code needs, and what the branch does. For the orchestrator to apply in revision 6.3.

## The 7 errata of the first pass

1. **Plan task `L0-CORE`, step 2 and plant (b): "a stub that returns a value for a 0.2.3 layout:
   the scene oracle is red".** This is wrong. The scene oracle covers `draw_world_visuals` only.
   No widget stub is on its path, and a 0.2.3 layout cannot name a 0.2.4 variant. Run: a planted
   widget stub left `visuals::scene_tests` green (label `l0core-scene-under-widget-plant`).
   The plants of the branch use the stub contract tests instead. Change the plan text.
2. **Plan plant (a): "swap two input-kind arms".** Swapping `match` arms of distinct variants changes
   nothing. The plant swaps the handlers that two arms call (`PointerDown` and `PointerUp`).
3. **Plan 3.4 lists four activation files.** `FocusAlert` had no owner file, so `activation/alert.rs`
   was added. See entry 11 for the other files.
4. **Plan task text: "one stub function for each new `ItemField` variant".** None was made. The new
   variants appear only in `item_field_key` (a key table). The owner's list source writes the field
   with `ItemData::set`. Each stub file names the fields its owner fills.
5. **`Behaviour::*` and `Capability::*` arms (`sources.rs capability_matches`, `values.rs`)** are
   not in the plan list of stubs. They stay as `L0-API1` made them.
6. **The stub tests flip when an owner builds its variant.** Each owner replaces its own test. This is
   an inversion in the sense of SP 1a.
7. **The step proof of the first pass dropped the hook calls wherever they stood, dropped the call
   arguments and turned every `return` into `continue`.** Repaired (entry 15).

## New errata of the repair

8. **Plan 5.1 item 5 freezes `FocusAlert` with no field (review H1).** An alert row carries the same
   action value, so the activation cannot find the place or the unit (design 7.9, D:2059-2061).
   Verified: the mod schema has no `FocusAlert` (`rtk proxy grep -rn FocusAlert crates` finds only
   client code); a mod writes `action: Item(Action)` (design 6.10 note [9]), and `WidgetAction` is a
   client type. So the client action changes and the mod API does not. Now
   `WidgetAction::FocusAlert { x: f32, y: f32, unit: Option<u32> }`, and the core input
   `alerts::Input { x, y, unit }` has the same fields (D74: an agent drives it headless). Change
   5.1 item 5 to read this shape.
9. **Plan 3.4 and the stub text said that no owner adds a call line to `step` (review M1).** A click
   was already dispatched when `placement::update` ran (the review probe: 27 of 149 hook calls with a
   `PointerUp` already held a `Send`). Now `placement::intercept(...) -> bool` runs at the top of the
   event loop and a `true` skips the event (design 7.2: "a right click leaves placement and never
   gives an order"; GHO1 (d), D:2470). Its body is `core.placement.taking_input`; no code sets the
   field, so it returns `false` until `UI-PLACE`. The post-loop `update` hooks stay.
10. **Each owner edits only its own file (review M2).** Each owner has an empty type in its own file,
    wired one time. The table lists each shared edit point.

    | Shared edit point | Wired now | Does an owner still edit it? |
    | --- | --- | --- |
    | `Core` fields and `Core::new` | `placement::State`, `hints::State`, `alerts::State` | No. The owner adds fields to its own `State`. |
    | `InputEvent` and the match in `step` | `Placement(placement::Input)`, `Hint(hints::Input)`, `Alert(alerts::Input)`, each arm calls `<owner>::on_input` | No. An owner models its input kinds inside its own `Input`. A task with a new owner needs one variant and one arm. |
    | `Effect` and the match in `drive_core_adapter` | `Placement(placement::Output)` -> `effects/placement.rs`, `Hint(hints::Output)` -> `effects/hint.rs` | No for `UI-PLACE` and `UI-HINT`. `UI-ALERT` has no effect type: `FocusAlert` gives `CameraChanged` and `SelectionChanged`. `UI-RESULT` has none (entry 13). |
    | `WorldView` and `FrameWorld` | `placement_facts()`, `hint_facts()` with defaults; `FrameWorld` fields; builders `core_adapter/facts/placement.rs` and `hint.rs` | No for placement and hints. `UI-ALERT` and `UI-RESULT` read the replica (`feedback.rs`, the store getters), not the world view. |
    | `SourceContext` (22 struct literals) | field `core: &CoreFeed`; parts `place::Feed`, `hint::Feed`; `from_core` calls `place::feed` and `hint::feed` | No for placement and hints. `UI-HUD`, `UI-RESULT`, `UI2-*` read `replica` and the store getters that `SourceContext` already has. A task that needs core state of a new kind adds a part: one field and one literal. |
    | `VisualInputs` and `draw_world_visuals` | `placement::Feed` resource, `effects::Pool` resource, `register(app)` for each (called in `VisualsPlugin`), `draw(..)` calls after the last present concern and before `update_health_fills` | No for `UI-PLACE` and `L-FX-PRES`. `PRES-MODELS` and `FX2` (stage 2, `render.rs`) are not wired here. |

    Cost note: `CoreFeed::from_core` runs at the start of each `sync_widget_tree`, so an owner's
    `feed` must not clone lists (the client cost rule, design section 9).
11. **Plan 3.4 lists `activation/cancel.rs`, `leave.rs`, `rally.rs` as stubs (review L1).** They were
    dead code. Now: `cancel.rs` holds the arm `CancelProduction`, moved with its body unchanged
    (`docs/research/mp1/l0-core/move_equal.py`: 299 tokens equal), so `UI-HUD` extends a live
    function; `leave.rs` is called from `activate_widget` for `WidgetAction::Input { name: "leave_match" }`
    (a 0.2.3 layout cannot contain it: `crates/turts_mod_runtime/src/version_table.rs:446` has the row "engine action leave_match" as a 0.2.4 feature; I did not run a pack of such a package);
    `rally.rs` is deleted: a rally is an order of the smart path, not an activation. `UI2-HUD`
    creates its own file if it needs one.
12. **Plan 3.4 and 5.1 item 5 say "one stub function for each new effect family".** Only two families
    have a producer in the core (design 7.5, D:1916: placement, `Ghost`, the hint state, `FocusAlert`).
13. **`apply_alert` and `apply_result` are removed (review L2).** A match result comes from
    `MatchResult` through the store getters, not from `Core`; a camera move and a selection are
    `CameraChanged` and `SelectionChanged`. The plan lines for `UI-ALERT` and `UI-RESULT` that name
    "the alert family function" and "the result family function" in `core_adapter/effects.rs` need
    to be struck.
14. **Alert trigger stubs return `Vec<u32>` (review L3): not changed.** An alert with no unit and no
    `focus` needs a change in the tail of `collect_alert_events` (the `units.retain` that drops units
    of other slots, the `units.is_empty()` exit and the unit `0` case). That is more than 15 lines
    and it is behaviour. `UI-ALERT` owns that tail (evaluation per applied delta). Design 7.9 (D:2059-2061) says the place
    of an alert with no event unit is its `focus`, so a trigger without a unit is a design case; which
    triggers have none is NOT TRACED. The owner widens the return type when it builds them. No
    frozen shape depends on it.
15. **The purity proof.** `step_equal.py` now compares each argument with its parameter name, maps
    `return` to `continue` only at loop depth 0, checks the hooks in place (the intercept as the first
    statement of the loop, the three `update` hooks right after the `prune` that follows the loop and
    before the `let`, the three arms) and has a self-test of 12 mutations, among them the five of the
    review (inner `continue` to `return`, an argument to `0`, `&mut Vec::new()` at a call, `now_ms` to
    `0`, the hook calls moved).
16. **`docs/research/mp1/l0-split/run_proof.sh` (the proofs of `L0-SPLIT`) no longer applies verbatim.**
    - Move A: `draw_world_visuals` has two more calls (`placement::draw`, `effects::draw`) and two more
      fields in its destructure. At 0816a87 the script prints DIFFERENT at token 32 (the destructure).
      The re-review (run R6) removed those 4 lines and got EQUAL, so the cover of move A is not lost.
    - Move B: at 0816a87 the script prints DIFFERENT at token 2724 (the new arms `Effect::Placement`
      and `Effect::Hint`; old 2951 tokens, joined 2991) and `NOT-CALLED: placement_facts`,
      `NOT-CALLED: hint_facts` (the two new `WorldView` methods that the script takes for concern
      functions of the move, because they are new functions of `core_adapter.rs`). The re-review
      (R6) removed the repair lines and got 2951 = 2951 tokens, and the only complaint left was the
      NOT-CALLED line (I did not repeat that removal run).
    - No gate runs either script (the `agents` job and the roster check do not). They are records of
      the first pass. The behaviour oracle is the scene oracle (byte-equal and green) and the
      adapter tests. A planted `draw` that adds a visual turns the scene oracle red (plant records
      `L0-CORE/v-*`). Do not re-run these two scripts on a later head; re-pin them or drop them.
17. **Typo "The the"** in the stub text of `hints.rs` and `alerts.rs`: gone with the rewrite.
18. **`visuals/scene_tests.rs` setup** has two more `init_resource` lines (`placement::Feed`,
    `effects::Pool`), because `VisualInputs` has two more fields. No expected value changed.

## Follow-up (after the re-check `rereview-l0-core.md`)

19. **The fact builders are stateless and ran every frame (M-R1).** `frame_world` called
    `facts::placement::build(replica, content)` and `facts::hint::build(replica, content)` and each
    returned a new `Facts`. The plan asks for one rebuild for each applied delta (plan:977, :990),
    which needs a cache. Now `build(&mut Facts, &Replica, &ClientContent)` updates the facts that the
    last frame left. The facts of both owners live in `OwnerFacts` (`core_adapter/facts.rs`), kept in
    `CoreRuntime::facts`: `drive_core_adapter` takes them before `frame_world` and puts them back at
    its end. So an owner keeps its cache (for example the replica tick of its last rebuild) in its own
    `Facts` type, in `turts_interaction/src/<owner>.rs`, and edits its own `facts/<owner>.rs`; it does
    not edit `frame_world`. An early return of `drive_core_adapter` after `frame_world` drops the
    facts, so the next frame starts from the default (a cold cache, not a wrong one). Test:
    `core_adapter::adapter_tests::owner_facts_are_kept_from_frame_to_frame`.
20. **The text of `taking_input` said "takes every input" (L-R1).** Read literally, it swallows
    `KeyUp`, `FocusLost`, `PointerLeft`, `UiCapture` and `InputEvent::Placement` too, which leaves held
    keys and buttons set and stops `on_input`. The text now lists what `intercept` may take
    (`PointerDown` and `PointerUp` of the left and right button, `KeyDown` of the cancel key) and
    what it must not take, and says that the camera still sees every event (`camera_controller.step`
    reads `events`, not the loop), except the wheel events that `on_wheel` marks as ignored. Design
    7.2 and GHO1 (d) need no change.
21. **`run_proof.sh` was blind to `taking_input: true` in a literal or in a manual `Default` (L-R3).**
    `taking_input.sh` now allows only the field declaration, the body of `intercept` and comments, and
    refuses every other line that names the field. The dynamic case is owned by the test
    `hook_tests::intercept_stub_takes_no_event_in_a_fresh_core`. Plant records `L0-CORE/t-*`.
22. **`leave::activate` has no way to send `LeaveMatch` (re-check L-R2).** `UI-RESULT` adds one
    parameter to `activate_widget`. No concurrent owner edits that header. Accepted by the re-check.
