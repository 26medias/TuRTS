RESULT=PASS

# A2b1 repair after the stop ruling (pre-MP1 slice)

Seat: `simulation-core-engineer`, build mode. Date: 2026-11-12. Branch `pre-mp1/a2b1-fallback`, base `afb9c6de`,
head `f119dc39` (pushed; not merged). Lane `/home/julien/Projects/TuRTS-pre-a2b1`. Ruling:
`docs/research/mp1/ruling-slice-a2b1-stop.md`. Receipts: `/home/julien/Projects/TuRTS/target/mc-evidence/<label>/`
(labels `pre-a2b1-r-*`). `origin/pre-mp1/integration` is still `afb9c6de` (fetch), so no merge.

Hold (not mine): CEO touch point 1 (P `:401-406`). A2b1 does not merge before the answer.

## Commits

| Commit | Content |
| --- | --- |
| 67face27 | Step 1: item pins of `angle` and `step`; the sweep reaches the escape arm |
| 7b0ccdd5 | The fallback pool of one, the gather protocol, the guards, the 8 R13 observation changes |
| f8d44545, 3e85f88b | Registry rows of the step-1 plants and plants (a) to (c) |
| 91b1e889 | Repair: the five stop items, (b) hex slot test, (c) K1 and K2, (e) row text |
| f119dc39 | Registry rows: hex slot, pool projection constant, pooled chain |

## The five stop items (D 9.21; old and new side by side)

Each item keeps its scenario, its tick count and its messages, except item 4 (an inversion, ruling item 4). No
expected value comes from a run.

| Item | Kind | Old read | New read | Plant (red) |
| --- | --- | --- | --- | --- |
| 1 `plain_movement_tests.rs:187` (test `:146`) | observation | `step_frame(..)`; `output[0].plain_progress.expect(..)`; `progress.pool_slots.is_empty()` | `step_frame_with_fallbacks(..)`; `output[0].progress.filter(is_fallback).expect(..)` (same message); `request pool.slots.len() <= 1` (base `plain.rs:297` takes `skip(1)`) | `primary_count == 0` test made unreachable in `fallback.rs`: red `left: Vec2(0.5, 0.65) right: Vec2(0.5, 0.5)` (`pre-a2b1-r-plant-item1`) |
| 1 `:394` (test `:359`) | observation | `plain_progress.unwrap()`; `slot == (1.0, 1.0)`; `pool_slots.is_empty()` | fallback `progress`; `slot == (1.0, 1.0)` unchanged (= pool point: clamp of (1, 1) to [0.5, 1.0]); request `pool.slots.len() <= 1` | same plant: red `left: Moving right: Blocked` |
| 2 `orders/tests.rs:2692` | observation | injects `PlainMoveProgress{original_point (20,20), slot (8,8), Blocked, stalled 20, claims 3, failure_recorded false}`; reads `PlainMoveProgress.failure_recorded` | injects a fallback `MoveProgress` (same activity, stalled, claims, best distance), id = `next_pool_id + 1` (counter written), `MovementPools` entry point (20, 20), spacing 1.1 (2 x 0.5 + 0.1), primary 1, slots `[(8, 8)]`; reads `fallback.failure_recorded` | `run.rs` ack reads `failure_recorded` inverted: red "prior A is acknowledged" (`-item2`) |
| 3 `frame_hash.rs:211` | observation + repair | `step_frame` (drops requests; pool id 0 carried into later frames); `plain > 0` "no unit reached a plain progress" | `step_frame_with_fallbacks` + `assign_fallback_pools` with a chain counter, pools added to the next frame (as `kernel_reference.rs`); `fallback_pools > 0` "no unit reached a fallback pool" | step returns no requests: red "no unit reached a fallback pool" (`-item3`) |
| 4 `probe_flags.rs:117` | inversion (D `:21`, `:44`) | tick 1 `"plain"`, `"none"`; ticks 2 to 11 `Some(true)`, tick 12 `Some(false)` (plain branch) | tick 1 `was_projected Some(true)`, not separated (kept), `"pool"`, `"Moving"`; the test checks that no unit is within 0.5 + r + 0.1 of (60, 18), so the point is a primary slot. Ticks 2 to 200: `Some(true)` on a tick, then `Some(false)` on a later tick (the line to (60, 18) crosses the trooper row at y = 18, x 26.1 to 32.7, and is clear after x 33.2); `"pool"` until the unit stands on the point. N = 200 because arrival takes about 35 / (3 x 0.05) = 233 ticks. Test name not changed (REG) | `was_projected = Some(true \|\| was_projected)`: red "never reports a step with no projection" (`-item4`) |
| 5 `ledger_logs.rs:530` | inversion | `MUSTER_EMPTY` without, `NEVER_FILLED` without `plain_movement_progress` | both lists gain `plain_movement_progress` (doc: no creator after A2b1; A2b2 deletes the field, P `:949`); new assertion: a sampled Muster state holds a `movement_progress` row with the key `fallback` | fallback marker dropped in `fallback.rs`: red "muster: no sampled Tick state holds a fallback pool of one" (`-item5`) |

The two reads `group_movement.rs:477` and `:3022` stay as in 7b0ccdd5 (ruling: allowed). The hunk at `:2083`
(`slots - primary_count <= 6`) is labelled "tightened" (ruling).

## Conditions before merge

