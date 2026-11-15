# Shared visibility acceptance

Status: native acceptance and independent foundation review pass. Final browser acceptance remains open.
SDK15 remains the playable build on main. Tested main commits are pushed under standing CEO approval.

## Scope

API 0.2.16 adds optional authored map fog and completed-unit sight circles.
Protocol 9 carries atomic permitted state, coverage, and bounded stale sightings.
One authoritative phase basis controls human export, bot queries, and target actions.
Old packages keep AllVisible gameplay. Default Skirmish content stays AllVisible.
Terrain occlusion, stealth, geology, scripts, lobby bot administration, live package replacement, and spectator delivery remain outside this milestone.

Evidence root: `/home/julien/Projects/TuRTS/target/mc-evidence/`.
The subdirectory `shared-visibility/` holds independent review logs.

## Acceptance progress

| Row | Evidence run | Current result |
| --- | --- | --- |
| SV1 | `shared-visibility-authored-tests.log`: 4 authoring tests. `shared-visibility-new-match-tests.log`: populated second install refuses without changing knowledge; fresh match has no old memory. Client fixture tests load two structurally different packages, including renamed Stockyard. | These focused checks PASS. API-floor and full compatibility campaign remain. |
| SV2 | `shared-visibility-core-tests-fixed.log`: 15 simulation tests and 5 shared-rule tests. Actual movement, construction completion, overlapping sources, teams, healthless/disabled sources, source death, and exact boundary rules pass. | Focused checks PASS. The complete simulation suite now passes 409 tests with 1 ignored measurement. Exact circle-oracle checks pass 6 tests. Full row coverage remains under review. |
| SV3 | Same simulation log: hidden movement/death equality, stale sightings, observed empty locations, whole-oldest-tick retention, sticky truncation, and bounded large-circle cache. | Focused checks PASS. Long-cycle memory measurement remains. |
| SV4 | `shared-visibility/review-export.log`, `review-human-wire.log`, `review-outbound-green.log`, and `review.md`. Initial protocol assembly tests: `shared-visibility-core-tests-fixed.log` (7 PASS). Client atomic tests: `shared-visibility-client-fixtures-tests.log` (7 PASS). Verified-map guards: `shared-visibility-client-identity-tests.log` (2 PASS). Allocation preflight: `shared-visibility-client-budget-tests-fixed.log` (3 PASS). | Focused checks PASS. Ordered 50/100 ms RTT, cancellation, overflow, and sender-failure checks also pass. The complete client library passes 405 tests with 1 ignored measurement; 8 visibility integrations pass. Final independent review remains. |
| SV5 | Simulation hidden attack, queued target loss, extraction, and hidden/absent node tests pass. The independent simulation run includes repeated command replay across all phase knowledge bases: `shared-visibility/review-sim-visibility.log`. | These checks PASS. Full public placement/query metadata coverage remains. |
| SV6 | `shared-visibility-sv6-first/browser/result.json`: two silent browsers pass scouting, stale memory, hidden movement, old-location clearing, ground/minimap fog, and no hidden model/health/picking. | Intermediate browser PASS. Final-build rerun and cold-play probes remain. |
| SV7 | `docs/measurements/shared-visibility.md` records four timing diagnostics and two intermediate browser observations. | Original 5 ms budget FAIL. Earlier v2 diagnostics FAIL. The latest short v2 diagnostic passes after spatial lookup/order fixes. No full acceptance pass claimed. Final timing and nine-case browser campaign remain. |

## Independent review

The independent foundation review passes with no unresolved material finding.
Current evidence: `shared-visibility/review.md`.
The reviewer ran 4 export checks, 1 real human wire check, 16 simulation checks, and 5 outbound checks.
The final independent release execution also passes 409 simulation, 3 delayed-ACK pipeline, 8 protocol assembly, and 7 export tests.
Four exact vendored socket tests also pass, including cancellation of a blocked write and byte-permit release.
The vendor test originally failed to compile because its assertion called Display on an error without that trait.
The assertion now matches the explicit cancellation reason and passes.
The vendor review used an archived lean manifest with the exact production source and dependencies.
It omitted unrelated upstream demo dev dependencies. It used the shared target and did not change production source for the test.

