RESULT=PASS

# Follow-up of L0-CORE (re-check `rereview-l0-core.md`: M-R1, L-R1, L-R3, L-R4)

Branch `mp1/l0-core-fu`, from 0816a87, in `/home/julien/Projects/TuRTS-mp1-l0-core`. Pushed to origin at a3caf8b. Not merged.
Commits:
- 2b22856: the code, the scripts and the errata.
- 85c6843: the plant records.
- a3caf8b: the red text of one plant record.

Evidence logs: `/home/julien/Projects/TuRTS/target/mc-evidence/<label>/command.log`.
Scratch: the scripts of this follow-up are in the repository (`docs/research/mp1/l0-core/taking_input.sh`); I used no other script.

## Per item

**M-R1 facts builders without state: done.**
- I verified first: `frame_world` ran in each `drive_core_adapter` call and called `facts::placement::build(replica, content)` and `facts::hint::build(replica, content)`, each returning a new `Facts` (`core_adapter.rs:567-568`, call at `:773` in 0816a87).
- Now `build(&mut Facts, &Replica, &ClientContent)` updates the facts of the last frame. Both facts live in `OwnerFacts` (`core_adapter/facts.rs`), kept in `CoreRuntime::facts`.
- `drive_core_adapter` takes them (`std::mem::take`) before `frame_world` and puts them back at its end (`runtime.facts = world.facts`). `FrameWorld` returns references to them.
- An owner keeps its cache (for example the replica tick of its last rebuild) in its own `Facts` type (`turts_interaction/src/<owner>.rs`) and edits only `core_adapter/facts/<owner>.rs`. It edits no `frame_world` line.
- An early return after `frame_world` drops the facts, so the next frame starts cold (a cold cache, not a wrong one). This is in errata 19.
- Stub behaviour is unchanged: both `build` bodies are empty. The scene oracle is byte-equal (`cmp` 9044820, exit 0).
- Test: `core_adapter::adapter_tests::owner_facts_are_kept_from_frame_to_frame`. A `cfg(test)` counter in `OwnerFacts` shows that the same value reaches `build` in each frame.
- Red first: plant `f-owner-facts-are-new-for-each-frame` (`take` replaced by `Default::default()`: the stateless first form). Red text "the owner facts must be kept: each frame builds the same value once" (`l0core-fu-plants-f`, red and restored). My first plant (do not restore the facts) failed with another assert text, and I replaced it by this one.
- Why it can fail: with the stateless form the counter stays at 1.

**L-R1 `taking_input` text: done.** `turts_interaction/src/placement.rs` (header and the field doc).
- It lists what `intercept` may take: `PointerDown` and `PointerUp` of the left and the right button, and `KeyDown` of the cancel key.
- It lists what it must not take: `KeyUp`, `FocusLost`, `PointerLeft`, `PointerMove`, `UiCapture`, `Wheel`, `Minimap` and `InputEvent::Placement`. A skipped handler does not clear the held keys and buttons or run `on_input`.
- When it takes a `PointerUp`, it clears `pointer_down` and the held button itself.
- The camera still sees every event: `camera_controller.step` reads `events` (`lib.rs:287`), except the wheel events that `on_wheel` marks as ignored (so an intercepted `Wheel` would reach the camera even with Ctrl held).
- Errata 20.

**L-R3 `taking_input` proof: done (script), and the test owns the dynamic case.**
- `docs/research/mp1/l0-core/taking_input.sh [REV]` allows only three kinds of line that name `taking_input`: the declaration `pub taking_input: bool,`, the body `core.placement.taking_input`, and comments. Any other line is `TAKING-INPUT-USED` (exit 1). This covers `taking_input: true` in a literal or in a manual `Default`, and any write. `run_proof.sh` calls it.
- Plant `t-taking-input-default-true-is-refused-by-the-script`: red, exit 1.
- The dynamic case is `hook_tests::intercept_stub_takes_no_event_in_a_fresh_core`; the script header names it. Plant `t-taking-input-default-true-is-red-in-the-test`: red (the group runs the 2 `hook_tests`; the red text is the setup assert; `intercept_stub_takes_no_event_in_a_fresh_core` also fails in that run).

