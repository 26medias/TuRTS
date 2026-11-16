# Terrain work orders

Status: all M4 acceptance checks pass on the integration branch.
The three local launchers use the final terrain build. The actual Skirmish launcher passes its smoke check.

API 0.2.17 adds authored plateau terrain, public geological generation rules, movement classes, excavation/fill tools, physical cargo and explicit unloading.
Protocol 10 installs permitted terrain, work, contacts and accounts on one atomic basis.
The server keeps chosen buried columns and seed provenance outside public packages and HTTP roots.
Quarry uses physical cargo. Terrain Stockyard uses renamed tools and slot balances through the same API.

Evidence root: `/home/julien/Projects/TuRTS/target/mc-evidence/`.
Paths below are relative to that root. Earlier failed logs remain beside the passing runs.

| Row | Evidence | State |
| --- | --- | --- |
| TW1 | Schema/runtime compatibility campaign, process acceptance, and both real public-package browser scenarios below. | PASS |
| TW2 | Lifecycle/accounting audit, three FullCargo unloads, AppendUnload continuity, and physical/slot payment scenarios. | PASS |
| TW3 | Physical claims, identifier inversion, same-seed replay, depletion/provenance and retained-order checks. | PASS |
| TW4 | Exact navigation/support tests, raised picking, ordinary worker clearance, and rover passage in both packages. | PASS |
| TW5 | Independent audit, HTTP privacy, atomic export, memory bounds and process resync. | PASS |
| TW6 | Both full browser scenarios, both zoom limits, and all three final scripted cold-play probes. | PASS |
| TW7 | All four final native configurations pass 80,000 measured ticks. All three 400-unit High views pass. | PASS |

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
The final browser scenarios, three High-detail camera cases and all scripted cold-play probes pass. The final routing source passes the full native retiming run.
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
The earlier full warmed native performance run passes. Final browser acceptance also passes. The final routing source passes the fresh full native measurement.

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
This is a helper measurement. The full-service result is recorded below. The final browser checks remain pending.
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
This cold preflight does not establish the timing limits. The later full warmed run below passes them.

The prepared400-model terrain fixture passes normal typed package validation. Its73 source files remain unchanged after packing.
Evidence: `terrain-work-orders/browser-fps-fixture-frozen-v2/typed-pack-result.json`.

## Earlier warmed native checkpoint

This earlier passing measurement uses source checkpoint `a73ed248c13bb81399a65d059fbaa4e8e22d58c7`. It does not measure the later approach and route fixes.
The unchanged workload passes all four configurations. Each has 100 warm-up intervals and 1,000 measured intervals of 20 ticks.
The run measures 80,000 ticks after 8,000 warm-up ticks. Both native clients pass their tick and batch timing limits.
Every configuration ends with zero service debt. All server p99, p99.9, maximum-tick and maximum-debt bounds pass.

| Visibility | Application ACK delay | Server p99 | Maximum tick | Result |
| --- | --- | --- | --- | --- |
| Fog | 50 ms | 38.125 ms | 63.556 ms | PASS |
| Fog | 100 ms | 37.108 ms | 59.053 ms | PASS |
| AllVisible | 50 ms | 39.238 ms | 96.598 ms | PASS |
| AllVisible | 100 ms | 38.263 ms | 56.825 ms | PASS |

Evidence: `terrain-work-orders/tw7-final-1000.json`, `terrain-work-orders/tw7-final-1000.log` and `terrain-work-orders/tw7-final-1000-metadata.json`.
The run uses CPUs 0, 2, 4 and 6. It ran from 2026-11-16 00:27:59 UTC to 01:08:23 UTC.
The measured executable SHA-256 is `6b04f0b4206f947c0c4cf9e3bee26d0c541325ca14326ccd0d612cf74ccacaae`.

The remaining optimizations preserve exact rules and public outputs. They cache exact ground support, narrow only separation searches, merge sorted terrain deltas and borrow immutable surface material names.
Differential checks compare the old arithmetic, movement trajectories and serialized terrain projections.
They pass 47 pure rules tests, 37 terrain tests, 40 movement tests with each probe setting, two borrowed-surface tests and six export tests.
Changed-target strict Clippy and formatting pass.
Evidence: `terrain-work-orders/normalization-rules.log`, `ground-cache-terrain.log`, `separation-movement.log`, `separation-movement-probes.log`, `surface-borrowed-tests.log` and `export-allvisible-tests.log` in that directory.
The independent feature review precedes these performance changes. The differential checks and full timing run cover the later changes.
No workload, quality, deadline, timeout or memory limit was reduced.

## Final browser findings

