VERDICT=FAIL HIGH=1 FINDINGS=7 RUNS=23 NOT_CHECKED=5

# Review of pre-MP1 slice task A2a (one step function, bit-equal)

Reviewer: qa-lead. Date: 2026-11-12. Candidate: `ab37cea` on `pre-mp1/a2a-step` (3 commits on `8722f8e`).
Lane: `/home/julien/Projects/TuRTS-pre-a2a-review`, cache `target/mc-pre-a2a-review`. Run receipts:
`/home/julien/Projects/TuRTS/target/mc-evidence/a2a-rev-*/`. Scratch (scripts, traces, probe):
`/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/a2a-review/`.

Summary. The merged step is equal to the old step on every case that I could run. That includes a case that the harness
never reaches. The instrument has one High gap: the late-arrival branch (`frame.rs:798-804`) runs 0 times in
5,683,800 unit frames. A fault there that changes the output stays green on `kernel_reference`. My two-unit probe
reaches the branch and goes red. The repair changes only the test.

## Findings

1. **High.** `crates/turts_sim/src/movement/frame.rs:798-804` (late arrival). The 64-seed release run reaches this
   branch 0 times, pooled and no-pool (counters 37, 38, 39 and 56 = 0). Fault F4 deletes the `Resting` write at
   `:800-802`: `kernel_reference` is GREEN (5,683,800 unit frames), and my probe is RED (`progress` is
   `Some(.. activity: Moving ..)` against `None`). Plant (h) of the builder behaves the same way: the harness is green
   and the probe is red (`sep=true` against `false`). The branch is reachable with two units: a mover (radius 0.5,
   speed 4, at (0.5, 0.9), target (0.5, 0.5)) and a static unit (radius 2, at (0.5, 3.0)). The projection pushes the
   mover past the corner, and the clamp puts it exactly on the target. On the clean candidate, both kernels give
   (0x3f000000, 0x3f000000), target None, and equal flags. Probe:
   `scratchpad/a2a-review/a2a_review_probe.rs`. Fix: add this frame (pooled and no-pool) to `start_frame`, and add a
   late-arrival count (pooled > 0, no-pool > 0) to the `Seen` asserts. Then show F4 and (h) red.
2. **Medium.** `frame.rs:764-770` (the direction when two units are on the same point). The branch is reached 0 times
   (counters 35 and 36). Fault F5 (`<` to `>`) is GREEN. The branch is reachable when two units share one position
   exactly. The text is the same as in both old branches, so the A2a risk is low. A4 and A3 reuse the harness (R9).
   Fix: add one frame with two movers of different groups on the same point.
3. **Medium.** The release form of the harness runs in no gate. `cargo test -p turts_sim --locked` (the F-RUST form,
   plan `:329`) runs **0 tests** of `kernel_reference`, and also 0 of `probe_flags`, because the feature is off.
   The CI form (`cargo test --workspace --locked`, `ci.yml:35`) turns the feature on through `turts_pack`
   (`crates/turts_pack/Cargo.toml:19`) and runs the debug form: 16 seeds, at most 30 movers, 145,200 unit frames,
   9.19 s. The seeds with 800 to 5,000 movers run only by hand. The debug form caught F3 (seed 14). Fix: put
   `cargo test --release -p turts_sim --locked --features test-probes --test kernel_reference` in the gate list of
   A2b1, A2b2, A4 and A3 and in REG (or in slow gate G-RUST) until A3 merges. Add `--features test-probes` to the
   F-RUST line for `turts_sim`.
4. **Low.** `frame.rs:650`: the `distance <= 0.025` clause never decides (counter 59 = 0). The harness speeds are 1 to
   6 (`kernel_reference.rs:169`, `:340`), so `step_limit` is 0.05 or more. A unit with a speed below 0.5 at 20 Hz
   reaches the clause. Fix: add one slow mover.
