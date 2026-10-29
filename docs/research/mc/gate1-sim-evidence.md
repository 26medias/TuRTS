VERDICT=fail FINDINGS=8 HIGH=2 RUNS=10 NOT_CHECKED=5

Artefact: docs/superpowers/specs/2026-10-29-mc-player-control-design.md (DRAFT; D65 and D73 Decided lines, designs Proposed; K-calls open for the CEO). Probes: /tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/probe/

## Findings

1. design:452-513, crates/turts_server/src/economylog.rs:578-663, High. No phase, no capture, no validator plan. The validator replays the M1 straight-line step and requires orders unchanged. Avoidance (6.4), waypoint pops (4.5), slot changes (6.5), attack chase, and a queued gather that writes `EconomyOrders` (6.1) all fail it ("movement differs from its prior target and speed"). Fix: name `SimSet::Orders` between Commands and Movement. Run each audited write in `capture_operation` (one unit per operation). Add `UnitOrders` to `AuditState`. Make the validator call the sim step. Add a red muster economy-log test.
2. design:490-503, K17, GM40, High. The rules cannot be shown to pass GM40. Arithmetic: s−2·0.05=0.95 < 2r=1.0, so two arrived neighbours can overlap. Separation reach 1.1 > s=1.05. Nearest-first greedy fills the near side, and idle units never yield. "Nearest free slot" has no pool when slots = N. Separation size and weight are not given. Probe: 25/40 Blocked, 37 overlaps at rest, 1 beyond R_near. With hard projection: 11 Blocked, 5 overlaps. With order-keeping assignment: 2 Blocked, 10 overlaps. Fix: snap to the slot on arrival, or tolerance < 0.025. Separation reach ≤ s. Same-group resting units yield. Use an order-keeping assignment and a defined spare pool. GM40 asserts 0 Blocked.
3. design:482-486, Medium. A second order to the same point reuses the same first-N slots. Only static footprints are dropped. Probe C2: 103 overlaps, 26 Blocked. Envelopes of one group that arrive in different ticks (K4 split) get the same defect. Fix: drop slots of resting units and live move slots.
4. design:507-513, 1.7 item 9, Medium. Greedy spreading is O(N²). Probe: 4,096 units = 12.7 ms, 8,192 = 68 ms, one thread, i9-14900K. The tick at 20 Hz is 50 ms. 5.3 allows 8,192 per slot per tick. Combat is O(weapons × entities) (`combat.rs:89-199`), and K5 adds weapons. Fix: O(N log N) assignment, or capped work with a pending state (S:13.6). Record spreading and combat at 1,000 and 5,000 units, not only avoidance.
5. design:432 (K6), 6.6, Medium. The clamp keeps NaN as NaN and accepts +inf (probe). The 5.2 checks drop `target_not_finite` (`command.rs:276`). Fix: refuse non-finite targets before the clamp.
6. design:365-383, Medium. Gather never completes, so an order queued after it never runs. A queued gather that fails at run time (depleted node) has no outcome after `accepted`. No rule for a static weapon whose target leaves range. Fix: give each behaviour a completion rule and a run-time failure record.
7. design:1.6, REG1, Medium. All-or-nothing tests contradict D65: `crates/turts_pack/tests/economy_ordering.rs:120-175,236-260` and `crates/turts_sim/tests/command.rs:233`. The design lists only the last. Fix: list each test to invert.
8. design:5.3, 6.5, Low. Limits are in ticks, but the tick rate is Open (D18). The rate-limit owner (sim or server) is not named, so replay content (invariant 15) is open. Fix: use seconds and name the owner.

Answers: K5. Only `content_m3.rs:339` and M3D:139 (no reason) depend on the rule. `choose_target` reads `Position` each tick (`combat.rs:165-170`). An inserter keeps its boot binding by unit id (`boot_m3.rs:187-199`), as mobile gatherers do today. K8: no sim change. The records exist (`economy.rs:83-86`, `messages.rs:224-259`). Section 14: ORD2 is red on HEAD. GMU and GMA go red. GM40 goes red, but green is not shown (finding 2). Its determinism half names no red build; mutant: HashMap neighbour order.

## What I ran

| command | claim | refuting result | observed |
|---|---|---|---|
| grep design: capture, finite, economylog, receiver | 6.x names a transaction and a finite check | a match | none |
| read economylog.rs:578-711 | the audit permits the 6.4 motion | non-straight motion accepted | straight line only |
| gm40.py (sep 0.1/0.0/0.04) | K17 holds, 40 units | 0 overlaps, 0 Blocked | 37/35/32 overlaps; ≥20 Blocked |
| gm40_hard.py, gm40_formation.py | a fix variant passes | 0 overlaps | 5 and 10 overlaps |
| gm40.py case C2 | 6.3 handles a second group | 0 overlaps | 103 |
| cost.rs ×2 | 6.7 O(N) is the cost | spread 8,192 < 5 ms | 68-70 ms |
| gma.py | GMA cannot go red with avoidance off | GREEN | RED (63 pairs) |
| clamp.rs | the K6 clamp is safe | NaN refused | NaN→NaN, inf→31.5 |
| grep MoveTo unit lists | 1.6 lists every all-or-nothing test | none more | economy_ordering.rs:144,242 |
| grep "weapons must be static" | K5 cost | other users | one test |

## Not checked

Real ECS avoidance cost (`iter_entities` per unit) and the D57 VM. Rust determinism across processes. The lightyear message size (NOT TRACED). The K2 behaviour set against S:180. Whether avoidance changes the frozen M2 stream (NOT TRACED; geometry suggests no).

## What I changed: none

## Open questions for the orchestrator

1. Needs decision. Option (a): the economy-log validator runs the sim movement step. Option (b): movement becomes a captured operation per tick. Evidence: economylog.rs:578-663.
2. Do the separation parameters go in the spec, or in a prototype task gated by GM40?
