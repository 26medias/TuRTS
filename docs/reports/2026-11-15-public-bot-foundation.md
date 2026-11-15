# Public bot foundation

Status: implementation, foundation reviews, and final browser product probes pass. Final integration remains with Root.
API 0.2.15; network protocol 8.
The final prepared browser candidate is `target/mc-evidence/skirmish-public-bots-prepared`, sealed from clean source `54a893bc6241`.
The accepted launcher remains on SDK14 until Root installs the candidate and integrates the milestone.

## Delivered capability

The renderer-independent `turts_bot` crate exposes controller factories, bounded observations, deterministic physical selection, proposals, coordination, and exact commitment reconciliation.
Portable mod profiles compose opening, recovery, production, and squad controllers.
Bots submit ordinary commands through shared human/bot admission. They have no private gameplay mutation path.
Per-member receipts retain exact accounts, payments, reservations, held requests, jobs, sites, and terminal effects.
Human exports and bot observations share a permission policy. Production remains all-visible.
Trusted logs preserve profile, limits, seed, human connection epochs, command identity, and results.
Recorded-command replay and regenerated comparison reproduce authoritative state and command traces.

Skirmish now uses its public profile for construction, gathering, a paid mixed army, recovery, and combat.
Stockyard exercises physical inventories and separate construction/production actors through the same framework.
A renamed package and external Rust controller prove the extension boundary without game-specific server strategy.

## Acceptance evidence

All paths below are relative to `/home/julien/Projects/TuRTS/target/mc-evidence/public-bots/`.
Results describe the executed candidates. They do not imply that every earlier check was rerun after every later fixture or profile correction.

| Row | Executed result | Evidence |
| --- | --- | --- |
| BF1 | Public core and external controller composition/replacement passed. Framework dependency scan excludes Bevy, simulation, host, transport, and renderer dependencies. | `core-squads-final.log` (44 tests), `bot-boundary.log` |
| BF2 | Profile/library loading, custom host factories, validation paths, explicit-empty API15 floors, and unchanged older-package behavior passed. | `loader-final.log` (6 tests), `client-profiles-first.log` (2 tests), `api-version-final.log`, `runtime-fixtures-final.log` |
| BF3 | Balance/conversion and inventory packages build, gather, produce, and fire through ordinary commands. The renamed fixture passed. Final normal Skirmish completes its tower and Warrior by tick 1261. | `live-packages-fourth.log`, `optimized-native-final.jsonl`, `skirmish-long-loop-final.log`, final browser results below |
| BF4 | Recovery, source-specific accounting, partial groups, delayed feedback, cancellations, destruction, and both receipt/basis arrival orders passed. Blocked publication retains exact COT progress and terminal payments. | `bf4-recovery-second.log`, `actors-scheduled-third.log`, `actor-death-fourth.log`, `review-regressions-final.log`, `review/lazy-publication-actors.log`, `review/lazy-publication-accounting.log` |
| BF5 | Shared admission, ownership/sequence/budget refusals, private-account omission, hidden-present/absent metadata equality, paging, ties, and lease expiry passed. | `server-final.log`, `observation-export-first.log`, `review/observation.log`, `review/privacy.log`, `review/trace.log` |
| BF6 | Seed/state isolation, genuine creation-ID inversion, canonical recorded/regenerated traces, and independent audit rejection of coherent ID-priority tampering passed. | `canonical-replay.log`, `replay-human-second.log`, `review/lazy-publication-replay-pause.log`, `review/immediate-priority-review.md` |
| BF7 | Bot-vs-bot without a human, HumanPresent pause/reconnect with retained COT state, connection epochs, defeat/end guards, and atomic invalid installation passed. | `optimized-native-final.jsonl`, `replay-human-second.log`, `review/lazy-publication-replay-pause.log` |
| BF8 | Required unchanged 1,000-phase workload passed at `3396efb0`: complete-phase p99 **4.906851 ms**, against 5 ms. Earlier run: **25.078596 ms**. Later financial-priority, saving-policy, and profile fixes were **not remeasured**. | `bf8-optimized-final/bf8-measurement.json`, `bf8-optimized-final.log`, `bf8-optimized-final-exit.json`; [measurement record](../measurements/public-bots.md) |
| BF9 | Fresh silent Skirmish browser won at **188.322 seconds** with its castle at **1000 health**. Stockyard public-profile browser passed at **42.696 seconds**. All three cold-play product probes passed. The isolated checker returned **PARTIAL-PASS**, with full human/reference readiness still open. Foundation review ran code and its material findings are fixed. | `final/win/result.json`, `final/win.log`, `final/stockyard/result.json`, `final/stockyard.log`, `final/cold-check-isolated.log`, `review/review.md` |

