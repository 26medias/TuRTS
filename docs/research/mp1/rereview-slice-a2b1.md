VERDICT=PASS

# Re-check: pre-MP1 slice task A2b1, second repair pass (19187cb8)

Line 2 (seat contract): VERDICT=pass-with-changes FINDINGS=4 HIGH=0 RUNS=31 NOT_CHECKED=4

Seat: performance-architect, review mode. Date: 2026-11-12. Candidate 19187cb8 (`pre-mp1/a2b1-fallback`), previous
head f119dc39, base afb9c6de. Lane `/home/julien/Projects/TuRTS-pre-a2b1-review`, cache `target/mc-pre-a2b1-review`.
Receipts: `/home/julien/Projects/TuRTS/target/mc-evidence/a2b1-rerev-*`. Evidence: the folder `rr/` next to this file.
At the end, the lane is clean at 19187cb8 (`git status --porcelain` is empty) and I deleted the probe binaries from
the cache. What I changed: none.

## Findings (no High)

M1. Medium, instrument. `crates/turts_sim/src/movement/frame.rs:1063-1091` (`lazy_pool_index`), `:881-891` (stall
spare). The lazy path of the H1 repair has no red test. These are two faults of my own:
- LZ1: the lazy build skips the clearance refresh (`frame.rs:1085`, `0..0`).
- LZ2: the stall spare does not use the lazy index (`.or(None)` at `frame.rs:888-890`).
With each fault, `cargo test -p turts_sim --features test-probes` passes 362/362, and the release `kernel_reference`
passes 3/3. Each fault changes the authored Muster scenario: 580 of 154,500 unit-ticks differ from tick 203 (unit
31, the unit that takes a spare at tick 202), and 10 units end at another place. `probe_neutral` cannot see it,
because it compares feature off with feature on of one build. The clean head is bit-equal to f119dc39 in the same
scenario (see 1a), so the lazy path is correct today, but no instrument holds it.
Fix: put chains with pools of one in `kernel_reference` (the reference keeps the eager index), and compare them on all
ticks. Or add a test with a stalled member of a pool of one whose expected spare comes from the ring rule.

L1. Low, instrument. `frame.rs:962-963` (pushed or projected branch). My fault F6 (`then_some(movement_target)`)
passes 362/362. It changes a real case: an engine trip (a `MoveTarget` with no order) to (0.2, 10), inside the edge
inset (r = 0.5), with a neighbour that projects the unit on its first step. The head rests on the clamped point
(0.5, 10). With F6 the unit stalls at the edge, takes a spare and rests at (1.6, 10). The base rests at
(1.05, 9.04737). A legacy `MoveTo` is clamped at the command, so the same case through `MoveTo` does not change.
F6 is closed: it is not equivalent, and the head behaviour follows MC design 6.3 step 4 ("after the clamp"). Fix: a
test for a pushed engine trip to an edge point, with the expected value from the clamp rule.
NOT TRACED: whether a production engine trip (for example a gather approach) writes a target inside the edge inset.