5. **Low.** `frame.rs:921`: the early arrival of a no-pool unit now calls `update_target_claims`. The old no-pool early
   arrival did not (old `frame.rs:620-642`). `target_claims` is read only at `frame.rs:390-392`, inside
   `retained_escape` together with `is_free(target)` (`:410-416`). The arrived unit is on that target, and its radius
   is above 0 (`turts_mod_runtime/src/content.rs:167`), so `is_free` is false and the output does not change. The
   harness cannot see this state. Also, with NaN positions only: the old `all(>= ..)` (old `:612-619`) against the new
   `!any(< ..)` (`:635-643`). Fix: a comment at `:921`.
6. **Low (forward).** `reference.rs:21-31`: the copy calls the live helpers (`step`, `make_plain_progress`,
   `step_plain_progress`, `pool_index`, `search`, `spread`). At A2a no helper file changed (`git diff --stat 8722f8e
   ab37cea`: only `frame.rs`, `mod.rs` (+2), `probe.rs` (+32), `reference.rs` and two new tests). If a later task (A2b1,
   A4 or A3) edits a helper, the harness compares new code with new code. Fix: each later task asserts that
   `git diff <base> -- <helper files>` is empty, or copies the helper.
7. **Low.** Plan `:80` logs the `probe.rs` sha256 `aaa45aaa…`. At `ab37cea` it is
   `a0e861062cb9bebf7528dad354da1d5ef00bc108c05ec3e27dc7beb1b60a0295`. No checker pins it: the classifier hashes only
   the emitter files (`tests/m3/refreeze_classify.py:585-616`; `tests/m3/README.md:159-162`), and
   `refreeze_allowed.json` holds no hash. Log the new value on the board at the A2a merge. A2c has nothing to update.

## 1. Copy proof

`git show 8722f8e:…/frame.rs` lines 61 to 1104 compared with `git show ab37cea:…/reference.rs` lines 33 to 1077
(`rtk proxy diff`, exit 1). There are exactly two hunks: the doc comment and the name (`1,3c1,3`), and `665a666`
`..Default::default()`. The header (`reference.rs:11-12`) states the same. So the reference is a true copy. The helpers
that it calls did not change in A2a (finding 6).

## 2. Reach table (64-seed release run, counters planted in a copy; `a2a-rev-cov-release-1`)

| # | Branch in `frame.rs` | Count | A plant can change the output? |
| --- | --- | --- | --- |
| 0/1 | pooled / no-pool entries (`:563`) | 1,485,276 / 831,783 | reached |
| 2 | no-pool target occupied, plain fallback (`:580`) | 5,569 | reached |
| 3/4 | early arrival pooled / no-pool (`:650`) | 824 / 623 | reached |
| 5 | pooled, in reach but occupied | 16,021 | reached |
| 6 | no-pool, in reach but occupied | 0 | no: the entry check `:570-580` takes the same case |
| 7/8 | velocity pooled / no-pool (`:664-668`) | 1,484,452 / 825,591 | reached |
| 9 | no-pool velocity ZERO (`:669`) | 0 | no: a distance <= 1e-8 arrives (case 6 is empty) |
| 10, 11, 12 | near_same_group; same-group skip; pass-mode skip | 11,783; 215,874; 20,619 | reached |
| 13/14 | push pooled / no-pool (`:701-705`) | 140,289 / 5,399 | reached |
| 15/16 | side step pooled / no-pool | 92,510 / 3,432 | reached |
| 17 | same-group push | 7,874 | reached |
| 19/20 | `step()` path pooled / no-pool | 1,350,348 / 820,495 | reached |
| 21/22 | pooled cap on / off | 54,949 / 79,155 | reached |
| 23/24 | no-pool cap on / off | 2,717 / 4,515 | reached |
| 25-33 | projection, iterations that moved: 0..8 | 2,214,549; 46,911; 2,387; 514; 166; 143; 86; 45; 45,242 (8 of 8) | reached |
| 34 | projection same-group skip | 567,012 | reached |
| 35/36 | same-point direction (`:764-770`) | 0 / 0 | **yes** (F5 green; finding 2) |
| 37/38 | late arrival pooled / no-pool (`:798`) | 0 / 0 | **yes** (F4 green; finding 1) |
| 39/56 | late arrival with `separated` pooled / no-pool | 0 / 0 | **yes**, flag only (plant (h)) |
| 40 | on the target but occupied | 5 | reached |
| 41/42 | made progress yes / no | 1,451,194 / 33,258 | reached |
| 43, 44, 45, 46, 47 | deadline; pass mode; spare slot; no spare; claim limit | 1,193; 103; 688; 311; 91 | reached |
| 48, 49, 50 | no-pool fallback; better; `stalled_ticks = 1` | 7,391; 6,824; 567 | reached |
| 51 | no-pool, no fallback | 818,200 | reached |
| 52 | `Blocked` with overlap gives an escape | 211 | reached |
| 53/54 | the clamp moves pooled / no-pool | 24 / 211 | reached |
| 57/58 | pooled entry Resting / pass mode | 248,554 / 189,350 | reached |
| 59 | only the 0.025 clause decides | 0 | yes, for speed < 0.5 (finding 4) |

