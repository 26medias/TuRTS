# Pre-MP1 slice gate 2 (plan and ruling re-check): simulation-core-engineer lens

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=FAIL FINDINGS=12 HIGH=2 RUNS=9 NOT_CHECKED=3

The ruling answers H1, M1, M3, M4, M5, L1 and L2 correctly in direction. Its float rule for A2a is wrong (H1 below). The plan has one ordering defect that breaks the A2b/A2c boundary (H2). My probes changed no tracked file: `git status --porcelain` in TuRTS-mp1 prints nothing.

## Answer to 1a (probe numbers)

The two branches compute three values with different float expressions, not one:
- start velocity: `movement.rs:1968-1972` against `:2130`;
- push: `:1985` against `:2163`;
- speed cap: `:2003-2004` against `:2184-2185`.

I ran a scratch copy of the kernel with a mode switch. Each mode ran 64 seeds × 300 ticks. The pooled set used two groups of 20 to 60 units, and every fourth seed had a static unit of radius 4. The no-progress set used raw `MoveTarget` units whose paths cross.

| Mode | Pooled seeds that differ | No-progress seeds that differ |
|---|---|---|
| No-progress expressions win (the text of D, RUL :41, plan :601) | **46 of 64** | 0 |
| Pool expressions win | **0 of 64** | 1 of 64 |

- All modes: the `turts_sim` and `turts_pack` suites gave the same result, the stream test included. In each mode 3 tests failed, all on the same environment precondition: `economy_variants.rs:36`, "build m2_variants first".
- So no suite test pins pooled positions bitwise. GM40, GMA and GMU being green proves nothing about equality. Only `kernel_reference` can see the change, and plan A2a step 2 (`:626`) would stop A2a.
- Both kinds of frame cannot stay bit-equal with one expression. With the pool expressions, only the first contact tick of a no-progress trip changes, and that tick always enters the fallback (`:2052`), which A2b replaces.

## Findings

**High**

- **H1.** D row A2a, RUL `:41`, plan `:601`: the A2a float rule is wrong.
    - Fix text: "A2a selects the three expressions by pool presence, so the output is bit-equal on every frame. A2b removes the switch, and the pool expressions win. P0 must be equal at A2b, with the Q1b count of separated no-progress ticks recorded."
- **H2.** A2b deletes the plain reservations and identities in `orders/` (D row A2b, plan `:653`).
    - But the plain move branch (`orders.rs:2842-2900`) still calls `record_plain_move_reservation` (`:3622`), `remember_plain_move_identity`, `plain_move_reservation_exists` (`:3115`) and `arbitrate_plain_move_reservations` (`:1189`). Only A2c reroutes that branch.
    - So A2b either does not compile, or takes the A2c change before the trace.
    - Fix: move the deletion of `orders/plain.rs` into A2c, after the reroute.

**Medium**

- **M1. Answer to 1b.** The fallback today already uses the same spread code: `make_plain_progress` (`:925-955`) calls `spread_destinations_with_context`, with 6 spares and 3 claims. A harvester still sees three changes:
    - The pool is collected at arrival (`:2383-2404`). Today the `Resting` plain progress persists (`:759-761`, `:848-850`), and it gates `set_target` (`gather.rs:1248-1256`) and `run_orders` (`orders.rs:770-790`).
    - A different `Blocked` recovery: `:1659-1842` against `:718-752`.
    - The pool index chooses the spare slot, not the 6 local spares.
    - Also, pool ids come from `next_pool_id` (`orders.rs:258-259`), but `step_frame` is a pure function. The plan does not say who gives the fallback pool its id.
    - Fix text for A2b: "State the pool-id source. Do not collect a fallback pool while the gather stage still targets its goal, as for `:2399-2400`. Add a guard that counts the fallback pools of each trip." The deadline guard alone passes an oscillation that delivers in the end.
