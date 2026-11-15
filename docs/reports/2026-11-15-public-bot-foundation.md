# Public bot foundation

Status: integration in progress. Do not treat this report as acceptance.
The accepted launcher still uses the merged paid-work and repair build.
This milestone targets API 0.2.15 and keeps protocol 8.

## Implemented scope

The pure `turts_bot` crate provides controllers, a registry, bounded queries, physical selection, coordination, and exact commitment reconciliation.
Portable mod profiles compose opening, recovery, production, and squad controllers.
The server submits bot effects through the ordinary command admission and simulation paths.
Per-member receipts preserve exact payment sources, held requests, jobs, sites, and terminal effects.
Human exports and bot observations share a permission policy. The default remains all-visible.
Trusted logs identify bot profiles, limits, seeds, human connection epochs, commands, and results.
The replay tests exercise recorded commands and regeneration against exact state and traces.

## Checks run so far

Evidence root: `/home/julien/Projects/TuRTS/target/mc-evidence/public-bots/`.

| Check | Observed result | Evidence |
| --- | --- | --- |
| Public core and external controller integration | 44 tests passed, including the squad fix | `core-squads-final.log` |
| Framework dependency boundary | No Bevy, host, simulation, transport, or renderer dependency | `bot-boundary.log` |
| Portable loading and API floor | All 6 tests passed | `loader-final.log` |
| Client aggregate profiles and custom factories | Both tests passed | `client-profiles-first.log` |
| Typed actor log validation | All 7 external tests and 1 independent witness test passed | `audit-actor-first.log`, `audit-witness-first.log` |
| Initial server suite | 140 passed; 2 live fixtures initially failed | `server-first.log` |
| Real group admission and held completion | 6 passed; death fixture needed both foreign weapons held | `actors-scheduled-third.log` |
| COT debit followed by real same-tick weapon death | Passed after fixing the fixture | `actor-death-fourth.log` |
| Skirmish and Stockyard live public profiles | Both built, harvested, produced units, and fired | `live-packages-fourth.log` |
| Human permission projection | All 8 tests passed | `observation-export-first.log` |

The live scenarios exposed an equal-distance squad selection bug. The fix ranks distinct positions and content without unit IDs.
Replay exposed an old audit rule that rejected ordinary Construct orders. All four independent Construct tests now pass (`audit-construct-third.log`).
Recorded and regenerated replay pass with real human input on both connection epochs (`replay-human-second.log`).
The audit state now captures the immutable construction cap needed to verify place-versus-hold decisions.

The independent foundation review ran 23 focused tests and found two faults (`review/review.md`).
Wildcard orders now match the ordinary command contract.
Accounting row exhaustion now freezes only the affected bot. It retains exact payment and terminal facts until publication recovers.
The review regressions first reproduced the isolation/recovery failures. All five focused checks then passed (`review-regressions-before.log`, `review-regressions-final.log`).
The reviewer did not repeat the whole review after these fixes.

## Remaining acceptance

BF1-BF9 are not yet all complete.
Open checks include final integration replay after the accounting fix, renamed fixture execution, genuine creation-ID inversion, the complete-phase performance measurement, final integration checks, browser profiles, and cold-play probes.
The final report must replace this section with a row-by-row acceptance result and evidence paths.
No main merge or push is authorized by this incomplete report.

## Limits

This is the public bot foundation of MP2. It does not complete MP2.
Fog, remembered sightings, scripts, lobby bot selection, and advanced tactics remain outside this milestone.
Custom Rust factories are trusted host-linked code. Downloadable profiles configure available factories.
Other hardware and maximum unit counts remain unverified.
