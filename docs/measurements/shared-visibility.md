# Shared visibility measurements

Status: SDK16 acceptance is open. The accepted launcher still runs SDK15.
Evidence root: `/home/julien/Projects/TuRTS/target/mc-evidence/`.

## Original SV7 timing contract: failed diagnostics

The workload uses 1,000 units and two human replicas on four selected CPUs: 0,2,4,6.
Each diagnostic uses 100 warm-up intervals and 10 measured intervals of 20 ticks.
These short runs are not the required 1,000-interval acceptance run.
The original ceiling is p99 <=5 ms of summed server and client work per 20-tick interval.
Every result below fails that ceiling. Preserve those results when evaluating any later contract amendment.

| Build step | Median sum, ms | Diagnostic p99 sum, ms | Evidence |
| --- | ---: | ---: | --- |
| Exact phase baseline | 86.093 | 93.371 | `shared-visibility/sv7-diagnostic-exact` |
| Stale-only memory, borrowed capture, sparse client updates | 57.676 | 60.176 | `shared-visibility/sv7-diagnostic-opt1` |
| Typed capture, circle deltas, flat wire diff | 55.835 | 68.875 | `shared-visibility/sv7-diagnostic-opt2` |
| Cached permissions, exact circle kernel, reused unit names | 46.640 | 48.963 | `shared-visibility/sv7-diagnostic-opt3` |

The last diagnostic reports server p99 28.512 ms and client p99 20.929 ms per interval.
It does not include the complete simulation in its server total.
It cannot establish full server tick capacity or cloud hosting cost.
Samples retain movement, combat, harvesting, hide/reveal, wire, and retained-memory counters.
No unit, sight source, visibility transition, or model detail was removed.

## Real browser observations

The first nine-case campaign stopped after a near-camera draw-count assertion failed.
The assertion counted offscreen fallback meshes as visible detail loss.
The corrected driver checks each visible model against its actual GLB material draw groups.
A fresh final-build campaign remains required.

| Case | Frames per second | Frame p95 | Observation | Evidence |
| --- | ---: | ---: | --- | --- |
| Fog, 400 mixed detailed units, normal camera | 59.603 | 16.7 ms | 400 visible; High detail; 932 shared groups; no ordinary fallback groups | `shared-visibility-sv7-browser-first/fog400-normal/` |
| Fog, same 400 units, near camera | 60.003 | Recorded in artifact | 296 visible; 104 offscreen fallback meshes; old driver assertion failed | `shared-visibility-sv7-browser-first/fog400-near/` |

These are intermediate-build observations. They are not final SDK16 acceptance.
The remaining camera and baseline cases are pending.

## Transport

`shared-visibility/pipeline/` records the frozen 50 ms and 100 ms activation-ACK cases.
Each delivers all 1,301 bases in order, including 1,200 measured movement ticks.
Maximum oldest transaction age is 60 ms and 95 ms respectively.
Maximum successor backlog is 5,069 and 5,071 bytes. No routine restart occurs.
A separate slow-viewer test proves restart/disconnect without stopping the healthy viewer.
`shared-visibility-outbound-reuse-failure-tests.log` now passes all 11 outbound tests,
including both RTT cases, cancellation, ring overflow, and fatal sequence exhaustion.

## Remaining measurement work

The independent audit found no product derivation for the original 5 ms summed-second allocation.
A deadline-based amendment is under Anvil review. It has no acceptance result yet.
Keep the workload and original failed evidence unchanged.
Run fresh full-server and per-client timing after the amended contract and plan are approved.
Run all nine browser cases on the final build. Record final source identity and machine details.
