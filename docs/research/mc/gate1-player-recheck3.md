VERDICT=pass-with-changes FINDINGS=1 HIGH=0 RUNS=3 NOT_CHECKED=1

## Items

- B1: fixed. Guide page 5 "Controls" names queue, groups, centre, stop, F10 and F1 (D:1059-1063). The groups row is in the forge HUD (D:1064).
- M1: fixed. `Pending` is in the list (D:449). The panels show "Order pending" (D:610-611).

## Blind walk of the D:1210 goals

Machine names: hover tooltip (D:813). Research, pause: pages 2 and 3, card. Stronger: damage (D:754). Refusal: I try a right click with the turret selected. Turret range: range circle. Group, centre, two points, stop: page 5. Scroll speed, Stop key: menu (D:914). I found every action. No wait over 60 s, except the start "0/10" (NOT TRACED).

## New findings

L1. D:763. Low. Each `Bindings` item shows the action "name". The design gives no player label, so I may read `group_assign_1`. Nine groups give 27 rows. Fix: a player label, and one row for each group action.

## What I ran

| run | claim | refute | observed |
| --- | --- | --- | --- |
| read D:447-451, 606-614, 763, 1052-1066 | B1, M1 fixed | a key or text missing | present |
| blind walk of D:1210 with guide and HUD only | every action found | a goal with no hint | none |
| grep "label" | D:763 player words | a label field | none |

## What I changed

none
