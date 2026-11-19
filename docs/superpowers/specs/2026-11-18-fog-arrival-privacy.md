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
(`route::CONTACT_GAP`, the rule that the rest of the kernel already uses). There is no
prediction of contact. (The first build predicted a touch from the target of B. An independent
review showed that this read a hidden order, and that it was not symmetric at float precision.)

**R3. Physical first arrival, decided after the sweep.** `arrival::settle_foreign` runs at the
end of each tick. For each body it has two positions: before the sweep and after the sweep.
M has goal G (the point of its order) and is in the arrival window of G. For a known body B it
reads the radius, the speed and the two positions. It never reads the goal, the target or the
order state of B.

- A *contender* is a known body of another owner that is in the arrival window of G and came
  nearer to G in this tick (a fact, from the two positions). A body in the window that moved
  in this tick and came into contact range of M in this tick is a contender too: in the tick of first contact the
  sweep stops the second body or turns it to the side, so its motion in that tick is not a
  measure of where it was going.
- *Start.* M compares two times from the positions before the sweep: its own distance to G
  divided by its speed, and the same for the contender. The sweep order of the contact tick
  cannot change these positions.
  - The contender is earlier: it arrived first. M gets a temporary stop (`arrival_yield`) on
    the ring round G (the two radii plus 0.0001).
  - The times are equal (1e-6 s, the tolerance of the shared rule): no winner. M gets a final
    stop halfway: G plus the unit vector from B to M times half the sum of the radii. The
    other body makes the same decision from its own goal, so the two stops are symmetric.
  - A contender that was in contact range of M before the sweep starts nothing. That contest
    is already decided; positions after a contact are not a fair measure of time.
- *Keep.* The next tick keeps a temporary stop while a known body of another owner that can
  move is in the arrival window of G (`keep_foreign_yields`, positions only). At the end of
  that tick the stop stays while there is a contender, or a known body of another owner that
  moves in the goal. If not, M gets its goal back. When the first arrival stops on G, the
  holder rule that the kernel already has makes the stop final.
- A stop that the shared rule of the owner gave in this tick is not changed.

**R4. Sweep order.** Under fog the rows are in physical order (position, radius, speed). The
order has no term that comes from a goal.

Unit ids, row order and slot numbers have no effect on a visible result (permutation tests).

## Limits of work and memory

`settle_foreign` returns after one scan when all bodies have one owner. If not, it makes one
pass on all rows with a constant number of hash probes each, and one binary search for each
row. Each mover in its arrival window keeps at most 16 known bodies of other owners and at
most 16 resting bodies of its owner. The lists are filtered by knowledge and by owner before
the cut, so a hidden body or an own body cannot push a known body of another owner out.
`keep_foreign_yields` runs only when a temporary stop from the tick before exists.

The bound is not linear in one case: each row examines each mover whose goal is in the cells
near it. Many movers with goals in one cell and many bodies near that cell give (movers ×
bodies) distance checks. The shared rule of the base build has the same shape. The review
measured 2,000 bodies of two owners, each with its own goal one step away (debug build, 3
ticks, machine under load): 15.8 s with fog, 139.6 s with no basis.

## What a test must show

1. Paired worlds, ordinary orders: a hidden enemy order does not change our unit before contact.
2. Paired worlds: equal enemy bodies with different goals give equal results for our unit, also
   in the tick of contact.
3. Paired worlds: a hidden order that cannot move its body changes nothing.
4. First arrival keeps the point with no overlap: opposite, perpendicular and mixed radii.
5. A small time advantage (0.02 s, less than one tick) keeps the point under fog.
6. An exact tie has no winner, for each choice of unit ids, rows and slots, and for 900 cases
   of distance, radius, speed, axis and sight.
7. A body that passes does not keep the point.
8. A hidden body near our goal changes nothing; the same body in sight does (the fixture is live).

## Known limits

1. The first arrival is decided when knowledge starts. Two bodies with no sight that touch in
   the tick in which one of them reaches G: the sweep order of that tick decides who is on G.
   The order is physical (position), not a unit id.
2. Two bodies of two owners that are in contact before they come to the arrival window have no
   contest. The body that reaches G keeps it; the closest-arrival rule of the base build places
   the other body.
3. A body with no sight that gives way and moves out of contact range does not know the first
   arrival any more. It gets its goal back, touches again and gives way again, until the first
   arrival stops on G. The end state is correct.
4. A body that the sweep stops with no motion at all in the tick of first contact is not a
   contender in that tick. The 900 tie cases on one axis show no such case.
5. An exact tie at an angle to the axes can have a winner when float error makes the two times
   differ by more than 1e-6 s. The base build with no basis has the same kind of limit, at a
   lower rate with sight. The review measured 340 ties at general angles: 33 have a winner
   with no basis, 32 under fog with no sight, 67 under fog with mutual sight. The cause of
   the higher rate with sight is not traced.
6. Asymmetric sight: when only one owner sees the other body, only that owner decides before
   contact. The two results are then not symmetric.
7. With sight, the closest-arrival rule of the base build gives a body another point as soon as
   a visible body is in its goal disc, also before the arrival window. The base build with no
   basis does the same. `settle_foreign` then uses the point of the order as G.
8. Three or more owners in one exact tie use a ring rule that is not tested.