**L-R4 errata 16: done.** `errata.md` entry 16 is complete, with these figures from my run of `l0-split/run_proof.sh 149f1ed 0816a87`:
- Move A prints DIFFERENT at token 32 (the destructure of `VisualInputs`); the re-check run R6 got EQUAL without the 4 repair lines.
- Move B: old 2951 tokens, joined 2991, DIFFERENT at token 2724 (the new arms `Effect::Placement` and `Effect::Hint`), plus `NOT-CALLED: placement_facts` and `NOT-CALLED: hint_facts` (the new `WorldView` methods, taken for concern functions of the move).
- The re-check run R6 got 2951 = 2951 without the repair lines. I did not repeat that removal run.
- No gate runs either script. Entries 19 to 22 are new: M-R1, L-R1, L-R3, and L-R2 (accepted by the re-check, no change).

## Finding of this follow-up (not in the re-check)

Two plant records of `L0-SPLIT` stopped applying at the head 0816a87:
- `L0-SPLIT/EFFECTS/effect-shell-action-call-dropped` (`core_adapter.rs`, hunk context changed by my `Effect::Placement` and `Effect::Hint` arms);
- `L0-SPLIT/SCENE/health-fill-call-dropped` (`visuals.rs`, hunk context changed by my two `draw` calls).

The re-check ran the 27 records of `l0-core.json` only; it did not see them. I found them with `mp1_plants.py --files <every .rs file changed since 9044820>` (`l0core-fu-plants-changed-files`: `REFUSED ... re-pin the record`). I re-pinned both (patch context, `commit` set to 2b22856) and ran them red and restored (`l0core-fu-plants-changed-files-2`, `l0core-fu-plants-f`). Both are in 85c6843.

Effect: `mp1/integration` already has 0816a87, so its slow gate (`--all`) refuses those two records until this branch merges.

## No change of behaviour
- `visuals/scene_oracle.txt`: `cmp` against 9044820, exit 0.
- Test files: `git diff --numstat 0816a87 HEAD` shows one test file changed, `core_adapter/adapter_tests.rs`, 25 insertions and 0 deletions. No expected value changed.
- Static test-name check: 1849 at base, 1871 at head, 0 removed.

## Gates on the clean head 85c6843 (code identical to a3caf8b; the last commit changes a JSON record only)
All exit 0, wrapper exit 0.

| Gate | Label | Result |
| --- | --- | --- |
| fmt check | `l0core-fu-gate-fmt` | ok |
| clippy workspace, all targets, `-D warnings` | `l0core-fu-gate-clippy` | ok |
| `cargo test -p turts_client` | `l0core-fu-gate-client` | 458 passed, 4 ignored |
| `cargo test -p turts_interaction` | `l0core-fu-gate-interaction` | 123 passed |
| `cargo test -p turts_checks` | `l0core-fu-gate-checks` | ok |
| `cargo test --workspace` | `l0core-fu-gate-workspace` | 1899 passed, 36 ignored, 0 failed |
| wasm32 check | `l0core-fu-gate-wasm` | ok |
| `scripts/boundary.sh` | `l0core-fu-gate-boundary` | ok |
| `driver_rule.py tests/browser` | `l0core-fu-gate-driver` | ok |
| `mp1_plants.py --self-test` | `l0core-fu-gate-plantself` | ok |
| `run_proof.sh 9044820 HEAD` | shell, exit 0 | step proof EQUAL, self-test OK (12 mutations), move EQUAL, `taking_input.sh` OK, 0 tests removed |

Plants: 30 records in `l0-core.json` plus the `L0-SPLIT` records on the same files: `l0core-fu-plants-all`, 44 plants, 43 red-restored, 1 failed. The failure was the red text of `t-taking-input-default-true-is-red-in-the-test` (a test name is also in the green output). I changed the text and ran the 6 records of `placement.rs` again: `l0core-fu-plants-t`, 6 of 6 red-restored (a3caf8b).

## NOT TRACED
- The removal run of the re-check R6 for move B (I quote its figure).
- A build of the merged tree, any browser or headed run.
- The frame cost of a filled `OwnerFacts` (empty now).
- Records of other tasks on files that this branch changed: I ran `--files` for every `.rs` file changed since 9044820 (16 plants, all applied or re-pinned); I did not run `--all`.
