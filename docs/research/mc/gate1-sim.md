VERDICT=fail FINDINGS=8 HIGH=2 RUNS=10 NOT_CHECKED=5

Status: DRAFT; D65 and D73 lines Decided, designs Proposed; K-calls open. Full evidence: /tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/gate1-sim-evidence.md. Probes: /tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/probe/

## Findings

1. High. design:452-513; `crates/turts_server/src/economylog.rs:578-663`. The design names no phase, no capture, and no change to the validator. The validator replays the M1 straight line and requires the orders to stay the same. Avoidance, waypoint pops, slot changes, and a queued gather fail it. Fix: add `SimSet::Orders`. Put each audited write in `capture_operation`. Add `UnitOrders` to `AuditState`. Make the validator run the sim step.
2. High. design:490-503 (K17, GM40). The rules do not show that GM40 can pass. s−0.1=0.95 < 2r=1.0, and the separation reach 1.1 > s. Idle units never yield. With exactly N slots, "nearest free slot" has no pool. Probe: 25/40 Blocked and 37 overlaps at rest. Fix: snap to the slot on arrival, use a reach ≤ s, let the group yield, keep the unit order in the assignment, and assert 0 Blocked.
3. Medium. design:482-486. A second order to the same point takes the same slots: 103 overlaps in the probe. Fix: skip the slots of resting units and of live moves.
4. Medium. design:507-513. Spreading is O(N²). 8,192 units take 68 ms, and the tick at 20 Hz is 50 ms. Combat is O(W·E) (`combat.rs:89-199`). Fix: an O(N log N) assignment or capped work (S:13.6).
5. Medium. K6 (design:432). The clamp keeps NaN, and it accepts +inf. 5.2 drops `target_not_finite` (`command.rs:276`). Fix: check that the target is finite first.
6. Medium. design:365-383. Gather never completes, so an order queued after it never runs. A queued order that fails when it starts has no outcome. Fix: give each behaviour a completion rule and a failure record.
7. Medium. REG1. `economy_ordering.rs:120-175,236-260` test all-or-nothing, which D65 forbids. Fix: list these tests to invert.
8. Low. 5.3, 6.5. The limits are in ticks, but the tick rate is Open (D18). The design does not say which part enforces the rate limit. Fix: use seconds and name the part.

K5: only `content_m3.rs:339` uses the rule. K8: the sim does not change. Section 14: ORD2 is red on HEAD. GMU, GMA and GM40 can go red; GM40 green is not shown.

## What I ran

| command | claim | refuting result | observed |
|---|---|---|---|
| grep design | 6.x captures its writes | a match | none |
| read economylog | the validator allows 6.4 | curved paths allowed | straight line only |
| gm40.py ×3 | K17 holds | 0 overlaps | 32-37 |
| gm40 variants | a fix passes | 0 overlaps | 5, 10 |
| gm40 C2 | 6.3 handles a second group | 0 overlaps | 103 |
| cost.rs ×2 | 6.7 O(N) | <5 ms | 68 ms |
| gma.py | GMA cannot go red | GREEN | RED |
| clamp.rs | K6 is safe | NaN refused | NaN kept |
| grep MoveTo | 1.6 lists all | none more | 2 more |
| grep static | K5 cost | other users | 1 test |

## Not checked

The ECS avoidance cost on the D57 VM. Determinism across processes. The lightyear message size (NOT TRACED). K2 against S:180. The frozen M2 stream under avoidance (NOT TRACED).

## What I changed

none

## Open questions for the orchestrator

1. Needs decision. (a) The validator runs the sim movement step. (b) Movement becomes one captured operation for each tick. Evidence: economylog.rs:578-663.
2. Do the separation numbers go in the spec, or in a prototype task that GM40 gates?