L2. Low, merge. `tests/m2/refutations.json`. `git merge-tree` against `origin/pre-mp1/a1d-oracle` (e2ba109c) gives a
content conflict: both branches append rows. `crates/turts_checks/tests/movement_writers.rs` merges with no conflict.
I put the seven merged `turts_checks` files of the merge tree in the lane: `cargo test -p turts_checks` passes
(38 tests, with A1d's new visitor, `run_constant_writers`, and the pin `("method", "MovementPools", 4)`). Then I
restored the lane exactly. Integration (afb9c6de) is a fast-forward.

L3. Low, cost (for A2c). `ecs.rs:159-173` and `orders/run.rs` `fallback_progress`. In each tick, every pool of one
costs a `BTreeMap::retain` step, a set insert, and up to 3 `fallback_progress` clones. The early-window profile
shows `ExtractIf` at 2.2 % and `fallback_progress` at 3.0 % of the head tick. These parts are the measured residual
(see 2). A2c gives a pool of one to every single move, so this cost grows linearly with single moves. It is bounded
and small.

## 1. H1

(a) Bit-equality to f119dc39, from my own differential (debug probes, and release crowds of 300 ticks). I compared
the full dumps with `cmp`: every unit and tick, position bits, target bits, pool id, slot bits, activity and the
fallback flag. All 14 files are equal:
- the Muster scenario, 300 ticks;
- the DET1 script in process, 420 ticks;
- my seven small scenarios and the edge-push cases;
- the 1,000 crowd and the 5,000 crowd, 300 ticks each (sha256 b61e7b2d... and 77e08bf4..., equal on both builds).
The audit-state hashes are also equal (Muster 09b14ec4a8dab29b, DET1 6bb24033ceb95dc1). `probe_neutral` gives
e6e4417f9cbe5fa5 and 49fc63ffd1d8c57d. The release `kernel_reference` gives the same `Seen` counts (2,646,899 unit
frames, 12,991 fallback pools).

Float-boundary risk. The kept index and the lazy index can hold different clearance values. My bound on the
difference:
- The step skips a refresh only for a unit outside the refresh reach R = r_u + max_r, at both its old and its new
  position: the f32 test is d² > fl(R²) (`pool_index.rs:296-300`, `grid.rs:98-100`).
- Such a unit can still be a clearance hit at the boundary, where f64(d32) - r_u <= max_r (`pool_index.rs:127-136`).
  Its clearance is then at least max_r - 2 ulp(R).
- Clearances are read only in threshold tests >= r - 1e-5 (`pool_index.rs:183-222`). Here r <= max_r. The candidate
  choice reads only `assigned_slots` and `is_free` (`:247-266`).
- So a stale or late value is on the same side of each threshold while 2 ulp(R) < 1e-5, which is true for R below
  about 40 map units.
Result: the late index can differ in state, but it gives the same decision. I could not construct a case below that
radius. The builder's reason ("a late build gives the same clearances") is not exact. Its conclusion holds for
results.

Can another pool's member take a slot that this pool holds? No. A slot is held through `assigned_slots` keys that
include the pool id (`pool_index.rs:254-256`, `search.rs:108-111`). The slot grid feeds only the clearance refresh of
the same pool's index (`pool_index.rs:296-317`). Other pools are kept apart by bodies and spread reservations, which
read no pool index. I found no case.

(b) `mem::take` (`ecs.rs:85`, `:105`). Between the two calls there is no early return: the code reads `UnitOrders`,
then runs the pure step. This is an exclusive system. A panic in the step ends the app, so no reader sees the empty
resource. The pin of 4 is still a count guard against a fifth call site. It cannot tell a take and give-back pair
from two other writes.

(c) Timing. Release, i9-14900K, load 8-11, whole `app.update()`, 20 Hz assumed, a packed crowd of single `MoveTo`.
Twelve interleaved pairs at 1,000 units, medians:

| Measure | base | new head | ratio of the pairs (median) |
| --- | --- | --- | --- |
| tick 1 | 57.5 ms | 59.6 ms | |
| ticks 2-41, p50 | 1.588 ms | 1.645 ms | 1.03 |
| ticks 2-41, p99 | 2.06 ms | 2.25 ms | |
| ticks 42-301, p50 | 1.718 ms | 1.789 ms | 1.05 |

Three rounds with f119dc39 at 1,000: ticks 2-41 p50 3.60-3.67 ms, ticks 42-301 p50 3.75-3.79 ms (about 2.2 times the
base).

Three rounds at 5,000:

| Measure | base | new head | f119dc39 |
| --- | --- | --- | --- |
| tick 1 | 1.33-1.90 s | 1.41-1.52 s | |
| ticks 2-41, p50 | 13.1-24.0 ms | 13.5-13.6 ms | 24.7-28.7 ms |
| ticks 42-301, p50 | 11.7-21.2 ms | 12.4-13.3 ms | 33.1-33.3 ms |

## 2. The open cost

I could not reproduce it. Over 12 pairs at 1,000, the early window is +3 % (medians) and +2.7 % (minima), not 1.7
times. In my rounds 1 and 3 the base itself reached 2.85-2.87 ms, inside the builder's band of "new" values
(2.77-2.91 ms). So the builder's figure is in the load noise of this machine.

The LBR profile of the early window (10 crowds x 40 ticks) shows no `lazy_pool_index` and no `MovementPoolIndex::new`
above the noise. The new-only items are `ExtractIf` (the pool retain) at 2.2 % and `fallback_progress` at 3.0 %
(L3).

A unit that stays `Blocked` and still overlaps rebuilds its lazy index in each tick (`frame.rs:458`). This costs at
most what the old eager build cost for that pool. A cheaper form: build the lazy index only when `retained_escape` is
`None` (`frame.rs:451`, `:504`). Today the code computes `pool_escape` when it does not use it. This form is
bit-equal, because the computation is pure, except for a `cfg(test)` counter.

This cost does not block the A2b1 merge.

Cost gate for A2c:
1. The count test `pools_of_one_build_no_index_in_a_free_frame` stays green.
2. A new count: in one tick, the lazy index builds are at most the number of members that are `Blocked` or at the
   stall deadline.
3. A ratio: the packed crowd of 1,000 and of 5,000 single `MoveTo` (1.1 spacing, targets 60 above), 12 interleaved
   pairs against the A2c base. Ticks 2-41 p50 and ticks 42-301 p50 each <= 1.15 times the base, by the median of the
   pair ratios.

## 3. New tests and faults

All faults restored exactly (`git diff` empty), next run fresh. Each fault ran `cargo test -p turts_sim --locked
--features test-probes`.

| Fault | Result now |
| --- | --- |
| F1, reservation not updated (`frame.rs:212`) | red: `two_same_tick_fallbacks_take_two_ring_slots` |
| F2, `gather_goal` never written (`gather.rs:1429`) | red: `fallback_ends_when_the_gather_endpoint_changes` |
| F5, 9.17 at the escape clear (`frame.rs:240`) | red: `a_blocked_fallback_keeps_blocked_when_its_escape_ends_without_a_step` |
| Tie, angle key negated (`spread.rs:236`) | red: `fallback_ring_tie_takes_the_smallest_angle`, the same-tick test, 3 spread tests, the helper pin |
| LZ1, LZ2 | green (M1) |
| F6 | green (L1) |

Expected values:
- The same-tick test, the tie test and the gather deadline compute their values from the ring rule or the geometry
  in the test (`group_movement.rs:4052-4108`, `:4110-4185`, `:4188-4255`).
- The tie test also checks its own rule against (31.05, 16.40096), my independent value from the first pass.

The M2 test (`spread_tests.rs:1057`) builds the state by hand: a `Blocked` fallback with primary 1, an escape, and no
overlap. A real run can reach this state kind: the stall sets `Blocked` with `failed_blocked`, and `frame.rs:980-991`
then sets the escape. My first-pass reach plant F5c reached this site in `kernel_reference` runs at f119dc39.

The count test can go red: the builder's eager-index plant made it red. My lazy faults do not touch it.

## 4. F6

Closed. It is a real fault with an effect (L1), not an equivalent mutant.

## 5. Gates (19187cb8)

| Gate | Result |
| --- | --- |
| `cargo test -p turts_sim --locked` | 352/0 |
| the same with `--features test-probes` | 362/0 |
| release `kernel_reference` | 3/3 |
| pack, audit, server, checks | 514/0; `m2_operation_stream_matches_baseline` ok |
| clippy `-D warnings` | exit 0 |
| `boundary.sh` | exit 0 |
| `probe_neutral.sh` | exit 0 |
| traces | emitted at the head; `cmp` equal to the base traces d4c73ebe..., 805309b2..., 19dcf9a3..., 0db10e01... |
| stream | bcbf72c2... (file and emitted) |
| classifier | `--self-test` 74/74, `--p0` PASS, `--counts` PASS |
| merge | see L2 |

## NOT TRACED

- A boundary case with R above about 40 map units.
- Production engine trips that target inside the edge inset.
- The DET1 two-process run: ignored, I did not run it.
- Numbers on a 4-vCPU host.

## What I ran (claim | result that would refute | observed)

- `a2b1-rerev-dbg-*-r2`, `-rel-*-r2`: H1 is bit-equal to f119dc39 | any `cmp` difference | none in 14 files
- `-fault-*` (7 runs): the new tests go red on F1, F2, F5 and the tie fault | green | red; LZ1, LZ2 and F6 green
- `-eff-*` (3 faults x 3 runs): the lazy path and F6 are reached | equal dumps | Muster and edge-push change
- timing in `rr/timing/` (3 rounds x 3 builds, then 12 pairs): the builder's early cost of 1.7 times | ratio 1.7 or
  more | 1.03
- perf LBR, early window, base and new | lazy builds dominate | not seen; retain 2.2 %
- `-g-*` (12 runs): the builder's gates | any red | none
- `git merge-tree` x2, and `-merge-checks`: A1d merges | a conflict or a red check | only the refutations rows
