# Terrain work orders

Status: implementation and acceptance in progress. This milestone is not accepted.
The SDK16 browser launchers remain on the accepted main build.

API 0.2.17 adds authored plateau terrain, public geological generation rules, movement classes, excavation/fill tools, physical cargo and explicit unloading.
Protocol 10 installs permitted terrain, work, contacts and accounts on one atomic basis.
The server keeps chosen buried columns and seed provenance outside public packages and HTTP roots.
Quarry uses physical cargo. Terrain Stockyard uses renamed tools and slot balances through the same API.

Evidence root: `/home/julien/Projects/TuRTS/target/mc-evidence/`.
Paths below are relative to that root. Earlier failed logs remain beside the passing runs.

| Row | Current evidence | Remaining checks |
| --- | --- | --- |
| TW1 | Public schema/runtime checks, the native compatibility campaign and process acceptance pass. See the logs below. | Final browser staging of both public packages. |
| TW2 | Lifecycle, atomic transport and nine independent audit cases pass. See `terrain-work-orders/review.md`. | Final workload conservation and browser unloading. |
| TW3 | Physical claim, allocator inversion, same-seed replay and retained-order audit checks pass. | Final frozen workload and browser scenarios. |
| TW4 | `terrain-work-orders/sim/navigation-pause-final.log`; `terrain-work-orders/pure-kernel-ray-pruning.log`; client interaction and package tests. | Real browser passage, rover, height picking and local rendering checks. |
| TW5 | Independent review, real HTTP privacy, atomic export, memory caps and process resync checks pass. See `terrain-work-orders/review.md` and process logs. | Final browser transport and cold-play checks. |
| TW6 | Browser drivers are prepared. Native interaction and client checks pass. | Real pointer/key scenarios for both packages, both zoom extremes and final cold-play probes. |
| TW7 | Functionality preflight passes. The first warmed diagnostic exceeds the server p99 limit. See the measurements below. | Optimize, run the full frozen workload and run three 400-unit High-detail browser camera cases. |

The focused client run passes 423 library tests, eight shared-visibility tests and two real-package tests.
The simulation checkpoint passes 439 library tests, with one ignored measurement.
The assembled server run passes twelve terrain tests and thirteen export tests.
These are component results. They do not certify the remaining acceptance rows.

The existing 64 MiB pending-state guard remains unchanged.
An actual boundary check decodes a fully exposed 128×128 grid: 16,384 cells and 147,502 encoded bytes.
A fully exposed 1,024×1,024 grid has 11,272,238 encoded bytes but exceeds the conservative decoded-memory budget and is refused.
The authoring storage bound is not a hosting-capacity claim. Million-cell full exposure remains a capacity limit to address before claiming that scale.
Evidence: `terrain-work-orders/protocol-terrain-grid-boundary-lib.log`.

The independent implementation review passes 102 focused tests. Its test compilation finding is fixed.
Evidence: `terrain-work-orders/review.md`.
The final browser build, High-detail frame rate and scripted cold-play probes have not run.
No spec acceptance criterion has been relaxed. The frozen timing workload and thresholds remain unchanged.

The full native campaign ran once. It passed 3,033 tests and found 32 failures in stale fixtures and checks.
The affected targets pass after those repairs. The obsolete formula-count ledger check was removed.
Evidence: `terrain-work-orders/final-native.log`, `terrain-work-orders/affected-native/exits.json`,
`terrain-work-orders/checks-final.log` and `terrain-work-orders/no-content-final.log`.
Strict workspace Clippy, formatting, the WebAssembly check, dependency boundaries and probe neutrality pass.
All 21 tooling commands pass, including the focused recheck of a stale browser mutation anchor.
Evidence: `terrain-work-orders/final-clippy-clean.log`, `terrain-work-orders/final-format-clean.log`,
`terrain-work-orders/final-wasm-check.log`, `terrain-work-orders/final-boundary.log`,
`terrain-work-orders/final-probe-neutral.log` and `terrain-work-orders/final-tooling/exits.json`.
Process acceptance passes across the original campaign and its scoped continuation.
Only the failed E9 snapshot oracle and the remaining rows were rerun.
Evidence: `terrain-work-orders/final-process-acceptance.log`,
`terrain-work-orders/final-process-acceptance-continued.log` and `terrain-work-orders/final-process-acceptance/`.
These compatibility checks precede the exact arithmetic optimization below.
Full warmed performance acceptance remains pending.

