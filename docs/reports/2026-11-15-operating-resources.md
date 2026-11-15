# Operating resources — acceptance draft

Status: preview checks pass. Final release acceptance and independent review remain pending.
This report maps existing evidence. It does not declare the milestone complete.
Integration reference: `codex/operating-resources` at `4ed93660`.

Evidence root (E): `/home/julien/Projects/TuRTS/target/mc-evidence/operating-resources/`.
Paths below are relative to E. Earlier component evidence applies to its recorded commit.
Final integration checks must confirm the assembled release.

## Change

API 0.2.13 supports completed-unit slot supply, periodic balance income, and capability-specific shortage rules.
Producer and constructor progress use the smallest active Rate constraint. Rates do not multiply.
The merged catalog determines exact integer RateQ, with the existing 65,536 scale limit.
Weapons use Disable. Manual Disable remains separate.

Supply effects freeze before commands. Live admission totals track changes within the tick.
Producer outputs and site completions share admission in physical source-coordinate order.
Exact source ties refuse both candidates. Runtime IDs do not rank visible admission outcomes.
Income clocks start at actual activation. Successful payment starts the next period.
A full account holds the whole due batch. It does not accumulate missed periods.

## Acceptance evidence

| Row | Executed evidence and remaining checks |
| --- | --- |
| OR1 | Component PASS. `runtime/result.json` and `runtime/final.log`: 321 tests, including seven operating tests with 31 scenarios. A small two-slot, two-supply package passes public archive validation. Negative cases cover references, kinds, duplicates, rates, reserved effects/scopes/counts, income health/amount/period, API floor, and merged-catalog scale. `boot/final.log` covers source-aware zero-period refusal and maximum authored period. |
| OR2 | Component and preview PASS. `backend-sim-tests.log`: boot exclusions, declared zeros, manual Disable, spawn, removal, zero health, revival, combat destruction, and immutable tick boundary. `engine/first.log`: mixed producer/site admission and retained-site rules. `audit/removal-stream.jsonl` and `audit/final.log`: real queued removal releases income and supply; effects update next tick. Final release confirmation is pending. |
| OR3 | Component and preview PASS. `engine/first.log`: exact minimum rates, global Q, fractional recovery, site health, unchanged payment/FIFO, and overflow recovery. `browser-full-checked/result.json`: full work 41/41 ticks; shortage work 30/61 ticks; constructor progress is recorded. Final release browser confirmation is pending. |
| OR4 | Component and preview PASS for executed checks. `supply/power-callers.log`, `backend-sim-tests.log`, and `browser-full-checked/result.json` cover weapon shortage, restoration, and preserved manual Disable. Owner separation also has export/replica coverage. Final release confirmation is pending. |
| OR5 | PARTIAL. `income/result.json`, `income/lifecycle/result.json`, `backend-sim-tests.log`, and `audit/stream.jsonl` cover two tick rates, 1,050 ms at 20 Hz, explicit activation, removal/revival, site/dead exclusions, and successful periodic credits. `browser-full-checked/result.json` records balance changes. Gathered has no implemented accumulator: the HUD source is a Null stub and stage-1 match results have empty stats. The distinct IncomeCredited event is tested; an actual Gathered before/after assertion remains unavailable. Later-completion clock timing needs final integrated evidence. |
| OR6 | Component PASS. `income/lifecycle/result.json` covers atomic wrong/stale/reversed grant refusal. `income/result.json` covers two due sources sharing a full account, an unaffected account, retry without backlog, and event-ordinal overflow. `engine/first.log` covers supply/income admission bounds, swapped factory/site IDs with unchanged physical winner, preserved final progress/resources/IDs, and recovery. |
| OR7 | PARTIAL. `income/result.json` compares repeated clock/account/event states. `audit/final.log` compares repeated real periodic-income streams. Existing no-operating frozen streams still need an operating-integration run; commands are below. This report does not claim equality for unexecuted historical streams. |
| OR8 | Component PASS. `client/export-checks.log` and `client/export-replica-checks.log` cover real server projections through postcard into replicas: owner privacy, foreign-record refusal, explicit zeros, delta-to-zero, resync, and fresh legacy-match reset. `client/summary.md` records counts and scope. |
| OR9 | Preview PASS: `browser-full-checked/result.json`, `browser-full-checked.log`, and screenshots in that directory. Real mouse orders remove a provider, build a replacement, restore work, preserve manual weapon Disable, then permit firing. The fixture also checks peer privacy, income, and explicit zero totals. Final release run is pending. Earlier failed/partial attempts remain evidence of fixes; they are not passes. |
| OR10 | PENDING. `clippy-final.log` records a successful strict workspace Clippy run. `backend-sim-tests.log` records 364 passed and one ignored. `audit/final.log` records 72 passed. Final full gates, compatibility equality, release cold probes, actual launcher, completed independent review, findings closure, and final README update remain pending. Review execution logs alone are not review approval. |

## Audit boundary

The new validator derives active contributions and live totals from immutable catalog definitions and active units.
It checks clock payloads and periods. It independently reconstructs successful income account batches and queued contributor removal.
Negative cases reject catalog-inconsistent clocks/contributions and coherent event tampering.

This is bounded replay coverage. No new independent model covers no-event full-account retries, scaled producer/site completion, or shortage-induced changes to public capability-rate rows.
The real simulation tests cover several of these behaviors. They do not make the auditor a complete independent simulation.
See `audit/result.json`, `audit/stream.jsonl`, and `audit/removal-stream.jsonl`.

## Compatibility and final checks

Run native Cargo through `~/.codex/bin/turts-cargo` when sharing a target.
The existing compatibility command is:

```sh
~/.codex/bin/turts-cargo test -p turts_pack --test m3_m2_stream --test forge_fixed_point --test forge_renamed_clone --test economy_variants --locked
```

Keep `UPDATE_M3_BASELINE` unset. The M2 test compares 400 ticks of base, haul, acme, and Cedar against `tests/m3/m2_operation_stream.json`.
Forge targets check fixed-point behavior and renamed-catalog trace equality.
These checks are also included in workspace tests.

Current `.github/workflows/ci.yml` requires format, strict native workspace Clippy, workspace WASM check excluding the server, and native workspace tests.
It also requires probe neutrality, dependency boundaries, Python harness/browser tests, re-freeze classifier tests, and `scripts/acceptance.sh`.
Run the required set once for the final integration candidate. Do not create additional lane gates.
The reviewed plan additionally requires the final release build, three scripted cold probes from `docs/qa/cold-play.md`, and the actual launcher check.
Do not replace the accepted launcher or invite a player before those probes pass.

## Corrections and limits

The authored period uses u32 milliseconds and u16 Hz. Their product fits u64 ticks.
OR1 therefore accepts the authored maximum and tests generic converter overflow separately. No arbitrary deadline cap was added.
Network supply, Queued counts, and BlockStart remain reserved and rejected.
Default Skirmish content stays unchanged; operating behavior is exercised in a separate mod fixture.
No final-release performance claim is made here. This draft does not authorize a main push.