## Corrections and limits

A fog map's qualified identity is limited to 512 UTF-8 bytes.
The fixed 1024-byte frame repeats this identity in each transaction stamp.
Validation rejects an overlong opt-in fog identity at the authored `id` field before installation.
AllVisible authoring keeps its previous identity rules.

The previous wording about atomic reinstall exceeded the existing engine API.
A populated world rejects a second installation atomically. Each new match uses a fresh world.
The regression verifies that refusal preserves existing knowledge and a fresh match carries no stale records.
This milestone does not add live hot package replacement.

Coverage source indexes have an 8 MiB aggregate cache cap.
A circle that cannot fit uses the same exact geometry as a streaming iterator.
The regression keeps three million-cell circles exact while only two are cached.

The intermediate browser fog probe passes. The 400-unit normal-camera observation is 59.603 FPS. Final-build cold readiness and SV7 acceptance remain open.

## Ordered transport correction

One tagged StateTransactionFrame message now preserves Begin/Chunk/End order across batched delivery.
The sender uses the registered Lightyear message ID and framing. It tracks real transport MessageIds.
A bounded successor pipeline removes the per-transaction round-trip barrier.
Cancellation uses the last issued stamp as a retirement watermark. The active complete view stays available.
The focused spec clarification took two short turns with the same reviewer because the first turn found the cancellation gap.
The plan passed one turn with that same reviewer. The original broad spec review was not repeated.

New focused checks pass:

- `shared-visibility-client-ordered-tests.log`: 3 checks, including two complete transactions in one application batch.
- `shared-visibility-server-ordered-tests.log`: 5 outbound checks with registered raw framing and bounded real message credits.
- `shared-visibility-human-wire-ordered-tests.log`: the real two-human Hello/slot-specific atomic knowledge test.
- `shared-visibility-cancel-watermark-tests.log`: early successor cancellation preserves the active view and rejects the old chain.

The sustained 50/100 ms delayed-ACK workload passes. All 11 outbound checks now pass, including cancellation, overflow recovery, and fatal sender cleanup: `shared-visibility-outbound-reuse-failure-tests.log`. The final independent implementation check remains open.


## Repair queue regression fixed

A new real-tick test found that appending repair could cancel an admitted player move.
The implicit repair-approach cleanup now requires an active repair action.
Legacy and modern singleton moves retain their motion when repair is appended.
Queued repair rechecks visibility at activation and refuses hidden/absent targets generically.
The regression requires no healing on every intervening tick, a real refusal, and final destination arrival.
The complete simulation suite passes 407 tests: `shared-visibility-sim-predicate-tests.log`.
Additional raw API package gating, paginated/private query metadata, and reversed real-wire peer-order checks pass.
Evidence: `shared-visibility-api16-authored-tests.log`, `shared-visibility-query-metadata-tests.log`, and `shared-visibility-viewer-order-tests.log`.

## Final native candidate

`shared-visibility/final-native.log` passes 2,924 workspace tests, with 47 ignored measurements/process cases.
Strict workspace Clippy, formatting, the WASM check, probe-neutral hashes, and production boundary checks pass.
The 21 tooling commands pass. The updated AllVisible browser-oracle unit checks pass all 46 tests.
The full process acceptance cases pass, including lossy transport, killing health zero without resync, reorder/stall, both HD1 commands, and the current clean native reference.
Evidence: `final-clippy.log`, `final-format.log`, `final-wasm-check.log`, `final-probe-neutral.log`, `final-boundary.log`, `final-tooling/`, and `final-process-acceptance/` under `shared-visibility/`.

The final 1,000-interval native performance campaign passes all three modes and 60,000 measured server ticks.
Full-server p99 is 33.226 ms with fog, 33.028 ms in equivalent AllVisible, and 33.544 ms with concentrated units.
Both native clients pass every timing limit. See `docs/measurements/shared-visibility.md` for raw evidence and limits.
The final browser build, nine High-detail camera cases, and cold player probes remain unverified.

An obsolete HD1 cache check expected the old wrapper marker. It now verifies both current native and host markers.
Completed debug test executables were removed after their processes ended. This freed 13.84 GiB without removing evidence or shared libraries.
