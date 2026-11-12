# Pre-MP1 slice gate: simulation-core-engineer lens

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=FAIL FINDINGS=8 HIGH=1 RUNS=10 NOT_CHECKED=2

The central A2 claim is true: I proved it with a probe. A2 fails as written for one reason: its deletion list removes the movement branch that all the engine-made moves use.

## Findings

**High**

**H1. Needs decision.** The deleted range includes the arrival code of the branch for a unit with a `MoveTarget` and no progress (`movement.rs:1880-2085`, arrival at `:1932-1966`, fallbacks at `:1893` and `:2053`). That branch moves:
- gather trips (`gather.rs:1246-1263`);
- queued Move and Attack starts (`orders.rs:2505-2578`);
- the attack approach (`:2295-2305`), follow (`:2445-2460`) and resume (`:1004`);
- a single `CommandKind::MoveTo` (`command.rs:495`);
- `--synthetic-units` (`synthetic.rs:72,92`).

Probe: base, haul and cedar spend 256, 230 and 289 unit-ticks on this branch, and 0 on `PlainMoveProgress` or `MoveProgress`. Options:
- (a) Keep this branch for engine-made trips. Then "one model" is not true.
- (b) A pool of one, made in the same tick, for each writer. The base, haul and cedar streams then change.
- (c) Limit A2 to player single moves.

Text change for A2: "The branch for a `MoveTarget` with no progress (`:1880-2085`) stays/goes [decision]. Its users are listed. `PlainGatherGoal` moves to [x]."

**Medium**

**M1.** The pool of a lone unit is collected in its arrival tick (`movement.rs:2383-2404`, `:2497-2508`). Probe: 0 pools and 0 `MoveProgress` after arrival in all lone cases. Only Attack-Point and follow units keep it (`:2399-2400`, `:2456-2469`). Input `:100` is refuted. Close section 8.2 with this text.

**M2.** A player sees a change. A second unit is ordered to P while the first unit still moves to P:
- Plain path: the first unit gives up P (`:1882-1893` sees the reserved target of the later unit).
- Pooled path: the later unit takes a hex slot.

Section 1 says "changes no rule that a player sees". Change it to: "except a second move to a point that a moving unit holds". Add that case as a guard. "Displaces only on arrival" is false: the plain path displaces in the first frame.

**M3.** The cap claim is half true. `SPREAD_CAP` is at `orders.rs:1190` and `:1657`. A Move order with group 0 counts in the cap already (`:1265`), and the units it leaves out already take the pooled path (`:1317-1354`). Only a single `CommandKind::MoveTo` skips the cap. After A2, of 3,000 orders in one tick, 2,048 units start at tick T and 952 at T+1. The synthetic load sends no orders. It writes `MoveTarget` for 1/40 of the units per tick (`synthetic.rs:88-93`), so the cap does not apply. Its meaning depends on H1.

**M4.** The A3 proof is false at the edge. `within` drops points that are no more than 7.2e-7 inside `reach` when reach/cell is a whole number. That is the usual case: reach `r+max_radius`, cell `2*max_radius`. The probe found 43 misses and 41 results that change with the cell size. Two callers have a zero margin (`movement.rs:1704-1705`, `:2308-2309`, `>= r+other.r`). Fix: `rings = ceil(reach/cell)+1` (`grid.rs:63`); the probe then finds 0 misses. Land the fix and the property test before the cell change. I count 27 `within` call sites in `movement.rs`, not 21, plus `orders.rs:1115` and `combat.rs:330`.

**M5.** Classifier P3: an operation for an order with a name falls on the tick after arrival. In the acme probe, arrival is tick 123 and the operation is at tick 124, because Orders runs before Movement (`plugin.rs:97-98`). Text: "the accept tick or the tick after arrival".

**Low**

**L1.** A4 is a real seam only as functions for one unit at a time on `&mut FrameState`, called inside one loop in unit-id order. The loop changes shared state as it goes: units, points, grid, pool indexes, claims, `target_claims`, at 9 places where a move is written. If each phase runs over all units before the next phase starts, the results change. The `spare_claim` range `:2290-2400` also holds the write-back and the resting collection. The order A2, A4, A3 is right after H1 is decided. The reference frames must include no-progress, escape, Blocked, follow and Attack-Point units.

**L2.** The depletion repair (uncommitted in `/home/julien/Projects/TuRTS-mc-fix-deplete`, `orders.rs:1027-1031`) keeps receiver trips after depletion. That is more H1 traffic. It also edits the replay validator, which A1 deletes. Run the pre-A2 trace after the repair is merged, and add a guard for a delivery after depletion.

**L3.** A single move dispatches in the same tick on both paths. First move tick 2 on all three paths.

## What I ran

All cargo runs exited 0. The first build failed to compile (101, missing import). That was not a red.

| Run | Claim tested | Result that would refute | Observed |
|---|---|---|---|
| Lone probe: 13 scenarios × MoveTo / Order group 0 / group 1 | Input `:98-99` | A difference in position bits for a lone unit | None. Only S12 differs |
| 5,000 units: load, then 100 resting ticks | Section 8.2 | Pools remain after arrival | 0 pools on all paths |
| 3,000 orders in one tick | Input `:102` | Order group 0 is not capped | Capped at 2,048 |
| Stream movement-state probe | Input `:106` | base, haul, cedar use `PlainMoveProgress` | Not used |
| acme pooled vs plain | Input `:105` | No new operations | Operations at ticks 1, 1, 124 |
| `within` vs a brute-force scan, 24,000 queries | Input `:140` | A miss | 43 misses |
| The same with rings+1 | The fix | A miss | 0 misses |
| greps (`within`, `MoveTarget` writers, `SPREAD_CAP`) and the repair diff | Input `:143`, A2 scope, section 7 | — | As cited |

## Probe results (lone unit, plain path vs pooled path)

| Case | Bits equal | Arrival | Final |
|---|---|---|---|
| S1 diagonal | yes | 228 = 228 | (32, 32) |
| S2 18.07 | yes | 69 | equal |
| S3 arbitrary | yes | 256 | equal |
| S4 off map | yes | 375 | (63.5, 0.5) |
| S5, S6, S8 occupied or reserved by a resting unit | yes | 108 | (19.45, 19.047) |
| S7 static unit r4 far away | yes | 82 | equal |
| S9, S10 zero or tiny distance | yes | 2 | equal |
| S11 sequential, after arrival | yes | 228 / 452 / 695 | equal |
| S12 second order while the first unit moves | **no**, first difference at tick 21 | 230 / 231 vs 233 / 227 | P goes to the other unit |
| S13 crossing paths | yes | 169 / 164 | equal |

Resting cost per tick: 9.17, 9.48 and 9.71 ms. I ran each one time, so the difference can be noise. NOT TRACED: the machine load.

## Not checked

- The `refresh_slot_clearance` cap argument: I read it and it holds, but I did not run it.
- The exact operations of a legacy envelope with no order (NOT TRACED).

## What I changed

None. I removed my probe files. `git status --porcelain` shows one file that is not mine: `crates/turts_sim/tests/zz_mp1_obstacle_probe.rs`. Another agent probably made it. I did not touch it.

## Open questions for the orchestrator

- H1 needs a CEO decision between options (a), (b) and (c).
- Who owns the stray probe file?

Raw output: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/` (`lone.txt`, `all.txt`, `stream_*.txt`, `zz_grid_probe*.txt`, and the probe sources).