The cold probes cover first clicks at 1280x720 and 1024x768, plus the button under the start overlay.
Their input, screenshot hashes, sounds, buttons, text, and overlay checks passed.
The first checker invocation lacked the shared GPU-lock marker because each browser used its own isolated Xvfb.
Root reran the checker with `--no-lock-check --partial`; no marker was fabricated.
This skips lock attestation. It does not complete the manual, fresh-agent statement, or reference-red checks.

## Gates and corrected failures

The optimized native library/server campaign passed **607 tests** (`optimized-native-final.jsonl`).
The Node campaign passed 215 tests (`node-final-second.log`); the later harness campaign passed 222 (`node-harness-final.log`).
Scoped content scanning passed 15 tests (`content-scan-final.log`), formula checks passed 7 (`formula-final.log`), and Python CI-list checks passed 7 (`python-ci-list-final.log`).
Formatting, Clippy, API/runtime fixtures, source checks, and process acceptance have retained logs under the evidence root.
`clippy-optimized-final.log` records the optimized candidate's Clippy pass; `process-acceptance/acceptance.log` and `process-acceptance-remaining.log` record process checks.
Older workspace and diagnostic logs include failures. Their names alone are not evidence of a final full-workspace pass.
Later code/profile fixes received focused tests and browser probes; this report does not claim a fresh full campaign or BF8 run for `54a893bc`.

The single whole-feature foundation review found wildcard-order and global accounting-exhaustion faults.
Both are fixed with focused regressions (`review-regressions-before.log`, `review-regressions-final.log`).
The demand-publication optimization received one focused review of fresh bases, exact per-tick capture, bounded endpoint storage, and replay cleanup (`review/lazy-publication-review.md`).
The actual immediate Produce/Construct ID-priority fault is fixed using physical source order, signed-zero normalization, symmetric ties, and an independent audit schedule (`review/immediate-priority-review.md`).
Its independent new targets passed 6 and 5 tests. Existing production tests passed 16. A malformed synthetic fixture failed, then passed after receiving valid positions.

Browser acceptance exposed real late-game policy starvation that the earlier short native fixture did not reach.
The public strategy now saves for mixed-army goals instead of repeatedly spending scarce credits on cheaper units.
Worker expansion and the normal opening have a long-running native regression (`skirmish-long-loop-final.log`).
Stockyard model fit and worker priority were corrected. Final browser probes use the resulting prepared package.
No further whole-feature review or extra process gate was added.

## Remaining limits and delivery

This delivers the public bot foundation of MP2, not all MP2 or the full engine.
Fog, remembered sightings, scripts, creator-authorized lobby bot selection, and advanced tactics remain open.
Custom Rust factories are trusted host-linked code. Downloadable profiles configure factories available on that host.
The benchmark establishes one local workload on four selected i9-14900K cores. Other hardware and maximum scale remain unverified.
The measured performance pass predates the final financial and strategy corrections.
Full human/reference cold-play readiness and a full match between two people remain open.
Root owns the frozen accepted worktree, launcher switch, remaining integration checks, and main merge. Main must not be pushed.

## Accepted launcher

The three global launchers now use the frozen clean source checkout `/home/julien/Projects/TuRTS-public-bots-play` at `54a893bc` and `skirmish-public-bots-prepared`.
The normal launcher was run under its own Xvfb. Its Chrome used a fresh profile and `--alsa-output-device=null`.
The populated game world and screenshot are recorded in `final/global-launcher-ready.json` and `final/global-launcher-ready.png`; startup is in `final/global-launcher.log`.
The two-human and public-host launch flags were retained. Full games in these modes were not repeated for SDK15.
