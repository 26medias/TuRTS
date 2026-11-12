RESULT=PASS

# Repair of A2a after the review (`review-slice-a2a.md`)

Branch `pre-mp1/a2a-step`, pushed. Head `1bff266bacc2031fe0bc74473c45247a18952f4e` (one repair commit on `ab37cea`, with `[skip ci]` and the Co-Authored-By line). Nothing merged. Evidence receipts: `/home/julien/Projects/TuRTS/target/mc-evidence/pre-a2a-fix-*/command.log`. Scripts: `scratchpad/a2a-fix/` (`run.sh`, `fault.py`).

Files changed (`git diff --stat ab37cea HEAD`):
- `crates/turts_sim/tests/kernel_reference.rs`: anchor frames, new `Seen` counts, helper pins, header.
- `crates/turts_sim/src/movement/reference.rs`: a local copy of `step`, and the header.
- `crates/turts_sim/src/movement/frame.rs`: a 5-line comment only (no code line changed: `git diff` minus comment lines = 0).

## Per item

- **High 1 (late arrival).** `late_arrival_frame(pooled)`: the reviewer's frame, with and without a pool, in `anchors()`. New `Seen` counts `late_arrival_pooled` and `late_arrival_free` (a step with steering run, target `None`, no `failed_blocked`). Release run: 1 and 1, each above 0. Each anchor raises its own count in its own run (`anchor {name} did not reach its branch` otherwise).
  - F4 (`Resting` write at the late arrival removed): RED in release (`pre-a2a-fix-f4-late-resting`: `anchor late arrival, pool tick 0 unit 1: field progress: live pool 1 ... Moving ... reference none`) and RED in the debug CI form (`pre-a2a-fix-f4-debug`).
  - Plant (h) (`probe.separated = separated`): RED (`pre-a2a-fix-h-probesep`: `field probe.separated: live true reference false`).
  - Both restored with `git checkout -- crates/turts_sim/src`, `git status --porcelain` empty, then green (`pre-a2a-fix-kr-release-final`, 0 differences).
- **Medium 2 (same point).** `same_point_frame()`: two lanes, speed 100 (step 5), a static unit at the exact landing point of `step()`; mover id 1 below static id 2 (+X), static id 3 below mover id 4 (-X). `Seen.same_point` = 2. Fault `<` to `>` at the degenerate direction: RED (`pre-a2a-fix-f5-samepoint`: `anchor same point tick 0 unit 1: field position ...`). Restored, green.
- **Low 4 (0.025 clause).** `slow_clause_frame(pooled)`: speed 0.4 (step 0.02), target 0.024 away. `Seen.slow_clause_pooled` and `slow_clause_free` = 1 each. Fault (clause `distance <= 0.025` removed): RED (`pre-a2a-fix-slow-clause`). Restored, green.
- **Low 5 (comment at the claims call).** Added above `update_target_claims(.., new_target)` in `frame.rs`. Verified: the old no-pool early arrival had no such call (`git show 8722f8e:.../frame.rs` lines 620-642); `target_claims` is read only at `frame.rs:391` inside the `Blocked` branch; `retained_escape` there needs `!target_is_blocked_by_stationary` and `is_free(target)` as well, and the arrived unit stands on that target with a radius above 0, so both are false whatever the count is; the map is rebuilt from the unit targets at the start of each frame (`frame.rs:169`). A pool unit already made this call. The NaN-only difference (`all(>=)` against `!any(<)`) is not in the comment.
- **Low 6 (helpers).** I chose the pin, not a copy. Test `reference_helpers_are_unchanged` pins the sha256 of `movement/plain.rs`, `pool_index.rs`, `search.rs`, `spread.rs` and `src/grid.rs`. Reason: those are about 1,300 lines of private types plus `grid.rs`; a pin is one line each and stops the silent case. Its message tells a later task to copy the old text into `reference.rs` first and then update the pin. I also copied the 12-line `step` into `reference.rs`, so an edit of `movement::step` cannot change the reference. Red proof: one added newline in `spread.rs` gives RED (`pre-a2a-fix-helper-pin`: `src/movement/spread.rs changed, and the reference copy calls it`); restored, green. NOT covered: `angle`, `MovementPool` and `MoveProgress` in `movement/mod.rs` (stated in the `reference.rs` header).
- **Medium 3 (debug form and the release command).** The anchors run in both builds (20 ticks each); debug run: 10 s, 2 passed (`pre-a2a-fix-kr-debug-green`), and F4 is red in debug. The header of `kernel_reference.rs` and the doc of the test state the exact command for A2b1, A2b2, A4, A3 and the slice regression, until A3 merges: `cargo test --release -p turts_sim --locked --features test-probes --test kernel_reference`. It also says that `cargo test -p turts_sim --locked` runs 0 tests of this file. The plan text for the F-RUST line and the gate lists of the later tasks is not edited by me (Root's file).

## Release run after the repair

`kernel_reference`: 19,300 frames, 5,684,000 unit frames, 0 differences. Every `Seen` kind above 0: no-progress moving 3,431,673; pooled moving 1,485,046; resting kept 453,314; follow 119,700; Blocked 2,434; escape 4,811; plain 2,681,713; zero-sign arrival 255; late arrival pooled 1 and no pool 1; same point 2; slow clause pooled 1 and no pool 1; phases 2,317,101 / 2,310,082 / 2,310,082 / 1,484,452 / 14,157.

## Equal values

Four trace hashes (`base` d4c73ebe…7b2, `haul` 805309b2…148, `acme` 19dcf9a3…1d2, `cedar` 0db10e01…718): `cmp` equal to the base run. Stream `bcbf72c2…e370b`: equal. `refreeze_classify.py --p0`: `RESULT PASS`. `probe_neutral.sh`: `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`. `probe.rs` sha256 `a0e86106…0295` (unchanged by the repair). `reference.rs` sha256 now `6d032235ccf366f29eb52571bdaa17cf8e9a58c7930a09597516ac9909e639df`.

## Gates (exit codes)

fmt 0; clippy `--workspace --all-targets --locked -D warnings` 0; `cargo test -p turts_sim` without probes 0; with `--features test-probes` 0; release `kernel_reference` and `one_step_function` 0; `cargo test -p turts_pack -p turts_audit -p turts_server -p turts_checks` 0; `cargo test -p turts_audit` 0; wasm32 check 0; `boundary.sh` 0; classifier self-test 0 (74/74 plants red); trace and `--p0` 0; `probe_neutral.sh` 0; `mc_group` release `--ignored` 0 (8 passed); EL1 release 0 (1 passed).

## NOT TRACED

- Review findings 3 (gate lists of later tasks) and 7 (log the `probe.rs` hash at the merge): Root's documents, not changed here.
- Hosted CI after the push (`[skip ci]`).
- A fresh review of this repair (qa-lead).
