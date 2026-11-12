# Errata and open items: weapon range (follow-up of `mp1/combat-fixes`)

Status of each item is stated. Nothing here changes content validation or a design decision.

## 1. Minimum weapon range: not decided

Fact: content validation accepts any finite positive weapon range
(`crates/turts_mod_runtime/src/content_m3_validation.rs:33`, `check_finite_positive(..., "weapon.range", ...)`).

Effect before the follow-up: a range far below the coordinate magnitude (below about 9.5e-7 times
the largest coordinate; example 0.001 at a coordinate of 3960) made the stand-off of an Attack
order cross the target, and the unit never fired.

Effect now (`weapon_stand_off`, `crates/turts_sim/src/economy/combat.rs`): the stand-off is exact by
construction. When the ring point fails `within_weapon_range`, each offset coordinate moves one
quantum toward the target until the rule holds, and the loop has a hard bound (`STAND_OFF_MAX_STEPS`).
The result is in range for every positive range, and it is never past the target. The reproduction and
27 case classes are tests (`weapon_stand_off_is_in_range_for_a_range_far_below_the_coordinates`,
`weapon_stand_off_is_in_range_for_the_reviewed_case_classes`).

So a minimum range relative to the map size is moot for the Attack approach. It stays a remark for
the content author: when a range is below the spacing of `f32` at the coordinate (of the order of
1e-7 times the coordinate), the stand-off can be the target point itself (a distance of 0 passes the
rule), which lies inside the target body. Status: needs decision only if a mod can author such a range; shipped
content cannot (ranges 4.0 and 8.0, maps up to 64).

## 2. Weapon range: centre to centre, or edge to edge? Open design item (design X2), for `game-designer`

Status: open. Needs decision. Not decided by this branch.

Fact today: the range is measured from the centre of the weapon unit to the centre of the target
(`within_weapon_range`, `combat.rs`: `origin.distance_squared(point) <= range * range`; the unit
positions are centres). The ring point of the approach is at a centre distance of exactly `range`
(`attack.rs:31-39`, `weapon_stand_off`).

Consequence: when `range` is below the sum of the radii of the attacker and the target, the ring
point lies inside the target body. The movement arrival test refuses a point that overlaps another
body (`crates/turts_sim/src/movement/frame.rs:775-783`, `occupied`, with the tolerance 1.0e-5).

What the unit does (pinned by the test
`a_range_below_the_two_radii_never_fires_and_the_unit_rests_beside_the_target`, radii 0.5 and 0.5,
sum 1.0, ranges 0.4 and 0.9, target at distance 5):
- the unit walks to a free slot beside the target, outside the range, and rests there (observed
  distance 1.5 for range 0.4 and 2.0 for range 0.9; the test does not pin these two numbers, it pins
  "at least the sum of the radii and above the range");
- the Attack order stays current, no failure is recorded (no `OutOfRange`, no `TargetGone`);
- the weapon never fires (400 ticks, target health unchanged).
At range 1.0 (equal to the sum) the unit stops at contact and fires (tick 27 in the probe of this
task; not pinned by a test).

Options for the decision (no choice is made here):
- A. Keep centre to centre. Add a content validation rule: a weapon range must be at least the sum of
  the largest radius of an attacker and of any target it can hit, or the Attack order fails with
  `OutOfRange` when the stand-off is inside the target body.
- B. Edge to edge. The rule becomes `centre distance <= range + radius(attacker) + radius(target)`.
  Every range in existing content then means a longer reach by the two radii (1.0 for the shipped
  units), so Muster balance and the frozen traces change. It needs both radii in
  `within_weapon_range`, in `choose_target` and in `grid.within` (a bigger reach).

Evidence: the pin test; `crates/turts_sim/src/movement/frame.rs:775-783`; `attack.rs:31-39`.
