# Fog arrival that does not read a hidden goal

Date: 2026-11-18. Track: Foundation (server authority, fog privacy, determinism). Status: built on
branch `codex/arrival-privacy-physical` (base `3938bd07`), not merged.

## Problem

Two units of two owners get a point order to the same point. Before this change the arrival
step (`crates/turts_sim/src/movement/arrival.rs`, `prepare_with_guide`) put all movers with an
equal target into one group, for all owners. The group compared the arrival times and changed
the target of the later unit before the two bodies touched. Under restricted sight (fog) the
owner of one unit thus saw the private goal of an enemy unit that it could not see.

Evidence on the base commit: `ordinary_enemy_order_does_not_change_arrival_before_contact`
(`crates/turts_sim/tests/group_movement.rs`) is red. In tick 1 the `MoveTarget` of our unit
changes from (50,50) to (49.4999,50) when a hidden enemy 2.0 away gets an order to (50,50).

A second leak has the same cause: the sweep order of all rows was the arrival time to the own
goal. When two bodies of two owners touch, the hidden distance to the enemy goal decided which
body moved first (`the_distance_to_a_hidden_enemy_goal_does_not_change_the_contact_tick`, red
on the base commit).

## Rule

The rule applies only when a visibility basis is installed (fog). A world with no basis keeps
the old code path; its results do not change.

**R1. A goal is private to its owner.** A target group holds bodies of one owner only. The
shared rule (earliest arrival keeps the point, the later bodies stop next to it, an exact tie
has no winner) stays as it is inside one owner.

**R2. Knowledge.** A mover M knows a body B of another owner when one of these is true:
(a) the visibility basis permits B for the owner of M; (b) B is in contact range of M
(`route::CONTACT_GAP`, the rule that the rest of the kernel already uses); (c) M and B touch
during this tick on their straight motions (`touches_this_tick`). Case (c) uses the motion of B
in this tick only. It makes the result free of the order in which the sweep moves the two
bodies in the tick of first contact.

**R3. Physical first arrival.** M has goal G and is in the arrival window of G. A known body B
is a contender when B moves in this tick, and B stands in the goal (the distance from B to G is
less than the two radii). M compares two times: its own distance to G divided by its speed, and
the distance from B to G divided by the speed of B. M reads the position, radius and speed of
B. M never reads the goal of B.

- B is earlier: B arrived first. M stops at a point next to G (ring distance: the two radii
  plus 0.0001). The stop is temporary (`arrival_yield`). Each tick decides it again. When B
  stops on G, the holder rule that the kernel already has makes the stop final. When B leaves,
  M gets its goal back.
- The times are equal (1e-6 s, the tolerance of the shared rule) and B moves toward G: there is
  no winner. M stops halfway: G plus the unit vector from B to M times half the sum of the radii.
  This stop is final, as in the shared rule. The other body makes the same decision from its
  own goal, so the two stops are symmetric.
- A body that does not move is not a contender. The holder rule and the closest-arrival rule
  own that case, as before.

**R4. Sweep order.** Under fog the rows are in physical order (position, radius, speed). The
order has no term that comes from a goal.

Unit ids, row order and slot numbers have no effect on a visible result (permutation tests).

## Limits of work and memory

`foreign_arrivals` makes one pass on all rows (at most four hash probes each) and keeps at most
16 known bodies for each mover that is in its arrival window. The list is filtered by knowledge
before the cut, so a hidden body cannot push a known body out. With no second owner, or no
mover in a window, the pass does no more than two scans of the rows.

## What a test must show

1. Paired worlds, ordinary orders: a hidden enemy order does not change our unit before contact.
2. Paired worlds: equal enemy bodies with different goals give equal results for our unit, also
   in the tick of contact.
3. First arrival keeps the point with no overlap: opposite, perpendicular and mixed radii.
4. A small time advantage (0.02 s, less than one tick) keeps the point under fog.
5. An exact tie has no winner, for each choice of unit ids, rows and slots.
6. A body that passes does not keep the point.
7. A hidden body near our goal changes nothing; the same body in sight does (the fixture is live).

## Known limits

1. R2(c) reads the motion of a hidden body for the tick in which it would touch M. When a third
   body stops B in that tick, M reacts one tick early to a body that it did not touch.
2. An exact tie with a known body that moves toward G but has another goal stops M at half the
   radii from G. The stop is final.
3. Three or more owners in one exact tie use a ring rule that is not tested.
4. The shared rule inside one owner still scans all rows (`approach_clear`, `yield_goal`,
   `tie_destinations`), also rows of hidden bodies. This is older than this change.