**(a) Muster differential, base against head.** A scratch test (not committed) runs the `muster_world` scenario of
`frame_hash.rs` (20 grouped troopers, 20 single `MoveTo` to points 0.5 apart, 120 ticks) and writes the position
bits of each unit and tick; base (`afb9c6de`, detached checkout) also writes each new plain progress, head
(`f119dc39`) each new fallback pool (labels `pre-a2b1-r-diff-base`, `-diff-head`). Result: the first fallback is
tick 1, unit 21, on both builds, for the same 20 units 21 to 40. Before it (tick 1, units with a lower id) nothing
differs. The first position difference is tick 38, unit 40; 390 of 61,800 unit-ticks differ.

**(b) `fallback_takes_the_nearest_free_hex_slot`** (`group_movement.rs`). Goal P = (20, 20) holds a resting unit.
Statics stand on five of the six ring points. Expected slot: P + (s, 0) with s = 2 x 0.5 + 0.1. The test computes
it, it does not read it from the code. It asserts the pool point P, the slot within 1e-4, `Resting`, and position
equal to the slot. Plant (slot = pool point): red "the fallback slot Vec2(20.0, 20.0) is the legal ring point
Vec2(21.1, 20.0)" (`-plant-hex`). Registry row `A2b1-fallback-hex-slot`.

**(c) `kernel_reference`.**
- K1: `Seen::count` now counts only the unit frames that the tick compared; `fallback` stays on the live run.
  Release: `unit_frames` 2,646,899 = compared; each kind > 0 (no-progress moving 942, blocked 2,364, escape 3,203).
- K2: the old assertion is removed: 64 of 64 random seeds have a fallback (55 at tick 0), so "a random seed with no
  fallback" cannot hold. New assertion: no pooled chain has a fallback (each is compared on all 300 ticks); the
  header says so. Plant (pooled chains keep movers): red "a pooled chain has a fallback" (`-plant-k2`). Row
  `A2b1-kernel-reference-pooled-chain`.

**(d) `probe_neutral.sh`** after the item-3 repair: exit 0, feature off equal to feature on
(`pre-a2b1-r-probe-neutral`). New values: `frame_chains=e6e4417f9cbe5fa5` (fallback_pools=826),
`muster_world=49fc63ffd1d8c57d`. Base values: `d0d1b1c572163424`, `aa9db740578ed5c1`.

**(e) Plant (c)** red instrument: `fallback_pool_ids_come_from_the_order_counter_in_unit_id_order`
(`group_movement.rs`), red "unit 2 takes the next id" (first pass). The registry row text names it.

## Gates (head f119dc39)

| Gate | Exit | Note |
| --- | --- | --- |
| `cargo fmt --all --check` | 0 | |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | |
| `cargo test -p turts_sim --locked` | 0 | 347 passed |
| `cargo test -p turts_sim --locked --features test-probes` | 0 | 357 passed |
| release `kernel_reference` | 0 | 3/3, 118 s |
| `cargo test -p turts_pack -p turts_audit -p turts_server -p turts_checks --locked` | 0 | 514 passed; `m2_operation_stream_matches_baseline` ok |
| wasm32 check | 0 | |
| `scripts/boundary.sh` | 0 | |
| classifier `--self-test` | 0 | 74/74 plants red |
| classifier `--p0` (base traces sha `d4c73ebe`, `805309b2`, `19dcf9a3`, `0db10e01`) | 0 | P0 byte-equal, emitter `e5152ece…` |
| classifier `--counts` | 0 | plain 0; STOP PASS |
| stream sha256 | — | `bcbf72c2…e370b` unchanged |
| `probe_neutral.sh` | 0 | values above |
| `acceptance.sh` (`MC_JOBS=4`) | 0 | 814 s, load 10.93 at start; EL1 and DET1 ok; 0 FAILED |

`probe.rs` sha256 is now `f83a92f8…`.

## What A2b2 must know

- From A2b2 on, the D 9.18 cut-off is void. The live and the reference fallback requests must be bit-equal
  (unit id, point, slots, primary count), and all ticks of all 128 chains are compared; only the commit-1
  exception of P `:938` stays. The present harness fails at `kernel_reference.rs:904-917` (live first fallback
  `Some`, reference `None`) when the reference is refreshed: the reference must return its requests.
- `plain_movement_tests.rs:394` is a third port (ruling item 1), with `:187` and P `:958`.
- `frame_chains` of commit 2 changes by representation (ruling 2(c)): show `cmp` of commit 1 with the two hash
  lines removed against commit 2.
- `make_plain_progress` has no caller outside `reference.rs`; a build of `turts_sim` without `test-probes` warns
  `dead_code`. The plain branch of the step is dead code.
- `MoveProgress.fallback` (`MoveFallback { failure_recorded }`) and `gather_goal` replace the plain fields; the
  same-group check ignores pool id 0; `views.rs` reads a recorded fallback failure as terminal Blocked.
- New file `movement/fallback.rs` (formula ratchet). No `UnitOrders` write added or moved. A1d files not touched.

## NOT TRACED

- CEO touch point 1 answer (outside this task).
- `tests/m2/refutations.py` runs of the new rows with their own build root (disk); each row was shown red,
  restored (`git status --porcelain` empty) and green in the lane.
- `mc_group`, T7 and browser cases (not run).