Plant script: `scratchpad/a2a-review/plant_cov.py`. Plant (h) verdict: the builder's reason ("not reached") is
correct for the harness. It is not a property of the code: two units reach the branch, and the probe goes red.

## 3. What the harness compares

`kernel_reference.rs:413-469` compares every field of `MovementFrameOutput` (`frame.rs:43-54`) for every unit on every
tick: `unit_id`, `position` (bits), `target` (bits), all 9 fields of `MoveProgress` (`mod.rs:57-68`, f32 as bits),
`plain_progress` through derived `Debug` (`plain.rs:14-26`, all 9 fields), `physical_escape` (bits),
`failed_blocked`, `probe.separated` and `probe.was_projected`. It does not compare `probe.phases`, which the reference
does not set. `Debug` of an f32 prints `-0.0` apart from `0.0`. It hides only a difference in the NaN payload.
`plain_progress` covers no position: the positions are compared as bits. Each tick, both kernels take the same input
frame. A difference stops the run.

## 5. My faults (release, `kernel_reference` + probe; restore by `git checkout`, then `cmp` with `git show ab37cea:` = 0)

| Fault | `kernel_reference` | Probe | First red |
| --- | --- | --- | --- |
| F1: push switch inverted (`:701`) | RED | green | seed 4 tick 295 unit 9, position y 1107607586 vs 1107607587 |
| F2: `for _ in 0..7` (`:743`) | RED | green | seed 8 tick 260 unit 20 |
| F3: `velocity = velocity * step_limit / velocity_len` (`:727`) | RED (release and debug CI form) | green | seed 10 tick 97 unit 8; debug: seed 14 tick 3 unit 36 |
| F4: late arrival leaves out `Resting` (`:800-802`) | **GREEN** | RED | probe: progress Some(Moving) vs None |
| F5: `unit_id >` (`:766`) | **GREEN** | green | none |
| (h): `probe.separated = separated` (`:792`) | **GREEN** | RED | probe: sep=true vs false |

Fault script: `scratchpad/a2a-review/fault.py`. Fresh green after the restores: the clean runs
`a2a-rev-ws-debug-kref`, `a2a-rev-probe-neutral`, `a2a-rev-group-gmu` (each exit 0). `git status` is empty at the end.

## 6. Equal values

- Traces at `ab37cea`: base `d4c73ebe…7b2`, haul `805309b2…148`, acme `19dcf9a3…1d2`, cedar `0db10e01…718`, stream
  `bcbf72c2…70b`. All equal the stated values.
- Traces emitted with the movement files of `8722f8e` (`frame.rs`, `mod.rs`, `probe.rs` from the base): `cmp` exit 0
  for all four traces and the stream.
- `tests/m3/m2_operation_stream.json` sha256 `bcbf72c2…70b`; F-FROZEN tests green (26 + 1 + 1).
- `probe_neutral.sh`: `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`, feature off equals on.
- `refreeze_allowed.json` `a431dbd8…bc7` and the emitter `m2_trace.rs` `e5152ece…24` are unchanged. `probe.rs` changed
  (finding 7). No pin needs an update now or at A2c.

