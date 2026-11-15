# Shared visibility acceptance

Status: SV1-SV7 pass. Independent foundation review passes. The tested milestone is ready for main.
The three local launchers now use the sealed SDK16 build. Tested main commits are pushed under standing CEO approval.

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
| SV1 | Authoring, API-floor, new-match and package compatibility checks: `shared-visibility/final-native.log`, `shared-visibility-api16-authored-tests.log`, `shared-visibility-new-match-tests.log`. | PASS. Old packages keep AllVisible; both authored sharing modes and rejection boundaries are covered. |
| SV2 | Exact coverage, source lifecycle, teams, phase ordering and reversed creation/peer order: `shared-visibility/review-final-sim-release.log`, `shared-visibility-viewer-order-tests.log`, `shared-visibility/final-native.log`. | PASS. Includes real movement, completion, healthless/disabled sources and sight-source death. |
| SV3 | Stale observations, hidden-state equality, observed empty/death clearing and retention limits: simulation logs above; `shared-visibility/sv7-v2-full-1000.json`. | PASS. Long repeated hide/reveal/resync campaign retains bounded state; no duplicate history. |
| SV4 | Independent code-running review: `shared-visibility/review.md`. Atomic protocol/client/socket/export tests and final killing-health process logs under `shared-visibility/`. | PASS. Includes complete 50/100 ms delayed-ACK delivery, cancellation, cap/restart/isolation, authorized finite effects and health zero without resync. |
| SV5 | Ordinary real-tick target-loss tests, command replay, public placement/query metadata and hidden/absent pairs: `shared-visibility-query-metadata-tests.log`, `shared-visibility/final-native.log`, independent simulation logs. | PASS. Includes appending repair behind an active move without cancelling it. |
| SV6 | Final sealed build: `shared-visibility-final-sv6/browser/result.json`; three probes and checker in `shared-visibility-final-cold-fixed/`. | PASS. Two-slot scouting and all scripted cold probes pass. Checker PARTIAL-PASS; manual/reference checks remain open. |
| SV7 | Native raw samples: `shared-visibility/sv7-v2-full-1000.json`. Nine High-detail camera cases: `shared-visibility-sv7-browser-final/result.json`. | PASS v2. The 60,000-tick campaign and both clients meet the declared deadline bounds. All nine 400-unit browser cases reach 59.9-60.0 FPS. Original v1 ceiling still fails. |

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

## Final browser and launcher handoff

Prepared build: `/home/julien/Projects/TuRTS/target/mc-evidence/skirmish-shared-visibility-final-prepared`.
Frozen play source: `/home/julien/Projects/TuRTS-shared-visibility-play`, clean `81c37e1f`.
The prepare seal records source `ba4754436644`, package `af93c034d490`, and 80 runtime artifacts.
All nine browser cases preserve High models, textures, normal maps, shadows, and authored camera bounds.
All three scripted cold probes pass. No fresh human/reference campaign was run.

The actual global normal launcher starts this build, loads all models and minimap, and shows sound off.
Its fresh silent Chrome profile was removed after the check. Owned Xvfb was stopped.
Evidence: `shared-visibility-final-launcher/browser/result.json`, `game.png`, and display cleanup record.
WASM transferred as gzip: 15,371,667 compressed bytes from 61,816,960 raw bytes.
The two-human and public Internet launchers keep their tested flags. Full matches in those modes were not repeated.

A Cargo wrapper diagnostic polluted the prepared compiler-version field.
The wrapper now answers version queries without cache invalidation. Two direct regression checks pass outside Git.
Only that manifest metadata field was corrected; the original seal and proof of identical source/artifact hashes remain in `shared-visibility/cargo-version-metadata-repair.json` and its adjacent backup.
No server or browser binary changed after the measured candidate.

CI's two overlapping-unit clicks now search authored visual centers. Exact selection oracles remain.
The merged focused Node campaign passes all 50 tests: `shared-visibility/final-ci-browser-oracles.log`.
Original CI image replay also passes. A new hosted browser CI run remains to be observed after push.

Editable terrain, geological discovery and terrain occlusion remain outside SDK16.
M4 implementation continues separately; it is not part of this accepted build.
