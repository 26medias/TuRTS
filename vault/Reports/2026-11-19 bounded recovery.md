---
type: note
date: 2026-11-19
tags:
  - turts
  - report
ai-first: true
---

# Bounded blocked-recovery search (2026-11-18 handoff)

Where: branch `codex/bounded-recovery-fair`, worktree `TuRTS-bounded-recovery-fair`, base
`3938bd07` (`codex/dense-acceptance-integration`). Not merged. Main not touched.

## Done

- `a4fbc959`: sliced escape search with a stored cursor, exact region proof (order-free
  digest), relaxed mode under churn or cost, fair per-owner schedule (age, position, radius; no
  unit id), job state in `RecoverySearch` and in `AuditState.recovery_jobs`, counters in
  `NavigationDiagnostics` and the GM7 benchmark (candidates, visibility checks, digests, pool
  and spare-slot work, jobs, age, memory).
- `ad549add`: review findings closed; admission by row state (repairs the GM2 regression that
  the server suite found).
- `083e7e39`: the re-check found one new defect (idle rows took fresh slices before an
  overlapping searcher). Overlapping rows now come first in the fresh tier.
- Spec: `docs/superpowers/specs/2026-11-18-bounded-recovery.md` (Foundation track; gate 1 FAIL,
  then closed; code review PASS_WITH_FIXES, then closed).

## Measurements

- Stress scene of the handoff (16 x 16 map, 100 blocked movers): base 624,372 candidates in
  one tick. Branch: public charge for each owner and pass at most `OWNER_WORK` (32,768) plus the
  stated overshoot; measured peak about 37,500 public and 66,000 to 68,000 actual operations.
- Release probe (`crates/turts_sim/tests/recovery_probe.rs`, 40 ticks, 3 rounds, exclusive
  lock): 100 actors, median tick 2.61 ms (base) and 0.23 to 0.38 ms (branch); 400 actors,
  10.9 to 11.2 ms (base) and 0.75 to 0.92 ms (branch). Peak extra allocation: 545 KB to 568 KB
  (100 actors), 791 KB to 874 KB (400 actors).

## Tests on `083e7e39`

- fmt and clippy (`-D warnings`) pass.
- `turts_sim`, `turts_audit`, `turts_pack`: all pass except 4 tests that fail on the base with
  the same messages (`gmu_post_target_ends_on_a_ring_round_the_post`,
  `a_pooled_group_reads_pool_activity_real_flag_separated_and_pool_count`,
  `m2_operation_stream_matches_baseline`,
  `authored_force_move_never_fires_and_hold_remains_stationary_while_firing`).
- `turts_server --test terrain_group`: 37 passed, 0 failed, 7 ignored (8,956 s, debug build).
- `turts_server --test mc_feedback`: 1 failure,
  `group_order_uses_muster_unit_types_and_moves_all_eligible_mobile_owners` (63 of 290 moved);
  the base gives the same failure.

## Open for the CEO

- A relaxed search (after 2 failed proofs, or when a proof is too costly) commits a free point
  that may not be the nearest. Players can see this in dense churn.
- Identical movers stacked on one point share one allowance; 12 such movers took 50 ticks in a
  40 x 40 crowd (reviewer probe). The repair is one shared scan for each batch.
- The body escape after a failed order reads hidden bodies, as in the base.

## Next

The integration owner merges after the CEO agrees. The other sessions (fog audit replay, dense
movement liveness, GM2 terrain contact) get the change list from this report.
