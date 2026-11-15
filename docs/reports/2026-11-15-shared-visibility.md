# Shared visibility acceptance

Status: implementation and focused native checks are in progress. This milestone is not accepted.
SDK15 remains the playable build on local main. Main was not pushed.

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
| SV2 | `shared-visibility-core-tests-fixed.log`: 15 simulation tests and 5 shared-rule tests. Actual movement, construction completion, overlapping sources, teams, healthless/disabled sources, source death, and exact boundary rules pass. | Focused checks PASS. Full acceptance coverage remains under review. |
| SV3 | Same simulation log: hidden movement/death equality, stale sightings, observed empty locations, whole-oldest-tick retention, sticky truncation, and bounded large-circle cache. | Focused checks PASS. Long-cycle memory measurement remains. |
| SV4 | `shared-visibility/review-export.log`, `review-human-wire.log`, `review-outbound-corrected.log`, and `review.md`. Initial protocol assembly tests: `shared-visibility-core-tests-fixed.log` (7 PASS). Client atomic tests: `shared-visibility-client-fixtures-tests.log` (7 PASS). Verified-map guards: `shared-visibility-client-identity-tests.log` (2 PASS). Allocation preflight: `shared-visibility-client-budget-tests-fixed.log` (3 PASS). | Existing focused checks PASS. Ordered delayed-ACK pipeline and runtime cap evidence remain. |
| SV5 | Simulation hidden attack, queued target loss, extraction, and hidden/absent node tests pass. The independent simulation run includes repeated command replay across all phase knowledge bases: `shared-visibility/review-sim-visibility.log`. | These checks PASS. Full public placement/query metadata coverage remains. |
| SV6 | Public two-human scouting fixture and browser driver are implemented. | Browser run and final cold-play probes remain. |
| SV7 | Frozen workload is specified. Sender throughput correction and measurement harness are in progress. | Not measured. No performance pass claimed. |

## Independent review

The review is open for the ordered transport correction.
Current evidence: `shared-visibility/review.md`.
The reviewer ran 4 export checks, 1 real human wire check, 16 simulation checks, and 5 outbound checks.
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

No browser fog behavior, final-build cold readiness, 400-unit fog FPS, or SV7 CPU threshold is claimed yet.
