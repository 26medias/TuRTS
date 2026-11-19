# Bounded, fair blocked-recovery search

Status: Baseline (revised after gate 1). Date: 2026-11-18. Track: Foundation (it adds
audited movement state and changes a movement result when a search needs more than one
tick). Source: `target/mc-evidence/handoffs/2026-11-18-bounded-recovery.md`. Base: `3938bd07`.

Gate 1 (simulation-core-engineer, one lens): FAIL, 3 High. All findings are closed in this
revision; see section 8. Code review of `a4fbc959` by the same seat, with runs and mutants:
PASS_WITH_FIXES, 0 High; its findings are closed in section 9.

## 1. Problem

Three escape searches ran to completion inside one tick, with no work account:

- the group blocked escape `find_free_escape_destination` with the pool search `nearest_free`;
- the plain blocked escape `find_free_singleton_destination_known`;
- the body escape after a failed order (`PhysicalEscape`), `find_free_singleton_destination`.

Each candidate called `UniformGrid::within_unsorted`, which allocates. A fully blocked search
from the centre of a 128 x 128 map examines 78,246 lattice points. The handoff stress scene
(packed 16 x 16 map, 100 blocked movers) costs 624,372 candidates in one tick in the base
(counted from `finite_hex_ring_limit`). A cap that discards an unfinished scan starves it. A
plain cursor is not exact: a nearer site that was blocked when scanned can become free.

## 2. Contract

R1. **Bounded charge.** The public recovery charge of one owner in one solver pass is at most
`OWNER_WORK` (32,768) plus the overshoot of section 4. The public charge counts each lattice
candidate (1), a fixed cell charge for each grid query (9), each permitted body that touches
the query disc, and each claim key of the own pool. Actual operations (all cells and bodies
visited, hidden bodies included) are recorded apart. The terrain path can run the solver up
to `MAX_SOLVER_RETRIES` (8) times in one tick; each pass gets the same schedule.

R2. **Exact when the region is stable.** A search that ends in the slice where it started
gives the base result (one stated difference: when the scan reaches the ring limit, the group
search now returns its best point instead of none; every in-bounds point was examined). A
search that ends in a later slice is exact for the state of its final slice, unless it runs
relaxed (R3).

R3. **Relaxed under churn or cost.** A job runs relaxed after `RELAX_AFTER` (2) failed
proofs, or when one proof costs more than half its slice. A relaxed job does not prove its
region. It commits only a point that is free in the current state; the point may not be the
nearest. This is the only result change a player can see.

R4. **Fair service, no unit id.** A job has an age (ticks since it started). Fresh jobs get
`FRESH_SLICE` (1,024) in service order from half the budget. The oldest batch of jobs with a
scan gets the rest. Service order is age (descending), then x, y, radius (`total_cmp`). Rows
with an equal key are one batch and share the allowance equally. No unit id and no hash
order selects service. Limit: rows still commit in row (unit id) order inside one tick, as in
the base; when two jobs want the same point in one tick, the lower id commits it.

R5. **Privacy.** The group and plain searches read only bodies that `known` admits. The
digest and the public charge include only admitted bodies, selected by each body's own
radius, so hidden bodies and the frame maximum radius change neither results, proofs, slice
ends nor the schedule. The schedule reads only the row state, its age, its position and
its radius. Exception: the body escape after a failed order
(`PhysicalEscape`) reads all bodies, as in the base, because it resolves an actual overlap;
its result already depends on hidden bodies in the base, and now its slice ends do too.

R6. **Determinism and replay.** The job is a body component `RecoverySearch` (88 bytes).
`MovementFrame.recovery` carries it in; `MovementFrameOutput.recovery` (boxed) carries it out.
`AuditState.recovery_jobs` serializes it (serde default, skipped when empty). Replay builds the
frame from the previous audited state, so it rebuilds the same schedule and slices. A job of
another kind or pool is dropped; a row that leaves the recovery path loses its job.

R7. **No overlap and first arrival.** Unchanged: the committed point goes through the same
claim, step and certificate code as the base.

R8. **Bounded memory.** Retained: 92 bytes for each job (component plus id). Phase: one
schedule vector (24 + 16 bytes for each blocked row, counted with capacity) and one job map.
The scan allocates nothing (`UniformGrid::visit_within`).

## 3. Design

### 3.1 Job state

`RecoverySearch { kind: Group(pool) | Plain | Physical, age, scan }`. `RecoveryScan { origin,
radius, ring, side, step, best, examined, digest: [u64; 3], invalidations, relaxed }`.

### 3.2 Schedule (tick start, per owner)

Rows: speed set; and a physical escape, a blocked group progress with its pool, or a blocked
plain progress. Admission is by state only, not by tick-start overlap: an earlier row can
overlap a row mid-tick, and the base searched such a row at once (see section 9, GM2). Fresh rows (no scan) in service order get
`FRESH_SLICE` while half the budget lasts (a first batch that does not fit shares the half).
The oldest batch of rows with a scan shares the rest. With no such batch, the served fresh
rows share the rest. A row that needs a search in the tick but has no allowance waits one
tick (age + 1). It never falls back to an unbounded search.

### 3.3 Slice

