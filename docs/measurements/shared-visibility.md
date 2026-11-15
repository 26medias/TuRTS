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


## Deadline contract v2 diagnostics

The same reviewer approved the correction before these fresh runs.
The spec required one turn and one focused recheck. The plan passed one turn.
The correction preserves the original failed v1 artifacts and their ceiling.
It measures full server service against the 50 ms authored tick period, with a 40 ms p99 headroom target.
Each client also has per-tick and whole-application-batch service bounds.
`shared-visibility/v2-machine.json` records the machine, release profile, compiler, affinity, and source for the grid diagnostic.
The following runs still have only 10 measured intervals. Neither is acceptance.

| Source | Full-server p99, ms | Maximum, ms | Maximum debt, ms | Client tick p99, ms | Result | Evidence |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `a7171e2a` | 72.699 | 74.324 | 24.519 | 4.122 / 4.012 | FAIL server deadlines | `shared-visibility/sv7-v2-diagnostic-first` |
| `6f502803` | 43.659 | 46.658 | 0 | 2.721 / 2.738 | FAIL 40 ms server p99; clients pass | `shared-visibility/sv7-v2-diagnostic-grid` |

The second run uses packed integer grid hashing, one physical neighbor sort, and cached immutable combat candidate metadata.
Grid oracle queries retain exact membership and canonical ordering. Physical arbitration keeps its existing spatial order.
Live health remains uncached so an earlier same-tick kill still changes later targeting.
Both runs include actual registered transport receive/send costs and drain reliable credits, local packets, and activation ACKs.
No CPU work remains outside the final authored step's deadline sample.
The complete simulation suite passes 407 tests after these changes, with one ignored measurement.
The next optimization removes sorting only from pure Boolean collision/range checks.
It preserves the same candidate set and each all/any predicate. Source `f87c8b66` still fails: server p99 43.984 ms, maximum45.583 ms, zero debt. Client checks pass. Evidence: `shared-visibility/sv7-v2-diagnostic-predicates`.


A fresh CPU profile of `f87c8b66` identifies physical neighbor sorting as at least 26.38 percent of sampled CPU.
Candidate collection accounts for 16.44 percent. Coordinate hashing is no longer a leading cost.
Profile evidence: `shared-visibility/perf-v2-predicates/` includes the copied driver, SHA256, raw samples, and flat symbol reports.
The profiled run is diagnostic only. Its timing includes profiler overhead and cannot supply acceptance.
The next fix avoids physical sorting for Boolean occupancy checks and caches exact float total-order keys for consumers that need spatial order.


Source `d76cdcc1` passes the short v2 diagnostic after caching exact coordinate ordering and skipping pure occupancy sorts.
Full-server p99 is 33.763 ms, maximum36.963 ms, with zero maximum and final debt.
Client tick p99 is4.086/4.048 ms. Whole-batch p99 is0.736/0.670 ms.
Evidence: `shared-visibility/sv7-v2-diagnostic-spatial-cache`.
The complete simulation suite passes409 tests with one ignored measurement: `shared-visibility-sim-spatial-cache-tests.log`.
These are10 measured intervals. The full1,000-interval three-mode acceptance remains required.
