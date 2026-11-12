RESULT=PASS

# A2b1 repair pass 2 (pre-MP1 slice)

Seat: `simulation-core-engineer`, build mode. Date: 2026-11-12. Branch `pre-mp1/a2b1-fallback`, head `19187cb8`
(pushed; not merged). Previous head `f119dc39`, base `afb9c6de`. Review: `docs/research/mp1/review-slice-a2b1.md`
(`mp1-design`). Receipts: `/home/julien/Projects/TuRTS/target/mc-evidence/pre-a2b1-r2-*`. `origin/pre-mp1/integration`
is still `afb9c6de` (fetch): no merge. A2b1 does not merge before the re-check and the CEO answer.

## Commits

| Commit | Content |
| --- | --- |
| 9e08e06e | H1: no frame index for a pool of one; no pool clone. L1: merge walk. Tests for H1 (count), M1, M2, M3, L2 |
| c1f3fb6b | L3: the `probe_flags` comment |
| 1f9080d0 | Registry rows of the five new plants |
| 19187cb8 | Writer pin: the movement entry has 4 `MovementPools` writes (`ecs.rs:85`, `:105` take and give back) |

## H1: the cost (bit-equal)

Change (`movement/frame.rs:101-117`, `:1056-1091`; `movement/ecs.rs:79-86`, `:105`):
- A pool with `primary_count <= 1` (`is_pool_of_one`) gets no `MovementPoolIndex` in the frame setup and no slot in
  the frame slot grid. Its index is built only when its member looks for a free slot (the `Blocked` branch
  `frame.rs:458-475`, the stall spare `frame.rs:881-898`), by `lazy_pool_index`, from the units, the unit grid and
  the assigned slots of that moment. Reason for bit equality: the clearance of a slot depends only on the units
  within the refresh reach, and the frame refreshes a slot each time a unit moves near it
  (`pool_index.rs:283-321`, called after each move), so a late build gives the same clearances and availability.
- `integrate_movement` moves the pools into the frame with `std::mem::take` and back after the step: no clone.
- `pool_index.rs`, `spread.rs`, `plain.rs`, `search.rs`, `grid.rs`, `reference.rs` and the items `angle`, `step` are
  unchanged.

Bit-equality evidence (against `f119dc39`):
- `probe_neutral.sh` exit 0: `frame_chains=e6e4417f9cbe5fa5`, `muster_world=49fc63ffd1d8c57d` (equal).
- Muster differential, 300 ticks (scratch copy of `frame_hash.rs` `muster_world`, position bits of each unit and
  tick and each new fallback): `cmp` of the two dumps exit 0; 0 of 154,500 unit-ticks differ
  (`pre-a2b1-r2-diff-new`, `-diff-f119`).
- Release `kernel_reference` 3/3, the same `Seen` counts as at `f119dc39` (2,646,899 compared unit frames,
  12,991 fallback pools).
- Traces and stream: byte-equal (classifier `--p0` PASS, see gates).

Timing (release; i9-14900K, 28 threads, load 10 to 15; the review method: 1,000 and 5,000 single legacy `MoveTo` in a
packed crowd (radius 0.5, spacing 1.1, 100 per row), each to a point 60 above; whole `app.update()`; tick 1, then the
300 steady ticks 2 to 301; binaries interleaved; scratch probe `a2b1/timing/zz_a2b1_timing.rs`). Times in ms.

| Pair | base 1,000 mean / p50 | new 1,000 mean / p50 | base 5,000 mean / p50 | new 5,000 mean / p50 |
| --- | --- | --- | --- | --- |
| 1 | 2.890 / 2.725 | 2.692 / 2.793 | 13.976 / 12.825 | 25.014 / 24.734 |
| 2 | 2.461 / 2.055 | 1.825 / 1.741 | 13.306 / 12.964 | 13.489 / 13.112 |
| 3 | 1.746 / 1.648 | 1.758 / 1.659 | 11.860 / 11.688 | 12.445 / 12.044 |
| 4 | 1.708 / 1.629 | 1.724 / 1.643 | 12.092 / 11.913 | 12.213 / 12.172 |
| 5 | 1.723 / 1.635 | 1.718 / 1.642 | 13.453 / 12.444 | 14.729 / 13.978 |

