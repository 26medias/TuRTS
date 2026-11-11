---
type: decision
date: 2026-11-11
tags: [turts, mc, ruling, gmu]
ai-first: true
source: claude
status: active
---

# O14: GMU records the second-order `Blocked` count as an MC limit; a slot check gates "slots reused"

## For future Claude
The CEO delegated STOP-GMU1 to the studio on 2026-11-11 ("Use your subagents to find the best option, that's why you have a team. I'm just the CEO, this doesn't seem to be a decision for me."). Three seats gave the same answer. The orchestrator ruled and wrote design revision 15.

## Problem
Design row GMU said: a second order of another group to the same point ends with 0 overlaps and 0 `Blocked` units. On the real `muster` server, one unit of 20 ends `Blocked` on open ground (40, 30) and three at the clamped corner. The code follows design 6.3 to 6.5 as written (RUN117 trace). No tested rule variant gives 0 `Blocked` in a sweep of 25 points and 2 orders.

## Lenses
- simulation-core-engineer (RUN117): traced the cause; tested three variants; recommends the record.
- technical-director (RUN119): the row is wrong, the rules are right. The model case C2 proved one geometry (a second group from the west). Rules 6.4 and 6.5 permit `Blocked` behind resting units; row GMA already records this for cases F and I; GM07 (push) is `later:M4b`; PF04 is "recovers or gives up visibly". It adds a native slot check so that the plant "slots reused" still has a real red.
- player (RUN120): a small flaw; the unit stops at the edge of the crowd; the feedback is enough. Conditions: no overlaps stays gated; a first order keeps 0 `Blocked`; the rest position of a `Blocked` unit is recorded and bounded; M4b gates the case by name.

## Ruling
1. Row GMU (design revision 15): the same-point case is fixed at (40, 30) on `muster`, troopers 21 to 40 first, then 1 to 20. Gated: the first order has 0 `Blocked`; 0 overlaps at rest; each unit of both orders, `Done` or `Blocked`, rests within R_near = 4.58 of the point; the deadline; in the native run, each slot of the second order is at least s - 0.00001 from each slot of the first order. Recorded and not gated: the `Blocked` count of the second order and the rest position of each such unit, in `docs/measurements/mc.md`.
2. The native model case C2 (`second_group_same_point`) and row GM40 keep 0 `Blocked` as gates.
3. No engine rule changes in MC. M4b (push, chokes) owns the limit and gates this case at 0 `Blocked`.
4. Fault cycle P6 (slots reused) takes its red from the slot check.

## Rejected
- Inclusive "within s" (6.3): C2 and two follow tests fail; 5 `Blocked` units remain in the sweep.
- Away-side deflection (6.4): first-order `Blocked` units rise from 31 to 93.
- Open-side slot claim (6.5): no run supports it.
- A pool that skips slots behind the block: that is routing round units, M4 and M4b scope.
- Choosing a point where the build passes with no record: it hides the limit.

## Follow-up debt
- M4b exit rows: the `muster` reinforcement case at 0 `Blocked` (task in `vault/Tasks.md`).
- Player request, not in MC: one grouped notice and one error sound when several units of one order fail after acceptance (design 9.2 gives one notice per unit from `last_failure`). Owner: ux-designer, at MP1.
- Play-test: in PT1 or PT2 the CEO reinforces one point at least once in `forge`; the result goes in the play log.
- Task 22 instrument notes from RUN119: the GMA crossing test starts troopers 1 to 20 behind the idle rows (it mixes case I into the crossing); the comment at `gmu_same_point.rs:174-176` names the wrong cause.

## Evidence
`target/mc-evidence/t22-run112/stop-gmu-same-point/`, `target/mc-evidence/t22-run117-stop/`, labels `t22-run117-*`; design lines 604-684 and the GMU row; `crates/turts_sim/tests/group_movement.rs:1365-1388`; `docs/research/mc/gate1-sim-probe/gm40_hard.py:106-115`.