## Warmed diagnostic and exact arithmetic

The four-configuration diagnostic uses 100 warm-up intervals and ten measured intervals per configuration.
Server p99 is 51.56–52.63 ms, above the unchanged 40 ms limit. Both clients pass every timing bound.
The diagnostic completes all functional checks. It does not replace the 1,000-interval acceptance run.
Evidence: `terrain-work-orders/tw7-warm-diagnostic.json` and its metadata file.

A 35-second diagnostic profile identifies 128-bit remainder operations as the largest sampled cost.
The exact rational GCD now uses power-of-two shortcuts and binary reduction.
All 45 pure rules tests pass, including differential checks against the old division-based algorithm.
The integrated simulation terrain filter passes 26 tests. Strict Clippy passes for the changed rules crate.
Evidence: `terrain-work-orders/gcd-full-rules.log`, `terrain-work-orders/gcd-differential.log`,
`terrain-work-orders/gcd-sim-terrain.log` and `terrain-work-orders/gcd-clippy.log`.

The pinned isolated microbenchmark records 15 alternating samples per input family.
Median speedup is 22.678× for terrain-style dyadic inputs and 1.636× for full-width integer inputs.
Evidence: `terrain-work-orders/gcd-microbenchmark.log`.
This is a helper measurement. The optimized whole-service timing and final browser build remain unverified.
The independent feature review above precedes this arithmetic optimization.

## Integration findings

The frozen workload first rejected static units without explicit ForbidEdit foundations. The fixture now declares that existing rule.
The next run exposed a real harvesting panic. A non-spawn transaction queried a catalog that gathering had temporarily borrowed. Spawn preflight now reads that catalog only for actual spawns. The scoped-catalog regression passes: `terrain-work-orders/terrain-scoped-catalog-regression.log`.

The larger workload later hit state-transfer restarts. Initial snapshot decoding and native activation both pass. The initial transfer drains normally.
A diagnostic profile records two actual simulation ticks at336.637ms and343.707ms on CPUs0,2,4 and6.
Exact rational geometry dominates the145 CPU samples. This is a cold diagnostic, not a warmed capacity result.
Evidence: `terrain-work-orders/tw7-first-ticks-profile-metadata.json`, `tw7-first-ticks.perf.data`, `tw7-first-ticks-perf-report.txt`, and `tw7-service-trace/updates.jsonl` in that directory.

The implementation now reuses exact body coverage within a work phase. It refreshes coverage after queue promotion.
All32 focused terrain tests pass, including the differential occupancy oracle.
The same two ticks on the same four CPUs take29.780ms and28.692ms after the change. Both initial client payloads still activate.
Evidence: `terrain-work-orders/terrain-occupancy-reuse-tests.log` and `terrain-work-orders/tw7-occupancy-reuse-comparison.json`.
This comparison does not replace the complete warmed acceptance run.
No geometry approximation, workload reduction, timeout increase or memory-budget increase was made.

Nine real terrain audit checks pass. They cover work, unloading, accounting forgeries, replay,
identifier inversion, identical replacement orders, enable toggles and renamed content.
Evidence: `terrain-work-orders/review.md` and `terrain-work-orders/final-native.log`.

The later trace isolated a16.47-second simulation tick during a route change. Both clients had valid activated state; the long tick starved the real transfer deadline.
Whole-chunk flat certificates were too coarse after work changed a different part of the same chunk.
Truth and permitted planning now certify the exact requested subrectangle inside a mixed chunk.
The focused terrain suite passes33 tests. The diagnostic route-change tick now takes36.342ms without a restart.
The earlier completed trace was unpinned; the new trace uses CPUs0,2,4 and6. This is not a controlled capacity comparison.
Evidence: `terrain-work-orders/tw7-subrectangle-comparison.json` and `terrain-work-orders/tw7-tick11-perf-report.txt`.
The unchanged functionality preflight passes all four configurations and 240 actual simulation ticks.
It checks movement, combat, harvesting, jobs, terrain work, client activation and persistent depletion.
Evidence: `terrain-work-orders/tw7-preflight-subrectangle-final.log`.
This cold preflight does not establish the timing limits. Warmed acceptance remains pending.

The prepared400-model terrain fixture passes normal typed package validation. Its73 source files remain unchanged after packing.
Evidence: `terrain-work-orders/browser-fps-fixture-frozen-v2/typed-pack-result.json`.