- **M2.** The S2 and S3 tables put shared items into the `plain.rs` files that A2b deletes:
    - in `movement.rs`: the helpers at `:907-923` and `:957-990` (used at `:533`, `:568`, `:594`, `:632`, `:1242`, `:1767`, and by 8 calls to `update_target_claims`); also `:480-516` (`angle`, `step()`) is not in any file;
    - in `orders.rs`: `UnitMoveNeedsAllocation` `:152`, `SpreadIndexCache` `:178`, `record_dispatch_selection` `:1358`, `movement_envelope_capacity_available_after_replacing` `:3151` (A2c needs it through `:559`), `unit_has_pending_move` `:3286`, `clear_gather` `:3661`.
    - Fix: give each `plain.rs` an item list, not line ranges.
- **M3.** The S3 step 4 reviewer fault cannot go red. I planted `× 1.0001` at `:2004`. `group_movement`, `gmu_same_point`, `orders` and `movement` all stayed green (exit 0). Fix: use the `RESULT DIFFERENT` check of `item_equal.py` (as in S2), plus one behaviour plant that is first shown red.
- **M4.** A2b is too large for one builder run. There are 46 references in `orders.rs`, 76 in `movement.rs`, 12 in `group_movement.rs`, and more in `views.rs`, `gather.rs` and `command.rs`. The file list misses `orders.rs:745-790`, `:2044-2058` (it inserts `PlainMoveProgress` on failure), `:2303`, `:2435`, `:2458`, `gather.rs:1246-1263` and `command.rs:494`. Cut it in two:
    - A2b1: the fallback pool, the gather protocol and the guards;
    - A2b2: the deletion, driven by the compiler, with P0.
- **M5.** The one-step guard counts only code that has a counter, and `no_second_step_model` greps only names. Add this check: in `movement/`, `for _ in 0..8` occurs one time (today 3: `:807`, `:2009`, `:2192`), and `step(&MovementInput` occurs one time (today 3).
- **M6. Q4.** D row A2a says "each changed assertion has a reason". That contradicts a bit-equal refactor and lets a builder weaken a test. Fix text: "A2a changes no assertion. Any changed assertion stops A2a."

**Low**

- **L1. Answer to 1c.** The bounding-box form is sound. On the probe: 0 misses, 0 results that change with the cell size, and the gate case is found. The current code gives 44 misses and misses the gate case. It scans 9 cells or fewer at reach/cell = 1. `ceil+1` is not needed.
- **L2.** R2 is sound, but its condition is a compile check only. Use `cargo test -p turts_audit` and EL1.
- **L3.** Q1b: add `was_projected` (`:2008`) to the trace. The fallback starts on `separated || was_projected` (`:2052`).
- **L4.** The single `MoveTo` routing is at `command.rs:173-174` (the `len() > 1` condition), not only at `:488-495`.

R3 and R9 are sound. After A2b, the harness drops the plain-progress frames.

## Answer to 3 (early stops)

E1 is true only in part.
- The replay validator lets only movement change the state between operations (`validate_tick_state`). It checks Other-phase operations for phase continuity only (`economylog.rs:540-556`).
- So damage and armour, power, effects and the match lifecycle can start after E1, if every write is inside a captured operation. The lobby is on the server and is independent.
- Construction, rally, repair approach and automatic targeting add orders. These need validator branches, so they wait for E2 (A1c), not for A2c. They write `MoveTarget` the same way before and after A2, so they need no rework.
- Two exceptions: tests that pin a blocked engine trip change at A2b, and tests that pin single-move operations or the S12 case change at A2c.
- The M4 route follower waits for A4.

## Not checked

- GM40, GMA and GMU in the server `mc_group` tests (ignored tests; not run).
- The list of 9 items in R2.
- Whether the oscillation of M1 really occurs (NOT TRACED).

## What I changed

None in the repository.

The probes are in a scratch copy: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/expr/` (`summary.txt`, `seeds_*.txt`, `suite_*.txt`, `s3fault.txt`, `zz_grid_probe_*.txt`).
