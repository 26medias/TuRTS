

## C387: repair the engine for the GMU same-point claim; do not narrow the acceptance row (2026-11-11)

Conflict: design section 14 row GMU (design line 1245) says: a second order of another group to the same point ends with 0 overlaps and 0 Blocked units. RUN112 ran this on the real muster server. Open ground (40, 30), troopers 21 to 40 at tick 1 and 1 to 20 at tick 300: unit 19 ends `599,19,Blocked,blocked` at (37.84, 28.19); its slot is a pocket closed by six resting units 1.1 away. North-east corner (63.5, 47.5): units 22, 39 and 40 end Blocked. The native test second_group_same_point passes on another geometry. Cause in the movement code: NOT TRACED. Evidence: target/mc-evidence/t22-run112/stop-gmu-same-point.

Choice: the acceptance row stays as written (a change of the row is a CEO stop item, and nothing shows the row is wrong). The engine is repaired by the owner seat of the code, simulation-core-engineer, with the root cause traced first. The repair must stay inside the rules of design 6.3 to 6.5 (for example the reserved-cell skip of step 2 and the slot choice of a stalled unit); it adds no key, no step and no player-facing rule, and it changes no frozen value. If no repair exists inside those rules, the run stops and Root reports to the CEO. After the repair: the Task 6 tests, G-REF for the changed files, DET1, all movement fault cycles (P1 to P6, P10), G-ACC, and the tick-cost record run again, because they judge the changed code. The final Task 22 review is a fresh agent that did not write the repair.

Evidence: design lines 604-626, 663-674, 1245; RUN112 reply; brief section 4 lines 53 and 55.

Alternative not taken: narrow the GMU wording to the geometry that passes. That is a changed acceptance row and it hides a defect that a player can meet (a unit that gives up on open ground).

Global117/170 after RUN117 registration.