The prepared candidate builds successfully from source checkpoint `a73ed248`.
The first driver run opened a bare page URL. The browser client requires explicit server and WebSocket query parameters.
Both terrain drivers now supply those parameters. Their syntax checks pass. This repair changes no built artifact.

The connected browser receives live terrain and own units. Its first body selection fails.
Separate pointer move, down and up frames reproduce the failure. An 80 ms hold does not fix it.
The camera feedback system reconstructs the camera at zero ground height after the interaction adapter raises it to the current terrain.
On the ridge-centred fixture, rendering uses a 32-unit eye height while interaction expects 33 units.
The client fix must preserve the exact terrain camera base through shake and align raised-body hover.
Evidence: `terrain-work-orders/final-quarry-browser-sequenced-input/browser/selection-attempt-1.json`, `failure.png` and `result.json` in that folder.
No terrain browser acceptance row is claimed from these failed runs.

The camera feedback fix preserves the adapter's exact unshaken transform. Hover uses the live authorized body support.
The original system fails three new camera regressions. The fixed system passes those three tests and two raised-hover tests.
All 43 existing adapter tests pass. All 24 construction/shared-export tests pass. Strict client Clippy passes.
One shared export test needed the existing `bevy_app` crate declared as a direct client test dependency.
Evidence: `terrain-work-orders/final-camera-feedback-red.log`, `final-camera-feedback-green.log`, `final-hover-ground-tests.log`, `final-camera-adapter-tests.log`, `final-camera-construction.log` and `final-camera-clippy-fixed.log` in that directory.
The camera fix changes client presentation only. The measured server, rules and simulation source are unchanged.
Picking during nonzero cosmetic camera shake was not checked. The focused shake tests cover bounded displacement and exact return to the terrain base.

The rebuilt browser passes body selection, raised-body selection and fill/cut ghosts at both zoom limits.
It also preserves partial work through Disable/Enable and Stop. These partial results do not pass the complete browser row.
One run was interrupted by SIGTERM. Its partial captures remain preserved.
The next complete attempt found a probe mismatch: simulation occupancy uses the existing public `Blocked` state.
The probe now checks that state, its visible refusal text, and unchanged terrain and payment. No spec criterion changed.

The Quarry run then found a real FullCargo shortcut bug. The generated action used a local order name instead of the qualified catalog key.
The normal targeted Append unload works and resumes the same public work. The shortcut emits no command and needs the qualified-key fix.
Evidence: `terrain-work-orders/final-camera-quarry-browser-blocked-oracle/browser/result.json` and its retained input frames.

The renamed Stockyard run found a work-approach bug on its two-unit cell grid.
The chosen pose `(22, 19.65)` straddles the ridge and cannot support the worker. A legal west-side approach exists.
The work remains queued but never starts. The fix must choose a full-footprint legal approach from permitted geometry.
Evidence: `terrain-work-orders/final-camera-stockyard-browser/browser/result.json`.
Both product defects are fixed. The FullCargo shortcut now emits the qualified order key. A real panel-to-observer-to-adapter regression passes for Quarry and renamed cargo capabilities.
The approach planner checks supported side poses and certified direct routes before ordinary route search.
Evidence: `terrain-work-orders/final-full-cargo-action-tests.log` and `terrain-work-orders/approach/`.

The Stockyard passage then exposed a real coarse-routing limit. A two-unit opening fits its rover, but cell-centre routing cannot represent it.
An exhausted coarse search now falls back to half-cell vertices. Every edge and actual start/goal connector uses the exact support certificate.
The original unsnapped goal remains the destination. The existing search budgets remain unchanged.
Two Quarry regressions and the actual cleared Stockyard passage pass. All 42 focused terrain tests and strict simulation Clippy pass.
Evidence: `terrain-work-orders/half-route/` and `terrain-work-orders/half-route/stockyard-clear-bay-final.log`.

## Final browser and player evidence

Both complete terrain scenarios pass on the final runtime source `2073e15eb6c9`.
The prepared build is `skirmish-terrain-route-final-prepared`.

- Quarry: `terrain-work-orders/final-route-quarry-browser-live-widget/browser/result.json`.
- Terrain Stockyard: `terrain-work-orders/final-route-stockyard-browser/browser/result.json`.

They check ordinary body selection, raised picking, near/far cut and fill previews, partial pause/resume, Stop,
occupied-foundation refusal without charge, work continuation, passage clearance, rover traversal, paid fill, and PlacedNoYield.
Quarry also checks partial AppendUnload continuity and three FullCargo shortcuts. The driver waits for actual cargo reduction.
The worker clears the passage with an ordinary move. It is not evicted by the rover.
Stockyard uses free exit `(31, 19)`. The old `(31, 16)` destination overlapped a settled enemy at `(32, 16)`.
This repairs the passage fixture. Closest completion at occupied destinations is tracked separately in M4b.