- Ratio new / base of the steady mean: 1,000: 0.93, 0.74, 1.01, 1.01, 1.00 (median 1.00); 5,000: 1.79, 1.01, 1.05,
  1.01, 1.09 (median 1.05). Pair 1 at 5,000 ran at the highest load. Target (within 15 %): met in 4 of 5 pairs at
  5,000 and 5 of 5 at 1,000.
- `f119dc39`, the same method, 3 pairs: 1,000 mean 7.317, 3.797, 3.629; 5,000 mean 47.662, 33.033, 36.339.
- Tick 1: 1,000: base 44.3 to 80.4, new 45.5 to 52.6; 5,000: base 1,252 to 1,890, new 1,253 to 2,725 (pair 1).
- Early window (ticks 2 to 41, first probe run, 3 pairs): 1,000 p50 base 1.65 to 1.75, new 2.77 to 2.91,
  `f119dc39` 6.3 to 8.1; 5,000 p50 base 14.1 to 26.0, new 14.7 to 28.6. The new build is about 1.7 times the base at
  1,000 in the first 40 ticks of the crowd. The source is NOT TRACED (candidates: the pool `Blocked` and stall logic of
  D 9.18 in the packed start, and lazy index builds on stall).

Cost test: `movement::spread_tests::pools_of_one_build_no_index_in_a_free_frame` (a count, not a time; it runs in
every `cargo test -p turts_sim`, because a count needs no timing isolation): 40 units with their own pool of one in a
free frame give 0 pool-of-one index builds (`movement/mod.rs` test counter `POOL_OF_ONE_INDEX_BUILDS`). Plant (the
frame builds an index for each pool again): red "the step built 40 indexes for pools of one"
(`pre-a2b1-r2-plant-eager-index`). Row `A2b1-pool-of-one-eager-index`.

## L1

`assign_fallback_pools` (`movement/fallback.rs:77-113`) walks the outputs and the requests once (both in unit-id
order; a debug assertion checks the outputs). The ids are unchanged (all probe values and the differential equal).

## M1 (same-tick reservation)

From the code: the step reserves the footprint of the previous unit at its target after its own step
(`frame.rs:212-222`, `previous.target.unwrap_or(previous.position)`), and a fallback writes the slot into
`units[index].target` (`frame.rs:656` and `:962`, written at `:995`) before the next unit runs. So the present code gives two same-tick
fallbacks two slots; only the fault shows the defect. Test `two_same_tick_fallbacks_take_two_ring_slots`
(`group_movement.rs`): unit 1 rests on P (30, 15); units 2 and 3 take P as engine trips. Expected slots from the ring
rule in the test (s = 1.1; unit 2 the smallest angle, unit 3 the smallest angle at least s from it): (29.45, 14.04737)
and (30.55, 14.04737); and no two pools hold one slot. Plant F1 (the reservation keeps the input target): red "unit 3
takes Vec2(30.55, 14.047372), got Vec2(29.45, 14.047372)" (`-plant-f1`). Row `A2b1-same-tick-reservation`.

## M2 (9.17 at the escape clear with no step)

Test `movement::spread_tests::a_blocked_fallback_keeps_blocked_when_its_escape_ends_without_a_step`: a frame with one
unit, a `Blocked` fallback pool of one, a physical escape and no overlap. Expected (D 9.17): the escape ends, the
progress is unchanged; the same frame with an order pool clears the progress. It reaches `frame.rs:240-242` directly,
with no cut-off. Plant F5 (the guard `&& !progress.is_fallback()` removed): red "a Blocked fallback pool keeps Blocked
until a new order or a new trip (9.17)" (`-plant-f5`). Row `A2b1-escape-clear-917`.

## M3 (9.19)

