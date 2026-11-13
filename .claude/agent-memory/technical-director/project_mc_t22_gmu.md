---
name: mc-t22-gmu-facts
description: Verified 2026-11-11 facts on the MC GMU same-point red (RUN119): idle trooper wall, sweep totals are unit sums, C2 geometry, P6 lattice, PF04/GM10 text
metadata:
  type: project
---

Verified 2026-11-11 in lane TuRTS-mc-t22 at da3e508 (RUN119, answer mode):
- muster field.ron: troopers 1-40 start in rows of 8 at spacing 1.1 = s (rows y 18..22.4,
  x 25..32.7). An order to troopers 1-20 must pass the idle rows 21-40; idle units do not move
  for others in MC (design 6.4 :659-660). That is the cause of all 31 "first-order Blocked"
  units in the RUN117 sweep: every one is in a north_first=false case (sweep-strict command.log).
- zz_probe_sweep.rs totals are sums of Blocked units over 50 cases, not counts of cases.
- Native C2 (group_movement.rs:1364) = model C2 geometry: first 40 from (10,10) block, second
  40 of another slot from a (10,30) block west of P (40,30). Different from muster GMU geometry.
- Pool lattice is anchored at the target point (movement.rs:296-325), so with plant P6
  (reservations ignored) the second pool's primary slots coincide with the first group's slots.
- PF04 text: "A blocked unit recovers or gives up visibly." GM10: "Move to the nearest reachable
  point. Do not freeze." (docs/research/review-2026-10/inventory-engine.md:145,362). GM07 push is
  later:M4b (design :1546). Neither requires 0 Blocked.
- MoveProgress (pub slot) is readable from sim tests (group_movement.rs:268,460).

**Why:** GMU row repair (MC Task 22) and M4b debt depend on these.
**How to apply:** Re-check line numbers; Task 22 and a design revision 15 may move them.