1. A scan of another origin or radius is dropped.
2. If the scan is not relaxed: compute the digest. Members: admitted bodies with
   `distance(origin) <= examined + radius + own radius + 1e-3`, and own-pool claim keys within
   `examined`. Equal: continue (relax if the digest cost more than half the slice). Different:
   restart, `invalidations + 1`.
3. Group only: compute the pool escape live, charged against the slice. Its distance bounds the
   ring scan: stop when the nearest unseen ring is farther.
4. Continue the ring scan until the charge reaches the allowance or the base end rule holds.
5. Ended: choose as the base (pool vs outside by `nearest_angle_order`). A resumed relaxed scan
   first checks that its point is free now; if not, it restarts relaxed. Not ended: store the
   digest of the new examined region (skipped when relaxed).

The digest is a commutative sum of two 64-bit mixes plus a count. Its collision chance is
near 2^-64 for each check; it is not a proof. This is a stated limit.

### 3.4 Liveness

- A fresh job is served within `ceil(n_fresh / 16)` ticks (age order).
- Jobs with a scan are served first in, first out: a job waits for the remaining work of the
  jobs ahead of it, divided by the continuation budget.
- A served job ends after at most `ceil(C / S)` services in a stable region when a proof costs
  at most `S / 2`; else it relaxes. A relaxed job ends after `ceil(C / S)` services; its
  commit fails only when another body took the point first, and it then restarts.
- A search with no free point ends with none; the row starts a fresh job on the next tick.

## 4. Bound

Per owner and solver pass: `public <= OWNER_WORK + sum over served jobs of (one candidate +
one digest + one pool search)`, plus the tick-start overlap queries. A candidate or a digest
charges at most `9 + admitted bodies + own claims`. A pool search charges at most
`1 + 2 * slots` for each reach step plus 9 + touching bodies for each slot tested.

## 5. Accounting

`SweepBudget.recovery` (`RecoveryCounters`) is folded into `NavigationDiagnostics`
(`recovery_*`) on the terrain path; the GM7 benchmark exports the peaks. The stall spare-slot
search (`nearest_free` after a stall) is counted (`recovery_spare_slot_work_this_tick`) but not
scheduled: it runs at most once for each stall cycle of a mover and is bounded by its pool.

## 6. Tests

`crates/turts_sim/src/movement/tests/recovery_tests.rs`: T1 stale nearer site (and the same
case without the proof, which picks the stale point), T1b hidden bodies, T2 unrelated motion,
T3 equivalence with the base search (400 random scenes, hidden bodies, slots, pool bound),
T4 stress scene, T5 deep stable crowds, T6 churning crowd, T6b permanent churn and costly
proof, T7 schedule id-free and oldest first, T7b stacked batch, T8 determinism, T9 small
scene in one tick.

## 7. Plan

Built in one pass (light plan; foundation review at the end): grid visitor, job types and
fields, sliced scan, schedule, three call sites, ECS and audit plumbing, diagnostics, tests.

## 8. Gate 1 findings and their closure

- H1 digest cost stalls far searches: relax when a proof costs more than half the slice;
  oldest-first service gives the job the whole continuation budget.
- H2 hidden facts in slice ends: public charge; digest membership by own radius; no stored
  frame radius; visit-all visitor (no early exit).
- H3 holes in R1: retries stated (8 passes); batches share one allowance; pool search charged
  against the slice.
- M1: physical escape is scheduled; spare-slot search is counted (section 5).
- M2: a row without allowance waits (3.2).
- M3: the claim scan is charged publicly only for own-pool keys; the actual scan of the whole
  claim set is recorded (limit).
- M4: id commit order inside a tick is stated (R4); a search with no point restarts as a
  fresh job at age 0 (back of the queue).
- L1: sizes corrected (R6, R8). L2: the ring-limit difference is stated (R2) and T3 checks it.
  L3: `OWNER_WORK` is separate from `route::OWNER_CAP`.

## 9. Code review findings and their closure

- M-2 physical path reads hidden bodies: stated as the R5 exception (base semantics kept).
- Tick-start overlap charge: removed with the overlap filter (next item).
- Regression found by the server suite: `terrain_group`
  `gm2_browser_projected_destination_and_delayed_orders_complete` failed on `a4fbc959` (passes on
  the base). Trace: 23 waits, all `Physical`: rows with a body escape that started a tick clear
  were overlapped mid-tick by an earlier row and had no allowance. Fix: admission by state.
  After the fix the scenario passes (715 s) with 2 physical slices and 0 waits.
- M-3 tests that could not go red: two new tests. Mutants D (free check stops at the first
  blocker), E (hidden bodies in the public charge) and G (digest membership by the frame
  maximum radius) now each turn a test red (`grid_insertion_order_changes_no_result_or_charge`,
  `hidden_bodies_and_a_large_hidden_radius_leave_the_public_trace_unchanged`).
- L-1 loose stress bound: the assert now uses 16 fresh slices plus the oldest batch.
- L1/L-2 sizes: corrected in R6 and R8 from the printed sizes.
- M-1 stacked identical movers: stated limit, not repaired. They form one batch, share one
  allowance and contend for the same point. Reviewer measurements in a 40 x 40 crowd: 1 mover,
  1 tick; 4 movers, 6 ticks; 12 movers, 50 ticks, peak public charge 50,484, 12 of 12 commits
  relaxed. A repair is one shared scan for each batch that continues after each commit.
