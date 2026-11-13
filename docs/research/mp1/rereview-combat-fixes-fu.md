# Re-review: mp1/combat-fixes-fu at 7be9364c (performance-architect, review mode)

VERDICT=PASS HIGH=0 (LOW=3)

Probe sources: standoff_exact.rs (all inputs), standoff_maps.rs (map workloads), in this folder.
Logs: /home/julien/Projects/TuRTS/target/mc-evidence/combat-rerev-*/command.log.

## 1. Search on the new function (exact copy of stand_off_steps and step_toward)
27.85 M cases. Inputs: M 1 to 2^20; r 1e-3 to 1000; negative and mixed-sign targets; targets on an axis (x=0 or y=0); integer targets; axis, diagonal and random bearings.
- Fails the rule: 0. Passes the target (a coordinate crosses or overshoots): 0. Needs 64 steps: 0.
- Steps: 0 -> 17688722, 1 -> 10072971, 2 -> 83236, 3 -> 1292, 4 -> 199. Max 4.
- Result == target point: 259147, all degenerate (r <= 4 EPS M). 0 non-degenerate.
- The old reproduction (r=0.001 at 3960) now passes in 1 step: (3713.1162, 3960.1685).
- Release at the step bound: the loop ends with a point that fails the rule, with no fallback. The unit then rests there with a live order. Not reachable: max 4 measured; my bound is about 30 (below).

## 2. Termination argument
- Monotone, never past the target, and a distance of 0 passes: correct. This is confirmed by the probe.
- "A step changes a coordinate by at least one quantum": not exact. When a step crosses a binade upward, c+q rounds and the step can be q/2. Example: c = 1-2^-24, t >= 1+EPS, result 1.0. Termination still holds.
- Step bound: the M-term goes in 1 to 2 steps for each component. The r-term 7.5ur needs at most about 15 steps at u r/2 a step, or about 30 with half steps. 64 is safe. The test assertion <= 25 is tighter than the worst case, but the measured value is 4.
- "Lands on target" for both coordinates: only for r <= ~4.8e-7 M. The goal is then the target centre. frame.rs:773-782 finds it occupied, so the unit rests beside the target and never fires. This is the same behaviour as item 4. Errata 1 states it.

## 3. Largest inward shift (3 M random in-map pairs each)
- Muster 64x48, r=4: 5.3e-6 world units (max 2 steps). Forge 64x64, r=8: 5.2e-6.
- 16384 map, r=4 or r=8: 1.38e-3 (max 1 step).

## 4. Range below the two radii
- I used attack.rs from 9ff4f9c7 (the pre-repair attack_goal) and the current combat.rs. On that tree the pin test a_range_below_the_two_radii... is ok. So the behaviour is pre-existing.
- choose_target gives the same result bit for bit (see the first review).
- Errata: content_m3_validation.rs:33 is correct. frame.rs:775-783 is correct. Options A and B are fair. "Range 1.0 fires at tick 27": NOT TRACED.

## 5. Faults
- F1: step_toward steps 2 quanta and keeps the landing test <= q. All 6 related tests stay green. The overshoot is real for a gap in (q, 2q), but no test has an odd gap: the walks of 100 and 200 quanta are even.
- F2: the walk moves x only. Red: random_rings and reviewed_case_classes (debug_assert after 64 steps). The Muster sweep and the bearing test stay green.
- Restore after each fault: cmp equal, git status empty.

## 6. Runs at 7be9364c
- cargo test -p turts_sim: 379 passed, 0 failed.
- mc_order_rows: 8/8. formula_ratchet: 7/7.
- Production dist_sq count in combat.rs: 2. turts_checks is not changed.
- Stream sha256: bcbf72c2...e370b, unchanged.
- probe_neutral: frame_chains=d0d1b1c572163424, muster_world=aa9db740578ed5c1 (off == on).

## Findings
- LOW, combat.rs test step_toward_reaches_...: F1 survives. Fix: add assert_eq!(step_toward(1.0_f32.next_down(), 1.0 + f32::EPSILON), 1.0) and a walk with an odd quantum count.
- LOW, combat.rs comment of weapon_stand_off: "at least one quantum" should be "at least half a quantum at a binade crossing". The 25-step estimate is about 30 in the worst case.
- LOW, stand_off_steps: in release, the bound returns a failing point. It is unreachable, but a fallback to `target` (distance 0 passes) would make the result exact unconditionally.