A reachable case where 9.19 changes a result: the gather endpoint changes while a fallback is in flight. Test
`fallback_ends_when_the_gather_endpoint_changes` (`group_movement.rs`): worker A (legacy `Gather`) takes a fallback on
its node-1 leg (an idle worker stands on its approach point); worker C empties node 1 (stock 4) at tick 5; A must
gather at node 2 within `2 x (distance / speed) + 5 s` (deadline from the test geometry). Plant F2 (`gather_goal` never
written): `set_deposit` removes the target, the order phase does not see the changed endpoint (`orders/run.rs`
endpoint check reads `gather_goal`), the fallback keeps no target; red "worker A never gathers at node 2 after node 1
empties at tick 5; deadline 379 ticks" (`-plant-f2`). Row `A2b1-gather-goal-never-written`.
Probe (not committed): the same case with an MC Gather order ends the fallback one tick after the order fails
(stage Idle, `gather_goal` Some against None), so no fallback stays with no target (`pre-a2b1-r2-scratch-linger`).

## L2, L3

- `fallback_ring_tie_takes_the_smallest_angle`: P (30.3, 17.7), r 0.7, s 1.5, a post on the -120 degree point, a
  resting unit on P; the test takes the legal ring point with the smallest `atan2` (MC design 6.3 "ties by angle"):
  (31.05, 16.40096). Plant F3 (the second pool slot): red "... got Vec2(31.8, 17.7)" (`-plant-f3`). Row
  `A2b1-ring-tie-off-by-one`.
- `probe_flags.rs` doc: the projection window of ticks 2 to 200 is an observed property of the scenario, not a value
  that the geometry proves.

All plants: anchor once, `git checkout`, `git status --porcelain` empty, green at the gates.

## CEO sentence

Confirmed: at A2b1 the unit with the lower id gives the point up, whoever was sent first. The step reserves every
mover at its target at the start of the frame (`frame.rs:132`); the units step in id order; the lower id meets the
reservation of the higher id on the same point (`frame.rs:624`, `conflicts_with_reserved`) and takes the fallback
(`frame.rs:633`); its reservation then moves to its slot (`frame.rs:215`), so the higher id keeps the point. A single
`MoveTo` has no pool at A2b1 (`command.rs:173`, `:488-495`). "The first unit keeps the point" needs the pool path of
A2c (S12, plan `:1007`).

## Gates (head 19187cb8; sim source equal to 1f9080d0)

| Gate | Exit | Note |
| --- | --- | --- |
| `cargo fmt --all --check` | 0 | 1f9080d0; 19187cb8 checked with `--check` |
| clippy `--workspace --all-targets --locked -D warnings` | 0 | |
| `cargo test -p turts_sim --locked` | 0 | 352 passed |
| same `--features test-probes` | 0 | 362 passed |
| release `kernel_reference` | 0 | 3/3, 84.7 s (1f9080d0) |
| pack, audit, server, checks | 0 | 514 passed; `m2_operation_stream_matches_baseline` ok |
| wasm32 check | 0 | |
| `boundary.sh` | 0 | |
| traces | — | `d4c73ebe…`, `805309b2…`, `19dcf9a3…`, `0db10e01…` (equal) |
| stream sha256 | — | `bcbf72c2…e370b` unchanged |
| classifier `--self-test`, `--p0`, `--counts` | 0, 0, 0 | 74/74; P0 PASS; STOP PASS |
| `probe_neutral.sh` | 0 | `e6e4417f9cbe5fa5`, `49fc63ffd1d8c57d` |
| `acceptance.sh` (`MC_JOBS=4`) | 0 | 771 s, load 8.99 at start; EL1 and DET1 ok; 0 FAILED |

## NOT TRACED

- The source of the early-window cost (ticks 2 to 41 at 1,000).
- Timing on a 4-vCPU host (D57, D58); this machine is a loaded 28-thread desktop.
- Float-boundary cases where a late index could differ from the kept one (a unit exactly on the refresh reach);
  none seen in the differential, `kernel_reference` or the probe values.
- L4 (merge of `tests/m2/refutations.json` with A1d): not handled here; keep both row sets at the merge.
- `refutations.py` runs of the new rows with their own build root (disk); proved in the lane.
- CEO touch point 1 answer; T7 and browser cases.
