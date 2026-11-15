# Public bot phase measurement

Status: the first complete run failed. Optimization is in progress.
This record makes no performance acceptance claim for the current candidate.

The BF8 budget is 5 ms at p99 for a complete active bot phase.
Each sample includes observation and decisions, shared admission, ordinary command application, and all deferred receipt publication and participant-result work across 20 simulation ticks.
No baseline cost is subtracted.

The local CPU is an Intel Core i9-14900K. The run used logical CPUs 0, 2, 4, and 6.
These are four separate physical P cores. The build used the locked Rust toolchain and the release profile.
The source commit was `1fff0047`.
Machine and compiler details are in `target/mc-evidence/public-bots/bf8-measured-machine.txt`.

The workload has 1,000 units and two bots. The simulation runs at 20 ticks per second.
Both public profiles decide once per second with the frozen default operator limits.
Each bot owns 100 gatherers, 300 mobile combatants, 50 producers, 25 constructors, and 25 static providers.
There are 50 independent authored production goals per bot.
All 100 mixed Balance/inventory COT jobs must be active before samples begin.
High authored test health and long job durations keep the unit mix and work active.
No jobs are injected. The stock controllers submit ordinary commands.

The preflight passed after 139 warm-up intervals. Three more intervals retained all 100 jobs, collected 797 items, and fired 1,023 shots.
The complete run then collected 1,000 steady-state samples after the same readiness check.

| Measured part | p50 | p95 | p99 | Maximum |
| --- | ---: | ---: | ---: | ---: |
| Complete bot phase | 23.550 ms | 24.385 ms | 25.079 ms | 28.645 ms |
| Host, shared admission, and command application | 12.277 ms | 12.779 ms | 13.331 ms | 17.406 ms |
| Deferred receipt publication | 11.220 ms | 11.715 ms | 12.058 ms | 12.784 ms |
| Participant results | 0.033 ms | 0.037 ms | 0.039 ms | 0.050 ms |
| Full simulation across 20 ticks | 970.646 ms | 1000.593 ms | 1021.534 ms | 1054.561 ms |

The phase parts have separate percentile distributions. Their p99 values must not be added.
The run kept exactly 1,000 units. During measured phases it collected 135,526 items and fired 221,357 shots.

Evidence is under `/home/julien/Projects/TuRTS/target/mc-evidence/public-bots/`:

- `bf8-final/bf8-measurement.json`: workload, hashes, limits, counters, and timing distributions.
- `bf8-final.log`: required run and failed budget assertion.
- `bf8-preflight/bf8-preflight.json`: workload readiness proof.
- `perf-baseline/perf.data`: 30 seconds of user-mode sampling at the same ready workload.
- `perf-baseline/self.txt` and `children.txt`: allocation, parsing, and simulation hotspots. Rust caller unwinding was incomplete.

A separate 40-sample diagnostic uses the same readiness and workload. It adds test-only subphase timers.
It does not replace the required 1,000-sample acceptance run.
Other machines, maximum unit counts, and browser rendering performance are not established by this native measurement.

## Optimization diagnostics

These short runs use the same readiness check and 40 steady phases.
They are diagnostics, not performance acceptance.

| Diagnostic | Complete phase p99 | Role counting, mean per phase | Account rows, mean per phase | Receipt capture, mean per phase |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 25.081 ms | 10.116 ms | 4.978 ms | 3.602 ms |
| Immutable commands, actual-inventory lookup, allocation-free role matching | 13.918 ms | 0.826 ms | 3.683 ms | 3.660 ms |

The second diagnostic is `bf8-diagnostic-opt2/bf8-diagnostic-40.json` under the evidence directory above.
It predates shared immutable results/effects, borrowed payment validation, and sorted borrowed account rows.
Those later changes require a new measurement.
