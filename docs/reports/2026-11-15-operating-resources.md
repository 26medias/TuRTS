# Operating resources

Status: PASS. The milestone adds API 0.2.13. Protocol 7 stays unchanged.
The prepared runtime uses clean source `4ed93660356ba2970aeef22f25f89f91b90136da`.
Later commits retain tests, repair old test tools, and add this report.

Mods can define slot supply, capability-specific shortage effects, and periodic income.
The smallest active rate wins. Weapons use Disable independently of manual Disable.
Supply effects freeze before commands. Live admission tracks creation and removal within the tick.
Producer outputs and site completions share physical source priority. Exact ties hold symmetrically.
Income starts at actual activation. A full account retries the whole due batch without backfill.
Default Skirmish content and balance stay unchanged.

Evidence root (E): `/home/julien/Projects/TuRTS/target/mc-evidence/operating-resources/`.
All paths below are relative to E.

## Executed acceptance

| Row | Result and evidence |
| --- | --- |
| OR1 | PASS. `runtime/result.json`, `runtime/final.log`, `boot/final.log`: public fixture loading, wrong kinds/references, duplicates, invalid rates/periods, reserved scopes/effects, API floors, and merged-catalog Q. The maximum authored period fits; generic conversion overflow is tested separately. |
| OR2 | PASS. `backend-sim-tests.log`, `engine/first.log`, `income/lifecycle/result.json`, `audit/removal-stream.jsonl`: completion/spawn/removal/death/revival, sites, manual Disable, declared zeros, atomic hooks, and immutable tick boundaries. `final/operating/result.json` confirms real browser creation and removal. |
| OR3 | PASS. `engine/first.log`, `review/q-probes.log`: exact fractions, minimum constraints, health, retained constructor rules, FIFO, and recovery. `final/operating/result.json`: full work 41/41 ticks; producer shortage 31/62; site shortage exactly one-third speed. |
| OR4 | PASS. `supply/power-callers.log`, `backend-sim-tests.log`, `final/operating/result.json`: acquisition/fire inhibition, recovery, owner separation, and preserved manual Disable. |
| OR5 | PASS for the corrected event/account boundary. `income/result.json`, `income/lifecycle/result.json`, `audit/stream.jsonl`, `workspace-final.log`, `income-completion-final.log`: two tick rates, 1,050 ms, actual spawn/site-completion clocks, two full payment periods, removal/revival, no site/dead income, and no gather/delivery events from income. `final/operating/result.json` checks periodic visible credits. |
| OR6 | PASS. `income/result.json`, `income/lifecycle/result.json`, `engine/first.log`: whole-account retry, independent accounts, checked supply/income bounds, reversed creation IDs with the same physical winner, unchanged final progress/resources on refusal, and recovery. |
| OR7 | PASS. `audit/final.log`, `income/result.json`, `workspace-final.log`: repeated per-tick state/events and unchanged frozen base/haul/acme/Cedar streams, Forge fixed point, and renamed-package traces. No baseline was rewritten. |
| OR8 | PASS. `client/export-checks.log`, `client/export-replica-checks.log`: owner projection through postcard into replicas, foreign-record refusal, explicit zeros, resync, and new-match reset. |
| OR9 | PASS. `final/operating/result.json` and screenshots: real mouse Remove/Build, shortage, completion, recovery, manual Disable/Enable, income, owner privacy, one alert per transition, and zero totals. |
| OR10 | PASS. `review/review.md`, `review/result.json`, native logs listed below, and `final/summary.md`: independent code-running review, required integration checks, three final scripted player probes, normal win, performance floor, and actual launcher. |

## Integration checks

The workspace run covered 196 suites. Four old test-instrument failures were repaired and rerun.
The resulting coverage has 2,472 passing cases, including the added completion-clock regression; 40 workspace cases were ignored.
`workspace-final-summary.json`, `formula-final.log`, `server-final.log`, and `income-completion-final.log` record this scope.
The independent reviewer also ran 476 simulation cases, eight engine scenarios, runtime/audit checks, frozen compatibility, and two independent Q=6 completion probes.
No material review finding remained.

Strict Clippy, format, WASM workspace checking, dependency boundaries, probe neutrality, and nine existing harness checks pass.
See `clippy-retained-tests.log`, `clippy-oracle-final.log`, `format-final-candidate.log`, `wasm-final.log`, `boundary-final.log`, `probe-neutral-final.log`, and `harness-final/results.json`.
The new regression has its own focused final check.

Required release process acceptance passed across the original run and scoped continuations.
Passed rows were retained. Only failed rows and remaining rows were run afterward.
See `acceptance-final/`, `acceptance-oracle-rally-derived.log`, `acceptance-oracle-rally-f32.log`, `acceptance-after-e9.log`, and `acceptance-last-body.log`.
The final HD1 reference is `acceptance-final/hd1-native-reference-body-checked/reference/`.

Old test tools were repaired to accept producer rally rows, compare coordinates at exact wire f32 precision, and use the real Supply system.
The name-only formula check now records two state adapters.
HD1 now accepts the wrapper-managed shared cache and clicks an exposed body instead of an occluded ground point.
These repairs preserve exact result checks, source checks, and artifact checks.

## Final build

`final/combat-loop/result.json` passes Warrior training, tower construction, movement, melee, and tower fire.
`final/win/result.json` reaches normal Victory in 154.749 seconds with 640 harvested credits.
The player castle stays at 1,000 HP. The computer fields a Warrior and a completed tower.

All three scripted player probes pass at the required viewports.
`final/cold-check.log` correctly reports PARTIAL-PASS because manual and blind-player checks were not repeated.
The 400-unit release measures 59.403 FPS High and 59.004 FPS Low on the local RTX 3080.
All actors stay visible and take damage. Shared skinning and environment checks pass.
No detail was reduced. See `final/measure-400/result.json` and `docs/measurements/models.md`.

The three local launchers now use `target/mc-evidence/skirmish-operating-prepared` with the frozen operating-play checkout.
The actual single-player launcher passes model/map loading, compressed transfer, and Sound off.
See `final/launcher/result.json`. All test profiles were removed. Runner receipts report unchanged source and no survivors.
The two-player and public-tunnel launchers received the same path update; they were not rerun in this milestone.

## Corrections and limits

OR5 preserves the rule that income must not count toward Gathered.
The Gathered accumulator and its HUD are not implemented. The revised check proves the actual credit/event boundary.
A future statistics implementation must add the actual counter regression.
The same Anvil reviewer approved this narrow correction and its plan in `anvil-stat-clarification.md`.

Automatic arithmetic refusal has a typed authoritative diagnostic. This feature does not add an unsolicited owner wire notice.
The authored u32-ms/u16-Hz maximum fits u64 ticks. No arbitrary period cap was added.
Network supply, Queued counts, and BlockStart remain reserved.

The auditor independently models successful income batches, contributions, clocks, and queued removal.
New independent replay models for no-event full-account retries, scaled completions, and shortage-rate changes remain open.
Simulation tests cover these paths; they do not make the auditor a complete independent simulation.
The 2,000-unit and panning workloads, GPU process RSS, other hardware, human multiplayer, and full manual readiness were not repeated.
The previous three-type 2,000-unit High result remains below 30 FPS.
No main push was made.