| High camera | Mean FPS | Evidence |
| --- | --- | --- |
| Normal | 59.7 | `terrain-work-orders/final-route-high400-mixed-oracle/result.json` |
| Near | 59.5 | `terrain-work-orders/final-route-high400-remaining-views/result.json` |
| Far | 59.8 | `terrain-work-orders/final-route-high400-remaining-views/result.json` |

All cases retain 400 detailed High models, live terrain work, the frozen v3 assets and stats, combat, and the unchanged 30 FPS limit.
The normal row passed before that driver's next near-view assertion failed on a cached capture. That first result file therefore has overall status FAIL.
The focused near/far run passes both selected cases. The three rows above combine their actual passing samples.
The driver now waits for a fresh scene capture after camera changes. Raw samples are retained before assertions.
The combat oracle measures attack coverage across the sample because melee actors rest between strikes.
It still requires the full expected attack mix, health loss at every applicable victim, and all 400 actors alive.
No frozen assets, stats, commands, camera positions, sample duration, or frame-rate threshold changed.

All final scripted cold-play probes pass on this prepared build:

- First click, 1280×720: `terrain-work-orders/final-cold-first1280/internal/session.log`.
- First click, 1024×768: `terrain-work-orders/final-cold-first1024/internal/session.log`.
- Buttons, 1280×720: `terrain-work-orders/final-cold-buttons/internal/session.log`.

These paths are relative to the evidence root. Each run uses a fresh profile and null audio.
A new human play session, a complete human multiplayer match, and pointer alignment during nonzero cosmetic shake were not checked.

## Final native source verification

The actual final source preflight passes all 240 ticks and four configurations.
Evidence: `terrain-work-orders/tw7-final-route-preflight.log` and `tw7-final-route-preflight-metadata.json`.
The warmed ten-interval diagnostic passes all four deadlines. Its server p99.9 is 36.745–39.043 ms.
Evidence: `terrain-work-orders/tw7-final-route-warm-diagnostic.json`.
The full 1,000-interval run passes all four configurations. It measures 80,000 ticks after 8,000 warm-up ticks.
Both native clients pass tick and batch deadlines. All server p99, p99.9, maximum-tick and service-debt bounds pass.
Every configuration ends with zero service debt.

| Visibility | Application ACK delay | Server p99 | Server p99.9 | Maximum tick | Result |
| --- | --- | --- | --- | --- | --- |
| Fog | 50 ms | 36.523 ms | 39.718 ms | 60.262 ms | PASS |
| Fog | 100 ms | 37.119 ms | 42.403 ms | 61.027 ms | PASS |
| AllVisible | 50 ms | 37.469 ms | 40.775 ms | 47.908 ms | PASS |
| AllVisible | 100 ms | 38.374 ms | 42.478 ms | 60.733 ms | PASS |

Evidence: `terrain-work-orders/tw7-final-route-1000.json`, `.log` and `-metadata.json`.
The run used CPUs 0, 2, 4 and 6 from 2026-11-16 04:16:36 UTC to 04:56:12 UTC.
Its source is `6649262aff6c8222eb2042a83c2c7f9f6cff2d51`; runtime engine source remains the final browser checkpoint above.
The executable SHA-256 is `eb514ffee1821ecd20cb0ffdf22fc16debce8153e92dd6cbe1a25fd13cc4f9cf`.
The build record is `terrain-work-orders/tw7-final-route-build.jsonl`.

The independent whole-feature review ran 102 tests before the later performance and browser fixes.
The later fixes have focused regressions, browser acceptance, and the fresh final measurement described above.
No second whole-feature review loop was added. No product requirement or acceptance threshold was relaxed.


## Launcher and handoff

The actual `target/mc-evidence/play-skirmish.sh` starts the final prepared build and receives a populated authoritative world.
The test used its real wrapper, an owned Xvfb display, a fresh Chrome profile and `--alsa-output-device=null`.
The profile was removed on shutdown. Evidence: `terrain-work-orders/launcher-final-smoke/result.json`, `browser-result.json` and `world.png`.
The wrappers supply their recorded build settings so prepared-source validation also works from an ordinary shell.
Two-player and shared-host wrappers point to the same final source and bundle. Their complete match campaigns were not repeated for this terrain milestone.

A player can still build, harvest, train and fight in Skirmish. The two public terrain packages additionally support excavation, fill, physical or slot payments, explicit unloading, raised picking and traversable passages.
M4b group routing and M5a surveys remain separate unaccepted branches. Terrain sight and direct-fire obstruction is specified for M5b.
The million-cell fully exposed map limit, nonzero-shake picking and new human multiplayer session remain unverified as stated above.
