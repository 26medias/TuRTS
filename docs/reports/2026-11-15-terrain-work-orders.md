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
| TW1 | `terrain-work-orders/terrain-tool-source-first.log`; `terrain-client-anchor-current-final.log`; `terrain-work-orders/server-terrain-backend-final.log`. | Complete workspace and process compatibility campaign. |
| TW2 | `terrain-work-orders/sim/sim-lib-lifecycle-final.log`; atomic real-package transport in `terrain-work-orders/server-terrain-backend-final.log`. | Independent conservation audit and final assembled regression run. |
| TW3 | `terrain-work-orders/sim/navigation-pause-final.log`; real allocator inversion and established-claim tests in the simulation suite. | Independent replay certification and final workload conservation. |
| TW4 | `terrain-work-orders/sim/navigation-pause-final.log`; `terrain-work-orders/pure-kernel-ray-pruning.log`; client interaction and package tests. | Real browser passage, rover, height picking and local rendering checks. |
| TW5 | Twelve focused terrain checks in `terrain-work-orders/server-terrain-backend-final.log`; thirteen export checks in `terrain-work-orders/server-export-backend-final.log`; client atomic package checks in `terrain-client-anchor-current-final.log`. | Independent whole-feature review, HTTP/provenance inspection and final caps/resync campaign. |
| TW6 | Browser drivers are prepared. Native interaction and client checks pass. | Real pointer/key scenarios for both packages, both zoom extremes and final cold-play probes. |
| TW7 | Frozen driver and exact workload are committed before timing. | Native preflight, full timed workload and three 400-unit High-detail browser camera cases. |

The focused client run passes 423 library tests, eight shared-visibility tests and two real-package tests.
The simulation checkpoint passes 439 library tests, with one ignored measurement.
The assembled server run passes twelve terrain tests and thirteen export tests.
These are component results. They do not certify the remaining acceptance rows.

The existing 64 MiB pending-state guard remains unchanged.
An actual boundary check decodes a fully exposed 128×128 grid: 16,384 cells and 147,502 encoded bytes.
A fully exposed 1,024×1,024 grid has 11,272,238 encoded bytes but exceeds the conservative decoded-memory budget and is refused.
The authoring storage bound is not a hosting-capacity claim. Million-cell full exposure remains a capacity limit to address before claiming that scale.
Evidence: `terrain-work-orders/protocol-terrain-grid-boundary-lib.log`.

The final browser build, High-detail frame rate, scripted cold-play probes and independent implementation review have not run.
No spec acceptance criterion has been relaxed. The frozen timing workload and thresholds remain unchanged.

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

Four real terrain audit checks pass: ordinary dig/fill/depletion and failed fill; exact unloading; forged resource/depletion/revision rejection; and same-seed command replay with allocator inversion.
Evidence: `terrain-work-orders/audit-stream-sixth.log`. The complete audit suite and independent feature review remain pending.

The later trace isolated a16.47-second simulation tick during a route change. Both clients had valid activated state; the long tick starved the real transfer deadline.
Whole-chunk flat certificates were too coarse after work changed a different part of the same chunk.
Truth and permitted planning now certify the exact requested subrectangle inside a mixed chunk.
The focused terrain suite passes33 tests. The diagnostic route-change tick now takes36.342ms without a restart.
The earlier completed trace was unpinned; the new trace uses CPUs0,2,4 and6. This is not a controlled capacity comparison.
Evidence: `terrain-work-orders/tw7-subrectangle-comparison.json` and `terrain-work-orders/tw7-tick11-perf-report.txt`.
The unchanged full preflight and warmed acceptance remain pending.

The prepared400-model terrain fixture passes normal typed package validation. Its73 source files remain unchanged after packing.
Evidence: `terrain-work-orders/browser-fps-fixture-frozen-v2/typed-pack-result.json`.
