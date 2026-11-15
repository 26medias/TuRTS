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