## 7. Tick time (numbers only; the host load average was 19 to 21 on 28 threads, i9-14900K)

`mc_cost.rs` (ignored; pooled group move; whole `app.update()`, 20 samples), in the order base, candidate, base,
candidate:

| Run | 1,000 median / max ms | 5,000 median / max ms |
| --- | --- | --- |
| base-1 (`8722f8e` movement files) | 3.011 / 5.105 | 35.410 / 43.314 |
| cand-1 (`ab37cea`) | 3.691 / 3.781 | 20.825 / 74.909 |
| base-2 | 2.947 / 3.769 | 28.798 / 34.458 |
| cand-2 | 3.030 / 3.420 | 18.792 / 19.820 |

Same binary, kernel only (timing plant in `kernel_reference.rs`, live and reference in alternating order, 300 ticks
each; sum of ns live / reference): seed 61 (800 movers) 1.0947 / 1.1016 s; seed 62 (1,500) 1.2285 / 1.2277 s;
seed 63 (2,500) 0.9220 / 0.9301 s; seed 64 (5,000) 7.5476 / 7.5056 s (median per tick 25.15 / 25.41 ms, p99
45.26 / 42.68 ms). The `mc_cost` provenance line prints `commit=ab37cea` also for the base runs, because the tree was
dirty (`mc_cost.rs:487-503`).

## Commands (each through `exec_in_lane_v4.py`, label as given)

| Label | Command | Claim tested | Refuting result | Observed |
| --- | --- | --- | --- | --- |
| (git) | `rtk proxy diff old_fn.rs ref_fn.rs` | the copy is the old function (`reference.rs:11-12`) | a hunk other than the name and `..Default` | 2 hunks only |
| kref-release-green-1 | release `kernel_reference`, `one_step_function` | 0 differences on 64 seeds (builder) | a panic | 5,683,800 frames, ok; 3 ok |
| cov-release-1 | the same with counters | every branch reached | a count of 0 | table above |
| probe-late-clean-1 | probe | late arrival equal | live differs | equal, pooled and no-pool |
| f1, f2, f3, f4(-nff), f5, h | faults | the harness is red on each fault | green | see section 5 |
| frust-sim-nofeature | `cargo test -p turts_sim --test kernel_reference` | F-RUST runs the harness | 1 test | 0 tests |
| ws-debug-kref | `cargo test --workspace --test kernel_reference` | CI runs the harness | 0 tests | 1 test, 145,200 frames |
| f3-cap-order-debugws | F3, CI form | the debug form is red | green | red, seed 14 |
| trace-emit, trace-emit-base | `m2_trace` | traces equal | a `cmp` difference | equal, 5 files |
| mccost-base-1/2, cand-1/2; kernel-ab-timing-1 | timing | numbers only | n/a | section 7 |
| probe-neutral | `probe_neutral.sh` | values equal (builder) | other values | equal |
| ffrozen | F-FROZEN tests | stream frozen | red | green |
| group-gmu | `group_movement`, `gmu_same_point`, `probe_flags` | no red suite test | red | 38 + 1 ok; `probe_flags` 0 tests |

## NOT TRACED

- R2 condition: `cargo test -p turts_audit --locked` and EL1. I did not run them.
- `mc_group` (release, ignored). I did not run it.
- The builder's plants (a) to (f). I did not run them again. I ran only F1 to F5 and (h).
- `cargo fmt`, `cargo clippy`, `scripts/boundary.sh`, and `refreeze_classify.py --p0`. I compared the trace bytes
  with `cmp` in its place.
- The red of the builder's step 1 counters at the base (D 9.5).

What I changed: none in the repository. This file is the one repository-adjacent file (not committed). Memory:
`.claude/agent-memory/qa-lead/pre-mp1-a2a-review-2026-11-12.md`.
