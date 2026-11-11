

## STOP-GMU1: required CEO ruling on the GMU second-order same-point clause (2026-11-11)

Design section 14 row GMU (docs/superpowers/specs/2026-10-29-mc-player-control-design.md line 1245): "A second order of another group to the same point ends with 0 overlaps and 0 Blocked units". RUN112 showed it red on the real muster server (open ground: unit 19; corner: units 22, 39, 40). RUN117 traced the open-ground cause and found that the code follows design 6.3 to 6.5 as written; no variant inside those rules reached 0 Blocked across a sweep of 25 points and 2 orders; each variant that passes S1 breaks another gated case or needs a rule change. A change of a design rule that alters unit behaviour and a change of an acceptance row are both on the stop list (brief section 4 line 55). Root does not decide.

Options for the CEO:
1. Rule 6.3 step 2 inclusive ("within s" skips a slot at exactly s) with an exception for the follow target. Evidence: S1 passes; native C2 still fails (unit 80); 5 of 50 sweep cases fail. Needs more engine work and a model run.
2. Rule 6.4: deflect away from the neighbour, not always to the right. Evidence: S1 and C2 pass; first-order Blocked cases rise from 31 to 93 of the sweep. Worse in general.
3. Rule 6.5: a stalled unit claims only slots on its open side. NOT TESTED; needs a model run and a new gate.
4. Row GMU: keep "0 overlaps" gated; record the Blocked count of the second order as an MC limit, not gated, owned by M4/M4b (routes, push), as row GMA already does for cases F and I. The native model case C2 stays a gated test. RUN117 recommends this; Root agrees.

Until the ruling: Task 22 is held at da3e508 with its instruments, nine closed fault cycles and records. P6, G-ACC, the review and the merge wait. Tasks 23 and 24 need Task 22.

## C388: Task 21 takes the lease now and merges before Task 22 (2026-11-11)

Conflict: C383 said Task 22 merges first and Task 21 rebases on it. Task 22 is now held on STOP-GMU1 for a CEO ruling. Task 21 is source-ready after an independent source review and repair, and its Needs (14, 16, 17, 18, 20) are merged.

Choice: the compiler cache moves from target/mc-t22 to target/mc-t21 and RUN118 runs the Task 21 runtime now. Task 21 merges first when its gates, plants and independent runtime review pass; Task 22 then rebases on it (shared files: scripts/mc-browser.sh, tests/browser/harness/params-contract.test.js, harness/run.js; RUN115 found the merge of both clean). The wave push and CI read follow each merge. No gate or review is dropped.

Evidence: plan lines 46 and 674; board C383; RUN115 answer 8.

Alternative not taken: hold the lease idle until the CEO rules on STOP-GMU1.

Global118/170 after RUN118 registration.
